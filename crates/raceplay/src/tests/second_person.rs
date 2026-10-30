//! A second person's fire button spends *their* pickup and credits *their*
//! craft.
//!
//! `Race::spend_pickup` ran for slot 0 alone until `Race::tick` began to step
//! every person-flown slot, and its arms for the Rocket, the Plasma, the
//! Shuriken, the Quake and the Repulser carried a literal `0` as the shot's
//! owner. With slot 1 pressing fire that is a rocket owned by slot 0: it would
//! pass through slot 0's craft and strike slot 1's. Each test below presses
//! fire in slot 1 only, and asserts the shot is slot 1's.

use super::*;
use oag_gameplay::{Controller, PlayerInputs};
use oag_tables::weapons::Weapon;

/// A grid race with slot 1 person-flown, holding `weapon` under `table`, and
/// a few ticks run so the standings have located every craft.
fn second_person_holding(weapon: Weapon, table: oag_tables::weapons::WeaponStats) -> Race {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(table);
    race.sim.world.controllers[1] = Controller::Local;
    assert!(
        race.sim.world.person_flown(1),
        "slot 1 must be person-flown"
    );
    for _ in 0..10 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[1].pickup.weapon = Some(weapon);
    race
}

/// Presses fire in slot 1 for one tick and returns the race after it.
fn press_in_slot_1(race: &mut Race) {
    let mut buttons = Buttons::new();
    buttons.tick(0);
    let mut inputs = PlayerInputs::none();
    inputs.set(1, buttons.tick(SQUARE));
    race.tick(&inputs);
}

#[test]
fn a_second_persons_rocket_volley_is_theirs() {
    let mut race = second_person_holding(Weapon::Rocket, one_rocket_table());
    press_in_slot_1(&mut race);
    let shots: Vec<u8> = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Rocket))
        .map(|p| p.owner)
        .collect();
    assert!(!shots.is_empty(), "slot 1's press fired no rocket");
    assert!(
        shots.iter().all(|&owner| owner == 1),
        "slot 1's rockets are owned by {shots:?}"
    );
}

#[test]
fn a_second_persons_plasma_charge_and_cue_are_theirs() {
    let mut race = second_person_holding(Weapon::Plasma, one_plasma_table());
    press_in_slot_1(&mut race);
    let owner = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .find(|p| p.kind == Some(Weapon::Plasma))
        .map(|p| p.owner);
    assert_eq!(
        owner,
        Some(1),
        "slot 1's plasma charge is owned by {owner:?}"
    );
    let cue = race
        .drain_cues()
        .into_iter()
        .find(|e| e.cue == oag_gameplay::cue::Cue::Plasma)
        .expect("the press raised no Plasma cue");
    assert_eq!(cue.slot, 1, "the Plasma cue sounds from slot {}", cue.slot);
}

#[test]
fn a_second_persons_shuriken_is_theirs() {
    let mut race = second_person_holding(Weapon::Shuriken, one_shuriken_table());
    press_in_slot_1(&mut race);
    let owners: Vec<u8> = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Shuriken))
        .map(|p| p.owner)
        .collect();
    assert!(!owners.is_empty(), "slot 1's press threw nothing");
    assert!(
        owners.iter().all(|&owner| owner == 1),
        "owned by {owners:?}"
    );
}

#[test]
fn a_second_persons_quake_wave_is_theirs() {
    let mut race = second_person_holding(Weapon::Quake, one_quake_table());
    press_in_slot_1(&mut race);
    let wave = race.sim.world.quake.expect("slot 1's press fired no wave");
    assert_eq!(wave.owner, 1, "the wave would hit its own firer");
}

#[test]
fn a_second_persons_repulser_is_theirs() {
    let mut race = second_person_holding(Weapon::Repulser, one_repulser_table());
    press_in_slot_1(&mut race);
    let owners: Vec<u8> = race
        .sim
        .world
        .repulsers
        .iter()
        .flatten()
        .map(|r| r.owner)
        .collect();
    assert_eq!(owners, vec![1], "slot 1's repulser is owned by {owners:?}");
}

/// The Missile's reticle hold belongs to the primary slot's reticle, not to
/// slot 0 as such: with slot 0 an opponent and a person in slot 1, slot 1 is
/// the primary and its candidate needs the hold, while slot 0's does not.
#[test]
fn the_missiles_reticle_hold_follows_the_primary_slot() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_missile_table());
    race.sim.world.controllers[0] = Controller::Ai;
    race.sim.world.controllers[1] = Controller::Local;
    assert_eq!(race.sim.world.primary_slot(), 1);
    let stats = race
        .sim
        .weapons
        .as_ref()
        .and_then(oag_tables::weapons::WeaponStats::missile)
        .expect("the fixture authors a Missile");
    let forward = race.sim.world.ships[1].physics.body.forward();
    // Slot 0 is dead ahead of slot 1, and slot 1 dead ahead of nothing.
    race.sim.world.ships[0].physics.body.position =
        race.sim.world.ships[1].physics.body.position + forward * 60.0;
    let target_of = |race: &Race, owner: u8| {
        race.sim
            .world
            .projectiles
            .slots
            .iter()
            .find(|p| p.kind == Some(Weapon::Missile) && p.owner == owner)
            .map(|p| p.target)
    };
    assert!(race.fire_missile(1, &stats));
    assert_eq!(
        target_of(&race, 1),
        Some(None),
        "the primary's candidate was passed through before its reticle locked"
    );
}

/// The rubber band reads the best-placed person; a lone person is that person
/// wherever the AI stand.
#[test]
fn the_rubber_band_reads_the_best_placed_person() {
    let mut race = race_with_a_grid();
    let mut places = [8u8; oag_gameplay::MAX_SHIPS];
    places[0] = 3;
    places[1] = 1;
    places[2] = 2;
    assert_eq!(
        race.rubber_band_reference(&places),
        0,
        "one person: the AI in first place is nobody to chase"
    );
    race.sim.world.controllers[1] = Controller::Local;
    race.sim.world.controllers[2] = Controller::Local;
    assert_eq!(race.rubber_band_reference(&places), 1);
    places[0] = 1;
    places[1] = 2;
    assert_eq!(race.rubber_band_reference(&places), 0);
    places[1] = 1;
    assert_eq!(
        race.rubber_band_reference(&places),
        0,
        "a tie takes the lowest slot"
    );
}

/// The original field's round robin takes the craft no person flies, slot 0
/// among them when a person flies elsewhere.
#[test]
fn the_original_fields_opponents_are_the_craft_nobody_flies() {
    let mut race = race_with_a_grid();
    let count = race.sim.world.ship_count as usize;
    let (list, n) = race.ai_flown_slots();
    assert_eq!(&list[..n], &(1..count).collect::<Vec<_>>()[..]);
    race.sim.world.controllers[0] = Controller::Ai;
    race.sim.world.controllers[2] = Controller::Local;
    let (list, n) = race.ai_flown_slots();
    let expected: Vec<usize> = (0..count).filter(|&s| s != 2).collect();
    assert_eq!(&list[..n], &expected[..]);
}

/// A person's autopilot does not go shopping for weapon pads, whichever slot
/// they fly; the same craft handed to the AI does.
#[test]
fn no_person_seeks_a_weapon_pad() {
    let mut race = race_with_a_grid();
    race.sim.pad_seeking = PadSeeking::Every;
    race.sim.weapon_pad_line = vec![Some((6, 0.0))];
    race.sim.weapon_pad_refresh_left = vec![0.0];
    assert!(
        race.pad_for(1).is_some(),
        "the AI craft sees the pad: fixture is too weak"
    );
    race.sim.world.controllers[1] = Controller::Local;
    assert!(
        race.pad_for(1).is_none(),
        "a second person was sent for a pad"
    );
}

/// With slot 0 handed to the AI and a person in slot 1, slot 0's driver is
/// seeded and placed on the line like any opponent's rather than left as the
/// seedless default at index zero.
#[test]
fn an_ai_flown_slot_zero_gets_a_driver_of_its_own() {
    let mut race = race_with_a_grid();
    race.sim.world.controllers[0] = Controller::Ai;
    race.sim.world.controllers[1] = Controller::Local;
    assert_eq!(race.sim.world.ships[0].driver, oag_ai::Driver::default());
    race.tick(&PlayerInputs::none());
    let driver = race.sim.world.ships[0].driver;
    assert_ne!(
        driver.seed, 0,
        "slot 0 still drives as the seedless default"
    );
    let line = &race.sim.racing_line;
    let here = race.sim.world.ships[0].physics.body.position;
    assert_eq!(
        driver.index as usize,
        line.nearest(here, 0, line.len()),
        "slot 0's driver was not placed on the line"
    );
}

/// The weapon AI's "the player is ahead" reads any person, not the primary
/// alone: with persons in slots 0 and 1, slot 1 ahead of an AI craft in slot 2
/// and slot 0 behind it, the gap handed to the fire law is slot 1's.
#[test]
fn the_weapon_ai_sees_a_second_person_ahead() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_rocket_table());
    race.sim.weapon_ai = Some(
        oag_tables::weapons::ai::parse(
            r#"<WeaponAIStats>
                 <Rockets useAgainstPlayer="1.1" useAgainstAI="1.2" absorb="1.0"/>
               </WeaponAIStats>"#,
        )
        .expect("the fixture table must parse"),
    );
    race.sim.world.controllers[1] = Controller::Local;
    let progress = |race: &mut Race, slot: usize, at: f32| {
        race.sim.world.ships[slot].standing.progress = Some(at);
    };
    progress(&mut race, 0, 10.0);
    progress(&mut race, 2, 40.0);
    progress(&mut race, 1, 70.0);
    let situation = race
        .fire_situation(2, Weapon::Rocket, true)
        .expect("a rocket with odds and a course has a situation");
    let gap = situation.player_gap.expect("two people are on the grid");
    assert!(
        gap > 0.0,
        "the person ahead was not the one handed over: {gap}"
    );
}

/// A weapon pad hands a second person the human column of the odds table, by
/// their own place, and not the AI's. Turbo is weighted for people alone and
/// Shield for the AI alone, so which one slot 1 is handed says which column
/// was read; slot 0's Turbo is the control.
#[test]
fn a_second_person_draws_from_the_human_column_of_a_weapon_pad() {
    let table = oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Turbo"><Stats absorb="23" time="0.75"/></Weapon>
             <Weapon type="Shield"><Stats absorb="24" time="0.85"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Turbo"><Stats ai="0" back="1" front="1" human="1"/></Weapon>
               <Weapon type="Shield"><Stats ai="1" back="0" front="0" human="0"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse");
    let mut race = race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 0.0, table);
    let mut second = race.sim.world.ships[0];
    second.place_at(oag_gameplay::Pose {
        position: second.physics.body.position + second.physics.body.right() * 60.0,
        orientation: second.physics.body.orientation,
    });
    race.sim.world.ships[1] = second;
    race.sim.world.ships[1].pickup.weapon = None;
    race.sim.world.ship_count = 2;
    race.sim.world.controllers[1] = Controller::Local;
    for _ in 0..3 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.sim.world.ships[0].pickup.weapon,
        Some(Weapon::Turbo),
        "the control: the first person draws from the human column"
    );
    assert_eq!(
        race.sim.world.ships[1].pickup.weapon,
        Some(Weapon::Turbo),
        "the second person was dealt from the AI column"
    );
}

/// A Pulse Plasma bolt that sweeps into a craft detonates twice in one tick,
/// at the bolt and at the struck craft (`Plasma_SweepCraftHit` and
/// `Plasmas_Update`'s teardown, `plasma.md`); one that does not strike a craft
/// detonates once; HD's single explosion is not doubled.
#[test]
fn a_plasma_craft_hit_spawns_two_blasts_at_the_bolt_and_the_craft() {
    let live = |race: &Race| race.view.plasma_blasts.iter().flatten().count();
    let aimed_at_slot_1 = |hd: bool| {
        let mut race = second_person_holding(Weapon::Plasma, one_plasma_table());
        race.view.hd_plasma_blast = hd;
        let craft = race.sim.world.ships[1].physics.body.position;
        // Slot 1 is not the firer, and the bolt starts 3 units short of it.
        let speed = Vec3::Z * 60.0;
        race.sim
            .world
            .projectiles
            .spawn(Weapon::Plasma, craft - Vec3::Z * 3.0, speed, 0);
        race
    };

    let mut race = aimed_at_slot_1(false);
    let before = live(&race);
    let craft = race.sim.world.ships[1].physics.body.position;
    race.tick(&PlayerInputs::none());
    let at: Vec<Vec3> = race
        .view
        .plasma_blasts
        .iter()
        .flatten()
        .map(|blast| blast.position)
        .collect();
    assert_eq!(at.len() - before, 2, "a craft hit is two blasts: {at:?}");
    assert!(
        at.iter().any(|p| (*p - craft).length() < 1e-3),
        "one blast is at the struck craft {craft:?}: {at:?}"
    );

    let mut hd = aimed_at_slot_1(true);
    hd.tick(&PlayerInputs::none());
    assert_eq!(live(&hd), 1, "HD's one explosion is not doubled");

    let race = second_person_holding(Weapon::Plasma, one_plasma_table());
    assert_eq!(
        race.plasma_craft_hit_blast_point(Weapon::Plasma, None),
        None,
        "a wall hit has no second blast"
    );
}
