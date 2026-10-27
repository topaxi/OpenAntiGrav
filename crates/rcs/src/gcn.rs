//! The PS4's compiled shader programs (Sea Islands GCN microcode), as Omega ships them.
//!
//! Omega's `.rcsmaterial` files and its `eboot.bin` carry their programs in one wrapper. Every
//! program found so far opens with a `Shdr` tag; the code follows it, then Sony's `OrbShdr`
//! footer, then a reflection table naming the constant buffer's members, the textures, the
//! samplers and the interpolants. All little-endian.
//!
//! ```text
//! Shdr header (offsets from the tag)
//! +0x00  [u8; 4] "Shdr"
//! +0x10  u32     low 24 bits: bytes from the code's start to the footer's end
//! +0x18  u32     the code's start, relative to +0x10
//! +0x38  to the code, on a vertex program: input-usage records of varying length, then one
//!        4-byte record per vertex input (u8 slot, u8 first VGPR, u8 size, u8 0), then exports;
//!        the inputs are found by their slots (see [`Program::vertex_inputs`]), and everything
//!        else in the header is unread
//! code
//! +0x00  s_mov_b32 vcc_hi, N      ff 03 eb be, N: the footer is (N + 1) * 8 bytes on
//! ...    to s_endpgm, exactly the footer's length
//! then bytes up to the footer, unread
//! OrbShdr footer, 0x1c bytes
//! +0x00  [u8; 7] "OrbShdr"
//! +0x07  u8      version (7 on every program read)
//! +0x08  u32     bits 2-5: stage, 1 vertex and 0 pixel; bits 8-31: code length in bytes
//! +0x0c  4 x u32 unread
//! reflection
//! +0x00  u32     resources (constant buffers and textures), 6 words each
//! +0x08  u32     constants (buffer members), 9 words each
//! +0x0c  u32     samplers, 4 words each
//! +0x10  u8      inputs, u8 outputs, 4 words each
//! +0x18  u32     string table size, in bytes
//! +0x04, +0x14   zero on every program read
//! +0x1c  the records in that order, then the string table
//! resource:  u32 slot, u32 size, u32 kind, u32 members, u32 first member, i32 name
//! constant:  u32 kind, u32 byte offset, u32 size, 4 x u32 unread, i32 name, i32 type name
//! sampler:   u32 0, u32 0, u32 slot, i32 name
//! input/output: u32 kind, u32 0, i32 name, i32 semantic
//! ```
//!
//! A program is held to the code marker pointing exactly at `OrbShdr`, the header's span
//! agreeing with it, and the code decoding to `s_endpgm` at exactly the footer's length
//! ([`Program::parse`]). The reflection is held separately to every name, an `i32` relative to
//! its own field, landing inside the string table; a reflection that does not close is a named
//! [`Error`] in [`Program::reflection`], not a refused program. The layout and
//! its census are on `docs/formats/gcn.md`; the instruction set is [`isa`].

pub mod isa;
mod opcodes;
mod sample;

pub use sample::{SignedSamplers, SignedTextures, signed_samplers};

use core::fmt;
use core::ops::Range;

pub use isa::{DecodeError, Detail, Encoding, Instruction, Operand};

/// The wrapper's tag.
pub const SHDR: &[u8; 4] = b"Shdr";
/// The footer's tag.
pub const ORBSHDR: &[u8; 7] = b"OrbShdr";
/// `s_mov_b32 vcc_hi, <literal>`, the first instruction of every program.
pub const CODE_MARKER: u32 = 0xbeeb_03ff;
/// The footer's size.
pub const FOOTER_LEN: usize = 0x1c;

/// Which pipeline stage a program runs at, from the footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stage {
    Pixel,
    Vertex,
    /// A stage number no measured program uses; kept, never guessed at.
    Other(u8),
}

/// A constant-buffer or texture record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resource {
    pub slot: u32,
    /// Bytes; zero on a texture.
    pub size: u32,
    /// `0x6a0416` on every constant buffer read, `0x70502` on a 2D texture.
    pub kind: u32,
    /// Indices into [`Reflection::constants`].
    pub members: Range<usize>,
    pub name: String,
}

/// A constant-buffer member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constant {
    /// Low byte: components less one (`0x03` a float4); `0x23` a float4x4.
    pub kind: u32,
    /// Byte offset in the buffer; `s_buffer_load` immediates count dwords, so the dword is
    /// `offset / 4`.
    pub offset: u32,
    pub size: u32,
    /// Words 3-6: `0, -1, 0, -1` on every member read.
    pub unread: [u32; 4],
    pub name: String,
    pub type_name: String,
}

/// A sampler record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sampler {
    pub slot: u32,
    pub name: String,
}

/// An input or output record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// Low byte: components less one; byte 2: semantic index; byte 3: the slot a vertex input
    /// is fetched into ([`VertexInput::semantic`]).
    pub kind: u32,
    pub name: String,
    pub semantic: String,
}

impl Attribute {
    /// The slot byte, which a vertex program's [`VertexInput::semantic`] matches.
    #[must_use]
    pub fn slot(&self) -> u8 {
        (self.kind >> 24) as u8
    }
}

/// The reflection table after the footer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reflection {
    pub resources: Vec<Resource>,
    pub constants: Vec<Constant>,
    pub samplers: Vec<Sampler>,
    pub inputs: Vec<Attribute>,
    pub outputs: Vec<Attribute>,
}

impl Reflection {
    /// The constant a dword of the buffer named `buffer` falls in, and the component.
    #[must_use]
    pub fn constant_at(&self, buffer: usize, dword: u32) -> Option<(&Constant, u32)> {
        let resource = self.resources.get(buffer)?;
        self.constants[resource.members.clone()]
            .iter()
            .find(|c| (c.offset..c.offset + c.size).contains(&(dword * 4)))
            .map(|c| (c, dword - c.offset / 4))
    }

    /// The constant buffer's index in [`Self::resources`], when the program has exactly one.
    #[must_use]
    pub fn constant_buffer(&self) -> Option<usize> {
        let mut buffers = self
            .resources
            .iter()
            .enumerate()
            .filter(|(_, r)| r.size > 0);
        match (buffers.next(), buffers.next()) {
            (Some((k, _)), None) => Some(k),
            _ => None,
        }
    }
}

/// One vertex input the fetch shader fills before the program runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VertexInput {
    /// Matches an input [`Attribute::slot`].
    pub semantic: u8,
    pub vgpr: u8,
    /// Components fetched.
    pub size: u8,
    pub unread: u8,
}

/// One program, decoded.
#[derive(Clone, Debug, PartialEq)]
pub struct Program<'a> {
    /// Byte offset of the `Shdr` tag in the file.
    pub at: usize,
    /// Byte offset of the code in the file.
    pub code_at: usize,
    pub code: &'a [u8],
    pub stage: Stage,
    /// The footer's 32-bit info word (stage and length).
    pub info: u32,
    pub instructions: Vec<Instruction>,
    pub reflection: Result<Reflection>,
    /// The vertex inputs' fetch table; empty on a program that is not a vertex program.
    pub vertex_inputs: Result<Vec<VertexInput>>,
    /// The header's input-usage records (`+0x4c`, the count at `+0x38` less one): where each
    /// resource, sampler and table pointer sits. See [`Usage`].
    pub usage: Vec<Usage>,
}

/// One input-usage record of the `Shdr` header, four bytes.
///
/// `kind` 0 is a texture (T#), 1 a sampler (S#), 2 a constant buffer, `0x13` a pointer to the
/// resource table (a texture's dword offset in it is eight times its `slot`), `0x1b` a pointer
/// to an extended-user-data table. `slot` is the API slot, the one the reflection's resource and
/// sampler records carry. `register` is the first SGPR of an inline one; of an
/// extended-table one it is `16` plus the dword offset into that table ([`Usage::TABLE_BASE`]).
/// Read off `Diffuse_Normal_Specular`'s and `Mageffect08_LH_1`'s programs against their `s_load`
/// offsets and the reflection (`docs/formats/gcn.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Usage {
    pub kind: u8,
    pub slot: u8,
    pub register: u8,
    /// Unread; `3` on a texture.
    pub flags: u8,
}

impl Usage {
    /// The record kind that opens the header's fixed tail (`0f 00 10 00`, `11 00 12 00`, ...).
    pub const TAIL: u8 = 0x0f;
    /// Kind: a texture descriptor (T#).
    pub const TEXTURE: u8 = 0;
    /// Kind: a pointer to the resource table, T# at eight dwords a slot.
    pub const RESOURCE_TABLE: u8 = 0x13;
    /// Kind: a pointer to an extended-user-data table.
    pub const EXTENDED_TABLE: u8 = 0x1b;
    /// A table resource's `register` less its dword offset in the table.
    pub const TABLE_BASE: u32 = 16;
}

/// Why a `Shdr` tag is not a program this reader accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The header runs past the file.
    TruncatedHeader,
    /// The code offset does not land on `s_mov_b32 vcc_hi, <literal>`.
    NoCodeMarker { word: u32 },
    /// The marker's literal does not point at `OrbShdr`.
    NoFooter,
    /// The header's code-to-footer-end span disagrees with the footer's position.
    SpanMismatch { header: usize, measured: usize },
    /// The footer's code length runs past the footer.
    LengthPastFooter { length: usize },
    /// An instruction could not be decoded.
    Decode(DecodeError),
    /// The code does not end on `s_endpgm` exactly at the footer's length.
    NoEndAtLength { last: String },
    /// A vertex header holds no run of records matching the reflection's input slots, or
    /// more than one.
    VertexInputs { matches: usize },
    /// A stage whose reflection layout is unread.
    StageUnread(u8),
    /// A reflection record or name falls outside the table.
    Reflection(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHeader => f.write_str("Shdr header truncated"),
            Self::NoCodeMarker { word } => write!(f, "no code marker (first word {word:#010x})"),
            Self::NoFooter => f.write_str("code marker does not point at OrbShdr"),
            Self::SpanMismatch { header, measured } => {
                write!(f, "header span {header:#x}, measured {measured:#x}")
            }
            Self::LengthPastFooter { length } => write!(f, "code length {length:#x} past footer"),
            Self::Decode(e) => write!(f, "{e}"),
            Self::NoEndAtLength { last } => write!(f, "code ends on `{last}`, not s_endpgm"),
            Self::VertexInputs { matches } => {
                write!(
                    f,
                    "{matches} runs in the vertex header match the input slots"
                )
            }
            Self::StageUnread(n) => write!(f, "the reflection of a stage-{n} program is unread"),
            Self::Reflection(what) => write!(f, "reflection: {what}"),
        }
    }
}

impl core::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;

fn u32_at(file: &[u8], at: usize) -> Option<u32> {
    file.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
}

/// Every `Shdr` tag in `file` that is not the tail of an `OrbShdr`, in order.
#[must_use]
pub fn tags(file: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = file[at..].windows(4).position(|w| w == SHDR) {
        let tag = at + i;
        if tag < 3 || &file[tag - 3..tag] != b"Orb" {
            out.push(tag);
        }
        at = tag + 4;
    }
    out
}

/// Every `Shdr` tag in `file`, parsed.
#[must_use]
pub fn scan(file: &[u8]) -> Vec<(usize, Result<Program<'_>>)> {
    tags(file)
        .into_iter()
        .map(|at| (at, Program::parse(file, at)))
        .collect()
}

/// The header's usage records: from `+0x4c`, four bytes each, up to the first record of kind
/// [`Usage::TAIL`] (the fixed pixel-input tail every program ends its header with) or the code.
fn usage(file: &[u8], at: usize, code_at: usize) -> Vec<Usage> {
    let records = file.get(at + 0x4c..code_at).unwrap_or_default();
    records
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[kind, slot, register, flags]| Usage {
            kind,
            slot,
            register,
            flags,
        })
        .take_while(|u| u.kind != Usage::TAIL)
        .collect()
}

impl<'a> Program<'a> {
    /// Parses the program whose `Shdr` tag is at byte `at` of `file`.
    ///
    /// # Errors
    ///
    /// See [`Error`]: every closure the module docs list is enforced.
    pub fn parse(file: &'a [u8], at: usize) -> Result<Self> {
        let word = |o: usize| u32_at(file, at + o).ok_or(Error::TruncatedHeader);
        let code_at = at + 0x10 + word(0x18)? as usize;
        let span = (word(0x10)? & 0x00ff_ffff) as usize;
        let marker = u32_at(file, code_at).ok_or(Error::TruncatedHeader)?;
        if marker != CODE_MARKER {
            return Err(Error::NoCodeMarker { word: marker });
        }
        let lit = u32_at(file, code_at + 4).ok_or(Error::TruncatedHeader)? as usize;
        let footer = code_at + (lit + 1) * 8;
        if file.get(footer..footer + 7) != Some(&ORBSHDR[..]) || file.len() < footer + FOOTER_LEN {
            return Err(Error::NoFooter);
        }
        let measured = footer + FOOTER_LEN - code_at;
        if span != measured {
            return Err(Error::SpanMismatch {
                header: span,
                measured,
            });
        }
        let info = u32_at(file, footer + 8).expect("inside the footer");
        let length = (info >> 8) as usize;
        if code_at + length > footer {
            return Err(Error::LengthPastFooter { length });
        }
        let stage = match (info >> 2) & 0xf {
            0 => Stage::Pixel,
            1 => Stage::Vertex,
            n => Stage::Other(n as u8),
        };
        let code = &file[code_at..code_at + length];
        let instructions = isa::decode_all(code).map_err(Error::Decode)?;
        match instructions.last() {
            Some(last) if last.is_end() => {}
            last => {
                return Err(Error::NoEndAtLength {
                    last: last.map_or_else(String::new, ToString::to_string),
                });
            }
        }
        let reflection = match stage {
            Stage::Other(n) => Err(Error::StageUnread(n)),
            _ => reflection(file, footer + FOOTER_LEN),
        };
        let vertex_inputs = match (&reflection, stage) {
            (Ok(r), Stage::Vertex) => vertex_inputs(&file[at + 0x38..code_at], &r.inputs),
            (Err(e), Stage::Vertex) => Err(e.clone()),
            _ => Ok(Vec::new()),
        };
        Ok(Self {
            at,
            code_at,
            code,
            stage,
            info,
            instructions,
            reflection,
            vertex_inputs,
            usage: usage(file, at, code_at),
        })
    }

    /// The vertex input fetched into `vgpr`, with its reflection record.
    #[must_use]
    pub fn input_in(&self, vgpr: u8) -> Option<(&VertexInput, &Attribute)> {
        let input = self
            .vertex_inputs
            .as_ref()
            .ok()?
            .iter()
            .find(|v| (v.vgpr..v.vgpr + v.size).contains(&vgpr))?;
        let attribute = self
            .reflection
            .as_ref()
            .ok()?
            .inputs
            .iter()
            .find(|a| a.slot() == input.semantic)?;
        Some((input, attribute))
    }

    /// The vertex input named `name` (`position`, `tangent`), with its record.
    #[must_use]
    pub fn input_named(&self, name: &str) -> Option<(&VertexInput, &Attribute)> {
        let attribute = self
            .reflection
            .as_ref()
            .ok()?
            .inputs
            .iter()
            .find(|a| a.name == name)?;
        let input = self
            .vertex_inputs
            .as_ref()
            .ok()?
            .iter()
            .find(|v| v.semantic == attribute.slot())?;
        Some((input, attribute))
    }
}

/// The vertex-input records in a vertex program's header (`header`, from `+0x38` to the code):
/// the one run of 4-byte records whose slot bytes are the reflection's input slots in order,
/// each fetching 1 to 4 components with a zero last byte. No other count in the header was
/// found to say where the run starts, so it is found by this match, and a header with no such
/// run or with two is refused by name.
fn vertex_inputs(header: &[u8], inputs: &[Attribute]) -> Result<Vec<VertexInput>> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let words: &[[u8; 4]] = header.as_chunks::<4>().0;
    let fits = |start: usize| {
        inputs.iter().enumerate().all(|(k, a)| {
            words
                .get(start + k)
                .is_some_and(|w| w[0] == a.slot() && (1..=4).contains(&w[2]) && w[3] == 0)
        })
    };
    let starts: Vec<usize> = (0..words.len()).filter(|&k| fits(k)).collect();
    let [start] = starts[..] else {
        return Err(Error::VertexInputs {
            matches: starts.len(),
        });
    };
    Ok(words[start..start + inputs.len()]
        .iter()
        .map(|w| VertexInput {
            semantic: w[0],
            vgpr: w[1],
            size: w[2],
            unread: w[3],
        })
        .collect())
}

fn reflection(file: &[u8], at: usize) -> Result<Reflection> {
    let word = |o: usize| u32_at(file, o).ok_or(Error::Reflection("header past the file"));
    let resources = word(at)? as usize;
    let constants = word(at + 8)? as usize;
    let samplers = word(at + 12)? as usize;
    let io = word(at + 16)?;
    let (inputs, outputs) = ((io & 0xff) as usize, ((io >> 8) & 0xff) as usize);
    if word(at + 4)? != 0 || word(at + 20)? != 0 || io >> 16 != 0 {
        return Err(Error::Reflection("an unread header word is not zero"));
    }
    let strings_len = word(at + 24)? as usize;
    let records = at + 28;
    let strings = records + 24 * resources + 36 * constants + 16 * (samplers + inputs + outputs);
    if file.len() < strings + strings_len {
        return Err(Error::Reflection("records past the file"));
    }
    let name = |field: usize| -> Result<String> {
        let rel = u32_at(file, field).expect("checked") as i32;
        let target = field as i64 + i64::from(rel);
        let range = strings as i64..(strings + strings_len) as i64;
        if !range.contains(&target) {
            return Err(Error::Reflection("a name outside the string table"));
        }
        let start = target as usize;
        let end = file[start..strings + strings_len]
            .iter()
            .position(|&b| b == 0)
            .ok_or(Error::Reflection("an unterminated name"))?;
        Ok(String::from_utf8_lossy(&file[start..start + end]).into_owned())
    };
    let w = |o: usize| u32_at(file, o).expect("checked");
    let mut out = Reflection::default();
    for k in 0..resources {
        let r = records + 24 * k;
        let (members, first) = (w(r + 12) as usize, w(r + 16) as usize);
        if first + members > constants {
            return Err(Error::Reflection("members past the constant table"));
        }
        out.resources.push(Resource {
            slot: w(r),
            size: w(r + 4),
            kind: w(r + 8),
            members: first..first + members,
            name: name(r + 20)?,
        });
    }
    let base = records + 24 * resources;
    for k in 0..constants {
        let r = base + 36 * k;
        out.constants.push(Constant {
            kind: w(r),
            offset: w(r + 4),
            size: w(r + 8),
            unread: [w(r + 12), w(r + 16), w(r + 20), w(r + 24)],
            name: name(r + 28)?,
            type_name: name(r + 32)?,
        });
    }
    let base = base + 36 * constants;
    for k in 0..samplers {
        let r = base + 16 * k;
        out.samplers.push(Sampler {
            slot: w(r + 8),
            name: name(r + 12)?,
        });
    }
    let base = base + 16 * samplers;
    for k in 0..inputs + outputs {
        let r = base + 16 * k;
        let attribute = Attribute {
            kind: w(r),
            name: name(r + 8)?,
            semantic: name(r + 12)?,
        };
        if k < inputs {
            out.inputs.push(attribute);
        } else {
            out.outputs.push(attribute);
        }
    }
    Ok(out)
}

impl fmt::Display for Program<'_> {
    /// One instruction a line, each `s_buffer_load` annotated with the constant it reads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reflection = self.reflection.as_ref().ok();
        let cb = reflection.and_then(Reflection::constant_buffer);
        for i in &self.instructions {
            write!(f, "{:5x}  {i}", i.at)?;
            if let (Some(cb), Some(name)) = (cb, i.name)
                && name.starts_with("s_buffer_load")
                && let Some(Operand::Imm(dword)) = i.src[1]
                && let Some(Operand::Sgpr { count, .. }) = i.dst[0]
            {
                let names: Vec<String> = (0..u32::from(count))
                    .filter_map(|k| reflection?.constant_at(cb, dword + k))
                    .map(|(c, comp)| format!("{}[{comp}]", c.name))
                    .collect();
                if !names.is_empty() {
                    write!(f, "    ; {}", names.join(" "))?;
                }
            }
            if let (Some(lit), false) = (i.literal(), i.encoding < Encoding::Vop2) {
                write!(f, "    ; {lit:#x} = {}", f32::from_bits(lit))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
