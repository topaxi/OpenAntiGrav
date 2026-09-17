//! The Plasma's own detonation: the halo and two hemispheres
//! `PlasmaBlast_Construct` loads, and nothing drew until now - see
//! `docs/ghidra/functions/psp-pulse-usa/plasma.md#the-blast-objects-own-per-tick-animation-plasmablast_update`
//! for the reading this plays back.
//!
//! **Render-side view state only**, on the same terms every other transient
//! effect in [`super::view::RaceView`] is: a blast's own position and age
//! never move a determinism hash, because nothing in the simulation asks how
//! long ago a wall started glowing - the original's own equivalent
//! (`PlasmaBlast_Update`) is drawing-only state too, per that page's own
//! reading of what it touches.
//!
//! This is a new top-level module of `race` rather than a child of
//! `race::weapons`, the shape the handover brief for this asked for, because
//! `mod blast_models;` has to live in the parent module's own file and
//! `race/weapons.rs` is another member's lane on this pass; declaring the
//! module from `race.rs` instead needs no line in a file this thread does
//! not own.
//!
//! # Two mechanisms share this module, and do not unify
//!
//! **Pulse** scrubs a baked anim-time track per model - `PlasmaBlast_Update`'s
//! own `age * rate` passed to `Node_SetAnimTimeTree`, played back through
//! [`Drawable::write_node_anims`] below. **Wipeout HD** eases a per-model
//! *scale* instead - `WeaponExplosions_Draw`'s own `cur += (target - cur) *
//! rate`, read off `ps3-hdfury-eu/plasma.md` - and hides all three at 1.3 s
//! (`WeaponExplosions_Collapse`, confidence 85, independently corroborated
//! 2026-09-17 by decompiling both `0x00127770` and its Draw counterpart:
//! Draw's own per-model window gate (1.7/1.3/1.3 s) is a *different*
//! mechanism than Collapse's node-flag clear, so the ring's own ease
//! technically keeps computing to 1.7 s, but nothing after 1.3 s is visible
//! once Collapse has run - the two facts do not contradict, they answer
//! different questions). Nothing here derives one title's numbers from the
//! other's: [`Race::advance_plasma_blast_models`] and
//! [`Race::plasma_blast_draws`] both branch on `RaceView::hd_plasma_blast`
//! and run one body or the other, never a blend.
//!
//! `PlasmaBlastModels`'s own field names - `halo`, `hemisphere2`,
//! `hemisphere1` - stay Pulse's on **both** titles; see that struct's own
//! doc comment for why HD's ring/sphere/halo trio loads into them by
//! ordinal position rather than earning a rename.

use super::*;

/// How many render-side Plasma-blast instances can be live at once.
///
/// Matches [`oag_gameplay::projectile::MAX_PROJECTILES`], the Plasma bolt
/// pool's own cap - which bounds how many bolts could detonate on the same
/// tick - not a measured figure for how many blasts the original itself
/// keeps around at once (its own object is heap-allocated per detonation, in
/// `Plasma_SpawnDetonation`, with no fixed pool this project has found).
pub(super) const PLASMA_BLAST_SLOTS: usize = oag_gameplay::projectile::MAX_PROJECTILES;

/// `PlasmaBlast_Update`'s own hardcoded retire time.
///
/// **Recovered, confidence 88** - see plasma.md's "The bolt's own detonation
/// lasts a hardcoded 1.5 seconds": a `lui a0,0x3fc0` immediate two
/// instructions before the age comparison, not a shared `DAT_` constant.
pub(super) const PLASMA_BLAST_LIFETIME_SECONDS: f32 = 1.5;

/// Per-model anim-time rate `PlasmaBlast_Update` scrubs
/// `Node_SetAnimTimeTree` with every tick, as `age * rate`.
///
/// **Recovered, confidence 75** - see plasma.md's own reading. In
/// `PlasmaBlast_Construct`'s own load order: the halo first, then
/// `hemisphere2`, then `hemisphere1` - matching [`Scene`]'s own field order
/// for the three drawable pools.
pub(super) const PLASMA_BLAST_HALO_ANIM_RATE: f32 = 0.1;
/// See [`PLASMA_BLAST_HALO_ANIM_RATE`].
pub(super) const PLASMA_BLAST_HEMISPHERE2_ANIM_RATE: f32 = 0.07;
/// See [`PLASMA_BLAST_HALO_ANIM_RATE`].
pub(super) const PLASMA_BLAST_HEMISPHERE1_ANIM_RATE: f32 = 0.07;

/// Wipeout HD/Fury's own object lifetime for a Plasma detonation -
/// `WeaponExplosions_Update`'s own `age >= 3.5f` reap. **Not the same
/// constant as [`PLASMA_BLAST_LIFETIME_SECONDS`]**; the two titles' objects
/// are unrelated code, read off unrelated executables, and only happen to
/// share a name in this engine's own type. See plasma.md.
pub(super) const HD_BLAST_LIFETIME_SECONDS: f32 = 3.5;

/// When `WeaponExplosions_Collapse` fires and hides all three models -
/// confidence 85 in plasma.md, independently re-derived 2026-09-17 by
/// decompiling `0x00127770` itself. See this module's own doc comment for
/// why the ring's own ease keeps running past this point with nothing to
/// show for it.
pub(super) const HD_BLAST_HIDE_AT_SECONDS: f32 = 1.3;

/// The three models' own `(target, rate)` ease pair, in
/// [`PlasmaBlastModels`]'s own ordinal order - ring, sphere, halo - read off
/// `WeaponExplosions_Construct`/`_Draw`. Confidence 88 for the rates and
/// lifetimes (corroborated byte-for-byte against `ps4-omega-eu`'s own static
/// initialiser); the *targets* are HD's own and not Omega's - see plasma.md's
/// closing hedge on why the PS4's `rcsmodel` re-exports are suspected to
/// carry a different base scale, unverified.
pub(super) const HD_BLAST_TARGETS: [f32; 3] = [100.0, 7.1, 7.0];
/// See [`HD_BLAST_TARGETS`].
pub(super) const HD_BLAST_RATES: [f32; 3] = [0.01, 0.3, 0.2];

/// The three `.vex` files the blast's own detonation loads, grouped into
/// one field on [`options::Loaded`] and one parameter of [`Scene::new`]
/// rather than three of each - the line budget `race/load.rs` and
/// `race/scene.rs` had left for a fifth weapon-model kind was exactly zero.
///
/// **Field names are Pulse's on both titles, by ordinal position rather than
/// by name.** On Wipeout HD, `halo` holds `HD_plasma_ring`, `hemisphere2`
/// holds `HD_plasma_sphere` and `hemisphere1` holds `HD_plasma_halo` - the
/// load order `WeaponExplosions_Start` (`0x00127cd0`) uses, matching
/// [`HD_BLAST_TARGETS`]/[`HD_BLAST_RATES`]'s own ordering. Left unrenamed
/// deliberately: which ease algorithm a loaded trio plays is decided at draw
/// time from `RaceView::hd_plasma_blast`, never from which field a model
/// sits in, and `load/weapon_models.rs` is shared with a concurrent pass on
/// the Cannon's own textures - renaming this struct's fields would turn an
/// append-append merge into a semantic one for no reading gained. See
/// `oag_title::weapons::HdPlasmaBlast`'s own doc comment for the same
/// ordinal note on the title-table side.
#[derive(Debug, Clone, Default)]
pub struct PlasmaBlastModels {
    pub halo: Option<Model>,
    pub hemisphere2: Option<Model>,
    pub hemisphere1: Option<Model>,
}

/// The three drawable pools [`Scene`] builds from a [`PlasmaBlastModels`] -
/// grouped for the same reason that is.
#[derive(Debug)]
pub(in crate::race) struct PlasmaBlastDrawables {
    pub(in crate::race) halo: Vec<Drawable>,
    pub(in crate::race) hemisphere2: Vec<Drawable>,
    pub(in crate::race) hemisphere1: Vec<Drawable>,
}

impl PlasmaBlastDrawables {
    /// Builds all three pools with `build_one` - `Scene::new`'s own
    /// `weapon_drawables` closure, reused rather than duplicated so this
    /// pool is built exactly like the Rocket's, the Mine's, the Bomb's and
    /// the Cannon round's.
    pub(in crate::race) fn build(
        models: PlasmaBlastModels,
        mut build_one: impl FnMut(Option<Model>) -> Result<Vec<Drawable>>,
    ) -> Result<Self> {
        Ok(Self {
            halo: build_one(models.halo)?,
            hemisphere2: build_one(models.hemisphere2)?,
            hemisphere1: build_one(models.hemisphere1)?,
        })
    }
}

/// One live render-side blast: where it detonated, and how long ago.
///
/// **View state, not `World` state** - see this module's own doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlasmaBlast {
    pub(super) position: Vec3,
    pub(super) age: f32,
    /// Wipeout HD's own per-model scale ease, `cur` in
    /// [`HD_BLAST_TARGETS`]/[`HD_BLAST_RATES`]'s own ordinal order. Unused
    /// and left at zero on Pulse, where [`PLASMA_BLAST_HALO_ANIM_RATE`] and
    /// its two siblings drive the picture instead - see this module's own
    /// doc comment.
    ///
    /// **Starts at `0.0`, chosen not measured.** `WeaponExplosions_Reset`
    /// seeds each `cur` from `*0x00994070+n`, a value this session did not
    /// resolve; zero draws the explosion growing in from nothing rather than
    /// snapping to some unread starting size, and is the same "absent
    /// evidence, visible absence" call `CLAUDE.md` asks for elsewhere.
    pub(super) hd_ease: [f32; 3],
}

/// One blast's own transform and per-model anim-time scrub, for one frame.
///
/// **One `matrix` serves all three models.** `PlasmaBlast_Update`'s own
/// per-model scale/offset nudge - the first float of each of the three
/// `DAT_08ab0f34`/`f44`/`f54` triples `PlasmaBlast_Construct` reads - is
/// `0.0` for every model (see plasma.md), so the halo and both hemispheres
/// sit at the identical transform in this engine too and differ only in
/// which `.vex` they are and which anim-time value plays their own baked
/// motion, if any. **True on Pulse; on HD the same shared `matrix` is
/// additionally scaled per model** - see [`Self::hd_scale`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlasmaBlastDraw {
    pub(super) matrix: Mat4,
    /// Pulse only - `None` on HD, where [`Self::hd_scale`] plays the
    /// mechanism instead. See this module's own doc comment for why the two
    /// do not unify.
    pub(super) halo_seconds: f32,
    /// See [`Self::halo_seconds`].
    pub(super) hemisphere2_seconds: f32,
    /// See [`Self::halo_seconds`].
    pub(super) hemisphere1_seconds: f32,
    /// HD only: each of the three models' own uniform scale this tick, in
    /// [`PlasmaBlastModels`]'s own ordinal order (ring, sphere, halo).
    /// `[0.0; 3]` past [`HD_BLAST_HIDE_AT_SECONDS`], matching
    /// `WeaponExplosions_Collapse`'s own node-hide - see this module's own
    /// doc comment. `None` on Pulse.
    pub(super) hd_scale: Option<[f32; 3]>,
}

impl Race {
    /// Starts a render-side blast at `position`, in the first free slot -
    /// dropped silently if all [`PLASMA_BLAST_SLOTS`] are already live, the
    /// same "no evicting a running effect" rule
    /// [`weapons::visuals::advance_one_flare`]'s own flare-stage fallback
    /// takes.
    ///
    /// Called from [`Race::ignite_blast`] for every Plasma detonation - a
    /// wall hit and a 10 s timeout alike, since `Plasmas_Update`'s own
    /// teardown pass does not branch on which ended the bolt.
    pub(in crate::race) fn spawn_plasma_blast_model(&mut self, position: Vec3) {
        if let Some(slot) = self.view.plasma_blasts.iter().position(Option::is_none) {
            self.view.plasma_blasts[slot] = Some(PlasmaBlast {
                position,
                age: 0.0,
                hd_ease: [0.0; 3],
            });
        }
    }

    /// Ages every live blast one tick and retires whichever just crossed its
    /// title's own lifetime - [`PLASMA_BLAST_LIFETIME_SECONDS`] on Pulse,
    /// [`HD_BLAST_LIFETIME_SECONDS`] on HD, branched on
    /// `RaceView::hd_plasma_blast`. On HD this also advances the three-model
    /// scale ease and, the tick a blast first crosses
    /// [`HD_BLAST_HIDE_AT_SECONDS`], plays `WO_PLASMA_LIGHTNING_COLLAPSE` at
    /// its position - `WeaponExplosions_Collapse`'s own trigger, read
    /// 2026-09-17.
    ///
    /// Called from [`Race::tick`], after the impacts loop that can spawn a
    /// new one this same tick - a blast spawned this tick draws at `age ==
    /// 0.0` for the rest of the frame before its first advance, the same
    /// one-tick order [`Race::advance_projectile_flares`] already runs a
    /// freshly-placed flare in.
    pub(in crate::race) fn advance_plasma_blast_models(&mut self, dt: f32) {
        let hd = self.view.hd_plasma_blast;
        // Looked up once, before the loop borrows `plasma_blasts` mutably -
        // `self.view.stage.play` needs `&mut self.view` too, and nothing
        // below can hold both borrows at once.
        let collapse_effect = hd
            .then(|| self.view.effects.get(PLASMA_LIGHTNING_COLLAPSE_EFFECT).cloned())
            .flatten();
        let mut collapsed_at = Vec::new();
        for slot in &mut self.view.plasma_blasts {
            let Some(blast) = slot else { continue };
            let was_visible = blast.age < HD_BLAST_HIDE_AT_SECONDS;
            blast.age += dt;
            let lifetime = if hd {
                for (ease, (target, rate)) in blast
                    .hd_ease
                    .iter_mut()
                    .zip(HD_BLAST_TARGETS.into_iter().zip(HD_BLAST_RATES))
                {
                    *ease += (target - *ease) * rate;
                }
                if was_visible && blast.age >= HD_BLAST_HIDE_AT_SECONDS {
                    collapsed_at.push(blast.position);
                }
                HD_BLAST_LIFETIME_SECONDS
            } else {
                PLASMA_BLAST_LIFETIME_SECONDS
            };
            if blast.age >= lifetime {
                *slot = None;
            }
        }
        if let Some(effect) = collapse_effect {
            for position in collapsed_at {
                self.view.stage.play(&effect, position, 1.0);
            }
        }
    }

    /// This frame's transform, per-model scrub and (HD only) per-model
    /// scale for every render-side blast slot, in slot order - `None` for a
    /// slot with nothing live, which the caller uses to decide what to
    /// write and what to draw.
    ///
    /// `camera_position` decides the basis on both titles alike.
    /// **`PlasmaBlast_Update`'s own camera read is not fully resolved** -
    /// plasma.md's own hedge is that it is *some* function of the active
    /// camera's current orientation, not established to be any one specific
    /// axis - so this draws the ordinary "face the camera" billboard as a
    /// stated substitute for that unresolved vector math. **Chosen, not
    /// measured; no confidence score**, the same footing
    /// [`weapons::visuals::advance_quake_visual`] already gives its own
    /// unread orientation term. HD's own `WeaponExplosions_Start` fits a
    /// basis to the track surface instead (`FUN_000a97f0`, unread this
    /// session) - the billboard stands in for that too, on the same terms.
    #[must_use]
    pub(in crate::race) fn plasma_blast_draws(
        &self,
        camera_position: Vec3,
    ) -> [Option<PlasmaBlastDraw>; PLASMA_BLAST_SLOTS] {
        let hd = self.view.hd_plasma_blast;
        std::array::from_fn(|slot| {
            self.view.plasma_blasts[slot].map(|blast| {
                let matrix = billboard_matrix(blast.position, camera_position);
                if hd {
                    let visible = blast.age < HD_BLAST_HIDE_AT_SECONDS;
                    PlasmaBlastDraw {
                        matrix,
                        halo_seconds: 0.0,
                        hemisphere2_seconds: 0.0,
                        hemisphere1_seconds: 0.0,
                        hd_scale: Some(if visible { blast.hd_ease } else { [0.0; 3] }),
                    }
                } else {
                    PlasmaBlastDraw {
                        matrix,
                        halo_seconds: blast.age * PLASMA_BLAST_HALO_ANIM_RATE,
                        hemisphere2_seconds: blast.age * PLASMA_BLAST_HEMISPHERE2_ANIM_RATE,
                        hemisphere1_seconds: blast.age * PLASMA_BLAST_HEMISPHERE1_ANIM_RATE,
                        hd_scale: None,
                    }
                }
            })
        })
    }
}

/// A billboard basis at `position`, facing `camera_position` - see
/// [`Race::plasma_blast_draws`]'s own doc comment for why this is a stated
/// substitute rather than a decoded read.
fn billboard_matrix(position: Vec3, camera_position: Vec3) -> Mat4 {
    let forward = (camera_position - position)
        .try_normalize()
        .unwrap_or(Vec3::Z);
    // `Vec3::Y` degenerates when the camera sits directly above or below the
    // blast; `any_orthonormal_vector` is the fallback, the same guard
    // `Race::projectile_model_matrices` already uses for a straight-up
    // rocket.
    let reference = if forward.dot(Vec3::Y).abs() > 0.999 {
        forward.any_orthonormal_vector()
    } else {
        Vec3::Y
    };
    let right = forward.cross(reference).normalize_or_zero();
    let up = right.cross(forward);
    Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        forward.extend(0.0),
        position.extend(1.0),
    )
}

#[cfg(test)]
mod tests;
