//! The weather's mist overlay: two layers of `Data\Tex\ScreenFX\Mist.mip`
//! added over the whole screen, zooming toward the camera as it drives and
//! crossfading so the zoom never visibly restarts. Pulse's PSP `BOOT.BIN`,
//! read on 2026-10-04: `WeatherMist_Update` (`0x088f99f8`),
//! `CameraMotion_Sample` (`0x088f959c`), `WeatherMist_UpdateLayer`
//! (`0x088fa38c`) and `WeatherMist_Draw` (`0x088f9ab4`). The evidence, the
//! addresses and a confidence per claim are in
//! `docs/ghidra/functions/psp-pulse-usa/weather.md`, "The mist overlay".
//!
//! - **[`Motion::between`]** is the camera's step between two frames, in the
//!   camera's own axes: the eye's move (less the mist's drift times `dt`), the
//!   yaw and pitch steps of its back axis, and its roll. Read to the
//!   instruction and recomputed live from the original's own matrices (88).
//! - **[`Layers::step`]** is one frame of both layers: each zooms by the forward
//!   move, scrolls by the sideways and vertical moves and the turn, and fades
//!   `tri(t) * opacity` (85, the UV extents and the alpha live-checked to four
//!   places).
//! - **[`Layers::quads`]** gives the two screen quads' texture coordinates,
//!   rotated by the roll and scaled by `DisplayScale * tan(fov / 2)`.
//! - **[`Pipeline`]** draws them additively, depth test off, the bloom's glow
//!   mask untouched, after the world and before the HUD - the render queue's
//!   key `0x4f000000`, the screen flash's own layer, read live.
//!
//! Render state only: nothing here reaches a hashed simulation value, and the
//! layers' random UV offsets come from the caller's own seeded [`Rng`].

use std::f32::consts::{PI, TAU};

use oag_core::Rng;
use oag_core::math::Vec3;

use crate::psys::field::Frame;

mod pipeline;

pub use pipeline::{Pipeline, Vertex, vertices};

/// How far a layer's phase moves per unit of forward travel, `mist+0x88`
/// (`0x3ca3d70a`, set by `WeatherMist_Construct`).
pub const ZOOM_RATE: f32 = 0.02;

/// The top of the range a layer's random UV offset is drawn from:
/// `Psys_RandFloatRange(0, 0.99)` (`0x3f7d70a4`).
pub const OFFSET_RANGE: f32 = 0.99;

/// What a circuit's `<Weather>` element authors for the overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// `TexScale`: how many times the texture repeats across the screen's
    /// width at a layer's widest.
    pub tex_scale: f32,
    /// `AspectRatio`: the `v` extent is `tex_scale` times this.
    pub aspect: f32,
    /// `DisplayScale`: multiplies the field of view's half-angle tangent into
    /// the final UV scale.
    pub display_scale: f32,
}

/// The camera's step between two frames, as `CameraMotion_Sample` leaves it
/// at `mist+0x290..+0x2b8`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Motion {
    /// The step along the camera's right axis.
    pub dx: f32,
    /// The step along the camera's up axis, **negated**.
    pub dy: f32,
    /// The step along the camera's back axis: positive driving forward.
    pub dz: f32,
    /// The yaw step of the back axis, radians: positive turning left.
    pub dyaw: f32,
    /// The pitch step of the back axis, radians.
    pub dpitch: f32,
    /// The camera's roll this frame, `g_camera_roll` (`0x08ab10a8`).
    pub roll: f32,
}

impl Motion {
    /// The step from `previous` to `current`, with the mist's `drift` (world
    /// units a second) taken out over `dt` seconds.
    ///
    /// The original moves `-eye`, the fourth row of its camera matrix, so the
    /// move is `previous - current`; it then puts it in the view's axes with
    /// the instruction `Camera_SubmitScene` builds the view translation with,
    /// which is the dot product with each camera axis here.
    #[must_use]
    pub fn between(previous: &Frame, current: &Frame, drift: Vec3, dt: f32) -> Self {
        let step = (previous.position - current.position) - drift * dt;
        let (yaw_now, pitch_now) = yaw_pitch(current.back);
        let (yaw_then, pitch_then) = yaw_pitch(previous.back);
        Self {
            dx: step.dot(current.right),
            dy: -step.dot(current.up),
            dz: step.dot(current.back),
            dyaw: -wrap(yaw_now - yaw_then),
            dpitch: wrap(pitch_now - pitch_then),
            roll: camera_roll(current),
        }
    }
}

/// `(-atan2f(back.x, back.z), -asinf(back.y))`, `mist+0x2a4`/`+0x2a0`.
fn yaw_pitch(back: Vec3) -> (f32, f32) {
    (-back.x.atan2(back.z), -back.y.asin())
}

/// Folds an angle step into `(-pi, pi]`, the two compares at `0x088f97dc` and
/// `0x088f981c`.
fn wrap(step: f32) -> f32 {
    if step > PI {
        step - TAU
    } else if step < -PI {
        step + TAU
    } else {
        step
    }
}

/// The camera's roll, `g_camera_roll` (`0x08ab10a8`) as `Camera_SubmitScene`
/// writes it: `acosf` of the `y` of the up axis with its part along the
/// horizontal back direction taken out, negative when that vector leans
/// against the horizontal right.
#[must_use]
pub fn camera_roll(camera: &Frame) -> f32 {
    let back = camera.back;
    let flat_x = if back.x * back.x + back.z * back.z <= 0.0 {
        1.0
    } else {
        back.x
    };
    let length = (flat_x * flat_x + back.z * back.z).sqrt();
    let flat = Vec3::new(flat_x / length, 0.0, back.z / length);
    let up = camera.up;
    let mut lean = up - flat * up.dot(flat);
    if lean.length_squared() <= 0.0 {
        lean.x = 1.0;
    }
    let lean = lean / lean.length();
    let mut roll = if lean.y == 0.0 { 0.0 } else { lean.y.acos() };
    let right = Vec3::new(camera.right.x, 0.0, camera.right.z);
    let right = right / right.length();
    if right.dot(lean) < 0.0 {
        roll = -roll;
    }
    roll
}

/// One layer's texture rectangle before the roll: corners `(u0, v0)` and
/// `(u1, v1)`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Rect {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

/// The two layers' running state, `mist+0x3c..+0x80` and the quads they
/// leave.
#[derive(Debug, Clone, PartialEq)]
pub struct Layers {
    /// The sideways and vertical scrolls, `+0x3c`/`+0x40` and `+0x48`/`+0x4c`.
    scroll: [[f32; 2]; 2],
    /// The turn's scrolls, `+0x54`/`+0x58` and `+0x5c`/`+0x60`.
    turn: [[f32; 2]; 2],
    /// The zoom phases, `+0x44` and `+0x50`.
    phase: [f32; 2],
    /// `t = 1 - phase` as last stepped, `+0x74`/`+0x78`.
    t: [f32; 2],
    /// The random UV offsets, `+0x64..+0x70`.
    offset: [[f32; 2]; 2],
    rect: [Rect; 2],
    alpha: [f32; 2],
    roll: f32,
}

impl Layers {
    /// As `WeatherMist_Construct` leaves them: layer 1 half a phase and half
    /// a scroll on, four random offsets drawn.
    ///
    /// `t` starts at `0`, chosen, not measured: the node's bytes before the
    /// first step were not read, and the only use of the old `t` is the
    /// re-roll test, which a first step from `0` merely fires once.
    #[must_use]
    pub fn new(rng: &mut Rng) -> Self {
        let mut draw = || rng.next_f32() * OFFSET_RANGE;
        let offset = [[draw(), draw()], [draw(), draw()]];
        Self {
            scroll: [[0.0, 0.0], [0.5, 0.5]],
            turn: [[0.0, 0.0], [0.0, 0.0]],
            phase: [0.0, 0.5],
            t: [0.0, 0.0],
            offset,
            rect: [Rect::default(); 2],
            alpha: [0.0, 0.0],
            roll: 0.0,
        }
    }

    /// One frame of both layers, `WeatherMist_UpdateLayer` for layer 0 then 1.
    /// The original steps only while the opacity is above zero.
    pub fn step(&mut self, config: &Config, motion: &Motion, opacity: f32, rng: &mut Rng) {
        for layer in 0..2 {
            self.step_layer(layer, config, motion, opacity, rng);
        }
        self.roll = motion.roll;
    }

    fn step_layer(
        &mut self,
        layer: usize,
        config: &Config,
        motion: &Motion,
        opacity: f32,
        rng: &mut Rng,
    ) {
        let mut phase = (self.phase[layer] + motion.dz * ZOOM_RATE) % 1.0;
        if phase < 0.0 {
            phase += 1.0;
        }
        self.phase[layer] = phase;
        let previous = self.t[layer];
        let t = 1.0 - phase;
        self.t[layer] = t;
        // A wrap re-rolls the layer's offsets, so a restarted zoom does not
        // show the same texels again.
        if (previous - t).abs() > 0.5 {
            self.offset[layer] = [rng.next_f32() * OFFSET_RANGE, rng.next_f32() * OFFSET_RANGE];
        }
        let wide = config.tex_scale;
        let tall = config.tex_scale * config.aspect;
        let [scroll_u, scroll_v] = &mut self.scroll[layer];
        let [turn_u, turn_v] = &mut self.turn[layer];
        *scroll_u += motion.dx * t * ZOOM_RATE * wide;
        *turn_u += motion.dyaw * config.aspect * t * wide;
        *turn_v += motion.dpitch * t * tall;
        *scroll_v += motion.dy * t * ZOOM_RATE * tall;
        let fade = if t >= 0.5 { 2.0 - 2.0 * t } else { 2.0 * t };
        let u0 = self.offset[layer][0] + (1.0 - t) * wide * 0.5 + (*turn_u + *scroll_u) % 1.0;
        let v0 = self.offset[layer][1] + (1.0 - t) * tall * 0.5 + (*turn_v + *scroll_v) % 1.0;
        self.alpha[layer] = fade * opacity;
        self.rect[layer] = Rect {
            u0,
            v0,
            u1: u0 + t * wide,
            v1: v0 + t * tall,
        };
    }

    /// Each layer's colour alpha as the draw hands it to the GE: the float
    /// truncated to a byte, then back to `0..=1`.
    #[must_use]
    pub fn alphas(&self) -> [f32; 2] {
        self.alpha.map(|alpha| (alpha * 255.0) as u8 as f32 / 255.0)
    }

    /// The two quads' texture coordinates at the clip-space corners
    /// [`CORNERS`], each rotated by the roll about its own centre and scaled
    /// by `scale` (`DisplayScale * tan(fov / 2)`) about it.
    #[must_use]
    pub fn quads(&self, scale: f32) -> [[[f32; 2]; 4]; 2] {
        let (sin, cos) = self.roll.sin_cos();
        self.rect.map(|rect| {
            let centre = [(rect.u0 + rect.u1) * 0.5, (rect.v0 + rect.v1) * 0.5];
            let corners = [
                [rect.u0, rect.v0],
                [rect.u0, rect.v1],
                [rect.u1, rect.v0],
                [rect.u1, rect.v1],
            ];
            corners.map(|[u, v]| {
                let du = u - centre[0];
                let dv = v - centre[1];
                [
                    (du * cos - dv * sin) * scale + centre[0],
                    (du * sin + dv * cos) * scale + centre[1],
                ]
            })
        })
    }
}

/// The clip-space corners `WeatherMist_Construct` gives both quads, in strip
/// order: `(u0, v0)` is the bottom right.
pub const CORNERS: [[f32; 2]; 4] = [[1.0, -1.0], [1.0, 1.0], [-1.0, -1.0], [-1.0, 1.0]];

#[cfg(test)]
mod tests;
