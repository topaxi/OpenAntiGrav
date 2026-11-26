//! The weapon pads spend a title's pickup roster: the allowed list gates both draws, and the
//! player order carries a person's roll past a disallowed weapon.

use super::*;
use oag_gameplay::{Controller, PlayerInputs};
use oag_tables::weapons::Weapon;

/// A table weighting only the Shuriken, beside an authored Rocket with no weight at all: a
/// person's roll can reach the Rocket only by being passed on to it.
fn shuriken_only_odds() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Rocket"><Stats ai="0" back="0" front="0" human="0"/></Weapon>
               <Weapon type="Shuriken"><Stats ai="1" back="0" front="0" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}

/// Slot 0 is a person and slot 1 an AI, both standing on the pad with empty slots.
fn two_on_a_pad() -> Race {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        0.0,
        shuriken_only_odds(),
    );
    let mut second = race.core.sim.world.ships[0];
    second.place_at(oag_gameplay::Pose {
        position: second.physics.body.position + second.physics.body.right() * 60.0,
        orientation: second.physics.body.orientation,
    });
    race.core.sim.world.ships[1] = second;
    race.core.sim.world.ship_count = 2;
    race.core.sim.world.controllers[1] = Controller::Ai;
    race.core.sim.allowed_weapons = Arc::from([Weapon::Rocket]);
    race.core.sim.pickup_order = Some(&[Weapon::Shuriken, Weapon::Rocket]);
    race
}

#[test]
fn a_pad_passes_a_persons_disallowed_roll_on_and_hands_the_ai_nothing() {
    let mut race = two_on_a_pad();
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.core.sim.world.ships[0].pickup.weapon,
        Some(Weapon::Rocket),
        "the person's Shuriken roll goes to the next allowed weapon"
    );
    assert_eq!(
        race.core.sim.world.ships[1].pickup.weapon, None,
        "the AI's roll leaves the disallowed Shuriken out, and nothing else is weighted"
    );
}
