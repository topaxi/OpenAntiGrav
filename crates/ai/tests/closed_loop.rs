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

use oag_ai::{Driver, Frame, Line, Tuning};
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

/// The same craft, with airbrakes that do something.
///
/// **A second fixture rather than an extension of the first, and the split is
/// load-bearing.** [`handling`] leaves `airbrake` and `brakes` at
/// `Handling::ZERO`, so in it a driver's airbrake commands go nowhere at all:
/// the ramped states never leave zero, `ShipState::brake` never rises, and
/// every `imbalance` term in `oag_physics::airbrake` is zero. Braking there is
/// `thrust = 0` and nothing else. The bounds in
/// [`a_craft_settles_onto_the_line_instead_of_weaving`] were calibrated against
/// exactly that craft, so giving it airbrakes would silently re-baseline the
/// regression this file exists for.
/// [`the_default_fixture_has_no_airbrakes_and_the_regression_bounds_know_it`]
/// pins the split.
///
/// **None of these is the game's**, per ADR-0006 - round figures picked to give
/// a craft whose airbrakes slow it, yaw it and cost it grip on the same order
/// as its other forces, so the differential has something to be measured
/// against.
fn handling_with_airbrakes() -> Handling {
    Handling {
        airbrake: params::Airbrake {
            // A lateral force gain, not a drag, and already scaled by 1e-4 -
            // see `docs/physics/README.md` and `params::Airbrake`.
            amount: 0.0005,
            drag: 1.0,
            gain: 400.0,
            falloff: 400.0,
            // Sized against the steering it assists: `engine::steering` gives a
            // yaw torque of `steer * turning.amount`, so up to 100 here, while
            // this term gives `speed * turn * imbalance * 0.001` - about 18 at
            // 150 units a second and a full differential. An assist worth a
            // fifth of the stick, which is what an assist should be.
            turn: 2.0,
            // Already scaled by 1e-4, so this lives on `0.0..=0.01`: 0.004 is
            // forty per cent of the lateral grip kept at full airbrake.
            slidegrip: 0.004,
            sideshift: 20.0,
        },
        brakes: params::Brakes {
            // **Already scaled by -0.01 and so negative** - the force is applied
            // along `+unit(velocity)` and the sign is carried here.
            amount: -0.4,
            gain: 400.0,
            falloff: 400.0,
        },
        ..handling()
    }
}

/// An oval: two 120-unit-radius half circles joined by 400-unit straights.
///
/// Deliberately not a circle. A constant-curvature loop is the easy case - the
/// controller can settle on one steering value and stay there. What excites an
/// oscillator is the *transitions*, entering and leaving the corners, which is
/// where a real circuit's problems are too.
fn oval() -> Line {
    oval_of(120.0, 400.0)
}

fn oval_of(radius: f32, straight: f32) -> Line {
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

    // A corridor `CORRIDOR` units either side, so a seeded driver has somewhere
    // to be other than the line. A real track's comes off the disc, one bound
    // per control point; this one is even, because what is under test is the
    // driver and not the shape of a corridor.
    //
    // **The frame is built from the line itself and world up**, which is only
    // right because this plane is flat. A real track rolls, and there the axis
    // is the sample's own `lateral` - see `docs/formats/track.md`.
    let corridor = (0..points.len())
        .map(|index| {
            let here = points[index];
            let next = points[(index + 1) % points.len()];
            let along = (next - here).normalize_or_zero();
            Frame {
                lateral: along.cross(Vec3::Y).normalize_or_zero(),
                left: -CORRIDOR,
                right: CORRIDOR,
            }
        })
        .collect();
    Line::with_corridor(points, corridor)
}

/// How much room the oval's corridor gives either side of the line.
const CORRIDOR: f32 = 12.0;

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
    /// Mean *signed* distance, positive to the left of the line.
    ///
    /// This is the one that says which part of the corridor a driver held. The
    /// unsigned mean above cannot: two craft running the same distance off the
    /// line on opposite sides have the same one.
    mean_offset: f32,
    /// How far along the line it travelled, in points.
    progress: usize,
    /// How often the steering command crossed zero, counting only crossings
    /// where it had committed to a side first.
    ///
    /// A ringing loop reverses far more often than a tracking one, so this is
    /// the guard on adding a second path into the steering.
    steer_reversals: usize,
    /// Ticks the steering command spent pinned at full lock.
    ///
    /// **What the differential is for, measured at the outcome rather than at
    /// the trigger.** Saturation is the loop asking for more yaw than the
    /// steering input can deliver; an assist that supplies some of it gets the
    /// craft onto the rate it wanted sooner, so the loop comes off the stop
    /// earlier. Counting the ticks the assist *fired* would just be counting
    /// its own gate.
    saturated_ticks: usize,
}

fn drive_the_oval(ticks: usize) -> Run {
    drive_the_oval_with(ticks, Tuning::default())
}

fn drive_the_oval_with(ticks: usize, tuning: Tuning) -> Run {
    drive_the_oval_as(ticks, tuning, Driver::default(), &handling(), &oval())
}

fn drive_the_oval_as(
    ticks: usize,
    tuning: Tuning,
    mut driver: Driver,
    handling: &Handling,
    line: &Line,
) -> Run {
    let handling = *handling;

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
    let mut total_offset = 0.0f32;
    let mut samples = 0usize;
    let mut laps = 0usize;
    let mut last_index = 0usize;
    let mut steer_reversals = 0usize;
    let mut saturated_ticks = 0usize;
    let mut committed = 0.0f32;

    for tick in 0..ticks {
        let controls = driver.drive(&state, line, &tuning);
        oag_physics::step(&mut state, &controls, &handling, &env, &Plane, dt);

        let index = driver.index as usize;
        if index < last_index && last_index - index > line.len() / 2 {
            laps += 1;
        }
        last_index = index;

        let error = cross_track(line, index, state.body.position);
        // Ignore the first second: the craft starts at rest and the settling
        // transient is not the property under test.
        if tick > 60 {
            peak_error = peak_error.max(error.abs());
            if controls.steer_x.abs() >= 1.0 {
                saturated_ticks += 1;
            }
            total_error += error.abs();
            total_offset += error;
            samples += 1;

            // Only a command that picked a side counts as having a side to
            // reverse from, or every wander through zero scores.
            if controls.steer_x.abs() > 0.1 {
                let side = controls.steer_x.signum();
                if committed != 0.0 && side != committed {
                    steer_reversals += 1;
                }
                committed = side;
            }
        }
        assert!(
            state.body.position.is_finite(),
            "the craft left the world on tick {tick}"
        );
    }

    Run {
        peak_error,
        mean_error: total_error / samples.max(1) as f32,
        mean_offset: total_offset / samples.max(1) as f32,
        progress: laps * line.len() + last_index,
        steer_reversals,
        saturated_ticks,
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

/// A grid's worth of seeded drivers, each driven round the same oval.
fn drive_the_field(ticks: usize) -> Vec<Run> {
    (1..8)
        .map(|slot| {
            drive_the_oval_as(
                ticks,
                Tuning::default(),
                Driver::for_slot(0xC0FFEE, slot),
                &handling(),
                &oval(),
            )
        })
        .collect()
}

/// **The reason personalities exist.** Seven drivers on one line hold seven
/// different parts of the corridor, rather than the same centimetre of it.
///
/// Asserted on the *signed* mean offset, and on the spread between drivers
/// rather than on any driver's own number: what is wrong with a field that
/// tracks one line is that they agree, and this is the direct measurement of
/// them not agreeing.
#[test]
fn a_field_of_seeded_drivers_does_not_drive_one_line() {
    let runs = drive_the_field(1800);
    let offsets: Vec<f32> = runs.iter().map(|run| run.mean_offset).collect();

    let low = offsets.iter().copied().fold(f32::INFINITY, f32::min);
    let high = offsets.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        high - low > 4.0,
        "the field spread over {:.1} units of a {:.0}-unit corridor: {offsets:?}",
        high - low,
        CORRIDOR * 2.0
    );

    // And the spread is the field's, not two outliers with five craft still
    // nose to tail. Mean absolute deviation rather than a nearest-neighbour
    // check: **two drivers are allowed to pick similar lines**, the same way two
    // human drivers are, and a test that forbade it would be pinning the
    // distribution's luck rather than the property under test.
    let mean = offsets.iter().sum::<f32>() / offsets.len() as f32;
    let deviation = offsets
        .iter()
        .map(|offset| (offset - mean).abs())
        .sum::<f32>()
        / offsets.len() as f32;
    assert!(
        deviation > 1.5,
        "the field deviates {deviation:.1} units from its own mean: {offsets:?}"
    );
}

/// The spread is the corridor's to give. A driver that used more of it than
/// [`Tuning::corridor_use`] allows would be relying on a clamp that only knows
/// about the point it is aiming at, and on a real track the room either side of
/// the line is the only thing between the field and the scenery.
#[test]
fn a_seeded_driver_stays_inside_the_corridor() {
    // The lag between aiming and arriving, which the corridor does not bound:
    // the controller's own tracking error, measured at 7.3 units peak on this
    // oval by the run above. Loose for the same reason that bound is loose.
    const TRACKING: f32 = 12.0;

    for (slot, run) in drive_the_field(1800).iter().enumerate() {
        assert!(
            run.peak_error < CORRIDOR + TRACKING,
            "driver {slot} got {:.1} units from the line, past a corridor of {CORRIDOR}",
            run.peak_error
        );
    }
}

/// Different corner speeds are what open a gap and close it again, so the field
/// must not arrive at the same place on the same tick either.
#[test]
fn a_field_of_seeded_drivers_strings_out() {
    let progress: Vec<usize> = drive_the_field(1800)
        .iter()
        .map(|run| run.progress)
        .collect();
    let low = progress.iter().copied().min().unwrap_or(0);
    let high = progress.iter().copied().max().unwrap_or(0);
    assert!(
        high > low,
        "every driver covered exactly {high} points, so nothing separates them"
    );
    // And they all still got round: a spread produced by one craft stopping is
    // not a race.
    assert!(
        low > oval().len(),
        "the slowest driver covered {low} of {} points",
        oval().len()
    );
}

/// The seed is the whole of a driver's character, so the same seed has to give
/// the same drive - twice in one process, and by extension in a replay.
#[test]
fn the_same_seed_drives_the_same_race() {
    let one = drive_the_oval_as(
        600,
        Tuning::default(),
        Driver::for_slot(7, 3),
        &handling(),
        &oval(),
    );
    let two = drive_the_oval_as(
        600,
        Tuning::default(),
        Driver::for_slot(7, 3),
        &handling(),
        &oval(),
    );
    assert_eq!(one.progress, two.progress);
    assert_eq!(one.mean_offset, two.mean_offset);
    assert_eq!(one.peak_error, two.peak_error);
}

/// The finding that made [`handling_with_airbrakes`] necessary, pinned so the
/// next reader does not helpfully merge the two fixtures.
///
/// Every number this file's module docs quote - the 84.1 against the 7.3, the
/// bounds in [`a_craft_settles_onto_the_line_instead_of_weaving`] - was
/// measured against a craft whose airbrakes do **nothing**: `Handling::ZERO`
/// leaves `airbrake.gain` at zero, so the ramped states never move off zero
/// whatever the driver commands, and `brakes.amount` at zero, so the brake
/// state has nothing to apply. Braking in that fixture is `thrust = 0`. That is
/// deliberate, and it is why the differential is invisible to the six tests
/// above rather than silently changing them.
#[test]
fn the_default_fixture_has_no_airbrakes_and_the_regression_bounds_know_it() {
    let plain = handling();
    assert_eq!(plain.airbrake, params::Airbrake::default());
    assert_eq!(plain.brakes, params::Brakes::default());
    assert_eq!(plain.airbrake.gain, 0.0, "an airbrake that cannot ramp");
    assert_eq!(plain.brakes.amount, 0.0, "a brake that applies no force");

    let braked = handling_with_airbrakes();
    assert_ne!(braked.airbrake, plain.airbrake);
    assert_ne!(braked.brakes, plain.brakes);
}

/// The guard on the new fixture: if it were inert too, everything below would
/// pass by measuring nothing.
#[test]
fn the_airbrake_fixture_actually_slows_and_yaws_a_craft() {
    let handling = handling_with_airbrakes();
    let env = Environment::default();
    let dt = 1.0 / 60.0;

    let start = || {
        let mut state = ShipState {
            body: Body {
                position: Vec3::Y * handling.antigrav.ride_height * 0.5,
                linear_velocity: Vec3::new(0.0, 0.0, -200.0),
                mass: handling.physical.mass,
                ..Body::default()
            },
            ..ShipState::default()
        };
        oag_physics::damage::reset(&mut state, &handling.dimensions);
        state
    };

    let run = |controls: oag_physics::ShipControls| {
        let mut state = start();
        for _ in 0..60 {
            oag_physics::step(&mut state, &controls, &handling, &env, &Plane, dt);
        }
        state
    };

    let coasting = run(oag_physics::ShipControls::default());
    let braking = run(oag_physics::ShipControls {
        airbrake_left: 1.0,
        airbrake_right: 1.0,
        ..Default::default()
    });
    let forward = |state: &ShipState| -state.body.linear_velocity.z;
    assert!(
        forward(&braking) < forward(&coasting) - 1.0,
        "both airbrakes should slow it: {} against {}",
        forward(&braking),
        forward(&coasting)
    );

    // One side alone, with no steering, must rotate the craft toward that side.
    let left_only = run(oag_physics::ShipControls {
        airbrake_left: 1.0,
        ..Default::default()
    });
    // Positive yaw about the craft's own up is a turn to the **left**.
    assert!(
        left_only.body.angular_velocity.y > 1.0e-3,
        "the left airbrake alone should yaw it left, got {}",
        left_only.body.angular_velocity.y
    );
}

/// The stability guard. A second path into a loop that already oscillates is
/// what this crate was rewritten to remove, so the differential has to earn its
/// place without widening the wave or reversing more often.
#[test]
fn a_differential_braking_driver_does_not_ring() {
    let symmetric = Tuning {
        trail_gain: 0.0,
        trail_max: 0.0,
        ..Tuning::default()
    };
    let braked = handling_with_airbrakes();
    let with = drive_the_oval_as(1800, Tuning::default(), Driver::default(), &braked, &oval());
    let without = drive_the_oval_as(1800, symmetric, Driver::default(), &braked, &oval());

    assert!(
        with.peak_error < 20.0,
        "the craft got {:.1} units from its line, which is a weave rather than a wobble",
        with.peak_error
    );
    assert!(
        with.mean_error < 6.0,
        "the craft averaged {:.1} units from its line",
        with.mean_error
    );
    assert!(
        with.steer_reversals <= without.steer_reversals + 4,
        "the differential added reversals: {} against {}",
        with.steer_reversals,
        without.steer_reversals
    );
}

/// And what it buys, on a corner the steering alone cannot hold.
///
/// **The measurement that justifies the feature, and it only shows up where the
/// design says it should.** On [`oval`]'s 120-unit corners the craft is not
/// grip-limited - it spends about twenty ticks of an 1,800-tick run at full
/// lock - so there is no understeer to help with and the differential is
/// noise. Tighten the corners to 60 units and the same craft spends over seven
/// hundred ticks pinned at lock, which is the loop saying it cannot produce the
/// yaw it wants. That is the case this exists for.
///
/// Measured over 1,800 ticks on the 60-unit oval:
///
/// | | peak error | ticks at full lock |
/// | --- | --- | --- |
/// | symmetric braking | 21.4 | 776 |
/// | with the differential | **19.2** | **739** |
#[test]
fn a_differential_holds_a_corner_the_steering_alone_cannot() {
    let symmetric = Tuning {
        trail_gain: 0.0,
        trail_max: 0.0,
        ..Tuning::default()
    };
    let braked = handling_with_airbrakes();
    // Corners the craft genuinely cannot make on the stick alone.
    let tight = oval_of(60.0, 300.0);
    let with = drive_the_oval_as(1800, Tuning::default(), Driver::default(), &braked, &tight);
    let without = drive_the_oval_as(1800, symmetric, Driver::default(), &braked, &tight);

    assert!(
        without.saturated_ticks > 300,
        "this fixture only means anything while the loop is out of lock a lot, \
         and it was only there for {} ticks",
        without.saturated_ticks
    );
    assert!(
        with.peak_error < without.peak_error,
        "the differential should hold the corner better: {:.2} against {:.2}",
        with.peak_error,
        without.peak_error
    );
    assert!(
        with.saturated_ticks < without.saturated_ticks,
        "and get the craft off the stop sooner: {} against {}",
        with.saturated_ticks,
        without.saturated_ticks
    );
}

/// The other half of the claim: where the craft can already make the corner,
/// the differential stays out of the way.
///
/// It is not exactly nothing - it costs about a tenth of a unit of peak error
/// on the wide oval, against 7.4 - and that is recorded here rather than
/// hidden, because a bound that pretended it was zero would be a bound that
/// fails on the next tuning change for no reason worth investigating.
#[test]
fn a_differential_barely_touches_a_corner_the_craft_can_already_make() {
    let symmetric = Tuning {
        trail_gain: 0.0,
        trail_max: 0.0,
        ..Tuning::default()
    };
    let braked = handling_with_airbrakes();
    let with = drive_the_oval_as(1800, Tuning::default(), Driver::default(), &braked, &oval());
    let without = drive_the_oval_as(1800, symmetric, Driver::default(), &braked, &oval());

    assert!(
        without.saturated_ticks < 100,
        "the wide oval should not be grip-limited, but the loop was out of lock \
         for {} ticks",
        without.saturated_ticks
    );
    assert!(
        (with.peak_error - without.peak_error).abs() < 1.0,
        "the differential should be near-invisible here: {:.2} against {:.2}",
        with.peak_error,
        without.peak_error
    );
}
