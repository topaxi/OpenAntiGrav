//! How far the boot's media phase has got, and the four small helpers the
//! loaders report through.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use std::sync::Mutex;

/// How far the media phase has got.
///
/// A snapshot handed out by value, on the same terms as
/// [`crate::prefetch::Progress`]: the loading screen polls it from the frame
/// loop it already has and holds no lock while it draws.
///
/// There is no `finished` here because [`MediaWorker::is_finished`] already
/// answers that, and the thread is what knows - `done == total` is true for the
/// moment between the last load returning and the thread handing back its
/// [`Media`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaProgress {
    /// What [`MediaPlan::loads`] counted, or `0` before the phase starts.
    pub total: usize,
    /// Loads that have returned, successfully or not.
    pub done: usize,
    /// What is loading right now, by the entry name the plan gave.
    pub current: Option<String>,
    /// What that load is actually doing - see [`crate::movie::Step`].
    ///
    /// **This is the difference between a wait nobody notices and eighty
    /// seconds of one.** `Some(Step::Cached)` and `Some(Step::Transcoding)` sit
    /// under the same entry name and mean entirely different things to whoever
    /// is looking at the screen. `None` before a load has said anything, which
    /// includes the sound loads: [`crate::at3`] has its own cache and does not
    /// report through this.
    pub step: Option<crate::movie::Step>,
}

/// Names what is about to load. Overwrites rather than clears, so the label
/// under the bar never blinks empty between two loads.
///
/// The step is cleared, though, and must be: it described the *previous* load,
/// and carrying it over would caption a cache hit with the last transcode's
/// frame counter.
pub(super) fn starting(progress: &Mutex<MediaProgress>, what: &str) {
    let mut at = lock_media(progress);
    at.current = Some(what.to_string());
    at.step = None;
}

/// Counts a load that has returned, however it returned. See [`MediaPlan::loads`].
pub(super) fn loaded(progress: &Mutex<MediaProgress>) {
    lock_media(progress).done += 1;
}

/// The callback the movie loaders report their [`crate::movie::Step`] through.
pub(super) fn watching(progress: &Mutex<MediaProgress>) -> impl Fn(crate::movie::Step) + Sync {
    move |step| lock_media(progress).step = Some(step)
}

/// The same rule [`crate::prefetch`]'s own `lock` follows: a poisoned lock is a
/// worker that panicked, and the last snapshot it wrote is still a true
/// statement about what got done. Bringing the window down over it would swap a
/// boot with no movies for no boot at all.
pub(super) fn lock_media(
    progress: &Mutex<MediaProgress>,
) -> std::sync::MutexGuard<'_, MediaProgress> {
    progress
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
