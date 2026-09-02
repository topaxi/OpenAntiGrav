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
/// **Sixteen, and that number is ours rather than recovered or transliterated.**
/// A temporal upscaler conventionally derives its phase count from the ratio
/// between the presentation and render resolutions, so that a frame drawn at a
/// quarter of the pixels gets proportionally more samples to reconstruct from.
/// That formula belongs to the FSR 3.1 port that will actually consume this -
/// putting a guess at it here would dress an invention as an upstream constant,
/// which [ADR-0012](../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)'s
/// transliteration property exists to prevent. Sixteen is a plain, documented
/// default that a scale-aware count can replace in one place.
pub const PHASES: u32 = 16;

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

/// `frame`'s offset in pixels, each component in `-0.5..0.5`.
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
pub fn offset_pixels(frame: u32) -> (f32, f32) {
    let index = frame % PHASES + 1;
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
#[must_use]
pub fn matrix(frame: u32, size: (f32, f32)) -> Mat4 {
    let (x, y) = offset_pixels(frame);
    Mat4::from_translation(Vec3::new(
        2.0 * x / size.0.max(1.0),
        2.0 * y / size.1.max(1.0),
        0.0,
    ))
}

#[cfg(test)]
mod tests;
