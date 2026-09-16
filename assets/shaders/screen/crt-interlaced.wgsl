//! name = "CRT INTERLACED"
//! description = "The PS2 on a television over composite: one field a frame, phosphor decay between them, chroma smeared sideways, a slot mask and a curve."
//!
//! [[param]]
//! name = "interlace"
//! default = 1.0
//! min = 0.0
//! max = 1.0
//! step = 1.0
//!
//! [[param]]
//! name = "decay"
//! default = 0.55
//! min = 0.0
//! max = 0.95
//! step = 0.05
//!
//! [[param]]
//! name = "scanline_hardness"
//! default = 6.0
//! min = 2.0
//! max = 20.0
//! step = 0.5
//!
//! [[param]]
//! name = "composite"
//! default = 0.6
//! min = 0.0
//! max = 1.0
//! step = 0.05
//!
//! [[param]]
//! name = "mask_dark"
//! default = 0.6
//! min = 0.0
//! max = 1.0
//! step = 0.05
//!
//! [[param]]
//! name = "mask_light"
//! default = 1.4
//! min = 1.0
//! max = 2.0
//! step = 0.05
//!
//! [[param]]
//! name = "curvature"
//! default = 0.035
//! min = 0.0
//! max = 0.1
//! step = 0.005
//!
//! [[param]]
//! name = "corner"
//! default = 0.025
//! min = 0.0
//! max = 0.1
//! step = 0.005
//!
//! [[param]]
//! name = "brightness"
//! default = 1.25
//! min = 0.8
//! max = 2.0
//! step = 0.05

// Wipeout Pulse's PS2 port drew a 640x448 frame and the television showed it
// interlaced: 224 lines one field, the other 224 the next, sixty fields a
// second, with each line's phosphor fading while its neighbour was being
// drawn. That is what this does, literally - each frame this filter draws
// lights the lines of one field, alternating with `screen.frame`, and the
// other field's lines are last frame's output multiplied by `decay`. Over a
// 60 Hz window that is the real thing; over a faster one the fields alternate
// faster than they did, and over a slower one, slower. `interlace = 0` lights
// every line every frame and keeps the rest. **A still capture shows one
// field**: `--presented --screenshot` composites one frame with a black
// history, which is half the lines lit and the other half dark, exactly as a
// photograph of a television with a fast enough shutter is.
//
// It is also the one preset the notes behind `psp-3000` say *not* to use for
// a PSP: alternating fields is what a CRT did and is not what the 3000's
// panel does.
//
// `composite` is the cable: a composite signal carries chroma at a lower
// bandwidth than luma, so colour edges smear sideways where brightness edges
// stay put. The frame is split into Rec. 601 luma and chroma, the chroma is
// blurred over five source columns and the luma over three, and the two are
// recombined. At 0 it is an RGB cable.

// Linear light for the beam sums, re-encoded at the end - see
// `crt-shadow-mask` for why.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

fn warp(uv: vec2<f32>) -> vec2<f32> {
    let centred = uv * 2.0 - 1.0;
    let bent = centred * (1.0 + param_curvature() * dot(centred, centred) * vec2<f32>(1.0, 1.4));
    return bent * 0.5 + 0.5;
}

// Rec. 601 RGB to YIQ and back, which is the space a composite signal is
// modulated in.
fn to_yiq(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(c, vec3<f32>(0.299, 0.587, 0.114)),
        dot(c, vec3<f32>(0.596, -0.274, -0.322)),
        dot(c, vec3<f32>(0.211, -0.523, 0.312)),
    );
}

fn to_rgb(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(c, vec3<f32>(1.0, 0.956, 0.621)),
        dot(c, vec3<f32>(1.0, -0.272, -0.647)),
        dot(c, vec3<f32>(1.0, -1.106, 1.703)),
    );
}

// One scanline's colour under this pixel, through the cable: luma from the
// three nearest source columns, chroma from the five.
fn line_colour(uv: vec2<f32>, line: f32) -> vec3<f32> {
    let grid = screen.native_size;
    let y = (line + 0.5) / grid.y;
    let column = uv.x * grid.x - 0.5;
    let base = floor(column);
    let smear = param_composite();
    var luma_total = 0.0;
    var luma_weight = 0.0;
    var chroma_total = vec2<f32>(0.0);
    var chroma_weight = 0.0;
    for (var i = -2; i <= 2; i++) {
        let x = base + f32(i);
        let d = abs(column - x);
        let yiq = to_yiq(to_linear(frame_at(vec2<f32>((x + 0.5) / grid.x, y))));
        // Luma: a sharp tent over the three nearest columns.
        let wl = max(1.5 - d, 0.0);
        luma_total += yiq.x * wl;
        luma_weight += wl;
        // Chroma: a wide tent over all five, widened by the cable.
        let wc = max(1.0 + 2.0 * smear - d, 0.0);
        chroma_total += yiq.yz * wc;
        chroma_weight += wc;
    }
    return to_rgb(vec3<f32>(luma_total / luma_weight, chroma_total / chroma_weight));
}

fn screen_filter(uv_in: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let uv = warp(uv_in);
    let on_glass = select(0.0, 1.0, all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));

    let row = uv.y * screen.native_size.y - 0.5;
    let nearest = floor(row);
    let hardness = param_scanline_hardness();
    let d0 = row - nearest;
    let d1 = d0 - 1.0;
    // Which field is being drawn this frame, and whether each of the two
    // lines under this pixel belongs to it. With `interlace` off both do.
    let interlaced = param_interlace() > 0.5;
    let field = i32(screen.frame) & 1;
    let line0 = i32(nearest);
    let lit0 = select(1.0, select(0.0, 1.0, (line0 & 1) == field), interlaced);
    let lit1 = select(1.0, select(0.0, 1.0, ((line0 + 1) & 1) == field), interlaced);
    let w0 = exp2(-hardness * d0 * d0) * lit0;
    let w1 = exp2(-hardness * d1 * d1) * lit1;
    var drawn = line_colour(uv, nearest) * w0 + line_colour(uv, nearest + 1.0) * w1;

    // A slot mask in output pixels, staggered every other line.
    var x = pixel.x;
    if (i32(floor(pixel.y)) & 1) == 1 {
        x += 1.5;
    }
    let triad = i32(floor(x)) % 3;
    var mask = vec3<f32>(param_mask_dark());
    if triad == 0 {
        mask.r = param_mask_light();
    } else if triad == 1 {
        mask.g = param_mask_light();
    } else {
        mask.b = param_mask_light();
    }
    drawn *= mask;

    let edge = min(uv, 1.0 - uv);
    let corner = max(param_corner(), 0.001);
    let vignette = smoothstep(0.0, corner, edge.x) * smoothstep(0.0, corner, edge.y);
    drawn *= vignette * param_brightness() * on_glass;

    // What the other field left behind: last frame's output, faded. Read at
    // the *unwarped* coordinate, because that is where last frame's output
    // was written - and it already carries its mask and its corners, which
    // is why they were applied to `drawn` alone above rather than to both.
    let fading = to_linear(previous_at(uv_in)) * param_decay();
    let color = max(drawn, fading);

    return clamp(to_gamma(color), vec3<f32>(0.0), vec3<f32>(1.0));
}
