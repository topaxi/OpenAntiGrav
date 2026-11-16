//! The stages a race load reports to the loading screen.
//!
//! [`super::load`] is one long function on a worker thread, and Wipeout HD's
//! bar is driven by the stages of the original's own load, so the load says
//! where it has got to. It does so the way [`super::TextureSink`] hands the
//! load a device: a scope opened on the load's thread, so `load`'s signature
//! and every caller that has no screen (trace, `--dry-run`, tests) are
//! untouched, and [`reach`] is a no-op outside a scope.
//!
//! The stages are title-agnostic points in this build's own load order; which
//! fraction of the bar each one is worth is the title's table
//! (`oag_title::loading::Progression`). The original's four milestones sit at
//! its own constructors (`0x000b2af0`, `0x000b8590`, `0x0003c680`,
//! `0x0005cb40`); this build's order differs, so the two outer points are
//! identities and the inner two are placed by position, **chosen, not
//! measured** - see `docs/formats/hd-loading.md`.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use web_time::Instant;

/// A point in the race load, in the order the load reaches them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// The circuit's own file is read. The original's `0.4`: the track
    /// constructor has its first file open.
    TrackRead = 1,
    /// The circuit's geometry and environment are staged. The original's
    /// `0.5`: the world manager's constructor has finished.
    TrackBuilt = 2,
    /// The world is complete: spline, collision and gantry. The original's
    /// `0.75`, set when the mode constructor gets the world manager back.
    /// Placed by position here.
    WorldBuilt = 3,
    /// Every craft, sound and HUD asset is in. The original's `0.95`, set once
    /// the last craft is built. Placed by position here.
    CraftsBuilt = 4,
}

#[cfg(feature = "view")]
/// How far a race load has got, shared between the load thread and the screen.
#[derive(Debug, Clone, Default)]
pub struct LoadStages(Arc<AtomicU8>);

#[cfg(feature = "view")]
impl LoadStages {
    /// The highest [`Stage`] reached, as its number, or `0` before the first.
    #[must_use]
    pub fn reached(&self) -> u8 {
        self.0.load(Ordering::Relaxed)
    }

    /// Reports into `self` from the current thread until the scope drops.
    #[must_use]
    pub fn open(&self) -> StageScope {
        ACTIVE.with(|active| {
            *active.borrow_mut() = Some((Arc::clone(&self.0), Instant::now()));
        });
        StageScope
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<(Arc<AtomicU8>, Instant)>> = const { RefCell::new(None) };
}

#[cfg(feature = "view")]
/// Closes the scope [`LoadStages::open`] opened.
#[derive(Debug)]
pub struct StageScope;

#[cfg(feature = "view")]
impl Drop for StageScope {
    fn drop(&mut self) {
        ACTIVE.with(|active| *active.borrow_mut() = None);
    }
}

/// Why a load returned early: the process is exiting and nobody will read the result
/// (`oag_thread::gpu_workers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stopped;

impl std::fmt::Display for Stopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the process is exiting")
    }
}

impl std::error::Error for Stopped {}

/// Reports that the load on this thread has reached `stage`, then [`checkpoint`]s. Never goes
/// back, and reports nothing outside a [`StageScope`].
///
/// # Errors
///
/// [`Stopped`] when the process is exiting.
#[track_caller]
pub(super) fn reach(stage: Stage) -> Result<(), Stopped> {
    ACTIVE.with(|active| {
        if let Some((reached, since)) = active.borrow().as_ref() {
            reached.fetch_max(stage as u8, Ordering::Relaxed);
            log::debug!(
                "race load reached {stage:?} after {:.0} ms",
                since.elapsed().as_secs_f32() * 1000.0
            );
            hold_for_test();
        }
    });
    checkpoint()
}

/// Debug builds only: holds the load at each stage so the exit-during-a-load test has a load that
/// is still running when the process starts to leave, however fast the machine
/// (`drive_ground_truth`). It sits beside [`checkpoint`], not in it, so a load that dropped its
/// checks still waits here.
#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
fn hold_for_test() {
    if let Some(ms) = std::env::var("OAG_TEST_RACE_LOAD_STAGE_DELAY_MS")
        .ok()
        .and_then(|ms| ms.parse().ok())
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

#[cfg(not(all(debug_assertions, not(target_arch = "wasm32"))))]
fn hold_for_test() {}

/// Where the load may stop: `Err` once the process is exiting, so a window closed on the loading
/// screen does not wait out the rest of the load for a race nobody will see. Set only by
/// `gpu_workers::finish`, which `main` calls after the session is dropped, so a load nobody
/// leaves never takes the `Err`; the browser build never sets it. The result is dropped unread,
/// and nothing here logs above `trace`.
///
/// # Errors
///
/// [`Stopped`] when the process is exiting.
#[track_caller]
pub(super) fn checkpoint() -> Result<(), Stopped> {
    let at = std::panic::Location::caller();
    ACTIVE.with(|active| match active.borrow().as_ref() {
        Some((_, since)) => log::trace!(
            "race load checkpoint {}:{} after {:.0} ms",
            at.file(),
            at.line(),
            since.elapsed().as_secs_f32() * 1000.0
        ),
        None => log::trace!("race build checkpoint {}:{}", at.file(), at.line()),
    });
    if oag_thread::gpu_workers::stopping() {
        return Err(Stopped);
    }
    Ok(())
}

/// The archive reader a model builder takes for its textures (`oag_mesh`'s `build` family), that
/// finds nothing once the process is exiting: the builder skips each texture it was not given
/// instead of reading and decoding the rest, and the next [`checkpoint`] ends the load. The
/// model it returns is never read.
#[cfg(feature = "view")]
pub(super) fn reader(
    archives: &mut oag_assets::Archives,
) -> impl FnMut(&str) -> Option<Vec<u8>> + '_ {
    move |path| {
        if oag_thread::gpu_workers::stopping() {
            return None;
        }
        archives.read_name(path).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stage_is_seen_only_inside_a_scope_and_never_goes_back() {
        let stages = LoadStages::default();
        reach(Stage::TrackRead).unwrap();
        assert_eq!(stages.reached(), 0, "no scope open, nothing reported");
        {
            let _scope = stages.open();
            reach(Stage::WorldBuilt).unwrap();
            reach(Stage::TrackRead).unwrap();
            assert_eq!(stages.reached(), Stage::WorldBuilt as u8);
        }
        reach(Stage::CraftsBuilt).unwrap();
        assert_eq!(stages.reached(), 3, "the scope closed with its guard");
    }

    #[test]
    fn a_load_reports_its_stages_in_order() {
        let stages = LoadStages::default();
        let reported = std::thread::spawn({
            let stages = stages.clone();
            move || {
                let _scope = stages.open();
                [
                    Stage::TrackRead,
                    Stage::TrackBuilt,
                    Stage::WorldBuilt,
                    Stage::CraftsBuilt,
                ]
                .map(|stage| {
                    reach(stage).unwrap();
                    stages.reached()
                })
            }
        })
        .join()
        .unwrap();
        assert_eq!(reported, [1, 2, 3, 4]);
    }

    /// The stop is process-wide and one-way, which is why this is the one test of it in this
    /// binary (nextest runs each test in a process of its own).
    #[test]
    fn a_load_ends_at_its_next_checkpoint_once_the_process_is_exiting() {
        assert_eq!(checkpoint(), Ok(()), "a load nobody leaves goes on");
        let _worker = oag_thread::gpu_workers::Guard::enter("stages-test");
        assert!(!oag_thread::gpu_workers::finish_within(
            std::time::Duration::ZERO
        ));
        assert_eq!(checkpoint(), Err(Stopped));
        assert_eq!(reach(Stage::TrackRead), Err(Stopped));
    }
}
