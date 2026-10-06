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

mod alpha_gate;
mod opcode;

/// One fragment-program dword, whose 16-bit halves are stored swapped.
#[must_use]
fn word(data: &[u8], at: usize) -> Option<u32> {
    let lo = u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?);
    let hi = u16::from_be_bytes(data.get(at + 2..at + 4)?.try_into().ok()?);
    Some(u32::from(hi) << 16 | u32::from(lo))
}

/// The 16-byte code slots one parameter's `fslot` patches, or `[]` for a
/// chain that runs off the end of the data - a malformed block still
/// decodes, it just patches nothing.
///
/// A fragment program has no constant registers of its own: a source that
/// selects `CONST` takes its four floats from the 16 bytes right after the
/// instruction, so a parameter with no hardware register is instead patched
/// straight into that inline slot. Following the chain from `fslot` is what
/// turns a printed `0.0` into the parameter that put it there:
///
/// ```text
/// u16 index     at  at + fslot
/// u32 entry_off at  at + program_at + 0x18 + 4 * index   (rel. to at+program_at)
/// u16 count     at  at + program_at + entry_off
/// u16 slot[i]   at  at + program_at + entry_off + 2 + 2*i
/// ```
///
/// Port of `scripts/ps3-microcode.py`'s `fp_patch_slots`, the reference this
/// mirrors and which `docs/formats/rcsmaterial.md`'s "The glass family's
/// second slot" validated first against `diffuse_with_specular_from_alpha`'s
/// already-published const slots (`0x9`, `0x13`) before trusting it more
/// widely. Confidence 82, inherited from that reading.
fn patch_slots(data: &[u8], at: usize, program_at: usize, fslot: u16) -> Vec<u16> {
    (|| {
        let base = at + program_at;
        let index = usize::from(u16::from_be_bytes(
            data.get(at + usize::from(fslot)..at + usize::from(fslot) + 2)?
                .try_into()
                .ok()?,
        ));
        let entries = u32::from_be_bytes(data.get(base + 0x14..base + 0x18)?.try_into().ok()?);
        if index as u32 >= entries {
            return Some(Vec::new());
        }
        let entry_off = usize::try_from(u32::from_be_bytes(
            data.get(base + 0x18 + 4 * index..base + 0x18 + 4 * index + 4)?
                .try_into()
                .ok()?,
        ))
        .ok()?;
        let count = usize::from(u16::from_be_bytes(
            data.get(base + entry_off..base + entry_off + 2)?
                .try_into()
                .ok()?,
        ));
        (0..count)
            .map(|i| {
                let o = base + entry_off + 2 + 2 * i;
                Some(u16::from_be_bytes(data.get(o..o + 2)?.try_into().ok()?))
            })
            .collect()
    })()
    .unwrap_or_default()
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

/// The four lane indices a source's swizzle field selects.
fn swizzle_of(bits: u32) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (i, lane) in out.iter_mut().enumerate() {
        *lane = ((bits >> (9 + 2 * i)) & 3) as u8;
    }
    out
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
    /// Each source's four-component swizzle, as lane indices `0..=3`.
    ///
    /// Beside [`Self::sources`] rather than inside [`Source`] because it is a
    /// property of the *read*, not of where the value lives: the same register
    /// is read `.xyzw` by one instruction and `.wwww` by the next, and folding
    /// the swizzle into the enum would make those two different sources.
    ///
    /// The encoding is the reference decoder's: component `i` selects lane
    /// `(bits >> (9 + 2 * i)) & 3`, so an unswizzled read is `[0, 1, 2, 3]`.
    pub swizzles: [[u8; 4]; 3],
    /// The inline constant that followed, when a source selected one.
    pub constant: Option<[f32; 4]>,
    /// The constant's own 16-byte code slot, when [`Self::constant`] is
    /// `Some` - always this instruction's own slot plus one, since the
    /// constant occupies the 16 bytes immediately after it. This is the slot
    /// number [`Program::patches`]'s parameter-patch table names, so a caller
    /// can tell a real file-authored literal from one the engine overwrites
    /// at draw time.
    pub const_slot: Option<u16>,
    /// Whether each source is negated before use - bit 17 of its own word.
    /// `scripts/ps3-microcode.py` printed this from the start; this decoder
    /// dropped it, which reads `EX2(-(d*k)^2)` (the fog curve) as
    /// `EX2(+(d*k)^2)`, a saturate that is always 1.
    pub negate: [bool; 3],
    /// The instruction's 11-bit condition field, word 1 bits 18-28. `0x727`
    /// (`TR` on every lane) is the unconditional default - see
    /// `scripts/ps3-microcode.py`'s `fp_cond`, confidence 80 there.
    pub cond: u16,
    /// Whether this instruction ends the program.
    pub end: bool,
}

impl Instruction {
    /// The mnemonic, or `None` for an opcode this reading does not name.
    #[must_use]
    pub fn name(&self) -> Option<&'static str> {
        opcode::name(self.opcode)
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
                | "PK4B" | "PK2H" | "PK4UB" | "PK2US" | "LIT_EX2_NV40",
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
    /// Every parameter patched straight into the code, as `(code slot, name
    /// hash)` pairs - resolved once at parse time, since the chain needs the
    /// block's raw bytes [`Self::declared`] no longer carries. See
    /// [`Self::patches`].
    parameter_patches: Vec<(u16, u32)>,
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
            let swizzles = [
                swizzle_of(bits[0]),
                swizzle_of(bits[1]),
                swizzle_of(bits[2]),
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
            // The constant, when there is one, occupies the very next 16-byte
            // slot - the one a parameter's patch list names.
            let const_slot = if constant.is_some() {
                u16::try_from((pos - start) / 16 + 1).ok()
            } else {
                None
            };
            let insn = Instruction {
                opcode: ((d0 >> 24) & 0x3f) as u8,
                dst: ((d0 >> 1) & 0x3f) as u8,
                dst_half: d0 & (1 << 7) != 0,
                mask: ((d0 >> 9) & 0xf) as u8,
                saturate: d0 & (1 << 31) != 0,
                input: ((d0 >> 13) & 0xf) as u8,
                unit: ((d0 >> 17) & 0xf) as u8,
                sources,
                swizzles,
                constant,
                const_slot,
                negate: [
                    bits[0] & (1 << 17) != 0,
                    bits[1] & (1 << 17) != 0,
                    bits[2] & (1 << 17) != 0,
                ],
                cond: ((bits[0] >> 18) & 0x7ff) as u16,
                end: d0 & 1 != 0,
            };
            let done = insn.end;
            instructions.push(insn);
            pos += if constant.is_some() { 32 } else { 16 };
            if done {
                break;
            }
        }
        let parameter_patches = declared
            .parameter_patches
            .iter()
            .filter(|&&(_, vreg, _)| vreg == 0xffff)
            .flat_map(|&(hash, _, fslot)| {
                patch_slots(data, at, program_at, fslot)
                    .into_iter()
                    .map(move |slot| (slot, hash))
            })
            .collect();
        Some(Self {
            instructions,
            declared,
            parameter_patches,
        })
    }

    /// Every code slot the parameter named `hash` patches directly into the
    /// code, at draw time - `[]` when it is not patched that way (bound to a
    /// hardware constant register instead, or not declared at all).
    ///
    /// **What this is for.** [`Self::specular_exponent`] can read a literal
    /// `0.0` off the file honestly and still be reporting a value the file
    /// itself never uses - `pow(x, 0) = 1` is not a plausible authored
    /// shininess, and a disc-wide sweep found this the largest single
    /// bucket. Checking whether [`Instruction::const_slot`] of that chain's
    /// `MUL` is in the list this returns for [`crate::rcsmaterial::SPECULAR_POWER`]
    /// is what tells "the file really says zero" apart from "the engine
    /// overwrites this before it is ever zero on screen" - see
    /// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "Ships have no
    /// Lambert diffuse either".
    ///
    /// **Two different parameters patching the same slot is not excluded by
    /// this method** - it answers only "does `hash` patch `slot`", not "is
    /// `hash` the only one that does". Checked rather than assumed:
    /// `crates/render/examples/hd_specular_patch_census.rs` sweeps every
    /// pair of a resolved block's own declared parameters and found no two
    /// distinct ones ever sharing a slot, over 4,586 resolved blocks across
    /// 16 circuits - a one-to-one relation holds disc-wide as measured, not
    /// by construction.
    pub fn patches(&self, hash: u32) -> impl Iterator<Item = u16> + '_ {
        self.parameter_patches
            .iter()
            .filter(move |&&(_, h)| h == hash)
            .map(|&(slot, _)| slot)
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
    /// **colour channels of the last instruction that writes one** - not on
    /// the final instruction, which in most lit programs is `MOV H0.w,
    /// {const}` and would report nothing.
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

    /// The literal exponent of a `pow(N.H, e) = exp2(e * log2(N.H))` chain -
    /// a saturated `DP3` feeding `LG2` then a `MUL` by a constant then `EX2`
    /// - when that chain's result reaches the program's output colour.
    ///
    /// This is the specular term's exponent on every lit material read so
    /// far: `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "Ships have
    /// no Lambert diffuse either" reads the ship's own `pow(N.H, 40)` this
    /// way (`@0x40 LG2 / @0x43 MUL by 40 / @0x46 EX2`), and `mesh.wgsl`
    /// hard-codes the same idiom's exponent at a shared `32` - which the same
    /// page's disc sweep shows is only the commonest of six values (`5`,
    /// `10`, `32`, `26.156`, `40`, `300`).
    ///
    /// **Three filters, none optional, because the bare `LG2`/`MUL`/`EX2`
    /// shape alone is not specific to specular at all.** Measured disc-wide:
    /// the identical three instructions raise a texture sample to a power on
    /// `track_surface`'s own lightmap curve (`pow(lightmap.rgb, k)`,
    /// `renderer.md`'s block #9), compute `exp(x)` off a saturated dot by the
    /// unrelated `log2(e)` constant, and raise other quantities to a power
    /// reachability alone does not separate from a real specular term. So:
    ///
    /// - [`Self::dp3_feeding`] requires the `LG2` to read a register a
    ///   saturated `DP3` most recently wrote - the `N.H`/`N.L` idiom every
    ///   specular and sun-diffuse read in `renderer.md` shares, which a
    ///   texture-fed power curve does not.
    /// - The exponent literal is rejected when it is `log2(e)` (within
    ///   `1e-3`): `EX2(log2(e) * LG2(x)) = exp(x)`, the idiom `mesh.wgsl`'s
    ///   own fog curve names this constant for. A saturated dot raised
    ///   through `exp()` rather than `pow()` - a Fresnel falloff, say -
    ///   shares both other gates without being this idiom.
    /// - [`Self::reaches_output`] requires the chain's result to survive to
    ///   the last instruction that writes a colour channel - the same
    ///   instruction [`Self::taint`]'s own `last` picks out: most lit
    ///   programs end `MOV H0.w, {const}`, alpha from a constant, and
    ///   stopping at the final instruction would miss an earlier colour.
    ///
    /// Chains are tried **last first**: the specular term characteristically
    /// follows the diffuse sum (`renderer.md`'s block reads all place it
    /// after `N.L`), so the chain closest to the end wins first.
    ///
    /// `None` when no such chain passes every filter - every unlit and
    /// emissive program, and any block this pattern does not fit.
    ///
    /// **A literal `0.0` is not filtered here and is a real answer this
    /// method can return.** `pow(x, 0) = 1` is not a plausible authored
    /// shininess, and it is a large real bucket - the strongest candidate for
    /// `SpecularPower` patched at draw time rather than baked in the file,
    /// matching this page's own "patched at draw time" readings elsewhere.
    /// Deciding whether to trust a literal zero is a caller's policy, not
    /// this method's: it reports what the file says, faithfully.
    ///
    /// Confidence 80: the pattern is instruction shape, not block meaning, so
    /// a non-specular power fed by a saturated dot product and reaching the
    /// output would read as this method's answer too. **Re-measured
    /// disc-wide 2026-09-10, and orthogonal to `declares_zone`** (287/1,467
    /// toggling files resolve non-Zone) - `renderer.md`'s 2026-09-10 entry.
    #[must_use]
    pub fn specular_exponent(&self) -> Option<f32> {
        self.specular_exponent_chain().map(|(value, ..)| value)
    }

    /// The code slot [`Self::specular_exponent`]'s own chain's `MUL` constant
    /// occupies, when that chain exists - regardless of whether the value
    /// survived the method's `log2(e)` exclusion, so a caller distinguishing
    /// "the file really authors this exponent" from "the engine overwrites it
    /// at draw time" sees the slot even on the `0.0` bucket
    /// [`Self::specular_exponent`]'s own doc comment names. Feed it to
    /// [`Self::patches`] with [`crate::rcsmaterial::SPECULAR_POWER`] to ask
    /// that question.
    #[must_use]
    pub fn specular_exponent_slot(&self) -> Option<u16> {
        self.specular_exponent_chain().and_then(|(_, slot, _)| slot)
    }

    /// The index of the saturated `DP3` feeding [`Self::specular_exponent`]'s
    /// own chain, when that chain exists. [`Self::instructions`] indexed at
    /// the returned value is the `DP3` itself; its two
    /// [`Instruction::operands`] are the two vectors it dots, which a
    /// caller can trace back further by register identity the same way this
    /// method's own search does - `renderer.md`'s "Ships have no Lambert
    /// diffuse either" is the worked example, and its own limits.
    #[must_use]
    pub fn specular_exponent_dp3(&self) -> Option<usize> {
        self.specular_exponent_chain().map(|(_, _, dp3)| dp3)
    }

    /// The shared search behind [`Self::specular_exponent`],
    /// [`Self::specular_exponent_slot`] and [`Self::specular_exponent_dp3`] -
    /// see the first's doc comment for the three filters and why each is
    /// load-bearing.
    fn specular_exponent_chain(&self) -> Option<(f32, Option<u16>, usize)> {
        for i in (0..self.instructions.len()).rev() {
            if self.instructions[i].name() != Some("LG2") {
                continue;
            }
            let Some(dp3_i) = self.dp3_feeding(i) else {
                continue;
            };
            let lg2_dst = (self.instructions[i].dst, self.instructions[i].dst_half);
            let Some(mul_i) = self.instructions[i + 1..].iter().position(|insn| {
                insn.name() == Some("MUL")
                    && insn.constant.is_some()
                    && insn
                        .operands()
                        .any(|s| matches!(s, Source::Register { index, half } if (index, half) == lg2_dst))
            }) else {
                continue;
            };
            let mul_i = i + 1 + mul_i;
            let mul = &self.instructions[mul_i];
            let exponent = mul
                .constant
                .expect("checked by the position() predicate above");
            let mul_dst = (mul.dst, mul.dst_half);
            let Some(ex2_i) = self.instructions[mul_i + 1..].iter().position(|insn| {
                insn.name() == Some("EX2")
                    && insn
                        .operands()
                        .any(|s| matches!(s, Source::Register { index, half } if (index, half) == mul_dst))
            }) else {
                continue;
            };
            let ex2_i = mul_i + 1 + ex2_i;
            if self.reaches_output(ex2_i) {
                // The scalar rides in whichever lane the `MUL` actually
                // reads; the others are the constant slot's unused padding.
                let value = exponent
                    .into_iter()
                    .find(|v| *v != 0.0)
                    .unwrap_or(exponent[0]);
                // **A third false-positive class, found on a disc-wide sweep
                // and not a guess: `LG2` / `MUL log2(e)` / `EX2` is not this
                // idiom at all.** `EX2(log2(e) * LG2(x)) = exp(x)`, and
                // `mesh.wgsl`'s own fog curve names exactly this constant for
                // exactly this reason ("a MUL by log2(e) into EX2_SAT"). A
                // saturated dot product raised through `exp()` instead of
                // `pow()` is a Fresnel-style falloff sharing the specular
                // idiom's two gates without being it, not a shininess of
                // `1.4427`. Skip and keep looking, rather than returning a
                // value that is not the exponent this method promises.
                if (value - std::f32::consts::LOG2_E).abs() > 1e-3 {
                    return Some((value, mul.const_slot, dp3_i));
                }
            }
        }
        None
    }

    /// The index of the saturated `DP3` that most recently, lane-soundly
    /// wrote the register the `LG2` at `at` reads - the `N.H`/`N.L` idiom
    /// `renderer.md` reads under every specular and sun-diffuse term, and
    /// the discriminator a bare `LG2`/`MUL`/`EX2` instruction shape does not
    /// carry on its own: see [`Self::specular_exponent`]'s own doc comment
    /// for what this excludes and why it has to.
    ///
    /// **Lane-aware and clobber-checked, not "any nonzero mask" - a naive
    /// version shipped first and was wrong both ways, disc-wide (renderer.md,
    /// "Ships have no Lambert diffuse either"): 1,679 of 6,141 then-resolved
    /// blocks were credited off a wrong-lane `DP3`, and 5,556 missed a real
    /// one behind an unrelated write to a different lane.** `LG2` always
    /// reads exactly one lane in practice (`hd_lg2_swizzle_check.rs`).
    ///
    /// `None` when the `LG2`'s own operand is not a register, when no single
    /// instruction before it covers every lane it reads in one write, when a
    /// later partial write to any of those lanes clobbers it before `at`, or
    /// when the covering writer is not a saturated `DP3`.
    fn dp3_feeding(&self, at: usize) -> Option<usize> {
        let reader = &self.instructions[at];
        let Some(Source::Register { index, half }) = reader.operands().next() else {
            return None;
        };
        let reg = (index, half);
        let sw = reader.swizzles[0];
        let needed: u8 = (0..4)
            .filter(|&i| reader.mask & (1 << i) != 0)
            .fold(0u8, |acc, i| acc | (1 << sw[i]));
        let (w, writer) = self.instructions[..at]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, insn)| (insn.dst, insn.dst_half) == reg && insn.mask & needed == needed)?;
        if writer.name() != Some("DP3") || !writer.saturate {
            return None;
        }
        let clobbered = self.instructions[w + 1..at]
            .iter()
            .any(|insn| (insn.dst, insn.dst_half) == reg && insn.mask & needed != 0);
        (!clobbered).then_some(w)
    }

    /// Whether the value the instruction at `from` writes survives, by
    /// register identity, to the last instruction that writes a colour
    /// channel - [`Self::taint`]'s own definition of "the output", seeded
    /// on one register instead of on an interpolator.
    ///
    /// Half and full registers are merged into one file here, the same
    /// simplification [`Self::taint`] makes and for the same reason: this is
    /// a yes/no reachability question, where merging two registers can only
    /// widen what is found reachable, never narrow it - a safe
    /// over-approximation. [`Self::output_texels`] keeps them apart because
    /// that question extracts a specific channel rather than asking whether
    /// one is reached at all.
    fn reaches_output(&self, from: usize) -> bool {
        let seed = &self.instructions[from];
        let mut tainted: [[bool; 4]; 64] = [[false; 4]; 64];
        for (channel, slot) in tainted[usize::from(seed.dst) & 63].iter_mut().enumerate() {
            *slot = seed.mask & (1 << channel) != 0;
        }
        let mut reaches = false;
        for insn in &self.instructions[from + 1..] {
            let from_tainted = insn.operands().any(|s| {
                matches!(s, Source::Register { index, .. } if tainted[usize::from(index) & 63].iter().any(|b| *b))
            });
            let dst = usize::from(insn.dst) & 63;
            for (channel, slot) in tainted[dst].iter_mut().enumerate() {
                if insn.mask & (1 << channel) != 0 {
                    *slot = from_tainted;
                }
            }
            if insn.mask & 0b0111 != 0 {
                reaches = from_tainted;
            }
        }
        reaches
    }

    fn taint(&self, through_textures: bool) -> u16 {
        // `tainted[reg]` is a bit per channel per interpolator, flattened: the
        // register file is small and 16 interpolators fit a `u16` each.
        let mut tainted: [[u16; 4]; 64] = [[0; 4]; 64];
        // The last instruction to write a colour channel, which is what the
        // picture is - see below.
        let mut last: Option<u8> = None;
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
            // **The colour channels of the output, not the last write.** Most
            // lit programs finish on `MOV H0.w, {const}` - alpha from a
            // constant - and taking the taint standing on only that channel
            // reports zero while the colour written several instructions
            // earlier is discarded. Measured on seven Talon's Junction
            // materials: every one whose last instruction was that `MOV`
            // answered `0x0000` under the old rule, which is what made the
            // lit-versus-emissive split unusable.
            if insn.mask & 0b0111 != 0 {
                last = Some(insn.dst);
            }
        }
        let Some(dst) = last else {
            return 0;
        };
        let mut out = 0u16;
        for (channel, slot) in tainted[usize::from(dst) & 63].iter().enumerate() {
            if channel < 3 {
                out |= slot;
            }
        }
        out
    }
}

/// Where one lane of a fragment program's output comes from.
///
/// The answer [`Program::output_texels`] gives per output channel, and the
/// primitive the second-texture-role reading in `oag_render` is built on: a
/// material that samples two units says in its own microcode which of them is
/// the picture and which channel of which is the coverage, and every earlier
/// attempt at that question guessed from a file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Texel {
    /// Nothing this reading follows reaches this lane: an interpolator, a
    /// constant, or a chain it lost.
    #[default]
    Untraced,
    /// One texture unit's sample reaches it, and no other does.
    Unit {
        /// The unit sampled.
        unit: u8,
        /// Which channel of the sample, when every contribution agrees on one.
        /// `None` when the lane mixes channels of the same unit.
        channel: Option<u8>,
    },
    /// More than one unit reaches it.
    Mixed,
}

impl Texel {
    /// Two contributions to one lane, combined.
    ///
    /// [`Self::Untraced`] is the identity rather than an absorber, and that is
    /// the load-bearing choice: a texel multiplied by a light term or a
    /// constant is still that texel's, and treating the constant as diluting
    /// it would report `Untraced` for every shaded surface on the disc.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Untraced, x) | (x, Self::Untraced) => x,
            (Self::Mixed, _) | (_, Self::Mixed) => Self::Mixed,
            (
                Self::Unit {
                    unit: a,
                    channel: p,
                },
                Self::Unit {
                    unit: b,
                    channel: q,
                },
            ) => {
                if a == b {
                    Self::Unit {
                        unit: a,
                        channel: if p == q { p } else { None },
                    }
                } else {
                    Self::Mixed
                }
            }
        }
    }

    /// The unit this lane comes from, when exactly one does.
    #[must_use]
    pub fn unit(self) -> Option<u8> {
        match self {
            Self::Unit { unit, .. } => Some(unit),
            _ => None,
        }
    }
}

impl Program {
    /// Which texture unit and channel each of the output's four lanes comes
    /// from: `[x, y, z, w]`.
    ///
    /// A forward taint like [`Program::output_depends_on`]'s, and different
    /// from it in the two ways that matter for this question:
    ///
    /// - **Source swizzles are honoured.** That is the whole point. The
    ///   difference between a cloud plate whose alpha is unit 0's `.x` and one
    ///   whose alpha is unit 0's `.w` is a swizzle, and this project drew the
    ///   second for a year because nothing read the first.
    /// - **A texture sample starts a taint rather than propagating one.** The
    ///   value is the texture's; the coordinate that addressed it is not part
    ///   of the answer.
    ///
    /// # Half and full registers are two files here, and that is measured
    ///
    /// [`Program::output_depends_on`] treats `H2` and `R2` as one storage, on
    /// the grounds that they name the same register on NV40 - a safe
    /// over-approximation for a yes/no dependency question, where merging two
    /// registers can only *add* taint. It is not safe for this one, and a
    /// shipped program says so. `talons_junction/materials/clouds.rcsmaterial`,
    /// fragment block `0x1540`:
    ///
    /// ```text
    /// @25  MAD H2.xyz, H5, H4, H2   ; H4 is unit 1's sample - H2 now carries it
    /// @26  MAD R2.xyz, {c}, R1.wwww, {c}
    /// @27  MAD H2.xyz, R1.wwww, H2, R2   ; reads H2 back, still unit 1's
    /// ```
    ///
    /// Under one file `@26` clobbers what `@25` put in `H2` and `@27` reads a
    /// register the program never wrote, so the colour comes out
    /// [`Texel::Untraced`] where the disassembly plainly shows unit 1 reaching
    /// it. Two files reproduce the hand read. That is consistent with the
    /// hardware either way: NV40 packs `H[2i]`/`H[2i+1]` into `R[i]`, so
    /// `H2` and `R2` were never the same storage - index identity was the
    /// wrong pairing rather than the wrong idea, and a compiler that allocated
    /// these disjointly is served correctly by keeping them apart.
    ///
    /// A conditional or a loop would be read straight through; shipped
    /// fragment programs here are straight-line code and none was found with a
    /// branch, so the question does not arise on this disc.
    ///
    /// The output register is the one the **end instruction** writes, and its
    /// colour lanes are usually written several instructions earlier: a lit
    /// program characteristically finishes `MOV H0.w, <sample>.xxxx END`
    /// after an `ADD H0.xyz`. Taking all four lanes of that register is what
    /// makes both halves readable at once.
    #[must_use]
    pub fn output_texels(&self) -> [Texel; 4] {
        // Indexed `[half][register]`, the two files kept apart - see above.
        let mut file = [[[Texel::Untraced; 4]; 64]; 2];
        let mut output = (0usize, 0usize);
        for insn in &self.instructions {
            let dst = (usize::from(insn.dst_half), usize::from(insn.dst) & 63);
            // A dot product collapses every lane of its sources into one
            // scalar, so a per-lane read would miss most of what feeds it.
            let scalar = matches!(insn.name(), Some("DP3" | "DP4" | "DP2A" | "RFL"));
            let mut lanes = [Texel::Untraced; 4];
            if insn.is_texture() {
                // Destination lane `i` receives channel `i` of the sample, so
                // `TEX H0.x` is the red channel and nothing else.
                for (i, lane) in lanes.iter_mut().enumerate() {
                    *lane = Texel::Unit {
                        unit: insn.unit,
                        channel: Some(i as u8),
                    };
                }
            } else {
                for (slot, source) in insn.operands().enumerate() {
                    let Source::Register { index, half } = source else {
                        continue;
                    };
                    let held = file[usize::from(half)][usize::from(index) & 63];
                    for (i, lane) in lanes.iter_mut().enumerate() {
                        if scalar {
                            for read in held {
                                *lane = lane.merge(read);
                            }
                        } else {
                            let pick = usize::from(insn.swizzles[slot][i]);
                            *lane = lane.merge(held[pick]);
                        }
                    }
                }
            }
            for (i, slot) in file[dst.0][dst.1].iter_mut().enumerate() {
                if insn.mask & (1 << i) != 0 {
                    *slot = lanes[i];
                }
            }
            if insn.end {
                output = dst;
            }
        }
        file[output.0][output.1]
    }

    /// Whether a sample from `unit` is **added into** the program's result
    /// rather than replacing or modulating it.
    ///
    /// This is the one question that separates Wipeout HD's emissive family
    /// from every other two-texture material, and the renderer needs it
    /// because `mesh.wgsl` *selects* between its two textures where these
    /// *add* one to the other: `MAD H0.xyz, H0.wwww, H1, H0` is albedo plus
    /// diffuse-alpha times the tinted emissive sample. Without it those
    /// surfaces draw their diffuse alone and their glow is simply absent.
    ///
    /// **The test is an accumulate**: a `MAD` or `ADD` whose destination is
    /// also one of its sources, reached by a value the unit's sample flows
    /// into. The taint travels, so a sample folded through a tint still
    /// counts, and a register is `(ordinal, half)` for the same reason
    /// [`output_texels`](Self::output_texels) keeps the two files apart -
    /// `H2` and `R2` were never the same storage.
    ///
    /// **Confidence 88, from a disc-wide sweep rather than from the one block
    /// it was read on.** `scripts/hd_time_shapes.py` runs the same predicate
    /// over every fragment block of every material declaring the engine's
    /// `time`: 2,230 of 2,305 blocks accumulate, 290 of 291 materials take
    /// `time` into a texture coordinate, and the 35 materials that are not
    /// *purely* accumulate all mix accumulating blocks with multiply-only
    /// ones - the pass dimension, not a second combining rule. Exactly one
    /// material on the disc is multiply-only throughout.
    ///
    /// Straight-line code is assumed, as everywhere else in this module: no
    /// shipped fragment program here was found with a branch.
    #[must_use]
    pub fn accumulates(&self, unit: u8) -> bool {
        let mut sampled: std::collections::BTreeSet<(u8, bool)> = Default::default();
        for insn in &self.instructions {
            let dst = (insn.dst, insn.dst_half);
            if insn.is_texture() {
                if insn.unit == unit {
                    sampled.insert(dst);
                } else {
                    // A fetch overwrites, so a register reused by another
                    // unit's sample stops carrying this one's.
                    sampled.remove(&dst);
                }
                continue;
            }
            let sources: Vec<(u8, bool)> = insn
                .operands()
                .filter_map(|source| match source {
                    Source::Register { index, half } => Some((index, half)),
                    _ => None,
                })
                .collect();
            if !sources.iter().any(|s| sampled.contains(s)) {
                // Not fed by the sample, but it may still clobber the register
                // that was holding it.
                sampled.remove(&dst);
                continue;
            }
            // **The destination has to be the *addend*, not just present.**
            // `MAD H0, H0, H1, H3` names `H0` on both sides and is a multiply
            // of `H0` by the sample, not an accumulate of the sample into it;
            // the real shape is `MAD H0.xyz, H0.wwww, H1, H0`, where the third
            // operand is what is added to. `ADD` is commutative, so either
            // operand counts there.
            let addend = match insn.name() {
                Some("MAD") => {
                    insn.operands().nth(2)
                        == Some(Source::Register {
                            index: dst.0,
                            half: dst.1,
                        })
                }
                Some("ADD") => sources.contains(&dst),
                _ => false,
            };
            if addend {
                return true;
            }
            sampled.insert(dst);
        }
        false
    }
}

#[cfg(test)]
mod tests;
