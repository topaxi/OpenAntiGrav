//! Which sound cues a tick raises, and on which tick.
//!
//! The assertion that matters here is **positional**: a cue arriving is easy,
//! a cue arriving on the tick the thing that causes it happened is the part a
//! wiring bug breaks. Nothing here loads a bank or touches a mixer - what a
//! cue *sounds like* is `audio::sfx`'s problem and the disc-backed half is
//! `crates/game/tests/sfx_ground_truth.rs`.

use super::*;
use crate::audio::sfx::Cue;

/// A race whose ship is inside a speed pad from the first tick.
fn grid_on_a_speed_pad() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    setup.start_position = Some(oag_formats::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

#[test]
fn crossing_a_pad_raises_its_cue_on_the_tick_the_flare_is_armed() {
    let mut race = grid_on_a_speed_pad();
    assert!(race.pending_cues().is_empty(), "a cue before any tick ran");

    race.tick(&InputSnapshot::default());
    // The same edge, the same tick: `Ship_ApplySpeedupPad` calls
    // `ExhaustFlare_OnSpeedupPad` and `Sound_Play("SPEEDUPPAD")` from one
    // branch, so a port where the flare arms and the cue does not is wired
    // wrong however good the sound is.
    assert!(race.exhaust[0].boost_timer() > 0.0, "the flare did not arm");
    assert!(
        race.pending_cues().contains(&Cue::SpeedupPad),
        "the flare armed and the cue did not: {:?}",
        race.pending_cues()
    );
}

#[test]
fn sitting_on_a_pad_raises_the_cue_once_and_not_every_tick() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&InputSnapshot::default());
    let first = race
        .drain_cues()
        .iter()
        .filter(|&&c| c == Cue::SpeedupPad)
        .count();
    assert_eq!(first, 1);

    // The edge is "entered a *new* pad", not "is on one" - the same rule the
    // Zone score and the flare already follow. A pad held for a second would
    // otherwise machine-gun sixty copies of a half-second sample.
    for _ in 0..60 {
        race.tick(&InputSnapshot::default());
    }
    assert!(
        !race.drain_cues().contains(&Cue::SpeedupPad),
        "the pad re-fired while the craft sat on it"
    );
}

#[test]
fn draining_takes_the_queue_and_leaves_it_empty() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&InputSnapshot::default());
    assert!(!race.pending_cues().is_empty());
    let drained = race.drain_cues();
    assert!(!drained.is_empty());
    assert!(
        race.pending_cues().is_empty(),
        "a second reader would play the same cue twice"
    );
}

#[test]
fn a_track_with_no_pads_raises_no_pad_cue() {
    let mut race = race_with_pads(Mode::SingleRace, Vec::new());
    for _ in 0..120 {
        race.tick(&InputSnapshot::default());
        assert!(!race.drain_cues().contains(&Cue::SpeedupPad));
    }
}

/// Every cue is either raised as an edge or driven as a level - and nothing is
/// merely *loaded*.
///
/// This exists because that failed once: `shieldactive` was decoded, reported
/// in the load line and reachable through `Banks::pick`, and no code path
/// anywhere pushed it. Every test written at the time passed, because they all
/// asked whether a cue **loads** rather than whether anything plays it. So this
/// asserts over [`Cue::ALL`] and fails the day a cue is added without an
/// emitter.
#[test]
fn every_cue_has_something_that_raises_it() {
    // A cue driven from the *level* rather than from an edge: the audio layer
    // reads `Race::shield_is_up` and `Race::finished` directly, so these have
    // no entry in the queue by design.
    const BY_LEVEL: [Cue; 2] = [Cue::Engine, Cue::Shield];

    // A pad the whole grid stands on *and* a wall to scrape, so one run
    // reaches every edge. The wall is `respawn.rs`'s proven fixture - a
    // backwards-wound triangle registers no contact silently, so an invented
    // one here could make this pass by never testing anything.
    let mut setup = setup_with(
        hulled_handling(),
        vec![plane(1, -40.0, oag_physics::Surface::Wall, 0)],
    );
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    let mut race = Race::start(setup);

    let mut raised = std::collections::BTreeSet::new();
    let mut saw_impact = false;
    for tick in 0..240 {
        // A shield, put up mid-race and taken down again, so the rising edge,
        // the level and the shielded-contact branch are all exercised. Set
        // outside `tick` on purpose - this is a fixture, not a pickup grant.
        race.world.ships[0].physics.shield_pickup_timer =
            if (60..180).contains(&tick) { 1.0 } else { 0.0 };
        // Re-aimed at the wall every tick, so each one sees a fresh inbound
        // contact rather than the ship bouncing away after the first.
        let body = &mut race.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -39.7, 0.0);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        saw_impact |= race.tick(&InputSnapshot::default()).wall.impact;
        raised.extend(race.drain_cues());
    }
    assert!(
        saw_impact,
        "the fixture never reached the wall - not what this test means to check"
    );

    for cue in Cue::ALL {
        assert!(
            raised.contains(&cue) || BY_LEVEL.contains(&cue),
            "{} is loaded and nothing raises it",
            cue.name()
        );
    }
    // Named individually as well, so a future `BY_LEVEL` that quietly grew
    // cannot make the loop above vacuous.
    for cue in [
        Cue::SpeedupPad,
        Cue::Collision,
        Cue::Absorb,
        Cue::ShieldActive,
    ] {
        assert!(raised.contains(&cue), "{} was never raised", cue.name());
    }
}

#[test]
fn the_shield_announcer_fires_on_the_edge_and_not_on_the_level() {
    let mut race = grid_on_a_speed_pad();
    let mut announcements = 0;
    for tick in 0..180 {
        race.world.ships[0].physics.shield_pickup_timer = if tick >= 30 { 1.0 } else { 0.0 };
        race.tick(&InputSnapshot::default());
        announcements += race
            .drain_cues()
            .iter()
            .filter(|&&c| c == Cue::ShieldActive)
            .count();
    }
    // 150 ticks of shield, one announcement: `Shield_Activate` runs once per
    // activation, and a level-triggered version would say it 150 times.
    assert_eq!(announcements, 1);
}

/// Cues are an *output*, so a race that raises them must hash exactly like one
/// that does not. This is the guard on the committed determinism constants:
/// `docs/architecture/determinism.md` puts audio outside the simulation, and
/// the way that stays true is that nothing audio touches reaches the hasher.
#[test]
fn raising_a_cue_does_not_move_the_race_hash() {
    let mut with = grid_on_a_speed_pad();
    let mut without = grid_on_a_speed_pad();
    for _ in 0..30 {
        with.tick(&InputSnapshot::default());
        without.tick(&InputSnapshot::default());
        // One of the two has its queue drained every tick and the other never
        // does, so by the end they hold different queues entirely.
        with.drain_cues();
    }
    assert!(!without.pending_cues().is_empty(), "nothing was queued");
    assert!(with.pending_cues().is_empty());
    assert_eq!(with.state_hash(), without.state_hash());
}
