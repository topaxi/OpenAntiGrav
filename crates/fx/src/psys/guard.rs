//! The GE's guard band: a primitive with a vertex outside the 4096-wide screen
//! space is dropped, not clipped.
//!
//! **Read off two replayed frames of the original, not off its code.** Fired
//! from a standstill, `WO_REPULSER`'s `shazzam` templates (size 20, `+0x64`
//! `6.0`) are submitted at view depth `-11.8` with corners at screen
//! `x = -2189..2600` px about the centre, and PPSSPP's software renderer,
//! replaying that exact `.ppdmp`, draws no band: the frame is the blue tint
//! alone. Three updates later the same quad is submitted at depth `-71.2`,
//! corners `-341..452`, and the replay draws a white band across the road. The
//! boundary itself was not located. Confidence 80; the reference is PPSSPP's
//! rasteriser, not a PSP.
//!
//! Applied to template quads on Pulse's PSP source only: the law is the GE's,
//! so it very likely holds for every primitive, but only these were measured.

use oag_core::math::Vec3;

/// Pixels from the screen centre a vertex may sit and still be drawn.
const LIMIT_PX: f32 = 2048.0;

/// The PSP's own projection scale in pixels per unit of depth: the frame's
/// projection `0.981` and `1.732` times the viewport's `240` and `136`.
const PX_PER_DEPTH: [f32; 2] = [240.0 * 0.981, 136.0 * 1.732];

/// A particles' nearest depth to be judged at all. The original skips a
/// particle at `z > -1` before it submits anything.
const NEAREST: f32 = 1.0;

/// Where the camera is and how it is turned, for the screen-range test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardBand {
    pub eye: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

impl GuardBand {
    /// Whether the GE would drop a primitive with these corners.
    #[must_use]
    pub fn drops(&self, corners: impl IntoIterator<Item = Vec3>) -> bool {
        corners.into_iter().any(|corner| {
            let offset = corner - self.eye;
            let depth = offset.dot(self.forward);
            depth >= NEAREST && (offset.dot(self.right) * PX_PER_DEPTH[0] / depth).abs() > LIMIT_PX
                || depth >= NEAREST
                    && (offset.dot(self.up) * PX_PER_DEPTH[1] / depth).abs() > LIMIT_PX
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> GuardBand {
        GuardBand {
            eye: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
        }
    }

    /// The two measured frames: `shazzam`'s corners at depth 11.8 fall out of
    /// range, the same quad at depth 71.2 does not.
    #[test]
    fn the_measured_shazzam_quad_is_dropped_near_and_kept_far() {
        let quad = |depth: f32, x0: f32, x1: f32, y0: f32, y1: f32| {
            [
                Vec3::new(x0, y0, -depth),
                Vec3::new(x1, y0, -depth),
                Vec3::new(x1, y1, -depth),
                Vec3::new(x0, y1, -depth),
            ]
        };
        assert!(camera().drops(quad(11.8, -109.7, 130.3, -22.9, 17.1)));
        assert!(!camera().drops(quad(71.2, -103.3, 136.7, -16.8, 23.2)));
    }

    #[test]
    fn a_quad_behind_or_at_the_near_plane_is_not_judged() {
        let behind = [Vec3::new(5000.0, 0.0, 5.0)];
        assert!(!camera().drops(behind));
        assert!(!camera().drops([Vec3::new(5000.0, 0.0, -0.5)]));
    }
}
