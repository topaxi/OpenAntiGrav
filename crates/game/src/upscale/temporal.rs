//! What a *temporal* upscaler needs, and the one question the rest of the game
//! has to answer for it.
//!
//! Its own file rather than the top of `upscale.rs`, under the 1,000-line rule
//! in `scripts/check-file-size.py` - and a real seam: everything here is about
//! the difference between reconstructing from several frames and resampling
//! one, which is the whole distinction `Upscaler::is_temporal` names.

use crate::display::Upscaler;
use oag_render::post::fsr3;

/// What a temporal upscaler needs that a spatial one does not.
///
/// **`None` on every frame that is not a race**, which is most of them: the
/// launcher, the loading screen, the front end and the menus have no scene, no
/// depth and no motion (ADR-0038), so there is nothing for a history to
/// reconstruct from and `Upscaler::Fsr3` degrades to the blit there exactly as
/// `Upscaler::Fsr1` does.
///
/// **It is also where the adapter probe lands.** A caller supplies `Some` only
/// when `oag_render::post::fsr3::supported` said yes about the adapter it
/// built its device from - which is the one place that knows, and which is not
/// [`Framebuffer`]. So `None` covers three different "no"s with one shape: no
/// scene, no compute shaders, and no history yet. All three fall the same rung
/// down the ladder, which is what
/// [ADR-0012](../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
/// asks for.
///
/// Carried as one bundle rather than five parameters because it is one
/// question - "is this frame part of a moving scene, and what were its
/// camera and offset" - and because a caller that can answer part of it can
/// answer all of it.
#[derive(Debug, Clone, Copy)]
pub struct Temporal<'a> {
    /// The scene's depth attachment.
    pub depth: &'a wgpu::TextureView,
    /// The scene's velocity attachment.
    pub velocity: &'a wgpu::TextureView,
    /// How many samples the two above carry - 1, or MSAA's 4.
    ///
    /// **Travels with them rather than being read from the settings**, which is
    /// the distinction that matters: `[graphics] anti_aliasing` is what a
    /// player last chose, while this is what the scene on screen was actually
    /// built with, and the two differ for a whole race after the row moves -
    /// which is exactly what its `restart_required` note is about.
    pub sample_count: u32,
    /// The camera this frame was drawn with.
    pub camera: fsr3::Camera,
    /// The sub-pixel offset this frame was drawn with, in pixels.
    pub jitter: (f32, f32),
    /// The jitter sequence's length - `oag_render::jitter::phases`.
    pub phase_count: u32,
    /// Whether the history is meaningless: the first frame of a race, or a
    /// camera cut.
    pub reset: bool,
}

/// The jitter sequence this frame's scene should be drawn with, or `None` for
/// no jitter at all.
///
/// **The upscaler decides this, not a settings row**, and that inversion is the
/// point of the function existing. Sub-pixel jitter is not a picture setting: it
/// is an input a *temporal* reconstruction needs and a *spatial* one is actively
/// harmed by, because with nothing resolving the offsets they are a shimmer
/// bought for nothing
/// ([ADR-0039](../../../docs/architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)).
/// So FSR 3.1 turns it on whether or not `flag` is set, and everything else
/// leaves it to the flag.
///
/// `flag` is `--camera-jitter`, which stays an override for looking at jitter on
/// its own. It gets [`oag_render::jitter::DEFAULT_PHASES`] rather than a
/// ratio-derived count, because with no upscaler behind it there is no ratio -
/// nothing is resolving the frames it jitters.
///
/// `extent` is the rectangle the scene is actually drawn into and `output` the
/// presentation rectangle; the count is quadratic in their ratio, so it has to
/// be recomputed as a resolution controller moves the extent.
#[must_use]
pub fn jitter_phases(
    flag: bool,
    upscaler: Upscaler,
    supported: bool,
    extent: (u32, u32),
    output: (u32, u32),
) -> Option<u32> {
    // **`supported` too, not just the setting.** A `fsr3` chosen on an adapter
    // that cannot run it falls to FSR 1, which is spatial - and jittering for a
    // spatial resolve is the one combination ADR-0039 says is strictly worse
    // than not jittering at all.
    if upscaler.is_temporal() && supported {
        return Some(oag_render::jitter::phases(extent.0, output.0));
    }
    flag.then_some(oag_render::jitter::DEFAULT_PHASES)
}
