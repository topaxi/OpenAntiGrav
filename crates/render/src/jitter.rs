//! Sub-pixel camera jitter: the per-frame offset a temporal upscaler needs the
//! rasteriser to move by, so that several frames of the same still scene sample
//! it at different points inside each pixel.
//!
//! **Nothing here consumes it yet.** It is the last of the renderer-side
//! prerequisites [`modern features`](../../../docs/overview/modern-features.md)
//! lists for FSR 3.1, and it is built ahead of the upscaler that will read it
//! because the constraint it has to satisfy - not perturbing the culling
//! frustum or the velocity buffer - is a property of where it is applied, and
//! that is much cheaper to get right now than to retrofit. On its own, with a
//! spatial resolve or none, jitter makes the picture **worse**: it is a shimmer
//! with no reconstruction behind it. That is why it is off unless asked for.
//!
//! # Why this is a matrix and not a shader change
//!
//! The offset is applied by post-multiplying a clip-space translation onto the
//! view-projection: `T * VP`, where `T` translates by `(x, y, 0)`. Because a
//! translation's contribution is scaled by the vector's `w`, that is exactly
//! `clip.x += x * clip.w`, which after the perspective divide is a constant
//! offset in NDC and therefore a constant offset in pixels. `z` and `w` come
//! through untouched, so depth, the fog's view distance and the Zone glow's own
//! `clip.z` read all mean what they meant before.
//!
//! The same `T` is applied to the *previous* tick's matrix as well. Both clip
//! positions then shift by the same NDC amount, the velocity target measures
//! their difference, and the jitter cancels out of it exactly - which is what a
//! temporal upscaler wants, since it applies the offset itself and would
//! double-count one baked into the motion vectors. No `.wgsl` file changes.
//!
//! [ADR-0039](../../../docs/architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)
//! records the ordering this depends on.

use oag_core::math::{Mat4, Vec3};

/// How many frames the sequence runs for before repeating.
///
/// **Eight, and upstream's**: `basePhaseCount` in
/// `ffxFsr3UpscalerGetJitterPhaseCount`. This was a documented sixteen of our
/// own until 2026-09-03, standing in for the ratio-derived count because
/// deriving it belonged to the FSR 3.1 port and inventing it here would have
/// dressed a guess as an upstream constant - which
/// [ADR-0012](../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)'s
/// transliteration property exists to prevent. [`phases`] is that derivation,
/// and this is the base it scales from.
pub const BASE_PHASES: u32 = 8;

/// The sequence length a caller with no upscaler behind it uses.
///
/// `--camera-jitter` on its own has no presentation-to-render ratio to derive a
/// count from, because nothing is resolving the frames it jitters. It gets the
/// 1:1 answer, which is what [`phases`] returns for equal widths.
pub const DEFAULT_PHASES: u32 = BASE_PHASES;

/// How many phases a `render_width`-wide frame resolved to `display_width`
/// wants: `8 * (display / render)^2`.
///
/// Transliterated from `ffxFsr3UpscalerGetJitterPhaseCount` in AMD's
/// MIT-licensed `ffx_fsr3upscaler.cpp` (FidelityFX-SDK `v1.1.4`, commit
/// `c6efa6bf7f2027b3ec94f28578bb5965eabb9e55`). The licence travels in
/// `licences/AMD-FidelityFX-MIT.txt`; the port is
/// [fsr3.md](../../../docs/rendering/fsr3.md).
///
/// **Quadratic because the pixel count is.** A frame drawn at half the width is
/// drawn at a quarter of the pixels, so it takes four times as many phases
/// before the presentation grid has been sampled as densely - eight at 100 %,
/// thirty-two at 50 %.
///
/// Upstream truncates toward zero and does not clamp. The `max(1)` is
/// therefore ours and is the only departure - but it is a **guard rather than a
/// behaviour**: reaching zero needs a ratio below `sqrt(1/8)`, which is a render
/// scale past 283 %, and the deepest `Scale::OFFERED` holds is 200 % (a ratio of
/// one half, which truncates to two). What it actually catches is a degenerate
/// size, of which a window with no extent yet is the one that happens.
#[must_use]
pub fn phases(render_width: u32, display_width: u32) -> u32 {
    let ratio = display_width.max(1) as f32 / render_width.max(1) as f32;
    ((BASE_PHASES as f32 * ratio * ratio) as u32).max(1)
}

/// The radical inverse of `index` in `base` - one term of a Halton sequence.
///
/// Halton rather than a random offset because a low-discrepancy sequence covers
/// the pixel evenly at every prefix length, so a scene that stops moving after
/// four frames has still been sampled at four well-spread points rather than at
/// four that happen to cluster.
fn halton(mut index: u32, base: u32) -> f32 {
    let mut fraction = 1.0;
    let mut result = 0.0;
    while index > 0 {
        fraction /= base as f32;
        result += fraction * (index % base) as f32;
        index /= base;
    }
    result
}

/// `frame`'s offset in pixels within a `phases`-long sequence, each component
/// in `-0.5..0.5`.
///
/// **This is `ffxFsr3UpscalerGetJitterOffset` exactly**, which was found out
/// after the fact rather than designed: the function was written here from the
/// same reasoning upstream used, and reading `ffx_fsr3upscaler.cpp` during the
/// FSR 3.1 port turned it up line for line - `halton(index % phaseCount + 1,
/// 2) - 0.5` on x and base 3 on y. The two paragraphs below were this module's
/// own justification for each half and stand as upstream's reasoning too.
///
/// **The sequence is 1-indexed**, so frame 0 is `halton(1, _)` and not
/// `halton(0, _)`. Zero is the radical inverse's fixed point in every base - it
/// would put the first frame exactly on the pixel centre, wasting a phase on
/// the one offset that is indistinguishable from jitter being off.
///
/// Centred by subtracting a half, so the offsets straddle the pixel centre
/// instead of filling the quadrant above it. Uncentred, a whole render would sit
/// a consistent half-pixel off from an unjittered one.
#[must_use]
pub fn offset_pixels(frame: u32, phases: u32) -> (f32, f32) {
    let index = frame % phases.max(1) + 1;
    (halton(index, 2) - 0.5, halton(index, 3) - 0.5)
}

/// The clip-space translation that applies `frame`'s offset on a render target
/// `size` pixels across.
///
/// `size` is the **render** target's extent, not the presentation surface's:
/// the offset has to be a half-pixel of the pixels actually being rasterised,
/// which under a render scale below 100 % are the larger ones. NDC spans two
/// units across that extent, hence the doubling.
///
/// Post-multiply this onto a view-projection - `matrix(..) * view_projection` -
/// and onto the previous tick's, with the same `frame`. See this module's own
/// documentation for why both.
///
/// `phases` comes from [`phases`] when an upscaler is resolving these frames
/// and from [`DEFAULT_PHASES`] when nothing is.
#[must_use]
pub fn matrix(frame: u32, phases: u32, size: (f32, f32)) -> Mat4 {
    let (x, y) = offset_pixels(frame, phases);
    Mat4::from_translation(Vec3::new(
        2.0 * x / size.0.max(1.0),
        2.0 * y / size.1.max(1.0),
        0.0,
    ))
}

#[cfg(test)]
mod tests;
