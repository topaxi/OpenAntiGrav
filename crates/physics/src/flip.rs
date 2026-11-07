//! The Eliminator flip: a half turn about the craft's own up axis on a button press.
//!
//! The craft keeps its velocity, so its engine then pushes against its travel. Only the
//! heading is driven and only while the turn runs; steering is dropped for those ticks and
//! pitch is not. `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`.

use oag_core::math::{Quat, Vec3, sin_cos};

use crate::ship::Body;

/// Half a turn.
const HALF_TURN: f32 = std::f32::consts::PI;

/// A race's flip, resolved from its title at load.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlipLaw {
    /// How long the half turn takes, seconds.
    pub seconds: f32,
}

/// One craft's flip in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flip {
    /// A turn is running.
    pub active: bool,
    /// Radians turned so far, signed.
    pub angle: f32,
    /// Radians per second, signed.
    pub rate: f32,
    /// The forward axis at the press; the turn is measured from it.
    pub forward: Vec3,
}

impl Flip {
    /// No flip running.
    pub const IDLE: Self = Self {
        active: false,
        angle: 0.0,
        rate: 0.0,
        forward: Vec3::ZERO,
    };

    /// Starts a turn from `body`'s heading unless one is running. A craft leaning left turns
    /// left; a centred craft turns right (`0x000cec84`, `craft+0x6a58 >= 0`).
    pub fn start(&mut self, law: FlipLaw, body: &Body, lean: f32) {
        if self.active {
            return;
        }
        let speed = HALF_TURN / law.seconds;
        *self = Self {
            active: true,
            angle: 0.0,
            rate: if lean >= 0.0 { -speed } else { speed },
            forward: body.forward(),
        };
    }

    /// Advances a running turn by `dt` and writes the heading onto `body`.
    pub fn advance(&mut self, body: &mut Body, dt: f32) {
        if !self.active {
            return;
        }
        self.angle += self.rate * dt;
        if self.angle.abs() >= HALF_TURN {
            self.active = false;
            self.angle = self.angle.clamp(-HALF_TURN, HALF_TURN);
        }
        let up = body.up();
        let (sin, cos) = sin_cos(self.angle);
        let along = up * self.forward.dot(up);
        let across = self.forward - along;
        let heading = (across * cos + up.cross(across) * sin).normalize_or_zero();
        let forward = body.forward();
        if heading != Vec3::ZERO {
            body.orientation = (Quat::from_rotation_arc(forward, heading) * body.orientation).normalize();
        }
    }
}

impl Default for Flip {
    fn default() -> Self {
        Self::IDLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAW: FlipLaw = FlipLaw { seconds: 0.2 };
    const DT: f32 = 1.0 / 60.0;

    fn body() -> Body {
        Body {
            linear_velocity: Vec3::new(0.0, 0.0, -100.0),
            ..Body::default()
        }
    }

    #[test]
    fn a_flip_is_a_half_turn_in_twelve_ticks_and_leaves_the_velocity_alone() {
        let mut body = body();
        let velocity = body.linear_velocity;
        let mut flip = Flip::IDLE;
        flip.start(LAW, &body, 0.0);
        let mut ticks = 0;
        while flip.active {
            flip.advance(&mut body, DT);
            ticks += 1;
        }
        assert_eq!(ticks, 12);
        assert!(body.forward().dot(Vec3::NEG_Z) < -0.999, "{}", body.forward());
        assert!(body.up().dot(Vec3::Y) > 0.999);
        assert_eq!(body.linear_velocity, velocity);
    }

    #[test]
    fn a_centred_craft_turns_right_and_a_leaning_one_turns_left() {
        for (lean, sign) in [(0.0, -1.0), (0.5, -1.0), (-0.5, 1.0)] {
            let mut body = body();
            let mut flip = Flip::IDLE;
            flip.start(LAW, &body, lean);
            flip.advance(&mut body, DT);
            let turned = Vec3::NEG_Z.cross(body.forward()).dot(body.up());
            assert_eq!(turned.signum(), sign, "lean {lean}");
        }
    }

    #[test]
    fn a_second_press_during_a_turn_changes_nothing() {
        let mut body = body();
        let mut flip = Flip::IDLE;
        flip.start(LAW, &body, 0.0);
        flip.advance(&mut body, DT);
        let running = flip;
        flip.start(LAW, &body, -1.0);
        assert_eq!(flip, running);
    }
}
