//! The third wave: `Repulser_AdvanceWave`'s fork at a split junction.
//!
//! Read in `docs/ghidra/functions/psp-pulse-usa/repulser.md`, "The fork wave, read"
//! (confidence 88). When the forward wave's step crosses a junction with an
//! alternate successor and this Repulser has not forked (`+0x25c`), a third wave
//! starts at the junction and walks the alternate path for the steps left at the
//! crossing (mode 3), then takes the parent's full step count, rejoining the ring
//! at the branch's far end. On the spawn update its previous point equals its
//! current one (`+0x140 = +0x110`), so it sweeps nothing across the jump. Craft
//! only: `RepulserPool_SweepTargets` tests Mines and Bombs against waves 0 and 1.
//!
//! **Not built: a fork off the backward wave.** `AiTrack_StepBackward`
//! (`0x0887e2f0`) reads the same `path+0x10` junction as the forward stepper
//! (`0x0887e334`), so what a backward wave meets at a path's start is not
//! established.
//!
//! **Not built: the init-tick variant** (`Repulser_ForkAtJunction`, `0x08876634`,
//! 72), for a firer already on a branch. Chosen, not measured: it forks only when a
//! wave crosses a split.
//!
//! **Ours, chosen rather than measured:** ring samples, not control points (as in
//! [`super`]), and the crossing tested on ring indices, so a step landing exactly
//! on the split's first sample counts.

use super::{Front, normalize_or_zero};
use oag_core::math::Vec3;
use oag_race::Course;

/// The third wave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fork {
    /// Which [`Course::branches`] entry it is on, or `None` once it has rejoined
    /// the ring, after which [`Self::front`]'s index is a ring index.
    pub branch: Option<u16>,
    /// Sample index within the branch while [`Self::branch`] is set.
    pub offset: u32,
    /// Where it is, and where it was last update.
    pub front: Front,
}

impl Fork {
    /// The forward wave stepping from ring index `from` by `step` samples
    /// crosses a split, or `None`. The first branch in junction order that it
    /// crosses wins, as the first junction a stepping cursor meets would.
    pub(super) fn crossing(course: &Course, from: usize, step: usize) -> Option<Self> {
        let count = course.len();
        if count == 0 {
            return None;
        }
        course
            .branches()
            .iter()
            .enumerate()
            .find_map(|(index, branch)| {
                let gap = (branch.split + count - from) % count;
                let offset = step.checked_sub(gap)?;
                if gap == 0 || offset >= branch.len() {
                    return None;
                }
                let point = branch.centres[offset];
                Some(Self {
                    branch: Some(u16::try_from(index).ok()?),
                    offset: u32::try_from(offset).ok()?,
                    front: Front {
                        index: 0,
                        point,
                        previous: point,
                    },
                })
            })
    }

    /// One update: `step` samples along the branch, or along the ring once it
    /// has rejoined it.
    pub(super) fn advance(&mut self, course: &Course, step: usize) {
        let count = course.len();
        if count == 0 {
            return;
        }
        self.front.previous = self.front.point;
        if let Some(branch) = self.branch.and_then(|b| course.branches().get(b as usize)) {
            let len = branch.len();
            let offset = self.offset as usize;
            let ring = if offset + step < len {
                Err(offset + step)
            } else {
                Ok((branch.merge + offset + step - len) % count)
            };
            match ring {
                Err(next) => {
                    self.offset = u32::try_from(next).unwrap_or(self.offset);
                    self.front.point = branch.centres[next];
                }
                Ok(index) => {
                    self.branch = None;
                    self.offset = 0;
                    self.front.index = u32::try_from(index).unwrap_or(0);
                    if let Some(point) = course.centre(index) {
                        self.front.point = point;
                    }
                }
            }
            return;
        }
        let index = (self.front.index as usize + step) % count;
        if let Some(point) = course.centre(index) {
            self.front.point = point;
            self.front.index = u32::try_from(index).unwrap_or(self.front.index);
        }
    }

    /// Whether this update moved it - the spawn update does not.
    #[must_use]
    pub fn moved(&self) -> bool {
        normalize_or_zero(self.front.point - self.front.previous) != Vec3::ZERO
    }
}

#[cfg(test)]
mod tests;
