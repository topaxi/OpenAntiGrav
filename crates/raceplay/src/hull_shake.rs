//! The drawn hull's shake: a smooth random curve that turns and moves the hull's display matrix
//! a little every tick, faded out with speed. Wipeout HD's `HullShake_Update` and
//! `HullShake_Apply` (`docs/ghidra/functions/ps3-hdfury-eu/hull-shake.md`), with the law as title
//! data ([`oag_title::hull_shake::HullShake`]).
//!
//! View-only and unhashed: it draws from its own seeded stream per craft, never `world.rng`, and
//! it moves the drawn matrix only, never the body. The original draws from the libc `rand` every
//! other system shares, so its sequence is not reproducible here; the law (amplitude, rate and
//! fade) is.
//!
//! Stepped on the fixed tick, not per rendered frame. Two of the original's terms are per call
//! rather than per second (the fade input's `0.99` filter and the offset, the curve's change over
//! one frame), so HD's 60 Hz, which is this engine's tick, is the rate they keep their size at.
//!
//! **Chosen, not read:** the constructor's seeding of the four points (here: four fresh draws and
//! the parameter at `0`), and the first tick's previous value (here: the current one, so the hull
//! does not jump on the first frame; the original's ring of 60 is read only one slot back).
//! Every craft shakes, not only the player's: the calls sit inside the per-ship update and no
//! player test above them was read.

use oag_core::Rng;
use oag_core::math::{Mat4, Quat, Vec3};
use oag_title::hull_shake::HullShake as Law;

use super::Race;

/// Seed for the shake's draws, distinct from every other view stream so it shifts none of them.
pub(super) const HULL_SHAKE_SEED: u64 = 0x5_9a_2b_05;

/// One craft's shake.
#[derive(Debug, Clone)]
pub(super) struct Shake {
    /// The curve's four control points, `ship+0x6b00+0x40..+0x70`, in the original's axes.
    points: [Vec3; 4],
    /// The curve's parameter, `+0x80`, in `[0, 1)`.
    t: f32,
    /// The curve's value on the previous tick, the ring slot the original reads back.
    previous: Option<Vec3>,
    /// The filtered fade input, `+0x20`.
    fade_input: f32,
    /// This tick's matrix in the hull's local frame, between the drawn lean and the model yaw.
    local: Mat4,
    rng: Rng,
}

impl Shake {
    pub(super) fn new(law: &Law, slot: usize) -> Self {
        let mut rng = Rng::new(HULL_SHAKE_SEED + slot as u64);
        let points = std::array::from_fn(|_| draw_point(law, &mut rng));
        Self {
            points,
            t: 0.0,
            previous: None,
            fade_input: 0.0,
            local: Mat4::IDENTITY,
            rng,
        }
    }

    /// One tick: `HullShake_Update`, then the matrix `HullShake_Apply` multiplies in.
    pub(super) fn advance(&mut self, law: &Law, dt: f32, speed: f32) {
        // Evaluated at the parameter before the step, as the original does.
        let p = spline(&self.points, self.t);
        self.t += law.rate * dt;
        while self.t >= 1.0 {
            self.t -= 1.0;
            self.points.rotate_left(1);
            self.points[3] = draw_point(law, &mut self.rng);
        }
        let q = self.previous.unwrap_or(p);
        self.previous = Some(p);

        self.fade_input = law.fade_filter * self.fade_input
            + (1.0 - law.fade_filter) * law.fade_per_speed * speed;
        let fade = ((law.fade_zero - self.fade_input) * law.fade_slope).clamp(0.0, 1.0);

        // The original's axes are the hull's left, up and nose; ours are right, up and the
        // tail (`drawable::display_orientation`), a half turn about up. So a turn about its
        // left or its nose is the opposite turn about our `+X` or `+Z`, one about up the same,
        // and an offset's `x` and `z` change sign.
        let pitch = law.pitch_per_y_minus_z * (q.y - q.z) * fade;
        let yaw = law.yaw_per_x * q.x * fade;
        let roll = law.roll_per_x * q.x * fade;
        let rotation = Quat::from_rotation_x(-pitch)
            * Quat::from_rotation_z(-roll)
            * Quat::from_rotation_y(yaw);
        let offset = (p - q) * (law.offset_gain * fade);
        let offset = Vec3::new(-offset.x, offset.y, -offset.z) * oag_fx::exhaust::CRAFT_ROW_SCALE;
        // `HullShake_Apply` moves the translation along the node's rows after the turn.
        self.local = Mat4::from_rotation_translation(rotation, rotation * offset);
    }

    /// The matrix to compose between the drawn orientation and the model's own yaw and scale.
    pub(super) fn local(&self) -> Mat4 {
        self.local
    }
}

/// A new control point: `scale * (r - 0.5)` per axis, `r` in `[0, 1)`. HD's `rand` returns 30
/// bits and the code multiplies by `2^-30`, so its points are centred the same way.
fn draw_point(law: &Law, rng: &mut Rng) -> Vec3 {
    let [x, y, z] = law.point_scale;
    Vec3::new(
        x * (rng.next_f32() - 0.5),
        y * (rng.next_f32() - 0.5),
        z * (rng.next_f32() - 0.5),
    )
}

/// The uniform cubic B-spline through `points` at `t`, the original's four basis weights.
fn spline(points: &[Vec3; 4], t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let u = 1.0 - t;
    let b0 = u * u * u / 6.0;
    let b1 = (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0;
    let b2 = (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0;
    let b3 = t3 / 6.0;
    points[0] * b0 + points[1] * b1 + points[2] * b2 + points[3] * b3
}

impl Race {
    /// Turns the drawn hull shake on or off; off is what a still comparison against an original
    /// with its shake patched out wants. On by default where the title has one.
    pub fn set_hull_shake(&mut self, on: bool) {
        self.view.hull_shake_on = on;
        if !on {
            for shake in self.view.hull_shake.iter_mut().flatten() {
                shake.local = Mat4::IDENTITY;
            }
        }
    }

    /// Every active craft's shake, one tick.
    pub(super) fn advance_hull_shakes(&mut self) {
        let Some(law) = self.view.hull_shake_law else {
            return;
        };
        if !self.view.hull_shake_on {
            return;
        }
        for slot in 0..self.sim.world.ship_count as usize {
            let ship = &self.sim.world.ships[slot];
            if !ship.active {
                continue;
            }
            let speed = ship.physics.body.linear_velocity.length();
            let dt = self.sim.dt;
            self.view.hull_shake[slot]
                .get_or_insert_with(|| Shake::new(&law, slot))
                .advance(&law, dt, speed);
        }
    }

    /// `slot`'s shake matrix this tick, identity where there is none.
    pub(super) fn hull_shake_of(&self, slot: usize) -> Mat4 {
        self.view.hull_shake[slot]
            .as_ref()
            .map_or(Mat4::IDENTITY, Shake::local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn angle_of(m: Mat4) -> f32 {
        let (_, rotation, _) = m.to_scale_rotation_translation();
        rotation.to_axis_angle().1
    }

    /// The basis weights sum to one, so a curve through four equal points sits on them.
    #[test]
    fn the_spline_through_equal_points_is_that_point() {
        let point = Vec3::new(0.3, -0.2, 0.1);
        for t in [0.0, 0.25, 0.5, 0.99] {
            assert!((spline(&[point; 4], t) - point).length() < 1e-6);
        }
    }

    /// At rest the shake runs at full size: over ten seconds HD's law turns the hull by up to a
    /// few degrees (the control points bound roll at `0.05` rad, yaw `0.02`, pitch `0.045`) and
    /// never past them, and it does turn it.
    #[test]
    fn at_rest_the_hull_turns_by_degrees_and_no_more() {
        let law = Law::HD;
        let mut shake = Shake::new(&law, 0);
        let mut most = 0.0_f32;
        for _ in 0..600 {
            shake.advance(&law, 1.0 / 60.0, 0.0);
            most = most.max(angle_of(shake.local()));
        }
        assert!(
            most > 0.2_f32.to_radians(),
            "the hull barely turned: {most}"
        );
        assert!(
            most < 0.075,
            "the hull turned past the control points' bound: {most}"
        );
    }

    /// At pace the fade input passes `fade_zero` and the shake is gone: HD's `1.5 * speed`
    /// filtered `0.99` a tick reaches 80 at a speed of 60 within a few seconds.
    #[test]
    fn at_speed_the_shake_fades_to_nothing() {
        let law = Law::HD;
        let mut shake = Shake::new(&law, 0);
        for _ in 0..1200 {
            shake.advance(&law, 1.0 / 60.0, 100.0);
        }
        assert_eq!(shake.local(), Mat4::IDENTITY);
    }
}
