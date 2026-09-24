//! What one craft's weapon hits did this tick, seen from the victim.
//!
//! The out-parameter every weapon-damage path in this module writes: the
//! blast, the Cannon's direct hit, the Quake's wave and the LeachBeam's drain.
//! It is a per-tick *output* of the step, never `World` state, so nothing in
//! it reaches a state hash.

/// One ship slot's weapon hits this tick. Set and never cleared within a tick,
/// so two hits in one tick are reported once each way rather than the second
/// overwriting the first.
///
/// A caller with nothing to draw passes a scratch array; a short slice is
/// written as far as it goes rather than panicking, so `&mut []` is a legal
/// "do not tell me".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WeaponHit {
    /// A fired Shield pickup swallowed a hit.
    ///
    /// The original's weapon-damage drain (`Ship_ApplyPendingWeaponDamage`,
    /// `0x0883f13c`) takes a shield branch that discards the amount and calls
    /// `ShipShield_Hit` instead, so a swallowed hit is the *only* thing that
    /// makes the shell visibly react - see
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    pub absorbed: bool,
    /// A hit got through: `Ship_Damage` (`0x088439ac`) ran its weapon branch
    /// (`source == 2`) on this craft - racing, unshielded, a positive amount.
    ///
    /// That branch is what throws the struck hull's damage sparks: one or two
    /// `ShipCollisionFx_Trigger` calls on random `Ship Collision Fx` locators,
    /// after the subtraction and after the destroyed test, so **the killing hit
    /// sparks too**. See `docs/ghidra/functions/psp-pulse-usa/shield.md`,
    /// "`Ship_Damage`'s weapon branch throws the hit sparks".
    pub landed: bool,
}

impl WeaponHit {
    /// Folds one `Ship_Damage` outcome into this slot's report.
    pub fn record(&mut self, report: &oag_physics::damage::Shield) {
        self.absorbed |= report.absorbed;
        self.landed |= report.landed();
    }
}

/// Folds `report` into `hits[slot]`, when the slice reaches that far.
pub(crate) fn record(hits: &mut [WeaponHit], slot: usize, report: &oag_physics::damage::Shield) {
    if let Some(hit) = hits.get_mut(slot) {
        hit.record(report);
    }
}
