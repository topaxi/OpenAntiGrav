//! A fixed scenario the determinism gate runs the drivers over.
//!
//! The third of its kind, after [`oag_core::probe`] (the math foundation) and
//! `oag_physics::probe` (the force law). It exists because neither of those
//! reaches this crate: the physics probe drives a *scripted* input, so no
//! controller decision is in its hash, and `oag_gameplay`'s own determinism
//! scenario fields no opponent at all - `Driver::place` and
//! `Driver::provocation` are zero throughout it by design.
//!
//! So until this file existed, **every arithmetic decision an opponent makes
//! was outside the cross-platform gate**, which is how
//! [`Line::curvature`](crate::Line::curvature) came to call the platform's own
//! `acos` for months without anything failing. It calls
//! `oag_core::math::acos` now, and this is the gate that would have said so.
//!
//! # What it exercises that the other two cannot
//!
//! - [`Line::curvature`](crate::Line::curvature), over a **spread** of real
//!   angles rather than one - see [`RunResult::curvature`] and the scenario
//!   note on [`circuit`]. A gate whose corners are all one radius passes under
//!   any monotone error in the angle.
//! - The personality draws, which turn a seed into seven-plus spans in a frozen
//!   order.
//! - The social axes, which need craft that can see each other: the field here
//!   is four, built into a [`Field`] per craft per tick.
//! - The mistake and provocation counters, which are integer state carried
//!   between ticks and so cannot drift *gradually* - they either agree or they
//!   do not.
//!
//! # No disc image
//!
//! Same reasoning as `oag_physics::probe`, and the same constraint: `data/` is
//! gitignored, so a disc-backed scenario would never run on Windows or macOS,
//! which is exactly where a portability bug shows up. The circuit below is
//! built in this file.
//!
//! **The hulls and the circuit are invented**, per
//! [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md) -
//! round numbers picked to give a craft that corners and a circuit that makes
//! it work, not a measurement of anything. Whether the field gets round a
//! *real* track is `race_ground_truth::the_ai_drives_the_field_along_the_track`,
//! which needs the disc.

use crate::{Context, Driver, Field, Frame, Line, Pilot, Rival, Tuning};
use oag_core::hash::StateHasher;
use oag_core::math::{Quat, Vec3};
use oag_physics::{
    Body, Environment, Handling, Ray, RaycastHit, Raycaster, ShipState, Surface, params,
};

/// Our own fixed timestep. ADR-0007.
const TICK: f32 = 1.0 / 60.0;

/// How many craft take the scenario's grid.
///
/// Four rather than eight: the social axes need only that a craft has somebody
/// ahead, somebody behind and somebody alongside, and four halves the run time
/// of a gate three platforms pay for on every push.
pub const CRAFT: usize = 4;

/// How much room the circuit's corridor gives either side of the line.
const CORRIDOR: f32 = 14.0;

/// The span [`RunResult::curvature`] is measured over.
///
/// Fixed rather than the driver's own lookahead, which varies with speed: a
/// spread that moved because the craft was slower would say nothing about
/// whether the circuit still has corners in it.
const CURVATURE_SPAN: f32 = 30.0;

/// Which committed scenario a reference row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// Four seeded pilots, each a different built-in, released together.
    ///
    /// They start abreast and a little apart, so the field interacts from the
    /// first tick rather than after a settling straight.
    Field,
    /// The same circuit driven by one craft with a `Driver::default()`.
    ///
    /// The plain line-follower: no personality, no rivals, no social axis. It
    /// isolates the line-following arithmetic - the aim point, the curvature
    /// and the speed target - from everything the pilots add on top, so a
    /// divergence in one and not the other says which half moved.
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
    /// Hash of every tick's state and controls, folded together.
    ///
    /// Catches a divergence that happens mid-run and then cancels out, which a
    /// final-state hash alone would miss.
    pub trajectory_hash: u64,
    /// The smallest and largest curvature any craft was steering to, over the
    /// whole run, in radians per unit.
    ///
    /// **Reported so the test can assert the scenario still exercises a range
    /// of angles**, which is the property that makes the hashes above worth
    /// anything for this crate specifically. A circuit that flattened into one
    /// radius - or a driver whose index stopped advancing - would still hash
    /// consistently while testing almost nothing, and the spread is what makes
    /// that visible instead of silent.
    pub curvature: (f32, f32),
    /// How far the *least* travelled craft went, in world units.
    ///
    /// The other half of "this scenario still tests something". A hash over
    /// four craft that spun off on tick 30 and lay still for the rest of the
    /// run is perfectly reproducible and says nothing about a driver, and the
    /// two failures that would produce it - a controller that cannot hold this
    /// circuit, and a grid placed beside the line rather than on it - are
    /// exactly the ones an invented fixture invites.
    ///
    /// **Path length, not line index.** A driver's index is the answer to a
    /// windowed nearest-point search and jitters back and forth by a point or
    /// two every tick; summing its deltas measures that jitter far more than it
    /// measures progress. How far the hull actually moved cannot be argued
    /// with.
    pub travelled: f32,
}

/// An infinite horizontal floor at `y = 0`.
///
/// No walls: a craft that leaves the line keeps going rather than being caught
/// by geometry, so what the hash covers is the controller's own arithmetic and
/// not a contact response that `oag_physics::probe` already gates.
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
///
/// The airbrakes are the difference from `closed_loop.rs`'s default fixture,
/// which ends `..Handling::ZERO` and so leaves every airbrake and brake term at
/// zero. That fixture is deliberate there - its regression bounds were
/// calibrated against a craft whose airbrakes do nothing - but it is the wrong
/// choice here: a determinism gate that leaves a whole control path at zero
/// hashes a path nobody drives. This one commands, ramps and spends all of
/// them.
///
/// **None of these is the game's**, per ADR-0006.
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

/// The tuning the invented craft is driven with.
///
/// `Tuning::default()`'s `lateral_accel` is a measurement against the *real*
/// hulls, which corner harder than this one; `closed_loop.rs` records why the
/// two are not commensurate. The gate wants a craft that holds its line, so it
/// takes the same invented figure that file does.
fn tuning() -> Tuning {
    Tuning {
        lateral_accel: 55.0,
        ..Tuning::default()
    }
}

/// A closed circuit with corners of several different radii.
///
/// The shape is the scenario's whole point. An oval has one radius, so every
/// curvature the drivers ever compute is the same number and any error in the
/// angle that is monotone in that number moves nothing here. This loop opens
/// out into long sweeps and closes into tight bends, so
/// [`Line::curvature`](crate::Line::curvature) is sampled across a real range
/// and the speed target is repeatedly re-decided. [`RunResult::curvature`]
/// reports the spread that actually resulted, and `tests/determinism.rs`
/// asserts on it - a circuit that flattened would otherwise keep hashing
/// consistently while testing almost nothing.
///
/// # No `sin` or `cos`, and that is not a style choice
///
/// The obvious way to draw a circuit is arcs off `angle.cos()`, which is what
/// `closed_loop.rs`'s oval does. **It cannot be done here.** Those are the same
/// platform transcendentals this gate exists to keep out of the simulation, and
/// a fixture built from them would put a platform-dependent *circuit* under a
/// cross-platform hash - the gate would fail on macOS for a reason that had
/// nothing to do with the drivers, which is worse than not gating at all.
///
/// So the loop is a closed uniform Catmull-Rom spline through the control
/// points below: polynomial evaluation, `+ - * /` only, identical everywhere.
/// It also closes exactly by construction, and a seam would otherwise read as
/// one enormous curvature sample.
pub fn circuit() -> Line {
    /// The outline, anticlockwise in the XZ plane. Roughly evenly spaced, which
    /// is what keeps a *uniform* Catmull-Rom from overshooting between them.
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
    /// Samples per control-point span. Twelve spans of these over a loop about
    /// 2,600 units round is a point every five units or so, the order of
    /// magnitude a real track's spline carries.
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
            // The uniform Catmull-Rom basis, halved as it is conventionally
            // written. Interpolating, so the curve passes through every control
            // point and the outline above is the circuit rather than a hint at
            // it.
            points.push(
                (p1 * 2.0
                    + (p2 - p0) * t
                    + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2
                    + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3)
                    * 0.5,
            );
        }
    }

    // A corridor an even `CORRIDOR` either side. A real track's comes off the
    // disc, one bound per control point; what is under gate here is the driver.
    //
    // **The frame is `along.cross(Y)`, the driver's right**, matching
    // `Body::right` and the disc's own `sample.lateral`. Every synthetic
    // corridor in this crate had this backwards once, and symmetric bounds hid
    // it until a term needed the sign - see `closed_loop.rs`.
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

/// Places craft `slot` on the line's own first points, staggered like a grid.
///
/// Two columns eleven units apart across the line and twenty-two along it,
/// which is a grid's shape without being the recovered one - the real geometry
/// is `oag_gameplay::spawn::grid_pose` and belongs to a race, not to a gate.
///
/// **Derived from the line rather than written out**, so an edit to [`circuit`]
/// moves the grid with it instead of leaving four craft beside the track.
fn grid_pose(line: &Line, slot: usize) -> (Vec3, Quat) {
    let row = (slot / 2) as f32;
    let side = if slot.is_multiple_of(2) { -1.0 } else { 1.0 };
    let along_index = (row * 4.0) as usize;
    let here = line.point(along_index);
    let along = (line.point(along_index + 1) - here).normalize_or_zero();
    let across = along.cross(Vec3::Y).normalize_or_zero();
    (
        here + across * (side * 11.0) + Vec3::Y * 4.0,
        // `from_rotation_arc` is `sqrt` and arithmetic, not a transcendental -
        // see the note on `circuit` about why that matters here.
        Quat::from_rotation_arc(Vec3::Z, along),
    )
}

/// Across the line at `index`, pointing to the driver's right.
///
/// Built from the line itself rather than read off [`Frame::lateral`], because
/// that is what [`circuit`] built the corridor from in the first place and one
/// derivation is one thing to get the sign of wrong.
fn lateral(line: &Line, index: i64) -> Vec3 {
    let at = index.rem_euclid(line.len().max(1) as i64) as usize;
    let along = (line.point(at + 1) - line.point(at)).normalize_or_zero();
    along.cross(Vec3::Y).normalize_or_zero()
}

/// The rivals craft `slot` can see, out of the poses of all of them.
///
/// The same three questions `oag_game`'s `Race::field_for` answers - nearest
/// ahead, nearest behind, one alongside - resolved here off the drivers' own
/// line indices so the probe needs no race. It is not that function: this one
/// has no wrap-aware lap ordering, because four craft released together on one
/// straight never lap each other inside the scenario's length.
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

/// Runs `scenario` for `ticks` and returns its hashes and its curvature spread.
///
/// The same entry points a race steps: [`Driver::drive`] for the controls and
/// [`oag_physics::step`] for what they do, in that order, once per craft per
/// tick.
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
    // A different built-in pilot per craft, in a fixed order, so the run covers
    // four distinct sets of draws rather than four copies of one. `Solo` takes
    // a `Driver::default()`, whose zero seed is the plain line-follower.
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
            // Seeds a caller would derive from a race seed and a slot. Fixed
            // here, because a gate over an arbitrary seed is a gate over
            // whatever seed happened to run.
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

            // The angle the acos actually saw this tick, at the index this
            // craft is steering from. `CURVATURE_SPAN` rather than the
            // driver's own lookahead because the driver's varies with speed,
            // and a spread that moved because the craft was slower would say
            // nothing about the circuit.
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

/// One craft's hashable state: what it is doing, and what its driver remembers.
///
/// **Every `Driver` field, deliberately.** They are the whole of what a driver
/// carries between ticks, and a field left out of here is a field a divergence
/// can hide in - which is the failure `oag_gameplay::hash` records having had
/// when the pickups landed.
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
}
