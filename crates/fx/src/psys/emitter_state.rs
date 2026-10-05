//! One running emitter - split out of `psys.rs`, which is past the 1,000-line rule
//! and ratcheted.

use oag_core::math::Vec3;

/// One running emitter: which spec, how much longer, and where it is.
///
/// A child instance carries its own anchor and drift rather than a
/// reference to the parent particle that spawned it: the parent's slot can
/// be recycled under it, and an index into a pool that reuses slots is the
/// kind of aliasing that produces an effect anchored to the wrong thing.
/// The drift is the parent's velocity at spawn, integrated linearly - the
/// parent's own drag is not reapplied, so a fast, heavily damped parent
/// drags its child slightly too far.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EmitterState {
    pub(super) spec: u16,
    pub(super) ticks_left: f32,
    /// Ticks until the next emission; `<= 0` means "due now", matching the
    /// original's countdown at `instance + 0x04`, which emits on its very
    /// first update.
    pub(super) until_next: f32,
    pub(super) anchor: Vec3,
    pub(super) drift: Vec3,
    /// Velocity added to every particle this emitter spawns - a child
    /// inherits its parent particle's, scaled by the child's own
    /// `+0x4d0`.
    pub(super) inherited: Vec3,
    /// Whether this instance rides its own drift (a child) or the caller's
    /// anchor (a root).
    pub(super) is_child: bool,
    pub(super) active: bool,
}

impl EmitterState {
    pub(super) const IDLE: Self = Self {
        spec: 0,
        ticks_left: 0.0,
        until_next: 0.0,
        anchor: Vec3::ZERO,
        drift: Vec3::ZERO,
        inherited: Vec3::ZERO,
        is_child: false,
        active: false,
    };
}
