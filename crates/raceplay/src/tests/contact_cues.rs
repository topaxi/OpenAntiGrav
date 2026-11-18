//! HD's collision and grind cues: what a contact has to be before it is heard, and what the
//! cue is handed (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`).
//!
//! The law is a pure decision over one craft's kept contact; the race half checks that the
//! craft pair feeds it.

use super::*;
use crate::cues::{ContactImpulse, decide_contact_cues};
use oag_gameplay::cue::{ContactCue, ContactSurface, Cue};
use oag_title::contact_cue::ContactCues;

fn law() -> oag_title::contact_cue::ImpulseLaw {
    oag_hd::HD_IMPULSE_LAW
}

/// What one tick of a kept contact is heard as.
fn heard(state: &mut ContactImpulse, person: bool, shield: bool) -> [Option<(Cue, ContactCue)>; 2] {
    decide_contact_cues(state, &law(), person, shield)
}

/// A fresh craft that met `impulse` with `dot` between its forward axis and the normal.
fn met(impulse: f32, dot: f32, craft: bool) -> ContactImpulse {
    ContactImpulse {
        impulse,
        craft,
        dot,
        ..ContactImpulse::default()
    }
}

#[test]
fn a_grind_steps_to_medium_over_4_and_large_over_8() {
    for (impulse, severity) in [(3.9755, 0), (4.0588, 1), (7.0022, 1), (8.7752, 2)] {
        let [_, grind] = heard(&mut met(impulse, 0.0, false), true, false);
        let (cue, contact) = grind.expect("a person grinds");
        assert_eq!(cue, Cue::Grind);
        assert_eq!(contact.severity, severity, "impulse {impulse}");
    }
}

#[test]
fn a_hit_steps_to_medium_over_25_and_large_over_50() {
    for (impulse, severity) in [(21.36, 0), (30.51, 1), (49.33, 1), (52.15, 2), (139.9, 2)] {
        let [hit, _] = heard(&mut met(impulse, 0.0, false), true, false);
        let (cue, contact) = hit.expect("over the floor");
        assert_eq!(cue, Cue::Collision);
        assert_eq!(contact.severity, severity, "impulse {impulse}");
    }
}

#[test]
fn the_request_volume_is_the_impulse_law_in_the_original_s_integer_units() {
    let [hit, _] = heard(&mut met(21.362_507, 0.0, false), true, false);
    assert_eq!((hit.unwrap().1.volume * 1024.0) as i32, 986);
    let [_, grind] = heard(&mut met(4.168_955, 0.0, false), true, false);
    assert_eq!((grind.unwrap().1.volume * 1024.0) as i32, 789);
    let [hit, _] = heard(&mut met(32.716_37, 0.0, false), true, false);
    assert_eq!(hit.unwrap().1.volume, 1.0);
}

#[test]
fn a_hit_needs_over_18_unless_it_is_nose_on_and_over_3() {
    let hit = |impulse, dot| heard(&mut met(impulse, dot, false), false, false)[0].is_some();
    assert!(!hit(17.9, 0.0));
    assert!(hit(18.1, 0.0));
    assert!(hit(3.5, 0.9));
    assert!(!hit(2.9, 0.9));
}

#[test]
fn only_a_person_grinds_and_never_nose_on() {
    let grinds = |dot, person| heard(&mut met(0.6, dot, false), person, false)[1].is_some();
    assert!(grinds(0.0, true));
    assert!(!grinds(0.0, false), "an opponent never grinds");
    assert!(
        !grinds(0.9, true),
        "a nose-on contact is a hit, not a scrape"
    );
    assert!(!heard(&mut met(0.4, 0.0, false), true, false)[1].is_some());
}

#[test]
fn a_craft_pair_is_heard_on_the_craft_surface_and_a_wall_on_the_track() {
    let surface = |craft| {
        heard(&mut met(30.0, 0.0, craft), true, false)[0]
            .unwrap()
            .1
            .surface
    };
    assert_eq!(surface(true), ContactSurface::Craft);
    assert_eq!(surface(false), ContactSurface::Track);
}

#[test]
fn the_waits_hold_a_hit_for_six_ticks_and_a_grind_for_one() {
    let mut state = ContactImpulse::default();
    let (mut hits, mut grinds) = (Vec::new(), Vec::new());
    for tick in 0..14 {
        state.impulse = 30.0;
        let [hit, grind] = heard(&mut state, true, false);
        if hit.is_some() {
            hits.push(tick);
        }
        if grind.is_some() {
            grinds.push(tick);
        }
    }
    assert_eq!(hits, [0, 7]);
    assert_eq!(grinds, [0, 2, 4, 6, 8, 10, 12]);
}

#[test]
fn a_waiting_craft_counts_down_and_ignores_its_impulse() {
    let mut state = met(0.0, 0.0, false);
    state.hit_wait = 2;
    state.impulse = 200.0;
    assert!(heard(&mut state, false, false)[0].is_none());
    assert_eq!(state.hit_wait, 1);
}

#[test]
fn the_shield_swaps_a_person_s_two_cues_and_nobody_else_s() {
    let [hit, grind] = heard(&mut met(30.0, 0.0, false), true, true);
    assert_eq!(hit.unwrap().0, Cue::CollisionShield);
    assert_eq!(grind.unwrap().0, Cue::GrindShield);
    let [hit, grind] = heard(&mut met(30.0, 0.0, false), false, true);
    assert_eq!(hit.unwrap().0, Cue::Collision);
    assert!(grind.is_none());
}

#[test]
fn deciding_clears_the_tick_s_contact() {
    let mut state = met(30.0, 0.5, true);
    let _ = heard(&mut state, true, false);
    assert_eq!((state.impulse, state.craft, state.dot), (0.0, false, 0.0));
}

/// Two craft a unit apart nose to nose, closing at `speed` each, slot 0 flown by a person.
fn closing(speed: f32, contact_cues: ContactCues) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.contact_cues = contact_cues;
    let mut race = Race::start(setup);
    race.core.sim.world.ship_count = 2;
    race.core.sim.world.ships[1].active = true;
    race.core.sim.world.ships[1].handling = hulled_handling();
    let at = race.core.sim.world.ships[0].physics.body.position;
    race.core.sim.world.ships[1].physics.body.position = at + Vec3::new(0.0, 0.0, 1.0);
    race.core.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(0.0, 0.0, speed);
    race.core.sim.world.ships[1].physics.body.linear_velocity = Vec3::new(0.0, 0.0, -speed);
    race
}

fn hd() -> ContactCues {
    ContactCues::Impulse(oag_title::pre_race::Sourced::measured(law()))
}

/// HD's law with the grind switched on, to keep the grind's wiring tested while it is held back.
fn hd_with_grind() -> ContactCues {
    ContactCues::Impulse(oag_title::pre_race::Sourced::measured(
        oag_title::contact_cue::ImpulseLaw {
            grind_played: true,
            ..law()
        },
    ))
}

fn raised(race: &Race, slot: u8) -> Vec<oag_gameplay::cue::CueEvent> {
    race.core
        .sim
        .pending_cues()
        .iter()
        .filter(|event| event.slot == slot)
        .copied()
        .collect()
}

#[test]
fn a_hard_craft_bump_is_a_craft_surface_hit_on_both_hulls() {
    let mut race = closing(20.0, hd());
    race.resolve_craft_pairs();
    race.core.sim.raise_impulse_cues();
    let impulse = race.pair_impulses[0];
    assert!(impulse > 18.0, "the fixture bump is too gentle: {impulse}");
    let severity = if impulse > 50.0 {
        2
    } else {
        u8::from(impulse > 25.0)
    };
    let volume = (impulse * law().hit_volume_per_impulse + law().hit_volume_base).min(1.0);
    for slot in [0, 1] {
        let hits: Vec<_> = raised(&race, slot)
            .into_iter()
            .filter(|event| event.cue == Cue::Collision)
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "slot {slot}: {:?}",
            race.core.sim.pending_cues()
        );
        assert_eq!(
            hits[0].contact,
            Some(ContactCue {
                severity,
                surface: ContactSurface::Craft,
                volume,
            }),
            "slot {slot}, impulse {impulse}"
        );
    }
}

/// A gentle bump (`impulse` over the grind floor, under the hit's) on a two-craft fixture.
fn gentle_bump(contact_cues: ContactCues) -> Race {
    let mut race = closing(0.5, contact_cues);
    race.resolve_craft_pairs();
    race.core.sim.raise_impulse_cues();
    let impulse = race.pair_impulses[0];
    assert!(
        impulse > 0.5 && impulse < 18.0,
        "the fixture bump is not gentle: {impulse}"
    );
    race
}

#[test]
fn a_gentle_craft_bump_is_a_grind_for_the_person_and_silent_for_the_opponent() {
    let race = gentle_bump(hd_with_grind());
    let person: Vec<_> = raised(&race, 0).iter().map(|event| event.cue).collect();
    assert_eq!(person, [Cue::Grind]);
    assert!(
        raised(&race, 1).is_empty(),
        "{:?}",
        race.core.sim.pending_cues()
    );
}

#[test]
fn hd_s_grind_is_held_back_and_its_hit_is_not() {
    assert!(!law().grind_played, "HD's grind is held back");
    let race = gentle_bump(hd());
    assert!(
        race.core.sim.pending_cues().is_empty(),
        "{:?}",
        race.core.sim.pending_cues()
    );

    let mut race = closing(20.0, hd());
    race.resolve_craft_pairs();
    race.core.sim.raise_impulse_cues();
    let cues: Vec<_> = raised(&race, 0).iter().map(|event| event.cue).collect();
    assert_eq!(
        cues,
        [Cue::Collision],
        "a hard bump still hits, and never grinds"
    );
}

#[test]
fn the_re_armed_law_raises_no_impulse_cue_for_a_craft_bump() {
    let mut race = closing(20.0, ContactCues::Rearmed);
    race.resolve_craft_pairs();
    race.core.sim.raise_impulse_cues();
    assert!(race.pair_impulses[0] > 18.0);
    assert!(
        race.core.sim.pending_cues().is_empty(),
        "{:?}",
        race.core.sim.pending_cues()
    );
    assert!(
        race.core
            .sim
            .contact_impulse
            .iter()
            .all(|state| *state == ContactImpulse::default())
    );
}
