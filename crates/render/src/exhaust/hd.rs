//! Wipeout HD's engine trail: a three-fin tube extruded over a 54-sample
//! history, recovered from the running game rather than from a preset.
//!
//! HD builds its ribbon nothing like the PSP's `Trail_InitPreset` path this
//! crate's [`super::Exhaust`] reproduces: `TrailEffectManager` keeps a ring of
//! 54 `{orientation basis, position, colour}` samples per craft (0x50 bytes
//! each) and an SPU job extrudes 324 vertices - three full-width fins, two
//! edge vertices per fin per ring - which RSX draws with 954 indices through
//! `hd_enginetrail_bluered.rcsmaterial`. Every constant below was measured
//! from the live game's own buffers (RPCS3, GDB stub, 2026-08-24) or read
//! from the disc's `Data/ships/shipeffectstweaks.txt`, which names the
//! runtime's tuning block field for field. Evidence and method:
//! `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`.
//!
//! The one deliberate divergence: the original pushes one sample per rendered
//! frame at a variable timestep, this engine one per fixed 60 Hz tick
//! ([ADR-0007]) - so the ring spans exactly 0.9 s here and a frame-rate-
//! dependent span there.
//!
//! [ADR-0007]: https://docs.rs/

use oag_core::math::Vec3;

use crate::mesh::GpuVertex;

/// Samples in the history ring.
///
/// Measured twice over: the ring is 54 records of 0x50 bytes in the manager's
/// per-trail block, and the output buffer the manager registers is 0x2d90
/// bytes = 54 rings x 3 fins x 2 edges x 36-byte vertices.
pub const SAMPLES: usize = 54;

/// Half-width of each fin in world units.
///
/// Every edge vertex in the dumped buffers sits exactly 0.5 from the ring
/// centre, at the head and at the tail alike - the fin width does not taper.
pub const FIN_HALF_WIDTH: f32 = 0.5;

/// Fins per ring, each spanning the full diameter through the trail line.
///
/// The dumped cross-section is three flat strips 60 degrees apart - fin 0
/// along the sample's up axis, fins 1 and 2 rotated +-60 degrees about the
/// segment direction - not a closed prism: each fin's two edges are
/// diametrically opposite. The PSP ribbon is the same idea with four fins at
/// 90 degrees.
pub const FINS: usize = 3;

/// The angle between fins, in radians.
pub const FIN_STEP: f32 = std::f32::consts::PI / 3.0;

/// `u` shrink per ring, as a fraction of the head's own `u`.
///
/// The dumped vertex `u` runs `u_head * (1 - k * 4/54)` for ring `k` - a fit
/// exact to four decimals on six buffers across three craft at three speeds.
/// The head `u` itself carries the stretch (see [`u_head`]), so the noise
/// streaks elongate as the craft speeds up.
pub const U_FALLOFF_PER_RING: f32 = 4.0 / SAMPLES as f32;

/// `Tex UScale Max` from `Data/ships/shipeffectstweaks.txt`.
///
/// The head `u` is `1 - this * speed01`: 0.6 authored, so `u` spans the whole
/// texture once at rest and 40 % of it at full speed.
pub const TEX_USCALE_MAX: f32 = 0.6;

/// `Tex Scroll Speed Delta Min` / `Max`: the wrapping scroll phase advances
/// by this much **per call** - the original adds it per rendered frame with
/// no `dt`, exactly like the PSP's per-call boost decay, and this engine adds
/// it per 60 Hz tick.
pub const SCROLL_DELTA: (f32, f32) = (0.08, 0.1);

/// `Tex Scroll Speed Ship Range`: the km/h that counts as `speed01 = 1`.
///
/// `Ship Min` is authored 0.0 and is folded in here as the absent term.
pub const SPEED01_RANGE_KMH: f32 = 1000.0;

/// `Tex Scroll Speed Thrust Contrib`: the throttle's additive share of
/// `speed01`.
pub const SPEED01_THRUST_CONTRIB: f32 = 0.25;

/// Rings over which the vertex colour fades from white to pure red.
///
/// Measured: green and blue fall linearly from 255 at the head to 0 at ring
/// 27 - half the ring - and stay 0 to the tail. Red never moves. Under the
/// material's own blue-red lerp this deepens a Fury craft's red trail toward
/// the tail and darkens a classic craft's blue one through violet; both fall
/// out of the same multiply.
pub const TINT_FADE_RINGS: f32 = 27.0;

/// Rings over which the vertex alpha attacks from 0 at the nozzle to full.
///
/// Measured as `min(k * 10/54, 1)`: the ramp spans 5.4 rings. The alpha then
/// falls linearly to 0 at the tail (`1 - k/54`), and the whole curve scales
/// by the flare's brightness - `intensity * speed_ramp`, the product HD's
/// `EngineFlare_Update` writes into the trail block at `+0x11d8` every frame.
pub const ALPHA_ATTACK_PER_RING: f32 = 10.0 / SAMPLES as f32;

/// One history sample: where the nozzle was and which way the craft's up
/// pointed when it was laid down.
///
/// The original stores the full orientation basis; the extrusion needs the up
/// axis (fin 0's direction, and the reference the other two fins rotate from)
/// and takes the tangent from neighbouring positions. Keeping only `up` is a
/// representation choice, not a divergence - the reconstruction below matches
/// the dumped vertices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub position: Vec3,
    pub up: Vec3,
}

/// One craft's trail state: the ring plus the wrapping scroll phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tube {
    ring: [Sample; SAMPLES],
    len: usize,
    write: usize,
    /// The `TrailSpeed` phase - `EngineFlare_PlaceShapes`' wrapping
    /// accumulator, patched into the material's `TrailSpeed` slot and added
    /// to `u` by both texture lookups. Baked into the vertex `u` here, which
    /// lands it in both lookups exactly as the original's vertex program
    /// (`c[210]`) and fragment patch (slot 0x3) do.
    phase: f32,
}

impl Default for Tube {
    fn default() -> Self {
        Self::new()
    }
}

impl Tube {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ring: [Sample {
                position: Vec3::ZERO,
                up: Vec3::ZERO,
            }; SAMPLES],
            len: 0,
            write: 0,
            phase: 0.0,
        }
    }

    /// Records one sample and advances the scroll phase, once per tick.
    ///
    /// `speed01` is [`speed01`]'s value for this tick; the phase advances by
    /// `lerp(0.08, 0.1, speed01)` per call, as the original does per frame.
    pub fn push(&mut self, position: Vec3, up: Vec3, speed01: f32) {
        self.ring[self.write] = Sample { position, up };
        self.write = (self.write + 1) % SAMPLES;
        self.len = (self.len + 1).min(SAMPLES);
        let delta = SCROLL_DELTA.0 + (SCROLL_DELTA.1 - SCROLL_DELTA.0) * speed01;
        self.phase = (self.phase + delta).rem_euclid(1.0);
    }

    /// Clears the history for a race start or respawn, keeping the phase -
    /// the original's accumulator lives on the flare and survives a respawn.
    pub fn clear(&mut self) {
        self.len = 0;
        self.write = 0;
    }

    /// Whether there is a full ring to extrude.
    ///
    /// The original's SPU job always runs; what it extrudes from a part-empty
    /// ring at a race start has not been read, so this engine reuses the
    /// PSP ribbon's own recovered gate - draw nothing until the ring fills -
    /// rather than inventing a partial-trail look.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.len >= SAMPLES
    }

    /// The samples, newest first.
    fn samples(&self) -> [Sample; SAMPLES] {
        let mut out = self.ring;
        for (k, slot) in out.iter_mut().enumerate() {
            let i = (self.write + SAMPLES - 1 - k) % SAMPLES;
            *slot = self.ring[i];
        }
        out
    }

    /// Extrudes the tube: 53 segments x 3 fins x 6 vertices = 954, the
    /// original's own index count.
    ///
    /// `brightness` is `intensity * speed_ramp` (see
    /// [`ALPHA_ATTACK_PER_RING`]), `speed01` scales the `u` stretch, and
    /// `red_mix` is 1.0 on a Fury-skinned craft (`concept1`, `nitro`,
    /// `detonator`, `chrome_c1` - the strings `Ship_SetFurySkinFlag_q`
    /// compares) and 0.0 otherwise; it rides the vertex `lit` slot into the
    /// shader, which lerps the blue texture's colour toward the red one's
    /// with it.
    #[must_use]
    pub fn vertices(&self, brightness: f32, speed01: f32, red_mix: f32) -> Vec<GpuVertex> {
        if !self.ready() {
            return Vec::new();
        }
        let samples = self.samples();
        let head_u = u_head(speed01);

        // Per-ring attributes, shared by the three fins.
        let ring_u = |k: usize| head_u * (1.0 - k as f32 * U_FALLOFF_PER_RING) + self.phase;
        let ring_alpha = |k: usize| {
            let attack = (k as f32 * ALPHA_ATTACK_PER_RING).min(1.0);
            let fall = 1.0 - k as f32 / SAMPLES as f32;
            brightness * attack * fall
        };
        let ring_tint = |k: usize| (1.0 - k as f32 / TINT_FADE_RINGS).max(0.0);
        // The segment tangent: central difference inside the ring, one-sided
        // at its ends, matching the dumped normals' behaviour where samples
        // bunch up (they go degenerate rather than flip).
        let tangent = |k: usize| {
            let ahead = &samples[k.saturating_sub(1)];
            let behind = &samples[(k + 1).min(SAMPLES - 1)];
            (ahead.position - behind.position).normalize_or_zero()
        };

        let mut out = Vec::with_capacity(VERTICES_PER_CRAFT);
        for fin in 0..FINS {
            // Fin 0 along the sample's up; 1 and 2 rotated +-60 degrees
            // about the tangent. `fin as f32 - 1.0` orders them -60, 0, +60,
            // which only relabels the dumped 0, +60, -60.
            let angle = (fin as f32 - 1.0) * FIN_STEP;
            let (sin, cos) = angle.sin_cos();
            for k in 0..SAMPLES - 1 {
                let corner = |k: usize, edge: f32| {
                    let s = &samples[k];
                    let d = tangent(k);
                    // Rodrigues' rotation of `up` about `d`, then the edge
                    // offset along it.
                    let side = d.cross(s.up);
                    let fin_dir = (s.up * cos + side * sin).normalize_or_zero();
                    let tint = ring_tint(k);
                    GpuVertex {
                        position: (s.position + fin_dir * (edge * FIN_HALF_WIDTH)).to_array(),
                        normal: d.cross(fin_dir).to_array(),
                        colour: [1.0, tint, tint, ring_alpha(k)],
                        texcoord: [ring_u(k), (edge + 1.0) * 0.5],
                        lit: red_mix,
                        ..bytemuck::Zeroable::zeroed()
                    }
                };
                let a0 = corner(k, -1.0);
                let a1 = corner(k, 1.0);
                let b0 = corner(k + 1, -1.0);
                let b1 = corner(k + 1, 1.0);
                out.extend_from_slice(&[a0, a1, b0, a1, b1, b0]);
            }
        }
        out
    }
}

#[cfg(test)]
impl Tube {
    pub(crate) fn phase_for_tests(&self) -> f32 {
        self.phase
    }
}

/// The head vertex's `u`: `1 - 0.6 * speed01`.
///
/// Not a scroll - at a steady speed the head `u` is a constant and the
/// pattern's apparent motion is all in the wrapping phase and the noise
/// displacement. Measured equal (to four decimals) to the value HD's flare
/// writes into the trail block at `+0x11ec` on three craft in two frames.
#[must_use]
pub fn u_head(speed01: f32) -> f32 {
    1.0 - TEX_USCALE_MAX * speed01
}

/// The normalised speed the scroll and stretch ride on.
///
/// `(speed_kmh - Ship Min) / Ship Range + thrust * Thrust Contrib`, clamped -
/// the authored `Ship Min` is 0. `speed_kmh` is this engine's own km/h
/// (`Exhaust::speed_kmh`); HD's field reads in the same hundreds-of-km/h
/// scale its HUD shows.
#[must_use]
pub fn speed01(speed_kmh: f32, thrust: f32) -> f32 {
    (speed_kmh / SPEED01_RANGE_KMH + thrust * SPEED01_THRUST_CONTRIB).clamp(0.0, 1.0)
}

/// One craft's vertex budget: 53 segments x 3 fins x 6 vertices.
pub const VERTICES_PER_CRAFT: usize = (SAMPLES - 1) * FINS * 6;

/// `Thrust Chase Rate`: the flame's lag filters step by this per 120 Hz
/// substep - `EngineFlare_Update` runs `dt * 60` substeps of two filters, and
/// the boost blend's decay is `* (1 - rate)` per substep, which is the
/// measured `0.8 ^ (2k)` sequence.
pub const THRUST_CHASE_RATE: f32 = 0.2;

/// `Thrust Min/Max Scale XY`: the flame's cross-section scale at zero and
/// full smoothed throttle.
pub const THRUST_SCALE_XY: (f32, f32) = (1.0, 1.5);

/// `Thrust Min/Max Scale Z`: the flame's length at zero and full smoothed
/// throttle - a quarter-length idle flame.
pub const THRUST_SCALE_Z: (f32, f32) = (0.25, 1.5);

/// `Thrust Extra Boost Scale XY` / `Z`: what the boost blend adds on top of
/// the throttle scale, and the whole of the `EF_Boost` shapes' length -
/// `EngineFlare_PlaceShapes` scales the plume's Z by `blend * 2.0` with no
/// visibility branch at all, so the plume grows out of the nozzle and
/// collapses back rather than blinking.
pub const BOOST_EXTRA_XY: f32 = 1.4;
/// See [`BOOST_EXTRA_XY`].
pub const BOOST_EXTRA_Z: f32 = 2.0;

/// The flame's per-craft animation state: HD's `EngineFlare` blends.
///
/// Two numbers, from `EngineFlare_Update`/`_PlaceShapes` (0x002a3100 /
/// 0x002a1f00): a smoothed throttle `s` chasing the thrust input, and a boost
/// blend `b` that snaps to 1.0 while the boost timer is above the gate - the
/// same 0.2 the PSP's `BOOST_GATE` recovers, HD's own copy read at
/// `DAT_008b2ff4` - and decays `* (1 - 0.2)` per substep after. The second,
/// slower afterburner blend (`Afterburner Chase Rate` 0.03, scale 0.5) rides
/// a Fury mechanic this simulation does not model yet and is deliberately
/// not reproduced; the load report should say so where this type is used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flame {
    s: f32,
    b: f32,
}

impl Default for Flame {
    fn default() -> Self {
        Self::new()
    }
}

impl Flame {
    #[must_use]
    pub const fn new() -> Self {
        Self { s: 0.0, b: 0.0 }
    }

    /// Advances one 60 Hz tick - two of the original's 120 Hz substeps.
    pub fn advance(&mut self, thrust_on: bool, boosting: bool) {
        let target = if thrust_on { 1.0 } else { 0.0 };
        for _ in 0..2 {
            self.s += (target - self.s) * THRUST_CHASE_RATE;
        }
        if boosting {
            self.b = 1.0;
        } else {
            for _ in 0..2 {
                self.b *= 1.0 - THRUST_CHASE_RATE;
            }
        }
    }

    /// `EF_Main`'s (cross-section, length) scale:
    /// `lerp(min, max, s) + b * extra` on each axis.
    #[must_use]
    pub fn main_scale(&self) -> (f32, f32) {
        let xy = THRUST_SCALE_XY.0
            + (THRUST_SCALE_XY.1 - THRUST_SCALE_XY.0) * self.s
            + self.b * BOOST_EXTRA_XY;
        let z = THRUST_SCALE_Z.0
            + (THRUST_SCALE_Z.1 - THRUST_SCALE_Z.0) * self.s
            + self.b * BOOST_EXTRA_Z;
        (xy, z)
    }

    /// `EF_Boost`'s length scale, `b * 2.0` - its cross-section stays 1.
    #[must_use]
    pub fn boost_scale_z(&self) -> f32 {
        self.b * BOOST_EXTRA_Z
    }

    /// Whether the plume is worth a draw call at all: below a 8-bit quantum
    /// of length nothing of it would survive the additive blend.
    #[must_use]
    pub fn boost_visible(&self) -> bool {
        self.b > 1.0 / 255.0
    }

    /// The boost blend, exposed for tests and the trace capture.
    #[must_use]
    pub fn boost_blend(&self) -> f32 {
        self.b
    }
}

#[cfg(test)]
mod tests;
