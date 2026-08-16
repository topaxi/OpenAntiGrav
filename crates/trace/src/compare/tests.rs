//! What the comparison engine in [`super`] is asserted to do: the per-field
//! tolerances, how an error's trend is classified, misalignment between two
//! traces, and the report it prints.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `compare.rs`: the tests are 454 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;
use crate::trace::Trace;

/// A capture-shaped fixture with `ticks` rows, where the ship travels along
/// `+z` at a constant speed. Hand-authored: a real trace is derived game data
/// and can never be committed, and inventing one that claimed to be a
/// recording would be worse than having none.
fn fixture(ticks: usize, mutate: impl Fn(usize, &mut [f32; 3])) -> Trace {
    let mut csv = String::from(
        "tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
         right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
         pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed\n",
    );
    for tick in 0..ticks {
        let mut position = [10.0, 2.5, -30.0 + tick as f32 * 0.366_99];
        mutate(tick, &mut position);
        csv.push_str(&format!(
            "{tick},0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,{},{},{},0,0,22,22\n",
            position[0], position[1], position[2]
        ));
    }
    Trace::parse(&csv).unwrap()
}

/// The recorded yaw rate the captures show when the stick is held over:
/// `+1.51 rad/s` about row 1 on 199 of 199 ticks,
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
const RECORDED_YAW_RATE: f32 = 1.51;

/// The gap `YAW_INVERSE_INERTIA` closes: the recovered steering law is
/// verified at instruction level and predicts a yaw rate about 22x too
/// high, so a run without it turns 22 times too fast. **`YAW_INVERSE_INERTIA`
/// is itself recovered, not a stand-in** - it replaced the fitted
/// `YAW_DRIVE_CALIBRATION` once its writer was disassembled; this comment
/// described the old arrangement.
const YAW_AUTHORITY_ERROR: f32 = 22.1;

/// The same fixture with an angular velocity of `rate` about the up axis on
/// every tick. Hand-authored like everything else here.
fn turning(ticks: usize, rate: f32) -> Trace {
    let mut trace = fixture(ticks, |_, _| {});
    for frame in &mut trace.frames {
        frame.angular_velocity = Some(Vec3::new(0.0, rate, 0.0));
        frame.stun_timer = Some(0.0);
    }
    trace
}

/// **The measurement this column was added for.** A run whose yaw authority is
/// 22x off must be reported as a divergence in the angular velocity, on tick
/// 0, rather than only showing up ticks later as an accumulated attitude
/// error - which is what the harness could see before and why the yaw
/// question could not move.
#[test]
fn a_yaw_rate_the_size_of_the_open_yaw_authority_error_diverges() {
    let recorded = turning(40, RECORDED_YAW_RATE);
    let simulated = turning(40, RECORDED_YAW_RATE / YAW_AUTHORITY_ERROR);
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    let divergence = comparison.first_divergence.clone().expect("must diverge");
    assert_eq!(divergence.tick, 0);
    let angular = comparison.field(Field::AngularVelocity);
    assert_eq!(angular.first_exceeded_tick, Some(0));
    assert_eq!(angular.compared_ticks, 40);
    assert!(
        (angular.max_error - RECORDED_YAW_RATE * (1.0 - 1.0 / YAW_AUTHORITY_ERROR)).abs() < 1e-3,
        "{}",
        angular.max_error
    );
}

/// The other half of the tolerance choice: a straight-line capture records an
/// angular velocity of *exactly* zero, so a purely relative test would call
/// any simulated dust a relative error of one and report tick 0 of every
/// straight capture as the divergence. The absolute floor is what stops it.
#[test]
fn a_straight_line_is_not_diverged_by_dust_around_a_recorded_zero() {
    let recorded = turning(40, 0.0);
    let mut simulated = recorded.clone();
    for frame in &mut simulated.frames {
        frame.angular_velocity = Some(Vec3::new(1e-9, -2e-9, 1e-9));
    }
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert!(!comparison.diverged(), "{comparison}");

    // And the floor is low enough to still catch a real rotation: a
    // hundredth of the recorded turn rate is 1.5e-2, well over 1e-4.
    for frame in &mut simulated.frames {
        frame.angular_velocity = Some(Vec3::new(0.0, RECORDED_YAW_RATE / 100.0, 0.0));
    }
    assert!(compare(&recorded, &simulated, &Tolerances::default()).diverged());
}

/// An absent column is the absence of a measurement. It must not be compared
/// against a zero, and it must not be reported as a field that agreed.
#[test]
fn an_absent_column_is_not_compared_rather_than_agreed() {
    // The recording predates the column; the simulated run always has one.
    let recorded = fixture(40, |_, _| {});
    let simulated = turning(40, RECORDED_YAW_RATE);
    assert_eq!(recorded.frames[0].angular_velocity, None);

    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    let angular = comparison.field(Field::AngularVelocity);
    assert_eq!(angular.compared_ticks, 0);
    assert_eq!(angular.first_exceeded_tick, None);
    assert_eq!(angular.max_error, 0.0, "nothing was measured");
    assert!(!comparison.diverged(), "{comparison}");

    // And the report says so out loud, because a max error of zero on its own
    // reads exactly like a perfect match.
    let report = comparison.to_string();
    assert!(report.contains("angular_velocity"), "{report}");
    assert!(report.contains("not compared"), "{report}");
}

/// Every field a legacy capture *does* carry is compared exactly as before,
/// which is the whole backwards-compatibility claim.
#[test]
fn a_legacy_capture_still_compares_on_every_field_it_carries() {
    let recorded = fixture(40, |_, _| {});
    let comparison = compare(&recorded, &recorded, &Tolerances::default());
    for summary in &comparison.fields {
        match summary.field {
            // Every optional group the fixture does not carry: the two
            // angular readings, the stun timer, and the eight flare fields.
            Field::AngularVelocity
            | Field::AngularRate
            | Field::StunTimer
            | Field::BoostTimer
            | Field::PlumeTimer
            | Field::Intensity
            | Field::HalfSize
            | Field::EngineOn
            | Field::FlareSpeedKmh
            | Field::SpeedRamp
            | Field::BoostAccumulator => {
                assert_eq!(summary.compared_ticks, 0, "{:?}", summary.field);
            }
            other => assert_eq!(summary.compared_ticks, 40, "{other:?}"),
        }
    }
    assert!(!comparison.diverged());
}

/// The stun timer gates thrust and lateral grip, so a run that fails to arm
/// it is a run producing thrust the original did not - the force-balance
/// question in `HANDOVER.md`. Half a second of it is 500x the tolerance.
#[test]
fn a_stun_the_simulation_missed_is_a_divergence() {
    let mut recorded = turning(8, 0.0);
    let simulated = recorded.clone();
    for frame in &mut recorded.frames[3..] {
        frame.stun_timer = Some(0.5);
    }
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    let divergence = comparison.first_divergence.clone().expect("must diverge");
    assert_eq!(divergence.check.field, Field::StunTimer);
    assert_eq!(divergence.tick, 3);
    assert_eq!(comparison.field(Field::StunTimer).compared_ticks, 8);
}

/// `compare` indexes its per-field accumulators against [`Field::ALL`] by
/// position, so a field added to the enum and forgotten there is silently
/// never compared - and the report would look complete while missing one.
#[test]
fn every_field_is_in_the_list_exactly_once() {
    assert_eq!(Field::ALL.len(), FIELD_COUNT);
    for (index, field) in Field::ALL.iter().enumerate() {
        assert_eq!(
            Field::ALL.iter().position(|f| f == field),
            Some(index),
            "{field:?} appears twice"
        );
    }
}

#[test]
fn identical_traces_do_not_diverge() {
    let trace = fixture(40, |_, _| {});
    let comparison = compare(&trace, &trace, &Tolerances::default());
    assert!(!comparison.diverged());
    assert_eq!(comparison.compared_ticks, 40);
    assert_eq!(comparison.field(Field::Position).max_error, 0.0);
    assert_eq!(
        comparison.field(Field::Position).trend.verdict,
        TrendVerdict::Exact
    );
}

#[test]
fn a_difference_inside_tolerance_is_not_reported() {
    let recorded = fixture(40, |_, _| {});
    // 0.005 units, under the 0.01 absolute band, on every tick.
    let simulated = fixture(40, |_, position| position[0] += 0.005);
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert!(!comparison.diverged(), "{comparison}");
    let summary = comparison.field(Field::Position);
    assert!((summary.max_error - 0.005).abs() < 1e-6);
    assert_eq!(summary.first_exceeded_tick, None);
}

#[test]
fn a_difference_beyond_tolerance_is_reported_at_the_right_tick() {
    let recorded = fixture(40, |_, _| {});
    let simulated = fixture(40, |tick, position| {
        if tick >= 7 {
            position[0] += 0.25;
        }
    });
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    let divergence = comparison.first_divergence.clone().expect("must diverge");
    assert_eq!(divergence.tick, 7);
    assert_eq!(divergence.index, 7);
    assert_eq!(divergence.check.field, Field::Position);
    assert!((divergence.check.error - 0.25).abs() < 1e-5);
    assert_eq!(
        comparison.field(Field::Position).first_exceeded_tick,
        Some(7)
    );
    // Everything else agreed, so nothing else may claim a divergence.
    assert!(
        comparison
            .fields
            .iter()
            .filter(|s| s.field != Field::Position)
            .all(|s| s.first_exceeded_tick.is_none())
    );
}

/// The protocol asks for the trend explicitly: an error that grows every tick
/// is systematic even while it is still inside tolerance.
#[test]
fn a_growing_error_inside_tolerance_is_still_reported_as_growing() {
    let recorded = fixture(40, |_, _| {});
    let simulated = fixture(40, |tick, position| {
        position[0] += tick as f32 * 0.000_2;
    });
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert!(!comparison.diverged(), "0.0078 max, inside 0.01 absolute");
    let trend = comparison.field(Field::Position).trend;
    assert_eq!(trend.verdict, TrendVerdict::Growing);
    assert!(trend.slope_per_tick > 0.0);
    assert!(trend.last_quarter_mean > trend.first_quarter_mean);
}

#[test]
fn a_constant_offset_is_bounded_not_growing() {
    let recorded = fixture(40, |_, _| {});
    let simulated = fixture(40, |_, position| position[0] += 0.005);
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert_eq!(
        comparison.field(Field::Position).trend.verdict,
        TrendVerdict::Bounded
    );
}

#[test]
fn a_decaying_error_is_reported_as_shrinking() {
    let recorded = fixture(40, |_, _| {});
    let simulated = fixture(40, |tick, position| {
        position[0] += 0.005 / (1.0 + tick as f32);
    });
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert_eq!(
        comparison.field(Field::Position).trend.verdict,
        TrendVerdict::Shrinking
    );
}

/// Groundedness is quantised, so any difference at all is a bug: it is the
/// one field where "close enough" has no meaning.
#[test]
fn a_discrete_field_has_no_slack() {
    let mut recorded = fixture(4, |_, _| {});
    let mut simulated = recorded.clone();
    simulated.frames[2].grounded = 0.5;
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    let divergence = comparison.first_divergence.clone().expect("must diverge");
    assert_eq!(divergence.check.field, Field::Grounded);
    assert_eq!(divergence.tick, 2);

    // And the smallest representable difference is still a difference.
    recorded.frames[1].grounded = 1.0;
    simulated.frames[1].grounded = 1.0 + f32::EPSILON;
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert_eq!(comparison.first_divergence.expect("must diverge").tick, 1);
}

#[test]
fn orientation_is_compared_as_an_angle_in_radians() {
    let mut recorded = fixture(4, |_, _| {});
    let mut simulated = recorded.clone();
    // A milliradian of yaw, ten times the 1e-4 tolerance.
    let angle = 1e-3f32;
    for frame in &mut simulated.frames {
        frame.forward = Vec3::new(angle.sin(), 0.0, angle.cos());
        frame.row0 = Vec3::new(angle.cos(), 0.0, -angle.sin());
    }
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    let divergence = comparison.first_divergence.expect("must diverge");
    assert!(matches!(
        divergence.check.field,
        Field::Row0 | Field::Forward
    ));
    assert!((divergence.check.error - angle).abs() < 1e-6);

    // Half the tolerance is not a divergence.
    for frame in &mut simulated.frames {
        let small = 5e-5f32;
        frame.forward = Vec3::new(small.sin(), 0.0, small.cos());
        frame.row0 = Vec3::new(small.cos(), 0.0, -small.sin());
    }
    recorded.frames.iter_mut().for_each(|frame| {
        frame.forward = Vec3::Z;
        frame.row0 = Vec3::X;
    });
    assert!(!compare(&recorded, &simulated, &Tolerances::default()).diverged());
}

#[test]
fn velocity_is_relative_so_a_fast_ship_gets_more_slack_than_a_slow_one() {
    let mut recorded = fixture(4, |_, _| {});
    let mut simulated = recorded.clone();
    for frame in &mut simulated.frames {
        // 0.01 on 22 units/s is 4.5e-4 relative: inside 1e-3.
        frame.velocity.z += 0.01;
        frame.speed += 0.01;
    }
    assert!(!compare(&recorded, &simulated, &Tolerances::default()).diverged());

    for frame in &mut recorded.frames {
        frame.velocity = Vec3::new(0.0, 0.0, 1.0);
        frame.speed = 1.0;
    }
    for frame in &mut simulated.frames {
        frame.velocity = Vec3::new(0.0, 0.0, 1.01);
        frame.speed = 1.01;
    }
    // The same absolute error on 1 unit/s is 1e-2 relative: outside it.
    assert!(compare(&recorded, &simulated, &Tolerances::default()).diverged());
}

/// Every tolerance test is a `>`, and `NaN > x` is false, so a poisoned run
/// would report perfect agreement on every field at once - the single most
/// misleading thing this tool could say. A tumbling ship is exactly what it
/// will be pointed at, so this is pinned per quantity.
#[test]
fn a_non_finite_value_is_never_within_tolerance() {
    for poison in [f32::NAN, f32::INFINITY] {
        let recorded = fixture(4, |_, _| {});

        let mut simulated = recorded.clone();
        simulated.frames[2].position.x = poison;
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison
            .first_divergence
            .clone()
            .unwrap_or_else(|| panic!("{poison} position must diverge"));
        assert_eq!(divergence.tick, 2);
        assert_eq!(divergence.check.field, Field::Position);
        assert!(!comparison.field(Field::Position).max_error.is_finite());

        // An attitude, where the normalising in `angle_between` would
        // otherwise launder a NaN into an angle of zero.
        let mut simulated = recorded.clone();
        simulated.frames[1].forward = Vec3::new(poison, poison, poison);
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        let divergence = comparison
            .first_divergence
            .unwrap_or_else(|| panic!("{poison} attitude must diverge"));
        assert_eq!(divergence.tick, 1);
        assert_eq!(divergence.check.field, Field::Forward);

        // And a velocity, which is judged purely relatively.
        let mut simulated = recorded.clone();
        simulated.frames[3].speed = poison;
        let comparison = compare(&recorded, &simulated, &Tolerances::default());
        assert_eq!(
            comparison
                .first_divergence
                .unwrap_or_else(|| panic!("{poison} speed must diverge"))
                .tick,
            3
        );
    }
}

/// The largest error and the largest *relative* error need not fall on the
/// same tick, and the protocol judges velocity relatively - so a slow tick's
/// large relative error must survive a fast tick's large absolute one.
#[test]
fn the_largest_relative_error_is_not_lost_to_a_later_absolute_one() {
    let mut recorded = fixture(6, |_, _| {});
    let mut simulated = recorded.clone();
    for frame in recorded.frames.iter_mut().chain(&mut simulated.frames) {
        frame.velocity = Vec3::ZERO;
        frame.speed = 0.0;
    }
    // Tick 1: 0.01 against 0.02 - half a unit of relative error, a hundredth
    // of one absolute.
    recorded.frames[1].speed = 0.01;
    simulated.frames[1].speed = 0.02;
    // Tick 5: fifty times the absolute error, a fiftieth of the relative one.
    recorded.frames[5].speed = 22.0;
    simulated.frames[5].speed = 22.5;

    let summary = *compare(&recorded, &simulated, &Tolerances::default()).field(Field::Speed);
    assert_eq!(
        summary.max_error_tick, 5,
        "the biggest error is the late one"
    );
    assert!((summary.max_error - 0.5).abs() < 1e-6);
    assert!(
        (summary.max_relative - 0.5).abs() < 1e-6,
        "the early tick's relative error survived: {}",
        summary.max_relative
    );
}

#[test]
fn traces_of_different_lengths_compare_the_shared_prefix() {
    let recorded = fixture(40, |_, _| {});
    let simulated = fixture(10, |_, _| {});
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert_eq!(comparison.compared_ticks, 10);
    assert_eq!(comparison.recorded_ticks, 40);
    assert!(!comparison.diverged());
}

#[test]
fn a_gap_in_one_trace_is_reported_as_misalignment() {
    let recorded = fixture(8, |_, _| {});
    let mut simulated = recorded.clone();
    for frame in &mut simulated.frames[4..] {
        frame.tick += 1;
    }
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert_eq!(comparison.first_misalignment, Some(4));
}

#[test]
fn the_report_names_the_tick_the_field_and_the_tolerance() {
    let recorded = fixture(12, |_, _| {});
    let simulated = fixture(12, |tick, position| {
        if tick >= 3 {
            position[2] += 1.0;
        }
    });
    let report = compare(&recorded, &simulated, &Tolerances::default()).to_string();
    assert!(report.contains("first divergence at tick 3"), "{report}");
    assert!(report.contains("position"), "{report}");
    assert!(report.contains("0.01 absolute"), "{report}");
}
