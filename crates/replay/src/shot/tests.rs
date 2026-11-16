use oag_gameplay::input::Button;
use oag_gameplay::{InputSnapshot, PlayerInputs};

use super::*;
use crate::{Header, Recorder};

fn run(ticks: u32) -> Replay {
    let mut recorder = Recorder::new(
        Header::new("pulse", "t", "time_trial", "venom", "d", u64::MAX),
        7,
    );
    for tick in 0..ticks {
        let mut snapshot = InputSnapshot {
            stick_x: (tick as f32 * 0.37).sin(),
            ..InputSnapshot::default()
        };
        if tick % 5 < 3 {
            snapshot.buttons.begin_frame(Button::Cross.bit());
        }
        recorder.record(&PlayerInputs::single(snapshot), || u64::from(tick));
    }
    recorder.finish()
}

#[test]
fn a_record_round_trips_byte_for_byte() {
    let record = ShotRecord::new(run(500), 321, 0xDEAD_BEEF_0123_4567);
    assert_eq!(record.replay.ticks(), 321, "cut at the shot");
    let bytes = record.to_bytes();
    let read = ShotRecord::from_bytes(&bytes).expect("reads");
    assert_eq!(read, record);
    assert_eq!(read.to_bytes(), bytes, "the chunk is reproduced exactly");
    assert_eq!(read.replay.header.seed, u64::MAX);
    assert_eq!(read.replay.inputs, record.replay.inputs);
}

#[test]
fn a_record_is_smaller_than_the_stream_it_holds() {
    let record = ShotRecord::new(run(3_000), 3_000, 1);
    assert!(record.to_bytes().len() < record.replay.to_bytes().len());
}

#[test]
fn a_damaged_or_foreign_chunk_is_refused() {
    let bytes = ShotRecord::new(run(100), 100, 1).to_bytes();
    assert!(matches!(ShotRecord::from_bytes(&[]), Err(ShotError::Frame)));
    let mut other = bytes.clone();
    other[0] = 9;
    assert!(matches!(
        ShotRecord::from_bytes(&other),
        Err(ShotError::Frame)
    ));
    assert!(ShotRecord::from_bytes(&bytes[..bytes.len() / 2]).is_err());
}

#[test]
fn a_tick_past_the_stream_is_refused() {
    let mut record = ShotRecord::new(run(100), 100, 1);
    record.tick = 101;
    assert!(matches!(
        ShotRecord::from_bytes(&record.to_bytes()),
        Err(ShotError::Short {
            tick: 101,
            held: 100
        })
    ));
}
