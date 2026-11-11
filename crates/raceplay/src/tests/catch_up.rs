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
    race.tick_sim_only(&PlayerInputs::none());
    blast(&mut race, true);
    for _ in 0..5 {
        race.tick_sim_only(&PlayerInputs::none());
    }
    race.settle_presentation();
    let late = race.take_late_report();
    assert_eq!((late.blasts, late.oldest_age), (1, 5), "{late:?}");

    // Presented on a drawn tick, then replayed: matched, not started again.
    let mut race = Box::new(armed_pair());
    race.tick(&PlayerInputs::none());
    blast(&mut race, false);
    race.tick_sim_only(&PlayerInputs::none());
    blast(&mut race, true);
    race.settle_presentation();
    let late = race.take_late_report();
    assert_eq!((late.blasts, late.already_presented), (0, 1), "{late:?}");
}

/// A blast first seen late starts its screen flash where a flash started on time is by now,
/// not at its first key.
#[test]
fn a_blast_first_seen_late_starts_its_screen_flash_aged() {
    const AGE: usize = 9;
    let dt = 1.0 / 60.0;
    let hit = |race: &mut Race, quiet: bool| {
        race.quiet = quiet;
        // A Rocket that reached a craft: screen flash kind 0.
        race.note_blast(
            oag_tables::weapons::Weapon::Rocket,
            Vec3::ZERO,
            Some(1),
            Quat::IDENTITY,
        );
        race.quiet = false;
    };
    let frames = |race: &mut Race| -> Vec<Option<[f32; 4]>> {
        let flash = race.view.screen_flash.as_mut().unwrap();
        (0..6)
            .map(|_| {
                flash.advance(dt, Vec3::ZERO);
                flash.colour()
            })
            .collect()
    };

    // Seen on time, then `AGE` drawn frames on.
    let mut on_time = Box::new(armed_pair());
    on_time.view.screen_flash = Some(oag_fx::flash::ScreenFlash::default());
    on_time.tick(&PlayerInputs::none());
    hit(&mut on_time, false);
    let flash = on_time.view.screen_flash.as_mut().unwrap();
    for _ in 0..AGE {
        flash.advance(dt, Vec3::ZERO);
    }
    let fresh_first = {
        let mut fresh = oag_fx::flash::ScreenFlash::default();
        fresh.start(oag_fx::flash::BLAST, Vec3::ZERO);
        fresh.advance(dt, Vec3::ZERO);
        fresh.colour()
    };

    // First met in a quiet tick, settled `AGE` ticks later.
    let mut late = Box::new(armed_pair());
    late.view.screen_flash = Some(oag_fx::flash::ScreenFlash::default());
    late.tick_sim_only(&PlayerInputs::none());
    hit(&mut late, true);
    for _ in 0..AGE {
        late.tick_sim_only(&PlayerInputs::none());
    }
    late.settle_presentation();
    assert_eq!(late.take_late_report().blasts, 1);

    let (late, on_time) = (frames(&mut late), frames(&mut on_time));
    assert!(late[0].is_some(), "the late flash never drew");
    assert_eq!(late, on_time, "the late flash is not where one on time is");
    assert_ne!(late[0], fresh_first, "the late flash started fresh");
}

/// A Rocket first drawn mid-flight is given its flight effect as old as the Rocket is: what
/// the effect put out since the launch is there, as on a client that drew every tick, where a
/// fresh attach would begin with the launch flash at a rocket long past its launch.
#[test]
fn a_rocket_first_drawn_mid_flight_gets_its_effect_as_old_as_it_is() {
    const AGE: u32 = 12;
    let dt = 1.0 / 60.0;
    let velocity = Vec3::new(0.0, 0.0, 300.0);
    let launch = Vec3::new(0.0, 50.0, 0.0);
    let flying = |ticks: u32| -> Box<Race> {
        let mut race = Box::new(armed_pair());
        let options = Options {
            own_rockets: true,
            ..Options::default()
        };
        let mut read = |name: &str| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/fallback")
                .join(name.replace('\\', "/"));
            std::fs::read(path).ok()
        };
        let mut library = oag_fx::psys::Library::new();
        let loaded =
            crate::own_rockets::load_with(&options, oag_pulse::TITLE, &mut read, &mut library);
        let mut report = Vec::new();
        race.view.handles = loaded.lend(std::mem::take(&mut race.view.handles), &mut report);
        assert!(
            race.view.handles.get(Trigger::RocketFlare).is_some(),
            "{report:?}"
        );
        assert!(race.sim.world.projectiles.spawn_riding(
            oag_tables::weapons::Weapon::Rocket,
            launch,
            velocity,
            1,
            Vec3::Y,
            0.0,
        ));
        fly(&mut race, ticks);
        race
    };
    /// Moves the one Rocket `ticks` ticks on, undrawn.
    fn fly(race: &mut Race, ticks: u32) {
        let rocket = &mut race.sim.world.projectiles.slots[0];
        rocket.position += rocket.velocity * (ticks as f32 / 60.0);
        rocket.lifetime -= ticks as f32 / 60.0;
    }
    let alive = |race: &Race| {
        let effect = race.view.handles.get(Trigger::RocketFlare).unwrap();
        race.view.stage.alive_of_effect(effect)
    };

    // Drawn every tick of the flight.
    let mut ridden = flying(1);
    ridden.advance_projectile_flares();
    for _ in 1..AGE {
        ridden.view.stage.advance(dt, &mut ridden.view.stage_rng);
        fly(&mut ridden, 1);
        ridden.advance_projectile_flares();
    }
    // First drawn on the tick it is fired: the effect starts with nothing out.
    let mut fresh = flying(1);
    fresh.advance_projectile_flares();
    // First drawn `AGE` ticks into the flight.
    let mut late = flying(AGE);
    late.advance_projectile_flares();

    let (ridden, fresh, late) = (alive(&ridden), alive(&fresh), alive(&late));
    assert!(
        ridden > fresh + 8,
        "the flight lays particles: {fresh} then {ridden}"
    );
    assert!(
        late.abs_diff(ridden) <= ridden / 4,
        "first drawn mid-flight {late}, drawn all along {ridden}, fresh {fresh}"
    );
}
