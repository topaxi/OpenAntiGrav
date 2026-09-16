//! name = "CRT APERTURE GRILLE"
//! description = "A flat-faced aperture-grille monitor: vertical phosphor stripes, scanlines that fatten with brightness, a little sideways bloom and two damper wires."
//!
//! [[param]]
//! name = "beam_min"
//! default = 0.18
//! min = 0.05
//! max = 0.5
//! step = 0.01
//!
//! [[param]]
//! name = "beam_max"
//! default = 0.55
//! min = 0.2
//! max = 1.0
//! step = 0.01
//!
//! [[param]]
//! name = "bloom"
//! default = 0.3
//! min = 0.0
//! max = 1.0
//! step = 0.05
//!
//! [[param]]
//! name = "grille"
//! default = 0.35
//! min = 0.0
//! max = 1.0
//! step = 0.05
//!
//! [[param]]
//! name = "grille_pitch"
//! default = 3.0
//! min = 2.0
//! max = 6.0
//! step = 1.0
//!
//! [[param]]
//! name = "damper_wires"
//! default = 1.0
//! min = 0.0
//! max = 1.0
//! step = 1.0
//!
//! [[param]]
//! name = "brightness"
//! default = 1.2
//! min = 0.8
//! max = 2.0
//! step = 0.05

// The other kind of tube: a flat face with vertical RGB phosphor stripes
// behind a grille of fine wires, held by two horizontal damper wires a third
// of the way down and up that cast faint shadows across the picture. No
// curvature, no shadow-mask stagger. What it has that `crt-shadow-mask` does
// not is a beam whose spot *widens with brightness* - a bright scanline is a
// fat one and a dark one is a thin line with black between - which is the
// single most recognisable thing about this kind of monitor next to the
// stripes.
//
// The scanline profile is a gaussian in the distance from the line's centre
// whose width is `beam_min` for black and `beam_max` for white, in line
// units. Each output pixel sums the two nearest lines. `bloom` adds a wider,
// dimmer tap either side of each source column, which is what a bright
// pixel's glow into its neighbours looks like through the glass.

// Linear light for the beam sums, re-encoded at the end - see
// `crt-shadow-mask` for why.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
}

fn beam(distance: f32, brightness: f32) -> f32 {
    let width = mix(param_beam_min(), param_beam_max(), brightness);
    let x = distance / width;
    return exp(-x * x);
}

// The source on scanline `line` under this pixel, with the bloom taps.
fn line_colour(uv: vec2<f32>, line: f32) -> vec3<f32> {
    let grid = screen.native_size;
    let y = (line + 0.5) / grid.y;
    let dx = 1.0 / grid.x;
    let centre = to_linear(frame_at(vec2<f32>(uv.x, y)));
    let glow = param_bloom();
    let sides = to_linear(frame_at(vec2<f32>(uv.x - dx, y)))
        + to_linear(frame_at(vec2<f32>(uv.x + dx, y)));
    return (centre + sides * glow * 0.5) / (1.0 + glow);
}

fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let row = uv.y * screen.native_size.y - 0.5;
    let nearest = floor(row);
    let d0 = row - nearest;
    let d1 = 1.0 - d0;

    let c0 = line_colour(uv, nearest);
    let c1 = line_colour(uv, nearest + 1.0);
    // Per channel, because a red line is fat where its green is thin.
    let w0 = vec3<f32>(beam(d0, c0.r), beam(d0, c0.g), beam(d0, c0.b));
    let w1 = vec3<f32>(beam(d1, c1.r), beam(d1, c1.g), beam(d1, c1.b));
    var color = c0 * w0 + c1 * w1;

    // The grille: `grille_pitch` output pixels per triad, each channel lit
    // in its own third and dimmed by `grille` outside it.
    let pitch = max(param_grille_pitch(), 1.0);
    let phase = fract(pixel.x / pitch) * 3.0;
    let stripe = i32(floor(phase)) % 3;
    let dark = 1.0 - param_grille();
    var mask = vec3<f32>(dark);
    if stripe == 0 {
        mask.r = 1.0;
    } else if stripe == 1 {
        mask.g = 1.0;
    } else {
        mask.b = 1.0;
    }
    color *= mask;

    // The damper wires: two shadows one output pixel tall, at a third and
    // two thirds of the height, softened over a pixel either side.
    if param_damper_wires() > 0.5 {
        let h = screen.output_size.y;
        let wire_a = abs(pixel.y - h / 3.0);
        let wire_b = abs(pixel.y - 2.0 * h / 3.0);
        let shadow = 1.0 - 0.35 * (smoothstep(1.5, 0.0, wire_a) + smoothstep(1.5, 0.0, wire_b));
        color *= shadow;
    }

    return clamp(to_gamma(color * param_brightness()), vec3<f32>(0.0), vec3<f32>(1.0));
}
