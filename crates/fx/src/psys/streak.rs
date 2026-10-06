//! A two-point particle's geometry: draw classes 6 and 7.
//!
//! `ParticleSystem_DrawParticle` (`0x089186bc`) hands both classes the
//! particle's half-size (`+0x30`), its stretch (`+0x64`, a hard-coded
//! `1.0`), its previous or spawn point (`+0x50`, called `origin` here) and
//! its position, both in view space. Each class then builds its own strip,
//! read off the executable on 2026-09-24:
//!
//! - **Class 6, `ParticleSystem_DrawStreak` (`0x08916820`): a wedge.** With
//!   `dir = normalize(origin - position)`, `perp = (dir.y, -dir.x) * half`
//!   and `cap = dir * half`, four strip vertices:
//!
//!   | # | position | `u` | `v` |
//!   | --- | --- | --- | --- |
//!   | 0 | `position - cap - perp` | 1 | 0 |
//!   | 1 | `position - cap + perp` | 0 | 0 |
//!   | 2 | `position + cap - perp` | 1 | 1 |
//!   | 3 | `origin + cap + perp` | 0 | 1 |
//!
//!   Vertex 2 sits by the **position**, not the origin (`lv.q C200, 0(s1)`
//!   at `0x08916ba0`, `s1` being the position argument), so the quad is not
//!   a rectangle: a sprite-sized head at the particle and a sliver out to
//!   the origin. A zero-length streak collapses to a plain square.
//! - **Class 6 on an emitter's own particle, `ParticleSystem_DrawPoolBars`
//!   (`0x08917c7c`): a bar, not the wedge.** `ParticleSystem_DrawEmitterPool`
//!   sends the pool's class 6 here; the wedge above is the template path only,
//!   and no PSP template is class 6, so every class-6 particle on the disc is a
//!   bar. With `dir = normalize(origin - position)` (screen `(0, 1)` at zero
//!   length), `perp = (dir.y, -dir.x) * size` and `cap = dir * size * aspect`
//!   (`res+0x4c8`, `vscl.p C400, C400, S701` at `0x08917f7c`), the corners are
//!   `position - cap -+ perp` and `origin + cap -+ perp`: a rectangle from end to
//!   end, `2 size` wide, reaching `aspect * size` past each end. `u` is 1 on the
//!   `-perp` edge, `v` 0 at the position and 1 at the origin (the UV words at
//!   `+0x10..+0x1c` of `FUN_08917358`'s frame table). So a particle that has not
//!   moved is a sliver `2 size` across and `2 aspect size` tall - `WO_REPULSER`'s
//!   aspect `0.05` makes it a thin horizontal streak. Confidence 82, static.
//! - **Class 7, `ParticleSystem_DrawCappedStreak` (`0x08916d00`): a capped
//!   bar.** Eight vertices, `u` 1 on the `-perp` edge and 0 on the `+perp`
//!   one, and `v` `0 -> 0.5` over the cap beyond the position, a constant
//!   `0.5` along the body and `0.5 -> 1` over the cap beyond the origin.
//!   The body samples one texture row, stretched.
//!
//! Both sample the emitter's own sprite and atlas cell the way a billboard
//! does ([`super::sprite`]). Class 7's eight vertices are drawn here as one
//! quad whose along-coordinate `psys.wesl` folds back into the same
//! piecewise `v`, which is exact because `v` is linear in that coordinate on
//! each of the three spans, and keeps a streak to the six vertices
//! [`super::MAX_VERTICES`] budgets for.
//!
//! **Built in world space, not view space - chosen, not measured.** The
//! original pushes both endpoints to one depth before building the strip
//! (the farther, `min(z)`, for class 6; the mean for class 7), so the strip
//! is screen-parallel with a width that is `half` at that depth. Here the
//! perpendicular is `dir x (right x up)`, which is the view-space
//! `(dir.y, -dir.x)` for a streak lying in the screen plane, and each end
//! keeps its own depth. No eye position reaches this code to do otherwise.
//!
//! **Pulse on the PSP only.** A source whose executable is unread keeps the
//! procedural profile - see [`super::Effect::without_pulse_psp_draw`].

use oag_core::math::Vec3;

use super::{Effect, EmitterSpec, Particle, quad, sprite};
use oag_mesh::mesh::GpuVertex;

impl Effect {
    /// Draws every streak with the procedural profile again and keeps every
    /// particle on its spawn frame, the way every source did before Pulse's
    /// own strips ([this module](self)) and frame advance
    /// ([`super::frames`]) were read. Also gives back the run law
    /// ([`EmitterSpec::short_run`]), read on the same executable.
    ///
    /// **For every source but Pulse on the PSP, by choice (2026-09-24)**,
    /// the same line [`Self::without_extents`] draws: the strips are read
    /// off Pulse's PSP `BOOT.BIN` alone, and Pure's is a different
    /// executable that ships sprites of its own.
    pub fn without_pulse_psp_draw(&mut self) {
        for spec in &mut self.emitters {
            spec.streak = StreakDraw::Procedural;
            spec.frames = super::FrameAdvance::Still;
            spec.short_run = false;
        }
        self.mute_templates();
        self.mute_rotation();
    }
}

/// Which strip a two-point particle is built as.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreakDraw {
    /// One rectangle with the procedural profile in `psys.wesl`, whatever
    /// the class - every source but Pulse on the PSP.
    Procedural,
    /// Class 6 on a template, the wedge.
    Wedge,
    /// Class 6 on an emitter's own particle, the bar, with the record's
    /// `res+0x4c8` as the cap's share of the size.
    Bar {
        /// `res+0x4c8`.
        aspect: f32,
    },
    /// Class 7, the capped bar.
    Capped,
}

impl StreakDraw {
    /// The strip an emitter's own particle of draw class `class` builds;
    /// [`Self::Procedural`] for anything but 6 and 7. A template's class 6 is
    /// [`Self::for_template`].
    #[must_use]
    pub fn of(class: Option<u32>, aspect: f32) -> Self {
        match class {
            Some(6) => Self::Bar { aspect },
            Some(7) => Self::Capped,
            _ => Self::Procedural,
        }
    }
}

impl StreakDraw {
    /// The same class drawn as a template (`ParticleSystem_DrawParticle`), whose
    /// class 6 is the wedge.
    #[must_use]
    pub fn for_template(self) -> Self {
        match self {
            Self::Bar { .. } => Self::Wedge,
            other => other,
        }
    }
}

/// `psys.wesl`'s `normal.x` for a class-7 bar: sample the sheet, and fold
/// the along-coordinate into the capped `v`.
const CAPPED: f32 = 2.0;

/// Appends one streak particle's six vertices to `out`.
pub(super) fn extend(
    out: &mut Vec<GpuVertex>,
    spec: &EmitterSpec,
    particle: &Particle,
    half: f32,
    rgba: [f32; 4],
    right: Vec3,
    up: Vec3,
) {
    let [r, g, b, alpha] = rgba;
    let colour = [r, g, b];
    let back = right.cross(up);
    let rect = spec
        .sheet_rect
        .filter(|_| spec.streak != StreakDraw::Procedural);
    let Some(rect) = rect else {
        out.extend_from_slice(&procedural(particle, half, colour, alpha, right, up, back));
        return;
    };
    let cell = spec.atlas.cell(rect, particle.frame);
    let along = particle.origin - particle.position;
    let length = along.length();
    // `(0, 1, 0)` in view space for a zero-length streak: the camera's up.
    let dir = if length > 1e-6 { along / length } else { up };
    let perp = dir.cross(back).try_normalize().unwrap_or(right) * half;
    let cap = dir * half;
    let (position, origin) = (particle.position, particle.origin);
    let corner = |at: Vec3, u: f32, v: f32| GpuVertex {
        position: at.to_array(),
        normal: [0.0, 0.0, 0.0],
        colour: [colour[0], colour[1], colour[2], alpha],
        texcoord: [u, v],
        ..bytemuck::Zeroable::zeroed()
    };
    if let StreakDraw::Bar { aspect } = spec.streak {
        let cap = cap * aspect;
        let mut strip = [
            corner(position - cap - perp, 1.0, 0.0),
            corner(position - cap + perp, 0.0, 0.0),
            corner(origin + cap - perp, 1.0, 1.0),
            corner(origin + cap + perp, 0.0, 1.0),
        ];
        sprite::map_to_cell(&mut strip, cell);
        out.extend_from_slice(&[strip[0], strip[1], strip[2], strip[1], strip[3], strip[2]]);
        return;
    }
    if spec.streak == StreakDraw::Wedge {
        let mut strip = [
            corner(position - cap - perp, 1.0, 0.0),
            corner(position - cap + perp, 0.0, 0.0),
            corner(position + cap - perp, 1.0, 1.0),
            corner(origin + cap + perp, 0.0, 1.0),
        ];
        sprite::map_to_cell(&mut strip, cell);
        out.extend_from_slice(&[strip[0], strip[1], strip[2], strip[1], strip[3], strip[2]]);
        return;
    }
    // Class 7: `u` is final, `v` carries the along-coordinate `0..1` from
    // the cap past the position to the cap past the origin, and the cell's
    // `v` range and the caps' share of the length ride along for the shader.
    let total = length + 2.0 * half;
    let share = if total > 0.0 { half / total } else { 0.5 };
    let bar = |at: Vec3, u: f32, s: f32| GpuVertex {
        normal: [CAPPED, cell[1], cell[3]],
        lit: share,
        ..corner(at, cell[0] + (cell[2] - cell[0]) * u, s)
    };
    let a0 = bar(position - cap - perp, 1.0, 0.0);
    let a1 = bar(position - cap + perp, 0.0, 0.0);
    let b0 = bar(origin + cap - perp, 1.0, 1.0);
    let b1 = bar(origin + cap + perp, 0.0, 1.0);
    out.extend_from_slice(&[a0, a1, b0, a1, b1, b0]);
}

/// The profile every source drew before the strips were read: one
/// rectangle spanning both points plus a `half`-sized cap at each end.
fn procedural(
    particle: &Particle,
    half: f32,
    colour: [f32; 3],
    alpha: f32,
    right: Vec3,
    up: Vec3,
    back: Vec3,
) -> [GpuVertex; 6] {
    let centre = (particle.position + particle.origin) * 0.5;
    let along = particle.position - particle.origin;
    let length = along.length();
    let dir = if length > 1e-6 { along / length } else { up };
    // Perpendicular to the streak in the camera plane; degenerate when the
    // streak points at the camera, and then `right`.
    let perp = dir.cross(back).try_normalize().unwrap_or(right);
    let half_span = length * 0.5 + half;
    quad(
        centre,
        dir * half_span,
        perp * half,
        half / half_span,
        colour,
        alpha,
    )
}
