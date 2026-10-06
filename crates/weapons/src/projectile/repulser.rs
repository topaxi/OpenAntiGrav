//! The Repulser: a blast at the firer, then two shockwaves that walk the track.
//!
//! Nothing here flies, and nothing is a field on the firing craft.
//! `Weapon_FireRepulser` (`0x0886ce8c`) copies four `<Stats>` onto the firer's
//! record and nothing reads them back; the weapon is a pool entity living
//! `blast_time + wave_time` seconds. For `blast_time` it shows itself around the
//! firer; then `Repulser_SpawnWaves` (`0x08876300`) starts two waves at the
//! firer's place on the AI track and `Repulser_Update` (`0x08875400`) walks them
//! along the control points, **five forward and two back per update** (immediates
//! at `0x08875518` and `0x0887553c`, no `dt`). Every other craft a wave's centre
//! sweeps is hit once: full `damage`, full `slowdown_time` and a shove of the full
//! `blastforce` toward the wave (`Repulser_HitCraft`, `0x0886d254`). Laid Mines
//! and Bombs a wave sweeps go off. Addresses:
//! `docs/ghidra/functions/psp-pulse-usa/repulser.md`.
//!
//! # What is recovered and what is ours
//!
//! **Recovered** (confidence 80-84, decompile plus instruction-level checks, not
//! runtime-verified): the two phases and lifetime, step counts and directions, the
//! wave at the midpoint of the track's edges, the sweep test ([`sweeps`]), the hit
//! law, the once-per-craft latch, the owner's exemption, the Mine/Bomb sweep.
//!
//! **Ours, each chosen rather than measured:**
//!
//! - Points per tick, not per frame: the original steps once per update; this runs
//!   at the fixed 60 Hz it is authored for (`docs/psp/frame-pacing.md`).
//! - The ring, then a branch: the waves walk [`oag_race::Course`]'s primary chain;
//!   at a split the first wave to cross starts the third on the alternate path
//!   ([`fork`], read at 88). Not built: the init-tick variant
//!   (`Repulser_ForkAtJunction`, `0x08876634`, 72) for a firer already on a branch.
//! - Ring points, not control points: [`oag_race::Course`] samples
//!   [`oag_race::Course::STEPS_PER_SEGMENT`] per interval, so five control points
//!   is twenty ring points. The firer's ring index starts the waves, so a wave can
//!   sit up to three quarters of an interval (about 4.5 units) off the original's.
//! - The craft's corridor width at its nearest ring point, where the original
//!   interpolates a fresh AI-track sample.

use crate::{Craft, MAX_SHIPS};
pub use fork::Fork;

pub mod fork;
use oag_core::math::Vec3;
use oag_tables::weapons::RepulserStats;

/// How many Repulsers can be live at once: `Weapon_FireRepulser`'s
/// `pool->live < 0x10` (`0x0886cec8`, `sltiu a3,s1,0x10`).
pub const POOL_SIZE: usize = 16;

/// Control points the forward wave moves per update: `li t2,0x5` at
/// `0x08875518`, with direction `t0 = 0`.
pub const FORWARD_POINTS_PER_TICK: usize = 5;

/// Control points the backward wave moves per update: `li t2,0x2` at
/// `0x0887553c`, with direction `t0 = 1`.
pub const BACKWARD_POINTS_PER_TICK: usize = 2;

/// How far past either end of this tick's swept segment a craft still counts as
/// swept: the `1.0` in `Repulser_WaveSweepsPoint`'s `f <= 1.0 && b >= -1.0`.
pub const SWEEP_SLACK: f32 = 1.0;

/// One wave's front: where it is on the ring and where its centre was.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Front {
    /// Ring index on [`oag_race::Course`].
    pub index: u32,
    /// The wave's centre this tick: the midpoint of the track's edges at
    /// [`Self::index`] ([`oag_race::Course::centre`]).
    pub point: Vec3,
    /// [`Self::point`] last tick. The sweep is the segment between the two.
    pub previous: Vec3,
}

impl Front {
    fn at(course: &oag_race::Course, index: usize) -> Option<Self> {
        let point = course.centre(index)?;
        Some(Self {
            index: u32::try_from(index).ok()?,
            point,
            previous: point,
        })
    }
}

/// One live Repulser.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Repulser {
    /// The ship slot that fired it. Never hit by its own waves.
    pub owner: u8,
    /// Seconds since the fire (`+0x1ec`).
    pub age: f32,
    /// [`RepulserStats::damage`], copied at launch.
    pub damage: f32,
    /// [`RepulserStats::blastforce`], copied at launch.
    pub force: f32,
    /// [`RepulserStats::slowdown_time`], copied at launch.
    pub slowdown_time: f32,
    /// [`RepulserStats::blast_time`], copied at launch.
    pub blast_time: f32,
    /// [`RepulserStats::wave_time`], copied at launch.
    pub wave_time: f32,
    /// The forward and the backward wave, or `None` during the blast phase.
    pub fronts: Option<[Front; 2]>,
    /// Which craft this Repulser has already hit (`+0x1f0`, eight bytes).
    pub hit: [bool; MAX_SHIPS],
    /// The third wave, once a wave has crossed a split (`+0x25c` latches it:
    /// one fork per Repulser). See [`fork`].
    pub fork: Option<Fork>,
}

impl Repulser {
    /// A Repulser just fired by `owner`.
    #[must_use]
    pub fn launch(owner: u8, stats: &RepulserStats) -> Self {
        Self {
            owner,
            age: 0.0,
            damage: stats.damage,
            force: stats.blastforce,
            slowdown_time: stats.slowdown_time,
            blast_time: stats.blast_time,
            wave_time: stats.wave_time,
            fronts: None,
            hit: [false; MAX_SHIPS],
            fork: None,
        }
    }

    /// Whether the waves have started.
    #[must_use]
    pub fn waves_running(&self) -> bool {
        self.fronts.is_some()
    }

    /// One update of `Repulser_Update`: age it, start or walk the waves, and say
    /// whether it lives on. `false` retires it **after** this tick's sweep, the
    /// original's order (`RepulserPool_Update` sweeps first).
    ///
    /// `owner_index` is the firer's ring index now: the waves start from where the
    /// firer is when `blast_time` runs out (`+0x22c` copied from `craft+0xad8` in
    /// `Repulser_SpawnWaves`). With none the start waits a tick (**chosen**). On
    /// the start tick the waves do not move (`previous == point`, so [`sweeps`]
    /// cannot fire): the init call of `Repulser_AdvanceWave` (`t3 = 1`) skips the
    /// walk.
    pub fn advance(
        &mut self,
        course: &oag_race::Course,
        owner_index: Option<usize>,
        dt: f32,
    ) -> bool {
        self.age += dt;
        let count = course.len();
        if let Some(fronts) = self.fronts.as_mut() {
            if count > 0 {
                let per_point = oag_race::Course::STEPS_PER_SEGMENT;
                let forward = (FORWARD_POINTS_PER_TICK * per_point) % count;
                let backward = (BACKWARD_POINTS_PER_TICK * per_point) % count;
                let steps = [forward, count - backward];
                // The fork rides inside its parent's `Repulser_AdvanceWave`, after
                // the parent's walk: an existing one takes the full step, then a new
                // one spawns at most once.
                if let Some(fork) = self.fork.as_mut() {
                    fork.advance(course, forward);
                }
                // Only the forward wave forks, see [`fork`].
                if self.fork.is_none() {
                    self.fork = Fork::crossing(course, fronts[0].index as usize, forward);
                }
                for (front, step) in fronts.iter_mut().zip(steps) {
                    let index = (front.index as usize + step) % count;
                    front.previous = front.point;
                    if let Some(point) = course.centre(index) {
                        front.point = point;
                        front.index = u32::try_from(index).unwrap_or(front.index);
                    }
                }
            }
        } else if self.blast_time < self.age
            && let Some(index) = owner_index
            && let Some(front) = Front::at(course, index)
        {
            self.fronts = Some([front, front]);
        }
        self.age < self.blast_time + self.wave_time
    }

    /// Which front, if any, swept `point` this tick, given the corridor `width`
    /// there.
    #[must_use]
    pub fn swept_by(&self, point: Vec3, width: f32) -> Option<Front> {
        self.fronts?
            .into_iter()
            .find(|front| sweeps(front.point, front.previous, point, width))
    }

    /// [`Self::swept_by`] with the fork wave tried last, for craft:
    /// `RepulserPool_SweepTargets` tests it only if waves 0 and 1 missed, never
    /// against Mines or Bombs.
    #[must_use]
    pub fn swept_by_any(&self, point: Vec3, width: f32) -> Option<Front> {
        self.swept_by(point, width).or_else(|| {
            self.fork
                .map(|fork| fork.front)
                .filter(|front| sweeps(front.point, front.previous, point, width))
        })
    }

    /// `RepulserPool_SweepTargets`'s craft half: every other active craft a wave
    /// swept this tick and not yet hit takes the hit.
    ///
    /// No shield test here, as in the original: shove and slowdown land regardless
    /// and [`oag_physics::damage::apply_weapon`] decides the damage, as
    /// [`super::blast::blast`] does. `hits` gets each hit as absorbed or landed;
    /// the return is the slots hit this tick, for the `REPULSORHIT` cue.
    pub fn apply_hits<S: Craft>(
        &mut self,
        ships: &mut [S; MAX_SHIPS],
        ship_count: u8,
        course: &oag_race::Course,
        rules: oag_physics::DamageRules,
        hits: &mut [super::WeaponHit],
    ) -> [bool; MAX_SHIPS] {
        let mut struck = [false; MAX_SHIPS];
        if self.fronts.is_none() {
            return struck;
        }
        for (slot, ship) in ships.iter_mut().enumerate().take(ship_count as usize) {
            if slot == self.owner as usize || self.hit[slot] || !ship.active() {
                continue;
            }
            let Some(width) = ship
                .course_index()
                .and_then(|index| course.corridor_width(index as usize))
            else {
                continue;
            };
            let position = ship.physics().body.position;
            let Some(front) = self.swept_by_any(position, width) else {
                continue;
            };
            // `vsub.q` craft minus wave (`0x0886d2a0`), then `neg.s` on
            // `blastForce` (`0x0886d30c`): the shove points from craft toward the
            // wave's centre, along its travel, as the craft sits between the
            // centres of last tick and this.
            let direction = normalize_or_zero(position - front.point);
            ship.physics_mut()
                .body
                .apply_impulse(direction * -self.force);
            *ship.pending_slowdown_mut() += self.slowdown_time;
            let dimensions = ship.dimensions();
            let report = oag_physics::damage::apply_weapon(
                ship.physics_mut(),
                &dimensions,
                self.damage,
                rules,
            );
            super::hit::record(hits, slot, &report);
            self.hit[slot] = true;
            struck[slot] = true;
        }
        struck
    }
}

/// The VFPU normalise this module uses: `vcmp.s EQ` and `vcmovt.s` swap a zero
/// length for `MaxFloat` before the reciprocal, so a zero vector stays zero.
fn normalize_or_zero(v: Vec3) -> Vec3 {
    let length = v.length();
    let length = if length == 0.0 { f32::MAX } else { length };
    v * (1.0 / length)
}

/// `Repulser_WaveSweepsPoint` (`0x0886cfc8`): did a wave whose centre moved from
/// `previous` to `point` pass over `target`? It must lie between the centres along
/// the travel to within [`SWEEP_SLACK`], and within `width` of the axis. Both
/// along-track terms must be non-zero, so an unmoved wave (`previous == point`)
/// sweeps nothing.
#[must_use]
pub fn sweeps(point: Vec3, previous: Vec3, target: Vec3, width: f32) -> bool {
    let travel = normalize_or_zero(point - previous);
    let ahead = travel.dot(target - point);
    let behind = travel.dot(target - previous);
    if !(ahead <= SWEEP_SLACK && behind >= -SWEEP_SLACK && ahead != 0.0 && behind != 0.0) {
        return false;
    }
    let offset = target - point;
    let across = normalize_or_zero(travel.cross(travel.cross(offset)));
    let distance = across.dot(offset);
    -width < distance && distance < width
}

#[cfg(test)]
mod tests;
