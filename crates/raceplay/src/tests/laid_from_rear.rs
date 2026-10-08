//! Where a charge is laid: Pulse at the body, Pure at the rear anchor.

use super::*;
use oag_gameplay::PlayerInputs;

/// The Mine's laid position, as its distance behind the craft's body along its
/// own forward, after one tick of a one-charge drop.
fn laid_back_by(from_rear: bool) -> f32 {
    let mut race = race_with_a_grid();
    race.view.laid_from_rear = from_rear;
    race.sim.weapons = Some(one_mine_table());
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Mine);
    race.sim.world.ships[0].pickup.begin_drop(1);
    race.tick(&PlayerInputs::none());
    let body = race.sim.world.ships[0].physics.body;
    let laid = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .find(|p| p.kind == Some(oag_tables::weapons::Weapon::Mine))
        .expect("the drop laid a mine")
        .position;
    (body.position - laid).dot(body.forward())
}

/// Pure's `Mine_Init` and `Bomb_Init` take the rear anchor, measured 4.875
/// behind the body; Pulse's is the body itself. Dropping the rule puts Pure's
/// charges back inside the hull, where the original's are not.
#[test]
fn a_title_that_lays_from_the_rear_starts_its_charge_behind_the_body() {
    let at_body = laid_back_by(false);
    let at_rear = laid_back_by(true);
    assert!(at_body.abs() < 0.5, "Pulse lays at the body: {at_body}");
    assert!(
        (at_rear - 4.875).abs() < 0.5,
        "Pure lays at the rear anchor: {at_rear}"
    );
}
