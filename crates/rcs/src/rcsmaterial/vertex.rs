//! Reading a material's **vertex** microcode, for the one question its
//! fragment sibling cannot answer: where a surface's texture coordinate comes
//! from.
//!
//! [`super::fragment`] traces what a program does with the texels it samples.
//! This traces the varying it samples *at*, and there is exactly one thing
//! this project needed from it: **Wipeout HD's circuit shaders do not all
//! agree on which way up a texture coordinate is.**
//!
//! `talons_junction/track_wall`'s lit race-pass variant opens
//!
//! ```text
//! 4  ADD o[TC6].y, -v[3].yyyy, c[206].yyyy
//! 5  MOV o[TC6].x, v[3].xxxx
//! ```
//!
//! with `v[3]` the block's own declared `Uv1` and `c[206].y` the 1.0 that
//! instruction 9 uses as the `-1` of a `t * 2 - 1` tangent unpack. So the
//! coordinate the fragment program samples with is `1 - Uv1.y`, not `Uv1.y`.
//! `track_surface` does no such thing in any of its 64 vertex blocks, and its
//! coordinates were always right.
//!
//! Reading the flip rather than applying it everywhere matters because it is
//! **not** a global convention: 18 of `track_wall`'s 94 blocks flip and none
//! of `track_surface`'s do, so a renderer that flips unconditionally trades
//! one set of wrong surfaces for another.
//!
//! # The encoding
//!
//! NV40's, the same source `scripts/ps3-microcode.py` documents: four
//! big-endian dwords per instruction, the vector opcode at `d1 >> 22`, the
//! scalar at `d1 >> 27`, one input-register index per instruction at
//! `d1 >> 8`, and three 17-bit source fields split across `d1`, `d2` and `d3`.
//! A source's low two bits are its register file - 1 temporary, 2 input, 3
//! constant - and bit 16 is its negate.

use super::Block;

/// A vector opcode's source slots, as nouveau's own emitter fills them.
///
/// `ADD` is `src0 + src2` and `MAD` all three; the one-operand ops read `src0`
/// alone and everything else reads the first two. Reading a slot an opcode
/// does not use is how a stale field turns into a false positive, which is why
/// this table exists rather than a scan of all three.
fn source_slots(vec_op: u8) -> &'static [usize] {
    match vec_op {
        // MOV, ARL, FRC, FLR, SSG, ARR
        0x01 | 0x0d | 0x0e | 0x0f | 0x16 | 0x17 => &[0],
        // ADD
        0x03 => &[0, 2],
        // MAD
        0x04 => &[0, 1, 2],
        _ => &[0, 1],
    }
}

/// The highest vector opcode NV40 defines (`TXL`).
const MAX_VEC_OP: u8 = 0x19;
/// The highest scalar opcode NV40 defines (`POPA`).
const MAX_SCA_OP: u8 = 0x14;
/// Source register file: a vertex input register, `v[n]`.
const SOURCE_INPUT: u32 = 2;
/// The negate bit of a 17-bit source field.
const SOURCE_NEGATE: u32 = 1 << 16;
/// `o[TC0]`, the first destination this reading cares about; `o[POS]` is 0 and
/// the five interpolators between are colours, fog and point size.
const FIRST_TEXCOORD_DEST: u32 = 7;

/// One decoded instruction, reduced to the fields this reading uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction {
    /// The vector opcode, `0` for an instruction that is scalar-only.
    pub vec_op: u8,
    /// Which vertex input register every source in this instruction that names
    /// one refers to - there is one such index per instruction, not per source.
    pub input: u8,
    /// The three 17-bit source fields, in slot order.
    pub sources: [u32; 3],
    /// The vector half's destination register, meaningful only when
    /// [`Self::dest_is_output`].
    pub dest: u32,
    /// Whether the vector half writes an output register rather than a
    /// temporary.
    pub dest_is_output: bool,
}

impl Instruction {
    fn decode(words: [u32; 4]) -> Self {
        let [d0, d1, d2, d3] = words;
        Self {
            vec_op: ((d1 >> 22) & 0x1f) as u8,
            input: ((d1 >> 8) & 0x0f) as u8,
            sources: [
                ((d1 & 0xff) << 9) | ((d2 >> 23) & 0x1ff),
                (d2 >> 6) & 0x1_ffff,
                ((d2 & 0x3f) << 11) | ((d3 >> 21) & 0x7ff),
            ],
            dest: (d3 >> 2) & 0x1f,
            dest_is_output: d0 & (1 << 30) != 0,
        }
    }

    /// Whether this instruction negates vertex input `slot` into an output
    /// texture coordinate.
    fn negates_into_a_texcoord(&self, slot: u32) -> bool {
        if self.vec_op == 0 || !self.dest_is_output || self.dest < FIRST_TEXCOORD_DEST {
            return false;
        }
        if u32::from(self.input) != slot {
            return false;
        }
        source_slots(self.vec_op).iter().any(|&n| {
            let source = self.sources[n];
            source & 3 == SOURCE_INPUT && source & SOURCE_NEGATE != 0
        })
    }
}

/// One vertex `SHO` block: what it is fed, and what it does with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// Every vertex attribute the block declares, as `(name hash, input
    /// register)`.
    pub attributes: Vec<(u32, u32)>,
    /// The instruction stream.
    pub instructions: Vec<Instruction>,
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*data.get(at)?, *data.get(at + 1)?]))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *data.get(at)?,
        *data.get(at + 1)?,
        *data.get(at + 2)?,
        *data.get(at + 3)?,
    ]))
}

impl Program {
    /// Reads the vertex block at `at`, or `None` if nothing there decodes.
    ///
    /// The code does not begin at a fixed offset inside the block: a defaults
    /// section of varying size sits between the instruction count and the
    /// stream, so the start is found by probing 16-byte alignments until the
    /// first several instructions all carry opcodes NV40 defines. That is the
    /// same search `scripts/ps3-microcode.py` performs and it is why this
    /// answers `None` rather than guessing.
    #[must_use]
    pub fn parse(data: &[u8], at: usize) -> Option<Self> {
        if data.get(at..at + 4)? != b"SHO\x08" {
            return None;
        }
        let field = |n: usize| u16_at(data, at + 8 + n * 2).map(usize::from);
        let attribute_count = field(1)?;
        let attributes_at = field(4)?;
        let program_at = field(7)?;

        let mut attributes = Vec::with_capacity(attribute_count);
        for i in 0..attribute_count {
            let entry = at + attributes_at + i * 8;
            attributes.push((u32_at(data, entry)?, u32_at(data, entry + 4)?));
        }

        let base = at + program_at;
        let count = usize::from(u16_at(data, base)?);
        let words = |offset: usize| -> Option<[u32; 4]> {
            Some([
                u32_at(data, offset)?,
                u32_at(data, offset + 4)?,
                u32_at(data, offset + 8)?,
                u32_at(data, offset + 12)?,
            ])
        };
        let plausible = |probe: usize| {
            (0..count.min(8)).all(|k| {
                words(base + probe + k * 16).is_some_and(|w| {
                    let vec_op = ((w[1] >> 22) & 0x1f) as u8;
                    let sca_op = ((w[1] >> 27) & 0x1f) as u8;
                    vec_op <= MAX_VEC_OP && sca_op <= MAX_SCA_OP && (vec_op != 0 || sca_op != 0)
                })
            })
        };
        let probe = (1..0x20)
            .map(|n| n * 0x10)
            .find(|&probe| plausible(probe))?;
        let instructions = (0..count)
            .map(|k| words(base + probe + k * 16).map(Instruction::decode))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            attributes,
            instructions,
        })
    }

    /// Reads the block a [`super::Variant`] names.
    #[must_use]
    pub fn of(data: &[u8], block: Block) -> Option<Self> {
        Self::parse(data, block.offset)
    }

    /// Which vertex input register the attribute named `hash` arrives in.
    #[must_use]
    pub fn attribute_slot(&self, hash: u32) -> Option<u32> {
        self.attributes
            .iter()
            .find(|&&(name, _)| name == hash)
            .map(|&(_, slot)| slot)
    }

    /// The texture-coordinate interpolators (`0` for `o[TC0]`) this program
    /// writes attribute `hash` into, unchanged or not: every instruction that
    /// reads the attribute's input register and writes an output at or past
    /// `o[TC0]`. Empty for an attribute the block does not declare.
    #[must_use]
    pub fn texcoords_fed_by(&self, hash: u32) -> Vec<u32> {
        let Some(slot) = self.attribute_slot(hash) else {
            return Vec::new();
        };
        self.instructions
            .iter()
            .filter(|i| {
                u32::from(i.input) == slot
                    && i.vec_op != 0
                    && i.dest_is_output
                    && i.dest >= FIRST_TEXCOORD_DEST
                    && i.sources.iter().any(|&s| s & 3 == SOURCE_INPUT)
            })
            .map(|i| i.dest - FIRST_TEXCOORD_DEST)
            .collect()
    }

    /// Whether this program writes the **negated** attribute `hash` into an
    /// output texture coordinate - the `v = 1 - v` flip the module documents.
    ///
    /// `false` for an attribute the block does not declare, which is the
    /// direction that leaves a coordinate alone.
    #[must_use]
    pub fn flips(&self, hash: u32) -> bool {
        let Some(slot) = self.attribute_slot(hash) else {
            return false;
        };
        self.instructions
            .iter()
            .any(|i| i.negates_into_a_texcoord(slot))
    }
}

#[cfg(test)]
mod tests;
