//! Carrying a ribbon with its craft when the craft is put somewhere else.

use oag_core::math::{Quat, Vec3};

use super::Exhaust;

impl Exhaust {
    /// Moves the whole position history rigidly: the point `from` goes to `to` and every
    /// direction (and the offset of every sample from `from`) turns by `turn`.
    ///
    /// A craft put back on a pose after it has run (a posed capture's `--pose-hold`) takes its
    /// ribbon with it. Left where it was laid, the ribbon hangs off the old pose: a resting craft
    /// that yawed under the clock then shows a streak at the angle it had turned to.
    pub fn move_rigidly(&mut self, turn: Quat, from: Vec3, to: Vec3) {
        let place = |p: Vec3| turn * (p - from) + to;
        for p in &mut self.trail {
            *p = place(*p);
        }
        for back in &mut self.trail_back {
            *back = turn * *back;
        }
        if let Some((p, back)) = &mut self.trail_dropped {
            *p = place(*p);
            *back = turn * *back;
        }
    }

    /// The newest trail sample, `(nozzle, back)`: where the last tick laid the ribbon's head and
    /// the direction the exhaust pointed.
    #[must_use]
    pub fn newest_trail_sample(&self) -> Option<(Vec3, Vec3)> {
        self.trail_samples().next()
    }
}
