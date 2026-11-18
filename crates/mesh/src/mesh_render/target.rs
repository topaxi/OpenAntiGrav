//! Which colour space a render target holds, told by its format.
//!
//! Split out of `mesh_render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_gpu::formats::SceneStore;

/// Whether a render target holds linear light rather than gamma-encoded colour.
///
/// **The format, with what the scene stores, is the statement.** A race under a post
/// chain draws into [`oag_gpu::formats::SCENE_FORMAT`], or Wipeout HD's into the 8-bit
/// [`oag_gpu::formats::SCENE_FORMAT_8BIT`] holding encoded values, and every shader in
/// this crate that can draw into it switches to linear output when built against it -
/// no per-drawable flag exists to fall out of step with the attachment. See
/// [`oag_gpu::formats::is_scene_target`].
#[must_use]
pub fn is_linear_target(format: wgpu::TextureFormat, store: SceneStore) -> bool {
    oag_gpu::formats::is_scene_target(format, store)
}

/// The pipeline-constant list that switches a shader's `linear_out` override
/// for `format`, its `encoded_out` for a scene target that stores encoded
/// values ([`SceneStore::Encoded`]) and its `square_decode` for one in 2048's
/// gamma-2 space ([`SceneStore::Squared`]), both ignored on a gamma target, and
/// its `packed_out` for a gamma target that is a 5-6-5 card
/// ([`SceneStore::Packed565`], with `packed_gamma` for [`SceneStore::Packed565Gamma`]) - `&[]` on any other gamma target, so a pipeline
/// built against one is bit-identical to what it was before linear targets existed.
#[must_use]
pub fn linear_constants(
    format: wgpu::TextureFormat,
    store: SceneStore,
) -> &'static [(&'static str, f64)] {
    match (is_linear_target(format, store), store) {
        (true, SceneStore::Encoded) => &[("linear_out", 1.0), ("encoded_out", 1.0)],
        (true, SceneStore::Squared) => &[("linear_out", 1.0), ("square_decode", 1.0)],
        (true, SceneStore::SrgbTexels) => &[("linear_out", 1.0), ("srgb_texels", 1.0)],
        (true, SceneStore::Computed | SceneStore::Packed565 | SceneStore::Packed565Gamma) => {
            &[("linear_out", 1.0)]
        }
        (false, SceneStore::Packed565) => &[("packed_out", 1.0)],
        (false, SceneStore::Packed565Gamma) => &[("packed_out", 1.0), ("packed_gamma", 1.0)],
        (false, _) => &[],
    }
}

/// [`linear_constants`] as a whole fragment-stage options block, for the
/// pipelines whose only compilation option it is.
#[must_use]
pub fn fragment_options(
    format: wgpu::TextureFormat,
    store: SceneStore,
) -> wgpu::PipelineCompilationOptions<'static> {
    wgpu::PipelineCompilationOptions {
        constants: linear_constants(format, store),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Omega's store selects the shader's sRGB texel decode on the linear scene, and only there.
    #[test]
    fn srgb_texels_set_the_override_on_the_linear_scene() {
        let scene = oag_gpu::formats::SCENE_FORMAT;
        assert!(linear_constants(scene, SceneStore::SrgbTexels).contains(&("srgb_texels", 1.0)));
        assert!(
            !linear_constants(scene, SceneStore::Computed)
                .iter()
                .any(|(name, _)| *name == "srgb_texels")
        );
        assert!(
            linear_constants(wgpu::TextureFormat::Rgba8Unorm, SceneStore::SrgbTexels).is_empty()
        );
    }

    /// HD's 8-bit scene switches its draws to linear output with the encode, and an
    /// `Rgba8Unorm` target that stores anything else stays a gamma target.
    #[test]
    fn the_8bit_scene_is_linear_only_when_it_stores_encoded_values() {
        let eight = oag_gpu::formats::SCENE_FORMAT_8BIT;
        assert_eq!(
            linear_constants(eight, SceneStore::Encoded),
            linear_constants(oag_gpu::formats::SCENE_FORMAT, SceneStore::Encoded)
        );
        for store in [
            SceneStore::Computed,
            SceneStore::Squared,
            SceneStore::SrgbTexels,
        ] {
            assert!(linear_constants(eight, store).is_empty(), "{store:?}");
        }
    }
}
