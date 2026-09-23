//! Round trips and desyncs against a toy simulation.
//!
//! The toy is deliberately small but has the property that matters: its state
//! depends on every input it is handed and on a seeded generator, so a changed
//! input anywhere changes every hash after it. The disc-backed half - a real
//! race on a real circuit - is `crates/game/tests/replay_ground_truth.rs`.

use oag_core::{Rng, StateHasher};
use oag_gameplay::input::{Button, Input};

use super::*;
use crate::header::GhostInfo;
use crate::pose::Pose;
use crate::{Recorder, Verifier};

/// A craft on a line: the stick moves it, cross spends a random boost.
struct Toy {
    rng: Rng,
    position: f32,
}

impl Toy {
    fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            position: 0.0,
        }
    }

    fn step(&mut self, inputs: &PlayerInputs) {
        let snapshot = inputs.get(0);
        self.position += snapshot.stick_x;
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
    Verifier::run(replay, initial, |inputs| {
        toy.step(inputs);
        toy.hash()
    })
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
