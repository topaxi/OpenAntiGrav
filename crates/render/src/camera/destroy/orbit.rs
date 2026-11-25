//! HD's Zone end camera: a close orbit round the craft (mode 2, `Camera_UpdateOrbitView`).
//!
//! The law is `docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`, rows Z2a to Z2f. For 2.5 s
//! the eye spins fast and tight at a narrowing field, then drifts slowly at the normal one until
//! the results screen.
//!
//! ```text
//! t += dt
//! yaw   += dt * s * 0.1 (wrapped to 2 pi),  s = 9t + 1 for t < 2.5, else 0.5
//! pitch += dt * 0.35                        while t < 2.5
//! focus += (craft - focus) * 0.6            per frame
//! dist  += (target_dist - dist) * 0.02      5.0 for t < 2.5, else 3.0
//! fov   += (target_fov - fov) * 0.01        20 for t < 2.5, else 65
//! eye    = focus + dir * dist,  dir = the craft's rotation . Ry(yaw) . Rx(-0.7 + pitch) . back
//! ```
//!
//! The object's own fields were read per frame on RPCS3 (`destroy-camera.md`, "Measured on RPCS3: the
//! Zone end orbit"): `dist` starts at 0.5 and goes `0.5 -> 5.0 -> 3.0` at 0.02 a frame, the field
//! `65 -> 20 -> 65` at 0.01, yaw and pitch as above, the spin positive (`+0x3ac` = 1). But the eye that
//! reaches the screen is not `focus + dir * dist`: it is 13.9 units from the craft at the cut and still 9.1,
//! 8.9 and 7.0 at 1.4, 1.6 and 2.5 s while `dist` is 3.1, 3.3 and 4.0. The eye's offset from the focus
//! eases toward `dir * dist` from the chase eye's offset; a per-frame rate of 0.01 reproduces the four
//! distances to within about 0.5 (10.0, 9.6, 9.2, 6.3 against 10.1, 9.1, 8.9, 7.0). The craft is drawn for
//! the first 2.5 s only, and the eye never comes inside its box in that time.
//!
//! # Chosen, not measured
//!
//! - The eye's ease (a fit to four distances, one circuit, one speed; the code that does it is unread).
//! - The spin direction is fixed positive (read positive on Vineta K); Omega's enter picks it from the
//!   track's direction at the craft, or at random.
//! - Which row of the composed matrix is the eye direction (Z2d, confidence 55): the image of the
//!   craft's rear axis is taken, which puts the eye behind and above at yaw 0 (the read eye is behind and
//!   above, about 20 degrees off the modelled direction).
//! - The camera's up is the craft's own, since the matrix is applied on its body rotation.

use super::{Basis, Destroy};
use oag_core::math::{Quat, Vec3};

/// How long the fast, tight phase lasts, in seconds (block `+0x14`).
pub const FAST_SECS: f32 = 2.5;
/// The yaw rate's climb per second of the fast phase, and its start: `9 t + 1`.
pub const YAW_CLIMB: f32 = 9.0;
/// The yaw rate after the fast phase, before the direction factor.
pub const YAW_SLOW: f32 = 0.5;
/// The direction factor on the yaw rate (`+0.1`, or `-0.1` with the direction byte clear).
pub const YAW_FACTOR: f32 = 0.1;
/// The pitch's turn rate in the fast phase, radians a second (block `+0x34`).
pub const PITCH_RATE: f32 = 0.35;
/// The pitch the eye is held down by (`Rx(-0.7)`).
pub const PITCH_OFFSET: f32 = -0.7;
/// How much of the way the focus closes on the craft per frame (block `+0x24`).
pub const FOCUS_RATE: f32 = 0.6;
/// The distance the orbit starts at (read live: 0.83 at the fourth frame of the object's `+0x238`).
pub const START_DIST: f32 = 0.5;
/// How much of the way the eye's offset closes on `dir * dist` per frame: a fit to four read eye
/// distances (see the module docs), not a read constant.
pub const EYE_RATE: f32 = 0.01;
/// The field the orbit starts at, in degrees (enter).
pub const START_FOV: f32 = 65.0;
/// The distance target in the fast phase, then after it (block `+0x2c`, `+0x3c`).
pub const DIST_TARGETS: (f32, f32) = (5.0, 3.0);
/// The field target in the fast phase, then after it (block `+0x30`, `+0x38`).
pub const FOV_TARGETS: (f32, f32) = (20.0, 65.0);
/// How much of the way the distance closes on its target per frame.
pub const DIST_RATE: f32 = 0.02;
/// How much of the way the field closes on its target per frame.
pub const FOV_RATE: f32 = 0.01;

/// The orbit, from the frame it takes over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    /// Seconds since the orbit began.
    pub t: f32,
    /// The spin about the craft's up, radians.
    pub yaw: f32,
    /// The turn about the craft's side, radians.
    pub pitch: f32,
    /// The orbit's radius: where the eye is heading, not where it is.
    pub dist: f32,
    /// The eye's offset from the focus, world space: it eases toward the orbit's own offset.
    pub offset: Vec3,
    /// The vertical field, degrees.
    pub fov: f32,
    /// The point looked at.
    pub focus: Vec3,
}

impl Orbit {
    /// The frame the camera takes over a craft at `craft`, from the chase camera's `eye`.
    #[must_use]
    pub fn start(craft: Vec3, eye: Vec3) -> Self {
        Self {
            t: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            dist: START_DIST,
            offset: eye - craft,
            fov: START_FOV,
            focus: craft,
        }
    }

    /// One frame of `dt` seconds, the craft now at `craft`.
    pub fn advance(&mut self, dt: f32, craft: Vec3, orientation: Quat) {
        self.t += dt;
        let fast = self.t < FAST_SECS;
        let rate = if fast {
            YAW_CLIMB * self.t + 1.0
        } else {
            YAW_SLOW
        };
        self.yaw = (self.yaw + dt * rate * YAW_FACTOR).rem_euclid(std::f32::consts::TAU);
        if fast {
            self.pitch += dt * PITCH_RATE;
        }
        self.focus += (craft - self.focus) * FOCUS_RATE;
        let (dist, fov) = if fast {
            (DIST_TARGETS.0, FOV_TARGETS.0)
        } else {
            (DIST_TARGETS.1, FOV_TARGETS.1)
        };
        self.dist += (dist - self.dist) * DIST_RATE;
        self.fov += (fov - self.fov) * FOV_RATE;
        let heading = self.orbit_offset(orientation);
        self.offset += (heading - self.offset) * EYE_RATE;
        // The ease is a chord across a turning orbit: it must not bring the eye inside the radius it is
        // heading for.
        let reach = self.offset.length();
        if reach > f32::EPSILON && reach < self.dist {
            self.offset *= self.dist / reach;
        }
    }

    /// Where the eye is heading, from the focus.
    fn orbit_offset(&self, orientation: Quat) -> Vec3 {
        let turn =
            Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(PITCH_OFFSET + self.pitch);
        orientation * (turn * Vec3::Z) * self.dist
    }

    /// The picture, with the craft turned to `orientation`.
    #[must_use]
    pub fn pose(&self, orientation: Quat) -> Destroy {
        Destroy {
            eye: self.focus + self.offset,
            focus: self.focus,
            focus_target: self.focus,
            fov: self.fov,
            focus_rate: FOCUS_RATE,
            fov_rate: FOV_RATE,
            substeps_per_second: None,
            basis: Basis::LookAt,
            up: orientation * Vec3::Y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    /// The chase eye RPCS3 read at the blast, 13.88 units from the craft.
    const CHASE: Vec3 = Vec3::new(0.0, 8.0, 11.34);

    fn start() -> Orbit {
        Orbit::start(Vec3::ZERO, CHASE)
    }

    fn run_to(seconds: f32) -> Orbit {
        let mut orbit = start();
        while orbit.t < seconds {
            orbit.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        }
        orbit
    }

    #[test]
    fn the_yaw_rate_climbs_from_a_tenth_to_2_35_then_drops_to_0_05() {
        let mut orbit = start();
        orbit.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        let first = orbit.yaw / FRAME;
        assert!((first - 0.1 * (9.0 * FRAME + 1.0)).abs() < 1e-3, "{first}");
        let before = run_to(FAST_SECS - 2.0 * FRAME);
        let mut last = before;
        last.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        let rate = (last.yaw - before.yaw) / FRAME;
        assert!((rate - 2.35).abs() < 0.03, "{rate}");
        let mut slow = run_to(FAST_SECS + FRAME);
        let yaw = slow.yaw;
        slow.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        let rate = (slow.yaw - yaw) / FRAME;
        assert!((rate - 0.05).abs() < 1e-3, "{rate}");
    }

    #[test]
    fn the_pitch_turns_only_in_the_fast_phase() {
        let fast = run_to(FAST_SECS);
        assert!(
            (fast.pitch - PITCH_RATE * FAST_SECS).abs() < 0.01,
            "{}",
            fast.pitch
        );
        let mut later = fast;
        for _ in 0..120 {
            later.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        }
        assert_eq!(later.pitch, fast.pitch);
    }

    /// `20 + 45 * 0.99^150` is 29.97: the field is near 30 degrees at 2.5 s, not at its
    /// target of 20, and then eases back toward 65.
    #[test]
    fn the_field_reaches_about_thirty_at_2_5_s_then_eases_back_to_65() {
        let at = run_to(FAST_SECS);
        assert!((at.fov - 30.0).abs() < 1.0, "{}", at.fov);
        let mut later = at;
        for _ in 0..300 {
            later.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        }
        assert!(later.fov > 55.0 && later.fov < 65.0, "{}", later.fov);
    }

    /// The object's own `+0x238`, read per frame: 0.5, then `5 - 4.5 x 0.98^n` toward 5, and after
    /// 2.5 s toward 3 (4.03 at 2.5 s on RPCS3's 70 frames, 3.07 at 7 s).
    #[test]
    fn the_radius_goes_from_half_a_unit_to_five_and_settles_toward_three() {
        assert_eq!(start().dist, START_DIST);
        let at = run_to(FAST_SECS);
        let expect = 5.0 - 4.5 * 0.98_f32.powi(150);
        assert!((at.dist - expect).abs() < 0.1, "{} vs {expect}", at.dist);
        let mut later = at;
        for _ in 0..600 {
            later.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        }
        assert!((later.dist - DIST_TARGETS.1).abs() < 0.02, "{}", later.dist);
    }

    /// The eye that reached the screen was 13.9 from the craft at the cut and 9.1, 8.9 and 7.0 at 1.4,
    /// 1.6 and 2.5 s (RPCS3): it starts at the chase eye's offset and eases to the orbit's, so it is
    /// far at first and never inside the radius it is heading for.
    #[test]
    fn the_eye_starts_at_the_chase_eye_and_eases_to_the_orbit() {
        let start = start();
        let pose = start.pose(Quat::IDENTITY);
        assert!((pose.eye.distance(pose.focus) - 13.88).abs() < 0.01);
        let mut orbit = start;
        let mut last = f32::MAX;
        for frame in 0..150 {
            orbit.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
            let d = orbit.pose(Quat::IDENTITY).eye.length();
            assert!(
                d > orbit.dist * 0.75,
                "frame {frame}: eye {d} inside the radius {}",
                orbit.dist
            );
            assert!(
                d < last + 0.5,
                "frame {frame}: eye went out, {d} after {last}"
            );
            last = d;
        }
    }

    #[test]
    fn the_eye_is_behind_and_above_the_craft_at_the_start_and_looks_at_it() {
        let pose =
            Orbit::start(Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 10.0, 14.0)).pose(Quat::IDENTITY);
        let off = pose.eye - pose.focus;
        assert!(
            (off.length() - 185.0_f32.sqrt()).abs() < 1e-4,
            "the eye starts as far as the chase eye"
        );
        assert!(off.z > 0.0 && off.y > 0.0, "{off:?}");
        let at_the_craft = pose.view().transform_point3(pose.focus);
        assert!(at_the_craft.z < 0.0);
    }

    #[test]
    fn the_orbit_turns_with_the_craft() {
        let orientation = Quat::from_rotation_z(1.2);
        let mut turned = start();
        let mut plain = start();
        for _ in 0..90 {
            turned.advance(FRAME, Vec3::ZERO, orientation);
            plain.advance(FRAME, Vec3::ZERO, Quat::IDENTITY);
        }
        let pose = turned.pose(orientation);
        assert!((pose.up - orientation * Vec3::Y).length() < 1e-5);
        assert!((pose.eye - plain.pose(Quat::IDENTITY).eye).length() > 0.1);
    }

    #[test]
    fn the_focus_closes_six_tenths_of_the_way_a_frame() {
        let mut orbit = start();
        orbit.advance(FRAME, Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY);
        assert!((orbit.focus.x - 6.0).abs() < 1e-5);
    }
}
