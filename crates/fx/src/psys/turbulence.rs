//! Modifier type `0x11`: a Perlin-noise force on every live particle
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "The grey rows were sparks
//! that never moved").
//!
//! Pulse's `FUN_088fab50` and HD's `0x0027e118` build it the same way, each tick and per
//! particle, before the position steps by the velocity:
//!
//! 1. `p = position * 100`. One noise point per axis: `x` at `p * f.x`, `y` at
//!    `(p + (135.13, -123.45, 12.39)) * f.y`, `z` at `(p + (3931.49, 11241.24, -141.89)) * f.z`,
//!    `f` being `params[0..3]`.
//! 2. When `params[3]` is not zero the manager's clock, in seconds, is added to each point's
//!    `z`, after the second octave's points (`2 * point`) were taken, so that octave stands
//!    still in time.
//! 3. Each axis is `noise3(point) + 0.5 * noise3(2 * point)`, `0` where its frequency is `0`,
//!    times `params[4..7]`.
//! 4. With `params[8] < 1` the force is added to the velocity times the tick count; otherwise
//!    to the position.
//!
//! `noise3` is Ken Perlin's reference gradient noise (`FUN_088f2be4`, `0x0027dbc0`), its
//! tables filled from the C library's `rand()` on first use (`FUN_088f2604`). Here they are
//! drawn once from a seeded [`Rng`]: the law and its statistics are the original's, the
//! particular field is chosen, not measured.

use std::sync::OnceLock;

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_pob as pob;

use super::particle::Particle;
use super::{Effect, EmitterSpec, TICK_HZ};

/// `pob::Modifier::kind` of the noise force.
pub const KIND: u32 = 0x11;

/// The offsets the `y` and `z` axes sample the field at, so the three are independent.
const Y_OFFSET: Vec3 = Vec3::new(135.13, -123.45, 12.39);
const Z_OFFSET: Vec3 = Vec3::new(3931.49, 11241.24, -141.89);

/// The noise force one emitter authors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turbulence {
    /// `params[0..3]`: each axis's spatial frequency, on `position * 100`.
    pub frequency: [f32; 3],
    /// `params[3] != 0`: the field drifts with the clock.
    pub timed: bool,
    /// `params[4..7]`: each axis's amplitude, units per tick per tick.
    pub amplitude: [f32; 3],
    /// `params[8] >= 1`: the force moves the position instead of the velocity.
    pub on_position: bool,
    /// Whether the player applies it - only for an effect a title lists, see
    /// [`Effect::with_turbulence`].
    pub on: bool,
}

impl Turbulence {
    /// The first type-`0x11` node of `record`'s modifier list, switched off.
    #[must_use]
    pub fn from_record(record: &pob::Emitter) -> Option<Self> {
        let node = record.modifiers.iter().find(|m| m.kind == KIND)?;
        let p = node.params;
        Some(Self {
            frequency: [p[0], p[1], p[2]],
            timed: p[3] != 0.0,
            amplitude: [p[4], p[5], p[6]],
            on_position: p[8] >= 1.0,
            on: false,
        })
    }

    /// The force at `position` when the clock reads `clock` seconds, units per tick per tick.
    #[must_use]
    pub fn force(&self, position: Vec3, clock: f32) -> Vec3 {
        let noise = table();
        let p = position * 100.0;
        let points = [p, p + Y_OFFSET, p + Z_OFFSET];
        let mut out = [0.0f32; 3];
        for (axis, (out, point)) in out.iter_mut().zip(points).enumerate() {
            let frequency = self.frequency[axis];
            if frequency == 0.0 {
                continue;
            }
            let point = point * frequency;
            let second = point * 2.0;
            let mut first = point;
            if self.timed {
                first.z += clock;
            }
            *out = (noise.sample(first) + noise.sample(second) * 0.5) * self.amplitude[axis];
        }
        Vec3::new(out[0], out[1], out[2])
    }
}

/// Adds `spec`'s force to `particle`, if it has one switched on: to the velocity (kept per
/// second here, so times [`TICK_HZ`]) or to the position, `dt_ticks` ticks' worth.
pub(super) fn apply(spec: &EmitterSpec, particle: &mut Particle, clock: f32, dt_ticks: f32) {
    let Some(turbulence) = spec.turbulence.filter(|t| t.on) else {
        return;
    };
    let force = turbulence.force(particle.position, clock) * dt_ticks;
    if turbulence.on_position {
        particle.position += force;
    } else {
        particle.velocity += force * TICK_HZ;
    }
}

impl Effect {
    /// Applies every emitter's noise force (modifier type `0x11`), which the player
    /// otherwise skips: for an effect a title lists (`oag_title::Effects::turbulence`).
    pub fn with_turbulence(&mut self) {
        for spec in &mut self.emitters {
            if let Some(turbulence) = &mut spec.turbulence {
                turbulence.on = true;
            }
        }
    }

    /// Whether any emitter applies a noise force.
    #[must_use]
    pub fn has_turbulence(&self) -> bool {
        self.emitters
            .iter()
            .any(|spec| spec.turbulence.is_some_and(|t| t.on))
    }
}

/// Entries in the permutation; the tables carry `2 * B + 2`.
const B: usize = 0x100;

/// Perlin's three-dimensional gradient noise tables.
struct Noise3 {
    permutation: Vec<usize>,
    gradient: Vec<Vec3>,
}

/// The one table every effect samples, as the original's.
fn table() -> &'static Noise3 {
    static TABLE: OnceLock<Noise3> = OnceLock::new();
    TABLE.get_or_init(|| Noise3::new(&mut Rng::new(0x0011_5eed)))
}

impl Noise3 {
    /// `FUN_088f2604`: identity permutation and a normalised gradient per entry, each
    /// component `(rand % 512 - 256) / 256`; the permutation shuffled by `rand & 0xff`
    /// swaps from the top down; both tables repeated past `B`.
    fn new(rng: &mut Rng) -> Self {
        let mut permutation = vec![0usize; B + B + 2];
        let mut gradient = vec![Vec3::ZERO; B + B + 2];
        let mut component = || ((rng.next_u32() % 512) as f32 - 256.0) / 256.0;
        for (index, (slot, g)) in permutation
            .iter_mut()
            .zip(&mut gradient)
            .enumerate()
            .take(B)
        {
            *slot = index;
            let v = Vec3::new(component(), component(), component());
            let length = v.length();
            *g = if length > 0.0 { v / length } else { v };
        }
        for index in (1..B).rev() {
            let other = (rng.next_u32() & 0xff) as usize;
            permutation.swap(index, other);
        }
        for index in 0..B + 2 {
            permutation[B + index] = permutation[index];
            gradient[B + index] = gradient[index];
        }
        Self {
            permutation,
            gradient,
        }
    }

    /// `FUN_088f2be4`: the noise at `v`, about `-1..1`.
    fn sample(&self, v: Vec3) -> f32 {
        let split = |c: f32| {
            // Truncated through a signed word, as the original's conversion.
            let shifted = c + 4096.0;
            let cell = shifted as i32;
            let fraction = shifted - cell as f32;
            (
                (cell & 0xff) as usize,
                (cell.wrapping_add(1) & 0xff) as usize,
                fraction,
            )
        };
        let (bx0, bx1, rx0) = split(v.x);
        let (by0, by1, ry0) = split(v.y);
        let (bz0, bz1, rz0) = split(v.z);
        let (rx1, ry1, rz1) = (rx0 - 1.0, ry0 - 1.0, rz0 - 1.0);
        let p = &self.permutation;
        let (i, j) = (p[bx0], p[bx1]);
        let (b00, b10, b01, b11) = (p[i + by0], p[j + by0], p[i + by1], p[j + by1]);
        let s_curve = |t: f32| t * t * (3.0 - t * 2.0);
        let (sx, sy, sz) = (s_curve(rx0), s_curve(ry0), s_curve(rz0));
        let at = |b: usize, x: f32, y: f32, z: f32| {
            let g = self.gradient[b];
            x * g.x + y * g.y + z * g.z
        };
        let lerp = |t: f32, a: f32, b: f32| a + t * (b - a);
        let face = |bz: usize, z: f32| {
            let a = lerp(sx, at(b00 + bz, rx0, ry0, z), at(b10 + bz, rx1, ry0, z));
            let b = lerp(sx, at(b01 + bz, rx0, ry1, z), at(b11 + bz, rx1, ry1, z));
            lerp(sy, a, b)
        };
        lerp(sz, face(bz0, rz0), face(bz1, rz1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sparks() -> Turbulence {
        Turbulence {
            frequency: [0.3, 0.1, 0.3],
            timed: true,
            amplitude: [0.1, 0.1, 0.1],
            on_position: false,
            on: true,
        }
    }

    #[test]
    fn the_noise_is_zero_on_every_lattice_point_and_bounded_between() {
        let noise = table();
        for v in [Vec3::ZERO, Vec3::new(3.0, -7.0, 12.0)] {
            assert_eq!(noise.sample(v), 0.0);
        }
        let mut rng = Rng::new(3);
        let mut biggest = 0.0f32;
        for _ in 0..10_000 {
            let v = Vec3::new(rng.next_f32(), rng.next_f32(), rng.next_f32()) * 100.0;
            biggest = biggest.max(noise.sample(v).abs());
        }
        assert!(biggest > 0.3 && biggest < 1.0, "{biggest}");
    }

    #[test]
    fn a_zero_frequency_axis_has_no_force_and_the_others_vary_in_space_and_time() {
        let mut t = sparks();
        t.frequency[1] = 0.0;
        let a = t.force(Vec3::new(1.23, 0.5, -2.7), 0.0);
        assert_eq!(a.y, 0.0);
        assert!(a.x != 0.0 && a.z != 0.0, "{a:?}");
        assert_ne!(a, t.force(Vec3::new(1.24, 0.5, -2.7), 0.0));
        assert_ne!(a, t.force(Vec3::new(1.23, 0.5, -2.7), 0.37));
        t.timed = false;
        assert_eq!(
            t.force(Vec3::new(1.23, 0.5, -2.7), 0.0),
            t.force(Vec3::new(1.23, 0.5, -2.7), 5.0)
        );
    }

    /// A stationary emitter's particles stay on its point unless the effect switches its
    /// noise force on; then they drift off it.
    #[test]
    fn the_force_moves_a_motionless_particle_only_when_switched_on() {
        use crate::psys::System;
        use crate::psys::tests::{effect, run};

        let spread = |on: bool| {
            let mut effect = (*effect("sparks", false, 30.0)).clone();
            effect.emitters[0].turbulence = Some(Turbulence {
                on: false,
                ..sparks()
            });
            if on {
                effect.with_turbulence();
            }
            assert_eq!(effect.has_turbulence(), on);
            let mut rng = Rng::new(5);
            let mut system = System::new();
            system.ignite(&effect, Vec3::ZERO, 1.0);
            run(&mut system, &effect, 8, &mut rng);
            system.positions().map(Vec3::length).fold(0.0f32, f32::max)
        };
        assert_eq!(spread(false), 0.0);
        assert!(spread(true) > 0.05, "{}", spread(true));
    }
}
