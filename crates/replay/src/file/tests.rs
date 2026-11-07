//! Round trips and desyncs against a toy simulation.
//!
//! The toy is deliberately small but has the property that matters: its state
//! depends on every input it is handed and on a seeded generator, so a changed
//! input anywhere changes every hash after it. The disc-backed half - a real
//! race on a real circuit - is `crates/raceplay/tests/replay_ground_truth.rs`.

use oag_core::{Rng, StateHasher};
use oag_gameplay::input::{Button, Input};

use super::*;
use crate::event::{Event, Kind};
use crate::header::{Fork, GhostInfo, Setup};
use crate::pose::Pose;
use crate::{Recorder, Verifier};

/// A craft on a line: the stick moves it, cross spends a random boost.
struct Toy {
    rng: Rng,
    position: f32,
    /// What the stick is multiplied by: changed between ticks by a [`Kind::PilotAssist`] event,
    /// and not part of the hash, as Pilot Assist's level is not part of the race's.
    gain: f32,
}

impl Toy {
    fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            position: 0.0,
            gain: 1.0,
        }
    }

    fn apply(&mut self, event: &Event) {
        if event.kind() == Some(Kind::PilotAssist) {
            self.gain = event.value as f32;
        }
    }

    fn step(&mut self, inputs: &PlayerInputs) {
        let snapshot = inputs.get(0);
        self.position += snapshot.stick_x * self.gain;
        if snapshot.buttons.is_held(Button::Cross) {
            self.position += self.rng.next_f32();
        }
    }

    fn hash(&self) -> u64 {
        let mut hasher = StateHasher::new();
        hasher.write_f32(self.position);
        for word in self.rng.snapshot() {
            hasher.write_u32(word);
        }
        hasher.finish()
    }
}

fn scripted(ticks: u32) -> Vec<PlayerInputs> {
    let mut input = Input::new();
    (0..ticks)
        .map(|tick| {
            input.begin_frame(if tick % 90 < 50 {
                Button::Cross.bit()
            } else {
                0
            });
            let snapshot = InputSnapshot {
                buttons: input,
                stick_x: ((tick / 30) % 3) as f32 * 0.5 - 0.5,
                ..InputSnapshot::EMPTY
            };
            PlayerInputs::single(snapshot.sanitised())
        })
        .collect()
}

fn record(ticks: u32) -> Replay {
    let mut toy = Toy::new(7);
    let header = Header::new("toy", "line", "time_trial", "venom", "none", 7);
    let mut recorder = Recorder::new(header, toy.hash());
    for inputs in scripted(ticks) {
        toy.step(&inputs);
        recorder.record(&inputs, || toy.hash());
    }
    recorder.finish()
}

fn replay_hashes(replay: &Replay) -> Result<u64, crate::Desync> {
    let mut toy = Toy::new(replay.header.seed);
    let initial = toy.hash();
    Verifier::run(replay, initial, |events, inputs| {
        for event in events {
            toy.apply(event);
        }
        toy.step(inputs);
        toy.hash()
    })
}

/// A recording in which the gain is changed from 1 to 2 before tick 500 and back before 700.
fn record_with_events(ticks: u32) -> Replay {
    let mut toy = Toy::new(7);
    let header = Header::new("toy", "line", "time_trial", "venom", "none", 7);
    let mut recorder = Recorder::new(header, toy.hash());
    for (tick, inputs) in scripted(ticks).into_iter().enumerate() {
        for (at, value) in [(500, 2), (700, 1)] {
            if tick == at {
                let event = Event::new(recorder.ticks(), 0, Kind::PilotAssist, value);
                toy.apply(&event);
                recorder.event(0, Kind::PilotAssist, value);
            }
        }
        toy.step(&inputs);
        recorder.record(&inputs, || toy.hash());
    }
    recorder.finish()
}

#[test]
fn a_recording_round_trips_through_its_bytes() {
    let mut replay = record(1_000);
    replay.header.ghost = Some(GhostInfo {
        lap: 2,
        start_tick: 10,
        lap_ticks: 3,
    });
    replay.ghost = Some(GhostLap {
        lap_ticks: 3,
        poses: vec![
            Pose {
                position: oag_core::math::Vec3::new(1.0, 2.0, 3.0),
                rotation: oag_core::math::Quat::IDENTITY,
            };
            4
        ],
    });
    replay
        .header
        .options
        .insert("scheme".into(), "veteran".into());
    let bytes = replay.to_bytes();
    let read = Replay::from_bytes(&bytes).expect("reads back");
    assert_eq!(read, replay);
    assert_eq!(read.hashes.len(), 16, "one a second over 1,000 ticks");
}

#[test]
fn an_untouched_recording_replays_to_the_last_tick() {
    let replay = Replay::from_bytes(&record(1_000).to_bytes()).expect("reads back");
    assert_eq!(replay_hashes(&replay), Ok(1_000));
}

/// The desync test: one changed input, and the next stored hash catches it.
#[test]
fn a_tampered_input_is_caught_at_the_next_stored_hash() {
    let mut replay = record(1_000);
    replay.inputs[0][500].stick_x = 0.75;
    let desync = replay_hashes(&replay).expect_err("a changed input must desync");
    assert_eq!(
        desync.tick, 540,
        "the first hash after tick 500 is tick 540"
    );
    assert_ne!(desync.expected, desync.found);
}

#[test]
fn a_race_built_differently_is_refused_before_the_first_tick() {
    let replay = record(120);
    let other = Toy::new(8);
    let desync = Verifier::new(&replay, other.hash()).expect_err("another seed");
    assert_eq!(desync.tick, 0);
}

#[test]
fn a_damaged_or_cut_short_file_is_refused() {
    let bytes = record(600).to_bytes();
    assert!(matches!(
        Replay::from_bytes(&bytes[..bytes.len() - 20]),
        Err(ReadError::Truncated(_) | ReadError::Checksum)
    ));
    // Flip one bit in the middle of the input stream: every chunk may still
    // parse, and the checksum is what tells.
    let mut flipped = bytes.clone();
    let middle = flipped.len() / 2;
    flipped[middle] ^= 0x01;
    assert!(Replay::from_bytes(&flipped).is_err());
    assert!(matches!(
        Replay::from_bytes(b"RIFF...."),
        Err(ReadError::BadMagic)
    ));
    let mut future = bytes;
    future[4] = 9;
    assert!(matches!(
        Replay::from_bytes(&future),
        Err(ReadError::Version(9))
    ));
}

#[test]
fn a_chunk_this_version_does_not_know_is_skipped() {
    let replay = record(120);
    let bytes = replay.to_bytes();
    // Rebuild the file with an extra chunk before the checksum.
    let end = bytes.len() - (4 + 4 + 8);
    let mut extended = bytes[..end].to_vec();
    chunk(&mut extended, *b"XTRA", b"from the future");
    let sum = checksum(&extended);
    chunk(&mut extended, *b"END_", &sum.to_le_bytes());
    assert_eq!(Replay::from_bytes(&extended).expect("still reads"), replay);
}

#[test]
fn truncating_keeps_only_the_hashes_of_ticks_still_recorded() {
    let mut replay = record(1_000);
    replay.truncate(610);
    assert_eq!(replay.ticks(), 610);
    assert_eq!(replay.hashes.len(), 10);
    assert_eq!(replay_hashes(&replay), Ok(610));
}

#[test]
fn an_event_applied_between_the_same_two_ticks_reproduces_and_one_left_out_desyncs() {
    let replay = Replay::from_bytes(&record_with_events(1_000).to_bytes()).expect("reads back");
    assert_eq!(
        replay.events,
        vec![
            Event::new(500, 0, Kind::PilotAssist, 2),
            Event::new(700, 0, Kind::PilotAssist, 1)
        ]
    );
    assert_eq!(replay_hashes(&replay), Ok(1_000));

    let mut without = replay.clone();
    without.events.clear();
    let desync = replay_hashes(&without).expect_err("the event changed the run");
    assert_eq!(
        desync.tick, 540,
        "the first hash after the change at tick 500"
    );

    // The stick is centred over ticks 480 to 509 and not at 479, so only an event moved onto a
    // tick with a deflection is another run.
    let mut early = replay;
    early.events[0].tick = 479;
    assert!(
        replay_hashes(&early).is_err(),
        "applied before another tick it is another run"
    );
}

#[test]
fn events_at_a_tick_are_the_ones_recorded_before_it() {
    let replay = record_with_events(1_000);
    assert!(replay.events_at(499).is_empty());
    assert_eq!(replay.events_at(500).len(), 1);
    assert!(replay.events_at(501).is_empty());
}

#[test]
fn a_cut_file_keeps_the_events_of_the_ticks_it_keeps() {
    let mut replay = record_with_events(1_000);
    replay.truncate(600);
    assert_eq!(
        replay.events,
        vec![Event::new(500, 0, Kind::PilotAssist, 2)]
    );
    assert_eq!(replay_hashes(&replay), Ok(600));
}

#[test]
fn an_event_of_a_kind_this_build_does_not_know_is_kept_and_not_understood() {
    let mut replay = record(120);
    replay.events.push(Event {
        tick: 10,
        seat: 0,
        kind: 3,
        value: 9,
    });
    let read = Replay::from_bytes(&replay.to_bytes()).expect("reads");
    assert_eq!(read, replay);
    assert_eq!(
        read.events[0].kind(),
        None,
        "controller is reserved, not implemented"
    );
}

#[test]
fn a_replay_with_no_events_writes_no_events_chunk() {
    let bytes = record(120).to_bytes();
    assert!(!bytes.windows(4).any(|w| w == b"EVNT"));
}

fn head_text(bytes: &[u8]) -> String {
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    String::from_utf8(bytes[16..16 + len].to_vec()).unwrap()
}

/// The same recording with `HEAD` rewritten by `edit`, the checksum redone.
fn edit_head(replay: &Replay, edit: impl FnOnce(&str) -> String) -> Vec<u8> {
    let bytes = replay.to_bytes();
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let text = edit(&head_text(&bytes));
    let mut out = bytes[..8].to_vec();
    chunk(&mut out, *b"HEAD", text.as_bytes());
    let end = bytes.len() - (4 + 4 + 8);
    out.extend_from_slice(&bytes[16 + len..end]);
    let sum = checksum(&out);
    chunk(&mut out, *b"END_", &sum.to_le_bytes());
    out
}

#[test]
fn a_header_starts_with_its_schema_line_and_the_read_says_it_was_there() {
    let replay = record(120);
    let bytes = replay.to_bytes();
    let first = head_text(&bytes).lines().next().unwrap().to_string();
    assert_eq!(first, oag_schema::inline("replay-header", 1));
    let read = Replay::read(&bytes).expect("reads");
    assert!(read.schema_header);
    assert_eq!(read.replay, replay);
}

/// A file from before the line existed: the header text a build without the line wrote.
#[test]
fn an_old_file_without_the_schema_line_loads_and_reports_its_absence() {
    let replay = record(120);
    let old = edit_head(&replay, |text| {
        text.lines().skip(1).collect::<Vec<_>>().join("\n")
    });
    let read = Replay::read(&old).expect("an old file still loads");
    assert!(!read.schema_header);
    assert_eq!(read.replay, replay);
    assert_eq!(Replay::from_bytes(&old).expect("loads"), replay);
}

#[test]
fn a_header_naming_another_kind_or_a_later_major_is_refused_by_name() {
    let replay = record(120);
    let other = edit_head(&replay, |t| {
        t.replace("/v1/replay-header.tosd", "/v1/pilot.tosd")
    });
    match Replay::from_bytes(&other) {
        Err(ReadError::Schema(oag_schema::Error::WrongKind { found, expected })) => {
            assert_eq!(
                (found.as_str(), expected.as_str()),
                ("pilot", "replay-header")
            );
        }
        other => panic!("{other:?}"),
    }
    let later = edit_head(&replay, |t| {
        t.replace("/v1/replay-header.tosd", "/v2/replay-header.tosd")
    });
    match Replay::from_bytes(&later) {
        Err(ReadError::Schema(oag_schema::Error::UnknownMajor {
            found: 2,
            supported: 1,
            ..
        })) => {}
        other => panic!("{other:?}"),
    }
    let text = Replay::from_bytes(&later).unwrap_err().to_string();
    assert!(
        text.contains("v2") && text.contains("replay-header"),
        "{text}"
    );
}

#[test]
fn the_setup_and_fork_tables_round_trip_and_an_absent_one_reads_as_none() {
    let mut replay = record(120);
    replay.header.setup = Some(Setup {
        mode: "single_race".into(),
        class: "Venom".into(),
        opponent_teams: vec!["a".into(), "b".into()],
        opponents: true,
        laps_override: Some(2),
        ..Setup::default()
    });
    replay.header.fork = Some(Fork {
        tick: 90,
        offered_at: 80,
        seats: vec![0],
        parent_build: "x".into(),
        parent_hash: "0x00000000000000ff".into(),
    });
    assert_eq!(Replay::from_bytes(&replay.to_bytes()).unwrap(), replay);
    assert_eq!(record(120).header.setup, None);
    // A fork written with only some keys reads: the rest are defaults.
    let partial = edit_head(&record(120), |t| format!("{t}\n[fork]\ntick = 5\n"));
    assert_eq!(
        Replay::from_bytes(&partial)
            .unwrap()
            .header
            .fork
            .unwrap()
            .tick,
        5
    );
}
