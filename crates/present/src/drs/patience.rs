//! What a discrete rung needs before it gives back what it shed: a run of frames that all
//! predict a fit, a run that doubles when a restore does not hold, and a window that pays each
//! doubling back. Shared by the blur rung ([`super::blur`]) and the reflection rung
//! ([`super::reflection`]); the module docs of the first say why each part is there.

use super::blur::{MAX_BACKOFF, relapse_frames, restore_frames};

/// A rung's restore patience and relapse backoff.
#[derive(Debug, Clone, Default)]
pub(super) struct Patience {
    /// Consecutive frames a restore was predicted to fit.
    run: u32,
    /// Frames since the last restore, or since the last relapse window that held, while a
    /// backoff is still being paid back.
    since_restore: Option<u32>,
    /// Doublings of the restore patience earned by restores that did not hold.
    backoff: u32,
    /// The restore patience at this frame's target, before backoff.
    frames: u32,
}

impl Patience {
    /// The restore patience in force, after backoff.
    fn patience(&self) -> u32 {
        self.frames << self.backoff.min(MAX_BACKOFF)
    }

    /// Called once per recorded frame at a target `period`, before any decision.
    pub(super) fn begin_frame(&mut self, period: f32) {
        self.frames = restore_frames(period);
        if let Some(since) = self.since_restore.as_mut() {
            *since += 1;
            if *since >= relapse_frames(period) {
                // A window held: one doubling paid back, and the next window starts counting.
                self.backoff = self.backoff.saturating_sub(1);
                self.since_restore = (self.backoff > 0).then_some(0);
            }
        }
    }

    /// A shed was taken: the run is over, and a shed inside a relapse window is a restore
    /// that did not hold (or one paying back an earlier backoff, which did not hold either).
    pub(super) fn shed(&mut self) {
        self.run = 0;
        if self.since_restore.take().is_some() {
            self.backoff = (self.backoff + 1).min(MAX_BACKOFF);
        }
    }

    /// Counts one frame toward a restore; `true` once the run is long enough, which starts the
    /// relapse window. A frame that does not fit breaks the run.
    pub(super) fn ready(&mut self, fits: bool) -> bool {
        if !fits {
            self.run = 0;
            return false;
        }
        self.run += 1;
        if self.run < self.patience() {
            return false;
        }
        self.run = 0;
        self.since_restore = Some(0);
        true
    }

    /// A stall throws the run away; the backoff is a property of the machine and stays.
    pub(super) fn reset_run(&mut self) {
        self.run = 0;
    }
}
