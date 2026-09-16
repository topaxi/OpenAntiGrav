//! name = "PSP-3000 LCD"
//! description = "The 3000's sharper panel: a faint horizontal structure that combs during motion, and a little persistence."
//!
//! [[param]]
//! name = "persistence"
//! default = 0.12
//! min = 0.0
//! max = 0.8
//! step = 0.02
//!
//! [[param]]
//! name = "static_lines"
//! default = 0.035
//! min = 0.0
//! max = 0.2
//! step = 0.005
//!
//! [[param]]
//! name = "motion_lines"
//! default = 0.075
//! min = 0.0
//! max = 0.3
//! step = 0.005
//!
//! [[param]]
//! name = "subpixel"
//! default = 0.08
//! min = 0.0
//! max = 0.5
//! step = 0.01
//!
//! [[param]]
//! name = "cell"
//! default = 1.0
//! min = 1.0
//! max = 8.0
//! step = 1.0
//!
//! [[param]]
//! name = "saturation"
//! default = 1.0
//! min = 0.5
//! max = 1.5
//! step = 0.05

// The PSP-3000's panel, as a display characteristic rather than as a 480x272
// grid: the structure is derived from the *output* pixels, so a 1080p window
// behaves like a 1080p-dense 3000 panel rather than a 272-row one stretched
// over it. `cell` is how many output pixels one simulated LCD pixel spans -
// 1 by default, 2 at 1080p reads as a 544-row panel.
//
// What the 3000 does, and why this is shaped the way it is:
//
// - Its "interlacing" is not fields. Contemporary reports describe thin
//   horizontal lines or combing appearing *during motion* around
//   high-contrast edges; the structure is the panel's own horizontal subpixel
//   arrangement, and motion is what makes it noticeable. So the lines here are
//   a shallow luminance ripple, never black, with a second ripple that only
//   appears where the frame differs from the last one.
// - It was the *faster* panel. The 1000/2000's smear is what `psp-lcd` does;
//   here persistence stays low.
// - Its colour was described as more saturated than the 1000/2000's.
//   `saturation` is neutral by default - nothing here was measured against a
//   panel, and a colour transform is the easiest thing to get plausibly wrong.
//
// None of these numbers is a measurement. See
// `docs/rendering/screen-filters.md` for where they come from and the
// stronger and weaker settings tried.

fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let cell = max(param_cell(), 1.0);
    let current = frame_at(uv);
    let previous = previous_at(uv);

    var color = mix(current, previous, param_persistence());

    // The static structure: one shallow cosine per two simulated rows,
    // centred on each row so the ripple sits between them.
    let row = pixel.y / cell;
    let phase = cos(row * 3.14159265);
    color *= 1.0 - param_static_lines() * phase;

    // The motion-visible combing. Where the picture changed since last frame
    // the same ripple deepens; where it did not, nothing is added, so a menu
    // standing still stays almost clean.
    let motion = luma(abs(current - previous));
    let visible = smoothstep(0.015, 0.18, motion);
    color *= 1.0 - param_motion_lines() * phase * visible;

    // The horizontal RGB stripe: each simulated column favours one channel,
    // energy-preserving, so the average over three columns is the input.
    let column = i32(floor(pixel.x / cell)) % 3;
    let favoured = vec3<f32>(
        select(0.0, 1.0, column == 0),
        select(0.0, 1.0, column == 1),
        select(0.0, 1.0, column == 2),
    );
    color *= 1.0 + param_subpixel() * (3.0 * favoured - 1.0);

    let grey = vec3<f32>(luma(color));
    color = mix(grey, color, param_saturation());

    return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
}
