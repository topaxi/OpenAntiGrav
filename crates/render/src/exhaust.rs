//! The ship's engine exhaust: an additive camera-facing flare at the nozzle.
//!
//! Recovered at instruction level from the PSP executable's `Engine Flare` node
//! class (`0x3bf`). The evidence, the addresses and a confidence score per claim
//! are in `docs/ghidra/functions/psp-pulse-usa/exhaust.md`; this module implements
//! what that page describes and cites it rather than restating it.
//!
//! # What the original does, in one paragraph
//!
//! A ship carries **exactly one** `Engine Flare` node, named `engine_flare`, a
//! direct child of `world` - one centred nozzle, not one per visible engine, on
//! all eight teams whose `Ship.vex` resolves by name. Its handler draws a single
//! four-vertex quad, textured
//! `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip`, blended
//! **additively weighted by source alpha**, and sizes it from an intensity that
//! ramps with thrust and a speed term. The size and the alpha are re-randomised
//! every frame: the flicker is in the original.
//!
//! Its constructor also builds a **`Trail`** (class `0x3c8`) as a child, in code,
//! with `Trail_InitPreset`'s preset 2: a 10-sample position-history ribbon in
//! three layers, textured `Data\Tex\engineFlare\Engine_noise.mip`. That ribbon is
//! the streak behind a racing craft, and the three staggered intensity ramps
//! `Exhaust_Update` computes are **its per-layer colours** - not three concentric
//! quads, which is how they were first read here.
//!
//! Because a `Trail` reaches the ship through its constructor rather than through
//! an authored node, no `Ship.vex` contains one. An earlier version of this module
//! concluded from that absence that racing craft had no trail at all. They do; see
//! the docs page for how the wrong inference was made and what refuted it.
//!
//! # Two halves, deliberately
//!
//! [`Exhaust`] is the state and the maths, with no `wgpu` in it at all, so the
//! recovered constants are testable on a machine with no graphics driver - the
//! same split `track` and `collision` already use. [`Pipeline`] is the GPU side.
//!
//! # What is *not* the original
//!
//! The original submits its quad after projection, with the GE's matrices set to
//! identity, so its half-size is in post-projection units and the sprite does not
//! shrink with distance. This module builds the quad in **world space** from the
//! camera basis instead - a deliberate choice, since that artefact is calibrated
//! for 480x272 and this renderer must hold up at 4K and in ultrawide (see
//! `docs/rendering/README.md`, "reproduce the output, not the pipeline"). The one
//! constant that departs is [`HALF_SIZE_TO_WORLD`], which carries its own
//! reasoning and its own confidence score rather than being folded in silently.
//!
//! The ribbon itself is recovered end to end (2026-08-02, static read plus a
//! live PPSSPP capture; see exhaust.md's GE-state and vertex-bake sections):
//! a four-fin cross around the position history, vertex colours baked as
//! *authored* blue/magenta/white times a steep [`trail_fade`] toward the tail,
//! multiplied per frame by the grey intensity ramp, drawn **pure additive**
//! (`dst + src`, not weighted by alpha). The fade curve is why the original's
//! exhaust reads as a compact bloom: the back half of the ribbon is
//! numerically invisible.
//!
//! `Trail` is *also* authored twice on `shipwreck.vex` (`trail_con_left_wing`,
//! `trail_con_right_wing`); the wreck is out of scope here.

use oag_core::Rng;
use oag_core::math::Vec3;

use crate::mesh::GpuVertex;

/// Trail layers.
///
/// Three, staggered so each fades in later than the last. Not a quality setting:
/// `Trail_InitPreset`'s preset 2 - the one `ExhaustFlare_Init` passes - sets the
/// layer count to 3, and `Exhaust_Update` writes exactly three colours.
pub const LAYERS: usize = 3;

/// Samples in the trail's position history, from `ring+0x04` at preset 2.
///
/// **The original refuses to draw until the ring is full**, so the trail takes
/// this many ticks to appear from a standing start. Reproduced, because it is
/// visible: the trail does not spring into existence at full length.
pub const TRAIL_SAMPLES: usize = 10;

/// Per-layer half-width in world units, from `ring+0x78` / `+0xa8` / `+0xd8` at
/// preset 2.
///
/// **Absolute, not a multiple of the flare's half-size.** `Trail_DrawRibbon`
/// computes a layer's width as `LAYER_WIDTH[n] * ring+0x1c`. `Trail_InitRing`
/// sets `ring+0x1c` to `1.0`, but that value never survives a race tick:
/// `Exhaust_Update` overwrites it every frame with
/// `intensity * `[`TRAIL_WIDTH_GAIN`]` + `[`TRAIL_WIDTH_BASE`] - live-confirmed
/// `0.3737` at intensity `0.497` - so the drawn widths are these times a
/// `0.2..0.55` scale, not these directly. Treating the init-time `1.0` as the
/// scale, which this module did until 2026-08-02, drew the ribbon roughly
/// 2-3x too wide.
pub const LAYER_WIDTH: [f32; LAYERS] = [1.0, 0.7, 0.5];

/// Base of the runtime width scale `Exhaust_Update` writes into `ring+0x1c`.
pub const TRAIL_WIDTH_BASE: f32 = 0.2;

/// Intensity gain of the runtime width scale: `intensity * 0.35 + 0.2`.
pub const TRAIL_WIDTH_GAIN: f32 = 0.35;

/// The authored per-layer colours from `Trail_ApplyPreset`'s preset 2, as the
/// exact `N/255` values the shipped floats encode.
///
/// **These are alive, and an earlier reading declared them dead.**
/// `Trail_BakeVertexColours` bakes them into the ring's vertex data at init -
/// before `Exhaust_Update` ever overwrites the ring's own colour fields - and
/// the GE multiplies them by the per-frame grey ramp (lighting on, lights off,
/// `sceGuColorMaterial(7)`: vertex colour is the material, `sceGuAmbient` the
/// light). Outer layer deep blue, middle magenta, core near-white: the
/// original's violet-white bloom.
pub const LAYER_COLOUR: [[f32; 3]; LAYERS] = [
    [0.0, 8.0 / 255.0, 128.0 / 255.0],
    [155.0 / 255.0, 0.0, 124.0 / 255.0],
    [210.0 / 255.0, 210.0 / 255.0, 210.0 / 255.0],
];

/// Per-sample step of the baked colour fade, `ring+0x08` at preset 2.
///
/// Also the per-sample `u` texcoord step the bake writes (`0.1` per sample,
/// before the per-layer `sceGuTexScale`).
pub const TRAIL_FADE_STEP: f32 = 0.1;

/// Shape constant of the baked colour fade, the global `DAT_08ac00a0`.
///
/// The bake computes `powf(1.0 - TRAIL_FADE_STEP * i, 1.0 / TRAIL_FADE_SHAPE)`
/// per sample - an exponent of `3.333`, so the curve runs
/// `1.0, 0.70, 0.475, 0.30, 0.18, 0.099, 0.047, 0.018, 0.005, 0.0005` and the
/// back half of the ribbon contributes nothing visible. Live-confirmed against
/// the baked vertex bytes (`0xd2, 0x63, 0x26, 0x09, 0x00` on the white layer at
/// samples 0/2/4/6/8).
pub const TRAIL_FADE_SHAPE: f32 = 0.3;

/// Scale of the stored per-sample exhaust direction, `DAT_08a84c40`.
///
/// `Exhaust_Update` stores `-(craft matrix row 2) * 200000.0` as each trail
/// point's direction, and the craft's row carries the global `0.75` model scale
/// ([`CRAFT_ROW_SCALE`]), so the vector's magnitude is ~150,000 (live:
/// `150,080`). The offset table's tiny weights multiply *this*, which is why
/// "the displacement is numerically nil" was wrong: the tail stretches about
/// 3.5 world units backwards. See [`trail_stretch`].
pub const TRAIL_DIRECTION_SCALE: f32 = 200_000.0;

/// The craft world matrix's global scale, carried by the row
/// [`TRAIL_DIRECTION_SCALE`] multiplies.
///
/// The same `0.75` the collider and hover-height reads recovered
/// (`hover::TARGET_GLOBAL_SCALE` in `oag-physics`); duplicated here because
/// this crate deliberately depends on nothing but `oag-core` and `wgpu`.
pub const CRAFT_ROW_SCALE: f32 = 0.75;

/// The GE's u16 texcoord convention doubles the bake's intended fractions.
///
/// `Trail_BakeVertexColours` writes texcoords as `(u16)(fraction * 65535)`,
/// but the GE decodes 16-bit texcoords as `value / 32768` (unsigned - PPSSPP's
/// `Step_TcU16ToFloat`, `* (1.0f / 32768.0f)`), so every baked fraction lands
/// on screen at just under twice its written value: the noise tiles
/// `0.2 * texscale` per sample along the ribbon and wraps **twice** around the
/// four-fin tube, not once. The scroll offsets are exempt - `sceGuTexOffset`
/// is applied after the decode, undoubled.
pub const TEXCOORD_U16_GAIN: f32 = 65535.0 / 32768.0;

/// Fins per segment: the ribbon is a diamond tube, not a flat quad.
///
/// `Trail_DrawRibbon`'s 10-vertex strip walks the rim `+up, +right, -up,
/// -right, +up` (camera columns), closing the loop - four quads through the
/// trail line per segment, with the `v` texcoord running once around the rim
/// in steps of `0.25`.
pub const TRAIL_FINS: usize = 4;

/// Width at the head of the trail, `ring+0x24` at preset 2.
pub const TRAIL_HEAD_TAPER: f32 = 1.0;

/// The taper's per-second rate, `ring+0x2c`, which `Trail_InitPreset` derives as
/// `-(ring[0x24] / (capacity * dt)) * 0.75` = **-4.5**.
///
/// The `0.75` is preset 2's own `fVar7`. Kept as the derivation rather than as the
/// product so the three presets stay distinguishable.
pub const TRAIL_TAPER_RATE: f32 = -(TRAIL_HEAD_TAPER / (TRAIL_SAMPLES as f32 * SUBSTEP_DT)) * 0.75;

/// Per-layer `u` scroll rate, from `ring+0x88` / `+0xb8` / `+0xe8` at preset 2.
///
/// **These were first read as position offsets along the exhaust direction**,
/// which put the three layers up to 6 units apart and stretched the plume badly.
/// They are texture scroll rates: `Trail_DrawRibbon` does
/// `u += rate * dt * 3.0`, wraps to `[0,1]` and hands the result to
/// `sceGuTexOffset`. Laying the per-layer block out settles it - see the block
/// table on `exhaust.md` - and the giveaway is that the *same* stride-`0x30` block
/// holds the colour fields `Exhaust_Update` demonstrably writes.
///
/// Negative, so the noise flows *along* the ribbon away from the nozzle.
pub const LAYER_SCROLL_U: [f32; LAYERS] = [-1.5, -3.0, -6.0];

/// Per-layer `v` scroll rate, from `ring+0x8c` / `+0xbc` / `+0xec` at preset 2.
///
/// The middle layer runs the other way, which is what stops the three reading as
/// one texture at three brightnesses.
pub const LAYER_SCROLL_V: [f32; LAYERS] = [1.0, -1.0, 1.0];

/// The `u` scroll's extra gain. `Trail_DrawRibbon` multiplies only `u` by this.
pub const SCROLL_U_GAIN: f32 = 3.0;

/// Per-layer `u` texture scale, from `ring+0x90` / `+0xc0` / `+0xf0` at preset 2,
/// passed to `sceGuTexScale`.
///
/// The noise tiles this many times along the ribbon, so the inner layer is the
/// most finely detailed. `v` is `1.0` on all three (`ring+0x94` / `+0xc4` /
/// `+0xf4`), so nothing tiles across the width.
pub const LAYER_TEX_SCALE_U: [f32; LAYERS] = [7.5, 6.0, 4.5];

/// The offset-table time constant, `Trail_BuildOffsetTable`'s argument.
///
/// The per-sample weight is `w(t) = (-1/K) * (exp(-t/K) - 1)`, an exponential
/// ease rather than a linear trail-off.
pub const TRAIL_EASE: f32 = 80.0;

/// The substep the original's lag filters run at, exactly as it appears in the
/// instruction stream (`Exhaust_Update` divides `dt` by this).
///
/// Not `1.0 / 60.0`: the shipped constant is this literal, and the simulation is
/// fixed at 60 Hz anyway ([ADR-0007]), so the two agree to within the literal's
/// own rounding.
///
/// [ADR-0007]: https://docs.rs/
pub const SUBSTEP_DT: f32 = 0.016_666_668;

/// Scales every layer's alpha. `DAT_08a84c3c` in the executable.
pub const LAYER_ALPHA_SCALE: f32 = 0.7;

/// Where each layer starts fading in, as a fraction of intensity.
///
/// With [`LAYER_GAIN`] these are `1/(1 - start)`, so every layer reaches full
/// opacity together at intensity `1.0`.
pub const LAYER_START: [f32; LAYERS] = [0.0, 0.25, 0.5];

/// Per-layer gain, from the executable's literals: `1.0`, `1.33`, `2.0`.
///
/// `1.33` is the shipped value, not `1.0 / 0.75 = 1.3333…` - kept as shipped
/// because a "tidied" constant is a different constant.
pub const LAYER_GAIN: [f32; LAYERS] = [1.0, 1.33, 2.0];

/// Speed to km/h. The original multiplies by this before every speed test.
pub const SPEED_TO_KMH: f32 = 3.6;

/// Speed at which the speed term starts to contribute, in km/h.
pub const RAMP_FLOOR_KMH: f32 = 100.0;

/// Span over which the speed term reaches 1.0, in km/h - so it saturates at 600.
pub const RAMP_SPAN_KMH: f32 = 500.0;

/// The speed term's share of the boost accumulator's floor.
pub const RAMP_FLOOR_SHARE: f32 = 0.6;

/// Divisor applied to thrust when charging the boost accumulator.
pub const THRUST_CHARGE_DIVISOR: f32 = 3000.0;

/// Per-call decay of the boost accumulator while thrust is off.
///
/// Per *call*, not per second - the original decays by this literal without
/// touching `dt`, and it is reproduced that way. At 60 Hz it empties in 6 ticks.
pub const BOOST_DECAY: f32 = 0.1;

/// Boost-timer threshold above which the engine counts as on regardless of thrust.
pub const BOOST_GATE: f32 = 0.2;

/// How long the revealed `<Team>boost.vex` plume stays up, `flare+0x88`'s
/// hide threshold at preset 2.
///
/// Independent of [`BOOST_SECONDS`]: the plume's own accumulator only resets
/// at the reveal, so once up it runs this full span regardless of how long
/// `boost_timer` stays above [`BOOST_GATE`].
pub const PLUME_SECONDS: f32 = 1.5;

/// How long a speed pad lights the flare for, in seconds.
///
/// **A literal in the code, not a tunable.** `ExhaustFlare_OnSpeedupPad`
/// (`0x08904f10`) builds `0x3f4ccccd` with a `lui`/`ori` pair and stores it
/// straight to `self+0xb8`; there is no XML path and no per-class table.
/// Confidence **90** - one caller, one literal, and the store sits in a delay
/// slot so it happens even when the flare has no craft.
///
/// **It is deliberately not `<SpeedupPads time>`.** The *force* runs for the
/// speed class's own duration, which is a fraction of this on every shipped
/// class, so the flare outlives the shove by design: a short push and a long
/// look. Tying the two together is the obvious-looking mistake and the original
/// does not do it - see `oag_game::race`'s pad trigger.
///
/// **This is only a third of what a boost changes.** `boost_timer` reaches
/// exactly three things in `Exhaust_Update`: this size, the reveal of the
/// additive `<Team>boost.vex` plume once the timer passes [`BOOST_GATE`], and
/// the `engine_on` flag - with thrust off, `boost_timer > `[`BOOST_GATE`]
/// alone keeps the engine counted as on, which feeds the engine sound and the
/// intensity ramp. The plume then runs on **its own** [`PLUME_SECONDS`] timer
/// (`self+0x88`, reset at the reveal), so it long outlives the `0.8` here.
/// This module tracks that timer ([`Exhaust::plume_visible`]) but does not
/// draw the mesh itself - `oag-game` owns loading and drawing
/// `<Team>boost.vex` beside the ship.
pub const BOOST_SECONDS: f32 = 0.8;

/// Intensity gain per second while the engine is on.
pub const INTENSITY_RISE: f32 = 0.25;

/// Intensity loss per second while it is off. Twice the rise: it dies faster
/// than it lights.
pub const INTENSITY_FALL: f32 = 0.5;

/// Base and span of the half-size's intensity term: `intensity * 0.6 + 0.4`.
pub const HALF_SIZE_SPAN: f32 = 0.6;
/// See [`HALF_SIZE_SPAN`].
pub const HALF_SIZE_BASE: f32 = 0.4;
/// Multiplier on the intensity term of the half-size.
pub const HALF_SIZE_GAIN: f32 = 2.5;
/// Multiplier on the boost timer's contribution to the half-size.
pub const HALF_SIZE_BOOST_GAIN: f32 = 8.0;

/// Bounds of the per-frame size flicker, from `Rng_RangeF(0.75, 1.25)`.
pub const FLICKER: (f32, f32) = (0.75, 1.25);

/// Bounds of the per-frame alpha flicker, from `Rng_RangeI(200, 255)`, as
/// fractions of full opacity.
pub const ALPHA_FLICKER: (f32, f32) = (200.0 / 255.0, 1.0);

/// The flare quad's on-screen width-to-height ratio: `480 / 272`.
///
/// `ExhaustFlare_Draw` writes equal extents, `x = cx +/- half_size` and
/// `y = cy +/- half_size`, in the post-projection space it submits in, and the
/// PSP viewport maps a unit of NDC `x` onto 240 pixels against 136 for `y`.
/// The shipped flare is therefore drawn **1.76x wider than tall on screen**,
/// and the `128x64` flare texture is authored for exactly that stretch.
/// Drawing the quad square, which this module did until 2026-08-02, compressed
/// the art's horizontal lobes and read visibly narrower than the original.
pub const FLARE_ASPECT: f32 = 480.0 / 272.0;

/// Converts the recovered half-size into world units.
///
/// **This is the one number here that is not the original's.** The original's
/// half-size is applied after projection, so it is in the GE's post-projection
/// units and the sprite keeps a constant on-screen size at any distance. Drawing
/// in world space instead is what makes the effect resolution-independent, and it
/// needs the size in world units, for which no conversion factor exists anywhere
/// in the executable to read.
///
/// `2.15`, **measured**, replacing the earlier sanity-check `1.0`
/// (2026-08-02). Method: two intensity-saturated frames - a live PPSSPP race
/// at 83 km/h and our capture at 459 km/h - both cameras at their recovered
/// speed-independent distance, comparing the bloom's visible-core width as a
/// fraction of the hull's on-screen width (the same `128x64` art on both
/// sides, so core-to-hull ratios compare quad sizes directly). Original:
/// `~0.56` hull widths; ours at `1.0`: `~0.26`; ratio `2.15`.
///
/// **Confidence 65.** Better than the old sanity check but still a fit: the
/// measurement rides on the visible-core threshold and on our camera's field
/// of view matching the original's, neither of which is pinned. It stays
/// named and isolated rather than multiplied into [`HALF_SIZE_GAIN`], where
/// it would silently corrupt a recovered value.
pub const HALF_SIZE_TO_WORLD: f32 = 2.15;

/// Per-frame state of one ship's exhaust.
///
/// Mirrors [`crate::camera::chase::Chase`] in shape and for the same reasons: it
/// is render-only state that the game crate owns and advances on the simulation's
/// fixed tick, so it never enters `World` and never touches a determinism hash.
///
/// `Copy`, because a `World` holding one per ship should stay memcpy-shaped
/// ([ADR-0003](https://docs.rs/)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exhaust {
    intensity: f32,
    boost_accumulator: f32,
    boost_timer: f32,
    speed_kmh: f32,
    engine_on: bool,
    half_size: f32,
    alpha: f32,
    plume_timer: f32,
    plume_visible: bool,
    /// Position history, newest at `write - 1`.
    trail: [Vec3; TRAIL_SAMPLES],
    /// The exhaust direction at each sample, so a ribbon segment keeps the
    /// orientation the craft had when it was laid down rather than the current one.
    trail_back: [Vec3; TRAIL_SAMPLES],
    trail_len: usize,
    trail_write: usize,
    /// Per-layer texture scroll, wrapped to `[0, 1)` as the original wraps it.
    scroll: [[f32; 2]; LAYERS],
}

impl Default for Exhaust {
    fn default() -> Self {
        Self::new()
    }
}

impl Exhaust {
    /// A cold engine: no intensity, no boost, nothing drawn.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            intensity: 0.0,
            boost_accumulator: 0.0,
            boost_timer: 0.0,
            speed_kmh: 0.0,
            engine_on: false,
            half_size: 0.0,
            alpha: 0.0,
            plume_timer: 0.0,
            plume_visible: false,
            trail: [Vec3::ZERO; TRAIL_SAMPLES],
            trail_back: [Vec3::ZERO; TRAIL_SAMPLES],
            trail_len: 0,
            trail_write: 0,
            scroll: [[0.0; 2]; LAYERS],
        }
    }

    /// Advances one simulation tick.
    ///
    /// `thrust` is the craft's raw thrust input and `speed` the magnitude of its
    /// linear velocity, in world units per second. `rng` supplies the flicker -
    /// seeded, never OS entropy, so a capture at tick *n* is reproducible and
    /// `--screenshot` stays meaningful.
    ///
    /// Order matters and follows the original: the boost timer decays, then
    /// intensity moves, then the size and alpha are drawn from the new intensity.
    pub fn advance(&mut self, dt: f32, thrust: f32, speed: f32, rng: &mut Rng) {
        // Accumulates every tick regardless of visibility, matching the
        // original adding to `flare+0x88` at the top of the frame.
        self.plume_timer += dt;

        self.speed_kmh = speed * SPEED_TO_KMH;
        let ramp = ((self.speed_kmh - RAMP_FLOOR_KMH) / RAMP_SPAN_KMH).clamp(0.0, 1.0);

        // The original charges per call and decays per call, neither scaled by
        // `dt`. Reproduced rather than corrected: at the fixed 60 Hz this project
        // runs the simulation at, per-call and per-tick are the same thing.
        if thrust > 0.0 {
            self.boost_accumulator += thrust / THRUST_CHARGE_DIVISOR;
        } else {
            self.boost_accumulator -= BOOST_DECAY;
        }
        self.boost_accumulator = self
            .boost_accumulator
            .clamp(0.0, 1.0)
            .max(ramp * RAMP_FLOOR_SHARE);

        self.engine_on = thrust > 0.0 || self.boost_timer > BOOST_GATE;

        // Reveal is edge-triggered on the visibility bit being clear, so
        // crossing a second pad while the plume is up neither restarts nor
        // extends it. Hidden separately and after: a pad crossed late enough
        // that `boost_timer` is still above `BOOST_GATE` when the `1.5 s`
        // expires re-reveals on the very next tick rather than staying latched
        // shut, which is the original's own per-tick check, not a one-shot.
        if self.engine_on && self.boost_timer > BOOST_GATE && !self.plume_visible {
            self.plume_visible = true;
            self.plume_timer = 0.0;
        }
        if self.plume_visible && self.plume_timer >= PLUME_SECONDS {
            self.plume_visible = false;
        }

        // `Exhaust_UpdateEngineSound` is where the original keeps this ramp - it
        // drives the engine note's volume - but the three layer alphas and the
        // half-size all read the same field, so it belongs to the picture too.
        self.intensity += if self.engine_on {
            dt * INTENSITY_RISE
        } else {
            -dt * INTENSITY_FALL
        };
        self.intensity = self.intensity.clamp(0.0, 1.0);

        self.boost_timer = (self.boost_timer - dt).max(0.0);

        let base = (self.intensity * HALF_SIZE_SPAN + HALF_SIZE_BASE) * HALF_SIZE_GAIN
            + self.boost_timer * HALF_SIZE_BOOST_GAIN;
        self.half_size = base * range(rng, FLICKER) * HALF_SIZE_TO_WORLD;
        self.alpha = range(rng, ALPHA_FLICKER);

        // `Trail_DrawRibbon` advances these itself, at a per-frame constant that is
        // `0x3c888889` = 0.016666668 - the same 1/60 as the lag substep, so at this
        // project's fixed rate `dt` is that constant and the two agree.
        for n in 0..LAYERS {
            let u = self.scroll[n][0] + LAYER_SCROLL_U[n] * dt * SCROLL_U_GAIN;
            let v = self.scroll[n][1] + LAYER_SCROLL_V[n] * dt;
            // Wrapped rather than left to grow: the original wraps to keep the
            // offset small, and an unbounded texcoord loses precision.
            self.scroll[n] = [u.rem_euclid(1.0), v.rem_euclid(1.0)];
        }
    }

    /// Records one trail sample.
    ///
    /// `nozzle` is the `engine_flare` locator in world space and `back` the unit
    /// direction the exhaust points, i.e. away from the craft's nose. The original
    /// pushes **one sample per frame** with no `dt` anywhere in `Trail_Update`;
    /// called from the fixed tick, that is one per tick.
    ///
    /// Separate from [`Exhaust::advance`] only because the state machine does not
    /// need to know where the ship is, and keeping it that way is what lets every
    /// ramp be unit-tested without a pose.
    pub fn push_trail(&mut self, nozzle: Vec3, back: Vec3) {
        self.trail[self.trail_write] = nozzle;
        self.trail_back[self.trail_write] = back;
        self.trail_write = (self.trail_write + 1) % TRAIL_SAMPLES;
        self.trail_len = (self.trail_len + 1).min(TRAIL_SAMPLES);
    }

    /// Clears the history, for a race start or a respawn.
    ///
    /// Without this a respawn draws a ribbon stretching from wherever the craft
    /// was rescued from, straight across the track.
    pub fn clear_trail(&mut self) {
        self.trail_len = 0;
        self.trail_write = 0;
    }

    /// Whether the ribbon is drawable.
    ///
    /// `Trail_DrawRibbon` returns without drawing unless the ring is **full**, so
    /// this is the original's own gate rather than a guard against empty geometry.
    #[must_use]
    pub fn trail_ready(&self) -> bool {
        self.trail_len >= TRAIL_SAMPLES
    }

    /// The trail's samples, newest first.
    ///
    /// Walked backwards from the write cursor, which is the order
    /// `Trail_DrawRibbon` walks its ring in.
    fn trail_samples(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        (0..self.trail_len).map(move |k| {
            let i = (self.trail_write + TRAIL_SAMPLES - 1 - k) % TRAIL_SAMPLES;
            (self.trail[i], self.trail_back[i])
        })
    }

    /// The ribbon's vertices: three layers of four-fin diamond tubes.
    ///
    /// Empty until [`Exhaust::trail_ready`]. Per segment the rim runs
    /// `+up, +right, -up, -right` and closes - `Trail_DrawRibbon`'s own
    /// 10-vertex strip, built from the camera matrix's first two columns, so
    /// the cross is camera-aligned. Each layer runs the full history at its own
    /// [`LAYER_WIDTH`] times the runtime width scale
    /// (`intensity * 0.35 + 0.2`), scroll and tiling.
    ///
    /// The colour per vertex is `authored layer colour x grey ramp x baked
    /// fade`, premultiplied here because the ribbon's recovered blend is pure
    /// additive (`dst + src.rgb`, both factors `GU_FIX` white): the alpha
    /// channel is set to `1.0` and contributes nothing, exactly as the GE
    /// ignores it. The flare's per-frame alpha flicker does **not** apply to
    /// the ribbon - the original's flicker lands only on the flare quad's
    /// colour (`flare+0xc8`).
    ///
    /// Each sample is displaced along the direction it was recorded with by
    /// [`trail_stretch`] - about 3.5 world units at the tail.
    #[must_use]
    pub fn trail_vertices(&self, right: Vec3, up: Vec3) -> Vec<GpuVertex> {
        if !self.trail_ready() {
            return Vec::new();
        }
        let ramps = self.layer_alphas();
        let samples: Vec<(Vec3, Vec3)> = self.trail_samples().collect();
        let width_scale = self.intensity * TRAIL_WIDTH_GAIN + TRAIL_WIDTH_BASE;

        let mut out = Vec::with_capacity(MAX_TRAIL_VERTICES);
        // The rim, in `Trail_DrawRibbon`'s own order. The fifth entry closes
        // the tube; `v` runs 0..1 once around it in quarters.
        let rim = [up, right, -up, -right, up];
        for layer in 0..LAYERS {
            let ramp = ramps[layer];
            let base_width = LAYER_WIDTH[layer] * width_scale;
            let colour = LAYER_COLOUR[layer];
            // The texture scale and scroll are baked into the texcoords rather
            // than set as pipeline state, because all three layers share one
            // draw. `sceGuTexScale(su, 1.0)` plus `sceGuTexOffset(u, v)` is the
            // same thing: tile `su` times along the ribbon, shifted. The bake
            // steps `u` by `TRAIL_FADE_STEP` per sample, not by `1/(n-1)`.
            let su = LAYER_TEX_SCALE_U[layer];
            let [ou, ov] = self.scroll[layer];
            for k in 0..samples.len() - 1 {
                let (a, back_a) = samples[k];
                let (b, back_b) = samples[k + 1];
                let pa = a + back_a * trail_stretch(k);
                let pb = b + back_b * trail_stretch(k + 1);
                let wa = base_width * trail_taper(k);
                let wb = base_width * trail_taper(k + 1);
                let fade_a = ramp * trail_fade(k);
                let fade_b = ramp * trail_fade(k + 1);
                let ca = [colour[0] * fade_a, colour[1] * fade_a, colour[2] * fade_a];
                let cb = [colour[0] * fade_b, colour[1] * fade_b, colour[2] * fade_b];
                let ua = k as f32 * TRAIL_FADE_STEP * TEXCOORD_U16_GAIN * su + ou;
                let ub = (k + 1) as f32 * TRAIL_FADE_STEP * TEXCOORD_U16_GAIN * su + ou;
                for fin in 0..TRAIL_FINS {
                    let va = fin as f32 * 0.25 * TEXCOORD_U16_GAIN + ov;
                    let vb = (fin + 1) as f32 * 0.25 * TEXCOORD_U16_GAIN + ov;
                    let a0 = rib_vertex(pa + rim[fin] * wa, ca, ua, va);
                    let a1 = rib_vertex(pa + rim[fin + 1] * wa, ca, ua, vb);
                    let b0 = rib_vertex(pb + rim[fin] * wb, cb, ub, va);
                    let b1 = rib_vertex(pb + rim[fin + 1] * wb, cb, ub, vb);
                    out.extend_from_slice(&[a0, a1, b0, a1, b1, b0]);
                }
            }
        }
        out
    }

    /// Sets the intensity straight to where a steady state at this thrust and
    /// speed would put it, without ramping.
    ///
    /// For a race start and a respawn, so the flare does not visibly light up
    /// over four seconds from a standing start. The original's own equivalent is
    /// the rising-edge snap in `Exhaust_UpdateEngineSound`, which assigns rather
    /// than filters when the engine turns on; [`crate::camera::chase::Chase`]
    /// carries the same idea as `snapped`.
    pub fn snap(&mut self, thrust: f32, speed: f32) {
        self.speed_kmh = speed * SPEED_TO_KMH;
        let ramp = ((self.speed_kmh - RAMP_FLOOR_KMH) / RAMP_SPAN_KMH).clamp(0.0, 1.0);
        self.boost_accumulator = ramp * RAMP_FLOOR_SHARE;
        self.engine_on = thrust > 0.0;
        self.intensity = if self.engine_on { 1.0 } else { 0.0 };
        self.boost_timer = 0.0;
        // Mid-range rather than flickered: a snap has no previous frame to differ
        // from, and seeding the flicker here would make the first drawn frame
        // depend on how many times `snap` had been called.
        self.half_size = (self.intensity * HALF_SIZE_SPAN + HALF_SIZE_BASE)
            * HALF_SIZE_GAIN
            * HALF_SIZE_TO_WORLD;
        self.alpha = ALPHA_FLICKER.1;
    }

    /// Arms the boost timer, which widens and brightens the flare and is what
    /// reveals the `<Team>boost.vex` model in the original.
    pub fn boost(&mut self, seconds: f32) {
        self.boost_timer = self.boost_timer.max(seconds);
    }

    /// Intensity, `0.0` to `1.0`.
    #[must_use]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Speed in km/h, as the original's own tests use it.
    #[must_use]
    pub fn speed_kmh(&self) -> f32 {
        self.speed_kmh
    }

    /// Whether the engine counts as lit this tick.
    #[must_use]
    pub fn engine_on(&self) -> bool {
        self.engine_on
    }

    /// The boost accumulator, `0.0` to `1.0`.
    ///
    /// Recovered and maintained, but nothing in the original's *visual* path was
    /// found reading it - the size uses the boost **timer** instead. Kept because
    /// dropping a recovered term to tidy the model is how a fit gets mistaken for
    /// a reading later.
    #[must_use]
    pub fn boost_accumulator(&self) -> f32 {
        self.boost_accumulator
    }

    /// Seconds left on the boost, which is what the flare's size actually reads.
    ///
    /// Exposed so a caller that arms [`Self::boost`] can check the flare lets go
    /// when whatever armed it does. [`Self::boost_accumulator`] is **not** that
    /// quantity - it tracks the throttle ramp as well, so it is non-zero on an
    /// ordinary accelerating ship.
    #[must_use]
    pub fn boost_timer(&self) -> f32 {
        self.boost_timer
    }

    /// Half-size of the flare quad, in world units.
    #[must_use]
    pub fn half_size(&self) -> f32 {
        self.half_size
    }

    /// This frame's flickered alpha, `0.0` to `1.0`.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Whether the `<Team>boost.vex` plume should be drawn this tick.
    #[must_use]
    pub fn plume_visible(&self) -> bool {
        self.plume_visible
    }

    /// Per-layer opacity, innermost first.
    ///
    /// Layer *n* stays at zero until intensity passes [`LAYER_START`]`[n]`, then
    /// climbs at [`LAYER_GAIN`]`[n]` so all three saturate together at `1.0`.
    #[must_use]
    pub fn layer_alphas(&self) -> [f32; LAYERS] {
        let mut out = [0.0; LAYERS];
        for (n, slot) in out.iter_mut().enumerate() {
            *slot = ((self.intensity - LAYER_START[n]) * LAYER_GAIN[n] * LAYER_ALPHA_SCALE)
                .clamp(0.0, 1.0);
        }
        out
    }

    /// The flare's vertices: **one** quad, as `ExhaustFlare_Draw` draws it.
    ///
    /// `nozzle` is the `engine_flare` locator in world space; `right` and `up`
    /// come from the camera, which is what makes the quad face the viewer. The
    /// `right` extent is stretched by [`FLARE_ASPECT`], reproducing the
    /// original's post-projection square rendering 1.76x wider than tall
    /// through the PSP viewport.
    ///
    /// The colour is white with the flickered alpha and nothing else - all four
    /// of the original's vertices take the single colour at `flare+0xc8`. The
    /// three staggered layer ramps do **not** touch the flare; they are the
    /// ribbon's per-layer colours, and an earlier version of this module drew
    /// three concentric flare quads from them, which was an invention. The
    /// flare's intensity response is entirely in its *size*
    /// (`(intensity * 0.6 + 0.4) * 2.5`), so an idle engine shows a small
    /// glow rather than nothing - as the original does on the start line.
    #[must_use]
    pub fn vertices(&self, nozzle: Vec3, right: Vec3, up: Vec3) -> Vec<GpuVertex> {
        let half = self.half_size;
        quad(nozzle, right * (half * FLARE_ASPECT), up * half, self.alpha).to_vec()
    }
}

/// A value in `lo..hi`, from the seeded generator.
fn range(rng: &mut Rng, (lo, hi): (f32, f32)) -> f32 {
    lo + (hi - lo) * rng.next_f32()
}

/// Displacement along the exhaust direction for the sample at index `i`, in
/// world units. **Fully recovered, confidence 88** - no free scale remains.
///
/// `Trail_BuildOffsetTable` fills its table with
/// `w(t) = (-1/K) * (exp(-t/K) - 1)` for `K = `[`TRAIL_EASE`]` = 80` and
/// `t = i * dt`, and `Trail_DrawRibbon` multiplies each sample's *stored
/// direction* by it. An earlier reading evaluated the weights (`~2.6e-6` per
/// sample), called the displacement "numerically nil" and pinned a
/// `TRAIL_LENGTH` of zero here - missing that the stored direction is
/// `-(craft row 2) * 200000` ([`TRAIL_DIRECTION_SCALE`]), magnitude ~150,000
/// once the row's own `0.75` scale is in. The product is a real, modest
/// backwards stretch: `0` at the head, `~3.5` units at the tail, eased rather
/// than linear. Live-confirmed: the stored vector read back at magnitude
/// `150,080`.
#[must_use]
pub fn trail_stretch(sample: usize) -> f32 {
    let t = sample as f32 * SUBSTEP_DT;
    let w = -(1.0 / TRAIL_EASE) * ((-t / TRAIL_EASE).exp() - 1.0);
    w * TRAIL_DIRECTION_SCALE * CRAFT_ROW_SCALE
}

/// The baked per-sample colour fade: `powf(1 - 0.1 i, 3.333)`.
///
/// `Trail_BakeVertexColours` writes this into the ring's vertex colours once at
/// init; the exponent is `1 / `[`TRAIL_FADE_SHAPE`]. The steepness is the
/// single biggest reason the original's exhaust reads as a compact bloom - the
/// curve is under `0.05` by sample 6 of 10.
#[must_use]
pub fn trail_fade(sample: usize) -> f32 {
    (1.0 - TRAIL_FADE_STEP * sample as f32)
        .max(0.0)
        .powf(1.0 / TRAIL_FADE_SHAPE)
}

/// The per-sample width multiplier: **1.0 at the head, 0.325 at the tail**.
///
/// This is the ribbon's whole silhouette and it was missed at first, which is most
/// of why our plume read as a uniform slab against the original's tapering one.
/// Read off the instructions rather than the decompiler
/// (`0x0892ae60`..`0x0892ae84`):
///
/// ```text
/// f12  = layer_width * ring[0x1c]      ; ring+0x1c is 1.0
/// S020 = f12 * S700                    ; S700 varies per sample
/// vscl.q C730, C000, S020              ; scales the camera basis by it
/// vscl.q C720, C010, S020
/// ```
///
/// `S700` is `point.q0.x + point.q1.x * table[i].x`, which resolves to
/// `ring[0x24] + ring[0x2c] * (i * dt)` = `1.0 - 0.075 * i`, reaching `0.325` at
/// sample 9. The decompiler renders `S700` as a position component, which is what
/// made it look like nonsense - the first lane of a trail point is this scalar and
/// the position occupies the other three.
#[must_use]
pub fn trail_taper(sample: usize) -> f32 {
    (TRAIL_HEAD_TAPER + TRAIL_TAPER_RATE * sample as f32 * SUBSTEP_DT).max(0.0)
}

/// One ribbon vertex: the rgb already carries `authored colour x ramp x fade`,
/// and the alpha channel is inert under the ribbon's pure-additive blend
/// ([`TRAIL_BLEND`]).
fn rib_vertex(p: Vec3, rgb: [f32; 3], u: f32, v: f32) -> GpuVertex {
    GpuVertex {
        position: p.to_array(),
        normal: [0.0, 0.0, 1.0],
        // Alpha 1.0: under [`TRAIL_BLEND`] both factors are One, so the shader's
        // alpha output never reaches the colour result - same as the GE.
        colour: [rgb[0], rgb[1], rgb[2], 1.0],
        texcoord: [u, v],
        // Emissive, like the flare: no light rig.
        lit: 0.0,
        v_cycles: 0.0,
    }
}

/// The blend the original's display list sets, as `wgpu` spells it.
///
/// `sceGuBlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0, 0xffffff)` from
/// `ExhaustFlare_BuildDisplayList` - source weighted by its own alpha, destination
/// at unity, added. `0xffffff` is the fixed destination factor, i.e. `(1, 1, 1)`,
/// which is what makes it additive rather than a lerp.
///
/// Alpha is written the same way so a target that is later read as premultiplied
/// does not disagree with the colour channels.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The ribbon's blend, which is **not** the flare's.
///
/// `Trail_BuildStateList` records
/// `sceGuBlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)` - both factors
/// fixed white, so the ribbon composites as `dst + src` with alpha contributing
/// nothing. The fade a viewer sees is entirely in the vertex rgb
/// ([`trail_fade`]); weighting by source alpha as well - which this module did
/// while it assumed the flare's blend - double-counts every fade and was part
/// of why no tuning of the old constants converged.
pub const TRAIL_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The maximum vertices [`Pipeline`]'s buffer holds: the flare's one quad.
///
/// One, not one per layer: the three staggered ramps are the ribbon's, and
/// `ExhaustFlare_Draw` writes exactly four vertices sharing one colour.
pub const MAX_VERTICES: usize = 6;

/// The ribbon's vertex budget: three layers of `TRAIL_SAMPLES - 1` segments,
/// each a four-quad diamond tube.
pub const MAX_TRAIL_VERTICES: usize = LAYERS * (TRAIL_SAMPLES - 1) * TRAIL_FINS * 6;

/// The flare's draw pipeline: the first blended, depth-tested pipeline in this
/// crate.
///
/// # Why depth write is off
///
/// Not a stylistic choice. The original resolves its own transparency ordering
/// with a 20-bit back-to-front sort key spanning 3,000 world units
/// (`ExhaustFlare_Submit`, and `Gfx_FlushRenderManager` is what sorts it), so it
/// never depends on writing depth for a transparent. Depth *test* stays on, so
/// the hull still occludes the flare correctly; depth *write* is off so the flare
/// cannot occlude anything drawn after it, including another ship's flare.
///
/// Everything else matches `mesh_render`'s pipeline exactly, because the exhaust
/// is a third `set_pipeline` inside that pass rather than a pass of its own:
/// same target format, same [`crate::mesh_render::DEPTH_FORMAT`], sample count 1,
/// and the same 128-byte uniform block so
/// [`crate::mesh_render::UNIFORMS_SIZE`] describes both.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    /// The ribbon's own pipeline: identical but for [`TRAIL_BLEND`], because
    /// the original's trail display list sets a different blend than the
    /// flare's (`GU_FIX`/`GU_FIX` against `GU_SRC_ALPHA`/`GU_FIX`).
    trail_pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    texture: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// Vertices actually uploaded by the last [`Pipeline::upload`].
    count: u32,
    /// `Data\Tex\engineFlare\Engine_noise.mip`, the ribbon's own texture.
    ///
    /// The original binds one of three slots per layer
    /// (`g_engine_noise_textures[...]`); all three hold the same handle, so one
    /// bind group covers it.
    noise: wgpu::BindGroup,
    trail_vertices: wgpu::Buffer,
    trail_count: u32,
}

impl Pipeline {
    /// Builds the pipeline and uploads the flare texture.
    ///
    /// `flare` is RGBA8, normally
    /// `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` decoded by
    /// `oag_formats::texture`. `format` must be the target the caller's render
    /// pass writes, and `sample_count` must match its multisample state - see
    /// `mesh_render::build`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        flare: &FlareTexture,
        noise: &FlareTexture,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("exhaust"),
            source: wgpu::ShaderSource::Wgsl(include_str!("exhaust.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("exhaust uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("exhaust flare"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("exhaust"),
            bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let build_pipeline = |label: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GpuVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                            4 => Float32
                        ],
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    // A camera-facing quad has no meaningful winding: the basis it
                    // is built from flips as the camera orbits. The original
                    // disables cull for both the flare and the ribbon.
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: crate::mesh_render::DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = build_pipeline("exhaust", BLEND);
        let trail_pipeline = build_pipeline("exhaust trail", TRAIL_BLEND);

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exhaust uniforms"),
            size: crate::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("exhaust uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exhaust vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let trail_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exhaust trail vertices"),
            size: (MAX_TRAIL_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture = flare.bind(device, queue, &texture_layout, "engine flare", false);
        let noise = noise.bind(device, queue, &texture_layout, "engine noise", true);

        Self {
            pipeline,
            trail_pipeline,
            uniforms,
            bind_group,
            texture,
            vertices,
            count: 0,
            noise,
            trail_vertices,
            trail_count: 0,
        }
    }

    /// Uploads this frame's camera matrix and geometry.
    ///
    /// Takes `&mut self` only for the vertex count. Both writes go through
    /// `queue`, which is why the caller's `render` can stay `&self` - the state
    /// the flare animates lives on the game side and is advanced on the
    /// simulation tick, not here.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        vertices: &[GpuVertex],
        trail: &[GpuVertex],
    ) {
        // The vertex shader ignores `model`; the identity keeps the shared block
        // well-defined rather than leaving half of it uninitialised.
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        let n = vertices.len().min(MAX_VERTICES);
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices[..n]));
        self.count = n as u32;

        let t = trail.len().min(MAX_TRAIL_VERTICES);
        if t > 0 {
            queue.write_buffer(&self.trail_vertices, 0, bytemuck::cast_slice(&trail[..t]));
        }
        self.trail_count = t as u32;
    }

    /// Draws into a pass the caller already opened.
    ///
    /// Must be issued **after** the opaque geometry: with depth write off, the
    /// flare relies on the hull's depth already being present to be occluded by
    /// it.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.count == 0 && self.trail_count == 0 {
            return;
        }
        pass.set_bind_group(0, &self.bind_group, &[]);

        // Ribbon first, nozzle sprite second. Both blends are additive so the
        // order does not change the result, but drawing the large translucent
        // thing before the small bright one matches how the original's depth
        // sort orders them and keeps the sprite reading as the hottest part of
        // the plume. The ribbon uses its own pipeline: its recovered blend is
        // `dst + src`, not the flare's alpha-weighted one.
        if self.trail_count > 0 {
            pass.set_pipeline(&self.trail_pipeline);
            pass.set_bind_group(1, &self.noise, &[]);
            pass.set_vertex_buffer(0, self.trail_vertices.slice(..));
            pass.draw(0..self.trail_count, 0..1);
        }
        if self.count > 0 {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(1, &self.texture, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..self.count, 0..1);
        }
    }
}

/// An RGBA8 image for the flare, with its dimensions.
///
/// A plain struct rather than a borrow of `oag_formats`' type, so this crate
/// keeps depending on nothing but `oag-core` and `wgpu` for the exhaust: the
/// caller decodes the `.mip` and hands the pixels over.
#[derive(Debug, Clone)]
pub struct FlareTexture {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8.
    pub rgba: Vec<u8>,
}

impl FlareTexture {
    /// A soft radial glow, for when the disc's own texture is unavailable.
    ///
    /// Used by tests and by the headless capture path. **Not** a silent
    /// substitute in the game: the caller reports a missing archive entry rather
    /// than quietly drawing this, because a stand-in that looks plausible is how
    /// a decode failure survives review.
    #[must_use]
    pub fn placeholder(size: u32) -> Self {
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        let centre = (size as f32 - 1.0) / 2.0;
        for y in 0..size {
            for x in 0..size {
                let dx = (x as f32 - centre) / centre;
                let dy = (y as f32 - centre) / centre;
                // Squared falloff, so the edge reaches zero rather than clipping.
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let a = (1.0 - d) * (1.0 - d);
                rgba.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
        Self {
            width: size,
            height: size,
            rgba,
        }
    }

    fn bind(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        label: &str,
        wrap_v: bool,
    ) -> wgpu::BindGroup {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width.max(1) * 4),
                rows_per_image: Some(self.height.max(1)),
            },
            wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // The ribbon's state list sets `sceGuTexWrap(GU_REPEAT, GU_REPEAT)`, and
        // it needs both: `u` tiles `LAYER_TEX_SCALE_U` times along the trail
        // plus a scroll offset, and `v` runs once around the diamond tube plus
        // its own scroll, so either coordinate routinely leaves `[0, 1]`. The
        // flare quad samples exactly 0..1 and keeps `v` clamped so its soft
        // edge cannot bleed the opposite row in under bilinear filtering.
        let v_mode = if wrap_v {
            wgpu::AddressMode::Repeat
        } else {
            wgpu::AddressMode::ClampToEdge
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: v_mode,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }
}

/// Six vertices - two triangles - for one camera-facing quad.
///
/// Wound as an explicit triangle list rather than a strip, matching how the
/// rest of this module fills its buffers.
///
/// The colour's alpha carries the flicker, which the additive blend then
/// weights by - `src.rgb * src.a + dst.rgb`, recovered from
/// `ExhaustFlare_BuildDisplayList`.
fn quad(centre: Vec3, right: Vec3, up: Vec3, alpha: f32) -> [GpuVertex; 6] {
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + right * sx + up * sy).to_array(),
        // The flare is emissive: it must not pick up the mesh light rig, which is
        // what `lit = 0.0` means to the shader this crate already ships.
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, alpha],
        texcoord: [u, v],
        lit: 0.0,
        v_cycles: 0.0,
    };
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng::new(1)
    }

    /// The speed term's two endpoints, straight from `Exhaust_Update`.
    #[test]
    fn the_speed_ramp_is_zero_at_100_kmh_and_one_at_600() {
        let mut e = Exhaust::new();
        let mut r = rng();

        // 100 km/h is the floor: the ramp contributes nothing, so with no thrust
        // the accumulator's floor is zero too.
        e.advance(SUBSTEP_DT, 0.0, RAMP_FLOOR_KMH / SPEED_TO_KMH, &mut r);
        assert_eq!(e.boost_accumulator(), 0.0);

        // 600 km/h saturates it, and the floor is 0.6 of that.
        let mut e = Exhaust::new();
        e.advance(
            SUBSTEP_DT,
            0.0,
            (RAMP_FLOOR_KMH + RAMP_SPAN_KMH) / SPEED_TO_KMH,
            &mut r,
        );
        assert!(
            (e.boost_accumulator() - RAMP_FLOOR_SHARE).abs() < 1e-6,
            "{}",
            e.boost_accumulator()
        );
    }

    /// Beyond 600 km/h the ramp clamps rather than growing.
    #[test]
    fn the_speed_ramp_clamps_above_600_kmh() {
        let mut e = Exhaust::new();
        let mut r = rng();
        e.advance(SUBSTEP_DT, 0.0, 10_000.0, &mut r);
        assert!((e.boost_accumulator() - RAMP_FLOOR_SHARE).abs() < 1e-6);
    }

    /// The three layers cross zero at 0, 0.25 and 0.5, and all reach full at 1.0.
    ///
    /// This is the shape the staggered gains exist to produce, and it is the one
    /// property a wrong transcription of `1.33` or `2.0` would break.
    #[test]
    fn the_three_layers_stagger_and_then_saturate_together() {
        let mut e = Exhaust::new();

        e.intensity = 0.0;
        assert_eq!(e.layer_alphas(), [0.0, 0.0, 0.0]);

        // Just above each start, that layer is live and the later ones are not.
        e.intensity = 0.01;
        let a = e.layer_alphas();
        assert!(a[0] > 0.0 && a[1] == 0.0 && a[2] == 0.0, "{a:?}");

        e.intensity = 0.26;
        let a = e.layer_alphas();
        assert!(a[1] > 0.0 && a[2] == 0.0, "{a:?}");

        e.intensity = 0.51;
        let a = e.layer_alphas();
        assert!(a[2] > 0.0, "{a:?}");

        // At full intensity all three arrive together at the ceiling - but only
        // *nearly*, and the near-miss is the original's own.
        //
        // Layers 0 and 2 land on `LAYER_ALPHA_SCALE` exactly (`1.0 * 1.0 * 0.7`
        // and `0.5 * 2.0 * 0.7`). Layer 1 reaches `0.75 * 1.33 * 0.7 = 0.69825`,
        // 0.25 % short, because the shipped gain is the literal `1.33` and not
        // `4/3`. Asserting exact equality here failed, which is the check earning
        // its keep: had the constant been "tidied" to `1.0 / 0.75` on the way in,
        // this test would have passed and the value would have been wrong.
        e.intensity = 1.0;
        let full = e.layer_alphas();
        assert_eq!(full[0], LAYER_ALPHA_SCALE);
        assert_eq!(full[2], LAYER_ALPHA_SCALE);
        assert!(
            (full[1] - 0.698_25).abs() < 1e-5,
            "middle layer at {}",
            full[1]
        );
        for (n, alpha) in full.iter().enumerate() {
            assert!(
                (*alpha - LAYER_ALPHA_SCALE).abs() < LAYER_ALPHA_SCALE * 0.005,
                "layer {n} at {alpha} is more than 0.5% off the ceiling"
            );
        }
    }

    /// Intensity rises at 0.25/s and falls at 0.5/s, clamped to `[0, 1]`.
    #[test]
    fn intensity_rises_at_a_quarter_and_falls_at_a_half_per_second() {
        let mut e = Exhaust::new();
        let mut r = rng();

        // One second of thrust at 60 Hz.
        for _ in 0..60 {
            e.advance(1.0 / 60.0, 100.0, 0.0, &mut r);
        }
        assert!(
            (e.intensity() - INTENSITY_RISE).abs() < 1e-3,
            "{}",
            e.intensity()
        );

        // Then one second off. Falling twice as fast, it overshoots zero and
        // clamps rather than going negative.
        for _ in 0..60 {
            e.advance(1.0 / 60.0, 0.0, 0.0, &mut r);
        }
        assert_eq!(e.intensity(), 0.0);
    }

    /// An idle engine draws a small glow, not nothing.
    ///
    /// The flare's intensity response is entirely in its size - the colour is
    /// white at the flickered alpha regardless - so at intensity 0 the quad
    /// runs at `0.4` of its saturated base. The original shows exactly this on
    /// the start line before the countdown ends. (The *ribbon* still vanishes
    /// cold: the layer ramps are its colours.)
    #[test]
    fn an_idle_engine_glows_small_rather_than_vanishing() {
        let mut e = Exhaust::new();
        let mut r = rng();
        assert_eq!(e.layer_alphas(), [0.0, 0.0, 0.0]);
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        assert!(e.alpha() >= ALPHA_FLICKER.0 - 1e-6);
        let idle = e.half_size();
        assert!(idle > 0.0);

        // Saturate and compare: the size ratio is the recovered 0.4 base
        // against the full 1.0, within the flicker's spread.
        for _ in 0..600 {
            e.advance(SUBSTEP_DT, 100.0, 0.0, &mut r);
        }
        let hot = e.half_size();
        let ratio = idle / hot;
        let bound = (HALF_SIZE_BASE / (HALF_SIZE_SPAN + HALF_SIZE_BASE))
            * (FLICKER.1 / FLICKER.0).max(FLICKER.0 / FLICKER.1);
        assert!(
            ratio < bound + 1e-3,
            "idle {idle} vs hot {hot}: ratio {ratio} above {bound}"
        );
    }

    /// `snap` puts a thrusting craft straight at full intensity, and does not
    /// depend on how many times it has been called.
    #[test]
    fn snap_does_not_ramp_and_is_idempotent() {
        let mut e = Exhaust::new();
        e.snap(100.0, 100.0);
        assert_eq!(e.intensity(), 1.0);
        let once = e;
        e.snap(100.0, 100.0);
        assert_eq!(once, e);
    }

    /// The flicker is seeded, so the same tick sequence gives the same picture.
    ///
    /// This is what keeps `--screenshot` comparable between runs; an unseeded
    /// flicker would make every capture differ and none of them wrong.
    #[test]
    fn the_flicker_is_reproducible_from_the_seed() {
        let run = || {
            let mut e = Exhaust::new();
            let mut r = Rng::new(7);
            for _ in 0..120 {
                e.advance(1.0 / 60.0, 100.0, 90.0, &mut r);
            }
            (e.half_size(), e.alpha())
        };
        assert_eq!(run(), run());
    }

    /// The flicker stays inside its recovered bounds over a long run.
    #[test]
    fn the_flicker_stays_within_its_bounds() {
        let mut e = Exhaust::new();
        let mut r = rng();
        for _ in 0..600 {
            e.advance(1.0 / 60.0, 100.0, 120.0, &mut r);
            assert!(
                e.alpha() >= ALPHA_FLICKER.0 - 1e-6 && e.alpha() <= 1.0,
                "{}",
                e.alpha()
            );
        }
        // At full intensity the unflickered half-size is 2.5 times the world
        // conversion, so the flicker bounds it by 0.75 and 1.25 of that.
        let steady = HALF_SIZE_GAIN * HALF_SIZE_TO_WORLD;
        assert!(
            e.half_size() >= steady * FLICKER.0 - 1e-3
                && e.half_size() <= steady * FLICKER.1 + 1e-3,
            "{}",
            e.half_size()
        );
    }

    /// Boost widens the flare, via the timer rather than the accumulator.
    #[test]
    fn boost_widens_the_flare_while_its_timer_runs() {
        let mut e = Exhaust::new();
        let mut r = rng();
        for _ in 0..600 {
            e.advance(1.0 / 60.0, 100.0, 120.0, &mut r);
        }
        let steady = e.half_size();
        e.boost(1.0);
        e.advance(1.0 / 60.0, 100.0, 120.0, &mut r);
        assert!(e.half_size() > steady, "{} vs {steady}", e.half_size());
    }

    /// The flare is one quad, emitted every frame, so the count never changes.
    #[test]
    fn the_vertex_count_is_constant() {
        let mut e = Exhaust::new();
        let mut r = rng();
        let cold = e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y).len();
        for _ in 0..300 {
            e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);
        }
        assert_eq!(cold, MAX_VERTICES);
        assert_eq!(e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y).len(), cold);
    }

    /// The flare renders wider than tall by the PSP viewport's 480:272.
    ///
    /// Equal post-projection extents through a 480x272 viewport is the
    /// original's own stretch; a square flare - which this module drew at
    /// first - compresses the wide-authored `128x64` art and reads visibly
    /// narrower than the running game.
    #[test]
    fn the_flare_is_wider_than_tall_by_the_viewport_aspect() {
        let mut e = Exhaust::new();
        let mut r = rng();
        for _ in 0..600 {
            e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);
        }
        let v = e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y);
        let width = v.iter().map(|v| v.position[0]).fold(f32::MIN, f32::max)
            - v.iter().map(|v| v.position[0]).fold(f32::MAX, f32::min);
        let height = v.iter().map(|v| v.position[1]).fold(f32::MIN, f32::max)
            - v.iter().map(|v| v.position[1]).fold(f32::MAX, f32::min);
        assert!(
            (width / height - FLARE_ASPECT).abs() < 1e-5,
            "aspect {} vs {FLARE_ASPECT}",
            width / height
        );
    }

    /// The quad is built from the camera basis, so it faces the viewer.
    #[test]
    fn the_quad_follows_the_camera_basis() {
        let mut e = Exhaust::new();
        let mut r = rng();
        e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);

        // Spanned by x and y: every vertex has z equal to the centre's.
        let v = e.vertices(Vec3::new(0.0, 0.0, 5.0), Vec3::X, Vec3::Y);
        assert!(v.iter().all(|v| (v.position[2] - 5.0).abs() < 1e-6));

        // Rotate the basis a quarter turn and the quad lies in x/z instead.
        let v = e.vertices(Vec3::new(0.0, 7.0, 0.0), Vec3::X, Vec3::Z);
        assert!(v.iter().all(|v| (v.position[1] - 7.0).abs() < 1e-6));
    }

    /// The ribbon draws nothing until the ring is full, which is the original's
    /// own gate - `Trail_DrawRibbon` returns early on a partial ring.
    #[test]
    fn the_ribbon_waits_for_a_full_ring() {
        let mut e = Exhaust::new();
        for k in 0..TRAIL_SAMPLES - 1 {
            e.push_trail(Vec3::new(0.0, 0.0, -(k as f32)), -Vec3::Z);
            assert!(!e.trail_ready(), "ready after only {} samples", k + 1);
            assert!(e.trail_vertices(Vec3::X, Vec3::Y).is_empty());
        }
        e.push_trail(Vec3::new(0.0, 0.0, -9.0), -Vec3::Z);
        assert!(e.trail_ready());
        assert_eq!(
            e.trail_vertices(Vec3::X, Vec3::Y).len(),
            MAX_TRAIL_VERTICES,
            "a full ring must emit every layer's every segment"
        );
    }

    /// A respawn must not leave a ribbon stretched across the track.
    #[test]
    fn clearing_the_trail_stops_it_drawing() {
        let mut e = Exhaust::new();
        for k in 0..TRAIL_SAMPLES {
            e.push_trail(Vec3::new(0.0, 0.0, -(k as f32)), -Vec3::Z);
        }
        assert!(e.trail_ready());
        e.clear_trail();
        assert!(!e.trail_ready());
        assert!(e.trail_vertices(Vec3::X, Vec3::Y).is_empty());
    }

    /// The ribbon tapers in width from head to tail, and its colour fades on
    /// the recovered `powf(1 - 0.1 i, 1/0.3)` curve.
    ///
    /// Both halves matter, and both were misread once: the taper (`1 - 0.075 i`,
    /// 0.325 at sample 9) was missed entirely at first, and the colour fade was
    /// declared absent while it sat baked in the ring's vertex colours.
    #[test]
    fn the_ribbon_tapers_in_width_and_fades_in_colour() {
        let mut e = Exhaust::new();
        let mut r = rng();
        for _ in 0..600 {
            e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);
        }
        // A straight run down -z, so widths read directly off the vertex spans.
        for k in 0..TRAIL_SAMPLES {
            e.push_trail(Vec3::new(0.0, 0.0, -(k as f32) * 3.0), -Vec3::Z);
        }
        let v = e.trail_vertices(Vec3::X, Vec3::Y);

        let per_segment = TRAIL_FINS * 6;
        let per_layer = (TRAIL_SAMPLES - 1) * per_segment;

        // Fin 0's first two vertices sit at head+up*w and head+right*w, so the
        // rim radius reads off either component. Head against last segment's
        // tail end (its `b`-side vertices, offset 2 within the fin).
        let radius = |vertex: usize| {
            let p = v[vertex].position;
            (p[0].powi(2) + p[1].powi(2)).sqrt()
        };
        let head = radius(0);
        let tail = radius(per_layer - per_segment + 2);
        assert!(head > tail, "head {head} must be wider than tail {tail}");

        // The recovered taper ratio at the extremes.
        let ratio = trail_taper(TRAIL_SAMPLES - 1) / trail_taper(0);
        assert!((ratio - 0.325).abs() < 1e-3, "taper ratio {ratio}");

        // The head width is the layer width times the runtime scale - the
        // intensity-driven `0.2..0.55`, saturated here - never the authored
        // width alone.
        let scale = 1.0 * TRAIL_WIDTH_GAIN + TRAIL_WIDTH_BASE;
        assert!(
            (head - LAYER_WIDTH[0] * scale).abs() < 1e-4,
            "head {head} vs {}",
            LAYER_WIDTH[0] * scale
        );

        // Colour fades along the ribbon on the baked curve; alpha stays 1.0
        // because the additive blend ignores it.
        let head_lum = v[0].colour[2];
        let tail_lum = v[per_layer - per_segment + 2].colour[2];
        assert!(
            tail_lum < head_lum * 0.01,
            "tail colour {tail_lum} must be under 1% of head {head_lum}"
        );
        assert!(v.iter().all(|vert| vert.colour[3] == 1.0));

        // Layer 0 is the authored deep blue times the ramp: red stays zero.
        assert_eq!(v[0].colour[0], 0.0);
        assert!(v[0].colour[2] > v[0].colour[1], "blue must dominate green");
    }

    /// The taper is monotonic and never negative.
    #[test]
    fn the_taper_falls_monotonically_and_stays_positive() {
        let mut last = f32::INFINITY;
        for i in 0..TRAIL_SAMPLES {
            let w = trail_taper(i);
            assert!(w < last, "not falling at {i}");
            assert!(w > 0.0, "taper went non-positive at {i}");
            last = w;
        }
        assert!((trail_taper(0) - 1.0).abs() < 1e-6);
    }

    /// The baked fade matches the live-captured vertex bytes.
    ///
    /// The white layer's baked colours at samples 0/2/4/6/8 read
    /// `0xd2 0x63 0x26 0x09 0x00` in the emulator - `210, 99, 38, 9, 0` - and
    /// the bake is `authored * fade * 255` truncated. Reproducing those bytes
    /// pins both the curve and its exponent against ground truth.
    #[test]
    fn the_fade_reproduces_the_live_captured_vertex_bytes() {
        let authored = LAYER_COLOUR[2][0];
        let baked: Vec<u32> = (0..TRAIL_SAMPLES)
            .step_by(2)
            .map(|i| (authored * trail_fade(i) * 255.0) as u32)
            .collect();
        assert_eq!(baked, vec![210, 99, 38, 9, 0]);
    }

    /// The backwards stretch is zero at the head and about 3.5 units at the
    /// tail - the recovered `w(t) * 200000 * 0.75`, no free scale.
    #[test]
    fn the_trail_stretch_is_recovered_end_to_end() {
        assert_eq!(trail_stretch(0), 0.0);
        let mut last = -1.0;
        for i in 0..TRAIL_SAMPLES {
            let w = trail_stretch(i);
            assert!(w > last, "not monotonic at {i}");
            last = w;
        }
        let tail = trail_stretch(TRAIL_SAMPLES - 1);
        assert!((tail - 3.51).abs() < 0.02, "tail stretch {tail}");
    }

    /// The flare is emissive, so it must not take the mesh shader's light rig.
    #[test]
    fn the_flare_is_never_lit() {
        let e = Exhaust::new();
        let v = e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y);
        assert!(v.iter().all(|v| v.lit == 0.0));
    }

    /// Ticks after the arming tick until the plume hides, with no further
    /// boosts crossed in between.
    ///
    /// Derived rather than hardcoded from [`PLUME_SECONDS`]`/`[`SUBSTEP_DT`]:
    /// `plume_timer` is an `f32` accumulator, and repeated addition of
    /// [`SUBSTEP_DT`] does not land on exactly `90 * SUBSTEP_DT` after 90
    /// additions, so a test asserting an exact tick index against the rounded
    /// division was off by one. The tests below only ever compare two runs
    /// driven by this same accumulation, never a hardcoded index, so they do
    /// not depend on where the boundary actually falls.
    fn plume_hide_tick() -> usize {
        let mut e = Exhaust::new();
        let mut r = rng();
        e.boost(BOOST_SECONDS);
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        assert!(e.plume_visible(), "did not reveal on the arming tick");
        for tick in 1..200 {
            e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
            if !e.plume_visible() {
                return tick;
            }
        }
        panic!("plume never hid within 200 ticks");
    }

    /// The plume is latched: crossing a second pad while it is up neither
    /// restarts nor extends its own 1.5 s countdown.
    #[test]
    fn a_second_boost_mid_plume_does_not_extend_it() {
        let baseline = plume_hide_tick();

        let mut e = Exhaust::new();
        let mut r = rng();
        e.boost(BOOST_SECONDS);
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        assert!(e.plume_visible());

        for tick in 1..=baseline {
            if tick == baseline / 2 {
                // Still visible: re-arming the boost timer here must not
                // reset the plume's own countdown.
                e.boost(BOOST_SECONDS);
            }
            e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
            let should_be_hidden = tick == baseline;
            assert_eq!(
                e.plume_visible(),
                !should_be_hidden,
                "wrong visibility at tick {tick}, expected relative to \
                 baseline hide tick {baseline}"
            );
        }
    }

    /// A pad crossed late enough that `boost_timer` is still above
    /// `BOOST_GATE` when the plume's own `1.5 s` expires produces a deferred
    /// back-to-back second plume: hide this tick, reveal the next. A
    /// one-shot latch that swallowed the second boost would be wrong - see
    /// this module's `BOOST_SECONDS` doc comment.
    #[test]
    fn a_late_pad_re_reveals_the_plume_the_tick_after_it_hides() {
        let baseline = plume_hide_tick();

        let mut e = Exhaust::new();
        let mut r = rng();
        e.boost(BOOST_SECONDS);
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        assert!(e.plume_visible());

        for _ in 1..baseline {
            e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        }
        assert!(
            e.plume_visible(),
            "still up one tick before its own deadline"
        );

        // Cross a second pad one tick before the deadline, freshly arming
        // `boost_timer` so it is still comfortably above `BOOST_GATE` once
        // the plume's own deadline hits.
        e.boost(BOOST_SECONDS);

        // Crosses the deadline: hides, without re-revealing on the same tick
        // even though `boost_timer` is high again.
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        assert!(!e.plume_visible(), "did not hide at its own deadline");

        // The very next tick: the latch is clear again and `boost_timer` is
        // still above the gate, so it reveals again.
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        assert!(
            e.plume_visible(),
            "did not re-reveal the tick after hiding, though boost_timer \
             was still armed"
        );
    }
}
