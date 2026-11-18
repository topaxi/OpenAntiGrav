//! Wipeout HD's Plasma wind-up and release effects and its bolt ball's frame
//! (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "The effects and their triggers" and
//! "The bolt's two frames").

use super::*;

use oag_tables::weapons::Weapon;
use oag_title::engine_effects::{PLASMA_CHARGING_EFFECT, PLASMA_LAUNCH_EFFECT};

/// A grid with one Plasma bolt of slot 0's in projectile slot 0, `charge` seconds of
/// wind-up left, and the two HD effects lent to their triggers when `hd`.
fn grid_with_a_bolt(hd: bool, charge: f32) -> Race {
    let mut race = race_with_a_grid();
    race.core.sim.world.ships[0].muzzle = Some(Vec3::new(0.0, 0.0, 6.0));
    if hd {
        let charging = super::trail_hits::a_one_shot_burst(PLASMA_CHARGING_EFFECT);
        let launch = super::trail_hits::a_one_shot_burst(PLASMA_LAUNCH_EFFECT);
        race.view.handles = std::mem::take(&mut race.view.handles)
            .lending(Trigger::PlasmaCharging, Some(Arc::new(charging)))
            .lending(Trigger::PlasmaLaunch, Some(Arc::new(launch)));
    }
    let bolt = &mut race.core.sim.world.projectiles.slots[0];
    bolt.kind = Some(Weapon::Plasma);
    bolt.owner = 0;
    bolt.charge = charge;
    bolt.position = Vec3::new(0.0, 1.0, -5.0);
    bolt.velocity = Vec3::new(0.0, 0.0, -80.0);
    bolt.surface = Vec3::Y;
    race
}

fn advance(race: &mut Race) {
    race.advance_projectile_flares();
    race.advance_plasma_launch();
}

/// **Red to green.** `Plasma_Init` starts the wind-up effect with the bolt, and
/// `Plasma_Launch` the release effect on the tick the wind-up ends, once.
#[test]
fn a_charging_bolt_rides_the_wind_up_effect_and_its_release_plays_the_launch_once() {
    let mut race = grid_with_a_bolt(true, 0.5);
    assert_eq!(race.view.stage.playing_count(), 0);
    advance(&mut race);
    assert_eq!(
        race.view.stage.playing_count(),
        1,
        "the wind-up effect rides the charging bolt, and nothing has left yet"
    );
    advance(&mut race);
    assert_eq!(
        race.view.stage.playing_count(),
        1,
        "still one while it charges"
    );
    race.core.sim.world.projectiles.slots[0].charge = 0.0;
    advance(&mut race);
    assert_eq!(
        race.view.stage.playing_count(),
        2,
        "the release effect starts on the tick the wind-up ends"
    );
    advance(&mut race);
    assert_eq!(race.view.stage.playing_count(), 2, "once per bolt");
}

/// A title whose table names neither effect plays nothing for a Plasma bolt.
#[test]
fn a_title_with_neither_effect_plays_none() {
    let mut race = grid_with_a_bolt(false, 0.5);
    advance(&mut race);
    race.core.sim.world.projectiles.slots[0].charge = 0.0;
    advance(&mut race);
    assert_eq!(race.view.stage.playing_count(), 0);
}

/// The ball's matrix is the live object's: model `X` down the carried up, `Z` along the
/// firing direction, rows `0.75^2`, `0.75^2` and `0.75^3` long.
#[test]
fn the_bolt_ball_is_squashed_along_its_heading_at_the_measured_scale() {
    let race = grid_with_a_bolt(false, 0.0);
    let bolt = race.core.sim.world.projectiles.slots[0];
    let matrix = race.projectile_matrix(Weapon::Plasma, 0, &bolt);
    let (x, y, z) = (
        matrix.x_axis.truncate(),
        matrix.y_axis.truncate(),
        matrix.z_axis.truncate(),
    );
    assert!((x.length() - 0.5625).abs() < 1e-6, "{x:?}");
    assert!((y.length() - 0.5625).abs() < 1e-6, "{y:?}");
    assert!((z.length() - 0.421_875).abs() < 1e-6, "{z:?}");
    assert!(x.normalize().dot(-Vec3::Y) > 0.999_99, "X is down: {x:?}");
    assert!(
        z.normalize().dot(Vec3::NEG_Z) > 0.999_99,
        "Z is the heading: {z:?}"
    );
    assert!(x.cross(y).dot(z) > 0.0, "a rotation, not a reflection");
    assert_eq!(matrix.w_axis.truncate(), bolt.position);
}

/// The head's and the wind-up's emitters emit about the firing craft's backward axis.
#[test]
fn the_bolts_effect_frame_points_its_y_back_along_the_heading() {
    let up = crate::plasma_launch::effect_up(Vec3::new(0.0, 0.0, -80.0));
    assert!((up - Vec3::Z).length() < 1e-6, "{up:?}");
    assert_eq!(crate::plasma_launch::effect_up(Vec3::ZERO), Vec3::Y);
}
