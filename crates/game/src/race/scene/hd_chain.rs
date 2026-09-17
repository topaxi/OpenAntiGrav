//! Builds Wipeout HD's post chain, pulled out of `Scene::new` under the
//! 1,000-line rule in `scripts/check-file-size.py` - `scene.rs` was already
//! at the ceiling and the LeachBeam ribbon's own pipeline needed the room. A
//! move, with no behaviour change.

use log::warn;

/// A linear float scene target, the read `FunkLayerBloom` passes and the
/// encode. Present exactly when the circuit authors an `HDR and Bloom` block;
/// see `oag_render::post::hd_bloom` for what of it is the microcode's. A
/// failure is reported and dropped the way the PSP bloom's is: a race
/// without it is the pre-HDR picture, not a broken one.
///
/// **`bloom_enabled` reaches this chain too, and until 2026-09-09 it did
/// not.** The switch gated only the PSP chain, so Wipeout HD - the one title
/// this chain draws for - blooms whatever the player sets. Measured:
/// flipping the setting moved a Pulse frame's clipped-white share from
/// 2.07 % to 0.41 % and left an HD frame byte-identical. `Glow::Suppressed`
/// still runs the exposure resolve, which is what encodes the linear scene
/// target at all; see that enum for why turning the whole chain off would be
/// a different and wrong thing.
pub(super) fn build(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    hd_bloom: Option<oag_render::post::hd_bloom::Params>,
    bloom_enabled: bool,
) -> Option<oag_render::post::hd_bloom::Chain> {
    let glow = if bloom_enabled {
        oag_render::post::hd_bloom::Glow::Drawn
    } else {
        oag_render::post::hd_bloom::Glow::Suppressed
    };
    match hd_bloom
        .map(|params| oag_render::post::hd_bloom::Chain::new(device, format, size, params, glow))
        .transpose()
    {
        Ok(hd) => hd,
        Err(e) => {
            warn!("hd post chain unavailable ({e}) - the frame draws without it");
            None
        }
    }
}
