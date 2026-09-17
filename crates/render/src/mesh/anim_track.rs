//! [`AnimTrack`]: one entry of [`crate::mesh::Model::anim_tracks`].
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_vex::vex;

use super::rcs;

/// One entry of [`crate::mesh::Model::anim_tracks`]: a UV `(scale, offset)`
/// sampled at a point in the model's animation clock, from whichever title's
/// own texture animation mechanism authored it.
///
/// Two titles, two mechanisms, one shared shader table: `mesh.wgsl`'s
/// `TexAnims` already applies `uv * scale + offset` per vertex without caring
/// which - see [`crate::mesh::GpuVertex::anim`]. Pulse/Pure walk a material's
/// own `TEXOFFSET`/`TEXSCALE` keyframe block; Wipeout HD samples a material's
/// own Edge Animation Tools curve against its static `uvOffset`/`uvScale`
/// parameters - see [`rcs::curve_track`].
#[derive(Debug, Clone)]
pub enum AnimTrack {
    /// A Pulse/Pure `TEXOFFSET`/`TEXSCALE` block, `oag_vex::vex` already
    /// decodes.
    Psp(vex::TexTransform),
    /// A Wipeout HD material's own animated curve, plus whatever of its
    /// static `uvOffset`/`uvScale` the curve does not drive.
    Rcs(rcs::curve_track::UvCurveTrack),
}

impl AnimTrack {
    /// This frame's `(scale, offset)`, the same shape either variant
    /// produces. See [`vex::TexTransform::sample`] and
    /// [`rcs::curve_track::UvCurveTrack::sample`].
    #[must_use]
    pub fn sample(&self, seconds: f32) -> ([f32; 2], [f32; 2]) {
        match self {
            Self::Psp(track) => track.sample(seconds),
            Self::Rcs(track) => track.sample(seconds),
        }
    }
}
