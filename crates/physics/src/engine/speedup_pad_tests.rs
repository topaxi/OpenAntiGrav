//! What the speed-up pad boost in [`super`] is asserted to do. Split out of `engine.rs` under the
//! 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::{PAD_RAMP_FLOOR, PAD_RAMP_RATE, SPEEDPAD_JUMP_THRESHOLD, speedup_pad};
use crate::params::{Handling, SpeedupPads};
use crate::ship::{ShipControls, ShipState};
use oag_core::math::Vec3;

/// Upright, with the pitch axis centred: the tilt branch not taken.
const UP: Vec3 = Vec3::Y;

fn centred() -> ShipControls {
    ShipControls::default()
}

/// The pitch axis held the way the input layer reports d-pad Up, which is
/// **negative**. See `speedup_pad`'s own documentation for why.
fn pitched_up() -> ShipControls {
    ShipControls {
        steer_y: -1.0,
        ..ShipControls::default()
    }
}

/// Round invented numbers. `time` is deliberately well above
/// [`PAD_RAMP_FLOOR`] so both branches of the law are reachable.
fn padded_handling() -> Handling {
    Handling {
        speedup_pads: SpeedupPads {
            amount: 2.0,
            time: 0.5,
        },
        ..Handling::ZERO
    }
}

const DT: f32 = 1.0 / 60.0;

#[test]
fn a_ship_that_has_touched_no_pad_gets_no_boost() {
    let mut state = ShipState::default();
    assert_eq!(
        speedup_pad(&mut state, &centred(), &padded_handling(), UP, None, DT),
        Vec3::ZERO
    );
    assert_eq!(state.pad_timer, 0.0);
}

/// The shape of the law: `amount * 10 * remaining` while more than [`PAD_RAMP_FLOOR`] is left, flat
/// `amount` after, and **the decrement happens first**, so the first tick is already one `dt` down
/// the ramp, not at `amount * 10 * time`.
#[test]
fn the_boost_ramps_down_and_then_holds_flat() {
    let handling = padded_handling();
    let mut state = ShipState::default();

    let first = speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::NEG_Z), DT);
    let peak = handling.speedup_pads.amount * PAD_RAMP_RATE * (handling.speedup_pads.time - DT);
    assert!((first.length() - peak).abs() < 1e-3, "{first} vs {peak}");
    assert!(
        first.length() < handling.speedup_pads.amount * PAD_RAMP_RATE * handling.speedup_pads.time,
        "reading the force before the decrement would give the larger value"
    );

    // Falls monotonically while the ramp branch is live.
    let mut previous = first.length();
    while state.pad_timer > PAD_RAMP_FLOOR {
        let now = speedup_pad(&mut state, &centred(), &handling, UP, None, DT).length();
        assert!(now < previous, "{now} is not below {previous}");
        previous = now;
    }

    // And then holds flat at `amount` until the timer runs out.
    while state.pad_timer > 0.0 {
        let now = speedup_pad(&mut state, &centred(), &handling, UP, None, DT).length();
        assert!((now - handling.speedup_pads.amount).abs() < 1e-6, "{now}");
    }
    assert_eq!(
        speedup_pad(&mut state, &centred(), &handling, UP, None, DT),
        Vec3::ZERO,
        "an expired boost applies nothing"
    );
}

/// The two branches meet rather than step, which is the whole reason the
/// constants are a reciprocal pair.
#[test]
fn the_ramp_and_the_flat_branch_meet_at_the_crossover() {
    assert_eq!(PAD_RAMP_RATE * PAD_RAMP_FLOOR, 1.0);
}

/// Standing on a pad re-arms the timer every tick, so the force does not decay inside the volume:
/// a slow crossing boosts longer than a fast one, which is why `Environment::pad_hit` is a
/// per-tick containment answer, not an entry edge.
#[test]
fn staying_inside_a_pad_holds_the_boost_at_its_peak() {
    let handling = padded_handling();
    let mut state = ShipState::default();

    let first = speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::NEG_Z), DT).length();
    for _ in 0..120 {
        let now =
            speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::NEG_Z), DT).length();
        assert!((now - first).abs() < 1e-6, "{now} drifted from {first}");
    }
}

/// Leaving the pad does not re-aim the remaining boost: a ship that turns after crossing keeps
/// being pushed the way the pad faced, the original holding `craft+0x1b0` rather than recomputing.
#[test]
fn the_push_direction_is_held_after_the_pad_is_behind_the_ship() {
    let handling = padded_handling();
    let mut state = ShipState::default();

    speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::X), DT);
    let later = speedup_pad(&mut state, &centred(), &handling, UP, None, DT);
    assert_eq!(later.normalize(), Vec3::X);
    assert_eq!(state.pad_direction, Vec3::X);
}

/// `<Special speedpad_jump>`: the numbers, not just the shape. At the live
/// `0.1` against a unit pad direction the boost tilts by `atan(0.1)` and
/// gains `sqrt(1.01)`, which is the arithmetic that makes this "not a jump".
#[test]
fn the_tilt_is_a_small_angle_and_a_smaller_force_increase() {
    let handling = Handling {
        speedpad_jump: 0.1,
        ..padded_handling()
    };
    let mut plain = ShipState::default();
    let mut tilted = ShipState::default();

    let without = speedup_pad(&mut plain, &centred(), &handling, UP, Some(Vec3::X), DT);
    let with = speedup_pad(&mut tilted, &pitched_up(), &handling, UP, Some(Vec3::X), DT);

    let angle = without
        .normalize()
        .dot(with.normalize())
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();
    assert!((angle - 5.71).abs() < 0.01, "{angle} degrees");

    let gain = with.length() / without.length();
    assert!((gain - 1.01f32.sqrt()).abs() < 1e-5, "{gain}");
    // Toward the hull's up, not away from it and not sideways.
    assert!(with.dot(UP) > 0.0 && without.dot(UP) == 0.0, "{with}");
}

/// The gate is on the **negative** side of the pitch axis, where `oag_gameplay::ship_controls` puts
/// d-pad Up. Taking the positive side would tilt on nose-down, the failure this pins.
#[test]
fn only_a_pitch_up_input_tilts_the_boost() {
    let handling = Handling {
        speedpad_jump: 0.5,
        ..padded_handling()
    };
    let plain = {
        let mut state = ShipState::default();
        speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::X), DT)
    };

    for (name, steer_y, tilts) in [
        ("centred", 0.0, false),
        ("nose down, full", 1.0, false),
        (
            "nose down, past the threshold",
            SPEEDPAD_JUMP_THRESHOLD,
            false,
        ),
        ("nose up, just under the threshold", -0.49, false),
        ("nose up, on the threshold", -SPEEDPAD_JUMP_THRESHOLD, true),
        ("nose up, full", -1.0, true),
    ] {
        let controls = ShipControls {
            steer_y,
            ..ShipControls::default()
        };
        let mut state = ShipState::default();
        let force = speedup_pad(&mut state, &controls, &handling, UP, Some(Vec3::X), DT);
        assert_eq!(force.dot(UP) > 0.0, tilts, "{name}: {force}");
        if !tilts {
            assert_eq!(force, plain, "{name} moved the boost");
        }
    }
}

/// The tilt is read from *this* tick's input, not baked into `pad_direction` when the pad armed the
/// boost, so it can start and stop while the boost runs on behind the pad (the original recomputes
/// `dir` inside the timer block).
#[test]
fn the_tilt_follows_the_input_rather_than_the_pad() {
    let handling = Handling {
        speedpad_jump: 0.5,
        ..padded_handling()
    };
    let mut state = ShipState::default();

    // Crossed with the axis centred, so nothing is stored tilted.
    speedup_pad(&mut state, &centred(), &handling, UP, Some(Vec3::X), DT);
    assert_eq!(state.pad_direction, Vec3::X, "the stored axis is the pad's");

    // Pitch up after the pad: the remaining boost tilts anyway.
    let after = speedup_pad(&mut state, &pitched_up(), &handling, UP, None, DT);
    assert!(after.dot(UP) > 0.0, "{after}");
    assert_eq!(state.pad_direction, Vec3::X, "and the state stays clean");

    // Let go: it stops tilting on the very next tick.
    let released = speedup_pad(&mut state, &centred(), &handling, UP, None, DT);
    assert_eq!(released.dot(UP), 0.0, "{released}");
}

/// A disc whose `<Special speedpad_jump>` is zero, or a load that could not
/// read the file, boosts exactly as it did before this branch existed.
#[test]
fn a_zero_jump_changes_nothing() {
    let handling = padded_handling();
    assert_eq!(handling.speedpad_jump, 0.0, "the fixture's default");
    let mut plain = ShipState::default();
    let mut pitched = ShipState::default();
    assert_eq!(
        speedup_pad(&mut plain, &centred(), &handling, UP, Some(Vec3::X), DT),
        speedup_pad(
            &mut pitched,
            &pitched_up(),
            &handling,
            UP,
            Some(Vec3::X),
            DT
        )
    );
}
