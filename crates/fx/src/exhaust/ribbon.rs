//! The engine trails drawn between two ticks: a ring read at fractional ages.
//!
//! A trail records one sample a tick, newest first, and draws sample `k` at age
//! `k`: its stretch, taper, fade and texture coordinate are functions of that
//! age. Between two ticks a frame shows the state `1 - alpha` of a tick before
//! the newest, so the nozzle is drawn where the hull is ([`TrailHead`]) at age
//! 0, and every older sample `k` at age `k - 1 + alpha`. At `alpha = 1` that is
//! age `k` exactly, the ring as recorded; at `alpha` just above 0 the ring has
//! shifted one place and reads the same ages as the frame drawn just before
//! the tick, so the ribbon is continuous across the tick.
//!
//! The ring's oldest sample then sits at age `n - 2 + alpha`, short of the
//! tail's `n - 1`, so a frame between ticks adds one point: the sample that
//! fell off the ring at the last push, met at the tail's age by a lerp. The
//! ring itself, and with it every query of it (the HD trail-hit test reads
//! `Tube::nearest`), is unchanged. The extra segment is why the vertex budgets,
//! [`super::TRAIL_MAX_VERTICES_PER_CRAFT`] and
//! [`super::hd::MAX_VERTICES_PER_CRAFT`], count one segment more than the ring
//! holds.

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use super::{
    Exhaust, LAYER_COLOUR, LAYER_SCROLL_U, LAYER_SCROLL_V, LAYER_TEX_SCALE_U, LAYER_WIDTH, LAYERS,
    PS2_FIRST_LAYER, PS2_GS_COLOUR_SCALE, SCROLL_U_GAIN, SUBSTEP_DT, TEXCOORD_U16_GAIN,
    TRAIL_FADE_STEP, TRAIL_FINS, TRAIL_MAX_VERTICES_PER_CRAFT, TRAIL_SAMPLES, TRAIL_WIDTH_BASE,
    TRAIL_WIDTH_GAIN, TrailLaw, rib_vertex, trail_fade_at, trail_stencil, trail_stretch_at,
    trail_taper_at,
};

/// Where a frame drawn between two ticks puts a trail's nozzle, and how far
/// between the ticks it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrailHead {
    /// The nozzle as this draw places the hull.
    pub position: Vec3,
    /// `1.0` at the tick (the ring drawn as recorded, only the head moved),
    /// below it between two ticks whose second pushed a sample.
    pub alpha: f32,
}

/// The ring's points at their ages, newest first, into `out`; returns how many.
///
/// `samples` is the ring newest first, `dropped` the sample the last push
/// pushed off it (`None` while there is none), `head` the drawn nozzle (`None`
/// draws the ring as recorded). `with_position` puts a position on a sample,
/// `lerp` blends two. `out` holds at least `samples.len() + 1`.
pub(crate) fn aged<T: Copy>(
    samples: &[T],
    dropped: Option<T>,
    head: Option<TrailHead>,
    with_position: impl Fn(T, Vec3) -> T,
    lerp: impl Fn(T, T, f32) -> T,
    out: &mut [(T, f32)],
) -> usize {
    let n = samples.len();
    for (k, sample) in samples.iter().enumerate() {
        out[k] = (*sample, k as f32);
    }
    let Some(head) = head else {
        return n;
    };
    if n == 0 {
        return 0;
    }
    out[0].0 = with_position(samples[0], head.position);
    if head.alpha >= 1.0 {
        return n;
    }
    for (k, point) in out.iter_mut().enumerate().take(n).skip(1) {
        point.1 = (k - 1) as f32 + head.alpha;
    }
    match dropped {
        Some(dropped) => {
            out[n] = (
                lerp(samples[n - 1], dropped, 1.0 - head.alpha),
                (n - 1) as f32,
            );
            n + 1
        }
        None => n,
    }
}

/// A PSP ribbon point: where it was laid down and the exhaust direction then.
type RibbonPoint = (Vec3, Vec3);

impl Exhaust {
    /// [`Self::trail_vertices`], appended to a list the caller owns.
    ///
    /// The form the renderer uses, because a fixed 648 vertices per craft
    /// returned by value is a 36 KiB allocation per craft per frame - eight of
    /// them on a full grid, and none of them living past the upload. See
    /// `race::Scene::scratch`.
    ///
    /// `head`, when given, is the nozzle where a frame drawn between two ticks
    /// puts it, and every older sample is drawn at its fractional age (the
    /// module doc), so the whole ribbon moves on every frame. `None` draws the
    /// samples as recorded.
    pub fn extend_trail_vertices(
        &self,
        out: &mut Vec<GpuVertex>,
        head: Option<TrailHead>,
        right: Vec3,
        up: Vec3,
        law: TrailLaw,
    ) {
        if !self.trail_ready() {
            return;
        }
        let (ramps, first_layer, glow_stamp) = match law {
            TrailLaw::Ramped => (self.layer_alphas(), 0, true),
            TrailLaw::Baked => ([PS2_GS_COLOUR_SCALE; LAYERS], PS2_FIRST_LAYER, false),
        };
        let mut ring = [(Vec3::ZERO, Vec3::ZERO); TRAIL_SAMPLES];
        for (slot, sample) in ring.iter_mut().zip(self.trail_samples()) {
            *slot = sample;
        }
        let mut points = [((Vec3::ZERO, Vec3::ZERO), 0.0); TRAIL_SAMPLES + 1];
        let count = aged::<RibbonPoint>(
            &ring,
            self.trail_dropped,
            head,
            |(_, back), position| (position, back),
            |(a, back_a), (b, back_b), t| (a.lerp(b, t), back_a.lerp(back_b, t)),
            &mut points,
        );
        let points = &points[..count];
        let back_by = head
            .filter(|head| head.alpha < 1.0)
            .map_or(0.0, |head| 1.0 - head.alpha);
        let width_scale = self.intensity * TRAIL_WIDTH_GAIN + TRAIL_WIDTH_BASE;

        out.reserve(TRAIL_MAX_VERTICES_PER_CRAFT);
        // The rim, in `Trail_DrawRibbon`'s own order. The fifth entry closes
        // the tube; `v` runs 0..1 once around it in quarters.
        let rim = [up, right, -up, -right, up];
        for layer in first_layer..LAYERS {
            let ramp = ramps[layer];
            let base_width = LAYER_WIDTH[layer] * width_scale;
            let colour = LAYER_COLOUR[layer];
            // The texture scale and scroll are baked into the texcoords rather
            // than set as pipeline state, because all three layers share one
            // draw. `sceGuTexScale(su, 1.0)` plus `sceGuTexOffset(u, v)` is the
            // same thing: tile `su` times along the ribbon, shifted. The bake
            // steps `u` by `TRAIL_FADE_STEP` per sample, not by `1/(n-1)`.
            let su = LAYER_TEX_SCALE_U[layer];
            let [mut ou, mut ov] = self.scroll[layer];
            // Between ticks the scroll is a share of its last step back, the
            // step [`Exhaust::advance`] takes each tick at the fixed `dt`.
            if back_by > 0.0 {
                ou = (ou - LAYER_SCROLL_U[layer] * SUBSTEP_DT * SCROLL_U_GAIN * back_by)
                    .rem_euclid(1.0);
                ov = (ov - LAYER_SCROLL_V[layer] * SUBSTEP_DT * back_by).rem_euclid(1.0);
            }
            for k in 0..points.len() - 1 {
                let ((a, back_a), age_a) = points[k];
                let ((b, back_b), age_b) = points[k + 1];
                let pa = a + back_a * trail_stretch_at(age_a);
                let pb = b + back_b * trail_stretch_at(age_b);
                let wa = base_width * trail_taper_at(age_a);
                let wb = base_width * trail_taper_at(age_b);
                let fade_a = ramp * trail_fade_at(age_a);
                let fade_b = ramp * trail_fade_at(age_b);
                let ca = [colour[0] * fade_a, colour[1] * fade_a, colour[2] * fade_a];
                let cb = [colour[0] * fade_b, colour[1] * fade_b, colour[2] * fade_b];
                // One stencil byte per segment, not interpolated across it:
                // [`trail_stencil`], read at the segment's start age so the
                // mask slides with the ribbon between ticks (at a tick the age
                // is the segment index and the byte is the recorded one).
                let glow = if glow_stamp {
                    let last = (TRAIL_SAMPLES - 2) as f32;
                    let at = age_a.min(last);
                    let whole = at.floor();
                    let low = trail_stencil(self.intensity, whole as usize);
                    let high = trail_stencil(self.intensity, (whole + 1.0).min(last) as usize);
                    low + (high - low) * (at - whole)
                } else {
                    0.0
                };
                let ua = age_a * TRAIL_FADE_STEP * TEXCOORD_U16_GAIN * su + ou;
                let ub = age_b * TRAIL_FADE_STEP * TEXCOORD_U16_GAIN * su + ou;
                for fin in 0..TRAIL_FINS {
                    let va = fin as f32 * 0.25 * TEXCOORD_U16_GAIN + ov;
                    let vb = (fin + 1) as f32 * 0.25 * TEXCOORD_U16_GAIN + ov;
                    let a0 = rib_vertex(pa + rim[fin] * wa, ca, ua, va, glow);
                    let a1 = rib_vertex(pa + rim[fin + 1] * wa, ca, ua, vb, glow);
                    let b0 = rib_vertex(pb + rim[fin] * wb, cb, ub, va, glow);
                    let b1 = rib_vertex(pb + rim[fin + 1] * wb, cb, ub, vb, glow);
                    out.extend_from_slice(&[a0, a1, b0, a1, b1, b0]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
