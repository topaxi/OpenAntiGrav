//! The particle interpreter: plays a `.pob` emitter tree the original
//! authored, from the user's own disc.
//!
//! Every number this module acts on comes out of
//! [`oag_formats::pob::Emitter`] - schedules, shapes, speeds, lifetimes, the
//! 256-entry colour tables, the keyframed size and alpha channels, the drag
//! modifier, the child/sibling tree. Nothing is transcribed. That is the
//! whole point of it: the previous pass hand-copied one file's four emitters
//! into a `const` table, which was accurate (the parser reproduces it) but
//! neither re-derivable nor usable for the other 34 effects on the disc.
//!
//! # What runs here and what does not
//!
//! [`System`] is the live half - a fixed pool of particles and emitter
//! states, no `wgpu` in it, advanced on the caller's fixed tick. [`Effect`]
//! is the parsed asset it plays, built once at load. The split matters
//! because an [`Effect`] is a kilobyte per emitter (the colour tables) and
//! is shared by every [`System`] playing it, while a [`System`] is fixed
//! arrays a caller can keep one of per ship and reset by assignment.
//!
//! Render-only, like [`crate::exhaust`]: none of this enters `World` or a
//! determinism hash.
//!
//! # The original's units, converted once
//!
//! The interpreter runs on a tick-count `dt`: speeds are world units per
//! **tick**, gravity units per tick², lifetimes and schedules integer
//! **ticks**. [`TICK_HZ`] is the only conversion, and it happens where a
//! value is used, not where it is parsed.
//!
//! # Deliberately not implemented
//!
//! Each of these is authored in files this module already parses, and each
//! would be a visible difference on some effect:
//!
//! - **Sprite atlases and textures.** Emitters name a developer `.tga` path
//!   through their slot table and index a grid of frames
//!   ([`oag_formats::pob::Emitter::atlas_grid`]); this module draws a
//!   procedural radial falloff instead. Decoding the shipped sprite is the
//!   follow-up that would let the streak's `v` layout be adopted too - see
//!   [`crate::sparks`].
//! - **Billboard roll.** The rotation-speed channel is parsed and unused;
//!   quads here are axis-aligned to the camera.
//! - **The emitter extent.** Particles spawn at the anchor point rather than
//!   scattered over the emitter's radius (`0.006` to `2.45` units across the
//!   corpus, severity-scaled). Sub-visible on the collision sparks, not on
//!   an explosion's smoke ring.
//! - **The emission-scale channel** and the animated-attribute array, both
//!   of which re-derive parameters over an emitter's life.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_formats::pob::{self, Channel, ChannelMode, ParticleSystem};

use crate::mesh::GpuVertex;

/// The original's fixed simulation rate, ticks per second.
///
/// Emitter schedules, speeds and lifetimes in a `.pob` are all expressed in
/// ticks; this is the one conversion constant between them and seconds.
pub const TICK_HZ: f32 = 60.0;

/// Particles one [`System`] can hold at once.
///
/// Sized from the corpus rather than tuned: the heaviest tree on the disc,
/// `WO_ROCKET_EXPLO`, schedules about 100 concurrent particles across its
/// seven emitters, and the collision sparks about 57 across four. An
/// emission that would overflow replaces the particle nearest death (see
/// [`expendable_slot`]) rather than being dropped.
pub const MAX_PARTICLES: usize = 256;

/// Emitter states one [`System`] can run at once.
///
/// One per emitter in the effect, plus one per live child instance: a
/// per-particle child means an emitter state per *particle* of its parent,
/// which is what needs the headroom.
pub const MAX_EMITTER_STATES: usize = 64;

/// How a particle picks its initial direction.
///
/// The original chooses by emitter shape **and** velocity mode together -
/// `ParticleSystem_SpawnBurst` dispatches the shape, and the sphere shapes
/// then ignore the aim fields entirely. See
/// `docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "The emit frame
/// and the velocity dispatch, settled".
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// Away from the emitter, along the spawn offset's own direction -
    /// what a sphere or hemisphere shape does for velocity modes 0 and 1
    /// alike. `hemisphere` forces the emitter-local up component positive
    /// (shape 7).
    Radial {
        /// Shape 7 rather than shape 4.
        hemisphere: bool,
    },
    /// A random tangent to the spawn direction - a sphere shape under
    /// velocity mode 2.
    Tangent {
        /// Shape 7 rather than shape 4.
        hemisphere: bool,
    },
    /// `ParticleSystem_AimedVelocity`: elevation over the emitter's
    /// horizontal plane, azimuth added to the spawn heading, both jittered
    /// by the cone half-angle.
    Aimed {
        /// Radians, `+0x50`.
        elevation: f32,
        /// Radians, `+0x54`, added to a uniform heading and so absorbed by
        /// it - kept because it is authored.
        azimuth: f32,
        /// Radians, `+0x58` converted from degrees.
        jitter: f32,
    },
    /// `ParticleSystem_ConeVelocity`: an angle `U(-a, a)` about the
    /// emitter's up axis with `a` the cone half-angle, uniform in azimuth.
    Cone {
        /// Radians, `+0x58` converted from degrees.
        half_angle: f32,
    },
}

/// How a particle is turned into geometry - the draw class the executable's
/// blend table maps the emitter's render-mode index to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Render {
    /// A camera-facing quad at one point: draw classes 1, 2 and 3. Class 3
    /// also carries a roll, which this module does not apply.
    Billboard,
    /// A quad spanned between two points, classes 6 and 7. `from_spawn`
    /// keeps the second point at the spawn position (resource flag
    /// `0x2000000`), which makes a burst radiate; otherwise it is refreshed
    /// every tick and the particle trails its own motion.
    Streak {
        /// Resource flag `0x2000000`.
        from_spawn: bool,
    },
}

/// A particle's blend class - `+0xc0`, dispatched by
/// `ParticleSystem_ApplyBlendClass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Class 2: `src_alpha, one`. What every bright emitter uses.
    Additive,
    /// Class 3: `src_alpha, one_minus_src_alpha`. What smoke uses, and why
    /// a dark smoke colour is visible at all - added, it would vanish.
    ///
    /// Class 1 is an alpha-*test* path with no blend at all; it is drawn
    /// here as alpha-over, which is the closer of the two available
    /// pipelines. No corpus emitter uses it.
    AlphaOver,
}

/// What byte value an emitter's colour table and alpha channel treat as
/// fully bright.
///
/// **The two releases author the same effects at different colour scales**,
/// and it is the one thing in a `.pob` that cannot be read out of the file.
/// Measured over both corpora on 2026-08-12:
///
/// | release | RGB reaches | alpha channel reaches |
/// | --- | --- | --- |
/// | PSP, 35 systems | `255` | `255` |
/// | PS2, 41 systems | **`127`, never more** | **`127.5`, never more** |
///
/// The same effect differs by exactly a factor of two: `WO_ROCKET_FLARE`'s
/// first palette entry is `[255, 255, 255, 255]` on the PSP and caps at
/// `127` on the PS2. That is the PS2 GS's convention, where `0x80` rather
/// than `0xff` is 1.0 - and `127.5` is exactly half of `255`.
///
/// **It cannot be detected per file.** One PSP effect's brightest channel is
/// `40` and another's is `216`, so "nothing above 127, therefore PS2" would
/// misread a legitimately dark PSP effect and draw it twice as bright. The
/// container header carries no version or platform word either - both
/// releases write the same `+0x0a` and `+0x0c`. So the caller says, from the
/// source it opened.
///
/// Confidence **80**: the corpus split is total on both discs and the
/// same-effect factor of two is exact, but nothing in the PS2 executable has
/// been read to confirm how it consumes these bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourScale {
    /// `0..=255`, the PSP GE's convention.
    Full,
    /// `0..=127.5`, the PS2 GS's.
    Half,
}

impl ColourScale {
    /// What to divide an authored byte by to get `0..=1`.
    #[must_use]
    pub fn divisor(self) -> f32 {
        match self {
            Self::Full => 255.0,
            Self::Half => 127.5,
        }
    }
}

/// How a particle takes its colour from the emitter's 256-entry table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourMode {
    /// `+0xbc == 2`: one random entry, drawn once at spawn.
    RandomEntry,
    /// Anything else: `palette[age * 255.999]`, walked over the particle's
    /// life.
    OverLife,
}

/// One emitter of an [`Effect`], in the original's own units.
///
/// A translation of [`oag_formats::pob::Emitter`], not a re-reading of it:
/// the fields this module cannot yet act on are left behind in the parsed
/// record rather than carried here (see the module doc comment).
#[derive(Debug, Clone, PartialEq)]
pub struct EmitterSpec {
    /// The record's authored name, for tests and diagnostics.
    pub name: String,
    /// Ticks the emitter emits for. Ignored under [`EmitterSpec::looping`].
    pub duration_ticks: f32,
    /// The emitter runs until its owner stops it, and
    /// [`EmitterSpec::duration_ticks`] is not a countdown -
    /// [`oag_formats::pob::flags::LOOPING`].
    ///
    /// This is what separates an effect a caller *attaches* - the rocket's
    /// flare, a craft's engine flare, the rain - from one it *fires*. An
    /// attached effect authors a duration anyway (the flare's is 100 ticks,
    /// under three seconds of a ten-second flight) and it means nothing;
    /// reading it as a countdown makes the effect stop halfway through and
    /// invites inventing a re-trigger to cover the gap.
    pub looping: bool,
    /// Ticks between emissions, min and max.
    ///
    /// **Both are at least 1**, which [`Effect::parse`] clamps and the emission
    /// loop depends on: a `(0, 0)` interval never advances `until_next`, so the
    /// loop that drains it spawns for ever and hangs the frame. No disc asset
    /// can produce one, but this is a `pub` struct with `pub` fields and
    /// hand-building an emitter is an invitation the type should not extend -
    /// finding R1 of the 2026-08-18 review. [`Emitters::step`] clamps at the use
    /// site too, so a hand-built `(0, 0)` emits once a tick rather than looping.
    pub interval_ticks: (u32, u32),
    /// Particles per emission, min and max.
    pub per_emission: (u32, u32),
    /// Particle lifetime centre and spread, ticks: `centre + spread * U(-1, 1)`.
    pub lifetime_ticks: (f32, f32),
    /// Ejection speed centre and spread, world units per tick, before the
    /// system's scale.
    pub speed_per_tick: (f32, f32),
    /// Direction law.
    pub direction: Direction,
    /// Per-axis exponential velocity decay applied once per tick; `ONE`
    /// where the emitter authors no drag modifier.
    pub drag_per_tick: Vec3,
    /// Downward acceleration, world units per tick², already gated on the
    /// emitter's `0x200` flag: zero where the flag is clear, however the
    /// field is authored.
    pub gravity_per_tick2: f32,
    /// The emitter's own live-particle cap; an emission that would exceed it
    /// is skipped whole.
    pub live_cap: usize,
    /// Drawn half-size in world units, over the particle's life, before the
    /// system's scale.
    pub size: Channel,
    /// Alpha over the particle's life, in the source's own byte scale -
    /// divide by [`EmitterSpec::colour_divisor`] rather than by 255.
    pub alpha: Channel,
    /// What [`EmitterSpec::alpha`] treats as fully opaque, from the source's
    /// [`ColourScale`]. The palette is already normalised at parse; this is
    /// the alpha channel's share of the same conversion.
    pub colour_divisor: f32,
    /// The emitter's 256 RGBA entries, premultiplied to `0..=1`. **Read
    /// from the user's disc at runtime and never committed** - see
    /// ADR-0006.
    pub palette: Box<[[f32; 4]; 256]>,
    /// Which of them a particle takes.
    pub colour_mode: ColourMode,
    /// Geometry class.
    pub render: Render,
    /// Blend class.
    pub blend: Blend,
    /// A system attached to each particle this emitter spawns, as an index
    /// into [`Effect::emitters`].
    pub particle_child: Option<usize>,
    /// A system spawned once where a particle of this emitter dies.
    pub death_child: Option<usize>,
    /// The probability the parent particle actually spawns this emitter, if
    /// it is a child.
    pub spawn_probability: f32,
    /// How much of the parent particle's velocity this emitter's own
    /// particles inherit, if it is a child.
    pub velocity_inherit: f32,
}

/// A parsed `.pob` ready to play: the root emitter first, then the tree
/// depth-first, exactly as [`oag_formats::pob::ParticleSystem::emitters`]
/// returns it.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// The resource's own name, e.g. `WO_SHIP_COLL_SPARK_DAMAGE`.
    pub name: String,
    /// The emitter tree.
    pub emitters: Vec<EmitterSpec>,
    /// Which emitters start on [`System::ignite`]: the root and its sibling
    /// chain, but not a child, which starts when its parent's particle does.
    roots: Vec<usize>,
}

/// Something in a `.pob` this module cannot play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob would not parse at all.
    Format(pob::Error),
    /// An emitter's render-mode index is past the executable's eight-entry
    /// blend table, so it has no draw handler. Refused rather than guessed:
    /// a wrong guess here draws nothing, which looks exactly like the
    /// effect never being triggered.
    UnknownDrawClass {
        /// The emitter's name.
        emitter: String,
        /// The index found.
        render_mode: u32,
    },
    /// An emitter's blend class is not one the GE state selector switches
    /// on.
    UnknownBlendClass {
        /// The emitter's name.
        emitter: String,
        /// The class found.
        blend_class: u32,
    },
    /// A channel block's mode has no traced consumer.
    UnknownChannelMode {
        /// The emitter's name.
        emitter: String,
        /// The raw mode word.
        mode: u32,
    },
    /// More emitters than [`MAX_EMITTER_STATES`] can ever run.
    TooManyEmitters {
        /// How many the file holds.
        count: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Format(error) => write!(f, "{error}"),
            Self::UnknownDrawClass {
                emitter,
                render_mode,
            } => write!(f, "{emitter}: render mode {render_mode} has no draw class"),
            Self::UnknownBlendClass {
                emitter,
                blend_class,
            } => write!(f, "{emitter}: blend class {blend_class} is not dispatched"),
            Self::UnknownChannelMode { emitter, mode } => {
                write!(f, "{emitter}: channel mode {mode} has no consumer")
            }
            Self::TooManyEmitters { count } => {
                write!(f, "{count} emitters, more than {MAX_EMITTER_STATES}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<pob::Error> for Error {
    fn from(error: pob::Error) -> Self {
        Self::Format(error)
    }
}

impl Effect {
    /// Parses `data`, a whole `.pob` blob, into a playable effect.
    ///
    /// # Errors
    ///
    /// The blob failing to parse, or an emitter using a draw class, blend
    /// class or channel mode with no traced consumer - each of which would
    /// otherwise reach the pool as a silent no-draw.
    pub fn parse(data: &[u8], scale: ColourScale) -> Result<Self, Error> {
        let system = ParticleSystem::parse(data)?;
        let records = system.emitters(data)?;
        if records.len() > MAX_EMITTER_STATES {
            return Err(Error::TooManyEmitters {
                count: records.len(),
            });
        }

        let emitters = records
            .iter()
            .map(|record| EmitterSpec::from_record(record, scale))
            .collect::<Result<Vec<_>, _>>()?;

        // Everything reachable as a child starts with its parent's
        // particles, not with the effect; whatever is left is a root or one
        // of its siblings, which the parser already flattened in order.
        let mut is_child = vec![false; emitters.len()];
        for spec in &emitters {
            for child in [spec.particle_child, spec.death_child]
                .into_iter()
                .flatten()
            {
                is_child[child] = true;
            }
        }
        let roots = (0..emitters.len()).filter(|&i| !is_child[i]).collect();

        Ok(Self {
            name: system.name,
            emitters,
            roots,
        })
    }

    /// The emitters [`System::ignite`] starts.
    #[must_use]
    pub fn roots(&self) -> &[usize] {
        &self.roots
    }
}

impl EmitterSpec {
    /// How long one instance of this emitter runs, in ticks - infinite
    /// under [`EmitterSpec::looping`], which is what stops the schedule
    /// ever expiring on its own.
    #[must_use]
    pub fn run_ticks(&self) -> f32 {
        if self.looping {
            f32::INFINITY
        } else {
            self.duration_ticks
        }
    }

    /// Translates one parsed record.
    fn from_record(record: &pob::Emitter, scale: ColourScale) -> Result<Self, Error> {
        let emitter = || record.name.clone();

        let render = match record.draw_class() {
            Some(1..=3) => Render::Billboard,
            Some(6 | 7) => Render::Streak {
                from_spawn: record.flags & pob::flags::STREAK_FROM_SPAWN != 0,
            },
            _ => {
                return Err(Error::UnknownDrawClass {
                    emitter: emitter(),
                    render_mode: record.render_mode,
                });
            }
        };
        let blend = match record.blend_class {
            2 => Blend::Additive,
            1 | 3 => Blend::AlphaOver,
            other => {
                return Err(Error::UnknownBlendClass {
                    emitter: emitter(),
                    blend_class: other,
                });
            }
        };
        for channel in [
            &record.size,
            &record.alpha,
            &record.rotation_speed,
            &record.emission_scale,
        ] {
            if let ChannelMode::Unknown(mode) = channel.mode {
                return Err(Error::UnknownChannelMode {
                    emitter: emitter(),
                    mode,
                });
            }
        }

        let jitter = record.cone_degrees.to_radians();
        let hemisphere = record.shape == 7;
        let direction = match (record.shape, record.velocity_mode) {
            (4 | 7, 2) => Direction::Tangent { hemisphere },
            (4 | 7, _) => Direction::Radial { hemisphere },
            (_, 1) => Direction::Aimed {
                elevation: record.elevation,
                azimuth: record.azimuth,
                jitter,
            },
            _ => Direction::Cone { half_angle: jitter },
        };

        let mut palette = Box::new([[0.0f32; 4]; 256]);
        let divisor = scale.divisor();
        for (out, entry) in palette.iter_mut().zip(record.colours.iter()) {
            *out = [
                f32::from(entry[0]) / divisor,
                f32::from(entry[1]) / divisor,
                f32::from(entry[2]) / divisor,
                f32::from(entry[3]) / divisor,
            ];
        }

        let drag = record.drag_per_tick().unwrap_or([1.0; 3]);
        let interval = (
            record.interval_ticks.0.max(1) as u32,
            record.interval_ticks.1.max(record.interval_ticks.0).max(1) as u32,
        );
        let per_emission = (
            record.per_emission.0.max(0) as u32,
            record.per_emission.1.max(record.per_emission.0).max(0) as u32,
        );

        Ok(Self {
            name: record.name.clone(),
            duration_ticks: record.duration_ticks.max(0.0),
            looping: record.looping(),
            interval_ticks: interval,
            per_emission,
            lifetime_ticks: (
                record.lifetime_ticks.0 as f32,
                record.lifetime_ticks.1 as f32,
            ),
            speed_per_tick: record.speed_per_tick,
            direction,
            drag_per_tick: Vec3::new(drag[0], drag[1], drag[2]),
            gravity_per_tick2: if record.gravity_enabled() {
                record.gravity_per_tick2
            } else {
                0.0
            },
            live_cap: record.live_cap.max(0) as usize,
            size: record.size.clone(),
            alpha: record.alpha.clone(),
            colour_divisor: divisor,
            palette,
            colour_mode: if record.colour_mode == 2 {
                ColourMode::RandomEntry
            } else {
                ColourMode::OverLife
            },
            render,
            blend,
            particle_child: record.particle_child,
            death_child: record.death_child,
            spawn_probability: record.child_spawn_probability.clamp(0.0, 1.0),
            velocity_inherit: record.child_velocity_inherit,
        })
    }
}

/// One live particle. Dead when [`Particle::life`] reaches zero.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    /// The streak's other end: the spawn point under
    /// [`Render::Streak::from_spawn`], the previous tick's position
    /// otherwise - the particle-row-`+0x50` mechanism of
    /// `ParticleSystem_UpdateParticles`.
    origin: Vec3,
    life: f32,
    max_life: f32,
    /// Which [`Effect::emitters`] entry authored it.
    spec: u16,
    /// The palette entry drawn at spawn, under [`ColourMode::RandomEntry`].
    colour_index: u8,
    /// The `0..=1` samples the two [`ChannelMode::Random`] channels draw
    /// once at spawn; unused for the other modes.
    size_sample: f32,
    alpha_sample: f32,
    /// The system's scale at spawn - the original's severity, which
    /// multiplies both speed and drawn size.
    scale: f32,
}

impl Particle {
    const DEAD: Self = Self {
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        origin: Vec3::ZERO,
        life: 0.0,
        max_life: 0.0,
        spec: 0,
        colour_index: 0,
        size_sample: 0.0,
        alpha_sample: 0.0,
        scale: 0.0,
    };

    fn alive(self) -> bool {
        self.life > 0.0
    }
}

/// One running emitter: which spec, how much longer, and where it is.
///
/// A child instance carries its own anchor and drift rather than a
/// reference to the parent particle that spawned it: the parent's slot can
/// be recycled under it, and an index into a pool that reuses slots is the
/// kind of aliasing that produces an effect anchored to the wrong thing.
/// The drift is the parent's velocity at spawn, integrated linearly - the
/// parent's own drag is not reapplied, so a fast, heavily damped parent
/// drags its child slightly too far.
#[derive(Debug, Clone, Copy, PartialEq)]
struct EmitterState {
    spec: u16,
    ticks_left: f32,
    /// Ticks until the next emission; `<= 0` means "due now", matching the
    /// original's countdown at `instance + 0x04`, which emits on its very
    /// first update.
    until_next: f32,
    anchor: Vec3,
    drift: Vec3,
    /// Velocity added to every particle this emitter spawns - a child
    /// inherits its parent particle's, scaled by the child's own
    /// `+0x4d0`.
    inherited: Vec3,
    /// Whether this instance rides its own drift (a child) or the caller's
    /// anchor (a root).
    is_child: bool,
    active: bool,
}

impl EmitterState {
    const IDLE: Self = Self {
        spec: 0,
        ticks_left: 0.0,
        until_next: 0.0,
        anchor: Vec3::ZERO,
        drift: Vec3::ZERO,
        inherited: Vec3::ZERO,
        is_child: false,
        active: false,
    };
}

/// A live instance of an [`Effect`]: one pool of particles and the emitters
/// filling it.
///
/// Render-only state with no `wgpu` in it and no reference to the effect it
/// plays, so a caller can keep one per ship or per projectile and reset it
/// by assignment. Fixed arrays rather than `Vec`s, for the same reason the
/// simulation uses them (ADR-0003), though nothing here is hashed.
#[derive(Debug, Clone, PartialEq)]
pub struct System {
    particles: [Particle; MAX_PARTICLES],
    emitters: [EmitterState; MAX_EMITTER_STATES],
    /// The original's severity: multiplies ejection speed and drawn size,
    /// never particle counts.
    scale: f32,
    anchor: Vec3,
    ignitions: u32,
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    /// No live particles, no running emitters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            particles: [Particle::DEAD; MAX_PARTICLES],
            emitters: [EmitterState::IDLE; MAX_EMITTER_STATES],
            scale: 1.0,
            anchor: Vec3::ZERO,
            ignitions: 0,
        }
    }

    /// Starts `effect`'s root emitters at `point`.
    ///
    /// `scale` is the original's severity - the instance field that
    /// multiplies every emitter's ejection speed and every particle's drawn
    /// size, and nothing else. `1.0` is the neutral value; the collision
    /// sparks derive theirs from the impact (see [`crate::sparks`]).
    ///
    /// Nothing spawns here: the first particles appear on the next
    /// [`System::advance`], exactly like the original's first emitter
    /// update. Unconditional - a caller wanting one burst per contact owns
    /// that discipline.
    pub fn ignite(&mut self, effect: &Effect, point: Vec3, scale: f32) {
        self.scale = scale;
        self.anchor = point;
        for &root in effect.roots() {
            let spec = &effect.emitters[root];
            self.start(root, spec.run_ticks(), point, Vec3::ZERO, Vec3::ZERO);
        }
        self.ignitions += 1;
    }

    /// Stops every emitter without touching the live particles.
    ///
    /// What the owner of an attached effect calls when it goes away: a
    /// rocket that detonates takes its flare's *emission* with it and
    /// leaves the flare's last particles to finish their own lives, which
    /// is why smoke can outlive the thing that made it.
    ///
    /// A [`EmitterSpec::looping`] effect ends no other way.
    pub fn stop(&mut self) {
        for state in &mut self.emitters {
            state.active = false;
        }
    }

    /// Emits due particles, then ages, moves and expires live ones, by `dt`
    /// seconds.
    ///
    /// `anchor` is where the effect's *root* emitters are now - the
    /// original's emitter node rides whatever it is parented to, so moving
    /// it is the caller's job and a burst from a moving scrape strings out
    /// along the hull's path. Child instances keep their own anchors.
    pub fn advance(&mut self, effect: &Effect, dt: f32, anchor: Vec3, rng: &mut Rng) {
        let dt_ticks = dt * TICK_HZ;
        let moved = anchor - self.anchor;
        self.anchor = anchor;

        self.emit(effect, dt_ticks, moved, rng);
        self.integrate(effect, dt, dt_ticks, rng);
    }

    /// Runs every active emitter's schedule for one tick.
    ///
    /// Child instances a spawn asks for are started **after** the whole
    /// loop, for the same reason [`System::integrate`] defers its death
    /// children: starting one mid-loop can claim the very slot being
    /// updated (`start_at` recycles the active instance nearest its end
    /// once the pool saturates), and the write-back would then either lose
    /// the child or run the child's state under the parent's spec. It also
    /// matches the original, whose freshly attached instance first updates
    /// on the *next* `ParticleSystem_Update`.
    fn emit(&mut self, effect: &Effect, dt_ticks: f32, moved: Vec3, rng: &mut Rng) {
        let mut children: Vec<(usize, Vec3, Vec3)> = Vec::new();
        for index in 0..self.emitters.len() {
            if !self.emitters[index].active {
                continue;
            }
            // A root instance follows the caller's anchor; a child rides
            // the parent particle it was spawned off.
            let state = &mut self.emitters[index];
            if state.is_child {
                state.anchor += state.drift * (dt_ticks / TICK_HZ);
            } else {
                state.anchor += moved;
            }

            // The schedule is advanced in place and only the three values a
            // spawn needs are copied out, so nothing here can be clobbered
            // by a spawn.
            let (spec_index, anchor, inherited) = {
                let state = &self.emitters[index];
                (state.spec, state.anchor, state.inherited)
            };
            let spec = &effect.emitters[usize::from(spec_index)];
            loop {
                let state = &mut self.emitters[index];
                if state.until_next > 0.0 || state.ticks_left <= 0.0 {
                    break;
                }
                // **`max(1)` here as well as in `Effect::parse`.** A zero
                // interval leaves `until_next` where it was and this loop
                // never terminates - see [`EmitterSpec::interval_ticks`]. The
                // parser cannot produce one; a caller constructing the pub
                // struct by hand can, and a hung frame is a bad way to find
                // out.
                state.until_next += random_range(rng, spec.interval_ticks).max(1.0);
                let count = random_range(rng, spec.per_emission) as usize;
                if spec.live_cap > 0 && self.live_for(spec_index) + count > spec.live_cap {
                    continue;
                }
                for _ in 0..count {
                    children.extend(self.spawn(effect, spec_index, anchor, inherited, rng));
                }
            }
            let state = &mut self.emitters[index];
            state.until_next -= dt_ticks;
            state.ticks_left -= dt_ticks;
            if state.ticks_left <= 0.0 {
                state.active = false;
            }
        }

        for (child, anchor, velocity) in children {
            self.start_child(effect, child, anchor, velocity, rng);
        }
    }

    /// Ages and moves every live particle.
    fn integrate(&mut self, effect: &Effect, dt: f32, dt_ticks: f32, rng: &mut Rng) {
        // Deaths are collected first: spawning a death child inside the
        // loop would borrow the pool twice, and a child spawned this tick
        // must not also be advanced by it.
        let mut deaths: Vec<(usize, Vec3, Vec3)> = Vec::new();
        for particle in &mut self.particles {
            if !particle.alive() {
                continue;
            }
            let spec = &effect.emitters[usize::from(particle.spec)];
            if !matches!(spec.render, Render::Streak { from_spawn: true }) {
                particle.origin = particle.position;
            }
            // `ParticleSystem_Update` precomputes `pow(k, dt_ticks)` per
            // axis once a tick; `dt_ticks` is 1 at the fixed step, so the
            // exponent is normally 1 - kept general so a different step
            // stays correct.
            let drag = Vec3::new(
                spec.drag_per_tick.x.powf(dt_ticks),
                spec.drag_per_tick.y.powf(dt_ticks),
                spec.drag_per_tick.z.powf(dt_ticks),
            );
            particle.velocity *= drag;
            particle.velocity.y += spec.gravity_per_tick2 * TICK_HZ * TICK_HZ * dt;
            particle.position += particle.velocity * dt;
            particle.life -= dt;
            if particle.life <= 0.0 {
                let (position, velocity) = (particle.position, particle.velocity);
                *particle = Particle::DEAD;
                if let Some(child) = spec.death_child {
                    deaths.push((child, position, velocity));
                }
            }
        }

        for (child, position, velocity) in deaths {
            self.start_child(effect, child, position, velocity, rng);
        }
    }

    /// One new particle for emitter `spec_index`, at `anchor`.
    ///
    /// Returns the per-particle child the emitter asks for, if any, for the
    /// caller to start once it is done walking the emitter pool.
    fn spawn(
        &mut self,
        effect: &Effect,
        spec_index: u16,
        anchor: Vec3,
        inherited: Vec3,
        rng: &mut Rng,
    ) -> Option<(usize, Vec3, Vec3)> {
        let spec = &effect.emitters[usize::from(spec_index)];
        let direction = direction_for(spec.direction, rng);
        // `centre + spread * U(-1, 1)`, the original's `Psys_RandSpread`,
        // units per tick converted to per second once.
        let speed = (spec.speed_per_tick.0 + spec.speed_per_tick.1 * signed_unit(rng))
            * self.scale
            * TICK_HZ;
        let life_ticks =
            (spec.lifetime_ticks.0 + spec.lifetime_ticks.1 * signed_unit(rng)).max(1.0);
        let life = life_ticks / TICK_HZ;

        let particle = Particle {
            position: anchor,
            velocity: direction * speed + inherited,
            origin: anchor,
            life,
            max_life: life,
            spec: spec_index,
            colour_index: (rng.next_u32() & 0xff) as u8,
            size_sample: rng.next_f32(),
            alpha_sample: rng.next_f32(),
            scale: self.scale,
        };
        let slot = expendable_slot(&self.particles);
        self.particles[slot] = particle;

        spec.particle_child
            .map(|child| (child, particle.position, particle.velocity))
    }

    /// Starts a child instance if its probability says so.
    ///
    /// `velocity` is the parent particle's, unscaled: it is what the child
    /// instance *rides*. The child's own `+0x4d0` scales only what the
    /// child's particles inherit on top of their own ejection - a
    /// `SMOKEMUSHROOM` at `0.03` still travels with the debris it is
    /// attached to, while its smoke barely carries any of that motion.
    fn start_child(
        &mut self,
        effect: &Effect,
        child: usize,
        anchor: Vec3,
        velocity: Vec3,
        rng: &mut Rng,
    ) {
        let spec = &effect.emitters[child];
        if spec.spawn_probability < 1.0 && rng.next_f32() >= spec.spawn_probability {
            return;
        }
        self.start_at(
            child,
            spec.run_ticks(),
            anchor,
            velocity,
            velocity * spec.velocity_inherit,
            true,
        );
    }

    /// Claims an emitter slot, replacing the one nearest its own end when
    /// every slot is busy.
    fn start(&mut self, spec: usize, ticks: f32, anchor: Vec3, drift: Vec3, inherited: Vec3) {
        self.start_at(spec, ticks, anchor, drift, inherited, false);
    }

    fn start_at(
        &mut self,
        spec: usize,
        ticks: f32,
        anchor: Vec3,
        drift: Vec3,
        inherited: Vec3,
        is_child: bool,
    ) {
        let mut slot = 0;
        let mut shortest = f32::INFINITY;
        for (index, state) in self.emitters.iter().enumerate() {
            if !state.active {
                slot = index;
                break;
            }
            if state.ticks_left < shortest {
                shortest = state.ticks_left;
                slot = index;
            }
        }
        self.emitters[slot] = EmitterState {
            spec: spec as u16,
            ticks_left: ticks,
            until_next: 0.0,
            anchor,
            drift,
            inherited,
            is_child,
            active: true,
        };
    }

    /// Live particles authored by one emitter - what its `+0xa0` cap counts.
    ///
    /// **Counted across every live instance of that emitter, where the
    /// original's cap is per instance.** It binds identically for a root,
    /// which only ever has one instance; it is stricter than the original
    /// for a per-particle child that runs several at once with a tight cap.
    /// Making it exact needs a particle to remember which *instance* spawned
    /// it, and an instance slot is recycled, so that is a generation counter
    /// rather than a field - not worth it until an effect is seen to need
    /// it.
    fn live_for(&self, spec: u16) -> usize {
        self.particles
            .iter()
            .filter(|particle| particle.alive() && particle.spec == spec)
            .count()
    }

    /// How many particles are currently live.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.particles.iter().filter(|p| p.alive()).count()
    }

    /// Whether anything is still emitting or alive.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.alive_count() > 0 || self.emitters.iter().any(|state| state.active)
    }

    /// How many times [`System::ignite`] has fired, ever.
    ///
    /// For a caller's trigger-discipline tests: particle counts cannot tell
    /// "one burst trickling" from "a burst per tick" once a burst spans tens
    /// of ticks.
    #[must_use]
    pub fn ignitions(&self) -> u32 {
        self.ignitions
    }

    /// This frame's geometry, split by blend class: `(additive,
    /// alpha_over)` - the two GE configurations the original's state
    /// selector switches between.
    ///
    /// `right` and `up` come from the camera. Billboards face the viewer;
    /// streaks span their two stored points with a camera-perpendicular
    /// width, following the streak-quad builder at `0x08916820`.
    #[must_use]
    pub fn vertices(
        &self,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
    ) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
        let forward = right.cross(up);
        let mut additive = Vec::new();
        let mut alpha_over = Vec::new();
        for particle in &self.particles {
            if !particle.alive() {
                continue;
            }
            let spec = &effect.emitters[usize::from(particle.spec)];
            let age = 1.0 - (particle.life / particle.max_life).clamp(0.0, 1.0);
            let half = channel_sample(&spec.size, age, particle.size_sample) * particle.scale;
            let alpha =
                channel_sample(&spec.alpha, age, particle.alpha_sample) / spec.colour_divisor;
            let colour = match spec.colour_mode {
                ColourMode::RandomEntry => spec.palette[usize::from(particle.colour_index)],
                // `palette[(int)(age * 255.999)]`, the original's own
                // index; the alpha byte comes from the channel either way.
                ColourMode::OverLife => spec.palette[(age * 255.999) as usize & 0xff],
            };
            let out = match spec.blend {
                Blend::Additive => &mut additive,
                Blend::AlphaOver => &mut alpha_over,
            };
            let (centre, axis_a, axis_b, cap) = match spec.render {
                // `cap = 0.5` collapses the shader's cap/cross profile to
                // the plain radial falloff a round sprite wants - see
                // `sparks.wgsl`.
                Render::Billboard => (particle.position, right * half, up * half, 0.5),
                Render::Streak { .. } => {
                    let centre = (particle.position + particle.origin) * 0.5;
                    let along = particle.position - particle.origin;
                    let length = along.length();
                    let dir = if length > 1e-6 { along / length } else { up };
                    // Perpendicular to the streak in the camera plane - the
                    // view-space `(dir.y, -dir.x)` of the original, done in
                    // world space. Degenerate when the streak points at the
                    // camera; fall back to `right`.
                    let perp = dir.cross(forward).try_normalize().unwrap_or(right);
                    // Half the span plus a size-sized cap at each end, the
                    // way `ParticleSystem_DrawStreak` extends the quad past
                    // both points - its stretch factor is a hard-coded
                    // `1.0`, so this is the recovered geometry and not an
                    // approximation. A zero-length streak still draws a
                    // `half`-sized glow, the same degenerate case.
                    let half_span = length * 0.5 + half;
                    (centre, dir * half_span, perp * half, half / half_span)
                }
            };
            out.extend_from_slice(&quad(
                centre,
                axis_a,
                axis_b,
                cap,
                [colour[0], colour[1], colour[2]],
                alpha,
            ));
        }
        (additive, alpha_over)
    }
}

/// Where the original looks an effect up: `Data\Psys\<name>.POB`, built at
/// `FUN_089156a0` and hashing to the blob's own WAD entry on all 35 PSP
/// systems.
///
/// Backslashes, the way the executable writes them.
#[must_use]
pub fn effect_path(name: &str) -> String {
    format!(r"Data\Psys\{name}.POB")
}

/// The effects a caller has loaded, by name.
///
/// The generic half of playing the disc's own effects: every
/// `Data\Psys\*.POB` reaches a [`Stage`] the same way, so nothing about an
/// effect needs code of its own. What is *not* generic, and cannot be, is
/// **when** each one fires - that is per-effect reverse-engineering, and the
/// caller that recovered a trigger is the one that names the effect here.
///
/// A `Vec` rather than a map: a race loads a handful of effects, lookups
/// happen at trigger time rather than per particle, and the order stays the
/// caller's so a loader report reads the same way twice.
#[derive(Debug, Clone, Default)]
pub struct Library {
    effects: Vec<(String, std::sync::Arc<Effect>)>,
}

impl Library {
    /// An empty library.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `effect` under `name`, replacing any effect already there.
    pub fn insert(&mut self, name: &str, effect: Effect) {
        let effect = std::sync::Arc::new(effect);
        match self.effects.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = effect,
            None => self.effects.push((name.to_string(), effect)),
        }
    }

    /// The effect loaded under `name`, if it loaded at all.
    ///
    /// `None` is the normal answer in a headless test with no disc, and the
    /// answer a caller must handle by drawing nothing.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&std::sync::Arc<Effect>> {
        self.effects
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, effect)| effect)
    }

    /// How many effects loaded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Whether nothing loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Their names, in load order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.effects.iter().map(|(name, _)| name.as_str())
    }
}

/// Effects a [`Stage`] plays at once.
///
/// A [`System`] is one effect's pool, so a second rocket needs a second
/// [`System`], not a bigger one: `WO_ROCKET_EXPLO` alone schedules about 150
/// concurrent particles and four of its seven emitters author a `2000` cap
/// that never binds, so two explosions sharing a pool would evict each
/// other's particles rather than queue. Twenty-four covers what a race
/// actually puts on screen - one engine flare per craft on a source that
/// authors one (eight), a handful of rockets in flight each carrying a
/// flare, plus their detonations - at about 500 KB of pool.
pub const MAX_INSTANCES: usize = 24;

/// Instances [`Stage::attach`] refuses to take, so a detonation always has
/// somewhere to play.
///
/// Without it a full grid's engine flares plus a volley of rockets would own
/// every instance and the explosions - the effect the player is actually
/// looking at - would be the ones dropped.
const RESERVED_FOR_BURSTS: usize = 6;

/// A [`Stage`] instance a caller still owns.
///
/// Carries the instance's generation, so a handle kept past
/// [`Stage::detach`] is inert rather than aimed at whatever took the slot
/// next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Playing {
    index: u16,
    generation: u32,
}

/// One slot of a [`Stage`].
#[derive(Debug, Clone)]
struct Instance {
    /// `None` on a slot that has never been used.
    effect: Option<std::sync::Arc<Effect>>,
    system: System,
    /// Where the root emitters are. Fixed for a burst; the owner's current
    /// position for an attached instance.
    anchor: Vec3,
    /// Set while a caller owns this instance. An attached instance is never
    /// recycled out from under its owner.
    attached: bool,
    /// Bumped every time the slot is claimed.
    generation: u32,
}

impl Instance {
    const IDLE: Self = Self {
        effect: None,
        system: System::new(),
        anchor: Vec3::ZERO,
        attached: false,
        generation: 0,
    };

    fn busy(&self) -> bool {
        self.attached
            || self
                .effect
                .as_deref()
                .is_some_and(|_| self.system.is_running())
    }
}

/// Several [`Effect`]s playing at once, each in its own [`System`].
///
/// This is the general mechanism the whole `Data\Psys` set wants and the
/// reason no effect needs a hand-authored stand-in any more: an effect is
/// either **fired** ([`Stage::play`]) - a detonation, an impact burst, one
/// position for its whole life - or **attached** ([`Stage::attach`]) to
/// something that moves, followed every tick ([`Stage::follow`]) and stopped
/// when its owner goes away ([`Stage::detach`]). The two cases are the
/// original's own: `Rocket_Init` attaches `WO_ROCKET_FLARE` to the rocket
/// while `Rocket_Update` fires `WO_ROCKET_EXPLO_TRACK` where it hits.
///
/// Render-only state, like the [`System`]s in it. What is *not* here is any
/// decision about which effect plays when - that is per-effect
/// reverse-engineering and lives with the caller that recovered it.
#[derive(Debug, Clone)]
pub struct Stage {
    instances: Box<[Instance]>,
}

impl Default for Stage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage {
    /// An empty stage, with every pool allocated up front.
    ///
    /// Boxed rather than inline: [`MAX_INSTANCES`] pools is a few hundred
    /// kilobytes and the owner of a stage is itself often moved.
    #[must_use]
    pub fn new() -> Self {
        Self {
            instances: vec![Instance::IDLE; MAX_INSTANCES].into_boxed_slice(),
        }
    }

    /// Fires `effect` at `point` and forgets it.
    ///
    /// `scale` is the original's severity - see [`System::ignite`]. Returns
    /// `None` only when every instance is attached, which
    /// [`RESERVED_FOR_BURSTS`] is there to prevent.
    ///
    /// Recycles the *unattached* instance with the fewest live particles
    /// when nothing is free: a burst that has thinned out is the least
    /// visible thing to cut short.
    pub fn play(
        &mut self,
        effect: &std::sync::Arc<Effect>,
        point: Vec3,
        scale: f32,
    ) -> Option<Playing> {
        let index = self.free_slot().or_else(|| {
            self.instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| !instance.attached)
                .min_by_key(|(_, instance)| instance.system.alive_count())
                .map(|(index, _)| index)
        })?;
        Some(self.claim(index, effect, point, scale, false))
    }

    /// Starts `effect` at `point` and hands the caller a handle to keep it
    /// there.
    ///
    /// The caller must [`Stage::follow`] it every tick and [`Stage::detach`]
    /// it when whatever carries it goes away; an attached instance is never
    /// recycled, so a handle that is never detached leaks its slot for as
    /// long as the stage lives.
    ///
    /// Returns `None` when fewer than [`RESERVED_FOR_BURSTS`] instances
    /// would be left free. A caller that gets `None` draws nothing rather
    /// than something invented.
    pub fn attach(
        &mut self,
        effect: &std::sync::Arc<Effect>,
        point: Vec3,
        scale: f32,
    ) -> Option<Playing> {
        if self.free_count() <= RESERVED_FOR_BURSTS {
            return None;
        }
        let index = self.free_slot()?;
        Some(self.claim(index, effect, point, scale, true))
    }

    /// Moves an attached instance's root emitters to `point`.
    ///
    /// A no-op on a stale handle. Particles already emitted stay where they
    /// were, which is what strings a flare out behind a moving rocket.
    pub fn follow(&mut self, playing: Playing, point: Vec3) {
        if let Some(instance) = self.get_mut(playing) {
            instance.anchor = point;
        }
    }

    /// Stops an attached instance emitting and releases the slot back to the
    /// stage.
    ///
    /// Live particles finish their own lives - see [`System::stop`] - so the
    /// slot stays busy for a moment longer and only then becomes reusable.
    pub fn detach(&mut self, playing: Playing) {
        if let Some(instance) = self.get_mut(playing) {
            instance.attached = false;
            instance.system.stop();
        }
    }

    /// Advances every playing instance by `dt` seconds.
    pub fn advance(&mut self, dt: f32, rng: &mut Rng) {
        for instance in &mut self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            if !instance.attached && !instance.system.is_running() {
                continue;
            }
            instance.system.advance(effect, dt, instance.anchor, rng);
        }
    }

    /// Every playing instance's geometry, split by blend class the same way
    /// [`System::vertices`] splits one.
    #[must_use]
    pub fn vertices(&self, right: Vec3, up: Vec3) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
        let mut additive = Vec::new();
        let mut alpha_over = Vec::new();
        for instance in &self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            let (mut a, mut b) = instance.system.vertices(effect, right, up);
            additive.append(&mut a);
            alpha_over.append(&mut b);
        }
        (additive, alpha_over)
    }

    /// How many instances are emitting or still hold live particles.
    #[must_use]
    pub fn playing_count(&self) -> usize {
        self.instances.iter().filter(|i| i.busy()).count()
    }

    /// Live particles across the whole stage - what
    /// [`Pipeline::upload`]'s buffer has to hold.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.instances
            .iter()
            .map(|instance| instance.system.alive_count())
            .sum()
    }

    /// Whether `playing` still names the instance it was handed out for.
    #[must_use]
    pub fn is_playing(&self, playing: Playing) -> bool {
        self.instance(playing).is_some()
    }

    fn free_slot(&self) -> Option<usize> {
        self.instances.iter().position(|instance| !instance.busy())
    }

    fn free_count(&self) -> usize {
        self.instances.iter().filter(|i| !i.busy()).count()
    }

    fn claim(
        &mut self,
        index: usize,
        effect: &std::sync::Arc<Effect>,
        point: Vec3,
        scale: f32,
        attached: bool,
    ) -> Playing {
        let instance = &mut self.instances[index];
        instance.generation = instance.generation.wrapping_add(1);
        instance.effect = Some(effect.clone());
        instance.system = System::new();
        instance.anchor = point;
        instance.attached = attached;
        instance.system.ignite(effect, point, scale);
        Playing {
            index: index as u16,
            generation: instance.generation,
        }
    }

    fn instance(&self, playing: Playing) -> Option<&Instance> {
        self.instances
            .get(usize::from(playing.index))
            .filter(|instance| instance.generation == playing.generation && instance.attached)
    }

    fn get_mut(&mut self, playing: Playing) -> Option<&mut Instance> {
        self.instances
            .get_mut(usize::from(playing.index))
            .filter(|instance| instance.generation == playing.generation && instance.attached)
    }
}

/// A channel's value at normalized age `age`, with `sample` standing in for
/// the uniform draw a [`ChannelMode::Random`] channel makes once at spawn.
fn channel_sample(channel: &Channel, age: f32, sample: f32) -> f32 {
    match channel.mode {
        // `Psys_RandFloatRange(lo, hi)` - the keyframes are authored on
        // some of these channels and the interpreter never reads them.
        ChannelMode::Random => channel.lo + (channel.hi - channel.lo) * sample,
        ChannelMode::Constant => channel.hi,
        ChannelMode::Keyframed | ChannelMode::Unknown(_) => channel.scaled_at(age),
    }
}

/// `Psys_RandIntRange(min, max)`, inclusive.
fn random_range(rng: &mut Rng, range: (u32, u32)) -> f32 {
    let (min, max) = range;
    if max <= min {
        return min as f32;
    }
    (min + rng.below(max - min + 1)) as f32
}

/// The index of a dead particle, or the one with the least life left.
///
/// Ascending scan, so the choice depends only on the pool's own state.
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

/// One spawn direction for a [`Direction`] law.
fn direction_for(direction: Direction, rng: &mut Rng) -> Vec3 {
    match direction {
        Direction::Radial { hemisphere } | Direction::Tangent { hemisphere } => {
            let mut d = sphere_direction(rng);
            if hemisphere {
                // `ParticleSystem_EmitSphere`'s shape-7 branch: `abs()` on
                // the emitter-local up. The emitter frames this project
                // plays are identity-rotated, so that is world up.
                d.y = d.y.abs();
            }
            if matches!(direction, Direction::Tangent { .. }) {
                // Mode 2 builds a random tangent to the spawn direction.
                let other = sphere_direction(rng);
                d.cross(other).try_normalize().unwrap_or(d)
            } else {
                d
            }
        }
        Direction::Aimed {
            elevation,
            azimuth,
            jitter,
        } => {
            // `ParticleSystem_AimedVelocity`: `y = sin(elevation ± jitter)`,
            // horizontal components scaled by the matching cosine, heading
            // taken from the spawn direction's - uniform here - rotated by
            // the authored azimuth, which uniform absorbs.
            let elev = elevation + jitter * signed_unit(rng);
            let heading = rng.next_f32() * std::f32::consts::TAU + azimuth;
            let (sin_e, cos_e) = elev.sin_cos();
            let (sin_a, cos_a) = heading.sin_cos();
            Vec3::new(cos_a * cos_e, sin_e, sin_a * cos_e)
        }
        Direction::Cone { half_angle } => {
            // `ParticleSystem_ConeVelocity` draws `U(-a, a)` off the
            // emitter's axis and takes its sin/cos; the axis is the
            // emitter's up, uniform in azimuth about it.
            let tilt = half_angle * signed_unit(rng);
            let heading = rng.next_f32() * std::f32::consts::TAU;
            let (sin_t, cos_t) = tilt.sin_cos();
            let (sin_a, cos_a) = heading.sin_cos();
            Vec3::new(sin_t * cos_a, cos_t, sin_t * sin_a)
        }
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

/// Six vertices - two triangles - for one camera-facing quad.
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
        // The `lit` slot is repurposed by this pipeline: particles are
        // emissive (never lit by the mesh rig), so it carries the cap
        // fraction the fragment profile needs - see `sparks.wgsl`.
        lit: cap,
        ..bytemuck::Zeroable::zeroed()
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

/// The maximum vertices [`Pipeline`]'s buffer holds: one quad for every
/// particle a full [`Stage`] can hold.
///
/// Derived rather than picked. A buffer sized for one [`System`] would
/// silently drop whole effects off a busy grid - the truncation in
/// [`Pipeline::upload`] cuts at a vertex, so an over-long frame loses the
/// tail of the last quads and reads as an explosion that never happened.
pub const MAX_VERTICES: usize = MAX_INSTANCES * MAX_PARTICLES * 6;

/// The particle draw pipeline - two of them, one per blend class, sharing
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
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        velocity: crate::mesh_render::Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("psys"),
            source: wgpu::ShaderSource::Wgsl(include_str!("psys.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("psys uniforms"),
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
            label: Some("psys"),
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
                    // The velocity target, when the race adds one, rides
                    // along **write-masked empty** for the reason the
                    // exhaust's does: a particle quad is rebuilt from scratch
                    // every draw and writes no depth, so the velocity at its
                    // pixels stays the surface's behind it. See
                    // `mesh_render::Velocity`.
                    targets: &{
                        let mut targets = vec![Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(blend),
                            // Colour only - the original's particle draw path
                            // (`FUN_08915fd0`) calls `Bloom_SetPixelMask(g_bloom, 0)`,
                            // protecting the glow mask. See `crate::post::bloom`.
                            write_mask: wgpu::ColorWrites::COLOR,
                        })];
                        targets.extend(velocity.target(true));
                        targets
                    },
                    compilation_options: crate::mesh_render::fragment_options(format),
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
        let additive = build("psys additive", BLEND);
        let alpha_over = build("psys alpha-over", BLEND_ALPHA_OVER);

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("psys uniforms"),
            size: crate::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("psys uniforms"),
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
        let additive_vertices = buffer("psys additive vertices");
        let alpha_vertices = buffer("psys alpha-over vertices");

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
    /// as [`System::vertices`] returns them.
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

        // A `Stage` cannot produce more than this, so an overflow means the
        // buffer and the pool have drifted apart rather than that the scene
        // is busy - worth failing on in a debug build instead of quietly
        // losing the tail.
        debug_assert!(
            additive.len() <= MAX_VERTICES && alpha_over.len() <= MAX_VERTICES,
            "particle vertices past the buffer: {} additive, {} alpha, cap {MAX_VERTICES}",
            additive.len(),
            alpha_over.len(),
        );
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
mod tests;
