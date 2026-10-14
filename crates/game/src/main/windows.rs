//! Multi-window local play: `--windows N`.
//!
//! One process, one `RaceSim`, one tick, one device - and up to eight OS windows,
//! view `i` filling window `i`. The design is `docs/rendering/multi-window.md`.
//!
//! Window 1 is [`Gpu::output`](crate::gpu::Gpu) and the front end's, exactly as in a run
//! with one window; every further window is a [`Seat`]: an [`Output`] on the same device
//! and a [`Framebuffer`] of its own, drawing the view of one grid slot. This module is the
//! parts that need no GPU - which window an event is for, when "no window has focus" is
//! true, and what a seat window is called - and the seat itself.
//!
//! **Chosen, not measured**, all of it: the original has no multi-window mode.

use std::time::Duration;

use web_time::Instant;

use oag_present::perf;
use oag_present::upscale::Framebuffer;
use winit::window::WindowId;

use crate::gpu::Output;

/// A further window of a multi-window race, and what it draws with.
pub(crate) struct Seat {
    pub(crate) output: Output,
    /// This window's own scene target, presentation target and post chain, at its own
    /// size. The scene target's *allocation* is window 1's (the scene's depth and
    /// multisample attachments are sized to one allocation and a pass wants equal
    /// sizes), its *extent* is this window's own.
    pub(crate) framebuffer: Framebuffer,
    /// The grid slot this window shows. Window `k` (counting window 1 as 0) shows slot
    /// `k`, so the slot is stable when an earlier window closes.
    pub(crate) slot: usize,
    /// How long this window's swapchain acquire blocked, and its present took, per frame.
    /// Logged every [`PACING_LOG_FRAMES`] frames: the pacing measurement.
    pub(crate) acquire: perf::Meter,
    pub(crate) present: perf::Meter,
}

/// How often a multi-window run says what each window's acquire and present cost.
pub(crate) const PACING_LOG_FRAMES: u64 = 600;

/// Which window an event is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    /// Window 1: the front end's, and the one whose close ends the run.
    Primary,
    /// The seat at this index of the open seats.
    Seat(usize),
}

/// Routes `id` to a window: window 1, a seat, or nobody (a window already closed).
pub(crate) fn which(id: WindowId, primary: WindowId, seats: &[WindowId]) -> Option<Which> {
    if id == primary {
        Some(Which::Primary)
    } else {
        seats.iter().position(|seat| *seat == id).map(Which::Seat)
    }
}

/// What a window closing means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Closed {
    /// Window 1: the run ends, as it always did.
    Quit,
    /// A seat window: that seat is flown by the AI from now on and the window goes.
    Seat(usize),
    /// A window this run already let go of (winit reports `Destroyed` after the close).
    Nothing,
}

/// The decision a close request or a destroyed window comes to. Both mean the same
/// thing: under X11 with no window manager a window is destroyed outright, and winit
/// reports `Destroyed` rather than `CloseRequested`.
pub(crate) fn closed(id: WindowId, primary: WindowId, seats: &[WindowId]) -> Closed {
    match which(id, primary, seats) {
        Some(Which::Primary) => Closed::Quit,
        Some(Which::Seat(index)) => Closed::Seat(index),
        None => Closed::Nothing,
    }
}

/// How long no window may have focus before the race counts as being away.
///
/// Moving between two windows of the same run reports `Focused(false)` for the one and
/// `Focused(true)` for the other, and the two need not arrive in the same pass of the
/// event loop. A quarter of a second is long past that and short of a player noticing a
/// race that did not pause. **Chosen, not measured.**
pub(crate) const FOCUS_GRACE: Duration = Duration::from_millis(250);

/// Which of the run's windows have keyboard focus, and when none has.
///
/// With one window a lost focus is the window being away. With several it is only away
/// when it is lost by all of them: clicking from window 1 into window 2 must not pause
/// the race for everyone.
#[derive(Debug, Default)]
pub(crate) struct Focus {
    focused: Vec<WindowId>,
    /// When the last focused window lost it, until a window gains it again or the loss is
    /// reported.
    lost_at: Option<Instant>,
}

impl Focus {
    /// Records `id` gaining or losing focus at `now`.
    pub(crate) fn changed(&mut self, id: WindowId, gained: bool, now: Instant) {
        if gained {
            if !self.focused.contains(&id) {
                self.focused.push(id);
            }
            self.lost_at = None;
        } else {
            self.focused.retain(|held| *held != id);
            if self.focused.is_empty() && self.lost_at.is_none() {
                self.lost_at = Some(now);
            }
        }
    }

    /// Whether the focus has been with no window for [`FOCUS_GRACE`], once: reported
    /// once per loss, and not again until a window has gained focus and lost it.
    pub(crate) fn away(&mut self, now: Instant) -> bool {
        match self.lost_at {
            Some(at) if now.duration_since(at) >= FOCUS_GRACE => {
                self.lost_at = None;
                true
            }
            _ => false,
        }
    }

    /// Forgets a window that is gone: its focus can no longer be lost.
    pub(crate) fn forget(&mut self, id: WindowId) {
        self.focused.retain(|held| *held != id);
    }
}

/// Whether every slot `1..people` has a window among `open`, the slots of the open seats:
/// when a race of `people` is shown as windows rather than cut up inside window 1.
pub(crate) fn covers(open: &[usize], people: usize) -> bool {
    people > 1 && (1..people).all(|slot| open.contains(&slot))
}

/// The title of the window of seat `player` (1-based): the app's title and the join
/// screen's own word for a person.
#[cfg_attr(
    any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
    allow(dead_code)
)]
pub(crate) fn title(app: &str, player_word: &str, player: usize) -> String {
    format!("{app} - {player_word} {player}")
}

/// The render target a window of `size` wants, at the profile's render scale: the
/// largest of these over the open windows is the one allocation they share.
pub(crate) fn shared_allocation(
    own: (u32, u32),
    seats: impl IntoIterator<Item = (u32, u32)>,
) -> (u32, u32) {
    seats
        .into_iter()
        .fold(own, |all, wanted| (all.0.max(wanted.0), all.1.max(wanted.1)))
}

#[cfg(test)]
#[path = "windows/tests.rs"]
mod tests;
