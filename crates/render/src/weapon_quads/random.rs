//! The muzzle flash's per-tick roll: rotation, size, and fade.
//!
//! **Deliberately not `oag_core::rng::Rng`, and not fed from
//! `world.rng`.** The flash is a render-only embellishment - nothing in
//! `oag_gameplay` reads its rotation, size or alpha back - so drawing it
//! from the simulation's own seeded stream would advance that stream once
//! per live round per tick for a value the simulation never uses, moving
//! every committed determinism hash for a purely cosmetic reason. This
//! hash is seeded from the round's own pool slot and the render tick, both
//! already stable per frame, so the same round on the same tick always
//! rolls the same flash - useful for a repeatable screenshot - without
//! touching anything the gate hashes. The exact PSP PRNG stream
//! (`Psys_RandFloatRange`/`Psys_RandIntRange`) is not reproduced: only the
//! *ranges* in `geometry.rs` are recovered, this generator's own sequence
//! is chosen, not measured.

use super::geometry::{FLASH_ALPHA_RANGE, FLASH_SIZE_RANGE, FLASH_SIZE_SCALE};

/// A cheap integer hash (xorshift-multiply, the "lowbias32" mix), good
/// enough to decorrelate three draws from one `(slot, tick)` pair without
/// pulling in a dependency for a render-only embellishment.
fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// `hash_u32` remapped to `[0, 1)`.
fn hash01(seed: u32) -> f32 {
    (hash_u32(seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// This tick's rotation (radians), half-size (world units, post-scale) and
/// alpha (`0..1`) for one live round's muzzle flash.
///
/// `slot` should be the round's own index in `Projectiles::slots` and
/// `tick` the simulation's own tick counter - both already carried on
/// `World`, so nothing new needs to be threaded through for repeatability.
#[must_use]
pub fn flash_roll(slot: u32, tick: u32) -> (f32, f32, f32) {
    ranged_flash_roll(slot, tick, FLASH_SIZE_RANGE)
}

/// [`flash_roll`] with a title's own pre-scale half-size range in place of
/// Pulse's [`FLASH_SIZE_RANGE`] - see `oag_title::weapons::CannonLook`.
#[must_use]
pub fn ranged_flash_roll(slot: u32, tick: u32, size_range: (f32, f32)) -> (f32, f32, f32) {
    let base = slot.wrapping_mul(0x9e37_79b1) ^ tick.wrapping_mul(0x85eb_ca6b);
    let rotation = hash01(base) * std::f32::consts::TAU;
    let size_t = hash01(base ^ 0x1);
    let half_size = (size_range.0 + size_t * (size_range.1 - size_range.0)) * FLASH_SIZE_SCALE;
    let alpha_t = hash01(base ^ 0x2);
    let (lo, hi) = (FLASH_ALPHA_RANGE.0 as f32, FLASH_ALPHA_RANGE.1 as f32);
    let alpha = (lo + alpha_t * (hi - lo)) / 255.0;
    (rotation, half_size, alpha)
}

/// One more roll from the same `(slot, tick)` seed as [`ranged_flash_roll`],
/// uniform in `range` - Wipeout HD's Z stretch of its muzzle-flash model,
/// rolled beside the flash's own size every tick of the window. Decorrelated
/// from the three draws above by its own seed salt; the sequence is chosen,
/// not measured, as the rest of this module's is.
#[must_use]
pub fn stretch_roll(slot: u32, tick: u32, range: (f32, f32)) -> f32 {
    let base = slot.wrapping_mul(0x9e37_79b1) ^ tick.wrapping_mul(0x85eb_ca6b);
    range.0 + hash01(base ^ 0x3) * (range.1 - range.0)
}
