//! The LeachBeam's own ribbon: the segmented energy strip drawn between the
//! holder and the craft it has locked onto.
//!
//! Recovered from `LeachBeam_Advance` (`0x08873fa0`), `LeachBeam_InitLocked`
//! (`0x08873d3c`), `LeachBeam_Construct` (`0x08872aa0`), `LeachBeam_LoadTexture`
//! (`0x088730d4`), `LeachBeam_BuildStrip` (`0x088739b0`) and
//! `LeachBeam_SubmitStrip` (`0x088731c4`). Addresses, evidence and a
//! confidence score per claim are in
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s
//! "2026-09-17: the LeachBeam ribbon's own texture" section; this module
//! implements what that page describes and cites it rather than restating it.
//!
//! # Two halves, the same split [`crate::exhaust`] uses
//!
//! [`Ribbon`] and [`build`] are the state and the geometry, with no `wgpu` in
//! them at all: testable on a machine with no graphics driver. [`Pipeline`] is
//! the other half, and owns everything GPU-shaped.
//!
//! # What is recovered and what is chosen
//!
//! **Recovered**: the segment-count formula
//! (`ceil((6.0/range)*min(distance,range)*6.0)`, already ported to
//! `crates/render/src/exhaust.rs`... no - see
//! `oag_gameplay::projectile::leach_beam` for the weapon's own numbers, this
//! crate only draws), the strip half-width ([`HALF_WIDTH`], `1.0` world unit),
//! the amplitude range ([`AMPLITUDE_MAX`], `[0.0, 2.0]`) and bucket span
//! ([`AMPLITUDE_BUCKET_SPAN`], `3` segments share one value), the base colour
//! (opaque white), the disconnect fade (linear over
//! [`oag_gameplay::projectile::leach_beam::DISCONNECT_LINGER_SECONDS`]), the
//! zero-alpha taper at both chain endpoints, the crossed-double-strip
//! structure (two ribbons sharing one spine, submitted as one triangle strip),
//! and the blend - identical to [`crate::exhaust::BLEND`]
//! (`Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0, 0xffffff)`, the same call
//! `exhaust::BLEND`'s own doc comment cites, so this reuses that constant
//! rather than re-deriving it).
//!
//! **Chosen, no confidence score, and why**:
//!
//! - **The two axes the strips displace along.** The original crosses them
//!   along two axes read off the craft's own scene-graph node basis, which
//!   this engine's [`oag_gameplay::world::Ship`] has no equivalent of - it
//!   carries a physics body, not a `.vex` node chain. [`build`] instead picks
//!   an arbitrary pair perpendicular to the owner-target line and to each
//!   other, stable for a given direction so the ribbon does not swim as the
//!   beam turns.
//! - **The amplitude re-roll cadence.** The original re-rolls one bucket
//!   whenever the ribbon's own scroll cursor wraps, "roughly once a second" -
//!   the same cursor `oag_gameplay::projectile::leach_beam`'s own doc comment
//!   already declines to model, because reproducing it needs render geometry
//!   the simulation crate must not carry. [`Ribbon::advance`] re-rolls one
//!   bucket on a fixed one-second timer instead, off its own `rng` - never
//!   `World::rng`, the same rule [`crate::exhaust::Exhaust::advance`]'s own
//!   `rng` parameter follows.
//!
//! Nothing here is invented in place of unread data: the picture drawn is
//! `Data\Weapons\Textures\pulse_leechbeam1_ADD.mip`, the disc's own texture,
//! and every number that shapes the strip but the two above is a recovered
//! constant.

use oag_core::{Rng, math::Vec3};

use crate::mesh::GpuVertex;

/// Half the strip's own width - `LeachBeam_InitLocked` copies this from
/// `DAT_08a7cc04`, read directly as `0x3f800000` = `1.0`. Confidence **90**.
pub const HALF_WIDTH: f32 = 1.0;

/// Entries in the amplitude table `LeachBeam_InitLocked` fills at
/// `instance+0xb4`, each an independent draw at construction. Confidence
/// **88** - a decompiled fixed-count loop.
pub const AMPLITUDE_BUCKETS: usize = 12;

/// Upper bound of `Psys_RandFloatRange(0.0, 2.0)`, the draw every amplitude
/// bucket uses - `0x40000000` is `2.0f`. Confidence **88**.
pub const AMPLITUDE_MAX: f32 = 2.0;

/// Consecutive chain segments that share one amplitude bucket -
/// `DAT_08a7cc00`, read directly as `0x40400000` = `3.0`. Confidence **90**.
pub const AMPLITUDE_BUCKET_SPAN: f32 = 3.0;

/// Seconds between amplitude bucket re-rolls. **Chosen, not measured** - see
/// the module doc comment's second bullet.
pub const AMPLITUDE_REROLL_SECONDS: f32 = 1.0;

/// The scroll phase's own rate: `LeachBeam_InitLocked` sets `instance+0x1188`
/// (the rate) to `1.0`, and `LeachBeam_Advance` advances the phase by
/// `rate * dt * 2.0` every tick before wrapping it into `[0, 1)`. Folding the
/// recovered `2.0` into this constant keeps [`Ribbon::advance`] a single
/// multiply. Confidence **80**: the rate field's own use beyond this one
/// tick's advance was not chased, so a title that changes it would not be
/// caught by this constant.
pub const SCROLL_RATE: f32 = 2.0;

/// Segments a beam this long draws, from `LeachBeam_Advance`'s own formula.
/// Confidence **90** - already ported once, in
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s
/// `LeachBeam_Advance` reading.
#[must_use]
pub fn segment_count(distance: f32, range: f32) -> u32 {
    if range <= 0.0 {
        return 1;
    }
    let clamped = distance.min(range);
    (((6.0 / range) * clamped * 6.0).ceil() as u32).max(1)
}

/// Generous headroom over the largest `segment_count` a `250`-unit
/// (Race-table) or authored range can reach: at `distance >= range` the
/// formula is range-independent and caps at `36`. `40` leaves margin for a
/// title that authors a shorter range without changing this constant.
pub const MAX_SEGMENTS: u32 = 40;

/// Vertices [`build`] can ever emit: two crossed strips, `segments + 1`
/// pairs each.
pub const MAX_VERTICES: usize = 4 * (MAX_SEGMENTS as usize + 1);

/// The ribbon's own render-side state: the amplitude table and the scroll
/// phase, neither of which [`oag_gameplay::projectile::leach_beam::Beam`]
/// carries - see the module doc comment's "chosen" section for why.
#[derive(Debug, Clone)]
pub struct Ribbon {
    amplitudes: [f32; AMPLITUDE_BUCKETS],
    /// `0..1`, wrapped every tick - the original's `instance+0x1180`.
    scroll_phase: f32,
    /// Seconds until the next single-bucket re-roll.
    reroll_timer: f32,
    /// Which bucket the next re-roll touches - round-robin, since the
    /// original's own cursor-driven index is not reproduced (see module doc).
    next_bucket: usize,
}

impl Ribbon {
    /// A freshly fired beam's ribbon - every bucket drawn once, matching
    /// `LeachBeam_InitLocked`'s own construction-time loop.
    #[must_use]
    pub fn new(rng: &mut Rng) -> Self {
        let mut amplitudes = [0.0; AMPLITUDE_BUCKETS];
        for amplitude in &mut amplitudes {
            *amplitude = rng.next_f32() * AMPLITUDE_MAX;
        }
        Self {
            amplitudes,
            scroll_phase: 0.0,
            reroll_timer: AMPLITUDE_REROLL_SECONDS,
            next_bucket: 0,
        }
    }

    /// One tick: scrolls the wiggle and, on the chosen cadence, re-rolls one
    /// amplitude bucket. `rng` is render-side, never `World::rng` - see the
    /// module doc comment.
    pub fn advance(&mut self, dt: f32, rng: &mut Rng) {
        self.scroll_phase = (self.scroll_phase + dt * SCROLL_RATE).rem_euclid(1.0);
        self.reroll_timer -= dt;
        if self.reroll_timer <= 0.0 {
            self.reroll_timer += AMPLITUDE_REROLL_SECONDS;
            self.amplitudes[self.next_bucket] = rng.next_f32() * AMPLITUDE_MAX;
            self.next_bucket = (self.next_bucket + 1) % AMPLITUDE_BUCKETS;
        }
    }
}

/// A stable pair of axes perpendicular to `forward` and to each other.
///
/// **Chosen, not measured** - see the module doc comment. `Vec3::Y` is the
/// reference axis unless `forward` runs nearly parallel to it, in which case
/// `Vec3::X` takes over, so the pair never degenerates for a beam fired
/// straight up or down.
fn cross_axes(forward: Vec3) -> (Vec3, Vec3) {
    let reference = if forward.y.abs() > 0.95 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let a = forward.cross(reference).normalize_or_zero();
    let b = forward.cross(a).normalize_or_zero();
    (a, b)
}

/// Builds the ribbon's geometry for one tick, as a flat vertex list ready for
/// [`Pipeline::upload`] - one `GU_TRIANGLE_STRIP`-shaped `TriangleStrip`, the
/// same primitive `LeachBeam_SubmitStrip` draws.
///
/// `owner`/`target` are the two craft's current world positions, `range` is
/// [`oag_gameplay::projectile::leach_beam::Beam::range`], and `alpha` is the
/// link's own coverage this tick - `1.0` while connected, ramping to `0.0`
/// over the disconnect linger, computed by the caller from
/// [`oag_gameplay::projectile::leach_beam::Beam::disconnected_at`] the same
/// way `LeachBeam_BuildStrip`'s own alpha branch does.
///
/// Empty when the two craft coincide (nothing to draw a direction from) or
/// `range` is non-positive.
#[must_use]
pub fn build(ribbon: &Ribbon, owner: Vec3, target: Vec3, range: f32, alpha: f32) -> Vec<GpuVertex> {
    let separation = target - owner;
    let distance = separation.length();
    if distance < 1e-4 {
        return Vec::new();
    }
    let forward = separation / distance;
    let segments = segment_count(distance, range).min(MAX_SEGMENTS) as usize;
    let (axis_a, axis_b) = cross_axes(forward);

    let mut vertices = Vec::with_capacity(4 * (segments + 1));
    for axis in [axis_a, axis_b] {
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let base = owner + separation * t;
            let bucket = ((i as f32) / AMPLITUDE_BUCKET_SPAN).ceil() as usize;
            let amplitude = ribbon.amplitudes[bucket.min(AMPLITUDE_BUCKETS - 1)];
            let angular_step = std::f32::consts::TAU / segments as f32 * ribbon.scroll_phase;
            let wiggle = ((i as f32 + 1.0) * angular_step).sin() * amplitude;
            let point = base + axis * wiggle;

            // The original forces both chain endpoints to alpha zero
            // regardless of the connected/disconnected branch - see the
            // "colour write is the disconnect fade" evidence on the docs
            // page. Under the additive blend this hides the seam where the
            // two crossed strips meet in one triangle strip.
            let vertex_alpha = if i == 0 || i == segments { 0.0 } else { alpha };
            let colour = [1.0, 1.0, 1.0, vertex_alpha];

            vertices.push(rib_vertex(point - axis * HALF_WIDTH, colour, 0.0, t));
            vertices.push(rib_vertex(point + axis * HALF_WIDTH, colour, 1.0, t));
        }
    }
    vertices
}

/// One ribbon vertex. `u` is the UV column `LeachBeam_InitLocked`'s
/// alternating zero-fill leaves at each rail (`0.0`/`1.0`, across the strip's
/// width); `v` runs `0..1` along the chain, this build's own choice of how to
/// fill the coordinate the original's own displacement writer left alone.
fn rib_vertex(p: Vec3, colour: [f32; 4], u: f32, v: f32) -> GpuVertex {
    GpuVertex {
        position: p.to_array(),
        normal: [0.0, 0.0, 1.0],
        colour,
        texcoord: [u, v],
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    }
}

pub mod pipeline;
pub use pipeline::Pipeline;

#[cfg(test)]
mod tests;
