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
//! **Pulse** plays each model's authored node and texture tracks at the
//! blast's age itself (`PlasmaBlast_Update`'s `Node_SetAnimTimeTree`,
//! [`Drawable::write_node_anims`] below; the GE dumps match the tracks at rate
//! 1, 2026-10-30) and eases each model's *scale* the way HD does, `cur +=
//! (target - cur) * rate`, with its own targets and the three small constants
//! this module read as anim-time rates until then ([`PULSE_BLAST_RATES`]), and
//! a grey tint ramp per model. **Wipeout HD** eases a per-model scale too -
//! `WeaponExplosions_Draw`'s own `cur += (target - cur) * rate`, read off
//! `ps3-hdfury-eu/plasma.md` - and hides all three at 1.3 s
//! (`WeaponExplosions_Collapse`, confidence 85, independently corroborated
//! 2026-09-17 by decompiling both `0x00127770` and its Draw counterpart:
//! Draw's own per-model window gate (1.7/1.3/1.3 s) is a *different*
//! mechanism than Collapse's node-flag clear, so the ring's own ease
//! technically keeps computing to 1.7 s, but nothing after 1.3 s is visible
//! once Collapse has run - the two facts do not contradict, they answer
//! different questions). Nothing here derives one title's numbers from the
//! other's: [`Race::advance_plasma_blast_models`] and
//! [`Race::plasma_blast_draws`] both branch on `RaceView::hd_plasma_blast`
//! and run one body or the other, never a blend. **Placement differs too**:
//! Pulse's hemispheres sit in a world-fixed track frame and its halo takes
//! the camera's orientation; HD's trio is a camera-facing billboard.
//!
//! `PlasmaBlastModels`'s own field names - `halo`, `hemisphere2`,
//! `hemisphere1` - stay Pulse's on **both** titles; see that struct's own
//! doc comment for why HD's ring/sphere/halo trio loads into them by
//! ordinal position rather than earning a rename.

use super::*;

/// How many render-side Plasma-blast instances can be live at once.
///
/// Matches [`oag_weapons::projectile::MAX_PROJECTILES`], the Plasma bolt
/// pool's own cap - which bounds how many bolts could detonate on the same
/// tick - not a measured figure for how many blasts the original itself
/// keeps around at once (its own object is heap-allocated per detonation, in
/// `Plasma_SpawnDetonation`, with no fixed pool this project has found).
pub(super) const PLASMA_BLAST_SLOTS: usize = oag_weapons::projectile::MAX_PROJECTILES;

/// `PlasmaBlast_Update`'s own hardcoded retire time.
///
/// **Recovered, confidence 88** - see plasma.md's "The bolt's own detonation
/// lasts a hardcoded 1.5 seconds": a `lui a0,0x3fc0` immediate two
/// instructions before the age comparison, not a shared `DAT_` constant.
pub(super) const PLASMA_BLAST_LIFETIME_SECONDS: f32 = 1.5;

/// Pulse's per-model scale ease, `cur += (target - cur) * rate` once per
/// tick from `cur = 0`, in [`PlasmaBlastModels`]'s own ordinal order: the
/// halo, then `hemisphere2`, then `hemisphere1`.
///
/// **The rates are `PlasmaBlast_Update`'s three small constants** (`DAT_08ab0f6c`
/// `0.1`, `DAT_08ab0f70` and `DAT_08ab0f74` `0.07`), which this module read as
/// per-model anim-time rates until 2026-10-30: the GE dumps show the node
/// animations and UV tracks play at the blast's age itself (rate 1), and the
/// rates are this ease. **The targets are fitted, not read from the
/// executable**: the first ease step puts the model matrix's first-row norm at
/// `target * rate` (9.47, 7.9 and 32.0 at the first drawn tick), and the
/// ticks after follow `target * (1 - (1 - rate)^n)` (26.5 at n = 3, 47.8 at
/// n = 6 on `hemisphere2`; increments of `0.93` of the one before). See
/// plasma.md, "the blast scale is an ease". Confidence 70: the early ticks fit
/// to the dump's precision; the late scale (`hemisphere2` 130.5 at age 1.05
/// against 133.9 here) sits below the ease on the boots whose frames stalled.
pub(super) const PULSE_BLAST_TARGETS: [f32; 3] = [320.0, 135.3, 112.75];
/// See [`PULSE_BLAST_TARGETS`].
pub(super) const PULSE_BLAST_RATES: [f32; 3] = [0.1, 0.07, 0.07];

/// What the eased scale multiplies: the GE draws these three models with
/// positions normalised to about a quarter of a unit, where our decoder
/// returns the `.vex`'s own extent (`hemisphere2` 8.65, the halo 10.0,
/// `hemisphere1` 7.6), so the world size is `ease * 0.025 * extent`.
/// Measured on the dumps' world bounding boxes: `hemisphere2` 40.2 predicted
/// against 40.2 at age 0.38, the halo 130.4 against 129.4, `hemisphere1` 29.4
/// against 31.3 (a rotated box reads wide). **Fitted, not read**, confidence
/// 75; one constant for the three models because the three fits agree within
/// 6 %.
pub(super) const PULSE_MODEL_UNIT: f32 = 0.025;

/// The age at which the halo stops drawing, seconds. **Chosen, not measured**:
/// the dumps bracket it (drawn at 0.55, gone at 0.717) and nothing between them
/// was dumped. The hemispheres draw to [`PLASMA_BLAST_LIFETIME_SECONDS`].
pub(super) const PULSE_HALO_HIDE_AT_SECONDS: f32 = 0.65;

/// The tint a model carries at `age`, the grey `AMBIENTCOLOR` the GE stream
/// shows (`0x5C`), in the same ordinal order as [`PULSE_BLAST_TARGETS`].
///
/// Three linear ramps with their own end times, read to the byte in 16 dumps:
/// the halo `min(1, 1.25 (1 - age))`, `hemisphere2` `min(1, (1.5 - age) / 1.2)`
/// and `hemisphere1` `1 - age`, each floored at 0. Confidence 85 for the
/// curves, 80 that they are the ramps `PlasmaBlast_Construct` builds.
#[must_use]
pub(super) fn pulse_tints(age: f32) -> [f32; 3] {
    [
        (1.25 * (1.0 - age)).clamp(0.0, 1.0),
        ((1.5 - age) / 1.2).clamp(0.0, 1.0),
        (1.0 - age).clamp(0.0, 1.0),
    ]
}

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
///
/// **A uniform scale on metres-sized models, and the picture is meant to be
/// that big.** The ring and halo are discs of radius 8.8 and 9.0, the sphere
/// a ball of radius 4.1, so at 60 Hz the sphere is a 29-metre bubble within
/// a quarter of a second and the ring a disc of several hundred metres by
/// 1.3 s. The scale was not what made the 2026-09-17 picture wrong; the
/// trio now also culls as its materials say (each shell is authored twice,
/// once each way) and the discs take HD's own right-handed basis. See
/// plasma.md's 2026-09-23 section and `load::weapon_models::cull_as_authored`.
pub(super) const HD_BLAST_TARGETS: [f32; 3] = [100.0, 7.1, 7.0];
/// See [`HD_BLAST_TARGETS`].
pub(super) const HD_BLAST_RATES: [f32; 3] = [0.01, 0.3, 0.2];

/// The three `.vex` files the blast's own detonation loads, grouped into
/// one field on [`options::Loaded`] and one parameter of [`Scene::new`]
/// rather than three of each - the line budget `race/load.rs` and
/// `race/scene.rs` had left for a fifth weapon-model kind was exactly zero.
///
/// **`ball` rides along for the same reason, not because it is a blast
/// model.** It is the bolt's own head - live for the whole flight, not just
/// the detonation - but a true fourth top-level field or parameter on
/// `Scene`/`Scene::new` is exactly the zero-headroom case the paragraph
/// above already describes, and this container already threads through
/// [`options::Loaded`] and [`Scene::new`] as one value. See
/// [`oag_title::weapons::WeaponModels::plasma_ball`].
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
    /// The bolt's own head - see this struct's own doc comment for why it
    /// is here.
    pub ball: Option<Model>,
    /// The Shuriken's blade, which rides here for the same reason.
    pub shuriken: Option<Model>,
}

/// The drawable pools [`Scene`] builds from a [`PlasmaBlastModels`] -
/// grouped for the same reason that is, [`Self::ball`] included.
#[derive(Debug)]
pub(crate) struct PlasmaBlastDrawables {
    pub(crate) halo: Vec<Drawable>,
    pub(crate) hemisphere2: Vec<Drawable>,
    pub(crate) hemisphere1: Vec<Drawable>,
    /// The bolt's own head, one per projectile slot - not one of the
    /// blast's three, see [`PlasmaBlastModels`]'s own doc comment.
    pub(crate) ball: Vec<Drawable>,
    /// The Shuriken's blade, one per projectile slot.
    pub(crate) shuriken: Vec<Drawable>,
}

impl PlasmaBlastDrawables {
    /// Builds every pool with `build_one` - `Scene::new`'s own
    /// `weapon_drawables` closure, reused rather than duplicated so this
    /// pool is built exactly like the Rocket's, the Mine's, the Bomb's and
    /// the Cannon round's.
    pub(crate) fn build(
        models: PlasmaBlastModels,
        mut build_one: impl FnMut(Option<Model>) -> Result<Vec<Drawable>>,
    ) -> Result<Self> {
        Ok(Self {
            halo: build_one(models.halo)?,
            hemisphere2: build_one(models.hemisphere2)?,
            hemisphere1: build_one(models.hemisphere1)?,
            ball: build_one(models.ball)?,
            shuriken: build_one(models.shuriken)?,
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
    /// The track's frame at the detonation, `(heading, up, right)`: what
    /// Pulse's two hemispheres keep for their whole life (world-fixed, not
    /// facing the camera) - see [`pulse_frame`]. Unused on HD.
    pub(super) frame: [Vec3; 3],
    /// Each model's own scale ease, `cur` in
    /// [`PULSE_BLAST_TARGETS`]/[`PULSE_BLAST_RATES`]'s (or, on HD,
    /// [`HD_BLAST_TARGETS`]/[`HD_BLAST_RATES`]'s) own ordinal order.
    ///
    /// **Starts at `0.0`, read on HD.** `WeaponExplosions_Reset` seeds each
    /// `cur` from `0x00994070 + 4n`, which sits in `.bss` with no static
    /// writer - the only other reference to it is a read. Confidence 80: a
    /// runtime write through a computed pointer is not ruled out. See
    /// plasma.md's 2026-09-23 section. Pulse's first drawn tick is one ease
    /// step in, matching the first dump (`target * rate`).
    pub(super) ease: [f32; 3],
}

/// One blast's own transforms, tints and animation clock, for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlasmaBlastDraw {
    /// Each model's own placement with its ease scale folded in, in
    /// [`PlasmaBlastModels`]'s ordinal order (halo, `hemisphere2`,
    /// `hemisphere1`). On HD all three share one basis; on Pulse the halo
    /// takes the camera's orientation and the hemispheres the bolt's frame.
    /// A model that is hidden carries a zero scale.
    pub(super) matrices: [Mat4; 3],
    /// The grey ambient each model draws with, `1.0` on HD (which has no such
    /// ramp read) - see [`pulse_tints`].
    pub(super) tints: [f32; 3],
    /// The blast's age in seconds. On HD every model's animation time
    /// (`WeaponExplosions_Draw` -> `AnimNode_UpdateTransformTree` ->
    /// `MeshImporter_SetTime`) and what each material's `UV_offset` is bound
    /// to; on Pulse the `Node_SetAnimTimeTree` argument of all three models
    /// (`PlasmaBlast_Update` passes the age, not `age * rate`: the node spins
    /// and UV scrolls measured in the dumps are the authored tracks at rate
    /// 1). See plasma.md, 2026-10-05 and 2026-10-30.
    pub(super) clock: f32,
    /// Which title's mechanism drew it: HD binds the clock to its materials
    /// (`Drawable::write_clocked`), Pulse scrubs node and texture tracks.
    pub(super) hd: bool,
}

impl Race {
    /// Starts a render-side blast at `position`, in the first free slot -
    /// dropped silently if all [`PLASMA_BLAST_SLOTS`] are already live, the
    /// same "no evicting a running effect" rule
    /// [`weapons::visuals::advance_one_flare`]'s own flare-stage fallback
    /// takes. The hemispheres' frame is read off the track here, see
    /// [`Race::track_frame_at`].
    ///
    /// Called from [`Race::ignite_blast`] for every Plasma detonation - a
    /// wall hit and a 10 s timeout alike, since `Plasmas_Update`'s own
    /// teardown pass does not branch on which ended the bolt.
    pub(crate) fn spawn_plasma_blast_model(&mut self, position: Vec3) {
        let frame = self.track_frame_at(position);
        if let Some(slot) = self.view.plasma_blasts.iter().position(Option::is_none) {
            self.view.plasma_blasts[slot] = Some(PlasmaBlast {
                position,
                age: 0.0,
                frame,
                ease: [0.0; 3],
            });
        }
    }

    /// `(heading, up, right)` of the circuit at the sample nearest `position`:
    /// the tangent, the sample's own up (the negated down axis, made square to
    /// the heading) and `heading x up`. The axes the original's first
    /// hemisphere keeps are those of the track at the detonation (the dumps'
    /// `(0.995, 0.095, 0)`, `(-0.093, 0.968, 0.234)`, `(0.022, -0.232,
    /// 0.972)` against Talon's Junction there); **that this is the
    /// track-fit `PlasmaBlast_Construct` builds is inferred, not read**. A
    /// circuit with no spline gets the world axes.
    fn track_frame_at(&self, position: Vec3) -> [Vec3; 3] {
        let Some((_, sample, _)) = self.sim.spline.nearest(position) else {
            return [Vec3::NEG_Z, Vec3::Y, Vec3::X];
        };
        let up = (-Vec3::from_array(sample.down))
            .try_normalize()
            .unwrap_or(Vec3::Y);
        let tangent = Vec3::from_array(sample.tangent);
        let heading = (tangent - up * tangent.dot(up))
            .try_normalize()
            .unwrap_or_else(|| up.any_orthonormal_vector());
        [heading, up, heading.cross(up)]
    }

    /// Ages every live blast one tick, steps its three scale eases and
    /// retires whichever just crossed its title's own lifetime -
    /// [`PLASMA_BLAST_LIFETIME_SECONDS`] on Pulse, [`HD_BLAST_LIFETIME_SECONDS`]
    /// on HD, branched on `RaceView::hd_plasma_blast`. On HD this also, the
    /// tick a blast first crosses [`HD_BLAST_HIDE_AT_SECONDS`], plays
    /// `WO_PLASMA_LIGHTNING_COLLAPSE` at its position -
    /// `WeaponExplosions_Collapse`'s own trigger, read 2026-09-17.
    ///
    /// Called from [`Race::tick`], after the impacts loop that can spawn a
    /// new one this same tick - a blast spawned this tick draws at `age ==
    /// 0.0` for the rest of the frame before its first advance, the same
    /// one-tick order [`Race::advance_projectile_flares`] already runs a
    /// freshly-placed flare in.
    pub(crate) fn advance_plasma_blast_models(&mut self, dt: f32) {
        let hd = self.view.hd_plasma_blast;
        // Looked up once, before the loop borrows `plasma_blasts` mutably -
        // `self.view.stage.play` needs `&mut self.view` too, and nothing
        // below can hold both borrows at once.
        let collapse_effect = hd
            .then(|| {
                self.view
                    .handles
                    .get(Trigger::PlasmaLightningCollapse)
                    .cloned()
            })
            .flatten();
        let (targets, rates) = if hd {
            (HD_BLAST_TARGETS, HD_BLAST_RATES)
        } else {
            (PULSE_BLAST_TARGETS, PULSE_BLAST_RATES)
        };
        let mut collapsed_at = Vec::new();
        for slot in &mut self.view.plasma_blasts {
            let Some(blast) = slot else { continue };
            let was_visible = blast.age < HD_BLAST_HIDE_AT_SECONDS;
            blast.age += dt;
            for (ease, (target, rate)) in blast.ease.iter_mut().zip(targets.into_iter().zip(rates))
            {
                *ease += (target - *ease) * rate;
            }
            let lifetime = if hd {
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

    /// This frame's placement, tint and clock for every render-side blast
    /// slot, in slot order - `None` for a slot with nothing live, which the
    /// caller uses to decide what to write and what to draw. `camera` is the
    /// camera's world transform (the inverse of its view matrix).
    ///
    /// **Pulse: the halo follows the camera's orientation, the hemispheres
    /// do not follow the camera at all.** `hemisphere1`'s world basis is
    /// exactly constant over 16 dumps while the view turns, `hemisphere2`
    /// spins about a world axis at the node track's own rate, and the halo's
    /// axis moves only as the camera's own direction does (5 degrees in 0.37
    /// s while the eye travels through the shell): see [`pulse_frame`] and
    /// [`pulse_halo_frame`]. That a one-sided `hemisphere1` is world-fixed is
    /// what lets the craft's own travel carry the eye through its wall, which
    /// is the white-out's cut at age 0.25 (plasma.md, 2026-10-30; confidence
    /// 80). **Which pose the hemispheres' frame is derived from is chosen**,
    /// see [`pulse_frame`].
    ///
    /// **On HD the basis is read, and is two different ones.**
    /// `WeaponExplosions_Start` gives the ring and the halo a per-viewport
    /// camera-facing basis with `Z` pointing away from the viewer - which
    /// [`facing_away`] reproduces - and the sphere the track-fitted basis
    /// `FUN_000a97f0` returns, unread past its call. The sphere takes the
    /// same camera-facing basis here as a stated substitute for that fit:
    /// it is round, so only where its texture's seam falls differs.
    /// **Chosen, not measured.** Each model is then scaled uniformly by its
    /// own `cur` - read, confidence 90, see plasma.md's 2026-09-23 section.
    #[must_use]
    pub(crate) fn plasma_blast_draws(
        &self,
        camera: Mat4,
    ) -> [Option<PlasmaBlastDraw>; PLASMA_BLAST_SLOTS] {
        let hd = self.view.hd_plasma_blast;
        std::array::from_fn(|slot| {
            self.view.plasma_blasts[slot].map(|blast| {
                if hd {
                    let matrix =
                        facing_away(billboard_matrix(blast.position, camera.w_axis.truncate()));
                    let visible = blast.age < HD_BLAST_HIDE_AT_SECONDS;
                    PlasmaBlastDraw {
                        matrices: blast.ease.map(|ease| {
                            matrix * Mat4::from_scale(Vec3::splat(if visible { ease } else { 0.0 }))
                        }),
                        tints: [1.0; 3],
                        clock: blast.age,
                        hd: true,
                    }
                } else {
                    let hemispheres = pulse_frame(blast.position, blast.frame);
                    let halo = pulse_halo_frame(blast.position, camera);
                    let halo_scale = if blast.age < PULSE_HALO_HIDE_AT_SECONDS {
                        blast.ease[0]
                    } else {
                        0.0
                    };
                    let unit = |ease: f32| Mat4::from_scale(Vec3::splat(ease * PULSE_MODEL_UNIT));
                    PlasmaBlastDraw {
                        matrices: [
                            halo * unit(halo_scale),
                            hemispheres * unit(blast.ease[1]),
                            hemispheres * unit(blast.ease[2]),
                        ],
                        tints: pulse_tints(blast.age),
                        clock: blast.age,
                        hd: false,
                    }
                }
            })
        })
    }
}

/// The frame Pulse's two hemispheres are placed in, world-fixed from the
/// detonation: local `x` along the track's heading, `y` its up and `z` its
/// right, translated to `position` ([`Race::track_frame_at`] has what this
/// frame is read off and what is inferred).
fn pulse_frame(position: Vec3, [heading, up, right]: [Vec3; 3]) -> Mat4 {
    Mat4::from_cols(
        heading.extend(0.0),
        up.extend(0.0),
        right.extend(0.0),
        position.extend(1.0),
    )
}

/// The halo's frame: the camera's own orientation and nothing of its
/// position - local `z` along the way the camera looks, `y` its up, `x` its
/// left (the dumps' rows `(0.043, -0.012, -0.999)`, `(0.050, 0.999, -0.010)`,
/// `(0.998, -0.049, 0.044)` while the camera heads along `+x`). Confidence
/// 70: one capture heading, and the orientation-only reading is the one
/// `PlasmaBlast_Update`'s read of the camera's `+0x48/+0x58/+0x68` (three
/// `z` components, negated and normalised) fits (plasma.md).
fn pulse_halo_frame(position: Vec3, camera: Mat4) -> Mat4 {
    let look = camera.transform_vector3(Vec3::NEG_Z).normalize_or_zero();
    let up = camera.transform_vector3(Vec3::Y).normalize_or_zero();
    Mat4::from_cols(
        up.cross(look).extend(0.0),
        up.extend(0.0),
        look.extend(0.0),
        position.extend(1.0),
    )
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
    // `reference x forward`, then `forward x right`: a rotation. The other
    // order (this function's own until 2026-09-24) is a reflection, which
    // drew Pulse's blast shells mirrored - see
    // `Race::projectile_model_matrices`'s own doc comment for the same fix.
    let right = reference.cross(forward).normalize_or_zero();
    let up = forward.cross(right);
    Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        forward.extend(0.0),
        position.extend(1.0),
    )
}

/// `billboard_matrix`'s basis rebuilt the way Wipeout HD builds its own for
/// the two discs: `Z` pointing **away** from the camera, the billboard's up
/// kept, and `X = Y x Z` - a rotation, as `billboard_matrix` itself now is
/// too (it was a reflection until 2026-09-24; fixing it negated `X` alone,
/// so this function's output did not move). The handedness matters once back
/// faces are culled: a reflection reverses every triangle's winding on
/// screen, so the cull would hide exactly the faces the original draws.
///
/// The cross product is the original's too: `Start` rebuilds the first row
/// from the other two (`0x001281b4`..`0x001281cc`, a `vperm`/`vmaddfp`/
/// `vnmsubfp` cross).
///
/// `WeaponExplosions_Start` (`0x00127cd0`) writes each viewport's basis with
/// its third row the **negated** vector it read out of that viewport's camera
/// matrix (`vsubfp` from zero at `0x001280e8`), and the halo is authored as a
/// single-sided disc whose normals all point down `-Z` under a material that
/// culls back faces - so the only orientation in which the original can show
/// it at all is `-Z` towards the viewer. Confidence 80: which camera column
/// the vector is was not traced element by element, and the disc's own
/// authoring is what settles the sign. See plasma.md's 2026-09-23 section.
fn facing_away(matrix: Mat4) -> Mat4 {
    let up = matrix.y_axis.truncate();
    let away = -matrix.z_axis.truncate();
    Mat4::from_cols(
        up.cross(away).extend(0.0),
        matrix.y_axis,
        away.extend(0.0),
        matrix.w_axis,
    )
}

#[cfg(test)]
mod tests;
