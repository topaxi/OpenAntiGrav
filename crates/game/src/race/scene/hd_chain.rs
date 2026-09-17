//! Building Wipeout HD's post chain for [`super::Scene::new`].
//!
//! Split out for the 1,000-line rule in `scripts/check-file-size.py`; a
//! move, with no behaviour change - `Scene::new` had this inline before the
//! Cannon's own two quads needed a field, a parameter and a construction
//! call there and left it with nowhere to put them.

use super::*;

/// Wipeout HD's post chain: a linear float scene target, the read
/// FunkLayerBloom passes and the encode. Present exactly when the circuit
/// authors an `HDR and Bloom` block - see `oag_render::post::hd_bloom` for
/// what of it is the microcode's. A failure is reported and dropped the way
/// the PSP bloom's is: a race without it is the pre-HDR picture, not a
/// broken one.
///
/// **`bloom_enabled` reaches this chain too, and until 2026-09-09 it did
/// not.** The switch gated only the PSP chain below, so Wipeout HD - the
/// one title this chain draws for - blooms whatever the player sets.
/// Measured: flipping the setting moved a Pulse frame's clipped-white share
/// from 2.07 % to 0.41 % and left an HD frame byte-identical.
/// `Glow::Suppressed` still runs the exposure resolve, which is what
/// encodes the linear scene target at all; see that enum for why turning
/// the whole chain off would be a different and wrong thing.
///
/// Returns the chain itself (or `None`), the caller's own surface format -
/// **the format is the statement about colour space**: with the chain in
/// place every pipeline built after this call switches to the linear float
/// target and its own linear output, see `mesh_render::is_linear_target` -
/// and that caller format again, unshadowed, for the one pass that runs
/// after every chain has already encoded into the caller's view (see
/// `motion_blur` in `Scene::new`).
pub(super) fn build(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    hd_bloom: Option<oag_render::post::hd_bloom::Params>,
    bloom_enabled: bool,
) -> (
    Option<oag_render::post::hd_bloom::Chain>,
    wgpu::TextureFormat,
    wgpu::TextureFormat,
) {
    let glow = if bloom_enabled {
        oag_render::post::hd_bloom::Glow::Drawn
    } else {
        oag_render::post::hd_bloom::Glow::Suppressed
    };
    let hd = match hd_bloom
        .map(|params| oag_render::post::hd_bloom::Chain::new(device, format, size, params, glow))
        .transpose()
    {
        Ok(hd) => hd,
        Err(e) => {
            warn!("hd post chain unavailable ({e}) - the frame draws without it");
            None
        }
    };
    let caller_format = format;
    let format = if hd.is_some() {
        oag_render::post::hd_bloom::SCENE_FORMAT
    } else {
        format
    };
    (hd, format, caller_format)
}
