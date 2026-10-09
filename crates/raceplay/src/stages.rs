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

/// How far a race load has got, shared between the load thread and the screen.
#[derive(Debug, Clone, Default)]
pub struct LoadStages(Arc<AtomicU8>);

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

/// Closes the scope [`LoadStages::open`] opened.
#[derive(Debug)]
pub struct StageScope;

impl Drop for StageScope {
    fn drop(&mut self) {
        ACTIVE.with(|active| *active.borrow_mut() = None);
    }
}

/// Reports that the load on this thread has reached `stage`. Never goes back,
/// and does nothing outside a [`StageScope`].
pub(super) fn reach(stage: Stage) {
    ACTIVE.with(|active| {
        if let Some((reached, since)) = active.borrow().as_ref() {
            reached.fetch_max(stage as u8, Ordering::Relaxed);
            log::info!(
                "race load reached {stage:?} after {:.0} ms",
                since.elapsed().as_secs_f32() * 1000.0
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stage_is_seen_only_inside_a_scope_and_never_goes_back() {
        let stages = LoadStages::default();
        reach(Stage::TrackRead);
        assert_eq!(stages.reached(), 0, "no scope open, nothing reported");
        {
            let _scope = stages.open();
            reach(Stage::WorldBuilt);
            reach(Stage::TrackRead);
            assert_eq!(stages.reached(), Stage::WorldBuilt as u8);
        }
        reach(Stage::CraftsBuilt);
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
                    reach(stage);
                    stages.reached()
                })
            }
        })
        .join()
        .unwrap();
        assert_eq!(reported, [1, 2, 3, 4]);
    }
}
