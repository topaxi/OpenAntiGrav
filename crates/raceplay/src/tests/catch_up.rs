//! A fire the client first learns of inside a correction's replay is shown late, once.
//!
//! The pair is a client and a server over the same two-person race with slot 1 a remote
//! person holding a Repulser. The server's tick sequence carries slot 1's fire; the client
//! predicted none (a remote person's last held input has no button edge). Reconciling the
//! client replays the server's inputs, and the replay is where the client first meets the fire.

use super::*;
use oag_gameplay::cue::Cue;
use oag_gameplay::{Controller, PlayerInputs};

/// Ticks both sides agree on before the window under test.
const AGREED: usize = 20;
/// Ticks of the window the correction replays.
const WINDOW: usize = 12;
/// The tick in the window slot 1 presses fire.
const FIRE_AT: usize = 3;

fn armed_pair() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_repulser_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.controllers[1] = Controller::Remote;
    let mut second = race.sim.world.ships[0];
    second.place_at(oag_gameplay::Pose {
        position: second.physics.body.position + second.physics.body.right() * 60.0,
        orientation: second.physics.body.orientation,
    });
    race.sim.world.ships[1] = second;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].pickup.weapon = Some(oag_tables::weapons::Weapon::Repulser);
    for ship in &mut race.sim.world.ships[..2] {
        ship.physics.craft_state = oag_physics::CraftState::Racing;
    }
    race
}

/// Slot 1's input for window tick `at`: a fire press on the one tick when `fires`.
fn window_input(at: usize, fires: bool) -> PlayerInputs {
    let mut buttons = Buttons::new();
    let mut snapshot = buttons.tick(0);
    if fires && at == FIRE_AT {
        snapshot = buttons.tick(SQUARE);
    }
    let mut inputs = PlayerInputs::none();
    inputs.set(1, snapshot);
    inputs
}

/// The server's state at the start of the window, and the client at the same point. Boxed:
/// a `Race` is large enough that two of them on a debug test thread's stack overflow it.
fn agreed() -> (Box<Race>, Box<Race>) {
    let (mut server, mut client) = (Box::new(armed_pair()), Box::new(armed_pair()));
    for _ in 0..AGREED {
        server.tick_sim_only(&PlayerInputs::none());
        client.tick(&PlayerInputs::none());
        client.drain_cues();
    }
    (server, client)
}

/// Runs the client's prediction of the window, `fires` saying whether it guessed the fire, and
/// returns the server's snapshot at the window's start with the inputs the truth carried.
fn predict(
    server: &mut Race,
    client: &mut Race,
    guessed_fire: bool,
) -> (RaceSim, Vec<PlayerInputs>) {
    let authoritative = server.snapshot_sim();
    for at in 0..WINDOW {
        client.tick(&window_input(at, guessed_fire));
        client.drain_cues();
    }
    let truth: Vec<PlayerInputs> = (0..WINDOW).map(|at| window_input(at, true)).collect();
    for inputs in &truth {
        server.tick_sim_only(inputs);
    }
    (authoritative, truth)
}

#[test]
fn a_fire_first_seen_in_the_replay_is_started_late() {
    let (mut server, mut client) = agreed();
    let (authoritative, truth) = predict(&mut server, &mut client, false);
    assert!(
        client.sim.world.repulsers.iter().flatten().count() == 0,
        "the client predicted a fire it had no input for"
    );
    client.reconcile(&authoritative, truth);
    assert_eq!(client.sim.state_hash(), server.sim.state_hash());
    let late = client.take_late_report();
    assert_eq!(late.cues, 1, "{late:?}");
    assert!(
        late.oldest_age >= (WINDOW - FIRE_AT - 2) as u32 && late.oldest_age <= WINDOW as u32,
        "the fire is as old as the ticks since it: {late:?}"
    );
    let raised: Vec<Cue> = client.drain_cues().into_iter().map(|e| e.cue).collect();
    assert_eq!(
        raised,
        vec![Cue::Repulsor],
        "the fire's sound reaches the drain"
    );
}

#[test]
fn a_fire_the_client_already_presented_is_not_presented_again() {
    let (mut server, mut client) = agreed();
    let (authoritative, truth) = predict(&mut server, &mut client, true);
    client.reconcile(&authoritative, truth);
    let late = client.take_late_report();
    assert_eq!(late.cues, 0, "{late:?}");
    assert!(late.already_presented >= 1, "{late:?}");
    assert!(client.drain_cues().is_empty());
}

#[test]
fn a_second_correction_over_the_same_window_adds_nothing() {
    let (mut server, mut client) = agreed();
    let (authoritative, truth) = predict(&mut server, &mut client, false);
    client.reconcile(&authoritative, truth.clone());
    client.drain_cues();
    let first = client.take_late_report();
    assert_eq!(first.cues, 1);
    client.reconcile(&authoritative, truth);
    let again = client.take_late_report();
    assert_eq!(again.cues, 0, "the same fire started twice: {again:?}");
    assert!(client.drain_cues().is_empty());
}

#[test]
fn a_fire_too_old_to_sound_is_dropped() {
    let (mut server, mut client) = agreed();
    let authoritative = server.snapshot_sim();
    let long = 80;
    let truth: Vec<PlayerInputs> = (0..long).map(|at| window_input(at, true)).collect();
    for inputs in &truth {
        server.tick_sim_only(inputs);
    }
    client.reconcile(&authoritative, truth);
    assert_eq!(client.take_late_report().cues, 0);
}

/// A blast a replay raises and the client never played is started at its age; one it played is
/// not. Journalled directly: a blast needs a projectile landing, which is
/// `oag_weapons`' to test.
#[test]
fn a_blast_first_seen_in_the_replay_is_started_at_its_age_and_a_played_one_is_not() {
    // Called between ticks, so `quiet` (set for the length of a replayed tick) is set by hand.
    let blast = |race: &mut Race, quiet: bool| {
        race.quiet = quiet;
        race.note_blast(
            oag_tables::weapons::Weapon::Rocket,
            Vec3::ZERO,
            None,
            Quat::IDENTITY,
        );
        race.quiet = false;
    };
    let mut race = Box::new(armed_pair());
    let start = u32::try_from(race.sim.world.tick).unwrap();
    race.tick_sim_only(&PlayerInputs::none());
    blast(&mut race, true);
    for _ in 0..5 {
        race.tick_sim_only(&PlayerInputs::none());
    }
    race.catch_up(start);
    let late = race.take_late_report();
    assert_eq!((late.blasts, late.oldest_age), (1, 5), "{late:?}");

    // Presented on a drawn tick, then replayed: matched, not started again.
    let mut race = Box::new(armed_pair());
    let start = u32::try_from(race.sim.world.tick).unwrap();
    race.tick(&PlayerInputs::none());
    blast(&mut race, false);
    race.tick_sim_only(&PlayerInputs::none());
    blast(&mut race, true);
    race.catch_up(start);
    let late = race.take_late_report();
    assert_eq!((late.blasts, late.already_presented), (0, 1), "{late:?}");
}
