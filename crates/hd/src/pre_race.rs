//! Wipeout HD / Fury's pre-race flyby: after the loading screen the race opens on a
//! fly-over of the circuit with a `START RACE` prompt, and one tap of cross skips it
//! (`docs/gameplay/race-intro.md`, "Other titles").
//!
//! What was watched on RPCS3 (2026-10-07, Talons Junction): the fly-over plays after the
//! load, cuts between shots, shows no HUD, waits on the prompt and loops at the file's length
//! until cross is tapped. The camera file's mode choice, the hold and lock, the HUD delay and
//! the field of view were not measured, so each says so.

use oag_title::pre_race::{Ending, PreRace, Skip, Sourced};
use oag_title::{Platform, Platforms};

/// HD's flyby on a PS3 disc or install.
pub const PRE_RACE: PreRace = PreRace {
    on: Platforms::Only(&[Platform::Ps3]),
    // The executable builds `%s\%sstart_grid%s.vex`, `start_grid_de` and `start_grid_ta`; the
    // plain pair is taken for a normal race. Which mode reads which is not measured.
    grid: "start_grid.vex",
    grid_reversed: Some("start_grid_reversed.vex"),
    // The `gridCamera` leaf is the same Maya export as Pulse's, so Pulse's measured field is
    // borrowed until HD's own is read off a capture.
    fov_degrees: Sourced::chosen(54.308_994),
    hold_ticks: Sourced::chosen(0),
    lock_ticks: Sourced::chosen(0),
    hud_delay_ticks: Sourced::chosen(0),
    // Watched through a 230 s hold on RPCS3 (2026-10-07, Talons Junction): shot 19 repeats shot 1
    // after 187 s, the file's 188.33 s, and the prompt never leaves; nothing ended it.
    ending: Sourced::measured(Ending::OnlyBySkip),
    // `docs/rendering/start-gantry.md`: the harness "taps cross once to skip the track fly-over".
    skip: Sourced::measured(Skip::Press),
    panel: false,
};
