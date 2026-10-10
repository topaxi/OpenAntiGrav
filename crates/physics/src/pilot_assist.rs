//! Pilot Assist: a spring on the track spline's corridor that yaws the craft away
//! from a wall it is about to meet and pushes it sideways off one it is touching,
//! paid for with a throttle percentage.
//!
//! HD's `PilotAssist_Update` (`0x000f8fe8`) and `PilotAssist_ThrustScale`
//! (`0x000f8c58`), read live on RPCS3 at confidence 85; 2048 runs the same code on
//! its own tables. Evidence and the measured on/off runs:
//! `docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md`; behaviour:
//! `docs/physics/pilot-assist.md`.
//!
//! Every number comes from [`Params`], which the caller reads off the title's own
//! `handlingstats.xml`. This module holds only the law's own literals: the two
//! probe radii, the blend rates, the upright and heading cosines and the `25`
//! torque threshold.
//!
//! # Levels
//!
//! 2048 offers three: Off, Normal and Extreme ([`Level`]). Both run this one law.
//! Extreme runs it on the global table at full strength; Normal on the ship's own
//! gentler `<Assist>` block, with a blend target that follows speed ([`Ramp`]).
//!
//! # Sign conventions
//!
//! HD's positive local yaw is a right turn and its lateral force runs along body
//! row `0x1d0`, the craft's **left**. This engine's positive local yaw is a left
//! turn ([`crate::engine::steering`]), so the yaw is negated on the way in and the
//! force runs along `-right`. The spline's `lateral` axis points to the craft's
//! right on a craft travelling forward, as HD's live run showed.

use oag_core::math::Vec3;

use crate::ship::Body;

/// The probe radius at the look-ahead point (`0x008a93c8`).
const RADIUS_AHEAD: f32 = 2.0;
/// The probe radius at the craft (`0x008a93cc`).
const RADIUS_HERE: f32 = 5.0;
/// Below this dot of heading and track tangent, a probe reads no push (`0x00782900`).
const MIN_HEADING_DOT: f32 = -0.6;
/// Above this dot of the track's `down` and the craft's up, the craft is not upright
/// on the track and the blend falls (`0x00782910`).
const MAX_UPRIGHT_DOT: f32 = -0.5;
/// When two candidate records' heights differ by more than this, the nearer wins.
const SIBLING_HEIGHT_GAP: f32 = 25.0;
/// The blend's rise, per second (`0x008a93c8`).
const BLEND_RISE: f32 = 2.0;
/// The blend's fall, per second (`0x008a93d0`).
const BLEND_FALL: f32 = 20.0;
/// A yaw torque past this marks the tick as a correction (`0x008a93b0`).
const ACTING_TORQUE: f32 = 25.0;

/// The three settings of 2048's Pilot Assist list, which every title offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Level {
    /// No assist.
    #[default]
    Off,
    /// The gentle shape: the ship's `<Assist>` block, strength ramped in with speed.
    Normal,
    /// HD's authored law (2048 calls it Super).
    Extreme,
}

impl Level {
    /// Every level, in the order a menu lists them.
    pub const ALL: [Self; 3] = [Self::Off, Self::Normal, Self::Extreme];

    /// The token a settings file, a menu row and a recording header spell it as.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Normal => "normal",
            Self::Extreme => "extreme",
        }
    }

    /// [`Self::name`] read back; `None` for any other spelling.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.name() == name)
    }
}

/// Below this ramped strength Normal does not run at all (`0x3a83126f`).
pub const MIN_STRENGTH: f32 = 0.001;

/// 2048's Normal strength: `k * full`, where `k` runs `0..1` from `min_speed` over
/// `ramp_up_range` (`0x811d0010`-`0x811d0130`).
///
/// `full` is `notInUseStrength`, the candidate for the multiplier the original
/// reads at `(*(craft+0x8c))+0x10`; that offset was not tied to the attribute in
/// the code (confidence 65 in `docs/ghidra/functions/vita-2048-eu-v104/pilot-assist.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ramp {
    /// `SteerAssist min_speed`.
    pub min_speed: f32,
    /// `SteerAssist ramp_up_range`.
    pub ramp_up_range: f32,
    /// The strength at the top of the ramp.
    pub full: f32,
}

impl Ramp {
    /// The blend target at `speed`.
    #[must_use]
    pub fn strength(&self, speed: f32) -> f32 {
        let k = ((speed - self.min_speed) / self.ramp_up_range).clamp(0.0, 1.0);
        k * self.full
    }
}

/// One level's law: its numbers and, for Normal, the speed ramp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Law {
    /// The numbers.
    pub params: Params,
    /// `None` runs at strength `1`, as Extreme does.
    pub ramp: Option<Ramp>,
}

/// The laws a race offers for the player's craft and class; `None` where it has none.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Laws {
    /// [`Level::Normal`]'s.
    pub normal: Option<Law>,
    /// [`Level::Extreme`]'s.
    pub extreme: Option<Law>,
}

impl Laws {
    /// The law `level` runs, `None` for Off or a level this race has no law for.
    #[must_use]
    pub fn get(&self, level: Level) -> Option<Law> {
        match level {
            Level::Off => None,
            Level::Normal => self.normal,
            Level::Extreme => self.extreme,
        }
    }
}

/// One speed class's numbers, verbatim from `<PilotAssist>`/`<PilotAssistPenalty>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// Look-ahead at rest.
    pub la_dist_const: f32,
    /// Look-ahead per unit of speed.
    pub la_dist_vel_mul: f32,
    /// Look-ahead ceiling.
    pub la_dist_max: f32,
    /// Lateral push per unit of edge intrusion at the craft.
    pub spring_mul: f32,
    /// Yaw torque per unit of edge intrusion ahead.
    pub torque_mul: f32,
    /// Yaw torque clamp.
    pub max_torque: f32,
    /// No yaw torque while the craft spins faster than this.
    pub max_ang_vel: f32,
    /// Throttle percentage while enabled.
    pub general_thrust_percent: f32,
    /// Throttle percentage while the penalty runs.
    pub thrust_percent_on_use: f32,
    /// Seconds the penalty runs after a correction.
    pub penalty_duration: f32,
}

/// One located spline record, what `AiTrack_LocatePosition` writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corridor {
    /// The lifted spline position.
    pub position: Vec3,
    /// Along the path.
    pub tangent: Vec3,
    /// Into the surface.
    pub down: Vec3,
    /// Across the path, toward the right-hand edge.
    pub lateral: Vec3,
    /// Distance from the spline to the left-hand edge.
    pub half_width_left: f32,
    /// Distance from the spline to the right-hand edge.
    pub half_width_right: f32,
}

/// The located record and, on a fork, the one on the neighbouring path.
pub type Candidates = [Option<Corridor>; 2];

/// What the step needs from outside: the class's numbers, whether the option is on
/// for this craft this tick, and the corridor at the craft and ahead of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
    /// The class's numbers.
    pub params: Params,
    /// The blend target while grounded and upright: `1` for Extreme, [`Ramp::strength`]
    /// for Normal. At or below `0.001` the law does not run.
    pub strength: f32,
    /// The option is on and the race lets it act (racing, not Zone or Detonator).
    pub enabled: bool,
    /// Records located at the craft.
    pub here: Candidates,
    /// Records located at [`look_ahead`].
    pub ahead: Candidates,
}

/// `craft+0x380`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct State {
    /// `+0x4`, the last tick's enabled flag.
    pub enabled: bool,
    /// `+0x0`: the sign of a correction this tick in HD's convention (`-1` turned the
    /// craft left, off a right-hand wall), else `0`. The HUD indicator's source.
    pub acting: i8,
    /// `+0xc`, seconds of thrust penalty left.
    pub penalty_timer: f32,
    /// `+0x10`, `0..1`.
    pub blend: f32,
}

/// What one tick adds to the craft.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Output {
    /// Body-local yaw torque, in this engine's sign.
    pub local_yaw: f32,
    /// World-space force.
    pub world_force: Vec3,
}

/// The point the yaw is steered by: `la = min(laDistConst + speed * laDistVelMul,
/// laDistMax)` along the craft's forward axis.
#[must_use]
pub fn look_ahead(body: &Body, params: &Params) -> Vec3 {
    let speed = body.linear_velocity.length();
    let distance = (params.la_dist_const + speed * params.la_dist_vel_mul).min(params.la_dist_max);
    body.position + body.forward() * distance
}

/// The throttle scale, `PilotAssist_ThrustScale`. The penalty is tested first, so it
/// outlives the option being switched off.
#[must_use]
pub fn thrust_scale(state: &State, params: Option<&Params>) -> f32 {
    let Some(params) = params else {
        return 1.0;
    };
    if state.penalty_timer > 0.0 {
        params.thrust_percent_on_use * 0.01
    } else if state.enabled {
        params.general_thrust_percent * 0.01
    } else {
        1.0
    }
}

/// How far inside `radius` of each edge `point` sits, on one record:
/// `(left >= 0, right <= 0)`.
fn intrusion(point: Vec3, heading: Vec3, radius: f32, record: &Corridor) -> (f32, f32) {
    if heading.dot(record.tangent) < MIN_HEADING_DOT {
        return (0.0, 0.0);
    }
    let d = (point - record.position).dot(record.lateral);
    let left = (radius - record.half_width_left - d).max(0.0);
    let right = (record.half_width_right - radius - d).min(0.0);
    (left, right)
}

/// `PilotAssist_ProbeCorridor` (`0x000f8cb8`) over the located record and its
/// neighbour: the nearer by height when they are more than [`SIBLING_HEIGHT_GAP`]
/// apart, else the one the craft fits better (the smaller push).
fn probe(point: Vec3, heading: Vec3, radius: f32, candidates: &Candidates) -> (f32, f32) {
    let height = |record: &Corridor| (point - record.position).dot(record.down).abs();
    match candidates {
        [Some(first), None] | [None, Some(first)] => intrusion(point, heading, radius, first),
        [Some(first), Some(second)] => {
            let a = intrusion(point, heading, radius, first);
            let b = intrusion(point, heading, radius, second);
            let (ha, hb) = (height(first), height(second));
            let low = ha.min(hb);
            if ha - low > SIBLING_HEIGHT_GAP || hb - low > SIBLING_HEIGHT_GAP {
                if ha > hb { b } else { a }
            } else if b.0.abs().max(b.1.abs()) < a.0.abs().max(a.1.abs()) {
                b
            } else {
                a
            }
        }
        [None, None] => (0.0, 0.0),
    }
}

/// One tick of `PilotAssist_Update`. `grounded` is any hull point in contact this
/// tick.
pub fn update(state: &mut State, input: &Input, body: &Body, grounded: bool, dt: f32) -> Output {
    state.acting = 0;
    state.penalty_timer = (state.penalty_timer - dt).max(0.0);
    state.enabled = input.enabled && input.strength > MIN_STRENGTH;
    if !state.enabled {
        return Output::default();
    }
    let params = &input.params;
    let heading = body.forward();
    let (ahead_left, ahead_right) = probe(
        look_ahead(body, params),
        heading,
        RADIUS_AHEAD,
        &input.ahead,
    );
    let (here_left, here_right) = probe(body.position, heading, RADIUS_HERE, &input.here);

    let upright = input
        .here
        .iter()
        .flatten()
        .next()
        .is_none_or(|record| record.down.dot(body.up()) <= MAX_UPRIGHT_DOT);
    let target = if grounded && upright {
        input.strength
    } else {
        0.0
    };
    state.blend = if state.blend <= target {
        (state.blend + BLEND_RISE * dt).min(target)
    } else {
        (state.blend - BLEND_FALL * dt).max(target)
    };

    // HD's push runs along its left row; `-right` here.
    let world_force = -body.right() * ((here_left + here_right) * params.spring_mul * state.blend);

    let mut torque = ((ahead_left + ahead_right) * params.torque_mul * state.blend)
        .clamp(-params.max_torque, params.max_torque);
    if body.angular_velocity.length() > params.max_ang_vel {
        torque = 0.0;
    }
    if torque.abs() > ACTING_TORQUE {
        state.acting = if torque > 0.0 { 1 } else { -1 };
        state.penalty_timer = params.penalty_duration;
    }
    Output {
        local_yaw: -torque,
        world_force,
    }
}

#[cfg(test)]
mod tests;
