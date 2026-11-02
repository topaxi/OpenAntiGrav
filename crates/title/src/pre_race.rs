//! The pre-race flyby a title plays before its countdown: which file holds the camera, how
//! long it holds still, when it may be skipped, and where each number came from.
//!
//! The camera is a `gridCamera` animation in a `start_grid.vex` beside the circuit's
//! `track.vex` (see `oag_vex::grid_camera`). What differs per title is data, so it is here and
//! not a comparison on a title's name (ADR-0058).

use crate::effects::{Origin, Platforms};

/// A value and where it came from, so a number nobody measured says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    /// The named title's law, not re-measured on this one.
    pub const fn inherited(value: T, from: &'static str) -> Self {
        Self {
            value,
            origin: Origin::InheritedFrom(from),
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
    /// The circuit's voice-over, played through the music player from the tick the lock counter
    /// reaches zero until the flyby ends: an archive entry name with `{}` where the circuit's
    /// number goes (the leading digits of its `PI_Track` id, as `atoi` reads them, so `03_Track`
    /// is `3`). `None`: the title plays none.
    pub voice_over: Option<Sourced<&'static str>>,
    /// The sound effects held silent through the flyby and faded back in part-way through it.
    /// `None`: the title's effects play from the first tick.
    pub effects_fade_in: Option<Sourced<EffectsFade>>,
}

/// The flyby's sound-effects fade: silent from the first tick, then a linear ramp from silence
/// that starts when the lock counter reaches a value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectsFade {
    /// The lock counter value (it counts down to zero over `lock_ticks`) at which the ramp starts.
    pub at_counter: u32,
    /// The gain added on each tick after it, until it reaches one.
    pub per_tick: f32,
}

impl PreRace {
    /// The flyby tick the effects fade-in starts on, with its step per tick, or `None` for a title
    /// with no such fade. The counter reads `lock_ticks - tick` after tick `tick`'s decrement.
    #[must_use]
    pub fn effects_fade_start(&self) -> Option<(u32, f32)> {
        let fade = self.effects_fade_in?.value;
        Some((
            self.lock_ticks.value.saturating_sub(fade.at_counter),
            fade.per_tick,
        ))
    }

    /// The entry holding the voice-over for the circuit whose `PI_Track` id is `track_id`, or
    /// `None` for a title with none or an id that starts with no digit.
    #[must_use]
    pub fn voice_over_entry(&self, track_id: &str) -> Option<String> {
        let template = self.voice_over?.value;
        let digits: String = track_id.chars().take_while(char::is_ascii_digit).collect();
        let number: u64 = digits.parse().ok()?;
        Some(template.replace("{}", &number.to_string()))
    }
}
