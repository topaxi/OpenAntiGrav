//! The enhanced weather's road spray: **this project's own, chosen, not
//! measured.** The original's rain never touches a craft's wake
//! (`docs/ghidra/functions/psp-pulse-usa/weather.md`); with the weather
//! setting at `enhanced` each craft throws a fine, short-lived mist off the
//! wet road behind and below it, more of it the faster it goes and the wetter
//! the road is.
//!
//! **Presentation, not simulation.** Nothing here writes `World`: the emitters
//! are read from the craft's drawn pose, the road under it is found with the
//! blob shadow's read-only cast, and nothing runs at `original`. The spray has
//! its own random stream, apart from the weather's drops and the splashes'.
//! Art is the splash art's dot cell ([`super::splash`]).

use oag_core::Rng;
use oag_core::math::{SPEED_TO_KMH, Vec3};
use oag_mesh::mesh::GpuVertex;
use oag_physics::collide::{Ray, Raycaster};

use super::splash::quad;

/// Droplets alive at once, over every craft. Past it a new one is dropped.
pub const CAPACITY: usize = 320;

/// The speed (km/h) at which a craft throws its full share, and the most
/// droplets one craft throws a tick at full speed on a fully wet road.
pub const FULL_SPEED_KMH: f32 = 600.0;
pub const MAX_PER_TICK: f32 = 5.0;

/// A craft slower than this throws nothing.
pub const MIN_SPEED_KMH: f32 = 60.0;

/// How high off the road a craft may be and still throw spray, as a multiple
/// of its hull height.
const REACH_HULLS: f32 = 2.5;

/// Seconds a droplet lasts, and its half-size in world units.
const LIFE: f32 = 0.35;
const SIZE: f32 = 0.15;

/// The share of the craft's speed a droplet is born with (forward, as the
/// air it is torn from is moving) and the drag per second that stops it, so it
/// hangs `CARRY * speed / DRAG` behind where it was thrown (a few units at
/// speed); its upward kick and sideways spread in world units a second, and
/// gravity. Droplets are thrown anywhere along the path the craft covered this
/// tick, so a fast craft leaves a continuous mist, not one puff a tick.
const CARRY: f32 = 0.1;
const DRAG: f32 = 30.0;
const KICK: (f32, f32) = (1.0, 3.0);
const SPREAD: f32 = 3.0;
const GRAVITY: f32 = 4.5;

/// The most opaque a droplet gets.
const ALPHA: f32 = 0.4;

const SEED: u64 = 0x5b2a_7c19;

/// The mist: a slower, softer, larger puff a craft leaves in the air behind it,
/// **chosen, not measured**, in addition to the droplets. It rises and widens
/// over [`MIST_LIFE`] seconds so the chase camera sees it above and behind the
/// hull, where a droplet's 0.35 s life ends under it.
///
/// Alive at once, over every craft; past it a new puff is dropped (the player's
/// craft is thrown first, so a full pool starves the field, not the player).
pub const MIST_CAPACITY: usize = 240;

/// The most puffs one craft throws a tick at full speed on a fully wet road.
pub const MIST_MAX_PER_TICK: f32 = 1.0;

/// Seconds a puff lasts, its half-size at birth and at death (world units),
/// and the most opaque it gets (it fades in over the first
/// [`MIST_FADE_IN`] of its life and out over the rest).
const MIST_LIFE: f32 = 1.4;
const MIST_SIZE: (f32, f32) = (0.8, 3.0);
const MIST_ALPHA: f32 = 0.12;
const MIST_FADE_IN: f32 = 0.08;

/// Its rise in world units a second (up the road's normal), the sideways spread
/// either way, the share of the craft's speed it is born with and the drag per
/// second that stops it, and how far above the road it starts.
const MIST_RISE: (f32, f32) = (2.0, 4.0);
const MIST_SPREAD: f32 = 4.0;
const MIST_CARRY: f32 = 0.98;
const MIST_DRAG: f32 = 0.3;
const MIST_LIFT: f32 = 0.8;

const MIST_SEED: u64 = 0x3c71_e5a3;

/// One craft's mouth: where its road is, which way it faces and how fast.
#[derive(Debug, Clone, Copy)]
pub struct Emitter {
    pub at: Vec3,
    pub normal: Vec3,
    pub forward: Vec3,
    pub half_width: f32,
    pub speed_kmh: f32,
}

#[derive(Debug, Clone, Copy)]
struct Droplet {
    at: Vec3,
    carry: Vec3,
    velocity: Vec3,
    normal: Vec3,
    age: f32,
}

#[derive(Debug, Clone, Copy)]
struct Puff {
    at: Vec3,
    carry: Vec3,
    velocity: Vec3,
    age: f32,
}

/// A puff's opacity at `age` seconds: in over the first [`MIST_FADE_IN`] of its
/// life, out over the rest.
fn mist_alpha(age: f32) -> f32 {
    let t = (age / MIST_LIFE).clamp(0.0, 1.0);
    MIST_ALPHA * (t / MIST_FADE_IN).min(1.0) * (1.0 - t)
}

impl Puff {
    /// Where it is: carried forward until the air stops it, risen and spread.
    fn position(&self) -> Vec3 {
        let slid = (1.0 - (-MIST_DRAG * self.age).exp()) / MIST_DRAG;
        self.at + self.carry * slid + self.velocity * self.age
    }
}

/// The droplets and the mist alive now.
#[derive(Debug, Clone)]
pub struct Spray {
    live: Vec<Droplet>,
    rng: Rng,
    mist: Vec<Puff>,
    mist_rng: Rng,
}

impl Default for Spray {
    fn default() -> Self {
        Self {
            live: Vec::with_capacity(CAPACITY),
            rng: Rng::new(SEED),
            mist: Vec::with_capacity(MIST_CAPACITY),
            mist_rng: Rng::new(MIST_SEED),
        }
    }
}

/// How many mist puffs one craft throws a tick: the droplets' rule, scaled to
/// [`MIST_MAX_PER_TICK`].
#[must_use]
pub fn mist_rate(speed_kmh: f32, wet: f32) -> f32 {
    rate(speed_kmh, wet) / MAX_PER_TICK * MIST_MAX_PER_TICK
}

/// How many droplets one craft throws a tick: zero when the road is dry or
/// the craft slow, rising linearly with both.
#[must_use]
pub fn rate(speed_kmh: f32, wet: f32) -> f32 {
    if wet <= 0.0 || speed_kmh < MIN_SPEED_KMH {
        return 0.0;
    }
    MAX_PER_TICK * wet.clamp(0.0, 1.0) * (speed_kmh / FULL_SPEED_KMH).min(1.0)
}

impl Spray {
    /// How many are alive.
    #[must_use]
    pub fn len(&self) -> usize {
        self.live.len()
    }

    /// Whether no droplet and no mist is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.live.is_empty() && self.mist.is_empty()
    }

    /// How many mist puffs are alive.
    #[must_use]
    pub fn mist_len(&self) -> usize {
        self.mist.len()
    }

    /// Drops every droplet and puff, as the setting going back to `original` does.
    pub fn clear(&mut self) {
        self.live.clear();
        self.mist.clear();
    }

    /// Ages every droplet `dt` seconds, removes the finished and throws the
    /// tick's new ones from `emitters` on a road `wet` (`0.0..=1.0`).
    pub fn advance(&mut self, dt: f32, wet: f32, emitters: &[Emitter]) {
        for drop in &mut self.live {
            drop.age += dt;
        }
        self.live.retain(|drop| drop.age < LIFE);
        for puff in &mut self.mist {
            puff.age += dt;
        }
        self.mist.retain(|puff| puff.age < MIST_LIFE);
        for emitter in emitters {
            let expected = mist_rate(emitter.speed_kmh, wet);
            let mut count = expected.floor() as usize;
            if self.mist_rng.next_f32() < expected.fract() {
                count += 1;
            }
            for _ in 0..count {
                self.breathe(emitter, dt);
            }
        }
        for emitter in emitters {
            let expected = rate(emitter.speed_kmh, wet);
            let mut count = expected.floor() as usize;
            if self.rng.next_f32() < expected.fract() {
                count += 1;
            }
            for _ in 0..count {
                self.throw(emitter, dt);
            }
        }
    }

    fn breathe(&mut self, emitter: &Emitter, dt: f32) {
        if self.mist.len() >= MIST_CAPACITY {
            return;
        }
        let normal = emitter.normal.normalize_or_zero();
        let side = normal.cross(emitter.forward).normalize_or_zero();
        let rise = MIST_RISE.0 + (MIST_RISE.1 - MIST_RISE.0) * self.mist_rng.next_f32();
        let spread = (self.mist_rng.next_f32() * 2.0 - 1.0) * MIST_SPREAD;
        let lateral = (self.mist_rng.next_f32() * 2.0 - 1.0) * emitter.half_width;
        let speed = emitter.speed_kmh / SPEED_TO_KMH;
        let along = emitter.forward * (speed * dt * self.mist_rng.next_f32());
        self.mist.push(Puff {
            at: emitter.at + side * lateral + normal * MIST_LIFT - along,
            carry: emitter.forward * (speed * MIST_CARRY),
            velocity: normal * rise + side * spread,
            age: 0.0,
        });
    }

    fn throw(&mut self, emitter: &Emitter, dt: f32) {
        if self.live.len() >= CAPACITY {
            return;
        }
        let normal = emitter.normal.normalize_or_zero();
        let side = normal.cross(emitter.forward).normalize_or_zero();
        let edge = if self.rng.next_f32() < 0.5 { -1.0 } else { 1.0 };
        let lateral = edge * emitter.half_width * (0.8 + 0.4 * self.rng.next_f32());
        let kick = KICK.0 + (KICK.1 - KICK.0) * self.rng.next_f32();
        let spread = (self.rng.next_f32() * 2.0 - 1.0) * SPREAD;
        let speed = emitter.speed_kmh / SPEED_TO_KMH;
        let along = emitter.forward * (speed * dt * self.rng.next_f32());
        self.live.push(Droplet {
            at: emitter.at + side * lateral - along,
            carry: emitter.forward * (speed * CARRY),
            velocity: normal * kick + side * spread,
            normal,
            age: 0.0,
        });
    }

    /// Appends the droplets, camera-facing, into `alpha_over`. `cell` is the
    /// splash art's rectangle on the particle sheet; the spray uses its dot.
    pub fn extend_vertices(
        &self,
        alpha_over: &mut Vec<GpuVertex>,
        right: Vec3,
        up: Vec3,
        cell: [f32; 4],
    ) {
        let width = cell[2] - cell[0];
        let dot = [cell[0] + width * 0.5, cell[1], cell[2], cell[3]];
        for puff in &self.mist {
            let t = puff.age / MIST_LIFE;
            let size = MIST_SIZE.0 + (MIST_SIZE.1 - MIST_SIZE.0) * t;
            alpha_over.extend(quad(
                puff.position(),
                right * size,
                up * size,
                [0.85, 0.92, 1.0],
                mist_alpha(puff.age),
                dot,
            ));
        }
        for drop in &self.live {
            let t = drop.age / LIFE;
            let height = drop.age * drop.age * -0.5 * GRAVITY;
            let slid = (1.0 - (-DRAG * drop.age).exp()) / DRAG;
            let at = drop.at + drop.carry * slid + drop.velocity * drop.age + drop.normal * height;
            if (at - drop.at).dot(drop.normal) < 0.0 {
                continue;
            }
            alpha_over.extend(quad(
                at,
                right * SIZE,
                up * SIZE,
                [0.85, 0.92, 1.0],
                ALPHA * (1.0 - t),
                dot,
            ));
        }
    }
}

impl crate::Race {
    /// Every racing craft's emitter on this draw: the road under it, found
    /// along its own down axis within [`REACH_HULLS`] hull heights, and left out
    /// where that road lies in a covered section.
    pub fn spray_emitters(&self) -> Vec<Emitter> {
        let mut out = Vec::new();
        for slot in 0..self.sim.world.ship_count as usize {
            let ship = &self.sim.world.ships[slot];
            if !ship.active {
                continue;
            }
            let speed_kmh = ship.physics.body.linear_velocity.length() * SPEED_TO_KMH;
            if speed_kmh < MIN_SPEED_KMH {
                continue;
            }
            let body = self.body_between(slot);
            let reach = ship.handling.dimensions.height * REACH_HULLS;
            let mut hits = [None; 4];
            let found = self.sim.collision.raycast_all(
                Ray::new(body.position, -body.up(), reach),
                None,
                false,
                &mut hits,
            );
            let Some(hit) = hits
                .iter()
                .flatten()
                .take(found)
                .filter(|hit| hit.surface.is_hoverable())
                .min_by(|a, b| a.distance.total_cmp(&b.distance))
            else {
                continue;
            };
            if self.point_is_under_cover(hit.point) {
                continue;
            }
            let forward = body.forward();
            let half_length = ship.handling.dimensions.length * 0.5;
            out.push(Emitter {
                at: hit.point - forward * half_length * 0.8 + hit.normal * 0.1,
                normal: hit.normal,
                forward,
                half_width: ship.handling.dimensions.width * 0.5,
                speed_kmh,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests;
