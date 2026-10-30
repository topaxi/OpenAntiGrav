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
//! spend (`entity+0x130`/`+0x138`/`+0x13c`).
//!
//! **The hit gate is the craft's smoothed wave height (confidence 88-92, measured
//! 2026-10-26).** Every craft carries `h` (`entity+0x870`), integrated
//! `h += (raw - h) * 8 * dt` every tick whether or not a wave is in flight
//! ([`smooth_lift`]), where `raw` is the wave's [`amplitude`] while the craft is
//! within half of [`half_width`] of the bump's centre ([`Wave::raw_lift`]). The
//! hit lands on the rising edge of `h > 0.1` ([`LIFT_HIT_THRESHOLD`]) and the
//! per-craft latch (`entity+0x860 & 0x40`) clears when `h` falls back. A laid
//! mine trips on the raw height `> 0.1`. The cockpit eye rises by `h`. There is
//! no kick and no body ride: the original does neither. See
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "2026-10-26:
//! what a Quake does to the craft it reaches". [`QuakeStats::radius`] is not read
//! by the hit path any more (it superseded the 2026-09-07 reading that it was the
//! gate).
//!
//! **Ours.** The wave's position is one `f32` distance along the course, in
//! [`oag_race::Standing::progress`]'s units, not the original's segment index plus
//! parametric `t`; equivalent for a closed ring and no new `World` state.
//! `crates/raceplay/src/weapons.rs::advance_quake` recovers the world position for
//! the visual (it needs [`oag_race::Course`], which this crate cannot hold). The
//! original matches the craft against the wave per road span; this uses distance
//! along the course, which is the same to first order because the spans are
//! distance-exact. **Chosen, not measured:** a craft on a split's other branch is
//! treated by course distance too, where the original compares span segments.

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

/// The bump's height at its peak, in world units.
///
/// `g_quake_peak_amplitude`, `0x08a88540`: `12.0f`, reached through a
/// relocated `lui`/`lwc1` pair in `Quake_SpanAmplitude` (`0x0891b600`).
/// Confidence 90.
pub const PEAK_AMPLITUDE: f32 = 12.0;

/// Seconds from launch to the peak: a literal in `Quake_SpanAmplitude`.
pub const RISE_SECONDS: f32 = 0.3;

/// Half the bump's width at launch, in world units: `Quake_SpanHalfWidth`
/// (`0x0891b654`) is `age * 50 / 5 + 25`.
pub const HALF_WIDTH_AT_LAUNCH: f32 = 25.0;

/// How much wider the bump gets over [`LIFETIME_SECONDS`] - the `50` in
/// `age * 50 / 5`.
pub const HALF_WIDTH_GROWTH: f32 = 50.0;

/// How far ahead of the firing craft the bump is born, in the direction it
/// travels: `g_quake_launch_lead`, `0x08a7ccac`, `15.0f`, which `Quake_Init`
/// adds to the launch span's position. Confidence 80.
pub const LAUNCH_LEAD: f32 = 15.0;

/// The bump's height at `age` seconds after launch.
///
/// A linear rise to [`PEAK_AMPLITUDE`] over [`RISE_SECONDS`], then a linear fall
/// that reaches zero at [`LIFETIME_SECONDS`]. Zero outside `0..LIFETIME_SECONDS`
/// rather than extrapolated: the original retires every span there.
#[must_use]
pub fn amplitude(age: f32) -> f32 {
    if !(0.0..LIFETIME_SECONDS).contains(&age) {
        0.0
    } else if age < RISE_SECONDS {
        PEAK_AMPLITUDE * age / RISE_SECONDS
    } else {
        PEAK_AMPLITUDE - PEAK_AMPLITUDE * (age - RISE_SECONDS) / (LIFETIME_SECONDS - RISE_SECONDS)
    }
}

/// Half the bump's width at `age` seconds after launch.
#[must_use]
pub fn half_width(age: f32) -> f32 {
    age * HALF_WIDTH_GROWTH / LIFETIME_SECONDS + HALF_WIDTH_AT_LAUNCH
}

/// How fast the craft's wave height `h` chases the raw height, per second.
///
/// The `8.0` in `h += (raw - h) * 8 * dt`. Confidence 88
/// (`cannon-quake-leachbeam.md`, "2026-10-26").
pub const LIFT_RATE: f32 = 8.0;

/// The wave height above which a craft is hit (on the rising edge of the
/// smoothed height) and a laid mine trips (on the raw height).
///
/// Confidence 88, same page.
pub const LIFT_HIT_THRESHOLD: f32 = 0.1;

/// One tick of a craft's smoothed wave height towards `raw`.
///
/// Written out as the original does, `h + (raw - h) * 8 * dt`, so the float
/// operations are in a fixed order (`oag-weapons` is bound by the determinism
/// rules: no `mul_add`, no reassociation).
#[must_use]
pub fn smooth_lift(h: f32, raw: f32, dt: f32) -> f32 {
    h + (raw - h) * LIFT_RATE * dt
}

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
    /// Seconds of slowdown the wave charges a craft it passes over, once.
    pub slowdown_time: f32,
    /// Per-slot latch: whether this craft's smoothed wave height is above
    /// [`LIFT_HIT_THRESHOLD`], so a hit is credited on the rising edge only.
    /// Mirrors `entity+0x860 & 0x40`.
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

    /// The raw wave height at a point `progress` along a course of this
    /// `length`: [`amplitude`] while the point is within half of [`half_width`]
    /// of the bump's centre, otherwise `0.0`.
    ///
    /// The bump sits [`LAUNCH_LEAD`] ahead of [`Self::progress`], and both
    /// helpers are evaluated at the wave's age plus one `dt`, the lag
    /// `Quake_UpdateSpan` stores them at (and the renderer draws the road with,
    /// `oag_render::ripple::Wave::at`). Read after [`Self::advance`]. The distance
    /// is the shortest way round the ring, which wraps across the start line.
    #[must_use]
    pub fn raw_lift(&self, progress: f32, length: f32, dt: f32) -> f32 {
        let lagged = self.age + dt;
        let centre = self.progress + self.direction * LAUNCH_LEAD;
        let raw = (progress - centre).abs();
        let distance = if length > 0.0 {
            let wrapped = raw.rem_euclid(length);
            wrapped.min(length - wrapped)
        } else {
            raw
        };
        if distance < half_width(lagged) * 0.5 {
            amplitude(lagged)
        } else {
            0.0
        }
    }

    /// One tick of every craft's smoothed wave height, `wave` being the wave in
    /// flight if there is one.
    ///
    /// Runs for every craft every tick, **wave or none**: with no wave the raw
    /// height is `0.0` and `h` decays at [`LIFT_RATE`], which is how it keeps
    /// falling after an expired wave is dropped. A slot that is inactive or not
    /// yet located is raw `0.0` too.
    pub fn update_lift<S: Craft>(
        wave: Option<&Self>,
        ships: &mut [S; MAX_SHIPS],
        ship_count: u8,
        length: f32,
        dt: f32,
    ) {
        for ship in ships.iter_mut().take(ship_count as usize) {
            let raw = match (wave, ship.progress()) {
                (Some(wave), Some(progress)) if ship.active() => {
                    wave.raw_lift(progress, length, dt)
                }
                _ => 0.0,
            };
            let lift = ship.quake_lift_mut();
            *lift = smooth_lift(*lift, raw, dt);
        }
    }

    /// One tick of the wave's hit test against every active ship, after
    /// [`Self::update_lift`].
    ///
    /// The owner is excluded outright, as the original's `craft_is_shooter`
    /// short-circuit (what `FUN_088418e0`'s block branches on), and so is a craft
    /// whose smoothed height has not passed [`LIFT_HIT_THRESHOLD`]: both clear the
    /// latch. Shielded craft take no hit and their latch clears, as the original's
    /// shield branch (`entity+0x1b8 & 0x10`) clears the bit. `hits[slot]` records
    /// absorbed or landed, see [`super::WeaponHit`], as for [`super::blast::blast`]
    /// and [`super::cannon::direct_hit`].
    pub fn apply_hits<S: Craft>(
        &mut self,
        ships: &mut [S; MAX_SHIPS],
        ship_count: u8,
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
            let shielded = ship.physics().shield_pickup_timer > 0.0;
            let lifted = ship.quake_lift() > LIFT_HIT_THRESHOLD;
            if lifted && !shielded {
                if !self.hit[slot] {
                    *ship.pending_slowdown_mut() += self.slowdown_time;
                    let dimensions = ship.dimensions();
                    let report = oag_physics::damage::apply_weapon(
                        ship.physics_mut(),
                        &dimensions,
                        self.damage,
                        rules,
                    );
                    super::hit::record(hits, slot, oag_tables::weapons::Weapon::Quake, &report);
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
