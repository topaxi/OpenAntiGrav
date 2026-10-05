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
//! simulation itself only ever produces one [`oag_weapons::projectile::Impact`]
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
//! **The basis's column order is a measured fact, not a choice** - see
//! [`bomb_blast_basis`]'s own doc comment: `oag-view --mesh` on both `.vex`
//! files shows a dome resting pole-up and a ring lying flat, both on their
//! own local `Y`, so `dir` has to sit in this basis's own `Y` column for
//! either model to draw upright. An earlier version of this basis put `dir`
//! in `Z` (mirroring `blast_models::billboard_matrix`'s own column order for
//! a camera-facing basis) and scaled the wrong pair of axes on the shockwave
//! as a direct consequence - caught by comparing a rendered capture against
//! these two mesh views, not by re-reading the decompile.
//!
//! **Substituted, chosen rather than measured, no confidence score - carried
//! from [`oag_weapons::projectile::mine::frozen_pose`]'s own hedge:** what
//! `dir` itself *is*. The basis crosses the frozen craft orientation's own
//! up axis (`orientation * Vec3::Y`) against the executable's own `(0, 0,
//! 1)` reference vector (`Vec3::Z` directly - see `bomb_blast_basis`'s own
//! doc comment for why no "PSP space to this engine's space" translation is
//! needed here), in place of the unlocated rear-emitter's own `+0x60` row
//! crossed against that same reference. Which real-world axis `+0x60`
//! carries on the original's own bomb entity is not established; only which
//! *slot* of the basis the authored models expect it to fill is.
//!
//! **The shockwave's fading alpha is applied (2026-10-01).** `BombBlast_Update` stamps a white word whose alpha
//! is the `+0xf4` ease (`1.0 -> 0.0`) onto the shockwave's meshes through `Image_SetVertexColours` (`mesh+0x6c`,
//! `0x089122b4`). It reaches the GE as the **ambient light's** alpha: the ring's strips are drawn with lighting
//! on, no lights, `MATERIALUPDATE` `7` (the vertex colours are the material) and `0x5d` (ambient light alpha)
//! written just before each strip - `0xf4` and `0xb8` in two dumps of the ship explosion's ring, which are
//! the ease at `0.957` and `0.722` - so the drawn alpha is the vertex alpha times the ease, which is what
//! [`Drawable::tint`] does with `[1, 1, 1, alpha]`. (A first reading of this paragraph read the **material**
//! ambient registers `0x55`/`0x58` instead, found them untouched and called the fade a no-op; the ring's
//! authored vertex words are indeed untouched in the buffer, as a lit material would leave them.) It was
//! recovered and left unapplied until the ship explosion's own shockwave needed the same mechanism.
//!
//! # The ship explosion's shockwave (2026-10-01)
//!
//! `Ship_SpawnExplosionBig` (`FUN_088407b0`) builds a second object of the same
//! `Bomb_Shockwave.vex` (`FUN_0885ecf0`, update `FUN_0885efc4`), read whole and
//! logged live on PPSSPP: at the craft's own position (the matrix's translation,
//! before the explosion effect's `4.0` drop), basis from the matrix's up row
//! (the same Gram-Schmidt against `(0, 0, 1)` the Bomb's basis uses), **uniform**
//! scale easing `0.1 -> 20` at `0.05` a tick, an alpha `1 -> 0` at `0.02` (applied, as above), retired at `1.5 s`. Live: scale `0.1, 1.095, 2.04, 2.938, 3.791, 4.602`, alpha
//! `1, 0.98, 0.9604, 0.9412 ...`, age `1.3343` at scale `18.545` and alpha
//! `0.3569`. The ring is `5.37` units across its radius at scale `1`, so by the
//! fifth tick it spans the whole start grid, flat at the craft's height - which a
//! chase camera at the same height sees edge-on as the thin orange line along the
//! horizon that the original's explosion leaves for about fifty frames. (The white band that
//! covers the first five frames is **not** the ring: it is the explosion's `Glow` template, drawn with
//! its own sprite since 2026-10-01 - see `ship-shockwave.md`.) Confidence **90** for the law (decompile
//! and one live boot agree to every digit); the picture reading is **seen once**. The original's
//! updates step `(int)(dt / (1/60))` times with no remainder, which on PPSSPP skipped a third of the
//! frames; ours steps once a tick (`ship-shockwave.md`).

use super::*;

/// How many render-side Bomb-blast instances can be live at once.
///
/// [`oag_weapons::projectile::MAX_PROJECTILES`], the same bound
/// [`blast_models::PLASMA_BLAST_SLOTS`] takes and for the same reason: it is
/// this engine's own projectile-pool cap, not a measured figure for how many
/// the original itself keeps around (its own object is heap-allocated per
/// detonation, with no fixed pool this project has found).
pub(super) const BOMB_BLAST_SLOTS: usize = oag_weapons::projectile::MAX_PROJECTILES;

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

/// The shockwave's own fading alpha ease, `+0xf4`/`+0xf8`/`+0xfc`, applied as
/// [`Drawable::tint`]'s alpha - see this module's own doc comment.
pub(super) const SHOCKWAVE_ALPHA: Ease = Ease {
    start: 1.0,
    target: 0.0,
    rate: 0.1,
};

/// The ship explosion's shockwave scale ease (`FUN_0885ecf0` writes `0.1` and `DAT_08ab0f30` = `20.0`
/// and `DAT_08ab0f2c` = `0.05`, `FUN_0885efc4` steps it once a tick), applied to all three axes.
pub(super) const SHIP_SHOCKWAVE_SCALE: Ease = Ease {
    start: 0.1,
    target: 20.0,
    rate: 0.05,
};

/// The ship explosion's shockwave alpha ease: `1.0`, `0.0` and `DAT_08ab0f28` = `0.02`.
pub(super) const SHIP_SHOCKWAVE_ALPHA: Ease = Ease {
    start: 1.0,
    target: 0.0,
    rate: 0.02,
};

/// When `FUN_0885efc4` retires the ship explosion's shockwave, in seconds (`1.5 <= age`).
pub(super) const SHIP_SHOCKWAVE_LIFETIME_SECONDS: f32 = 1.5;

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
    /// The Repulser's field model, `Data\Weapons\pulse_repulsorwave.vex` - not
    /// the Bomb's, but the same kind of thing (an eased, faded, view-side ring),
    /// and it rides this container rather than a seventh element of every
    /// weapon tuple from `load.rs` to `Scene::new`, all of which sit at their
    /// line ceilings. Played by [`super::repulser_field`].
    pub repulser_field: Option<Model>,
    /// The magstrip effect's two models, `[MagEffect1, MagEffect2]` - riding
    /// this container for the Repulser field's own reason above. Played by
    /// [`super::mag_floor_fx`].
    pub mag_floor: [Option<Model>; 2],
}

/// The drawable pools [`Scene`] builds from a [`BombBlastModels`].
#[derive(Debug)]
pub(crate) struct BombBlastDrawables {
    pub(crate) hemisphere: Vec<Drawable>,
    pub(crate) shockwave: Vec<Drawable>,
    /// One per pool slot - see [`BombBlastModels::repulser_field`].
    pub(crate) repulser_field: Vec<Drawable>,
    /// One per ship slot - see [`BombBlastModels::mag_floor`].
    pub(crate) mag_floor: [Vec<Drawable>; 2],
}

impl BombBlastDrawables {
    /// Builds both pools with `build_one` - `Scene::new`'s own
    /// `weapon_drawables` closure, reused so this pool is built exactly like
    /// the Rocket's, the Mine's, the Bomb's own body and the Cannon round's.
    pub(crate) fn build(
        models: BombBlastModels,
        mut build_one: impl FnMut(Option<Model>) -> Result<Vec<Drawable>>,
    ) -> Result<Self> {
        Ok(Self {
            hemisphere: build_one(models.hemisphere)?,
            shockwave: build_one(models.shockwave)?,
            repulser_field: build_one(models.repulser_field)?,
            mag_floor: {
                let [first, second] = models.mag_floor;
                // A pool per ship, not per projectile: every craft builds one
                // effect and nothing else does.
                let mut per_ship = |model| {
                    build_one(model).map(|mut pool: Vec<Drawable>| {
                        pool.truncate(oag_weapons::MAX_SHIPS);
                        pool
                    })
                };
                [per_ship(first)?, per_ship(second)?]
            },
        })
    }
}

/// One live render-side blast: where it detonated, which way it is oriented,
/// and the three running ease values `BombBlast_Update` ticks every frame.
///
/// **View state, not `World` state** - see this module's own doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BombBlast {
    /// Which object this is: a Bomb's two models, or a ship explosion's lone shockwave.
    pub(super) kind: BlastKind,
    pub(super) position: Vec3,
    /// The frozen bomb's own forward axis - see this module's own doc
    /// comment on what this substitutes for `+0x60`.
    pub(super) dir: Vec3,
    pub(super) age: f32,
    pub(super) hemisphere_scale: f32,
    pub(super) shockwave_scale: f32,
    pub(super) shockwave_alpha: f32,
}

/// Which of the two objects a [`BombBlast`] slot holds - both are `Bomb_Shockwave.vex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlastKind {
    /// `BombBlast_Construct`'s: a hemisphere and a shockwave that widens in its own plane.
    Bomb,
    /// `Ship_SpawnExplosionBig`'s: the shockwave alone, scaled on all three axes.
    ShipExplosion,
}

/// One blast's own transform and visibility, for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BombBlastDraw {
    pub(super) hemisphere_matrix: Mat4,
    pub(super) hemisphere_visible: bool,
    pub(super) shockwave_matrix: Mat4,
    pub(super) shockwave_visible: bool,
    /// The alpha [`Drawable::tint`] multiplies the shockwave's vertex colours by.
    pub(super) shockwave_alpha: f32,
    /// The blast's age in seconds: the time both models' texture tracks play at.
    /// `BombBlast_Construct` seeds each model with `Node_SetAnimTimeTree(0.0)` and
    /// the mesh update then integrates the clock's delta, so the texture time *is*
    /// the age at rate 1 (measured live, `docs/rendering/scenery-animation.md`).
    pub(super) age: f32,
}

impl Race {
    /// Starts a render-side blast at `position`, oriented by `orientation` -
    /// dropped silently if all [`BOMB_BLAST_SLOTS`] are already live, the
    /// same "no evicting a running effect" rule
    /// [`Race::spawn_plasma_blast_model`] takes.
    ///
    /// Called from [`Race::ignite_blast`] for every Bomb detonation.
    pub(crate) fn spawn_bomb_blast_model(&mut self, position: Vec3, orientation: Quat) {
        let Some(slot) = self.view.bomb_blasts.iter().position(Option::is_none) else {
            return;
        };
        // `Vec3::Y`, not `NEG_Z`: `oag-view --mesh` on both `.vex` files
        // shows a dome resting pole-up and a ring lying flat, both on their
        // own local Y - see this module's own doc comment for why that
        // settles which axis this substitutes for the unlocated
        // `entity+0x60` row, and `mine::frozen_pose`'s own hedge for why the
        // substitution itself (the frozen craft orientation standing in for
        // the rear emitter's own transform) carries no confidence score.
        let dir = (orientation * Vec3::Y).try_normalize().unwrap_or(Vec3::Y);
        self.view.bomb_blasts[slot] = Some(BombBlast {
            kind: BlastKind::Bomb,
            position,
            dir,
            age: 0.0,
            hemisphere_scale: HEMISPHERE_SCALE.start,
            shockwave_scale: SHOCKWAVE_SCALE.start,
            shockwave_alpha: SHOCKWAVE_ALPHA.start,
        });
    }

    /// Starts a ship explosion's shockwave at `position` (the craft's own, not the explosion
    /// effect's dropped one) with the craft's up as `dir` - see this module's doc comment.
    /// Dropped silently when every slot is live, like [`Self::spawn_bomb_blast_model`].
    pub(crate) fn spawn_ship_shockwave(&mut self, position: Vec3, up: Vec3) {
        let Some(slot) = self.view.bomb_blasts.iter().position(Option::is_none) else {
            return;
        };
        self.view.bomb_blasts[slot] = Some(BombBlast {
            kind: BlastKind::ShipExplosion,
            position,
            dir: up.try_normalize().unwrap_or(Vec3::Y),
            age: 0.0,
            hemisphere_scale: HEMISPHERE_SCALE.start,
            shockwave_scale: SHIP_SHOCKWAVE_SCALE.start,
            shockwave_alpha: SHIP_SHOCKWAVE_ALPHA.start,
        });
    }

    /// Ages every live blast one tick, steps its three eases and retires
    /// whichever just crossed [`LIFETIME_SECONDS`].
    ///
    /// Called from [`Race::tick`], after the impacts loop that can spawn a
    /// new one this same tick - the same one-tick order
    /// [`Race::advance_plasma_blast_models`] already runs a freshly-spawned
    /// blast in.
    pub(crate) fn advance_bomb_blast_models(&mut self, dt: f32) {
        for slot in &mut self.view.bomb_blasts {
            let Some(blast) = slot else { continue };
            blast.age += dt;
            if blast.kind == BlastKind::ShipExplosion {
                blast.shockwave_scale = SHIP_SHOCKWAVE_SCALE.step(blast.shockwave_scale);
                blast.shockwave_alpha = SHIP_SHOCKWAVE_ALPHA.step(blast.shockwave_alpha);
                if blast.age >= SHIP_SHOCKWAVE_LIFETIME_SECONDS {
                    *slot = None;
                }
                continue;
            }
            blast.hemisphere_scale = HEMISPHERE_SCALE.step(blast.hemisphere_scale);
            blast.shockwave_alpha = SHOCKWAVE_ALPHA.step(blast.shockwave_alpha);
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
    pub(crate) fn bomb_blast_draws(&self) -> [Option<BombBlastDraw>; BOMB_BLAST_SLOTS] {
        std::array::from_fn(|slot| {
            self.view.bomb_blasts[slot].map(|blast| {
                if blast.kind == BlastKind::ShipExplosion {
                    return BombBlastDraw {
                        hemisphere_matrix: Mat4::IDENTITY,
                        hemisphere_visible: false,
                        shockwave_matrix: bomb_blast_basis(blast.position, blast.dir)
                            * Mat4::from_scale(Vec3::splat(blast.shockwave_scale)),
                        shockwave_visible: true,
                        shockwave_alpha: blast.shockwave_alpha,
                        age: blast.age,
                    };
                }
                let basis = bomb_blast_basis(blast.position, blast.dir);
                // The shockwave's own copy of the basis, pulled back along
                // `dir` by a fixed `1.0` - `DAT_08ab1070`, read directly.
                let shockwave_position = blast.position - blast.dir * SHOCKWAVE_PULLBACK;
                let shockwave_basis = bomb_blast_basis(shockwave_position, blast.dir);
                BombBlastDraw {
                    hemisphere_matrix: basis
                        * Mat4::from_scale(Vec3::splat(blast.hemisphere_scale)),
                    hemisphere_visible: blast.age < HEMISPHERE_HIDE_AT_SECONDS,
                    // Row 0 (`right`) and row 2 (the orthogonalised
                    // reference) scale; row 1 (`dir`, this basis's own `Y`
                    // column) does not - see [`bomb_blast_basis`]'s own doc
                    // comment for the column order this depends on. A flat
                    // ring growing in the plane perpendicular to its own
                    // `dir`, i.e. staying level while it widens.
                    shockwave_matrix: shockwave_basis
                        * Mat4::from_scale(Vec3::new(
                            blast.shockwave_scale,
                            1.0,
                            blast.shockwave_scale,
                        )),
                    shockwave_visible: true,
                    shockwave_alpha: blast.shockwave_alpha,
                    age: blast.age,
                }
            })
        })
    }
}

/// An orthonormal basis at `position` with `dir` as its own local `Y` axis -
/// the shape `Bomb_Detonate`'s own preamble builds (Gram-Schmidt-orthogonalise
/// a reference vector against `dir`, then cross for the third axis), not
/// [`blast_models::billboard_matrix`]'s camera-facing shape, though the
/// arithmetic step is the same.
///
/// **Columns are `(right, dir, reference, position)` - `dir` in the `Y`
/// slot, not `Z`.** Read off `oag-view --mesh` on both `.vex` files: the
/// hemisphere is a dome resting pole-up on its own local `Y`, and the
/// shockwave is a ring lying flat with its face normal on the same axis -
/// so whatever `entity+0x60` truly is on the original's own bomb entity,
/// **this basis's `Y` column is the slot both authored models expect their
/// own "up" to sit in**, a measured fact about the assets rather than a
/// choice about the basis. Putting `dir` in `Y` is therefore required by the
/// meshes, independently of the earlier, since-corrected reading that put it
/// in `Z` to match [`blast_models::billboard_matrix`]'s own column order -
/// that version scaled the wrong pair of axes on the shockwave (see the
/// `Y`-only-unscaled comment at its own call site) until this reading caught
/// it against the rendered meshes.
///
/// `reference` is `Vec3::Z` - `DAT_08a907c0`, read directly as `(0, 0, 1,
/// 0)` - not a translated "world up": `CRAFT_BLAST_DROP`'s own `y - 2.5`
/// (`Rocket_HitCraft`) already establishes this engine's world axes agree
/// with the executable's own without a swap, so the literal constant is
/// usable as `Vec3::Z` as-is. See this module's own doc comment for what is
/// chosen instead: which real-world direction `dir` substitutes for.
pub(crate) fn bomb_blast_basis(position: Vec3, dir: Vec3) -> Mat4 {
    let world_reference = Vec3::Z;
    let reference = if dir.dot(world_reference).abs() > 0.999 {
        dir.any_orthonormal_vector()
    } else {
        (world_reference - dir * dir.dot(world_reference)).normalize_or_zero()
    };
    let right = dir.cross(reference);
    Mat4::from_cols(
        right.extend(0.0),
        dir.extend(0.0),
        reference.extend(0.0),
        position.extend(1.0),
    )
}

#[cfg(test)]
mod tests;
