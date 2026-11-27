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
//! default = 0.5
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
//
// What keeps colour where it was (the doc's "Colour audit"):
//
// - The grid only darkens, so nothing clips and no channel is lost.
// - `gap` and `subpixel` are shares of *light* taken, applied through the
//   panel's 2.2 curve, so every level and hue is dimmed by the same share.
// - Each output pixel takes the average of the structure over its own
//   footprint, so every channel gets the same share at every window size,
//   including the ones where a cell is not a whole number of pixels.
// - The gaps sit between all the stripes, not only at the cell's edges, so
//   the stripes lose the same amount.
// - The smear mixes the structured picture with the last structured picture,
//   so a still frame settles to one frame's structure instead of the mask
//   feeding back into itself.

// Light let through from 0 to `x`, per stripe (red, green, blue), where `x`
// counts LCD pixels: each stripe is a third of a pixel and opens with a gap
// `width` of a stripe wide.
fn stripes_upto(x: f32, width: f32) -> vec3<f32> {
    let whole = floor(x);
    let rest = vec3<f32>(x - whole) - vec3<f32>(0.0, 1.0 / 3.0, 2.0 / 3.0);
    let third = vec3<f32>(1.0 / 3.0);
    let length = vec3<f32>(whole / 3.0) + clamp(rest, vec3<f32>(0.0), third);
    let dark = vec3<f32>(whole * width / 3.0) + clamp(rest, vec3<f32>(0.0), third * width);
    return length - param_gap() * dark;
}

// The same along rows, where each LCD pixel opens with a gap `width` of its
// height.
fn rows_upto(y: f32, width: f32) -> f32 {
    let whole = floor(y);
    return y - param_gap() * (whole * width + min(y - whole, width));
}

fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let grid = screen.native_size;
    let cell = uv * grid;

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

    // A grey floor under everything - the backlight through a panel that
    // never quite closes - and a cooler white if asked for.
    var color = param_black_lift() + current * (1.0 - param_black_lift());
    color *= mix(vec3<f32>(1.0), vec3<f32>(0.94, 0.97, 1.04), param_cool());

    // The grid over it, averaged across this pixel's footprint. At a window
    // narrower than the LCD the footprint covers whole stripes and the
    // structure becomes an even dimming, which is the right answer there.
    let half = 0.5 * grid * screen.texel;
    let width = param_gap_width();
    let passed = (stripes_upto(cell.x + half.x, width) - stripes_upto(cell.x - half.x, width))
        / (2.0 * half.x);
    let row = (rows_upto(cell.y + half.y, width) - rows_upto(cell.y - half.y, width))
        / (2.0 * half.y);
    let total = passed.r + passed.g + passed.b;
    let taken = param_subpixel();
    let light = (vec3<f32>((1.0 - taken) * total) + taken * passed) * row;
    color *= pow(light, vec3<f32>(1.0 / 2.2));

    // The slow panel: what was showing last frame is still partly showing.
    return clamp(mix(color, previous_at(uv), param_persistence()), vec3<f32>(0.0), vec3<f32>(1.0));
}
