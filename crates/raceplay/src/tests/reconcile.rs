//! An in-process loopback client and server, and what reconciling one against
//! the other is asserted to do.
//!
//! The first piece of network play, and deliberately the piece with no network
//! in it. The protocol is settled - the server is the sole authority, each
//! client predicts ahead with its own input, and a correction is a
//! snapshot-and-replay rather than a smoothed nudge - and what it asks for
//! before a transport or `oag-net` exists is exactly this: two simulations in
//! one test binary, inputs handed across directly, asserting the client's
//! post-reconciliation [`RaceSim::state_hash`] matches the server's.
//!
//! What that buys is the thing a protocol cannot be designed without: proof
//! that [`Race::snapshot_sim`]/[`Race::restore_sim`] plus a replay of buffered
//! inputs lands on the authoritative state exactly, against the real
//! [`Race::tick`] rather than against a model of it.
//!
//! # The misprediction is injected on the primary slot, and on a remote one
//!
//! The first test below mispredicts the player's own slot, the simplest shape.
//! What a real netcode client mispredicts is someone else's craft, and that
//! needed [`Race::tick`] to step more than [`World::primary_slot`]: until
//! 2026-10-13 a second `Remote` slot's [`InputSnapshot`] reached no craft and
//! moved no hash, measured as two races ticked 300 times, identical but for
//! slot 1's stick held hard over, hashing the same. It steps every
//! person-flown slot now, so [`a_remote_slots_input_reaches_its_own_craft`]
//! holds the positive of that measurement and
//! [`a_mispredicted_remote_slot_reconciles_to_the_servers_hash`] runs the
//! reconciliation with the wrong guess on slot 1.
//!
//! # Why the stick and not a button
//!
//! Also measured: the thrust buttons change nothing for the first 272 ticks,
//! because [`oag_race::RaceState::thrust_gated`] holds the start-line
//! countdown. Steering is not gated, so `stick_x` diverges two races inside
//! thirty ticks and these tests stay short.

use super::*;
use oag_gameplay::{Controller, PlayerInputs};

/// Ticks the client and server agree on before anything goes wrong. The
/// authoritative snapshot is taken here.
const AGREED_TICKS: u32 = 20;

/// Ticks the client runs ahead on its own prediction. Long enough that a wrong
/// prediction is a visibly different trajectory and not a rounding difference.
const PREDICTED_TICKS: u32 = 40;

/// A race with an engine that responds, and the player's own off-track rescue
/// switched off so a craft that wanders cannot be teleported back and hide a
/// divergence behind a respawn.
fn loopback_race() -> Race {
    let mut handling = hulled_handling();
    handling.engine.amount = 20.0;
    handling.engine.accelcap = 1000.0;
    without_player_rescue(Race::start(setup(handling)))
}

/// One tick's input for the player's slot, steering by `stick_x`.
fn steering(stick_x: f32) -> PlayerInputs {
    PlayerInputs::single(
        InputSnapshot {
            stick_x,
            ..InputSnapshot::default()
        }
        .sanitised(),
    )
}

/// Ticks a race and throws the cues away.
///
/// Every caller here does this, because a cue is a per-tick *output* awaiting
/// a drain (ADR-0018) and there is no audio in this process to drain it. It is
/// also the one place a replay is visibly not free: the replayed ticks raise
/// their cues a second time, and only the fact that nothing is listening makes
/// that harmless. See [`Race::restore_sim`].
fn advance(race: &mut Race, inputs: &PlayerInputs) {
    race.tick(inputs);
    race.drain_cues();
}

/// The whole claim, end to end: a client that predicted wrong, handed the
/// server's authoritative snapshot and its own buffered inputs, arrives at the
/// server's exact state.
///
/// Four assertions in sequence, and the third is the one that stops this being
/// a restatement of determinism:
///
/// 1. Client and server agree while they are fed the same inputs.
/// 2. The client's prediction diverges from the server's truth. **If this
///    failed the test would pass vacuously**, reconciling a state that was
///    already correct.
/// 3. Restoring the snapshot alone does not get there - the client is now
///    correct at [`AGREED_TICKS`] and stale by [`PREDICTED_TICKS`].
/// 4. Replaying the true inputs over that restored state reaches the server's
///    hash exactly.
#[test]
fn a_mispredicted_run_reconciles_to_the_servers_hash() {
    let mut server = loopback_race();
    let mut client = loopback_race();

    for _ in 0..AGREED_TICKS {
        let inputs = steering(1.0);
        advance(&mut server, &inputs);
        advance(&mut client, &inputs);
    }
    assert_eq!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "client and server disagree while being fed identical inputs - \
         nothing after this would mean anything"
    );

    // What the server would send: its own state at the tick it has finished,
    // which the client tags and keeps until it is contradicted.
    let authoritative = server.snapshot_sim();
    let authoritative_tick = server.sim.world.tick;

    // The client buffers what it believes the truth is, and it is wrong: it
    // steers the other way for the whole run. A real client's buffer holds its
    // own captured input; here it holds the prediction, so that the replay
    // below can be handed the correction and show the two differ.
    let truth: Vec<PlayerInputs> = (0..PREDICTED_TICKS).map(|_| steering(1.0)).collect();
    let predicted: Vec<PlayerInputs> = (0..PREDICTED_TICKS).map(|_| steering(-1.0)).collect();

    for inputs in &truth {
        advance(&mut server, inputs);
    }
    for inputs in &predicted {
        advance(&mut client, inputs);
    }
    assert_ne!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "the prediction did not actually diverge, so reconciling it proves nothing"
    );

    client.restore_sim(&authoritative);
    assert_eq!(
        client.sim.world.tick, authoritative_tick,
        "restoring did not wind the clock back to the snapshot's own tick"
    );
    assert_ne!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "the restored client already matched the server, so the replay below \
         is not what closes the gap"
    );

    for inputs in &truth {
        advance(&mut client, inputs);
    }
    assert_eq!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "reconciliation did not land on the authoritative state"
    );
    assert_eq!(
        client.sim.world.tick, server.sim.world.tick,
        "the replay left the client on a different tick from the server"
    );
}

/// The common case, and the reason reconciliation is invisible in it: a client
/// that predicted correctly reconciles to the state it was already in.
///
/// Worth its own test rather than being assumed from the one above. A
/// correction that is a no-op when nothing was wrong is what keeps the picture
/// steady between the rare ticks where something was.
#[test]
fn reconciling_a_correct_prediction_changes_nothing() {
    let mut server = loopback_race();
    let mut client = loopback_race();

    for _ in 0..AGREED_TICKS {
        let inputs = steering(1.0);
        advance(&mut server, &inputs);
        advance(&mut client, &inputs);
    }
    let authoritative = server.snapshot_sim();

    let truth: Vec<PlayerInputs> = (0..PREDICTED_TICKS).map(|_| steering(0.5)).collect();
    for inputs in &truth {
        advance(&mut server, inputs);
        advance(&mut client, inputs);
    }

    let predicted = client.sim.state_hash();
    assert_eq!(
        predicted,
        server.sim.state_hash(),
        "the client did not predict correctly, so this is the other test"
    );

    client.restore_sim(&authoritative);
    for inputs in &truth {
        advance(&mut client, inputs);
    }

    assert_eq!(
        predicted,
        client.sim.state_hash(),
        "reconciling a correct prediction moved the client off it"
    );
}

/// A loopback race with a second craft on the grid, flown by `controller`.
///
/// The craft is slot 0's, copied and set well to its right: sharing a spawn
/// point would have [`Race::resolve_craft_pairs`] fling the two apart and hide
/// what slot 1's stick did behind the collision. Slot 1 is active, because
/// a slot with no craft is not flown at all - the previous version of this test
/// left it inactive, which is why it could pass for the wrong reason.
fn two_person_race(controller: Controller) -> Race {
    let mut race = loopback_race();
    // The fixture's craft cannot turn (`Handling::ZERO` has no yaw authority),
    // and "slot 1 turned" is the claim, so both get some.
    race.sim.world.ships[0].handling.turning = oag_physics::params::Turning {
        amount: 60.0,
        falloff: 5.0,
        gain: 5.0,
    };
    let mut second = race.sim.world.ships[0];
    let pose = oag_gameplay::Pose {
        position: second.physics.body.position + second.physics.body.right() * 60.0,
        orientation: second.physics.body.orientation,
    };
    second.place_at(pose);
    race.sim.world.ships[1] = second;
    race.sim.world.ships[1].active = true;
    race.sim.world.ship_count = 2;
    race.sim.world.controllers[1] = controller;
    race
}

/// One tick's input for both slots: slot 0 steers by `own`, slot 1 by `other`.
fn two_steering(own: f32, other: f32) -> PlayerInputs {
    let held = |stick_x: f32| {
        InputSnapshot {
            stick_x,
            ..InputSnapshot::default()
        }
        .sanitised()
    };
    let mut inputs = PlayerInputs::none();
    inputs.set(0, held(own));
    inputs.set(1, held(other));
    inputs
}

/// A second person's input reaches *their* craft and nobody else's.
///
/// The positive of `a_remote_slots_input_does_not_yet_reach_the_hash`, which
/// this replaces. Two races identical but for slot 1's stick: it must move the
/// hash, turn slot 1's craft, and leave slot 0's alone - the third being the
/// guard against the step reading slot 0's snapshot for everybody. And the
/// slot is stepped by the player step, not the opponent loop: a person's slot
/// has an [`Evaluations`](crate::player_step::Evaluations) entry and an AI
/// slot's does not.
#[test]
fn a_remote_slots_input_reaches_its_own_craft() {
    for controller in [Controller::Remote, Controller::Local] {
        let mut idle = two_person_race(controller);
        let mut hard_over = two_person_race(controller);
        let start_heading = hard_over.sim.world.ships[1].physics.body.forward();

        for _ in 0..60 {
            let evaluations = hard_over.tick(&two_steering(0.0, 1.0));
            assert!(
                evaluations.get(1).is_some(),
                "{controller:?}: slot 1 was not stepped"
            );
            hard_over.drain_cues();
            advance(&mut idle, &two_steering(0.0, 0.0));
        }

        assert_ne!(
            idle.sim.state_hash(),
            hard_over.sim.state_hash(),
            "{controller:?}: slot 1's stick did not reach the hash"
        );
        let turned = hard_over.sim.world.ships[1].physics.body.forward();
        let held_straight = idle.sim.world.ships[1].physics.body.forward();
        assert!(
            (turned - start_heading).length() > 1.0e-3,
            "{controller:?}: slot 1's craft did not turn: {start_heading:?} -> {turned:?}"
        );
        assert!(
            (held_straight - start_heading).length() < (turned - start_heading).length(),
            "{controller:?}: the idle craft turned as much as the one held over"
        );
        assert_eq!(
            idle.sim.world.ships[0].physics.body.position,
            hard_over.sim.world.ships[0].physics.body.position,
            "{controller:?}: slot 1's input moved slot 0's craft"
        );
    }
}

/// An AI slot gives no [`Evaluations`](crate::player_step::Evaluations) entry
/// and obeys no snapshot: the person-flown set did not swallow the grid.
#[test]
fn an_ai_slot_is_not_stepped_as_a_player() {
    let mut idle = two_person_race(Controller::Ai);
    let mut shouted_at = two_person_race(Controller::Ai);
    for _ in 0..60 {
        let evaluations = shouted_at.tick(&two_steering(0.0, 1.0));
        assert!(
            evaluations.get(1).is_none(),
            "an AI slot was stepped as a player"
        );
        shouted_at.drain_cues();
        advance(&mut idle, &two_steering(0.0, 0.0));
    }
    assert_eq!(
        idle.sim.state_hash(),
        shouted_at.sim.state_hash(),
        "a snapshot in an AI slot reached the simulation"
    );
}

/// What a real netcode client does: it predicts *someone else's* craft wrong,
/// is handed the server's snapshot, replays, and lands on the server's hash.
///
/// [`a_mispredicted_run_reconciles_to_the_servers_hash`] with the wrong guess
/// on slot 1. The same non-vacuous guards, for the same reasons: the guess must
/// actually diverge, and the restored client must still differ from the server
/// until the replay closes it.
#[test]
fn a_mispredicted_remote_slot_reconciles_to_the_servers_hash() {
    let mut server = two_person_race(Controller::Remote);
    let mut client = two_person_race(Controller::Remote);

    for _ in 0..AGREED_TICKS {
        advance(&mut server, &two_steering(0.5, 1.0));
        advance(&mut client, &two_steering(0.5, 1.0));
    }
    assert_eq!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "client and server disagree while being fed identical inputs"
    );
    let authoritative = server.snapshot_sim();

    // The local player's own input is known exactly; the remote player's is a
    // guess, and the guess is the opposite of what they did.
    let truth: Vec<PlayerInputs> = (0..PREDICTED_TICKS)
        .map(|_| two_steering(0.5, 1.0))
        .collect();
    let guessed: Vec<PlayerInputs> = (0..PREDICTED_TICKS)
        .map(|_| two_steering(0.5, -1.0))
        .collect();
    for inputs in &truth {
        advance(&mut server, inputs);
    }
    for inputs in &guessed {
        advance(&mut client, inputs);
    }
    assert_ne!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "the guess about slot 1 did not diverge, so reconciling it proves nothing"
    );

    client.restore_sim(&authoritative);
    assert_ne!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "the restored client already matched the server"
    );
    for inputs in &truth {
        advance(&mut client, inputs);
    }
    assert_eq!(
        server.sim.state_hash(),
        client.sim.state_hash(),
        "reconciliation did not land on the authoritative state for a remote slot"
    );
    assert_eq!(client.sim.world.tick, server.sim.world.tick);
}

/// [`Race::restore_sim`] puts the simulation back and leaves the view alone.
///
/// The camera perspective stands in for the whole view half: it is view-only
/// state with a public read, and it is not something any tick recomputes from
/// the world. A restore that took the whole `Race` rather than its `sim` would
/// put the perspective back too, which is the jolt the split exists to
/// prevent.
///
/// It says nothing about the *replay*. Re-running ticks after a restore moves
/// the chase camera, the sparks and the shake again by design - see
/// [`Race::restore_sim`]'s own doc comment.
#[test]
fn restoring_the_sim_leaves_the_view_alone() {
    let mut race = loopback_race();
    for _ in 0..AGREED_TICKS {
        advance(&mut race, &steering(1.0));
    }
    let snapshot = race.snapshot_sim();

    race.set_camera_view(oag_display::display::CameraView::Internal);
    for _ in 0..AGREED_TICKS {
        advance(&mut race, &steering(-1.0));
    }
    race.restore_sim(&snapshot);

    assert_eq!(
        race.camera_view(),
        oag_display::display::CameraView::Internal,
        "restoring the simulation reset the camera perspective with it"
    );
    assert_eq!(
        race.sim.world.tick,
        u64::from(AGREED_TICKS),
        "the simulation half did not go back"
    );
}
