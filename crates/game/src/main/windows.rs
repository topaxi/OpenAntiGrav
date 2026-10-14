//! Multi-window local play: `--windows N`.
//!
//! One process, one `RaceSim`, one tick, one device - and up to eight OS windows,
//! view `i` filling window `i`. The design is `docs/rendering/multi-window.md`.
//!
//! Window 1 is [`Gpu::output`](crate::gpu::Gpu) and the front end's, exactly as in a run
//! with one window; every further window is a [`Screen`]: an [`Output`] on the same device
//! and a [`Framebuffer`] of its own. **A window holds a list of seats**: which grid slots it
//! shows, and how it cuts them, are `oag_raceplay::Race::windows` (window `i` of the race is
//! window `i` of the run), and inside a window the seats are cut exactly as a split screen
//! cuts its views. This module is the parts that need no GPU - which window an event is for,
//! when "no window has focus" is true, and what a further window is called - and the screen
//! itself.
//!
//! **Chosen, not measured**, all of it: the original has no multi-window mode.

use std::time::Duration;

use web_time::Instant;

use oag_present::perf;
use oag_present::upscale::Framebuffer;
use winit::window::WindowId;

use crate::gpu::Output;

/// A further window of a multi-window race, and what it draws with.
pub(crate) struct Screen {
    pub(crate) output: Output,
    /// This window's own scene target, presentation target and post chain, at its own
    /// size. The scene target's *allocation* is window 1's (the scene's depth and
    /// multisample attachments are sized to one allocation and a pass wants equal
    /// sizes), its *extent* is this window's own.
    pub(crate) framebuffer: Framebuffer,
    /// How long this window's swapchain acquire blocked, and its present took, per frame.
    /// Logged every [`PACING_LOG_FRAMES`] frames: the pacing measurement.
    pub(crate) acquire: perf::Meter,
    pub(crate) present: perf::Meter,
    /// This window's own GPU timing rings: dynamic resolution is fed the cost summed over the
    /// windows of a frame (`Session::feed_windows`).
    pub(crate) rings: crate::session::timing::Rings,
}

/// How often a multi-window run says what each window's acquire and present cost.
pub(crate) const PACING_LOG_FRAMES: u64 = 600;

/// Which window an event is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    /// Window 1: the front end's, and the one whose close ends the run.
    Primary,
    /// The further window at this index of the open ones.
    Screen(usize),
}

/// Routes `id` to a window: window 1, a further window, or nobody (a window already closed).
pub(crate) fn which(id: WindowId, primary: WindowId, screens: &[WindowId]) -> Option<Which> {
    if id == primary {
        Some(Which::Primary)
    } else {
        screens.iter().position(|screen| *screen == id).map(Which::Screen)
    }
}

/// What a window closing means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Closed {
    /// Window 1: the run ends, as it always did.
    Quit,
    /// A further window: every seat it held is flown by the AI from now on and the window
    /// goes.
    Screen(usize),
    /// A window this run already let go of (winit reports `Destroyed` after the close).
    Nothing,
}

/// The decision a close request or a destroyed window comes to. Both mean the same
/// thing: under X11 with no window manager a window is destroyed outright, and winit
/// reports `Destroyed` rather than `CloseRequested`.
pub(crate) fn closed(id: WindowId, primary: WindowId, screens: &[WindowId]) -> Closed {
    match which(id, primary, screens) {
        Some(Which::Primary) => Closed::Quit,
        Some(Which::Screen(index)) => Closed::Screen(index),
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

    /// The window that most recently gained focus and still has it, or `None` while no
    /// window has it. With two reported focused for a pass of the loop (the gain arriving
    /// before the loss) the later gain is the one a raw key event (which names no window)
    /// is read as being for.
    pub(crate) fn current(&self) -> Option<WindowId> {
        self.focused.last().copied()
    }

    /// Forgets a window that is gone: its focus can no longer be lost.
    pub(crate) fn forget(&mut self, id: WindowId) {
        self.focused.retain(|held| *held != id);
    }
}

/// The first seat of each window of a multi-window race, window 1 first: the seats a person is
/// presumed to sit at before any key is pressed (`oag_input::pad::Assignment::set_keyboard_reach`).
/// A window with no seat is left out, so the list is not indexed by window.
pub(crate) fn first_seats<'a>(windows: impl IntoIterator<Item = &'a [usize]>) -> Vec<usize> {
    windows
        .into_iter()
        .filter_map(|seats| seats.first().copied())
        .collect()
}

/// The slots whose Start a click is: `clicks` has bit `i` set for a click on window `i` (window 1
/// is 0), and each clicked window's first seat is a slot, as the bitmask the pause reads. A window
/// with no seat, or one the race does not have, is nobody's Start.
pub(crate) fn click_slots(clicks: u8, windows: &[&[usize]]) -> u8 {
    windows
        .iter()
        .enumerate()
        .filter(|(window, _)| *window < 8 && clicks & (1 << window) != 0)
        .filter_map(|(_, seats)| seats.first())
        .fold(0, |mask, slot| mask | 1 << slot)
}

/// The render target a window of `size` wants, at the profile's render scale: the
/// largest of these over the open windows is the one allocation they share.
pub(crate) fn shared_allocation(
    own: (u32, u32),
    screens: impl IntoIterator<Item = (u32, u32)>,
) -> (u32, u32) {
    screens
        .into_iter()
        .fold(own, |all, wanted| (all.0.max(wanted.0), all.1.max(wanted.1)))
}

/// What a window holding `count` seats from slot `first` is called: the join screen's own word
/// for a person and the seats' numbers, `PLAYER 3` for one and `PLAYER 3 - 4` for several. A
/// window with nobody to show is the word alone. Its title is the app's name and this, and a
/// holding frame draws it.
pub(crate) fn label(word: &str, first: usize, count: usize) -> String {
    match count {
        0 => word.to_string(),
        1 => format!("{word} {}", first + 1),
        _ => format!("{word} {} - {}", first + 1, first + count),
    }
}

#[cfg(test)]
#[path = "windows/tests.rs"]
mod tests;
