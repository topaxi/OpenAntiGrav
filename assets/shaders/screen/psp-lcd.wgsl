//! name = "PSP LCD"
//! description = "The 1000/2000's panel: the 480x272 pixel grid, its RGB stripe, lifted blacks and the smear of a slow LCD."
//!
//! [[param]]
//! name = "persistence"
//! default = 0.4
//! min = 0.0
//! max = 0.8
//! step = 0.05
//!
//! [[param]]
//! name = "native_grid"
//! default = 1.0
//! min = 0.0
//! max = 1.0
//! step = 1.0
//!
//! [[param]]
//! name = "gap"
//! default = 0.25
//! min = 0.0
//! max = 0.8
//! step = 0.05
//!
//! [[param]]
//! name = "gap_width"
//! default = 0.12
//! min = 0.02
//! max = 0.4
//! step = 0.02
//!
//! [[param]]
//! name = "subpixel"
//! default = 0.25
//! min = 0.0
//! max = 0.8
//! step = 0.05
//!
//! [[param]]
//! name = "black_lift"
//! default = 0.04
//! min = 0.0
//! max = 0.2
//! step = 0.01
//!
//! [[param]]
//! name = "cool"
//! default = 0.0
//! min = 0.0
//! max = 1.0
//! step = 0.05

// The original panel, as a grid: 480x272 LCD pixels however large the window
// is, each with a dark border and an RGB stripe inside it. This is the
// "native-display" reading - the picture is resampled onto the authored grid
// first, so a render scale above 100% is still shown through 130,560 pixels -
// as against `psp-3000`, which keeps the window's own pixels and applies the
// panel's character to them. `native_grid = 0` turns the resample off and
// draws the grid over the window's own picture instead.
//
// The 1000 and 2000 shared a slow panel: the ghosting behind a moving craft
// was the thing every review of the 2000 mentioned, and `persistence` at
// 0.4 is PPSSPP's own default for the same idea (its "LCD Persistence"
// shader, whose range is 0.3 to 0.8). Their blacks were grey next to the
// 3000's, which `black_lift` approximates; the 2000's panel was reported as
// bluish next to the 3000's, which `cool` offers and leaves off, since
// nothing here was measured against one. See
// `docs/rendering/screen-filters.md`.

fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let grid = screen.native_size;
    // Where in its LCD pixel this output pixel is, 0..1 on each axis.
    let cell = uv * grid;
    let inside = fract(cell);

    var current: vec3<f32>;
    if param_native_grid() > 0.5 {
        // A 2x2 box around the cell's centre, so a supersampled frame averages
        // down rather than being decimated to one of its pixels.
        let centre = (floor(cell) + 0.5) / grid;
        let quarter = 0.25 / grid;
        current = 0.25 * (
            frame_at(centre + vec2<f32>(-quarter.x, -quarter.y)) +
            frame_at(centre + vec2<f32>(quarter.x, -quarter.y)) +
            frame_at(centre + vec2<f32>(-quarter.x, quarter.y)) +
            frame_at(centre + vec2<f32>(quarter.x, quarter.y))
        );
    } else {
        current = frame_at(uv);
    }

    // The slow panel: what was showing last frame is still partly showing.
    var color = mix(current, previous_at(uv), param_persistence());

    // A grey floor under everything - the backlight through a panel that
    // never quite closes - and a cooler white if asked for.
    color = param_black_lift() + color * (1.0 - param_black_lift());
    color *= mix(vec3<f32>(1.0), vec3<f32>(0.94, 0.97, 1.04), param_cool());

    // The border between LCD pixels. `gap_width` is in cell units, so it is a
    // fraction of the cell at any window size; at a small window the cell is
    // a pixel or two and the sampler blurs the border into a slight darkening,
    // which is the right answer there.
    let width = param_gap_width();
    let border_x = smoothstep(0.0, width, inside.x) * smoothstep(1.0, 1.0 - width, inside.x);
    let border_y = smoothstep(0.0, width, inside.y) * smoothstep(1.0, 1.0 - width, inside.y);
    color *= 1.0 - param_gap() * (1.0 - border_x * border_y);

    // The vertical RGB stripe inside each cell, energy-preserving across the
    // three thirds.
    let third = i32(floor(inside.x * 3.0)) % 3;
    let favoured = vec3<f32>(
        select(0.0, 1.0, third == 0),
        select(0.0, 1.0, third == 1),
        select(0.0, 1.0, third == 2),
    );
    color *= 1.0 + param_subpixel() * (3.0 * favoured - 1.0);

    return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
}
