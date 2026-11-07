//! [`Timeline`] on a fixture race: where a seek lands, what ends the prefix, what the ceiling
//! does, and that a visit leaves the live race alone. The same claims on every title's data are
//! `tests/timeline_ground_truth.rs`.

use super::reconcile_predict::fixture;
use super::*;
use crate::timeline::{KEYFRAME_TICKS, OpenError, Pass, SeekError, Timeline};
use oag_gameplay::PlayerInputs;
use oag_physics::pilot_assist::Level;
use oag_replay::{Header, Replay};

const TICKS: u64 = 400;

fn inputs(tick: u64) -> PlayerInputs {
    PlayerInputs::single(
        InputSnapshot {
            stick_x: (tick as f32 * 0.07).sin() * 0.8,
            ..InputSnapshot::default()
        }
        .sanitised(),
    )
}

/// A recorded run with Pilot Assist changed on a keyframe's tick and between two keyframes, and
/// its text at every tick.
fn recording(ticks: u64, events: &[(u64, Level)]) -> (Replay, Vec<String>) {
    let mut race = Race::start(fixture());
    race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
    let mut text = vec![format!("{:?}", race.sim)];
    for tick in 0..ticks {
        for (_, level) in events.iter().filter(|(at, _)| *at == tick) {
            race.change_pilot_assist(*level);
        }
        race.tick(&inputs(tick));
        race.drain_cues();
        text.push(format!("{:?}", race.sim));
    }
    (race.recorded().expect("recording"), text)
}

/// The first bytes that differ, since a whole `RaceSim` is more text than a failure can print.
fn same_as_recorded(timeline: &Timeline, recorded: &str, tick: u64) {
    let text = format!("{:?}", timeline.probe().sim);
    if let Some(at) = text.bytes().zip(recorded.bytes()).position(|(a, b)| a != b) {
        let window = |s: &str| s[at.saturating_sub(120)..(at + 80).min(s.len())].to_string();
        panic!(
            "tick {tick} differs from the recorded run:\n{}\n{}",
            window(&text),
            window(recorded)
        );
    }
    assert_eq!(text.len(), recorded.len(), "tick {tick}");
}

fn opened(replay: Replay) -> Timeline {
    let mut timeline = Timeline::open(fixture(), replay).expect("the race it records");
    assert_eq!(timeline.wait(), Pass::Complete);
    timeline
}

/// Fails if a seek restores with no resimulation, if an event is applied twice or not at all,
/// or if a stale probe walks on from the wrong tick: the order is out of step on purpose.
#[test]
fn a_seek_lands_on_the_recorded_run_in_any_order_and_across_events() {
    let on_a_keyframe = 2 * KEYFRAME_TICKS;
    let between = 3 * KEYFRAME_TICKS + 17;
    let (replay, text) = recording(
        TICKS,
        &[(on_a_keyframe, Level::Extreme), (between, Level::Normal)],
    );
    let mut timeline = opened(replay);
    assert_eq!(timeline.prefix(), Some(TICKS));
    let mut targets: Vec<u64> = (1..TICKS).collect();
    // Coprime with the length, so every tick comes up, far from the last.
    targets.sort_by_key(|tick| (tick * 97) % 397);
    for target in targets {
        assert_eq!(timeline.seek(target), Ok(target));
        same_as_recorded(&timeline, &text[target as usize], target);
    }
}

/// The level is not in the state hash, so only the text can tell. A probe that lost the event
/// on keyframe 120 would read the old level at tick 121.
#[test]
fn an_event_on_a_keyframes_own_tick_applies_after_it() {
    let at = 2 * KEYFRAME_TICKS;
    let (replay, _) = recording(TICKS, &[(at, Level::Extreme)]);
    let mut timeline = opened(replay);
    timeline.seek(at).expect("in the prefix");
    let before = timeline.probe().pilot_assist_level();
    timeline.seek(at + 1).expect("in the prefix");
    assert_ne!(before, timeline.probe().pilot_assist_level());
    assert_eq!(timeline.probe().pilot_assist_level(), Level::Extreme);
}

#[test]
fn stepping_matches_seeking_and_stops_at_the_prefix() {
    let (replay, text) = recording(130, &[]);
    let mut timeline = opened(replay);
    timeline.seek(0).expect("in the prefix");
    for tick in 1..=130 {
        assert_eq!(timeline.step(), Ok(Some(tick)));
    }
    assert!(format!("{:?}", timeline.probe().sim) == text[130]);
    assert_eq!(timeline.step(), Ok(None));
    assert_eq!(
        timeline.seek(131),
        Err(SeekError::NotBuilt {
            tick: 131,
            prefix: Some(130)
        })
    );
}

/// A flipped stick at tick 200 is caught at the stored hash of tick 240: the prefix ends at the
/// one before, and no keyframe past it is kept.
#[test]
fn a_desync_ends_the_prefix_at_the_last_agreeing_hash() {
    let (mut replay, _) = recording(TICKS, &[]);
    let flipped = &mut replay.inputs[0][200];
    flipped.stick_x = if flipped.stick_x > 0.0 { -1.0 } else { 1.0 };
    let (expected, _) = (replay.hashes[3], ());
    let mut timeline = Timeline::open(fixture(), replay).expect("agrees at tick zero");
    let Pass::Desynced(desync) = timeline.wait() else {
        panic!("{:?}", timeline.pass());
    };
    assert_eq!((desync.tick, desync.expected), (240, expected));
    assert_eq!(timeline.prefix(), Some(180));
    assert_eq!(timeline.keyframes(), 4, "ticks 0, 60, 120 and 180");
    assert_eq!(timeline.seek(180), Ok(180));
    assert!(timeline.seek(181).is_err());
}

#[test]
fn the_wrong_race_is_refused_at_tick_zero() {
    let (mut replay, _) = recording(10, &[]);
    replay.initial_hash ^= 1;
    let Err(OpenError::Desync(desync)) = Timeline::open(fixture(), replay) else {
        panic!("another race must be refused");
    };
    assert_eq!(desync.tick, 0);
}

/// A ceiling of a few keyframes doubles the interval until they fit, and every tick still
/// reproduces, each from further away.
#[test]
fn past_the_ceiling_the_interval_doubles_and_a_seek_still_arrives() {
    let (replay, text) = recording(TICKS, &[]);
    let one = Race::start(fixture()).sim.snapshot_bytes();
    let mut timeline =
        Timeline::open_within(fixture(), replay, 4 * one + one / 2).expect("the race it records");
    assert_eq!(timeline.wait(), Pass::Complete);
    assert_eq!(timeline.interval(), 2 * KEYFRAME_TICKS);
    assert_eq!(timeline.keyframes(), 4, "ticks 0, 120, 240 and 360");
    assert_eq!(timeline.seek(239), Ok(239));
    assert!(format!("{:?}", timeline.probe().sim) == text[239]);
}

/// A restored probe shares the keyframes' constant half, so a timeline holds one.
#[test]
fn a_keyframe_holds_the_shared_half_not_a_copy_of_it() {
    let (replay, _) = recording(130, &[]);
    let mut timeline = opened(replay);
    timeline.seek(61).expect("in the prefix");
    let reference = timeline.probe().sim.clone();
    timeline.seek(125).expect("in the prefix");
    assert!(timeline.probe().sim.shares_constant_half_with(&reference));
}

/// One race goes through a visit and one does not: same hash at every tick, and the same text
/// on both sides of it.
#[test]
fn a_visit_leaves_the_live_race_as_it_found_it() {
    let start = || {
        let mut race = Race::start(fixture());
        race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
        race
    };
    let (mut alone, mut visited) = (start(), start());
    let mut timeline = Timeline::live(fixture(), &visited);
    for tick in 0..TICKS {
        alone.tick(&inputs(tick));
        alone.drain_cues();
        visited.tick(&inputs(tick));
        visited.drain_cues();
        timeline.keep(&visited);
        if tick == 299 {
            let before = format!("{:?}", visited.sim);
            timeline.visit(&visited);
            for target in [12, 299, 61, 300, 150] {
                assert_eq!(timeline.seek(target), Ok(target));
            }
            assert!(format!("{:?}", timeline.probe().sim.world.tick) != before);
            assert!(format!("{:?}", visited.sim) == before, "the visit wrote");
        }
        assert_eq!(alone.sim.state_hash(), visited.sim.state_hash(), "{tick}");
    }
    assert!(format!("{:?}", alone.sim) == format!("{:?}", visited.sim));
}

/// With nothing recorded there are no inputs to resimulate, so a seek stops at a keyframe.
#[test]
fn a_race_that_records_nothing_rewinds_to_whole_keyframes() {
    let mut race = Race::start(fixture());
    let mut timeline = Timeline::live(fixture(), &race);
    for tick in 0..130 {
        race.tick(&inputs(tick));
        race.drain_cues();
        timeline.keep(&race);
    }
    timeline.visit(&race);
    assert_eq!(timeline.seek(100), Ok(60));
    assert_eq!(timeline.probe().sim.world.tick, 60);
    assert_eq!(timeline.step(), Ok(None));
}
