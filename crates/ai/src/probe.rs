//! A fixed scenario the determinism gate runs the drivers over.
//!
//! The third probe, after [`oag_core::probe`] and `oag_physics::probe`, which
//! cannot reach this crate (scripted input; no opponent). Until this existed
//! **every arithmetic decision an opponent makes was outside the cross-platform
//! gate**, which is how [`Line::curvature`](crate::Line::curvature) called the
//! platform's own `acos` for months.
//!
//! It exercises what the others cannot: [`Line::curvature`](crate::Line::curvature)
//! over a **spread** of angles ([`RunResult::curvature`], [`circuit`]); the
//! personality draws; the social axes (four craft, a [`Field`] each per tick);
//! and the mistake and provocation counters, integer state that agrees or does
//! not.
//!
//! No disc image: `data/` is gitignored, so a disc-backed scenario would never
//! run on Windows or macOS, where portability bugs show. **The hulls and circuit
//! are invented** per
//! [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md);
//! the real-track check is
//! `race_ground_truth::the_ai_drives_the_field_along_the_track`.

use crate::{Context, Driver, Field, Frame, Line, Pilot, Rival, Tuning};
use oag_core::hash::StateHasher;
use oag_core::math::{Quat, Vec3};
use oag_physics::{
    Body, Environment, Handling, Ray, RaycastHit, Raycaster, ShipState, Surface, params,
};

/// Our own fixed timestep. ADR-0007.
const TICK: f32 = 1.0 / 60.0;

/// How many craft take the scenario's grid. Four, not eight: the social axes
/// need somebody ahead, behind and alongside, and four halves the run time of a
/// gate three platforms pay for on every push.
pub const CRAFT: usize = 4;

/// How much room the circuit's corridor gives either side of the line.
const CORRIDOR: f32 = 14.0;

/// The span [`RunResult::curvature`] is measured over. Fixed, not the driver's
/// lookahead: a spread that moved because the craft was slower would say
/// nothing about whether the circuit still has corners.
const CURVATURE_SPAN: f32 = 30.0;

/// Which committed scenario a reference row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Four seeded pilots, each a different built-in, released together,
    /// abreast and a little apart so the field interacts from the first tick.
    Field,
    /// The same circuit driven by one craft with a `Driver::default()`: the
    /// plain follower, isolating aim, curvature and speed target from what the
    /// pilots add, so a divergence says which half moved.
    Solo,
}

impl std::fmt::Display for Scenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Field => write!(f, "field"),
            Self::Solo => write!(f, "solo"),
        }
    }
}

/// What one run of the drivers is worth, for hashing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunResult {
    /// Hash of the final state of every craft and every driver.
    pub final_hash: u64,
    /// Hash of every tick's state and controls, folded together: catches a
    /// divergence that happens mid-run and cancels out.
    pub trajectory_hash: u64,
    /// The smallest and largest curvature any craft was steering to, in radians
    /// per unit.
    ///
    /// **Reported so the test can assert the scenario still exercises a range
    /// of angles**: a circuit flattened to one radius, or a driver whose index
    /// stopped advancing, would hash consistently while testing nothing.
    pub curvature: (f32, f32),
    /// How far the *least* travelled craft went, in world units: the other half
    /// of "this scenario still tests something" (craft spun off at tick 30 hash
    /// reproducibly and say nothing). **Path length, not line index**, which
    /// jitters by a point or two per tick and would measure that instead.
    pub travelled: f32,
}

/// An infinite horizontal floor at `y = 0`. No walls, so the hash covers the
/// controller's arithmetic and not a contact response `oag_physics::probe`
/// already gates.
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

/// A craft that can drive, with invented numbers, **and airbrakes that work**.
/// `closed_loop.rs`'s fixture leaves every airbrake term at zero, which a
/// determinism gate must not (it would hash a path nobody drives). **None of
/// these is the game's**, ADR-0006.
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

/// The tuning the invented craft is driven with: `Tuning::default()`'s
/// `lateral_accel` is measured against the *real* hulls, which corner harder
/// (`closed_loop.rs` records why), so this takes that file's invented figure.
fn tuning() -> Tuning {
    Tuning {
        lateral_accel: 55.0,
        ..Tuning::default()
    }
}

/// A closed circuit with corners of several different radii.
///
/// The shape is the point. An oval has one radius, so every curvature is the
/// same number and a monotone angle error moves nothing. This loop opens into
/// long sweeps and closes into tight bends so [`Line::curvature`](crate::Line::curvature)
/// is sampled across a range; [`RunResult::curvature`] reports the spread and
/// `tests/determinism.rs` asserts on it.
///
/// # No `sin` or `cos`
///
/// Arcs off `angle.cos()` (as `closed_loop.rs`'s oval does) are the platform
/// transcendentals this gate exists to keep out: the gate would fail on macOS
/// for a reason unrelated to the drivers. So the loop is a closed uniform
/// Catmull-Rom spline through the control points: `+ - * /` only, identical
/// everywhere, and closed exactly (a seam would read as one enormous
/// curvature).
pub fn circuit() -> Line {
    /// The outline, anticlockwise in the XZ plane. Roughly evenly spaced, which
    /// keeps a *uniform* Catmull-Rom from overshooting.
    const CONTROL: [(f32, f32); 12] = [
        (320.0, -420.0),
        (320.0, 60.0),
        (250.0, 330.0),
        (40.0, 470.0),
        (-190.0, 450.0),
        (-330.0, 250.0),
        (-300.0, 30.0),
        (-160.0, -80.0),
        (-260.0, -250.0),
        (-220.0, -430.0),
        (-20.0, -480.0),
        (180.0, -470.0),
    ];
    /// Samples per control-point span: twelve spans over a loop about 2,600
    /// units round is a point every five units or so, like a real spline.
    const PER_SPAN: usize = 40;

    let control = |index: i32| -> Vec3 {
        let wrapped = index.rem_euclid(CONTROL.len() as i32) as usize;
        let (x, z) = CONTROL[wrapped];
        Vec3::new(x, 0.0, z)
    };

    let mut points: Vec<Vec3> = Vec::with_capacity(CONTROL.len() * PER_SPAN);
    for span in 0..CONTROL.len() as i32 {
        let (p0, p1, p2, p3) = (
            control(span - 1),
            control(span),
            control(span + 1),
            control(span + 2),
        );
        for step in 0..PER_SPAN {
            let t = step as f32 / PER_SPAN as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            // The uniform Catmull-Rom basis, halved as conventionally written.
            // Interpolating: the curve passes through every control point.
            points.push(
                (p1 * 2.0
                    + (p2 - p0) * t
                    + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2
                    + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3)
                    * 0.5,
            );
        }
    }

    // A corridor an even `CORRIDOR` either side (a real one comes off the
    // disc). **The frame is `along.cross(Y)`, the driver's right**, matching
    // `Body::right` and `sample.lateral`. Every synthetic corridor here had
    // this backwards once, and symmetric bounds hid it; see `closed_loop.rs`.
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

/// Places craft `slot` on the line's own first points, staggered like a grid:
/// two columns eleven units apart across and twenty-two along, a grid's shape
/// without being the recovered one (`oag_gameplay::spawn::grid_pose` belongs to
/// a race). **Derived from the line**, so an edit to [`circuit`] moves the grid
/// with it.
fn grid_pose(line: &Line, slot: usize) -> (Vec3, Quat) {
    let row = (slot / 2) as f32;
    let side = if slot.is_multiple_of(2) { -1.0 } else { 1.0 };
    let along_index = (row * 4.0) as usize;
    let here = line.point(along_index);
    let along = (line.point(along_index + 1) - here).normalize_or_zero();
    let across = along.cross(Vec3::Y).normalize_or_zero();
    (
        here + across * (side * 11.0) + Vec3::Y * 4.0,
        // `from_rotation_arc` is `sqrt` and arithmetic, not a transcendental:
        // see [`circuit`].
        Quat::from_rotation_arc(Vec3::Z, along),
    )
}

/// Across the line at `index`, pointing to the driver's right. Built from the
/// line, not [`Frame::lateral`], because [`circuit`] built the corridor from it:
/// one derivation is one thing to get the sign of wrong.
fn lateral(line: &Line, index: i64) -> Vec3 {
    let at = index.rem_euclid(line.len().max(1) as i64) as usize;
    let along = (line.point(at + 1) - line.point(at)).normalize_or_zero();
    along.cross(Vec3::Y).normalize_or_zero()
}

/// The rivals craft `slot` can see, out of the poses of all of them.
///
/// The three questions `oag_game`'s `Race::field_for` answers (nearest ahead,
/// behind, alongside), resolved off the drivers' line indices so the probe needs
/// no race. No wrap-aware lap ordering: four craft released together on one
/// straight never lap each other here.
fn field_for(slot: usize, drivers: &[Driver], states: &[ShipState], line: &Line) -> Field {
    let mine = drivers[slot].index as i64;
    let my_state = &states[slot];
    let mut field = Field {
        place: 1,
        ..Field::EMPTY
    };
    let mut best_ahead = i64::MAX;
    let mut best_behind = i64::MAX;
    let mut best_alongside = f32::MAX;

    for (other, driver) in drivers.iter().enumerate() {
        if other == slot {
            continue;
        }
        let theirs = driver.index as i64;
        let along = theirs - mine;
        let separation = states[other].body.position.distance(my_state.body.position);
        let closing = (my_state.body.linear_velocity - states[other].body.linear_velocity)
            .dot(my_state.body.forward())
            .abs();
        let toward = states[other].body.position - my_state.body.position;
        let offset = toward.dot(lateral(line, mine));
        let rival = Rival {
            slot: other as u8,
            gap: separation,
            offset,
            closing,
            range: separation,
            cos_bearing: toward.normalize_or_zero().dot(my_state.body.forward()),
        };
        if along > 0 {
            if along < best_ahead {
                best_ahead = along;
                field.ahead = Some(rival);
            }
            field.place += 1;
        } else if along < 0 && -along < best_behind {
            best_behind = -along;
            field.behind = Some(rival);
        }
        if along.abs() <= 4 && separation < best_alongside {
            best_alongside = separation;
            field.alongside = Some(rival);
        }
    }
    field
}

/// Runs `scenario` for `ticks` and returns its hashes and curvature spread,
/// stepping what a race steps: [`Driver::drive`] then [`oag_physics::step`],
/// once per craft per tick.
#[must_use]
pub fn run(scenario: Scenario, ticks: u32) -> RunResult {
    let line = circuit();
    let handling = handling();
    let tuning = tuning();
    let environment = Environment::default();

    let craft = match scenario {
        Scenario::Field => CRAFT,
        Scenario::Solo => 1,
    };
    // A different built-in pilot per craft in a fixed order, so the run covers
    // four sets of draws. `Solo` takes `Driver::default()`, the plain follower.
    let pilots: [&Pilot; CRAFT] = [
        &Pilot::BALANCED,
        &Pilot::AGGRESSIVE,
        &Pilot::PASSIVE,
        &Pilot::SHY,
    ];

    let mut states: Vec<ShipState> = (0..craft)
        .map(|slot| {
            let (position, orientation) = grid_pose(&line, slot);
            ShipState {
                body: Body {
                    position,
                    orientation,
                    ..Body::default()
                },
                ..ShipState::default()
            }
        })
        .collect();
    let mut drivers: Vec<Driver> = (0..craft)
        .map(|slot| match scenario {
            // Seeds a caller would derive from a race seed and a slot, fixed:
            // a gate over an arbitrary seed is a gate over whichever ran.
            Scenario::Field => Driver {
                seed: 0x51ed_0000 + slot as u32,
                ..Driver::default()
            },
            Scenario::Solo => Driver::default(),
        })
        .collect();

    let mut trajectory = StateHasher::new();
    let mut lowest = f32::MAX;
    let mut highest = 0.0f32;
    let mut travelled = vec![0.0f32; craft];
    let mut was_at: Vec<Vec3> = states.iter().map(|state| state.body.position).collect();

    for _ in 0..ticks {
        let fields: Vec<Field> = (0..craft)
            .map(|slot| match scenario {
                Scenario::Field => field_for(slot, &drivers, &states, &line),
                Scenario::Solo => Field::EMPTY,
            })
            .collect();

        for slot in 0..craft {
            let pilot = match scenario {
                Scenario::Field => pilots[slot % CRAFT],
                Scenario::Solo => &Pilot::BALANCED,
            };
            let controls = drivers[slot].drive(
                &states[slot],
                &Context {
                    line: &line,
                    tuning: &tuning,
                    pilot,
                    field: &fields[slot],
                    yaw_ceiling: None,
                    plan: None,
                },
            );
            oag_physics::step(
                &mut states[slot],
                &controls,
                &handling,
                &environment,
                &Plane,
                TICK,
            );

            // The angle the acos saw this tick, at the index this craft steers
            // from; `CURVATURE_SPAN`, not the driver's speed-varying lookahead.
            let curvature = line.curvature(drivers[slot].index as usize, CURVATURE_SPAN);
            lowest = lowest.min(curvature);
            highest = highest.max(curvature);

            travelled[slot] += states[slot].body.position.distance(was_at[slot]);
            was_at[slot] = states[slot].body.position;

            write_craft(&mut trajectory, &states[slot], &drivers[slot]);
            trajectory.write_f32(controls.steer_x);
            trajectory.write_f32(controls.steer_y);
            trajectory.write_f32(controls.thrust);
            trajectory.write_f32(controls.airbrake_left);
            trajectory.write_f32(controls.airbrake_right);
        }
    }

    let mut final_hash = StateHasher::new();
    for slot in 0..craft {
        write_craft(&mut final_hash, &states[slot], &drivers[slot]);
    }

    RunResult {
        final_hash: final_hash.finish(),
        trajectory_hash: trajectory.finish(),
        curvature: (if lowest == f32::MAX { 0.0 } else { lowest }, highest),
        travelled: travelled.into_iter().fold(f32::MAX, f32::min),
    }
}

/// The speed plan [`crate::SpeedPlan::build`] learns on [`circuit`], hashed:
/// every sample's ceiling, target and verified pace as bits, then the build's
/// report. The plan is built at race start on a player's machine, so a platform
/// that learned a different plan would field a different race.
#[must_use]
pub fn speed_plan() -> (u64, crate::plan::Report) {
    let line = circuit();
    let handling = handling();
    let tuning = tuning();
    let placed = |index: usize| {
        let here = line.point(index);
        let along = (line.point(index + 1) - here).normalize_or_zero();
        ShipState {
            body: Body {
                position: here + Vec3::Y * 4.0,
                orientation: Quat::from_rotation_arc(Vec3::Z, along),
                ..Body::default()
            },
            ..ShipState::default()
        }
    };
    let course = crate::plan::Course {
        line: &line,
        samples: &[],
        raycaster: &Plane,
        env: Environment::default(),
        dt: TICK,
        off_line: CORRIDOR * 2.0,
        respawn: &placed,
        reset: &|_, _, _| false,
        max_airborne: f32::INFINITY,
        pad: &|_, _| None,
    };
    let craft = crate::plan::Craft {
        handling,
        start: placed(0),
        start_index: 0,
    };
    let (plan, report) = crate::SpeedPlan::build(&course, &craft, &tuning);
    let mut hasher = StateHasher::new();
    for index in 0..plan.len() {
        hasher.write_f32(plan.ceiling(index));
        hasher.write_f32(plan.target(index));
        hasher.write_f32(plan.pace(index));
    }
    hasher.write_u64(report.steps);
    hasher.write_u32(report.passes);
    hasher.write_u32(report.lowerings);
    hasher.write_u32(report.unresolved.len() as u32);
    hasher.write_u32(report.respawns);
    hasher.write_u32(report.verify_failures);
    hasher.write_u32(report.verify_contacts);
    hasher.write_u32(report.verify_respawns);
    hasher.write_u32(report.verify_lap_ticks.unwrap_or(u32::MAX));
    (hasher.finish(), report)
}

/// One craft's hashable state: what it is doing, and what its driver remembers.
///
/// **Every `Driver` field, deliberately**: a field left out is a field a
/// divergence can hide in, as `oag_gameplay::hash` recorded when pickups landed.
fn write_craft(hasher: &mut StateHasher, state: &ShipState, driver: &Driver) {
    hasher.write_vec3(state.body.position);
    hasher.write_quat(state.body.orientation);
    hasher.write_vec3(state.body.linear_velocity);
    hasher.write_vec3(state.body.angular_velocity);
    hasher.write_u32(driver.index);
    hasher.write_u32(driver.seed);
    hasher.write_u32(driver.phase);
    hasher.write_u8(driver.place);
    hasher.write_u32(u32::from(driver.provocation));
    hasher.write_u32(driver.pilot);
    hasher.write_u32(u32::from(driver.mistake));
    hasher.write_u32(driver.peak_curvature);
}
