//! Drives the differential airbrake against a line that is never *exactly*
//! straight, as a real disc track's spline samples never are.
//!
//! Split out of `closed_loop.rs`, which sits at its `BASELINE` ceiling in
//! `scripts/check-file-size.py`. The helpers are deliberate near-duplicates, as
//! in `ship_spawn_ground_truth.rs`, `mine_ground_truth.rs` and
//! `plasma_ground_truth.rs`: nothing here is `pub`, so a second binary builds
//! its own.
//!
//! # Why this fixture exists
//!
//! `closed_loop.rs`'s `oval_of` builds straights from `push_straight`, so
//! [`Line::curvature`] reads exactly `0.0` and `corner_target` returns
//! [`f32::INFINITY`]. A real line never does: over two laps of Talon's Junction
//! (Pulse PSP) zero of 5,336 ticks had an infinite target, against 303 on
//! `oval_of`. A fix reading `!target.is_finite()` as "straight" passes every
//! `oval_of` test and regresses on disc geometry, as happened once already (see
//! `oag_ai::driver::pace::trail` and `docs/gameplay/ai.md`, "Airbrakes, and what
//! a differential one actually does"). [`noisy_oval_of`] is the one fixture here
//! where that is no longer true.

use oag_ai::{Context, Driver, Field, Frame, Line, Pilot, Tuning};
use oag_core::math::{Quat, Vec3};
use oag_physics::{
    Body, Environment, Handling, Ray, RaycastHit, Raycaster, ShipState, Surface, params,
};

/// An infinite horizontal floor at `y = 0`. See `closed_loop.rs`'s copy.
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

/// A craft that can drive, with invented numbers. See `closed_loop.rs`'s
/// `handling` for what each field is for and why none is the game's.
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

/// The tuning this file's invented craft is driven with. See `closed_loop.rs`'s
/// `tuning` for why `Tuning::default` does not describe this hull.
fn tuning() -> Tuning {
    Tuning {
        lateral_accel: 55.0,
        ..Tuning::default()
    }
}

/// The same craft, with airbrakes that do something. See `closed_loop.rs`'s
/// `handling_with_airbrakes` for why it is a second fixture.
fn handling_with_airbrakes() -> Handling {
    Handling {
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
        ..handling()
    }
}

/// How much room the oval's corridor gives either side of the line.
const CORRIDOR: f32 = 12.0;

/// [`closed_loop.rs`'s `oval_of`](../closed_loop.rs) with a lateral wobble on the
/// straights so three consecutive points are never *exactly* collinear.
///
/// `wobble` is the amplitude in units; the period is fixed at 40 units so the
/// curvature is small and roughly constant regardless of it
/// ([`a_realistic_straight_never_fires_the_differential`] has the measured
/// range). Returns the line and the *first* straight's point count, so curvature
/// can be measured over just that stretch.
fn noisy_oval_of(radius: f32, straight: f32, wobble: f32) -> (Line, usize) {
    let spacing = 2.5f32;
    let period = 40.0f32;
    let mut points = Vec::new();

    let push_wobbly_straight = |points: &mut Vec<Vec3>, x: f32, from: f32, to: f32| {
        let steps = ((to - from).abs() / spacing) as usize;
        for step in 0..steps {
            let t = step as f32 / steps as f32;
            let z = from + (to - from) * t;
            let offset = wobble * (std::f32::consts::TAU * z / period).sin();
            points.push(Vec3::new(x + offset, 0.0, z));
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
    push_wobbly_straight(&mut points, radius, -half, half);
    let first_straight_len = points.len();
    push_arc(
        &mut points,
        Vec3::new(0.0, 0.0, half),
        0.0,
        std::f32::consts::PI,
    );
    push_wobbly_straight(&mut points, -radius, half, -half);
    push_arc(
        &mut points,
        Vec3::new(0.0, 0.0, -half),
        std::f32::consts::PI,
        std::f32::consts::TAU,
    );

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
    (Line::with_corridor(points, corridor), first_straight_len)
}

/// Signed distance from the craft to its line, across the line. See
/// `closed_loop.rs`'s copy.
fn cross_track(line: &Line, index: usize, position: Vec3) -> f32 {
    let here = line.point(index);
    let next = line.point(index + 1);
    let along = (next - here).normalize_or_zero();
    let across = along.cross(Vec3::Y);
    (position - here).dot(across)
}

/// Runs the craft round the line and reports what happened. See
/// `closed_loop.rs`'s `Run` for what each field means.
struct Run {
    peak_error: f32,
    steer_reversals: usize,
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

    let start = line.point(0);
    let heading = (line.point(1) - start).normalize_or_zero();
    let mut state = ShipState {
        body: Body {
            position: start + Vec3::Y * handling.antigrav.ride_height * 0.5,
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
    let mut steer_reversals = 0usize;
    let mut committed = 0.0f32;

    for tick in 0..ticks {
        let controls = driver.drive(
            &state,
            &Context {
                line,
                tuning: &tuning,
                pilot,
                field: &Field::EMPTY,
                yaw_ceiling: None,
                plan: None,
            },
        );
        oag_physics::step(&mut state, &controls, &handling, &env, &Plane, dt);

        let index = driver.index as usize;
        let error = cross_track(line, index, state.body.position);
        if tick > 60 {
            peak_error = peak_error.max(error.abs());
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
        steer_reversals,
    }
}

/// The regression [`noisy_oval_of`] exists for: the differential's
/// `speed < target` gate was replaced by a curvature floor and an exit-decay
/// check. `!target.is_finite()`, tried first, passed every exactly-straight
/// synthetic test and still ground a real opponent's shield to zero, since no
/// disc track's `target` is ever literally infinite (module doc).
#[test]
fn a_realistic_straight_never_fires_the_differential() {
    let tuning = tuning();
    // 0.02 units of wobble over a 40-unit period reads as ~0.00038 curvature on
    // the straight (measured below), inside the 0.0003-0.0005 band Talon's
    // Junction's straights carry and well under `trail_curvature_floor`, which
    // the assertion checks rather than assumes.
    let (line, straight_len) = noisy_oval_of(120.0, 400.0, 0.02);
    // `Line::curvature` walks `3 * span` ahead of `index`, so an index within
    // that of the straight's end reads into the following arc: excluded, not
    // mistaken for the wobble.
    let span = 11.0f32;
    let margin = (span * 3.0 / 2.5).ceil() as usize;
    let max_curvature = (0..straight_len.saturating_sub(margin))
        .map(|index| line.curvature(index, span))
        .fold(0.0f32, f32::max);
    assert!(
        max_curvature < tuning.trail_curvature_floor,
        "the wobble is not a fair stand-in for real spline noise: {max_curvature} >= the floor"
    );
    assert!(
        max_curvature > 0.0,
        "the wobble must produce *some* curvature or this fixture is oval_of again"
    );

    let braked = handling_with_airbrakes();
    let with = drive_the_oval_as(
        1800,
        tuning,
        Driver::default(),
        &braked,
        &line,
        &Pilot::BALANCED,
    );
    let symmetric = Tuning {
        trail_gain: 0.0,
        trail_max: 0.0,
        ..tuning
    };
    let without = drive_the_oval_as(
        1800,
        symmetric,
        Driver::default(),
        &braked,
        &line,
        &Pilot::BALANCED,
    );

    assert!(
        with.peak_error < 20.0,
        "the craft got {:.1} units from its line on a fixture with no real corner tight \
         enough to need the differential - it should behave like the plain oval",
        with.peak_error
    );
    assert!(
        with.steer_reversals <= without.steer_reversals + 4,
        "the differential added reversals on a straight with realistic noise: {} against {}",
        with.steer_reversals,
        without.steer_reversals
    );
}
