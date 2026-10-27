//! Sea Islands (GCN 1.1) instruction words to instructions with operands.
//!
//! Every field position, prefix and operand code below is from AMD's *Sea Islands Series
//! Instruction Set Architecture* (revision 1.0, February 2013), chapter 13 "Microcode Formats".
//! The section is named at each encoding. Two of that chapter's field tables disagree with the
//! rest of the manual, and the manual's own other table wins both times, as the disc does:
//!
//! - SMRD `OP` is `[26:22]` (table 7.1); section 13.2 prints `[26:21]`, which overlaps `SDST`.
//! - MUBUF `OP` is `[24:18]` (table 8.2); section 13.6 prints `[15:8]`, which overlaps `OFFSET`.
//!
//! **Three words outside the Sea Islands manual** occur in Omega's programs, and are decoded from
//! AMD's *"Vega" Instruction Set Architecture* (28 July 2017) instead, as the PS4 Pro's
//! additions: they appear only in some programs (none of the plain-named circuit materials
//! read), and each program that has them still closes on `s_endpgm` at exactly its footer's
//! length, which a wrong instruction length could not survive.
//!
//! - **VOP3P** (packed 16-bit math, Vega section 13.3.6): Vega's field layout and opcode numbers,
//!   but the prefix byte `[31:24]` reads `0xCC` on the disc where Vega's manual has `0xD3`.
//! - **SDWA** (Vega 13.3.7, VOPC's in 13.3.8): a VOP1/VOP2/VOPC `SRC0` of 249 takes a second dword
//!   of sub-dword selects.
//! - **DPP** (Vega 13.3.9): `SRC0` 250, a second dword of lane-swizzle controls.
//!
//! What this decoder does **not** know, by design: how many address registers an image or
//! buffer instruction reads (it depends on the resource's dimension, which lives in a descriptor
//! at run time), so those print their first register only; and register widths of 64-bit
//! operands are taken from the opcode's type suffix (`_b64`, `_f64`), which is the manual's own
//! naming rule (section 12.1) but only cosmetic here.

use core::fmt;

use super::opcodes;

/// The microcode format a word was encoded in (section 13.1, table 13.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Encoding {
    Sop2,
    Sopk,
    Sop1,
    Sopc,
    Sopp,
    Smrd,
    Vop2,
    Vop1,
    Vopc,
    Vop3,
    Vintrp,
    Ds,
    Mubuf,
    Mtbuf,
    Mimg,
    Exp,
    Flat,
    /// The PS4 Pro's packed 16-bit math (not in the Sea Islands manual; see the module docs).
    Vop3p,
}

impl Encoding {
    /// The manual's name for the format.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Sop2 => "SOP2",
            Self::Sopk => "SOPK",
            Self::Sop1 => "SOP1",
            Self::Sopc => "SOPC",
            Self::Sopp => "SOPP",
            Self::Smrd => "SMRD",
            Self::Vop2 => "VOP2",
            Self::Vop1 => "VOP1",
            Self::Vopc => "VOPC",
            Self::Vop3 => "VOP3",
            Self::Vintrp => "VINTRP",
            Self::Ds => "DS",
            Self::Mubuf => "MUBUF",
            Self::Mtbuf => "MTBUF",
            Self::Mimg => "MIMG",
            Self::Exp => "EXP",
            Self::Flat => "FLAT",
            Self::Vop3p => "VOP3P",
        }
    }

    /// Which format a first instruction word is, from its prefix bits; `None` for a prefix no
    /// Sea Islands format uses. Longest prefixes are tested first, since SOP1/SOPC/SOPP sit
    /// inside SOPK's, SOPK's inside SOP2's, and VOP1/VOPC inside VOP2's.
    #[must_use]
    pub fn of(word: u32) -> Option<Self> {
        if word >> 31 == 0 {
            return Some(match word >> 25 {
                0x3f => Self::Vop1,
                0x3e => Self::Vopc,
                _ => Self::Vop2,
            });
        }
        if word >> 30 == 0b10 {
            return Some(match word >> 23 {
                0x17d => Self::Sop1,
                0x17e => Self::Sopc,
                0x17f => Self::Sopp,
                _ if word >> 28 == 0b1011 => Self::Sopk,
                _ => Self::Sop2,
            });
        }
        if word >> 27 == 0b11000 {
            return Some(Self::Smrd);
        }
        if word >> 24 == 0xcc {
            return Some(Self::Vop3p);
        }
        match word >> 26 {
            0b11_0100 => Some(Self::Vop3),
            0b11_0010 => Some(Self::Vintrp),
            0b11_0110 => Some(Self::Ds),
            0b11_0111 => Some(Self::Flat),
            0b11_1000 => Some(Self::Mubuf),
            0b11_1010 => Some(Self::Mtbuf),
            0b11_1100 => Some(Self::Mimg),
            0b11_1110 => Some(Self::Exp),
            _ => None,
        }
    }

    /// Whether the format is two words long before any literal.
    #[must_use]
    pub fn is_64_bit(self) -> bool {
        matches!(
            self,
            Self::Vop3
                | Self::Vop3p
                | Self::Ds
                | Self::Mubuf
                | Self::Mtbuf
                | Self::Mimg
                | Self::Exp
                | Self::Flat
        )
    }
}

/// One half, or both, of a 64-bit special register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Half {
    Lo,
    Hi,
    Both,
}

/// One operand, as the source-operand enumeration of section 13.1 (SOP2's `SSRC0`) and 13.3
/// (VOP's 9-bit `SRC0`) number them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operand {
    /// Scalar registers `first..first + count`.
    Sgpr {
        first: u8,
        count: u8,
    },
    /// Vector registers `first..first + count`.
    Vgpr {
        first: u8,
        count: u8,
    },
    /// Trap temporaries (codes 112-123).
    Ttmp {
        first: u8,
        count: u8,
    },
    Vcc(Half),
    Exec(Half),
    Tba(Half),
    Tma(Half),
    M0,
    Scc,
    Vccz,
    Execz,
    /// Code 254 in a vector source: the LDS-direct read.
    LdsDirect,
    /// Codes 128-208: an inline integer, 0 to 64 and -1 to -16.
    Int(i8),
    /// Codes 240-247: an inline float, +-0.5, 1, 2 or 4.
    Float(f32),
    /// Code 255: the 32-bit literal that follows the instruction.
    Literal(u32),
    /// An immediate field that is not a source code: an SMRD dword offset, an interpolation
    /// parameter select.
    Imm(u32),
    /// A code the manual lists as reserved.
    Reserved(u16),
}

impl Operand {
    /// A scalar-operand code (8 bits, or the low 9 bits of a vector source) of `count`
    /// registers. `literal` is the dword after the instruction, if one was read.
    fn scalar(code: u16, count: u8, literal: Option<u32>) -> Self {
        let half = |lo: bool| {
            if count >= 2 {
                Half::Both
            } else if lo {
                Half::Lo
            } else {
                Half::Hi
            }
        };
        match code {
            0..=103 => Self::Sgpr {
                first: code as u8,
                count,
            },
            106 | 107 => Self::Vcc(half(code == 106)),
            108 | 109 => Self::Tba(half(code == 108)),
            110 | 111 => Self::Tma(half(code == 110)),
            112..=123 => Self::Ttmp {
                first: (code - 112) as u8,
                count,
            },
            124 => Self::M0,
            126 | 127 => Self::Exec(half(code == 126)),
            128..=192 => Self::Int((code - 128) as i8),
            193..=208 => Self::Int(-((code - 192) as i8)),
            240 => Self::Float(0.5),
            241 => Self::Float(-0.5),
            242 => Self::Float(1.0),
            243 => Self::Float(-1.0),
            244 => Self::Float(2.0),
            245 => Self::Float(-2.0),
            246 => Self::Float(4.0),
            247 => Self::Float(-4.0),
            251 => Self::Vccz,
            252 => Self::Execz,
            253 => Self::Scc,
            254 => Self::LdsDirect,
            255 => literal.map_or(Self::Reserved(255), Self::Literal),
            256..=511 => Self::Vgpr {
                first: (code - 256) as u8,
                count,
            },
            _ => Self::Reserved(code),
        }
    }

    fn vgpr(first: u32, count: u8) -> Self {
        Self::Vgpr {
            first: first as u8,
            count,
        }
    }

    fn sgpr(first: u32, count: u8) -> Self {
        Self::Sgpr {
            first: first as u8,
            count,
        }
    }

    /// The literal's or inline float's value as an `f32`, for reading constants.
    #[must_use]
    pub fn as_f32(self) -> Option<f32> {
        match self {
            Self::Literal(bits) => Some(f32::from_bits(bits)),
            Self::Float(f) => Some(f),
            Self::Int(i) => Some(f32::from(i)),
            _ => None,
        }
    }
}

fn range(f: &mut fmt::Formatter<'_>, prefix: &str, first: u8, count: u8) -> fmt::Result {
    if count <= 1 {
        write!(f, "{prefix}{first}")
    } else {
        write!(
            f,
            "{prefix}[{first}:{}]",
            u16::from(first) + u16::from(count) - 1
        )
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let special = |f: &mut fmt::Formatter<'_>, name: &str, half: Half| match half {
            Half::Lo => write!(f, "{name}_lo"),
            Half::Hi => write!(f, "{name}_hi"),
            Half::Both => write!(f, "{name}"),
        };
        match *self {
            Self::Sgpr { first, count } => range(f, "s", first, count),
            Self::Vgpr { first, count } => range(f, "v", first, count),
            Self::Ttmp { first, count } => range(f, "ttmp", first, count),
            Self::Vcc(h) => special(f, "vcc", h),
            Self::Exec(h) => special(f, "exec", h),
            Self::Tba(h) => special(f, "tba", h),
            Self::Tma(h) => special(f, "tma", h),
            Self::M0 => f.write_str("m0"),
            Self::Scc => f.write_str("scc"),
            Self::Vccz => f.write_str("vccz"),
            Self::Execz => f.write_str("execz"),
            Self::LdsDirect => f.write_str("lds_direct"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(x) => write!(f, "{x:?}"),
            Self::Literal(bits) => write!(f, "{bits:#x}"),
            Self::Imm(v) => write!(f, "{v:#x}"),
            Self::Reserved(c) => write!(f, "<reserved {c}>"),
        }
    }
}

/// The fields an encoding carries beyond its registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    None,
    /// SOPP's and SOPK's `SIMM16`.
    Simm16(i16),
    /// VINTRP's attribute and channel.
    Interp {
        attr: u8,
        chan: u8,
    },
    Ds {
        offset0: u8,
        offset1: u8,
        gds: bool,
    },
    Buffer {
        offset: u16,
        offen: bool,
        idxen: bool,
        glc: bool,
        addr64: bool,
        lds: bool,
        slc: bool,
        tfe: bool,
        /// MTBUF's data and number formats; zero on MUBUF.
        dfmt: u8,
        nfmt: u8,
    },
    Image {
        dmask: u8,
        unorm: bool,
        glc: bool,
        da: bool,
        r128: bool,
        tfe: bool,
        lwe: bool,
        slc: bool,
    },
    Export {
        target: u8,
        en: u8,
        compr: bool,
        done: bool,
        vm: bool,
    },
    Flat {
        glc: bool,
        slc: bool,
        tfe: bool,
    },
    /// VOP3P's lane selects (Vega 13.3.6): `OPSEL` and `OPSEL_HI` one bit per source, `NEG_HI`.
    Packed {
        op_sel: u8,
        op_sel_hi: u8,
        neg_hi: u8,
    },
    /// SDWA's selects (Vega 13.3.7): 0-3 a byte, 4 the low word, 5 the high word, 6 the dword.
    Sdwa {
        dst_sel: u8,
        dst_unused: u8,
        src0_sel: u8,
        src1_sel: u8,
    },
    /// DPP's controls (Vega 13.3.9).
    Dpp {
        ctrl: u16,
        bound_ctrl: bool,
        bank_mask: u8,
        row_mask: u8,
    },
}

/// One decoded instruction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instruction {
    /// Byte offset of the instruction in the program's code.
    pub at: usize,
    pub encoding: Encoding,
    /// The `OP` field; for VOP3 the full 9-bit number (a VOPC, VOP2 or VOP1 opcode plus its
    /// base, section 13.3).
    pub op: u16,
    /// The manual's name, lower case; `None` for a number its table does not list.
    pub name: Option<&'static str>,
    /// Size in bytes, the literal included.
    pub size: u8,
    pub dst: [Option<Operand>; 2],
    pub src: [Option<Operand>; 4],
    /// VOP3 `NEG` and `ABS`, one bit per source.
    pub neg: u8,
    pub abs: u8,
    pub clamp: bool,
    /// VOP3 `OMOD`: 1 = *2, 2 = *4, 3 = /2.
    pub omod: u8,
    pub detail: Detail,
}

/// Why a word could not be decoded. Each stops the program: past it the stream is out of step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// A first word whose prefix bits name no Sea Islands format.
    UnknownEncoding { at: usize, word: u32 },
    /// The code ends inside an instruction or its literal.
    Truncated { at: usize },
    /// A VOP3 source of code 255: VOP3 cannot carry a literal (section 13.3, "VOP3").
    LiteralInVop3 { at: usize },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEncoding { at, word } => {
                write!(f, "unknown encoding {word:#010x} at {at:#x}")
            }
            Self::Truncated { at } => write!(f, "instruction truncated at {at:#x}"),
            Self::LiteralInVop3 { at } => write!(f, "literal operand in a VOP3 at {at:#x}"),
        }
    }
}

impl core::error::Error for DecodeError {}

fn bit(word: u32, n: u32) -> bool {
    word >> n & 1 != 0
}

fn field(word: u32, lo: u32, width: u32) -> u32 {
    (word >> lo) & ((1 << width) - 1)
}

fn name_in(table: &'static [&'static str], op: u32) -> Option<&'static str> {
    table
        .get(op as usize)
        .copied()
        .filter(|name| !name.is_empty())
}

/// The manual's compare names (section 13.3, VOPC): `v_cmp{,x,s,sx}_<op>_<type>` by base.
fn vopc_name(op: u32) -> Option<&'static str> {
    VOPC_NAMES.get(op as usize).copied().flatten()
}

macro_rules! vopc_family {
    ($prefix:literal, $ty:literal, 16) => {
        [
            concat!($prefix, "_f_", $ty),
            concat!($prefix, "_lt_", $ty),
            concat!($prefix, "_eq_", $ty),
            concat!($prefix, "_le_", $ty),
            concat!($prefix, "_gt_", $ty),
            concat!($prefix, "_lg_", $ty),
            concat!($prefix, "_ge_", $ty),
            concat!($prefix, "_o_", $ty),
            concat!($prefix, "_u_", $ty),
            concat!($prefix, "_nge_", $ty),
            concat!($prefix, "_nlg_", $ty),
            concat!($prefix, "_ngt_", $ty),
            concat!($prefix, "_nle_", $ty),
            concat!($prefix, "_neq_", $ty),
            concat!($prefix, "_nlt_", $ty),
            concat!($prefix, "_tru_", $ty),
        ]
    };
    ($prefix:literal, $ty:literal, 8) => {
        [
            concat!($prefix, "_f_", $ty),
            concat!($prefix, "_lt_", $ty),
            concat!($prefix, "_eq_", $ty),
            concat!($prefix, "_le_", $ty),
            concat!($prefix, "_gt_", $ty),
            concat!($prefix, "_lg_", $ty),
            concat!($prefix, "_ge_", $ty),
            concat!($prefix, "_tru_", $ty),
        ]
    };
}

/// All 256 VOPC numbers: sixteen families of 16 or 8 compares, plus the four class tests at
/// `0x88`, `0x98`, `0xa8`, `0xb8` (section 13.3, "Single Vector Compare Operations").
static VOPC_NAMES: [Option<&str>; 256] = {
    let mut t: [Option<&str>; 256] = [None; 256];
    let f16: [[&str; 16]; 8] = [
        vopc_family!("v_cmp", "f32", 16),
        vopc_family!("v_cmpx", "f32", 16),
        vopc_family!("v_cmp", "f64", 16),
        vopc_family!("v_cmpx", "f64", 16),
        vopc_family!("v_cmps", "f32", 16),
        vopc_family!("v_cmpsx", "f32", 16),
        vopc_family!("v_cmps", "f64", 16),
        vopc_family!("v_cmpsx", "f64", 16),
    ];
    let f8: [[&str; 8]; 8] = [
        vopc_family!("v_cmp", "i32", 8),
        vopc_family!("v_cmpx", "i32", 8),
        vopc_family!("v_cmp", "i64", 8),
        vopc_family!("v_cmpx", "i64", 8),
        vopc_family!("v_cmp", "u32", 8),
        vopc_family!("v_cmpx", "u32", 8),
        vopc_family!("v_cmp", "u64", 8),
        vopc_family!("v_cmpx", "u64", 8),
    ];
    let mut fam = 0;
    while fam < 8 {
        let mut i = 0;
        while i < 16 {
            t[fam * 16 + i] = Some(f16[fam][i]);
            i += 1;
        }
        let mut j = 0;
        while j < 8 {
            t[0x80 + fam * 16 + j] = Some(f8[fam][j]);
            j += 1;
        }
        fam += 1;
    }
    t[0x88] = Some("v_cmp_class_f32");
    t[0x98] = Some("v_cmpx_class_f32");
    t[0xa8] = Some("v_cmp_class_f64");
    t[0xb8] = Some("v_cmpx_class_f64");
    t
};

/// The name of a VOP3 opcode, through its VOPC, VOP2 or VOP1 base (section 13.3).
fn vop3_name(op: u32) -> Option<&'static str> {
    match op {
        0..=255 => vopc_name(op),
        256..=319 => name_in(&opcodes::VOP2, op - 256),
        320..=383 => name_in(&opcodes::VOP3, op - 320),
        _ => name_in(&opcodes::VOP1, op - 384),
    }
}

/// Register counts of an opcode's destination and sources, from its type suffixes: one suffix
/// types everything, two type the destination and then the sources (`v_cvt_f32_f64`).
fn widths(name: &str) -> (u8, u8) {
    let typed = |t: &&str| {
        t.len() > 1
            && matches!(t.as_bytes()[0], b'b' | b'f' | b'i' | b'u')
            && t[1..].bytes().all(|c| c.is_ascii_digit())
    };
    let width = |t: &str| if &t[1..] == "64" { 2 } else { 1 };
    let mut types = name.split('_').filter(typed);
    match (types.next(), types.next_back()) {
        (None, _) => (1, 1),
        (Some(one), None) => (width(one), width(one)),
        (Some(dst), Some(src)) => (width(dst), width(src)),
    }
}

/// Whether a second source is 32 bits wide whatever the suffix: shift counts, field widths,
/// bit indices and exponents.
fn narrow_src1(name: &str) -> bool {
    [
        "s_lshl_",
        "s_lshr_",
        "s_ashr_",
        "s_bfe_",
        "s_bfm_",
        "v_lshl_",
        "v_lshr_",
        "v_ashr_",
        "v_ldexp_",
        "v_trig_preop_",
    ]
    .iter()
    .any(|p| name.starts_with(p))
}

struct Reader<'a> {
    code: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn word(&self, n: usize) -> Result<u32, DecodeError> {
        let start = self.at + 4 * n;
        self.code
            .get(start..start + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
            .ok_or(DecodeError::Truncated { at: self.at })
    }
}

/// Decodes the instruction at byte `at` of `code`.
///
/// # Errors
///
/// See [`DecodeError`]. An opcode number missing from the manual's table inside a known format
/// is **not** an error: the format fixes the length, so it decodes with [`Instruction::name`]
/// `None` and the stream stays in step.
pub fn decode(code: &[u8], at: usize) -> Result<Instruction, DecodeError> {
    let r = Reader { code, at };
    let w0 = r.word(0)?;
    let encoding = Encoding::of(w0).ok_or(DecodeError::UnknownEncoding { at, word: w0 })?;
    let mut i = Instruction {
        at,
        encoding,
        op: 0,
        name: None,
        size: if encoding.is_64_bit() { 8 } else { 4 },
        dst: [None; 2],
        src: [None; 4],
        neg: 0,
        abs: 0,
        clamp: false,
        omod: 0,
        detail: Detail::None,
    };
    // A 32-bit format's literal is the next dword (section 13.1, "Can be followed by a 32-bit
    // literal constant"); it is read once and shared by every source that names code 255.
    let literal = |uses: bool, i: &mut Instruction| -> Result<Option<u32>, DecodeError> {
        if uses {
            let lit = r.word(1)?;
            i.size = 8;
            Ok(Some(lit))
        } else {
            Ok(None)
        }
    };
    match encoding {
        Encoding::Sop2 => {
            let op = field(w0, 23, 7);
            let (s0, s1, d) = (field(w0, 0, 8), field(w0, 8, 8), field(w0, 16, 7));
            i.op = op as u16;
            i.name = name_in(&opcodes::SOP2, op);
            let (dw, sw) = i.name.map_or((1, 1), widths);
            let s1w = if i.name.is_some_and(narrow_src1) {
                1
            } else {
                sw
            };
            let s0w = if i.name.is_some_and(|n| n.starts_with("s_bfm_")) {
                1
            } else {
                sw
            };
            let lit = literal(s0 == 255 || s1 == 255, &mut i)?;
            i.dst[0] = Some(Operand::scalar(d as u16, dw, None));
            i.src[0] = Some(Operand::scalar(s0 as u16, s0w, lit));
            i.src[1] = Some(Operand::scalar(s1 as u16, s1w, lit));
        }
        Encoding::Sopk => {
            let op = field(w0, 23, 5);
            i.op = op as u16;
            i.name = name_in(&opcodes::SOPK, op);
            i.dst[0] = Some(Operand::scalar(field(w0, 16, 7) as u16, 1, None));
            i.detail = Detail::Simm16(field(w0, 0, 16) as u16 as i16);
            // S_SETREG_IMM32_B32 carries its value as a literal (section 13.1, SOPK op 21).
            if op == 21 {
                let lit = literal(true, &mut i)?;
                i.src[0] = lit.map(Operand::Literal);
            }
        }
        Encoding::Sop1 => {
            let op = field(w0, 8, 8);
            let (s0, d) = (field(w0, 0, 8), field(w0, 16, 7));
            i.op = op as u16;
            i.name = name_in(&opcodes::SOP1, op);
            let (dw, sw) = i.name.map_or((1, 1), widths);
            let sw = if i.name.is_some_and(|n| n.starts_with("s_bitset")) {
                1
            } else {
                sw
            };
            let lit = literal(s0 == 255, &mut i)?;
            let (has_dst, has_src) = match i.name {
                Some("s_getpc_b64") => (true, false),
                Some("s_setpc_b64" | "s_rfe_b64" | "s_cbranch_join") => (false, true),
                _ => (true, true),
            };
            if has_dst {
                i.dst[0] = Some(Operand::scalar(d as u16, dw, None));
            }
            if has_src {
                i.src[0] = Some(Operand::scalar(s0 as u16, sw, lit));
            }
        }
        Encoding::Sopc => {
            let op = field(w0, 16, 7);
            let (s0, s1) = (field(w0, 0, 8), field(w0, 8, 8));
            i.op = op as u16;
            i.name = name_in(&opcodes::SOPC, op);
            let (_, sw) = i.name.map_or((1, 1), widths);
            let lit = literal(s0 == 255 || s1 == 255, &mut i)?;
            let s1w = if i.name.is_some_and(|n| n.starts_with("s_bitcmp")) {
                1
            } else {
                sw
            };
            i.src[0] = Some(Operand::scalar(s0 as u16, sw, lit));
            i.src[1] = Some(Operand::scalar(s1 as u16, s1w, lit));
        }
        Encoding::Sopp => {
            let op = field(w0, 16, 7);
            i.op = op as u16;
            i.name = name_in(&opcodes::SOPP, op);
            i.detail = Detail::Simm16(field(w0, 0, 16) as u16 as i16);
        }
        Encoding::Smrd => {
            let op = field(w0, 22, 5);
            i.op = op as u16;
            i.name = name_in(&opcodes::SMRD, op);
            let count = match i.name {
                Some(n) if n.ends_with("x16") => 16,
                Some(n) if n.ends_with("x8") => 8,
                Some(n) if n.ends_with("x4") => 4,
                Some(n) if n.ends_with("x2") => 2,
                Some("s_memtime") => 2,
                _ => 1,
            };
            let sdst = field(w0, 15, 7);
            let sbase = field(w0, 9, 6) << 1;
            let imm = bit(w0, 8);
            let offset = field(w0, 0, 8);
            match i.name {
                Some("s_dcache_inv" | "s_dcache_inv_vol") => {}
                Some("s_memtime") => i.dst[0] = Some(Operand::scalar(sdst as u16, 2, None)),
                _ => {
                    i.dst[0] = Some(Operand::scalar(sdst as u16, count, None));
                    let base = if i.name.is_some_and(|n| n.starts_with("s_buffer")) {
                        4
                    } else {
                        2
                    };
                    i.src[0] = Some(Operand::sgpr(sbase, base));
                    // IMM = 1: an 8-bit dword offset; IMM = 0: an SGPR holding a byte offset,
                    // or code 255 for a literal byte offset (table 7.1).
                    i.src[1] = Some(if imm {
                        Operand::Imm(offset)
                    } else if offset == 255 {
                        Operand::Literal(literal(true, &mut i)?.expect("read"))
                    } else {
                        Operand::scalar(offset as u16, 1, None)
                    });
                }
            }
        }
        Encoding::Vop2 if matches!(field(w0, 0, 9), 249 | 250) => extended(&r, w0, &mut i)?,
        Encoding::Vop1 if matches!(field(w0, 0, 9), 249 | 250) => extended(&r, w0, &mut i)?,
        Encoding::Vopc if matches!(field(w0, 0, 9), 249 | 250) => extended(&r, w0, &mut i)?,
        Encoding::Vop3p => decode_vop3p(&r, w0, &mut i)?,
        Encoding::Vop2 => {
            let op = field(w0, 25, 6);
            let (s0, s1, d) = (field(w0, 0, 9), field(w0, 9, 8), field(w0, 17, 8));
            i.op = op as u16;
            i.name = name_in(&opcodes::VOP2, op);
            let (dw, sw) = i.name.map_or((1, 1), widths);
            // V_MADMK_F32 and V_MADAK_F32 always carry the constant K as a literal
            // (section 13.3, VOP2 ops 32 and 33).
            let k = matches!(op, 32 | 33);
            let lit = literal(s0 == 255 || k, &mut i)?;
            i.dst[0] = Some(match op {
                1 => Operand::scalar(d as u16, 1, None),
                _ => Operand::vgpr(d, dw),
            });
            i.src[0] = Some(Operand::scalar(s0 as u16, sw, lit));
            let v1 = Operand::vgpr(s1, sw);
            match op {
                32 => {
                    i.src[1] = lit.map(Operand::Literal);
                    i.src[2] = Some(v1);
                }
                33 => {
                    i.src[1] = Some(v1);
                    i.src[2] = lit.map(Operand::Literal);
                }
                _ => i.src[1] = Some(v1),
            }
            // The carry ops write VCC, and the borrow-in ops and V_CNDMASK_B32 read it.
            if (37..=42).contains(&op) {
                i.dst[1] = Some(Operand::Vcc(Half::Both));
            }
            if matches!(op, 0 | 40..=42) {
                i.src[2] = Some(Operand::Vcc(Half::Both));
            }
        }
        Encoding::Vop1 => {
            let op = field(w0, 9, 8);
            let (s0, d) = (field(w0, 0, 9), field(w0, 17, 8));
            i.op = op as u16;
            i.name = name_in(&opcodes::VOP1, op);
            let (dw, sw) = i.name.map_or((1, 1), widths);
            let lit = literal(s0 == 255, &mut i)?;
            if !matches!(op, 0 | 65) {
                i.dst[0] = Some(match op {
                    2 => Operand::scalar(d as u16, 1, None),
                    _ => Operand::vgpr(d, dw),
                });
                i.src[0] = Some(Operand::scalar(s0 as u16, sw, lit));
            }
        }
        Encoding::Vopc => {
            let op = field(w0, 17, 8);
            let (s0, s1) = (field(w0, 0, 9), field(w0, 9, 8));
            i.op = op as u16;
            i.name = vopc_name(op);
            let (_, sw) = i.name.map_or((1, 1), widths);
            let lit = literal(s0 == 255, &mut i)?;
            i.dst[0] = Some(Operand::Vcc(Half::Both));
            i.src[0] = Some(Operand::scalar(s0 as u16, sw, lit));
            let s1w = if i.name.is_some_and(|n| n.contains("class")) {
                1
            } else {
                sw
            };
            i.src[1] = Some(Operand::vgpr(s1, s1w));
        }
        Encoding::Vop3 => decode_vop3(&r, w0, &mut i)?,
        Encoding::Vintrp => {
            let op = field(w0, 16, 2);
            i.op = op as u16;
            i.name = name_in(&opcodes::VINTRP, op);
            i.dst[0] = Some(Operand::vgpr(field(w0, 18, 8), 1));
            let vsrc = field(w0, 0, 8);
            // V_INTERP_MOV_F32's VSRC selects P10, P20 or P0, not a register.
            i.src[0] = Some(if op == 2 {
                Operand::Imm(vsrc)
            } else {
                Operand::vgpr(vsrc, 1)
            });
            i.detail = Detail::Interp {
                attr: field(w0, 10, 6) as u8,
                chan: field(w0, 8, 2) as u8,
            };
        }
        Encoding::Ds => {
            let w1 = r.word(1)?;
            let op = field(w0, 18, 8);
            i.op = op as u16;
            i.name = name_in(&opcodes::DS, op);
            i.dst[0] = Some(Operand::vgpr(field(w1, 24, 8), 1));
            i.src[0] = Some(Operand::vgpr(field(w1, 0, 8), 1));
            i.src[1] = Some(Operand::vgpr(field(w1, 8, 8), 1));
            i.src[2] = Some(Operand::vgpr(field(w1, 16, 8), 1));
            i.detail = Detail::Ds {
                offset0: field(w0, 0, 8) as u8,
                offset1: field(w0, 8, 8) as u8,
                gds: bit(w0, 17),
            };
        }
        Encoding::Mubuf | Encoding::Mtbuf => {
            let w1 = r.word(1)?;
            let mtbuf = encoding == Encoding::Mtbuf;
            let op = if mtbuf {
                field(w0, 16, 3)
            } else {
                field(w0, 18, 7)
            };
            i.op = op as u16;
            i.name = if mtbuf {
                name_in(&opcodes::MTBUF, op)
            } else {
                name_in(&opcodes::MUBUF, op)
            };
            let data = match i.name {
                Some(n) if n.ends_with("_xyzw") || n.ends_with("x4") => 4,
                Some(n) if n.ends_with("_xyz") || n.ends_with("x3") => 3,
                Some(n) if n.ends_with("_xy") || n.ends_with("x2") => 2,
                _ => 1,
            };
            let (offen, idxen, addr64) = (bit(w0, 12), bit(w0, 13), bit(w0, 15));
            let addr = if (offen && idxen) || addr64 { 2 } else { 1 };
            i.dst[0] = Some(Operand::vgpr(field(w1, 8, 8), data));
            i.src[0] = Some(Operand::vgpr(field(w1, 0, 8), addr));
            i.src[1] = Some(Operand::sgpr(field(w1, 16, 5) << 2, 4));
            i.src[2] = Some(Operand::scalar(field(w1, 24, 8) as u16, 1, None));
            i.detail = Detail::Buffer {
                offset: field(w0, 0, 12) as u16,
                offen,
                idxen,
                glc: bit(w0, 14),
                addr64,
                lds: !mtbuf && bit(w0, 16),
                slc: bit(w1, 22),
                tfe: bit(w1, 23),
                dfmt: if mtbuf { field(w0, 19, 4) as u8 } else { 0 },
                nfmt: if mtbuf { field(w0, 23, 3) as u8 } else { 0 },
            };
        }
        Encoding::Mimg => {
            let w1 = r.word(1)?;
            let op = field(w0, 18, 7);
            i.op = op as u16;
            i.name = name_in(&opcodes::MIMG, op);
            let dmask = field(w0, 8, 4) as u8;
            let r128 = bit(w0, 15);
            let gather = i.name.is_some_and(|n| n.starts_with("image_gather4"));
            let data = if gather {
                4
            } else {
                dmask.count_ones().max(1) as u8
            };
            i.dst[0] = Some(Operand::vgpr(field(w1, 8, 8), data));
            i.src[0] = Some(Operand::vgpr(field(w1, 0, 8), 1));
            i.src[1] = Some(Operand::sgpr(
                field(w1, 16, 5) << 2,
                if r128 { 4 } else { 8 },
            ));
            if i.name.is_some_and(|n| {
                n.contains("sample") || n.contains("gather4") || n.contains("get_lod")
            }) {
                i.src[2] = Some(Operand::sgpr(field(w1, 21, 5) << 2, 4));
            }
            i.detail = Detail::Image {
                dmask,
                unorm: bit(w0, 12),
                glc: bit(w0, 13),
                da: bit(w0, 14),
                r128,
                tfe: bit(w0, 16),
                lwe: bit(w0, 17),
                slc: bit(w0, 25),
            };
        }
        Encoding::Exp => {
            let w1 = r.word(1)?;
            i.name = Some("exp");
            let en = field(w0, 0, 4) as u8;
            for k in 0..4 {
                if en >> k & 1 != 0 {
                    i.src[k] = Some(Operand::vgpr(field(w1, 8 * k as u32, 8), 1));
                }
            }
            i.detail = Detail::Export {
                target: field(w0, 4, 6) as u8,
                en,
                compr: bit(w0, 10),
                done: bit(w0, 11),
                vm: bit(w0, 12),
            };
        }
        Encoding::Flat => {
            let w1 = r.word(1)?;
            let op = field(w0, 18, 7);
            i.op = op as u16;
            i.name = name_in(&opcodes::FLAT, op);
            i.dst[0] = Some(Operand::vgpr(field(w1, 24, 8), 1));
            i.src[0] = Some(Operand::vgpr(field(w1, 0, 8), 2));
            i.src[1] = Some(Operand::vgpr(field(w1, 8, 8), 1));
            i.detail = Detail::Flat {
                glc: bit(w0, 16),
                slc: bit(w0, 17),
                tfe: bit(w1, 23),
            };
        }
    }
    Ok(i)
}

/// VOP3a and VOP3b (section 13.3): `OP` `[25:17]`, `VDST` `[7:0]`; VOP3a has `ABS` `[10:8]` and
/// `CLAMP` 11, VOP3b an `SDST` `[14:8]` in their place. The second word holds `SRC0` `[8:0]`,
/// `SRC1` `[17:9]`, `SRC2` `[26:18]`, `OMOD` `[28:27]` and `NEG` `[31:29]`.
fn decode_vop3(r: &Reader<'_>, w0: u32, i: &mut Instruction) -> Result<(), DecodeError> {
    let w1 = r.word(1)?;
    let op = field(w0, 17, 9);
    i.op = op as u16;
    i.name = vop3_name(op);
    let codes = [field(w1, 0, 9), field(w1, 9, 9), field(w1, 18, 9)];
    // VOP3b: the carry ops, V_DIV_SCALE and the 64-bit multiply-adds write an SGPR pair.
    let vop3b = matches!(op, 293..=298 | 365 | 366 | 374 | 375);
    let sources = match op {
        0..=255 => 2,
        256 | 296..=298 => 3,
        // V_MAC_F32 and V_MAC_LEGACY_F32 accumulate into VDST and read two sources.
        256..=319 => 2,
        353..=364 | 372 => 2,
        320..=383 => 3,
        384 | 449 => 0,
        _ => 1,
    };
    let (dw, sw) = i.name.map_or((1, 1), widths);
    for (k, &code) in codes.iter().enumerate().take(sources) {
        if code == 255 {
            return Err(DecodeError::LiteralInVop3 { at: r.at });
        }
        let count = match (k, i.name) {
            (1, Some(n)) if narrow_src1(n) || n.contains("class") => 1,
            // V_CNDMASK_B32's mask and the borrow-in ops' carry are SGPR pairs.
            (2, _) if matches!(op, 256 | 296..=298) => 2,
            (2, Some("v_mad_u64_u32" | "v_mad_i64_i32")) => 2,
            (_, Some("v_mad_u64_u32" | "v_mad_i64_i32")) => 1,
            _ => sw,
        };
        i.src[k] = Some(Operand::scalar(code as u16, count, None));
    }
    let vdst = field(w0, 0, 8);
    if sources == 0 {
        return Ok(());
    }
    i.dst[0] = Some(match op {
        0..=255 => Operand::scalar(vdst as u16, 2, None),
        257 | 386 => Operand::scalar(vdst as u16, 1, None),
        _ => Operand::vgpr(vdst, dw),
    });
    if vop3b {
        i.dst[1] = Some(Operand::scalar(field(w0, 8, 7) as u16, 2, None));
    } else {
        i.abs = field(w0, 8, 3) as u8;
        i.clamp = bit(w0, 11);
    }
    i.omod = field(w1, 27, 2) as u8;
    i.neg = field(w1, 29, 3) as u8;
    Ok(())
}

/// A VOP1, VOP2 or VOPC whose `SRC0` is 249 (SDWA) or 250 (DPP): the real operands are in a
/// second dword (Vega 13.3.7 to 13.3.9). Registers are one dword wide; neither form is used
/// with 64-bit operands.
fn extended(r: &Reader<'_>, w0: u32, i: &mut Instruction) -> Result<(), DecodeError> {
    let w1 = r.word(1)?;
    i.size = 8;
    let sdwa = field(w0, 0, 9) == 249;
    let vsrc1 = field(w0, 9, 8);
    let (op, name, dst, src1) = match i.encoding {
        Encoding::Vop2 => {
            let op = field(w0, 25, 6);
            (
                op,
                name_in(&opcodes::VOP2, op),
                Some(Operand::vgpr(field(w0, 17, 8), 1)),
                true,
            )
        }
        Encoding::Vop1 => {
            let op = field(w0, 9, 8);
            (
                op,
                name_in(&opcodes::VOP1, op),
                Some(Operand::vgpr(field(w0, 17, 8), 1)),
                false,
            )
        }
        _ => {
            let op = field(w0, 17, 8);
            // SDWAB: `SD` 47 picks an SGPR pair at `SDST` [46:40] over VCC.
            let dst = if sdwa && bit(w1, 15) {
                Operand::scalar(field(w1, 8, 7) as u16, 2, None)
            } else {
                Operand::Vcc(Half::Both)
            };
            (op, vopc_name(op), Some(dst), true)
        }
    };
    i.op = op as u16;
    i.name = name;
    i.dst[0] = dst;
    let src0 = field(w1, 0, 8);
    if sdwa {
        i.src[0] = Some(if bit(w1, 23) {
            Operand::scalar(src0 as u16, 1, None)
        } else {
            Operand::vgpr(src0, 1)
        });
        if src1 {
            i.src[1] = Some(if bit(w1, 31) {
                Operand::scalar(vsrc1 as u16, 1, None)
            } else {
                Operand::vgpr(vsrc1, 1)
            });
        }
        i.neg = u8::from(bit(w1, 20)) | u8::from(bit(w1, 28)) << 1;
        i.abs = u8::from(bit(w1, 21)) | u8::from(bit(w1, 29)) << 1;
        let vopc = i.encoding == Encoding::Vopc;
        if !vopc {
            i.clamp = bit(w1, 13);
            i.omod = field(w1, 14, 2) as u8;
        }
        i.detail = Detail::Sdwa {
            dst_sel: if vopc { 6 } else { field(w1, 8, 3) as u8 },
            dst_unused: if vopc { 0 } else { field(w1, 11, 2) as u8 },
            src0_sel: field(w1, 16, 3) as u8,
            src1_sel: field(w1, 24, 3) as u8,
        };
    } else {
        i.src[0] = Some(Operand::vgpr(src0, 1));
        if src1 {
            i.src[1] = Some(Operand::vgpr(vsrc1, 1));
        }
        i.neg = u8::from(bit(w1, 20)) | u8::from(bit(w1, 22)) << 1;
        i.abs = u8::from(bit(w1, 21)) | u8::from(bit(w1, 23)) << 1;
        i.detail = Detail::Dpp {
            ctrl: field(w1, 8, 9) as u16,
            bound_ctrl: bit(w1, 19),
            bank_mask: field(w1, 24, 4) as u8,
            row_mask: field(w1, 28, 4) as u8,
        };
    }
    if i.encoding == Encoding::Vop2 && matches!(op, 0 | 40..=42) {
        i.src[2] = Some(Operand::Vcc(Half::Both));
    }
    Ok(())
}

/// VOP3P names by number (Vega section 12.10, "VOP3P Instructions").
const VOP3P: [(u32, &str); 22] = [
    (0, "v_pk_mad_i16"),
    (1, "v_pk_mul_lo_u16"),
    (2, "v_pk_add_i16"),
    (3, "v_pk_sub_i16"),
    (4, "v_pk_lshlrev_b16"),
    (5, "v_pk_lshrrev_b16"),
    (6, "v_pk_ashrrev_i16"),
    (7, "v_pk_max_i16"),
    (8, "v_pk_min_i16"),
    (9, "v_pk_mad_u16"),
    (10, "v_pk_add_u16"),
    (11, "v_pk_sub_u16"),
    (12, "v_pk_max_u16"),
    (13, "v_pk_min_u16"),
    (14, "v_pk_fma_f16"),
    (15, "v_pk_add_f16"),
    (16, "v_pk_mul_f16"),
    (17, "v_pk_min_f16"),
    (18, "v_pk_max_f16"),
    (32, "v_mad_mix_f32"),
    (33, "v_mad_mixlo_f16"),
    (34, "v_mad_mixhi_f16"),
];

/// VOP3P (Vega 13.3.6): `VDST` [7:0], `NEG_HI` [10:8], `OPSEL` [13:11], `OPSEL_HI2` 14, `CLMP`
/// 15, `OP` [22:16]; second word as VOP3's, with `OPSEL_HI` [60:59] for sources 0 and 1 where
/// VOP3 has `OMOD`.
fn decode_vop3p(r: &Reader<'_>, w0: u32, i: &mut Instruction) -> Result<(), DecodeError> {
    let w1 = r.word(1)?;
    let op = field(w0, 16, 7);
    i.op = op as u16;
    i.name = VOP3P.iter().find(|(n, _)| *n == op).map(|(_, name)| *name);
    let sources = if matches!(op, 0 | 9 | 14 | 32..=34) {
        3
    } else {
        2
    };
    for k in 0..sources {
        let code = field(w1, 9 * k as u32, 9);
        if code == 255 {
            return Err(DecodeError::LiteralInVop3 { at: r.at });
        }
        i.src[k] = Some(Operand::scalar(code as u16, 1, None));
    }
    i.dst[0] = Some(Operand::vgpr(field(w0, 0, 8), 1));
    i.clamp = bit(w0, 15);
    i.neg = field(w1, 29, 3) as u8;
    i.detail = Detail::Packed {
        op_sel: field(w0, 11, 3) as u8,
        op_sel_hi: field(w1, 27, 2) as u8 | (field(w0, 14, 1) as u8) << 2,
        neg_hi: field(w0, 8, 3) as u8,
    };
    Ok(())
}

/// Decodes `code` from its first byte to its last, every instruction in turn.
///
/// # Errors
///
/// The first [`DecodeError`]; nothing past it is in step.
pub fn decode_all(code: &[u8]) -> Result<Vec<Instruction>, DecodeError> {
    let mut out = Vec::with_capacity(code.len() / 6);
    let mut at = 0;
    while at < code.len() {
        let i = decode(code, at)?;
        at += usize::from(i.size);
        out.push(i);
    }
    Ok(out)
}

impl Instruction {
    /// The literal this instruction carries, if any.
    #[must_use]
    pub fn literal(&self) -> Option<u32> {
        self.src.iter().flatten().find_map(|s| match s {
            Operand::Literal(v) => Some(*v),
            _ => None,
        })
    }

    /// Whether this is `s_endpgm`.
    #[must_use]
    pub fn is_end(&self) -> bool {
        self.encoding == Encoding::Sopp && self.op == 1
    }

    /// A branch's target byte offset (SOPP branches: `PC + 4 + SIMM16 * 4`, section 4.4).
    #[must_use]
    pub fn branch_target(&self) -> Option<usize> {
        let Detail::Simm16(s) = self.detail else {
            return None;
        };
        (self.encoding == Encoding::Sopp && matches!(self.op, 2 | 4..=9 | 23..=26))
            .then(|| (self.at as i64 + 4 + i64::from(s) * 4) as usize)
    }
}

fn source(f: &mut fmt::Formatter<'_>, op: Operand, neg: bool, abs: bool) -> fmt::Result {
    match (neg, abs) {
        (false, false) => write!(f, "{op}"),
        (true, false) => write!(f, "-{op}"),
        (false, true) => write!(f, "|{op}|"),
        (true, true) => write!(f, "-|{op}|"),
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(n) => f.write_str(n)?,
            None => write!(f, "<{} op {}>", self.encoding.name(), self.op)?,
        }
        match self.detail {
            Detail::Sdwa { .. } => f.write_str("_sdwa")?,
            Detail::Dpp { .. } => f.write_str("_dpp")?,
            _ => {}
        }
        if let Detail::Export { target, .. } = self.detail {
            match target {
                0..=7 => write!(f, " mrt{target}")?,
                8 => f.write_str(" mrtz")?,
                9 => f.write_str(" null")?,
                12..=15 => write!(f, " pos{}", target - 12)?,
                32..=63 => write!(f, " param{}", target - 32)?,
                _ => write!(f, " tgt{target}")?,
            }
            for (k, s) in self.src.iter().enumerate() {
                f.write_str(if k == 0 { " " } else { ", " })?;
                match s {
                    Some(s) => write!(f, "{s}")?,
                    None => f.write_str("off")?,
                }
            }
        } else {
            let mut first = true;
            for op in self.dst.iter().flatten() {
                f.write_str(if first { " " } else { ", " })?;
                first = false;
                write!(f, "{op}")?;
            }
            for (k, op) in self.src.iter().enumerate() {
                let Some(op) = *op else { continue };
                f.write_str(if first { " " } else { ", " })?;
                first = false;
                source(f, op, self.neg >> k & 1 != 0, self.abs >> k & 1 != 0)?;
            }
        }
        match self.detail {
            Detail::None | Detail::Export { .. } => {}
            Detail::Simm16(s) => match self.branch_target() {
                Some(t) => write!(f, " {t:#x}")?,
                // S_WAITCNT: VM_CNT [3:0], EXP_CNT [6:4], LGKM_CNT [12:8] (section 13.1, SOPP op 12).
                None if self.encoding == Encoding::Sopp && self.op == 12 => {
                    let s = s as u16;
                    let (vm, exp, lgkm) = (s & 0xf, (s >> 4) & 7, (s >> 8) & 0x1f);
                    let mut any = false;
                    for (name, v, max) in [
                        ("vmcnt", vm, 0xf),
                        ("expcnt", exp, 7),
                        ("lgkmcnt", lgkm, 0x1f),
                    ] {
                        if v != max {
                            write!(f, " {name}({v})")?;
                            any = true;
                        }
                    }
                    if !any {
                        write!(f, " {s:#x}")?;
                    }
                }
                None if self.encoding == Encoding::Sopp && s == 0 => {}
                None => write!(f, " {:#x}", s as u16)?,
            },
            Detail::Interp { attr, chan } => {
                write!(
                    f,
                    ", attr{attr}.{}",
                    ["x", "y", "z", "w"][usize::from(chan)]
                )?;
            }
            Detail::Ds {
                offset0,
                offset1,
                gds,
            } => {
                write!(f, " offset0:{offset0} offset1:{offset1}")?;
                if gds {
                    f.write_str(" gds")?;
                }
            }
            Detail::Buffer {
                offset,
                offen,
                idxen,
                glc,
                addr64,
                lds,
                slc,
                tfe,
                dfmt,
                nfmt,
            } => {
                if self.encoding == Encoding::Mtbuf {
                    write!(f, " format:[{dfmt},{nfmt}]")?;
                }
                for (on, flag) in [
                    (offen, "offen"),
                    (idxen, "idxen"),
                    (addr64, "addr64"),
                    (glc, "glc"),
                    (slc, "slc"),
                    (lds, "lds"),
                    (tfe, "tfe"),
                ] {
                    if on {
                        write!(f, " {flag}")?;
                    }
                }
                if offset != 0 {
                    write!(f, " offset:{offset}")?;
                }
            }
            Detail::Image {
                dmask,
                unorm,
                glc,
                da,
                r128,
                tfe,
                lwe,
                slc,
            } => {
                write!(f, " dmask:{dmask:#x}")?;
                for (on, flag) in [
                    (unorm, "unorm"),
                    (glc, "glc"),
                    (da, "da"),
                    (r128, "r128"),
                    (tfe, "tfe"),
                    (lwe, "lwe"),
                    (slc, "slc"),
                ] {
                    if on {
                        write!(f, " {flag}")?;
                    }
                }
            }
            Detail::Packed {
                op_sel,
                op_sel_hi,
                neg_hi,
            } => write!(
                f,
                " op_sel:{op_sel:#05b} op_sel_hi:{op_sel_hi:#05b} neg_hi:{neg_hi:#05b}"
            )?,
            Detail::Sdwa {
                dst_sel,
                dst_unused,
                src0_sel,
                src1_sel,
            } => {
                const SEL: [&str; 8] = [
                    "BYTE_0", "BYTE_1", "BYTE_2", "BYTE_3", "WORD_0", "WORD_1", "DWORD", "sel7",
                ];
                write!(
                    f,
                    " dst_sel:{} dst_unused:{dst_unused} src0_sel:{} src1_sel:{}",
                    SEL[usize::from(dst_sel)],
                    SEL[usize::from(src0_sel)],
                    SEL[usize::from(src1_sel)]
                )?;
            }
            Detail::Dpp {
                ctrl,
                bound_ctrl,
                bank_mask,
                row_mask,
            } => {
                write!(
                    f,
                    " dpp_ctrl:{ctrl:#x} row_mask:{row_mask:#x} bank_mask:{bank_mask:#x}"
                )?;
                if bound_ctrl {
                    f.write_str(" bound_ctrl")?;
                }
            }
            Detail::Flat { glc, slc, tfe } => {
                for (on, flag) in [(glc, "glc"), (slc, "slc"), (tfe, "tfe")] {
                    if on {
                        write!(f, " {flag}")?;
                    }
                }
            }
        }
        if let Detail::Export {
            compr, done, vm, ..
        } = self.detail
        {
            for (on, flag) in [(compr, "compr"), (done, "done"), (vm, "vm")] {
                if on {
                    write!(f, " {flag}")?;
                }
            }
        }
        if self.clamp {
            f.write_str(" clamp")?;
        }
        match self.omod {
            1 => f.write_str(" mul:2")?,
            2 => f.write_str(" mul:4")?,
            3 => f.write_str(" div:2")?,
            _ => {}
        }
        Ok(())
    }
}
