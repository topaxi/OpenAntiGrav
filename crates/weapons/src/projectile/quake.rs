//! The Quake: a single travelling point on the track's own path, not a projectile.
//!
//! Nothing here flies. `Weapon_FireQuake` (`0x0886c600`) gives the wave no
//! position or velocity: it locates the firing craft on the track's spline and
//! stores a travelling distance. See
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`; the single
//! instance lives in `oag_gameplay::World::quake`.
//!
//! # What is recovered and what is ours
//!
//! **Recovered.** The wave advances at a fixed, unauthored `270.0` units a second
//! along the spline ([`SPEED_UNITS_PER_SECOND`]), in a direction chosen once at
//! launch by dotting the craft's forward against the track tangent. The hit and
//! slowdown half reuses the shared pending-hit channel the Missile and Mine/Bomb
//! spend (`entity+0x130`/`+0x138`/`+0x13c`), gated on a per-craft latch
//! (`entity+0x860 & 0x40`) found 2026-09-07 to be a smoothed-proximity test; see
//! [`Self::progress_delta`] and the [`radius`
//! substitution](oag_tables::weapons::QuakeStats::radius).
//!
//! **Ours.** The wave's position is one `f32` distance along the course, in
//! [`oag_race::Standing::progress`]'s units, not the original's segment index plus
//! parametric `t`; equivalent for a closed ring and no new `World` state.
//! `crates/raceplay/src/weapons.rs::advance_quake` recovers the world position for
//! the visual (it needs [`oag_race::Course`], which this crate cannot hold).
//! **The smoothed-proximity debounce is not reproduced: chosen, not measured.**
//! This uses a flat in/out-of-`radius` test on circular distance, edge-triggered on
//! [`Wave::hit`] like the original's latch. The original's analogue quantity
//! crosses `0.1` at an unauthored `8.0` per second and needs the span-table fields
//! (`Quake_SpanIntensityAt`'s `+0x5c`/`+0x64`/`+0x6c`) read only at shape level.

use crate::{Craft, MAX_SHIPS};
use oag_tables::weapons::QuakeStats;

/// World units of track the wave crosses every second.
///
/// **Recovered, confidence 85**
/// (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
/// "`Quake_Update` is the missing per-frame advance"). `Quake_Update`
/// (`0x0891d268`) advances a parametric position at `270.0 * dt / <per-launch
/// value>`, but the same `270.0` is also spent as `ABS(270.0) * dt` against
/// Euclidean segment lengths, a plain world-distance rate, which this takes. A
/// fixed engine literal, not one of [`QuakeStats`]'s four attributes.
pub const SPEED_UNITS_PER_SECOND: f32 = 270.0;

/// Seconds a fired Quake lives, measured from launch.
///
/// **Recovered, confidence 85** (same page, "The lifetime is 5.0 seconds from
/// launch, and the age is inherited"). Each road span record ages by `dt` and
/// sets its retire byte (`+0x7d`) the frame `age + dt` exceeds `5.0` (`lui a0,
/// 0x40a0` at `0x0891cafc`, read at instruction level). It is one wave lifetime
/// because `Quake_ArmSpan` (`0x0891b714`) copies the caller's age into the new span
/// (`+0x74 = param_2`) and both propagation functions pass the parent's `+0x74`
/// along, from `Quake_Init`'s `0`. A fixed engine literal, not authored.
pub const LIFETIME_SECONDS: f32 = 5.0;

/// The single travelling wave a fired Quake ever has, matching `Weapon_FireQuake`'s
/// single-slot pool (same page), so `oag_gameplay::World` carries `Option<Wave>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wave {
    /// Which ship slot fired it. Excluded from every hit test: the original clears
    /// the shooter's latch outright.
    pub owner: u8,
    /// Distance from the course's start line, in `oag_race::Standing::progress`'s
    /// units: `0.0..course.length()`, wrapping.
    pub progress: f32,
    /// Which way around the ring: `1.0` toward increasing progress, `-1.0`
    /// decreasing. Fixed at launch, as `Quake_Init`'s one-shot dot product.
    pub direction: f32,
    /// Energy the wave costs a craft it passes, once. Copied from
    /// [`QuakeStats::damage`] at launch; only one wave exists at a time.
    pub damage: f32,
    /// The wave's proximity gate, see [`QuakeStats::radius`].
    pub radius: f32,
    /// Seconds of slowdown the wave charges a craft it passes over, once.
    pub slowdown_time: f32,
    /// Per-slot latch: whether this craft is inside [`Self::radius`], so a hit is
    /// credited on the rising edge only. Mirrors `entity+0x860 & 0x40`.
    pub hit: [bool; MAX_SHIPS],
    /// Seconds since launch, against [`LIFETIME_SECONDS`].
    ///
    /// The one piece of per-wave state the original keeps per road span; the spans
    /// inherit each other's age (see [`LIFETIME_SECONDS`]), so one scalar is the
    /// same clock.
    pub age: f32,
}

impl Wave {
    /// A freshly launched wave at the firing craft's progress.
    ///
    /// `forward_dot_tangent` (craft forward against track tangent) is the caller's
    /// job: only `crates/raceplay/src` holds a `Course`. Only its sign is used;
    /// exactly `0.0` takes the positive direction rather than stall. **Chosen, not
    /// measured**: `Quake_Init`'s tie-break there is not established.
    #[must_use]
    pub fn launch(owner: u8, progress: f32, forward_dot_tangent: f32, stats: &QuakeStats) -> Self {
        Self {
            owner,
            progress,
            direction: if forward_dot_tangent < 0.0 { -1.0 } else { 1.0 },
            damage: stats.damage,
            radius: stats.radius,
            slowdown_time: stats.slowdown_time,
            hit: [false; MAX_SHIPS],
            age: 0.0,
        }
    }

    /// Advances the wave one tick along a course of this `length`, wrapping, and
    /// ages it. The age advances on every path: a non-positive length leaves
    /// [`Self::progress`] alone, but a wave that cannot move must still expire or
    /// a second Quake can never be fired.
    pub fn advance(&mut self, length: f32, dt: f32) {
        self.age += dt;
        if length <= 0.0 {
            return;
        }
        self.progress =
            (self.progress + self.direction * SPEED_UNITS_PER_SECOND * dt).rem_euclid(length);
    }

    /// Whether the wave has outlived [`LIFETIME_SECONDS`]. Read after
    /// [`Self::advance`] and before [`Self::apply_hits`], so the expiring tick lands
    /// no hit, as the span's retire byte (`+0x7d`) short-circuits ripple and
    /// propagation.
    #[must_use]
    pub fn expired(&self) -> bool {
        self.age > LIFETIME_SECONDS
    }

    /// The shortest distance around a ring of this `length` between the wave and
    /// `other`: a ring wraps, so the straight difference overstates it across the
    /// start line.
    #[must_use]
    pub fn progress_delta(&self, other: f32, length: f32) -> f32 {
        let raw = (other - self.progress).abs();
        if length > 0.0 {
            raw.min(length - raw)
        } else {
            raw
        }
    }

    /// One tick of the wave's hit test against every active ship.
    ///
    /// The owner is excluded outright, as the original's `craft_is_shooter`
    /// short-circuit (what `FUN_088418e0`'s block branches on). A ship with no
    /// [`oag_race::Standing::progress`] yet is skipped. Shielded craft take no hit
    /// and their latch clears, as the original's shield branch (`entity+0x1b8 &
    /// 0x10`) clears the bit. `hits[slot]` records absorbed or landed, see
    /// [`super::WeaponHit`], as for [`super::blast::blast`] and
    /// [`super::cannon::direct_hit`].
    pub fn apply_hits<S: Craft>(
        &mut self,
        ships: &mut [S; MAX_SHIPS],
        ship_count: u8,
        length: f32,
        rules: oag_physics::DamageRules,
        hits: &mut [super::WeaponHit],
    ) {
        for (slot, ship) in ships.iter_mut().enumerate().take(ship_count as usize) {
            if slot as u8 == self.owner {
                self.hit[slot] = false;
                continue;
            }
            if !ship.active() {
                continue;
            }
            let Some(progress) = ship.progress() else {
                continue;
            };
            let shielded = ship.physics().shield_pickup_timer > 0.0;
            let within = self.progress_delta(progress, length) <= self.radius;
            if within && !shielded {
                if !self.hit[slot] {
                    *ship.pending_slowdown_mut() += self.slowdown_time;
                    let dimensions = ship.dimensions();
                    let report = oag_physics::damage::apply_weapon(
                        ship.physics_mut(),
                        &dimensions,
                        self.damage,
                        rules,
                    );
                    super::hit::record(hits, slot, &report);
                }
                self.hit[slot] = true;
            } else {
                self.hit[slot] = false;
            }
        }
    }
}

#[cfg(test)]
mod tests;
