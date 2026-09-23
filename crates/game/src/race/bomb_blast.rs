//! The Bomb's own detonation: the hemisphere and the shockwave
//! `BombBlast_Construct` loads, plus `WO_BOMB_SMOKERING` - the psys effect
//! that already plays through [`super::weapons::visuals`]'s ordinary
//! [`super::effect_names::BOMB_SMOKERING_EFFECT`] mechanism and needs no
//! render-side state of its own. See
//! `docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-23-the-blasts-own-per-tick-animator-read`
//! for the full read this plays back - `BombBlast_Construct` (`0x08872078`)
//! and `BombBlast_Update` (`0x0887250c`), confidence 90/85.
//!
//! **Pulse (PSP) only.** Wipeout HD's own Bomb detonation is unread and
//! Pure's Bomb authors no `timetodie` at all - see `mine.md`'s own
//! `Drop::bomb` doc comment - so `bomb_blast_pulse` is `None` on both other
//! titles' [`oag_title::weapons::WeaponModels`] and nothing here fires.
//!
//! **Render-side view state only**, on [`super::blast_models`]'s own terms:
//! a blast's own position and age never move a determinism hash, because the
//! simulation itself only ever produces one [`oag_gameplay::projectile::Impact`]
//! at the moment of detonation - everything after that is presentation. See
//! that module's own doc comment for the fuller argument.
//!
//! This is a new top-level module of `race`, not a child of `race::weapons`
//! or of `blast_models` - the same reasoning `blast_models`'s own doc comment
//! gives, and doubly true here: `blast_models.rs` is the Plasma's own file,
//! landed by a concurrent pass on this same lane structure, and a second
//! weapon's state does not belong inside its container.
//!
//! # What is recovered, and what this engine substitutes
//!
//! **Recovered, confidence 85-90, direct decompile of both functions plus a
//! live-memory read of every constant below.** The two models' own transform
//! math, the three eases (`hemisphere_scale`, `shockwave_scale`,
//! `shockwave_alpha`) and their rates, the `0.1 s` gate on the shockwave's
//! own growth, the `1.55 s` hemisphere hide and the `4.0 s` object retire are
//! all read off `BombBlast_Update`'s own decompilation, corroborated against
//! `BombBlast_Construct`'s own writes of the same offsets. See `mine.md`.
//!
//! **Substituted, chosen rather than measured, no confidence score - carried
//! from [`oag_gameplay::projectile::mine::frozen_pose`]'s own hedge:** the
//! basis this engine builds crosses the frozen craft orientation's own
//! forward axis (`orientation * Vec3::NEG_Z`) against this engine's world up
//! (`Vec3::Y`), in place of the unlocated rear-emitter's own `+0x60` row
//! crossed against the executable's own `(0, 0, 1)` reference vector. Both
//! substitutions are the same "PSP asset space to this engine's own space"
//! translation every other ported basis in this codebase already makes (see
//! `blast_models::billboard_matrix`'s own `Vec3::Y` reference), not a fresh
//! reading of either constant.
//!
//! **Not wired: the shockwave's own fading alpha.** `BombBlast_Update`
//! writes a white, fading vertex colour onto the shockwave's own submeshes
//! every tick (`+0xf4` easing `1.0 -> 0.0`) - recovered, and left unapplied
//! here. [`Drawable::tint`] is this engine's equivalent call, but it wants a
//! `&mut Vec<GpuVertex>` scratch buffer this pool does not carry, and
//! `Scene`'s own struct is at its file's line ceiling - see this crate's
//! `CLAUDE.md` on `scripts/check-file-size.py`. The shockwave draws at full
//! authored opacity for its own `4.0 s` lifetime and then disappears outright
//! with the rest of the object, rather than fading - a recovered mechanism
//! partially applied, not an invented one.

use super::*;

/// How many render-side Bomb-blast instances can be live at once.
///
/// [`oag_gameplay::projectile::MAX_PROJECTILES`], the same bound
/// [`blast_models::PLASMA_BLAST_SLOTS`] takes and for the same reason: it is
/// this engine's own projectile-pool cap, not a measured figure for how many
/// the original itself keeps around (its own object is heap-allocated per
/// detonation, with no fixed pool this project has found).
pub(super) const BOMB_BLAST_SLOTS: usize = oag_gameplay::projectile::MAX_PROJECTILES;

/// `BombBlast_Update`'s own hardcoded object retire time - the whole blast,
/// hemisphere, shockwave and all, is torn down here. Confidence 85: a `4.0`
/// immediate compared directly against `+0xd0` (age), two instructions before
/// the teardown call.
pub(super) const LIFETIME_SECONDS: f32 = 4.0;

/// When the hemisphere's own node hides - confidence 85, the same
/// decompilation as [`LIFETIME_SECONDS`]. The shockwave and the object itself
/// live on past this point.
pub(super) const HEMISPHERE_HIDE_AT_SECONDS: f32 = 1.55;

/// The hemisphere's own uniform scale ease: `(start, target, rate)`, ticked
/// every frame from spawn with no gate. Read off `BombBlast_Construct`'s own
/// stores at `+0xdc`/`+0xe0`/`+0xe4`.
pub(super) const HEMISPHERE_SCALE: Ease = Ease {
    start: 2.0,
    target: 4.0,
    rate: 0.1,
};

/// The shockwave's own *radial* scale ease - applied to two of its three
/// basis axes only, see [`bomb_blast_basis`] - gated to start only once
/// [`SHOCKWAVE_SCALE_DELAY_SECONDS`] has passed. Read off `+0xe8`/`+0xec`/
/// `+0xf0`.
pub(super) const SHOCKWAVE_SCALE: Ease = Ease {
    start: 0.0,
    target: 12.0,
    rate: 0.075,
};

/// See [`SHOCKWAVE_SCALE`]. Read off the `if (0.1 < age)` guard wrapping that
/// ease alone in `BombBlast_Update`.
pub(super) const SHOCKWAVE_SCALE_DELAY_SECONDS: f32 = 0.1;

/// The shockwave's own fading alpha ease, `+0xf4`/`+0xf8`/`+0xfc` -
/// **recovered but not drawn**, see this module's own doc comment. Reached
/// only by `bomb_blast::tests::the_shockwave_alpha_ease_fades_from_opaque_to_nothing`
/// until a `Drawable::tint` scratch buffer lands for this pool, hence the
/// `allow` - a real caller, not a lint silenced for its own sake.
#[allow(dead_code)]
pub(super) const SHOCKWAVE_ALPHA: Ease = Ease {
    start: 1.0,
    target: 0.0,
    rate: 0.1,
};

/// How far the shockwave sits from the hemisphere, pulled back along the
/// basis's own `dir` axis - `DAT_08ab1070`, read directly as `1.0`.
pub(super) const SHOCKWAVE_PULLBACK: f32 = 1.0;

/// One `(start, target, rate)` ease, ticked `current += (target - current) *
/// rate` once a tick.
///
/// **One step a tick, not `BombBlast_Update`'s own sub-stepped loop.** The
/// original re-simulates in fixed `1/60 s` increments or `dt`, however long a
/// frame took; this engine's own tick is fixed at `60 Hz` always
/// (`docs/architecture/determinism.md`), so `dt / (1/60)` is always `1` here
/// and the loop collapses to its own single body with nothing lost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Ease {
    pub(super) start: f32,
    pub(super) target: f32,
    pub(super) rate: f32,
}

impl Ease {
    pub(super) const fn step(self, current: f32) -> f32 {
        current + (self.target - current) * self.rate
    }
}

/// The two `.vex` files the blast's own detonation loads, grouped into one
/// field the same way [`blast_models::PlasmaBlastModels`] groups its own
/// three.
#[derive(Debug, Clone, Default)]
pub struct BombBlastModels {
    /// `Data\Weapons\explosion_hemisphere.vex`.
    pub hemisphere: Option<Model>,
    /// `Data\Weapons\Bomb_Shockwave.vex`.
    pub shockwave: Option<Model>,
}

/// The drawable pools [`Scene`] builds from a [`BombBlastModels`].
#[derive(Debug)]
pub(in crate::race) struct BombBlastDrawables {
    pub(in crate::race) hemisphere: Vec<Drawable>,
    pub(in crate::race) shockwave: Vec<Drawable>,
}

impl BombBlastDrawables {
    /// Builds both pools with `build_one` - `Scene::new`'s own
    /// `weapon_drawables` closure, reused so this pool is built exactly like
    /// the Rocket's, the Mine's, the Bomb's own body and the Cannon round's.
    pub(in crate::race) fn build(
        models: BombBlastModels,
        mut build_one: impl FnMut(Option<Model>) -> Result<Vec<Drawable>>,
    ) -> Result<Self> {
        Ok(Self {
            hemisphere: build_one(models.hemisphere)?,
            shockwave: build_one(models.shockwave)?,
        })
    }
}

/// One live render-side blast: where it detonated, which way it is oriented,
/// and the three running ease values `BombBlast_Update` ticks every frame.
///
/// **View state, not `World` state** - see this module's own doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BombBlast {
    pub(super) position: Vec3,
    /// The frozen bomb's own forward axis - see this module's own doc
    /// comment on what this substitutes for `+0x60`.
    pub(super) dir: Vec3,
    pub(super) age: f32,
    pub(super) hemisphere_scale: f32,
    pub(super) shockwave_scale: f32,
}

/// One blast's own transform and visibility, for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BombBlastDraw {
    pub(super) hemisphere_matrix: Mat4,
    pub(super) hemisphere_visible: bool,
    pub(super) shockwave_matrix: Mat4,
    pub(super) shockwave_visible: bool,
}

impl Race {
    /// Starts a render-side blast at `position`, oriented by `orientation` -
    /// dropped silently if all [`BOMB_BLAST_SLOTS`] are already live, the
    /// same "no evicting a running effect" rule
    /// [`Race::spawn_plasma_blast_model`] takes.
    ///
    /// Called from [`Race::ignite_blast`] for every Bomb detonation.
    pub(in crate::race) fn spawn_bomb_blast_model(&mut self, position: Vec3, orientation: Quat) {
        let Some(slot) = self.view.bomb_blasts.iter().position(Option::is_none) else {
            return;
        };
        let dir = (orientation * Vec3::NEG_Z)
            .try_normalize()
            .unwrap_or(Vec3::NEG_Z);
        self.view.bomb_blasts[slot] = Some(BombBlast {
            position,
            dir,
            age: 0.0,
            hemisphere_scale: HEMISPHERE_SCALE.start,
            shockwave_scale: SHOCKWAVE_SCALE.start,
        });
    }

    /// Ages every live blast one tick, steps its three eases and retires
    /// whichever just crossed [`LIFETIME_SECONDS`].
    ///
    /// Called from [`Race::tick`], after the impacts loop that can spawn a
    /// new one this same tick - the same one-tick order
    /// [`Race::advance_plasma_blast_models`] already runs a freshly-spawned
    /// blast in.
    pub(in crate::race) fn advance_bomb_blast_models(&mut self, dt: f32) {
        for slot in &mut self.view.bomb_blasts {
            let Some(blast) = slot else { continue };
            blast.age += dt;
            blast.hemisphere_scale = HEMISPHERE_SCALE.step(blast.hemisphere_scale);
            if blast.age > SHOCKWAVE_SCALE_DELAY_SECONDS {
                blast.shockwave_scale = SHOCKWAVE_SCALE.step(blast.shockwave_scale);
            }
            if blast.age >= LIFETIME_SECONDS {
                *slot = None;
            }
        }
    }

    /// This frame's transform and visibility for every render-side blast
    /// slot, in slot order - `None` for a slot with nothing live.
    #[must_use]
    pub(in crate::race) fn bomb_blast_draws(&self) -> [Option<BombBlastDraw>; BOMB_BLAST_SLOTS] {
        std::array::from_fn(|slot| {
            self.view.bomb_blasts[slot].map(|blast| {
                let basis = bomb_blast_basis(blast.position, blast.dir);
                // The shockwave's own copy of the basis, pulled back along
                // `dir` by a fixed `1.0` - `DAT_08ab1070`, read directly.
                let shockwave_position = blast.position - blast.dir * SHOCKWAVE_PULLBACK;
                let shockwave_basis = bomb_blast_basis(shockwave_position, blast.dir);
                BombBlastDraw {
                    hemisphere_matrix: basis
                        * Mat4::from_scale(Vec3::splat(blast.hemisphere_scale)),
                    hemisphere_visible: blast.age < HEMISPHERE_HIDE_AT_SECONDS,
                    // Row 0 (`right`) and row 2 (`up`) scale; row 1 (`dir`)
                    // does not - see this module's own doc comment on the
                    // column mapping. A flat ring growing in the plane
                    // perpendicular to its own `dir`.
                    shockwave_matrix: shockwave_basis
                        * Mat4::from_scale(Vec3::new(
                            blast.shockwave_scale,
                            1.0,
                            blast.shockwave_scale,
                        )),
                    shockwave_visible: true,
                }
            })
        })
    }
}

/// An orthonormal basis at `position` facing `dir` - the same cross-product
/// shape [`blast_models::billboard_matrix`] builds for a camera-facing
/// billboard, reused here for a *direction*-facing one instead. Columns are
/// `(right, up, dir, position)`, matching every other placement matrix in
/// this codebase - see this module's own doc comment for why `dir` takes the
/// executable's own row-1 reading rather than a literal row transcription.
fn bomb_blast_basis(position: Vec3, dir: Vec3) -> Mat4 {
    let reference = if dir.dot(Vec3::Y).abs() > 0.999 {
        dir.any_orthonormal_vector()
    } else {
        Vec3::Y
    };
    let right = dir.cross(reference).normalize_or_zero();
    let up = right.cross(dir);
    Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        dir.extend(0.0),
        position.extend(1.0),
    )
}

#[cfg(test)]
mod tests;
