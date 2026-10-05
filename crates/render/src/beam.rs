//! The LeachBeam's own ribbon: the jagged energy arc drawn between the holder
//! and the craft it has locked onto.
//!
//! Recovered from `LeachBeam_Advance` (`0x08873fa0`), `LeachBeam_InitLocked`
//! (`0x08873d3c`), `LeachBeam_LoadTexture` (`0x088730d4`),
//! `LeachBeam_BuildStrip` (`0x088739b0`), `LeachBeam_SubmitStrip`
//! (`0x088731c4`) and `LeachBeam_MarkPulse` (`0x088732f8`). Addresses,
//! evidence and a confidence score per claim are in
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "2026-09-23:
//! the ribbon re-read, and measured in play"; this module implements what that
//! section describes and cites it rather than restating it.
//!
//! # Two halves, the same split [`crate::exhaust`] uses
//!
//! [`Ribbon`] and [`build`] are the state and the geometry, with no `wgpu` in
//! them at all: testable on a machine with no graphics driver. [`Pipeline`] is
//! the other half, and owns everything GPU-shaped.
//!
//! # The picture, in one paragraph
//!
//! A chain of `segment_count` points runs from the shooter to the target. Every
//! point but the first is pushed off the straight line along **two** axes read
//! off the shooter's own orientation - the first at full amplitude, the second
//! at half - by a sine whose phase is indexed by a per-tick cursor, so the kinks
//! travel down the beam toward the shooter one segment a tick. Two strips are
//! built over that one chain, one widened along the **camera's** right and one
//! along its up, so the arc reads as a line from every angle. The texture's `u`
//! flips `0`/`1` at every chain point, so the static band repeats once per
//! segment, and slides along `u` at two repeats a second.
//!
//! # What is recovered and what is chosen
//!
//! **Recovered**: every number and every rule above, plus the strip
//! half-width ([`HALF_WIDTH`]), the amplitude table ([`AMPLITUDE_BUCKETS`]
//! draws in `[0, `[`AMPLITUDE_MAX`]`]`, [`AMPLITUDE_BUCKET_SPAN`] segments a
//! bucket, and the thirteenth entry the index reaches), the one-bucket re-roll
//! at each pulse, the pulse itself (the cursor wrapping to zero) and the pulse
//! strength that re-arms at most once a second, the endpoint taper and the
//! disconnect fade, the blend and the glow-mask stamp ([`GLOW_MASK`]).
//! Measured in play on PPSSPP too: the cursor steps every frame at 60 Hz, the
//! `WO_LEACHBEAM_ENERGY` respawn fires on every wrap, and the pulse strength
//! re-arms about once a second.
//!
//! **The track tube** (`LeachBeam_KeepInTrack`, `LeachBeam_ReaimChain`): every
//! undisplaced chain point is located on the track and, below the road or
//! within [`tube::EDGE_MARGIN`] of an edge, re-aimed back inside - see
//! [`tube`]. On a straight, or with no track to locate on, the chain is the
//! straight line.
//!
//! **Chosen, no confidence score**: nothing in the geometry. The shooter's
//! basis is the drawn hull's own rotation (the physics body's orientation and
//! its visual roll), which is this engine's equivalent of the scene-graph node
//! the original reads.

use oag_core::{Rng, math::Vec3};

use crate::mesh::GpuVertex;

pub mod tube;
pub use tube::TubeFrame;

/// What places a chain point on the track: the stand-in for
/// `AiTrack_LocatePosition`, supplied by the caller because this crate owns no
/// track. See [`tube`].
pub type Locate<'a> = &'a dyn Fn(Vec3) -> Option<TubeFrame>;

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

/// The second displacement axis' share of its bucket - the literal `0.5`
/// `LeachBeam_Advance` multiplies it by. Confidence **85**.
pub const SECOND_AXIS_SCALE: f32 = 0.5;

/// The scroll phase's own rate: `LeachBeam_InitLocked` sets `instance+0x1188`
/// (the rate) to `1.0`, and `LeachBeam_Advance` advances the phase by
/// `rate * dt * 2.0` every tick before wrapping it into `[0, 1)`.
/// `LeachBeam_SubmitStrip` hands the phase to `Gu_TexOffset` as the `u`
/// offset. Confidence **85**.
pub const SCROLL_RATE: f32 = 2.0;

/// Seconds the pulse strength stays live after a pulse, and the shortest gap
/// between two pulses - `LeachBeam_MarkPulse`'s `> 1.0` and
/// `LeachBeam_PulseStrength`'s `<= 1.0`. Confidence **85**.
pub const PULSE_SECONDS: f32 = 1.0;

/// What the ribbon stamps into the glow mask: `LeachBeam_SubmitStrip`'s
/// `Gu_StencilFunc(GU_ALWAYS, DAT_08ab1078, 0xff)` with
/// `Gu_StencilOp(KEEP, KEEP, REPLACE)`, and `DAT_08ab1078` reads `0x28`. The
/// PSP's stencil is the framebuffer's alpha byte, which is what
/// [`crate::post::bloom`] reads. Confidence **80**: the value and the op are
/// direct reads; that every fragment of the strip writes it, transparent
/// texels included, follows from the alpha test being off in the same
/// function.
pub const GLOW_MASK: f32 = 40.0 / 255.0;

/// Segments a beam this long draws, from `LeachBeam_Advance`'s own formula.
/// Confidence **90**.
#[must_use]
pub fn segment_count(distance: f32, range: f32) -> u32 {
    if range <= 0.0 {
        return 1;
    }
    (((6.0 / range) * distance.min(range) * 6.0).ceil() as u32).max(1)
}

/// Generous headroom over the largest `segment_count` can reach: at
/// `distance >= range` the formula is range-independent and caps at `36`.
pub const MAX_SEGMENTS: u32 = 40;

/// Vertices [`build`] can ever emit: two strips of `segments + 1` pairs -
/// `LeachBeam_SubmitStrip`'s own `(segment_count * 2 + 2) * 2`.
pub const MAX_VERTICES: usize = 4 * (MAX_SEGMENTS as usize + 1);

/// The ribbon's own render-side state: the amplitude table, the scroll phase,
/// the per-tick cursor and the pulse clock, none of which
/// [`oag_weapons::projectile::leach_beam::Beam`] carries - the simulation
/// must not hold render geometry, and this state is driven by it.
#[derive(Debug, Clone)]
pub struct Ribbon {
    /// Twelve draws, then the thirteenth entry the displacement index reaches:
    /// `ceil(35 / 3) = 12` for a full-range beam, which in the original reads
    /// one float past the table - `instance+0xe4`, the half-width. Kept as a
    /// real entry here so the read is in bounds and the value the original
    /// sees.
    amplitudes: [f32; AMPLITUDE_BUCKETS + 1],
    /// `0..1`, `instance+0x1180`.
    scroll_phase: f32,
    /// `instance+0x130`: counts ticks modulo twice the segment count.
    counter: u32,
    /// `instance+0xa8`: the counter modulo the segment count.
    cursor: u32,
    /// The cursor and counter the current geometry was built with.
    drawn: (u32, u32),
    /// `instance+0x134`, the beam's age as the ribbon sees it.
    age: f32,
    /// `instance+0x138`, initialised to `-1.0` by `LeachBeam_InitLocked`.
    last_pulse: f32,
}

impl Ribbon {
    /// A freshly fired beam's ribbon - every bucket drawn once, matching
    /// `LeachBeam_InitLocked`'s own construction-time loop.
    #[must_use]
    pub fn new(rng: &mut Rng) -> Self {
        let mut amplitudes = [HALF_WIDTH; AMPLITUDE_BUCKETS + 1];
        for amplitude in &mut amplitudes[..AMPLITUDE_BUCKETS] {
            *amplitude = rng.next_f32() * AMPLITUDE_MAX;
        }
        Self {
            amplitudes,
            scroll_phase: 0.0,
            counter: 0,
            cursor: 0,
            drawn: (0, 0),
            age: 0.0,
            last_pulse: -1.0,
        }
    }

    /// One tick of `LeachBeam_Advance`, with the two craft `distance` apart.
    ///
    /// Returns whether this tick ran the **pulse block** - the cursor sitting
    /// at zero - which is where the original re-spawns
    /// `WO_LEACHBEAM_ENERGY` and plays `LEACHENERGY`. The pulse strength
    /// ([`Self::pulse_strength`]) re-arms inside the same block, but only once
    /// a second has passed since it last did. `rng` is render-side, never
    /// `World::rng`.
    pub fn advance(&mut self, dt: f32, distance: f32, range: f32, rng: &mut Rng) -> bool {
        self.scroll_phase = (self.scroll_phase + dt * SCROLL_RATE).rem_euclid(1.0);
        self.age += dt;
        let segments = segment_count(distance, range).min(MAX_SEGMENTS);
        let pulsed = self.cursor == 0;
        if pulsed {
            if self.age - self.last_pulse > PULSE_SECONDS {
                self.last_pulse = self.age;
            }
            let bucket = ((segments - 1) as f32 / AMPLITUDE_BUCKET_SPAN).ceil() as usize + 1;
            if bucket < AMPLITUDE_BUCKETS {
                self.amplitudes[bucket] = rng.next_f32() * AMPLITUDE_MAX;
            }
        }
        self.drawn = (self.cursor, self.counter);
        self.counter = (self.counter + 1) % (2 * segments);
        self.cursor = self.counter % segments;
        pulsed
    }

    /// `LeachBeam_PulseStrength` (`0x08873020`): twice the seconds since the
    /// last pulse inside the one-second window, else zero. Written each tick to
    /// the firing craft's weapon record, where the hull overlay reads it - see
    /// [`crate::hull_overlay::pulse`], the same shape.
    #[must_use]
    pub fn pulse_strength(&self) -> Option<f32> {
        crate::hull_overlay::pulse(self.age - self.last_pulse)
    }

    /// Where `WO_LEACHBEAM_ENERGY` sits this tick: the undisplaced chain point
    /// `segment_count - 1 - cursor` (after the track tube has bent it, see
    /// [`tube::walk`]), which `LeachBeam_Advance` writes into the
    /// effect's own matrix (`instance+0x120`) - so the effect starts one
    /// segment short of the target at each pulse and walks back to the
    /// shooter one segment a tick. `None` when the cursor is past the chain,
    /// where the original's match never fires and the effect stays put.
    #[must_use]
    pub fn energy_point(
        &self,
        owner: Vec3,
        target: Vec3,
        range: f32,
        locate: Locate,
    ) -> Option<Vec3> {
        let separation = target - owner;
        let segments = segment_count(separation.length(), range).min(MAX_SEGMENTS);
        let index = segments.checked_sub(1)?.checked_sub(self.drawn.0)?;
        tube::walk(owner, target, segments, locate)
            .get(index as usize)
            .copied()
    }
}

/// Everything [`build`] needs besides the ribbon.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// The shooter's node origin.
    pub owner: Vec3,
    /// The target's node origin.
    pub target: Vec3,
    /// The shooter's own right and up axes - rows 0 and 1 of the node matrix
    /// `LeachBeam_Advance` reads.
    pub owner_right: Vec3,
    pub owner_up: Vec3,
    /// The camera's world-space right and up: `LeachBeam_BuildStrip` widens
    /// its two strips along view-space `x` and `y`.
    pub camera_right: Vec3,
    pub camera_up: Vec3,
    /// [`oag_weapons::projectile::leach_beam::Beam::range`].
    pub range: f32,
    /// The link's coverage: `1.0` while connected, the disconnect fade after.
    pub alpha: f32,
}

/// The chain `LeachBeam_Advance` writes to `instance+0x170`: the shooter's
/// origin, then `segments` displaced points ending at the target.
fn chain(ribbon: &Ribbon, frame: &Frame, segments: u32, locate: Locate) -> Vec<Vec3> {
    let separation = frame.target - frame.owner;
    let step = separation / segments as f32;
    let span = (6.0 / frame.range) * separation.length().min(frame.range);
    let angular_step = std::f32::consts::TAU / segments as f32 * span;
    let axis_a = step.cross(-frame.owner_up).normalize_or_zero();
    let axis_b = step.cross(frame.owner_right).normalize_or_zero();
    let (cursor, counter) = ribbon.drawn;
    let amplitude = |index: u32| {
        let bucket = (index as f32 / AMPLITUDE_BUCKET_SPAN).ceil() as usize;
        ribbon.amplitudes[bucket.min(AMPLITUDE_BUCKETS)]
    };

    // The axes, `span` and `angular_step` keep the straight line's values: the
    // original computes them once before its loop, and only the base walk
    // bends.
    let bases = tube::walk(frame.owner, frame.target, segments, locate);
    let mut points = Vec::with_capacity(segments as usize + 1);
    points.push(frame.owner);
    for i in 0..segments {
        let base = bases[i as usize + 1];
        let a = (i + cursor) % segments;
        let b = (i + counter / 2) % segments;
        let push_a = ((a + 1) as f32 * angular_step).sin() * amplitude(a);
        let push_b = ((b + 1) as f32 * angular_step).sin() * amplitude(b) * SECOND_AXIS_SCALE;
        points.push(base + axis_a * push_a + axis_b * push_b);
    }
    points
}

/// Builds the ribbon's geometry for one frame, as a flat vertex list ready for
/// [`Pipeline::upload`] - one triangle strip, the same primitive
/// `LeachBeam_SubmitStrip` draws, laid out exactly as `LeachBeam_BuildStrip`
/// lays out its own GE vertex array.
///
/// Pair `i` (`0..=segments`) is chain point `i` widened along the camera's
/// right; pair `segments + 1 + i` the same point widened along its up. Pair
/// `segments` is then overwritten into a zero-area bridge between the two
/// strips, so the first strip stops at point `segments - 1`. Alpha is zero at
/// points `0` and `segments - 1` - not at `segments`, which only the second
/// strip draws, at full alpha.
///
/// `locate` places a point on the track for the tube bend; one that returns
/// `None` everywhere draws the straight-line chain.
///
/// Empty when the two craft coincide or `range` is non-positive.
#[must_use]
pub fn build(ribbon: &Ribbon, frame: &Frame, locate: Locate) -> Vec<GpuVertex> {
    let distance = (frame.target - frame.owner).length();
    if distance < 1e-4 || frame.range <= 0.0 {
        return Vec::new();
    }
    let segments = segment_count(distance, frame.range).min(MAX_SEGMENTS);
    let points = chain(ribbon, frame, segments, locate);
    let n = segments as usize;

    let mut vertices = vec![bytemuck::Zeroable::zeroed(); 4 * (n + 1)];
    for (strip, across) in [frame.camera_right, frame.camera_up]
        .into_iter()
        .enumerate()
    {
        for (i, point) in points.iter().enumerate() {
            let pair = strip * (n + 1) + i;
            let alpha = if i == 0 || i == n - 1 {
                0.0
            } else {
                frame.alpha
            };
            let u = (pair & 1) as f32 + ribbon.scroll_phase;
            let colour = [1.0, 1.0, 1.0, alpha];
            vertices[2 * pair] = rib_vertex(*point - across * HALF_WIDTH, colour, [u, 0.0]);
            vertices[2 * pair + 1] = rib_vertex(*point + across * HALF_WIDTH, colour, [u, 1.0]);
        }
    }
    // The bridge: pair `segments` takes the last drawn vertex of the first
    // strip and the first vertex of the second, so every triangle through it
    // has zero area.
    vertices[2 * n].position = vertices[2 * n - 1].position;
    vertices[2 * n + 1].position = vertices[2 * n + 2].position;
    vertices
}

/// One ribbon vertex.
fn rib_vertex(p: Vec3, colour: [f32; 4], texcoord: [f32; 2]) -> GpuVertex {
    GpuVertex {
        position: p.to_array(),
        normal: [0.0, 0.0, 1.0],
        colour,
        texcoord,
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    }
}

pub mod pipeline;
pub use pipeline::Pipeline;

/// Wipeout HD's own LeachBall: the model `LeachBall_Advance` (`0x00114c78`)
/// carries along the beam, one trip per drain - a different mechanism from
/// the ribbon above and HD-only. Read in
/// `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`'s "2026-09-25: the
/// LeachBall's period, its trigger search, and the ball wired" section.
///
/// **The position law alone, not a mesh draw.** The model itself
/// (`hd_leachbeam_ball_bloomring.vex`) is loaded but not drawn here -
/// `race/scene.rs` and `race/load.rs` both sit at `scripts/check-file-size.py`'s
/// 1,000-line ceiling with no headroom for a new drawable pool, and folding
/// this weapon into the unrelated `blast_models::PlasmaBlastModels` container
/// the way Plasma's own bolt head already does felt like compounding that
/// workaround rather than reusing it cleanly - left unwired with the reason
/// recorded rather than forced in. What this module gives the caller is
/// enough to place `WO_LEACHBEAM_ABSORB` correctly, and to draw the mesh once
/// either constraint above is gone.
pub mod hd_ball {
    use super::Vec3;

    /// Beam length at/under which a drain trip takes its shortest period -
    /// `DAT_008a9c88` = `20.0`, read directly. Confidence 85 for the constant
    /// and the clamp; **that the clamped input is the beam's own length is
    /// not confirmed** - `LeachBall_Advance`'s `param_4[4]` traces to
    /// `strip+0x10`, and what writes that field was not read this pass.
    /// Chosen, not measured, for what the input represents - only the
    /// arithmetic on it is recovered.
    pub const PERIOD_MIN_LENGTH: f32 = 20.0;
    /// Beam length at/over which a drain trip takes its longest period -
    /// `DAT_008a9c8c` = `100.0`. See [`PERIOD_MIN_LENGTH`].
    pub const PERIOD_MAX_LENGTH: f32 = 100.0;
    /// The period at [`PERIOD_MIN_LENGTH`] - `DAT_008a9c80` = `0.3` seconds,
    /// read directly. Confidence 85.
    pub const PERIOD_MIN_SECONDS: f32 = 0.3;
    /// The period at [`PERIOD_MAX_LENGTH`] - `DAT_008a9c90` = `1.0` second,
    /// read directly. Confidence 85.
    pub const PERIOD_MAX_SECONDS: f32 = 1.0;

    /// `LeachBall_Advance`'s own remap: `clamp(length, MIN_LENGTH,
    /// MAX_LENGTH)` mapped linearly onto `[MIN_SECONDS, MAX_SECONDS]` - a
    /// direct decompile of the `_opd_FUN_002a3718` call site. Confidence 85
    /// for the arithmetic; see [`PERIOD_MIN_LENGTH`] for what is chosen about
    /// the input.
    #[must_use]
    pub fn period(length: f32) -> f32 {
        let t = ((length.abs() - PERIOD_MIN_LENGTH) / (PERIOD_MAX_LENGTH - PERIOD_MIN_LENGTH))
            .clamp(0.0, 1.0);
        PERIOD_MIN_SECONDS + t * (PERIOD_MAX_SECONDS - PERIOD_MIN_SECONDS)
    }

    /// One tick of the render-side accumulator: advances `elapsed` by `dt`
    /// and wraps it against [`period`] - the drain trip `LeachBall_Advance`
    /// runs inline every time its own accumulator passes the period,
    /// spawning `WO_LEACHBEAM_ABSORB` at the wrap. Returns whether this tick
    /// wrapped.
    ///
    /// **Assumes `dt` never exceeds one period**, true at this engine's fixed
    /// 60 Hz for [`PERIOD_MIN_SECONDS`] (18 ticks) - the same simplification
    /// [`super::Ribbon::advance`]'s own modulo-based cursor already takes
    /// over the original's unbounded-timestep `while` loop.
    pub fn advance(elapsed: &mut f32, dt: f32, length: f32) -> bool {
        *elapsed += dt;
        let period = period(length);
        if *elapsed >= period {
            *elapsed -= period;
            true
        } else {
            false
        }
    }

    /// Where the ball sits along the beam this tick: `target` at fraction
    /// `0.0` (the trip's start), `owner` at fraction `1.0` (the wrap) - a
    /// straight lerp, not the strip's own displaced chain.
    ///
    /// **Both halves of this are chosen, not measured.** The original's own
    /// fraction-to-point lookup (`_opd_FUN_00116308`) walks the strip's
    /// authored chain rather than a straight line, and flips which end is
    /// fraction `0` on a condition this pass did not resolve
    /// (`param_2[2] == *param_2`); the target-to-owner direction taken here
    /// is inferred only from where the wrap's own `WO_LEACHBEAM_ABSORB`
    /// lands (read as the shooter's own matrix, by analogy with Pulse's
    /// `WO_LEACHBEAM_ENERGY`, which arrives at the shooter the same way -
    /// see [`super::Ribbon::energy_point`]) and is not itself confirmed.
    #[must_use]
    pub fn position(elapsed: f32, length: f32, owner: Vec3, target: Vec3) -> Vec3 {
        let fraction = (elapsed / period(length)).clamp(0.0, 1.0);
        target + (owner - target) * fraction
    }
}

#[cfg(test)]
mod tests;
