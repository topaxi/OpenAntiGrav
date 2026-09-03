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

/// The scalable cost that sits exactly on the budget for `target`, with no
/// fixed cost at all.
fn on_budget(target: Target) -> f32 {
    target.scalable_budget(0.0).expect("a target that is on")
}

/// A frame whose whole cost is scalable - the shape every test here fed
/// before [`Cost`] split it, and still the right default: what these assert is
/// the *policy*, and the fixed half is an input to the budget rather than to
/// the policy. The tests that care about it name it.
fn cost(scalable: f32) -> Cost {
    Cost {
        scalable,
        fixed: 0.0,
    }
}

#[test]
fn off_never_moves_the_scale() {
    let mut drs = Controller::new();
    let limits = limits(Target::OFF);
    // A frame ten times over any plausible budget.
    assert_eq!(drs.record(cost(0.5), limits), None);
    assert_eq!(drs.record(cost(0.000_01), limits), None);
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
            drs.record(cost(budget * ratio), limits(target)),
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
        drs.record(cost(heavy), limits);
    }
    assert!(drs.scale() < 1.0, "the load should have driven it down");

    // Now a scene costing a quarter of its budget, forever.
    let cheap = on_budget(target) * 0.25;
    for _ in 0..500 {
        drs.record(cost(cheap), limits);
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
        if falling.record(cost(budget * 4.0), limits).is_some() {
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
        if rising.record(cost(budget * 0.1), limits).is_some() {
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
        drs.record(cost(budget * ratio), limits);
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
        drs.record(cost(budget * 50.0), limits);
        let (w, h) = drs.extent(limits);
        assert!((720..=1440).contains(&w), "{w} outside the bounds");
        assert!((408..=816).contains(&h), "{h} outside the bounds");
    }
    assert_eq!(drs.extent(limits), (720, 408));
    for _ in 0..500 {
        drs.record(cost(budget * 0.001), limits);
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
        drs.record(cost(on_budget(target) * 20.0), limits);
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
    assert!(
        drs.record(cost(over), limits).is_some(),
        "the first step lands"
    );
    for i in 0..COOLDOWN {
        assert_eq!(
            drs.record(cost(over), limits),
            None,
            "acted {i} frames into the cooldown"
        );
    }
    assert!(
        drs.record(cost(over), limits).is_some(),
        "and then it may act again"
    );
}

#[test]
fn a_stall_holds_the_scale_rather_than_resetting_it() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    for _ in 0..40 {
        drs.record(cost(on_budget(target) * 4.0), limits);
    }
    let held = drs.scale();
    assert!(held < 1.0);
    drs.reset();
    // The scale is where it was, and the readings in flight are dropped.
    assert_eq!(drs.scale(), held);
    assert_eq!(drs.record(cost(on_budget(target) * 100.0), limits), None);
}

#[test]
fn a_zero_or_negative_measurement_is_dropped() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    assert_eq!(drs.record(cost(0.0), limits), None);
    assert_eq!(drs.record(cost(-1.0), limits), None);
    assert_eq!(drs.record(cost(f32::NAN), limits), None);
    assert_eq!(drs.record(cost(f32::INFINITY), limits), None);
    assert_eq!(drs.scale(), 1.0);
}

#[test]
fn the_budget_is_a_share_of_the_target_period_and_nothing_else() {
    let sixty = Target::OFFERED[2];
    assert_eq!(sixty.hz(), Some(60));
    let budget = sixty.scalable_budget(0.0).expect("on");
    assert!((budget - (1.0 - RESIDUAL_SHARE) / 60.0).abs() < f32::EPSILON);
    // Twice the rate is half the budget, exactly.
    let one_twenty = Target::OFFERED[4];
    assert!((budget / one_twenty.scalable_budget(0.0).expect("on") - 2.0).abs() < 1e-5);
    assert_eq!(Target::OFF.scalable_budget(0.0), None);
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
        drs.record(cost(budget * 4.0), limits);
    }
    let held = drs.scale();
    assert!(held < 1.0);

    // A run of cheap frames one short of what a rise costs, then a single
    // comfortable one, which breaks it.
    for _ in 0..(RISE_PATIENCE - 1) {
        assert_eq!(drs.record(cost(budget * 0.5), limits), None);
    }
    // In the deadband: comfortable, not headroom.
    assert_eq!(drs.record(cost(budget * 0.9), limits), None);
    for _ in 0..(RISE_PATIENCE - 1) {
        assert_eq!(
            drs.record(cost(budget * 0.5), limits),
            None,
            "the run restarted, so this must not be enough"
        );
    }
    assert_eq!(drs.scale(), held, "nothing earned a rise");

    // And one more completes an unbroken run.
    assert!(drs.record(cost(budget * 0.5), limits).is_some());
    assert!(drs.scale() > held);
}

/// An overrun answers the very next frame, unlike a rise.
#[test]
fn a_fall_needs_no_patience_at_all() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    assert!(
        drs.record(cost(on_budget(target) * 4.0), limits).is_some(),
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
    let held = target.at_most(Some(60)).scalable_budget(0.0).expect("on");
    let sixty = Target::OFFERED[2].scalable_budget(0.0).expect("on");
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

/// The measured fixed cost is **subtracted**, not absorbed into a share.
///
/// The bug ADR-0042 replaces, as an assertion: a controller whose budget was a
/// constant fraction of the period could not see the FSR 3.1 chain at all, so
/// the same scalable cost was comfortable whether the chain took nothing or
/// most of the frame. Here it must not be.
#[test]
fn a_measured_fixed_cost_shrinks_the_budget_by_exactly_itself() {
    let target = Target::OFFERED[4];
    let period = target.period().expect("on");
    let free = target.scalable_budget(0.0).expect("on");
    // A third of the frame spent on something the extent cannot shrink.
    let fixed = period / 3.0;
    let squeezed = target.scalable_budget(fixed).expect("on");
    assert!(
        (free - squeezed - fixed).abs() < 1e-6,
        "the budget must fall by the fixed cost and by nothing else: {free} - {squeezed} != {fixed}"
    );

    // And the policy acts on the difference. A cost comfortably inside the
    // free budget is over the squeezed one.
    let scalable = free * 0.85;
    let mut idle = Controller::new();
    assert_eq!(
        idle.record(
            Cost {
                scalable,
                fixed: 0.0
            },
            limits(target)
        ),
        None,
        "inside the deadband with nothing fixed"
    );
    let mut busy = Controller::new();
    let moved = busy.record(Cost { scalable, fixed }, limits(target));
    assert!(
        moved.is_some() && busy.scale() < 1.0,
        "the same frame is over budget once the fixed cost is counted"
    );
}

/// A target the fixed cost alone cannot fit is reported and **not** chased.
///
/// The failure this prevents is the one `Target::at_most` names from the other
/// direction: giving up every pixel to buy frames that never arrive. No GPU is
/// needed to drive it, which matters - Xvfb cannot size a pass, so this
/// branch has no other way to be covered.
#[test]
fn an_unreachable_target_parks_rather_than_grinding_to_the_floor() {
    let target = Target::OFFERED[4];
    let period = target.period().expect("on");
    let limits = limits(target);
    let mut drs = Controller::new();
    assert!(!drs.unreachable(), "nothing has been measured yet");

    // The fixed half alone overruns the whole period, so no budget is left at
    // any resolution.
    let cost = Cost {
        scalable: period * 0.2,
        fixed: period * 1.1,
    };
    for _ in 0..64 {
        assert_eq!(drs.record(cost, limits), None, "there is nothing to decide");
    }
    assert!(drs.unreachable(), "and it has to say so");
    assert_eq!(
        drs.scale(),
        1.0,
        "the scale must not have moved: shrinking buys frames that never arrive"
    );

    // A budget that comes back clears it, so a spell that ends is reported as
    // ending rather than latching for the run.
    let affordable = Cost {
        scalable: target.scalable_budget(0.0).expect("on") * 0.9,
        fixed: 0.0,
    };
    drs.record(affordable, limits);
    assert!(!drs.unreachable());
}

/// Parked at the floor and still over budget is unreachable too.
///
/// The other half of the branch above, and the one a player actually reaches:
/// the budget is positive, the controller has spent every step it has, and the
/// frame is still late. A silent controller sitting at the floor missing its
/// target looks exactly like a controller that has decided the frame is fine.
#[test]
fn sitting_at_the_floor_over_budget_reports_unreachable() {
    let target = Target::OFFERED[4];
    let limits = limits(target);
    let budget = target.scalable_budget(0.0).expect("on");
    let mut drs = Controller::new();
    // Far enough over that the clamped fall walks it down to the floor.
    let cost = Cost {
        scalable: budget * 20.0,
        fixed: 0.0,
    };
    for _ in 0..256 {
        drs.record(cost, limits);
    }
    assert!(
        drs.scale() <= 0.5 + f32::EPSILON,
        "the 50 % floor is where it should have stopped, not below: {}",
        drs.scale()
    );
    assert!(
        drs.unreachable(),
        "still over budget with nowhere left to go"
    );
}

/// A rise is refused unless the frame after it would still have room.
///
/// **The oscillation this prevents is a property of the grid, not of the
/// deadband.** One step is a fixed fraction of the *ceiling*, so it is a
/// growing fraction of the *cost* as the scale falls: 10 % at the top of the
/// range and 21 % at half scale, against a 15 %-wide band. Below about 70 %
/// scale, every rise the band permits lands outside it and is undone - which
/// is exactly the two-point ping-pong `RISE_PATIENCE` was added to stop,
/// reappearing at the other end of the range once the budget could see the
/// whole scalable cost.
#[test]
fn a_rise_that_would_overshoot_the_band_is_refused() {
    let target = Target::OFFERED[4];
    let budget = target.scalable_budget(0.0).expect("on");
    // A ceiling of 1440 with a 720 floor: half scale is the lowest step, and
    // the widest relative step this controller can take.
    let limits = limits(target);

    // Walk it down to the floor first, so the rise under test starts there.
    let mut drs = Controller::new();
    for _ in 0..200 {
        drs.record(
            Cost {
                scalable: budget * 8.0,
                fixed: 0.0,
            },
            limits,
        );
    }
    assert!((drs.scale() - 0.5).abs() < 1e-6, "parked at the floor");

    // 0.77 of budget is inside the "under" range, so the old policy would have
    // risen after `RISE_PATIENCE` frames - and one step up from half scale
    // multiplies the cost by (11/10)^2 = 1.21, landing at 0.93 by the model
    // and past 1.0 in practice.
    let tempting = Cost {
        scalable: budget * 0.77,
        fixed: 0.0,
    };
    for _ in 0..(COOLDOWN + RISE_PATIENCE + 4) {
        assert_eq!(
            drs.record(tempting, limits),
            None,
            "a rise from the floor at 0.77 of budget overshoots and must be refused"
        );
    }
    assert!((drs.scale() - 0.5).abs() < 1e-6);

    // With real room, it still climbs - or the guard would have replaced an
    // oscillation with a controller that never recovers.
    let roomy = Cost {
        scalable: budget * 0.4,
        fixed: 0.0,
    };
    let mut rose = false;
    for _ in 0..64 {
        rose |= drs.record(roomy, limits).is_some();
    }
    assert!(rose, "a genuinely cheap frame must still earn a rise");
    assert!(drs.scale() > 0.5);
}
