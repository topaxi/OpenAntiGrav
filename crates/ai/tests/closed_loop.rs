//! Drives a real craft round a real oval, with the real force law.
//!
//! # Why this exists
//!
//! The controller's unit tests ask what it holds on one tick from one pose. That
//! cannot see the failure this file was written for, because the failure is a
//! *loop* property: the first version of this AI tracked the line perfectly on
//! any single tick and drove wall to wall in a race, which is what the plant
//! makes inevitable.
//!
//! **The plant is a double integrator with a lag in front of it.**
//! `oag_physics::engine::steering` feeds `Accumulators::local_angular`, which is
//! **torque** - so the steering input commands yaw *acceleration*, not yaw rate
//! and not heading. On top of that `controls::ramp_steering` moves the steering
//! state toward its target at a finite rate, so the input itself lags. A
//! controller that sets steering proportionally to how far off the line it is
//! will overshoot, correct, overshoot the other way, and keep doing it: that is
//! a limit cycle, not a tuning error, and no gain fixes it.
//!
//! So the assertion here is **how often the craft crosses its own line**. A craft
//! tracking a line crosses it a few times settling in. A craft in a wall-to-wall
//! wave crosses it on every swing.
//!
//! # The numbers that justified the rewrite
//!
//! **This test was written first and watched failing.** Against the
//! proportional-on-error controller it replaced, on the oval below over 1,800
//! ticks:
//!
//! | | peak error from the line |
//! | --- | --- |
//! | proportional on error | **84.1** |
//! | yaw-rate tracking | **7.3** |
//!
//! On a line whose corners are 120 units across, 84 units off it is not tracking
//! anything.
//!
//! **Amplitude is what is asserted, and counting line crossings was tried and
//! discarded.** The two controllers cross the line about as often as each other
//! (19 against 16 over the same run), because a craft that tracks a line tightly
//! still crosses it at every corner entry and exit - four to eight times a lap
//! on this oval. The count measures the geometry more than the controller. How
//! far it goes is what distinguishes tracking from weaving.
//!
//! # What this is not
//!
//! **It is not the game.** The surface is an infinite flat plane, the handling is
//! invented (see [`handling`]), and a real circuit rolls, banks, narrows and has
//! walls. Nothing here says the field gets round `16_Track` - that is
//! `race_ground_truth::the_ai_drives_the_field_along_the_track`, which needs a
//! disc image. What this says is that the controller is stable against a plant of
//! the right *shape*, which is the property that was broken.

use oag_ai::{Driver, Line, Tuning};
use oag_core::math::{Quat, Vec3};
use oag_physics::{
    Body, Environment, Handling, Ray, RaycastHit, Raycaster, ShipState, Surface, params,
};

/// An infinite horizontal floor at `y = 0`.
///
/// Enough for the hover probes to find a surface and for the craft to have grip;
/// there are no walls, so a craft that leaves the line simply keeps going, which
/// is what lets the cross-track error be measured rather than clipped.
struct Plane;

impl Raycaster for Plane {
    fn raycast(&self, ray: Ray, _skip: Option<u32>, _include_reset: bool) -> Option<RaycastHit> {
        // Only downward rays meet the floor.
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

/// A craft that can drive, with invented numbers.
///
/// **None of these is the game's.** Per ADR-0006 no shipped value is in this
/// repository, and these were picked to give a craft that accelerates, corners
/// and settles on the plane above - round figures a reader can check the test's
/// arithmetic against, not a measurement of anything. The one that matters to
/// what is being asserted is `turning.amount`, because it is the gain of the
/// double integrator the controller has to be stable against.
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
        ..Handling::ZERO
    }
}

/// An oval: two 120-unit-radius half circles joined by 400-unit straights.
///
/// Deliberately not a circle. A constant-curvature loop is the easy case - the
/// controller can settle on one steering value and stay there. What excites an
/// oscillator is the *transitions*, entering and leaving the corners, which is
/// where a real circuit's problems are too.
fn oval() -> Line {
    let radius = 120.0f32;
    let straight = 400.0f32;
    let spacing = 2.5f32;
    let mut points = Vec::new();

    let push_straight = |points: &mut Vec<Vec3>, x: f32, from: f32, to: f32| {
        let steps = ((to - from).abs() / spacing) as usize;
        for step in 0..steps {
            let t = step as f32 / steps as f32;
            points.push(Vec3::new(x, 0.0, from + (to - from) * t));
        }
    };
    let push_arc = |points: &mut Vec<Vec3>, centre: Vec3, from: f32, to: f32| {
        let steps = ((to - from).abs() * radius / spacing) as usize;
        for step in 0..steps {
            let angle = from + (to - from) * step as f32 / steps as f32;
            points.push(centre + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin()));
        }
    };

    let half = straight * 0.5;
    push_straight(&mut points, radius, -half, half);
    push_arc(
        &mut points,
        Vec3::new(0.0, 0.0, half),
        0.0,
        std::f32::consts::PI,
    );
    push_straight(&mut points, -radius, half, -half);
    push_arc(
        &mut points,
        Vec3::new(0.0, 0.0, -half),
        std::f32::consts::PI,
        std::f32::consts::TAU,
    );
    Line::new(points)
}

/// Signed distance from the craft to its line, across the line.
fn cross_track(line: &Line, index: usize, position: Vec3) -> f32 {
    let here = line.point(index);
    let next = line.point(index + 1);
    let along = (next - here).normalize_or_zero();
    let across = Vec3::Y.cross(along);
    (position - here).dot(across)
}

/// Runs the craft round the oval and reports what happened.
struct Run {
    /// The furthest it ever got from the line, either side.
    peak_error: f32,
    /// Mean distance from the line over the run.
    mean_error: f32,
    /// How far along the line it travelled, in points.
    progress: usize,
}

fn drive_the_oval(ticks: usize) -> Run {
    drive_the_oval_with(ticks, Tuning::default())
}

fn drive_the_oval_with(ticks: usize, tuning: Tuning) -> Run {
    let line = oval();
    let handling = handling();
    let mut driver = Driver::default();

    // On the line, pointing along it, at the ride height, at rest.
    let start = line.point(0);
    let heading = (line.point(1) - start).normalize_or_zero();
    let mut state = ShipState {
        body: Body {
            position: start + Vec3::Y * handling.antigrav.ride_height * 0.5,
            // `Body::forward` is `-Z`, so the rotation that takes `-Z` onto the
            // line's own direction is the craft's start pose.
            orientation: Quat::from_rotation_arc(Vec3::NEG_Z, heading),
            mass: handling.physical.mass,
            ..Body::default()
        },
        ..ShipState::default()
    };
    oag_physics::damage::reset(&mut state, &handling.dimensions);

    let env = Environment::default();
    let dt = 1.0 / 60.0;

    let mut peak_error = 0.0f32;
    let mut total_error = 0.0f32;
    let mut samples = 0usize;
    let mut laps = 0usize;
    let mut last_index = 0usize;

    for tick in 0..ticks {
        let controls = driver.drive(&state, &line, &tuning);
        oag_physics::step(&mut state, &controls, &handling, &env, &Plane, dt);

        let index = driver.index as usize;
        if index < last_index && last_index - index > line.len() / 2 {
            laps += 1;
        }
        last_index = index;

        let error = cross_track(&line, index, state.body.position);
        // Ignore the first second: the craft starts at rest and the settling
        // transient is not the property under test.
        if tick > 60 {
            peak_error = peak_error.max(error.abs());
            total_error += error.abs();
            samples += 1;
        }
        assert!(
            state.body.position.is_finite(),
            "the craft left the world on tick {tick}"
        );
    }

    Run {
        peak_error,
        mean_error: total_error / samples.max(1) as f32,
        progress: laps * line.len() + last_index,
    }
}

/// The regression this file exists for.
///
/// The bounds are loose against the rewritten controller (7.3 peak, ~2 mean) and
/// nowhere near the broken one (84.1 peak), so this catches a return of the wave
/// rather than grading the driving. Tightening them to the current numbers would
/// make every future tuning change a test failure.
#[test]
fn a_craft_settles_onto_the_line_instead_of_weaving() {
    let run = drive_the_oval(1800);
    assert!(
        run.peak_error < 20.0,
        "the craft got {:.1} units from its line, which is a weave rather than a wobble",
        run.peak_error
    );
    assert!(
        run.mean_error < 6.0,
        "the craft averaged {:.1} units from its line",
        run.mean_error
    );
}

/// Stability is worth nothing if it is achieved by stopping. The craft has to
/// get round, which on this oval means covering most of it inside 30 seconds.
#[test]
fn a_craft_makes_progress_round_the_oval() {
    let run = drive_the_oval(1800);
    let line = oval();
    assert!(
        run.progress > line.len(),
        "the craft covered {} of {} points, so it did not complete the oval",
        run.progress,
        line.len()
    );
}
