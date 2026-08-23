//! Finding an input script that drives our simulation through a world-space gate.
//!
//! Authoring an input script by hand costs an emulator round trip per guess: set
//! up the save state, replay, look at where the craft ended up, adjust a hold by
//! five ticks, repeat. Our own simulation tracks the original closely enough over
//! the first few hundred ticks (see `docs/tools/oag-trace.md`) that the guessing
//! can happen here instead, at no emulator cost at all, and only the script that
//! already works gets replayed into PPSSPP.
//!
//! So this is not a *search* over scripts. It is a controller - pure pursuit,
//! bang-bang on the d-pad, the same shape as the one
//! `scripts/psp-autopilot.py` flies the original with - run against
//! [`crate::replay::drive_with`], whose **recording** of what it pressed is the
//! deliverable. A controller that reaches the gate emits one script; there is
//! nothing to optimise over because the controller closes the loop itself.
//!
//! # What this is not
//!
//! **Nothing here is a claim about the original's AI.** The steering law was
//! written to get a craft to a point, and no page in `docs/` should cite it. What
//! it produces - a per-tick button state - is authored input in exactly the sense
//! [`crate::script`] means, and carries no derived game data.
//!
//! **A plan that crosses the gate in our simulation has not been shown to cross
//! it in the original.** The two agree closely early and drift later, so a short
//! plan is likelier to transfer than a long one, and the emulator replay is the
//! only thing that settles it. See `docs/tools/autopilot-planning.md`.

use oag_core::math::Vec3;
use oag_formats::track::{HOVER_LIFT, Sample};
use oag_gameplay::input::Button;
use oag_physics::{Environment, Handling, Raycaster, ShipState};

use crate::replay::{DriveOptions, Held, drive_with};
use crate::trace::Trace;

/// A world-space gate a plan has to pass through.
///
/// A plane and a width rather than a box, because what a plan has to get right
/// is *where it crosses*, and the answer wanted from a run is a signed lateral
/// miss distance rather than a yes or no. A speed pad's own trigger volume is a
/// box and is tested separately - see [`Plan::inside`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gate {
    /// The point the plan aims to cross.
    pub centre: Vec3,
    /// The direction a craft should be travelling when it crosses. Normalised on
    /// construction; the gate plane is the plane through `centre` normal to this.
    pub direction: Vec3,
    /// Half the gate's width, across the track. Only used to report whether a
    /// crossing counts as a hit.
    pub half_width: f32,
}

impl Gate {
    /// A gate with its direction normalised.
    #[must_use]
    pub fn new(centre: Vec3, direction: Vec3, half_width: f32) -> Self {
        Self {
            centre,
            direction: direction.normalize_or_zero(),
            half_width,
        }
    }
}

/// The steering law's tunables.
///
/// Defaults are `scripts/psp-autopilot.py`'s, which were tuned against the
/// original rather than against us; they are a starting point here rather than a
/// measurement, and a plan that oscillates wants a longer lookahead before it
/// wants anything else.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Lookahead distance at a standstill.
    pub look_min: f32,
    /// Extra lookahead per unit of speed.
    pub look_speed: f32,
    /// Lookahead ceiling.
    pub look_max: f32,
    /// Lateral error below which nothing is held: the straight-ahead band.
    pub deadband: f32,
    /// Lateral error past which the inside airbrake comes on as well.
    pub brake_at: f32,
    /// Forward alignment below which the throttle is released, so a craft aimed
    /// away from the line coasts round instead of driving further off it.
    pub reverse_at: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            look_min: 18.0,
            look_speed: 0.55,
            look_max: 90.0,
            deadband: 0.045,
            brake_at: 0.32,
            reverse_at: -0.2,
        }
    }
}

/// The line the controller chases: a finite run of points, start to gate and on.
///
/// Not a ring, unlike `scripts/psp-autopilot.py`'s: a plan ends at its gate, so
/// there is nothing to wrap around to and a lookahead that runs off the end
/// should stop at the end rather than teleport to the beginning.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// The points, in the order they are driven.
    pub points: Vec<Vec3>,
    /// Index of the point that sits on the gate itself.
    pub gate_index: usize,
}

impl Path {
    /// Builds the line from the craft's start to a gate, along the track.
    ///
    /// The spine is the track's own authored racing line - the same
    /// `lift + racing_line * lateral` `oag-trace track` writes as its `line_*`
    /// columns, so the planner chases the line that is on the disc rather than a
    /// line it invented. Onto that is added a ramp: the gate's own offset from
    /// the racing line, faded in linearly from nothing at the start to all of it
    /// at the gate. A gate that sits on the line therefore leaves the racing line
    /// untouched, and one that sits 8 units off it is converged onto over the
    /// whole approach rather than swerved at in the last few metres.
    ///
    /// `run_on` extends the line past the gate along the gate's own direction, so
    /// that pure pursuit still has something to aim at while it is crossing.
    /// Without it the aim point collapses onto the gate and the steering spikes
    /// exactly where the crossing is being measured.
    ///
    /// Returns `None` if `samples` is empty.
    #[must_use]
    pub fn to_gate(samples: &[Sample], start: Vec3, gate: &Gate, run_on: f32) -> Option<Self> {
        let line = |s: &Sample| -> Vec3 {
            let pos = Vec3::from_array(s.pos);
            let down = Vec3::from_array(s.down);
            let lateral = Vec3::from_array(s.lateral);
            pos - HOVER_LIFT * down + s.racing_line * lateral
        };
        let count = samples.len();
        if count == 0 {
            return None;
        }

        let nearest = |target: Vec3, from: usize| -> usize {
            (0..count)
                .map(|step| (from + step) % count)
                .min_by(|a, b| {
                    let da = line(&samples[*a]).distance_squared(target);
                    let db = line(&samples[*b]).distance_squared(target);
                    da.total_cmp(&db)
                })
                .unwrap_or(from)
        };
        // The start is located over the whole ring; the gate only over what lies
        // *ahead* of the start, so a gate a craft has already passed is planned
        // as a lap away rather than as a reverse.
        let first = nearest(start, 0);
        let last = nearest(gate.centre, first);
        let span = (last + count - first) % count;

        let mut points = Vec::with_capacity(span + 8);
        let offset = gate.centre - line(&samples[last]);
        for step in 0..=span {
            let fraction = if span == 0 {
                1.0
            } else {
                step as f32 / span as f32
            };
            points.push(line(&samples[(first + step) % count]) + fraction * offset);
        }
        let gate_index = points.len() - 1;

        let mut travelled = 10.0;
        while travelled <= run_on {
            points.push(gate.centre + gate.direction * travelled);
            travelled += 10.0;
        }
        Some(Self { points, gate_index })
    }

    /// The nearest point to `position`, searched in a window around `around`.
    ///
    /// Windowed rather than global for the reason `psp-autopilot.py` gives:
    /// a track passes near itself, and a global search lets the progress index
    /// jump across the gap. Here it also keeps a craft that has stopped from
    /// latching onto the run-on points behind the gate.
    #[must_use]
    pub fn nearest(&self, position: Vec3, around: usize) -> usize {
        let back = around.saturating_sub(12);
        let ahead = (around + 90).min(self.points.len().saturating_sub(1));
        (back..=ahead)
            .min_by(|a, b| {
                self.points[*a]
                    .distance_squared(position)
                    .total_cmp(&self.points[*b].distance_squared(position))
            })
            .unwrap_or(around)
    }

    /// The point `distance` further along the line, clamped to its end.
    #[must_use]
    pub fn ahead(&self, index: usize, distance: f32) -> Vec3 {
        let mut travelled = 0.0;
        let mut at = index;
        while travelled < distance && at + 1 < self.points.len() {
            travelled += self.points[at].distance(self.points[at + 1]);
            at += 1;
        }
        self.points[at]
    }
}

/// Where and how a run crossed the gate plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    /// The tick the crossing was detected on: the first whose signed distance to
    /// the gate plane is not negative.
    pub tick: usize,
    /// The crossing point itself, interpolated between the two straddling ticks.
    pub point: Vec3,
    /// Signed distance from the gate's centre across the track, positive to the
    /// craft's right. Compare against [`Gate::half_width`].
    pub offset: f32,
    /// Speed at the crossing tick.
    pub speed: f32,
}

/// What [`to_gate`] found.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// The input the controller chose, tick by tick: the artefact.
    pub script: crate::Script,
    /// The run it chose that input on.
    pub trace: Trace,
    /// The gate crossing, if there was one.
    pub crossing: Option<Crossing>,
    /// The first tick the craft was inside the pad's own trigger box, when the
    /// gate came from a pad and so there is a box to ask.
    pub inside: Option<usize>,
}

/// Everything the planner needs beyond the physics itself.
#[derive(Debug, Clone)]
pub struct PlanOptions {
    /// How the run is stepped. `ticks` is the giving-up point, not the length:
    /// a plan stops when it has crossed its gate.
    pub drive: DriveOptions,
    /// Hold nothing for this many ticks at the start.
    ///
    /// **Not a stylistic choice.** `scripts/psp-trace.py --script-lead N` never
    /// sends a script's first `N` ticks - see
    /// [`crate::Script::with_capture_lead`] - so a plan that used them would be
    /// driving an input the emulator will not receive, and the run validated here
    /// would not be the run replayed there. Planning them as released makes the
    /// two the same run. Default 2, matching every capture taken so far.
    pub lead: usize,
    /// Keep planning this many ticks past the crossing, so the emitted script
    /// does not end on the gate itself.
    pub after: usize,
    /// The steering law's tunables.
    pub tuning: Tuning,
    /// A pad trigger box to test containment against, when the gate came from one.
    pub volume: Option<oag_formats::pads::PadVolume>,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            drive: DriveOptions {
                ticks: 1800,
                ..DriveOptions::default()
            },
            lead: 2,
            after: 60,
            tuning: Tuning::default(),
            volume: None,
        }
    }
}

/// Runs the controller and returns the script it chose.
///
/// The loop is [`drive_with`]'s, so the physics, the input mapping and the
/// sampled-before-the-step convention are the ones every other scenario run uses.
/// All that is added is the decision: aim a lookahead distance along `path`,
/// resolve the direction to it onto the craft's own right axis, and hold the
/// d-pad that reduces it.
#[must_use]
pub fn to_gate<R: Raycaster + ?Sized>(
    initial: ShipState,
    handling: &Handling,
    environment: &Environment,
    raycaster: &R,
    path: &Path,
    gate: &Gate,
    options: &PlanOptions,
) -> Plan {
    let mut index = 0usize;
    let mut previous: Option<(Vec3, f32)> = None;
    let mut crossing: Option<Crossing> = None;
    let mut inside: Option<usize> = None;
    let mut stop_at: Option<usize> = None;

    let (trace, chosen) = drive_with(
        initial,
        handling,
        environment,
        raycaster,
        &options.drive,
        |tick, state| {
            let position = state.body.position;
            if let Some(volume) = &options.volume
                && inside.is_none()
                && volume.contains(position.to_array())
            {
                inside = Some(tick);
            }

            let distance = (position - gate.centre).dot(gate.direction);
            if crossing.is_none()
                && let Some((before, was)) = previous
                && was < 0.0
                && distance >= 0.0
            {
                let fraction = -was / (distance - was);
                let point = before + (position - before) * fraction;
                crossing = Some(Crossing {
                    tick,
                    point,
                    offset: lateral_miss(point, gate),
                    speed: state.body.linear_velocity.length(),
                });
                stop_at = Some(tick + options.after);
            }
            previous = Some((position, distance));

            if stop_at.is_some_and(|end| tick >= end) {
                return None;
            }
            if tick < options.lead {
                return Some(Held::default());
            }
            // Past the gate the plan is over and the line has run out, so the
            // steering law has nothing left to aim at: keeping it running would
            // chase the last run-on point from behind and read as "turn round".
            // The tail is only there so the script does not end on the gate.
            if crossing.is_some() {
                return Some(Held::from_buttons(1 << Button::Cross.index()));
            }

            index = path.nearest(position, index);
            Some(steer(state, path, index, &options.tuning))
        },
    );

    Plan {
        script: crate::Script { states: chosen },
        trace,
        crossing,
        inside,
    }
}

/// One tick of the controller: what to hold, given where the craft is.
///
/// Pure pursuit. The aim point is a fixed distance ahead along the line, scaled
/// with speed so a fast craft looks further; the error is that direction's
/// component along the ship's own right axis, which is the axis the game's
/// steering ramp consumes. Bang-bang, because a PSP d-pad has three positions and
/// a script is a list of button names.
///
/// The inside airbrake past a wider threshold than the steering's is what a
/// player does, so a straight is never braked.
fn steer(state: &ShipState, path: &Path, index: usize, tuning: &Tuning) -> Held {
    let body = &state.body;
    let speed = body.linear_velocity.length();
    let look = (tuning.look_min + tuning.look_speed * speed).min(tuning.look_max);
    let want = (path.ahead(index, look) - body.position).normalize_or_zero();
    let error = want.dot(body.right());
    let forward = want.dot(body.forward());

    let mut buttons = 1u32 << Button::Cross.index();
    if error > tuning.deadband {
        buttons |= 1 << Button::Right.index();
        if error > tuning.brake_at {
            buttons |= 1 << Button::R.index();
        }
    } else if error < -tuning.deadband {
        buttons |= 1 << Button::Left.index();
        if error < -tuning.brake_at {
            buttons |= 1 << Button::L.index();
        }
    }
    // Aimed away from where the line goes: thrust would only drive it further
    // off, so coast and let the steering come round.
    if forward < tuning.reverse_at {
        buttons &= !(1 << Button::Cross.index());
    }
    Held::from_buttons(buttons)
}

/// How far across the gate a crossing point missed its centre, positive right.
///
/// "Across" is the gate's direction crossed with world up, which is right for
/// every gate that is not banked to vertical. A pad on a wall would need the
/// track's own up and none exists on the shipped tracks; the miss distance is a
/// report, not simulation state, so this stays the simple reading and says so.
fn lateral_miss(point: Vec3, gate: &Gate) -> f32 {
    let across = gate.direction.cross(Vec3::Y).normalize_or_zero();
    (point - gate.centre).dot(across)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A straight run of samples along `+X`, flat, three units wide, no offset line.
    fn straight(count: usize) -> Vec<Sample> {
        (0..count)
            .map(|index| Sample {
                pos: [index as f32 * 10.0, 0.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                down: [0.0, -1.0, 0.0],
                lateral: [0.0, 0.0, 1.0],
                half_width_left: 10.0,
                half_width_right: 10.0,
                ai_bound_left: -8.0,
                ai_bound_right: 8.0,
                racing_line: 0.0,
                section_id: 0,
                flags: 0,
            })
            .collect()
    }

    #[test]
    fn the_line_ends_on_the_gate_and_reaches_it_gradually() {
        let samples = straight(11);
        let gate = Gate::new(Vec3::new(100.0, HOVER_LIFT, 6.0), Vec3::X, 4.0);
        let path = Path::to_gate(&samples, Vec3::new(0.0, HOVER_LIFT, 0.0), &gate, 0.0).unwrap();

        assert_eq!(path.gate_index, path.points.len() - 1);
        assert!((path.points[path.gate_index] - gate.centre).length() < 1e-4);
        // Halfway along, half the gate's own offset has been taken up: the ramp
        // is what stops a plan swerving at the gate instead of converging on it.
        assert!((path.points[5].z - 3.0).abs() < 1e-4);
        assert!(path.points[0].z.abs() < 1e-4);
    }

    #[test]
    fn the_run_on_continues_past_the_gate_along_its_own_direction() {
        let samples = straight(11);
        let gate = Gate::new(Vec3::new(100.0, HOVER_LIFT, 0.0), Vec3::X, 4.0);
        let path = Path::to_gate(&samples, Vec3::new(0.0, HOVER_LIFT, 0.0), &gate, 30.0).unwrap();

        assert_eq!(path.points.len(), path.gate_index + 4);
        assert!((path.points.last().unwrap().x - 130.0).abs() < 1e-4);
    }

    #[test]
    fn a_lookahead_past_the_end_stops_at_the_end() {
        let samples = straight(4);
        let gate = Gate::new(Vec3::new(30.0, HOVER_LIFT, 0.0), Vec3::X, 4.0);
        let path = Path::to_gate(&samples, Vec3::new(0.0, HOVER_LIFT, 0.0), &gate, 0.0).unwrap();

        assert_eq!(path.ahead(0, 1.0e6), *path.points.last().unwrap());
    }

    #[test]
    fn the_miss_distance_is_positive_to_the_right_of_a_gate() {
        // Facing `+X` with world up `+Y`, the driver's right is `+Z`.
        let gate = Gate::new(Vec3::ZERO, Vec3::X, 4.0);
        assert!(lateral_miss(Vec3::new(0.0, 0.0, 2.0), &gate) > 0.0);
        assert!(lateral_miss(Vec3::new(0.0, 0.0, -2.0), &gate) < 0.0);
    }

    #[test]
    fn the_steering_law_holds_the_d_pad_the_error_asks_for() {
        let samples = straight(11);
        let gate = Gate::new(Vec3::new(100.0, HOVER_LIFT, 40.0), Vec3::X, 4.0);
        let path = Path::to_gate(&samples, Vec3::new(0.0, HOVER_LIFT, 0.0), &gate, 30.0).unwrap();

        let mut state = ShipState::default();
        state.body.position = Vec3::new(0.0, HOVER_LIFT, 0.0);
        let held = steer(&state, &path, 0, &Tuning::default());

        // The line runs away to the craft's right, and a default body faces `-Z`
        // with its right at `+X`, so the aim point is off to the *left* of the
        // nose. What matters is that exactly one of the two is held.
        assert!(held.is_held(Button::Left) != held.is_held(Button::Right));
    }
}
