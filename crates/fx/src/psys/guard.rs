//! What the PSP does with a particle near the eye or off its screen, in two halves: the
//! executable's own depth tests by draw class, and the GE's drop of a triangle with a vertex
//! outside its 4096-wide screen space.
//!
//! **The executable** (`docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "Near the eye:
//! what the PSP does with an emitted particle"):
//!
//! - an emitter's square (draw classes 1 and 2): none nearer than 1 unit, alpha times
//!   `depth - 1` up to 2 units;
//! - a rolled quad (class 3): no test at all;
//! - a bar (classes 6 and 7): none if either end is nearer than 1 unit;
//! - a sprite template: none nearer than 1 unit.
//!
//! **The GE**, measured on PPSSPP's software renderer by editing one emitted bar of a recorded
//! frame and replaying it (same section): a triangle with a vertex more than [`LIMIT_PX`] from
//! the screen centre is not drawn, however much of it covers the screen, and the other triangle
//! of the same quad still draws. A vertex nearer than the near plane is clipped there instead,
//! so a screen-parallel quad at or inside [`NEAR_PLANE`] draws nothing.
//!
//! The reference is PPSSPP's rasteriser, not a PSP. The fixed projection is a standstill race
//! frame's: Pulse's race camera was not checked for a speed fov. Which sources run this is
//! `oag_title::Looks::psp_particle_clip`.
//!
//! **A sprite template meets the GE drop only on `WO_REPULSER`** ([`TEMPLATE_GUARDED`]):
//! dropping the Quake's moved its start frame away from the original's, which is a white wash
//! there. Until the Quake's own quad is measured, templates elsewhere keep every triangle.

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use super::between_ticks::shift_of;
use super::near::NearEye;
use super::{Effect, EmitterSpec, Particle};

/// Pixels from the screen centre a vertex may sit and still be drawn.
const LIMIT_PX: f32 = 2048.0;

/// The PSP's own projection scale in pixels per unit of depth: the frame's
/// projection `0.981` and `1.732` times the viewport's `240` and `136`.
const PX_PER_DEPTH: [f32; 2] = [240.0 * 0.981, 136.0 * 1.732];

/// The race camera's near plane in world units, from the projection in a race frame
/// (`m10 = -1.0012`, `m14 = -2.4849`).
pub const NEAR_PLANE: f32 = 1.2417;

/// The CPU draws no square, bar or template nearer than this.
const CPU_CULL: f32 = 1.0;

/// An emitter's square is faded from [`CPU_CULL`] up to this.
const FADE_END: f32 = 2.0;

/// The one effect whose sprite templates meet the GE drop ([the module](self)).
const TEMPLATE_GUARDED: &str = "WO_REPULSER";

/// Which of the PSP's pool routines draws a particle: the record's draw class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawClass {
    /// Classes 1 and 2, `ParticleSystem_DrawPoolSquares`.
    Square,
    /// Class 3, `ParticleSystem_DrawRolledQuads`.
    Rolled,
    /// Classes 6 and 7, `ParticleSystem_DrawPoolBars` and its capped twin.
    Bar,
}

impl DrawClass {
    /// The routine for a record's draw class. A class with no routine here is a square: the
    /// parser rejects it before a particle of it exists.
    #[must_use]
    pub fn of(class: Option<u32>) -> Self {
        match class {
            Some(3) => Self::Rolled,
            Some(6 | 7) => Self::Bar,
            _ => Self::Square,
        }
    }
}

/// Where the camera is and how it is turned, for the screen-range test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardBand {
    pub eye: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

impl GuardBand {
    /// Whether the GE would drop a primitive with these corners.
    #[must_use]
    pub fn drops(&self, corners: impl IntoIterator<Item = Vec3>) -> bool {
        corners.into_iter().any(|corner| {
            let offset = corner - self.eye;
            let depth = offset.dot(self.forward);
            // A vertex inside the near plane is clipped there, never range-tested.
            depth > NEAR_PLANE
                && [(self.right, PX_PER_DEPTH[0]), (self.up, PX_PER_DEPTH[1])]
                    .into_iter()
                    .any(|(axis, scale)| (offset.dot(axis) * scale / depth).abs() > LIMIT_PX)
        })
    }
}

impl Effect {
    /// Runs this effect's particles under the PSP's near tests and the GE's screen range.
    pub fn with_psp_clip(&mut self) {
        self.psp_clip = true;
    }
}

/// The CPU's depth tests: a particle's alpha under them, or `None` when it draws nothing.
/// Unchanged when the effect does not run them or the build knows no eye.
pub(super) fn depth_law(
    effect: &Effect,
    spec: &EmitterSpec,
    particle: &Particle,
    eye: Option<NearEye>,
    alpha: f32,
) -> Option<f32> {
    let Some(eye) = eye.filter(|_| effect.psp_clip) else {
        return Some(alpha);
    };
    let depth = eye.depth(particle);
    if spec.draw_class == DrawClass::Bar {
        let far_end = (particle.origin - eye.eye).dot(eye.forward);
        let kept = depth >= CPU_CULL && far_end >= CPU_CULL && (depth + far_end) * 0.5 > NEAR_PLANE;
        return kept.then_some(alpha);
    }
    if depth <= NEAR_PLANE {
        return None;
    }
    let faded = !spec.template && spec.draw_class == DrawClass::Square && depth < FADE_END;
    Some(if faded {
        alpha * (depth - CPU_CULL)
    } else {
        alpha
    })
}

/// The GE's drop, on the quads one particle appended to `out` from `start`: six vertices, two
/// triangles each. A dropped triangle is collapsed onto a vertex of its neighbour, so the list
/// keeps six a quad and holds no vertex out of range; a quad that loses both is removed.
pub(super) fn drop_triangles(
    effect: &Effect,
    spec: &EmitterSpec,
    eye: Option<NearEye>,
    (right, up): (Vec3, Vec3),
    out: &mut Vec<GpuVertex>,
    start: usize,
) {
    let Some(eye) = eye.filter(|_| effect.psp_clip) else {
        return;
    };
    if spec.template && effect.name != TEMPLATE_GUARDED {
        return;
    }
    let view = GuardBand {
        eye: eye.eye,
        right,
        up,
        forward: eye.forward,
    };
    let drawn = |v: &GpuVertex| Vec3::from(v.position) + shift_of(v) * (eye.draw_alpha - 1.0);
    let mut at = start;
    while at + 6 <= out.len() {
        let first = view.drops(out[at..at + 3].iter().map(drawn));
        let second = view.drops(out[at + 3..at + 6].iter().map(drawn));
        match (first, second) {
            (true, true) => {
                out.drain(at..at + 6);
                continue;
            }
            (true, false) => {
                let kept = out[at + 3];
                out[at..at + 3].fill(kept);
            }
            (false, true) => {
                let kept = out[at];
                out[at + 3..at + 6].fill(kept);
            }
            (false, false) => {}
        }
        at += 6;
    }
}

#[cfg(test)]
mod tests;
