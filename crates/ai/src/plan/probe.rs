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

use super::{Course, Craft, Driver, Failure, Run, SpeedPlan, Tuning, forward_speed, longitudinal};

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
    pub climb: f32,
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
    let yaw = crate::hull_yaw_ceiling(&craft.handling);
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
                last_position: None,
            };
        }
    }
    rows
}

fn fallback_lateral<R: Raycaster + ?Sized>(course: &Course<'_, R>, index: usize) -> Vec3 {
    let n = course.line.len();
    let along = course.line.point((index + 1) % n) - course.line.point(index);
    along.cross(Vec3::Y).normalize_or_zero()
}
