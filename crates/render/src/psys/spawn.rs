//! Where in the emitter a particle is born - the emitter extent.
//!
//! `ParticleSystem_SpawnBurst` switches on the emitter's shape (`+0x30`) and
//! each emit function places the particle's spawn offset in the emitter's own
//! frame before `ParticleSystem_InitParticle` carries it through the node
//! matrix. Two shapes are read and implemented here; the rest stay at the
//! anchor, as every shape did before:
//!
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
