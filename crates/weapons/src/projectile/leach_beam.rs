//! The LeachBeam: a link held open between two craft, draining one into the other.
//!
//! Nothing here flies. Like [`super::quake`], it has no entry in
//! [`super::Projectiles`]: `Weapon_FireLeachBeam` (`0x08866658`) spawns no body, it
//! resolves a link to a craft the Missile's lock picked and transfers energy each
//! tick. See `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`;
//! the single instance lives in `oag_gameplay::World::leach_beam`.
//!
//! # What is recovered and what is ours
//!
//! **Recovered.**
//!
//! - One beam in the whole race: `Weapon_FireLeachBeam`'s
//!   `if (pool->live != 0) return;` on a world cursor.
//! - Firing without a lock does nothing: `LeachBeam_InitUnlocked` (`0x08872da8`)
//!   builds `kind = 2` targeting the shooter, and `LeachBeam_UpdatePool`
//!   (`0x08866b08`) gives it no distance test, drain or damage, only expiry at
//!   [`UNLOCKED_FIZZLE_SECONDS`]. The pickup is spent and a cue plays.
//! - The transfer: `LeachBeam_Drain` (`0x08866804`), `LeachBeam_DrainRate`
//!   (`0x08872edc`), `LeachBeam_RepairRate` (`0x08872f18`): the victim's
//!   [`LeachBeamStats::damage`] out and the shooter's [`LeachBeamStats::repair`]
//!   in each tick, each times [`LeachBeamStats::energy_multiplier`] on its own
//!   first tick.
//! - Both accumulators' consumers: `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`)
//!   through `Ship_Damage`, `Ship_ApplyPendingWeaponRepair` (`0x0883f228`) through
//!   `Ship_AddShield` (`0x0883ddc8`).
//! - The lifetime [`LeachBeamStats::active_time`], from `LeachBeam_Advance`
//!   (`0x08873fa0`).
//! - Disconnect: past [`LeachBeamStats::range`], or against an invulnerable or
//!   shielded target, the link is marked disconnected and
//!   `LeachBeam_LingerExpired` (`0x088730a4`) retires it
//!   [`DISCONNECT_LINGER_SECONDS`] later.
//! - A shielded shooter suppresses both halves: `LeachBeam_Drain`'s first line.
//!
//! **Ours.**
//!
//! - **The drain runs every tick the link holds. Chosen, not measured.** The
//!   original gates each tick on `LeachBeam_PulseStrength` (`0x08873020`) being
//!   positive (ramps `0.0` to `2.0` over the second after each re-arm, zero on the
//!   re-arm tick), re-armed on the ribbon cursor's first wrap once a second has
//!   passed (1.17 s apart in play), so it stops draining up to `segment_count`
//!   ticks each second. That needs the ribbon cursor, which is render-side
//!   (`oag_fx::beam::Ribbon`). This build transfers slightly more per second.
//! - [`LeachBeamStats::slow_ship_factor`] is spent as the original does:
//!   `LeachBeam_Drain` copies it to the victim and `Ship_ApplyPendingWeaponDamage`
//!   writes it to `+0x31c`, the one-shot thrust scale `Ship_UpdateEngine` reads.
//!   [`Beam::drain`] arms `oag_gameplay::Ship::pending_thrust_scale` on each
//!   unabsorbed drain of a racing victim; the composition root hands it to
//!   `oag_physics::Environment::thrust_scale` next step. Not a slowdown timer
//!   (see `oag_physics::slowdown`). One tick of latency at each end is the
//!   recovered ordering.
//! - Per-tick, not per-second: neither rate function nor `LeachBeam_Drain` scales
//!   by `dt`, so the original is frame-rate dependent and this build is tied to
//!   60 Hz (`docs/architecture/adr/0007-fixed-timestep-vs-original.md`).

use crate::{Craft, MAX_SHIPS};
use oag_physics::damage::CraftState;
use oag_tables::weapons::LeachBeamStats;

/// Seconds a beam fired with no lock lasts before it gives up.
///
/// **Recovered, confidence 78.** `LeachBeam_UnlockedExpired` (`0x08873068`) is
/// `return 0.75 < instance->age;` on the `kind == 2` arm. A fixed engine literal,
/// not [`LeachBeamStats::active_time`], which governs a locked beam.
pub const UNLOCKED_FIZZLE_SECONDS: f32 = 0.75;

/// Seconds a disconnected beam lingers before the pool retires it.
///
/// **Recovered, confidence 80.** `LeachBeam_LingerExpired` (`0x088730a4`) is
/// `return instance->disconnected_at + 0.5 < instance->age;`, so the visual can
/// fade instead of vanishing.
pub const DISCONNECT_LINGER_SECONDS: f32 = 0.5;

/// What a fired beam is: locked onto somebody, or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `LeachBeam_InitLocked`'s `kind = 1`: a real link to another craft.
    Locked,
    /// `LeachBeam_InitUnlocked`'s `kind = 2`: no target, does nothing; see
    /// [`UNLOCKED_FIZZLE_SECONDS`].
    Unlocked,
}

/// The single LeachBeam a race ever has in flight, matching
/// `Weapon_FireLeachBeam`'s pool cursor, so `oag_gameplay::World` carries
/// `Option<Beam>` rather than a [`super::Projectiles`] slot, as for
/// [`super::quake::Wave`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beam {
    /// Which ship slot fired it.
    pub owner: u8,
    /// Which ship slot it is fastened to. On [`Kind::Unlocked`] it is
    /// [`Self::owner`], as `LeachBeam_InitUnlocked` targets the shooter.
    pub target: u8,
    /// Locked onto a craft, or fired at nothing.
    pub kind: Kind,
    /// Seconds since it was fired: the original's `instance+0x134`.
    pub age: f32,
    /// The age at which the link broke, or `None`. The original's `instance+0x148`
    /// from `LeachBeam_MarkDisconnected` (`0x08873090`) plus a flag at `+0x144`,
    /// which one `Option` replaces.
    pub disconnected_at: Option<f32>,
    /// Whether the victim's half still owes its one-shot
    /// [`LeachBeamStats::energy_multiplier`]: the original's `instance+0x58`.
    pub first_drain: bool,
    /// The same for the shooter's half, a separate byte (`instance+0x59`).
    pub first_repair: bool,
    /// [`LeachBeamStats::damage`], copied at launch (as [`super::quake::Wave`]
    /// does) so a `Beam` can be built in a test without a disc.
    pub damage: f32,
    /// [`LeachBeamStats::repair`], copied at launch.
    pub repair: f32,
    /// [`LeachBeamStats::range`], copied at launch.
    pub range: f32,
    /// [`LeachBeamStats::active_time`], copied at launch.
    pub active_time: f32,
    /// [`LeachBeamStats::energy_multiplier`], copied at launch.
    pub energy_multiplier: f32,
    /// [`LeachBeamStats::slow_ship_factor`], copied at launch: the one-shot
    /// thrust scale [`Self::drain`] arms on the victim each tick it lands.
    pub slow_ship_factor: f32,
}

/// What one tick of a beam did, for the composition root to draw and play: a
/// per-tick output, never state (`docs/architecture/adr/0018-audio-mixer-architecture.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// The link transferred energy this tick.
    pub drained: bool,
    /// The link broke this tick, having held at least one tick.
    pub disconnected: bool,
    /// The beam retired this tick; `oag_gameplay::World::leach_beam` is now `None`.
    pub retired: bool,
    /// A hit landed on a target whose Shield pickup swallowed it: the only thing
    /// that makes the shell react. The same out-parameter shape as
    /// [`super::blast::blast`], narrowed to the one victim.
    pub absorbed: bool,
    /// A drain got through, see [`super::WeaponHit::landed`]. The LeachBeam is
    /// `craft+0x138 == 7`, the one weapon throwing
    /// `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` instead of `WO_SHIP_COLL_SPARK_DAMAGE`.
    pub landed: bool,
}

impl Beam {
    /// A beam fired with a lock, onto `target`.
    #[must_use]
    pub fn locked(owner: u8, target: u8, stats: &LeachBeamStats) -> Self {
        Self {
            owner,
            target,
            kind: Kind::Locked,
            age: 0.0,
            disconnected_at: None,
            first_drain: true,
            first_repair: true,
            damage: stats.damage,
            repair: stats.repair,
            range: stats.range,
            active_time: stats.active_time,
            energy_multiplier: stats.energy_multiplier,
            slow_ship_factor: stats.slow_ship_factor,
        }
    }

    /// A beam fired with no lock: does nothing and expires. Kept because
    /// `Weapon_FireLeachBeam` branches on the lock after claiming the pool slot
    /// and clearing the fire bit, so the pickup is spent either way.
    #[must_use]
    pub fn unlocked(owner: u8, stats: &LeachBeamStats) -> Self {
        Self {
            kind: Kind::Unlocked,
            ..Self::locked(owner, owner, stats)
        }
    }

    /// Whether this beam is still transferring, as of its current age.
    #[must_use]
    pub fn connected(&self) -> bool {
        self.kind == Kind::Locked && self.disconnected_at.is_none()
    }

    /// One tick: age the beam, test the link, move the energy, say whether it is
    /// finished.
    ///
    /// Returns `true`-style via [`Report::retired`] when the caller should set
    /// `oag_gameplay::World::leach_beam` to `None`. The order is
    /// `LeachBeam_UpdatePool`'s: age, link test, drain, retire.
    pub fn advance<S: Craft>(
        &mut self,
        ships: &mut [S; MAX_SHIPS],
        ship_count: u8,
        rules: oag_physics::DamageRules,
        dt: f32,
    ) -> Report {
        let mut report = Report::default();
        self.age += dt;

        if self.kind == Kind::Unlocked {
            report.retired = self.age >= UNLOCKED_FIZZLE_SECONDS;
            return report;
        }

        if let Some(broken_at) = self.disconnected_at {
            report.retired = self.age >= broken_at + DISCONNECT_LINGER_SECONDS;
            return report;
        }

        if self.link_broken(ships, ship_count) {
            self.disconnected_at = Some(self.age);
            report.disconnected = true;
            return report;
        }

        let hit = self.drain(ships, rules);
        report.absorbed = hit.absorbed;
        report.landed = hit.landed();
        report.drained = true;
        report
    }

    /// Whether the link should break this tick, per `LeachBeam_UpdatePool`'s
    /// chain: lifetime out, either craft left the race, the target shielded,
    /// either craft not in `Ship_State` 1 (racing), or further apart than
    /// [`Self::range`]. The original also breaks on a positive `entity+0x120`
    /// leach accumulator; one beam per race means nothing writes it.
    ///
    /// The target's invulnerability bit (`target+0x860 & 0x1000`) is not
    /// reproduced: it is the respawn/rescue state, checked here through
    /// `oag_gameplay::Ship::active` and the craft state.
    fn link_broken<S: Craft>(&self, ships: &[S; MAX_SHIPS], ship_count: u8) -> bool {
        if self.age >= self.active_time {
            return true;
        }
        let (Some(owner), Some(target)) = (
            slot(ships, ship_count, self.owner),
            slot(ships, ship_count, self.target),
        ) else {
            return true;
        };
        if !owner.active() || !target.active() {
            return true;
        }
        if target.physics().shield_pickup_timer > 0.0 {
            return true;
        }
        // A craft that is exploding, eliminated or waiting to respawn lets go; a
        // broken link never re-forms, so a respawned target is not drained.
        if owner.physics().craft_state != CraftState::Racing
            || target.physics().craft_state != CraftState::Racing
        {
            return true;
        }
        let separation = owner.physics().body.position - target.physics().body.position;
        separation.length() > self.range
    }

    /// One tick of the transfer; returns what `Ship_Damage` did to the victim.
    ///
    /// The shooter's own Shield pickup suppresses both halves, as
    /// `LeachBeam_Drain`'s first line does.
    fn drain<S: Craft>(
        &mut self,
        ships: &mut [S; MAX_SHIPS],
        rules: oag_physics::DamageRules,
    ) -> oag_physics::damage::Shield {
        let owner_index = self.owner as usize;
        let target_index = self.target as usize;
        if ships[owner_index].physics().shield_pickup_timer > 0.0 {
            return oag_physics::damage::Shield::default();
        }

        let taken = take_once(&mut self.first_drain, self.damage, self.energy_multiplier);
        let target = &mut ships[target_index];
        let dimensions = target.dimensions();
        let hit =
            oag_physics::damage::apply_weapon(target.physics_mut(), &dimensions, taken, rules);
        // `Ship_ApplyPendingWeaponDamage`'s `kind == 7` arm: in the no-shield
        // branch, behind `pending > 0` and racing, the victim's `craft+0x31c`
        // takes `slowShipFactor`; see `Ship::pending_thrust_scale`.
        if taken > 0.0
            && !hit.absorbed
            && target.physics().craft_state == oag_physics::damage::CraftState::Racing
        {
            *target.pending_thrust_scale_mut() = self.slow_ship_factor;
        }

        let given = take_once(&mut self.first_repair, self.repair, self.energy_multiplier);
        let owner = &mut ships[owner_index];
        let dimensions = owner.dimensions();
        oag_physics::damage::add(owner.physics_mut(), &dimensions, given);

        hit
    }
}

/// One half's amount for this tick, consuming its own first-tick flag. Both rate
/// functions are this, differing in offset and flag.
fn take_once(first: &mut bool, rate: f32, multiplier: f32) -> f32 {
    if *first {
        *first = false;
        rate * multiplier
    } else {
        rate
    }
}

/// The ship in `index`, or `None` when the slot is past the field.
fn slot<S: Craft>(ships: &[S; MAX_SHIPS], ship_count: u8, index: u8) -> Option<&S> {
    if index >= ship_count {
        return None;
    }
    ships.get(index as usize)
}

#[cfg(test)]
mod tests;
