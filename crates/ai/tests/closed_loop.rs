//! Drives a real craft round a real oval, with the real force law.
//!
//! # Why this exists
//!
//! The controller's unit tests cannot see the failure this file was written for,
//! a *loop* property: the first version of this AI tracked the line on any single
//! tick and drove wall to wall in a race.
//!
//! **The plant is a double integrator with a lag in front of it.**
//! `oag_physics::engine::steering` feeds `Accumulators::local_angular`, a
//! **torque**, so steering commands yaw *acceleration*; `controls::ramp_steering`
//! lags the input itself. A controller proportional to line error overshoots and
//! keeps doing it: a limit cycle, not a tuning error, and no gain fixes it.
//!
//! # The numbers that justified the rewrite
//!
//! **Written first and watched failing.** Against the proportional-on-error
//! controller it replaced, on the oval below over 1,800 ticks:
//!
//! | | peak error from the line |
//! | --- | --- |
//! | proportional on error | **84.1** |
//! | yaw-rate tracking | **7.3** |
//!
//! **Amplitude is what is asserted; counting line crossings was tried and
//! discarded**: both controllers cross about as often (19 against 16), since a
//! tight tracker still crosses at every corner entry and exit.
//!
//! # What this is not
//!
//! **It is not the game**: a flat infinite plane, invented handling (see
//! [`handling`]). Whether the field gets round `16_Track` is
//! `race_ground_truth::the_ai_drives_the_field_along_the_track`, which needs a
//! disc. This says the controller is stable against a plant of the right
//! *shape*.

use oag_ai::{Driver, Field, Frame, Line, Pilot, Rival, Span, Tuning};
use oag_core::math::{Quat, Vec3};
use oag_physics::{
    Body, Environment, Handling, Ray, RaycastHit, Raycaster, ShipState, Surface, params,
};

/// An infinite horizontal floor at `y = 0`: enough for the hover probes and
/// grip. No walls, so cross-track error is measured rather than clipped.
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

/// A craft that can drive, with invented numbers.
///
/// **None of these is the game's** (ADR-0006): round figures a reader can check
/// the arithmetic against. The one that matters is `turning.amount`, the gain of
/// the double integrator the controller must be stable against.
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

/// The tuning this file's invented craft is driven with.
///
/// **`Tuning::default()`'s `lateral_accel` is measured against the real hulls
/// and does not describe this one**: the disc's craft carry `grip_ground` 10
/// and `accelcap` 17 in loader units, [`handling`] invents 40 and 60 (187
/// against 300 top speed), so at the shipped 180 this fixture slides 81 units
/// off its line. The invented hull gets an invented matching tuning, so every
/// number here measures **controller stability, not the speed target** (real
/// data: `race_ground_truth::the_ai_drives_the_field_along_the_track`).
/// `the_default_tuning_is_not_this_ones` pins the split.
fn tuning() -> Tuning {
    Tuning {
        lateral_accel: 55.0,
        ..Tuning::default()
    }
}

/// The same craft, with airbrakes that do something.
///
/// **A second fixture, and the split is load-bearing.** [`handling`] leaves
/// `airbrake` and `brakes` at `Handling::ZERO`, so airbrake commands go nowhere
/// and braking is `thrust = 0`. The bounds in
/// [`a_craft_settles_onto_the_line_instead_of_weaving`] were calibrated against
/// that craft, so giving it airbrakes would silently re-baseline the regression.
/// [`the_default_fixture_has_no_airbrakes_and_the_regression_bounds_know_it`]
/// pins it. **None of these is the game's** (ADR-0006): round figures giving
/// airbrakes that slow, yaw and cost grip on the order of the other forces.
fn handling_with_airbrakes() -> Handling {
    Handling {
        airbrake: params::Airbrake {
            // A lateral force gain, not a drag, scaled by 1e-4: see
            // `docs/physics/README.md` and `params::Airbrake`.
            amount: 0.0005,
            drag: 1.0,
            gain: 400.0,
            falloff: 400.0,
            // Against `engine::steering`'s yaw torque of `steer * turning.amount`
            // (up to 100) this gives `speed * turn * imbalance * 0.001`, about 18
            // at 150 units a second and a full differential: an assist worth a
            // fifth of the stick.
            turn: 2.0,
            // Scaled by 1e-4, so `0.0..=0.01`: 0.004 keeps forty per cent of
            // lateral grip at full airbrake.
            slidegrip: 0.004,
            sideshift: 20.0,
        },
        brakes: params::Brakes {
            // **Scaled by -0.01 and so negative**: the force is along
            // `+unit(velocity)` and the sign is carried here.
            amount: -0.4,
            gain: 400.0,
            falloff: 400.0,
        },
        ..handling()
    }
}

/// An oval: two 120-unit-radius half circles joined by 400-unit straights.
/// Deliberately not a circle: constant curvature is the easy case, and what
/// excites an oscillator is the *transitions* into and out of corners.
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

    // A corridor `CORRIDOR` either side, even because the driver is under test,
    // not a corridor's shape (a real one comes off the disc per control point).
    //
    // **The frame is built from the line and world up**, right only because this
    // plane is flat; on a real track it is the sample's `lateral`
    // (`docs/formats/track.md`).
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
    // The driver's right, matching `Body::right` and the disc - see `oval`.
    let across = along.cross(Vec3::Y);
    (position - here).dot(across)
}

/// Runs the craft round the oval and reports what happened.
struct Run {
    /// The furthest it ever got from the line, either side.
    peak_error: f32,
    /// Mean distance from the line over the run.
    mean_error: f32,
    /// Mean *signed* distance, positive to the left of the line: which part of
    /// the corridor a driver held (the unsigned mean cannot tell opposite sides
    /// apart).
    mean_offset: f32,
    /// How far along the line it travelled, in points.
    progress: usize,
    /// How often the steering command crossed zero, counting only crossings from
    /// a committed side. A ringing loop reverses far more than a tracking one:
    /// the guard on adding a second path into the steering.
    steer_reversals: usize,
    /// Ticks the steering command spent pinned at full lock.
    ///
    /// **What the differential is for, measured at the outcome**: saturation is
    /// the loop asking for more yaw than the stick delivers, and an assist gets
    /// it off the stop sooner. Counting ticks the assist *fired* would count its
    /// own gate.
    saturated_ticks: usize,
}

fn drive_the_oval(ticks: usize) -> Run {
    drive_the_oval_with(ticks, tuning())
}

fn drive_the_oval_with(ticks: usize, tuning: Tuning) -> Run {
    drive_the_oval_as(
        ticks,
        tuning,
        Driver::default(),
        &handling(),
        &oval(),
        &Pilot::BALANCED,
    )
}

fn drive_the_oval_as(
    ticks: usize,
    tuning: Tuning,
    mut driver: Driver,
    handling: &Handling,
    line: &Line,
    pilot: &Pilot,
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
        let controls = driver.drive(
            &state,
            // `Context::new` already carries the empty field and the `None`
            // hull ceiling this loop wants - see `Context::yaw_ceiling`.
            &oag_ai::Context {
                pilot,
                ..oag_ai::Context::new(line, &tuning)
            },
        );
        oag_physics::step(&mut state, &controls, &handling, &env, &Plane, dt);

        let index = driver.index as usize;
        if index < last_index && last_index - index > line.len() / 2 {
            laps += 1;
        }
        last_index = index;

        let error = cross_track(line, index, state.body.position);
        // Skip the first second: the craft starts at rest and settling is not the
        // property under test.
        if tick > 60 {
            peak_error = peak_error.max(error.abs());
            if controls.steer_x.abs() >= 1.0 {
                saturated_ticks += 1;
            }
            total_error += error.abs();
            total_offset += error;
            samples += 1;

            // Only a command that picked a side has one to reverse from, or every
            // wander through zero scores.
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

/// The regression this file exists for. The bounds are loose against the
/// rewritten controller (7.3 peak, ~2 mean) and nowhere near the broken one
/// (84.1), so this catches the wave's return without grading the driving;
/// tightening them would make every tuning change a failure.
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
                tuning(),
                Driver::for_slot(0xC0FFEE, slot),
                &handling(),
                &oval(),
                &Pilot::BALANCED,
            )
        })
        .collect()
}

/// **The reason personalities exist**: seven drivers on one line hold seven
/// parts of the corridor, not the same centimetre. Asserted on the *signed* mean
/// offset and the spread between drivers: what is wrong with a field tracking
/// one line is that they agree.
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

    // The spread is the field's, not two outliers with five craft nose to tail:
    // mean absolute deviation, not nearest-neighbour, since **two drivers may
    // pick similar lines** and forbidding it would pin the distribution's luck.
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

/// The spread is the corridor's to give. Using more than
/// [`Tuning::corridor_use`] allows would rely on a clamp that knows only the
/// point being aimed at, and the room either side is all that stands between a
/// real field and the scenery.
#[test]
fn a_seeded_driver_stays_inside_the_corridor() {
    // The aiming-to-arriving lag the corridor does not bound: the controller's
    // own tracking error, 7.3 units peak on this oval. Loose like that bound.
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
    // They all still got round: a spread from one craft stopping is not a race.
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
        tuning(),
        Driver::for_slot(7, 3),
        &handling(),
        &oval(),
        &Pilot::BALANCED,
    );
    let two = drive_the_oval_as(
        600,
        tuning(),
        Driver::for_slot(7, 3),
        &handling(),
        &oval(),
        &Pilot::BALANCED,
    );
    assert_eq!(one.progress, two.progress);
    assert_eq!(one.mean_offset, two.mean_offset);
    assert_eq!(one.peak_error, two.peak_error);
}

/// The finding that made [`handling_with_airbrakes`] necessary, pinned so the
/// next reader does not merge the two fixtures.
///
/// Every number in this file's module docs (84.1 against 7.3, the bounds in
/// [`a_craft_settles_onto_the_line_instead_of_weaving`]) was measured on a craft
/// whose airbrakes do **nothing**: `Handling::ZERO` leaves `airbrake.gain` and
/// `brakes.amount` at zero, so braking is `thrust = 0`. Deliberate, and why the
/// differential is invisible to the six tests above.
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

/// The guard on the new fixture: were it inert too, everything below would pass
/// by measuring nothing.
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

/// The stability guard: a second path into a loop that already oscillates is
/// what this crate was rewritten to remove, so the differential must not widen
/// the wave or reverse more often.
#[test]
fn a_differential_braking_driver_does_not_ring() {
    let symmetric = Tuning {
        trail_gain: 0.0,
        trail_max: 0.0,
        ..tuning()
    };
    let braked = handling_with_airbrakes();
    let with = drive_the_oval_as(
        1800,
        tuning(),
        Driver::default(),
        &braked,
        &oval(),
        &Pilot::BALANCED,
    );
    let without = drive_the_oval_as(
        1800,
        symmetric,
        Driver::default(),
        &braked,
        &oval(),
        &Pilot::BALANCED,
    );

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
/// **It only shows up where the design says.** On [`oval`]'s 120-unit corners
/// the craft spends about twenty of 1,800 ticks at full lock, so there is no
/// understeer to help; at 60 units it spends over seven hundred pinned, the
/// loop saying it cannot make the yaw it wants.
///
/// **The gain is not monotonic in how tight the corner is**; do not "improve"
/// this fixture by tightening it. Peak error over 2,400 ticks:
///
/// | corners | with | without |
/// | --- | --- | --- |
/// | 120-unit, 400-unit straights | 7.5 | 7.4 |
/// | **60-unit, 300-unit straights** | **19.2** | **21.4** |
/// | 45-unit, 500-unit straights | 55.2 | 56.3 |
/// | 35-unit, 600-unit straights | 98.5 | 98.8 |
/// | 25-unit, 700-unit straights | 163.3 | 164.6 |
///
/// A 98-unit error on a 35-unit corner is a craft *missing the turn and
/// rejoining*, with no grip left for extra yaw to buy. The benefit peaks where
/// the craft is **marginally** past the steering's authority: the 60-unit case.
#[test]
fn a_differential_holds_a_corner_the_steering_alone_cannot() {
    let symmetric = Tuning {
        trail_gain: 0.0,
        trail_max: 0.0,
        ..tuning()
    };
    let braked = handling_with_airbrakes();
    // Corners the craft genuinely cannot make on the stick alone.
    let tight = oval_of(60.0, 300.0);
    let with = drive_the_oval_as(
        1800,
        tuning(),
        Driver::default(),
        &braked,
        &tight,
        &Pilot::BALANCED,
    );
    let without = drive_the_oval_as(
        1800,
        symmetric,
        Driver::default(),
        &braked,
        &tight,
        &Pilot::BALANCED,
    );

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

/// The other half: where the craft can already make the corner, the
/// differential stays out of the way. Not exactly nothing (about a tenth of a
/// unit of peak error on the wide oval, against 7.4), recorded because a bound
/// pretending zero fails on the next tuning change for no reason worth chasing.
#[test]
fn a_differential_barely_touches_a_corner_the_craft_can_already_make() {
    let symmetric = Tuning {
        trail_gain: 0.0,
        trail_max: 0.0,
        ..tuning()
    };
    let braked = handling_with_airbrakes();
    let with = drive_the_oval_as(
        1800,
        tuning(),
        Driver::default(),
        &braked,
        &oval(),
        &Pilot::BALANCED,
    );
    let without = drive_the_oval_as(
        1800,
        symmetric,
        Driver::default(),
        &braked,
        &oval(),
        &Pilot::BALANCED,
    );

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

/// The safety net on the four built-in characters: a pilot that cannot get
/// round puts a craft in the scenery in a real race, and the disc-backed test
/// that would catch it does not run in CI.
#[test]
fn every_built_in_pilot_gets_round_the_oval() {
    let line = oval();
    // **Both fixtures**: `handling()` has no airbrakes, so a pilot whose trait is
    // `trail` would be validated on a craft that cannot use it (see
    // `the_default_fixture_has_no_airbrakes_and_the_regression_bounds_know_it`).
    for craft in [handling(), handling_with_airbrakes()] {
        for (name, pilot) in Pilot::BUILT_IN {
            for slot in 1..3u32 {
                let run = drive_the_oval_as(
                    1800,
                    tuning(),
                    Driver::for_slot(0xC0FFEE, slot),
                    &craft,
                    &line,
                    &pilot,
                );
                assert!(
                    run.progress > line.len(),
                    "{name} in slot {slot} did not finish a lap: {} points",
                    run.progress
                );
                assert!(
                    run.peak_error < 20.0,
                    "{name} in slot {slot} got {:.1} units from its line",
                    run.peak_error
                );
            }
        }
    }
}

/// The characters must be distinguishable by driving, not just numbers:
/// aggression is late braking plus commitment, so it shows as distance covered.
#[test]
fn an_aggressive_pilot_gets_further_round_than_a_shy_one() {
    let line = oval();
    let mut aggressive_wins = 0;
    for slot in 1..8u32 {
        let driver = Driver::for_slot(0xC0FFEE, slot);
        // On the airbrake fixture, so `trail` (1.0-1.4 against 0.1-0.5) is
        // compared, not inert.
        let run = |pilot: &Pilot| {
            drive_the_oval_as(
                1800,
                tuning(),
                driver,
                &handling_with_airbrakes(),
                &line,
                pilot,
            )
            .progress
        };
        if run(&Pilot::AGGRESSIVE) > run(&Pilot::SHY) {
            aggressive_wins += 1;
        }
    }
    assert_eq!(
        aggressive_wins, 7,
        "the aggressive pilot should out-run the shy one from every seed"
    );
}

/// What two craft driving against each other did. Its own struct: a [`Run`] is a
/// lap-counted solo drive, and reusing it would make `progress` drop its lap
/// term and `peak_error` mean "on the last tick".
struct Pair {
    /// The closest the two ever came after settling, centre to centre.
    closest: f32,
    /// The furthest each got from the line, either side, over the whole run.
    peak_error: [f32; 2],
    /// The mean signed offset each held, positive to that craft's right.
    mean_offset: [f32; 2],
}

/// Two craft on one line, each seeing the other, with the real force law.
///
/// **The only test here that closes the social loop.** Other awareness tests
/// hand a driver a `Field` held still; this measures each craft from the
/// other's actual state every tick, so a yielding rule that oscillates or walks
/// two craft into each other shows as a number. Craft 0 leads, craft 1 chases
/// from 25 units back and six across. Still not the game: flat plane, no walls,
/// gap along the line rather than `oag_race::Standing`.
fn drive_two_round_the_oval(ticks: usize, pilots: [&Pilot; 2]) -> Pair {
    let line = oval();
    let handling = handling_with_airbrakes();
    let env = Environment::default();
    let dt = 1.0 / 60.0;
    let tuning = tuning();

    let heading = (line.point(1) - line.point(0)).normalize_or_zero();
    let across = heading.cross(Vec3::Y).normalize_or_zero();
    let start = |sideways: f32, back: f32| {
        let mut state = ShipState {
            body: Body {
                position: line.point(0)
                    + Vec3::Y * handling.antigrav.ride_height * 0.5
                    + across * sideways
                    - heading * back,
                orientation: Quat::from_rotation_arc(Vec3::NEG_Z, heading),
                mass: handling.physical.mass,
                ..Body::default()
            },
            ..ShipState::default()
        };
        oag_physics::damage::reset(&mut state, &handling.dimensions);
        state
    };

    let mut states = [start(3.0, 0.0), start(-3.0, 25.0)];
    let mut drivers = [Driver::for_slot(0xC0FFEE, 1), Driver::for_slot(0xC0FFEE, 2)];
    let mut closest = f32::INFINITY;
    let mut peak_error = [0.0f32; 2];
    let mut total_offset = [0.0f32; 2];
    let mut samples = 0usize;

    for tick in 0..ticks {
        // Each craft's view of the other, measured this tick.
        let fields: Vec<Field> = (0..2)
            .map(|me| {
                let (mine, theirs) = (&states[me], &states[1 - me]);
                let forward = mine.body.forward();
                let to_them = theirs.body.position - mine.body.position;
                let gap = to_them.dot(forward);
                let range = to_them.length();
                let approach = theirs.body.linear_velocity.dot(forward)
                    - mine.body.linear_velocity.dot(forward);
                let rival = Rival {
                    slot: (1 - me) as u8,
                    gap,
                    offset: to_them.dot(mine.body.right()),
                    closing: if gap >= 0.0 { -approach } else { approach },
                    range,
                    cos_bearing: if range > f32::EPSILON {
                        to_them.dot(forward) / range
                    } else {
                        1.0
                    },
                };
                let mut field = Field {
                    place: me as u8 + 1,
                    ..Field::EMPTY
                };
                if gap >= 0.0 {
                    field.ahead = Some(rival);
                } else {
                    field.behind = Some(rival);
                }
                field
            })
            .collect();

        for craft in 0..2 {
            let controls = drivers[craft].drive(
                &states[craft],
                &oag_ai::Context {
                    pilot: pilots[craft],
                    field: &fields[craft],
                    ..oag_ai::Context::new(&line, &tuning)
                },
            );
            oag_physics::step(&mut states[craft], &controls, &handling, &env, &Plane, dt);
            assert!(
                states[craft].body.position.is_finite(),
                "craft {craft} left the world on tick {tick}"
            );
        }

        if tick > 60 {
            closest = closest.min(states[0].body.position.distance(states[1].body.position));
            for craft in 0..2 {
                let error = cross_track(
                    &line,
                    drivers[craft].index as usize,
                    states[craft].body.position,
                );
                peak_error[craft] = peak_error[craft].max(error.abs());
                total_offset[craft] += error;
            }
            samples += 1;
        }
    }

    let divisor = samples.max(1) as f32;
    Pair {
        closest,
        peak_error,
        mean_offset: [total_offset[0] / divisor, total_offset[1] / divisor],
    }
}

/// Two craft that can see each other must not converge on one line, the failure
/// the personality system exists to prevent now that rules move craft *toward*
/// each other.
#[test]
fn two_craft_that_can_see_each_other_do_not_converge() {
    for (name, pilot) in Pilot::BUILT_IN {
        let pair = drive_two_round_the_oval(1800, [&pilot, &Pilot::AGGRESSIVE]);
        // A hull is 8 long and 4 wide: clear of contact without asserting they
        // never race closely.
        assert!(
            pair.closest > 5.0,
            "{name}: two craft closed to {:.1} units, which is contact",
            pair.closest
        );
        for craft in 0..2 {
            assert!(
                pair.peak_error[craft] < 25.0,
                "{name}: craft {craft} got {:.1} units off its line",
                pair.peak_error[craft]
            );
        }
    }
}

/// **Proves the social axes are load-bearing, not decorative**: zeroing
/// `courtesy`, `defence` and `caution` on all four built-in pilots left every
/// other test in the crate passing, since unit tests build a `Personality` by
/// hand and cannot see a pilot that declares nothing.
///
/// The chaser sits to the leader's **left**, so a yielding leader drifts right
/// and a covering one left; `mean_offset` is positive to the craft's right, so
/// the yielder must come out above the coverer.
///
/// **Two variants of one pilot, not two pilots**: `SHY` against `AGGRESSIVE`
/// also differ in `line_bias` and `width`, which dominate. These declare
/// identical spans on every axis but the social pair, so only the lean can move
/// the result.
#[test]
fn a_yielding_leader_gives_way_where_a_covering_one_does_not() {
    let yielding = Pilot {
        courtesy: Span::fixed(0.9),
        defence: Span::fixed(0.0),
        ..Pilot::BALANCED
    };
    let covering = Pilot {
        courtesy: Span::fixed(0.0),
        defence: Span::fixed(0.9),
        ..Pilot::BALANCED
    };
    let chaser = Pilot::AGGRESSIVE;

    let gave_way = drive_two_round_the_oval(1800, [&yielding, &chaser]);
    let covered = drive_two_round_the_oval(1800, [&covering, &chaser]);

    // **A tenth of a unit of mean offset, held over 1,800 ticks**: small because
    // the social lean is a minority of the aim budget (see `SOCIAL_MAX`, which
    // exists because an unbounded term swerved craft into the player and a
    // wall). A consistent bias over a run, not a lurch.
    assert!(
        gave_way.mean_offset[0] > covered.mean_offset[0] + 0.1,
        "a yielding leader should move away from a chaser on its left, and a \
         covering one toward it: {:.2} against {:.2}",
        gave_way.mean_offset[0],
        covered.mean_offset[0]
    );
}

/// The companion to
/// [`the_default_fixture_has_no_airbrakes_and_the_regression_bounds_know_it`]:
/// stops the next reader collapsing two different things into one.
/// `Tuning::default()`'s `lateral_accel` was swept against the **real** hulls
/// and is roughly three times what this invented craft holds; driving it with
/// that puts the craft 81 units off its line, which would read as weaving when
/// it is the speed target asking for grip the hull lacks.
#[test]
fn the_default_tuning_is_not_this_ones() {
    assert_ne!(
        tuning().lateral_accel,
        Tuning::default().lateral_accel,
        "if these ever agree, one of them stopped describing its own craft"
    );
    // Shared, so a change to the real tuning still reaches this file.
    assert_eq!(tuning().rate_gain, Tuning::default().rate_gain);
    assert_eq!(tuning().trail_gain, Tuning::default().trail_gain);
    assert_eq!(tuning().brake_floor, Tuning::default().brake_floor);
}
