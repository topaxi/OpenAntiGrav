//! The bank-to-yaw coupling's gain, which depends on the hover variant.

use crate::ShipState;

/// The same coupling in `Ship_HoverFourCorner`'s epilogue, which Zone runs where every
/// other mode runs `Ship_HoverTwoPoint`: `0x0884b778 - 0x0884b794` loads `0x42480000`
/// (`50.0`) where the two-point law (`0x0884ad2c`) uses `30.0`, behind the same
/// `craft+0x2a4 != 0` guard and the same `(1 - magLockBlend)`. Selected by
/// [`crate::ShipState::four_corner`], under [`crate::laws::Laws::bank_to_yaw_zone`], which
/// is a title's: Wipeout HD's Zone keeps its racing `30`. See
/// `docs/ghidra/functions/psp-pulse-usa/zone-rest.md`.
pub const BANK_TO_YAW_GAIN_FOUR_CORNER: f32 = 50.0;

/// Wipeout Pure's two-point gain: its two-point hover (`0x0892dea8`) loads `50.0`, the gain
/// Pulse keeps for its four-corner hover, and its four-corner hover (`0x0892e5b8`) loads `50.0`
/// too. Measured live: `torque.y / right.y = 50.0000` on 129 banked frames
/// (`docs/physics/pure-physics.md`). Pure has no mag strips, so the blend term is moot there.
pub const BANK_TO_YAW_GAIN_PURE: f32 = 50.0;

/// The gain this craft's hover variant uses, under the title's laws.
pub(super) fn gain(state: &ShipState, laws: &crate::laws::Laws) -> f32 {
    if state.four_corner {
        laws.bank_to_yaw_zone
    } else {
        laws.bank_to_yaw
    }
}
