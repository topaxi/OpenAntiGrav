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
//! # The misprediction is injected on the player's own slot, and that is a
//! # limitation rather than a choice
//!
//! The natural shape for this test would be a *remote* slot predicted wrong.
//! It would assert nothing. [`Race::tick`] still steps only
//! [`World::primary_slot`] - ADR-0052 says so in as many words, and its own
//! "the player step still runs once" consequence is why - so a non-primary
//! slot's [`InputSnapshot`] reaches no craft and moves no hash. Measured here
//! before this file was written: two races ticked 300 times, identical but for
//! slot 1's stick held hard over in one of them, hash the same. So the
//! divergence is injected where the engine can currently express one, on the
//! primary slot, and the mechanism under test - restore, replay, converge - is
//! the same mechanism either way.
//! [`a_remote_slots_input_does_not_yet_reach_the_hash`] holds the measurement
//! so it fails loudly when the loop bound widens.
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

/// A non-primary slot's input reaches no craft and no hash, so the
/// misprediction above could not have been injected on a remote slot.
///
/// This is the measurement the module doc cites, kept as a test so the day
/// `Race::tick` grows its loop bound past `primary_slot` the claim fails
/// loudly instead of quietly becoming false. **A failure here is good news** -
/// it means a second slot is flown, and the test above can move onto a remote
/// one, which is what a real client mispredicts.
#[test]
fn a_remote_slots_input_does_not_yet_reach_the_hash() {
    let mut held = loopback_race();
    let mut swung = loopback_race();
    for race in [&mut held, &mut swung] {
        race.sim.world.ship_count = 2;
        race.sim.world.controllers[1] = Controller::Remote;
    }

    for tick in 0..AGREED_TICKS + PREDICTED_TICKS {
        let mut steady = PlayerInputs::none();
        steady.set(
            1,
            InputSnapshot {
                stick_x: 1.0,
                ..InputSnapshot::default()
            }
            .sanitised(),
        );
        let mut swinging = PlayerInputs::none();
        swinging.set(
            1,
            InputSnapshot {
                stick_x: if tick % 2 == 0 { 1.0 } else { -1.0 },
                ..InputSnapshot::default()
            }
            .sanitised(),
        );
        advance(&mut held, &steady);
        advance(&mut swung, &swinging);
    }

    assert_eq!(
        held.sim.state_hash(),
        swung.sim.state_hash(),
        "a remote slot's input now reaches the hash - good, and the \
         misprediction test should move onto one"
    );
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
