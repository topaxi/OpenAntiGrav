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
//! **View space when the camera is known.** The original pushes both endpoints
//! onto one depth along the line to the eye before building the strip (the
//! mean of the two for the pool bar and class 7, the farther for the template
//! wedge), so the strip is screen-parallel and its width is `half` at that
//! depth; [`at_one_depth`] does it, and the perpendicular is then exactly the
//! view-space `(dir.y, -dir.x)`. Without a camera (`view` is `None`: a source
//! the strips were not read on, and the pools whose caller passes none) the
//! perpendicular is `dir x (right x up)` and each end keeps its own depth,
//! which is the same for a streak lying in the screen plane - chosen, not
//! measured.
//!
//! **Pulse on the PSP only.** A source whose executable is unread keeps the
//! procedural profile - see [`super::Effect::without_pulse_psp_draw`].

use oag_core::math::Vec3;

use super::between_ticks::set_shift;
use super::guard::GuardBand;
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
        self.reset_pulse_psp_draw(Kept::NONE);
    }

    /// [`Self::without_pulse_psp_draw`], except for the laws `kept` names: an emitter's own
    /// class-6 particle keeps the bar [`Kept::bar`] names (HD's view-space bar,
    /// [`StreakDraw::FarBar`], [`KeptBar::Far`], the draw HD's `PartRender` job and Omega's CPU
    /// twin build for render mode 5, `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "The
    /// class-6 draw law"; or the pool bar Pulse's and Pure's PSP executables build,
    /// [`StreakDraw::Bar`], [`KeptBar::Mean`], Pure's `FUN_088746c8` reading
    /// `ParticleSystem_DrawPoolBars`'s strip law,
    /// `docs/ghidra/functions/psp-pure-usa/particle-bars.md`), and the sprite templates play when
    /// [`Kept::templates`] is set. Per effect, for the ones a title lists
    /// (`oag_title::Effects::{far_bars, pool_bars, templates}`): each law was read on a named
    /// effect alone.
    pub fn without_pulse_psp_draw_but(&mut self, kept: Kept) {
        self.reset_pulse_psp_draw(kept);
    }

    fn reset_pulse_psp_draw(&mut self, kept: Kept) {
        self.rides_owner_frame = false;
        for spec in &mut self.emitters {
            spec.under_alpha = false;
            spec.streak = match (spec.streak, kept.bar) {
                (StreakDraw::Bar { aspect }, KeptBar::Far) => StreakDraw::FarBar { aspect },
                (StreakDraw::Bar { aspect }, KeptBar::Mean) => StreakDraw::Bar { aspect },
                _ => StreakDraw::Procedural,
            };
            spec.frames = super::FrameAdvance::Still;
            spec.short_run = false;
        }
        if !kept.templates {
            self.mute_templates();
        }
        self.mute_rotation(kept.templates);
    }
}

/// Which Pulse PSP draw laws an effect keeps when [`Effect::without_pulse_psp_draw_but`] takes
/// the rest away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kept {
    /// The class-6 bar.
    pub bar: KeptBar,
    /// The sprite templates, played with their own roll and stretch.
    pub templates: bool,
}

impl Kept {
    /// Nothing: [`Effect::without_pulse_psp_draw`].
    pub const NONE: Self = Self {
        bar: KeptBar::No,
        templates: false,
    };

    /// Only the class-6 bar `bar` names.
    #[must_use]
    pub const fn bar(bar: KeptBar) -> Self {
        Self {
            bar,
            templates: false,
        }
    }
}

/// Which class-6 bar an effect keeps: see [`Kept`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptBar {
    /// None: every streak is procedural.
    No,
    /// The pool bar, both ends on the mean of their two view depths ([`StreakDraw::Bar`]).
    Mean,
    /// HD's bar, both ends on the farther depth ([`StreakDraw::FarBar`]).
    Far,
}

impl Effect {
    /// Draws the root emitter in the layer before the alpha-over particles, as the
    /// original's draw list for `WO_PLASMA_HEAD` does (arcs, then the dark disc over
    /// them, then ring, spikes, glow: `docs/ghidra/functions/psp-pulse-usa/plasma.md`,
    /// "the head's draw order"). Pulse's PSP file only; [`Self::without_pulse_psp_draw`]
    /// takes it back.
    pub fn draw_root_under_alpha(&mut self) {
        if let Some(root) = self.emitters.first_mut() {
            root.under_alpha = true;
        }
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
    /// Class 6 on HD's renderer (render mode 5): the same rectangle as
    /// [`Self::Bar`], with both ends on the **farther** of their two view
    /// depths where Pulse's takes the mean. With no sprite placed it is
    /// shaded with the procedural profile, standing in for the texture the
    /// original binds (`record+0xca4`, unread).
    FarBar {
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
    /// What an emitter's authored colour is multiplied by for this draw, given its
    /// `kColourScale` (`+0xc84`): HD's vertex program decodes the colour with a 2.2 power and
    /// multiplies the linear result by the scale, so the same product through our 2.2 decode
    /// (`target.wesl`'s `decode`) is the authored colour times `scale^(1/2.2)`. Only a
    /// [`Self::FarBar`] takes it. **Inherited from Omega**, whose CPU renderer reads the word
    /// as `kColourScale`; HD's own reader of it is unread
    /// (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "The class-6 draw law").
    #[must_use]
    pub fn colour_gain(self, scale: f32) -> f32 {
        match self {
            Self::FarBar { .. } => scale.max(0.0).powf(1.0 / 2.2),
            _ => 1.0,
        }
    }

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

/// Both ends of a two-point particle moved onto one depth along the line to
/// the eye, which keeps each on its own screen point and makes the pair
/// co-planar, so the strip built in that plane is screen-parallel
/// (`docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "`FUN_08917c7c`"
/// and "`FUN_08916d00`"): the mean of the two depths, except the template
/// wedge and HD's class-6 bar, which take the farther. Returned as `(position, origin)` in world
/// space. A particle at or behind the eye keeps its own points.
fn at_one_depth(view: &GuardBand, draw: StreakDraw, particle: &Particle) -> (Vec3, Vec3) {
    let (position, origin) = (particle.position, particle.origin);
    let depth = |p: Vec3| (p - view.eye).dot(view.forward);
    let (dp, dor) = (depth(position), depth(origin));
    if dp < 1e-3 || dor < 1e-3 {
        return (position, origin);
    }
    let common = match draw {
        StreakDraw::Wedge | StreakDraw::FarBar { .. } => dp.max(dor),
        _ => (dp + dor) * 0.5,
    };
    let onto = |p: Vec3, d: f32| view.eye + (p - view.eye) * (common / d);
    (onto(position, dp), onto(origin, dor))
}

/// `psys.wesl`'s `normal.x` for a class-7 bar: sample the sheet, and fold
/// the along-coordinate into the capped `v`.
const CAPPED: f32 = 2.0;

/// Appends one streak particle's six vertices to `out`, each carrying its
/// tick move for the vertex shader ([`super::between_ticks`]).
///
/// The head moves with the particle. The tail is the previous tick's position
/// and moves the same way, except under `from_spawn`, where it is the spawn
/// point and stays put; a vertex between the two ends takes its share of each
/// by where it lies along the streak.
pub(super) fn extend(
    out: &mut Vec<GpuVertex>,
    spec: &EmitterSpec,
    particle: &Particle,
    half: f32,
    rgba: [f32; 4],
    (right, up): (Vec3, Vec3),
    view: Option<&GuardBand>,
) {
    let start = out.len();
    extend_at_tick(out, spec, particle, half, rgba, (right, up), view);
    let head = particle.shift();
    let from_spawn = matches!(spec.render, super::Render::Streak { from_spawn: true });
    let along = particle.position - particle.origin;
    let length2 = along.length_squared();
    for vertex in &mut out[start..] {
        let shift = if from_spawn && length2 > 1e-12 {
            let at = (Vec3::from(vertex.position) - particle.origin).dot(along) / length2;
            head * at.clamp(0.0, 1.0)
        } else if from_spawn {
            Vec3::ZERO
        } else {
            head
        };
        set_shift(vertex, shift);
    }
}

/// [`extend`]'s six vertices at the tick, with no move.
fn extend_at_tick(
    out: &mut Vec<GpuVertex>,
    spec: &EmitterSpec,
    particle: &Particle,
    half: f32,
    rgba: [f32; 4],
    (right, up): (Vec3, Vec3),
    view: Option<&GuardBand>,
) {
    let [r, g, b, alpha] = rgba;
    let colour = [r, g, b];
    let back = right.cross(up);
    let rect = spec
        .sheet_rect
        .filter(|_| spec.streak != StreakDraw::Procedural);
    let Some(rect) = rect else {
        match spec.streak {
            // HD's bar with no sprite placed: the bar's corners, shaded with
            // the procedural profile.
            StreakDraw::FarBar { aspect } => {
                let (position, origin, dir, perp) =
                    strip_frame(spec, particle, half, (right, up), view);
                let reach = half * aspect;
                let half_span = (origin - position).length() * 0.5 + reach;
                out.extend_from_slice(&quad(
                    (position + origin) * 0.5,
                    dir * half_span,
                    perp,
                    if half_span > 0.0 {
                        reach / half_span
                    } else {
                        0.5
                    },
                    colour,
                    alpha,
                ));
            }
            _ => out.extend_from_slice(&procedural(particle, half, colour, alpha, right, up, back)),
        }
        return;
    };
    let (position, origin, dir, perp) = strip_frame(spec, particle, half, (right, up), view);
    let cap = dir * half;
    let length = (origin - position).length();
    let cell = spec.atlas.cell(rect, particle.frame);
    let corner = |at: Vec3, u: f32, v: f32| GpuVertex {
        position: at.to_array(),
        normal: [0.0, 0.0, 0.0],
        colour: [colour[0], colour[1], colour[2], alpha],
        texcoord: [u, v],
        ..bytemuck::Zeroable::zeroed()
    };
    if let StreakDraw::Bar { aspect } | StreakDraw::FarBar { aspect } = spec.streak {
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

/// A strip's two ends and its frame: `(position, origin, dir, perp)`, the ends
/// on one depth when the camera is known ([`at_one_depth`]), `dir` along the
/// strip and `perp` across it with length `half`.
fn strip_frame(
    spec: &EmitterSpec,
    particle: &Particle,
    half: f32,
    (right, up): (Vec3, Vec3),
    view: Option<&GuardBand>,
) -> (Vec3, Vec3, Vec3, Vec3) {
    let back = right.cross(up);
    let (position, origin) = match view {
        Some(view) => at_one_depth(view, spec.streak, particle),
        None => (particle.position, particle.origin),
    };
    // `(0, 1, 0)` in view space for a zero-length streak: the camera's up.
    let (dir, perp) = if view.is_some() {
        let along = (origin - position) - back * (origin - position).dot(back);
        let length = along.length();
        let dir = if length > 1e-6 { along / length } else { up };
        (dir, dir.cross(back) * half)
    } else {
        let along = origin - position;
        let length = along.length();
        let dir = if length > 1e-6 { along / length } else { up };
        (dir, dir.cross(back).try_normalize().unwrap_or(right) * half)
    };
    (position, origin, dir, perp)
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
