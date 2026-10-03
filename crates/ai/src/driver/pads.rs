//! Swinging over a pad the caller wants this driver to take.
//!
//! # Ours, chosen, not measured
//!
//! Nothing read of the original says its opponents steer for a pad. The
//! weapon-pad list (`world+0x10c`, count `+0x1d4`) has three documented
//! readers - `World_CollectNodeLists`, `WeaponPads_TestCraft` and the
//! weapons-off reset, all in `docs/ghidra/functions/psp-pulse-usa/pads.md` -
//! and none of them is AI code. That is a reading of the documented callers,
//! not an exhaustive cross-reference. So this is smarter driving of our own,
//! under the rule that the AI obeys the player's physics: it chooses where in
//! the road to be and nothing else.
//!
//! # A pull on the final offset, not another lean
//!
//! [`Driver::drift`]'s other terms are fractions of the corridor scaled by
//! `corridor_use` and the pilot's `width`, which keeps them well inside the
//! road - and would keep a pad near the edge out of reach. So this one blends
//! the **finished** lateral offset toward the pad's own offset across the line,
//! and the corridor clamp after it is still the backstop.
//!
//! A pad sits still in the corridor, so the target is the pad's offset from the
//! line rather than from the craft: one that moved with the craft would chase
//! itself as the craft swung toward it.

use super::Driver;
use crate::Pad;

/// How far ahead a pad starts to pull, in units along the line.
///
/// **Chosen.** About one second of travel at VENOM racing speed: long enough
/// that the swing is a lean rather than a swerve, short enough that a pad a
/// corner away does not drag the craft off its line through the corner before
/// it.
pub const LOOKAHEAD: f32 = 150.0;

/// Inside this distance the pull is total: the craft aims straight over the
/// pad. **Chosen.**
const HOLD: f32 = 40.0;

impl Driver {
    /// `offset` - the lateral offset [`Driver::drift`] settled on, in units -
    /// pulled toward `pad`, or `offset` itself when there is no pad.
    ///
    /// **A charge being dodged wins outright**: while `dodging` the pad is
    /// ignored, because a mine laid on a pad is exactly the one not to drive
    /// over. Returns its argument untouched rather than blending by zero, so a
    /// race that never hands a pad over computes the same bits it always did.
    pub(super) fn toward_pad(offset: f32, pad: Option<Pad>, dodging: bool) -> f32 {
        let Some(pad) = pad else {
            return offset;
        };
        if dodging || pad.distance <= 0.0 || pad.distance >= LOOKAHEAD {
            return offset;
        }
        let pull = if pad.distance <= HOLD {
            1.0
        } else {
            let t = (LOOKAHEAD - pad.distance) / (LOOKAHEAD - HOLD);
            t * t * (3.0 - 2.0 * t)
        };
        offset + (pad.offset - offset) * pull
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(distance: f32, offset: f32) -> Option<Pad> {
        Some(Pad { distance, offset })
    }

    #[test]
    fn no_pad_leaves_the_offset_bit_for_bit() {
        for offset in [0.0, -0.0, 1.5, -3.25] {
            assert_eq!(
                Driver::toward_pad(offset, None, false).to_bits(),
                f32::to_bits(offset)
            );
        }
    }

    #[test]
    fn a_near_pad_is_aimed_at_and_a_far_one_is_ignored() {
        assert_eq!(Driver::toward_pad(1.0, pad(10.0, 6.0), false), 6.0);
        assert_eq!(Driver::toward_pad(1.0, pad(LOOKAHEAD, 6.0), false), 1.0);
        let halfway = Driver::toward_pad(0.0, pad((LOOKAHEAD + HOLD) * 0.5, 6.0), false);
        assert!(halfway > 0.0 && halfway < 6.0, "{halfway}");
    }

    #[test]
    fn a_charge_being_dodged_beats_the_pad() {
        assert_eq!(Driver::toward_pad(-2.0, pad(10.0, 6.0), true), -2.0);
    }
}
