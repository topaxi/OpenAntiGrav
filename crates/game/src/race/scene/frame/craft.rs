//! Two shield questions and one flame transform, per craft.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. All three
//! are pure functions of the race state that [`super::Scene::render`] asks
//! once per craft per frame - which is what made them the seam: they are the
//! only things in that file that do not touch a pass, an encoder or the
//! scene's own state.

use oag_core::math::{Mat4, Vec3};

use crate::race::Race;

/// Whether one craft's hull-shaped shield shell is drawn this frame.
///
/// **The player's is swapped for the cockpit sphere, not merely hidden.**
/// `ShipShield_Update` branches on `craft+0x6d` and draws the shell for an
/// external camera or the sphere for an internal one, clearing the other's draw
/// flag - so exactly one is up per craft. `craft+0x6d` is the same byte
/// [`crate::display::CameraView::draws_own_ship`] reads, which is why that is
/// the test here rather than a second source of truth.
///
/// Only slot 0 can take the cockpit branch: the byte is set for the local
/// player alone, and an opponent's shell is drawn from outside whatever the
/// player's camera is doing.
pub(super) fn shell_visible(race: &Race, slot: usize) -> bool {
    race.shield_of(slot).visible() && (slot != 0 || race.draws_own_ship())
}

/// Whether the cockpit sphere is drawn this frame: the player's shield is up
/// and the camera is inside the hull.
pub(super) fn cockpit_shield_visible(race: &Race) -> bool {
    !race.draws_own_ship() && race.shield_of(0).visible() && race.ship_active(0)
}

/// The scale HD's flame groups breathe with, about the nozzle the flare model
/// is baked at - identity on every other source and where no locator exists.
///
/// `EF_Main` scales its cross-section and length from the smoothed throttle
/// plus the boost blend; `EF_Boost` only its length, by `blend * 2.0`. The
/// spikes' per-shape random flicker (`Spikes Random Scale Min/Max`, 0.65 to
/// 0.85) needs a per-shape split this scene does not carry yet and is
/// documented rather than drawn - see
/// `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`.
pub(super) fn hd_flame_transform(race: &Race, slot: usize, boost: bool) -> Mat4 {
    if !race.hd_trail_active() {
        return Mat4::IDENTITY;
    }
    let Some(nozzle) = race.nozzle_local_of(slot) else {
        return Mat4::IDENTITY;
    };
    let flame = race.hd_flame_of(slot);
    let scale = if boost {
        Vec3::new(1.0, 1.0, flame.boost_scale_z())
    } else {
        let (xy, z) = flame.main_scale();
        Vec3::new(xy, xy, z)
    };
    Mat4::from_translation(nozzle) * Mat4::from_scale(scale) * Mat4::from_translation(-nozzle)
}
