//! Keeping the machine awake while someone is playing.
//!
//! A compositor counts only the input it handles itself, and the pad is read straight from the
//! device, so a player on a controller looks idle to it and the screen saver or lock fires
//! mid-race. [`Idle`] asks the platform to hold off while [`Rule::wanted`]: a race is running, or
//! a pad was touched within [`PAD_GRACE`]. A paused race, or a menu nobody touches, lets the
//! machine idle. Only a Wayland desktop has a backend (`idle-inhibit-unstable-v1`); every other
//! platform compiles to nothing (`docs/tools/oag-game.md`, "Idle inhibition").
//!
//! This is output-side state: the simulation never sees it, and it is never hashed.

use std::time::Duration;

use log::trace;
use web_time::Instant;
use winit::window::{Window, WindowId};

#[cfg(target_os = "linux")]
mod wayland;

/// How long after the last pad input a menu still counts as being played.
pub const PAD_GRACE: Duration = Duration::from_secs(60);

/// When the machine should be kept awake. The clock is passed in, so a test needs no wall time.
#[derive(Debug, Default, Clone, Copy)]
pub struct Rule {
    last_pad: Option<Instant>,
}

impl Rule {
    /// A pad was touched at `now`.
    pub fn pad_spoke(&mut self, now: Instant) {
        self.last_pad = Some(now);
    }

    /// Whether to hold the inhibitor at `now`. `race_running` is a race that is neither paused
    /// nor finished.
    #[must_use]
    pub fn wanted(&self, race_running: bool, now: Instant) -> bool {
        race_running
            || self
                .last_pad
                .is_some_and(|last| now.saturating_duration_since(last) < PAD_GRACE)
    }
}

/// What a platform offers: hold or release the inhibitor on the window it was opened for.
pub trait Backend {
    fn hold(&mut self, on: bool);
}

/// The rule, the window it is bound to, and whether the inhibitor is held now.
#[derive(Default)]
pub struct Idle {
    rule: Rule,
    /// Minimised or hidden: the protocol is per surface and a hidden game should not inhibit.
    hidden: bool,
    held: bool,
    bound: Option<WindowId>,
    backend: Option<Box<dyn Backend>>,
}

impl std::fmt::Debug for Idle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Idle")
            .field("rule", &self.rule)
            .field("hidden", &self.hidden)
            .field("held", &self.held)
            .field("backend", &self.backend.is_some())
            .finish()
    }
}

impl Idle {
    /// The window was hidden or shown again.
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    /// Whether the inhibitor is held.
    #[must_use]
    pub fn held(&self) -> bool {
        self.held
    }

    /// Binds to the window `id`, opening a backend with `open` when it is a window not yet bound
    /// (the first, or a recreated one). Does nothing for the window already bound, whether or not
    /// the platform had a backend for it.
    pub fn bind(&mut self, id: WindowId, open: impl FnOnce() -> Option<Box<dyn Backend>>) {
        if self.bound == Some(id) {
            return;
        }
        self.backend = None;
        self.held = false;
        self.bound = Some(id);
        self.backend = open();
    }

    /// One frame: notes a pad input, then holds or releases on the edge only.
    pub fn update(&mut self, race_running: bool, pad_spoke: bool, now: Instant) {
        if pad_spoke {
            self.rule.pad_spoke(now);
        }
        let want = !self.hidden && self.rule.wanted(race_running, now);
        if want != self.held
            && let Some(backend) = &mut self.backend
        {
            backend.hold(want);
            self.held = want;
            trace!("idle inhibitor: {}", if want { "held" } else { "released" });
        }
    }

    /// [`Self::bind`] for a real window, then [`Self::update`].
    pub fn update_window(
        &mut self,
        window: Option<&Window>,
        race_running: bool,
        pad_spoke: bool,
        now: Instant,
    ) {
        if let Some(window) = window {
            self.bind(window.id(), || open(window));
        }
        self.update(race_running, pad_spoke, now);
    }

    /// Lets go now, whatever the rule says, and holds off until the next [`Self::update`].
    pub fn release(&mut self) {
        if self.held
            && let Some(backend) = &mut self.backend
        {
            backend.hold(false);
            self.held = false;
        }
    }
}

#[cfg(target_os = "linux")]
fn open(window: &Window) -> Option<Box<dyn Backend>> {
    wayland::open(window)
}

#[cfg(not(target_os = "linux"))]
fn open(_: &Window) -> Option<Box<dyn Backend>> {
    None
}

#[cfg(test)]
mod tests;
