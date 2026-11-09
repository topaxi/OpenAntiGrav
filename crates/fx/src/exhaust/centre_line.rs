//! The centre line of a trail, as a craft-versus-trail test measures it.
//!
//! Shared by the PSP-preset ribbon ([`Exhaust`]) and HD's tube
//! ([`super::hd::Tube`]): both are a ring of positions, and "is this craft inside the trail"
//! asks how far a point is from the polyline through them.

use oag_core::math::Vec3;

use super::{Exhaust, LAYER_WIDTH, TRAIL_WIDTH_BASE, TRAIL_WIDTH_GAIN};

/// The closest point on the polyline through `points`, in order, to `point`, and how far it is.
/// `None` for fewer than two points.
///
/// Segment-wise rather than sample-wise: at race pace the samples are several units apart, so
/// testing the stored points alone would miss a craft sitting between two of them.
pub(crate) fn nearest_on_polyline(
    points: impl Iterator<Item = Vec3>,
    point: Vec3,
) -> Option<(Vec3, f32)> {
    let mut best: Option<(Vec3, f32)> = None;
    let mut previous: Option<Vec3> = None;
    for b in points {
        if let Some(a) = previous {
            let span = b - a;
            let length_squared = span.length_squared();
            // A degenerate segment - which every one of them is at a race start, where the
            // whole ring sits inside a fraction of a unit - collapses to its own endpoint
            // rather than dividing by zero.
            let t = if length_squared > 1e-12 {
                ((point - a).dot(span) / length_squared).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let on = a + span * t;
            let distance = (point - on).length();
            if best.is_none_or(|(_, d)| distance < d) {
                best = Some((on, distance));
            }
        }
        previous = Some(b);
    }
    best
}

impl Exhaust {
    /// The closest point on this ribbon's centre line to `point`, and how far it is, once the
    /// ring is full (the same gate the ribbon is drawn on, [`Self::trail_ready`]).
    #[must_use]
    pub fn trail_nearest(&self, point: Vec3) -> Option<(Vec3, f32)> {
        if !self.trail_ready() {
            return None;
        }
        nearest_on_polyline(self.trail_samples().map(|(position, _)| position), point)
    }

    /// A box holding every sample of the ring, so a point farther than a distance from the box
    /// is farther than that from [`Self::trail_nearest`]'s line. `None` until the ring is full.
    #[must_use]
    pub fn trail_bounds(&self) -> Option<(Vec3, Vec3)> {
        self.trail_ready().then(|| {
            self.trail_samples().fold(
                (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                |(low, high), (position, _)| (low.min(position), high.max(position)),
            )
        })
    }

    /// How far the ribbon reaches either side of its centre line at the head: the widest
    /// layer's width at the current intensity (`Exhaust_Update`'s `intensity * 0.35 + 0.2`
    /// scale, [`LAYER_WIDTH`]). The ribbon tapers behind the head, so this is the most it
    /// reaches.
    #[must_use]
    pub fn trail_half_width(&self) -> f32 {
        LAYER_WIDTH[0] * (self.intensity * TRAIL_WIDTH_GAIN + TRAIL_WIDTH_BASE)
    }
}
