//! `.rcsmaterial`: the shader-variant table a Wipeout HD material ships.
//!
//! A material is not one shader. It is a **variant table** - up to 68 of them -
//! and which one a draw uses is chosen by a two-part key that the executable
//! builds from strings. That key is the whole of this module; the `SHO` blocks
//! it points at are the microcode, which nothing here decodes.
//!
//! See [`rcsmaterial.md`](../../../docs/formats/rcsmaterial.md), "What selects a
//! variant", for the evidence. In short:
//!
//! ```text
//! variant = the record whose ([0], [1]) == (hash(class), hash(features))
//! ```
//!
//! with `hash` the `~crc32` this format uses everywhere, `[0]` one of four
//! vertex-processing [`Class`] names, and `[1]` the concatenation of the
//! enabled [`Features`] tokens in one canonical order. **All 143 distinct
//! permutations across all 29,520 variants of all 693 materials on the disc
//! reproduce exactly**, and `(class, features)` is unique within every file, so
//! it is a key rather than a filter.
//!
//! # Half the key is a property of the chunk
//!
//! [`Features::for_chunk`] answers the half a model file already decides -
//! whether the chunk carries a lightmap coordinate, its own colour set, or
//! neither. The other half is a property of the **pass** (`Sun`, `ShadowMap`,
//! the spot lights, the Zone modes), and this module deliberately does not
//! guess it: [`Features`] is a set the caller completes.

use crate::rcsmodel::VertexDecl;
use oag_formats::ByteOrder;

/// The `~crc32` this format hashes names with.
///
/// The same function `oag_rcs::rcsmodel` uses for attribute names and the
/// `SHO` tables use for parameters - see
/// [`crc32.md`](../../../docs/formats/rcsmaterial.md).
#[must_use]
pub fn name_hash(name: &str) -> u32 {
    !crc32(name.as_bytes())
}

/// Bare CRC-32 (the reflected, `0xedb88320` variant), uninverted.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

/// Bytes in one variant record.
pub const RECORD_LEN: usize = 0x40;

/// A material's vertex-processing class - the `[0]` half of the key.
///
/// Four exist across the whole disc, and all four are NUL-terminated strings in
/// `EBOOT.elf` at `0x7a3560`. `StaticUncompressed` rests on a **single**
/// occurrence plus its preimage and is weaker than the other three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// Rigid world geometry with quantised positions. 17,333 variants.
    Static,
    /// [`Class::Static`] plus the Quake weapon's travelling displacement wave.
    ///
    /// Its vertex program is the `Static` one with a world-space displacement
    /// in front of the transform, driven by four extra `float4` whose names all
    /// fall to preimage: `quakePointA`, `quakePointB`, `quakeOffset`,
    /// `quakeTrackUpNormal`. 7,797 variants.
    StaticQuake,
    /// Geometry carried by a moving body - ships, weapons. 4,389 variants.
    RigidBody,
    /// Positions as full floats rather than quantised. One occurrence.
    StaticUncompressed,
}

impl Class {
    /// Every class, in the order the executable's own string table lists them.
    pub const ALL: [Self; 4] = [
        Self::Static,
        Self::StaticQuake,
        Self::RigidBody,
        Self::StaticUncompressed,
    ];

    /// The string the executable builds the key from.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Static => "Static",
            Self::StaticQuake => "StaticQuake",
            Self::RigidBody => "RigidBody",
            Self::StaticUncompressed => "StaticUncompressed",
        }
    }

    /// The `[0]` word of a record selecting this class.
    #[must_use]
    pub fn hash(self) -> u32 {
        name_hash(self.name())
    }

    /// Which class a `[0]` word names, or `None` for one this reading does not
    /// know - reported rather than defaulted, because a fifth class would be a
    /// finding.
    #[must_use]
    pub fn from_hash(hash: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.hash() == hash)
    }
}

/// The feature tokens, **in the canonical order the names concatenate**.
///
/// This order is derived, not assumed: it is the topological sort of the
/// pairwise precedence graph over all 143 shipped permutation names, which is
/// acyclic, and every one of the 143 rebuilds from it and hashes to its own
/// record. Where two tokens never co-occur - the `Ambient`/`IleLightmap`/
/// `IleVertex` group, `SVC0`/`SVC1`, `HalfBright`/`ShadowToAlpha`,
/// `ZoneMode`/`ZoneTrans`, the `Spot*` group - nothing constrains their
/// relative position and nothing depends on it.
///
/// **`Spot3`, `FalseLight` and `NoAlbedo` are in the executable's table and in
/// no shipped permutation.** They are carried here so the table is the
/// executable's, not a subset of what happens to ship.
pub const TOKENS: [&str; 22] = [
    "AmbientShadow",
    "HalfBright",
    "IBL",
    "ShadowToAlpha",
    "SunOcclusionLightmap",
    "SunOcclusionVertex",
    "ZAlphaOnly",
    "ZoneMode",
    "ZoneTrans",
    "Ambient",
    "IleLightmap",
    "IleVertex",
    "Sun",
    "ShadowMap",
    "Spot0",
    "Spot1",
    "Spot2",
    "Spot3",
    "SVC0",
    "SVC1",
    "FalseLight",
    "NoAlbedo",
];

/// **The permutation word: twelve bits of fields, not one bit per token.**
///
/// `Features::from_pass_word` decodes it. The layout is read out of
/// `0x003f1028`, a startup loop that runs a mask from 0 to 4095
/// (`cmpdi 7,28,4096` at `0x3f11d0`), builds each permutation name, hashes it
/// (`bl 0x5a2090`, then `not 3,3` - the `~crc32` above) and fills a
/// 4096-entry table at the global `0x00d3e220`. `0x003f0ff8` is the whole
/// accessor - `slwi 3,3,2 ; lwz 9,-21696(2) ; lwzx 3,9,3 ; blr` - so the
/// key's second half is literally `table[word]`.
///
/// | Bits | Meaning |
/// | --- | --- |
/// | 0 | `Sun` |
/// | 1-2 | `Ambient` / `IleVertex` / `IleLightmap` / `IBL` |
/// | 3-4 | `Spot0` / `Spot1` / `Spot2` / `Spot3` |
/// | 5 | set `ShadowToAlpha`, clear `HalfBright` |
/// | 6 | `ShadowMap` |
/// | 7 | `FalseLight` |
/// | 8 | `ZoneMode` |
/// | 9 | `ZoneTrans` |
/// | 10 | `NoAlbedo` |
/// | 11 | set `SVC1`, clear `SVC0` |
///
/// Two of the twenty-two [`TOKENS`] are the *clear* side of a bit rather than
/// a bit of their own, and four more are not in this word at all:
/// `ZAlphaOnly`, `AmbientShadow`, `SunOcclusionLightmap` and
/// `SunOcclusionVertex` are hashed standalone by `0x003f1300` into
/// `0x00d3e220 + 0x4a20`, past the 0x4000 bytes the table occupies.
///
/// **A previous reading of this had one bit per token, from a "pointer table"
/// at `0x8b7f08`. There is no such table.** Those 24 consecutive words are a
/// slice of the **TOC** at base `0x8bd3c4`: the token strings sit together in
/// `.rodata` because they were declared together, so their TOC entries sit
/// together too. The entry immediately before them points at `0x00d3e220`
/// itself and two entries after them are `time` and `viewProj` - neither of
/// which belongs to a flag layout. A selector built on that order would have
/// picked the wrong variant every time.
///
/// Confidence 93, and the sweep is **not** what earns it. Sweeping all 4096
/// words does give 4096 distinct hashes with no collisions, and accounts for
/// all 143 shipped permutations - but that result is *invariant under any
/// permutation of which bit means what*, since relabelling the bits is a
/// bijection on 0..4095 and leaves the generated name set alone. The sweep is
/// therefore decisive about the token vocabulary, the field structure and the
/// concatenation order, and says nothing about numeric bit positions. Those
/// come from the loop's own bit tests, corroborated by four independent
/// agreements in the pass code.
pub const PASS_WORD_BITS: u32 = 12;

/// The permutation word an **ordinary lit race pass** sets on a track surface.
///
/// `Sun`, and nothing else the pass decides: `HalfBright` and `SVC0` are the
/// clear sides of bits 5 and 11, and `Spot0` is field value zero, so the word
/// is `1`. Its name is `HalfBrightAmbientSunSpot0SVC0`, which the executable
/// ships as a literal at `0x7b17d0` - the only permutation name that appears in
/// the binary as a string rather than being built.
///
/// The chunk half is added on top: `| 0b100` for a lightmapped chunk
/// (`IleLightmap`), `| 0b010` for a vertex-coloured one (`IleVertex`).
///
/// Confidence 95, and it is a **code reading rather than a disc measurement**:
/// see [`PASS_WORD_BITS`] on why the 143-of-143 sweep cannot corroborate a bit
/// position. `Job RenderTrackWithLights_zWriters` (`0x408fa8`) and `_blended`
/// (`0x4074e0`) - reached through slot 3 of each job's vtable - never write
/// bits 5, 6 or 7. `ShadowToAlpha` belongs to the shadow
/// compositing pass (`li 9,32` at `0x405d48`) and `ShadowMap` only to the
/// model and ship path (`li 6,64` at `0x3ea58c`), which is why no shipped
/// variant pairs `ShadowMap` with an `Ile*` token.
pub const LIT_RACE_PASS: u32 = 0b0000_0000_0001;

/// A set of [`TOKENS`], which is the `[1]` half of the key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Features(u32);

impl Features {
    /// The empty set, whose name is the empty string.
    pub const NONE: Self = Self(0);

    /// The set holding exactly the named token, or [`Self::NONE`] for a name
    /// that is not one.
    #[must_use]
    pub fn token(name: &str) -> Self {
        match TOKENS.iter().position(|t| *t == name) {
            Some(bit) => Self(1 << bit),
            None => Self::NONE,
        }
    }

    /// This set with `other`'s tokens added.
    #[must_use]
    pub fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether the named token is in this set.
    #[must_use]
    pub fn has(self, name: &str) -> bool {
        TOKENS
            .iter()
            .position(|t| *t == name)
            .is_some_and(|bit| self.0 & (1 << bit) != 0)
    }

    /// The set a **permutation word** names - see [`PASS_WORD_BITS`] for the
    /// layout and where it was read.
    ///
    /// Bits above 11 are ignored: the word is twelve bits wide and a caller
    /// holding more has a different quantity.
    #[must_use]
    pub fn from_pass_word(word: u32) -> Self {
        let one = |set: bool, name: &str| {
            if set { Self::token(name) } else { Self::NONE }
        };
        Self::NONE
            .with(one(word & 1 != 0, "Sun"))
            .with(Self::token(
                ["Ambient", "IleVertex", "IleLightmap", "IBL"][(word >> 1) as usize & 3],
            ))
            .with(Self::token(
                ["Spot0", "Spot1", "Spot2", "Spot3"][(word >> 3) as usize & 3],
            ))
            .with(Self::token(if word >> 5 & 1 != 0 {
                "ShadowToAlpha"
            } else {
                "HalfBright"
            }))
            .with(one(word >> 6 & 1 != 0, "ShadowMap"))
            .with(one(word >> 7 & 1 != 0, "FalseLight"))
            .with(one(word >> 8 & 1 != 0, "ZoneMode"))
            .with(one(word >> 9 & 1 != 0, "ZoneTrans"))
            .with(one(word >> 10 & 1 != 0, "NoAlbedo"))
            .with(Self::token(if word >> 11 & 1 != 0 {
                "SVC1"
            } else {
                "SVC0"
            }))
    }

    /// The tokens, in canonical order.
    pub fn tokens(self) -> impl Iterator<Item = &'static str> {
        TOKENS
            .into_iter()
            .enumerate()
            .filter(move |(bit, _)| self.0 & (1 << bit) != 0)
            .map(|(_, name)| name)
    }

    /// The string the executable builds the key from.
    #[must_use]
    pub fn name(self) -> String {
        self.tokens().collect()
    }

    /// The `[1]` word of a record selecting this set.
    #[must_use]
    pub fn hash(self) -> u32 {
        name_hash(&self.name())
    }

    /// **The half of the key a chunk decides, as bits of a permutation word.**
    ///
    /// Composing with [`Self::with`] instead is a trap and does not work:
    /// `Features` is a *set*, so unioning a chunk's `IleLightmap` onto a pass
    /// word that already carries `Ambient` leaves both set and names a
    /// permutation no material ships. Bits 1-2 are a four-way **field**, so
    /// they have to be replaced rather than added.
    ///
    /// ```
    /// use oag_rcs::rcsmaterial::{Features, LIT_RACE_PASS};
    /// let word = Features::chunk_word(LIT_RACE_PASS, None);
    /// assert_eq!(Features::from_pass_word(word).name(), "HalfBrightAmbientSunSpot0SVC0");
    /// ```
    #[must_use]
    pub fn chunk_word(base: u32, decl: Option<&VertexDecl>) -> u32 {
        let field = match decl {
            Some(d) if d.lightmap_texcoord().is_some() => 2,
            // `light_colour_set`, not `vertex_colour`: this asks which
            // shader permutation the chunk needs, and a ship hull's
            // `VertexColour1` needs `Ambient` like any other chunk with no
            // baked light term - see `VertexDecl::light_colour_set`'s own
            // doc comment for why the two questions are not one.
            Some(d) if d.light_colour_set() => 1,
            // No lightmap coordinate and no colour set: `Ambient`, field zero.
            _ => 0,
        };
        (base & !0b110) | (field << 1)
    }

    /// **The half of the key a chunk's own declaration decides.**
    ///
    /// Measured over all 29,520 variants: `IleLightmap` implies a `lightmapUV`
    /// attribute on 5,796 of 5,796, and `IleVertex` implies a colour set on
    /// 4,464 of 4,464. The converses hold too, with *named* rather than
    /// residual exceptions - a `lightmapUV` without `IleLightmap` occurs on
    /// exactly 360 variants and every one is the standalone permutation
    /// `SunOcclusionLightmap`.
    ///
    /// `SVC0` is unconditional here: `SVC1` selects a vertex colour stream the
    /// **SPU** writes (`SpuVertexColours`, `0x868f8229`), which no `.rcsmodel`
    /// carries and this project does not produce.
    ///
    /// Confidence 75 - a correlation over the assets and a string table, with
    /// no code that computes the key read. It does not answer the
    /// pass-determined tokens, and the caller must add those.
    #[must_use]
    pub fn for_chunk(decl: Option<&VertexDecl>) -> Self {
        let base = Self::token("SVC0");
        let Some(decl) = decl else {
            return base.with(Self::token("Ambient"));
        };
        if decl.lightmap_texcoord().is_some() {
            base.with(Self::token("IleLightmap"))
        } else if decl.light_colour_set() {
            base.with(Self::token("IleVertex"))
        } else {
            base.with(Self::token("Ambient"))
        }
    }
}

/// Where one of a variant's two programs lives, and what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Byte offset of the `SHO` block within the file.
    pub offset: usize,
    /// Its length in bytes.
    pub len: usize,
    /// The program's content hash.
    ///
    /// Constant across every variant sharing an offset - 25,040 of 25,040 on
    /// the vertex side and 10,276 of 10,276 on the fragment side - which is why
    /// variants share blocks rather than duplicating them.
    pub program_hash: u32,
}

/// One row of the variant table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    /// The `[0]` half of the key, or `None` for a hash this reading does not
    /// know.
    pub class: Option<Class>,
    /// The raw `[0]` word, kept so an unknown class is still reportable.
    pub class_hash: u32,
    /// The `[1]` half of the key, as the file states it.
    pub feature_hash: u32,
    /// The vertex program.
    pub vertex: Block,
    /// The fragment program.
    pub fragment: Block,
}

/// What a program **declares** it is fed, read from its `SHO` block header.
///
/// **What it does not tell you is which lighting family the surface is in**,
/// and that was tried. Splitting on "declares the `lightmap` sampler" /
/// "declares `constantAmbientColour`" / "neither" puts **447 of Talon's
/// Junction's 978 drawn chunks in the third bucket**, and that bucket is not a
/// family: alongside the billboards and scanline screens it holds
/// `track_wall` (22 chunks), `glasstest` (33), `simplefogdiffuse` (31) and
/// `diffusewithalphachannel` (29) - ordinary lit surfaces. The reason is
/// structural. A surface lit only by the interpolated per-vertex term declares
/// neither of those two, **because both of its light sources are
/// interpolators rather than uniforms**, and an interpolator is not in this
/// table. Telling a lit surface from an emissive one needs the microcode - it
/// is the question of whether `f[TC1]` reaches the albedo multiply - not the
/// header. Treating the third bucket as unlit would have drawn the track walls
/// at full albedo.
///
/// Layout, at the block's own offset: `SHO\x08`, a `u32` that is 1 for a
/// fragment program, then eight big-endian `u16` - version, three counts
/// (attributes, parameters, samplers), three table offsets, and where the
/// program data starts. An attribute record is 8 bytes, a parameter record 12
/// and a sampler record 8. The three tables abut exactly, which is the framing
/// check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Declared {
    /// Every parameter name hash the block declares.
    pub parameters: Vec<u32>,
    /// Every sampler name hash, with the texture unit it binds.
    pub samplers: Vec<(u32, u32)>,
    /// Every parameter's own record, past the hash [`Self::parameters`]
    /// already carries: `(hash, vreg, fslot)`, in table order.
    ///
    /// `vreg` is `0xffff` when the parameter has no hardware constant
    /// register of its own and is patched straight into the fragment code
    /// instead - see
    /// [`crate::rcsmaterial::fragment::Program::patches`], which walks
    /// `fslot` the rest of the way: a byte offset (from this block's own
    /// start) to a `u16` index into an offset table in the program
    /// sub-header, each entry a list of the 16-byte code slots that
    /// parameter overwrites. Read on
    /// `docs/formats/rcsmaterial.md`, "The glass family's second slot:
    /// traced, not solved", and ported from `scripts/ps3-microcode.py`'s
    /// `fp_patch_slots`/`fp_patch_map`, the reference this mirrors.
    pub parameter_patches: Vec<(u32, u16, u16)>,
}

/// `~crc32("lightmap")`, the sampler a prelit surface's atlas binds to.
pub const LIGHTMAP_SAMPLER: u32 = 0x37b5_db58;
/// `~crc32("constantAmbientColour")`, the `Lighting.Constant ambient colour`
/// key of a circuit's `.envsettings`.
pub const CONSTANT_AMBIENT: u32 = 0x81db_67ea;
/// `~crc32("directionalLight0Colour")`, the sun's colour.
pub const SUN_COLOUR: u32 = 0x2dba_643d;
/// `~crc32("directionalLight0DirectionWorldSpace")`, the sun's direction.
pub const SUN_DIRECTION: u32 = 0x02df_31e5;
/// `~crc32("SpecularPower")`, the specular exponent -
/// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "Ships have no
/// Lambert diffuse either" names it as the parameter a resolved `0.0`
/// specular-exponent chain is the strongest candidate for having been
/// patched from, at draw time, rather than baked in the file.
pub const SPECULAR_POWER: u32 = 0x81e0_e773;

impl Declared {
    /// Reads one `SHO` block's declaration tables, or `None` if `at` does not
    /// open one.
    #[must_use]
    pub fn parse(data: &[u8], at: usize) -> Option<Self> {
        if at + 0x18 > data.len() || &data.get(at..at + 4)? != b"SHO\x08" {
            return None;
        }
        let u16_at = |o: usize| -> usize {
            usize::from(u16::from_be_bytes([data[at + o], data[at + o + 1]]))
        };
        let u32_at = |o: usize| -> Option<u32> {
            Some(u32::from_be_bytes(data.get(o..o + 4)?.try_into().ok()?))
        };
        let u16_at_abs = |o: usize| -> Option<u16> {
            Some(u16::from_be_bytes(data.get(o..o + 2)?.try_into().ok()?))
        };
        let (attrs, params, samplers) = (u16_at(0x0a), u16_at(0x0c), u16_at(0x0e));
        let (o0, o1, o2) = (u16_at(0x10), u16_at(0x12), u16_at(0x14));
        // The framing check: each table starts where the previous one ended.
        if o1 != o0 + 8 * attrs || o2 != o1 + 12 * params {
            return None;
        }
        // One record is `u32 hash, u16 ty, u16 count, u16 vreg, u16 fslot` -
        // `parameters` and `parameter_patches` read the same 12 bytes, kept as
        // two fields rather than one so every existing reader of the bare
        // hash list stays untouched.
        let parameter_patches: Vec<(u32, u16, u16)> = (0..params)
            .filter_map(|i| {
                let r = at + o1 + i * 12;
                Some((u32_at(r)?, u16_at_abs(r + 8)?, u16_at_abs(r + 10)?))
            })
            .collect();
        let parameters = parameter_patches.iter().map(|&(h, ..)| h).collect();
        let samplers = (0..samplers)
            .filter_map(|i| Some((u32_at(at + o2 + i * 8)?, u32_at(at + o2 + i * 8 + 4)?)))
            .collect();
        Some(Self {
            parameters,
            samplers,
            parameter_patches,
        })
    }

    /// Whether the block binds the circuit's baked lighting atlas.
    #[must_use]
    pub fn samples_lightmap(&self) -> bool {
        self.samplers.iter().any(|(h, _)| *h == LIGHTMAP_SAMPLER)
    }

    /// Whether the block declares the scene's **directional light**.
    ///
    /// Either half of it counts: a program fed the sun's direction is fed the
    /// sun. Together with [`Self::takes_constant_ambient`] this is the
    /// three-way lighting key - see `oag_mesh::mesh::slots::NO_SUN` for why
    /// asking about the directional light is what makes the split hold where
    /// an earlier one did not.
    #[must_use]
    pub fn takes_directional_light(&self) -> bool {
        self.parameters
            .iter()
            .any(|hash| matches!(*hash, SUN_COLOUR | SUN_DIRECTION))
    }

    /// Whether the block is patched with `constantAmbientColour`.
    #[must_use]
    pub fn takes_constant_ambient(&self) -> bool {
        self.parameters.contains(&CONSTANT_AMBIENT)
    }
}

/// A parsed `.rcsmaterial`: its variant table, and nothing else yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RcsMaterial {
    /// Every variant, in file order.
    pub variants: Vec<Variant>,
}

/// What can go wrong reading one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not enough bytes for the 16-byte header.
    TooShort {
        /// What was supplied.
        got: usize,
    },
    /// The variant table does not fit inside the file.
    OutOfBounds {
        /// The byte it wanted.
        end: usize,
        /// The file length.
        len: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => {
                write!(f, "a .rcsmaterial is at least 16 bytes, got {got}")
            }
            Self::OutOfBounds { end, len } => {
                write!(f, "the variant table ends at {end}, past the file's {len}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl RcsMaterial {
    /// Reads the variant table.
    ///
    /// The header is four big-endian words: the variant count, the table's
    /// offset, the offset of a pointer-relocation list, and zero. Each record
    /// is [`RECORD_LEN`] bytes of sixteen words, and **the last record's
    /// trailing sixteen zero bytes are elided** - which is why the length check
    /// below allows the table to end `0x10` short.
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 16 {
            return Err(Error::TooShort { got: data.len() });
        }
        let count = ByteOrder::Big.u32(data, 0) as usize;
        let table = ByteOrder::Big.u32(data, 4) as usize;
        let end = table + count * RECORD_LEN;
        // The elision above: the final record is short by its four zero words.
        if count > 0 && end.saturating_sub(0x10) > data.len() {
            return Err(Error::OutOfBounds {
                end,
                len: data.len(),
            });
        }
        let word = |at: usize| {
            if at + 4 <= data.len() {
                ByteOrder::Big.u32(data, at)
            } else {
                0
            }
        };
        let variants = (0..count)
            .map(|i| {
                let r = table + i * RECORD_LEN;
                let class_hash = word(r);
                Variant {
                    class: Class::from_hash(class_hash),
                    class_hash,
                    feature_hash: word(r + 4),
                    vertex: Block {
                        offset: word(r + 0x10) as usize,
                        len: word(r + 0x18) as usize,
                        program_hash: word(r + 0x20),
                    },
                    fragment: Block {
                        offset: word(r + 0x14) as usize,
                        len: word(r + 0x1c) as usize,
                        program_hash: word(r + 0x24),
                    },
                }
            })
            .collect();
        Ok(Self { variants })
    }

    /// The variant a `(class, features)` key selects, or `None` if the file
    /// ships no such permutation.
    ///
    /// `None` is a real answer and not a failure: a material carries only the
    /// permutations its circuit needs, so asking for `Spot3` of a material that
    /// never sees three spot lights is expected to miss.
    #[must_use]
    pub fn variant(&self, class: Class, features: Features) -> Option<&Variant> {
        let (c, f) = (class.hash(), features.hash());
        self.variants
            .iter()
            .find(|v| v.class_hash == c && v.feature_hash == f)
    }
}

mod factor;
pub mod fragment;
pub mod names;
pub mod vertex;

#[cfg(test)]
mod tests;
