//! Wipeout HD / Fury's pre-race flyby: after the loading screen the race opens on a
//! fly-over of the circuit with a `START RACE` prompt, and one tap of cross skips it
//! (`docs/gameplay/race-intro.md`, "Other titles").
//!
//! What was watched on RPCS3 (2026-10-07, Talons Junction): the fly-over plays after the
//! load, cuts between shots, shows no HUD, waits on the prompt and loops at the file's length
//! until cross is tapped. Which file a mode flies is read off the executable, not watched; the
//! hold and lock, the HUD delay and the field of view were not measured, so each says so.

use oag_title::pre_race::{Ending, ModeGrid, PreRace, Skip, Sourced};
use oag_title::{Platform, Platforms};

/// HD's flyby on a PS3 disc or install.
pub const PRE_RACE: PreRace = PreRace {
    on: Platforms::Only(&[Platform::Ps3]),
    // `Track_Construct_q` (`0x000b2af0`) formats `%s\%sstart_grid%s.vex`, with `_reversed` for the
    // suffix on a circuit run backwards, and takes the plain pair for every mode but the rows
    // below; a file that is not on the disc falls back to the forward `%s\start_grid.vex`
    // (`docs/ghidra/functions/ps3-hdfury-eu/start-grid-selection.md`).
    grid: "start_grid.vex",
    grid_reversed: Some("start_grid_reversed.vex"),
    by_mode: &[ModeGrid {
        // Game modes 5 and 10 (Time Trial, Speed Lap) and 6 (Zone): `_ta`, the fly-by that frames
        // the one craft's slot. The executable's other `_ta` modes (7, 13, 14, 21) are not modes
        // of ours. Mode 2 (`Demo`) takes `start_grid_de`; this project has no demo.
        modes: &["time_trial", "speed_lap", "zone"],
        grid: "start_grid_ta.vex",
        grid_reversed: Some("start_grid_ta_reversed.vex"),
    }],
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
    voice_over: None,
    effects_fade_in: None,
};

#[cfg(test)]
mod tests {
    use super::PRE_RACE;

    #[test]
    fn a_solo_mode_tries_ta_then_the_plain_file() {
        for mode in ["time_trial", "speed_lap", "zone"] {
            assert_eq!(
                PRE_RACE.grid_files(mode, false),
                ["start_grid_ta.vex", "start_grid.vex"],
                "{mode}"
            );
            assert_eq!(
                PRE_RACE.grid_files(mode, true),
                ["start_grid_ta_reversed.vex", "start_grid.vex"],
                "{mode}"
            );
        }
    }

    #[test]
    fn every_other_mode_tries_the_plain_pair() {
        for mode in ["single_race", "eliminator", "head_to_head", "tournament"] {
            assert_eq!(PRE_RACE.grid_files(mode, false), ["start_grid.vex"]);
            assert_eq!(
                PRE_RACE.grid_files(mode, true),
                ["start_grid_reversed.vex", "start_grid.vex"]
            );
        }
    }
}
