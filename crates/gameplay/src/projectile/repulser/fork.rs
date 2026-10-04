//! The third wave: `Repulser_AdvanceWave`'s fork at a split junction.
//!
//! Read in `docs/ghidra/functions/psp-pulse-usa/repulser.md`, "The fork wave,
//! read" (confidence 88). When a wave's step crosses a junction with an
//! alternate successor (forward) or an alternate predecessor (backward), and
//! this Repulser has not forked before (`+0x25c`), a third wave starts at the
//! junction and walks the alternate path for the steps left at the crossing
//! (mode 3). From the next update on, it takes the parent's full step count.
//! At the far end of the branch it rejoins the ring and walks on. On the spawn
//! update its previous point is its current one (`+0x140 = +0x110`), so it
//! sweeps nothing across the jump. Craft only: `RepulserPool_SweepTargets`
//! tests Mines and Bombs against waves 0 and 1.
//!
//! **Not built: the init-tick variant** (`Repulser_ForkAtJunction`,
//! `0x08876634`, 72), which starts the third wave when the firer already sits
//! on a branch beside its sibling. Chosen, not measured: a Repulser fired from
//! a branch forks only when a wave crosses a split.
//!
//! **Ours, chosen rather than measured:** ring samples, not control points, as
//! for the two primary waves ([`super`]'s own list); the crossing is tested on
//! ring indices, so a step that lands exactly on the split's first sample counts.

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
    /// Whether it walks backward, as the wave it forked from does.
    pub backward: bool,
    /// Where it is, and where it was last update.
    pub front: Front,
}

impl Fork {
    /// The wave that steps from ring index `from` by `step` samples (backward if
    /// `backward`) crosses a split, or `None`. The first branch in junction
    /// order wins, as the first junction a stepping cursor meets would.
    pub(super) fn crossing(
        course: &Course,
        from: usize,
        step: usize,
        backward: bool,
    ) -> Option<Self> {
        let count = course.len();
        if count == 0 {
            return None;
        }
        for (index, branch) in course.branches().iter().enumerate() {
            let len = branch.len();
            let (gap, offset) = if backward {
                // Leaving the merge's first sample backward, onto the branch's
                // last.
                let gap = (from + count - (branch.merge + count - 1) % count) % count;
                (gap, len.checked_sub(1 + step.checked_sub(gap)?)?)
            } else {
                let gap = (branch.split + count - from) % count;
                (gap, step.checked_sub(gap)?)
            };
            if gap == 0 || gap > step || offset >= len {
                continue;
            }
            let point = branch.centres[offset];
            return Some(Self {
                branch: u16::try_from(index).ok(),
                offset: u32::try_from(offset).ok()?,
                backward,
                front: Front {
                    index: 0,
                    point,
                    previous: point,
                },
            });
        }
        None
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
            let ring = if self.backward {
                match offset.checked_sub(step) {
                    Some(next) => Err(next),
                    // Off the branch's first sample: onto the split's previous
                    // ring sample, and on from there.
                    None => Ok((branch.split + count * 2 - 1 - (step - offset - 1)) % count),
                }
            } else if offset + step < len {
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
        let index = if self.backward {
            (self.front.index as usize + count - step % count) % count
        } else {
            (self.front.index as usize + step) % count
        };
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
