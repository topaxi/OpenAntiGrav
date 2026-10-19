//! A network client seated in slot 1 hears, sees and is announced to for its
//! own craft: wall sparks and shake, the contact sound, the countdown voice
//! and the reticle all read the seat, not slot 0 nor `primary_slot`.
//!
//! Both slots are person-flown, as on a client; only `set_seat` is the view's.

use super::*;
use oag_gameplay::cue::Cue;
use oag_gameplay::{Controller, PlayerInputs};
use oag_tables::weapons::Weapon;

fn two_people_against_a_wall(seat: Option<usize>) -> Race {
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
    race.sim.world.controllers[1] = Controller::Local;
    assert!(
        race.sim.world.person_flown(1),
        "slot 1 must be person-flown"
    );
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
fn a_seat_one_client_gets_its_own_wall_sparks_shake_and_sound() {
    let mut race = two_people_against_a_wall(Some(1));
    push_toward_wall(&mut race, 1);
    let evaluations = race.tick(&PlayerInputs::none());
    assert!(
        evaluations.get(1).is_some_and(|e| e.wall.impact),
        "the fixture never put slot 1 on the wall"
    );
    assert_eq!(
        race.spark_ignitions(),
        1,
        "seat 1's own contact threw no sparks"
    );
    assert!(
        race.view.shake.active(),
        "seat 1's own contact shook nothing"
    );
    assert_eq!(collisions(&race), vec![1]);
}

#[test]
fn a_seat_one_client_is_not_shaken_by_slot_zeros_wall() {
    let mut race = two_people_against_a_wall(Some(1));
    push_toward_wall(&mut race, 0);
    let evaluations = race.tick(&PlayerInputs::none());
    assert!(evaluations.get(0).is_some_and(|e| e.wall.impact));
    assert_eq!(
        race.spark_ignitions(),
        0,
        "slot 0's wall threw seat 1's sparks"
    );
    assert!(!race.view.shake.active(), "slot 0's wall shook seat 1");
}

#[test]
fn a_seat_zero_client_is_not_shaken_by_slot_ones_wall() {
    let mut race = two_people_against_a_wall(None);
    push_toward_wall(&mut race, 1);
    race.tick(&PlayerInputs::none());
    assert_eq!(race.spark_ignitions(), 0);
    assert!(!race.view.shake.active());
}

/// The cooldown is per slot and ticks for every person flown, so which seat a
/// machine holds changes nothing about when a second cue may sound.
#[test]
fn the_contact_cue_cadence_does_not_depend_on_the_seat() {
    let cadence = |seat| {
        let mut race = two_people_against_a_wall(seat);
        let mut heard = Vec::new();
        for _ in 0..90 {
            push_toward_wall(&mut race, 1);
            race.tick(&PlayerInputs::none());
            heard.push(collisions(&race).len());
            race.drain_cues();
        }
        (heard, race.sim.contact_cue_cooldown)
    };
    let (heard, cooldowns) = cadence(Some(1));
    assert!(
        heard.iter().sum::<usize>() >= 2,
        "the scrape never re-sounded"
    );
    assert_eq!(cadence(None), (heard, cooldowns));
}

#[test]
fn the_start_voice_is_the_seats() {
    let mut race = two_people_against_a_wall(Some(1));
    race.sim.countdown_voice = true;
    race.raise_start_cue(Cue::Ready);
    assert_eq!(race.pending_cues()[0].slot, 1);
}

#[test]
fn a_seat_one_reticle_is_drawn_for_seat_ones_weapon() {
    let mut race = two_people_against_a_wall(Some(1));
    race.sim.weapons = Some(one_leach_beam_table());
    race.sim.world.ships[1].pickup.weapon = Some(Weapon::LeachBeam);
    race.sim.world.ships[0].pickup.weapon = None;
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.sight().held(),
        oag_race::sight::Held::LeachBeam,
        "the seat's reticle ignored its own Leach Beam"
    );
}

#[test]
fn the_boost_kick_and_the_flaps_follow_the_seats_craft() {
    let mut race = two_people_against_a_wall(Some(1));
    race.view.boost_fov_kick = oag_display::display::BoostFovKick::DEFAULT;
    race.sim.world.ships[1].physics.pad_timer = 1.0;
    race.sim.world.ships[1].physics.airbrake_left = 100.0;
    race.tick(&PlayerInputs::none());
    assert!(
        race.view.boost_kick > 0.0,
        "slot 1's boost did not open the kick"
    );
    assert!(
        race.airbrake_flaps()[0] > 0.0,
        "slot 1's airbrake did not open the flap"
    );
}
