//! Does Pulse's rigid body lose `0.01 * dt` of its speed each frame to its own damping?
//!
//! Steps [`oag_physics::step`] once from every recorded pose of a held-throttle PSP capture
//! (`talons-junction-standing-start.csv`, player input from the scripted emulator, never
//! Autopilot) and compares our next velocity with the original's along the recorded direction
//! of travel, as a fraction of `speed * dt`. Without the body damping (`Body_Integrate`'s
//! `velocity -= velocity * h * 0.01`, `docs/ghidra/functions/psp-pulse-usa/rigid-body.md`)
//! one step lands that fraction fast; with it the residual is what the force law leaves.
//! HD's twin of this read `0.01002` off and `0.00003` on
//! (`docs/ghidra/functions/ps3-hdfury-eu/body-damping.md`). Pulse's numbers:
//! `docs/physics/body-damping.md`.
//!
//! A second walk over a capture `wall_angular_pose_walk_ground_truth.rs` also walks: that one
//! keeps the wall ticks this one drops, and steps once per row where this steps twice; the pair
//! costs about 0.1 s.

use oag_physics::laws::BodyDamping;
use oag_physics::{CollisionWorld, Environment, Handling, step};
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;

use super::wall_angular_pose_walk_ground_truth::{controls_of, load, workspace};

const CAPTURE: &str = "data/traces/talons-junction-standing-start.csv";

/// Below this speed the `speed * dt` denominator magnifies the force law's own residual.
const MIN_SPEED: f32 = 8.0;

/// Each clean row's forward residual over `speed * dt`, stepped with `damping`.
fn residuals(
    world: &CollisionWorld,
    handling: &Handling,
    trace: &Trace,
    damping: BodyDamping,
) -> Vec<f32> {
    let mut environment = Environment::default();
    environment.laws.body_damping = damping;
    let mut out = Vec::new();
    for pair in trace.frames.windows(2) {
        let (now, next) = (&pair[0], &pair[1]);
        let speed = now.velocity.length();
        // A long `dt` is a missed frame; a throttle edge is a step the original ramps.
        if speed < MIN_SPEED || !(0.015..=0.018).contains(&now.dt) || now.throttle != next.throttle
        {
            continue;
        }
        let mut state = initial_state(
            now,
            handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        let evaluated = step(
            &mut state,
            &controls_of(now),
            handling,
            &environment,
            world,
            now.dt,
        );
        if evaluated.wall.contacts > evaluated.wall.floor_contacts {
            continue;
        }
        let along = next.velocity.normalize_or_zero();
        let error = (state.body.linear_velocity - next.velocity).dot(along);
        out.push(error / (speed * now.dt));
    }
    out
}

fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// With the damping off our step runs `0.01 * speed * dt` fast on the median clean frame
/// (`+0.00961`); with Pulse's pair the residual is `-0.00043`. Fails if the pair's linear half
/// leaves `0.01`, or if the force law moves the forward residual by more than its size.
#[test]
#[ignore = "needs data/images/"]
fn one_step_from_each_held_throttle_frame_loses_what_the_original_loses() {
    let (world, handling) = load();
    let text = std::fs::read_to_string(workspace(CAPTURE)).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let off = residuals(&world, &handling, &trace, BodyDamping::NONE);
    let pulse = residuals(&world, &handling, &trace, BodyDamping::PULSE);
    let (off_median, pulse_median) = (median(off.clone()), median(pulse.clone()));
    println!(
        "{} frames: median forward residual / (speed * dt): off {off_median:+.5}, \
         Pulse's pair {pulse_median:+.5}",
        off.len()
    );
    assert!(off.len() >= 100, "only {} clean frames", off.len());
    assert!(
        (0.008..0.012).contains(&off_median),
        "without the damping one step lands {off_median:+.5} of speed * dt fast, not 0.01"
    );
    assert!(
        pulse_median.abs() < 0.002,
        "Pulse's pair leaves {pulse_median:+.5} of speed * dt"
    );
}
