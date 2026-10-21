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

use oag_physics::{CameraClamp, CameraLean, ShipState, controls::update_camera_lean};

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
        update_camera_lean(&mut state, stick, &CameraLean::PULSE, *dt);
        worst = worst
            .max((state.camera_lean - lean).abs())
            .max((state.camera_lean_follower - follower).abs());
    }
    assert!(worst < 0.002, "worst error {worst}");
}

const HD_CAPTURE: &str = include_str!("../../../data/traces/hd-talons-cockpit-lean.csv");

/// Feisar's `<Tilt internalAmount="1.4" internalSpeed="7">` with HD's code literals, as
/// `oag_raceplay::load::code_laws` fills them.
const HD_FEISAR: CameraLean = CameraLean {
    stick_scale: 0.015 * 1.4,
    follower_rate: 7.0,
    smooth_rate: 6.0,
    sideshift_offset: 0.5,
    clamp: CameraClamp::Hd,
};

/// `(clock, stick, lean, follower)` per frame of the HD capture.
fn hd_capture() -> Vec<[f32; 4]> {
    let mut lines = HD_CAPTURE.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<&str> = lines.next().expect("header").split(',').collect();
    let at = |name: &str| header.iter().position(|c| *c == name).expect(name);
    let columns = [
        at("clock"),
        at("stick"),
        at("cam_lean"),
        at("cam_lean_follower"),
    ];
    lines
        .map(|line| {
            let cells: Vec<f32> = line
                .split(',')
                .map(|c| c.parse().expect("number"))
                .collect();
            columns.map(|i| cells[i])
        })
        .collect()
}

/// Replays `lean` over the HD capture from its first row, one row of record latency; `(rms,
/// worst)`.
fn hd_fit(lean: &CameraLean) -> (f32, f32) {
    let rows = hd_capture();
    let mut state = ShipState {
        camera_lean: rows[0][2],
        camera_lean_follower: rows[0][3],
        ..ShipState::default()
    };
    let (mut sum, mut worst) = (0.0_f32, 0.0_f32);
    for pair in rows.windows(2) {
        let (record, row) = (pair[0], pair[1]);
        update_camera_lean(&mut state, record[1] / 100.0, lean, row[0] - record[0]);
        let error = (state.camera_lean - row[2]).abs();
        sum += error * error;
        worst = worst.max(error);
    }
    ((sum / (rows.len() - 1) as f32).sqrt(), worst)
}

/// Wipeout HD's cockpit lean (`ship+0x6a44`, `FUN_000eadb8`), polled from a running RPCS3 under
/// a scripted stick (2,681 rows): full right, centre, half left, centre, full right, centre,
/// full left, centre. It settles at `+/-2.0972` (`0.99868 * 0.015 * 1.4`), twice Pulse's, and lets
/// go at the follower's own rate, where Pulse's `0.3` limit would take twice as long.
///
/// The rows are polled, not stopped at a breakpoint, so a frame's record and lean can sit
/// either side of its update: RMS `0.009`, worst `0.18` at the edges. The Pulse gains give RMS
/// `0.35`, Pulse's step limit `0.24`, and `internalAmount` left out `0.6`.
#[test]
fn hds_cockpit_lean_runs_its_tilt_gains_and_lets_go_unbounded() {
    let rows = hd_capture();
    assert_eq!(rows.len(), 2681);
    assert!(rows.iter().any(|r| r[2] > 2.09) && rows.iter().any(|r| r[2] < -2.09));
    let (rms, worst) = hd_fit(&HD_FEISAR);
    assert!(rms < 0.015 && worst < 0.25, "rms {rms} worst {worst}");
    for wrong in [
        CameraLean::PULSE,
        CameraLean {
            clamp: CameraClamp::Pulse,
            ..HD_FEISAR
        },
        CameraLean {
            stick_scale: 0.015,
            ..HD_FEISAR
        },
        CameraLean {
            smooth_rate: 4.0,
            ..HD_FEISAR
        },
    ] {
        let (rms, _) = hd_fit(&wrong);
        assert!(rms > 0.025, "{wrong:?} fits too: rms {rms}");
    }
}
