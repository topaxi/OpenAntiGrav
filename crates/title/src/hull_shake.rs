//! A title's drawn-hull shake: the display matrix turned and moved a little every frame by a
//! smooth random curve, faded out with speed.
//!
//! Wipeout HD's `HullShake_Update` and `HullShake_Apply` (`0x000d0c70`, `0x000d39d0`, on the
//! object at `ship+0x6b00`) run after `Ship_UpdateEntity` composes the body and the drawn lean
//! (`docs/ghidra/functions/ps3-hdfury-eu/hull-shake.md`). Every number is the title's own code
//! literal, so it is data here and not a comparison on a title's name (ADR-0058). The view-side
//! player is `oag_raceplay`'s `hull_shake`.

/// One title's hull shake, in the original's own axes: `x` the hull's left, `y` up, `z` the nose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullShake {
    /// How far a new control point reaches on each axis: the point is `scale * (r - 0.5)` with
    /// `r` uniform in `[0, 1)`. HD `(4.0, 1.8, 1.8)`.
    pub point_scale: [f32; 3],
    /// How far the curve's parameter moves per second; one new point each time it passes 1. HD
    /// advances it by the frame time, so `1.0`.
    pub rate: f32,
    /// Roll about the nose per unit of the curve's `x`, radians. HD `0.025`.
    pub roll_per_x: f32,
    /// Yaw about up per unit of the curve's `x`, radians. HD `-0.01`.
    pub yaw_per_x: f32,
    /// Pitch about the left axis per unit of the curve's `y - z`, radians. HD `0.025`.
    pub pitch_per_y_minus_z: f32,
    /// The offset per unit of the curve's change over one frame, before the fade: HD's
    /// `5.0 * 0.8`.
    pub offset_gain: f32,
    /// The fade input's one-frame filter weight on its old value. HD `0.99`.
    pub fade_filter: f32,
    /// The filtered fade input at which the shake is gone. HD `80.0`.
    pub fade_zero: f32,
    /// The fade's slope per unit below [`Self::fade_zero`], clamped to `1.0`. HD `0.02`.
    pub fade_slope: f32,
    /// The fade input per unit of world speed. HD's `(*(ship+0x6944))->f[0x4c4]` is
    /// `1.5 * speed`: `engine-trail.md` measured `3.6` times it as world speed `* 5.4`.
    pub fade_per_speed: f32,
}

impl HullShake {
    /// Wipeout HD's, read from the disassembly of `HullShake_Update`/`HullShake_Apply`.
    pub const HD: Self = Self {
        point_scale: [4.0, 1.8, 1.8],
        rate: 1.0,
        roll_per_x: 0.025,
        yaw_per_x: -0.01,
        pitch_per_y_minus_z: 0.025,
        offset_gain: 5.0 * 0.8,
        fade_filter: 0.99,
        fade_zero: 80.0,
        fade_slope: 0.02,
        fade_per_speed: 1.5,
    };
}
