//! A network client seated in any slot N hears, sees and is announced to for its
//! own craft: wall sparks and shake, the contact sound, the countdown voice
//! and the reticle all read the seat, not slot 0 nor `primary_slot`.
//!
//! Four people are person-flown, as on a four-seat client; only `set_seat` is the view's. Every
//! test runs seats 1, 2 and 3 (slot 0 is the case the code used to assume).

use super::*;

/// The seats a client may hold besides slot 0.
const SEATS: [usize; 3] = [1, 2, 3];
use oag_gameplay::cue::Cue;
use oag_gameplay::{Controller, PlayerInputs};
use oag_tables::weapons::Weapon;

fn four_people_against_a_wall(seat: Option<usize>) -> Race {
    let mut setup = setup_with(
        hulled_handling(),
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    for slot in 1..4 {
        race.sim.world.controllers[slot] = Controller::Local;
        assert!(
            race.sim.world.person_flown(slot),
            "slot {slot} person-flown"
        );
    }
    race.set_seat(seat);
    race
}

/// Puts `slot`'s craft on an inbound wall approach, each person in their own
/// lane so the pair never touch.
fn push_toward_wall(race: &mut Race, slot: usize) {
    let body = &mut race.sim.world.ships[slot].physics.body;
    body.position = Vec3::new(20.0, -39.7, 10.0 * slot as f32);
    body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
    body.orientation = oag_core::math::Quat::IDENTITY;
    body.angular_velocity = Vec3::ZERO;
}

fn collisions(race: &Race) -> Vec<usize> {
    race.pending_cues()
        .iter()
        .filter(|cue| cue.cue == Cue::Collision)
        .map(|cue| usize::from(cue.slot))
        .collect()
}

#[test]
fn a_seated_client_gets_its_own_wall_sparks_shake_and_sound() {
    for seat in SEATS {
        let mut race = four_people_against_a_wall(Some(seat));
        push_toward_wall(&mut race, seat);
        let evaluations = race.tick(&PlayerInputs::none());
        assert!(
            evaluations.get(seat).is_some_and(|e| e.wall.impact),
            "the fixture never put slot {seat} on the wall"
        );
        assert_eq!(race.spark_ignitions(), 1, "seat {seat}: no sparks");
        assert!(race.view.contact.shake.active(), "seat {seat}: no shake");
        assert_eq!(collisions(&race), vec![seat]);
    }
}

#[test]
fn a_seated_client_is_not_shaken_by_another_slots_wall() {
    for seat in SEATS {
        for other in (0..4).filter(|&o| o != seat) {
            let mut race = four_people_against_a_wall(Some(seat));
            push_toward_wall(&mut race, other);
            let evaluations = race.tick(&PlayerInputs::none());
            assert!(evaluations.get(other).is_some_and(|e| e.wall.impact));
            assert_eq!(
                race.spark_ignitions(),
                0,
                "slot {other}'s wall threw seat {seat}'s sparks"
            );
            assert!(
                !race.view.contact.shake.active(),
                "slot {other}'s wall shook seat {seat}"
            );
        }
    }
}

#[test]
fn a_seat_zero_client_is_not_shaken_by_another_slots_wall() {
    for other in SEATS {
        let mut race = four_people_against_a_wall(None);
        push_toward_wall(&mut race, other);
        race.tick(&PlayerInputs::none());
        assert_eq!(race.spark_ignitions(), 0);
        assert!(!race.view.contact.shake.active());
    }
}

/// The cooldown is per slot and ticks for every person flown, so which seat a
/// machine holds changes nothing about when a second cue may sound.
#[test]
fn the_contact_cue_cadence_does_not_depend_on_the_seat() {
    let cadence = |seat| {
        let mut race = four_people_against_a_wall(seat);
        let mut heard = Vec::new();
        for _ in 0..90 {
            push_toward_wall(&mut race, 3);
            race.tick(&PlayerInputs::none());
            heard.push(collisions(&race).len());
            race.drain_cues();
        }
        (heard, race.sim.contact_cue_cooldown)
    };
    let (heard, cooldowns) = cadence(Some(3));
    assert!(
        heard.iter().sum::<usize>() >= 2,
        "the scrape never re-sounded"
    );
    for seat in [None, Some(1), Some(2)] {
        assert_eq!(cadence(seat), (heard.clone(), cooldowns));
    }
}

#[test]
fn the_start_voice_is_the_seats() {
    for seat in SEATS {
        let mut race = four_people_against_a_wall(Some(seat));
        race.sim.countdown_voice = true;
        race.raise_start_cue(Cue::Ready);
        assert_eq!(usize::from(race.pending_cues()[0].slot), seat);
    }
}

#[test]
fn a_seated_reticle_is_drawn_for_the_seats_weapon() {
    for seat in SEATS {
        let mut race = four_people_against_a_wall(Some(seat));
        race.sim.weapons = Some(one_leach_beam_table());
        for slot in 0..4 {
            race.sim.world.ships[slot].pickup.weapon = None;
        }
        race.sim.world.ships[seat].pickup.weapon = Some(Weapon::LeachBeam);
        race.tick(&PlayerInputs::none());
        assert_eq!(
            race.sight().held(),
            oag_race::sight::Held::LeachBeam,
            "seat {seat}: the reticle ignored its own Leach Beam"
        );
    }
}

#[test]
fn the_boost_kick_and_the_flaps_follow_the_seats_craft() {
    for seat in SEATS {
        let mut race = four_people_against_a_wall(Some(seat));
        race.view.boost_fov_kick = oag_display::display::BoostFovKick::DEFAULT;
        race.sim.world.ships[seat].physics.pad_timer = 1.0;
        race.sim.world.ships[seat].physics.airbrake_left = 100.0;
        race.tick(&PlayerInputs::none());
        assert!(race.view.boost_kick > 0.0, "seat {seat}: no boost kick");
        assert!(race.airbrake_flaps()[0] > 0.0, "seat {seat}: no flap");
    }
}
