//! The NV40 fragment microcode a `.rcsmaterial`'s `SHO` blocks carry.
//!
//! A port of the decoder in [`scripts/ps3-microcode.py`], which is the
//! reference and stays the reference: that script established the container
//! facts empirically - exhaustively, by scoring every candidate ordering - and
//! this reimplements the reading rather than re-deriving it. The instruction
//! encodings are NV40's, taken from Mesa's nouveau driver headers
//! (`nvfx_shader.h`, MIT-licensed; the RSX is an NV4x).
//!
//! Three container facts this depends on, none of them guessable:
//!
//! - **A fragment dword is stored with its 16-bit halves swapped**, so a word
//!   is `(u16 at +2) << 16 | (u16 at +0)`. See [`word`].
//! - **An instruction whose source selects an inline constant is followed by
//!   that constant's 16 bytes**, so the stream advances 32 rather than 16
//!   there.
//! - The code's byte length is at the program sub-header's `+0x00` and its
//!   offset at `+0x10`.
//!
//! [`scripts/ps3-microcode.py`]: ../../../../scripts/ps3-microcode.py

use super::Declared;

/// One fragment-program dword, whose 16-bit halves are stored swapped.
#[must_use]
fn word(data: &[u8], at: usize) -> Option<u32> {
    let lo = u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?);
    let hi = u16::from_be_bytes(data.get(at + 2..at + 4)?.try_into().ok()?);
    Some(u32::from(hi) << 16 | u32::from(lo))
}

/// Where one of an instruction's operands comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A temporary register - `R0`, or `H0` when `half`.
    Register {
        /// Register ordinal.
        index: u8,
        /// Whether it is a half-precision register.
        half: bool,
    },
    /// An interpolated input, whose index is the *instruction's* - see
    /// [`Instruction::input`], because all of an instruction's inputs read the
    /// same one.
    Input,
    /// The inline constant that follows the instruction.
    Constant,
    /// A source type this reading does not know.
    Unknown,
}

impl Source {
    fn of(bits: u32) -> Self {
        match bits & 3 {
            0 => Self::Register {
                index: ((bits >> 2) & 0x3f) as u8,
                half: bits & (1 << 8) != 0,
            },
            1 => Self::Input,
            2 => Self::Constant,
            _ => Self::Unknown,
        }
    }
}

/// One decoded instruction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instruction {
    /// The raw opcode, `(d0 >> 24) & 0x3f`.
    pub opcode: u8,
    /// Destination register ordinal.
    pub dst: u8,
    /// Whether the destination is half-precision.
    pub dst_half: bool,
    /// Destination write mask, `xyzw` in bits 0-3.
    pub mask: u8,
    /// Whether the result is saturated to `[0, 1]`.
    pub saturate: bool,
    /// Which interpolator every [`Source::Input`] of this instruction reads -
    /// `0` position, `1` COL0, `2` COL1, `3` FOGC, `4 + n` TC*n*, `0xe`
    /// FACING.
    pub input: u8,
    /// The texture unit, for the sampling opcodes.
    pub unit: u8,
    /// The three source slots, however many [`Self::arity`] uses.
    pub sources: [Source; 3],
    /// The inline constant that followed, when a source selected one.
    pub constant: Option<[f32; 4]>,
    /// Whether this instruction ends the program.
    pub end: bool,
}

impl Instruction {
    /// The mnemonic, or `None` for an opcode this reading does not name.
    #[must_use]
    pub fn name(&self) -> Option<&'static str> {
        Some(match self.opcode {
            0x00 => "NOP",
            0x01 => "MOV",
            0x02 => "MUL",
            0x03 => "ADD",
            0x04 => "MAD",
            0x05 => "DP3",
            0x06 => "DP4",
            0x07 => "DST",
            0x08 => "MIN",
            0x09 => "MAX",
            0x0a => "SLT",
            0x0b => "SGE",
            0x0c => "SLE",
            0x0d => "SGT",
            0x0e => "SNE",
            0x0f => "SEQ",
            0x10 => "FRC",
            0x11 => "FLR",
            0x12 => "KIL",
            0x13 => "PK4B",
            0x14 => "UP4B",
            0x15 => "DDX",
            0x16 => "DDY",
            0x17 => "TEX",
            0x18 => "TXP",
            0x19 => "TXD",
            0x1a => "RCP",
            0x1b => "RSQ",
            0x1c => "EX2",
            0x1d => "LG2",
            0x1e => "LIT",
            0x1f => "LRP",
            0x20 => "STR",
            0x21 => "SFL",
            0x22 => "COS",
            0x23 => "SIN",
            0x24 => "PK2H",
            0x25 => "UP2H",
            0x26 => "POW",
            0x27 => "PK4UB",
            0x28 => "UP4UB",
            0x29 => "PK2US",
            0x2a => "UP2US",
            0x2e => "DP2A",
            0x2f => "TXL",
            0x31 => "TXB",
            0x36 => "RFL",
            0x3a => "DIV",
            // 0x3b, 0x3c and 0x3d occur in shipped programs and are not in
            // nouveau's table; they stay unnamed rather than guessed, and the
            // reference decoder carries them as `op3B`/`op3D` for the same
            // reason.
            _ => return None,
        })
    }

    /// How many of [`Self::sources`] this opcode reads.
    ///
    /// Two unless the mnemonic says otherwise, which is the reference
    /// decoder's own rule.
    #[must_use]
    pub fn arity(&self) -> usize {
        match self.name() {
            Some("KIL") => 0,
            Some(
                "MOV" | "FRC" | "FLR" | "RCP" | "RSQ" | "EX2" | "LG2" | "COS" | "SIN" | "DDX"
                | "DDY" | "TEX" | "TXP" | "TXB" | "TXL" | "UP4B" | "UP2H" | "UP4UB" | "UP2US"
                | "PK4B" | "PK2H" | "PK4UB" | "PK2US",
            ) => 1,
            Some("MAD" | "DP2A" | "TXD" | "LRP") => 3,
            _ => 2,
        }
    }

    /// Whether this instruction samples a texture.
    #[must_use]
    pub fn is_texture(&self) -> bool {
        matches!(self.name(), Some("TEX" | "TXP" | "TXB" | "TXL" | "TXD"))
    }

    /// The sources this opcode actually reads.
    pub fn operands(&self) -> impl Iterator<Item = Source> {
        self.sources.into_iter().take(self.arity())
    }
}

/// A decoded fragment program.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Program {
    /// Instructions in stream order, ending at the one carrying the end bit.
    pub instructions: Vec<Instruction>,
    /// What the block declares it is fed.
    pub declared: Declared,
}

impl Program {
    /// Decodes the fragment program of the `SHO` block at `at`.
    ///
    /// `None` if `at` does not open a block, if its declaration tables do not
    /// frame, or if the code region runs past the data.
    #[must_use]
    pub fn parse(data: &[u8], at: usize) -> Option<Self> {
        let declared = Declared::parse(data, at)?;
        let program_at = usize::from(u16::from_be_bytes([data[at + 0x16], data[at + 0x17]]));
        let head = at + program_at;
        let code_len = u32::from_be_bytes(data.get(head..head + 4)?.try_into().ok()?) as usize;
        let code_off =
            u32::from_be_bytes(data.get(head + 0x10..head + 0x14)?.try_into().ok()?) as usize;
        let start = head + code_off;
        if start + code_len > data.len() {
            return None;
        }

        let mut instructions = Vec::new();
        let mut pos = start;
        while pos < start + code_len {
            let d0 = word(data, pos)?;
            let bits = [
                word(data, pos + 4)?,
                word(data, pos + 8)?,
                word(data, pos + 12)?,
            ];
            let sources = [
                Source::of(bits[0]),
                Source::of(bits[1]),
                Source::of(bits[2]),
            ];
            // A source selecting an inline constant makes the stream advance 32
            // rather than 16, and the four floats are that constant.
            let constant = sources.contains(&Source::Constant).then(|| {
                let mut out = [0.0f32; 4];
                for (k, slot) in out.iter_mut().enumerate() {
                    *slot = word(data, pos + 16 + 4 * k).map_or(0.0, f32::from_bits);
                }
                out
            });
            let insn = Instruction {
                opcode: ((d0 >> 24) & 0x3f) as u8,
                dst: ((d0 >> 1) & 0x3f) as u8,
                dst_half: d0 & (1 << 7) != 0,
                mask: ((d0 >> 9) & 0xf) as u8,
                saturate: d0 & (1 << 31) != 0,
                input: ((d0 >> 13) & 0xf) as u8,
                unit: ((d0 >> 17) & 0xf) as u8,
                sources,
                constant,
                end: d0 & 1 != 0,
            };
            let done = insn.end;
            instructions.push(insn);
            pos += if constant.is_some() { 32 } else { 16 };
            if done {
                break;
            }
        }
        Some(Self {
            instructions,
            declared,
        })
    }

    /// Which interpolators the program reads at all.
    ///
    /// A bit set over [`Instruction::input`] values, so `1 << 5` is `f[TC1]`.
    #[must_use]
    pub fn interpolators(&self) -> u16 {
        self.instructions
            .iter()
            .filter(|i| i.operands().any(|s| s == Source::Input))
            .fold(0u16, |acc, i| acc | 1u16 << i.input)
    }

    /// Which interpolators the program's **output** actually depends on.
    ///
    /// A forward taint over the register file, per channel: an instruction
    /// taints the channels it writes when any operand it reads is tainted, and
    /// clears them when none is - so a register that is overwritten stops
    /// carrying what it used to. The answer is the taint standing on the
    /// channels the final instruction writes.
    ///
    /// Two deliberate approximations, both toward **finding** a dependency
    /// rather than missing one, because the consequence of a false negative is
    /// worse: a surface wrongly called independent of its light term would be
    /// drawn unlit, at full albedo.
    ///
    /// - **Half and full registers are treated as one file.** `H0` and `R0`
    ///   name the same storage on NV40; keeping them apart would let taint
    ///   vanish across a precision change.
    /// - **Source swizzles are ignored.** A source reading only `.w` of a
    ///   tainted register counts as tainted. Tracking swizzles would narrow
    ///   this, and narrowing it is the direction that risks a false negative.
    ///
    /// A texture sample propagates taint from its coordinate, which is what
    /// makes this answer "does this interpolator reach the picture" rather
    /// than "is it added to it" - a coordinate reaching the output is a real
    /// dependency, just not a lighting one. Callers separating light from
    /// coordinate need [`Instruction::is_texture`] as well.
    #[must_use]
    pub fn output_depends_on(&self) -> u16 {
        self.taint(true)
    }

    /// Which interpolators reach the output **as values rather than as
    /// addresses**.
    ///
    /// [`Self::output_depends_on`] with taint blocked at a texture lookup. A
    /// `TEX` result depends on the texture's contents, not on the magnitude of
    /// the coordinate that addressed it, so propagating through one answers
    /// "did this interpolator reach the picture" where the lighting question is
    /// "did this interpolator get *combined* into it".
    ///
    /// That distinction is the whole point: on `track_surface` every block's
    /// output depends on `TC0` under the first rule, because `TC0` is the
    /// texture coordinate. Under this one only the blocks that add or multiply
    /// an interpolator into the result report it.
    #[must_use]
    pub fn output_lit_by(&self) -> u16 {
        self.taint(false)
    }

    fn taint(&self, through_textures: bool) -> u16 {
        // `tainted[reg]` is a bit per channel per interpolator, flattened: the
        // register file is small and 16 interpolators fit a `u16` each.
        let mut tainted: [[u16; 4]; 64] = [[0; 4]; 64];
        let mut last: Option<(u8, u8)> = None;
        for insn in &self.instructions {
            let mut from = 0u16;
            for source in insn.operands() {
                match source {
                    Source::Input => from |= 1u16 << insn.input.min(15),
                    Source::Register { index, .. } => {
                        for ch in tainted[usize::from(index) & 63] {
                            from |= ch;
                        }
                    }
                    _ => {}
                }
            }
            // A sampled value is the texture's, not the coordinate's.
            if insn.is_texture() && !through_textures {
                from = 0;
            }
            let dst = usize::from(insn.dst) & 63;
            for (channel, slot) in tainted[dst].iter_mut().enumerate() {
                if insn.mask & (1 << channel) != 0 {
                    // Written this instruction: it carries what was read, and
                    // nothing of what it held before.
                    *slot = from;
                }
            }
            last = Some((insn.dst, insn.mask));
        }
        let Some((dst, mask)) = last else {
            return 0;
        };
        let mut out = 0u16;
        for (channel, slot) in tainted[usize::from(dst) & 63].iter().enumerate() {
            if mask & (1 << channel) != 0 {
                out |= slot;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
