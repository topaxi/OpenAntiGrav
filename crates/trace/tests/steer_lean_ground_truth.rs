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

use oag_physics::{ShipControls, ShipState, controls::update_steer_lean};

const CAPTURE: &str =
    include_str!("../../../data/traces/talons-junction-steer-lean-airbrakes.csv");

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
    assert!(rows.iter().any(|r| r.left == 100.0 && r.right == 0.0 && r.stick == 0.0));
    assert!(rows.iter().any(|r| r.right == 100.0 && r.left == 0.0 && r.stick == 0.0));
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
        update_steer_lean(&mut state, &controls, row.dt);
        worst = worst.max((state.steer_lean - row.lean).abs());
    }
    assert!(worst < 1e-4, "worst error {worst}");
}
