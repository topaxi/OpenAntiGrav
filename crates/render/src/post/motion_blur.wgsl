// Camera motion blur: depth-based reprojection against the previous frame's
// camera, then a straight-line gather along the resulting screen-space
// velocity.
//
// This pass is the first consumer of the temporal infrastructure ADR-0013
// names as missing for TAA: a per-pixel motion vector. The vector here is
// camera-only - each pixel's depth is unprojected through the inverse of the
// current view-projection and reprojected through the previous frame's, so a
// static world point's screen travel is recovered exactly and a moving
// object's own motion is not seen at all (it blurs by however the *camera*
// moved across it). Per-object velocity needs the scene pass to write a
// velocity target, which is the next increment - see ADR-0028.
//
// **Works in perceptual space**, the same convention every pass in `post`
// draws in (see that module's docs): the gather averages the stored encoded
// values rather than decoding to linear light first. ADR-0020 makes the
// stored gamma encoding authoritative for this renderer, and a decode/encode
// pair here would make this the one pass that disagrees with the rest of the
// chain about what a texel means.

struct Constants {
    // Inverse of the camera the frame was just drawn with: pixel + depth back
    // to a world position.
    inv_view_proj: mat4x4<f32>,
    // The camera of the last frame that *differed* - see `MotionBlur::observe`
    // for why that is not simply "the previous render call".
    prev_view_proj: mat4x4<f32>,
    // The viewport rectangle inside the target, both in uv units of the whole
    // target: the scene can occupy a sub-rectangle of a capture, with the
    // aspect bars outside it, and NDC maps to this rectangle rather than to
    // the target.
    rect_offset: vec2<f32>,
    rect_size: vec2<f32>,
    // The same rectangle's size in pixels, for the stretch cap.
    rect_pixels: vec2<f32>,
    // What fraction of the frame-to-frame travel the shutter is open for.
    strength: f32,
    // The cap on how far the gather may reach, as a fraction of the viewport
    // height - the bound that keeps a camera cut from smearing the whole frame.
    max_stretch: f32,
}

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;
@group(0) @binding(3) var depth_tex: texture_depth_2d;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// The same fullscreen triangle `fxaa.wgsl` and the blit use: no vertex
// buffer, and no seam down a quad's diagonal.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

// An odd count so one tap lands exactly on the pixel itself, centred so the
// smear reaches half a shutter backwards and half forwards - a one-sided
// gather visibly drags the picture behind the camera instead of blurring it.
const TAPS: i32 = 9;

@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let center = textureSampleLevel(scene_tex, scene_sampler, in.uv, 0.0);

    // Into viewport-relative coordinates: 0..1 across the scene's own
    // rectangle, whatever sub-rectangle of the target that is.
    let local = (in.uv - constants.rect_offset) / constants.rect_size;
    if local.x < 0.0 || local.x > 1.0 || local.y < 0.0 || local.y > 1.0 {
        // An aspect bar. Nothing out here was drawn by the camera, so there
        // is no motion to reconstruct for it.
        return center;
    }

    // The depth buffer covers the whole target, so the fragment's own pixel
    // coordinate indexes it directly - no viewport mapping, no sampler.
    let depth = textureLoad(depth_tex, vec2<i32>(in.position.xy), 0);

    // Unproject through the current camera, reproject through the previous
    // one. wgpu NDC: x right, y *up* (so the uv y flips), z 0..1.
    let ndc = vec4<f32>(local.x * 2.0 - 1.0, 1.0 - local.y * 2.0, depth, 1.0);
    let world = constants.inv_view_proj * ndc;
    let prev = constants.prev_view_proj * (world / world.w);
    if prev.w <= 0.0 {
        // Behind the previous camera's eye: the reprojection is meaningless,
        // and a wrong smear is worse than a sharp pixel.
        return center;
    }
    let prev_ndc = prev.xy / prev.w;
    let prev_local = vec2<f32>(prev_ndc.x * 0.5 + 0.5, 0.5 - prev_ndc.y * 0.5);

    var travel = (local - prev_local) * constants.strength;

    // Cap the reach in pixels of the viewport, direction preserved: a camera
    // cut or a view switch produces one frame of arbitrarily large velocity,
    // and the cap is what turns that from a whole-frame smear into a bounded
    // one.
    let travel_px = travel * constants.rect_pixels;
    let speed_px = length(travel_px);
    let max_px = constants.max_stretch * constants.rect_pixels.y;
    if speed_px > max_px {
        travel *= max_px / speed_px;
    }
    if speed_px < 0.5 {
        // Under half a pixel of travel the gather is nine reads of the same
        // texel - the still-camera case, and most pixels of a gentle pan.
        return center;
    }

    // The straight-line gather, centred on the pixel. Taps are clamped to the
    // viewport rectangle rather than the target, so a smear at the frame's
    // edge stretches the edge rather than pulling an aspect bar's black in.
    var sum = vec4<f32>(0.0);
    for (var i = 0; i < TAPS; i += 1) {
        let t = f32(i) / f32(TAPS - 1) - 0.5;
        let tap = clamp(local + travel * t, vec2<f32>(0.0), vec2<f32>(1.0));
        let uv = constants.rect_offset + tap * constants.rect_size;
        sum += textureSampleLevel(scene_tex, scene_sampler, uv, 0.0);
    }
    return sum / f32(TAPS);
}

// The result back onto the scene target: the gather cannot sample the texture
// it is writing, so it lands in a scratch target and this carries it home.
@fragment
fn fs_copy(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(scene_tex, scene_sampler, in.uv, 0.0);
}
