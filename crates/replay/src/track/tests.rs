use super::*;
use oag_core::Rng;

/// A craft flying a winding line at about 5 units a tick, with a banked, wobbling hull, a hard
/// kick at every wall hit in `kicks`, and a jump of `jump` units at tick `jump_at`.
fn flight(seed: u64, ticks: u32, kicks: &[u32], jump_at: Option<(u32, Vec3)>) -> Vec<Pose> {
    let mut rng = Rng::new(seed);
    let mut position = Vec3::new(seed as f32 * 40.0, 0.0, 0.0);
    let mut heading = 0.0f32;
    let mut turn = 0.0f32;
    let mut out = Vec::new();
    for tick in 0..=ticks {
        turn = turn * 0.97 + (rng.next_f32() - 0.5) * 0.004;
        heading += turn;
        let mut step = Vec3::new(
            heading.sin(),
            0.02 * (tick as f32 * 0.05).sin(),
            -heading.cos(),
        ) * 5.0;
        if kicks.contains(&tick) {
            step += Vec3::new(2.5, 0.0, 1.5);
            heading += 0.3;
        }
        position += step;
        if let Some((at, by)) = jump_at
            && at == tick
        {
            position += by;
            heading += 1.0;
        }
        let roll = (tick as f32 * 0.02).sin() * 0.3 + turn * 20.0;
        out.push(Pose {
            position,
            rotation: Quat::from_rotation_y(-heading) * Quat::from_rotation_z(roll),
        });
    }
    out
}

fn grid(crafts: u64, ticks: u32) -> Vec<Vec<Pose>> {
    (0..crafts)
        .map(|seed| flight(seed, ticks, &[100 + seed as u32 * 7, 333], None))
        .collect()
}

fn record(truth: &[Vec<Pose>]) -> PoseTrack {
    let mut recorder = PoseRecorder::new(truth.len());
    for tick in 0..truth[0].len() {
        let poses: Vec<Pose> = truth.iter().map(|craft| craft[tick]).collect();
        assert!(recorder.push(&poses));
    }
    recorder.track().expect("pushed")
}

/// The reference: the whole race at once, refinement looping over every segment of a craft
/// until none is out of bounds, as the cost sweep of ADR-0087 did, against quantised keys.
fn offline(truth: &[Vec<Pose>]) -> PoseTrack {
    let last = (truth[0].len() - 1) as u32;
    let crafts = truth
        .iter()
        .map(|craft| {
            let mut ticks: std::collections::BTreeSet<u32> = (0..=last).step_by(6).collect();
            ticks.insert(last);
            loop {
                let keys: Vec<Key> = ticks
                    .iter()
                    .map(|&tick| Key::of(tick, &craft[tick as usize]))
                    .collect();
                let mut added = false;
                for pair in keys.windows(2) {
                    let (a, b) = (pair[0].pose(), pair[1].pose());
                    let worst = (pair[0].tick + 1..pair[1].tick)
                        .map(|tick| {
                            let fraction =
                                (tick - pair[0].tick) as f32 / (pair[1].tick - pair[0].tick) as f32;
                            (tick, over(&a, &b, fraction, &craft[tick as usize]))
                        })
                        .max_by(|x, y| x.1.total_cmp(&y.1));
                    if let Some((tick, excess)) = worst
                        && excess > 1.0
                    {
                        ticks.insert(tick);
                        added = true;
                    }
                }
                if !added {
                    return keys;
                }
            }
        })
        .collect();
    PoseTrack { last, crafts }
}

fn worst_error(track: &PoseTrack, truth: &[Vec<Pose>]) -> (f32, f32) {
    let mut worst = (0.0f32, 0.0f32);
    for (craft, poses) in truth.iter().enumerate() {
        for (tick, pose) in poses.iter().enumerate() {
            let got = track.pose(craft, tick as f64).expect("a craft");
            worst.0 = worst.0.max((got.position - pose.position).length());
            worst.1 = worst.1.max(degrees_between(got.rotation, pose.rotation));
        }
    }
    worst
}

#[test]
fn streaming_equals_offline() {
    for ticks in [0, 1, 5, 6, 7, 11, 12, 13, 599, 600, 1_201] {
        let truth = grid(3, ticks);
        let streamed = record(&truth);
        let reference = offline(&truth);
        assert_eq!(streamed, reference, "{ticks} ticks");
        assert_eq!(
            streamed.encode(),
            reference.encode(),
            "{ticks} ticks, bytes"
        );
    }
}

#[test]
fn every_recorded_tick_is_within_the_tolerance_of_the_truth() {
    let truth = grid(8, 3_000);
    let track = record(&truth);
    let (position, rotation) = worst_error(&track, &truth);
    assert!(position <= MAX_POSITION_ERROR + 1e-4, "{position} units");
    assert!(rotation <= MAX_ROTATION_ERROR + 1e-3, "{rotation} degrees");
    let refined: usize = (0..8).map(|craft| track.keys(craft)).sum();
    let base = 8 * (3_000 / 6 + 1);
    assert!(refined > base, "the wall kicks must have added keys");
}

#[test]
fn a_track_round_trips_through_its_bytes() {
    for ticks in [0, 1, 5, 6, 7, 13, 600, 1_201] {
        let track = record(&grid(4, ticks));
        assert_eq!(
            PoseTrack::decode(&track.encode()),
            Ok(track),
            "{ticks} ticks"
        );
    }
    let empty = PoseRecorder::new(0);
    assert!(empty.track().is_none());
}

#[test]
fn a_zero_craft_grid_is_a_valid_track() {
    let mut recorder = PoseRecorder::new(0);
    for _ in 0..20 {
        assert!(recorder.push(&[]));
    }
    let track = recorder.track().expect("ticks were pushed");
    assert_eq!((track.crafts(), track.last_tick()), (0, 19));
    assert_eq!(PoseTrack::decode(&track.encode()), Ok(track));
}

#[test]
fn a_copy_taken_mid_segment_is_the_track_to_that_tick_and_the_recorder_goes_on() {
    let truth = grid(2, 500);
    let mut recorder = PoseRecorder::new(2);
    let mut copies = Vec::new();
    for tick in 0..truth[0].len() {
        let poses: Vec<Pose> = truth.iter().map(|craft| craft[tick]).collect();
        recorder.push(&poses);
        if [3, 4, 6, 7, 250].contains(&tick) {
            copies.push((tick, recorder.track().expect("pushed")));
        }
    }
    for (tick, copy) in copies {
        let cut: Vec<Vec<Pose>> = truth.iter().map(|c| c[..=tick].to_vec()).collect();
        assert_eq!(copy, record(&cut), "a copy at tick {tick}");
    }
    assert_eq!(
        recorder.track(),
        Some(record(&truth)),
        "the copies changed nothing"
    );
}

#[test]
fn nothing_is_taken_past_the_ceiling() {
    let mut recorder = PoseRecorder::new(1);
    let pose = Pose {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
    };
    for _ in 0..=MAX_TICKS {
        assert!(recorder.push(&[pose]));
    }
    assert!(
        !recorder.push(&[pose]),
        "tick {} is one too many",
        MAX_TICKS + 1
    );
    assert_eq!(recorder.last_tick(), Some(u64::from(MAX_TICKS)));
    assert_eq!(
        recorder.track().expect("pushed").keys(0),
        (MAX_TICKS / 6 + 1) as usize
    );
}

#[test]
fn a_teleport_is_bounded_at_every_tick_and_not_smeared_between_two() {
    let jump_at = 400;
    let by = Vec3::new(300.0, 12.0, -250.0);
    let truth = vec![flight(1, 800, &[], Some((jump_at, by)))];
    let track = record(&truth);
    let (position, rotation) = worst_error(&track, &truth);
    assert!(position <= MAX_POSITION_ERROR + 1e-4, "{position} units");
    assert!(rotation <= MAX_ROTATION_ERROR + 1e-3, "{rotation} degrees");
    // The ticks either side of the jump are both keys: no interpolation fits across it.
    let ticks: Vec<u32> = track.crafts[0].iter().map(|key| key.tick).collect();
    assert!(
        ticks.contains(&(jump_at - 1)) && ticks.contains(&jump_at),
        "{ticks:?}"
    );
    // A frame between them shows the craft where it was, then where it is: never a hull
    // halfway along the jump.
    let before = track.pose(0, f64::from(jump_at - 1)).expect("a craft");
    let after = track.pose(0, f64::from(jump_at)).expect("a craft");
    assert!((after.position - before.position).length() > 250.0);
    for fraction in [0.1, 0.5, 0.99] {
        let between = track
            .pose(0, f64::from(jump_at - 1) + fraction)
            .expect("a craft");
        assert_eq!(between, before, "at +{fraction}");
    }
}

#[test]
fn a_step_that_is_motion_is_interpolated_across_a_single_tick() {
    let pose = |x: f32| Pose {
        position: Vec3::new(x, 0.0, 0.0),
        rotation: Quat::IDENTITY,
    };
    let mut recorder = PoseRecorder::new(1);
    for x in [0.0, 5.0, 10.0, 15.0] {
        recorder.push(&[pose(x)]);
    }
    let track = recorder.track().expect("pushed");
    let half = track.pose(0, 1.5).expect("a craft");
    assert!((half.position.x - 7.5).abs() < 1e-3, "{}", half.position.x);
}

#[test]
fn sampling_outside_the_track_holds_its_ends() {
    let truth = grid(1, 20);
    let track = record(&truth);
    assert_eq!(track.pose(0, -3.0), track.pose(0, 0.0));
    assert_eq!(track.pose(0, 99.0), track.pose(0, 20.0));
    assert!(track.pose(1, 0.0).is_none(), "no second craft");
}

#[test]
fn cutting_a_track_keeps_what_was_before_the_cut() {
    let truth = grid(2, 900);
    let track = record(&truth);
    for cut in [0u64, 1, 5, 6, 449, 450, 900, 5_000] {
        let short = track.truncated(cut);
        assert_eq!(short.last_tick(), cut.min(900));
        assert_eq!(
            PoseTrack::decode(&short.encode()),
            Ok(short.clone()),
            "cut at {cut}"
        );
        for craft in 0..2 {
            for tick in 0..=short.last_tick() {
                let a = short.pose(craft, tick as f64).expect("a craft");
                let b = track.pose(craft, tick as f64).expect("a craft");
                let position = (a.position - b.position).length();
                assert!(position <= 0.03, "cut at {cut}, tick {tick}: {position}");
            }
        }
    }
}

#[test]
fn a_damaged_payload_is_refused_by_byte() {
    let bytes = record(&grid(1, 100)).encode();
    assert!(
        PoseTrack::decode(&bytes[..bytes.len() - 1]).is_err(),
        "cut short"
    );
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(PoseTrack::decode(&longer).is_err(), "trailing byte");
    // 100 ticks: 17 segments, 85 mask bits in 11 bytes; the top three bits are padding. The
    // payload is the last tick, the craft count, the key bytes' length, then the mask.
    let key_bytes = Reader::le(&bytes[2..]).varint().expect("a length");
    let mut length = Vec::new();
    put_varint(&mut length, key_bytes);
    let mask_end = 2 + length.len() + mask_bytes(100);
    let mut padded = bytes.clone();
    padded[mask_end - 1] |= 0x80;
    assert!(
        PoseTrack::decode(&padded).is_err(),
        "a mask bit past the last tick"
    );
    assert!(PoseTrack::decode(&[]).is_err(), "empty");
}

/// The cost of saying which ticks are keys: a mask of five bits a segment against a varint
/// tick step a key.
#[test]
fn the_mask_is_cheaper_than_a_tick_step_a_key() {
    let track = record(&grid(8, 3_000));
    let keys: usize = (0..8).map(|craft| track.keys(craft)).sum();
    let steps: usize = track
        .crafts
        .iter()
        .map(|craft| {
            craft
                .windows(2)
                .map(|pair| {
                    let mut out = Vec::new();
                    put_varint(&mut out, u64::from(pair[1].tick - pair[0].tick));
                    out.len()
                })
                .sum::<usize>()
        })
        .sum();
    let mask = 8 * mask_bytes(3_000);
    assert!(
        mask < steps,
        "{mask} mask bytes against {steps} tick steps over {keys} keys"
    );
}
