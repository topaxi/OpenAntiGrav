//! Turning a craft's state and a line into the controls a craft is flown with.

use oag_core::math::Vec3;
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
/// The defaults below were tuned against this engine's own physics until eight
/// craft got round a lap, which is all they claim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Lookahead distance at a standstill.
    pub look_min: f32,
    /// Extra lookahead per unit of forward speed.
    pub look_speed: f32,
    /// Lookahead ceiling. Past this a craft stops seeing the corner it is in.
    pub look_max: f32,
    /// Gain on the combined heading and cross-track error.
    pub steer_gain: f32,
    /// Gain on the craft's own yaw rate, opposing it.
    ///
    /// The damping term is the *measured rotation*, not the change in error
    /// between ticks. A finite difference of the error would divide by `dt` and
    /// so amplify a one-tick wobble sixtyfold; yaw rate is the same quantity
    /// without the division, and it is a thing the body already knows.
    pub steer_damping: f32,
    /// How much of the steering command comes from being *off* the line, as
    /// opposed to *pointed away* from it.
    pub xtrack_gain: f32,
    /// Cross-track error past which the term saturates, so a craft that has been
    /// knocked far off the line steers hard rather than absurdly.
    pub xtrack_max: f32,
    /// Steering magnitude past which the inside airbrake comes on.
    ///
    /// **Ours, with nothing behind it.** The recovered `<Controller>` has no
    /// airbrake term at all, so either the original's opponents corner without
    /// them or they do it somewhere this project has not read. A craft flown on
    /// steering alone understeers out of the tighter corners at the higher speed
    /// classes, so this exists to get a lap finished.
    pub airbrake_at: f32,
    /// The lateral acceleration a craft is assumed to hold through a corner.
    ///
    /// Sets the speed target: on a corner of curvature `k` the target is
    /// `sqrt(lateral_accel / k)`, the standard cornering limit. Raising it makes
    /// a driver commit harder and, past what the hull can hold, into the wall.
    pub lateral_accel: f32,
    /// Fraction over target at which the airbrakes are used to slow down, rather
    /// than merely lifting off.
    pub brake_margin: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            look_min: 24.0,
            look_speed: 0.45,
            look_max: 120.0,
            steer_gain: 2.2,
            steer_damping: 0.35,
            xtrack_gain: 0.04,
            xtrack_max: 12.0,
            airbrake_at: 0.45,
            lateral_accel: 90.0,
            brake_margin: 0.08,
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
        let position = body.position;
        let forward = body.forward();
        let right = body.right();
        let up = body.up();

        let index = line.nearest(position, self.index as usize, SEARCH_WINDOW);
        self.index = index as u32;

        // Forward speed rather than speed: a craft sliding sideways at 60 is not
        // approaching its corner at 60, and the lookahead is about how far away
        // the corner is in time.
        let speed = body.linear_velocity.dot(forward).max(0.0);
        let look = (tuning.look_min + tuning.look_speed * speed).min(tuning.look_max);
        let (_, aim) = line.ahead(index, look);

        let steer = self.steering(position, aim, right, up, line.point(index), body, tuning);
        let (thrust, brake) = throttle(speed, line.curvature(index, look * 0.5), tuning);

        // The inside airbrake, on the side being steered toward. Held on top of
        // whatever braking the speed target asked for, so a craft that is both
        // too fast and turning hard gets both.
        let inside = ((steer.abs() - tuning.airbrake_at) / (1.0 - tuning.airbrake_at))
            .clamp(0.0, 1.0)
            .max(brake);
        let (left, right_brake) = if steer < 0.0 {
            (inside, brake)
        } else {
            (brake, inside)
        };

        ShipControls {
            steer_x: steer,
            thrust,
            airbrake_left: left,
            airbrake_right: right_brake,
            ..ShipControls::default()
        }
    }

    /// The steering command: where the craft is pointed, where it is, and how
    /// fast it is already turning.
    #[expect(
        clippy::too_many_arguments,
        reason = "a controller reads what it reads; bundling these into a struct \
                  would name the same seven values twice"
    )]
    fn steering(
        &self,
        position: Vec3,
        aim: Vec3,
        right: Vec3,
        up: Vec3,
        on_line: Vec3,
        body: &oag_physics::Body,
        tuning: &Tuning,
    ) -> f32 {
        // Pure pursuit: how far off the craft's nose the aim point sits, as a
        // fraction of the distance to it. Unit-free, so it does not change
        // meaning with the lookahead.
        let to_aim = (aim - position).normalize_or_zero();
        let heading = to_aim.dot(right);

        // Cross-track: which side of the line the craft is on, and by how much.
        // Clamped, so a craft thrown clear of the track by a collision asks for
        // full lock and not for more.
        let offset = (position - on_line).dot(right);
        let cross = (offset / tuning.xtrack_max).clamp(-1.0, 1.0) * tuning.xtrack_max;

        // Yaw about the craft's own up axis, which is the axis it steers about -
        // world up would be wrong the moment the track rolls or inverts.
        let yaw = body.angular_velocity.dot(up);

        let command =
            tuning.steer_gain * heading - tuning.xtrack_gain * cross - tuning.steer_damping * yaw;
        command.clamp(-1.0, 1.0)
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
    use oag_core::math::Quat;
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
        let mut driver = Driver::default();
        let tuning = Tuning::default();

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

    #[test]
    fn steering_hard_holds_the_inside_airbrake() {
        let mut driver = Driver::default();
        let tuning = Tuning::default();
        // Far enough off the line that the command saturates.
        let controls = driver.drive(
            &craft(Vec3::new(60.0, 0.0, 0.0), 40.0),
            &straight(),
            &tuning,
        );
        assert!(controls.steer_x < -tuning.airbrake_at);
        assert!(controls.airbrake_left > 0.0);
        assert_eq!(controls.airbrake_right, 0.0);
    }

    #[test]
    fn a_straight_has_no_speed_limit() {
        assert_eq!(throttle(500.0, 0.0, &Tuning::default()), (1.0, 0.0));
    }

    #[test]
    fn a_corner_taken_too_fast_brakes_and_taken_slowly_does_not() {
        let tuning = Tuning::default();
        // Curvature 0.01 is a 100-unit radius, so the target is sqrt(90/0.01).
        let target = (tuning.lateral_accel / 0.01).sqrt();
        assert_eq!(throttle(target * 0.5, 0.01, &tuning), (1.0, 0.0));
        assert_eq!(throttle(target * 2.0, 0.01, &tuning), (0.0, 1.0));
    }

    /// The damping term opposes the turn the craft is already making, or it is
    /// not damping.
    #[test]
    fn yaw_opposes_the_steering_command() {
        let mut driver = Driver::default();
        let tuning = Tuning::default();
        let mut state = craft(Vec3::new(6.0, 0.0, 0.0), 40.0);
        let undamped = driver.drive(&state, &straight(), &tuning);

        // Already yawing the way the controller wants to go.
        state.body.angular_velocity = Vec3::new(0.0, -1.0, 0.0);
        let mut driver = Driver::default();
        let damped = driver.drive(&state, &straight(), &tuning);
        assert!(
            damped.steer_x > undamped.steer_x,
            "damped {} should ask for less left lock than {}",
            damped.steer_x,
            undamped.steer_x
        );
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
