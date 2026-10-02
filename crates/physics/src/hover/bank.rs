//! The bank-to-yaw coupling's gain, which depends on the hover variant.

use super::BANK_TO_YAW_GAIN;
use crate::ShipState;

/// The same coupling in `Ship_HoverFourCorner`'s epilogue, which Zone runs where every
/// other mode runs `Ship_HoverTwoPoint`: `0x0884b778 - 0x0884b794` loads `0x42480000`
/// (`50.0`) where the two-point law (`0x0884ad2c`) uses `30.0`, behind the same
/// `craft+0x2a4 != 0` guard and the same `(1 - magLockBlend)`. Selected by
/// [`crate::ShipState::four_corner`]. See `docs/ghidra/functions/psp-pulse-usa/zone-rest.md`.
pub const BANK_TO_YAW_GAIN_FOUR_CORNER: f32 = 50.0;

/// The gain this craft's hover variant uses.
pub(super) fn gain(state: &ShipState) -> f32 {
    if state.four_corner {
        BANK_TO_YAW_GAIN_FOUR_CORNER
    } else {
        BANK_TO_YAW_GAIN
    }
}
