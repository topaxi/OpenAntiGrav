//! Does the lean the hull is drawn with follow the original's?
//!
//! `data/traces/talons-junction-steer-lean-airbrakes.csv` carries `entity+0x854` and
//! `entity+0x858` (`FUN_0883fab4`'s second filter) with the control record it reads:
//! the stick (`+0x00`) and both airbrakes (`+0x08`, `+0x0c`), through a full right
//! stick, each airbrake alone, the stick with either airbrake and both airbrakes
//! (`verification/scenarios/steer-lean-airbrakes.inputs`). The recorded stick and
//! airbrakes are fed in, not the script's values, so the original's stick shaping
//! (`stick_x = 1.0` is a record of `98.2`) does not enter the fit.
//!
//! Measured 2026-10-21: lean worst error below `1e-6` over 1,140 ticks, at one tick
//! of capture latency (row `k`'s lean is computed from row `k - 1`'s record, as the
//! camera lean's capture also found). Dropping the airbrake terms moves the worst
//! error to `0.52`, a `0.6` outward limit to `0.12`, and clamping on the target's
//! sign instead of the raw stick's to `0.04`.
//!
//! The second test is Wipeout HD's: `data/traces/hd-talons-steer-lean-airbrakes.csv`, an RPCS3
//! capture of `ship+0x6a54`/`+0x6a58` and the control record under the same inputs (Feisar,
//! `<Tilt shipAmount="1.4" shipSpeed="6">`), one row per race-clock frame. HD runs the same filter
//! with the stick term times `shipAmount`, the follower at `shipSpeed` and the smooth at `10.0`.
//! The rows are polled from a running emulator, not stopped at a breakpoint, so the record and
//! the lean can come from either side of a frame's update: the fit is RMS `0.0020`, worst `0.046`
//! at the input edges. Pulse's follower rate `5.0` instead gives RMS `0.045`, Pulse's smooth
//! `3.0` gives `0.137`, and `shipAmount` left out `0.217`.

use oag_physics::{ShipControls, ShipState, SteerLean, controls::update_steer_lean};

const CAPTURE: &str = include_str!("../../../data/traces/talons-junction-steer-lean-airbrakes.csv");
const HD_CAPTURE: &str = include_str!("../../../data/traces/hd-talons-steer-lean-airbrakes.csv");

/// Feisar's `<Tilt shipAmount="1.4" shipSpeed="6">` with HD's smooth `10.0`.
const HD_FEISAR: SteerLean = SteerLean {
    stick_amount: 1.4,
    follower_rate: 6.0,
    smooth_rate: 10.0,
};

struct Row {
    dt: f32,
    stick: f32,
    left: f32,
    right: f32,
    lean: f32,
}

fn capture() -> Vec<Row> {
    let mut lines = CAPTURE.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<&str> = lines.next().expect("header").split(',').collect();
    let at = |name: &str| header.iter().position(|c| *c == name).expect(name);
    let (dt, stick, left, right, lean) = (
        at("dt"),
        at("stick_steer"),
        at("stick_lab"),
        at("stick_rab"),
        at("steer_lean"),
    );
    lines
        .map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            let f = |i: usize| -> f32 { cells[i].parse().expect("number") };
            Row {
                dt: f(dt),
                stick: f(stick),
                left: f(left),
                right: f(right),
                lean: f(lean),
            }
        })
        .collect()
}

#[test]
fn the_drawn_lean_reproduces_the_original_through_stick_and_airbrakes() {
    let rows = capture();
    assert_eq!(rows.len(), 1140);
    // The capture holds every input the target has: a stick, and each airbrake alone.
    assert!(
        rows.iter()
            .any(|r| r.left == 100.0 && r.right == 0.0 && r.stick == 0.0)
    );
    assert!(
        rows.iter()
            .any(|r| r.right == 100.0 && r.left == 0.0 && r.stick == 0.0)
    );
    assert!(rows.iter().any(|r| r.left == 100.0 && r.right == 100.0));

    let mut state = ShipState::default();
    let mut worst = 0.0_f32;
    for pair in rows.windows(2) {
        let (record, row) = (&pair[0], &pair[1]);
        let controls = ShipControls {
            steer_x: record.stick / 100.0,
            airbrake_left: record.left / 100.0,
            airbrake_right: record.right / 100.0,
            ..ShipControls::default()
        };
        update_steer_lean(&mut state, &controls, &SteerLean::PULSE, row.dt);
        worst = worst.max((state.steer_lean - row.lean).abs());
    }
    assert!(worst < 1e-4, "worst error {worst}");
}

/// `(clock, stick, lab, rab, lean)` per frame of the HD capture.
fn hd_capture() -> Vec<[f32; 5]> {
    let mut lines = HD_CAPTURE.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<&str> = lines.next().expect("header").split(',').collect();
    let at = |name: &str| header.iter().position(|c| *c == name).expect(name);
    let columns = [
        at("clock"),
        at("stick"),
        at("lab"),
        at("rab"),
        at("steer_lean"),
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

/// Replays `lean` over the HD capture from its first row; `(rms, worst)`.
fn hd_fit(lean: &SteerLean) -> (f32, f32) {
    let rows = hd_capture();
    let mut state = ShipState::default();
    let (mut sum, mut worst) = (0.0_f32, 0.0_f32);
    for pair in rows.windows(2) {
        let (record, row) = (pair[0], pair[1]);
        let controls = ShipControls {
            steer_x: record[1] / 100.0,
            airbrake_left: record[2] / 100.0,
            airbrake_right: record[3] / 100.0,
            ..ShipControls::default()
        };
        update_steer_lean(&mut state, &controls, lean, row[0] - record[0]);
        let error = (state.steer_lean - row[4]).abs();
        sum += error * error;
        worst = worst.max(error);
    }
    ((sum / (rows.len() - 1) as f32).sqrt(), worst)
}

#[test]
fn hd_runs_the_same_filter_on_its_tilt_gains_and_a_faster_smooth() {
    let rows = hd_capture();
    assert_eq!(rows.len(), 1559);
    assert!(
        rows.iter()
            .any(|r| r[2] == 100.0 && r[3] == 0.0 && r[1] == 0.0)
    );
    assert!(
        rows.iter()
            .any(|r| r[3] == 100.0 && r[2] == 0.0 && r[1] == 0.0)
    );
    let (rms, worst) = hd_fit(&HD_FEISAR);
    assert!(rms < 0.005 && worst < 0.06, "rms {rms} worst {worst}");
    for wrong in [
        SteerLean {
            follower_rate: 5.0,
            ..HD_FEISAR
        },
        SteerLean {
            smooth_rate: 3.0,
            ..HD_FEISAR
        },
        SteerLean {
            stick_amount: 1.0,
            ..HD_FEISAR
        },
    ] {
        let (rms, _) = hd_fit(&wrong);
        assert!(rms > 0.02, "{wrong:?} fits too: rms {rms}");
    }
}
