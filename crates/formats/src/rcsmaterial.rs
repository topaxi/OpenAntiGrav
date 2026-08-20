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

use crate::ByteOrder;
use crate::rcsmodel::VertexDecl;

/// The `~crc32` this format hashes names with.
///
/// The same function `oag_formats::rcsmodel` uses for attribute names and the
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

/// The executable's own **flag-bit order** for the feature tokens, which is
/// *not* the order they concatenate in.
///
/// A pointer table at `EBOOT.elf` vaddr `0x8b7f08` holds 21 token strings
/// followed by the three [`Class`] names, and its index is the bit position a
/// render pass sets. **Bit 3 points at an empty string** - a flag that
/// contributes nothing to a permutation name - so it is `None` here rather than
/// silently closing the gap.
///
/// This is the layout, not the assignment: **which bits a given pass sets is
/// still unread.** The frame's own pass list is the anchor, thirteen job names
/// in order at vaddr `0x7b1008`: `PrecomputeTrackFrameData`, a visibility
/// fence, `RenderBillBoards`, `RenderModelShadowMaps`, `RenderSpotShadowMaps`,
/// `RenderTrackReflect`, `RenderTrackRefract`,
/// `RenderTrackWithLights_zWriters`, `RenderModelShadowsOnTrack`,
/// `RenderModelAmbientShadowsOnTrack`, `RenderTrackWithLights_blended`,
/// `RenderShips`, `ClearTrackVisibilityFlags`.
///
/// That the two orders differ is itself the finding that keeps [`TOKENS`]
/// honest: a builder walking *this* table would spell variant #5 of
/// `track_surface` `HalfBrightSunSVC0AmbientSpot0`, and the executable ships
/// the string `HalfBrightAmbientSunSpot0SVC0`. So the concatenation order is
/// straight-line code rather than a loop over this table, which is why
/// [`TOKENS`] had to be derived from the names instead.
pub const FLAG_BITS: [Option<&str>; 21] = [
    Some("ShadowToAlpha"),
    Some("HalfBright"),
    Some("Sun"),
    None,
    Some("ShadowMap"),
    Some("FalseLight"),
    Some("ZoneMode"),
    Some("ZoneTrans"),
    Some("NoAlbedo"),
    Some("SVC1"),
    Some("SVC0"),
    Some("IBL"),
    Some("Ambient"),
    Some("IleVertex"),
    Some("IleLightmap"),
    Some("Spot0"),
    Some("Spot1"),
    Some("Spot2"),
    Some("Spot3"),
    Some("ZAlphaOnly"),
    Some("AmbientShadow"),
];

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

    /// The set a render pass's flags word names, by [`FLAG_BITS`].
    ///
    /// Bits with no token - bit 3, and anything above 20 - are ignored rather
    /// than rejected: the layout is read and the *assignment* is not, so a word
    /// carrying a bit this reading cannot name is a finding for the caller
    /// rather than an error here.
    #[must_use]
    pub fn from_flags(flags: u32) -> Self {
        let mut out = Self::NONE;
        for (bit, token) in FLAG_BITS.iter().enumerate() {
            if let Some(name) = token.filter(|_| flags & (1 << bit) != 0) {
                out = out.with(Self::token(name));
            }
        }
        out
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
        } else if decl.vertex_colour().is_some() {
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

#[cfg(test)]
mod tests;
