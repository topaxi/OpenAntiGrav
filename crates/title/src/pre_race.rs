//! The pre-race flyby a title plays before its countdown: which file holds the camera, how
//! long it holds still, when it may be skipped, and where each number came from.
//!
//! The camera is a `gridCamera` animation in a `start_grid.vex` beside the circuit's
//! `track.vex` (see `oag_vex::grid_camera`). What differs per title is data, so it is here and
//! not a comparison on a title's name (ADR-0058).

use crate::effects::{Origin, Platforms};

/// A value and where it came from, so a number nobody measured says so.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sourced<T> {
    /// The value.
    pub value: T,
    /// Measured, inherited or chosen.
    pub origin: Origin,
}

impl<T> Sourced<T> {
    /// A value read off the original running or off its executable.
    pub const fn measured(value: T) -> Self {
        Self {
            value,
            origin: Origin::Measured,
        }
    }

    /// A value the project picked. Carries no confidence score.
    pub const fn chosen(value: T) -> Self {
        Self {
            value,
            origin: Origin::Chosen,
        }
    }
}

/// What ends the flyby besides a skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// The animation reaches its `AnimEnd`: the race then starts.
    AtAnimationEnd,
    /// The animation repeats and only a skip ends it, as a title that waits on a prompt does.
    OnlyBySkip,
}

/// How the player skips it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    /// A held confirm button, as Pulse's `Input_IsHeld` test reads it.
    Held,
    /// A press of the confirm button: the title waits at a prompt.
    Press,
}

/// One title's pre-race flyby.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreRace {
    /// Where the flyby plays: the platforms its numbers were taken on.
    pub on: Platforms,
    /// The file beside the circuit's `track.vex` that holds the camera.
    pub grid: &'static str,
    /// The file for a circuit run the other way round, where the title ships one. `None`: the
    /// forward file serves both.
    pub grid_reversed: Option<&'static str>,
    /// Vertical field of view, degrees. The `gridCamera` payload does not carry it.
    pub fov_degrees: Sourced<f32>,
    /// Ticks before the animation starts moving.
    pub hold_ticks: Sourced<u32>,
    /// Ticks before a skip or the end is honoured.
    pub lock_ticks: Sourced<u32>,
    /// Ticks the HUD stays hidden after the flyby ends.
    pub hud_delay_ticks: Sourced<u32>,
    /// What ends it besides a skip.
    pub ending: Sourced<Ending>,
    /// How it is skipped.
    pub skip: Sourced<Skip>,
    /// Whether the track-description panel (Pulse's `InGameTrackDescriptionScreen`) is drawn.
    pub panel: bool,
}
