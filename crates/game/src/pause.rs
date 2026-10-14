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
pub const OWNER_TIMEOUT: f64 = 30.0;

/// How long the resume countdown holds the race, in seconds: `3`, `2`, `1`, then
/// `GO` as the race moves. **Chosen**, though it is the 180 ticks the original
/// leaves between `ready` and `go` (`oag_raceplay::countdown::READY_TO_GO_TICKS`),
/// so the start voice's own cue spacing fits it.
pub const RESUME_SECONDS: f64 = 3.0;

/// Where `GO` is on the countdown art's own timeline, in seconds. Derived from the
/// texel grid of `Cockpit_321GO.vex` (`docs/rendering/start-gantry.md`: `GO` lit
/// from ~3.6 s, the first digit from 0.93 s).
const ART_GO_SECONDS: f32 = 3.6;

/// Where the countdown art is on its own clock when a resume starts, so that its
/// `GO` lands as [`RESUME_SECONDS`] runs out. **Chosen.**
pub const ART_CLOCK_START: f32 = ART_GO_SECONDS - RESUME_SECONDS as f32;

/// How long `GO` stays on screen after the race moves again, in seconds. **Chosen.**
pub const GO_LINGER: f64 = 1.5;

/// How far down a view the countdown sits when the view is one of a split, as a
/// fraction of the view's height. **Chosen**: the authored place (see
/// [`anchor_y`]) is over the craft in a short view, so a split lifts it to where a
/// view shows track and gantry.
pub const SPLIT_ANCHOR_Y: f32 = 0.30;

/// Where the title's countdown art sits on a screen of one view, as a fraction of
/// its height: `Cockpit_321GO`'s authored `(0, 0)` plus `OriginY 35` on the PSP's
/// 480x272 grid, 171 down (`oag_game::hud_countdown`). The text a title with no art
/// draws takes the same place.
pub const AUTHORED_ANCHOR_Y: f32 = 171.0 / 272.0;

/// How far down a view the countdown sits, as a fraction of its height: the
/// authored place for a whole screen, [`SPLIT_ANCHOR_Y`] for a view of a split.
pub fn anchor_y(views: usize) -> f32 {
    if views > 1 {
        SPLIT_ANCHOR_Y
    } else {
        AUTHORED_ANCHOR_Y
    }
}

/// What the countdown says at one moment. **Chosen**: one second each, `GO` from
/// the moment the race moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Count {
    Three,
    Two,
    One,
    Go,
}

impl Count {
    /// What is said at `art_seconds` on the art's clock ([`art_seconds`]).
    pub fn at(art_seconds: f32) -> Self {
        let elapsed = art_seconds - ART_CLOCK_START;
        if elapsed < 1.0 {
            Count::Three
        } else if elapsed < 2.0 {
            Count::Two
        } else if elapsed < 3.0 {
            Count::One
        } else {
            Count::Go
        }
    }

    /// The word, with `go` the language's `GO` ([`strings::GO`]).
    pub fn text(self, go: &str) -> &str {
        match self {
            Count::Three => "3",
            Count::Two => "2",
            Count::One => "1",
            Count::Go => go,
        }
    }
}

/// The pause the session is in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
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
    pub fn unowned(now: f64) -> Self {
        State::Paused {
            owner: None,
            by: None,
            since: now,
        }
    }

    /// Whether the simulation steps. **The one gate**: nothing reaches `Race::tick`
    /// in any other state.
    pub fn ticks(self) -> bool {
        self == State::Running
    }

    /// Whether the race is held, paused or counting down.
    pub fn frozen(self) -> bool {
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
pub struct Table {
    /// Bit `n` set while slot `n` has a device. Slots past the people at the table
    /// never pause, so their bits do not matter.
    pub present: u8,
    /// Whether a resume counts down: a race with more than one person, and not a
    /// start still being counted down by the race itself.
    pub countdown: bool,
}

/// One tick of the pause rule. `presses` is [`oag_input::Controls::take_presses`]'s
/// Start bitmask and `now` the session clock in seconds.
pub fn step(state: State, presses: u8, now: f64, table: Table) -> State {
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
pub fn go(before: State, after: State) -> bool {
    matches!(before, State::Resuming { .. }) && after == State::Running
}

/// Whether a Circle press leaves the paused race: the same owner rule as
/// resuming, so another player cannot quit someone else's pause either. Never
/// during the countdown.
pub fn leaves(state: State, presses: u8) -> bool {
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
pub fn present_seats<D: Copy + PartialEq>(assignment: &oag_input::pad::Assignment<D>) -> u8 {
    let Some(humans) = assignment.humans() else {
        return u8::MAX;
    };
    (0..humans.min(8))
        .filter(|&slot| assignment.drives(slot))
        .fold(0, |mask, slot| mask | 1 << slot)
}

/// Whether the table has more than one person at it, which is when a resume counts
/// down. A single-player resume stays as it was.
pub fn several_people<D: Copy + PartialEq>(assignment: &oag_input::pad::Assignment<D>) -> bool {
    assignment.humans().is_some_and(|humans| humans > 1)
}

/// The words the line under the pause says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    /// A player's pause, with whole seconds until it is anyone's.
    Owned { player: usize, seconds_left: u32 },
    /// A pause anyone may lift. `player` names who paused, when someone did.
    Open { player: Option<usize> },
    /// The countdown is running.
    Resuming,
}

/// What to say under `state` at `now`, or `None` while the race runs.
pub fn line(state: State, now: f64, table: Table) -> Option<Line> {
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
pub fn clock(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The countdown art's own clock in seconds, or `None` when it is not on screen:
/// through the countdown, and for [`GO_LINGER`] after `go_at`, the session clock
/// when the race moved again.
pub fn art_seconds(state: State, go_at: Option<f64>, now: f64) -> Option<f32> {
    match state {
        State::Resuming { since } => Some((now - since) as f32 + ART_CLOCK_START),
        State::Running => go_at
            .filter(|&at| now - at < GO_LINGER)
            .map(|at| (now - at) as f32 + ART_GO_SECONDS),
        State::Paused { .. } => None,
    }
}

/// The ids of the strings the line is made of, in `assets/ui/strings/english.toml`.
pub mod strings {
    /// `%s` is the player's number, `%s` again the time left as `m:ss`.
    pub const OWNED: &str = "OAG_PAUSE_OWNED";
    /// `%s` is the player's number.
    pub const OPEN: &str = "OAG_PAUSE_OPEN";
    /// A pause nobody pressed for: the window was away.
    pub const OPEN_UNOWNED: &str = "OAG_PAUSE_OPEN_UNOWNED";
    /// The countdown's last word, for a title that draws it as text.
    pub const GO: &str = "OAG_COUNTDOWN_GO";
}

#[cfg(test)]
mod tests;
