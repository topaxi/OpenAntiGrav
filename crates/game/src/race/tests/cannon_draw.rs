//! Where a Cannon round's model and flash are drawn, on each title's terms -
//! see `race/weapons/visuals/cannon.rs`.

use super::*;
use crate::race::CannonDraw;
use oag_gameplay::PlayerInputs;

/// A race with one live Cannon round fired from slot 0's own left side,
/// `ticks` ticks after the shot, the shooter pinned at the origin.
fn one_round(ticks: usize) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_cannon_table());
    let mut race = Race::start(setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Cannon);
    let mut buttons = Buttons::new();
    let mut fired = None;
    for _ in 0..120 {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
        race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));
        if race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .any(|p| p.kind.is_some())
        {
            fired = Some(());
            break;
        }
    }
    fired.expect("held fire lays a Cannon round within two seconds");
    // Only the first round: clear any other so the window test reads one.
    for _ in 0..ticks {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
        race.tick(&PlayerInputs::single(buttons.tick(0)));
    }
    race
}

/// HD's look, with a left and a right muzzle a unit either side of the hull's
/// own origin and a fixed height, so a matrix can be told apart by side.
fn hd_draw() -> CannonDraw {
    let left = Mat4::from_translation(Vec3::new(1.0, 0.5, 0.0));
    let right = Mat4::from_translation(Vec3::new(-1.0, 0.5, 0.0));
    CannonDraw {
        look: Some(oag_hd::race::CANNON_LOOK),
        muzzles: vec![[Some(left), Some(right)]; 8],
    }
}

fn cannon_rounds(race: &Race) -> usize {
    race.sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(oag_tables::weapons::Weapon::Cannon))
        .count()
}

#[test]
fn pulse_hangs_its_model_on_every_live_round() {
    let race = one_round(10);
    let matrices = race.cannon_model_matrices(&CannonDraw::default());
    assert_eq!(matrices.len(), cannon_rounds(&race));
    assert!(!matrices.is_empty());
}

#[test]
fn hd_draws_its_model_at_the_muzzle_only_inside_the_flash_window() {
    let race = one_round(0);
    let draw = hd_draw();
    let matrices = race.cannon_model_matrices(&draw);
    assert_eq!(matrices.len(), 1, "one round just fired, one flash");
    let ship = race.ship_model_matrix_of(0);
    let at = matrices[0].w_axis.truncate();
    let left = ship.transform_point3(Vec3::new(1.0, 0.5, 0.0));
    let right = ship.transform_point3(Vec3::new(-1.0, 0.5, 0.0));
    assert!(
        at.distance(left) < 1e-4 || at.distance(right) < 1e-4,
        "the flash sits on a cannon_flash locator, not the round: {at:?}"
    );
    // The same round half a second on, fire released: still in flight, and
    // no flash model at all.
    let later = one_round(30);
    assert!(cannon_rounds(&later) > 0, "the round is still flying");
    assert!(
        later.cannon_model_matrices(&draw).is_empty(),
        "the flash is gone once the round is past 0.1 s"
    );
}

#[test]
fn hd_flash_side_follows_the_side_the_round_left_from() {
    let race = one_round(0);
    let body = &race.sim.world.ships[0].physics.body;
    let round = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .find(|p| p.kind.is_some())
        .expect("a live round");
    let on_left = (round.position - body.position).dot(body.right()) < 0.0;
    let at = race.cannon_model_matrices(&hd_draw())[0].w_axis.truncate();
    let ship = race.ship_model_matrix_of(0);
    let expected = if on_left {
        Vec3::new(1.0, 0.5, 0.0)
    } else {
        Vec3::new(-1.0, 0.5, 0.0)
    };
    assert!(at.distance(ship.transform_point3(expected)) < 1e-4);
}

#[test]
fn hd_centres_its_flash_quad_on_the_muzzle_and_draws_none_without_one() {
    let race = one_round(0);
    let (mut bolt, mut flash) = (Vec::new(), Vec::new());
    race.cannon_quad_vertices(&hd_draw(), Vec3::X, Vec3::Y, &mut bolt, &mut flash);
    assert_eq!(bolt.len(), 12 * cannon_rounds(&race));
    assert_eq!(flash.len(), 6);
    let centre = flash
        .iter()
        .fold(Vec3::ZERO, |sum, v| sum + Vec3::from(v.position))
        / 6.0;
    let ship = race.ship_model_matrix_of(0);
    let muzzles = [Vec3::new(1.0, 0.5, 0.0), Vec3::new(-1.0, 0.5, 0.0)];
    // The six list vertices repeat two corners, so their mean is not the
    // square's centre; the two diagonal corners' midpoint is.
    let mid = (Vec3::from(flash[0].position) + Vec3::from(flash[4].position)) / 2.0;
    assert!(
        muzzles
            .iter()
            .any(|m| mid.distance(ship.transform_point3(*m)) < 1e-3),
        "flash centred at {mid:?} (list mean {centre:?})"
    );

    let bare = CannonDraw {
        look: Some(oag_hd::race::CANNON_LOOK),
        muzzles: Vec::new(),
    };
    let (mut bolt, mut flash) = (Vec::new(), Vec::new());
    race.cannon_quad_vertices(&bare, Vec3::X, Vec3::Y, &mut bolt, &mut flash);
    assert!(
        flash.is_empty(),
        "no locator, no flash - never one at the round"
    );
    assert!(!bolt.is_empty(), "the streak still draws");
}
