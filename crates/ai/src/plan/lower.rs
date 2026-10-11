//! How a failure in [`SpeedPlan::learn`] lowers the ceilings behind it.
//! Split out of `plan.rs` under the 1,000-line rule.

use super::{LOOKBACK_TICKS, STALL_SPEED, SpeedPlan, Trace};

/// How much a failure lowers the ceilings behind it, as a fraction of the speed
/// there. **Chosen, not measured**: small enough to stay near the limit, large
/// enough that a corner settles in a handful of tries.
const LOWER: f32 = 0.96;

/// How much faster than its slowest point in a failure's lookback a craft must
/// be going for the failure to count as one on the way out of that point (see
/// [`SpeedPlan::lower`]). **Chosen, not measured.**
const ACCELERATED: f32 = 1.25;

/// How a failure's lowering chooses the samples it lowers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lowering {
    /// From the slowest point of the lookback on, when the craft was
    /// accelerating out of it, and never on the failing tick's own speed. See
    /// [`SpeedPlan::lower`].
    FromTrough,
    /// The whole lookback, failing tick included: the search as it was before
    /// [`Self::FromTrough`]. Kept as the second search
    /// ([`SpeedPlan::build_within`]) - its stray floors are what clear
    /// `07_Track` VENOM's hairpin-exit pad and HD `04_Track` PHANTOM at
    /// 1227-1231. Chosen, not measured.
    Window,
}

/// How many failures at one site [`SpeedPlan::lower`] answers from the trough
/// on before it lowers its whole window again. **Chosen, not measured.**
const TROUGH_TRIES: u32 = 3;

impl SpeedPlan {
    /// Lowers the ceilings over the stretch `recent` covers, to [`LOWER`] of the
    /// speed the craft actually did there. Returns how many moved.
    pub(super) fn lower(&mut self, recent: &[Trace], tries: u32, lowering: Lowering) -> u32 {
        // Harder the more often this spot has failed: one [`LOWER`] for the
        // first three tries, two for the next three, and so on, so a corner
        // needing half the top speed fits inside [`super::MAX_TRIES`]. A loop of
        // multiplies, not `powi`: an intrinsic with no portability promise.
        let mut factor = LOWER;
        for _ in 0..tries / 3 {
            factor *= LOWER;
        }
        let mut moved = 0;
        // The last [`LOOKBACK_TICKS`] ticks **on the ground**: a craft that
        // left the road over a crest and hit a wall spent its recent past in the
        // air, where a ceiling means nothing; the takeoff speed has to come down.
        // Under [`Lowering::FromTrough`], not the failing tick itself: its speed
        // is what the wall left (`07_Track`'s hairpin at sample 2140 read 0.0
        // straight after 136), and lowering on it pinned that sample to the
        // floor.
        let skip = usize::from(lowering == Lowering::FromTrough);
        let window: Vec<&Trace> = recent
            .iter()
            .rev()
            .skip(skip)
            .filter(|trace| !trace.airborne)
            .take(LOOKBACK_TICKS)
            .collect();
        // **Only from the slowest point on, when the craft was accelerating out
        // of it**: what failed is the speed gained since, not the trough. At
        // low speed 45 ticks reach back over few samples, and every failure on
        // the way out of a slow corner lowered the corner again until it sat at
        // the floor (`07_Track` FLASH, samples 2143-2147, at 16 u/s with the
        // failures 40 samples on). **Past [`TROUGH_TRIES`] failures at one site
        // the whole window comes down again**: `07_Track` VENOM's hairpin exit
        // has a pad that throws a craft into the wall from any speed over about
        // 25 u/s, and only a slower trough clears it. Chosen, not measured.
        let trough = window
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.speed.total_cmp(&b.1.speed))
            .map_or(0, |(at, _)| at);
        let newest = window.first().map_or(0.0, |trace| trace.speed);
        let reach = if lowering == Lowering::FromTrough
            && tries <= TROUGH_TRIES
            && trough > 0
            && newest > window[trough].speed * ACCELERATED
        {
            trough
        } else {
            window.len()
        };
        for trace in window.into_iter().take(reach) {
            let i = trace.index as usize;
            if self.hold[i] {
                continue;
            }
            // Of the lower of what it did and what it was allowed, so a retry
            // always moves.
            let lowered = (trace.speed.min(self.ceiling[i]) * factor).max(STALL_SPEED * 4.0);
            if lowered < self.ceiling[i] {
                self.ceiling[i] = lowered;
                moved += 1;
            }
        }
        moved
    }
}
