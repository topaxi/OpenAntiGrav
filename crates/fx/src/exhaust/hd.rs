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

use oag_mesh::mesh::GpuVertex;

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
/// The dumped vertex `u` runs `u_head * (1 - k * 4/54)` for ring `k`, with no
/// additive term of any kind: the residual over all 54 rings is 2e-6 on the
/// snapshots that hold the block and its vertex buffer from one pause. The
/// head `u` itself carries the stretch (see [`u_head`]), so the noise streaks
/// elongate as the craft speeds up. The scroll is *not* here on the original -
/// see [`Tube::phase`] for where it is and why this engine folds it in anyway.
pub const U_FALLOFF_PER_RING: f32 = 4.0 / SAMPLES as f32;

/// What Wipeout HD plays on a craft that flies through an engine trail.
///
/// `Trail_SpawnHitEffect` (`0x002e3858`) consumes a per-trail "hit a ship"
/// flag the `Trails` SPU job raises, and `Trail_HitShipEffect` (`0x002d9ec0`)
/// spawns this system parented to the nearest of ten hull attachment nodes on
/// the craft involved. `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`.
pub const TRAIL_HITSHIP_EFFECT: &str = "WO_TRAIL_HITSHIP";

/// The Fury-skinned variant of [`TRAIL_HITSHIP_EFFECT`], chosen by the same
/// byte that turns the ribbon red.
///
/// The call site reads `lbz r5, 0x7d2c(r11)` straight into the variant
/// argument, and `craft + 0x7d2c` is what `Ship_SetFuryTrailFlag` writes and
/// `Trail_BuildDrawState` turns into the `engineTrail` colour mix - so the
/// sparks are red exactly when the ribbon is.
pub const TRAIL_HITSHIP_RED_EFFECT: &str = "WO_TRAIL_HITSHIP_RED";

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

/// Rings over which the vertex colour fades from its head tint to its tail tint.
///
/// Measured on the running game (2026-10-10, the SPU's output buffers read
/// for all eight trails): both variants lerp linearly over the first 27 rings
/// and hold the tail value to ring 53. **The ramp depends on the craft's
/// Fury-skin byte** (`craft + 0x7d2c`): overwriting the byte on a live race
/// changed every trail's vertex colours within a second.
pub const TINT_FADE_RINGS: f32 = 27.0;

/// The vertex colour at the nozzle and at ring 27 for a Fury-skinned craft
/// (`craft + 0x7d2c` = 1): white fading to pure red. Measured 255,255,255 to
/// 255,0,0 (u8).
pub const FURY_TINT_HEAD: [f32; 3] = [1.0, 1.0, 1.0];
/// See [`FURY_TINT_HEAD`].
pub const FURY_TINT_TAIL: [f32; 3] = [1.0, 0.0, 0.0];
/// The vertex colour at the nozzle and at ring 27 for a classic craft
/// (`craft + 0x7d2c` = 0): cyan fading to light violet, blue held at full.
/// Measured 63,255,255 to 254,127,255 (u8, ring 27 and every ring after it).
pub const CLASSIC_TINT_HEAD: [f32; 3] = [63.0 / 255.0, 1.0, 1.0];
/// See [`CLASSIC_TINT_HEAD`].
pub const CLASSIC_TINT_TAIL: [f32; 3] = [1.0, 127.0 / 255.0, 1.0];

/// The vertex colour rgb of ring `k` for a craft whose Fury-skin flag is
/// `red_mix` (1.0 Fury, 0.0 classic).
#[must_use]
pub fn tint_at(k: usize, red_mix: f32) -> [f32; 3] {
    let t = (k as f32 / TINT_FADE_RINGS).min(1.0);
    let mut out = [0.0; 3];
    for c in 0..3 {
        let classic = CLASSIC_TINT_HEAD[c] + (CLASSIC_TINT_TAIL[c] - CLASSIC_TINT_HEAD[c]) * t;
        let fury = FURY_TINT_HEAD[c] + (FURY_TINT_TAIL[c] - FURY_TINT_HEAD[c]) * t;
        out[c] = classic + (fury - classic) * red_mix;
    }
    out
}

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
    /// accumulator, living at the trail block's `+0x1210`.
    ///
    /// **The original never puts it in the vertex.** Its SPU job writes
    /// `u_head * (1 - k * 4/54)` and stops; the phase reaches the shader as
    /// the material parameter `TrailSpeed`, whose value *pointer* the trail
    /// manager's constructor binds to `&block[0x1210]` once, so the draw path
    /// reads the live accumulator through it. The vertex program then adds it
    /// to `u` for the noise lookup and the fragment program adds it to the raw
    /// `u` again for the colour lookup - **once on each path, not twice on
    /// one**. Baking it into the ring `u` here feeds both lookups the same
    /// coordinates the original computes, with one addition instead of two.
    /// Measured and read 2026-08-24;
    /// `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`.
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
    ///
    /// **The first push after [`Self::new`] or [`Self::clear`] seeds the whole
    /// ring with that sample.** Measured, not invented: at the earliest moment
    /// the manager exists in a loading race the live ring already holds 54
    /// valid samples bunched within 0.2 world units of the grid slot, and the
    /// SPU extrudes all 324 vertices from it with every vertex alpha at 0 -
    /// the brightness at `+0x11d8` is 0.0 below the speed ramp's floor. There
    /// is no fill gate anywhere: the trail fades in through the brightness
    /// ramp as the bunched ring stretches out (five snapshots, e0..e4 and
    /// r0/r1, 2026-08-24 on engine-trail.md).
    pub fn push(&mut self, position: Vec3, up: Vec3, speed01: f32) {
        if self.len == 0 {
            self.ring = [Sample { position, up }; SAMPLES];
        }
        self.ring[self.write] = Sample { position, up };
        self.write = (self.write + 1) % SAMPLES;
        self.len = (self.len + 1).min(SAMPLES);
        let delta = SCROLL_DELTA.0 + (SCROLL_DELTA.1 - SCROLL_DELTA.0) * speed01;
        self.phase = (self.phase + delta).rem_euclid(1.0);
    }

    /// Forgets the history so the next push re-seeds the ring at the new pose,
    /// keeping the phase - the original's accumulator lives on the flare and
    /// survives a respawn.
    ///
    /// Without it a respawn's ribbon would span the teleport. What the
    /// original does to the ring across a respawn is unread; re-bunching at
    /// the new pose is this engine's approximation, chosen because it is the
    /// same state the measured race start begins from.
    pub fn clear(&mut self) {
        self.len = 0;
        self.write = 0;
    }

    /// Whether the ring has been seeded at all.
    ///
    /// True from the first push on - the seeded ring is always full, which is
    /// what the live game's own ring is from the earliest readable frame of a
    /// race (see [`Self::push`]). False only for a tube nothing ever pushed
    /// to, e.g. a craft whose model has no nozzle locator.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.len >= 1
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

    /// The closest point on this trail's centre line to `point`, and how far
    /// it is - the query a craft-versus-ribbon test needs.
    ///
    /// **The centre line, not the extruded surface.** The three fins all pass
    /// through it and the ribbon reaches [`FIN_HALF_WIDTH`] either side, so a
    /// caller asking "is this craft in the trail" adds that to its own radius
    /// rather than intersecting 954 triangles - which is also the shape the
    /// original's own test has, since the `Trails` SPU job is handed one
    /// bounding-sphere-shaped vec4 per craft and nothing per fin
    /// (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`).
    ///
    /// Segment-wise rather than sample-wise: at race pace the ring spans 50
    /// world units over 54 samples, so testing the stored points alone would
    /// miss a craft sitting between two of them.
    #[must_use]
    pub fn nearest(&self, point: Vec3) -> Option<(Vec3, f32)> {
        if !self.ready() {
            return None;
        }
        let samples = self.samples();
        let mut best: Option<(Vec3, f32)> = None;
        for pair in samples.windows(2) {
            let (a, b) = (pair[0].position, pair[1].position);
            let span = b - a;
            let length_squared = span.length_squared();
            // A degenerate segment - which every one of them is at a race
            // start, where the whole ring sits inside 0.2 units - collapses to
            // its own endpoint rather than dividing by zero.
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
        best
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
        let mut out = Vec::with_capacity(VERTICES_PER_CRAFT);
        self.extend_vertices(&mut out, brightness, speed01, red_mix);
        out
    }

    /// [`Self::vertices`], appended to a list the caller owns - the form the
    /// renderer uses, for the reason
    /// [`crate::exhaust::Exhaust::extend_trail_vertices`] gives.
    pub fn extend_vertices(
        &self,
        out: &mut Vec<GpuVertex>,
        brightness: f32,
        speed01: f32,
        red_mix: f32,
    ) {
        if !self.ready() {
            return;
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
        let ring_tint = |k: usize| tint_at(k, red_mix);
        // The segment tangent: central difference inside the ring, one-sided
        // at its ends, matching the dumped normals' behaviour where samples
        // bunch up (they go degenerate rather than flip).
        let tangent = |k: usize| {
            let ahead = &samples[k.saturating_sub(1)];
            let behind = &samples[(k + 1).min(SAMPLES - 1)];
            (ahead.position - behind.position).normalize_or_zero()
        };

        out.reserve(VERTICES_PER_CRAFT);
        for fin in 0..FINS {
            // Fins at 0, 60 and 120 degrees from the sample's up, rotating
            // toward `side` - as *lines* the same set as "up and +-60", but
            // the directions fix the single-sided normals, and those decide
            // everything under the facing fade. The dumped normals are
            // `cross(back, fin_dir)` with `back` the older-minus-newer
            // tangent: `(0, 0.87, -0.5)` and `(0, 0.87, +0.5)` for the two
            // off-vertical fins - BOTH tilted toward the sample's up - and
            // `-side` for the vertical one. Reconstructed the mirror-naive
            // way (fins at -60/0/+60, `cross(forward, fin)`), one of the
            // three fins faces fully away from a chase camera and the trail
            // all but vanishes dead-astern; this exact construction is what
            // keeps two fins lit from above, and it reproduces the dump to
            // three decimals.
            let angle = fin as f32 * FIN_STEP;
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
                        // `cross(back, fin) = cross(fin, forward)`, the
                        // dumped orientation - see the fin comment above.
                        normal: fin_dir.cross(d).to_array(),
                        colour: [tint[0], tint[1], tint[2], ring_alpha(k)],
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
/// writes into the trail block at `+0x11ec`, on three craft in two frames and
/// again on seven race-start snapshots - where the head vertex's `u` is this
/// value and *not* this value plus the phase, which is what says the original
/// applies the scroll at draw time rather than in the extruded vertex.
#[must_use]
pub fn u_head(speed01: f32) -> f32 {
    1.0 - TEX_USCALE_MAX * speed01
}

/// What multiplies this engine's km/h into the flare's speed input (`+0x104`).
///
/// Re-exported from [`oag_core::math`] rather than defined here: `oag_sound`
/// feeds the same field to HD's crossfaded engine and cannot depend on the
/// renderer. The measurement is recorded on the definition.
pub use oag_core::math::SPEED_FIELD_GAIN;

/// The normalised speed the scroll and stretch ride on.
///
/// `(field - Ship Min) / Ship Range + thrust * Thrust Contrib`, clamped -
/// the authored `Ship Min` is 0 and the field is the craft's km/h times
/// [`SPEED_FIELD_GAIN`], so `speed01` saturates near 500 km/h at full
/// throttle.
#[must_use]
pub fn speed01(speed_kmh: f32, thrust: f32) -> f32 {
    (speed_kmh * SPEED_FIELD_GAIN / SPEED01_RANGE_KMH + thrust * SPEED01_THRUST_CONTRIB)
        .clamp(0.0, 1.0)
}

/// The trail-brightness speed ramp, HD's own: `(field - 100) / 500` clamped,
/// with the field again km/h times [`SPEED_FIELD_GAIN`].
///
/// The *shape* is Pulse's recovered ramp constant for constant (floor 100,
/// span 500, from `0x008b3010`/`0x008b3004`), but the gained field saturates
/// it near **400 km/h** - the live fields sat at 0.58..0.98 through an AI
/// race, so a craft at pace trails at or near full brightness where Pulse's
/// own ramp is still climbing at 600. `Exhaust::speed_ramp` stays Pulse's;
/// this is the HD consumer's.
#[must_use]
pub fn speed_ramp(speed_kmh: f32) -> f32 {
    ((speed_kmh * SPEED_FIELD_GAIN - 100.0) / 500.0).clamp(0.0, 1.0)
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

/// `Flare Radius` from `Data/ships/shipeffectstweaks.txt`: the sprite
/// flare's fade-scaled half-height term, in the same model space every other
/// per-craft tuning number in this file lives in - **not** world units on its
/// own.
///
/// The sprite is real and always built: `Enable Flare Sprite` is authored 1,
/// the flare's own init (`0x002a1528`) loads
/// `Data/Tex/EngineFlare/Engine_Flare_Rich.gtf` and four corner pairs, and
/// `EngineFlare_RenderTick` (`0x002a08a8`) builds the quad every frame - the
/// law is [`Sprite::half_height`]'s. A caller placing the sprite in world
/// space still owes it the craft's own global scale
/// (`crate::exhaust::CRAFT_ROW_SCALE`, the same factor `model_matrix_of`
/// applies to the sprite's own position) - see `race::effects::hd_sprite_quad`
/// and `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md` ("Eighth
/// session") for why an unscaled 3.0 draws roughly twice the footprint a
/// pixel-diff isolation measures for it.
pub const SPRITE_RADIUS: f32 = 3.0;

/// `Flare Radius Min`: the half-height's base term - what a fully faded
/// sprite still measures, not a floor under a jitter.
pub const SPRITE_RADIUS_MIN: f32 = 2.0;

/// `Max Radius Jitter`: the span of the half-height's one-sided random term.
///
/// `fmadds f13,f21,f13,f19` at `0x002a0d94` is `Max Radius Jitter *
/// this[0x25c] + Flare Radius Min`, `+0x25c` a per-instance 0..1 - so the
/// jitter only ever adds. Read 2026-09-15, engine-trail.md "Ninth session",
/// confidence 88.
pub const SPRITE_RADIUS_JITTER: f32 = 0.5;

/// `Slow Alpha Noise Min` / `Max`: the band the sprite's opacity wanders in.
pub const SPRITE_ALPHA_NOISE: (f32, f32) = (0.5, 0.8);

/// `Slow Alpha Noise Chase Speed`: the per-tick lerp toward the target.
pub const SPRITE_ALPHA_CHASE: f32 = 0.1;

/// `Slow Alpha Noise Timer`: ticks between picking a new target.
pub const SPRITE_ALPHA_RETARGET: u32 = 10;

/// `Flare Opacity Max`: the ceiling on the fade.
pub const SPRITE_OPACITY_MAX: f32 = 1.0;

/// `Flare Fadeout Dist`: the scaled camera distance the fade starts at.
pub const SPRITE_FADEOUT_DIST: f32 = 15.0;

/// `Flare Fadeout Range`: how much further the fade takes to reach zero.
pub const SPRITE_FADEOUT_RANGE: f32 = 15.0;

/// `Flare Highlight Power`: the exponent on the view dot - `cos^32`, a lobe
/// a few degrees wide around the nozzle axis.
pub const SPRITE_HIGHLIGHT_POWER: f32 = 32.0;

/// `Flare Highlight Boost`: the multiplier on the walked alpha.
pub const SPRITE_HIGHLIGHT_BOOST: f32 = 1.0;

/// Half-width over half-height: the `4.0` at `0x008b2f7c` that
/// `EngineFlare_RenderTick` scales the camera's row-0 axis by and nothing
/// else. It is the texture's own shape - `Engine_Flare_Rich.gtf` is 1024 x
/// 256 - so the quad keeps the texel aspect rather than squashing a streak
/// into a square.
pub const SPRITE_ASPECT: f32 = 4.0;

/// What multiplies the camera distance before the fadeout compares it.
///
/// **Chosen, not measured.** The original reads this off a per-view table -
/// `((float *)*(r2+0x5aa0))[max(view_index, 0)]`, one entry per split-screen
/// view - and nothing has read that table; its value is open on
/// engine-trail.md. `1.0` is the identity, which leaves the term exactly as
/// the decompile writes it in world units, and it is the value the seventh
/// session's own reasoning already assumed ("at chase range the distance
/// term is 1.0" holds for a scale near 1 and not otherwise).
pub const SPRITE_DISTANCE_SCALE: f32 = 1.0;

/// The sprite flare's per-craft state: the slow alpha walk and this tick's
/// jitter draw, both from the authored constants above. The fade and the
/// quad size are [`Self::fade`] and [`Self::half_height`], the law
/// `EngineFlare_RenderTick` (`0x002a08a8`) was read to hold on 2026-09-15
/// (engine-trail.md "Ninth session", confidence 88 for the quad and 90 for
/// the fade).
///
/// Read and deliberately **not** implemented, said here so the absence is a
/// decision: the `Max Chromatic Dispersion` fringe (a scalar handed to the
/// four-vertex submit, `FUN_002c4ad0`, which is unread), and the `Flare
/// Occluder Radius` z-pass occlusion query the original runs two frames
/// ahead of the draw - this engine has no readback path for it, so an
/// opponent's sprite shows through its own hull here where the original's
/// would not. `Flare Max Rotate Angle`, `Flare Size Clamp` and `Flare Depth
/// Bias` are never loaded by the draw at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    alpha: f32,
    target: f32,
    ticks: u32,
    jitter: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new()
    }
}

impl Sprite {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alpha: SPRITE_ALPHA_NOISE.0,
            target: SPRITE_ALPHA_NOISE.1,
            ticks: 0,
            jitter: 0.0,
        }
    }

    /// Advances one tick: re-targets the alpha walk every
    /// [`SPRITE_ALPHA_RETARGET`] ticks, chases it, and draws the jitter.
    ///
    /// The walk is the original's: the countdown at `+0x190` reloads from
    /// `Slow Alpha Noise Timer`, re-targets `+0x198 = lerp(Min, Max, rand)`,
    /// and `+0x194` chases it - measured cycling 10..1 on every AI flare live
    /// (engine-trail.md "Tenth session"). The jitter is re-drawn every tick;
    /// the original keeps it at `+0x25c` and how often that word is rewritten
    /// is not read, so the cadence is this engine's.
    ///
    /// `next` supplies uniform randoms in `0..1` - the caller's seeded
    /// per-craft stream, never OS entropy.
    pub fn advance(&mut self, mut next: impl FnMut() -> f32) {
        if self.ticks == 0 {
            self.ticks = SPRITE_ALPHA_RETARGET;
            let (lo, hi) = SPRITE_ALPHA_NOISE;
            self.target = lo + (hi - lo) * next();
        }
        self.ticks -= 1;
        self.alpha += (self.target - self.alpha) * SPRITE_ALPHA_CHASE;
        self.jitter = next();
    }

    /// The walked alpha, `+0x194` - one factor of [`Self::fade`], never the
    /// vertex alpha on its own.
    #[must_use]
    pub fn alpha_walk(&self) -> f32 {
        self.alpha
    }

    /// This tick's jitter draw in `0..1`.
    #[must_use]
    pub fn jitter(&self) -> f32 {
        self.jitter
    }

    /// The sprite's alpha this frame, `+0x18c`, or `None` when the original
    /// draws no quad at all.
    ///
    /// `distance` is from the camera to the flare in world units and
    /// `view_dot` the cosine between the camera's forward axis and the
    /// nozzle's outward axis, as [`sprite_view_dot`] defines it. Two
    /// early-outs, both the original's: a non-positive dot leaves before any
    /// of the fade math (`0x002a0c8c: ble -> epilogue`), so the sprite is a
    /// one-hemisphere lobe; and a fade that is not positive leaves right
    /// after its store (`stfs f13,0x18c(r31)` and its own `ble`), before the
    /// quad at `0x002a0d78` is built - measured live, the submit is reached
    /// only while the fade is positive. Past the fadeout there is no quad.
    #[must_use]
    pub fn fade(&self, distance: f32, view_dot: f32) -> Option<f32> {
        if view_dot <= 0.0 {
            return None;
        }
        let fade = sprite_fade(distance * SPRITE_DISTANCE_SCALE, view_dot, self.alpha);
        (fade > 0.0).then_some(fade)
    }

    /// The quad's half-height for a fade, in the tuning file's model space:
    /// `Min + Radius * clamp(fade, 0, 1) + Jitter * jitter01`.
    #[must_use]
    pub fn half_height(&self, fade: f32) -> f32 {
        sprite_half_height(fade, self.jitter)
    }
}

/// The fade law with the distance already scaled:
/// `min((1 - saturate((d - Dist) / Range)) * dot^Power * walk * Boost, Max)`.
///
/// The decompiler's own expression, confidence 90 (engine-trail.md "Ninth
/// session"): the distance term is 1.0 inside `Flare Fadeout Dist` and
/// reaches 0 at `Dist + Range`, the highlight is `powf(view_dot, 32)`, and
/// `Flare Opacity Max` ceilings the product. The two early-outs - a
/// `view_dot` of zero or less, a result of zero or less - are
/// [`Sprite::fade`]'s, not this function's.
#[must_use]
pub fn sprite_fade(scaled_distance: f32, view_dot: f32, alpha_walk: f32) -> f32 {
    let far = ((scaled_distance - SPRITE_FADEOUT_DIST) / SPRITE_FADEOUT_RANGE).clamp(0.0, 1.0);
    let highlight = view_dot.powf(SPRITE_HIGHLIGHT_POWER);
    ((1.0 - far) * highlight * (alpha_walk * SPRITE_HIGHLIGHT_BOOST)).min(SPRITE_OPACITY_MAX)
}

/// The half-height law, `0x002a0d7c`..`0x002a0dd4`, confidence 88:
/// `Flare Radius Min + Flare Radius * clamp(fade, 0, 1) + Max Radius Jitter *
/// jitter01`. The fade is clamped here, the alpha is not: the vertex takes
/// the stored fade as it is.
#[must_use]
pub fn sprite_half_height(fade: f32, jitter01: f32) -> f32 {
    SPRITE_RADIUS_MIN + SPRITE_RADIUS * fade.clamp(0.0, 1.0) + SPRITE_RADIUS_JITTER * jitter01
}

/// The view dot the highlight is raised to: the cosine between where the
/// camera looks and where the nozzle points.
///
/// The original sums the camera's row-2 axis against the flare node's own Z
/// axis with one side negated, both normalised, and draws only when the
/// result is positive - which hemisphere that is in world terms rests on two
/// sign conventions the page leaves unsettled. **Chosen, not measured**: the
/// positive side here is the camera looking *into* the nozzle - a craft
/// ahead, its exhaust toward the eye - which is the side an engine glow is
/// visible from; `camera_forward` points along the view and `nozzle_axis`
/// out of the nozzle (`-forward` of the craft), so the two oppose when the
/// camera sits behind the craft and the negation makes that `+1`.
#[must_use]
pub fn sprite_view_dot(camera_forward: Vec3, nozzle_axis: Vec3) -> f32 {
    -camera_forward
        .normalize_or_zero()
        .dot(nozzle_axis.normalize_or_zero())
}

/// The sprite's quad: [`SPRITE_ASPECT`] times wider than it is tall, `centre
/// +- right * half_height * 4 +- up * half_height`, the corner rule at
/// `0x002a0df4`..`0x002a0e00`; `alpha` is the fade, which the original stores
/// in the colour word as `0xffffff00 | (fade * 255)`.
#[must_use]
pub fn sprite_quad(
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    half_height: f32,
    alpha: f32,
) -> [GpuVertex; 6] {
    super::sprite::quad(
        centre,
        right * (half_height * SPRITE_ASPECT),
        up * half_height,
        alpha,
    )
}

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
