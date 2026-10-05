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
    target
        .scalable_budget(0.0, Residual::new())
        .expect("a target that is on")
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
    assert_eq!(drs.record(cost(0.5), None, limits), None);
    assert_eq!(drs.record(cost(0.000_01), None, limits), None);
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
            drs.record(cost(budget * ratio), None, limits(target)),
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
        drs.record(cost(heavy), None, limits);
    }
    assert!(drs.scale() < 1.0, "the load should have driven it down");

    // Now a scene costing a quarter of its budget, forever.
    let cheap = on_budget(target) * 0.25;
    for _ in 0..500 {
        drs.record(cost(cheap), None, limits);
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
        if falling.record(cost(budget * 4.0), None, limits).is_some() {
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
        if rising.record(cost(budget * 0.1), None, limits).is_some() {
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
        drs.record(cost(budget * ratio), None, limits);
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
        drs.record(cost(budget * 50.0), None, limits);
        let (w, h) = drs.extent(limits);
        assert!((720..=1440).contains(&w), "{w} outside the bounds");
        assert!((408..=816).contains(&h), "{h} outside the bounds");
    }
    assert_eq!(drs.extent(limits), (720, 408));
    for _ in 0..500 {
        drs.record(cost(budget * 0.001), None, limits);
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
        drs.record(cost(on_budget(target) * 20.0), None, limits);
        assert_eq!(drs.extent(limits), (720, 408));
    }
}

#[test]
fn a_step_is_followed_by_a_cooldown_covering_the_frames_in_flight() {
    // Four is the timer's own ring depth, and a reading resolves at least one
    // frame late - see `oag_gpu::timing::PassTimer`.
    const { assert!(COOLDOWN >= 4) };

    let target = Target::OFFERED[2];
    let limits = limits(target);
    let over = on_budget(target) * 4.0;
    let mut drs = Controller::new();
    assert!(
        drs.record(cost(over), None, limits).is_some(),
        "the first step lands"
    );
    for i in 0..COOLDOWN {
        assert_eq!(
            drs.record(cost(over), None, limits),
            None,
            "acted {i} frames into the cooldown"
        );
    }
    assert!(
        drs.record(cost(over), None, limits).is_some(),
        "and then it may act again"
    );
}

#[test]
fn a_stall_holds_the_scale_rather_than_resetting_it() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    for _ in 0..40 {
        drs.record(cost(on_budget(target) * 4.0), None, limits);
    }
    let held = drs.scale();
    assert!(held < 1.0);
    drs.reset();
    // The scale is where it was, and the readings in flight are dropped.
    assert_eq!(drs.scale(), held);
    assert_eq!(
        drs.record(cost(on_budget(target) * 100.0), None, limits),
        None
    );
}

#[test]
fn a_zero_or_negative_measurement_is_dropped() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    assert_eq!(drs.record(cost(0.0), None, limits), None);
    assert_eq!(drs.record(cost(-1.0), None, limits), None);
    assert_eq!(drs.record(cost(f32::NAN), None, limits), None);
    assert_eq!(drs.record(cost(f32::INFINITY), None, limits), None);
    assert_eq!(drs.scale(), 1.0);
}

#[test]
fn the_budget_is_a_share_of_the_target_period_and_nothing_else() {
    let sixty = Target::OFFERED[2];
    assert_eq!(sixty.hz(), Some(60));
    let budget = sixty.scalable_budget(0.0, Residual::new()).expect("on");
    assert!((budget - (1.0 - RESIDUAL_SHARE) / 60.0).abs() < f32::EPSILON);
    // Twice the rate is half the budget, exactly.
    let one_twenty = Target::OFFERED[4];
    assert!(
        (budget
            / one_twenty
                .scalable_budget(0.0, Residual::new())
                .expect("on")
            - 2.0)
            .abs()
            < 1e-5
    );
    assert_eq!(Target::OFF.scalable_budget(0.0, Residual::new()), None);
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
        drs.record(cost(budget * 4.0), None, limits);
    }
    let held = drs.scale();
    assert!(held < 1.0);

    // A run of cheap frames one short of what a rise costs, then a single
    // comfortable one, which breaks it.
    for _ in 0..(RISE_PATIENCE - 1) {
        assert_eq!(drs.record(cost(budget * 0.5), None, limits), None);
    }
    // In the deadband: comfortable, not headroom.
    assert_eq!(drs.record(cost(budget * 0.9), None, limits), None);
    for _ in 0..(RISE_PATIENCE - 1) {
        assert_eq!(
            drs.record(cost(budget * 0.5), None, limits),
            None,
            "the run restarted, so this must not be enough"
        );
    }
    assert_eq!(drs.scale(), held, "nothing earned a rise");

    // And one more completes an unbroken run.
    assert!(drs.record(cost(budget * 0.5), None, limits).is_some());
    assert!(drs.scale() > held);
}

/// An overrun answers the very next frame, unlike a rise.
#[test]
fn a_fall_needs_no_patience_at_all() {
    let target = Target::OFFERED[2];
    let limits = limits(target);
    let mut drs = Controller::new();
    assert!(
        drs.record(cost(on_budget(target) * 4.0), None, limits)
            .is_some(),
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
    let held = target
        .at_most(Some(60))
        .scalable_budget(0.0, Residual::new())
        .expect("on");
    let sixty = Target::OFFERED[2]
        .scalable_budget(0.0, Residual::new())
        .expect("on");
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
    let free = target.scalable_budget(0.0, Residual::new()).expect("on");
    // A third of the frame spent on something the extent cannot shrink.
    let fixed = period / 3.0;
    let squeezed = target.scalable_budget(fixed, Residual::new()).expect("on");
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
            None,
            limits(target)
        ),
        None,
        "inside the deadband with nothing fixed"
    );
    let mut busy = Controller::new();
    let moved = busy.record(Cost { scalable, fixed }, None, limits(target));
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
        assert_eq!(
            drs.record(cost, None, limits),
            None,
            "there is nothing to decide"
        );
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
        scalable: target.scalable_budget(0.0, Residual::new()).expect("on") * 0.9,
        fixed: 0.0,
    };
    drs.record(affordable, None, limits);
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
    let budget = target.scalable_budget(0.0, Residual::new()).expect("on");
    let mut drs = Controller::new();
    // Far enough over that the clamped fall walks it down to the floor.
    let cost = Cost {
        scalable: budget * 20.0,
        fixed: 0.0,
    };
    for _ in 0..256 {
        drs.record(cost, None, limits);
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
    let budget = target.scalable_budget(0.0, Residual::new()).expect("on");
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
            None,
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
            drs.record(tempting, None, limits),
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
        rose |= drs.record(roomy, None, limits).is_some();
    }
    assert!(rose, "a genuinely cheap frame must still earn a rise");
    assert!(drs.scale() > 0.5);
}

/// A residual that has been taught `reading` seconds, by the only route into
/// it there is: frames that ran past their own deadline, which is the gate
/// [`residual::OVERRUN`] holds a rise to.
///
/// The timed cost is deliberately longer than the period, so the frame
/// overruns whatever `reading` is - a machine can spend less than 5 % of a
/// frame outside its timers, and the helper must still be able to say so.
fn taught(reading: f32, period: f32) -> Residual {
    let mut residual = Residual::new();
    let timed = period * 1.2;
    for _ in 0..400 {
        residual.observe(timed + reading, timed, period);
    }
    residual
}

/// The estimate walks toward a residual that keeps repeating on frames with no
/// slack in them - which is the whole of ADR-0044.
///
/// The shape is the Steam Deck's: a machine that spends **less** outside its
/// timers than `RESIDUAL_SHARE` reserves, and so is shrunk by a budget that
/// holds back 2.2 ms of an 11.1 ms frame it never needed. The controller
/// cannot see that directly; what it can see is that the frame it is being
/// told about does not add up to what the constant claims.
#[test]
fn the_residual_converges_on_what_a_tight_frame_keeps_saying() {
    let target = Target::OFFERED[3];
    let period = target.period().expect("on");
    let truth = period * 0.05;
    let limits = limits(target);
    let mut drs = Controller::new();
    assert_eq!(drs.residual().learned(), None, "nothing is known yet");
    let reserved_before = drs.residual().seconds(period);
    assert!((reserved_before - RESIDUAL_SHARE * period).abs() < 1e-9);

    // A machine genuinely over its budget: the scalable cost alone is a whole
    // period, so no amount of sleep is available to hide in and the wall clock
    // is the work.
    let cost = Cost {
        scalable: period,
        fixed: 0.0,
    };
    let frame = cost.total() + truth;
    for _ in 0..400 {
        drs.record(cost, Some(frame), limits);
    }
    let learned = drs.residual().seconds(period);
    assert!(
        (learned - truth).abs() < truth * 0.05,
        "learned {learned} seconds against a truth of {truth}"
    );
    assert!(
        learned < reserved_before,
        "the reserve has to have come down, not merely moved"
    );
    // And the budget it feeds grew by exactly what stopped being reserved.
    let budget = target.scalable_budget(0.0, drs.residual()).expect("on");
    assert!((budget - (period - learned)).abs() < 1e-9);
}

/// A paced frame with room to spare teaches it nothing, which is ADR-0040's
/// argument still standing.
///
/// The loop sleeps to its deadline, so the wall clock reads as the target
/// period however little work the frame did - and `frame - timed` is then
/// almost the whole period, an enormous overestimate of a residual that is
/// tiny. Folding one of those in would shrink the budget, which lowers the
/// scale, which makes the next frame sleep longer: the runaway this gate
/// exists to make unrepresentable.
#[test]
fn a_sleeping_frame_cannot_teach_the_residual_anything() {
    let target = Target::OFFERED[3];
    let period = target.period().expect("on");
    let limits = limits(target);
    let mut drs = Controller::new();
    let cheap = cost(period * 0.05);
    for _ in 0..600 {
        // Paced: the frame took exactly its period however cheap it was.
        drs.record(cheap, Some(period), limits);
    }
    assert_eq!(
        drs.residual().learned(),
        None,
        "a frame with slack in it is not evidence of anything"
    );
    assert!((drs.residual().seconds(period) - RESIDUAL_SHARE * period).abs() < 1e-9);

    // The same, with something already learned: a comfortable stretch must not
    // undo what a tight one established.
    let mut drs = Controller::new();
    let tight = Cost {
        scalable: period,
        fixed: 0.0,
    };
    for _ in 0..400 {
        drs.record(tight, Some(tight.total() + period * 0.05), limits);
    }
    let learned = drs.residual().seconds(period);
    for _ in 0..600 {
        drs.record(cheap, Some(period), limits);
    }
    assert!(
        (drs.residual().seconds(period) - learned).abs() < 1e-9,
        "a sleeping frame moved an estimate a tight one had earned"
    );
}

/// A frame time far past the period *is* evidence - and the constant is what
/// bounds how far that evidence may be trusted.
///
/// A wall clock five times the period with a cheap scene is not sleep. It is
/// the untimed CPU cost ADR-0043 measured and could not reach: physics, AI and
/// draw calls for a full grid. The estimate is allowed to rise for it, because
/// refusing to would leave the budget claiming room the machine does not have.
/// And it may not rise past `RESIDUAL_SHARE`, because the one other thing
/// that overruns a period while still sleeping is a display refreshing below
/// the target, which this module cannot see. Capped, that mistake costs
/// exactly the budget this build already shipped.
#[test]
fn an_overrunning_frame_cannot_push_the_reserve_past_the_constant() {
    let target = Target::OFFERED[3];
    let period = target.period().expect("on");
    let limits = limits(target);
    let mut drs = Controller::new();
    let cheap = cost(period * 0.05);
    for _ in 0..500 {
        drs.record(cheap, Some(period * 5.0), limits);
    }
    let reserved = drs.residual().seconds(period);
    assert!(
        reserved <= RESIDUAL_SHARE * period + 1e-9,
        "reserved {reserved} of a {period} second frame, past the constant"
    );
    assert!(
        (target.scalable_budget(0.0, drs.residual()).expect("on")
            - target.scalable_budget(0.0, Residual::new()).expect("on"))
        .abs()
            < 1e-9,
        "the worst case has to be the budget this build already had"
    );
}

/// Fed no wall clock at all, the controller is the one that shipped: the
/// constant, on every frame, forever.
///
/// The `None` is not a special case for tests. It is the frame that carried a
/// track load, where the caller has a duration and knows it is not a frame
/// time - see [`Controller::reset`].
#[test]
fn nothing_is_learned_until_a_frame_can_prove_it() {
    let target = Target::OFFERED[3];
    let period = target.period().expect("on");
    let limits = limits(target);
    let mut fed = Controller::new();
    let mut blind = Controller::new();
    // Frames from the top of the deadband down to nearly free, every one of
    // them paced to the target - so every one of them has slack in it, and not
    // one is evidence of anything. A frame *over* budget would be, which is
    // the point: the estimate moves on the frames the controller is about to
    // act on and on no others.
    for i in 0..600 {
        let scalable = match i % 3 {
            0 => period * 0.7,
            1 => period * 0.4,
            _ => period * 0.05,
        };
        let cost = cost(scalable);
        fed.record(cost, Some(period), limits);
        blind.record(cost, None, limits);
        assert_eq!(
            fed.scale(),
            blind.scale(),
            "the two controllers parted company on frame {i}"
        );
    }
    assert_eq!(fed.residual().learned(), None);
    assert_eq!(blind.residual().learned(), None);
}

/// Whatever it learns, the reserve never grows - so no budget shrinks, no
/// scale falls that would not have fallen before, and no target becomes
/// unreachable that was reachable.
///
/// **The property the whole change rests on**, asserted over a grid rather
/// than at a point: `RESIDUAL_SHARE` is a ceiling on the reserve and a floor
/// under the budget, and a reader should not have to trust one worked example
/// for that.
#[test]
fn a_learned_residual_never_reserves_more_than_the_constant() {
    for target in Target::OFFERED {
        let Some(period) = target.period() else {
            continue;
        };
        for reading in [
            0.0,
            0.001,
            period * 0.01,
            period * 0.5,
            period,
            period * 8.0,
        ] {
            let residual = taught(reading, period);
            // **Read back at every target, not only the one it was taught
            // at.** A player may change `target_fps` mid-session, and what
            // holds the promise then is `Residual::seconds` clamping against
            // the period in force rather than the one that was current when
            // the estimate was written.
            for read_at in Target::OFFERED {
                let Some(read_period) = read_at.period() else {
                    continue;
                };
                for fixed in [
                    0.0,
                    read_period * 0.1,
                    read_period * 0.5,
                    read_period * 0.95,
                ] {
                    let learned = read_at.scalable_budget(fixed, residual).expect("on");
                    let constant = read_at.scalable_budget(fixed, Residual::new()).expect("on");
                    assert!(
                        learned >= constant - 1e-9,
                        "taught at {target}, read at {read_at}: a reading of {reading} against \
                         {fixed} fixed reserved more than the constant ({learned} < {constant})"
                    );
                    assert!(
                        learned > 0.0 || constant <= 0.0,
                        "taught at {target}, read at {read_at}: a learned reserve invented an \
                         unreachable target"
                    );
                }
            }
        }
    }
}

/// A load throws away the readings in flight and keeps what the session
/// learned, for the same reason it keeps the scale.
#[test]
fn a_stall_does_not_unlearn_the_residual() {
    let target = Target::OFFERED[3];
    let period = target.period().expect("on");
    let limits = limits(target);
    let mut drs = Controller::new();
    let tight = Cost {
        scalable: period,
        fixed: 0.0,
    };
    for _ in 0..400 {
        drs.record(tight, Some(tight.total() + period * 0.05), limits);
    }
    let learned = drs.residual().learned().expect("a tight run teaches it");
    drs.reset();
    assert_eq!(drs.residual().learned(), Some(learned));
}

mod machines;
