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
//! # The flare's space, corrected
//!
//! An earlier version of this module recorded the original as submitting its
//! quad after projection, in units that kept a constant on-screen size. A live
//! read at the draw (2026-08-07, see [`HALF_SIZE_TO_WORLD`]) shows otherwise:
//! the original transforms the nozzle into **view space** by a rigid matrix,
//! builds the quad there with world-sized extents, and draws it through the
//! scene's own projection - a view-space billboard that shrinks with distance.
//! This module's world-space camera-basis quad is therefore the same
//! construction, not a divergence, and [`HALF_SIZE_TO_WORLD`] is `1.0` rather
//! than a fitted conversion. The `480 / 272` stretch that used to ride on top
//! of it went with the same correction, on 2026-08-08 - see
//! [`Exhaust::vertices`].
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

use oag_core::{Rng, math::Vec3};

use oag_mesh::mesh::GpuVertex;

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
///
/// Re-exported from [`oag_core::math`] rather than defined here: the same
/// conversion turns an authored weapon speed into a velocity in
/// `oag_weapons::projectile`, which cannot see this crate.
pub use oag_core::math::SPEED_TO_KMH;

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
///
/// `flare+0x88` is also the **animation clock** of the plume's authored
/// keyframed u-scroll, whose track spans exactly these 90 frames - measured
/// at `TexAnim_UpdateTransform`'s entry, the time argument equals this timer
/// on every hit. So each boost plays the bright-to-dark texture sweep once,
/// ending as the plume hides; `race::Scene` reproduces that by sampling the
/// track at [`Exhaust::plume_timer`]. See
/// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`.
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
/// does not do it - see `oag_raceplay`'s pad trigger.
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

/// Converts the recovered half-size into world units: `1.0`, because the
/// original's half-size **is already in view units**, which a rigid view
/// transform makes the same size as world units.
///
/// Settled 2026-08-07 by reading the transform live rather than fitting
/// pixels: at a breakpoint in `ExhaustFlare_Draw` (`0x08904a30`), the matrix
/// the nozzle is transformed by - the display's stack top at `+0x1410` - read
/// back as a pure world-to-view rigid transform (orthonormal rows, fourth
/// column `0,0,0,1`), and `Math_TransformVec4` applies it with no perspective
/// divide. The GE's view and model matrices are then set to identity with the
/// **projection left live**, so the quad is a view-space billboard whose
/// `+/- half_size` extents are world-sized and shrink with distance like any
/// other geometry. This retires two earlier readings in order: the
/// "post-projection units, constant on-screen size" story this module used to
/// carry, and the `2.15` fitted here on 2026-08-02 - a matched-pose
/// comparison (`--pose-from` a captured crossing row, same recorded camera)
/// shows `1.0` reproducing the original's flare-to-hull ratio where `2.15`
/// read double. Confidence **85**: one live read, one binary, corroborated by
/// the matched-pose frame.
///
/// # That corroboration was retracted, and it is reinstated here on purpose
///
/// `exhaust.md` retracted the matched-pose leg because our craft was believed
/// to render `1.33x` too large, which would have contaminated any comparison
/// scaled to the hull. **The premise is refuted**: the craft is the right size
/// to 0.15 %, and what was actually wrong was a whole-frame zoom from the
/// original's speed-dependent field of view
/// (`docs/rendering/projection-vs-the-original.md`).
///
/// The retraction does not survive its premise, and the reason is worth stating
/// rather than leaving as an absence: this leg is a **flare-to-hull ratio taken
/// within each frame separately**, then compared. A uniform zoom about the
/// principal point scales the flare and the hull by the same factor, so it
/// cancels inside each frame's own ratio before the two are compared at all -
/// which makes this measurement one of the few in that whole family that the
/// misregistration could never have touched.
///
/// So the three legs stand and the confidence stays at 85. The live matrix read
/// at `ExhaustFlare_Draw` (`0x08904a30`) is independent of all of this and
/// `1.0` was never in doubt on its own.
pub const HALF_SIZE_TO_WORLD: f32 = 1.0;

/// Per-frame state of one ship's exhaust.
///
/// Mirrors `oag_render::camera::chase::Chase` in shape and for the same reasons: it
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
        // `speed_ramp()` is this same expression over the field just
        // written, so a caller reading it back gets what this tick used.
        let ramp = self.speed_ramp();

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
        let mut out = Vec::with_capacity(TRAIL_VERTICES_PER_CRAFT);
        self.extend_trail_vertices(&mut out, right, up);
        out
    }

    /// [`Self::trail_vertices`], appended to a list the caller owns.
    ///
    /// The form the renderer uses, because a fixed 648 vertices per craft
    /// returned by value is a 36 KiB allocation per craft per frame - eight of
    /// them on a full grid, and none of them living past the upload. See
    /// `race::Scene::scratch`.
    pub fn extend_trail_vertices(&self, out: &mut Vec<GpuVertex>, right: Vec3, up: Vec3) {
        if !self.trail_ready() {
            return;
        }
        let ramps = self.layer_alphas();
        let samples: Vec<(Vec3, Vec3)> = self.trail_samples().collect();
        let width_scale = self.intensity * TRAIL_WIDTH_GAIN + TRAIL_WIDTH_BASE;

        out.reserve(TRAIL_VERTICES_PER_CRAFT);
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
                // One stencil byte per segment, not interpolated: [`trail_stencil`].
                let glow = trail_stencil(self.intensity, k);
                let ua = k as f32 * TRAIL_FADE_STEP * TEXCOORD_U16_GAIN * su + ou;
                let ub = (k + 1) as f32 * TRAIL_FADE_STEP * TEXCOORD_U16_GAIN * su + ou;
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

    /// Sets the intensity straight to where a steady state at this thrust and
    /// speed would put it, without ramping.
    ///
    /// For a race start and a respawn, so the flare does not visibly light up
    /// over four seconds from a standing start. The original's own equivalent is
    /// the rising-edge snap in `Exhaust_UpdateEngineSound`, which assigns rather
    /// than filters when the engine turns on; `oag_render::camera::chase::Chase`
    /// carries the same idea as `snapped`.
    pub fn snap(&mut self, thrust: f32, speed: f32) {
        self.speed_kmh = speed * SPEED_TO_KMH;
        // `speed_ramp()` is this same expression over the field just
        // written, so a caller reading it back gets what this tick used.
        let ramp = self.speed_ramp();
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

    /// Seconds since the plume was revealed - the original's `flare+0x88`.
    ///
    /// **Not** [`Self::boost_timer`], and the difference is the whole reason
    /// this is exposed separately: the reveal is edge-triggered, so this
    /// restarts at a reveal and then runs its own [`PLUME_SECONDS`] span
    /// regardless of how long the boost that caused it lasts. It also keeps
    /// accumulating while the plume is hidden, matching the original adding to
    /// `flare+0x88` at the top of every frame rather than only while visible.
    ///
    /// Exposed for `oag-game --race --trace-out`, which writes it into the
    /// `plume_timer` column so our value can be differenced against a capture's.
    #[must_use]
    pub fn plume_timer(&self) -> f32 {
        self.plume_timer
    }

    /// The clamped speed ramp - the original's `flare+0x90`.
    ///
    /// Recomputed from the stored [`Self::speed_kmh`] rather than cached, which
    /// makes it exactly the local [`Self::advance`] derives and uses: there is
    /// one formula, not two that could drift. Zero below [`RAMP_FLOOR_KMH`] and
    /// one above `RAMP_FLOOR_KMH + RAMP_SPAN_KMH`.
    ///
    /// Exposed for the same reason as [`Self::plume_timer`]: the capture records
    /// this field, so the comparison needs our side of it.
    #[must_use]
    pub fn speed_ramp(&self) -> f32 {
        ((self.speed_kmh - RAMP_FLOOR_KMH) / RAMP_SPAN_KMH).clamp(0.0, 1.0)
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
    /// come from the camera, which is what makes the quad face the viewer.
    /// Both extents are the same `half_size`: **the quad is square**, and the
    /// `480 / 272` stretch this module applied until 2026-08-08 was an
    /// artefact. Three independent readings close it:
    ///
    /// - `ExhaustFlare_Draw` writes `cx +/- half` into x and `cy +/- half`
    ///   into y from the **same** register (`f13`,
    ///   `0x08904b58`..`0x08904ba4`), so the original's quad is an exact
    ///   view-space square.
    /// - the four UVs `ExhaustFlare_Init` writes span the whole `[0, 1]^2` of
    ///   `grabbedEngineFlare128x64x8.mip`, and that texture's glow measures
    ///   `sigma_u / sigma_v = 1.007` **in UV space** - authored round for a
    ///   square quad, and 2:1 in pixels only because the image is 2:1.
    /// - the projection read live at the flare draw is
    ///   `m11 / m00 = 1.764706` exactly, i.e. aspect-correct for 480x272, so
    ///   a view-space square projects square.
    ///
    /// The "visible glow measures about 1.7:1" observation on one of the
    /// original's own frames is contradicted by all three; the companion "a
    /// square flare read visibly narrower" comparison (2026-08-02) was taken
    /// while [`HALF_SIZE_TO_WORLD`] was still the fitted `2.15`, so it
    /// measured size rather than aspect. Read this before reinstating a
    /// stretch from a screenshot measurement.
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
        quad(nozzle, right * half, up * half, self.alpha).to_vec()
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
fn rib_vertex(p: Vec3, rgb: [f32; 3], u: f32, v: f32, glow: f32) -> GpuVertex {
    GpuVertex {
        position: p.to_array(),
        normal: [0.0, 0.0, 1.0],
        // Alpha is the **glow mask**, not an opacity: [`TRAIL_BLEND`]'s colour
        // factors are both One, so it never reaches the colour result - it is
        // stamped into the target's alpha for the bloom. See [`TRAIL_GLOW_GAIN`].
        colour: [rgb[0], rgb[1], rgb[2], glow],
        texcoord: [u, v],
        // Emissive, like the flare: no light rig.
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
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
    // **Alpha replaces rather than accumulates, and that is not the colour
    // blend.** `Trail_BuildStateList` opens the alpha channel with
    // `Gu_PixelMask(0)` and then writes it through
    // `sceGuStencilOp(KEEP, KEEP, REPLACE)` with `Gu_StencilFunc(GU_ALWAYS, ...)`,
    // so each fragment *stamps* the ribbon's glow value instead of adding to
    // what is there. Additive alpha would saturate the mask almost immediately -
    // twelve quads overlap at the nozzle - and the bloom reads that channel as
    // its bright-pass weight. See `oag_post::bloom`.
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::Zero,
        operation: wgpu::BlendOperation::Add,
    },
};

/// How much of the engine intensity reaches the glow mask, `ring+0x20`.
///
/// `Exhaust_Update` writes the ribbon's stencil base as `intensity * 0.5 * 0.9`
/// every frame (live-confirmed at `0.2233` for an intensity of `0.497`), and
/// `Trail_DrawRibbon` stamps it into the framebuffer's alpha. That channel is
/// the bloom's bright-pass mask, so this is what makes the exhaust glow at all -
/// see [`oag_post::bloom`].
///
/// **An earlier reading called this value "visually inert".** It is not; nothing
/// in the ribbon's own draw reads it, but the post-process does. See
/// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`.
pub const TRAIL_GLOW_GAIN: f32 = 0.5 * 0.9;

/// The glow mask at segment `k`, as a fraction of the head value.
///
/// Read at instruction level from `Trail_DrawRibbon` (`0x0892b214`-`0x0892b29c`)
/// on 2026-08-10, which is the pass that made the bloom usable:
///
/// ```text
/// f24  = 255.0                       ; lui a0, 0x437f
/// f22  = ring[0x20] * f24            ; the head value, in 0..255
/// f22  = f22 / n                     ; one step
/// f20  = ring[0x20] * f24            ; the running value
/// loop:  ref = trunc(f20) & 0xff     ; Gu_StencilFunc(GU_ALWAYS, ref, 0xff)
///        f20 = f20 - f22
/// ```
///
/// So the stencil reference is a **linear ramp from the head value down to
/// zero at the tail**, in the byte range the framebuffer's alpha channel
/// actually has. `ring[0x20]` is reloaded per layer, so each layer runs its
/// own ramp rather than continuing the previous one.
///
/// **Getting this wrong is not subtle.** Stamping the flat head value on every
/// segment - which this module did for one revision, because the step was
/// recorded as unrecovered - makes the ribbon's whole length a full-strength
/// bloom source and blows the exhaust out to a solid white cone. Measured
/// 2026-08-10 against the same frame both ways.
///
/// The original stamps one constant per segment: see [`trail_stencil`].
#[must_use]
pub fn trail_glow(intensity: f32, segment: usize) -> f32 {
    let n = (TRAIL_SAMPLES - 1) as f32;
    let fall = 1.0 - (segment as f32 / n);
    intensity * TRAIL_GLOW_GAIN * fall.max(0.0)
}

/// The stencil byte segment `k`'s draw stamps: [`trail_glow`] truncated, as
/// `trunc(f20)` is; measured off the original's EDRAM (`bloom.md`, 2026-10-08).
#[must_use]
pub fn trail_stencil(intensity: f32, segment: usize) -> f32 {
    (trail_glow(intensity, segment) * 255.0).floor() / 255.0
}

/// How many craft this crate's per-frame budgets are sized for.
///
/// The grid, which is eight in every Pulse race mode. Duplicated rather than
/// imported from `oag_gameplay::MAX_SHIPS` because this crate must not depend on
/// the simulation; the two are compared at compile time on the game crate's side,
/// where both are visible.
pub const MAX_TRAILS: usize = 8;

/// How many camera-facing quads [`Pipeline`]'s buffer holds.
///
/// **The flare itself is one per craft**, and one is what the original writes per
/// craft: not one per layer, because the three staggered ramps are the ribbon's
/// and `ExhaustFlare_Draw` writes exactly four vertices sharing one colour.
///
/// The rest of the budget is [`sprite`]'s, which is **not** recovered - it is
/// there so a caller with something else to billboard reuses this pipeline
/// rather than standing up a second one. Sized for a projectile and a blast
/// flash per slot of `oag_weapons::projectile::MAX_PROJECTILES`, which is 16;
/// the number is duplicated rather than imported for the same reason
/// [`MAX_TRAILS`] is.
///
/// So the whole budget, and the arithmetic to redo before appending anything
/// else here:
///
/// | source | quads |
/// | --- | ---: |
/// | one engine flare per craft ([`MAX_TRAILS`]) | 8 |
/// | one sprite per projectile in flight | 128 |
/// | a smoke puff per trail sample per projectile (8 x 128) | 1024 |
/// | three puffs per blast flash (3 x 128) | 384 |
/// | **total** | **1544** |
///
/// `oag_weapons::projectile::MAX_PROJECTILES` is 128 - sized for an Eliminator
/// race rather than for the original's 48-slot pool, since the hardware this
/// runs on is not a 2007 handheld. At six vertices a quad this buffer is about
/// half a megabyte, allocated once; only the quads actually produced are
/// uploaded each frame.
///
/// **A too-small budget here truncates silently** - [`Pipeline::upload`] clamps
/// with `min` - which is exactly how the projectile sprites first came out
/// invisible with nothing in the logs. Anything appending to this buffer has to
/// be counted here.
pub const MAX_SPRITES: usize = 1552;

/// The maximum vertices [`Pipeline`]'s buffer holds, six per [`MAX_SPRITES`].
pub const MAX_VERTICES: usize = MAX_SPRITES * 6;

/// The flares alone must not fill the budget, or a caller's [`sprite`] is
/// truncated by [`Pipeline::upload`]'s `min` with nothing to show for it.
///
/// A **compile-time** assertion rather than a test, because the failure it
/// guards against is invisible at runtime: the picture is simply missing the
/// thing that was appended. The right-hand side is [`MAX_SPRITES`]' own table:
/// one flare per craft, then per projectile slot a head sprite, eight trail
/// puffs and three blast puffs.
const _: () = assert!(MAX_SPRITES >= MAX_TRAILS + (1 + 8 + 3) * 128);

/// One craft's ribbon: three layers of `TRAIL_SAMPLES - 1` segments, each a
/// four-quad diamond tube.
pub const TRAIL_VERTICES_PER_CRAFT: usize = LAYERS * (TRAIL_SAMPLES - 1) * TRAIL_FINS * 6;

/// The ribbon's vertex budget: the larger of the PSP ribbon's
/// [`TRAIL_VERTICES_PER_CRAFT`] and HD's [`hd::VERTICES_PER_CRAFT`], for every
/// craft on the grid - all eight trail at once and they share one buffer.
pub const MAX_TRAIL_VERTICES: usize = MAX_TRAILS
    * if TRAIL_VERTICES_PER_CRAFT > hd::VERTICES_PER_CRAFT {
        TRAIL_VERTICES_PER_CRAFT
    } else {
        hd::VERTICES_PER_CRAFT
    };

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
/// same target format, same [`oag_mesh::mesh_render::DEPTH_FORMAT`], sample count 1,
/// and the same 128-byte uniform block so
/// [`oag_mesh::mesh_render::UNIFORMS_SIZE`] describes both.
pub mod hd;
mod texture;
pub use texture::FlareTexture;

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
    /// `oag_texture::texture`. `format` must match the caller's render pass and
    /// `sample_count` its multisample state - see `mesh_render::build`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        flare: &FlareTexture,
        noise: &FlareTexture,
        trail_shape: Option<&FlareTexture>,
        trail_blend: wgpu::BlendState,
        sample_count: u32,
        velocity: oag_mesh::mesh_render::Velocity,
        trail_stamps_mask: bool,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("exhaust"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/exhaust.wgsl")).into(),
            ),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("exhaust"),
            bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        // **The ribbon's second texture is a pipeline constant, not a uniform.**
        // Only a title that authors its ribbon supplies one, and whether the
        // shader reads slot 2 is a property of that title's asset rather than
        // of the frame - the same shape as `linear_out` beside it. A pipeline
        // built without one never samples the view bound there.
        let mut trail_constants = oag_mesh::mesh_render::linear_constants(format).to_vec();
        if trail_shape.is_some() {
            trail_constants.push(("trail_shape", 1.0));
        }
        let build_pipeline = |label: &str,
                              blend: wgpu::BlendState,
                              write_mask: wgpu::ColorWrites,
                              constants: &[(&str, f64)]| {
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
                    // The velocity target, when the race adds one, rides
                    // along **write-masked empty**: a flare or ribbon quad is
                    // rebuilt from scratch every draw with no frame-to-frame
                    // vertex correspondence, so there is no previous position
                    // to compute a velocity from, and these draws write no
                    // depth either - the velocity at their pixels stays the
                    // surface's behind them. See `mesh_render::Velocity` and
                    // `docs/rendering/motion-blur.md`.
                    targets: &{
                        let mut targets = vec![Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(blend),
                            write_mask,
                        })];
                        targets.extend(velocity.target(true));
                        targets
                    },
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants,
                        ..Default::default()
                    },
                }),
                primitive: wgpu::PrimitiveState {
                    // A camera-facing quad has no meaningful winding: the basis it
                    // is built from flips as the camera orbits. The original
                    // disables cull for both the flare and the ribbon.
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: oag_mesh::mesh_render::DEPTH_FORMAT,
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
        // **The flare writes colour only, and that is a deliberate limit on the
        // evidence rather than an oversight.** The ribbon's write to the bloom's
        // glow mask is explicit and recovered - `Trail_BuildStateList` opens the
        // channel and stamps a value through `sceGuStencilOp(..., REPLACE)`.
        // `ExhaustFlare_BuildDisplayList` only calls `Gu_PixelMask(0)`, which
        // opens every channel but is also just the default state, so "the flare
        // opts into the mask" is an inference from a state reset, not a read of
        // a write. Acting on it makes the flare's own alpha accumulate under
        // `BLEND` (`SrcAlpha`/`One`) across a quad that is `6.5` world units
        // wide during a boost, saturating the mask over most of the lower frame
        // and blowing the whole picture out - measured 2026-08-10. **Now read:
        // the flare writes no mask.** Its list disables the stencil test, and
        // the GE writes alpha only through a stencil op - see
        // `docs/rendering/glow-mask.md`. Only the ribbon feeds the bloom.
        let pipeline = build_pipeline(
            "exhaust",
            BLEND,
            wgpu::ColorWrites::COLOR,
            oag_mesh::mesh_render::linear_constants(format),
        );
        // The caller's, not [`TRAIL_BLEND`]: a title that authors its ribbon
        // passes its material's own pair - `race::Loaded::trail_blend`.
        // The PSP's ribbon stamps its ramp, the PS2's is in no mask.
        let trail_writes = if trail_stamps_mask {
            wgpu::ColorWrites::ALL
        } else {
            wgpu::ColorWrites::COLOR
        };
        let trail_pipeline =
            build_pipeline("exhaust trail", trail_blend, trail_writes, &trail_constants);

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exhaust uniforms"),
            size: oag_mesh::mesh_render::UNIFORMS_SIZE,
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

        let texture = flare.bind(device, queue, &texture_layout, "engine flare", false, None);
        let noise = noise.bind(
            device,
            queue,
            &texture_layout,
            "engine noise",
            true,
            trail_shape,
        );

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
        camera: Vec3,
        vertices: &[GpuVertex],
        trail: &[GpuVertex],
    ) {
        // The vertex shader reads only `model`'s fourth column, which carries
        // the camera's world position for the HD ribbon's facing fade - its
        // program interpolates `eyePositionWorldSpace - position`. The rest of
        // the matrix stays identity so the shared block is well-defined.
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [camera.x, camera.y, camera.z, 1.0];
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

mod sprite;
use sprite::quad;
pub use sprite::sprite;

#[cfg(test)]
mod tests;
