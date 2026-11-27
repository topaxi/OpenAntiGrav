//! The weapon pads spend a title's pickup roster: the allowed list gates both draws, the
//! player order carries a person's roll past a disallowed weapon, the AI's draw repeats where
//! the roster says so, and the one-live exclusions take a weapon the field holds or runs out of
//! both draws.

use super::*;
use oag_gameplay::{Controller, PlayerInputs};
use oag_tables::weapons::Weapon;
use oag_title::pickup_roster::PickupRoster;

/// A roster restricting nothing by itself: the tests set what they measure.
const ROSTER: PickupRoster = PickupRoster {
    race: &[],
    eliminator: &[],
    player_order: &[Weapon::Shuriken, Weapon::Rocket],
    ai_refuses_repeat: true,
    exclusions: &[],
};

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
    race.core.sim.pickup_roster = Some(ROSTER);
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

/// A table weighting `weighted` alone, at 1 in both columns, beside an authored Rocket weighing
/// nothing.
fn only_odds(weighted: &str) -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(&format!(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Rocket"><Stats ai="0" back="0" front="0" human="0"/></Weapon>
               <Weapon type="{weighted}"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#
    ))
    .expect("the fixture table must parse")
}

/// Slot 0 a person and slot 1 an AI on the pad, and slot 2 a dropped person's seat holding
/// `held` (an away seat keeps its pickup and fires nothing), all with `table`.
fn three_on_a_pad(table: oag_tables::weapons::WeaponStats, held: Option<Weapon>) -> Race {
    let mut race = race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 0.0, table);
    let first = race.core.sim.world.ships[0];
    for slot in 1..3 {
        let mut ship = first;
        ship.place_at(oag_gameplay::Pose {
            position: first.physics.body.position + first.physics.body.right() * 60.0 * slot as f32,
            orientation: first.physics.body.orientation,
        });
        race.core.sim.world.ships[slot] = ship;
    }
    race.core.sim.world.ship_count = 3;
    race.core.sim.world.controllers[1] = Controller::Ai;
    race.core.sim.world.controllers[2] = Controller::Away;
    race.core.sim.world.ships[2].pickup.weapon = held;
    race
}

/// HD's exclusions on a roster that enables the Rocket and `weapon`, walking `weapon` first.
fn excluding(weapon: Weapon) -> PickupRoster {
    let enabled: &'static [Weapon] = match weapon {
        Weapon::LeachBeam => &[Weapon::Rocket, Weapon::LeachBeam],
        Weapon::Quake => &[Weapon::Rocket, Weapon::Quake],
        _ => &[Weapon::Rocket, Weapon::Cannon],
    };
    PickupRoster {
        race: enabled,
        eliminator: enabled,
        player_order: match weapon {
            Weapon::LeachBeam => &[Weapon::LeachBeam, Weapon::Rocket],
            Weapon::Quake => &[Weapon::Quake, Weapon::Rocket],
            _ => &[Weapon::Cannon, Weapon::Rocket],
        },
        ai_refuses_repeat: false,
        exclusions: oag_hd::pickups::ROSTER.exclusions,
    }
}

/// Every craft enters the pad again on the next tick, as if it had left it.
fn leave_the_pad(race: &mut Race) {
    race.core.sim.weapon_pad_current.fill(None);
}

fn on_roster(mut race: Race, roster: PickupRoster) -> Race {
    race.core.sim.allowed_weapons = Arc::from(roster.race);
    race.core.sim.pickup_roster = Some(roster);
    race
}

/// How many of `grants` re-granted a slot what it was granted the time before, over
/// `grants` pad crossings on a 1:1 Rocket and Missile table.
fn repeats(ai_refuses_repeat: bool, grants: usize) -> [usize; 2] {
    let table = oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Rocket"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
               <Weapon type="Missile"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse");
    let mut race = three_on_a_pad(table, Some(Weapon::Rocket));
    race.core.sim.pickup_roster = Some(PickupRoster {
        ai_refuses_repeat,
        ..ROSTER
    });
    let mut repeated = [0; 2];
    for _ in 0..grants {
        let before = [0, 1].map(|slot| race.core.sim.world.ships[slot].pickup.last);
        for slot in 0..2 {
            race.core.sim.world.ships[slot].pickup.take();
        }
        leave_the_pad(&mut race);
        race.tick(&PlayerInputs::none());
        for slot in 0..2 {
            let pickup = race.core.sim.world.ships[slot].pickup;
            assert!(
                pickup.weapon.is_some(),
                "slot {slot} crossed the pad with an empty slot"
            );
            if before[slot].is_some() && pickup.last == before[slot] {
                repeated[slot] += 1;
            }
        }
    }
    repeated
}

/// HD's AI draw writes the slot without comparing the previous grant, so on two equal weights
/// about half its grants repeat; a person's never does, nor does an AI's on a title whose
/// roster keeps the refusal (`docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md`, "The AI
/// draw").
#[test]
fn hds_ai_draw_repeats_a_grant_and_a_persons_never_does() {
    // A refused repeat still comes through about once in 512 grants on two equal weights,
    // after `oag_weapons::pickup::REDRAW_ATTEMPTS` re-rolls.
    let refused = 0..=4;
    let [person, ai] = repeats(false, 64);
    assert!(
        refused.contains(&person),
        "a person's draw refuses a repeat: {person} of 64"
    );
    assert!(
        (16..=48).contains(&ai),
        "the AI's draw repeats about half its 64 grants on two equal weights: {ai}"
    );
    let [person, ai] = repeats(true, 64);
    assert!(
        refused.contains(&person) && refused.contains(&ai),
        "a roster keeping the refusal keeps it for the AI: {person} and {ai} of 64"
    );
}

/// A LeachBeam held anywhere in the field leaves both draws, the total and a person's walk
/// alike: a person's roll is not passed on to the Rocket, as it would be for a weapon the mode
/// disables, and with nothing else weighted both draws hand out nothing. Given up, it is drawn
/// again.
#[test]
fn a_leach_beam_held_in_the_field_leaves_both_draws() {
    let mut race = on_roster(
        three_on_a_pad(only_odds("LeachBeam"), Some(Weapon::LeachBeam)),
        excluding(Weapon::LeachBeam),
    );
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.core.sim.world.ships[0].pickup.weapon, None,
        "the person's draw"
    );
    assert_eq!(
        race.core.sim.world.ships[1].pickup.weapon, None,
        "the AI's draw"
    );

    race.core.sim.world.ships[2].pickup.take();
    leave_the_pad(&mut race);
    race.tick(&PlayerInputs::none());
    // The person, first to the pad, takes it out of the AI's draw in turn.
    assert_eq!(
        race.core.sim.world.ships[0].pickup.weapon,
        Some(Weapon::LeachBeam)
    );
    assert_eq!(race.core.sim.world.ships[1].pickup.weapon, None);
}

/// A Quake's wave in flight leaves both draws with nobody holding one. Once it is gone, the
/// first craft granted a Quake takes it out again for the second.
#[test]
fn a_quake_wave_in_flight_leaves_both_draws() {
    let mut race = on_roster(
        three_on_a_pad(only_odds("Quake"), None),
        excluding(Weapon::Quake),
    );
    race.core.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
        2,
        0.0,
        1.0,
        &oag_tables::weapons::QuakeStats::default(),
    ));
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.core.sim.world.ships[0].pickup.weapon, None,
        "the person's draw"
    );
    assert_eq!(
        race.core.sim.world.ships[1].pickup.weapon, None,
        "the AI's draw"
    );

    race.core.sim.world.quake = None;
    leave_the_pad(&mut race);
    race.tick(&PlayerInputs::none());
    let granted: Vec<Option<Weapon>> = (0..2)
        .map(|slot| race.core.sim.world.ships[slot].pickup.weapon)
        .collect();
    assert_eq!(granted, [Some(Weapon::Quake), None]);
}

/// One Cannon held leaves it in the draws and two take it out: of two craft crossing the pad
/// on the same tick beside one holder, exactly one is granted a Cannon.
#[test]
fn the_cannon_leaves_the_draws_at_two_held() {
    let mut race = on_roster(
        three_on_a_pad(only_odds("Cannon"), Some(Weapon::Cannon)),
        excluding(Weapon::Cannon),
    );
    race.tick(&PlayerInputs::none());
    let granted: Vec<Option<Weapon>> = (0..2)
        .map(|slot| race.core.sim.world.ships[slot].pickup.weapon)
        .collect();
    assert_eq!(
        granted
            .iter()
            .filter(|&&weapon| weapon == Some(Weapon::Cannon))
            .count(),
        1,
        "one Cannon held, then two: {granted:?}"
    );
}
