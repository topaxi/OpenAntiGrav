//! HD's sprite flare at the draw: `Race::hd_sprite_quad`.
//!
//! The law itself is tested where it lives, `oag_render::exhaust::hd`; what
//! these pin is the part only a `Race` can decide - which craft gets a quad
//! at all, and that the camera and the nozzle feed the law the way the
//! original's draw does (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`,
//! "Ninth session" and "Tenth session").

use super::*;
use crate::race::effects::FOLLOWED_SLOT;

/// A grid whose flares are HD's, with every craft's nozzle authored at its
/// own origin so the world-space nozzle is the craft's position.
fn race_with_hd_flares() -> Race {
    let mut race = race_with_a_grid();
    race.view.hd_trail_active = true;
    race.view.nozzles = vec![Some(Vec3::ZERO); oag_gameplay::MAX_SHIPS];
    for slot in 0..oag_gameplay::MAX_SHIPS {
        for _ in 0..10 {
            race.view.hd_sprite[slot].advance(|| 0.5);
        }
    }
    race
}

/// The camera's right and up, read off the view matrix the way the frame
/// does.
fn camera_axes(race: &Race) -> (Vec3, Vec3) {
    let camera = race.view();
    (
        Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x),
        Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y),
    )
}

/// The owner gate: the craft the view belongs to gets no sprite, every other
/// craft the camera can look into the nozzle of gets one.
#[test]
fn the_players_own_craft_gets_no_sprite_and_an_opponent_ahead_does() {
    let mut race = race_with_hd_flares();
    // Slot 1 straight ahead of the player on the +X straight, facing the
    // same way: the chase camera behind slot 0 looks along +X into slot 1's
    // nozzle, a few units off - well inside `Flare Fadeout Dist`.
    let player = race.ship().physics.body.position;
    race.sim.world.ships[1].physics.body.position = player + Vec3::X * 8.0;
    race.sim.world.ships[1].physics.body.orientation = race.ship().physics.body.orientation;
    let (right, up) = camera_axes(&race);
    assert!(
        race.hd_sprite_quad(FOLLOWED_SLOT, right, up).is_empty(),
        "the original turns the viewing player's craft away at 0x002a0bb4"
    );
    let quad = race.hd_sprite_quad(1, right, up);
    assert_eq!(quad.len(), 6, "an opponent ahead is one quad");
    // Its alpha is the fade, and on axis inside the fadeout the fade is the
    // walk itself - not the walk ceilinged or the walk squared.
    let expected = race.view.hd_sprite[1].alpha_walk();
    let alpha = quad[0].colour[3];
    assert!(
        alpha > 0.0 && alpha <= expected + 1e-6,
        "{alpha} vs {expected}"
    );
    // Off HD nothing draws at all, whatever the slot.
    race.view.hd_trail_active = false;
    assert!(race.hd_sprite_quad(1, right, up).is_empty());
}

/// The hemisphere and the range, through the race's own camera: an opponent
/// whose nozzle points away yields nothing, one past the fade range yields a
/// quad at alpha 0.
#[test]
fn an_opponent_facing_the_camera_or_beyond_the_fadeout_range_draws_nothing_visible() {
    let mut race = race_with_hd_flares();
    let player = race.ship().physics.body.position;
    let (right, up) = camera_axes(&race);
    // Turned about: the nozzle now points along +X, away from the eye.
    race.sim.world.ships[1].physics.body.position = player + Vec3::X * 8.0;
    race.sim.world.ships[1].physics.body.orientation =
        race.ship().physics.body.orientation * Quat::from_rotation_y(std::f32::consts::PI);
    assert!(
        race.hd_sprite_quad(1, right, up).is_empty(),
        "a non-positive view dot is the original's early-out, no quad"
    );
    // Facing the right way but past `Dist + Range` from the eye.
    race.sim.world.ships[1].physics.body.orientation = race.ship().physics.body.orientation;
    race.sim.world.ships[1].physics.body.position = player + Vec3::X * 200.0;
    let quad = race.hd_sprite_quad(1, right, up);
    assert_eq!(quad.len(), 6);
    assert_eq!(quad[0].colour[3], 0.0, "a quad at alpha 0, not no quad");
}
