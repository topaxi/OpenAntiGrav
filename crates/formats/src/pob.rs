//! `.pob` particle systems: a `SYSP` container around a tree of emitters.
//!
//! These are the `Data\Psys\*.POB` blobs the front-end and weapon effects
//! load by name (`Data\Psys\%s.POB`, built at `FUN_089156a0` in the PSP
//! `BOOT.BIN`). 35 exist in `Data.wad` on the PSP disc, one per authored
//! effect - `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_MISSILE_EXPLO`, `WO_RAIN`, and so
//! on. The PS2 port ships the identical container: 41 `SYSP` blobs in
//! `WADS2.WAD`, 32 sharing a name hash (and so a path) with PSP, parsed and
//! resolved by this same module with no code changes. See
//! `docs/formats/pob.md` for the evidence and open questions.
//!
//! # Container
//!
//! ```text
//! +0x00  char[4]  "SYSP"
//! +0x04  u32      header_len(16) + payload.len()
//! +0x08  u16      slot count, entries in the table at +0x10
//! +0x0a  u16      1 in every file seen
//! +0x0c  u32      1 in every file seen
//! +0x10  slot[count], 4 bytes each: u32, or 0xffffffff for an unused slot
//!        ...      32-byte NUL-terminated name, immediately after the table
//!        ...      the emitter tree - see `Emitter`
//! ```
//!
//! The `+0x04` field is redundant with the file's own length - it is validated
//! on parse rather than exposed - and the name field's position (immediately
//! after the table, not the fixed `+0x80` a single sample first suggested) is
//! confirmed by hashing the recovered name as a WAD entry path and finding it
//! agrees with the blob's own entry hash, on all 35 files in the corpus.
//!
//! # The slot table is a pointer-fixup table
//!
//! Confirmed by both decompilation and a live PPSSPP trace (`FUN_088f8e38`,
//! called from the generic resource loader `FUN_088f3540` on every fresh
//! load): a slot's raw value is not the offset of its data. It is the offset
//! of a **fixup site** - a 4-byte field, relative to the resource's own base
//! (`HEADER_LEN + slots.len() * SLOT_LEN`, i.e. right where the slot table
//! ends) - holding a second, baked offset from the same base. The loader adds
//! the base to that stored value in place, turning it from an on-disk
//! relative offset into a live pointer:
//!
//! ```text
//! fixup_site = resource_base + slots[i]
//! target     = resource_base + read_u32(fixup_site)   // what the site holds *before* the add
//! ```
//!
//! [`ParticleSystem::resolve_slot`] replays exactly this, entirely from the
//! file's own bytes - no runtime needed to compute it, only to discover it.
//! Verified live: all 26 real slots of `WO_SHIP_COLL_SPARK_DAMAGE` matched
//! this arithmetic exactly, `fixup site value + resource_base == the address
//! PPSSPP wrote there`, and the same two-hop resolution lands in bounds on
//! **all 436 real slots across all 35 files** with zero exceptions.
//!
//! # Many resolved targets are readable strings
//!
//! 187 of those 436 (43%) are themselves NUL-terminated ASCII: developer
//! texture paths (`Z:\WipeoutPSP\X2\Data\Psys\Tex\orange_glow2.tga`,
//! `Z:\Art_Resources\Psys\Tex\pointglow_32x32.tga`) and short authored layer
//! names (`GLOW`, `debris`, `thin_streaks`, `RINGS`). So a slot is a
//! per-channel record: sometimes a texture reference, sometimes a name,
//! sometimes (the non-string targets) a small run of floats whose layout is
//! still unread. Neither the record's full field layout nor what a *shared*
//! target (several slots resolving to the identical offset - the corpus's
//! most common case, five slots to one target) means beyond "these channels
//! fall back to the same default" is known yet - see the format page.
//!
//! # The emitter tree needs no fixup at all
//!
//! Separately from the slot table, and this is the part a port actually
//! wants: the resource base is also the **root emitter record's** `+0x00`,
//! and every emitter is a fixed-offset record of the same layout. Its
//! `+0x944`, `+0x948` and `+0x94c` hold, before fixup, plain offsets from
//! the same base - a death-effect child, a per-particle child and the next
//! sibling - so the whole tree walks straight out of the file's own bytes
//! with no runtime and no loader. [`ParticleSystem::emitters`] does exactly
//! that, and [`Emitter`] carries every field
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md` traced to a
//! consumer, in the original's own units.
//!
//! The tree is not flat: `WO_SHIP_COLL_SPARK_DAMAGE` is a four-emitter
//! sibling chain, but `WO_ROCKET_EXPLO` is seven records deep in both
//! directions - its root's *particles* each carry a `SMOKEMUSHROOM` system,
//! and one of its siblings does the same with `FIREMUSHROOM`.

/// The container magic.
pub const MAGIC: &[u8; 4] = b"SYSP";

/// Bytes before the slot table: magic, the size field, the count, and the two
/// constant header words.
pub const HEADER_LEN: usize = 0x10;

/// Bytes per slot-table entry.
pub const SLOT_LEN: usize = 4;

/// Bytes of the fixed name field following the slot table.
pub const NAME_LEN: usize = 32;

/// The slot value meaning "unused".
pub const EMPTY_SLOT: u32 = 0xffff_ffff;

/// Something wrong with a particle system blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the fixed header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The blob does not start with [`MAGIC`].
    NotSysp,
    /// One of the two header words seen as a constant 1 on every real file
    /// was something else.
    UnexpectedHeaderWord {
        /// The field, named by its byte offset.
        field: &'static str,
        /// The value found.
        value: u32,
    },
    /// The slot table and the fixed name field run past the end of the blob.
    TableOutOfRange {
        /// The declared slot count.
        count: u16,
    },
    /// No NUL byte in the 32-byte name field.
    NameNotTerminated,
    /// `+0x04` disagreed with `HEADER_LEN + payload.len()`, computed from the
    /// blob's own length. Every one of 35 real files agrees exactly, so a
    /// disagreement means this is not really a `.pob` payload rather than a
    /// damaged one.
    SizeDisagreement {
        /// What `+0x04` says.
        declared: u32,
        /// `HEADER_LEN + payload.len()`.
        computed: u32,
    },
    /// A slot's fixup site, or the target the site resolves to, runs past
    /// the end of the blob. Never observed on any of the 436 real slots
    /// across all 35 files.
    SlotOutOfRange {
        /// The slot's index in the table.
        index: usize,
    },
    /// An emitter record, at the offset a tree pointer named, does not fit
    /// in the blob.
    EmitterOutOfRange {
        /// The record's offset from the resource base.
        offset: usize,
    },
    /// An emitter record's 32-byte name field has no NUL byte.
    EmitterNameNotTerminated {
        /// The record's offset from the resource base.
        offset: usize,
    },
    /// A channel block declares more keyframes than the block can hold, or
    /// a negative count.
    ChannelKeyCount {
        /// The channel block's offset from the record's base.
        block: usize,
        /// The count declared.
        count: i32,
    },
    /// The emitter tree points back at a record already visited, or runs
    /// longer than [`MAX_EMITTERS`]. Neither happens on any real file; both
    /// would otherwise be an unbounded walk driven by file bytes.
    EmitterTreeUnbounded,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::NotSysp => write!(f, "the blob does not begin with SYSP"),
            Self::UnexpectedHeaderWord { field, value } => {
                write!(f, "header word {field} is {value}, expected 1")
            }
            Self::TableOutOfRange { count } => {
                write!(f, "the {count}-slot table and name field run past the end")
            }
            Self::NameNotTerminated => write!(f, "no NUL in the 32-byte name field"),
            Self::SizeDisagreement { declared, computed } => write!(
                f,
                "+0x04 declares {declared}, but header length plus the payload is {computed}"
            ),
            Self::SlotOutOfRange { index } => {
                write!(f, "slot {index}'s fixup site or target runs past the end")
            }
            Self::EmitterOutOfRange { offset } => {
                write!(f, "the emitter record at +{offset:#x} runs past the end")
            }
            Self::EmitterNameNotTerminated { offset } => {
                write!(f, "no NUL in the name of the emitter at +{offset:#x}")
            }
            Self::ChannelKeyCount { block, count } => {
                write!(f, "the channel block at +{block:#x} declares {count} keys")
            }
            Self::EmitterTreeUnbounded => {
                write!(f, "the emitter tree revisits a record or never ends")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// A parsed particle system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleSystem<'a> {
    /// The particle system's own name, e.g. `WO_SHIP_COLL_SPARK_DAMAGE`.
    ///
    /// `Data\Psys\<name>.POB` hashes to the blob's own WAD entry on all 35
    /// files in the corpus - see [`crate::wad::hash_name`].
    pub name: String,
    /// The slot table, one entry per declared slot: `None` for an unused slot
    /// (the raw `0xffffffff`), `Some(fixup_site)` otherwise. A slot's value is
    /// not its target's offset - it is a *fixup site* that must be resolved
    /// with [`Self::resolve_slot`]. See the module documentation.
    pub slots: Vec<Option<u32>>,
    /// Bytes after the name field: the rest of the root emitter record, and
    /// every other record and slot target in the file. Parsed by
    /// [`Self::emitters`] and [`Self::resolve_slot`], both of which want the
    /// whole blob rather than this slice, because the resource base - what
    /// every offset inside is relative to - is the name field, not this.
    pub payload: &'a [u8],
}

fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn half(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

/// Whether `data` looks like a particle system, without parsing it.
#[must_use]
pub fn looks_like_particle_system(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == MAGIC
}

impl<'a> ParticleSystem<'a> {
    /// Parses a particle system blob.
    ///
    /// # Errors
    ///
    /// Fails when the blob is too short, does not start with [`MAGIC`], one of
    /// the two constant header words is not 1, the slot table and name field
    /// do not fit, the name field has no NUL, or `+0x04` disagrees with the
    /// blob's own length. Every one of those is the blob contradicting itself,
    /// not a judgement call about the data.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        if &data[0..4] != MAGIC {
            return Err(Error::NotSysp);
        }
        let declared = word(data, 0x04);
        let count = half(data, 0x08);
        let header_a = u32::from(half(data, 0x0a));
        let header_c = word(data, 0x0c);
        if header_a != 1 {
            return Err(Error::UnexpectedHeaderWord {
                field: "+0x0a",
                value: header_a,
            });
        }
        if header_c != 1 {
            return Err(Error::UnexpectedHeaderWord {
                field: "+0x0c",
                value: header_c,
            });
        }

        let table_end = HEADER_LEN + usize::from(count) * SLOT_LEN;
        let name_end = table_end + NAME_LEN;
        if data.len() < name_end {
            return Err(Error::TableOutOfRange { count });
        }

        let slots = data[HEADER_LEN..table_end]
            .chunks_exact(SLOT_LEN)
            .map(|chunk| {
                let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                (value != EMPTY_SLOT).then_some(value)
            })
            .collect();

        let name_field = &data[table_end..name_end];
        let nul = name_field
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(Error::NameNotTerminated)?;
        let name = String::from_utf8_lossy(&name_field[..nul]).into_owned();

        let payload = &data[name_end..];
        let computed = (HEADER_LEN + payload.len()) as u32;
        if declared != computed {
            return Err(Error::SizeDisagreement { declared, computed });
        }

        Ok(Self {
            name,
            slots,
            payload,
        })
    }

    /// Resolves slot `index` to its target, as an offset into [`Self::payload`].
    ///
    /// Replays the pointer fixup the game's loader performs at load time (see
    /// the module documentation): reads the raw value stored at the slot's
    /// fixup site, adds the resource's own base to it, and returns that
    /// address as an offset into `payload` - so `payload[result..]` is the
    /// target. `data` must be the same bytes originally passed to
    /// [`Self::parse`].
    ///
    /// Returns `Ok(None)` for an unused slot, and also for an `index` past
    /// the end of [`Self::slots`] - there is no fixup site to resolve either
    /// way.
    ///
    /// # Errors
    ///
    /// [`Error::SlotOutOfRange`] if the fixup site, or the target it names,
    /// runs past the end of `data`. Never observed on any of the 436 real
    /// slots across all 35 files in the corpus, but `data` here is
    /// caller-supplied and not re-validated against `self`.
    pub fn resolve_slot(&self, data: &[u8], index: usize) -> Result<Option<usize>> {
        let Some(Some(slot)) = self.slots.get(index).copied() else {
            return Ok(None);
        };
        let resource_base = HEADER_LEN + self.slots.len() * SLOT_LEN;
        let site = resource_base + slot as usize;
        let Some(fixup_bytes) = data.get(site..site + SLOT_LEN) else {
            return Err(Error::SlotOutOfRange { index });
        };
        let baked = u32::from_le_bytes([
            fixup_bytes[0],
            fixup_bytes[1],
            fixup_bytes[2],
            fixup_bytes[3],
        ]);
        let target = resource_base + baked as usize;
        let name_end = resource_base + NAME_LEN;
        if target < name_end || data.len() <= target {
            return Err(Error::SlotOutOfRange { index });
        }
        Ok(Some(target - name_end))
    }

    /// Where the resource's own base sits in `data` - the byte the emitter
    /// tree's offsets, and the slot table's fixups, are both relative to.
    ///
    /// It is the first byte of the 32-byte name field, which is also the
    /// root emitter record's `+0x00`: the resource's name and its root
    /// emitter's name are the same bytes. See [`Emitter`].
    #[must_use]
    pub fn resource_base(&self) -> usize {
        HEADER_LEN + self.slots.len() * SLOT_LEN
    }

    /// Parses the emitter tree, root first.
    ///
    /// `data` must be the same bytes originally passed to [`Self::parse`].
    /// The returned order is a depth-first walk of the tree the file's own
    /// three pointer fields describe (see [`Emitter::death_child`],
    /// [`Emitter::particle_child`] and the `+0x94c` sibling chain), with the
    /// root at index 0; the child fields on the returned records are indices
    /// into the same vector.
    ///
    /// # Errors
    ///
    /// [`Error::EmitterOutOfRange`] when a tree pointer names a record that
    /// does not fit, [`Error::EmitterNameNotTerminated`] for an unterminated
    /// name, [`Error::ChannelKeyCount`] for a channel block whose keyframes
    /// do not fit, and [`Error::EmitterTreeUnbounded`] if the walk revisits
    /// a record or exceeds [`MAX_EMITTERS`]. None occurs on any of the 35
    /// PSP or 41 PS2 files in the corpus.
    pub fn emitters(&self, data: &[u8]) -> Result<Vec<Emitter>> {
        let base = self.resource_base();
        let mut out = Vec::new();
        let mut seen = Vec::new();
        walk_emitters(data, base, 0, &mut out, &mut seen)?;
        Ok(out)
    }
}

/// The most emitter records one file may describe.
///
/// A bound on a walk driven entirely by file bytes, not a measured limit:
/// the largest tree in the PSP corpus is `WO_ROCKET_EXPLO`'s seven.
pub const MAX_EMITTERS: usize = 64;

/// Bytes of an emitter record that must be present for every documented
/// field to be readable - the modifier-list pointer at `+0x9b4` is the last
/// of them.
pub const EMITTER_LEN: usize = 0x9b8;

/// Bytes per channel block ([`Channel`]); the first three blocks sit exactly
/// this far apart.
pub const CHANNEL_LEN: usize = 0xe0;

/// Keyframes a channel block can hold: what is left of [`CHANNEL_LEN`] after
/// the block's own five-word header, in `(time, value)` pairs.
pub const MAX_CHANNEL_KEYS: usize = (CHANNEL_LEN - 0x14) / 8;

/// Emitter-record flag bits, `+0x20`.
///
/// Named from their consumers in the runtime interpreter - see
/// `docs/ghidra/functions/psp-pulse-usa/particle-system.md`. Bits with no
/// constant here were not traced to a consumer.
pub mod flags {
    /// Spawn offsets and velocities skip the emitter node's matrix.
    pub const WORLD_SPACE: u32 = 0x2;
    /// Each particle takes a random initial billboard roll.
    pub const RANDOM_ROLL: u32 = 0x4;
    /// The rotation speed is sign-flipped on half the particles.
    pub const RANDOM_ROTATION_SIGN: u32 = 0x8;
    /// A burst's particles are spread along the emitter's motion over the
    /// frame rather than all placed at its current position.
    pub const SUBFRAME_SPREAD: u32 = 0x20;
    /// [`Emitter::gravity_per_tick2`] is applied. **Dormant when clear** -
    /// the field still holds an authored value on emitters that never use
    /// it.
    pub const GRAVITY: u32 = 0x200;
    /// Particles never expire.
    pub const IMMORTAL: u32 = 0x800;
    /// The spawn direction covers the sphere by a Fibonacci spiral instead
    /// of sampling it randomly.
    pub const UNIFORM_SPHERE: u32 = 0x20_0000;
    /// [`Emitter::duration_ticks`] counts bursts, not ticks.
    pub const REPEAT_COUNT: u32 = 0x80_0000;
    /// A streak's second point stays at the particle's spawn position
    /// instead of following it - what makes a burst radiate rather than
    /// trail.
    pub const STREAK_FROM_SPAWN: u32 = 0x200_0000;
    /// Each particle takes a random sprite-atlas frame.
    pub const RANDOM_ATLAS_FRAME: u32 = 0x400_0000;
}

/// How a channel block produces its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    /// Interpolate [`Channel::keys`] over the normalized age, then scale
    /// into `lo..=hi`.
    Keyframed,
    /// Constant [`Channel::hi`].
    Constant,
    /// A uniform sample in `lo..=hi`, drawn once at spawn. The keyframes are
    /// still authored and still parsed, and the interpreter ignores them.
    Random,
    /// A mode with no traced consumer; the raw value is kept so a caller can
    /// refuse it rather than guess.
    Unknown(u32),
}

impl ChannelMode {
    fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::Keyframed,
            2 => Self::Constant,
            3 => Self::Random,
            other => Self::Unknown(other),
        }
    }
}

/// One of the four channel blocks: a scalar that varies over a particle's
/// life, or over an emitter's.
///
/// `{period f32, mode u32, key count i32, lo f32, hi f32, (time, value)
/// pairs}` - see `docs/formats/pob.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    /// `+0x00`. Non-zero on a handful of emitters; no traced consumer.
    pub period: f32,
    /// `+0x04`.
    pub mode: ChannelMode,
    /// `+0x0c` and `+0x10`: the range the channel's `0..=1` value scales
    /// into, and the bounds [`ChannelMode::Random`] samples between.
    pub lo: f32,
    /// See [`Channel::lo`]. Also the whole value under
    /// [`ChannelMode::Constant`].
    pub hi: f32,
    /// `(normalized time, normalized value)` pairs, ascending in time.
    pub keys: Vec<(f32, f32)>,
}

impl Channel {
    /// The channel's `0..=1` value at normalized age `t`, before scaling.
    ///
    /// Linear between the bracketing keys, clamped at both ends - the shape
    /// the interpreter's load-time-baked keyframe machinery evaluates. Only
    /// meaningful for [`ChannelMode::Keyframed`]; the other modes ignore the
    /// keys.
    #[must_use]
    pub fn value_at(&self, t: f32) -> f32 {
        let Some(&(first_time, first_value)) = self.keys.first() else {
            return 1.0;
        };
        if t <= first_time {
            return first_value;
        }
        for pair in self.keys.windows(2) {
            let ((t0, v0), (t1, v1)) = (pair[0], pair[1]);
            if t <= t1 {
                let span = t1 - t0;
                if span <= 0.0 {
                    return v1;
                }
                return v0 + (v1 - v0) * ((t - t0) / span);
            }
        }
        self.keys.last().map_or(1.0, |&(_, value)| value)
    }

    /// [`Channel::value_at`] scaled into `lo..=hi`, which is what the
    /// interpreter feeds to the particle - the size in world units, the
    /// alpha in `0..=255`, the rotation speed in radians per tick.
    #[must_use]
    pub fn scaled_at(&self, t: f32) -> f32 {
        self.lo + (self.hi - self.lo) * self.value_at(t)
    }
}

/// One node of an emitter's modifier list (`+0x9b4`): a per-tick force
/// applied to every live particle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Modifier {
    /// `+0x24`. Only type 3, per-axis exponential drag, has a read
    /// consumer; the others dispatch to unread handlers.
    pub kind: u32,
    /// `+0x00`, the first nine floats of the node. For type 3 the first
    /// three are the per-axis decay factors applied once per tick.
    pub params: [f32; 9],
}

/// One emitter record, at its own base inside a resolved `.pob` resource.
///
/// The root emitter's base **is** the resource base, so its `+0x00` name is
/// the resource's own name; nested records live at their own offsets in the
/// same file and carry the same layout. Field offsets and their evidence are
/// in `docs/formats/pob.md`, "The emitter record layout is decoded".
///
/// Units are the original's own and are deliberately not converted here:
/// speeds are world units per **tick**, lifetimes and schedules are integer
/// **ticks**, gravity is units per tick². The renderer converts once, at the
/// point it builds its own spec.
#[derive(Debug, Clone, PartialEq)]
pub struct Emitter {
    /// `+0x00`, the record's own authored name - the resource's name on the
    /// root, a layer name (`bits`, `GLOW`, `SMOKERING`) on the rest.
    pub name: String,
    /// Where the record sits, relative to the resource base.
    pub offset: usize,
    /// `+0x20`. See [`flags`].
    pub flags: u32,
    /// `+0x24`, ticks - or bursts under [`flags::REPEAT_COUNT`].
    pub duration_ticks: f32,
    /// `+0x30`: 0 point, 3 cone, 4 sphere, 6 box, 7 hemisphere. Shapes 1, 2
    /// and 8 exist in the dispatch and are unread.
    pub shape: u32,
    /// `+0x34`, the emitter's spawn radius, world units, severity-scaled.
    ///
    /// The two further floats the format page groups with it (`+0x38`,
    /// `+0x40`) are [`Emitter::extent_unread`] instead: `+0x38` reads as
    /// uninitialised on several corpus files.
    pub extent: f32,
    /// `+0x38` and `+0x40`, kept raw. `+0x40` is a plausible second axis
    /// (`0.737` on `SMOKEMUSHROOM`, `2.94` on a `SMOKERING`); `+0x38` holds
    /// values like `-15910579.0` and `8.8e23` on records whose other fields
    /// are all sane, so it is not read as data here.
    pub extent_unread: [f32; 2],
    /// `+0x3c`: 0 exact, 1 spread, 2 `* sin(U(0, pi/2))`.
    pub radius_mode: u32,
    /// `+0x44`: 1 aimed; 2 tangent on sphere shapes; 0 and 2 otherwise cone.
    pub velocity_mode: u32,
    /// `+0x48` and `+0x4c`: ejection speed centre and spread, world units
    /// per **tick**, both severity-scaled.
    pub speed_per_tick: (f32, f32),
    /// `+0x50`, radians: elevation over the emitter's horizontal plane.
    pub elevation: f32,
    /// `+0x54`, radians: added to the spawn direction's heading.
    pub azimuth: f32,
    /// `+0x58`, **degrees**: the half-angle both the cone and aimed velocity
    /// laws jitter by.
    pub cone_degrees: f32,
    /// `+0x5c` and `+0x60`: particle lifetime centre and spread, integer
    /// ticks.
    pub lifetime_ticks: (i32, i32),
    /// `+0x64` and `+0x68`: ticks between emissions, min and max.
    pub interval_ticks: (i32, i32),
    /// `+0x6c` and `+0x70`: particles per emission, min and max.
    pub per_emission: (i32, i32),
    /// `+0x74`, units per tick², **applied only under [`flags::GRAVITY`]**.
    pub gravity_per_tick2: f32,
    /// `+0xa0`: an emission that would exceed this is skipped whole.
    pub live_cap: i32,
    /// `+0xb8`: an index into the executable's blend table, whose entry's
    /// top nibble is the draw class. See [`Emitter::draw_class`].
    pub render_mode: u32,
    /// `+0xbc`: 2 picks a random [`Emitter::colours`] entry per particle;
    /// anything else walks the table over the particle's life.
    pub colour_mode: u32,
    /// `+0xc0`: 1 alpha-test, 2 additive, 3 alpha-over.
    pub blend_class: u32,
    /// `+0xc4`, 256 RGBA entries.
    ///
    /// Boxed because it is 1 KiB and an emitter is moved around by value.
    /// **This is authored game content**: it may be read from the user's own
    /// disc at runtime but never committed, per ADR-0006.
    pub colours: Box<[[u8; 4]; 256]>,
    /// `+0x4d8`: drawn half-size in world units, over the particle's life.
    pub size: Channel,
    /// `+0x5b8`: alpha in `0..=255`, over the particle's life.
    pub alpha: Channel,
    /// `+0x698`: billboard roll speed, radians per tick.
    pub rotation_speed: Channel,
    /// `+0x858`: a multiplier on the emitter extent, over the *emitter's*
    /// age rather than a particle's.
    pub emission_scale: Channel,
    /// `+0x4cc`: multiplies the whole system's tick-count `dt`.
    pub playback_rate: f32,
    /// `+0x4d0`: how much of the parent particle's velocity a child system
    /// spawned off it inherits.
    pub child_velocity_inherit: f32,
    /// `+0x4d4`: the probability a parent particle spawns this system.
    pub child_spawn_probability: f32,
    /// `+0x93c`: records that re-derive the severity-scaled parameters every
    /// tick. Their contents are not parsed; a non-zero count means this
    /// emitter's parameters are *not* constant over a burst.
    pub animated_attributes: i32,
    /// `+0x9a0`: sprite-atlas grid, columns and rows.
    pub atlas_grid: (u16, u16),
    /// `+0x9ac`: frames the atlas animation walks; `1` with a larger grid
    /// means a random still frame per particle (see
    /// [`flags::RANDOM_ATLAS_FRAME`]).
    pub atlas_frames: i32,
    /// `+0x9b4`'s list, in order.
    pub modifiers: Vec<Modifier>,
    /// `+0x944`: a system spawned once when a particle of this emitter dies,
    /// as an index into the vector [`ParticleSystem::emitters`] returned.
    pub death_child: Option<usize>,
    /// `+0x948`: a system attached to **each** particle of this emitter,
    /// with probability the child's own
    /// [`Emitter::child_spawn_probability`]. Same indexing as
    /// [`Emitter::death_child`].
    pub particle_child: Option<usize>,
}

impl Emitter {
    /// The draw class the executable's blend table maps
    /// [`Emitter::render_mode`] to.
    ///
    /// The table at `DAT_08ab2260` holds eight `u32`s of the form
    /// `(index + 1) << 28 | low_byte`, so the class is simply the index plus
    /// one - 3 is a rotating billboard, 6 and 7 are the two-point streaks,
    /// 1 and 2 an inline camera-facing quad. `None` for an index past the
    /// table, which no corpus file uses and which a caller should refuse
    /// rather than draw as something else.
    #[must_use]
    pub fn draw_class(&self) -> Option<u32> {
        (self.render_mode < 8).then(|| self.render_mode + 1)
    }

    /// The per-axis exponential velocity decay applied once per tick, from
    /// the first type-3 [`Modifier`]; `None` where the emitter has none.
    #[must_use]
    pub fn drag_per_tick(&self) -> Option<[f32; 3]> {
        self.modifiers
            .iter()
            .find(|modifier| modifier.kind == MODIFIER_DRAG)
            .map(|modifier| [modifier.params[0], modifier.params[1], modifier.params[2]])
    }

    /// Whether [`Emitter::gravity_per_tick2`] is live on this emitter.
    #[must_use]
    pub fn gravity_enabled(&self) -> bool {
        self.flags & flags::GRAVITY != 0
    }
}

/// The one [`Modifier::kind`] with a read consumer: per-axis exponential
/// drag.
pub const MODIFIER_DRAG: u32 = 3;

/// Depth-first walk of the emitter tree, appending each record.
fn walk_emitters(
    data: &[u8],
    base: usize,
    offset: usize,
    out: &mut Vec<Emitter>,
    seen: &mut Vec<usize>,
) -> Result<usize> {
    if seen.contains(&offset) || out.len() >= MAX_EMITTERS {
        return Err(Error::EmitterTreeUnbounded);
    }
    seen.push(offset);

    let (mut emitter, death, particle, sibling) = parse_emitter(data, base, offset)?;
    let index = out.len();
    // Reserve this record's slot before recursing, so children land after
    // their parent and the index stays stable.
    out.push(emitter_placeholder());

    if let Some(child) = death {
        emitter.death_child = Some(walk_emitters(data, base, child, out, seen)?);
    }
    if let Some(child) = particle {
        emitter.particle_child = Some(walk_emitters(data, base, child, out, seen)?);
    }
    out[index] = emitter;

    if let Some(next) = sibling {
        walk_emitters(data, base, next, out, seen)?;
    }
    Ok(index)
}

/// An emitter's slot in the output before its children have been walked.
/// Never observable: every placeholder is overwritten by the record it
/// stands in for.
fn emitter_placeholder() -> Emitter {
    let channel = || Channel {
        period: 0.0,
        mode: ChannelMode::Constant,
        lo: 0.0,
        hi: 0.0,
        keys: Vec::new(),
    };
    Emitter {
        name: String::new(),
        offset: 0,
        flags: 0,
        duration_ticks: 0.0,
        shape: 0,
        extent: 0.0,
        extent_unread: [0.0; 2],
        radius_mode: 0,
        velocity_mode: 0,
        speed_per_tick: (0.0, 0.0),
        elevation: 0.0,
        azimuth: 0.0,
        cone_degrees: 0.0,
        lifetime_ticks: (0, 0),
        interval_ticks: (0, 0),
        per_emission: (0, 0),
        gravity_per_tick2: 0.0,
        live_cap: 0,
        render_mode: 0,
        colour_mode: 0,
        blend_class: 0,
        colours: Box::new([[0; 4]; 256]),
        size: channel(),
        alpha: channel(),
        rotation_speed: channel(),
        emission_scale: channel(),
        playback_rate: 0.0,
        child_velocity_inherit: 0.0,
        child_spawn_probability: 0.0,
        animated_attributes: 0,
        atlas_grid: (0, 0),
        atlas_frames: 0,
        modifiers: Vec::new(),
        death_child: None,
        particle_child: None,
    }
}

/// One emitter record, plus the three raw tree offsets its caller resolves.
#[expect(clippy::type_complexity, reason = "one call site, unpacked at once")]
fn parse_emitter(
    data: &[u8],
    base: usize,
    offset: usize,
) -> Result<(Emitter, Option<usize>, Option<usize>, Option<usize>)> {
    let start = base + offset;
    if data.len() < start + EMITTER_LEN {
        return Err(Error::EmitterOutOfRange { offset });
    }
    let record = &data[start..];

    let nul = record[..NAME_LEN]
        .iter()
        .position(|&byte| byte == 0)
        .ok_or(Error::EmitterNameNotTerminated { offset })?;
    let name = String::from_utf8_lossy(&record[..nul]).into_owned();

    let float = |at: usize| f32::from_bits(word(record, at));
    let int = |at: usize| word(record, at) as i32;

    let mut colours = Box::new([[0u8; 4]; 256]);
    for (entry, bytes) in colours.iter_mut().zip(record[0xc4..].chunks_exact(4)) {
        entry.copy_from_slice(bytes);
    }

    let modifiers = parse_modifiers(data, base, word(record, 0x9b4) as usize)?;

    let emitter = Emitter {
        name,
        offset,
        flags: word(record, 0x20),
        duration_ticks: float(0x24),
        shape: word(record, 0x30),
        extent: float(0x34),
        extent_unread: [float(0x38), float(0x40)],
        radius_mode: word(record, 0x3c),
        velocity_mode: word(record, 0x44),
        speed_per_tick: (float(0x48), float(0x4c)),
        elevation: float(0x50),
        azimuth: float(0x54),
        cone_degrees: float(0x58),
        lifetime_ticks: (int(0x5c), int(0x60)),
        interval_ticks: (int(0x64), int(0x68)),
        per_emission: (int(0x6c), int(0x70)),
        gravity_per_tick2: float(0x74),
        live_cap: int(0xa0),
        render_mode: word(record, 0xb8),
        colour_mode: word(record, 0xbc),
        blend_class: word(record, 0xc0),
        colours,
        size: parse_channel(record, 0x4d8)?,
        alpha: parse_channel(record, 0x5b8)?,
        rotation_speed: parse_channel(record, 0x698)?,
        emission_scale: parse_channel(record, 0x858)?,
        playback_rate: float(0x4cc),
        child_velocity_inherit: float(0x4d0),
        child_spawn_probability: float(0x4d4),
        animated_attributes: int(0x93c),
        atlas_grid: (half(record, 0x9a0), half(record, 0x9a2)),
        atlas_frames: int(0x9ac),
        modifiers,
        death_child: None,
        particle_child: None,
    };

    // Zero is "no pointer": it would name the root, which is never a child
    // of anything.
    let child = |at: usize| match word(record, at) as usize {
        0 => None,
        value => Some(value),
    };
    Ok((emitter, child(0x944), child(0x948), child(0x94c)))
}

/// One channel block at `block`, relative to the record's own base.
fn parse_channel(record: &[u8], block: usize) -> Result<Channel> {
    let count = word(record, block + 0x08) as i32;
    let keys = usize::try_from(count).map_err(|_| Error::ChannelKeyCount { block, count })?;
    if keys > MAX_CHANNEL_KEYS {
        return Err(Error::ChannelKeyCount { block, count });
    }
    let keys = (0..keys)
        .map(|i| {
            let at = block + 0x14 + i * 8;
            (
                f32::from_bits(word(record, at)),
                f32::from_bits(word(record, at + 4)),
            )
        })
        .collect();
    Ok(Channel {
        period: f32::from_bits(word(record, block)),
        mode: ChannelMode::from_raw(word(record, block + 0x04)),
        lo: f32::from_bits(word(record, block + 0x0c)),
        hi: f32::from_bits(word(record, block + 0x10)),
        keys,
    })
}

/// The modifier list starting at `offset` from the resource base.
fn parse_modifiers(data: &[u8], base: usize, offset: usize) -> Result<Vec<Modifier>> {
    /// Bytes of a node that must be readable: the `next` pointer is last.
    const NODE_LEN: usize = 0x34;

    let mut out = Vec::new();
    let mut next = offset;
    while next != 0 {
        let start = base + next;
        if data.len() < start + NODE_LEN {
            return Err(Error::EmitterOutOfRange { offset: next });
        }
        let node = &data[start..];
        let mut params = [0.0f32; 9];
        for (i, param) in params.iter_mut().enumerate() {
            *param = f32::from_bits(word(node, i * 4));
        }
        out.push(Modifier {
            kind: word(node, 0x24),
            params,
        });
        next = word(node, 0x30) as usize;
        if out.len() > MAX_EMITTERS {
            return Err(Error::EmitterTreeUnbounded);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a particle system blob by hand. No game data in any test.
    fn pob(name: &str, slots: &[Option<u32>], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&0u32.to_le_bytes()); // patched below
        out.extend_from_slice(&(slots.len() as u16).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        for slot in slots {
            out.extend_from_slice(&slot.unwrap_or(EMPTY_SLOT).to_le_bytes());
        }
        let mut name_field = [0u8; NAME_LEN];
        let bytes = name.as_bytes();
        let take = bytes.len().min(NAME_LEN - 1);
        name_field[..take].copy_from_slice(&bytes[..take]);
        out.extend_from_slice(&name_field);
        out.extend_from_slice(payload);

        let declared = (HEADER_LEN + payload.len()) as u32;
        out[0x04..0x08].copy_from_slice(&declared.to_le_bytes());
        out
    }

    #[test]
    fn a_particle_system_parses_and_its_name_and_payload_survive() {
        let data = pob(
            "WO_TEST_SPARK",
            &[Some(1220), None, Some(2508)],
            &[1, 2, 3, 4, 5],
        );
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(parsed.name, "WO_TEST_SPARK");
        assert_eq!(parsed.slots, vec![Some(1220), None, Some(2508)]);
        assert_eq!(parsed.payload, &[1, 2, 3, 4, 5]);
        assert!(looks_like_particle_system(&data));
    }

    #[test]
    fn a_name_filling_the_field_is_not_null_terminated() {
        let mut data = pob(&"A".repeat(NAME_LEN - 1), &[], &[]);
        // The builder always leaves a NUL at the field's last byte for a name
        // of this length; overwrite it so the field truly has none.
        let last = data.len() - 1;
        data[last] = b'A';
        assert_eq!(ParticleSystem::parse(&data), Err(Error::NameNotTerminated));
    }

    #[test]
    fn an_empty_slot_table_round_trips_with_no_particle_data() {
        let data = pob("WO_EMPTY", &[], &[]);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert!(parsed.slots.is_empty());
        assert!(parsed.payload.is_empty());
    }

    #[test]
    fn a_size_disagreement_is_refused() {
        let mut data = pob("WO_TEST", &[Some(1)], &[0; 10]);
        data[0x04..0x08].copy_from_slice(&999u32.to_le_bytes());
        assert!(matches!(
            ParticleSystem::parse(&data),
            Err(Error::SizeDisagreement { .. })
        ));
    }

    #[test]
    fn an_unexpected_header_word_is_refused() {
        let mut data = pob("WO_TEST", &[], &[]);
        data[0x0a..0x0c].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            ParticleSystem::parse(&data),
            Err(Error::UnexpectedHeaderWord {
                field: "+0x0a",
                value: 2
            })
        );
    }

    #[test]
    fn a_truncated_table_is_refused() {
        let data = pob("WO_TEST", &[Some(1), Some(2), Some(3)], &[]);
        assert!(matches!(
            ParticleSystem::parse(&data[..data.len() - 40]),
            Err(Error::TableOutOfRange { .. })
        ));
    }

    #[test]
    fn a_blob_without_the_magic_is_refused() {
        let mut data = pob("WO_TEST", &[], &[]);
        data[0] = b'X';
        assert_eq!(ParticleSystem::parse(&data), Err(Error::NotSysp));
        assert!(!looks_like_particle_system(&data));
    }

    /// A one-slot blob whose fixup site sits at the very start of `payload`
    /// and whose baked value (`40`) resolves 8 bytes further in, where a
    /// `MARK` marker sits - the same two-hop shape a live PPSSPP trace
    /// confirmed for `WO_SHIP_COLL_SPARK_DAMAGE`'s 26 real slots.
    fn pob_with_one_fixup() -> Vec<u8> {
        let mut payload = vec![0u8; 12];
        payload[0..4].copy_from_slice(&40u32.to_le_bytes());
        payload[8..12].copy_from_slice(b"MARK");
        pob("WO_TEST_FIXUP", &[Some(32)], &payload)
    }

    #[test]
    fn a_slot_resolves_through_its_fixup_site_to_the_target() {
        let data = pob_with_one_fixup();
        let parsed = ParticleSystem::parse(&data).expect("parse");
        let resolved = parsed.resolve_slot(&data, 0).expect("resolve");
        assert_eq!(resolved, Some(8));
        assert_eq!(&parsed.payload[8..12], b"MARK");
    }

    #[test]
    fn an_unused_slot_resolves_to_none() {
        let data = pob("WO_TEST", &[None], &[]);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(parsed.resolve_slot(&data, 0), Ok(None));
    }

    #[test]
    fn an_index_past_the_table_resolves_to_none() {
        let data = pob_with_one_fixup();
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(parsed.resolve_slot(&data, 5), Ok(None));
    }

    #[test]
    fn a_fixup_site_past_the_end_is_refused() {
        let data = pob_with_one_fixup();
        let parsed = ParticleSystem::parse(&data).expect("parse");
        // The fixup site is at table_end(20) + slot(32) = 52; cut well before it.
        let short = &data[..40];
        assert_eq!(
            parsed.resolve_slot(short, 0),
            Err(Error::SlotOutOfRange { index: 0 })
        );
    }

    /// Builds a `SYSP` blob around a hand-laid resource image, whose first
    /// 32 bytes are the name field the container wants *and* the root
    /// emitter record's name. No game data in any test.
    fn pob_from_resource(slots: &[Option<u32>], resource: &[u8]) -> Vec<u8> {
        assert!(resource.len() >= NAME_LEN);
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&0u32.to_le_bytes()); // patched below
        out.extend_from_slice(&(slots.len() as u16).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        for slot in slots {
            out.extend_from_slice(&slot.unwrap_or(EMPTY_SLOT).to_le_bytes());
        }
        out.extend_from_slice(resource);
        let declared = (HEADER_LEN + resource.len() - NAME_LEN) as u32;
        out[0x04..0x08].copy_from_slice(&declared.to_le_bytes());
        out
    }

    /// A minimal emitter record: a name, and whatever fields a test writes
    /// over it. Everything else is zero, which every field tolerates.
    struct Record(Vec<u8>);

    impl Record {
        fn new(name: &str) -> Self {
            let mut bytes = vec![0u8; EMITTER_LEN];
            let take = name.len().min(NAME_LEN - 1);
            bytes[..take].copy_from_slice(&name.as_bytes()[..take]);
            Self(bytes)
        }

        fn u32(mut self, at: usize, value: u32) -> Self {
            self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
            self
        }

        fn i32(self, at: usize, value: i32) -> Self {
            self.u32(at, value as u32)
        }

        fn f32(self, at: usize, value: f32) -> Self {
            self.u32(at, value.to_bits())
        }

        /// A channel block: mode, bounds, and `(time, value)` keys.
        fn channel(
            mut self,
            block: usize,
            mode: u32,
            lo: f32,
            hi: f32,
            keys: &[(f32, f32)],
        ) -> Self {
            self = self
                .u32(block + 0x04, mode)
                .i32(block + 0x08, keys.len() as i32)
                .f32(block + 0x0c, lo)
                .f32(block + 0x10, hi);
            for (i, &(time, value)) in keys.iter().enumerate() {
                self = self
                    .f32(block + 0x14 + i * 8, time)
                    .f32(block + 0x18 + i * 8, value);
            }
            self
        }
    }

    /// Lays records at their own offsets in one resource image, padding
    /// between them.
    fn resource(records: &[(usize, Record)]) -> Vec<u8> {
        let end = records
            .iter()
            .map(|(offset, _)| offset + EMITTER_LEN)
            .max()
            .unwrap_or(NAME_LEN);
        let mut image = vec![0u8; end];
        for (offset, record) in records {
            image[*offset..*offset + EMITTER_LEN].copy_from_slice(&record.0);
        }
        image
    }

    /// The shape of every real file: a root whose `+0x94c` chains siblings,
    /// each a full record of the same layout at its own offset.
    #[test]
    fn a_sibling_chain_parses_root_first_with_every_field() {
        const SECOND: usize = 0x1000;
        let image = resource(&[
            (
                0,
                Record::new("WO_TEST_ROOT")
                    .u32(0x20, flags::GRAVITY | flags::STREAK_FROM_SPAWN)
                    .f32(0x24, 32.0)
                    .u32(0x30, 7)
                    .f32(0x34, 0.5)
                    .u32(0x44, 1)
                    .f32(0x48, 1.5)
                    .f32(0x4c, 0.25)
                    .f32(0x50, 0.75)
                    .f32(0x58, 30.0)
                    .i32(0x5c, 16)
                    .i32(0x60, 6)
                    .i32(0x64, 4)
                    .i32(0x68, 4)
                    .i32(0x6c, 3)
                    .i32(0x70, 3)
                    .f32(0x74, -0.015)
                    .i32(0xa0, 64)
                    .u32(0xb8, 5)
                    .u32(0xbc, 2)
                    .u32(0xc0, 2)
                    .u32(0xc4, u32::from_le_bytes([1, 2, 3, 4]))
                    .channel(0x4d8, 0, 0.5, 2.5, &[(0.0, 0.0), (1.0, 1.0)])
                    .channel(0x5b8, 2, 0.0, 200.0, &[])
                    .i32(0x9ac, 1)
                    .u32(0x94c, SECOND as u32),
            ),
            (SECOND, Record::new("bits").f32(0x24, 4.0).u32(0xb8, 2)),
        ]);
        let data = pob_from_resource(&[None, None], &image);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(parsed.name, "WO_TEST_ROOT");

        let emitters = parsed.emitters(&data).expect("emitters");
        assert_eq!(emitters.len(), 2);

        let root = &emitters[0];
        assert_eq!(root.name, "WO_TEST_ROOT");
        assert_eq!(root.offset, 0);
        assert!(root.gravity_enabled());
        assert_eq!(
            root.flags & flags::STREAK_FROM_SPAWN,
            flags::STREAK_FROM_SPAWN
        );
        assert_eq!(root.duration_ticks, 32.0);
        assert_eq!(root.shape, 7);
        assert_eq!(root.extent, 0.5);
        assert_eq!(root.velocity_mode, 1);
        assert_eq!(root.speed_per_tick, (1.5, 0.25));
        assert_eq!(root.elevation, 0.75);
        assert_eq!(root.cone_degrees, 30.0);
        assert_eq!(root.lifetime_ticks, (16, 6));
        assert_eq!(root.interval_ticks, (4, 4));
        assert_eq!(root.per_emission, (3, 3));
        assert_eq!(root.gravity_per_tick2, -0.015);
        assert_eq!(root.live_cap, 64);
        assert_eq!(root.draw_class(), Some(6));
        assert_eq!(root.colour_mode, 2);
        assert_eq!(root.blend_class, 2);
        assert_eq!(root.colours[0], [1, 2, 3, 4]);
        assert_eq!(root.size.mode, ChannelMode::Keyframed);
        assert_eq!(root.size.scaled_at(0.0), 0.5);
        assert_eq!(root.size.scaled_at(1.0), 2.5);
        assert_eq!(root.alpha.mode, ChannelMode::Constant);
        assert_eq!(root.alpha.hi, 200.0);
        assert!(root.death_child.is_none());
        assert!(root.particle_child.is_none());

        assert_eq!(emitters[1].name, "bits");
        assert_eq!(emitters[1].offset, SECOND);
        assert_eq!(emitters[1].draw_class(), Some(3));
        assert!(!emitters[1].gravity_enabled());
    }

    /// `WO_ROCKET_EXPLO`'s shape: a root whose *particles* each carry a
    /// child system, which is why the walk cannot be a flat sibling list.
    #[test]
    fn a_per_particle_child_is_reachable_from_its_parent() {
        const CHILD: usize = 0x1000;
        const SIBLING: usize = 0x2000;
        let image = resource(&[
            (
                0,
                Record::new("WO_TEST_EXPLO")
                    .u32(0x948, CHILD as u32)
                    .u32(0x94c, SIBLING as u32),
            ),
            (CHILD, Record::new("SMOKEMUSHROOM").f32(0x4d4, 0.25)),
            (SIBLING, Record::new("DEBRIS")),
        ]);
        let data = pob_from_resource(&[], &image);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        let emitters = parsed.emitters(&data).expect("emitters");

        assert_eq!(emitters.len(), 3);
        let names: Vec<&str> = emitters.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["WO_TEST_EXPLO", "SMOKEMUSHROOM", "DEBRIS"]);
        assert_eq!(emitters[0].particle_child, Some(1));
        assert_eq!(emitters[1].child_spawn_probability, 0.25);
        // The sibling is a peer, not a child.
        assert!(emitters[0].death_child.is_none());
        assert!(emitters[2].particle_child.is_none());
    }

    /// The type-3 modifier node is where per-axis drag comes from; a list
    /// with no type-3 node means no drag at all, not drag of zero.
    #[test]
    fn the_drag_modifier_is_read_off_the_node_list() {
        const NODE: usize = 0x1000;
        let mut image = resource(&[(0, Record::new("WO_TEST").u32(0x9b4, NODE as u32))]);
        image.resize(NODE + 0x34, 0);
        image[NODE..NODE + 4].copy_from_slice(&0.98f32.to_le_bytes());
        image[NODE + 4..NODE + 8].copy_from_slice(&0.85f32.to_le_bytes());
        image[NODE + 8..NODE + 12].copy_from_slice(&0.5f32.to_le_bytes());
        image[NODE + 0x24..NODE + 0x28].copy_from_slice(&MODIFIER_DRAG.to_le_bytes());

        let data = pob_from_resource(&[], &image);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        let emitters = parsed.emitters(&data).expect("emitters");
        assert_eq!(emitters[0].drag_per_tick(), Some([0.98, 0.85, 0.5]));

        let plain = pob_from_resource(&[], &resource(&[(0, Record::new("WO_TEST"))]));
        let parsed = ParticleSystem::parse(&plain).expect("parse");
        assert_eq!(
            parsed.emitters(&plain).expect("emitters")[0].drag_per_tick(),
            None
        );
    }

    #[test]
    fn a_keyframed_channel_interpolates_and_clamps() {
        let channel = Channel {
            period: 0.0,
            mode: ChannelMode::Keyframed,
            lo: 0.0,
            hi: 200.0,
            keys: vec![(0.0, 1.0), (0.5, 1.0), (1.0, 0.0)],
        };
        assert_eq!(channel.scaled_at(-1.0), 200.0);
        assert_eq!(channel.scaled_at(0.5), 200.0);
        assert_eq!(channel.scaled_at(0.75), 100.0);
        assert_eq!(channel.scaled_at(2.0), 0.0);
    }

    /// A tree pointer back at an already-visited record would otherwise walk
    /// forever off file bytes.
    #[test]
    fn a_cyclic_tree_is_refused() {
        const SECOND: usize = 0x1000;
        let image = resource(&[
            (0, Record::new("WO_TEST").u32(0x94c, SECOND as u32)),
            (SECOND, Record::new("loop").u32(0x94c, 0)),
        ]);
        // `+0x94c == 0` is read as "no sibling", so point it at itself
        // instead - the same revisit, expressible in the file.
        let mut image = image;
        image[SECOND + 0x94c..SECOND + 0x950].copy_from_slice(&(SECOND as u32).to_le_bytes());
        let data = pob_from_resource(&[], &image);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(parsed.emitters(&data), Err(Error::EmitterTreeUnbounded));
    }

    #[test]
    fn a_child_past_the_end_is_refused() {
        let image = resource(&[(0, Record::new("WO_TEST").u32(0x94c, 0x9_0000))]);
        let data = pob_from_resource(&[], &image);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(
            parsed.emitters(&data),
            Err(Error::EmitterOutOfRange { offset: 0x9_0000 })
        );
    }

    #[test]
    fn a_channel_declaring_more_keys_than_fit_is_refused() {
        let image = resource(&[(0, Record::new("WO_TEST").i32(0x4d8 + 0x08, 999))]);
        let data = pob_from_resource(&[], &image);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(
            parsed.emitters(&data),
            Err(Error::ChannelKeyCount {
                block: 0x4d8,
                count: 999
            })
        );
    }

    /// The blend table holds eight entries; an index past it has no draw
    /// handler, so it must be refusable rather than drawn as something else.
    #[test]
    fn a_render_mode_past_the_blend_table_has_no_draw_class() {
        let image = resource(&[(0, Record::new("WO_TEST").u32(0xb8, 9))]);
        let data = pob_from_resource(&[], &image);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(
            parsed.emitters(&data).expect("emitters")[0].draw_class(),
            None
        );
    }

    #[test]
    fn a_target_landing_before_the_payload_is_refused() {
        let mut payload = vec![0u8; 12];
        payload[0..4].copy_from_slice(&0u32.to_le_bytes()); // baked=0 -> target==table_end
        let data = pob("WO_TEST", &[Some(32)], &payload);
        let parsed = ParticleSystem::parse(&data).expect("parse");
        assert_eq!(
            parsed.resolve_slot(&data, 0),
            Err(Error::SlotOutOfRange { index: 0 })
        );
    }
}
