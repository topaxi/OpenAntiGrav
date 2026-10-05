//! The wrapping box the weather's two effects fill: modifier type `0x13`.
//!
//! `WO_RAIN` and `WO_SNOW` each author one type-19 modifier, which
//! `FUN_088fb11c` runs on every live particle each tick. Read off its
//! decompile and confirmed against live memory on Fort Gale
//! (`docs/ghidra/functions/psp-pulse-usa/weather.md`):
//!
//! - **`params[0..3]` is the wind**, a world vector the weather node
//!   overwrites every frame - the authored values are never used.
//! - **`params[4]` is the box's half-extent**, `50` on both effects: a
//!   particle that leaves `[-e, e]` on an axis is moved a whole `2e` back
//!   inside, so a field of a few dozen particles fills a cube for ever.
//! - **The resource's `+0x98` is how far ahead of the camera the effect
//!   sits** ([`ParticleSystem::view_depth`]), `60` on `WO_RAIN` and `6` on the
//!   screen lens - the emitter's model matrix is a pure translation by `-depth`
//!   along the view's `z`.

use super::{Emitter, ParticleSystem};

/// The one [`super::Modifier::kind`] with this behaviour.
pub const MODIFIER_WRAP_BOX: u32 = 19;

/// A wrapping box, in the emitter's own field coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldBox {
    /// `params[4]`: the half-extent every axis wraps at.
    pub half_extent: f32,
}

impl ParticleSystem<'_> {
    /// `emitter`'s wrapping box, when it authors a type-19 modifier.
    ///
    /// `None` for every effect that is not the weather.
    #[must_use]
    pub fn field_box(&self, emitter: &Emitter) -> Option<FieldBox> {
        let modifier = emitter
            .modifiers
            .iter()
            .find(|modifier| modifier.kind == MODIFIER_WRAP_BOX)?;
        Some(FieldBox {
            half_extent: modifier.params[4],
        })
    }

    /// The resource's `+0x98`: how far ahead of the lens a view-space effect sits.
    ///
    /// Read for every emitter and meaningful for the weather's alone, where it
    /// is `60` on `WO_RAIN` and `WO_SNOW` and `6` on `WO_RAIN_LENS`.
    #[must_use]
    pub fn view_depth(&self, data: &[u8], emitter: &Emitter) -> Option<f32> {
        let record = data.get(self.resource_base() + emitter.offset..)?;
        Some(self.order.f32(record, 0x98))
    }
}
