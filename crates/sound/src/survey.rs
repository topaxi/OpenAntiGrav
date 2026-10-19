//! The music survey, run off the calling thread.
//!
//! [`MusicDiscs::survey`] opens the booted source and, for a PSP or PS2 boot,
//! lists its soundtrack and every image on the search path to find the other
//! release: 359 ms for Pulse PSP on the Steam Deck. At boot that runs before
//! the window, but a title picked in the launcher surveys inside the frame that
//! loads it. [`MusicDiscs::survey_in_background`] returns at once and answers
//! from a [`Later`] once the survey lands; until then the discs read as
//! "nothing known yet", and every music worker waits for the answer first
//! ([`MusicDiscs::wait_surveyed`]), so no start is lost to it.

use std::sync::{Arc, OnceLock};

use super::{Library, Listings, MusicDiscs};

/// A survey's answer, filled once by its worker and shared by every clone.
///
/// `None` inside is a value that was surveyed in place (or never surveyed),
/// whose own fields are the answer.
#[derive(Debug, Clone, Default)]
pub(crate) struct Later(Option<Arc<OnceLock<MusicDiscs>>>);

impl Later {
    /// Whether a survey is still running.
    pub(crate) fn is_running(&self) -> bool {
        self.0.as_ref().is_some_and(|cell| cell.get().is_none())
    }
}

impl MusicDiscs {
    /// [`Self::survey`] on a worker: returns at once, knowing nothing yet.
    ///
    /// Clones share the answer and the listings it reads. In the browser this
    /// surveys in place, as a worker that waits for it there cannot block.
    #[must_use]
    pub fn survey_in_background(booted: &str, library: &'static dyn Library) -> Self {
        if cfg!(target_arch = "wasm32") {
            return Self::survey(booted, library);
        }
        let cell = Arc::new(OnceLock::new());
        let listings = Listings::default();
        let discs = Self {
            library,
            listings: listings.clone(),
            later: Later(Some(Arc::clone(&cell))),
            ..Self::default()
        };
        let booted = booted.to_string();
        // Detached: the answer lands in `cell`, which every clone holds.
        drop(oag_thread::Task::spawn("music-survey", move || {
            // A survey that panics still answers, with nothing found, so a
            // worker waiting on it is not left waiting.
            let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Self::survey_into(&booted, library, listings.clone())
            }))
            .unwrap_or_else(|_| Self {
                library,
                listings,
                ..Self::default()
            });
            log::debug!("audio: music discs, {}", found.describe());
            let _ = cell.set(found);
        }));
        discs
    }

    /// The surveyed answer: this value itself, or what its background survey
    /// found (nothing yet while it runs). Never waits.
    pub(crate) fn settled(&self) -> &Self {
        self.later
            .0
            .as_ref()
            .and_then(|cell| cell.get())
            .unwrap_or(self)
    }

    /// Blocks until a background survey has answered; at once otherwise.
    ///
    /// For a worker's thread or a capture, never the frame thread.
    pub fn wait_surveyed(&self) {
        if let Some(cell) = &self.later.0 {
            cell.wait();
        }
    }
}
