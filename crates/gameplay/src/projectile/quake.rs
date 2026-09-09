//! The Quake: a single travelling point on the track's own path, not a
//! projectile at all.
//!
//! **Nothing here flies.** Every other module in [`super`] shares
//! [`super::Projectiles`]'s array and its floor-following flight; the Quake
//! has neither, because the original's own `Weapon_FireQuake` (`0x0886c600`)
//! never gives it a position or a velocity - it locates the firing craft on
//! the track's own spline and stores a travelling *distance*, not a point in
//! space. See
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` for the
//! whole reading and [`crate::World::quake`] for where the single instance
//! this weapon ever has lives.
//!
//! # What is recovered and what is ours
//!
//! **Recovered.** That the wave advances at a fixed, unauthored
//! `270.0` units a second along the track's own spline
//! ([`SPEED_UNITS_PER_SECOND`]), in a direction chosen once at launch by
//! dotting the firing craft's forward against the track's own tangent there.
//! That the hit/slowdown half reuses the same shared pending-hit channel the
//! Missile and the Mine/Bomb already spend (`entity+0x130`/`+0x138`/`+0x13c`),
//! gated on a per-craft latch (`entity+0x860 & 0x40`) that this project found
//! 2026-09-07 to be a smoothed-proximity test - see [`Self::progress_delta`]'s
//! doc comment for the recovered mechanism and the authored [`radius`
//! substitution](oag_tables::weapons::QuakeStats::radius) this port makes
//! for it.
//!
//! **Ours.** The exact representation: this crate tracks the wave's position
//! as a single `f32` distance-along-the-course, in the same units and the
//! same convention [`oag_race::Standing::progress`] already gives every
//! craft, rather than the original's segment-index-plus-parametric-`t` pair.
//! The two are equivalent for a closed ring and this shape needs no new
//! per-tick state on [`crate::World`] beyond what standings already compute -
//! see `crates/game/src/race/weapons.rs::advance_quake` for where the wave's
//! own world position is recovered from this distance for the visual, which
//! needs [`oag_race::Course`] and therefore cannot live in this crate at all
//! (`oag-gameplay` draws nothing - see `docs/architecture/adr/0003-no-ecs.md`'s
//! neighbour, the "simulation must not know a renderer exists" rule in
//! `CLAUDE.md`). **The original's own smoothed-proximity debounce is not
//! reproduced bit for bit** - this uses a flat in/out-of-`radius` test on the
//! circular distance between the wave's own progress and the craft's own
//! [`oag_race::Standing::progress`], edge-triggered on [`Wave::hit`] the same
//! way the original's latch is edge-triggered on `entity+0x860 & 0x40`. Both
//! are a "the wave passed over this craft, once" rule; the original's is a
//! smoothed analogue quantity crossing `0.1` at an unauthored `8.0`-per-second
//! rate, and reproducing that exactly needs the two span-table fields
//! (`Quake_SpanIntensityAt`'s own `+0x5c`/`+0x64`/`+0x6c`) this project has
//! only read at the shape level. **Chosen, not measured.**

use crate::world::{MAX_SHIPS, Ship};
use oag_tables::weapons::QuakeStats;

/// World units of track the wave crosses every second.
///
/// **Recovered, confidence 85** (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
/// "`Quake_Update` is the missing per-frame advance"). `Quake_Update`
/// (`0x0891d268`) advances a parametric position at `270.0 * dt /
/// <a per-launch value>`, but the *same* `270.0` is independently spent a
/// second way in the same function, directly as `ABS(270.0) * dt` against
/// Euclidean segment lengths - a plain world-distance rate, which is the
/// reading this constant takes rather than the parametric one. Not authored:
/// it is a fixed engine literal, not one of [`QuakeStats`]'s four attributes.
pub const SPEED_UNITS_PER_SECOND: f32 = 270.0;

/// Seconds a fired Quake lives, measured from launch.
///
/// **Recovered, confidence 85** (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
/// "The lifetime is 5.0 seconds from launch, and the age is inherited"). The
/// original does not carry the wave as one object with one clock: it arms a
/// *road span* record, each of which ages by `dt` and sets its own retire byte
/// (`+0x7d`) the frame `age + dt` exceeds `5.0` - the literal is a `lui a0,
/// 0x40a0` at `0x0891cafc`, read at instruction level rather than off the
/// decompiler. What makes that one wave lifetime rather than a per-span one is
/// that `Quake_ArmSpan` (`0x0891b714`) copies its caller's *age* into the new
/// span (`+0x74 = param_2`) instead of zeroing it, and both propagation
/// functions pass the parent span's own `+0x74` along. `Quake_Init` starts the
/// chain at `0`, so every span the ripple ever spreads to shares one clock
/// begun at launch and the whole apparatus retires together.
///
/// Not authored: like [`SPEED_UNITS_PER_SECOND`] it is a fixed engine literal,
/// not one of [`QuakeStats`]'s four attributes.
pub const LIFETIME_SECONDS: f32 = 5.0;

/// The single travelling wave a fired Quake ever has.
///
/// One instance for the whole race, matching `Weapon_FireQuake`'s own pool -
/// a single slot, not an array
/// (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`) - so
/// [`crate::World`] carries `Option<Wave>` rather than a fixed array the way
/// [`super::Projectiles`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wave {
    /// Which ship slot fired it. Excluded from every hit test - the original
    /// clears the latch outright for the shooter's own craft rather than
    /// running the proximity test against it at all.
    pub owner: u8,
    /// Distance from the course's own start line, in `oag_race::Standing::progress`'s
    /// own convention and units - `0.0..course.length()`, wrapping.
    pub progress: f32,
    /// Which way around the ring the wave travels: `1.0` toward increasing
    /// progress, `-1.0` toward decreasing. Fixed at launch and never
    /// re-evaluated, matching `Quake_Init`'s own one-shot dot product.
    pub direction: f32,
    /// Energy the wave costs a craft it passes over, once.
    ///
    /// Copied from [`QuakeStats::damage`] at launch rather than looked up
    /// every tick, since only one wave ever exists at a time - see the
    /// module doc comment on why this differs from the original's own
    /// craft-side caching.
    pub damage: f32,
    /// The wave's own proximity gate - see [`QuakeStats::radius`].
    pub radius: f32,
    /// Seconds of slowdown the wave charges a craft it passes over, once.
    pub slowdown_time: f32,
    /// Per-slot latch: whether this craft is currently inside [`Self::radius`]
    /// of the wave, so a hit is credited on the rising edge only rather than
    /// every tick a craft dawells inside the gate. Mirrors `entity+0x860 &
    /// 0x40` - see the module doc comment on how the two gates differ.
    pub hit: [bool; MAX_SHIPS],
    /// Seconds since launch, against [`LIFETIME_SECONDS`].
    ///
    /// **The one piece of per-wave state the original keeps per road span**, on
    /// the reasoning [`LIFETIME_SECONDS`]'s own doc comment gives: the original's
    /// spans inherit each other's age, so a single scalar here is the same
    /// clock, not an approximation of several.
    pub age: f32,
}

impl Wave {
    /// A freshly-launched wave, at the firing craft's own current progress.
    ///
    /// `forward_dot_tangent` is the dot product of the firing craft's own
    /// forward vector against the track's own tangent at its current
    /// position - the caller's job, since only `crates/game/src/race` holds
    /// an `oag_race::Course` to read a tangent from. Its sign alone is used;
    /// a value of exactly `0.0` (forward perpendicular to the track, e.g. a
    /// craft facing dead across it) takes the positive direction rather than
    /// stalling on a `0.0 * SPEED` that never moves - **chosen, not
    /// measured**, since `Quake_Init`'s own tie-break at a dot product of
    /// exactly zero is not established.
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

    /// Advances the wave one tick along a course of this `length`, wrapping,
    /// and ages it.
    ///
    /// **The age advances on every path, including the degenerate one.** A
    /// non-positive length leaves [`Self::progress`] alone - a course that
    /// short has nothing for the wave to travel around - but a wave that cannot
    /// move still has to expire, or the guard against firing a second Quake
    /// never opens again.
    pub fn advance(&mut self, length: f32, dt: f32) {
        self.age += dt;
        if length <= 0.0 {
            return;
        }
        self.progress =
            (self.progress + self.direction * SPEED_UNITS_PER_SECOND * dt).rem_euclid(length);
    }

    /// Whether the wave has outlived [`LIFETIME_SECONDS`] and its caller should
    /// drop it.
    ///
    /// Read *after* [`Self::advance`] and *before* [`Self::apply_hits`], so the
    /// tick a wave expires on lands no hit - matching the original, where the
    /// span's retire byte (`+0x7d`) short-circuits both its own ripple and its
    /// propagation on the frame it is set.
    #[must_use]
    pub fn expired(&self) -> bool {
        self.age > LIFETIME_SECONDS
    }

    /// The shortest distance around a ring of this `length` between the
    /// wave's own progress and `other`.
    ///
    /// Split out for its own test: a ring wraps, so the straight difference
    /// overstates the distance for a craft just the other side of the start
    /// line from the wave.
    #[must_use]
    pub fn progress_delta(&self, other: f32, length: f32) -> f32 {
        let raw = (other - self.progress).abs();
        if length > 0.0 {
            raw.min(length - raw)
        } else {
            raw
        }
    }

    /// One tick of the wave's own hit test against every active ship.
    ///
    /// **Owner excluded outright**, matching the original's own
    /// `craft_is_shooter` short-circuit rather than running the radius test
    /// against the shooter and always missing it - the two read the same
    /// from outside, but this is closer to what `FUN_088418e0`'s block
    /// actually branches on.
    ///
    /// A ship with no [`oag_race::Standing::progress`] yet (not yet located
    /// on the course) is skipped rather than treated as in or out of range -
    /// the same "no reading, no claim" shape that field itself takes.
    ///
    /// Shielded craft take no hit and their latch clears, matching the
    /// original's own shield branch (`entity+0x1b8 & 0x10`) clearing the bit
    /// rather than holding it.
    ///
    /// `absorbed[slot]` is set (never cleared) when a hit landed on a
    /// shielded craft's shell - the same out-parameter shape
    /// [`super::blast::blast`] and [`super::cannon::direct_hit`] both take,
    /// for the same reason: a swallowed hit is the only thing that makes the
    /// shell visibly react.
    pub fn apply_hits(
        &mut self,
        ships: &mut [Ship; MAX_SHIPS],
        ship_count: u8,
        length: f32,
        rules: oag_physics::DamageRules,
        absorbed: &mut [bool],
    ) {
        for (slot, ship) in ships.iter_mut().enumerate().take(ship_count as usize) {
            if slot as u8 == self.owner {
                self.hit[slot] = false;
                continue;
            }
            if !ship.active {
                continue;
            }
            let Some(progress) = ship.standing.progress else {
                continue;
            };
            let shielded = ship.physics.shield_pickup_timer > 0.0;
            let within = self.progress_delta(progress, length) <= self.radius;
            if within && !shielded {
                if !self.hit[slot] {
                    ship.pending_slowdown += self.slowdown_time;
                    let dimensions = ship.handling.dimensions;
                    let report = oag_physics::damage::apply_weapon(
                        &mut ship.physics,
                        &dimensions,
                        self.damage,
                        rules,
                    );
                    if let Some(flag) = absorbed.get_mut(slot) {
                        *flag |= report.absorbed;
                    }
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
