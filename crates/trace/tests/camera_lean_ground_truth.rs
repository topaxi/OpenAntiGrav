//! Does the steering lean the cockpit camera rolls by follow the original's?
//!
//! `data/traces/talons-junction-steer-lean.csv` carries `entity+0x844` and
//! `entity+0x848` (`FUN_0883fab4`) for a full right and then a half left, held long
//! enough to settle (`verification/scenarios/steer-lean.inputs`). The stick record
//! the filter reads is not in the capture, so each segment's target is taken from
//! where the follower settles, and everything else - the `5.4` gain, the `0.6` and
//! `0.3` delta limits, the `4/s` smoothing - has to reproduce the whole curve:
//! the rise, the slower return, and the half-left segment.
//!
//! Measured 2026-10-02: lean RMS 0.0003, follower RMS 0.0003 over 480 ticks, at
//! one tick of capture latency (the script's tick `k` reaches the stick at row
//! `k + 1`). Dropping either limit, or the gain, moves it by two orders of
//! magnitude; see the tolerance.

use oag_physics::{ShipState, controls::update_camera_lean};

const CAPTURE: &str = include_str!("../../../data/traces/talons-junction-steer-lean.csv");

const ROWS_FULL_RIGHT: std::ops::Range<usize> = 61..211;
const ROWS_HALF_LEFT: std::ops::Range<usize> = 271..421;

fn capture() -> Vec<(f32, f32, f32)> {
    let mut lines = CAPTURE.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<&str> = lines.next().expect("header").split(',').collect();
    let at = |name: &str| header.iter().position(|c| *c == name).expect(name);
    let (dt, follower, lean) = (at("dt"), at("cam_lean_follower"), at("cam_lean"));
    lines
        .map(|line| {
            let cells: Vec<f32> = line
                .split(',')
                .map(|c| c.parse().expect("number"))
                .collect();
            (cells[dt], cells[follower], cells[lean])
        })
        .collect()
}

#[test]
fn the_camera_lean_reproduces_the_original_through_a_turn_and_its_release() {
    let rows = capture();
    assert_eq!(rows.len(), 480);
    let settled = |range: &std::ops::Range<usize>| rows[range.end - 1].1;
    let (right, left) = (settled(&ROWS_FULL_RIGHT), settled(&ROWS_HALF_LEFT));
    assert!(right > 0.9 && left < -0.05, "{right} {left}");

    let mut state = ShipState::default();
    let mut worst = 0.0_f32;
    for (row, (dt, follower, lean)) in rows.iter().enumerate() {
        let stick = if ROWS_FULL_RIGHT.contains(&row) {
            right
        } else if ROWS_HALF_LEFT.contains(&row) {
            left
        } else {
            0.0
        };
        update_camera_lean(&mut state, stick, *dt);
        worst = worst
            .max((state.camera_lean - lean).abs())
            .max((state.camera_lean_follower - follower).abs());
    }
    assert!(worst < 0.002, "worst error {worst}");
}
