// Per-object motion blur: a reconstruction filter over the scene's own
// velocity buffer.
//
// Written from the published description of McGuire, Hennessy, Bukowski and
// Osman, *A Reconstruction Filter for Plausible Motion Blur* (I3D 2012),
// rather than transliterated from any implementation - the rule ADR-0012
// sets and the way `fxaa.wesl` was already done. The chain is the paper's:
// a tile pass reducing the velocity buffer to each tile's dominant motion
// (run separably, one axis at a time - see `fs_tile_max_x`), a neighbour pass
// spreading that dominance one tile outward, and a gather whose taps are
// weighted by depth ordering and by **both** surfaces' own velocities - the
// weighting that lets a moving object smear over a sharp background without
// the background bleeding across the object's silhouette. Both halves of
// "both" are load-bearing: see `fs_reconstruct`'s cylinder term.
//
// The velocity buffer itself is written by the scene - `mesh`'s
// `velocity_of`, real per-draw motion - so unlike the camera-reprojection
// pass this file replaced, a craft holding station beside the camera is
// measured as still rather than computed as moving. See
// `docs/rendering/motion-blur.md` and ADR-0030.
//
// **Works in perceptual space**, the same convention every pass in `post`
// draws in (ADR-0020): the gather averages stored encoded values.

import oag_shaders::fullscreen::fullscreen_clip;

struct Constants {
    // The viewport rectangle inside the target, in uv of the whole target:
    // a capture draws the scene into a sub-rectangle with aspect bars
    // outside it.
    rect_offset: vec2<f32>,
    rect_size: vec2<f32>,
    // The same rectangle in pixels.
    rect_pixels: vec2<f32>,
    // What fraction of the tick's motion the shutter is open for.
    strength: f32,
    // The cap on the gather's reach in pixels - also the tile edge, which is
    // what makes the neighbour pass's one-tile reach sufficient.
    max_px: f32,
    // One texel of the full-size targets, in uv.
    inv_size: vec2<f32>,
    // The tile edge in pixels, as an integer for the reduction loops.
    tile: u32,
    _pad: f32,
    // The camera's impact shake as a clip-space map from a pixel to where the
    // same direction sat a tick ago under the shake alone; identity for none.
    shake: mat4x4<f32>,
}

@group(0) @binding(0) var colour_tex: texture_2d<f32>;
@group(0) @binding(1) var colour_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;
// `prepare` output: xy the surface's velocity in uv units, z its depth.
@group(0) @binding(3) var prepared_tex: texture_2d<f32>;
// The dominant velocity of this pixel's tile neighbourhood.
@group(0) @binding(4) var tile_tex: texture_2d<f32>;

// The prepare pass's own inputs - the scene's raw velocity and depth
// attachments, single-sampled or multisampled. Separate group so the
// downstream passes never bind them.
@group(1) @binding(0) var velocity_tex: texture_2d<f32>;
@group(1) @binding(1) var depth_tex: texture_2d<f32>;
@group(1) @binding(2) var velocity_ms_tex: texture_multisampled_2d<f32>;
@group(1) @binding(3) var depth_ms_tex: texture_multisampled_2d<f32>;

// **The framebuffer position is the only coordinate any pass here reads, and
// that is what lets every pass rasterise the render extent rather than the
// whole allocation.** A `set_viewport` maps this triangle's clip space onto
// the viewport rectangle, so an interpolated 0..1 uv would measure across the
// *rectangle* while every texture bound here is the size of the whole target.
// `@builtin(position)` is in framebuffer pixels either way, so `target_uv`
// below is the one that keeps meaning the same thing under a viewport - and
// lands on the texel centre exactly, where the interpolant only did so
// because the two happened to coincide.
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.position = fullscreen_clip(index);
    return out;
}

// This fragment's uv in the *target's* space - see `VertexOutput`.
fn target_uv(position: vec4<f32>) -> vec2<f32> {
    return position.xy * constants.inv_size;
}

// The camera shake's own screen motion at this pixel, in the buffer's uv
// units, to be taken out of the velocity read there. The shake turns the view
// about the eye, so where a pixel's direction sat a tick ago depends on the
// pixel and not its depth: the depth sample only has to be a valid one for the
// homogeneous point. Exactly zero for the identity matrix, so a frame with no
// shake reads the buffer unchanged.
fn shake_motion(position: vec4<f32>, depth: f32) -> vec2<f32> {
    let uv = (target_uv(position) - constants.rect_offset) / constants.rect_size;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let before = constants.shake * vec4<f32>(ndc, depth, 1.0);
    if before.w <= 0.0 {
        return vec2<f32>(0.0);
    }
    return (ndc - before.xy / before.w) * vec2<f32>(0.5, -0.5);
}

// Velocity and depth folded into one texture, so every later pass reads one
// binding whatever the scene's sample count was. The multisampled variant
// reads sample 0 - resolving would average velocity across silhouette edges
// into a vector that describes neither surface.
//
// The velocity leaves with the camera's impact shake taken out of it - the
// buffer is true screen motion, and the shake is not motion to blur.
@fragment
fn fs_prepare(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.position.xy);
    let z = textureLoad(depth_tex, p, 0).x;
    let v = textureLoad(velocity_tex, p, 0).xy - shake_motion(in.position, z);
    return vec4<f32>(v, z, 0.0);
}

@fragment
fn fs_prepare_ms(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.position.xy);
    let z = textureLoad(depth_ms_tex, p, 0).x;
    let v = textureLoad(velocity_ms_tex, p, 0).xy - shake_motion(in.position, z);
    return vec4<f32>(v, z, 0.0);
}

// The velocity a pixel's gather follows is its *neighbourhood's* dominant
// motion, not its own: a sharp background pixel beside a fast object must
// still gather along the object's path to receive its smear. Tile-max
// reduces each `tile`-pixel square to its largest velocity; neighbour-max
// takes the largest of the 3x3 tiles around each.
//
// **Tile-max is separable, and has to be.** A max over a square is the max of
// the row maxima, so the square reduction splits into a horizontal pass
// (full-size to `ceil(w / tile)` by `h`) and a vertical one (to the tile
// grid). The reason is not the instruction count - both forms touch every
// pixel once - but occupancy: the square form runs `tile * tile` serial
// `textureLoad`s inside only `ceil(w / tile) * ceil(h / tile)` fragments,
// which at 1080p is 7,569 loads across 299 threads and left most of the GPU
// idle. Measured on this project's development adapter, splitting it cut the
// whole chain from 2.6 ms to 1.2 ms a frame - within noise of stubbing the
// reduction out entirely, so what is left of it is now free. See
// `docs/rendering/motion-blur.md` for the machine and the method.
@fragment
fn fs_tile_max_x(in: VertexOutput) -> @location(0) vec2<f32> {
    let row = i32(in.position.y);
    let first = i32(in.position.x) * i32(constants.tile);
    let limit = vec2<i32>(textureDimensions(prepared_tex)) - vec2<i32>(1);
    var best = vec2<f32>(0.0);
    var best_len = 0.0;
    for (var x = 0u; x < constants.tile; x += 1u) {
        let p = min(vec2<i32>(first + i32(x), row), limit);
        let v = textureLoad(prepared_tex, p, 0).xy;
        let len = dot(v, v);
        if len > best_len {
            best_len = len;
            best = v;
        }
    }
    return best;
}

// The vertical half, reading `fs_tile_max_x`'s output through the tile slot.
@fragment
fn fs_tile_max_y(in: VertexOutput) -> @location(0) vec2<f32> {
    let column = i32(in.position.x);
    let first = i32(in.position.y) * i32(constants.tile);
    let limit = vec2<i32>(textureDimensions(tile_tex)) - vec2<i32>(1);
    var best = vec2<f32>(0.0);
    var best_len = 0.0;
    for (var y = 0u; y < constants.tile; y += 1u) {
        let p = min(vec2<i32>(column, first + i32(y)), limit);
        let v = textureLoad(tile_tex, p, 0).xy;
        let len = dot(v, v);
        if len > best_len {
            best_len = len;
            best = v;
        }
    }
    return best;
}

@fragment
fn fs_neighbour_max(in: VertexOutput) -> @location(0) vec2<f32> {
    let centre = vec2<i32>(in.position.xy);
    let limit = vec2<i32>(textureDimensions(tile_tex)) - vec2<i32>(1);
    var best = vec2<f32>(0.0);
    var best_len = 0.0;
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            let p = clamp(centre + vec2<i32>(x, y), vec2<i32>(0), limit);
            let v = textureLoad(tile_tex, p, 0).xy;
            let len = dot(v, v);
            if len > best_len {
                best_len = len;
                best = v;
            }
        }
    }
    return best;
}

// An odd count so one tap can land on the pixel itself; the per-pixel jitter
// below hides the banding a fixed lattice of this few taps would show.
const TAPS: i32 = 15;
// The fewest taps a moving pixel gets, and the smear length each further tap
// is spent on - see `fs_reconstruct`.
const MIN_TAPS: i32 = 5;
const TAP_SPACING_PX: f32 = 4.0;

// How far apart two NDC depths must be before one surface counts as
// decisively in front of the other. Chosen for this renderer's projection
// family (0..1 depth, track-scaled far planes) by eye, not derived; the
// softness only shapes how silhouette edges hand over between the two cone
// weights.
const SOFT_Z: f32 = 0.005;

fn cone(distance: f32, radius: f32) -> f32 {
    return clamp(1.0 - distance / max(radius, 0.25), 0.0, 1.0);
}

fn cylinder(distance: f32, radius: f32) -> f32 {
    // A sub-pixel radius is a surface that is not smearing: no shared-motion
    // credit. Also what keeps `smoothstep` away from its undefined
    // zero-width-edge case, which on at least one adapter quietly returned
    // full weight and dimmed still surfaces beside moving ones.
    if radius < 0.25 {
        return 0.0;
    }
    return 1.0 - smoothstep(0.95 * radius, 1.05 * radius, distance);
}

// `1` as `a` comes decisively in front of `b`.
fn soft_depth_compare(a: f32, b: f32) -> f32 {
    return clamp(1.0 - (a - b) / SOFT_Z, 0.0, 1.0);
}

// One low-bias hash per pixel, for the tap jitter. Determinism rules bind
// the simulation, not a shader; this is still a pure function of the pixel,
// so a capture stays reproducible.
fn jitter(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5;
}

// A velocity in uv units into pixels of the viewport, shutter applied and
// capped - the shared meaning of "how far does this surface smear".
fn reach_px(velocity: vec2<f32>) -> vec2<f32> {
    let px = velocity * constants.strength * constants.rect_pixels;
    let len = length(px);
    if len > constants.max_px {
        return px * (constants.max_px / len);
    }
    return px;
}

// What one gather found: the weighted `sum` of its taps and their total
// `weight`, the centre tap's own share of it, and whether the neighbourhood
// moved at all - `colour` is the plain centre when it did not.
struct Gathered {
    colour: vec4<f32>,
    weight: f32,
    centre_weight: f32,
    moved: bool,
}

// The gather both resolutions share, at `position` on the full-size pixel
// grid.
fn gather(position: vec4<f32>) -> Gathered {
    var in: VertexOutput;
    in.position = position;
    let pixel = vec2<i32>(in.position.xy);
    let here = target_uv(in.position);
    let centre = textureSampleLevel(colour_tex, colour_sampler, here, 0.0);

    // Inside the viewport only: an aspect bar was drawn by no camera. Still
    // here with the raster restricted to the viewport, because a rectangle
    // rounds to whole pixels and this is what the boundary one does.
    let local = (here - constants.rect_offset) / constants.rect_size;
    if local.x < 0.0 || local.x > 1.0 || local.y < 0.0 || local.y > 1.0 {
        return Gathered(centre, 0.0, 0.0, false);
    }

    // **The tile lookup is bilinear between the four nearest tile centres.** A
    // nearest fetch gives every pixel of a tile the same dominant velocity, so
    // wherever the scene's motion ramps - the road ahead of a craft, most of
    // the screen in a turn - the gather's span jumps discontinuously at each
    // tile edge. The cone weights hide most of it, because they follow the
    // smooth per-pixel reach rather than the tile's; what survives is a step
    // in blur *intensity* on the tile lattice, 5-13 % of the smear's own level
    // in a single pixel column at every 87 px boundary, always the same sign.
    // That lattice is anchored to the window rather than to the world, so the
    // scene flows through a stationary grid and the eye reads squares.
    //
    // This used to offset the lookup by up to +/-half a tile with a per-pixel
    // hash, trading the lattice for noise. Measured in
    // `docs/rendering/motion-blur.md` that wobble was most of the grain in a
    // smear: every pixel landed on a different tile, so neighbours gathered
    // along different spans. Interpolating makes the span a continuous
    // function of the pixel instead, so there is neither a lattice nor noise.
    // Each of the four tiles is a neighbour-max, so the blend still covers a
    // pixel's own neighbourhood wherever motion is smooth; at a real edge of a
    // fast object it falls to a fraction of the object's reach over the tile's
    // width, which is the same ramp the object's own cone weights carry.
    let tile_last = vec2<i32>(textureDimensions(tile_tex)) - vec2<i32>(1);
    let tile_at = in.position.xy / f32(constants.tile) - vec2<f32>(0.5);
    let tile_base = vec2<i32>(floor(tile_at));
    let tile_f = tile_at - floor(tile_at);
    let v00 = textureLoad(tile_tex, clamp(tile_base, vec2<i32>(0), tile_last), 0).xy;
    let v10 = textureLoad(tile_tex, clamp(tile_base + vec2<i32>(1, 0), vec2<i32>(0), tile_last), 0).xy;
    let v01 = textureLoad(tile_tex, clamp(tile_base + vec2<i32>(0, 1), vec2<i32>(0), tile_last), 0).xy;
    let v11 = textureLoad(tile_tex, clamp(tile_base + vec2<i32>(1, 1), vec2<i32>(0), tile_last), 0).xy;
    let dominant = reach_px(mix(mix(v00, v10, tile_f.x), mix(v01, v11, tile_f.x), tile_f.y));
    let dominant_len = length(dominant);
    if dominant_len <= 0.5 {
        // Nothing in this neighbourhood moves as much as a pixel.
        return Gathered(centre, 0.0, 0.0, false);
    }

    let own = textureLoad(prepared_tex, pixel, 0);
    let own_reach = length(reach_px(own.xy));
    let own_depth = own.z;

    // The centre tap's weight: the paper's `N / (K * |v(X)|)` shape - a
    // fast-moving pixel trusts its own sample less, spreading its energy
    // along its path, while a still one keeps most of its own colour.
    // **As many taps as the smear is long, up to `TAPS`**: one per
    // `TAP_SPACING_PX` of the neighbourhood's reach, odd so one still lands on
    // the pixel, and never fewer than `MIN_TAPS`. A short smear sampled
    // fifteen times re-reads the same few texels; this spends the fetches
    // where the distance is. Measured on HD at 1600x900 on an integrated GPU,
    // the gather 13.7 -> 9.4 ms for a mean change of 0.7/255.
    let taps = clamp(i32(ceil(dominant_len / TAP_SPACING_PX)) | 1, MIN_TAPS, TAPS);
    let centre_weight = f32(taps) / max(own_reach, 0.5);
    var weight = centre_weight;
    var sum = centre * weight;

    let dither = jitter(in.position.xy);
    for (var i = 0; i < taps; i += 1) {
        if i == taps / 2 {
            continue; // the centre tap, already counted
        }
        let t = (f32(i) + dither + 1.0) / f32(taps + 1) - 0.5;
        let offset_px = dominant * t;
        let distance = length(offset_px);
        let uv = here + offset_px / constants.rect_pixels * constants.rect_size;
        // Clamped to the viewport, so a smear at the frame's edge stretches
        // the edge rather than pulling an aspect bar's black in.
        let tap_local = clamp((uv - constants.rect_offset) / constants.rect_size, vec2<f32>(0.0), vec2<f32>(1.0));
        let tap_uv = constants.rect_offset + tap_local * constants.rect_size;
        let dims = vec2<i32>(textureDimensions(prepared_tex)) - vec2<i32>(1);
        let tap_pixel = min(vec2<i32>(tap_uv / constants.inv_size), dims);

        let tap = textureLoad(prepared_tex, tap_pixel, 0);
        let tap_reach = length(reach_px(tap.xy));

        // The paper's three terms. A tap in front of the centre contributes
        // where its own motion covers the distance (its smear crosses us); a
        // tap behind contributes where the centre's motion covers it (we
        // smear over it); the cylinder term keeps two similarly-moving
        // surfaces - the inside of one blurred object - blending evenly.
        //
        // **The cylinder radius is the smaller of the two surfaces' own
        // reaches**, which is what the paper's product of two cylinders comes
        // to, and it is the only term with no depth gate. Widening it to the
        // neighbourhood's dominant reach - which is `>= own_reach` by
        // construction, so the substitution only ever adds weight - drops the
        // "is the centre even moving?" half of the test, and a still surface
        // in front of a fast one then takes a full-weight tap of whatever is
        // behind it. That is the background dragging across a foreground
        // silhouette the design page names as "the single difference between
        // an effect that reads as motion blur and one that reads as a bug".
        // Measured at a 22 % darkening on a still block over a fast
        // background before the fix; pinned by
        // `a_still_surface_over_a_moving_background_keeps_its_colour`. Do not
        // floor this radius the way `cone` floors its own: a zero reach here
        // *means* "not smearing", and `cylinder`'s own sub-pixel early-out is
        // what turns that into zero weight.
        let front = soft_depth_compare(tap.z, own_depth);
        let behind = soft_depth_compare(own_depth, tap.z);
        let w = front * cone(distance, tap_reach)
            + behind * cone(distance, own_reach)
            + cylinder(distance, min(tap_reach, own_reach)) * 2.0;

        sum += textureSampleLevel(colour_tex, colour_sampler, tap_uv, 0.0) * w;
        weight += w;
    }
    return Gathered(sum, weight, centre_weight, true);
}

// The full-resolution gather: every pixel, written straight into the
// scratch target `fs_copy` carries home.
@fragment
fn fs_reconstruct(in: VertexOutput) -> @location(0) vec4<f32> {
    let found = gather(in.position);
    if !found.moved {
        return found.colour;
    }
    return found.colour / max(found.weight, 1e-4);
}

// **At half resolution, and what it writes is a blend.** The pass runs over a
// target half the scene's size each way, so `position` is scaled to the
// full-size pixel grid first: a half-size texel centre `i + 0.5` lands on
// `2i + 1`, the corner four full-size pixels share. What it returns is the
// blurred colour premultiplied by `amount` - the share of the result that came
// from taps other than the centre - with `amount` in alpha, and `fs_composite`
// blends the scene toward it at full resolution. A neighbourhood that does not
// move returns zero and keeps every texel of its own detail; a still pixel a
// fast surface smears across takes the smear, because its taps carry the
// weight; only the smear itself is resolved at half resolution. A quarter of
// the gather's fragments - see `docs/rendering/motion-blur.md`.
//
// **Not a difference from the half-size centre**, which was tried first: added
// back onto the full-size scene, it leaves anything smaller than a half-size
// texel unsmeared, and `the_multisampled_prepare_variant_reads_sample_zero_
// and_the_chain_runs`'s two-pixel block stayed sharp.
@fragment
fn fs_reconstruct_half(half_res: VertexOutput) -> @location(0) vec4<f32> {
    let found = gather(vec4<f32>(half_res.position.xy * 2.0, half_res.position.zw));
    if !found.moved {
        return vec4<f32>(0.0);
    }
    let total = max(found.weight, 1e-4);
    let amount = 1.0 - found.centre_weight / total;
    return vec4<f32>((found.colour / total).rgb * amount, amount);
}

// The half-size blend `fs_reconstruct` wrote, laid over the scene at full
// resolution into the scratch target `fs_copy` then carries home.
//
// **Upsampled by depth as well as by distance.** Each of the four half-size
// texels around this pixel is weighted bilinearly *and* by how near the depth
// of the full-size pixel it was gathered for lies to this pixel's own, on the
// gather's own `SOFT_Z` scale - so a still surface's edge pixel takes its own
// side's blend and not the smear of a fast background beside it. Plain
// bilinear read 201 of 255 at the still block's silhouette in
// `a_still_surface_over_a_moving_background_keeps_its_colour`. Where no texel
// shares the pixel's depth, the nearest in depth stands in.
//
// The blend is premultiplied, so a texel with no smear pulls toward nothing
// rather than toward black. Bound differently from the gather: the blend is
// `colour_tex`, the full-size depths are `prepared_tex`, and the scene is read
// exactly through `tile_tex`'s load slot. Alpha is the scene's own - the glow
// mask the bloom has already read.
@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let scene = textureLoad(tile_tex, pixel, 0);
    let depth = textureLoad(prepared_tex, pixel, 0).z;
    let half_last = vec2<i32>(textureDimensions(colour_tex)) - vec2<i32>(1);
    let full_last = vec2<i32>(textureDimensions(prepared_tex)) - vec2<i32>(1);
    let h = in.position.xy * 0.5 - vec2<f32>(0.5);
    let base = vec2<i32>(floor(h));
    let f = h - floor(h);
    var sum = vec4<f32>(0.0);
    var total = 0.0;
    var nearest = vec4<f32>(0.0);
    var nearest_dz = 1.0e9;
    for (var j = 0; j < 2; j += 1) {
        for (var i = 0; i < 2; i += 1) {
            let texel = clamp(base + vec2<i32>(i, j), vec2<i32>(0), half_last);
            let gathered_for = min(texel * 2 + vec2<i32>(1), full_last);
            let dz = abs(textureLoad(prepared_tex, gathered_for, 0).z - depth);
            let smear = textureLoad(colour_tex, texel, 0);
            let bilinear = select(1.0 - f.x, f.x, i == 1) * select(1.0 - f.y, f.y, j == 1);
            let w = bilinear * max(1.0 - dz / SOFT_Z, 0.0);
            sum += smear * w;
            total += w;
            if dz < nearest_dz {
                nearest_dz = dz;
                nearest = smear;
            }
        }
    }
    let smear = select(nearest, sum / max(total, 1.0e-6), total > 1.0e-4);
    return vec4<f32>(scene.rgb * (1.0 - smear.a) + smear.rgb, scene.a);
}

// The result back onto the scene target: the gather cannot sample the
// texture it is writing, so it lands in a scratch target and this carries it
// home.
@fragment
fn fs_copy(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(colour_tex, colour_sampler, target_uv(in.position), 0.0);
}
