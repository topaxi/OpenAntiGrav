//! What a reconciliation does to the picture: the drawn pose stays where it was, the
//! simulation moves, the offset glides out, and none of it reaches the hash. See
//! [`crate::correction`].

use super::reconcile_predict::{Scenario, fixture, fixture_streams, held_to_the_track};
use super::*;
use crate::correction::HALF_LIFE_TICKS;

const WINDOW: usize = 6;
/// Past the countdown, so the craft are moving.
const START: usize = 300;

/// A server and a client that have raced `START` ticks together, then the client
/// `WINDOW` ticks more on a wrong guess about the remote craft, and the authoritative
/// snapshot the server has at the tick the client left it.
struct Scene {
    sc: Scenario,
    client: Race,
    authoritative: RaceSim,
}

fn scene() -> Scene {
    // Slot 1 brakes off and on, the others hold the throttle.
    let sc = Scenario {
        name: "weave".to_string(),
        streams: fixture_streams(START + 60, |slot, tick| slot != 1 || (tick / 5) % 2 == 0),
        remote: vec![1],
    };
    let mut server = held_to_the_track(&fixture());
    let mut client = held_to_the_track(&fixture());
    for t in 0..START {
        server.tick(&sc.truth(t));
        client.tick(&sc.truth(t));
    }
    let authoritative = server.snapshot_sim();
    for t in START..START + WINDOW {
        client.tick(&sc.guess(t, Some(START - 1)));
    }
    Scene {
        sc,
        client,
        authoritative,
    }
}

fn drawn(race: &Race, slot: usize) -> Vec3 {
    race.ship_model_matrix_of(slot).w_axis.truncate()
}

/// A correction moves the simulated craft and not the drawn one: the picture shows
/// the craft exactly where the last frame did, and the offset is the whole pop.
#[test]
fn a_correction_leaves_the_drawn_craft_where_it_was() {
    let Scene {
        sc,
        mut client,
        authoritative,
    } = scene();
    let before = drawn(&client, 1);
    let sim_before = client.sim.world.ships[1].physics.body.position;
    // Replaying the same wrong guess from the server's state reaches the same place;
    // the correction proper is the true input over the window.
    let truth = (START..START + WINDOW).map(|t| sc.truth(t));
    let reconciled = client.reconcile(&authoritative, truth);
    assert_eq!(reconciled.ticks, WINDOW);
    let sim_after = client.sim.world.ships[1].physics.body.position;
    assert!(
        reconciled.pops[1] > 0.5,
        "the scene must mispredict: {}",
        reconciled.pops[1]
    );
    assert!((sim_before - sim_after).length() > 0.5);
    assert!(
        (drawn(&client, 1) - before).length() < 1.0e-3,
        "the drawn craft moved by {}",
        (drawn(&client, 1) - before).length()
    );
    assert!((client.correction_offset_of(1) - (sim_before - sim_after)).length() < 1.0e-3);
}

/// The offset halves every [`HALF_LIFE_TICKS`] drawn ticks and is under a five
/// hundredth of the pop a second later.
/// A replay is not a drawn tick and does not move it.
#[test]
fn the_offset_glides_out_on_drawn_ticks_only() {
    let Scene {
        sc,
        mut client,
        authoritative,
    } = scene();
    client.reconcile(&authoritative, (START..START + WINDOW).map(|t| sc.truth(t)));
    let first = client.correction_offset_of(1).length();
    assert!(first > 0.5);
    client.tick_sim_only(&sc.truth(START + WINDOW));
    assert_eq!(
        client.correction_offset_of(1).length(),
        first,
        "a replay is not drawn"
    );
    let mut t = START + WINDOW;
    for _ in 0..HALF_LIFE_TICKS as usize {
        client.tick(&sc.truth(t));
        t += 1;
    }
    let half = client.correction_offset_of(1).length();
    assert!(
        (half / first - 0.5).abs() < 0.03,
        "after one half-life the offset is {half} of {first}"
    );
    for _ in 0..60 {
        client.tick(&sc.truth(t.min(START + 59)));
        t += 1;
    }
    assert!(
        client.correction_offset_of(1).length() < first / 500.0,
        "after ten half-lives the offset is {:?}",
        client.correction_offset_of(1)
    );
}

/// The smoothing is view-only: the state hash is the same with it on and off, and
/// off draws the craft where the replay left it.
#[test]
fn smoothing_changes_the_picture_and_not_the_hash() {
    let Scene {
        sc,
        mut client,
        authoritative,
    } = scene();
    let mut plain = Scene {
        sc: Scenario {
            name: String::new(),
            streams: sc.streams.clone(),
            remote: sc.remote.clone(),
        },
        client: held_to_the_track(&fixture()),
        authoritative: authoritative.clone(),
    };
    for t in 0..START + WINDOW {
        plain.client.tick(&sc.truth(t.min(START - 1)));
    }
    plain.client.set_correction_smoothing(false);
    client.reconcile(&authoritative, (START..START + WINDOW).map(|t| sc.truth(t)));
    plain
        .client
        .reconcile(&authoritative, (START..START + WINDOW).map(|t| sc.truth(t)));
    assert_eq!(client.sim.state_hash(), plain.client.sim.state_hash());
    assert!(client.correction_offset_of(1).length() > 0.5);
    assert_eq!(plain.client.correction_offset_of(1), Vec3::ZERO);
    assert!(
        (drawn(&client, 1) - drawn(&plain.client, 1)).length() > 0.5,
        "smoothing must draw the craft somewhere else"
    );
    for _ in 0..30 {
        client.tick(&sc.truth(START + WINDOW));
        plain.client.tick(&sc.truth(START + WINDOW));
        assert_eq!(client.sim.state_hash(), plain.client.sim.state_hash());
    }
}

/// A correction larger than [`crate::correction::SNAP_DISTANCE`] is drawn where it
/// lands: gliding a respawned craft across the track would draw a ghost.
#[test]
fn a_large_correction_is_drawn_where_it_lands() {
    let Scene {
        sc,
        mut client,
        authoritative,
    } = scene();
    let mut far = authoritative.clone();
    far.world.ships[1].physics.body.position += Vec3::new(0.0, 0.0, 200.0);
    client.reconcile(&far, (START..START + WINDOW).map(|t| sc.truth(t)));
    assert_eq!(client.correction_offset_of(1), Vec3::ZERO);
}
