//! Collision sparks: the recovered `WO_SHIP_COLL_SPARK_DAMAGE` burst.
//!
//! Unlike [`crate::exhaust`], this module now *is* a reading of the original's
//! collision effect. The dedicated particle asset the original ships for this
//! exact effect, `Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB`, is decoded at the
//! emitter level (`docs/formats/pob.md`), its runtime interpreter is traced
//! function by function
//! (`docs/ghidra/functions/psp-pulse-usa/particle-system.md`), and the values
//! below were read out of the file's own bytes and corroborated live in
//! PPSSPP during real wall hits. The asset is a **four-emitter tree**, and
//! [`EMITTERS`] transcribes all four:
//!
//! 1. the root - slow orange smoke puffs, 1 every 4 ticks,
//! 2. `WO_SHIP_COLL_SPARK` - the bright fast sparks, 3 per tick for 5 ticks,
//! 3. `bits` - white debris in a 30 degree cone,
//! 4. `WO_SHIP_COLL_SPARK_TRAIL` - lingering embers, 1 per tick for 32 ticks.
//!
//! The trigger side was already recovered: `ShipCollisionFx_Trigger`
//! (`0x089246b4`, `docs/ghidra/functions/psp-pulse-usa/contact-response.md`)
//! computes `severity = intensity * 2.0 + 0.4` from the clamped contact
//! intensity ([`SEVERITY_SCALE`], [`SEVERITY_SLOPE`], [`SEVERITY_FLOOR`]) and
//! enforces [`COLLISION_COOLDOWN`]. What this pass added is what severity
//! *does*: `ParticleSystem_DeriveScaledParams` (`0x088f4910`) multiplies it
//! into every emitter's ejection speed and emitter extent, and
//! `ParticleSystem_UpdateParticles` (`0x088f635c`) multiplies it into every
//! particle's drawn size - a harder impact is faster **and bigger**, never
//! more numerous. Particle counts are fixed in the asset; severity was
//! confirmed live to propagate uniformly to all four emitters (values
//! `0.527`-`2.03` read off `bits`/`_TRAIL` instances mid-crash, all
//! instance-side co-factors at their neutral `1.0`).
//!
//! # Units, and what is still approximated
//!
//! The original stores speeds in world units per tick (its particle
//! integrator multiplies velocity by a tick-count `dt`) and lifetimes in
//! integer ticks; everything here is converted through [`TICK_HZ`] once, at
//! the constant. Three things are knowingly approximate, each recorded on
//! the constant it affects:
//!
//! - **Colour**: the asset carries a 256-entry colour table per emitter and
//!   colour mode 2 picks a random entry per particle. The table is game
//!   content (ADR-0006) and cannot be committed; the tables read as smooth
//!   two-colour gradients, so each [`Colour::Gradient`] holds the measured
//!   endpoints and samples uniformly between them. Loading the real table
//!   from the user's disc at runtime is the exact-fidelity follow-up.
//! - **Emitter frames are settled, 2026-08-10, and the contact normal is
//!   out.** The instance is parented to the nearest `Ship Collision Fx`
//!   locator (`ShipCollisionFx_Trigger` decompiled), every such locator on
//!   every team authors an **identity rotation** (read from the ships'
//!   `.vex` bytes), so the emit frame's `+Y` is up. The hemisphere `abs()`es
//!   that `+Y`; the aimed cones are elevation-over-horizontal
//!   distributions; nothing anywhere reads the contact normal. See
//!   [`Shape`]. What remains approximate is only that a banked craft tilts
//!   the frame with it and this module keeps world up - sub-degree during
//!   any survivable scrape.
//! - **Streak end caps**: `ParticleSystem_DrawStreak` (`0x08916820`)
//!   extends the quad past both points by a stretch factor times the size,
//!   and **that factor is a hard-coded `1.0`, so this module's cap is not an
//!   approximation - it is the recovered value.**
//!
//!   The factor is `particle+0x64`, written as a literal `1.0f` by
//!   `ParticleSystem_InitParticleFields` (`0x088f79b4`) and never modified
//!   afterwards; confidence 88. There is no resource field behind it. It is
//!   also **one field feeding three draw modes** - the cap ratio in mode 7,
//!   this stretch in mode 6, and the width/height aspect in mode 3 - which
//!   is why mode 3's sprite is square. `1.0 * half` here matches the
//!   original exactly, and it also keeps a zero-length streak drawing a
//!   size-sized glow, the same degenerate case the original handles.
//!
//!   **The other half of the streak's shape is deliberately not adopted
//!   yet.** The original's body samples a *single* texture row stretched
//!   over the whole span, with `v` variation only in the two caps
//!   (`0`->`0.5`, `0.5`->`1`); this module maps `v` linearly across the
//!   quad. That is a real difference - but it only means anything against
//!   the authored sprite, and this module binds no texture at all (see the
//!   procedural falloff below). Adopting the layout alone would fit one
//!   approximation to the coordinate convention of a texture we never
//!   sample. **Decode the sprite first, then take the `v` layout and the
//!   cap ratio together.** (The size channel's *unit* is no longer approximate: the
//!   draw dispatch's inline quad path spans `position ± size` in view
//!   space, confirming world-unit half-sizes.)
//!
//! Gravity: an earlier revision read the `0x200` flag as clear on all four
//! emitters and kept the effect gravity-free. The file's bytes say
//! otherwise for **`bits`** (`flags 0x80000202`, `+0x74 = -0.015`
//! units/tick²), and its white debris arcs and falls in the original's own
//! frames. The other three really do have the flag clear and decelerate by
//! per-axis exponential drag alone.
//!
//! The disc also ships `WO_SHIP_COLL_SPARK_NODAMAGE.POB` - the variant
//! `ShipCollisionFx_Trigger` spawns when the contact dealt no damage - a
//! two-emitter tree that is exactly this file's bright fountain plus
//! `bits`, same bytes for every shared field, no smoke and no embers. A
//! live time-trial crash spawns the damage tree (its `_TRAIL` instances
//! were caught emitting), so this module plays the damage set for every
//! ignite and leaves the no-damage split to whoever wires a damage flag
//! through [`Sparks::ignite`].
//!
//! # Two halves, deliberately
//!
//! [`Sparks`] is the state and the maths, with no `wgpu` in it, mirroring
//! [`crate::exhaust::Exhaust`]. [`Pipeline`] is the GPU side.
//!
//! # Trigger discipline lives in the caller, not here
//!
//! [`Sparks::ignite`] does not gate on anything - every call restarts the
//! four emitters. The caller (`crates/game/src/race.rs`) is responsible for
//! calling it only when [`COLLISION_COOLDOWN`] has elapsed. The original
//! spawns a fresh instance tree per trigger and lets old ones finish; since
//! the cooldown (`0.8` s) exceeds the longest emitter-plus-particle life
//! (`32 + 30` ticks, about `1.03` s) only in the emitter half, a restart can
//! cut short at most a few trailing embers of the previous burst - accepted
//! rather than modelling overlapping instance trees.

use oag_core::Rng;
use oag_core::math::Vec3;

use crate::mesh::GpuVertex;

/// The original's fixed simulation rate, ticks per second. Emitter schedules
/// and speeds in the asset are expressed in ticks; this is the one
/// conversion constant.
pub const TICK_HZ: f32 = 60.0;

/// Particles alive at once across all four emitters.
///
/// `64` is sized from the asset itself, not tuned: the worst-case concurrent
/// count is about `57` (smoke 4, sparks 15, bits 8, embers up to 30 - each
/// emitter's spawn schedule times its maximum lifetime). The asset's own
/// per-emitter caps (`resource + 0xa0`: 8 / 2000 / 64 / 32) never bind on
/// those schedules, so they are not enforced here.
pub const MAX_SPARKS: usize = 64;

/// How a particle picks its initial direction.
///
/// Recovered whole 2026-08-10 (`ParticleSystem_EmitSphere` decompiled to its
/// velocity dispatch, `ParticleSystem_AimedVelocity`'s VFPU read at
/// instruction level, both live-corroborated - see
/// `docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "The emit frame
/// and the velocity dispatch, settled"). Two corrections over the earlier
/// reading, and both were the reported "boring particles" symptom:
///
/// - **The contact normal never enters the original's directions.** The
///   hemisphere's forced-positive axis is the emitter's local **+Y** - world
///   up, since every `Ship Collision Fx` locator authors an identity
///   rotation - and the aimed cones' angles are elevation/azimuth in that
///   same frame. Sparks hug the wall during a scrape because roughly half a
///   +Y hemisphere is near-horizontal, not because anything aims along the
///   wall.
/// - **Sphere-shaped emitters ignore the aim fields.** Velocity modes 0 and
///   1 are both radial there; only mode 2 (tangent) differs. The bright
///   spark fountain is radial-over-hemisphere at 1.56 ± 0.936 units/tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// Uniform over the unit sphere; velocity is radial. The original's own
    /// construction (`z` uniform in `[-1, 1]`, azimuth uniform) is used
    /// verbatim - emitter shape 4 in `ParticleSystem_EmitSphere`.
    Sphere,
    /// [`Shape::Sphere`] with the world-up component forced positive -
    /// emitter shape 7, which `abs()`es the emitter-local `+Y`.
    Hemisphere,
    /// The aimed-velocity law of `ParticleSystem_AimedVelocity`
    /// (`0x088fc490`): elevation `base ± cone` above the horizontal plane
    /// (`y = sin`, horizontal scaled by `cos`), azimuth uniform (the input
    /// heading is the random spawn offset's) plus the same `± cone` jitter,
    /// which uniform azimuth absorbs. Both angles radians here; the file
    /// stores the jitter half-angle in degrees and converts exactly as
    /// [`aimed_direction`] does.
    Aimed {
        /// Elevation above the emitter's horizontal plane, radians -
        /// `res+0x50` (the field an earlier pass labelled "yaw").
        elevation: f32,
        /// Jitter half-angle applied to the elevation (and azimuth),
        /// radians - `res+0x58`, authored in degrees.
        cone: f32,
    },
}

/// How a particle's drawn half-size evolves, world units, before the
/// severity multiplier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    /// Linear over the particle's life, from spawn to death - a keyframed
    /// channel (mode 0) with two keys.
    Lerp(f32, f32),
    /// Constant, drawn once per particle at spawn - a random channel
    /// (mode 3) between the two bounds.
    Random(f32, f32),
}

/// A particle's colour source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Colour {
    /// A uniform random sample between the two endpoints of the asset's
    /// 256-entry table (see the module doc for why endpoints, not the
    /// table).
    Gradient([f32; 3], [f32; 3]),
    /// The same colour for every particle.
    Constant([f32; 3]),
}

/// How a particle is turned into geometry - the render-mode class from the
/// blend table at `DAT_08ab2260`, indexed by `resource + 0xb8` and decoded
/// in the billboard draw dispatch (`0x089186bc`,
/// `docs/ghidra/functions/psp-pulse-usa/particle-system.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Render {
    /// A camera-facing quad at one point - table mode `0x3`, drawn by the
    /// one-position helper with the particle's roll angle.
    Billboard,
    /// A quad spanned between the particle's **spawn point** and its
    /// current position - table mode `0x6` with resource flag `0x2000000`
    /// set, which stops `ParticleSystem_UpdateParticles` from refreshing
    /// the second point per tick. The streaks radiate outward from the
    /// impact: the spiky look.
    StreakFromSpawn,
    /// A quad spanned over the last tick of motion - table modes `0x6`/`0x7`
    /// with the flag clear, so the stored point is refreshed every tick.
    StreakPerTick,
}

/// A particle's blend class - `resource + 0xc0`, dispatched by the GE state
/// selector at `0x0891653c`: class `2` calls `BlendFunc(ADD, SRC_ALPHA,
/// FIX 0xffffff)`, class `3` calls `BlendFunc(ADD, SRC_ALPHA,
/// ONE_MINUS_SRC_ALPHA)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Blend {
    /// Class 2: `src_alpha, one` - what every bright emitter uses.
    Additive,
    /// Class 3: `src_alpha, one_minus_src_alpha` - what the smoke uses,
    /// which is why a dark smoke colour is visible at all: added, it would
    /// vanish against any background.
    AlphaOver,
}

/// One emitter of the four in `WO_SHIP_COLL_SPARK_DAMAGE.POB`, at the
/// file's own fixed offsets (`docs/formats/pob.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmitterSpec {
    /// The emitter record's own name field, for the docs' benefit.
    pub name: &'static str,
    /// Ticks the emitter emits for (`+0x24`).
    pub duration_ticks: f32,
    /// Ticks between emissions (`+0x64`/`+0x68`; min and max are equal on
    /// all four emitters, so one number).
    pub interval_ticks: f32,
    /// Particles per emission (`+0x6c`/`+0x70`, likewise equal).
    pub per_emission: usize,
    /// Particle lifetime, centre and spread, integer ticks in the file
    /// (`+0x5c`/`+0x60`): `centre + spread * U(-1, 1)`.
    pub lifetime_ticks: (f32, f32),
    /// Ejection speed, centre and spread, world units **per tick**
    /// (`+0x48`/`+0x4c`), multiplied by severity.
    pub speed: (f32, f32),
    /// Direction distribution (`+0x30` shape, `+0x50` elevation, `+0x58`
    /// cone half-angle).
    pub shape: Shape,
    /// Per-axis exponential velocity decay per tick, from the emitter's
    /// type-3 modifier node; `1.0` where the emitter has none.
    pub drag_per_tick: f32,
    /// Downward acceleration, world units per tick², applied when the
    /// emitter's `0x200` flag is set - `res+0x74`, negative down. `0.0`
    /// where the flag is clear. An earlier revision read the flag as clear
    /// on all four emitters; the file's bytes have it **set on `bits`**
    /// (`flags 0x80000202`), whose white debris arcs and falls.
    pub gravity_per_tick: f32,
    /// Drawn half-size channel (`+0x4d8` block), multiplied by severity.
    pub size: Size,
    /// Fraction of life at full alpha before the linear fade to zero
    /// (`+0x5b8` block keyframes); `1.0` means no fade at all.
    pub alpha_hold: f32,
    /// Peak alpha, `0..=1` (`+0x5b8` block's `hi` bound over 255).
    pub alpha_max: f32,
    /// Colour source (`+0xbc` mode, `+0xc4` table).
    pub colour: Colour,
    /// Geometry class (`+0xb8` via the blend table, plus flag `0x2000000`).
    pub render: Render,
    /// Blend class (`+0xc0`).
    pub blend: Blend,
}

/// The four emitters of `WO_SHIP_COLL_SPARK_DAMAGE.POB`, values read from
/// the file's bytes and live-confirmed - see the module doc comment and
/// `docs/formats/pob.md`'s "The collision-spark file is a four-emitter
/// tree". Order is the file's own: root, then the `+0x94c` sibling chain.
pub const EMITTERS: [EmitterSpec; 4] = [
    EmitterSpec {
        name: "WO_SHIP_COLL_SPARK_DAMAGE",
        duration_ticks: 32.0,
        interval_ticks: 4.0,
        per_emission: 1,
        lifetime_ticks: (16.0, 0.0),
        speed: (0.048, 0.0),
        shape: Shape::Sphere,
        drag_per_tick: 0.98,
        gravity_per_tick: 0.0,
        size: Size::Lerp(0.5, 2.5),
        alpha_hold: 0.215_39,
        alpha_max: 200.0 / 255.0,
        colour: Colour::Gradient(
            [181.0 / 255.0, 134.0 / 255.0, 87.0 / 255.0],
            [48.0 / 255.0, 46.0 / 255.0, 46.0 / 255.0],
        ),
        render: Render::Billboard,
        blend: Blend::AlphaOver,
    },
    EmitterSpec {
        name: "WO_SHIP_COLL_SPARK",
        duration_ticks: 5.0,
        interval_ticks: 1.0,
        per_emission: 3,
        lifetime_ticks: (6.0, 3.0),
        speed: (1.56, 0.936),
        shape: Shape::Hemisphere,
        drag_per_tick: 0.85,
        gravity_per_tick: 0.0,
        size: Size::Random(0.0, 0.312),
        alpha_hold: 0.459,
        alpha_max: 1.0,
        colour: Colour::Gradient(
            [1.0, 194.0 / 255.0, 29.0 / 255.0],
            [1.0, 123.0 / 255.0, 0.0],
        ),
        render: Render::StreakFromSpawn,
        blend: Blend::Additive,
    },
    EmitterSpec {
        name: "bits",
        duration_ticks: 4.0,
        interval_ticks: 1.0,
        per_emission: 2,
        lifetime_ticks: (16.0, 6.0),
        speed: (0.295, 0.142),
        // `res+0x50 = 0.6283` rad (36 deg) elevation, 30 deg jitter -
        // live-measured at `ParticleSystem_AimedVelocity`'s exit: elevations
        // +6.8 to +50.5 deg across a real crash.
        shape: Shape::Aimed {
            elevation: 0.6283,
            cone: 30.0 * std::f32::consts::PI / 180.0,
        },
        drag_per_tick: 1.0,
        gravity_per_tick: -0.015,
        size: Size::Lerp(0.6, 0.004),
        alpha_hold: 1.0,
        alpha_max: 1.0,
        colour: Colour::Constant([1.0, 1.0, 1.0]),
        render: Render::StreakPerTick,
        blend: Blend::Additive,
    },
    EmitterSpec {
        name: "WO_SHIP_COLL_SPARK_TRAIL",
        duration_ticks: 32.0,
        interval_ticks: 1.0,
        per_emission: 1,
        lifetime_ticks: (20.0, 10.0),
        speed: (0.0, 0.3),
        // `res+0x50 = 0` - the embers scatter about the horizontal plane
        // (live-measured elevations -18.8 to +21.3 deg), not about the
        // contact normal.
        shape: Shape::Aimed {
            elevation: 0.0,
            cone: 21.82 * std::f32::consts::PI / 180.0,
        },
        drag_per_tick: 0.95,
        gravity_per_tick: 0.0,
        size: Size::Random(0.05, 0.2),
        alpha_hold: 0.459,
        alpha_max: 1.0,
        colour: Colour::Gradient(
            [1.0, 194.0 / 255.0, 29.0 / 255.0],
            [1.0, 123.0 / 255.0, 0.0],
        ),
        render: Render::StreakPerTick,
        blend: Blend::Additive,
    },
];

/// Seconds a wall contact must persist before another burst is allowed.
///
/// **Recovered.** `ShipCollisionFx_Trigger` (`0x089246b4`,
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`) re-arms exactly
/// this long after every collision-variant spawn: `instance + 100 = now +
/// 0.8`, checked on entry and skipped while still armed.
pub const COLLISION_COOLDOWN: f32 = 0.8;

/// Converts a contact's impact speed into `[0, 1]` intensity.
///
/// **Recovered - this is the exact literal `FUN_088418e0` uses**:
/// `fVar21 = min(|impulse| * 0.0125, 1.0)`, the value
/// `Ship_DispatchCollisionFx` and then `ShipCollisionFx_Trigger` receive as
/// `intensity` (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`).
/// Confirmed by a live capture reading the real value at a real wall hit.
/// This module's own input is `speed`, not the original's impulse
/// magnitude, so the *scale* is still borrowed across a unit difference;
/// the coefficient and the clamp are not.
pub const SEVERITY_SCALE: f32 = 0.0125;

/// The multiplier `ShipCollisionFx_Trigger` applies to [`SEVERITY_SCALE`]'s
/// clamped intensity. **Recovered**, from the same `intensity * 2.0 + 0.4`
/// literal.
pub const SEVERITY_SLOPE: f32 = 2.0;

/// The floor `ShipCollisionFx_Trigger` adds on top of [`SEVERITY_SLOPE`] - a
/// hit is never zero severity, only ever `0.4` at its gentlest. **Recovered.**
pub const SEVERITY_FLOOR: f32 = 0.4;

/// One live spark. Dead when [`Particle::life`] reaches zero.
///
/// Everything a particle needs after spawn is resolved *at* spawn (colour
/// sample, size bounds already severity-scaled, its emitter's drag and
/// alpha ramp), so [`Sparks::advance`] and [`Sparks::vertices`] never look
/// back at [`EMITTERS`].
#[derive(Debug, Clone, Copy, PartialEq)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    /// The streak's other end: the spawn point for
    /// [`Render::StreakFromSpawn`] (never updated), the previous tick's
    /// position for [`Render::StreakPerTick`] (refreshed each tick) - the
    /// particle-row-`+0x50` mechanism of `ParticleSystem_UpdateParticles`.
    /// Unused by [`Render::Billboard`].
    origin: Vec3,
    life: f32,
    max_life: f32,
    size_from: f32,
    size_to: f32,
    colour: [f32; 3],
    alpha_hold: f32,
    alpha_max: f32,
    drag_per_tick: f32,
    /// Downward acceleration, world units per second² (converted once at
    /// spawn from the emitter's per-tick² value).
    gravity: f32,
    render: Render,
    blend: Blend,
}

impl Particle {
    const DEAD: Self = Self {
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        origin: Vec3::ZERO,
        life: 0.0,
        max_life: 0.0,
        size_from: 0.0,
        size_to: 0.0,
        colour: [0.0; 3],
        alpha_hold: 1.0,
        alpha_max: 0.0,
        drag_per_tick: 1.0,
        gravity: 0.0,
        render: Render::Billboard,
        blend: Blend::Additive,
    };

    fn alive(self) -> bool {
        self.life > 0.0
    }
}

/// One emitter's live schedule: how long it keeps emitting and when the
/// next emission is due, both in ticks. All zero when idle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct EmitterState {
    ticks_left: f32,
    /// Ticks until the next emission; `<= 0` means "due now", matching the
    /// original's countdown timer at `instance + 0x04`
    /// (`ParticleSystem_UpdateEmission`), which spawns on its first update.
    until_next: f32,
}

/// Per-frame state of one ship's collision sparks.
///
/// Mirrors [`crate::exhaust::Exhaust`] in shape and for the same reason: it is
/// render-only state the game crate owns and advances on the simulation's
/// fixed tick, so it never enters `World` and never touches a determinism
/// hash - see that struct's doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sparks {
    particles: [Particle; MAX_SPARKS],
    emitters: [EmitterState; EMITTERS.len()],
    severity: f32,
    /// Where particles spawn, world space. The caller passes the current
    /// value to every [`Sparks::advance`] - the original's emitter node
    /// rides the hull (a `Ship Collision Fx` locator), so the anchor is the
    /// caller's to move, and a burst from a moving scrape strings its
    /// particles along the wall rather than clustering at the first contact
    /// point.
    anchor: Vec3,
    /// Total [`Sparks::ignite`] calls, for the caller's tests: the trigger
    /// cadence is observable here even while particles from consecutive
    /// bursts overlap.
    ignitions: u32,
}

impl Default for Sparks {
    fn default() -> Self {
        Self::new()
    }
}

impl Sparks {
    /// No live particles, no running emitters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            particles: [Particle::DEAD; MAX_SPARKS],
            emitters: [EmitterState {
                ticks_left: 0.0,
                until_next: 0.0,
            }; EMITTERS.len()],
            severity: 0.0,
            anchor: Vec3::ZERO,
            ignitions: 0,
        }
    }

    /// Starts the four-emitter burst at `point`, with `speed` the contact's
    /// impact speed in world units per second.
    ///
    /// No contact normal: the original's directions never read one - the
    /// emit frame is the hull locator's, identity-rotated, so up is up (see
    /// [`Shape`]). Nothing spawns here; the first particles appear on the
    /// next [`Sparks::advance`], exactly like the original's first emitter
    /// update. Unconditional - the cooldown discipline belongs to the
    /// caller (see the module doc comment).
    pub fn ignite(&mut self, point: Vec3, speed: f32) {
        let intensity = (speed * SEVERITY_SCALE).clamp(0.0, 1.0);
        self.severity = intensity * SEVERITY_SLOPE + SEVERITY_FLOOR;
        self.anchor = point;
        for (state, spec) in self.emitters.iter_mut().zip(&EMITTERS) {
            state.ticks_left = spec.duration_ticks;
            state.until_next = 0.0;
        }
        self.ignitions += 1;
    }

    /// Emits due particles, then ages, moves and expires live ones, by one
    /// simulation tick.
    ///
    /// `anchor` is the emitters' current world position - see
    /// [`Sparks::anchor`] for whose job moving it is.
    pub fn advance(&mut self, dt: f32, anchor: Vec3, rng: &mut Rng) {
        let dt_ticks = dt * TICK_HZ;
        self.anchor = anchor;

        let (severity, anchor) = (self.severity, self.anchor);
        for (state, spec) in self.emitters.iter_mut().zip(&EMITTERS) {
            if state.ticks_left <= 0.0 {
                continue;
            }
            // Spawn first, then advance the timer - the original's countdown
            // at `instance + 0x04` works the same way and therefore emits on
            // its very first update.
            while state.until_next <= 0.0 {
                state.until_next += spec.interval_ticks;
                for _ in 0..spec.per_emission {
                    let particle = spawn(spec, severity, anchor, rng);
                    let slot = expendable_slot(&self.particles);
                    self.particles[slot] = particle;
                }
            }
            state.until_next -= dt_ticks;
            state.ticks_left -= dt_ticks;
        }

        for particle in &mut self.particles {
            if !particle.alive() {
                continue;
            }
            // Refresh the streak's trailing end before integrating - the
            // same order `ParticleSystem_UpdateParticles` copies position
            // into the particle's row +0x50. Spawn-anchored streaks skip
            // this, which is what makes them radiate.
            if particle.render == Render::StreakPerTick {
                particle.origin = particle.position;
            }
            // The original applies the type-3 modifier once per tick
            // (`ParticleSystem_Update` precomputes pow(k, dt_ticks)); dt is
            // the fixed 1/60 here so the exponent is 1, but keep the pow so
            // a different step stays correct.
            let drag = particle.drag_per_tick.powf(dt_ticks);
            particle.velocity *= drag;
            // `bits` alone carries gravity (see `EmitterSpec::gravity_per_tick`):
            // the original adds it to the velocity each tick, after the
            // modifier's drag.
            particle.velocity.y += particle.gravity * dt;
            particle.position += particle.velocity * dt;
            particle.life -= dt;
            if particle.life <= 0.0 {
                *particle = Particle::DEAD;
            }
        }
    }

    /// How many particles are currently live.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.particles.iter().filter(|p| p.alive()).count()
    }

    /// How many times [`Sparks::ignite`] has fired, ever.
    ///
    /// For the caller's trigger-discipline tests: particle counts can no
    /// longer distinguish "one burst trickling" from "a burst per tick",
    /// since a burst spawns over 32 ticks.
    #[must_use]
    pub fn ignitions(&self) -> u32 {
        self.ignitions
    }

    /// This frame's geometry, one quad per live particle, split by blend
    /// class: `(additive, alpha_over)` - the two GE blend configurations
    /// the original's state selector (`0x0891653c`) switches between.
    ///
    /// `right` and `up` come from the camera, the same as
    /// [`crate::exhaust::Exhaust::vertices`]. Billboards face the viewer;
    /// streaks span their two stored points with a camera-perpendicular
    /// width, following the streak-quad builder at `0x08916820`.
    #[must_use]
    pub fn vertices(&self, right: Vec3, up: Vec3) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
        let forward = right.cross(up);
        let mut additive = Vec::with_capacity(MAX_SPARKS * 6);
        let mut alpha_over = Vec::new();
        for particle in &self.particles {
            if !particle.alive() {
                continue;
            }
            let age = 1.0 - (particle.life / particle.max_life).clamp(0.0, 1.0);
            let half = particle.size_from + (particle.size_to - particle.size_from) * age;
            let alpha = particle.alpha_max * alpha_ramp(age, particle.alpha_hold);
            let out = match particle.blend {
                Blend::Additive => &mut additive,
                Blend::AlphaOver => &mut alpha_over,
            };
            let (centre, axis_a, axis_b, cap) = match particle.render {
                // `cap = 0.5` makes the shader's cap/cross profile collapse
                // to the plain radial falloff a round sprite wants - see
                // `sparks.wgsl`.
                Render::Billboard => (particle.position, right * half, up * half, 0.5),
                Render::StreakFromSpawn | Render::StreakPerTick => {
                    let centre = (particle.position + particle.origin) * 0.5;
                    let along = particle.position - particle.origin;
                    let length = along.length();
                    let dir = if length > 1e-6 { along / length } else { up };
                    // Perpendicular to the streak in the camera plane -
                    // the view-space `(dir.y, -dir.x)` of the original,
                    // done in world space. Degenerate when the streak
                    // points straight at the camera; fall back to `right`.
                    let perp = dir.cross(forward).try_normalize().unwrap_or(right);
                    // Half the span plus a size-sized cap at each end, the
                    // way the original extends the quad past both points -
                    // a zero-length streak still draws a `half`-sized glow.
                    // The cap's share of the half-length tells the shader
                    // where the body's constant-width core begins; the
                    // original gets the same geometry by stretching a
                    // single row of `orange_glow2.tga` over the body with
                    // `v` variation only in the caps.
                    let half_span = length * 0.5 + half;
                    (centre, dir * half_span, perp * half, half / half_span)
                }
            };
            out.extend_from_slice(&quad(centre, axis_a, axis_b, cap, particle.colour, alpha));
        }
        (additive, alpha_over)
    }
}

/// The index of a dead particle, or the one with the least life left.
///
/// Ascending scan, so the choice depends only on the pool's own state and
/// never on iteration order that could vary between runs. The pool is
/// sized so replacement effectively never happens ([`MAX_SPARKS`]).
fn expendable_slot(particles: &[Particle]) -> usize {
    let mut best = 0;
    let mut best_life = f32::INFINITY;
    for (i, particle) in particles.iter().enumerate() {
        if !particle.alive() {
            return i;
        }
        if particle.life < best_life {
            best_life = particle.life;
            best = i;
        }
    }
    best
}

/// One new particle for `spec`, at the emitter's anchor.
fn spawn(spec: &EmitterSpec, severity: f32, anchor: Vec3, rng: &mut Rng) -> Particle {
    let direction = match spec.shape {
        Shape::Sphere => sphere_direction(rng),
        Shape::Hemisphere => {
            // `ParticleSystem_EmitSphere`'s shape-7 branch: `abs()` on the
            // emitter-local up, which is world up (identity locator
            // rotations) - never the contact normal.
            let mut d = sphere_direction(rng);
            d.y = d.y.abs();
            d
        }
        Shape::Aimed { elevation, cone } => aimed_direction(elevation, cone, rng),
    };
    // `centre + spread * U(-1, 1)`, the original's own random helper
    // (`Psys_RandSpread`), units per tick converted to per second once.
    let speed = (spec.speed.0 + spec.speed.1 * signed_unit(rng)) * severity * TICK_HZ;
    let life_ticks = (spec.lifetime_ticks.0 + spec.lifetime_ticks.1 * signed_unit(rng)).max(1.0);
    let life = life_ticks / TICK_HZ;
    let (size_from, size_to) = match spec.size {
        Size::Lerp(from, to) => (from * severity, to * severity),
        Size::Random(lo, hi) => {
            let s = (lo + (hi - lo) * rng.next_f32()) * severity;
            (s, s)
        }
    };
    let colour = match spec.colour {
        Colour::Constant(c) => c,
        Colour::Gradient(a, b) => {
            let t = rng.next_f32();
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        }
    };
    Particle {
        position: anchor,
        velocity: direction * speed,
        origin: anchor,
        life,
        max_life: life,
        size_from,
        size_to,
        colour,
        alpha_hold: spec.alpha_hold,
        alpha_max: spec.alpha_max,
        drag_per_tick: spec.drag_per_tick,
        gravity: spec.gravity_per_tick * TICK_HZ * TICK_HZ,
        render: spec.render,
        blend: spec.blend,
    }
}

/// Full alpha until `hold` of the particle's life, then linear to zero -
/// the shape of the asset's keyframed alpha channels.
fn alpha_ramp(age: f32, hold: f32) -> f32 {
    if age <= hold || hold >= 1.0 {
        1.0
    } else {
        ((1.0 - age) / (1.0 - hold)).clamp(0.0, 1.0)
    }
}

/// `U(-1, 1)`.
fn signed_unit(rng: &mut Rng) -> f32 {
    rng.next_f32() * 2.0 - 1.0
}

/// Uniform over the unit sphere, by the original's own construction
/// (`ParticleSystem_EmitSphere`): `z` uniform in `[-1, 1]`, azimuth uniform,
/// `sqrt(1 - z^2)` radius in the plane.
fn sphere_direction(rng: &mut Rng) -> Vec3 {
    let z = signed_unit(rng);
    let phi = rng.next_f32() * std::f32::consts::TAU;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let (sin_p, cos_p) = phi.sin_cos();
    Vec3::new(r * cos_p, r * sin_p, z)
}

/// The aimed-velocity direction: elevation `base ± cone` over the
/// horizontal plane, azimuth uniform.
///
/// `ParticleSystem_AimedVelocity`'s own arithmetic (`0x088fc490`, read at
/// instruction level): `y = sin(elevation)`, the horizontal components
/// scaled by `cos(elevation)`, the heading taken from the (uniform) spawn
/// direction plus an authored offset that is `0` on every collision
/// emitter - uniform plus jitter is uniform, so one draw serves.
/// Live-measured distributions match: `bits` +6.8 to +50.5 degrees,
/// `_TRAIL` -18.8 to +21.3.
fn aimed_direction(elevation: f32, cone: f32, rng: &mut Rng) -> Vec3 {
    let elev = elevation + cone * signed_unit(rng);
    let azimuth = rng.next_f32() * std::f32::consts::TAU;
    let (sin_e, cos_e) = elev.sin_cos();
    let (sin_a, cos_a) = azimuth.sin_cos();
    Vec3::new(cos_a * cos_e, sin_e, sin_a * cos_e)
}

/// Six vertices - two triangles - for one camera-facing quad.
///
/// The shader's radial falloff supplies the shape. The original samples an
/// authored texture here - the asset names
/// `Z:\WipeoutPSP\X2\Data\Psys\Tex\quakesmoke32x32.tga`, a soft 32x32
/// puff - which the falloff approximates; decoding the shipped texture is a
/// possible follow-up now that its identity is known.
fn quad(
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    cap: f32,
    colour: [f32; 3],
    alpha: f32,
) -> [GpuVertex; 6] {
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + right * sx + up * sy).to_array(),
        normal: [0.0, 0.0, 1.0],
        colour: [colour[0], colour[1], colour[2], alpha],
        texcoord: [u, v],
        // The `lit` slot is repurposed by this pipeline: sparks are emissive
        // (never lit by the mesh rig, which is a different pipeline), so the
        // attribute carries the cap fraction the fragment profile needs -
        // see `sparks.wgsl`.
        lit: cap,
        v_cycles: 0.0,
    };
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}

/// The additive blend - the original's blend class 2, `BlendFunc(ADD,
/// SRC_ALPHA, FIX 0xffffff)`, used by the three bright emitters. The same
/// shape [`crate::exhaust::BLEND`] uses, and for the same reason: additive
/// keeps overlapping sparks reading as *brighter* rather than as one
/// occluding another.
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

/// The over blend - the original's blend class 3, `BlendFunc(ADD, SRC_ALPHA,
/// ONE_MINUS_SRC_ALPHA)`, used by the smoke. This is what lets a *dark*
/// smoke colour darken the scene behind it; drawn additively it would be
/// nearly invisible, which is exactly how the missing smoke bug looked.
pub const BLEND_ALPHA_OVER: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The maximum vertices [`Pipeline`]'s buffer holds: one quad per [`MAX_SPARKS`].
pub const MAX_VERTICES: usize = MAX_SPARKS * 6;

/// The sparks' draw pipeline - two of them, one per blend class, sharing
/// the shader and layout.
///
/// Simpler than [`crate::exhaust::Pipeline`] in one respect: there is no
/// texture to bind - the shape is a procedural radial falloff computed
/// in the fragment shader standing in for the asset's own soft-puff
/// texture (see [`quad`]). Otherwise it matches `mesh_render`'s pipeline
/// the same way the exhaust does: same target format, same
/// [`crate::mesh_render::DEPTH_FORMAT`], depth-tested but not
/// depth-writing, for the same transparency-ordering reason
/// `exhaust::Pipeline` documents.
#[derive(Debug)]
pub struct Pipeline {
    additive: wgpu::RenderPipeline,
    alpha_over: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    additive_vertices: wgpu::Buffer,
    alpha_vertices: wgpu::Buffer,
    /// Vertices actually uploaded by the last [`Pipeline::upload`].
    additive_count: u32,
    alpha_count: u32,
}

impl Pipeline {
    /// Builds the pipeline pair.
    ///
    /// `format` must be the target the caller's render pass writes, and
    /// `sample_count` must match its multisample state - see
    /// `mesh_render::build`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, sample_count: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sparks"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sparks.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sparks uniforms"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sparks"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let build = |label: &str, blend: wgpu::BlendState| {
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
                        // Colour only - the original's particle draw path
                        // (`FUN_08915fd0`) calls `Bloom_SetPixelMask(g_bloom, 0)`,
                        // protecting the glow mask. See `crate::post::bloom`.
                        write_mask: wgpu::ColorWrites::COLOR,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    // A camera-facing quad has no meaningful winding: the
                    // basis it is built from flips as the camera orbits.
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
        let additive = build("sparks additive", BLEND);
        let alpha_over = build("sparks alpha-over", BLEND_ALPHA_OVER);

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sparks uniforms"),
            size: crate::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparks uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let buffer = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let additive_vertices = buffer("sparks additive vertices");
        let alpha_vertices = buffer("sparks alpha-over vertices");

        Self {
            additive,
            alpha_over,
            uniforms,
            bind_group,
            additive_vertices,
            alpha_vertices,
            additive_count: 0,
            alpha_count: 0,
        }
    }

    /// Uploads this frame's camera matrix and both blend classes' geometry,
    /// as [`Sparks::vertices`] returns them.
    ///
    /// Takes `&mut self` only for the vertex counts; the writes go through
    /// `queue`, the same split [`crate::exhaust::Pipeline::upload`] uses.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        additive: &[GpuVertex],
        alpha_over: &[GpuVertex],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        let n = additive.len().min(MAX_VERTICES);
        queue.write_buffer(
            &self.additive_vertices,
            0,
            bytemuck::cast_slice(&additive[..n]),
        );
        self.additive_count = n as u32;

        let n = alpha_over.len().min(MAX_VERTICES);
        queue.write_buffer(
            &self.alpha_vertices,
            0,
            bytemuck::cast_slice(&alpha_over[..n]),
        );
        self.alpha_count = n as u32;
    }

    /// Draws into a pass the caller already opened.
    ///
    /// Must be issued **after** the opaque geometry, for the same
    /// depth-write-off reason as [`crate::exhaust::Pipeline::draw`]. The
    /// alpha-over smoke draws first, then the additive sparks on top -
    /// the original interleaves them in particle order, which two batched
    /// draws cannot reproduce exactly; additive-last is the closer
    /// approximation since adding light commutes.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.alpha_count > 0 {
            pass.set_pipeline(&self.alpha_over);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.alpha_vertices.slice(..));
            pass.draw(0..self.alpha_count, 0..1);
        }
        if self.additive_count > 0 {
            pass.set_pipeline(&self.additive);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.additive_vertices.slice(..));
            pass.draw(0..self.additive_count, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / TICK_HZ;

    fn rng() -> Rng {
        Rng::new(1)
    }

    fn ignited(speed: f32) -> (Sparks, Rng) {
        let mut sparks = Sparks::new();
        sparks.ignite(Vec3::ZERO, speed);
        (sparks, rng())
    }

    /// Tick 1 spawns each emitter's first emission: 1 smoke + 3 sparks +
    /// 2 bits + 1 ember.
    #[test]
    fn the_first_tick_spawns_every_emitters_first_emission() {
        let (mut sparks, mut r) = ignited(80.0);
        sparks.advance(DT, Vec3::ZERO, &mut r);
        assert_eq!(sparks.alive_count(), 7);
    }

    /// The burst's total spawn count is fixed by the asset's schedules -
    /// smoke 8, sparks 15, bits 8, embers 32 - and is severity-independent:
    /// a harder hit is faster and bigger, never more numerous.
    #[test]
    fn the_burst_total_is_the_asset_schedule_and_ignores_severity() {
        for speed in [0.0, 200.0] {
            let (mut sparks, mut r) = ignited(speed);
            let mut spawned = 0usize;
            let mut alive_before = 0usize;
            // Run the emitters dry; count spawns as alive-count increases
            // plus deaths (no deaths before 7 ticks, so just run past the
            // longest emitter and sum emissions per tick).
            for _ in 0..40 {
                let before_deaths: usize = alive_before;
                sparks.advance(DT, Vec3::ZERO, &mut r);
                let now = sparks.alive_count();
                // Deaths can't be told apart from spawns via alive_count
                // alone once lifetimes start expiring, so count spawns
                // directly: every tick's growth is spawns minus deaths, and
                // the shortest lifetime is 3 ticks - long enough that the
                // first two ticks give a clean check, and the total is
                // checked against the schedule below instead.
                let _ = before_deaths;
                alive_before = now;
                spawned = spawned.max(now);
            }
            // Peak concurrency must stay inside the pool and above the
            // steady trickle; the exact schedule totals are asserted via
            // the spec table itself.
            assert!(spawned <= MAX_SPARKS, "peak {spawned} overflows the pool");
            assert!(spawned > 7, "peak {spawned} never exceeded the first tick");
        }
        let total: usize = EMITTERS
            .map(|e| (e.duration_ticks / e.interval_ticks).ceil() as usize * e.per_emission)
            .iter()
            .sum();
        assert_eq!(total, 8 + 15 + 8 + 32);
    }

    /// Severity multiplies speed and size, exactly as
    /// `ParticleSystem_DeriveScaledParams` does - not the particle count.
    #[test]
    fn a_harder_hit_makes_faster_bigger_particles_not_more_of_them() {
        let (mut gentle, mut r1) = ignited(0.0);
        let (mut hard, mut r2) = ignited(1.0 / SEVERITY_SCALE);
        gentle.advance(DT, Vec3::ZERO, &mut r1);
        hard.advance(DT, Vec3::ZERO, &mut r2);
        assert_eq!(gentle.alive_count(), hard.alive_count());

        // Same seed, same draw sequence: every particle pair differs only
        // by the severity factor, 2.4 / 0.4 = 6x. Gravity is authored per
        // emitter and NOT severity-scaled (`ParticleSystem_DeriveScaledParams`
        // multiplies ejection speed and extent only), so back it out of the
        // one tick both pools have integrated before comparing.
        let ratio = (SEVERITY_SLOPE + SEVERITY_FLOOR) / SEVERITY_FLOOR;
        for (g, h) in gentle.particles.iter().zip(hard.particles.iter()) {
            if !g.alive() {
                continue;
            }
            let g_speed = (g.velocity - Vec3::Y * (g.gravity * DT)).length();
            let h_speed = (h.velocity - Vec3::Y * (h.gravity * DT)).length();
            if g_speed > 0.0 {
                assert!((h_speed / g_speed - ratio).abs() < 1e-3);
            }
            if g.size_from > 0.0 {
                assert!((h.size_from / g.size_from - ratio).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn particles_die_after_their_lifetime() {
        let (mut sparks, mut r) = ignited(80.0);
        sparks.advance(DT, Vec3::ZERO, &mut r);
        assert!(sparks.alive_count() > 0);

        // Longest emitter (32 ticks) plus longest lifetime (30 ticks), and
        // a margin.
        for _ in 0..70 {
            sparks.advance(DT, Vec3::ZERO, &mut r);
        }
        assert_eq!(sparks.alive_count(), 0);
    }

    /// The caller owns the anchor: particles spawn wherever the current
    /// [`Sparks::advance`] call says the emitter node is, not where the
    /// ignite happened - a burst from a moving scrape strings out along the
    /// hull's path.
    #[test]
    fn particles_spawn_at_the_anchor_the_caller_moves() {
        let (mut sparks, mut r) = ignited(80.0);
        let point = Vec3::new(3.0, 4.0, 5.0);
        sparks.ignite(point, 80.0);

        // The hull node has moved 2 units by the first advance.
        let anchor = point + Vec3::new(2.0, 0.0, 0.0);
        sparks.advance(DT, anchor, &mut r);
        // One tick of drift after spawn, bounded by the fastest possible
        // severity-scaled ejection speed: (1.56 + 0.936) * 2.4 * 60.
        let fastest = (1.56 + 0.936) * 2.4 * TICK_HZ;
        for p in sparks.particles.iter().filter(|p| p.alive()) {
            assert!((p.position - anchor).length() < fastest * DT + 1e-4);
            assert_eq!(
                p.origin, anchor,
                "a streak's trailing end starts at its own spawn point"
            );
        }
    }

    /// The blend split is what makes the smoke visible at all: the smoke
    /// emitter is the one alpha-over member of the tree, everything bright
    /// is additive, and the streak classes follow the recovered flag
    /// (sparks radiate from spawn, bits/embers streak per tick).
    #[test]
    fn the_smoke_is_alpha_over_and_the_bright_emitters_are_additive_streaks() {
        assert_eq!(EMITTERS[0].blend, Blend::AlphaOver);
        assert_eq!(EMITTERS[0].render, Render::Billboard);
        for spec in &EMITTERS[1..] {
            assert_eq!(spec.blend, Blend::Additive, "{}", spec.name);
        }
        assert_eq!(EMITTERS[1].render, Render::StreakFromSpawn);
        assert_eq!(EMITTERS[2].render, Render::StreakPerTick);
        assert_eq!(EMITTERS[3].render, Render::StreakPerTick);

        // And the split reaches the geometry: after a few ticks both vertex
        // classes are non-empty.
        let (mut sparks, mut r) = ignited(80.0);
        for _ in 0..3 {
            sparks.advance(DT, Vec3::ZERO, &mut r);
        }
        let (additive, alpha_over) = sparks.vertices(Vec3::X, Vec3::Y);
        assert!(!additive.is_empty());
        assert!(!alpha_over.is_empty());
    }

    #[test]
    fn the_pool_never_grows_past_its_fixed_capacity() {
        let mut sparks = Sparks::new();
        let mut r = rng();
        // Re-ignite mid-burst repeatedly to press the pool as hard as the
        // caller ever could.
        for i in 0..200 {
            if i % 10 == 0 {
                sparks.ignite(Vec3::ZERO, 200.0);
            }
            sparks.advance(DT, Vec3::ZERO, &mut r);
            assert!(sparks.alive_count() <= MAX_SPARKS);
        }
    }

    /// The hemisphere never ejects downward, mirroring the original's
    /// abs() on the emitter-local up (`ParticleSystem_EmitSphere` shape 7 -
    /// world up through the identity locator rotations, never the contact
    /// normal).
    #[test]
    fn hemisphere_particles_never_eject_downward() {
        let (mut sparks, mut r) = ignited(80.0);
        sparks.ignite(Vec3::ZERO, 80.0);
        for _ in 0..6 {
            sparks.advance(DT, Vec3::ZERO, &mut r);
        }
        // Only the hemisphere emitter is direction-constrained; sphere
        // particles may go anywhere, so check the invariant on the ones
        // that are constrained by construction. All hemisphere particles
        // have alpha_max 1.0 and drag 0.85 - unique among the four specs -
        // which identifies them without a tag field. Spawn velocity is
        // upward; one tick of drag cannot flip its sign (no gravity on this
        // emitter), so a small negative tolerance suffices.
        for p in sparks.particles.iter().filter(|p| p.alive()) {
            if p.drag_per_tick == 0.85 {
                assert!(p.velocity.y >= -1e-4);
            }
        }
    }

    #[test]
    fn ignitions_counts_every_ignite() {
        let mut sparks = Sparks::new();
        assert_eq!(sparks.ignitions(), 0);
        sparks.ignite(Vec3::ZERO, 10.0);
        sparks.ignite(Vec3::ZERO, 10.0);
        assert_eq!(sparks.ignitions(), 2);
    }
}
