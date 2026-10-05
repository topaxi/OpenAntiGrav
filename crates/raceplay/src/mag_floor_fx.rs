//! The magstrip effect: `Data\visual_effects\MagEffect1.vex` and
//! `MagEffect2.vex`, shown under a craft while it is on a magstrip. See
//! `docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md`.
//!
//! **Recovered, confidence 95** (`MagFloorFx_Construct` `0x088590a8`,
//! `MagFloorFx_Update` `0x0885962c`, `MagFloorFx_Show`/`Hide`
//! `0x088598ac`/`0x088598d8`; the PS2 build's three counterparts agree; anchor
//! and tint read live on the running original):
//!
//! - **Every craft has one.** `Craft_Construct` builds it unconditionally and
//!   stores it at `craft+0x8bc`, so the player and every AI craft show it.
//! - **Shown exactly while this tick's mag-floor probe hits.**
//!   `Ship_CastHoverProbes` writes `craft+0x240` from a probe restricted to
//!   surface type 3 and calls `Show` on its rising edge and `Hide` on its
//!   falling edge - see [`contact_this_tick`] for how that bool is read back
//!   off the simulation here.
//! - **Both sit at `(0, -2.5, 0)` in the craft's own frame** - identity with
//!   row 3's `y` overwritten by `[0x08ab0ef0] = -2.5`.
//! - **MagEffect1 rides the craft** ([`ride_matrix`]): a translate-only child,
//!   so it keeps the craft's rotation, barrel roll and `0.75` scale.
//! - **MagEffect2 lies on the track** ([`lie_matrix`]): a child of the scene
//!   root that `MagFloorFx_Update` rebuilds every tick - up is the track's,
//!   forward the craft's nose made perpendicular to it, unit scale.
//! - **No tint.** `Image_SetVertexColours` stamps both with `0xffffffff`
//!   (`MagFloorFx_InitTint` `0x08859904` writes `1.0` four times), so the
//!   authored vertex colours draw as they are.
//!
//! - **Both play their authored animation**: the lightning's and the halo's
//!   texture scroll and the halo's spin, on the one animation clock (see
//!   `Scene::write_mag_floor_fx`).
//!
//! **Chosen, not measured:** the track's down comes from the spline sample
//! nearest the craft, where the original reads its located sample
//! (`craft+0xb10`) - the same stand-in the simulation's own mag lock is fed
//! (`tick.rs`).
//!
//! There is no sound to go with it: nothing in the original plays one on a
//! magstrip (same page, "the three remaining routes to a per-craft hum").

use super::*;
use oag_weapons::MAX_SHIPS;

/// Where both models sit in the craft's own frame: `[0x08ab0ef0]`, written
/// into the translation row's `y` by `MagFloorFx_Construct` and
/// `MagFloorFx_Update` alike (PS2: `[0x0027e950]`, the same `0xc0200000`).
pub(super) const OFFSET: Vec3 = Vec3::new(0.0, -2.5, 0.0);

/// Set to `on` to draw every craft's effect whether or not it is over a
/// magstrip - a way to compare the look against the original on a pose that
/// has no strip. Read on the view side only (the simulation never sees it),
/// and absent from a normal run.
pub(super) const FORCE_ENV: &str = "OAG_MAGFX";

/// Whether this tick's mag-floor probe hit, read back off the blend it drove.
///
/// The simulation keeps `craft+0x240` only as the input to
/// [`oag_physics::maglock::ramp`], which moves the blend `0.2` toward `1.0`
/// on a hit and toward `0.0` on a miss, clamped. So a hit is a blend that
/// rose, or one that is pinned at `1.0` (a miss from `1.0` leaves `0.8`); a
/// miss is one that fell or stayed at `0.0`. Exact for every pair the ramp can
/// produce, which `the_ramp_reads_back_as_the_probe_it_was_given` pins.
#[must_use]
pub(super) fn contact_this_tick(previous: f32, now: f32) -> bool {
    now > previous || now == 1.0
}

/// MagEffect1's model matrix: the craft's own `model` matrix with [`OFFSET`]
/// added in its frame. `Vex_UpdateNodeWorldMatrix`'s translate-only branch
/// keeps the parent's rows and carries the local translation through them.
#[must_use]
pub(super) fn ride_matrix(model: Mat4) -> Mat4 {
    model * Mat4::from_translation(OFFSET)
}

/// MagEffect2's model matrix, `MagFloorFx_Update`'s rows: up is the track's
/// (`-down`, normalised), forward is the craft's own nose with its component
/// along up removed, side is `up x forward`, all unit length; the translation
/// is the craft's own `model` carrying [`OFFSET`].
///
/// `model`'s columns are the original's world rows, so its column 2 is the
/// nose `W.row2` the original orthogonalises.
#[must_use]
pub(super) fn lie_matrix(model: Mat4, down: Vec3) -> Mat4 {
    let up = (-down).normalize_or_zero();
    let nose = model.z_axis.truncate();
    let forward = (nose - up * up.dot(nose)).normalize_or_zero();
    let side = up.cross(forward);
    Mat4::from_cols(
        side.extend(0.0),
        up.extend(0.0),
        forward.extend(0.0),
        model.transform_point3(OFFSET).extend(1.0),
    )
}

/// One craft's effect: shown or not, and the blend the last tick left.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Shown {
    previous_blend: f32,
    pub(super) on: bool,
}

impl Race {
    /// Reads every craft's mag-floor contact for the tick just run. Called
    /// once a tick from [`Race::tick`], after the simulation.
    ///
    /// Deliberately not from `Race::tick_cosmetics`: that path freezes the
    /// world, so the effect freezes with the craft it hangs under.
    pub(crate) fn advance_mag_floor_fx(&mut self) {
        for (slot, shown) in self.view.mag_floor_fx.iter_mut().enumerate() {
            let blend = self.sim.world.ships[slot].physics.mag_lock_blend;
            shown.on = contact_this_tick(shown.previous_blend, blend);
            shown.previous_blend = blend;
        }
    }

    /// This frame's `[MagEffect1, MagEffect2]` model matrices per ship slot,
    /// `None` for a craft off the strip, out of play, or with no spline
    /// sample under it.
    #[must_use]
    pub(crate) fn mag_floor_fx_draws(&self) -> [Option<[Mat4; 2]>; MAX_SHIPS] {
        let forced = std::env::var_os(FORCE_ENV).is_some_and(|v| v == "on");
        std::array::from_fn(|slot| {
            if slot >= usize::from(self.sim.world.ship_count)
                || !self.ship_active(slot)
                || !(forced || self.view.mag_floor_fx[slot].on)
            {
                return None;
            }
            let model = self.ship_model_matrix_of(slot);
            let position = self.sim.world.ships[slot].physics.body.position;
            let (_, sample, _) = self.sim.spline.nearest(position)?;
            Some([
                ride_matrix(model),
                lie_matrix(model, Vec3::from_array(sample.down)),
            ])
        })
    }
}

#[cfg(test)]
mod tests;
