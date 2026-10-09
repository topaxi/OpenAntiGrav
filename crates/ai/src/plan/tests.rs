//! The speed plan on a synthetic stadium: two straights and two hairpins on a
//! flat floor, no disc image anywhere.
//!
//! **The hull and the circuit are invented**, round numbers in the shape of
//! `tests/closed_loop_real_geometry.rs`'s own fixture, per ADR-0006. With no
//! walls on a plane, "failure" here is the [`Course::off_line`] excursion: a
//! craft that runs wide of the hairpin by more than that. The disc-backed half
//! is `crates/game/tests/speed_plan_ground_truth.rs`.

use oag_core::math::{Quat, Vec3, sin_cos};
use oag_physics::{
    Body, Environment, Handling, Ray, RaycastHit, Raycaster, ShipState, Surface, params,
};

use super::*;

/// An infinite horizontal floor at `y = 0`.
struct Plane;

impl Raycaster for Plane {
    fn raycast(&self, ray: Ray, _skip: Option<u32>, _include_reset: bool) -> Option<RaycastHit> {
        if ray.direction.y >= -1.0e-4 {
            return None;
        }
        let distance = ray.origin.y / -ray.direction.y;
        if distance < 0.0 || distance > ray.length {
            return None;
        }
        Some(RaycastHit {
            point: ray.origin + ray.direction * distance,
            normal: Vec3::Y,
            distance,
            surface: Surface::Floor,
            vertex_scalar: 1.0,
            collider: 0,
            triangle: None,
        })
    }

    fn raycast_all(
        &self,
        ray: Ray,
        skip: Option<u32>,
        include_reset: bool,
        out: &mut [Option<RaycastHit>],
    ) -> usize {
        match (self.raycast(ray, skip, include_reset), out.first_mut()) {
            (Some(hit), Some(slot)) => {
                *slot = Some(hit);
                1
            }
            _ => 0,
        }
    }
}

/// A craft that can drive and brake, with invented numbers.
fn handling() -> Handling {
    Handling {
        engine: params::Engine {
            accelcap: 60.0,
            amount: 1.0,
            ..Default::default()
        },
        turning: params::Turning {
            amount: 1.0,
            gain: 400.0,
            falloff: 400.0,
        },
        antigrav: params::Antigrav {
            grip_ground: 40.0,
            grip_air: 2.0,
            ride_height: 10.0,
            rebound: 1.0,
            ..Default::default()
        },
        physical: params::Physical {
            mass: 1.0,
            normal_gravity: 20.0,
            track_gravity: 20.0,
            ..Default::default()
        },
        dimensions: params::Dimensions {
            width: 4.0,
            height: 2.0,
            length: 8.0,
            shield: 100.0,
            ..Default::default()
        },
        airbrake: params::Airbrake {
            amount: 0.0005,
            drag: 1.0,
            gain: 400.0,
            falloff: 400.0,
            turn: 2.0,
            slidegrip: 0.004,
            sideshift: 20.0,
        },
        brakes: params::Brakes {
            amount: -0.4,
            gain: 400.0,
            falloff: 400.0,
        },
        ..Handling::ZERO
    }
}

/// The tuning this invented craft is steered with: the closed-loop fixture's.
fn tuning() -> Tuning {
    Tuning {
        lateral_accel: 55.0,
        ..Tuning::default()
    }
}

/// How far wide of its line the craft may run before the plan calls it off.
const OFF_LINE: f32 = 15.0;

/// A stadium: straights of `straight`, hairpins of `radius`. Built through
/// `oag_core::math::sin_cos`, so the fixture is the same bits everywhere. The
/// platform gate is
/// `tests/determinism.rs`; `a_build_is_bit_identical_twice` below only checks
/// that two builds in one process agree.
fn stadium(radius: f32, straight: f32) -> Line {
    let spacing = 2.5f32;
    let mut points = Vec::new();
    let half = straight * 0.5;
    let steps = (straight / spacing) as usize;
    for step in 0..steps {
        points.push(Vec3::new(
            radius,
            0.0,
            -half + straight * step as f32 / steps as f32,
        ));
    }
    let arc = (std::f32::consts::PI * radius / spacing) as usize;
    for step in 0..arc {
        let (s, c) = sin_cos(std::f32::consts::PI * step as f32 / arc as f32);
        points.push(Vec3::new(radius * c, 0.0, half + radius * s));
    }
    for step in 0..steps {
        points.push(Vec3::new(
            -radius,
            0.0,
            half - straight * step as f32 / steps as f32,
        ));
    }
    for step in 0..arc {
        let (s, c) = sin_cos(std::f32::consts::PI * (1.0 + step as f32 / arc as f32));
        points.push(Vec3::new(radius * c, 0.0, -half + radius * s));
    }
    Line::new(points)
}

/// A craft at rest on the line at `index`, nose along it.
fn placed(line: &Line, index: usize) -> ShipState {
    let handling = handling();
    let here = line.point(index);
    let heading = (line.point(index + 1) - here).normalize_or_zero();
    let mut state = ShipState {
        body: Body {
            position: here + Vec3::Y * handling.antigrav.ride_height * 0.5,
            orientation: Quat::from_rotation_arc(Vec3::NEG_Z, heading),
            mass: handling.physical.mass,
            ..Body::default()
        },
        ..ShipState::default()
    };
    oag_physics::damage::reset(&mut state, &handling.dimensions);
    state
}

fn craft(line: &Line) -> Craft {
    Craft {
        handling: handling(),
        start: placed(line, 0),
        start_index: 0,
    }
}

/// Builds the plan for a stadium.
fn build_on(line: &Line) -> (SpeedPlan, Report) {
    let respawn = |index: usize| placed(line, index);
    let course = Course {
        line,
        samples: &[],
        raycaster: &Plane,
        env: Environment::default(),
        dt: 1.0 / 60.0,
        off_line: OFF_LINE,
        respawn: &respawn,
        reset: &|_, _, _| false,
        max_airborne: f32::INFINITY,
        pad: &|_, _| None,
    };
    SpeedPlan::build(&course, &craft(line), &tuning())
}

/// Runs `plan`'s own verification lap pair and returns its report.
fn verify(line: &Line, plan: &SpeedPlan) -> Report {
    let respawn = |index: usize| placed(line, index);
    let course = Course {
        line,
        samples: &[],
        raycaster: &Plane,
        env: Environment::default(),
        dt: 1.0 / 60.0,
        off_line: OFF_LINE,
        respawn: &respawn,
        reset: &|_, _, _| false,
        max_airborne: f32::INFINITY,
        pad: &|_, _| None,
    };
    let craft = craft(line);
    let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
    let mut report = Report::default();
    plan.clone()
        .verify(&course, &craft, &tuning(), yaw, &mut report);
    report
}

fn tight() -> Line {
    stadium(50.0, 1500.0)
}

#[test]
fn full_throttle_runs_wide_of_the_hairpins() {
    // The premise every other test leans on: without a plan the craft does not
    // get round. If this ever passes clean, the fixture stopped testing anything.
    let line = tight();
    let report = verify(&line, &SpeedPlan::unlimited(&line));
    assert!(
        report.verify_failures > 0,
        "an unplanned craft got round clean, so the fixture is too easy: {report:?}"
    );
}

#[test]
fn the_built_plan_gets_round_clean() {
    let line = tight();
    let (plan, report) = build_on(&line);
    assert_eq!(report.verify_failures, 0, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    assert!(report.verify_lap_ticks.is_some(), "{report:?}");
    // Re-verified from outside the build, so the report is not the only witness.
    assert_eq!(verify(&line, &plan).verify_failures, 0);
}

#[test]
fn the_plan_limits_the_hairpins_and_not_the_straights() {
    let line = tight();
    let (plan, _) = build_on(&line);
    let n = line.len();
    let straight = (1500.0 / 2.5) as usize;
    let arc = n / 2 - straight;
    let apex = straight + arc / 2;
    assert!(
        plan.ceiling(apex).is_finite(),
        "the first hairpin's apex is unlimited"
    );
    // Mid-straight the plan asks for a speed far past the hairpin's: the
    // straight is driven flat out, not at a corner speed.
    let mid = straight / 3;
    assert!(
        plan.target(mid) > 3.0 * plan.ceiling(apex),
        "mid-straight target {} against apex ceiling {}",
        plan.target(mid),
        plan.ceiling(apex)
    );
}

#[test]
fn the_backward_pass_brakes_before_the_corner_not_in_it() {
    let line = tight();
    let (plan, _) = build_on(&line);
    // The first sample with a finite ceiling is where the learned limit
    // starts; the braking zone is the stretch before it whose target is
    // finite only because of the backward pass.
    // Searched from mid-straight: the far hairpin's exit wraps onto sample 0.
    let straight = (1500.0 / 2.5) as usize;
    let first_limit = (straight / 2..line.len())
        .find(|&i| plan.ceiling(i).is_finite())
        .expect("a hairpin is limited");
    let before = first_limit - 5;
    assert!(plan.ceiling(before).is_infinite());
    assert!(
        plan.target(before).is_finite(),
        "nothing brakes ahead of the first limited sample"
    );
    let zone: Vec<f32> = (before - 20..=first_limit)
        .map(|i| plan.target(i))
        .collect();
    assert!(
        zone.windows(2).all(|pair| pair[1] <= pair[0]),
        "the braking zone is not monotone: {zone:?}"
    );
}

#[test]
fn the_backward_pass_is_the_braking_law_and_nothing_else() {
    // One limited sample on an otherwise free line: every sample before it
    // reads `v0^2 = v1^2 + 2 a ds` off the one after, with `a` the table's,
    // until the speed is past anything the table could brake from.
    let line = stadium(50.0, 1500.0);
    let mut plan = SpeedPlan::unlimited(&line);
    // A strong, flat brake, so the zone ends well inside the ring.
    let mut sum = [0.0f32; 64];
    let mut count = [0u32; 64];
    sum[0] = 1000.0;
    count[0] = 1;
    plan.decel = Decel::from_samples(&sum, &count);
    let at = 400;
    plan.ceiling[at] = 30.0;
    plan.derive_targets();
    assert_eq!(plan.target(at), 30.0);
    assert!(
        plan.target(at + 1).is_infinite(),
        "braking leaks past the limit"
    );
    for i in (at - 50..at).rev() {
        let next = plan.target(i + 1);
        let a = plan.decel().at(next);
        let expected = (next * next + 2.0 * a * plan.spacing[i]).sqrt();
        assert_eq!(plan.target(i).to_bits(), expected.to_bits(), "sample {i}");
    }
    // And a held sample neither brakes nor is braked through.
    plan.hold[at - 10] = true;
    plan.derive_targets();
    assert!(plan.target(at - 10).is_infinite());
    assert!(plan.target(at - 11).is_infinite());
    assert!(plan.target(at - 9).is_finite());
}

#[test]
fn a_build_is_bit_identical_twice() {
    let line = tight();
    let (a, ra) = build_on(&line);
    let (b, rb) = build_on(&line);
    let bits = |plan: &SpeedPlan| -> Vec<u32> {
        (0..plan.len())
            .flat_map(|i| [plan.ceiling(i).to_bits(), plan.target(i).to_bits()])
            .collect()
    };
    assert_eq!(bits(&a), bits(&b));
    assert_eq!(ra, rb);
}

#[test]
fn the_follower_brakes_in_proportion_and_never_while_under() {
    assert_eq!(longitudinal(50.0, 60.0), (1.0, 0.0));
    assert_eq!(longitudinal(60.0, 60.0), (1.0, 0.0));
    let (thrust, brake) = longitudinal(61.0, 60.0);
    assert_eq!(thrust, 0.0);
    assert!(brake > 0.0 && brake < 1.0);
    assert_eq!(longitudinal(90.0, 60.0), (0.0, 1.0));
    assert_eq!(longitudinal(10.0, f32::INFINITY), (1.0, 0.0));
}

#[test]
fn a_brake_keeps_the_differential_it_is_folded_with() {
    // `right - left` survives whole whatever the brake, so the yaw the driver
    // asked for is not traded away for the deceleration.
    for brake in [0.0f32, 0.3, 0.9, 1.0] {
        for differential in [-0.6f32, 0.0, 0.25] {
            let (left, right) = airbrakes(brake, differential);
            assert!(
                (right - left - differential).abs() < 1e-6,
                "{brake} {differential}"
            );
            assert!((0.0..=1.0).contains(&left) && (0.0..=1.0).contains(&right));
        }
    }
}

#[test]
fn an_unmeasured_speed_borrows_the_nearest_measured_one_below() {
    let mut sum = [0.0f32; 64];
    let mut count = [0u32; 64];
    sum[5] = 100.0;
    count[5] = 2;
    sum[9] = 300.0;
    count[9] = 3;
    let decel = Decel::from_samples(&sum, &count);
    assert_eq!(decel.at(55.0), 50.0 * 0.8);
    assert_eq!(decel.at(75.0), 50.0 * 0.8);
    assert_eq!(decel.at(95.0), 100.0 * 0.8);
    assert_eq!(decel.at(500.0), 100.0 * 0.8);
    assert_eq!(decel.at(5.0), 50.0 * 0.8);
    assert_eq!(
        Decel::from_samples(&[0.0; 64], &[0; 64]),
        Decel::FALLBACK,
        "nothing measured is the fallback"
    );
}

/// A craft at `speed` along the stadium's first straight.
fn moving(line: &Line, speed: f32) -> ShipState {
    let mut state = placed(line, 10);
    let heading = (line.point(11) - line.point(10)).normalize_or_zero();
    state.body.linear_velocity = heading * speed;
    state
}

#[test]
fn a_driver_on_a_plan_brakes_for_the_plan_and_not_without_it() {
    // The wiring: `Context::plan` is what decides the longitudinal controls
    // when it is there. A straight the corner model would take flat out, with
    // a plan that says 20, is braked for; the same plan unlimited is not.
    let line = tight();
    let tuning = tuning();
    let state = moving(&line, 60.0);
    let mut slow = SpeedPlan::unlimited(&line);
    slow.ceiling.fill(20.0);
    slow.derive_targets();

    let drive = |plan: Option<&SpeedPlan>| {
        let mut driver = Driver {
            index: 10,
            ..Driver::default()
        };
        driver.drive(
            &state,
            &Context {
                plan,
                ..Context::new(&line, &tuning)
            },
        )
    };
    let braked = drive(Some(&slow));
    assert_eq!(braked.thrust, 0.0);
    assert!(braked.airbrake_left > 0.0 && braked.airbrake_right > 0.0);

    let free = drive(Some(&SpeedPlan::unlimited(&line)));
    assert_eq!(free.thrust, 1.0);
    assert_eq!(
        drive(None).thrust,
        1.0,
        "the corner model brakes on a straight"
    );
}

#[test]
fn a_plan_of_a_different_line_is_ignored() {
    // A plan whose length is not the line's was built for another layout, and
    // following it would brake at the wrong places.
    let line = tight();
    let tuning = tuning();
    let state = moving(&line, 60.0);
    let other = stadium(40.0, 300.0);
    let mut wrong = SpeedPlan::unlimited(&other);
    wrong.ceiling.fill(20.0);
    wrong.derive_targets();
    let mut driver = Driver {
        index: 10,
        ..Driver::default()
    };
    let controls = driver.drive(
        &state,
        &Context {
            plan: Some(&wrong),
            ..Context::new(&line, &tuning)
        },
    );
    assert_eq!(controls.thrust, 1.0);
}

#[test]
fn a_lower_level_holds_its_share_of_the_plans_pace_on_a_straight() {
    // An unlimited plan whose verification did 60 here: an Ace at 60 holds
    // the throttle, a level with a 0.88 pace share brakes to it.
    let line = tight();
    let state = moving(&line, 60.0);
    let mut plan = SpeedPlan::unlimited(&line);
    plan.pace.fill(60.0);
    let drive = |level: crate::Difficulty| {
        let tuning = level.tune(&tuning());
        let mut driver = Driver {
            index: 10,
            ..Driver::default()
        };
        driver.drive(
            &state,
            &Context {
                plan: Some(&plan),
                ..Context::new(&line, &tuning)
            },
        )
    };
    assert_eq!(drive(crate::Difficulty::Ace).thrust, 1.0);
    assert_eq!(drive(crate::Difficulty::Elite).thrust, 1.0);
    let novice = drive(crate::Difficulty::Novice);
    assert_eq!(novice.thrust, 0.0);
    assert!(novice.airbrake_left > 0.0);
}
