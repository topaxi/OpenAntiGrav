//! A diagnostic drive: the plan's own steering at one fixed speed, logged per
//! tick, so a wall the plan cannot get past can be told apart from a speed it
//! cannot carry.
//!
//! Nothing here is learned and nothing feeds a race. It exists because "the
//! craft touches this wall at 15 units/s too" is a statement about the line or
//! the steering, and answering it needs the craft's offset from the line, the
//! room the corridor gives there and what the driver asked for, tick by tick.

use oag_core::math::Vec3;
use oag_physics::Raycaster;

use super::{
    Course, Craft, Driver, Failure, LEAD_TICKS, RESPAWN_SKIP, Run, SpeedPlan, Tuning,
    forward_speed, longitudinal,
};

/// One tick of a [`drive`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
    pub tick: u32,
    pub index: u32,
    pub speed: f32,
    /// Signed distance from the line point across it, positive to the right.
    pub offset: f32,
    /// Height of the craft above the line point, along world Y.
    pub height: f32,
    /// The corridor's left bound there (zero or negative), or NaN with none.
    pub left: f32,
    /// The corridor's right bound there (zero or positive), or NaN with none.
    pub right: f32,
    pub steer: f32,
    pub position: Vec3,
    /// World-Y velocity.
    pub climb: f32,
    /// World-Y component of the craft's nose direction.
    pub pitch: f32,
    pub contact: bool,
    pub airborne: bool,
    /// What the tick counted as, the plan's own rules.
    pub failure: Option<Failure>,
}

/// Drives `course` from the grid at a speed held to `speed` (full throttle
/// under it, the plan's brake law over it) for `ticks` ticks, logging each.
///
/// A rescue-class failure puts the craft back down `skip` samples on, as the
/// plan's verification does; a wall contact only logs.
#[must_use]
pub fn drive<R: Raycaster + ?Sized>(
    course: &Course<'_, R>,
    craft: &Craft,
    tuning: &Tuning,
    speed: f32,
    ticks: u32,
    skip: usize,
) -> Vec<Row> {
    let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
    let n = course.line.len();
    let mut run = SpeedPlan::start_run(craft);
    let mut rows = Vec::with_capacity(ticks as usize);
    for _ in 0..ticks {
        let tick = course.tick(&mut run, craft, tuning, yaw, |run, _| {
            longitudinal(forward_speed(&run.state), speed)
        });
        let index = run.driver.index as usize;
        let point = course.line.point(index);
        let position = run.state.body.position;
        let frame = course.line.aim(index, 0.0).corridor;
        let lateral = frame.map_or_else(|| fallback_lateral(course, index), |f| f.lateral);
        rows.push(Row {
            tick: run.tick,
            index: run.driver.index,
            speed: forward_speed(&run.state),
            offset: (position - point).dot(lateral),
            height: position.y - point.y,
            left: frame.map_or(f32::NAN, |f| f.left),
            right: frame.map_or(f32::NAN, |f| f.right),
            steer: tick.steer,
            position,
            climb: run.state.body.linear_velocity.y,
            pitch: run.state.body.forward().y,
            contact: tick.contact,
            airborne: run.state.time_airborne > 0.0,
            failure: tick.failure,
        });
        if matches!(tick.failure, Some(f) if f != Failure::Wall) && n > 0 {
            let to = (index + skip) % n;
            run = Run {
                state: (course.respawn)(to),
                driver: Driver {
                    index: to as u32,
                    ..Driver::default()
                },
                progress: run.progress + skip as i64,
                tick: run.tick,
                slow: 0,
                best: run.progress + skip as i64,
                best_tick: run.tick,
                stuck_limit: run.stuck_limit,
                last_position: None,
            };
        }
    }
    rows
}

/// One tick of a [`verify`]: a [`Row`] and what the plan asked for there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Followed {
    pub row: Row,
    /// The plan's lowest target over the lead the follower reads.
    pub target: f32,
    /// The plan's ceiling at the sample under the craft.
    pub ceiling: f32,
    /// Whether that sample is a held run-up.
    pub hold: bool,
    pub thrust: f32,
    pub brake: f32,
    /// Samples travelled since the start, as the verification counts them.
    pub progress: i64,
    /// Whether this tick put the craft back on the line, as the
    /// verification's rescue does.
    pub respawned: bool,
}

/// Replays `plan`'s verification on `course`: two laps from `craft`'s start
/// following the plan at full share, nothing learned, every tick logged. The
/// same loop as the plan's own verification (`SpeedPlan::verify`), so the
/// failures it logs are the ones that kept the plan from verifying clean.
///
/// `stuck`, when given, also counts that many ticks without progress as a
/// [`Failure::Stalled`] and puts the craft back on, which the verification
/// itself never does: a craft held in place by the pre-integration clip keeps
/// its velocity, so it is never slow enough to stall.
#[must_use]
pub fn verify<R: Raycaster + ?Sized>(
    course: &Course<'_, R>,
    craft: &Craft,
    tuning: &Tuning,
    plan: &SpeedPlan,
    stuck: Option<u32>,
) -> Vec<Followed> {
    let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
    let n = plan.len() as i64;
    let mut run = SpeedPlan::start_run(craft);
    if let Some(stuck) = stuck {
        run.stuck_limit = stuck;
    }
    let mut rows = Vec::new();
    let limit = (4 * n as u64).max(4_000) as u32 * 2;
    while run.progress < 2 * n && run.tick < limit {
        let asked = std::cell::Cell::new((1.0, 0.0));
        let target = std::cell::Cell::new(f32::INFINITY);
        let tick = course.tick(&mut run, craft, tuning, yaw, |run, _| {
            let speed = forward_speed(&run.state);
            target.set(plan.target_ahead(run.driver.index as usize, speed, LEAD_TICKS, course.dt));
            asked.set(plan.follow(run, course.dt, 1.0));
            asked.get()
        });
        let index = run.driver.index as usize;
        let point = course.line.point(index);
        let position = run.state.body.position;
        let frame = course.line.aim(index, 0.0).corridor;
        let lateral = frame.map_or_else(|| fallback_lateral(course, index), |f| f.lateral);
        let (thrust, brake) = asked.get();
        let respawned = matches!(tick.failure, Some(f) if f != Failure::Wall);
        rows.push(Followed {
            row: Row {
                tick: run.tick,
                index: run.driver.index,
                speed: forward_speed(&run.state),
                offset: (position - point).dot(lateral),
                height: position.y - point.y,
                left: frame.map_or(f32::NAN, |f| f.left),
                right: frame.map_or(f32::NAN, |f| f.right),
                steer: tick.steer,
                position,
                climb: run.state.body.linear_velocity.y,
                pitch: run.state.body.forward().y,
                contact: tick.contact,
                airborne: run.state.time_airborne > 0.0,
                failure: tick.failure,
            },
            target: target.get(),
            ceiling: plan.ceiling(index),
            hold: plan.holds(index),
            thrust,
            brake,
            progress: run.progress,
            respawned,
        });
        if respawned && n > 0 {
            let to = (index + RESPAWN_SKIP) % n as usize;
            run = Run {
                state: (course.respawn)(to),
                driver: Driver {
                    index: to as u32,
                    ..Driver::default()
                },
                progress: run.progress + RESPAWN_SKIP as i64,
                tick: run.tick,
                slow: 0,
                best: run.progress + RESPAWN_SKIP as i64,
                best_tick: run.tick,
                stuck_limit: run.stuck_limit,
                last_position: None,
            };
        }
    }
    rows
}

/// One stage of a re-stepped tick ([`stages`]): where the craft was after it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage {
    pub name: &'static str,
    pub position: Vec3,
    pub velocity: Vec3,
}

/// Replays `plan`'s verification ([`verify`]) up to tick `at` and steps that
/// tick again by hand, in `oag_physics::step`'s own order, recording the
/// craft's position and velocity after each stage, plus the wall response's
/// contact counts. For a craft whose position stops moving while its speed
/// does not: which stage puts it back.
#[must_use]
pub fn stages<R: Raycaster + ?Sized>(
    course: &Course<'_, R>,
    craft: &Craft,
    tuning: &Tuning,
    plan: &SpeedPlan,
    at: u32,
) -> Restep {
    let yaw = crate::hull_yaw_ceiling(&craft.handling, &craft.start.body);
    let mut run = SpeedPlan::start_run(craft);
    while run.tick + 1 < at {
        let tick = course.tick(&mut run, craft, tuning, yaw, |run, _| {
            plan.follow(run, course.dt, 1.0)
        });
        if matches!(tick.failure, Some(f) if f != Failure::Wall) {
            return Restep {
                stages: Vec::new(),
                wall: oag_physics::WallResponse::default(),
                mag_lock: None,
                escape: Vec3::ZERO,
                track_sample: None,
                mag_contact: None,
            };
        }
    }
    let ctx = crate::Context {
        yaw_ceiling: Some(yaw),
        air_trim: crate::air_trim(&craft.handling, run.state.air_pitch_bias),
        ..crate::Context::new(course.line, tuning)
    };
    let mut controls = run.driver.drive(&run.state, &ctx);
    let (thrust, brake) = plan.follow(&run, course.dt, 1.0);
    let differential = controls.airbrake_right - controls.airbrake_left;
    let (left, right) = super::airbrakes(brake, differential);
    controls.thrust = thrust;
    controls.airbrake_left = left;
    controls.airbrake_right = right;
    let index = run.driver.index as usize;
    let before = run.state.body.position;
    let env = oag_physics::Environment {
        pad_hit: (course.pad)(run.last_position, before),
        ..course.env_at(index)
    };
    let dt = oag_physics::clamp_dt(course.dt);
    let state = &mut run.state;
    let mut out = Vec::new();
    let mut note = |name, state: &oag_physics::ShipState| {
        out.push(Stage {
            name,
            position: state.body.position,
            velocity: state.body.linear_velocity,
        });
    };
    note("start", state);
    state.body.clear_accumulators();
    state.body.torque += state.wall_torque;
    state.wall_torque = Vec3::ZERO;
    oag_physics::wall::pre_integration_clip(state, &craft.handling, &env, course.raycaster, dt);
    note("pre_integration_clip", state);
    let evaluated = oag_physics::forces::evaluate(
        state,
        &controls,
        &craft.handling,
        &env,
        course.raycaster,
        dt,
    );
    note("forces::evaluate", state);
    let mag_lock = evaluated.mag_lock;
    let escape = evaluated.hover.escape;
    let before_integration = state.body.position;
    oag_physics::integrate(&mut state.body, state.inertia_frame, dt);
    note("integrate", state);
    if let Some((correction, velocity)) =
        oag_physics::hover::sweep(state, &env, course.raycaster, before_integration)
    {
        state.body.position += correction;
        state.body.linear_velocity = velocity;
    }
    note("hover::sweep", state);
    let wall = oag_physics::wall::resolve(state, &craft.handling, &env, course.raycaster);
    note("wall::resolve", state);
    Restep {
        stages: out,
        wall,
        mag_lock,
        escape,
        track_sample: env.track_sample,
        mag_contact: state.mag_contact,
    }
}

/// What [`stages`] found.
#[derive(Debug, Clone, PartialEq)]
pub struct Restep {
    pub stages: Vec<Stage>,
    pub wall: oag_physics::WallResponse,
    /// The magstrip hold the force step ran, if any.
    pub mag_lock: Option<oag_physics::Hold>,
    /// The hover's penetration escape this tick.
    pub escape: Vec3,
    /// The spline sample the step was given.
    pub track_sample: Option<oag_physics::TrackSample>,
    /// The last magstrip hit the hold judged against.
    pub mag_contact: Option<oag_physics::MagContact>,
}

fn fallback_lateral<R: Raycaster + ?Sized>(course: &Course<'_, R>, index: usize) -> Vec3 {
    let n = course.line.len();
    let along = course.line.point((index + 1) % n) - course.line.point(index);
    along.cross(Vec3::Y).normalize_or_zero()
}
