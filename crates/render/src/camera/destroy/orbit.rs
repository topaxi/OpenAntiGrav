//! The Zone end camera of HD, 2048 and Omega: a close orbit round the craft (mode 2,
//! `Camera_UpdateOrbitView`), the same law on all three (`destroy-camera.md`, "The orbit on 2048 and
//! Omega").
//!
//! The law is `docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`, rows Z2a to Z2f. For 2.5 s
//! the eye spins fast and tight at a narrowing field, then drifts slowly at the normal one until
//! the results screen.
//!
//! ```text
//! t += dt
//! yaw   += dt * s * 0.1 (wrapped to 2 pi),  s = 9t + 1 for t < 2.5, else 0.5
//! focus += (craft - focus) * 0.6            per frame
//! dist  += (dist_target - dist) * 0.02      the target the previous frame wrote, 4.0 on the first
//! eye    = focus + dir * (5.0 + dist),  dir = the craft's rotation . Ry(yaw) . Rx(-0.7 + pitch) . back
//! dist_target = 5.0 for t < 2.5, else 3.0   for the next frame
//! fov   += (target_fov - fov) * 0.01        this frame's target: 20 for t < 2.5, else 65
//! pitch += dt * 0.35                        while t < 2.5, after the eye is built
//! ```
//!
//! The order of the distance ease and the pitch step is the originals'
//! (`docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`, "The orbit on 2048 and Omega").
//!
//! The eye is not smoothed: the render camera's eye read live is `5 + dist` from the focus on every
//! frame of a 7 s run (to 0.01), where `5.0` is the camera's constant `cam+0x3b4` (`EYE_STANDOFF`), and
//! its direction matched the modelled one to 0.01 on the frame read. The eye is therefore 5.5 from the
//! craft at the cut and grows with `dist`, which a pushbuffer scan had misread as a long eye coming in
//! (it reads the previous frame's eye against the craft's present position).

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
/// The eye's standoff from the focus beyond `dist`, along the same direction (`cam+0x3b4`, block
/// `+0x10` of the tuning table, never written after the constructor).
pub const EYE_STANDOFF: f32 = 5.0;
/// The distance target the enter writes, eased toward on the first frame only (`+0x23c`, row Z2a).
pub const START_DIST_TARGET: f32 = 4.0;
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
    /// The turn about the craft's side, radians, as this frame's eye is built with it.
    pub pitch: f32,
    /// The pitch step this frame earned, taken after its eye: applied on the next frame.
    pub pitch_step: f32,
    /// The orbit's radius, added to `EYE_STANDOFF` for the eye's distance from the focus.
    pub dist: f32,
    /// What `dist` eases toward on the next frame.
    pub dist_target: f32,
    /// The vertical field, degrees.
    pub fov: f32,
    /// The point looked at.
    pub focus: Vec3,
}

impl Orbit {
    /// The frame the camera takes over a craft at `craft`.
    #[must_use]
    pub fn start(craft: Vec3) -> Self {
        Self {
            t: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            pitch_step: 0.0,
            dist: START_DIST,
            dist_target: START_DIST_TARGET,
            fov: START_FOV,
            focus: craft,
        }
    }

    /// One frame of `dt` seconds, the craft now at `craft`.
    pub fn advance(&mut self, dt: f32, craft: Vec3) {
        self.t += dt;
        let fast = self.t < FAST_SECS;
        let rate = if fast {
            YAW_CLIMB * self.t + 1.0
        } else {
            YAW_SLOW
        };
        self.yaw = (self.yaw + dt * rate * YAW_FACTOR).rem_euclid(std::f32::consts::TAU);
        self.pitch += self.pitch_step;
        self.pitch_step = if fast { dt * PITCH_RATE } else { 0.0 };
        self.focus += (craft - self.focus) * FOCUS_RATE;
        let (dist, fov) = if fast {
            (DIST_TARGETS.0, FOV_TARGETS.0)
        } else {
            (DIST_TARGETS.1, FOV_TARGETS.1)
        };
        self.dist += (self.dist_target - self.dist) * DIST_RATE;
        self.dist_target = dist;
        self.fov += (fov - self.fov) * FOV_RATE;
    }

    /// The eye's offset from the focus.
    fn orbit_offset(&self, orientation: Quat) -> Vec3 {
        let turn =
            Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(PITCH_OFFSET + self.pitch);
        orientation * (turn * Vec3::Z) * (EYE_STANDOFF + self.dist)
    }

    /// The picture, with the craft turned to `orientation`.
    #[must_use]
    pub fn pose(&self, orientation: Quat) -> Destroy {
        Destroy {
            eye: self.focus + self.orbit_offset(orientation),
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

    fn start() -> Orbit {
        Orbit::start(Vec3::ZERO)
    }

    fn run_to(seconds: f32) -> Orbit {
        let mut orbit = start();
        while orbit.t < seconds {
            orbit.advance(FRAME, Vec3::ZERO);
        }
        orbit
    }

    #[test]
    fn the_yaw_rate_climbs_from_a_tenth_to_2_35_then_drops_to_0_05() {
        let mut orbit = start();
        orbit.advance(FRAME, Vec3::ZERO);
        let first = orbit.yaw / FRAME;
        assert!((first - 0.1 * (9.0 * FRAME + 1.0)).abs() < 1e-3, "{first}");
        let before = run_to(FAST_SECS - 2.0 * FRAME);
        let mut last = before;
        last.advance(FRAME, Vec3::ZERO);
        let rate = (last.yaw - before.yaw) / FRAME;
        assert!((rate - 2.35).abs() < 0.03, "{rate}");
        let mut slow = run_to(FAST_SECS + FRAME);
        let yaw = slow.yaw;
        slow.advance(FRAME, Vec3::ZERO);
        let rate = (slow.yaw - yaw) / FRAME;
        assert!((rate - 0.05).abs() < 1e-3, "{rate}");
    }

    #[test]
    fn the_pitch_turns_only_in_the_fast_phase() {
        let mut fast = run_to(FAST_SECS);
        // The last fast frame's step lands on the frame after it.
        fast.advance(FRAME, Vec3::ZERO);
        assert!(
            (fast.pitch - PITCH_RATE * FAST_SECS).abs() < 0.01,
            "{}",
            fast.pitch
        );
        let mut later = fast;
        for _ in 0..120 {
            later.advance(FRAME, Vec3::ZERO);
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
            later.advance(FRAME, Vec3::ZERO);
        }
        assert!(later.fov > 55.0 && later.fov < 65.0, "{}", later.fov);
    }

    /// The first frame eases from 0.5 toward the enter's 4.0, the next ones toward 5.0: 0.57 after one
    /// frame, and 0.8305 at the fourth, where HD's `+0x238` read 0.83 (an ease toward 5.0 from the
    /// first frame gives 0.59 and 0.849).
    #[test]
    fn the_radius_eases_toward_the_target_the_previous_frame_wrote() {
        let mut orbit = start();
        orbit.advance(FRAME, Vec3::ZERO);
        assert!((orbit.dist - 0.57).abs() < 1e-5, "{}", orbit.dist);
        for _ in 0..3 {
            orbit.advance(FRAME, Vec3::ZERO);
        }
        assert!((orbit.dist - 0.8305).abs() < 1e-3, "{}", orbit.dist);
    }

    /// Frame `n`'s eye is built with the pitch of `n - 1` steps: the step is taken after the eye.
    #[test]
    fn the_pitch_steps_after_the_eye_is_built() {
        let mut orbit = start();
        for frame in 1..=3 {
            orbit.advance(FRAME, Vec3::ZERO);
            let pitch = PITCH_RATE * FRAME * (frame - 1) as f32;
            let turn =
                Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(PITCH_OFFSET + pitch);
            let want = turn * Vec3::Z * (EYE_STANDOFF + orbit.dist);
            let eye = orbit.pose(Quat::IDENTITY).eye;
            assert!(
                (eye - want).length() < 1e-4,
                "frame {frame}: {eye:?}, want {want:?}"
            );
        }
    }

    /// The object's own `+0x238`, read per frame: 0.5, then close to `5 - 4.5 x 0.98^n` toward 5, and after
    /// 2.5 s toward 3 (4.03 at 2.5 s on RPCS3's 70 frames, 3.07 at 7 s).
    #[test]
    fn the_radius_goes_from_half_a_unit_to_five_and_settles_toward_three() {
        assert_eq!(start().dist, START_DIST);
        let at = run_to(FAST_SECS);
        let expect = 5.0 - 4.5 * 0.98_f32.powi(150);
        assert!((at.dist - expect).abs() < 0.1, "{} vs {expect}", at.dist);
        let mut later = at;
        for _ in 0..600 {
            later.advance(FRAME, Vec3::ZERO);
        }
        assert!((later.dist - DIST_TARGETS.1).abs() < 0.02, "{}", later.dist);
    }

    /// The render camera's eye read on RPCS3 is `5 + dist` from the focus on every frame (0.5 at the cut,
    /// 4.03 at 2.5 s on RPCS3's 70 frames, 3.06 at 7 s): 5.5, 9.0, 8.06, and a frame count's worth of
    /// `0.98^n` at ours. A smoothed eye (the 0.01 ease from the chase eye
    /// that stood here) is 13.9 out at the cut.
    #[test]
    fn the_eye_is_five_beyond_the_radius_from_the_focus_on_every_frame() {
        let mut orbit = start();
        assert!((orbit.pose(Quat::IDENTITY).eye.length() - 5.5).abs() < 1e-4);
        for frame in 0..420 {
            orbit.advance(FRAME, Vec3::ZERO);
            let pose = orbit.pose(Quat::IDENTITY);
            let want = EYE_STANDOFF + orbit.dist;
            assert!(
                (pose.eye.distance(pose.focus) - want).abs() < 1e-3,
                "frame {frame}: eye {} from the focus, want {want}",
                pose.eye.distance(pose.focus)
            );
        }
        let at_2_5 = run_to(FAST_SECS).pose(Quat::IDENTITY).eye.length();
        let expect = EYE_STANDOFF + 5.0 - 4.5 * 0.98_f32.powi(150);
        assert!((at_2_5 - expect).abs() < 0.05, "{at_2_5} vs {expect}");
    }

    #[test]
    fn the_eye_is_behind_and_above_the_craft_at_the_start_and_looks_at_it() {
        let pose = Orbit::start(Vec3::new(1.0, 2.0, 3.0)).pose(Quat::IDENTITY);
        let off = pose.eye - pose.focus;
        assert!((off.length() - EYE_STANDOFF - START_DIST).abs() < 1e-4);
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
            turned.advance(FRAME, Vec3::ZERO);
            plain.advance(FRAME, Vec3::ZERO);
        }
        let pose = turned.pose(orientation);
        assert!((pose.up - orientation * Vec3::Y).length() < 1e-5);
        assert!((pose.eye - plain.pose(Quat::IDENTITY).eye).length() > 0.1);
    }

    #[test]
    fn the_focus_closes_six_tenths_of_the_way_a_frame() {
        let mut orbit = start();
        orbit.advance(FRAME, Vec3::new(10.0, 0.0, 0.0));
        assert!((orbit.focus.x - 6.0).abs() < 1e-5);
    }
}
