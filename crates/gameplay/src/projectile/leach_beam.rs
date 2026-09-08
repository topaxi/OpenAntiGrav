//! The LeachBeam: a link held open between two craft, draining one into the
//! other for as long as it lasts.
//!
//! **Nothing here flies either.** Like [`super::quake`], this weapon has no
//! entry in [`super::Projectiles`]: `Weapon_FireLeachBeam` (`0x08866658`)
//! spawns no travelling body at all, it resolves a *link* to a craft the
//! Missile's own lock has already picked and then transfers energy along it
//! every tick. See
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` for the
//! whole reading and [`crate::World::leach_beam`] for where the single
//! instance this weapon ever has lives.
//!
//! # What is recovered and what is ours
//!
//! **Recovered.**
//!
//! - **One beam in the entire race at a time**, from `Weapon_FireLeachBeam`'s
//!   own `if (pool->live != 0) return;` on a world cursor rather than a
//!   per-craft cooldown - a stricter gate than any other weapon has.
//! - **That firing without a lock does nothing.** `LeachBeam_InitUnlocked`
//!   (`0x08872da8`) builds a `kind = 2` instance whose "target" is the shooter
//!   itself, and `LeachBeam_UpdatePool` (`0x08866b08`) gives kind 2 no distance
//!   test, no drain and no damage - only an expiry at
//!   [`UNLOCKED_FIZZLE_SECONDS`]. The pickup is spent and the player gets a cue.
//! - **The whole transfer**, from `LeachBeam_Drain` (`0x08866804`) and the two
//!   rate functions `LeachBeam_DrainRate` (`0x08872edc`) and
//!   `LeachBeam_RepairRate` (`0x08872f18`): the victim's
//!   [`LeachBeamStats::damage`] out and the shooter's
//!   [`LeachBeamStats::repair`] in, every tick, each multiplied once by
//!   [`LeachBeamStats::energy_multiplier`] on its own first tick.
//! - **Both accumulators' consumers**, which is what this weapon waited on:
//!   `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) spends the victim's through
//!   the ordinary `Ship_Damage` path, and `Ship_ApplyPendingWeaponRepair`
//!   (`0x0883f228`) spends the shooter's through `Ship_AddShield`
//!   (`0x0883ddc8`) straight into its own pool.
//! - **The lifetime**, [`LeachBeamStats::active_time`], from
//!   `LeachBeam_Advance` (`0x08873fa0`) returning `age < active_time`.
//! - **The disconnect gate and its linger**: past
//!   [`LeachBeamStats::range`], or against an invulnerable or shielded target,
//!   the link is marked disconnected rather than destroyed, and
//!   `LeachBeam_LingerExpired` (`0x088730a4`) retires it
//!   [`DISCONNECT_LINGER_SECONDS`] later.
//! - **That a shielded *shooter* suppresses both halves.** `LeachBeam_Drain`'s
//!   first line returns on the owner's own shield bit, before either credit.
//!
//! **Ours.**
//!
//! - **The drain runs every tick the link holds**, where the original gates each
//!   tick's drain on `LeachBeam_PulseStrength` (`0x08873020`) being positive -
//!   a value that ramps `0.0` to `2.0` over the second following each of the
//!   beam's roughly-once-a-second visual pulses, and is zero on the pulse tick
//!   itself. Since the pulses are about a second apart and the window is a
//!   second wide, the original drains on all but a scattering of ticks, and
//!   reproducing the scattering exactly needs the ribbon-scroll cursor that
//!   drives when a pulse fires - geometry this crate does not have and must not
//!   have. **Chosen, not measured**, and it is the one place this build
//!   knowingly transfers slightly more than the original per second.
//! - **[`LeachBeamStats::slow_ship_factor`] is parsed and not yet spent.** It is
//!   fully recovered - `LeachBeam_Drain` copies it onto the victim and
//!   `Ship_ApplyPendingWeaponDamage` writes it into the victim's handling record
//!   at `+0x31c`, the one-shot thrust scale `oag_physics::engine` documents and
//!   does not implement - but wiring it needs a new field on
//!   `oag_physics::ShipState`, which is a separate change with its own hash
//!   movement. Recorded here rather than approximated with a slowdown timer,
//!   which is a *different* mechanic this weapon deliberately does not use.
//! - **Per-tick, not per-second.** Neither rate function nor `LeachBeam_Drain`
//!   scales by `dt`, so the original's transfer is frame-rate dependent and this
//!   build's is tied to its own fixed 60 Hz (see
//!   `docs/architecture/adr/0007-fixed-timestep-vs-original.md`). Reproducing
//!   the instruction stream is the choice made here; a `dt`-scaled version would
//!   be a different number from the original at every frame rate rather than at
//!   one.

use crate::world::{MAX_SHIPS, Ship};
use oag_formats::weapons::LeachBeamStats;

/// Seconds a beam fired with no lock lasts before it gives up.
///
/// **Recovered, confidence 78.** `LeachBeam_UnlockedExpired` (`0x08873068`) is
/// the whole of it: `return 0.75 < instance->age;`, tested only on the
/// `kind == 2` arm of `LeachBeam_UpdatePool`. A fixed engine literal, not one of
/// [`LeachBeamStats`]'s nine attributes - in particular **not**
/// [`LeachBeamStats::active_time`], which governs a *locked* beam.
pub const UNLOCKED_FIZZLE_SECONDS: f32 = 0.75;

/// Seconds a disconnected beam lingers before the pool retires it.
///
/// **Recovered, confidence 80.** `LeachBeam_LingerExpired` (`0x088730a4`) is
/// `return instance->disconnected_at + 0.5 < instance->age;`. The original
/// marks a link disconnected rather than destroying it outright, precisely so
/// this window can run - which is what lets the beam's own visual fade instead
/// of vanishing between two frames.
pub const DISCONNECT_LINGER_SECONDS: f32 = 0.5;

/// What a fired beam is: locked onto somebody, or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `LeachBeam_InitLocked`'s `kind = 1` - a real link to another craft.
    Locked,
    /// `LeachBeam_InitUnlocked`'s `kind = 2`. Carries no target and does
    /// nothing at all; see [`UNLOCKED_FIZZLE_SECONDS`].
    Unlocked,
}

/// The single LeachBeam a race ever has in flight.
///
/// One instance for the whole race, matching `Weapon_FireLeachBeam`'s own pool
/// cursor - so [`crate::World`] carries `Option<Beam>` rather than a slot in
/// [`super::Projectiles`], exactly as it does for
/// [`super::quake::Wave`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beam {
    /// Which ship slot fired it.
    pub owner: u8,
    /// Which ship slot it is fastened to. Meaningless on [`Kind::Unlocked`],
    /// where it is set to [`Self::owner`] the way `LeachBeam_InitUnlocked`
    /// points the instance's target field at the shooter's own entity.
    pub target: u8,
    /// Locked onto a craft, or fired at nothing.
    pub kind: Kind,
    /// Seconds since it was fired. The original's `instance+0x134`, counted up
    /// by `LeachBeam_Advance`.
    pub age: f32,
    /// The age at which the link broke, or `None` while it still holds.
    ///
    /// The original's `instance+0x148`, written by
    /// `LeachBeam_MarkDisconnected` (`0x08873090`) alongside a flag at `+0x144`;
    /// one `Option` stands in for the pair, since the flag is exactly "has
    /// `+0x148` been written".
    pub disconnected_at: Option<f32>,
    /// Whether the victim's half of the transfer still owes its one-shot
    /// [`LeachBeamStats::energy_multiplier`]. The original's `instance+0x58`.
    pub first_drain: bool,
    /// The same for the shooter's half. The original's `instance+0x59`, a
    /// separate byte set and cleared independently - which is why this is two
    /// fields and not one.
    pub first_repair: bool,
    /// [`LeachBeamStats::damage`], copied at launch.
    ///
    /// Copied rather than looked up per tick for the reason
    /// [`super::quake::Wave`] gives about itself: only one instance exists at a
    /// time, and a `Beam` that carries its own numbers is a `Beam` a test can
    /// build without a disc.
    pub damage: f32,
    /// [`LeachBeamStats::repair`], copied at launch.
    pub repair: f32,
    /// [`LeachBeamStats::range`], copied at launch.
    pub range: f32,
    /// [`LeachBeamStats::active_time`], copied at launch.
    pub active_time: f32,
    /// [`LeachBeamStats::energy_multiplier`], copied at launch.
    pub energy_multiplier: f32,
}

/// What one tick of a beam did, for the composition root to draw and play.
///
/// A per-tick *output*, never state - the same rule
/// `docs/architecture/adr/0018-audio-mixer-architecture.md` sets for audio cues,
/// and for the same reason: a replay that recomputes the tick gets the same
/// report, and one that stored it could disagree with its own world.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// The link transferred energy this tick.
    pub drained: bool,
    /// The link broke this tick, having held at least one tick.
    pub disconnected: bool,
    /// The beam retired this tick and [`crate::World::leach_beam`] is now
    /// `None`.
    pub retired: bool,
    /// A hit landed on a target whose fired Shield pickup swallowed it - the
    /// only thing that makes the shell visibly react. The same out-parameter
    /// shape [`super::blast::blast`] and [`super::quake::Wave::apply_hits`]
    /// take, narrowed to one craft because a beam has exactly one victim.
    pub absorbed: bool,
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
        }
    }

    /// A beam fired with no lock, which will do nothing and expire.
    ///
    /// Kept rather than refused because the original spends the pickup and
    /// plays the cue either way - `Weapon_FireLeachBeam` branches on the lock
    /// *after* it has already claimed the pool slot and cleared the fire bit.
    /// A player who fires at nothing has fired.
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

    /// One tick: age the beam, test the link, move the energy, and say whether
    /// it is finished.
    ///
    /// Returns `true` when the beam should be retired - the caller sets
    /// [`crate::World::leach_beam`] to `None` on that. The order here is the
    /// original's own, from `LeachBeam_UpdatePool`: advance the age first (so
    /// the lifetime test sees this tick), then test the link, then drain, then
    /// retire.
    ///
    /// `absorbed`-style reporting rides on the returned [`Report`] rather than
    /// an out-parameter, because a beam has one victim and one shooter and there
    /// is nothing to index.
    pub fn advance(
        &mut self,
        ships: &mut [Ship; MAX_SHIPS],
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

        report.absorbed = self.drain(ships, rules);
        report.drained = true;
        report
    }

    /// Whether the link should break this tick.
    ///
    /// The five reasons, all from `LeachBeam_UpdatePool`'s own chain: the
    /// lifetime ran out, either craft left the race, the target put a Shield
    /// pickup up, or the two drifted further apart than [`Self::range`].
    ///
    /// **The target's invulnerability bit (`target+0x860 & 0x1000`) is not
    /// reproduced** - it is the respawn/rescue state, which this engine
    /// expresses through [`Ship::active`] and the craft state rather than a
    /// flag word, and the two are checked here in that form instead.
    fn link_broken(&self, ships: &[Ship; MAX_SHIPS], ship_count: u8) -> bool {
        if self.age >= self.active_time {
            return true;
        }
        let (Some(owner), Some(target)) = (
            slot(ships, ship_count, self.owner),
            slot(ships, ship_count, self.target),
        ) else {
            return true;
        };
        if !owner.active || !target.active {
            return true;
        }
        if target.physics.shield_pickup_timer > 0.0 {
            return true;
        }
        let separation = owner.physics.body.position - target.physics.body.position;
        separation.length() > self.range
    }

    /// One tick of the transfer. Returns whether the victim's shield swallowed
    /// it.
    ///
    /// **The shooter's own Shield pickup suppresses both halves**, matching
    /// `LeachBeam_Drain`'s first line - a craft that fires a beam and then
    /// raises a shield stops leaching, which reads as a bug and is what the
    /// instruction stream does.
    fn drain(&mut self, ships: &mut [Ship; MAX_SHIPS], rules: oag_physics::DamageRules) -> bool {
        let owner_index = self.owner as usize;
        let target_index = self.target as usize;
        if ships[owner_index].physics.shield_pickup_timer > 0.0 {
            return false;
        }

        let taken = take_once(&mut self.first_drain, self.damage, self.energy_multiplier);
        let target = &mut ships[target_index];
        let dimensions = target.handling.dimensions;
        let hit = oag_physics::damage::apply_weapon(&mut target.physics, &dimensions, taken, rules);

        let given = take_once(&mut self.first_repair, self.repair, self.energy_multiplier);
        let owner = &mut ships[owner_index];
        let dimensions = owner.handling.dimensions;
        oag_physics::damage::add(&mut owner.physics, &dimensions, given);

        hit.absorbed
    }
}

/// One half's amount for this tick, consuming its own first-tick flag.
///
/// Both rate functions are this, differing only in which offset and which flag
/// they read - so it is one helper and two call sites rather than two bodies.
fn take_once(first: &mut bool, rate: f32, multiplier: f32) -> f32 {
    if *first {
        *first = false;
        rate * multiplier
    } else {
        rate
    }
}

/// The ship in `index`, or `None` when the slot is past the field.
fn slot(ships: &[Ship; MAX_SHIPS], ship_count: u8, index: u8) -> Option<&Ship> {
    if index >= ship_count {
        return None;
    }
    ships.get(index as usize)
}

#[cfg(test)]
mod tests;
