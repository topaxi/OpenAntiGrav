// Subpixel Morphological Anti-Aliasing, ported to WGSL.
//
// Ported from `SMAA.hlsl` of Jorge Jimenez et al.'s SMAA
// (https://github.com/iryoku/smaa), which is MIT-licensed. The licence text
// travels with this project in `licences/SMAA-MIT.txt`.
//
// This is a transliteration of upstream's **MEDIUM** preset specifically:
// `SMAA_THRESHOLD = 0.1`, `SMAA_MAX_SEARCH_STEPS = 8`, diagonal pattern
// search and corner detection both disabled - exactly the defines upstream's
// own `SMAA_PRESET_MEDIUM` sets, not a reduced port of the HIGH/ULTRA path.
// The arithmetic that survives that preset is left as upstream has it,
// comments included, so it can be diffed against `SMAA.hlsl` function for
// function.
//
// Two deliberate portability departures from upstream, neither of which
// changes the arithmetic:
//
// - **No vertex-shader offset precompute.** Upstream's `SMAA*VS` functions
//   compute each pass's per-pixel texture offsets in the vertex shader and
//   interpolate them to the fragment stage, saving ALU on hardware where
//   interpolators are cheaper than pixel-shader math. WGSL does not allow
//   `array<vec4<f32>, 3>` as a vertex-to-fragment varying, and a fullscreen
//   triangle has three vertices - there is nothing to amortise here, so
//   every offset is recomputed directly in the fragment shader from the
//   interpolated UV instead.
// - **Direct module-scope resource access.** Upstream threads every texture
//   and sampler through its helper functions as parameters, because its
//   `SMAATexture2D` macro has to support shader models that cannot access a
//   global resource from inside a called function. WGSL has no such
//   restriction, so the helpers below read `edges_tex`, `area_tex` and so on
//   directly rather than carrying them as arguments.
//
// **Works in perceptual space** - the edge detection pass is `smaa.wesl`'s
// own reason to want sRGB-encoded values, the same as `fsr1.wesl` and
// `fxaa.wesl`. See `post`'s module docs.

import oag_shaders::colour::luma_rec709;
import oag_shaders::fullscreen::{fullscreen_clip, fullscreen_uv};

const THRESHOLD: f32 = 0.1;
const MAX_SEARCH_STEPS: i32 = 8;
const LOCAL_CONTRAST_ADAPTATION_FACTOR: f32 = 2.0;

const AREATEX_MAX_DISTANCE: f32 = 16.0;
const AREATEX_PIXEL_SIZE: vec2<f32> = vec2<f32>(1.0 / 160.0, 1.0 / 560.0);
const AREATEX_SUBTEX_SIZE: f32 = 1.0 / 7.0;
const SEARCHTEX_SIZE: vec2<f32> = vec2<f32>(66.0, 33.0);
const SEARCHTEX_PACKED_SIZE: vec2<f32> = vec2<f32>(64.0, 16.0);

// Which sub-rectangle of the (allocation-sized) targets was actually drawn.
//
// Every other number these passes want comes from `textureDimensions`, which
// returns the resource and so needs no help. This is the one thing that is not
// derivable: the mapping from the fullscreen triangle's 0..1 onto the drawn
// rectangle. Both fields are exactly 1.0 while the rectangle is the whole
// texture - see `post::sub_rectangle` - so the arithmetic below is inert until
// a resolution controller moves it. Bound separately per pass because each
// pass has its own layout; it is the same buffer behind all three.
struct Constants {
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
}

// One global for all three passes, at a binding no texture or sampler here
// uses: naga refuses a uniform that shares a slot with a resource of another
// kind, even in an entry point that reads neither, and the three passes each
// number their own textures from zero. All three layouts carry it, and it is
// the same buffer behind each.
@group(0) @binding(4) var<uniform> constants: Constants;

//-----------------------------------------------------------------------------
// Pass 1: Luma Edge Detection

@group(0) @binding(0) var edge_color_tex: texture_2d<f32>;
@group(0) @binding(1) var edge_point_sampler: sampler;

fn edge_sample_luma(uv: vec2<f32>) -> f32 {
    let at = min(uv, constants.uv_max);
    return luma_rec709(textureSample(edge_color_tex, edge_point_sampler, at).rgb);
}

struct EdgeVsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn edge_vs_main(@builtin(vertex_index) index: u32) -> EdgeVsOut {
    var out: EdgeVsOut;
    out.uv = fullscreen_uv(index);
    out.position = fullscreen_clip(index);
    return out;
}

@fragment
fn edge_fs_main(in: EdgeVsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(edge_color_tex));
    let rt = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let texcoord = in.uv * constants.uv_scale;
    let t4 = vec4<f32>(texcoord, texcoord);
    let rt4 = vec4<f32>(rt, rt);

    // offset[0]: left and top. offset[1]: right and bottom. offset[2]:
    // left-left and top-top.
    let offset0 = t4 + vec4<f32>(-1.0, 0.0, 0.0, -1.0) * rt4;
    let offset1 = t4 + vec4<f32>(1.0, 0.0, 0.0, 1.0) * rt4;
    let offset2 = t4 + vec4<f32>(-2.0, 0.0, 0.0, -2.0) * rt4;

    let luma_m = edge_sample_luma(texcoord);
    let luma_left = edge_sample_luma(offset0.xy);
    let luma_top = edge_sample_luma(offset0.zw);

    var delta = vec4<f32>(0.0);
    delta.x = abs(luma_m - luma_left);
    delta.y = abs(luma_m - luma_top);
    var edges = step(vec2<f32>(THRESHOLD), delta.xy);

    if edges.x + edges.y == 0.0 {
        discard;
    }

    let luma_right = edge_sample_luma(offset1.xy);
    let luma_bottom = edge_sample_luma(offset1.zw);
    delta.z = abs(luma_m - luma_right);
    delta.w = abs(luma_m - luma_bottom);
    var max_delta = max(delta.xy, delta.zw);

    let luma_left_left = edge_sample_luma(offset2.xy);
    let luma_top_top = edge_sample_luma(offset2.zw);
    delta.z = abs(luma_left - luma_left_left);
    delta.w = abs(luma_top - luma_top_top);
    max_delta = max(max_delta, delta.zw);

    let final_delta = max(max_delta.x, max_delta.y);
    edges = edges * step(vec2<f32>(final_delta), LOCAL_CONTRAST_ADAPTATION_FACTOR * delta.xy);

    return vec4<f32>(edges, 0.0, 0.0);
}

//-----------------------------------------------------------------------------
// Pass 2: Blending Weight Calculation

@group(0) @binding(0) var blend_edges_tex: texture_2d<f32>;
@group(0) @binding(1) var blend_area_tex: texture_2d<f32>;
@group(0) @binding(2) var blend_search_tex: texture_2d<f32>;
@group(0) @binding(3) var blend_linear_sampler: sampler;

struct BlendVsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn blend_vs_main(@builtin(vertex_index) index: u32) -> BlendVsOut {
    var out: BlendVsOut;
    out.uv = fullscreen_uv(index);
    out.position = fullscreen_clip(index);
    return out;
}

// SMAASearchLength: looks the number of extra texels to add for the last
// search step up in the search texture.
fn search_length(e: vec2<f32>, offset: f32) -> f32 {
    var scale = SEARCHTEX_SIZE * vec2<f32>(0.5, -1.0);
    var bias = SEARCHTEX_SIZE * vec2<f32>(offset, 1.0);
    scale = scale + vec2<f32>(-1.0, 1.0);
    bias = bias + vec2<f32>(0.5, -0.5);
    scale = scale * (vec2<f32>(1.0) / SEARCHTEX_PACKED_SIZE);
    bias = bias * (vec2<f32>(1.0) / SEARCHTEX_PACKED_SIZE);
    return textureSampleLevel(blend_search_tex, blend_linear_sampler, scale * e + bias, 0.0).r;
}

fn search_x_left(start: vec2<f32>, end: f32, rt: vec2<f32>) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(0.0, 1.0);
    loop {
        if !(texcoord.x > end && e.g > 0.8281 && e.r == 0.0) {
            break;
        }
        e = textureSampleLevel(blend_edges_tex, blend_linear_sampler, texcoord, 0.0).rg;
        texcoord = texcoord - vec2<f32>(2.0, 0.0) * rt;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.0) + 3.25;
    return rt.x * offset + texcoord.x;
}

fn search_x_right(start: vec2<f32>, end: f32, rt: vec2<f32>) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(0.0, 1.0);
    loop {
        if !(texcoord.x < end && e.g > 0.8281 && e.r == 0.0) {
            break;
        }
        e = textureSampleLevel(blend_edges_tex, blend_linear_sampler, texcoord, 0.0).rg;
        texcoord = texcoord + vec2<f32>(2.0, 0.0) * rt;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.5) + 3.25;
    return -rt.x * offset + texcoord.x;
}

fn search_y_up(start: vec2<f32>, end: f32, rt: vec2<f32>) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(1.0, 0.0);
    loop {
        if !(texcoord.y > end && e.r > 0.8281 && e.g == 0.0) {
            break;
        }
        e = textureSampleLevel(blend_edges_tex, blend_linear_sampler, texcoord, 0.0).rg;
        texcoord = texcoord - vec2<f32>(0.0, 2.0) * rt;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.0) + 3.25;
    return rt.y * offset + texcoord.y;
}

fn search_y_down(start: vec2<f32>, end: f32, rt: vec2<f32>) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(1.0, 0.0);
    loop {
        if !(texcoord.y < end && e.r > 0.8281 && e.g == 0.0) {
            break;
        }
        e = textureSampleLevel(blend_edges_tex, blend_linear_sampler, texcoord, 0.0).rg;
        texcoord = texcoord + vec2<f32>(0.0, 2.0) * rt;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.5) + 3.25;
    return -rt.y * offset + texcoord.y;
}

// SMAAArea: the area (0 to 1, packed rg) covered by the edge given the
// distance to both crossing edges and their own values.
fn smaa_area(dist: vec2<f32>, e1: f32, e2: f32, offset: f32) -> vec2<f32> {
    var texcoord = vec2<f32>(AREATEX_MAX_DISTANCE) * round(4.0 * vec2<f32>(e1, e2)) + dist;
    texcoord = AREATEX_PIXEL_SIZE * texcoord + 0.5 * AREATEX_PIXEL_SIZE;
    texcoord.y = AREATEX_SUBTEX_SIZE * offset + texcoord.y;
    return textureSampleLevel(blend_area_tex, blend_linear_sampler, texcoord, 0.0).rg;
}

@fragment
fn blend_fs_main(in: BlendVsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(blend_edges_tex));
    let rt = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let texcoord = in.uv * constants.uv_scale;
    let t4 = vec4<f32>(texcoord, texcoord);
    let rt4 = vec4<f32>(rt, rt);
    let pixcoord = texcoord * dims;

    // @PSEUDO_GATHER4: these two offsets are shifted by (-0.25, -0.125) and
    // (-0.125, -0.25) respectively so that a bilinear fetch through them
    // returns four neighbouring edge texels blended together - see
    // `search_x_left`/`search_y_up`'s 0.8281 threshold, which is what
    // decodes that blend back into "is there an edge here".
    let offset0 = t4 + vec4<f32>(-0.25, -0.125, 1.25, -0.125) * rt4;
    let offset1 = t4 + vec4<f32>(-0.125, -0.25, -0.125, 1.25) * rt4;
    let offset2 = vec4<f32>(rt.x, rt.x, rt.y, rt.y)
        * (vec4<f32>(-2.0, 2.0, -2.0, 2.0) * f32(MAX_SEARCH_STEPS))
        + vec4<f32>(offset0.x, offset0.z, offset1.y, offset1.w);

    var weights = vec4<f32>(0.0);
    let e = textureSample(blend_edges_tex, blend_linear_sampler, texcoord).rg;

    if e.g > 0.0 {
        // Edge at north: search left and right along the row.
        var coords = vec3<f32>(0.0);
        coords.x = search_x_left(offset0.xy, offset2.x, rt);
        coords.y = offset1.y;
        var d = vec2<f32>(0.0);
        d.x = coords.x;
        let e1 = textureSampleLevel(blend_edges_tex, blend_linear_sampler, coords.xy, 0.0).r;

        coords.z = search_x_right(offset0.zw, offset2.y, rt);
        d.y = coords.z;
        d = abs(round(dims.xx * d - pixcoord.xx));
        let sqrt_d = sqrt(d);

        let e2 = textureSampleLevel(
            blend_edges_tex,
            blend_linear_sampler,
            coords.zy,
            0.0,
            vec2<i32>(1, 0),
        ).r;

        weights = vec4<f32>(smaa_area(sqrt_d, e1, e2, 0.0), weights.ba);
    }

    if e.r > 0.0 {
        // Edge at west: search up and down along the column.
        var coords = vec3<f32>(0.0);
        coords.y = search_y_up(offset1.xy, offset2.z, rt);
        coords.x = offset0.x;
        var d = vec2<f32>(0.0);
        d.x = coords.y;
        let e1 = textureSampleLevel(blend_edges_tex, blend_linear_sampler, coords.xy, 0.0).g;

        coords.z = search_y_down(offset1.zw, offset2.w, rt);
        d.y = coords.z;
        d = abs(round(dims.yy * d - pixcoord.yy));
        let sqrt_d = sqrt(d);

        let e2 = textureSampleLevel(
            blend_edges_tex,
            blend_linear_sampler,
            coords.xz,
            0.0,
            vec2<i32>(0, 1),
        ).g;

        weights = vec4<f32>(weights.rg, smaa_area(sqrt_d, e1, e2, 0.0));
    }

    return weights;
}

//-----------------------------------------------------------------------------
// Pass 3: Neighborhood Blending

@group(0) @binding(0) var neighborhood_color_tex: texture_2d<f32>;
@group(0) @binding(1) var neighborhood_blend_tex: texture_2d<f32>;
@group(0) @binding(2) var neighborhood_linear_sampler: sampler;

struct NeighborhoodVsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn neighborhood_vs_main(@builtin(vertex_index) index: u32) -> NeighborhoodVsOut {
    var out: NeighborhoodVsOut;
    let uv = fullscreen_uv(index);
    out.uv = uv;
    out.position = fullscreen_clip(index);
    return out;
}

@fragment
fn neighborhood_fs_main(in: NeighborhoodVsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(neighborhood_color_tex));
    let rt = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let texcoord = in.uv * constants.uv_scale;
    let uv_max = constants.uv_max;
    let offset = min(
        vec4<f32>(texcoord, texcoord) + vec4<f32>(1.0, 0.0, 0.0, 1.0) * vec4<f32>(rt, rt),
        vec4<f32>(uv_max, uv_max),
    );

    var a = vec4<f32>(0.0);
    a.x = textureSample(neighborhood_blend_tex, neighborhood_linear_sampler, offset.xy).a;
    a.y = textureSample(neighborhood_blend_tex, neighborhood_linear_sampler, offset.zw).g;
    a.w = textureSample(neighborhood_blend_tex, neighborhood_linear_sampler, texcoord).x;
    a.z = textureSample(neighborhood_blend_tex, neighborhood_linear_sampler, texcoord).z;

    if dot(a, vec4<f32>(1.0)) < 1e-5 {
        return textureSampleLevel(
            neighborhood_color_tex,
            neighborhood_linear_sampler,
            min(texcoord, uv_max),
            0.0,
        );
    }

    let h = max(a.x, a.z) > max(a.y, a.w);

    var blending_offset = vec4<f32>(0.0, a.y, 0.0, a.w);
    var blending_weight = a.yw;
    if h {
        blending_offset = vec4<f32>(a.x, 0.0, a.z, 0.0);
        blending_weight = a.xz;
    }
    blending_weight = blending_weight / dot(blending_weight, vec2<f32>(1.0));

    // Clamped, unlike the blend pass's own edge taps: a colour tap that
    // reached past the drawn rectangle would pull the clear inward as a dark
    // fringe along the extent's boundary, which is the one artefact a
    // sub-extent would otherwise put on screen. The edge taps need no clamp -
    // the cleared region reads as no edge, exactly as the frame border does.
    let blending_coord = min(
        blending_offset * vec4<f32>(rt.x, rt.y, -rt.x, -rt.y) + vec4<f32>(texcoord, texcoord),
        vec4<f32>(uv_max, uv_max),
    );

    var color = blending_weight.x
        * textureSampleLevel(neighborhood_color_tex, neighborhood_linear_sampler, blending_coord.xy, 0.0);
    color = color
        + blending_weight.y
            * textureSampleLevel(neighborhood_color_tex, neighborhood_linear_sampler, blending_coord.zw, 0.0);
    return color;
}
