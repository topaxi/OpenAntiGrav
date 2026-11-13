//! The centre line of a trail, as a craft-versus-trail test measures it.
//!
//! Shared by the PSP-preset ribbon ([`Exhaust`]) and HD's tube
//! ([`super::hd::Tube`]): both are a ring of positions, and "is this craft inside the trail"
//! asks how far a point is from the polyline through them.

use oag_core::math::Vec3;

use super::{
    Exhaust, LAYER_WIDTH, TRAIL_FADE_SHAPE, TRAIL_FADE_STEP, TRAIL_SAMPLES, TRAIL_WIDTH_BASE,
    TRAIL_WIDTH_GAIN,
};

/// The closest point on the polyline through `points`, in order, to `point`: where it is, how
/// far away, and how far along the line (in samples from the first, fractional between two).
/// `None` for fewer than two points.
///
/// Only the first `limit` samples' worth of line is searched (`f32::INFINITY` for all of it): the
/// line is cut at that age, inside a segment when `limit` falls inside one, so a point beside
/// the cut-off part measures against the end of the kept part and never against what was cut.
///
/// Segment-wise rather than sample-wise: at race pace the samples are several units apart, so
/// testing the stored points alone would miss a craft sitting between two of them.
pub(crate) fn nearest_on_polyline(
    points: impl Iterator<Item = Vec3>,
    point: Vec3,
    limit: f32,
) -> Option<(Vec3, f32, f32)> {
    let mut best: Option<(Vec3, f32, f32)> = None;
    let mut previous: Option<Vec3> = None;
    for (k, b) in points.enumerate() {
        if let Some(a) = previous {
            let first = (k - 1) as f32;
            let kept = (limit - first).min(1.0);
            if kept <= 0.0 {
                break;
            }
            let span = (b - a) * kept;
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
            if best.is_none_or(|(_, d, _)| distance < d) {
                best = Some((on, distance, first + t * kept));
            }
        }
        previous = Some(b);
    }
    best
}

/// The least brightness, as a share of the head's, at which a craft in the ribbon counts as in
/// the trail: the ribbon is drawn at `TRAIL_FADE_SHAPE`'s curve of it, so below this a craft
/// is flying through a ribbon the player cannot see.
pub const TRAIL_HIT_MIN_BRIGHTNESS: f32 = 0.1;

impl Exhaust {
    /// The age, in samples back from the head, at which the drawn ribbon falls to
    /// [`TRAIL_HIT_MIN_BRIGHTNESS`] of the head's brightness: the colour fade
    /// ([`trail_fade_at`]) read at the preset's scaled age ([`Self::shape_age`]), inverted.
    ///
    /// The ratio to the head is the same on both laws: the PSP scales every layer by the grey
    /// ramp and the PS2 by the GS colour scale, one constant a law, and the PS2 leaving layer 0
    /// out changes which layers are drawn, not the fade curve they share
    /// (`docs/formats/pulse-absent-effects.md`, "Trail-hit burst reach").
    #[must_use]
    pub fn trail_lit_age(&self) -> f32 {
        let shape = (1.0 - TRAIL_HIT_MIN_BRIGHTNESS.powf(TRAIL_FADE_SHAPE)) / TRAIL_FADE_STEP;
        shape * (self.trail_cap - 1) as f32 / (TRAIL_SAMPLES - 1) as f32
    }

    /// The closest point on this ribbon's lit centre line to `point`, and how far it is, once
    /// the ring is full (the same gate the ribbon is drawn on, [`Self::trail_ready`]). The line
    /// ends where the ribbon fades below [`TRAIL_HIT_MIN_BRIGHTNESS`] ([`Self::trail_lit_age`]).
    #[must_use]
    pub fn trail_nearest(&self, point: Vec3) -> Option<(Vec3, f32)> {
        if !self.trail_ready() {
            return None;
        }
        nearest_on_polyline(
            self.trail_samples().map(|(position, _)| position),
            point,
            self.trail_lit_age(),
        )
        .map(|(on, away, _)| (on, away))
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
