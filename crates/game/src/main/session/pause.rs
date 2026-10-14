//! Who may resume a paused race, and the countdown that follows.
//!
//! **Chosen, not measured** throughout: the original is single-player and has no
//! pause ownership, no timeout and no resume countdown. The rules, all ours:
//!
//! - **Only the player who paused resumes** (maintainer's rule, 2026-10-13): anyone
//!   seated can pause, and the pause belongs to them until they press Start again
//!   or Circle to leave the race. A pause nobody pressed for (the window losing
//!   focus, `Session::window_away`) has no owner, and any player may lift it. With
//!   devices merged - every race that is not a split - every press is slot 0's, so a
//!   one-player race behaves as it always did.
//! - **The owner's claim expires** [`OWNER_TIMEOUT`] seconds after the pause itself
//!   (maintainer, 2026-10-14), and at once when the owner's device is gone (the seat
//!   has turned over to the AI). The pause then belongs to nobody.
//! - **A resume in a race with more than one person counts down** `3`, `2`, `1`,
//!   `GO` for [`RESUME_SECONDS`] before the race moves, so nobody is caught out
//!   (maintainer, 2026-10-14). The race stays frozen through it. Start during the
//!   countdown pauses again, and whoever pressed owns the new pause.
//!
//! [`step`] is the whole rule as a pure function of the state, this tick's Start
//! presses, "now" and which seats still have a person at them. The clock is the
//! session's wall clock, not simulation ticks: a frozen race has none, and nothing
//! here is hashed.

/// How long a player's claim on their own pause lasts, in seconds. **Chosen.**
pub(crate) const OWNER_TIMEOUT: f64 = 30.0;

/// How long the resume countdown holds the race, in seconds: `3`, `2`, `1`, then
/// `GO` as the race moves. **Chosen**, though it is the 180 ticks the original
/// leaves between `ready` and `go` (`oag_raceplay::countdown::READY_TO_GO_TICKS`),
/// so the start voice's own cue spacing fits it.
pub(crate) const RESUME_SECONDS: f64 = 3.0;

/// Where `GO` is on the countdown art's own timeline, in seconds. Derived from the
/// texel grid of `Cockpit_321GO.vex` (`docs/rendering/start-gantry.md`: `GO` lit
/// from ~3.6 s, the first digit from 0.93 s).
const ART_GO_SECONDS: f32 = 3.6;

/// Where the countdown art is on its own clock when a resume starts, so that its
/// `GO` lands as [`RESUME_SECONDS`] runs out. **Chosen.**
pub(crate) const ART_CLOCK_START: f32 = ART_GO_SECONDS - RESUME_SECONDS as f32;

/// How long `GO` stays on screen after the race moves again, in seconds. **Chosen.**
pub(crate) const GO_LINGER: f64 = 1.5;

/// The pause the session is in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum State {
    /// The race ticks.
    Running,
    /// The race is frozen.
    Paused {
        /// The slot that alone may resume or leave; `None` for a pause nobody
        /// pressed for, or one whose owner has timed out or gone.
        owner: Option<usize>,
        /// The slot whose Start paused, kept after `owner` lapses so the line can
        /// still name them; `None` for a pause nobody pressed for.
        by: Option<usize>,
        /// The session clock, in seconds, when the pause began.
        since: f64,
    },
    /// A resume was asked for and the countdown is running. The race is still
    /// frozen.
    Resuming {
        /// The session clock when the countdown began.
        since: f64,
    },
}

impl State {
    /// A pause nobody pressed for, started at `now`.
    pub(crate) fn unowned(now: f64) -> Self {
        State::Paused {
            owner: None,
            by: None,
            since: now,
        }
    }

    /// Whether the simulation steps. **The one gate**: nothing reaches `Race::tick`
    /// in any other state.
    pub(crate) fn ticks(self) -> bool {
        self == State::Running
    }

    /// Whether the race is held, paused or counting down.
    pub(crate) fn frozen(self) -> bool {
        !self.ticks()
    }

    fn pause_by(slot: usize, now: f64) -> Self {
        State::Paused {
            owner: Some(slot),
            by: Some(slot),
            since: now,
        }
    }
}

/// What [`step`] needs to know about the table besides the clock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Table {
    /// Bit `n` set while slot `n` has a device. Slots past the people at the table
    /// never pause, so their bits do not matter.
    pub present: u8,
    /// Whether a resume counts down: a race with more than one person, and not a
    /// start still being counted down by the race itself.
    pub countdown: bool,
}

/// One tick of the pause rule. `presses` is [`oag_input::Controls::take_presses`]'s
/// Start bitmask and `now` the session clock in seconds.
pub(crate) fn step(state: State, presses: u8, now: f64, table: Table) -> State {
    let pressed_by = (presses != 0).then(|| presses.trailing_zeros() as usize);
    match (state, pressed_by) {
        (State::Running, None) => State::Running,
        (State::Running | State::Resuming { .. }, Some(slot)) => State::pause_by(slot, now),
        (State::Resuming { since }, None) if now - since >= RESUME_SECONDS => State::Running,
        (State::Resuming { .. }, None) => state,
        (State::Paused { owner, by, since }, _) => {
            let owner = owner.filter(|&slot| claim_holds(slot, since, now, table.present));
            if pressed_by.is_some() && may_act(owner, presses) {
                if table.countdown {
                    State::Resuming { since: now }
                } else {
                    State::Running
                }
            } else {
                State::Paused { owner, by, since }
            }
        }
    }
}

/// Whether the countdown has just run out: [`step`] took `before` to
/// [`State::Running`] by waiting, rather than by a resume with no countdown.
pub(crate) fn go(before: State, after: State) -> bool {
    matches!(before, State::Resuming { .. }) && after == State::Running
}

/// Whether a Circle press leaves the paused race: the same owner rule as
/// resuming, so another player cannot quit someone else's pause either. Never
/// during the countdown.
pub(crate) fn leaves(state: State, presses: u8) -> bool {
    matches!(state, State::Paused { owner, .. } if presses != 0 && may_act(owner, presses))
}

/// Whether slot `slot` still holds a pause that began at `since`: inside the
/// timeout, and with a device at the seat.
fn claim_holds(slot: usize, since: f64, now: f64, present: u8) -> bool {
    now - since < OWNER_TIMEOUT && present & (1 << slot) != 0
}

fn may_act(owner: Option<usize>, presses: u8) -> bool {
    match owner {
        Some(slot) => presses & (1 << slot) != 0,
        None => presses != 0,
    }
}

/// The bitmask [`Table::present`] wants, from the device assignment: every slot
/// when devices are merged (a one-person race has nobody to lose), otherwise the
/// seated slots that still have a device - the predicate
/// `split_screen::follow_devices` turns a seat to the AI on.
pub(crate) fn present_seats<D: Copy + PartialEq>(
    assignment: &oag_input::pad::Assignment<D>,
) -> u8 {
    let Some(humans) = assignment.humans() else {
        return u8::MAX;
    };
    (0..humans.min(8))
        .filter(|&slot| assignment.drives(slot))
        .fold(0, |mask, slot| mask | 1 << slot)
}

/// Whether the table has more than one person at it, which is when a resume counts
/// down. A single-player resume stays as it was.
pub(crate) fn several_people<D: Copy + PartialEq>(
    assignment: &oag_input::pad::Assignment<D>,
) -> bool {
    assignment.humans().is_some_and(|humans| humans > 1)
}

/// The words the line under the pause says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Line {
    /// A player's pause, with whole seconds until it is anyone's.
    Owned { player: usize, seconds_left: u32 },
    /// A pause anyone may lift. `player` names who paused, when someone did.
    Open { player: Option<usize> },
    /// The countdown is running.
    Resuming,
}

/// What to say under `state` at `now`, or `None` while the race runs.
pub(crate) fn line(state: State, now: f64, table: Table) -> Option<Line> {
    match state {
        State::Running => None,
        State::Resuming { .. } => Some(Line::Resuming),
        State::Paused { owner, by, since } => {
            match owner.filter(|&slot| claim_holds(slot, since, now, table.present)) {
                Some(player) => Some(Line::Owned {
                    player,
                    seconds_left: (OWNER_TIMEOUT - (now - since)).ceil().max(0.0) as u32,
                }),
                None => Some(Line::Open { player: by }),
            }
        }
    }
}

/// `m:ss`, as the line shows the time left.
pub(crate) fn clock(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The countdown art's own clock in seconds, or `None` when it is not on screen:
/// through the countdown, and for [`GO_LINGER`] after `go_at`, the session clock
/// when the race moved again.
pub(crate) fn art_seconds(state: State, go_at: Option<f64>, now: f64) -> Option<f32> {
    match state {
        State::Resuming { since } => Some((now - since) as f32 + ART_CLOCK_START),
        State::Running => go_at
            .filter(|&at| now - at < GO_LINGER)
            .map(|at| (now - at) as f32 + ART_GO_SECONDS),
        State::Paused { .. } => None,
    }
}

impl super::Session {
    /// Whether the race is held for any reason, paused or counting down.
    pub(crate) fn paused(&self) -> bool {
        self.pause.frozen()
    }

    /// One tick of the pause rule against the live session, with this tick's Start
    /// `presses`, raising the start voice's cues on a title that has them.
    ///
    /// A resume while the race is still counting down its own start (or flying
    /// the pre-race camera) happens at once: that countdown is on screen, and a
    /// second one would draw over it.
    pub(crate) fn step_pause(&mut self, presses: u8) {
        let Stage::Race(stage) = &self.stage else {
            return;
        };
        let start_counted = oag_race::RaceState::thrust_gated(stage.race.sim.world.tick)
            || (!self.no_intro && stage.race.intro_to_play());
        let assignment = self.controls.assignment();
        let table = Table {
            present: present_seats(assignment),
            countdown: several_people(assignment) && !start_counted,
        };
        let before = self.pause;
        let after = step(before, presses, self.wall, table);
        if after == before {
            return;
        }
        self.pause = after;
        info!("pause: {before:?} -> {after:?}");
        let cue = match (before, after) {
            (State::Paused { .. }, State::Resuming { .. }) => Some(Cue::Ready),
            _ if go(before, after) => {
                self.go_at = Some(self.wall);
                Some(Cue::Go)
            }
            _ => None,
        };
        if let (Some(cue), Stage::Race(stage)) = (cue, &mut self.stage) {
            stage.race.raise_start_cue(cue);
        }
    }
}

use log::info;
use oag_sound::sfx::Cue;

use crate::stage::Stage;

#[cfg(test)]
#[path = "pause/tests.rs"]
mod tests;
