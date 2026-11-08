//! [`Viewer`] on a fixture race: playing a recording back through `Race::tick`, a seek's 60
//! drawn ticks landing on the recorded run, what a seek back from the end leaves, and what the
//! viewer does when the build stops matching. The same claims on every title's data are
//! `tests/replay_viewer_ground_truth.rs`.

use super::reconcile_predict::fixture;
use super::*;
use crate::timeline::Pass;
use crate::viewer::{Moved, SPEEDS, State, Viewer};
use oag_gameplay::PlayerInputs;
use oag_replay::{Header, Replay};

const TICKS: u64 = 400;

/// Thrust on for 80 ticks of every 120: the fixture's craft only moves when it is thrusting.
fn thrusting(tick: u64) -> bool {
    tick % 120 < 80
}

fn snapshot(tick: u64, thrust: bool) -> InputSnapshot {
    let mut buttons = oag_gameplay::input::Input::new();
    let mask = if thrust {
        oag_gameplay::input::Button::Cross.bit()
    } else {
        0
    };
    buttons.begin_frame(mask);
    buttons.begin_frame(mask);
    InputSnapshot {
        buttons,
        stick_x: (tick as f32 * 0.07).sin() * 0.8,
        ..InputSnapshot::default()
    }
    .sanitised()
}

fn inputs(tick: u64) -> PlayerInputs {
    PlayerInputs::single(snapshot(tick, thrusting(tick)))
}

/// A recorded run and its `RaceSim` text at every tick.
fn recording(ticks: u64) -> (Replay, Vec<String>) {
    let mut race = Race::start(fixture());
    race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
    let mut text = vec![format!("{:?}", race.sim)];
    for tick in 0..ticks {
        race.tick(&inputs(tick));
        race.drain_cues();
        text.push(format!("{:?}", race.sim));
    }
    (race.recorded().expect("recording"), text)
}

/// A viewer over the fixture and the race it draws, handed over as the session hands it.
fn open(replay: Replay) -> (Box<Viewer>, Box<Race>) {
    let mut viewer = Box::new(Viewer::open(fixture(), replay).expect("the race it records"));
    let mut race = Box::new(Race::start(fixture()));
    viewer.begin(&mut race).expect("the launch settings");
    while viewer.built().is_none() {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    (viewer, race)
}

/// Waits for the pass to end, for a test that reads where it ended.
fn pass_ended(viewer: &mut Viewer) -> Pass {
    loop {
        let pass = viewer.pass();
        if pass != Pass::Running {
            return pass;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

fn assert_stands_at(race: &Race, text: &[String], tick: u64) {
    let now = format!("{:?}", race.sim);
    let recorded = &text[tick as usize];
    if let Some(at) = now.bytes().zip(recorded.bytes()).position(|(a, b)| a != b) {
        let window = |s: &str| s[at.saturating_sub(120)..(at + 80).min(s.len())].to_string();
        panic!(
            "the race on screen at tick {tick} differs from the recorded run:\n{}\n{}",
            window(&now),
            window(recorded)
        );
    }
    assert_eq!(now.len(), recorded.len(), "tick {tick}");
}

/// Fails if playback drives the race with the wrong inputs or tick, or stops short.
#[test]
fn playing_a_recording_reproduces_every_tick_and_ends_at_the_last() {
    let (replay, text) = recording(TICKS);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    assert_eq!(
        viewer.update(&mut race),
        Moved::Seeked,
        "the first frame puts the race at 0"
    );
    assert_stands_at(&race, &text, 0);
    for tick in 1..=TICKS {
        assert_eq!(viewer.update(&mut race), Moved::Stepped(1));
        assert_stands_at(&race, &text, tick);
    }
    assert_eq!(viewer.state(), State::Ended);
    assert_eq!(
        viewer.update(&mut race),
        Moved::Still,
        "nothing past the last tick"
    );
    assert_eq!(viewer.at(), TICKS);
}

/// Fails if the 60 drawn ticks before a target are skipped, run on the wrong inputs, or the
/// keyframe is restored at the wrong tick: the order is out of step on purpose.
#[test]
fn a_seek_lands_on_the_recorded_run_wherever_it_goes() {
    let (replay, text) = recording(TICKS);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    let mut targets: Vec<u64> = (0..=TICKS).step_by(7).collect();
    targets.sort_by_key(|tick| (tick * 97) % 397);
    for target in targets {
        assert_eq!(viewer.seek(&mut race, target), Moved::Seeked);
        assert_eq!(viewer.at(), target);
        assert_stands_at(&race, &text, target);
    }
}

/// A seek past the built prefix is held at it, and the recording's end is the last tick.
#[test]
fn a_seek_past_the_end_stops_at_the_end() {
    let (replay, text) = recording(TICKS);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    viewer.seek(&mut race, TICKS + 5_000);
    assert_eq!(viewer.at(), TICKS);
    assert_stands_at(&race, &text, TICKS);
}

/// The board is taken once at the finish and a wreck's tick written once: a seek back from past
/// the finish must not leave them over a race that is running again.
#[test]
fn a_seek_back_clears_what_the_view_kept_about_a_later_moment() {
    let (replay, _) = recording(TICKS);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    viewer.seek(&mut race, TICKS);
    race.view.results = Some(crate::scoreboard::Board {
        rows: Vec::new(),
        laps_target: None,
        tick: TICKS,
    });
    race.view.wreck_ended_tick = Some(TICKS);
    viewer.seek(&mut race, 100);
    assert!(race.results().is_none());
    assert!(race.view.wreck_ended_tick.is_none());
}

/// A seek is a camera cut: the drawn view jumps, and the motion blur and a temporal upscaler
/// must not read the jump as motion (`docs/ui/replays.md`, "The viewer"). Without it, the first
/// frame after a seek smears the whole screen whatever the craft's speed.
#[test]
fn a_seek_is_a_camera_cut() {
    let (replay, _) = recording(TICKS);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    // The first update starts playback, which is a seek to the start.
    viewer.update(&mut race);
    viewer.seek(&mut race, 200);
    let before = (race.camera_cuts(), race.viewed_camera_cuts());
    let restores = race.viewer_restores();
    viewer.seek(&mut race, 300);
    assert_ne!(race.viewer_restores(), restores, "every pose jumped");
    assert_ne!(race.camera_cuts(), before.0, "the player's view");
    assert_ne!(race.viewed_camera_cuts(), before.1, "the view being drawn");
    let after = race.camera_cuts();
    viewer.update(&mut race);
    assert_eq!(race.camera_cuts(), after, "playing on is not a cut");
    assert_eq!(race.viewer_restores(), restores.wrapping_add(1));
}

/// Paused, nothing moves; a toggle from the end plays from the start; the speeds step in
/// sixteenths of a tick a frame.
#[test]
fn pause_and_speed() {
    let (replay, _) = recording(TICKS);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    viewer.update(&mut race);
    viewer.toggle(&mut race);
    assert!(!viewer.playing());
    assert_eq!(viewer.update(&mut race), Moved::Still);
    viewer.toggle(&mut race);
    assert_eq!(viewer.update(&mut race), Moved::Stepped(1));
    // Half speed steps every other frame.
    while viewer.speed() != SPEEDS[0] {
        viewer.faster();
    }
    let steps: Vec<Moved> = (0..4).map(|_| viewer.update(&mut race)).collect();
    assert_eq!(
        steps,
        [
            Moved::Still,
            Moved::Stepped(1),
            Moved::Still,
            Moved::Stepped(1)
        ]
    );
    viewer.seek(&mut race, TICKS);
    assert_eq!(viewer.state(), State::Ended);
    // Playing but at the end, then paused there: playing again starts over.
    viewer.toggle(&mut race);
    assert!(!viewer.playing());
    assert_eq!(
        viewer.toggle(&mut race),
        Moved::Seeked,
        "from the end, play starts over"
    );
    assert_eq!(viewer.at(), 0);
}

/// One input changed late in the run: the first stored hash past it disagrees. With no pose track
/// the viewer plays the part that agrees, stops at its last second, and says what stopped it;
/// nothing past it can be sought to.
#[test]
fn a_recording_this_build_does_not_reproduce_stops_at_the_last_agreeing_second() {
    let mut replay = perturbed();
    replay.pose_track = None;
    let (mut viewer, mut race) = open(replay);
    let Pass::Desynced(desync) = pass_ended(&mut viewer) else {
        panic!("the perturbed input must desync");
    };
    assert_eq!(desync.tick, 360, "the first stored hash after tick 330");
    assert_eq!(viewer.mismatch().map(|d| d.tick), Some(360));
    assert_eq!(viewer.built(), Some(300), "the last agreeing hash");
    while viewer.update(&mut race) != Moved::Still {}
    assert_eq!(viewer.at(), 300);
    assert_eq!(viewer.state(), State::Stopped);
    viewer.seek(&mut race, 399);
    assert_eq!(viewer.at(), 300, "a seek is held inside what agrees");
}

fn perturbed() -> Replay {
    let (mut replay, _) = recording(TICKS);
    // The fixture's craft does not answer its stick, so the run is changed by what it is handed:
    // a weapon the recorded run never had, between ticks 329 and 330.
    replay.events.push(oag_replay::event::Event {
        tick: 330,
        seat: 0,
        kind: oag_replay::event::Kind::Give.code(),
        value: 0,
    });
    replay
}

/// With a pose track the playhead goes on past the last agreeing second in POSE VIEW: every
/// craft stands where it was recorded, to the end of the track, and a seek there lands on a pose.
#[test]
fn past_the_last_agreeing_second_a_pose_track_plays_in_pose_view() {
    let replay = perturbed();
    let track = replay
        .pose_track
        .clone()
        .expect("a recording carries its poses");
    let (mut viewer, mut race) = open(replay);
    assert!(matches!(pass_ended(&mut viewer), Pass::Desynced(_)));
    while viewer.update(&mut race) != Moved::Still {}
    assert_eq!(
        viewer.at(),
        TICKS,
        "the track runs to the end of the recording"
    );
    assert_eq!(viewer.state(), State::PoseEnded);
    assert!(!viewer.pose_track_is_short());
    let slots = usize::from(race.ship_count());
    for slot in 0..slots {
        let shown = race.craft_pose(slot);
        let recorded = track.pose(slot, TICKS as f64).expect("a pose");
        assert!(
            (shown.position - recorded.position).length() < 1e-3,
            "slot {slot} stands where it was recorded"
        );
    }
    // Back into what agrees, and out again by a seek.
    viewer.seek(&mut race, 100);
    assert_eq!(viewer.state(), State::Ready);
    viewer.seek(&mut race, 350);
    assert_eq!(viewer.at(), 350);
    assert_eq!(viewer.state(), State::PoseView);
    let shown = race.craft_pose(0);
    let recorded = track.pose(0, 350.0).expect("a pose");
    assert!((shown.position - recorded.position).length() < 1e-3);
}

/// While the first keyframe is not in, the viewer holds the grid and moves nothing.
#[test]
fn nothing_moves_until_the_first_keyframe_is_in() {
    let (replay, _) = recording(TICKS);
    let mut viewer = Box::new(Viewer::open(fixture(), replay).expect("the race it records"));
    let mut race = Box::new(Race::start(fixture()));
    viewer.begin(&mut race).expect("the launch settings");
    let before = format!("{:?}", race.sim);
    let mut held = 0;
    while viewer.opening() {
        assert_eq!(viewer.state(), State::Opening);
        assert_eq!(viewer.update(&mut race), Moved::Still);
        assert_eq!(format!("{:?}", race.sim), before);
        held += 1;
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    // Whether the pass beat the first poll is the machine's; either way it opens and plays.
    let _ = held;
    assert_eq!(viewer.update(&mut race), Moved::Seeked);
}

/// A race watched does not record: nothing it does can be written as a replay or a ghost.
#[test]
fn the_race_on_screen_does_not_record() {
    let (replay, _) = recording(TICKS);
    let viewer = Box::new(Viewer::open(fixture(), replay).expect("opens"));
    let mut race = Box::new(Race::start(fixture()));
    race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
    assert!(race.recording());
    viewer.begin(&mut race).expect("settings");
    assert!(!race.recording());
}
