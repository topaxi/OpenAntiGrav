//! The particle interpreter: plays a `.pob` emitter tree the original
//! authored, from the user's own disc.
//!
//! Every number this module acts on comes out of
//! [`oag_pob::Emitter`] - schedules, shapes, speeds, lifetimes, the
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
//! - **Sprites on the PS2 and HD.** A PSP `.pob`'s own sprite is sampled, and
//!   2048's `.gxt` ones through [`Effect::parse_with`] - see [`sprite`] and
//!   [`streak`]. A PS2 `.pob` embeds none and HD's `.gtf` are not loaded, so
//!   those draw the procedural falloff in `psys.wesl`.
//! - The animated-attribute selectors `1`, `3`, `4` and `6`, which nothing on
//!   the Pulse discs authors.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_pob::{self as pob, Channel, ChannelMode, ParticleSystem};

use oag_mesh::mesh::GpuVertex;

mod emitter_state;
mod error;
pub mod field;
pub mod frames;
pub mod guard;
mod library;
mod path;
pub mod playback;
pub mod spawn;
pub mod sprite;
pub use path::{effect_path, effect_path_in};
pub mod streak;

use emitter_state::EmitterState;
pub use error::Error;
use frames::FrameAdvance;
pub use library::Library;
use spawn::Spawn;
use sprite::{Atlas, Sheet, Sprite};
use streak::StreakDraw;

/// The original's fixed simulation rate, ticks per second.
///
/// How much life still counts as spent: `life -= dt` can land a rounding error
/// either side of zero. The spawn order is measured, see [`Particle::fresh`].
const LIFE_EPSILON: f32 = 1e-5;

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
    /// Class 8: the shock-distortion and heat-haze emitters Omega authors,
    /// which its `psys_normal_heathaze` shader draws by sampling the scene
    /// behind the sprite. **Simulated and not drawn**: what that shader reads
    /// and writes is unrecovered, and a guessed blend would be an invention.
    /// [`Effect::undrawn_emitters`] names these so the loader can say so.
    Distort,
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
/// A translation of [`oag_pob::Emitter`], not a re-reading of it:
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
    /// [`oag_pob::flags::LOOPING`].
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
    /// Which strip a [`Render::Streak`] builds - see [`streak`].
    pub streak: StreakDraw,
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
    /// Where in the emitter a particle is born - see [`spawn`].
    pub spawn: Spawn,
    /// `+0x858`, over the emitter's own run: a multiplier on the extent.
    pub emission_scale: Channel,
    /// The selector-2 attribute record, over the emitter's run: the extent's co-factor.
    pub extent_animation: Option<Channel>,
    /// The emitter's clock and burst laws - see [`playback`].
    pub playback: playback::Playback,
    /// The emitter's own sprite, decoded off a PSP disc - see [`sprite`].
    pub sprite: Option<Sprite>,
    /// How [`EmitterSpec::sprite`] divides into frames.
    pub atlas: Atlas,
    /// How a particle walks those frames - see [`frames`].
    pub frames: FrameAdvance,
    /// Where [`EmitterSpec::sprite`] sits on its [`Library`]'s [`Sheet`],
    /// `[u0, v0, u1, v1]`; `None` until a library places it, and for every
    /// emitter drawn with the procedural profile.
    pub sheet_rect: Option<[f32; 4]>,
    /// Pulse PSP's run law: an emitter emits on its first update and then only while
    /// more than one tick of its duration is left, so a `duration` of `d` ticks emits
    /// `d - 1` times (and once for `1`), and a sprite template's particle dies with
    /// one tick of its life left, so a `6`-tick `Glow` is drawn at ages `0..=4`. Read
    /// live on `WO_SHIP_EXPLOSION` (smoke `10` ticks, 18 particles; fire `20`, 19;
    /// spikes `4`, 9; the glow's five draws). `false` for every other source -
    /// [`Effect::without_pulse_psp_draw`].
    pub short_run: bool,
    /// Spawn offsets and velocities skip the emitter node's matrix -
    /// [`pob::flags::WORLD_SPACE`], so [`System::set_frame_scale`] does not reach them.
    pub world_space: bool,
    /// Built from a sprite template, not an emitter record - see [`template`].
    pub template: bool,
    /// A template's rotating, stretched quad - see [`roll::Rotation`].
    pub rotation: Option<roll::Rotation>,
}

/// A parsed `.pob` ready to play: the root emitter first, then the tree
/// depth-first, exactly as [`oag_pob::ParticleSystem::emitters`]
/// returns it.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// The resource's own name, e.g. `WO_SHIP_COLL_SPARK_DAMAGE`.
    pub name: String,
    /// The emitter tree.
    pub emitters: Vec<EmitterSpec>,
    /// The wrapping box a weather effect fills - see [`field`].
    pub field: Option<pob::field::FieldBox>,
    /// The root's `+0x98`, how far ahead of the lens a weather effect sits.
    pub view_depth: f32,
    /// Which emitters start on [`System::ignite`]: the root and its sibling
    /// chain, but not a child, which starts when its parent's particle does.
    roots: Vec<usize>,
    /// See [`Effect::skipped_templates`].
    skipped_templates: usize,
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
        Self::parse_with(data, scale, &mut |_| None)
    }

    /// [`Self::parse`], taking a sprite the file embeds none of from `external`
    /// (given the authored texture path): 2048's `.gxt` files.
    ///
    /// # Errors
    /// As [`Self::parse`].
    pub fn parse_with(
        data: &[u8],
        scale: ColourScale,
        external: &mut dyn FnMut(&str) -> Option<Sprite>,
    ) -> Result<Self, Error> {
        let system = ParticleSystem::parse(data)?;
        let records = system.emitters(data)?;
        if records.len() > MAX_EMITTER_STATES {
            return Err(Error::TooManyEmitters {
                count: records.len(),
            });
        }

        let emitters = records
            .iter()
            .map(|record| {
                let sprite = Sprite::of_record(&system, data, record, external);
                EmitterSpec::from_record(record, scale, sprite)
            })
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

        let mut effect = Self {
            name: system.name.clone(),
            emitters,
            field: records.first().and_then(|root| system.field_box(root)),
            view_depth: records
                .first()
                .and_then(|root| system.view_depth(data, root))
                .unwrap_or(0.0),
            roots,
            skipped_templates: 0,
        };
        effect.add_templates(&system, data, &records, scale, external);
        Ok(effect)
    }

    /// The names of the emitters that play but draw nothing, because their
    /// blend class is [`Blend::Distort`].
    pub fn undrawn_emitters(&self) -> impl Iterator<Item = &str> {
        self.emitters
            .iter()
            .filter(|spec| spec.blend == Blend::Distort)
            .map(|spec| spec.name.as_str())
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
    fn from_record(
        record: &pob::Emitter,
        scale: ColourScale,
        sprite: Option<Sprite>,
    ) -> Result<Self, Error> {
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
            8 => Blend::Distort,
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
            lifetime_ticks: sample::lifetime_ticks(record),
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
            streak: StreakDraw::of(record.draw_class(), record.aspect),
            blend,
            particle_child: record.particle_child,
            death_child: record.death_child,
            spawn_probability: record.child_spawn_probability.clamp(0.0, 1.0),
            velocity_inherit: record.child_velocity_inherit,
            spawn: Spawn::of(record),
            emission_scale: record.emission_scale.clone(),
            extent_animation: record
                .attribute_animations
                .iter()
                .find(|animation| animation.selector == 2)
                .map(|animation| animation.channel.clone()),
            playback: playback::Playback::of(record),
            sprite,
            atlas: Atlas::of(record),
            frames: FrameAdvance::of(record, Atlas::of(record).frames()),
            sheet_rect: None,
            short_run: true,
            world_space: record.flags & pob::flags::WORLD_SPACE != 0,
            template: false,
            rotation: roll::Rotation::of_emitter(record),
        })
    }
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
    /// The instance's extent co-factor, `+0x2c` - `1.0` unless a caller
    /// stretches it, which only the Quake does. See [`spawn`].
    extent_scale: f32,
    /// The uniform scale of the matrix the instance was spawned with, which
    /// multiplies every root emitter's spawn offset and ejection velocity
    /// and nothing else - see [`System::set_frame_scale`].
    frame_scale: f32,
    /// World-space direction the emitter frame's `X` points along - what a
    /// [`Spawn::Line`] spreads its particles over. Unit length and
    /// perpendicular to `up` once [`System::advance`] has run.
    across: Vec3,
    anchor: Vec3,
    /// World-space direction the emitter frame's authored `+Y` currently
    /// maps to - see [`System::advance`]'s own `up` parameter. Always a unit
    /// vector; `new()`'s default is world up, unrotated.
    up: Vec3,
    /// `res+0x54` as a caller holds it for the instance - see [`System::set_azimuth`].
    azimuth: Option<f32>,
    /// The frame local-space particles were last carried to, while they ride it.
    ridden: Option<playback::Frame>,
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
            extent_scale: 1.0,
            frame_scale: 1.0,
            across: Vec3::X,
            anchor: Vec3::ZERO,
            up: Vec3::Y,
            azimuth: None,
            ridden: None,
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

    /// Changes this system's own severity - see [`Self::ignite`]'s own
    /// `scale` argument for what it multiplies. Every particle already alive
    /// keeps the size and speed it spawned with; only particles emitted
    /// *after* this call read the new value, the same way [`Self::ignite`]'s
    /// own assignment only ever affected what came after it.
    ///
    /// For an effect the caller re-scales every tick to something the
    /// original itself recomputes continuously - the Quake's own wave, whose
    /// width tracks the track's own as it travels - rather than one fixed at
    /// spawn. See [`Stage::rescale`].
    pub fn rescale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Sets the extent co-factor and the frame's `X` - what the Quake does
    /// to its `WO_QUAKE` every tick, and nothing else. See [`spawn`]: only
    /// where particles are born changes, never their size or speed.
    pub fn stretch(&mut self, extent_scale: f32, across: Vec3) {
        self.extent_scale = extent_scale;
        self.across = across;
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
    ///
    /// `up` is the world-space direction the emitter frame's authored `+Y`
    /// currently maps to - what [`Direction::Aimed`]'s elevation is measured
    /// from, [`Direction::Cone`]'s axis, and the component
    /// [`Direction::Radial`]'s hemisphere forces positive. Like `anchor`, a
    /// caller supplies it fresh every tick rather than once at
    /// [`System::ignite`]: nothing spawns before the first `advance`, and an
    /// attach point that rotates (a banking hull) needs the *live* frame, not
    /// the one it had when the emitter first fired. `Vec3::Y` is the
    /// original's own default - every `Ship Collision Fx` locator measured
    /// authors an identity rotation (`docs/formats/pob.md`, "the authored
    /// emitter-node frame") - and is what every caller but the collision
    /// sparks passes.
    pub fn advance(&mut self, effect: &Effect, dt: f32, anchor: Vec3, up: Vec3, rng: &mut Rng) {
        let dt_ticks = dt * TICK_HZ;
        let moved = anchor - self.anchor;
        self.anchor = anchor;
        self.up = up.try_normalize().unwrap_or(Vec3::Y);
        self.across = spawn::frame_x(self.across, self.up);
        self.ride(effect);
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
            let before = state.anchor;
            if state.is_child {
                state.anchor += state.drift * (dt_ticks / TICK_HZ);
            } else {
                state.anchor += moved;
            }

            // The schedule is advanced in place and only the three values a
            // spawn needs are copied out, so nothing here can be clobbered
            // by a spawn.
            let (spec_index, anchor, inherited, is_child) = {
                let state = &self.emitters[index];
                (state.spec, state.anchor, state.inherited, state.is_child)
            };
            let pull_back = before - anchor;
            // A child instance is attached with a matrix of unit rows, so only a
            // root carries the frame's scale.
            let frame_scale = if is_child { 1.0 } else { self.frame_scale };
            let spec = &effect.emitters[usize::from(spec_index)];
            let run = spec.run_ticks();
            let burst = spec.burst(self.emitters[index].ticks_left, frame_scale);
            let dt_ticks = dt_ticks * spec.playback.rate;
            loop {
                let state = &mut self.emitters[index];
                // `short_run`: past the first update, the last tick of a finite run
                // does not emit.
                let spent = spec.short_run && run.is_finite() && state.ticks_left < run;
                if state.until_next > 0.0
                    || state.ticks_left <= 0.0
                    || (spent && state.ticks_left <= dt_ticks)
                {
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
                let (start, pull_back) = spec.playback.burst_draws(pull_back, rng);
                for i in 0..count {
                    let burst = burst.particle((i, count), start, pull_back);
                    children.extend(self.spawn(effect, spec_index, anchor, inherited, burst, rng));
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
            // `+0xc` and `DAT_08b6207c`: every consumer runs on the emitter's own rate.
            let (dt, dt_ticks) = (dt * spec.playback.rate, dt_ticks * spec.playback.rate);
            template::ride(spec, particle, self.anchor);
            if std::mem::take(&mut particle.fresh) {
                if let Some(rotation) = &spec.rotation {
                    particle.roll = rotation.advance(particle, 0.0, 0.0, dt_ticks);
                }
                continue;
            }
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
            let before = 1.0 - (particle.life / particle.max_life).clamp(0.0, 1.0);
            particle.life -= dt;
            let after = 1.0 - (particle.life / particle.max_life).clamp(0.0, 1.0);
            if let Some(rotation) = &spec.rotation {
                particle.roll = rotation.advance(particle, before, after, dt_ticks);
            }
            let frames = spec.atlas.frames();
            spec.frames.step(particle, before, after, dt_ticks, frames);
            let last_tick = if spec.short_run && spec.template {
                dt
            } else {
                0.0
            };
            if particle.life <= LIFE_EPSILON + last_tick {
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
        burst: playback::Burst,
        rng: &mut Rng,
    ) -> Option<(usize, Vec3, Vec3)> {
        let spec = &effect.emitters[usize::from(spec_index)];
        let frame_scale = if spec.world_space {
            1.0
        } else {
            burst.frame_scale
        };
        let (direction, offset) = spawn::place(
            spec.spawn,
            (spec.direction, self.azimuth),
            self.scale * self.extent_scale * burst.extent * frame_scale,
            (self.across, self.up),
            burst.phi,
            rng,
        );
        let anchor = anchor + offset + burst.shift;
        // `centre + spread * U(-1, 1)`, the original's `Psys_RandSpread`,
        // units per tick converted to per second once.
        let speed = (spec.speed_per_tick.0 + spec.speed_per_tick.1 * signed_unit(rng))
            * self.scale
            * frame_scale
            * TICK_HZ;
        let life_ticks = (spec.lifetime_ticks.0 + spec.lifetime_ticks.1 * signed_unit(rng))
            .max(1.0)
            * burst.lifetime;
        let life = life_ticks / TICK_HZ;

        let mut particle = Particle {
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
            // `Psys_RandIntRange(0, frames - 1)` under the random-frame
            // flag, drawn only when a sprite on the sheet shows it, so the
            // procedural profile (every PS2 and HD effect) draws what it did.
            frame: spec.random_frame(rng),
            frame_at: 0.0,
            fresh: true,
            roll: 0.0,
            turn: 1.0,
            spin_sample: 0.0,
        };
        particle.frame_at = f32::from(particle.frame);
        if let Some(rotation) = &spec.rotation {
            rotation.start(&mut particle, rng);
        }
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

    /// Whether any emitter is still running - the part of an effect that
    /// reads its anchor. Particles already out live on without it.
    #[must_use]
    pub fn is_emitting(&self) -> bool {
        self.emitters.iter().any(|state| state.active)
    }

    /// Whether anything is still emitting or alive; the first live particle settles it.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.particles.iter().any(|p| p.alive()) || self.emitters.iter().any(|state| state.active)
    }

    /// How many times [`System::ignite`] has fired, ever - for trigger-discipline
    /// tests, which particle counts cannot serve once a burst spans tens of ticks.
    #[must_use]
    pub fn ignitions(&self) -> u32 {
        self.ignitions
    }

    /// This frame's geometry, split by blend class: `(additive, alpha_over)`,
    /// the two GE configurations the original's state selector switches
    /// between. `right` and `up` come from the camera; streaks follow the
    /// streak-quad builder at `0x08916820`.
    #[must_use]
    pub fn vertices(
        &self,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
    ) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
        let mut additive = Vec::new();
        let mut alpha_over = Vec::new();
        self.extend_vertices(&mut additive, &mut alpha_over, effect, right, up, None);
        (additive, alpha_over)
    }

    /// [`Self::vertices`], appended to two lists the caller owns - the
    /// renderer's form (a returned pair per effect allocated 1.99 MB a frame
    /// with rockets in the air). `guard` is the GE's screen-range cull, or
    /// `None` for a source it was not measured on.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
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
            let out: &mut Vec<GpuVertex> = match spec.blend {
                Blend::Additive => additive,
                Blend::AlphaOver => alpha_over,
                Blend::Distort => continue,
            };
            let rgb = [colour[0], colour[1], colour[2]];
            if let Render::Streak { .. } = spec.render {
                streak::extend(
                    out,
                    spec,
                    particle,
                    half,
                    [rgb[0], rgb[1], rgb[2], alpha],
                    right,
                    up,
                );
                continue;
            }
            // `cap = 0.5` collapses the shader's cap/cross profile to the
            // plain radial falloff a round sprite wants.
            let mut corners = match &spec.rotation {
                Some(rotation) => rotation.quad(particle, age, half, right, up, rgb, alpha),
                None => quad(particle.position, right * half, up * half, 0.5, rgb, alpha),
            };
            if let Some(rect) = spec.sheet_rect {
                sprite::map_to_cell(&mut corners, spec.atlas.cell(rect, particle.frame));
            }
            if guard::drops(guard, spec.template, &corners) {
                continue;
            }
            out.extend_from_slice(&corners);
        }
    }
}

/// Effects a [`Stage`] plays at once.
///
/// A [`System`] is one effect's pool, so a second rocket needs a second
/// [`System`], not a bigger one: `WO_ROCKET_EXPLO` alone schedules about 150
/// concurrent particles and four of its seven emitters author a `2000` cap
/// that never binds, so two explosions sharing a pool would evict each
/// other's particles rather than queue. Twenty-four covered what a race
/// actually puts on screen - one engine flare per craft on a source that
/// authors one (eight), a handful of rockets in flight each carrying a
/// flare, plus their detonations - at about 500 KB of pool.
///
/// **Thirty-two since the absorb burst landed, chosen, not measured.** One
/// absorb alone starts up to ten instances on Pulse (one per `Ship Collision
/// Fx` node, `oag_raceplay::absorb`), each alive about a second, which
/// against twenty-four would have recycled the detonations the old count was
/// sized for. About 670 KB of pool.
pub const MAX_INSTANCES: usize = 32;

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
    /// The emitter frame's `+Y` in world space - see [`Stage::orient`].
    up: Vec3,
}

impl Instance {
    const IDLE: Self = Self {
        effect: None,
        system: System::new(),
        anchor: Vec3::ZERO,
        attached: false,
        generation: 0,
        up: Vec3::Y,
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

    /// Points an attached instance's emitter-frame `+Y` along `up` - the
    /// Rocket's flare frame is quarter-turned so its `+Y` is the direction of
    /// travel (`rocket-visuals.md`, 2026-09-24). A no-op on a stale handle.
    pub fn orient(&mut self, playing: Playing, up: Vec3) {
        if let Some(instance) = self.get_mut(playing) {
            instance.up = up;
        }
    }

    /// Changes an attached instance's own severity - see [`System::rescale`].
    ///
    /// A no-op on a stale handle, the same shape [`Self::follow`] takes.
    pub fn rescale(&mut self, playing: Playing, scale: f32) {
        if let Some(instance) = self.get_mut(playing) {
            instance.system.rescale(scale);
        }
    }

    /// Sets an attached instance's extent co-factor and frame `X` - see
    /// [`System::stretch`].
    pub fn stretch(&mut self, playing: Playing, extent_scale: f32, across: Vec3) {
        if let Some(instance) = self.get_mut(playing) {
            instance.system.stretch(extent_scale, across);
        }
    }

    /// Stops an attached instance emitting and releases the slot back to the
    /// stage; live particles finish their own lives - see [`System::stop`] -
    /// so the slot stays busy a moment longer. [`Stage::kill`] ends them.
    pub fn detach(&mut self, playing: Playing) {
        if let Some(instance) = self.get_mut(playing) {
            instance.attached = false;
            instance.system.stop();
        }
    }

    /// Advances every playing instance by `dt` seconds.
    ///
    /// Each instance's own `up` - world `+Y` unless [`Stage::orient`] set one,
    /// which the Rocket's flare does: its frame is the rocket's, turned so
    /// `+Y` runs down the flight path. See [`System::advance`]'s own `up`
    /// parameter for what it steers.
    pub fn advance(&mut self, dt: f32, rng: &mut Rng) {
        for instance in &mut self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            if !instance.attached && !instance.system.is_running() {
                continue;
            }
            instance
                .system
                .advance(effect, dt, instance.anchor, instance.up, rng);
        }
    }

    /// Every playing instance's geometry, split by blend class the same way
    /// [`System::vertices`] splits one.
    #[must_use]
    pub fn vertices(&self, right: Vec3, up: Vec3) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
        let mut additive = Vec::new();
        let mut alpha_over = Vec::new();
        self.extend_vertices(&mut additive, &mut alpha_over, right, up, None);
        (additive, alpha_over)
    }

    /// [`Self::vertices`], appended to two lists the caller owns - the form
    /// the renderer uses, for the reason [`System::extend_vertices`] gives.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        for instance in &self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            instance
                .system
                .extend_vertices(additive, alpha_over, effect, right, up, guard);
        }
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
        instance.up = Vec3::Y;
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

mod direction;
mod sample;
use direction::{direction_for, signed_unit};
use sample::{channel_sample, random_range};
mod frame;
mod pipeline;
pub use pipeline::{BLEND, BLEND_ALPHA_OVER, MAX_VERTICES, Pipeline};

mod release;
mod riding;

mod particle;
use particle::{Particle, expendable_slot};
mod quad;
use quad::quad;

mod roll;
mod template;

#[cfg(test)]
mod frame_tests;
#[cfg(test)]
mod tests;
