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
