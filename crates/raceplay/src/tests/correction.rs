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

/// The camera of a client seated in slot 1 follows slot 1: the spring is advanced toward the
/// craft the view is built from, not toward slot 0.
#[test]
fn a_seat_one_camera_follows_seat_one() {
    let Scene { sc, mut client, .. } = scene();
    client.set_seat(Some(1));
    for t in 0..120 {
        client.tick(&sc.truth(START + t.min(59)));
    }
    let eye = client.tick_view().inverse().w_axis.truncate();
    let own = client.sim.world.ships[1].physics.body.position;
    let other = client.sim.world.ships[0].physics.body.position;
    println!(
        "eye to seat 1: {}, eye to slot 0: {}",
        (eye - own).length(),
        (eye - other).length()
    );
    assert!((eye - own).length() < (eye - other).length());
    assert!((eye - own).length() < 40.0, "{}", (eye - own).length());
}

/// Whichever seat a client holds, and whichever view it flies, the camera is built from that
/// seat's craft and from nothing else: seat 0 and seat 1 of one race frame two different
/// craft, each within the chase distance of its own.
#[test]
fn every_seat_camera_follows_its_own_craft() {
    use oag_display::display::CameraView;
    for view in [CameraView::Close, CameraView::Far, CameraView::Internal] {
        let mut eyes = Vec::new();
        for seat in 0..2 {
            let Scene { sc, mut client, .. } = scene();
            client.set_camera_view(view);
            client.set_seat(Some(seat));
            for t in 0..120 {
                client.tick(&sc.truth(START + t.min(59)));
            }
            let eye = client.camera_position();
            let ships = &client.sim.world.ships;
            let own = (eye - ships[seat].physics.body.position).length();
            let other = (eye - ships[1 - seat].physics.body.position).length();
            assert!(
                own < 40.0 && own < other,
                "{view:?} seat {seat}: eye {own} from its craft, {other} from the other"
            );
            eyes.push(eye);
        }
        assert!(
            (eyes[0] - eyes[1]).length() > 1.0,
            "{view:?}: one eye for two seats"
        );
    }
}

/// The way the chase camera points, as a unit vector in the world.
fn aim(race: &Race) -> Vec3 {
    (race.tick_view().inverse().z_axis.truncate()).normalize()
}

/// Shifts the server's copy of the client's own craft by `by` units sideways and corrects
/// onto it: what a contact the client guessed wrong looks like on its own craft. Returns the
/// degrees the camera turned at the correction (the eye is the spring's point and stays; the
/// camera swings to look at the craft where it now is), the pop, and the race.
fn own_craft_corrected(smooth: bool, by: f32) -> (f32, f32, Box<Race>) {
    let Scene {
        sc,
        mut client,
        authoritative,
    } = scene();
    client.set_seat(Some(0));
    client.set_correction_smoothing(smooth);
    let mut shifted = authoritative;
    let right = shifted.world.ships[0].physics.body.right();
    shifted.world.ships[0].physics.body.position += right * by;
    let before = aim(&client);
    let reconciled = client.reconcile(&shifted, (START..START + WINDOW).map(|t| sc.truth(t)));
    let turned = before
        .dot(aim(&client))
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();
    (turned, reconciled.pops[0], Box::new(client))
}

/// The craft the camera follows glides onto its corrected place with the camera: with
/// smoothing the camera does not swing at the correction, and without it it turns by the pop.
#[test]
fn the_own_craft_and_its_camera_glide_together() {
    let (turned, pop, client) = own_craft_corrected(true, 3.0);
    assert!(pop > 2.0, "the scene must move the own craft: {pop}");
    assert!(
        turned < 0.1,
        "the camera turned {turned} degrees at the correction"
    );
    let offset = client.correction_offset_of(0);
    assert!(
        offset.length() > 2.0,
        "the own craft carries the offset: {offset:?}"
    );
    let drawn_at = client.ship_tick_body(0).position;
    let sim_at = client.sim.world.ships[0].physics.body.position;
    assert!(
        ((drawn_at - sim_at) - offset).length() < 1.0e-4,
        "the hull is drawn with the offset"
    );

    let (turned, _, _) = own_craft_corrected(false, 3.0);
    println!("camera swing without smoothing: {turned} degrees");
    assert!(
        turned > 1.0,
        "without smoothing the camera swings: {turned}"
    );
}

/// Glides, and the hash does not know: on and off agree to the bit while the own craft is
/// drawn elsewhere. A correction past the snap distance (a respawn, a wreck) is drawn where
/// it lands even for the own craft.
#[test]
fn the_own_glide_is_view_only_and_a_snap_is_not_glided() {
    let (_, _, on) = own_craft_corrected(true, 3.0);
    let (_, _, off) = own_craft_corrected(false, 3.0);
    assert_eq!(on.sim.state_hash(), off.sim.state_hash());
    assert_eq!(off.correction_offset_of(0), Vec3::ZERO);
    let (turned, _, far) = own_craft_corrected(true, crate::correction::SNAP_DISTANCE + 5.0);
    assert_eq!(far.correction_offset_of(0), Vec3::ZERO);
    assert!(turned > 1.0, "a snap is drawn where it lands: {turned}");
}

/// A hit the client had not predicted is drawn where it lands, however small the move: the
/// own craft in a correction that took shield off it is not glided and shakes the camera once,
/// and contact wear (under [`crate::correction::HIT_SHIELD`]) is glided and shakes nothing.
#[test]
fn a_hit_that_first_shows_in_a_correction_is_not_softened() {
    let corrected = |lost: f32| {
        let Scene {
            sc,
            mut client,
            authoritative,
        } = scene();
        client.set_seat(Some(0));
        // The fixture's craft have no shield pool; give this one a pool of 100.
        client.sim.world.ships[0].handling.dimensions.shield = 100.0;
        client.sim.world.ships[0].physics.shield = 90.0;
        client.view.shake.cancel();
        let mut shifted = authoritative;
        shifted.world.ships[0].handling.dimensions.shield = 100.0;
        let right = shifted.world.ships[0].physics.body.right();
        shifted.world.ships[0].physics.body.position += right * 3.0;
        shifted.world.ships[0].physics.shield = 90.0 - lost;
        client.reconcile(&shifted, (START..START + WINDOW).map(|t| sc.truth(t)));
        client
    };
    let wear = corrected(0.2);
    assert!(wear.correction_offset_of(0).length() > 2.0, "wear glides");
    assert!(!wear.view.shake.active(), "wear shakes nothing");
    let hit = corrected(20.0);
    assert_eq!(
        hit.correction_offset_of(0),
        Vec3::ZERO,
        "a hit is drawn where it lands"
    );
    assert!(hit.view.shake.active(), "and shakes the camera there");
}

/// The shake fires once per hit: a second correction that takes nothing more off the shield
/// does not arm it again.
#[test]
fn a_hit_seen_in_a_correction_shakes_once() {
    let Scene {
        sc,
        mut client,
        authoritative,
    } = scene();
    client.set_seat(Some(0));
    client.sim.world.ships[0].handling.dimensions.shield = 100.0;
    client.sim.world.ships[0].physics.shield = 90.0;
    let mut hit = authoritative;
    hit.world.ships[0].handling.dimensions.shield = 100.0;
    hit.world.ships[0].physics.shield = 60.0;
    client.view.shake.cancel();
    let replay = || (START..START + WINDOW).map(|t| sc.truth(t));
    client.reconcile(&hit, replay());
    assert!(client.view.shake.active());
    client.view.shake.cancel();
    client.reconcile(&hit, replay());
    assert!(
        !client.view.shake.active(),
        "the same loss is not a new hit"
    );
}
