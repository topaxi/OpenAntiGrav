// Per-object motion blur: a reconstruction filter over the scene's own
// velocity buffer.
//
// Written from the published description of McGuire, Hennessy, Bukowski and
// Osman, *A Reconstruction Filter for Plausible Motion Blur* (I3D 2012),
// rather than transliterated from any implementation - the rule ADR-0012
// sets and the way `fxaa.wgsl` was already done. The chain is the paper's:
// a tile pass reducing the velocity buffer to each tile's dominant motion
// (run separably, one axis at a time - see `fs_tile_max_x`), a neighbour pass
// spreading that dominance one tile outward, and a gather whose taps are
// weighted by depth ordering and by **both** surfaces' own velocities - the
// weighting that lets a moving object smear over a sharp background without
// the background bleeding across the object's silhouette. Both halves of
// "both" are load-bearing: see `fs_reconstruct`'s cylinder term.
//
// The velocity buffer itself is written by the scene - `mesh.wgsl`'s
// `velocity_of`, real per-draw motion - so unlike the camera-reprojection
// pass this file replaced, a craft holding station beside the camera is
// measured as still rather than computed as moving. See
// `docs/rendering/motion-blur.md` and ADR-0030.
//
// **Works in perceptual space**, the same convention every pass in `post`
// draws in (ADR-0020): the gather averages stored encoded values.

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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// The same fullscreen triangle `fxaa.wgsl` and the blit use.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

// Velocity and depth folded into one texture, so every later pass reads one
// binding whatever the scene's sample count was. The multisampled variant
// reads sample 0 - resolving would average velocity across silhouette edges
// into a vector that describes neither surface.
@fragment
fn fs_prepare(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.position.xy);
    let v = textureLoad(velocity_tex, p, 0).xy;
    let z = textureLoad(depth_tex, p, 0).x;
    return vec4<f32>(v, z, 0.0);
}

@fragment
fn fs_prepare_ms(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.position.xy);
    let v = textureLoad(velocity_ms_tex, p, 0).xy;
    let z = textureLoad(depth_ms_tex, p, 0).x;
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

@fragment
fn fs_reconstruct(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let centre = textureSampleLevel(colour_tex, colour_sampler, in.uv, 0.0);

    // Inside the viewport only: an aspect bar was drawn by no camera.
    let local = (in.uv - constants.rect_offset) / constants.rect_size;
    if local.x < 0.0 || local.x > 1.0 || local.y < 0.0 || local.y > 1.0 {
        return centre;
    }

    let tile = vec2<i32>(vec2<u32>(in.position.xy) / constants.tile);
    let dominant = reach_px(textureLoad(tile_tex, tile, 0).xy);
    let dominant_len = length(dominant);
    if dominant_len <= 0.5 {
        // Nothing in this neighbourhood moves as much as a pixel.
        return centre;
    }

    let own = textureLoad(prepared_tex, pixel, 0);
    let own_reach = length(reach_px(own.xy));
    let own_depth = own.z;

    // The centre tap's weight: the paper's `N / (K * |v(X)|)` shape - a
    // fast-moving pixel trusts its own sample less, spreading its energy
    // along its path, while a still one keeps most of its own colour.
    var weight = f32(TAPS) / max(own_reach, 0.5);
    var sum = centre * weight;

    let dither = jitter(in.position.xy);
    for (var i = 0; i < TAPS; i += 1) {
        if i == TAPS / 2 {
            continue; // the centre tap, already counted
        }
        let t = (f32(i) + dither + 1.0) / f32(TAPS + 1) - 0.5;
        let offset_px = dominant * t;
        let distance = length(offset_px);
        let uv = in.uv + offset_px / constants.rect_pixels * constants.rect_size;
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
    return sum / max(weight, 1e-4);
}

// The result back onto the scene target: the gather cannot sample the
// texture it is writing, so it lands in a scratch target and this carries it
// home.
@fragment
fn fs_copy(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(colour_tex, colour_sampler, in.uv, 0.0);
}
