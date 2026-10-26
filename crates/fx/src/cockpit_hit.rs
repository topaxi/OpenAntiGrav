//! The static a weapon hit throws over the cockpit view: `CockpitHitFx`, read
//! off Pulse's PSP `BOOT.BIN` and watched live on PPSSPP on 2026-10-26. See
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "2026-10-26:
//! why a Quake hit feels stronger on the original".
//!
//! `Ship_Damage` (`0x088439ac`) calls `CockpitHitFx_Arm(1.5, 1.5, 0.6, 1.0, ..)`
//! on every weapon hit to the human, beside `Camera_ArmShake`. One node,
//! `g_cockpit_hit_fx` (`0x08ab2120`), holds one run at a time:
//!
//! - **Arm** (`0x088eeaf8`) starts it, and **an arm while it runs is ignored**:
//!   no restart, no stacking. The strip width is `1.5 * 192 = 288` pixels of the
//!   PSP's 480 by 272 frame and the rate is `1 / 0.6`.
//! - **Update** (`CockpitHitFx_Update`, `0x088eeb54`), once a frame: the age
//!   grows by the frame's `dt`, `p = age / 0.6`, and while `p < 1` both strips
//!   are `288 * (1 - p)` wide and a texture offset pair is redrawn as
//!   `rand(0..7) / 8`. At `p >= 1` the node stops. Live, on two boots, it ran
//!   for exactly 36 frames with `p` from 0 to 35/36.
//! - **Draw** (`CockpitHitFx_Draw`, `0x088eec8c`), queued at the screen flash's
//!   layer `0x4f000000` only while the camera is the internal view: the texture
//!   `Data\Tex\noisematt.mip` (64 by 64) on **two gradient strips**, the left
//!   over `x` in `[0, w]` and the right over `[480 - w, 480]`, full height.
//!   Vertex colours are white at alpha `(1 - p) * 255` on the screen-edge pair
//!   and `0x00000000` on the inner pair, so each strip is opaque noise at the
//!   screen's edge and clear at its inner edge. `u` runs `0` to `w / 64`, `v`
//!   `0` to `272 / 64`, both under a texture scale of `(0.125, 1)` and shifted
//!   by the offset pair; wrap is repeat, the blend `(ADD, SRC_ALPHA,
//!   ONE_MINUS_SRC_ALPHA)`, and depth, lighting and culling are off.
//!
//! **Chosen, not measured**: the texture filter (the draw inherits whatever the
//! previous draw left, and the state it restores afterwards, `(7, 1)`, reads as
//! linear), and the order against the screen flash within their shared layer
//! (unread). The offsets come from a render-side generator drawn once a tick,
//! where the original draws its own every frame, so a capture depends on the
//! tick count alone.

use oag_core::Rng;

mod pipeline;
pub use pipeline::{Drawer, Drawn, Pipeline};

#[cfg(test)]
mod tests;

/// Seconds a run lasts: `CockpitHitFx_Arm`'s third argument.
pub const DURATION: f32 = 0.6;

/// `+0x54`, the rate `p` grows at. Stored as an `f32` in the node, so the
/// reciprocal is taken in `f32` too.
const RATE: f32 = 1.0 / DURATION;

/// A strip's width at the hit, in pixels of the PSP's frame: `1.5 *
/// DAT_08a84a30`, which is `192`.
pub const STRIP_WIDTH: f32 = 288.0;

/// The PSP's frame, which the strips' widths and heights are authored in.
pub const FRAME: [f32; 2] = [480.0, 272.0];

/// The side of the noise texture, which the draw divides its extents by.
pub const TEXELS: u32 = 64;

/// The draw's texture scale on `u`: the noise is stretched about 8:1 across.
pub const UV_SCALE_U: f32 = 0.125;

/// The offset draws are `rand(0..7) / 8`: eighths of the texture.
const OFFSET_STEPS: u32 = 8;

/// One corner of a strip: a position in clip space, the texture coordinate and
/// the vertex colour the noise is modulated by.
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    /// Clip-space `x, y`: `-1, 1` is the view's top-left corner.
    pub position: [f32; 2],
    /// The texture coordinate, past `1` where the repeat wraps.
    pub uv: [f32; 2],
    /// Straight RGBA, `0..=1`.
    pub colour: [f32; 4],
}

/// The vertices of both strips, left then right, four each in the triangle-strip
/// order the original submits them: top edge, top inner, bottom edge, bottom inner.
pub const VERTICES: usize = 8;

/// The node's state: render-only, never in `World`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Node {
    running: bool,
    /// Armed and not yet advanced: the first frame drawn is `p = 0`, "at the
    /// hit", where the second is one step in.
    fresh: bool,
    age: f32,
    /// `+0x5c`, `+0x60`: the texture offset in eighths, redrawn every step.
    offset: [f32; 2],
}

impl Node {
    /// `CockpitHitFx_Arm`: starts a run, unless one is already going.
    pub fn arm(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        self.fresh = true;
        self.age = 0.0;
    }

    /// `CockpitHitFx_Update`: one step of `dt` seconds.
    pub fn advance(&mut self, dt: f32, rng: &mut Rng) {
        if !self.running {
            return;
        }
        if self.fresh {
            self.fresh = false;
        } else {
            self.age += dt;
            if self.progress() >= 1.0 {
                self.running = false;
                return;
            }
        }
        self.offset = [
            rng.below(OFFSET_STEPS) as f32,
            rng.below(OFFSET_STEPS) as f32,
        ]
        .map(|steps| steps / OFFSET_STEPS as f32);
    }

    /// Whether a run is going.
    #[must_use]
    pub fn running(&self) -> bool {
        self.running
    }

    /// `p`, `0` at the hit and approaching `1` as the run ends.
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.age * RATE
    }

    /// The strip width this step, in the PSP's pixels.
    #[must_use]
    pub fn width(&self) -> f32 {
        STRIP_WIDTH * (1.0 - self.progress())
    }

    /// The static as `CockpitHitFx_Draw` builds it, or `None` when nothing runs.
    #[must_use]
    pub fn strips(&self) -> Option<[Vertex; VERTICES]> {
        if !self.running {
            return None;
        }
        let w = self.width();
        // The colour the GE gets: alpha truncated to a byte, as the flash's is.
        let alpha = f32::from(((1.0 - self.progress()) * 255.0) as u8) / 255.0;
        let edge = [1.0, 1.0, 1.0, alpha];
        let clear = [0.0; 4];
        let [offset_u, offset_v] = self.offset;
        let reach = w / FRAME[0] * 2.0;
        let u_inner = offset_u + w / TEXELS as f32 * UV_SCALE_U;
        let v_bottom = offset_v + FRAME[1] / TEXELS as f32;
        let strip = |edge_x: f32, inner_x: f32| {
            let corner = |x: f32, y: f32, u: f32, v: f32, colour: [f32; 4]| Vertex {
                position: [x, y],
                uv: [u, v],
                colour,
            };
            [
                corner(edge_x, 1.0, offset_u, offset_v, edge),
                corner(inner_x, 1.0, u_inner, offset_v, clear),
                corner(edge_x, -1.0, offset_u, v_bottom, edge),
                corner(inner_x, -1.0, u_inner, v_bottom, clear),
            ]
        };
        let [l0, l1, l2, l3] = strip(-1.0, -1.0 + reach);
        let [r0, r1, r2, r3] = strip(1.0, 1.0 - reach);
        Some([l0, l1, l2, l3, r0, r1, r2, r3])
    }
}
