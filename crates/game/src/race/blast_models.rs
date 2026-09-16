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

/// One live render-side blast: where it detonated, and how long ago.
///
/// **View state, not `World` state** - see this module's own doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlasmaBlast {
    pub(super) position: Vec3,
    pub(super) age: f32,
}

/// One blast's own transform and per-model anim-time scrub, for one frame.
///
/// **One `matrix` serves all three models.** `PlasmaBlast_Update`'s own
/// per-model scale/offset nudge - the first float of each of the three
/// `DAT_08ab0f34`/`f44`/`f54` triples `PlasmaBlast_Construct` reads - is
/// `0.0` for every model (see plasma.md), so the halo and both hemispheres
/// sit at the identical transform in this engine too and differ only in
/// which `.vex` they are and which anim-time value plays their own baked
/// motion, if any.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlasmaBlastDraw {
    pub(super) matrix: Mat4,
    pub(super) halo_seconds: f32,
    pub(super) hemisphere2_seconds: f32,
    pub(super) hemisphere1_seconds: f32,
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
            self.view.plasma_blasts[slot] = Some(PlasmaBlast { position, age: 0.0 });
        }
    }

    /// Ages every live blast one tick and retires whichever just crossed
    /// [`PLASMA_BLAST_LIFETIME_SECONDS`].
    ///
    /// Called from [`Race::tick`], after the impacts loop that can spawn a
    /// new one this same tick - a blast spawned this tick draws at `age ==
    /// 0.0` for the rest of the frame before its first advance, the same
    /// one-tick order [`Race::advance_projectile_flares`] already runs a
    /// freshly-placed flare in.
    pub(in crate::race) fn advance_plasma_blast_models(&mut self, dt: f32) {
        for slot in &mut self.view.plasma_blasts {
            let Some(blast) = slot else { continue };
            blast.age += dt;
            if blast.age >= PLASMA_BLAST_LIFETIME_SECONDS {
                *slot = None;
            }
        }
    }

    /// This frame's transform and anim-time scrub for every render-side
    /// blast slot, in slot order - `None` for a slot with nothing live,
    /// which the caller uses to decide what to write and what to draw.
    ///
    /// `camera_position` decides the basis. **`PlasmaBlast_Update`'s own
    /// camera read is not fully resolved** - plasma.md's own hedge is that
    /// it is *some* function of the active camera's current orientation,
    /// not established to be any one specific axis - so this draws the
    /// ordinary "face the camera" billboard as a stated substitute for that
    /// unresolved vector math. **Chosen, not measured; no confidence
    /// score**, the same footing [`weapons::visuals::advance_quake_visual`]
    /// already gives its own unread orientation term. The rest of what this
    /// returns is the recovered reading: the anchor position (the bolt's own
    /// impact point, not a second track-surface probe), the three anim-time
    /// rates, and the absence of any per-model scale beyond the model's own
    /// authored size.
    #[must_use]
    pub(in crate::race) fn plasma_blast_draws(
        &self,
        camera_position: Vec3,
    ) -> [Option<PlasmaBlastDraw>; PLASMA_BLAST_SLOTS] {
        std::array::from_fn(|slot| {
            self.view.plasma_blasts[slot].map(|blast| PlasmaBlastDraw {
                matrix: billboard_matrix(blast.position, camera_position),
                halo_seconds: blast.age * PLASMA_BLAST_HALO_ANIM_RATE,
                hemisphere2_seconds: blast.age * PLASMA_BLAST_HEMISPHERE2_ANIM_RATE,
                hemisphere1_seconds: blast.age * PLASMA_BLAST_HEMISPHERE1_ANIM_RATE,
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
