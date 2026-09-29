//! The start-of-race voice: `ready` and `go`, on the ticks Pulse plays them.
//!
//! # What the original does
//!
//! Two cues, by name, both through the dry `Sound_PlayNamedInSlot`, and nothing
//! else started between them - no beeps, no per-digit cue. What `ready`'s six
//! waveforms in `speech.bnk` (3.73 s) say is not identified.
//!
//! - **`ready`** is played by `RaceMode_SetState(mode, 1)`, called when the
//!   intro substate machine (`RaceMode_UpdateIntro_q`) finishes.
//! - **`go`** is played by `RaceMode_UpdateCountdown` in the call that runs
//!   `Race_StartRacing` and `RaceMode_SetState(mode, 2)`, when the countdown
//!   state's timer reaches zero. That state lasts **180 ticks**: a
//!   `RaceMode_SetState` breakpoint saw state 1 and state 2 exactly 180.0
//!   frames apart, and the two cue starts are 180.0 frames apart in all three
//!   captures. The number is a constant, not circuit data: `RaceManager_Construct`,
//!   which every mode's constructor (Zone's `Zone_Create` too) calls, writes
//!   `manager+0x7bc = 4.0`, and the state ends when the whole seconds left reach
//!   zero, which is after 3.0 s.
//!
//! # Where the ticks come from
//!
//! Measured on Pulse (PSP) by breaking on `Scream_StartSound` and logging the
//! PSP cycle counter, anchored on `Ship_UpdateCraft` entries: the first entry at
//! which the craft's throttle reads non-zero is [`oag_race::COUNTDOWN_TICKS`]
//! on the docs' axis, and the `go` cue lands **between the entries numbered 270
//! and 271, that is 2.0 entries before it** (Time Trial 270.02 and 270.02, a
//! single race 270.06, Eliminator 270.79 against a coarser anchor). The
//! throttle is written by the craft update in between, which is the first one
//! the countdown state no longer gates - so the cue is voiced at the end of the
//! **last gated tick**: the craft entry numbered 271 still reads a zero
//! throttle after `go` has been voiced, and the state change that lifts the gate
//! is the one that plays it.
//!
//! So here `go` is raised in the step whose `World::tick` is
//! `COUNTDOWN_TICKS - 1`, the last one [`oag_race::RaceState::thrust_gated`]
//! holds, and `ready` [`READY_TO_GO_TICKS`] steps before it.
//!
//! # Scope
//!
//! Time Trial, a single race and Eliminator were captured (Eliminator reads
//! `elim_vo`, the others `SPEECH`); Zone shares the same two functions - it
//! reaches `RaceMode_UpdateCountdown` through its own state-1 handler - and
//! reads `zone_vo`, but was **not** captured live: its ticks rest on the shared code
//! and that constant, not on a measurement. Only the PSP pressing was
//! measured: the PS2 pressing is the same title data and is lent these ticks.
//! Pure, HD and 2048 carry `None` for [`oag_title::RaceDefaults::countdown_voice`]
//! and play nothing. See
//! `docs/ghidra/functions/psp-pulse-usa/countdown-voice.md`.

use oag_race::COUNTDOWN_TICKS;

use super::Race;
use crate::audio::sfx::{Cue, CueEvent};

/// Ticks from `ready` to `go`: the countdown state's own length.
///
/// **Measured**: 180.0 frames between the two `Scream_StartSound` hits, in every
/// capture, and between `RaceMode_SetState`'s two hits.
pub const READY_TO_GO_TICKS: u64 = 180;

/// The `World::tick` `go` is raised on: the last tick the thrust gate holds.
pub const GO_TICK: u64 = COUNTDOWN_TICKS - 1;

/// The `World::tick` `ready` is raised on.
pub const READY_TICK: u64 = GO_TICK - READY_TO_GO_TICKS;

impl Race {
    /// Raises `ready` and `go` on their ticks, on a title that voices its start.
    ///
    /// Called from [`Race::tick`] before the step, so `World::tick` is the tick
    /// being stepped. A cue is a per-tick output and no world state: nothing
    /// here can move a state hash.
    pub(super) fn tick_countdown_voice(&mut self) {
        if !self.sim.countdown_voice {
            return;
        }
        let cue = match self.sim.world.tick {
            READY_TICK => Cue::Ready,
            GO_TICK => Cue::Go,
            _ => return,
        };
        let player = self.sim.world.primary_slot();
        self.sim.cues.push(CueEvent::new(cue, player));
    }
}
