//! The destroy camera: what the player sees once their own craft has blown up.
//!
//! The original does not keep the chase camera. `Ship_SetState`'s case 4 hands
//! the second camera object (`DAT_08b32c64`, the one the photo mode drives) the
//! wrecked craft as its subject and puts it in mode 5. That camera is **not
//! carried by the craft at all**: it sits at one of the circuit's own authored
//! `Camera` nodes (`track.vex`, class `0xf7`, ten on Talon's Junction) and
//! looks at the wreck from there, zooming until the craft fills a fixed
//! 35 units of the picture. On Talon's Junction's start line that is a camera
//! 488 units away at a 4.1 degree field of view.
//!
//! Read in `FUN_0887fedc` (the pick), `FUN_08880984` (the zoom) and the shared
//! mode 5, 6 and 7 arm of `FUN_08880c04` (the look-at), and measured on a
//! running PPSSPP (2026-10-01, Talon's Junction, a Venom player craft put into
//! state 4 with `scripts/psp-wreck-capture.py --camera`): the eye, the focus,
//! the field of view per frame and the view matrix at 4.1 degrees all
//! reproduce, to the digits the tests below quote.
//!
//! # The law, per frame
//!
//! ```text
//! pick:    the station whose AIM POINT is nearest the craft (ties: the first)
//! fov0:    2*atan(30 / |eye - craft|) in degrees + rand(-10, 20), at least 3
//! each frame, after the craft has moved:
//!   fov_target = 2*atan(17.5 / |eye - craft|) in degrees
//!   focus_target = craft position        (frozen once the craft is in state 6)
//!   focus += (focus_target - focus) * focus_rate
//!   fov   += (fov_target - fov) * 0.06
//!   z = normalize(eye - focus) with z.y scaled by max(1 - fov * 0.008, 0.4)
//!   x = normalize(up x z), y = z x x       eye = the station's eye
//! ```
//!
//! # Chosen, not measured
//!
//! - The starting field's `rand(-10, 20)` is the original's unseeded
//!   `Psys_RandFloatRange`; the caller draws it from a stream of its own, so a
//!   frame series matches the original only once the field has eased to its
//!   target (about 60 frames).
//! - `focus_rate` is `0.4` on Venom, read live. Flash `0.5` and Rapier and
//!   Phantom `0.6` are read in the constructor `FUN_0887f9bc`, and never seen
//!   running.
//! - When the camera lets go of the wreck, and the original's habit of
//!   moving on to another craft after ten seconds, are not read; the caller
//!   decides.
//!
//! # HD, 2048 and Omega
//!
//! HD's mode 8 and 2048's and Omega's mode 9 are this view kept to the instruction
//! (`docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`, rows D1 to D7), with three
//! differences carried by [`Ease`]: both rates are drawn with a random factor on the cut, and a
//! frame is `trunc(dt * k)` substeps of `rate / (dt * k)` each, so it moves `1 - (1 - rate / n)^n`
//! of the way rather than `rate`. The submodules hold the other two views: [`orbit`], HD's Zone
//! end (mode 2), and [`Destroy::hold`], 2048's and Omega's hold mode 8.
//!
//! # Chosen, not measured (HD, 2048, Omega)
//!
//! - No look-at wobble: its noise source (`Noise_SampleFractal1D`, `0x0027e908`) is unread, so
//!   the look-at is the craft itself (row D5, open item 1).
//! - [`HOLD_LEAD`] is `7.5 x` the craft's velocity as read, in the sim's own velocity unit; the
//!   hold view's track-section tie (`0x8121476a`) is unread and left out (row W8).
//!
//! Pure maths over plain vectors, like its siblings.

pub mod orbit;

use oag_core::math::{Mat4, Vec3};

/// One authored camera: where it sits and the point it was aimed at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    /// The node's world position - the eye, taken as it stands.
    pub eye: Vec3,
    /// The world-space aim point from the node's payload (`+0x10`); what the
    /// pick measures against.
    pub aim: Vec3,
}

/// The extent the zoom frames, `camera+0x268` after `Camera_SetMode(.., 5)`.
pub const FRAME_SIZE: f32 = 35.0;

/// The extent the starting field frames: `camera+0x268` as the constructor
/// leaves it (`60.0`) when the pick runs, before mode 5 narrows it.
pub const START_FRAME_SIZE: f32 = 60.0;

/// How fast the field closes on its target, per frame (`camera+0x260`).
pub const FOV_RATE: f32 = 0.06;

/// The least the starting field may be, in degrees.
pub const START_FOV_FLOOR: f32 = 3.0;

/// The spread added to the starting field, in degrees: `rand(-10, 20)`.
pub const START_FOV_SPREAD: (f32, f32) = (-10.0, 20.0);

/// How much of the vertical component of the look direction each degree of
/// field removes (`0.008`), and the least it keeps (`0.4`).
pub const FLATTEN_PER_DEGREE: f32 = 0.008;
/// See [`FLATTEN_PER_DEGREE`].
pub const FLATTEN_FLOOR: f32 = 0.4;

/// How fast the focus closes on the craft, per frame, by speed class (Venom,
/// Flash, Rapier, Phantom); `camera+0x264`, set in `FUN_0887f9bc`.
pub const FOCUS_RATE_BY_CLASS: [f32; 4] = [0.4, 0.5, 0.6, 0.6];

/// How far ahead of the craft along its velocity the hold view looks, in velocities
/// (`Camera_UpdateHoldView`, `0x81217616`, row W8).
pub const HOLD_LEAD: f32 = 7.5;

/// The hold view's field, in degrees (row W8).
pub const HOLD_FOV: f32 = 65.0;

/// How the camera closes on its focus and field: one step per frame, or substeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ease {
    /// How fast the focus closes on the craft (`camera+0x270` on HD).
    pub focus_rate: f32,
    /// How fast the field closes on its target (`camera+0x234` on HD).
    pub fov_rate: f32,
    /// `Some(k)`: a frame is `trunc(dt * k)` substeps of `rate / (dt * k)` (row D6). `None`: one
    /// step of `rate` per frame (Pulse).
    pub substeps_per_second: Option<f32>,
}

impl Ease {
    /// Pulse's: one step of `rate` per frame, the field at [`FOV_RATE`].
    #[must_use]
    pub const fn per_frame(focus_rate: f32) -> Self {
        Self {
            focus_rate,
            fov_rate: FOV_RATE,
            substeps_per_second: None,
        }
    }
}

/// How [`Destroy::to_world`] turns eye and focus into a basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// The station view's: the vertical part of the look direction is scaled down at a wide
    /// field ([`FLATTEN_PER_DEGREE`], [`FLATTEN_FLOOR`]).
    Flattened,
    /// A plain look-at: the hold and orbit views.
    LookAt,
}

/// The index of the station whose aim point is nearest `subject`, the first of
/// equals; `None` for no stations.
#[must_use]
pub fn nearest_station(stations: &[Station], subject: Vec3) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, station) in stations.iter().enumerate() {
        let distance = station.aim.distance_squared(subject);
        if best.is_none_or(|(_, least)| distance < least) {
            best = Some((index, distance));
        }
    }
    best.map(|(index, _)| index)
}

/// The field, in degrees, at which `frame` units across fill the view from
/// `distance` away: `FUN_08880984`.
#[must_use]
pub fn framing_fov_degrees(frame: f32, distance: f32) -> f32 {
    2.0 * ((frame * 0.5) / distance).atan().to_degrees()
}

/// How many substeps a frame of `dt` seconds makes at `per_second`: `trunc(dt * k)` (row D6).
/// At 60 Hz that is 100 for HD's `6000` and, in `f32`, for 2048's `5999.9995` too.
#[must_use]
pub fn substeps(dt: f32, per_second: f32) -> u32 {
    (dt * per_second) as u32
}

/// The destroy camera, from the frame it takes over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Destroy {
    /// Where it sits, fixed.
    pub eye: Vec3,
    /// The point it looks at: the craft's position, eased.
    pub focus: Vec3,
    /// Where the focus is heading.
    pub focus_target: Vec3,
    /// The vertical field of view, in degrees.
    pub fov: f32,
    /// How fast the focus closes on its target, per frame.
    pub focus_rate: f32,
    /// How fast the field closes on its target, per frame.
    pub fov_rate: f32,
    /// See [`Ease::substeps_per_second`].
    pub substeps_per_second: Option<f32>,
    /// How the basis is built.
    pub basis: Basis,
    /// The up the basis leans on: the world's, except on the orbit, which turns with the craft.
    pub up: Vec3,
}

impl Destroy {
    /// The frame the camera takes over: picks a station and sets the starting
    /// field. `fov_offset` is the `rand(-10, 20)` draw ([`START_FOV_SPREAD`]);
    /// the focus rate in `ease` comes from [`FOCUS_RATE_BY_CLASS`]. `None` with no
    /// stations - a circuit with none leaves the chase camera as it is.
    #[must_use]
    pub fn start(stations: &[Station], subject: Vec3, ease: Ease, fov_offset: f32) -> Option<Self> {
        let station = stations[nearest_station(stations, subject)?];
        let reach = station.eye.distance(subject).max(f32::EPSILON);
        let fov = (framing_fov_degrees(START_FRAME_SIZE, reach) + fov_offset).max(START_FOV_FLOOR);
        Some(Self {
            eye: station.eye,
            focus: subject,
            focus_target: subject,
            fov,
            focus_rate: ease.focus_rate,
            fov_rate: ease.fov_rate,
            substeps_per_second: ease.substeps_per_second,
            basis: Basis::Flattened,
            up: Vec3::Y,
        })
    }

    /// A station-style pose with every ease left at Pulse's one step a frame: `eye` looking at
    /// `focus` at `fov` degrees, for callers that move the focus themselves.
    #[must_use]
    pub fn posed(eye: Vec3, focus: Vec3, fov: f32, focus_rate: f32) -> Self {
        Self {
            eye,
            focus,
            focus_target: focus,
            fov,
            focus_rate,
            fov_rate: FOV_RATE,
            substeps_per_second: None,
            basis: Basis::Flattened,
            up: Vec3::Y,
        }
    }

    /// The hold view (row W8): the eye stays where it was, the look-at is the craft plus
    /// [`HOLD_LEAD`] times its velocity, the field is [`HOLD_FOV`].
    #[must_use]
    pub fn hold(eye: Vec3, craft: Vec3, velocity: Vec3) -> Self {
        let focus = craft + velocity * HOLD_LEAD;
        Self {
            eye,
            focus,
            focus_target: focus,
            fov: HOLD_FOV,
            focus_rate: 1.0,
            fov_rate: 1.0,
            substeps_per_second: None,
            basis: Basis::LookAt,
            up: Vec3::Y,
        }
    }

    /// One frame of `dt` seconds, after the craft has moved to `subject`. `hold` stops the
    /// focus following it - the original does so once the craft is in state 6.
    pub fn advance(&mut self, subject: Vec3, hold: bool, dt: f32) {
        let reach = self.eye.distance(subject).max(f32::EPSILON);
        let target_fov = framing_fov_degrees(FRAME_SIZE, reach);
        if !hold {
            self.focus_target = subject;
        }
        match self.substeps_per_second {
            None => {
                self.focus += (self.focus_target - self.focus) * self.focus_rate;
                self.fov += (target_fov - self.fov) * self.fov_rate;
            }
            Some(per_second) => {
                let scale = dt * per_second;
                for _ in 0..substeps(dt, per_second) {
                    self.focus += (self.focus_target - self.focus) * (self.focus_rate / scale);
                    self.fov += (target_fov - self.fov) * (self.fov_rate / scale);
                }
            }
        }
    }

    /// The vertical field of view in radians, for [`super::projection`].
    #[must_use]
    pub fn fov_radians(&self) -> f32 {
        self.fov.to_radians()
    }

    /// The camera's world transform: right, up and back as columns, the eye as
    /// translation.
    #[must_use]
    pub fn to_world(&self) -> Mat4 {
        let flatten = match self.basis {
            Basis::Flattened => (1.0 - self.fov * FLATTEN_PER_DEGREE).max(FLATTEN_FLOOR),
            Basis::LookAt => 1.0,
        };
        let mut back = self.eye - self.focus;
        back.y *= flatten;
        let back = back.normalize_or(Vec3::Z);
        let right = self.up.cross(back).normalize_or(Vec3::X);
        let up = back.cross(right);
        Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            back.extend(0.0),
            self.eye.extend(1.0),
        )
    }

    /// The view matrix.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        self.to_world().inverse()
    }
}

#[cfg(test)]
mod tests;
