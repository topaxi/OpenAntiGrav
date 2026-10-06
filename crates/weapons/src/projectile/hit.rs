//! What one craft's weapon hits did this tick, seen from the victim: the
//! out-parameter of the blast, the Cannon's direct hit, the Quake's wave and the
//! LeachBeam's drain. A per-tick output, never `World` state, so it reaches no
//! state hash.

/// One ship slot's weapon hits this tick. Set and never cleared within a tick, so
/// two hits are both reported. A caller with nothing to draw passes a scratch
/// array; a short slice is written as far as it goes, so `&mut []` is a legal "do
/// not tell me".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WeaponHit {
    /// A fired Shield pickup swallowed a hit. The original's
    /// `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) discards the amount and calls
    /// `ShipShield_Hit`, the only thing that makes the shell react; see
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    pub absorbed: bool,
    /// A hit got through: `Ship_Damage` (`0x088439ac`) ran its weapon branch
    /// (`source == 2`) on a racing, unshielded craft with a positive amount. That
    /// branch throws the hull's damage sparks (`ShipCollisionFx_Trigger` on random
    /// `Ship Collision Fx` locators) after the subtraction and the destroyed test,
    /// so the killing hit sparks too. See `docs/ghidra/functions/psp-pulse-usa/shield.md`,
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
