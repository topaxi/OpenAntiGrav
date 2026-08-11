//! Turning a craft's state and a line into the controls a craft is flown with.
//!
//! # The plant, and why the controller has the shape it has
//!
//! `oag_physics::engine::steering` feeds `Accumulators::local_angular`, which is
//! **torque**. So the steering input commands yaw *acceleration* - not a yaw
//! rate, and certainly not a heading. `controls::ramp_steering` puts a
//! first-order lag in front of that, because the steering state moves toward its
//! target at a finite rate rather than jumping.
//!
//! A controller that sets steering in proportion to how far off the line it is
//! is therefore proportional feedback around a double integrator with a lag. It
//! overshoots, corrects, overshoots the other way, and keeps doing it. **That is
//! what the first version of this crate did, and it drove wall to wall.** No
//! gain fixes it; the loop has to be closed one derivative in.
//!
//! So: the geometry produces a **target turn rate**, and the loop is closed on
//! the turn rate the craft actually has. Proportional feedback on a rate is
//! derivative feedback on a heading, which is the damping the plant needs.
//! `tests/closed_loop.rs` is the regression, and it was watched failing before
//! this was written.

use oag_physics::{ShipControls, ShipState};

use crate::line::Line;

/// The controller's constants.
///
/// **These numbers are this project's own.** Both games author a per-class
/// controller on the disc - `Data\XML\AIControlStats.xml`, five attributes, read
/// in full at `docs/ghidra/functions/psp-pulse-usa/ai-stats.md` - and none of
/// its values appear here or anywhere else in the tree, per
/// [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md) and
/// the rule `docs/formats/handling-stats.md` states: a field name describes the
/// format, a tuning table is the content itself. The shipped values are read off
/// the player's own disc if anything ever wants them.
///
/// The defaults below were tuned against this engine's own physics, through
/// `tests/closed_loop.rs`, until a craft got round without weaving.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Lookahead distance at a standstill.
    pub look_min: f32,
    /// Extra lookahead per unit of forward speed.
    ///
    /// Roughly "how many seconds ahead the craft looks". Longer is calmer and
    /// cuts corners; shorter tracks the line harder and, past a point, is what
    /// makes the loop ring.
    pub look_speed: f32,
    /// Lookahead ceiling. Past this a craft stops seeing the corner it is in.
    pub look_max: f32,
    /// Gain from turn-rate error onto the steering input.
    pub rate_gain: f32,
    /// Ceiling on the turn rate the geometry may ask for, in radians per second.
    ///
    /// A craft thrown far off its line computes an enormous required curvature;
    /// without this it asks for a rate no hull can produce and holds full lock
    /// all the way through the recovery, which is its own kind of weave.
    pub max_turn_rate: f32,
    /// The lateral acceleration a craft is assumed to hold through a corner.
    ///
    /// Sets the speed target: on a corner of curvature `k` the target is
    /// `sqrt(lateral_accel / k)`, the standard cornering limit. Raising it makes
    /// a driver commit harder and, past what the hull can hold, into the wall.
    ///
    /// **Deliberately conservative**, because cornering here is steering and a
    /// speed target and nothing else. The original's own `<Controller>` has no
    /// airbrake term either - see the note on [`Tuning`] - which is consistent
    /// with that and is *not* evidence for it.
    pub lateral_accel: f32,
    /// How far ahead the speed target looks for the sharpest bend, as a multiple
    /// of the lookahead. A corner has to be seen before it is entered.
    pub brake_lookahead: f32,
    /// Fraction over target at which the airbrakes come on, rather than merely
    /// lifting off.
    pub brake_margin: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            look_min: 20.0,
            look_speed: 0.35,
            look_max: 90.0,
            rate_gain: 5.0,
            max_turn_rate: 1.2,
            lateral_accel: 55.0,
            brake_lookahead: 2.5,
            brake_margin: 0.05,
        }
    }
}

/// What a driver carries from one tick to the next.
///
/// One index, and therefore `Copy` and free of allocation - which is what lets
/// it sit on a `Ship` inside the world snapshot rather than beside it. A driver
/// whose state lived in the composition root would be invisible to a replay, and
/// two runs of the same race would not be the same race. See
/// [ADR-0003](../../../docs/architecture/adr/0003-no-ecs.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Driver {
    /// Where on the line this craft was last found.
    ///
    /// The seed for the next windowed search, so the search stays local and a
    /// craft cannot latch onto a section of track stacked above or below it.
    pub index: u32,
}

/// How much of the line either side of the last index a driver looks at.
///
/// Wide enough that a craft knocked sideways by a collision still finds itself,
/// narrow enough that the search cannot cross to a stacked section. Eight craft
/// pay it every tick, so it is also the cost that matters.
const SEARCH_WINDOW: usize = 48;

impl Driver {
    /// Chooses this tick's controls, and advances the driver's place on the line.
    ///
    /// Returns released controls when there is no line to follow: a track whose
    /// spline produced nothing leaves the opponents sitting on the grid, which is
    /// what this engine did before there was an AI at all.
    pub fn drive(&mut self, state: &ShipState, line: &Line, tuning: &Tuning) -> ShipControls {
        if line.is_empty() {
            return ShipControls::default();
        }

        let body = &state.body;
        let forward = body.forward();

        let index = line.nearest(body.position, self.index as usize, SEARCH_WINDOW);
        self.index = index as u32;

        // Forward speed rather than speed: a craft sliding sideways at 60 is not
        // approaching its corner at 60, and the lookahead is about how far away
        // the corner is in time.
        let speed = body.linear_velocity.dot(forward).max(0.0);
        let look = (tuning.look_min + tuning.look_speed * speed).min(tuning.look_max);

        let steer = self.steering(state, line, look, speed, tuning);
        let curvature = line.max_curvature(index, look * tuning.brake_lookahead, look * 0.5);
        let (thrust, brake) = throttle(speed, curvature, tuning);

        ShipControls {
            steer_x: steer,
            thrust,
            // **Both sides, or neither.** A single airbrake yaws the craft toward
            // the side it is held, so tying one to the steering command closes a
            // second feedback loop around a plant that already oscillates - which
            // is exactly what the first version of this crate did. Cornering on
            // the airbrakes is real and is deferred; see `docs/gameplay/ai.md`.
            airbrake_left: brake,
            airbrake_right: brake,
            ..ShipControls::default()
        }
    }

    /// The steering command, as a turn-rate error.
    ///
    /// Pure pursuit gives the curvature of the arc from the craft to a point on
    /// the line: `k = 2 * e / L^2`, where `e` is how far off the craft's nose the
    /// aim point sits and `L` is the distance to it. That curvature times the
    /// craft's speed is the yaw rate needed to get there, and the loop is closed
    /// on the difference between that and the rate the craft has.
    ///
    /// **`L` is the measured distance to the aim point, not the requested
    /// lookahead.** They differ on a curve, and `k` divides by its square, so
    /// using the request scales the whole command wrong exactly where the corner
    /// is.
    ///
    /// Cross-track error needs no term of its own: the aim point is on the line
    /// and the craft is not, so the vector between them already carries it. The
    /// first version added a second, separate cross-track term on top, which
    /// double-counted the same error.
    fn steering(
        &self,
        state: &ShipState,
        line: &Line,
        look: f32,
        speed: f32,
        tuning: &Tuning,
    ) -> f32 {
        let body = &state.body;
        let (_, aim, _) = line.ahead(self.index as usize, look);
        let to_aim = aim - body.position;
        let distance = to_aim.length();
        if distance <= f32::EPSILON {
            return 0.0;
        }

        // Positive is to the craft's right.
        let offset = to_aim.dot(body.right());
        let curvature = 2.0 * offset / (distance * distance);
        let wanted =
            (curvature * speed.max(1.0)).clamp(-tuning.max_turn_rate, tuning.max_turn_rate);

        // Positive yaw about the craft's own up axis turns it **left** - the
        // right-hand rule, with forward on `-Z`. Working in "rate of turning to
        // the right" keeps every sign here the same as the steering input's.
        // The craft's own up, never world up, or this is wrong the moment the
        // track rolls.
        let actual = -body.angular_velocity.dot(body.up());

        (tuning.rate_gain * (wanted - actual)).clamp(-1.0, 1.0)
    }
}

/// Thrust and braking from the speed the corner ahead allows.
///
/// `sqrt(lateral_accel / curvature)` is the ordinary cornering limit; a straight
/// has no limit and gets full throttle. Bang-bang rather than a proportional
/// controller because the craft's own engine ramp is the smoothing - see
/// `oag_physics::params::Engine`.
fn throttle(speed: f32, curvature: f32, tuning: &Tuning) -> (f32, f32) {
    if curvature <= f32::EPSILON {
        return (1.0, 0.0);
    }
    let target = (tuning.lateral_accel / curvature).sqrt();
    if speed <= target {
        (1.0, 0.0)
    } else if speed > target * (1.0 + tuning.brake_margin) {
        (0.0, 1.0)
    } else {
        (0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::{Quat, Vec3};
    use oag_physics::Body;

    fn straight() -> Line {
        Line::new(
            (0..64)
                .map(|step| Vec3::new(0.0, 0.0, -10.0 * step as f32))
                .collect(),
        )
    }

    /// A craft at the origin pointing down `-Z`, which is [`Body::forward`].
    fn craft(position: Vec3, speed: f32) -> ShipState {
        ShipState {
            body: Body {
                position,
                orientation: Quat::IDENTITY,
                linear_velocity: Vec3::new(0.0, 0.0, -speed),
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    #[test]
    fn no_line_means_no_input() {
        let mut driver = Driver::default();
        let controls = driver.drive(
            &craft(Vec3::ZERO, 0.0),
            &Line::default(),
            &Tuning::default(),
        );
        assert_eq!(controls, ShipControls::default());
    }

    #[test]
    fn a_craft_on_a_straight_holds_full_thrust_and_no_steering() {
        let mut driver = Driver::default();
        let controls = driver.drive(&craft(Vec3::ZERO, 50.0), &straight(), &Tuning::default());
        assert_eq!(controls.thrust, 1.0);
        assert!(controls.steer_x.abs() < 1e-3, "steer {}", controls.steer_x);
        assert_eq!(controls.airbrake_left, 0.0);
        assert_eq!(controls.airbrake_right, 0.0);
    }

    /// The sign is the whole thing: a craft to the *right* of its line must steer
    /// *left*, and getting it backwards drives the field into the outside wall on
    /// the first corner - which is exactly the trap the grid's lateral offset
    /// already sprung once. See `docs/ghidra/functions/psp-pulse-usa/grid.md`.
    #[test]
    fn a_craft_beside_the_line_steers_back_toward_it() {
        let tuning = Tuning::default();

        let mut driver = Driver::default();
        let right_of_line =
            driver.drive(&craft(Vec3::new(6.0, 0.0, 0.0), 40.0), &straight(), &tuning);
        assert!(
            right_of_line.steer_x < 0.0,
            "steer {} should be left of centre",
            right_of_line.steer_x
        );

        let mut driver = Driver::default();
        let left_of_line = driver.drive(
            &craft(Vec3::new(-6.0, 0.0, 0.0), 40.0),
            &straight(),
            &tuning,
        );
        assert!(
            left_of_line.steer_x > 0.0,
            "steer {} should be right of centre",
            left_of_line.steer_x
        );
    }

    /// The damping, and the reason the rewrite happened: a craft already turning
    /// the way the geometry wants must be asked for *less* lock, not the same.
    #[test]
    fn a_craft_already_turning_is_asked_for_less_lock() {
        let tuning = Tuning::default();
        let mut state = craft(Vec3::new(6.0, 0.0, 0.0), 40.0);

        let mut driver = Driver::default();
        let still = driver.drive(&state, &straight(), &tuning);

        // Yawing left, which is positive about the craft's own up axis, and is
        // the direction the controller wants to go.
        state.body.angular_velocity = Vec3::new(0.0, 0.3, 0.0);
        let mut driver = Driver::default();
        let turning = driver.drive(&state, &straight(), &tuning);
        assert!(
            turning.steer_x > still.steer_x,
            "already turning: {} should be less left lock than {}",
            turning.steer_x,
            still.steer_x
        );
    }

    /// Braking uses both sides. A single airbrake yaws the craft, which would
    /// close a second loop around the steering.
    #[test]
    fn braking_is_symmetric() {
        let tuning = Tuning::default();
        let line = Line::new(
            (0..128)
                .map(|step| {
                    let angle = std::f32::consts::TAU * step as f32 / 128.0;
                    Vec3::new(60.0 * angle.cos(), 0.0, 60.0 * angle.sin())
                })
                .collect(),
        );
        let mut driver = Driver::default();
        let controls = driver.drive(&craft(line.point(0), 200.0), &line, &tuning);
        assert_eq!(controls.airbrake_left, controls.airbrake_right);
        assert!(
            controls.airbrake_left > 0.0,
            "a 60-unit circle at 200 needs brakes"
        );
    }

    #[test]
    fn a_straight_has_no_speed_limit() {
        assert_eq!(throttle(500.0, 0.0, &Tuning::default()), (1.0, 0.0));
    }

    #[test]
    fn a_corner_taken_too_fast_brakes_and_taken_slowly_does_not() {
        let tuning = Tuning::default();
        let target = (tuning.lateral_accel / 0.01).sqrt();
        assert_eq!(throttle(target * 0.5, 0.01, &tuning), (1.0, 0.0));
        assert_eq!(throttle(target * 2.0, 0.01, &tuning), (0.0, 1.0));
    }

    /// The turn rate the geometry asks for is bounded, or a craft thrown clear of
    /// the track holds full lock through the whole recovery.
    #[test]
    fn the_requested_turn_rate_is_clamped() {
        let tuning = Tuning {
            rate_gain: 1.0,
            ..Tuning::default()
        };
        let mut driver = Driver::default();
        // Absurdly far off the line, at speed: the raw curvature is enormous.
        let controls = driver.drive(
            &craft(Vec3::new(500.0, 0.0, 0.0), 150.0),
            &straight(),
            &tuning,
        );
        assert!(
            controls.steer_x >= -1.0,
            "steer {} left the input range",
            controls.steer_x
        );
        assert!(controls.steer_x.abs() <= tuning.max_turn_rate * tuning.rate_gain + 1e-3);
    }

    /// The index is the search seed, so it has to survive the call.
    #[test]
    fn the_driver_remembers_where_it_was() {
        let mut driver = Driver::default();
        let line = straight();
        driver.drive(
            &craft(Vec3::new(0.0, 0.0, -200.0), 40.0),
            &line,
            &Tuning::default(),
        );
        assert_eq!(driver.index, 20);
    }
}
