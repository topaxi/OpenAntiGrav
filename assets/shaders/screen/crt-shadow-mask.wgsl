//! name = "CRT SHADOW MASK"
//! description = "A curved consumer CRT: gaussian scanlines on the title's own line count, a shadow mask, barrel curvature and dark corners."
//!
//! [[param]]
//! name = "scanline_hardness"
//! default = 8.0
//! min = 2.0
//! max = 20.0
//! step = 0.5
//!
//! [[param]]
//! name = "pixel_hardness"
//! default = 3.0
//! min = 1.0
//! max = 12.0
//! step = 0.5
//!
//! [[param]]
//! name = "mask"
//! default = 2.0
//! min = 0.0
//! max = 2.0
//! step = 1.0
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
//! default = 0.03
//! min = 0.0
//! max = 0.1
//! step = 0.005
//!
//! [[param]]
//! name = "corner"
//! default = 0.02
//! min = 0.0
//! max = 0.1
//! step = 0.005
//!
//! [[param]]
//! name = "brightness"
//! default = 1.15
//! min = 0.8
//! max = 2.0
//! step = 0.05

// A consumer CRT of the PS2's era, built from the technique every CRT shader
// of the last decade shares rather than from any one of them: each output
// pixel is the sum of the two nearest scanlines, each weighted by a gaussian
// in its distance from that line's centre, with the line's colour itself a
// three-tap horizontal gaussian across the source's columns - so a bright
// pixel bleeds sideways the way a beam does and a scanline fades above and
// below the way a beam's spot does. Over that goes a phosphor mask, and the
// whole thing is warped into a barrel and darkened at the corners. The
// hardness values are exponents of two per squared line unit, so 8 is a
// line at a quarter strength halfway to its neighbour.
//
// It reads best where the window has three or more pixels per scanline -
// 816 rows for the PSP's 272, 1350 for the PS2's 448 - and below that the
// lines beat against the pixel grid.
//
// The scanline count is the title's own: 272 lines on a PSP disc - which
// never met a CRT, but a player may want the look anyway - 448 on the PS2's
// PAL frame, 1080 on Wipeout HD's. `crt-interlaced` is this with the PS2's
// actual field alternation on top; `crt-aperture` is the flat-faced
// aperture-grille reading of the same idea.
//
// `mask`: 0 none, 1 aperture grille (vertical RGB stripes), 2 shadow mask
// (the stripes staggered by half a triad every other line).

// The beam maths runs in linear light and the result is re-encoded: a
// gaussian summed in gamma space loses brightness between lines that a
// phosphor never lost, and every CRT shader worth looking at makes the same
// round trip. A 2.2 power rather than the exact sRGB curve - the frame is
// gamma-encoded per ADR-0020 and nothing here is calibrated finer than that.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

// The barrel: pushes each axis out by the square of the other's distance
// from centre, which is what a curved glass face looks like head-on.
fn warp(uv: vec2<f32>) -> vec2<f32> {
    let centred = uv * 2.0 - 1.0;
    let bent = centred * (1.0 + param_curvature() * dot(centred, centred) * vec2<f32>(1.0, 1.4));
    return bent * 0.5 + 0.5;
}

// One scanline's colour under this pixel: three columns of the source on
// row `line`, gaussian-weighted by their horizontal distance.
fn line_colour(uv: vec2<f32>, line: f32) -> vec3<f32> {
    let grid = screen.native_size;
    let y = (line + 0.5) / grid.y;
    let column = uv.x * grid.x - 0.5;
    let base = floor(column);
    let hardness = param_pixel_hardness();
    var total = vec3<f32>(0.0);
    var weight = 0.0;
    for (var i = -1; i <= 1; i++) {
        let x = base + f32(i);
        let d = column - x;
        let w = exp2(-hardness * d * d);
        total += to_linear(frame_at(vec2<f32>((x + 0.5) / grid.x, y))) * w;
        weight += w;
    }
    return total / weight;
}

fn screen_filter(uv_in: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let uv = warp(uv_in);
    // Off the glass. Applied at the end rather than returned early: a
    // `textureSample` after a return some pixels took is not uniform control
    // flow, and naga refuses it.
    let on_glass = select(0.0, 1.0, all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));

    // Which scanline this output pixel is nearest, and how far from its
    // centre in line units; the line above or below contributes too.
    let row = uv.y * screen.native_size.y - 0.5;
    let nearest = floor(row);
    let hardness = param_scanline_hardness();
    let d0 = row - nearest;
    let d1 = d0 - 1.0;
    let w0 = exp2(-hardness * d0 * d0);
    let w1 = exp2(-hardness * d1 * d1);
    var color = line_colour(uv, nearest) * w0 + line_colour(uv, nearest + 1.0) * w1;

    // The mask, in output pixels: a triad every three columns, staggered
    // every other row for a shadow mask.
    let kind = param_mask();
    if kind > 0.5 {
        var x = pixel.x;
        if kind > 1.5 && (i32(floor(pixel.y)) & 1) == 1 {
            x += 1.5;
        }
        let triad = i32(floor(x)) % 3;
        let dark = param_mask_dark();
        let light = param_mask_light();
        var mask = vec3<f32>(dark);
        if triad == 0 {
            mask.r = light;
        } else if triad == 1 {
            mask.g = light;
        } else {
            mask.b = light;
        }
        color *= mask;
    }

    // Dark corners: the distance to the nearest edge on each axis, softened.
    let edge = min(uv, 1.0 - uv);
    // Never zero: `smoothstep` with equal edges divides by it.
    let corner = max(param_corner(), 0.001);
    let vignette = smoothstep(0.0, corner, edge.x) * smoothstep(0.0, corner, edge.y);
    color *= vignette * param_brightness() * on_glass;

    return clamp(to_gamma(color), vec3<f32>(0.0), vec3<f32>(1.0));
}
