//! The Vita's shader-program container (`GXP\0`), as Wipeout 2048 ships it.
//!
//! ```text
//! +0x00  u32   magic                "GXP\0"
//! +0x04  u8    major version        1 on every program measured
//! +0x05  u8    minor version        4 on every program measured
//! +0x06  u16   toolchain version    0x0165 on every program measured
//! +0x08  u32   size, the whole program including its name strings
//! +0x0c  u32   binary guid
//! +0x10  u32   source guid
//! +0x14  u32   flags; bit 0 set = fragment program, clear = vertex
//! +0x24  u32   parameter count
//! +0x28  u32   parameter table offset, relative to this field
//! +0x3c  u32   primary instruction count
//! +0x40  u32   primary code offset,   relative to this field
//! +0x44  u32   secondary instruction count
//! +0x48  u32   secondary code offset, relative to this field
//! +0x4c  u32   secondary code end,    relative to this field
//! +0x58  u32   uniform-image length, in 4-byte words
//! +0x70  u32   literal count
//! +0x74  u32   literal table offset,  relative to this field
//! +0x78  u32   count of the 8-byte records at +0x7c, zero on all but 72
//! +0x7c  u32   that table's offset,   relative to this field
//! +0x80  u32   count of the 4-byte records at +0x84
//! +0x84  u32   that table's offset,   relative to this field
//! +0x88  u32   count of the records at +0x8c, zero on every program measured
//! +0x8c  u32   that table's offset,   relative to this field
//! +0x90  u32   container count
//! +0x94  u32   container table offset, relative to this field
//! then `parameter count` entries, 16 bytes each:
//! +0x00  i32   name offset, relative to this field; 0 means unnamed
//! +0x04  u16   nibbles: category | type | component count | container index
//! +0x06  u16   unread
//! +0x08  u32   array size
//! +0x0c  u32   resource index
//! then the name strings, NUL-terminated, to the declared size exactly.
//! ```
//!
//! Everything is **little-endian**, like the rest of this title's package and
//! unlike [`crate::rcsmaterial`]'s PS3 microcode, which is a big-endian file.
//! The layout is Sony's own `SceGxmProgram`, as documented by Vita3K, *and*
//! was re-derived here from the bytes - see `docs/formats/gxp.md`, which
//! carries both readings and where they were taken.
//! [`scripts/vita-gxp.py`] is the reference this ports, the way
//! `scripts/ps3-microcode.py` is [`crate::rcsmaterial::fragment`]'s.
//!
//! [`scripts/vita-gxp.py`]: https://github.com/topaxi/OpenAntiGrav/blob/main/scripts/vita-gxp.py
//!
//! # Why closure, and not "the parse returned something"
//!
//! Every offset above was derived by sweeping the whole corpus - both eboots
//! and every entry of all three `.psarc`s, 97,902 magics - and keeping only
//! the assignment that survived all of it. Two rounds of that mattered:
//!
//! - **A candidate that is vacuously true survives a weak test.** `+0x20` and
//!   `+0x24` both pass "the parameter table lands inside the program" because
//!   `+0x20` is zero and zero parameters fit anywhere. Only the closure below,
//!   the last name's NUL landing exactly on the declared size, separates them,
//!   and it does so 3,540 to 0.
//! - **A field that is constant over a sample is not constant.** `+0x20` is
//!   zero on all 44,603 programs in one eboot and `data.psarc`, and carries
//!   `1` or `0x112` on 24 programs in the other two archives. It stays unread
//!   here. Sampling the corpus would have shipped it as `always_zero`. The
//!   same lesson at file scale: the base `eboot.elf` embeds 111 programs and
//!   the v1.04 patch's embeds 67, so both are swept.
//!
//! [`Program::parse`] therefore refuses anything that does not close, and
//! these are the checks, each of which held on every program in the corpus:
//!
//! - the secondary program's `offset + 8 * count` is exactly its declared end;
//! - the literal table begins exactly where the primary code ends, which is
//!   what ties `+0x74` (a second, redundant statement of that address) to
//!   `+0x3c`/`+0x40`;
//! - the six declared tables - literals, the uniform image, `+0x7c`, `+0x84`,
//!   `+0x8c` and the containers - tile the space between the primary code's
//!   end and the parameter table exactly, with no gap and no overlap. Their
//!   *order* is not assumed: the table at `+0x7c` follows the containers on
//!   the 72 skinned programs that have one and sits empty at the parameter
//!   table everywhere else, so the check sorts them by start;
//! - the last parameter name's NUL is the program's last byte.
//!
//! A wrong stride or a wrong base fails several of those within one program.
//!
//! # What is not read
//!
//! **The USSE instruction stream is located, not decoded.** Both programs'
//! extents close on their own declared counts and the bytes are handed back as
//! bytes. Naming the ISA's opcodes is separate work and there is no
//! half-finished disassembly here to mistake for one.
//!
//! Header fields absent from the table above are unread rather than guessed.

use core::fmt;

/// `"GXP\0"`, read as a little-endian `u32`.
pub const MAGIC: u32 = u32::from_le_bytes(*b"GXP\0");

/// The shortest header this module reads a field out of: `+0x94` plus its own
/// four bytes. No shipped program is anywhere near this small.
pub const HEADER_MIN: usize = 0x98;

/// Bytes per parameter entry.
pub const PARAMETER_LEN: usize = 16;

/// The most parameters a program may declare before the count is treated as
/// garbage rather than as a very large program. The corpus tops out at 32.
const MAX_PARAMETERS: u32 = 4096;

/// What went wrong reading a `GXP` program.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Fewer bytes than the header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The magic tag is not `"GXP\0"`.
    BadMagic {
        /// The four bytes found, as a `u32`.
        tag: u32,
    },
    /// The declared size is below [`HEADER_MIN`] or runs past the blob.
    ///
    /// On a scan of arbitrary bytes this is what a chance `GXP\0` looks like,
    /// so it says "not a container" rather than "a container that is broken".
    NotAContainer {
        /// The size the header declares.
        size: u32,
        /// Bytes actually available from the magic onwards.
        available: usize,
    },
    /// The parameter count is implausible, or the table runs off the end.
    BadParameterCount {
        /// The count found.
        count: u32,
        /// Where the table would start.
        offset: usize,
    },
    /// A parameter's name offset is outside the program, or its string is not
    /// terminated inside it.
    BadParameterName {
        /// Which parameter.
        index: usize,
        /// The offset resolved to.
        offset: usize,
    },
    /// One of the closure checks in the module docs failed. The program is a
    /// `GXP` container whose header does not describe itself consistently.
    DoesNotClose {
        /// Which check, as prose.
        what: &'static str,
        /// The offset the header implies.
        got: usize,
        /// The offset the rest of the header requires.
        want: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "{got} bytes is shorter than a GXP header"),
            Self::BadMagic { tag } => write!(f, "magic {tag:#010x} is not GXP\\0"),
            Self::NotAContainer { size, available } => write!(
                f,
                "declared size {size:#x} does not fit the {available} bytes available"
            ),
            Self::BadParameterCount { count, offset } => {
                write!(
                    f,
                    "{count} parameters at {offset:#x} do not fit the program"
                )
            }
            Self::BadParameterName { index, offset } => {
                write!(
                    f,
                    "parameter {index}'s name at {offset:#x} is not inside the program"
                )
            }
            Self::DoesNotClose { what, got, want } => {
                write!(
                    f,
                    "{what}: header says {got:#x}, the rest of it requires {want:#x}"
                )
            }
        }
    }
}

impl core::error::Error for Error {}

/// Reading a `GXP` program.
pub type Result<T> = core::result::Result<T, Error>;

/// What a parameter is bound as.
///
/// Named from the names the category carries and nothing else, which on this
/// container is as direct as the evidence gets: `0` holds `position`,
/// `normal` and `uv1`; `1` holds `viewProj` and `fogColour`; `2` holds
/// `lightmap`, `DiffuseMap` and `shadowMap`; `4` is the unnamed buffer a
/// skinned material's `skinPalette` indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// A vertex input. Only vertex programs carry these.
    Attribute,
    /// A constant the engine writes.
    Uniform,
    /// A texture unit.
    Sampler,
    /// A uniform buffer, unnamed on every program that carries one.
    UniformBuffer,
    /// A value this module has not seen and will not name.
    Unread(u8),
}

/// A parameter's `type` nibble, Sony's `SceGxmParameterType`.
///
/// Three of the ten documented values occur, and neither name is taken from
/// the enum's ordering alone.
///
/// **`9` occurs on exactly the 72 [`Category::UniformBuffer`] parameters and
/// nowhere else**, and `AGGREGATE` is index 9 - a uniform buffer is precisely
/// what an aggregate parameter is.
///
/// **A `1` parameter occupies exactly half the registers a `0` one of the
/// same width does**, measured on the `resource_index` gap between adjacent
/// uniforms: a `float3` or `float4` advances the register by 4 and a `1` of
/// either width advances it by 2, over 138,701 adjacent pairs. That is half
/// precision, and the enum's ordering could not have produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// `0`, 826,195 parameters.
    F32,
    /// `1`, 341,049 parameters, all of them uniforms.
    F16,
    /// `9`, the 72 uniform buffers and nothing else.
    Aggregate,
    /// A value this module has not seen and will not name.
    Unread(u8),
}

impl Type {
    fn from_nibble(n: u8) -> Self {
        match n {
            0 => Self::F32,
            1 => Self::F16,
            9 => Self::Aggregate,
            other => Self::Unread(other),
        }
    }
}

impl Category {
    fn from_nibble(n: u8) -> Self {
        match n {
            0 => Self::Attribute,
            1 => Self::Uniform,
            2 => Self::Sampler,
            4 => Self::UniformBuffer,
            other => Self::Unread(other),
        }
    }
}

/// One binding a program declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// The name, or empty when the offset field is zero - which means
    /// *unnamed*, not "a name at offset zero".
    pub name: String,
    /// What it is bound as.
    pub category: Category,
    /// How its value is stored.
    pub value_type: Type,
    /// Components, 0 to 4.
    pub components: u8,
    /// Which container (uniform buffer / texture bank) it lives in.
    pub container: u8,
    /// Elements, 1 for a scalar or vector, 4 for a `float4x4`.
    pub array_size: u32,
    /// Register or texture unit within the container.
    pub resource: u32,
}

/// A span of the program the header accounts for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// First byte.
    pub start: usize,
    /// One past the last.
    pub end: usize,
    /// What the header says is there.
    pub what: &'static str,
}

/// One decoded `GXP` container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// `+0x08`, and the length of the slice [`Program::parse`] consumed.
    pub size: usize,
    /// `+0x04` and `+0x05`.
    pub version: (u8, u8),
    /// `+0x06`.
    pub toolchain: u16,
    /// `+0x0c`.
    pub binary_guid: u32,
    /// `+0x10`.
    pub source_guid: u32,
    /// `+0x14`. Bit 0 is the only bit read; see [`Program::is_fragment`].
    pub flags: u32,
    /// `+0x3c` and `+0x40`: where the primary USSE stream is, undecoded.
    pub primary: Region,
    /// `+0x44`, `+0x48` and `+0x4c`: the secondary stream, likewise.
    pub secondary: Region,
    /// Instructions in [`Program::primary`], each 8 bytes.
    pub primary_instructions: u32,
    /// Instructions in [`Program::secondary`], each 8 bytes.
    pub secondary_instructions: u32,
    /// `+0x70`: literal records, 8 bytes each.
    pub literal_count: u32,
    /// `+0x90`: container records, 8 bytes each.
    pub container_count: u32,
    /// The bindings, in table order.
    pub parameters: Vec<Parameter>,
    /// Bytes inside the program that no header field claims:
    /// `primary.start - secondary.end`, the gap between the last code block
    /// and the primary program.
    ///
    /// It is **4 or 8 on every program in the corpus** and never zero, so it
    /// reads as alignment padding. It is reported rather than enforced - a
    /// third value would be a fact about the container, not a decode failure.
    pub unaccounted: usize,
}

fn le32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn le16(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

/// A self-relative offset: the field's own position plus its value.
fn rel(data: &[u8], at: usize) -> usize {
    at + le32(data, at) as usize
}

fn close(what: &'static str, got: usize, want: usize) -> Result<()> {
    if got == want {
        Ok(())
    } else {
        Err(Error::DoesNotClose { what, got, want })
    }
}

impl Program {
    /// Reads the program starting at byte zero of `data`.
    ///
    /// `data` may be longer than the program; the declared size decides how
    /// much is read, and [`Program::size`] reports it.
    ///
    /// # Errors
    ///
    /// See [`Error`]. Every closure check the module docs list is enforced
    /// here, so a program that parses at all is one whose header describes
    /// itself consistently from the first instruction to the last name byte.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_MIN {
            return Err(Error::TooShort { got: data.len() });
        }
        let tag = le32(data, 0);
        if tag != MAGIC {
            return Err(Error::BadMagic { tag });
        }
        let size = le32(data, 8);
        if (size as usize) < HEADER_MIN || size as usize > data.len() {
            return Err(Error::NotAContainer {
                size,
                available: data.len(),
            });
        }
        let b = &data[..size as usize];

        let param_count = le32(b, 0x24);
        let param_offset = rel(b, 0x28);
        let table_end =
            param_offset.saturating_add(PARAMETER_LEN.saturating_mul(param_count as usize));
        if param_count > MAX_PARAMETERS || param_offset < HEADER_MIN || table_end > b.len() {
            return Err(Error::BadParameterCount {
                count: param_count,
                offset: param_offset,
            });
        }

        let primary_instructions = le32(b, 0x3C);
        let primary_offset = rel(b, 0x40);
        let primary_end = primary_offset + 8 * primary_instructions as usize;
        let secondary_instructions = le32(b, 0x44);
        let secondary_offset = rel(b, 0x48);
        let secondary_end = rel(b, 0x4C);
        let uniform_words = le32(b, 0x58);
        let literal_count = le32(b, 0x70);
        let literal_offset = rel(b, 0x74);
        let t0_count = le32(b, 0x78);
        let t0_offset = rel(b, 0x7C);
        let table_count = le32(b, 0x80);
        let table_offset = rel(b, 0x84);
        let empty_offset = rel(b, 0x8C);
        let container_count = le32(b, 0x90);
        let container_offset = rel(b, 0x94);

        close(
            "the secondary program's count does not reach its declared end",
            secondary_offset + 8 * secondary_instructions as usize,
            secondary_end,
        )?;
        if primary_end > b.len() || secondary_end > b.len() {
            return Err(Error::DoesNotClose {
                what: "the code runs past the program",
                got: primary_end.max(secondary_end),
                want: b.len(),
            });
        }
        // The chain starts at `primary_end`, so requiring the literal table -
        // `+0x74` - to be its first link is `+0x74` stating the primary
        // program's end a second time and agreeing with `+0x40 + 8 * +0x3c`.
        // That is what ties the instruction count to the instruction width: a
        // 4- or 16-byte instruction fails here rather than silently.
        let image = literal_offset + 8 * literal_count as usize;
        let mut tables = [
            (literal_offset, image, "literals"),
            (
                image,
                image + 4 * uniform_words as usize,
                "the uniform image",
            ),
            (
                t0_offset,
                t0_offset + 8 * t0_count as usize,
                "the table at +0x7c",
            ),
            (
                table_offset,
                table_offset + 4 * table_count as usize,
                "the table at +0x84",
            ),
            (empty_offset, empty_offset, "the table at +0x8c"),
            (
                container_offset,
                container_offset + 8 * container_count as usize,
                "the container table",
            ),
        ];
        tables.sort_unstable();
        let mut at = primary_end;
        for (start, end, what) in tables {
            if start != at {
                return Err(Error::DoesNotClose {
                    what: Self::gap(what),
                    got: start,
                    want: at,
                });
            }
            at = end;
        }
        close(
            "the tables do not reach the parameter table",
            at,
            param_offset,
        )?;

        // `parameters` has already refused any name offset outside the
        // program, so indexing by `off` below cannot be out of bounds.
        let parameters = Self::parameters(b, param_offset, param_count as usize)?;
        let mut names_end = None;
        for index in 0..param_count as usize {
            let at = param_offset + PARAMETER_LEN * index;
            let delta = le32(b, at) as i32;
            if delta == 0 {
                continue;
            }
            let off = at.wrapping_add_signed(delta as isize);
            let end = b[off..].iter().position(|&c| c == 0).map(|n| off + n + 1);
            let Some(end) = end else {
                return Err(Error::BadParameterName { index, offset: off });
            };
            if off < table_end {
                return Err(Error::DoesNotClose {
                    what: "a parameter name overlaps the parameter table",
                    got: off,
                    want: table_end,
                });
            }
            names_end = Some(names_end.map_or(end, |e: usize| e.max(end)));
        }
        if let Some(end) = names_end {
            close(
                "the last parameter name does not end at the declared size",
                end,
                b.len(),
            )?;
        }

        let unaccounted = primary_offset.saturating_sub(secondary_end);
        Ok(Self {
            size: b.len(),
            version: (b[4], b[5]),
            toolchain: le16(b, 6),
            binary_guid: le32(b, 0x0C),
            source_guid: le32(b, 0x10),
            flags: le32(b, 0x14),
            primary: Region {
                start: primary_offset,
                end: primary_end,
                what: "primary code",
            },
            secondary: Region {
                start: secondary_offset,
                end: secondary_end,
                what: "secondary code",
            },
            primary_instructions,
            secondary_instructions,
            literal_count,
            container_count,
            parameters,
            unaccounted,
        })
    }

    /// Which table broke the chain, as a `&'static str` [`Error`] can hold.
    fn gap(what: &'static str) -> &'static str {
        match what {
            "literals" => "literals do not begin where the primary code ends",
            "the uniform image" => "the uniform image does not follow the literals",
            "the table at +0x7c" => "the table at +0x7c leaves a gap",
            "the table at +0x84" => "the table at +0x84 leaves a gap",
            "the table at +0x8c" => "the table at +0x8c leaves a gap",
            _ => "the container table leaves a gap",
        }
    }

    fn parameters(b: &[u8], offset: usize, count: usize) -> Result<Vec<Parameter>> {
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            let at = offset + PARAMETER_LEN * index;
            let delta = le32(b, at) as i32;
            let name = if delta == 0 {
                String::new()
            } else {
                let off = at.wrapping_add_signed(delta as isize);
                if off == 0 || off >= b.len() {
                    return Err(Error::BadParameterName { index, offset: off });
                }
                let Some(n) = b[off..].iter().position(|&c| c == 0) else {
                    return Err(Error::BadParameterName { index, offset: off });
                };
                String::from_utf8_lossy(&b[off..off + n]).into_owned()
            };
            let word = le16(b, at + 4);
            out.push(Parameter {
                name,
                category: Category::from_nibble((word & 0xF) as u8),
                value_type: Type::from_nibble(((word >> 4) & 0xF) as u8),
                components: ((word >> 8) & 0xF) as u8,
                container: (word >> 12) as u8,
                array_size: le32(b, at + 8),
                resource: le32(b, at + 12),
            });
        }
        Ok(out)
    }

    /// Whether this is a fragment program.
    ///
    /// Bit 0 of `+0x14`, and the only bit of that word this module reads.
    ///
    /// The evidence is one check and it is a strong one: a vertex program
    /// declares vertex inputs and a fragment program cannot, so across the
    /// corpus every program with the bit clear declares at least one
    /// [`Category::Attribute`] parameter and every program with it set
    /// declares none - **97,899 of 97,899, no exceptions**.
    ///
    /// A second, tempting check does *not* hold up and is not relied on: in
    /// the v1.04 patch the bit splits its 67 programs 32 / 35, exactly how
    /// that executable's 67 `_vp`/`_fp` shader-name strings split - but the
    /// base build has 111 programs against the same 67 names, so the
    /// container-to-name correspondence is not structural.
    #[must_use]
    pub fn is_fragment(&self) -> bool {
        self.flags & 1 != 0
    }

    /// The parameter of that name, if the program binds one.
    #[must_use]
    pub fn parameter(&self, name: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|p| p.name == name)
    }
}

/// Every `GXP` container in `data`, in offset order.
///
/// The framing is not assumed: a `.rcsmaterial` holds its programs
/// back to back with no directory, and `eboot.elf` holds three runs of them
/// with unrelated `.data` in between, so both are found by scanning for the
/// magic. A chance `GXP\0` in unrelated bytes surfaces as
/// [`Error::NotAContainer`] rather than as a decode failure.
pub fn programs(data: &[u8]) -> Vec<(usize, Result<Program>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 4 <= data.len() {
        let Some(hit) = data[at..].windows(4).position(|w| w == b"GXP\0") else {
            break;
        };
        let start = at + hit;
        out.push((start, Program::parse(&data[start..])));
        at = start + 4;
    }
    out
}

#[cfg(test)]
mod tests;
