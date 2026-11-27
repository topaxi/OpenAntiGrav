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
    let mut race = Race::start(
        fixture(),
        crate::tests::fixture_view(),
        crate::extensions::Prepared::default(),
    );
    race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
    let mut text = vec![format!("{:?}", race.core.sim)];
    for tick in 0..ticks {
        race.tick(&inputs(tick));
        race.core.sim.drain_cues();
        text.push(format!("{:?}", race.core.sim));
    }
    (race.recorded().expect("recording"), text)
}

/// A viewer over the fixture and a race not yet handed to it.
fn unbegun(replay: Replay) -> (Box<Viewer>, Box<Race>) {
    let viewer = Box::new(
        Viewer::open(
            fixture(),
            crate::tests::fixture_view(),
            crate::extensions::Prepared::default(),
            replay,
        )
        .expect("the race it records"),
    );
    (
        viewer,
        Box::new(Race::start(
            fixture(),
            crate::tests::fixture_view(),
            crate::extensions::Prepared::default(),
        )),
    )
}

/// A viewer over the fixture and the race it draws, handed over as the session hands it.
fn open(replay: Replay) -> (Box<Viewer>, Box<Race>) {
    let (mut viewer, mut race) = unbegun(replay);
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
    let now = format!("{:?}", race.core.sim);
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

/// Fails if a viewer asked to open on a tick plays from 0, opens before the pass has built that
/// far, leaves play running, or lands anywhere but the recorded run at that tick.
#[test]
fn a_viewer_started_at_a_tick_opens_paused_there() {
    let (replay, text) = recording(TICKS);
    let start = 250;
    let (viewer, mut race) = unbegun(replay);
    let mut viewer = Box::new((*viewer).starting_at(start));
    viewer.begin(&mut race).expect("the launch settings");
    assert_eq!(
        viewer.state(),
        State::Opening,
        "nothing is shown before the start is built"
    );
    let mut moved = Moved::Still;
    while moved == Moved::Still {
        moved = viewer.update(&mut race);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(moved, Moved::Seeked);
    assert_eq!(viewer.at(), start);
    assert!(!viewer.playing(), "it opens paused");
    assert_stands_at(&race, &text, start);
    assert_eq!(viewer.update(&mut race), Moved::Still, "and stays there");
    assert_eq!(viewer.at(), start);
}

/// Fails if a recording cut at the start tick does not open on its last tick, or a play from
/// there does not start it over.
#[test]
fn a_viewer_started_at_the_end_is_ended_and_plays_from_the_start() {
    let (replay, text) = recording(TICKS);
    let (viewer, mut race) = unbegun(replay);
    let mut viewer = Box::new((*viewer).starting_at(TICKS));
    viewer.begin(&mut race).expect("the launch settings");
    // A play pressed while the pass is still building is not left running.
    viewer.set_playing(true);
    while viewer.update(&mut race) == Moved::Still {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(viewer.at(), TICKS);
    assert_eq!(viewer.state(), State::Ended);
    assert!(!viewer.playing());
    assert_stands_at(&race, &text, TICKS);
    assert_eq!(viewer.toggle(&mut race), Moved::Seeked);
    assert_eq!(viewer.at(), 0);
    assert!(viewer.playing());
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

/// The clocks a HUD reads at tick `k` of the recorded run, at index `k`: `(lap_ticks, race_ticks)`.
fn clocks(ticks: u64) -> Vec<(u64, u64)> {
    let mut race = Race::start(
        fixture(),
        crate::tests::fixture_view(),
        crate::extensions::Prepared::default(),
    );
    let at = |race: &Race| {
        let readout = race.readout();
        (readout.lap_ticks, readout.race_ticks)
    };
    let mut clocks = vec![at(&race)];
    for tick in 0..ticks {
        race.tick(&inputs(tick));
        race.core.sim.drain_cues();
        clocks.push(at(&race));
    }
    clocks
}

/// What the viewer's HUD shows for the lap and the race at a tick is what the live race showed
/// at that tick, forwards and back, so the viewer's `current` is the lap clock (not the
/// recording's own clock, which counts the start-line countdown too). Fails if a seek leaves a
/// clock of the view side from another moment.
#[test]
fn the_hud_clocks_after_a_seek_are_the_live_races_at_that_tick() {
    let (replay, _) = recording(TICKS);
    let live = clocks(TICKS);
    assert!(
        live[TICKS as usize].1 > 0 && live[TICKS as usize].1 < TICKS,
        "the race clock starts after the countdown: {:?}",
        live[TICKS as usize]
    );
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    let mut targets: Vec<u64> = (0..=TICKS).step_by(13).collect();
    targets.sort_by_key(|tick| (tick * 97) % 397);
    for target in targets {
        viewer.seek(&mut race, target);
        let readout = race.readout();
        assert_eq!(
            (readout.lap_ticks, readout.race_ticks),
            live[target as usize],
            "the HUD's clocks after a seek to tick {target}"
        );
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
    race.core.outcome.results = Some(crate::scoreboard::Board {
        rows: Vec::new(),
        laps_target: None,
        tick: TICKS,
    });
    race.core.outcome.wreck_ended_tick = Some(TICKS);
    viewer.seek(&mut race, 100);
    assert!(race.core.results().is_none());
    assert!(race.core.outcome.wreck_ended_tick.is_none());
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
    let before = (race.view.camera_cuts(&race.core), race.viewed_camera_cuts());
    let restores = race.viewer_restores();
    viewer.seek(&mut race, 300);
    assert_ne!(race.viewer_restores(), restores, "every pose jumped");
    assert_ne!(
        race.view.camera_cuts(&race.core),
        before.0,
        "the player's view"
    );
    assert_ne!(race.viewed_camera_cuts(), before.1, "the view being drawn");
    let after = race.view.camera_cuts(&race.core);
    viewer.update(&mut race);
    assert_eq!(
        race.view.camera_cuts(&race.core),
        after,
        "playing on is not a cut"
    );
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
    let slots = usize::from(race.core.sim.ship_count());
    for slot in 0..slots {
        let shown = race.core.sim.craft_pose(slot);
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
    let shown = race.core.sim.craft_pose(0);
    let recorded = track.pose(0, 350.0).expect("a pose");
    assert!((shown.position - recorded.position).length() < 1e-3);
}

/// The file as it is after a build changed the tick-zero hash: read back from bytes sealed with
/// a hash this build's race does not have, so the file's own checksum is redone.
fn rehashed(mut replay: Replay) -> Replay {
    replay.initial_hash ^= 1;
    Replay::from_bytes(&replay.to_bytes()).expect("a sealed file")
}

/// A recording whose tick-zero hash no longer matches opens in POSE VIEW from its first tick when
/// it carries a pose track: no tick is simulated, a seek lands on the recorded pose, and nothing
/// of it is state a take-over could start from.
#[test]
fn a_tick_zero_mismatch_with_a_pose_track_opens_in_pose_view() {
    let replay = rehashed(recording(TICKS).0);
    let track = replay.pose_track.clone().expect("poses");
    let (mut viewer, mut race) = open(replay);
    assert_eq!(viewer.built(), Some(0), "no tick agrees");
    assert_eq!(viewer.mismatch().map(|d| d.tick), Some(0));
    assert!(viewer.stops_matching(), "the build-mismatch notice shows");
    assert_eq!(viewer.state(), State::PoseView);
    assert_eq!(viewer.update(&mut race), Moved::Seeked);
    for _ in 0..30 {
        assert_eq!(viewer.update(&mut race), Moved::Stepped(1));
    }
    assert_eq!(viewer.at(), 30);
    assert!(viewer.in_pose_view());
    assert_eq!(race.core.sim.world.tick, 0, "no tick of the simulation ran");
    viewer.seek(&mut race, 250);
    assert_eq!(viewer.at(), 250);
    assert_eq!(viewer.state(), State::PoseView);
    for slot in 0..usize::from(race.core.sim.ship_count()) {
        let recorded = track.pose(slot, 250.0).expect("a pose");
        assert!((race.core.sim.craft_pose(slot).position - recorded.position).length() < 1e-3);
    }
    viewer.seek(&mut race, TICKS);
    assert_eq!(viewer.state(), State::PoseEnded);
}

/// Without a pose track the file is refused, naming it; a header that no longer loads is refused
/// as before, whatever the file carries.
#[test]
fn a_tick_zero_mismatch_without_a_pose_track_is_refused_by_name() {
    let mut replay = rehashed(recording(TICKS).0);
    replay.pose_track = None;
    let Err(refusal) = Viewer::open(
        fixture(),
        crate::tests::fixture_view(),
        crate::extensions::Prepared::default(),
        replay,
    ) else {
        panic!("nothing to play instead");
    };
    assert!(matches!(
        refusal,
        crate::timeline::OpenError::NoPoseTrack(_)
    ));
    assert!(refusal.to_string().contains("no pose track"), "{refusal}");

    let mut replay = rehashed(recording(TICKS).0);
    replay.header.mods.push(oag_replay::header::ModUse {
        id: "absent".to_string(),
        ..Default::default()
    });
    let Err(refusal) = Viewer::open(
        fixture(),
        crate::tests::fixture_view(),
        crate::extensions::Prepared::default(),
        replay,
    ) else {
        panic!("a header this build cannot load");
    };
    assert!(matches!(refusal, crate::timeline::OpenError::Start(_)));
}

/// While the first keyframe is not in, the viewer holds the grid and moves nothing.
#[test]
fn nothing_moves_until_the_first_keyframe_is_in() {
    let (replay, _) = recording(TICKS);
    let (mut viewer, mut race) = unbegun(replay);
    viewer.begin(&mut race).expect("the launch settings");
    let before = format!("{:?}", race.core.sim);
    let mut held = 0;
    while viewer.opening() {
        assert_eq!(viewer.state(), State::Opening);
        assert_eq!(viewer.update(&mut race), Moved::Still);
        assert_eq!(format!("{:?}", race.core.sim), before);
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
    let (viewer, mut race) = unbegun(replay);
    race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
    assert!(race.recording());
    viewer.begin(&mut race).expect("settings");
    assert!(!race.recording());
}

/// L1 and R1 in the viewer follow the craft before or after the one on screen, wrapping; a
/// craft out of the race is skipped; and following another craft is the picture's and nothing
/// the simulation reads: the race on screen stays on the recorded run at every tick of a play
/// with the followed craft changed throughout.
#[test]
fn following_another_craft_moves_the_picture_and_not_one_tick_of_the_run() {
    let (replay, text) = recording(TICKS);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    viewer.update(&mut race);
    let start = race.core.player_slot();
    let count = usize::from(race.core.sim.world.ship_count);
    assert!(count > 1, "the fixture field");
    assert!(race.step_followed(true));
    assert_eq!(
        race.core.player_slot(),
        (start + 1) % count,
        "R1 follows the next craft"
    );
    assert!(race.step_followed(false));
    assert_eq!(race.core.player_slot(), start, "and L1 back");
    assert!(race.step_followed(false));
    assert_eq!(
        race.core.player_slot(),
        (start + count - 1) % count,
        "L1 wraps over the start"
    );
    for tick in 1..=TICKS {
        if tick % 37 == 0 {
            race.step_followed(tick % 2 == 0);
        }
        assert_eq!(viewer.update(&mut race), Moved::Stepped(1));
        assert_stands_at(&race, &text, tick);
    }
}

/// A craft that has left the race is passed over, and the first one still in it is where a
/// follow of a craft that is gone lands.
#[test]
fn following_skips_a_craft_out_of_the_race() {
    let (replay, _) = recording(20);
    let (mut viewer, mut race) = open(replay);
    assert_eq!(pass_ended(&mut viewer), Pass::Complete);
    viewer.update(&mut race);
    let start = race.core.player_slot();
    let next = (start + 1) % usize::from(race.core.sim.world.ship_count);
    race.core.sim.world.ships[next].physics.craft_state = oag_physics::CraftState::Eliminated;
    race.step_followed(true);
    assert_ne!(
        race.core.player_slot(),
        next,
        "an eliminated craft is skipped"
    );
    assert_ne!(race.core.player_slot(), start);
}

/// A recording lists the mods its race was set up with, and a race with none lists none.
#[test]
fn a_recording_lists_the_mods_its_race_was_set_up_with() {
    for on in [
        Vec::new(),
        vec![oag_replay::ActiveMod::new("a.data", "1.0.0")],
    ] {
        let view = crate::ViewSetup {
            active_mods: on.clone(),
            ..crate::tests::fixture_view()
        };
        let mut race = Race::start(fixture(), view, crate::extensions::Prepared::default());
        race.start_recording(Header::new("fixture", "t", "m", "c", "d", 0));
        let replay = race.recorded().expect("recording");
        assert_eq!(replay.header.active_mods, on);
    }
}
