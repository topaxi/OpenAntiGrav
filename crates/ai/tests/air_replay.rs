//! A scratch replay: the original's own captured driver state through this engine's
//! `steer::steer` and `steer::pitch`, frame by frame, printing both outputs side by side.
//!
//! The input is the `.replay.txt` that `scripts/psp-autopilot-air.py` writes beside its
//! capture (the column layout is that script's `flatten`), named by `OAG_AIR_REPLAY`.
//! The controller is Pulse's PHANTOM row of `AIControlStats.xml`, and the lane is `0`, the
//! player's driver's. Evidence it produced: `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`,
//! "The airborne driver, replayed live".

use oag_ai::original::{Controller, Pose, SteerInput, TrackPoint, airborne, pitch, steer};
use oag_core::math::Vec3;

fn v(c: &[f32], i: usize) -> Vec3 {
    Vec3::new(c[i], c[i + 1], c[i + 2])
}

fn point(c: &[f32], i: usize) -> TrackPoint {
    TrackPoint {
        position: v(c, i),
        tangent: v(c, i + 3),
        down: v(c, i + 6),
        lateral: v(c, i + 9),
        progress: c[i + 12],
        bound_left: c[i + 13],
        bound_right: c[i + 14],
        racing_line: c[i + 15],
    }
}

#[test]
#[ignore = "a scratch replay: set OAG_SWEEP and OAG_AIR_REPLAY"]
fn replay() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(path) = std::env::var_os("OAG_AIR_REPLAY") else {
        return;
    };
    let controller = Controller {
        steer_mul: 0.7,
        steer_damp: 5.057,
        look_ahead_secs: 0.3,
        xtrack_mul: 0.0091,
    };
    let text = std::fs::read_to_string(path).expect("replay input");
    for (n, line) in text.lines().enumerate() {
        let c: Vec<f32> = line.split(' ').map(|s| s.parse().expect("f32")).collect();
        let pose = Pose {
            position: v(&c, 0),
            up: v(&c, 3),
            forward: v(&c, 6),
            left: v(&c, 9),
            velocity: v(&c, 12),
            angular_velocity: v(&c, 15),
        };
        let own = point(&c, 18);
        let ahead = point(&c, 34);
        let target = v(&c, 50);
        let air = airborne(&pose, &own);
        let mut last_error = c[54];
        let ours = steer(
            &pose,
            &SteerInput {
                own: &own,
                own_lane: 0.0,
                ahead: &ahead,
                ahead_lane: 0.0,
                air,
            },
            &controller,
            &mut last_error,
        );
        let mut last_forward = v(&c, 55);
        let our_pitch = pitch(&pose, &own, c[58], 1.0, &mut last_forward);
        println!(
            "{n} pos {:.1} {:.1} {:.1} g {:.2} air {:.3} tgt_err {:.4} err ours {:.5} orig {:.5} steer ours {:.2} orig {:.2} L {:.1}/{:.1} R {:.1}/{:.1} pitch ours {:.2} orig {:.2}",
            c[0],
            c[1],
            c[2],
            c[58],
            air,
            (ahead.lane_point(0.0) - target).length(),
            last_error,
            c[64],
            ours.steer,
            c[59],
            ours.left,
            c[61],
            ours.right,
            c[62],
            our_pitch,
            c[63],
        );
    }
}
