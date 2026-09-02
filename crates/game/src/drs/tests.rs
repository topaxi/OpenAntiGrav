//! What the controller in [`super::policy`] is asserted to do, against fed
//! sequences rather than against whatever the machine happened to be doing.
//!
//! Its own file rather than a `#[cfg(test)]` block: the rule keys on
//! `#[cfg(test)]` and nothing else, and 200 lines is not enough for a policy
//! whose constants are only correct with respect to each other.

use super::*;

/// A 1440x816 allocation with a 50 % floor, aiming at 60.
fn limits(target: Target) -> Limits {
    Limits::new((1440, 816), (720, 408), target)
}

/// The scene cost that sits exactly on the budget for `target`.
fn on_budget(target: Target) -> f32 {
    target.scene_budget().expect("a target that is on")
}

#[test]
fn off_never_moves_the_scale() {
    let mut drs = Controller::new();
    let limits = limits(Target::OFF);
    // A frame ten times over any plausible budget.
    assert_eq!(drs.record(0.5, limits), None);
    assert_eq!(drs.record(0.000_01, limits), None);
    assert_eq!(drs.extent(limits), (1440, 816));
}

#[test]
fn a_frame_inside_the_deadband_does_not_twitch() {
    let target = Target::OFFERED[2];
    let mut drs = Controller::new();
    let budget = on_budget(target);
    // Sweep the band rather than sampling its ends: the failure this guards
    // against is a controller that acts somewhere in the middle of it.
    for i in 0..=15u8 {
        let ratio = 0.80 + f32::from(i) * 0.01;
        assert_eq!(
            drs.record(budget * ratio, limits(target)),
            None,
            "acted at {ratio} of budget"
        );
    }
    assert_eq!(drs.scale(), 1.0);
}

/// A loop paced *at* the target still climbs, because the budget is a share
/// of a frame rather than a measured one.
///
/// **This is the test that would have caught the formulation this module
/// rejects.** A controller fed `frame_interval - scene` as headroom is inert
/// under vsync or a frame limit: the loop sleeps to the target, so the slack
/// absorbs whatever the scene did not use and the ratio is 1.0 at every
/// scale. Nothing here feeds a frame interval at all - there is nowhere to
/// put one - and that is the property being pinned.
#[test]
fn a_loop_paced_at_the_target_still_reaches_the_ceiling() {
    let target = Target::OFFERED[2];
    let mut drs = Controller::new();
    // Start low, the way a controller that has just weathered a load is.
    let limits = limits(target);
    let heavy = on_budget(target) * 4.0;
    for _ in 0..200 {
        drs.record(heavy, limits);
    }
    assert!(drs.scale() < 1.0, "the load should have driven it down");

    // Now a scene costing a quarter of its budget, forever.
    let cheap = on_budget(target) * 0.25;
    for _ in 0..500 {
        drs.record(cheap, limits);
    }
    assert_eq!(drs.scale(), 1.0, "an idle scene must return to the ceiling");
    assert_eq!(drs.extent(limits), (1440, 816));
}

#[test]
fn an_overrun_falls_faster_than_an_underrun_rises() {
    let target = Target::OFFERED[2];
    let budget = on_budget(target);
    let limits = limits(target);

    // Bounded loops rather than "until it settles": a policy that cannot move
    // at all should fail this test, not hang it. That is not hypothetical -
    // a 4 % rise ceiling against a 5 % grid did exactly that, and a `while`
    // loop turned a wrong answer into a wedged test run.
    let mut falling = Controller::new();
    let mut fall_steps = 0;
    for _ in 0..400 {
        if falling.record(budget * 4.0, limits).is_some() {
            fall_steps += 1;
        }
        if falling.scale() <= 0.6 {
            break;
        }
    }
    assert!(falling.scale() <= 0.6, "the fall never reached 60 %");

    // Start the rising controller from where the falling one ended, so both
    // cover the same distance.
    let mut rising = falling.clone();
    let from = rising.scale();
    let mut rise_steps = 0;
    for _ in 0..4000 {
        if rising.record(budget * 0.1, limits).is_some() {
            rise_steps += 1;
        }
        if rising.scale() >= 1.0 {
            break;
        }
    }
    assert!(rising.scale() > from, "the rise never moved off {from}");
    assert_eq!(rising.scale(), 1.0, "the rise never reached the ceiling");
    assert!(
        rise_steps > fall_steps,
        "rising took {rise_steps} steps and falling {fall_steps}; a rise must be the slower of the two"
    );
}

/// The band and the grid are only correct with respect to each other.
///
/// One grid step changes the pixel count by `(1 + STEP)^2`, so it changes the
/// measured ratio by the same factor. If the band were narrower than that, a
/// single correction from just inside one edge would land just outside the
/// other and the scale would ping-pong between two adjacent grid points for
/// as long as the load held still - a controller that never settles, on a
/// signal that never changed.
#[test]
fn the_deadband_is_wider_than_one_grid_step() {
    let widened = (1.0 + STEP) * (1.0 + STEP);
    let band = DEADBAND.end() / DEADBAND.start();
    assert!(
        band > widened,
        "the deadband spans {band:.4} of budget and one step moves the cost by {widened:.4}"
    );
    // And the two ways of saying the grid have to agree, since the policy
    // counts in `GRID` and everything that reads a scale divides by `STEP`.
    assert!((STEP * GRID as f32 - 1.0).abs() < f32::EPSILON);
    // A rise of one step, and a fall of five: the asymmetry, as integers.
    const { assert!(RISE_STEPS < FALL_STEPS) };
    assert_eq!(
        RISE_STEPS, 1,
        "a rise that is not one step needs its own argument"
    );
}

#[test]
fn every_scale_it_emits_is_on_the_grid() {
    let target = Target::OFFERED[3];
    let limits = limits(target);
    let budget = on_budget(target);
    let mut drs = Controller::new();
    // A deterministic sawtooth of costs from a tenth of budget to eight
    // times it, which is every branch the policy has.
    for i in 0..400 {
        let ratio = 0.1 + (i % 40) as f32 * 0.2;
        drs.record(budget * ratio, limits);
        let on_grid = (drs.scale() / STEP).round() * STEP;
        assert!(
            (drs.scale() - on_grid).abs() < 1e-4,
            "scale {} is not on the {STEP} grid",
            drs.scale()
        );
    }
}

#[test]
fn it_never_goes_below_the_floor_or_above_the_ceiling() {
    let target = Target::OFFERED[4];
    let limits = limits(target);
    let budget = on_budget(target);
    let mut drs = Controller::new();
    for _ in 0..500 {
        drs.record(budget * 50.0, limits);
        let (w, h) = drs.extent(limits);
        assert!((720..=1440).contains(&w), "{w} outside the bounds");
        assert!((408..=816).contains(&h), "{h} outside the bounds");
    }
    assert_eq!(drs.extent(limits), (720, 408));
    for _ in 0..500 {
        drs.record(budget * 0.001, limits);
    }
    assert_eq!(drs.extent(limits), (1440, 816));
}

/// A floor above the ceiling is a state the menus permit and must not crash.
///
/// The floor row and the render-scale row are independent, and the warning
/// about a floor at or above the ceiling tells a player rather than stopping
/// them. `Ord::clamp` panics when `min > max`, so this would have been a
/// panic in the frame loop reached by moving two rows in the wrong order.
#[test]
fn a_floor_above_the_ceiling_collapses_onto_it() {
    let target = Target::OFFERED[2];
    // A 50 % render scale with a 100 % floor: the controller has nowhere to
    // go and must say so by standing still.
    let limits = Limits::new((720, 408), (1440, 816), target);
    assert_eq!(limits.floor(), (720, 408));
    let mut drs = Controller::new();
    for _ in 0..50 {
        drs.record(on_budget(target) * 20.0, limits);
        assert_eq!(drs.extent(limits), (720, 408));
    }
}

#[test]
fn a_step_is_followed_by_a_cooldown_covering_the_frames_in_flight() {
    // Four is the timer's own ring depth, and a reading resolves at least one
    // frame late - see `oag_render::timing::PassTimer`.
    const { assert!(COOLDOWN >= 4) };

    let target = Target::OFFERED[2];
    let limits = limits(target);
    let over = on_budget(target) * 4.0;
    let mut drs = Controller::new();
    assert!(drs.record(over, limits).is_some(), "the first step lands");
    for i in 0..COOLDOWN {
        assert_eq!(
            drs.record(over, limits),
            None,
            "acted {i} frames into the cooldown"
        );
    }
    assert!(
        drs.record(over, limits).is_some(),
        "and then it may act again"
    );
}

#[test]
fn a_stall_holds_the_scale_rather_than_resetting_it() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    for _ in 0..40 {
        drs.record(on_budget(target) * 4.0, limits);
    }
    let held = drs.scale();
    assert!(held < 1.0);
    drs.reset();
    // The scale is where it was, and the readings in flight are dropped.
    assert_eq!(drs.scale(), held);
    assert_eq!(drs.record(on_budget(target) * 100.0, limits), None);
}

#[test]
fn a_zero_or_negative_measurement_is_dropped() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    assert_eq!(drs.record(0.0, limits), None);
    assert_eq!(drs.record(-1.0, limits), None);
    assert_eq!(drs.record(f32::NAN, limits), None);
    assert_eq!(drs.record(f32::INFINITY, limits), None);
    assert_eq!(drs.scale(), 1.0);
}

#[test]
fn the_budget_is_a_share_of_the_target_period_and_nothing_else() {
    let sixty = Target::OFFERED[2];
    assert_eq!(sixty.hz(), Some(60));
    let budget = sixty.scene_budget().expect("on");
    assert!((budget - SCENE_SHARE / 60.0).abs() < f32::EPSILON);
    // Twice the rate is half the budget, exactly.
    let one_twenty = Target::OFFERED[4];
    assert!((budget / one_twenty.scene_budget().expect("on") - 2.0).abs() < 1e-5);
    assert_eq!(Target::OFF.scene_budget(), None);
}

#[test]
fn every_offered_target_round_trips_and_a_bad_one_says_what_it_knows() {
    for target in Target::OFFERED {
        let text = target.to_string();
        assert_eq!(
            text.parse::<Target>(),
            Ok(target),
            "{text} did not round-trip"
        );
    }
    assert_eq!("off".parse::<Target>(), Ok(Target::OFF));
    assert_eq!("OFF".parse::<Target>(), Ok(Target::OFF));
    assert_eq!(" 60 ".parse::<Target>(), Ok(Target::OFFERED[2]));
    // Zero is refused rather than taken as a second spelling of off.
    assert!("0".parse::<Target>().is_err());
    assert!("9999".parse::<Target>().is_err());
    let why = "sometimes".parse::<Target>().expect_err("not a rate");
    assert!(why.contains("off"), "{why}");
}

/// The default is off, which is the footing every enhancement here starts on.
#[test]
fn a_fresh_install_has_no_controller_running() {
    assert_eq!(Target::default(), Target::OFF);
    assert_eq!(Target::DEFAULT, Target::OFF);
    assert!(!Target::default().is_on());
}

/// One cheap frame is not headroom, and a run of them is.
///
/// The regression this pins was found in a running game rather than here: a
/// 4K race aiming at 144 flipped between 100 % and 95 % 143 times in a minute,
/// because the scene pass genuinely costs 0.75 to 1.07 of its budget at one
/// fixed scale as the camera moves. Every decision was individually right and
/// the picture still changed size twice a second.
#[test]
fn a_rise_has_to_be_earned_by_a_run_of_frames_with_room() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let budget = on_budget(target);
    let mut drs = Controller::new();

    // Drive it down first, so there is somewhere to rise to.
    for _ in 0..200 {
        drs.record(budget * 4.0, limits);
    }
    let held = drs.scale();
    assert!(held < 1.0);

    // A run of cheap frames one short of what a rise costs, then a single
    // comfortable one, which breaks it.
    for _ in 0..(RISE_PATIENCE - 1) {
        assert_eq!(drs.record(budget * 0.5, limits), None);
    }
    // In the deadband: comfortable, not headroom.
    assert_eq!(drs.record(budget * 0.9, limits), None);
    for _ in 0..(RISE_PATIENCE - 1) {
        assert_eq!(
            drs.record(budget * 0.5, limits),
            None,
            "the run restarted, so this must not be enough"
        );
    }
    assert_eq!(drs.scale(), held, "nothing earned a rise");

    // And one more completes an unbroken run.
    assert!(drs.record(budget * 0.5, limits).is_some());
    assert!(drs.scale() > held);
}

/// An overrun answers the very next frame, unlike a rise.
#[test]
fn a_fall_needs_no_patience_at_all() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    assert!(
        drs.record(on_budget(target) * 4.0, limits).is_some(),
        "a dropped frame has already been seen; there is nothing to wait for"
    );
}

/// A target above the limiter is held to the limiter.
///
/// Otherwise the budget is computed for frames the loop is not allowed to
/// produce: at 144 behind a 60 limit it would be 2.4x too tight, so the
/// controller would drop the resolution permanently and every pixel it gave up
/// would buy nothing.
#[test]
fn a_target_above_the_frame_limit_is_held_to_it() {
    let target = Target::OFFERED[5];
    assert_eq!(target.hz(), Some(144));
    assert_eq!(target.at_most(Some(60)).hz(), Some(60));
    // The budget follows, which is the whole point of the clamp.
    let held = target.at_most(Some(60)).scene_budget().expect("on");
    let sixty = Target::OFFERED[2].scene_budget().expect("on");
    assert!((held - sixty).abs() < f32::EPSILON);
}

/// A limiter at or above the target changes nothing, and neither does none.
///
/// `None` covers two different situations that must behave the same way:
/// `FrameLimit::UNLIMITED`, and `Vsync::On` where the display is the bound and
/// this build cannot ask a surface what its refresh is. **Guessing there would
/// be worse than not clamping**: the fallback available is the simulation's
/// 60, which would silently cap a 144 Hz panel's target at 60 while the menu
/// row still said 144.
#[test]
fn a_limiter_at_or_above_the_target_leaves_it_alone() {
    let target = Target::OFFERED[2];
    assert_eq!(target.at_most(Some(60)), target);
    assert_eq!(target.at_most(Some(144)), target);
    assert_eq!(target.at_most(None), target);
    // And off stays off whatever the limiter says.
    assert_eq!(Target::OFF.at_most(Some(30)), Target::OFF);
    assert!(!Target::OFF.at_most(Some(30)).is_on());
}

/// Every offered target, against every offered limit, lands on the lower.
#[test]
fn the_clamp_is_the_lower_of_the_two_for_every_offered_pair() {
    for target in Target::OFFERED {
        for limit in crate::perf::FrameLimit::OFFERED {
            let held = target.at_most(limit.hz());
            match (target.hz(), limit.hz()) {
                (Some(want), Some(bound)) => {
                    assert_eq!(held.hz(), Some(want.min(bound)), "{target} under {limit}");
                }
                // Off stays off; an unlimited limiter bounds nothing.
                _ => assert_eq!(held, target, "{target} under {limit}"),
            }
        }
    }
}
