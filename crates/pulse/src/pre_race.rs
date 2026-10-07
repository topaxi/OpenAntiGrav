//! Pulse's pre-race flyby, measured on the PSP: `docs/gameplay/race-intro.md`.
//!
//! The PS2 disc ships the same `start_grid.vex` files and executable strings but its flyby
//! has not been watched, so it is left out of [`PRE_RACE`]'s platforms.

use oag_title::pre_race::{Ending, PreRace, Skip, Sourced};
use oag_title::{Platform, Platforms};

/// Pulse's flyby on a PSP disc.
pub const PRE_RACE: PreRace = PreRace {
    on: Platforms::Only(&[Platform::Psp]),
    grid: "start_grid.vex",
    // The directory holds one file; a reversed circuit flies the forward file's path
    // (Metropia matched `02_Track`'s to 0.02 units over its first 100 frames).
    grid_reversed: None,
    // The original's render view, both circuits captured; not derivable from the file.
    fov_degrees: Sourced::measured(54.308_994),
    // The camera node waits `1.0` s of its own clock; the animation clock first read non-zero at
    // tick 29 on three runs.
    hold_ticks: Sourced::measured(28),
    // `mode+0x1a04` counts down from 60 and must read zero.
    lock_ticks: Sourced::measured(60),
    // `RaceMode_UpdateIntro` substate 2 waits 0.5 s before the HUD flags are set.
    hud_delay_ticks: Sourced::measured(30),
    ending: Sourced::measured(Ending::AtAnimationEnd),
    skip: Sourced::measured(Skip::Held),
    panel: true,
};
