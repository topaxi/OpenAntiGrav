//! Pulse's start gantry after `GO`: the digit panel stays in the aperture.
//!
//! **`#[ignore]`d and never run in CI.** It needs `data/images/pulse-psp-usa.chd`;
//! run it with `just test-data`.
//!
//! # What this pins
//!
//! `321Go_StartFinish.vex` teleports the digit panel (`start_light_321go`) and
//! its backlight (`start_light_background`) +9.99 in y at 6.000 s, "out of the
//! aperture", and loops their texture at the same instant. Run past it in open
//! air - which is all this project can do, the original having the track's own
//! structure to hide behind - and a full `3 2 1 GO` strip replays above the
//! banner: the maintainer's report from play. A capture of the original on
//! PPSSPP (`docs/rendering/start-gantry.md`, "What the original does after
//! `GO`") shows nothing of the kind: a stationary craft sees the panel hold a
//! strobing `GO` for the 21 s of race clock captured.
//!
//! So [`oag_game::race::gantry::held_on_go`] never lets the clock reach frame
//! 350. This test fails if that hold is dropped: at the raw clock the panel
//! is 9.99 units above the aperture, at the held one it never leaves it.

use oag_game::race::gantry::{GO_LOOP, held_on_go};
use oag_render::mesh;
use oag_vex::vex;

const GANTRY: &str = r"Data\Environments\321_Go\321Go_StartFinish.vex";

/// The panel's own translation in y at `seconds`, off the model's animated
/// node table - the same matrices the shader is handed.
fn panel_height(model: &mesh::Model, node: u32, seconds: f32) -> f32 {
    let draw = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .flatten()
    .find(|d| d.node == Some(node))
    .expect("the node draws");
    let first = model.indices[draw.range.start as usize] as usize;
    let slot = model.vertices[first].xform as usize;
    assert!(slot >= 1, "the panel is animated");
    let matrices = model.sample_anim_nodes(seconds);
    matrices[slot - 1][13]
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_digit_panel_never_leaves_the_aperture_once_go_is_up() {
    let Some(path) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives =
        oag_pulse::open(path.to_str().expect("image path is utf-8")).expect("open as Pulse");
    let blob = archives.read_name(GANTRY).expect("the gantry resolves");
    let nodes = vex::nodes(&blob).expect("the gantry parses");
    let model = mesh::build("gantry", &blob).expect("the gantry builds");
    let node = nodes
        .iter()
        .position(|n| {
            n.name
                .as_deref()
                .is_some_and(|name| name.ends_with("start_light_321goShape"))
        })
        .expect("the glyph node is present") as u32;

    // The negative control: the authored timeline, unheld, is what put the
    // strip above the banner.
    let raw = panel_height(&model, node, 7.0);
    assert!(
        raw > 9.0,
        "the authored track parks the panel above the aperture at 7 s: {raw}"
    );

    // 0.05 s steps out to two minutes: the countdown, the seam of the loop and
    // many periods past it.
    for step in 0..2400 {
        let raw = step as f32 * 0.05;
        let held = held_on_go(raw);
        assert!(
            held <= GO_LOOP.1,
            "held clock {held} runs past the loop end"
        );
        let y = panel_height(&model, node, held);
        assert!(
            y.abs() < 1.0,
            "the panel left the aperture at clock {raw} s (held to {held} s): y = {y}"
        );
    }
}
