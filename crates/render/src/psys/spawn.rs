//! Where in the emitter a particle is born - the emitter extent.
//!
//! `ParticleSystem_SpawnBurst` switches on the emitter's shape (`+0x30`) and
//! each emit function places the particle's spawn offset in the emitter's own
//! frame before `ParticleSystem_InitParticle` carries it through the node
//! matrix. Three shapes are read and implemented here; the rest stay at the
//! anchor, as every shape did before:
//!
//! - **Shape 3, a ring or a disc.** `FUN_088fc634`, read in full on 2026-10-01 and
//!   checked against a live detonation of `WO_BOMB_SMOKERING`: with `r` the scaled
//!   extent (`DAT_08ab2290`) and `phi` drawn `U(0, 2 pi)` per particle, `+0x3c` `0`
//!   places it at `(r cos phi, 0, r sin phi)` - a **ring** in the frame's `XZ` plane,
//!   its `Y` the cone's axis; `1` at a radius `Psys_RandSpread(r, +0x40)`; `2` anywhere
//!   in the **disc** (a point of the square `[-r, r]^2` redrawn until it lies inside the
//!   circle). Under an aimed velocity (mode 1, which every shape-3 emitter with an extent
//!   authors) the particle then flies along the heading of that offset - see [`place`].
//!   `r` is **not constant**: the emitter's animated-attribute record (selector 2,
//!   [`oag_vex::pob::attribute`]) scales the extent over its run, so the smoke ring's
//!   particles were born 13.1, 13.7, 15.0, 17.6 and 23.4 units out at emitter ticks 0, 1,
//!   3, 7 and 16 (two boots) against the authored `12.94`, and
//!   [`super::EmitterSpec::extent_animation`] plays it. Until then shape 3 was a point, on
//!   the strength of the collision sparks' extents of at most 0.1; the corpus authors `12.9`
//!   on the Bomb's smoke and the ship explosion's root, `10` on its debris, `13.6` on the
//!   Repulser's blast and `5.1` on the Rocket's own debris. **Not played:** flag
//!   `0x200000`, which steps `phi` evenly (`2 pi / count`) from a random start instead of
//!   drawing it (only `WO_REPULSER_BLAST` carries it), the sub-frame spread of a moving
//!   emitter, the sign of the aimed azimuth (unmeasured, and the debris authors `0.925`),
//!   and the Repulser's selector-5 record.
//! - **Shape 1, a line (or a flat rectangle).** `FUN_088fcfec`, read at
//!   instruction level on 2026-09-24: while `+0x3c` is `0..=2` the offset is
//!   `(U(-e, e), 0, U(-z, z))`, `e` the scaled extent global
//!   (`DAT_08ab2290`) and `z` the resource's **unscaled** `+0x40`. Every
//!   shape-1 emitter in `WO_QUAKE` authors `z = 0`, so its particles are
//!   born along the frame's `X` axis only.
//! - **Shapes 4 and 7, a sphere or hemisphere.** `ParticleSystem_EmitSphere`
//!   (`particle-system.md`): the offset is the same unit direction the
//!   particle flies along, at a radius shaped by `+0x3c` - `0` exactly `e`,
//!   `1` spread by `± +0x40` scaled the same way, `2` times
//!   `sin(U(0, pi/2))`.
//!
//! `e` is `+0x34` times the instance's extent co-factor (`+0x2c`), its
//! severity (`+0x34`) and the emission-scale channel, which is what
//! `ParticleSystem_DeriveScaledParams` and `ParticleSystem_UpdateEmission`
//! multiply together. **The extent co-factor is the only thing the Quake
//! rescales**: `Quake_Update` writes `edge_distance / 50` into slot 1 of the
//! seven-word block `FUN_088f443c` reads out of the instance and
//! `FUN_088f44d8` writes back - instance `+0x2c`, which
//! `ParticleSystem_DeriveScaledParams` multiplies into the three extent
//! fields and nothing else. Size and speed are left as authored.
//!
//! **Applied to Pulse on the PSP only** - the law is read off that
//! executable alone; other sources call
//! [`super::Effect::without_extents`] at load, by choice, until their own
//! executables are read.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_vex::pob;

use super::Effect;

impl Effect {
    /// Spawns every emitter's particles at its anchor again, the way every
    /// source did before the extent law was read - see [this module](self).
    ///
    /// **For every source but Pulse on the PSP, by the lead's choice
    /// (2026-09-24).** The law is read off Pulse's PSP `BOOT.BIN`
    /// (`ParticleSystem_EmitLine` `0x088fcfec`, `ParticleSystem_EmitSphere`
    /// `0x088fd340`, `ParticleSystem_DeriveScaledParams` `0x088f4910`); the
    /// PS2 ELF and HD's `EBOOT` have not been read, so their effects keep
    /// placing particles as they did until they are.
    pub fn without_extents(&mut self) {
        for spec in &mut self.emitters {
            spec.spawn = Spawn::Point;
        }
    }

    /// Whether any emitter places its particles by the extent law - `false`
    /// after [`Self::without_extents`], and for an effect made only of
    /// point emitters.
    #[must_use]
    pub fn has_extents(&self) -> bool {
        self.emitters.iter().any(|spec| spec.spawn != Spawn::Point)
    }
}

/// How an emitter places a new particle's spawn offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Spawn {
    /// At the anchor - shape 0, and every shape not read yet.
    Point,
    /// Shape 1: along the frame's `X`, and its `Z` by the unscaled depth.
    Line {
        /// `+0x34`, before scaling.
        extent: f32,
        /// `+0x40`, never scaled.
        depth: f32,
    },
    /// Shape 3: a ring or a disc in the frame's `XZ` plane, see [this module](self).
    Ring {
        /// `+0x34`, before scaling.
        extent: f32,
        /// `+0x40`, scaled like `extent`; only read under `mode == 1`.
        spread: f32,
        /// `+0x3c`: `0` the ring, `1` the ring spread, `2` the disc.
        mode: u32,
    },
    /// Shapes 4 and 7: out along the particle's own direction.
    Sphere {
        /// `+0x34`, before scaling.
        extent: f32,
        /// `+0x40`, scaled like `extent`; only read under `mode == 1`.
        spread: f32,
        /// `+0x3c`.
        mode: u32,
    },
}

impl Spawn {
    /// `record`'s own placement law.
    #[must_use]
    pub fn of(record: &pob::Emitter) -> Self {
        let depth = record.extent_unread[1];
        match (record.shape, record.radius_mode) {
            (1, 0..=2) => Self::Line {
                extent: record.extent,
                depth: if depth.is_finite() { depth } else { 0.0 },
            },
            (3, mode @ 0..=2) => Self::Ring {
                extent: record.extent,
                spread: if depth.is_finite() { depth } else { 0.0 },
                mode,
            },
            (4 | 7, mode) => Self::Sphere {
                extent: record.extent,
                spread: if depth.is_finite() { depth } else { 0.0 },
                mode,
            },
            _ => Self::Point,
        }
    }

    /// The spawn offset in world space.
    ///
    /// `scale` is the product that multiplies `+0x34` - severity, extent
    /// co-factor and emission-scale channel together. `direction` is the
    /// particle's own flight direction, which a sphere places it along;
    /// `across` and `up` are the emitter frame's world-space `X` and `Y`.
    /// Draws from `rng` only for the shapes that are not a point, so an
    /// effect made only of points consumes exactly what it did before.
    pub fn offset(
        self,
        scale: f32,
        direction: Vec3,
        across: Vec3,
        up: Vec3,
        rng: &mut Rng,
    ) -> Vec3 {
        let signed = |rng: &mut Rng| rng.next_f32() * 2.0 - 1.0;
        match self {
            Self::Point => Vec3::ZERO,
            Self::Line { extent, depth } => {
                let e = (extent * scale).max(1e-5);
                let x = e * signed(rng);
                let z = depth * signed(rng);
                across * x + across.cross(up) * z
            }
            Self::Ring {
                extent,
                spread,
                mode,
            } => {
                let e = (extent * scale).max(1e-5);
                let z_axis = across.cross(up);
                if mode >= 2 {
                    // A point of the square until it is in the circle: the original's own
                    // rejection loop, so the disc is uniform.
                    loop {
                        let (x, z) = (e * signed(rng), e * signed(rng));
                        if x * x + z * z <= e * e {
                            return across * x + z_axis * z;
                        }
                    }
                }
                let radius = if mode == 1 {
                    e + spread * scale * signed(rng)
                } else {
                    e
                };
                let phi = rng.next_f32() * std::f32::consts::TAU;
                across * (radius * phi.cos()) + z_axis * (radius * phi.sin())
            }
            Self::Sphere {
                extent,
                spread,
                mode,
            } => {
                let e = (extent * scale).max(1e-5);
                let radius = match mode {
                    0 => e,
                    1 => e + spread * scale * signed(rng),
                    2 => e * (rng.next_f32() * std::f32::consts::FRAC_PI_2).sin(),
                    _ => 0.0,
                };
                direction * radius
            }
        }
    }
}

/// A new particle's flight direction and spawn offset, in that order of dependence.
///
/// A ring or a disc under an **aimed** velocity (every shape-3 emitter with an extent
/// authors velocity mode 1) places the particle first and aims it along the spawn
/// offset's own heading: `FUN_088fc634` hands `ParticleSystem_AimedVelocity` the
/// normalised `(x, 0, z)` it just wrote, so the smoke leaves the ring outward rather
/// than in a direction drawn apart from where it was born. Every other pairing draws
/// the direction first, as it always did (a sphere places the particle along it, and
/// a tangent sphere stays at the anchor).
#[must_use]
pub(super) fn place(
    spawn: Spawn,
    direction: super::Direction,
    scale: f32,
    across: Vec3,
    up: Vec3,
    rng: &mut Rng,
) -> (Vec3, Vec3) {
    if let (
        Spawn::Ring { .. },
        super::Direction::Aimed {
            elevation,
            azimuth,
            jitter,
        },
    ) = (spawn, direction)
    {
        let offset = spawn.offset(scale, Vec3::ZERO, across, up, rng);
        let radial = offset.try_normalize().unwrap_or(across);
        let elevation = elevation + jitter * (rng.next_f32() * 2.0 - 1.0);
        let (sin_a, cos_a) = azimuth.sin_cos();
        let heading = radial * cos_a + up.cross(radial) * sin_a;
        return (heading * elevation.cos() + up * elevation.sin(), offset);
    }
    let aim = super::direction_for(direction, up, rng);
    let placement = match (spawn, direction) {
        (Spawn::Sphere { .. }, super::Direction::Tangent { .. }) => Spawn::Point,
        (other, _) => other,
    };
    (aim, placement.offset(scale, aim, across, up, rng))
}

/// `across` made perpendicular to `up`, falling back to the frame's own
/// horizontal when the two are parallel - the frame's `X` axis.
#[must_use]
pub fn frame_x(across: Vec3, up: Vec3) -> Vec3 {
    (across - up * across.dot(up))
        .try_normalize()
        .unwrap_or_else(|| up.any_orthonormal_vector())
}

#[cfg(test)]
mod tests;
