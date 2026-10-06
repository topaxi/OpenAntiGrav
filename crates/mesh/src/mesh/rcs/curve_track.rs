//! Generic replay of a `.rcsmodel` material's own Edge Animation Tools curve.
//!
//! **Nothing here is gantry-specific.** `oag_rcs::rcsmodel::material::Curve`
//! is already generic - it reads whatever channels a material's own `+0x20`
//! pointer names, on any `.rcsmodel` - and a disc-wide sweep found 79 of 379
//! files carrying at least one live curve (35 front-end flyers, 33 billboards,
//! 10 environment models and `weapons/reticule_missile`), every one of them
//! frozen at frame zero before this module existed. This is what makes
//! [`Model::anim_tracks`](crate::mesh::Model::anim_tracks) hold one for every
//! material that carries one, the same way it already holds a Pulse/Pure
//! material's own `TEXOFFSET` block - see [`crate::mesh::AnimTrack`].
//!
//! # How a channel's value reaches `uvOffset`/`uvScale`
//!
//! `AnimCurve_EvaluateChannels` stores a sampled value into the material's own
//! float table **by `(target_index, component)`** -
//! `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 section -
//! which is a *replace* of that one component, not an add: every component a
//! curve does not drive keeps the material's own static, authored value. So
//! this module starts from the material's own static `uvOffset`/`uvScale`
//! parameters (`(0,0,0,0)`/`(1,1,1,1)` for a material that authors neither)
//! and overwrites only the components a curve's own channels name, matched by
//! the channel's resolved parameter name hash rather than position - see
//! [`Channel::name_hash`](oag_rcs::rcsmodel::material::curve::Channel::name_hash).

use std::sync::Arc;

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmodel::{self, material::Curve};

use crate::mesh::{ANIM_TRACK_LIMIT, AnimTrack};

/// One material's own curve, packaged so [`AnimTrack::sample`] needs nothing
/// but a time in seconds.
///
/// **Holds the whole `.rcsmodel` blob, shared.** [`Curve`]'s own offsets (and
/// the `EdgeAnim` clip's underneath it) are resolved against the file this
/// material was parsed from, not against a sub-slice - `oag_rcs::edgeanim`'s
/// self-relative offsets are file-absolute - so a caller cannot hand back a
/// smaller byte range without re-basing every offset the format carries.
/// `.rcsmodel` files measured are tens of kilobytes; several curved materials
/// on one file share one [`Arc`] rather than each cloning it.
#[derive(Clone)]
pub struct UvCurveTrack {
    blob: Arc<[u8]>,
    curve: Curve,
    /// The material's own static `uvOffset`, `(0,0,0,0)` when it authors none
    /// - the value every component the curve does not drive keeps.
    offset: [f32; 4],
    /// The material's own static `uvScale`, `(1,1,1,1)` when it authors none.
    scale: [f32; 4],
}

impl std::fmt::Debug for UvCurveTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UvCurveTrack")
            .field("blob_len", &self.blob.len())
            .field("curve", &self.curve)
            .field("offset", &self.offset)
            .field("scale", &self.scale)
            .finish()
    }
}

impl UvCurveTrack {
    /// This frame's `(scale, offset)`, `xy` of each - the two components
    /// `shaders/types.wesl`'s `TexAnims` actually reads.
    #[must_use]
    pub fn sample(&self, seconds: f32) -> ([f32; 2], [f32; 2]) {
        let mut offset = self.offset;
        let mut scale = self.scale;
        let uv_offset_hash = rcsmaterial::name_hash("uvOffset");
        let uv_scale_hash = rcsmaterial::name_hash("uvScale");
        for (channel, value) in self.curve.sample(&self.blob, seconds) {
            let target = if channel.name_hash == uv_offset_hash {
                Some(&mut offset)
            } else if channel.name_hash == uv_scale_hash {
                Some(&mut scale)
            } else {
                // A channel driving some other named parameter - nothing this
                // renderer reads yet. Left alone rather than guessed at.
                None
            };
            if let Some(target) = target {
                target[usize::from(channel.component)] = value;
            }
        }
        ([scale[0], scale[1]], [offset[0], offset[1]])
    }
}

/// The static value of a material's own named parameter, or `default` when it
/// authors none.
fn parameter(
    parameters: &[oag_rcs::rcsmodel::material::Parameter],
    hash: u32,
    default: [f32; 4],
) -> [f32; 4] {
    parameters
        .iter()
        .find(|p| p.hash == hash)
        .map_or(default, |p| p.value)
}

/// Every material of `model` that carries a curve, as `(material_anim, new
/// tracks)`: `material_anim[slot]` is the track a material at that slot
/// drives through [`crate::mesh::GpuVertex::anim`], `0` for a material with
/// nothing to animate - the exact shape [`crate::mesh::Model::material_anim`]
/// documents.
///
/// `model_blob` is cloned into one shared [`Arc`] the first time a curve is
/// actually found, and not at all on the (overwhelmingly common) file with
/// none.
#[must_use]
pub(super) fn material_anim_tracks(
    model: &rcsmodel::Model,
    model_blob: &[u8],
) -> (Vec<u32>, Vec<AnimTrack>) {
    let uv_offset_hash = rcsmaterial::name_hash("uvOffset");
    let uv_scale_hash = rcsmaterial::name_hash("uvScale");
    let mut material_anim = Vec::with_capacity(model.materials.len());
    let mut tracks: Vec<AnimTrack> = Vec::new();
    let mut blob: Option<Arc<[u8]>> = None;
    for material in &model.materials {
        let Some(curve) = material.curve.clone() else {
            material_anim.push(0);
            continue;
        };
        // Past the shader's table, a further curve draws unanimated rather
        // than the build failing - the same ceiling the PSP path's own
        // `anim_tracks` dedup already respects.
        if tracks.len() + 1 >= ANIM_TRACK_LIMIT {
            material_anim.push(0);
            continue;
        }
        let blob = blob.get_or_insert_with(|| Arc::from(model_blob)).clone();
        let offset = parameter(&material.parameters, uv_offset_hash, [0.0; 4]);
        let scale = parameter(&material.parameters, uv_scale_hash, [1.0; 4]);
        tracks.push(AnimTrack::Rcs(UvCurveTrack {
            blob,
            curve,
            offset,
            scale,
        }));
        material_anim.push(u32::try_from(tracks.len()).unwrap_or(0));
    }
    (material_anim, tracks)
}
