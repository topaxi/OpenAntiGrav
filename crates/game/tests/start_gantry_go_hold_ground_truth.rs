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
//! What holds it is Pulse's race manager (`RaceManager_Update`, `0x08829778`,
//! read from `BOOT.BIN`, confidence 85): before the first line crossing it
//! keeps the gantry's time in `[3.2, 5.5)`, and [`Clock::PULSE`] runs that
//! law. This test fails if that window is dropped: at the raw clock the panel
//! is 9.99 units above the aperture, on Pulse's clock it never leaves it.

use oag_game::race::gantry::{Clock, PULSE_PRE_LAP_WINDOW};
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

    // Every tick out to two minutes: the countdown, the seam of the window
    // and many periods past it, with the craft never crossing the line.
    for tick in 0..7200 {
        let held = Clock::PULSE.seconds(tick);
        let raw = tick as f32 / 60.0;
        assert!(
            held < PULSE_PRE_LAP_WINDOW.to,
            "clock {held} runs past the pre-lap window at tick {tick}"
        );
        let y = panel_height(&model, node, held);
        assert!(
            y.abs() < 1.0,
            "the panel left the aperture at clock {raw} s (held to {held} s): y = {y}"
        );
    }
}
