//! The class of a USSE instruction word.
//!
//! Every shader program on the Vita runs PowerVR SGX543 USSE code, one 64-bit
//! little-endian word per instruction. [`classify`] reads the fixed bits of a
//! word and names the instruction class it belongs to, or refuses it with
//! `None`. It does not read operands: those are read by
//! `scripts/vita-usse.py`, the reference, which lists the classes marked by
//! [`Class::operands_read`] and counts the rest by class.
//!
//! The 35 classes are ordered: a word belongs to the first layout whose fixed
//! bits it carries, because the short-opcode classes are prefixes of the
//! long ones. `docs/formats/gxp.md` ("The instruction stream") records how
//! each was established and what it survived.

use super::Region;

/// An instruction class. The names are ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    /// `mad2`.
    Mad2,
    /// `nmad32`.
    Nmad32,
    /// `nmad16`.
    Nmad16,
    /// `mad`.
    Mad,
    /// `dp`.
    Dp,
    /// `dual`.
    Dual,
    /// `comp`.
    Comp,
    /// `mov`.
    Mov,
    /// `pck`.
    Pck,
    /// `tst`.
    Tst,
    /// `tstmsk`.
    Tstmsk,
    /// `bw`.
    Bw,
    /// `sop2`.
    Sop2,
    /// `sop2m`.
    Sop2m,
    /// `sop3`.
    Sop3,
    /// `imad8`.
    Imad8,
    /// `imad16`.
    Imad16,
    /// `imad32`.
    Imad32,
    /// `illegal22`.
    Illegal22,
    /// `illegal23`.
    Illegal23,
    /// `illegal24`.
    Illegal24,
    /// `imad8x2`.
    Imad8x2,
    /// `imad32x2`.
    Imad32x2,
    /// `illegal27`.
    Illegal27,
    /// `smp`.
    Smp,
    /// `phase`.
    Phase,
    /// `nop`.
    Nop,
    /// `branch`.
    Branch,
    /// `slsi`.
    Slsi,
    /// `sbo`.
    Sbo,
    /// `kill`.
    Kill,
    /// `limm`.
    Limm,
    /// `depthf`.
    Depthf,
    /// `spec`.
    Spec,
    /// `ldst`.
    Ldst,
}

/// One entry per class, in match order: the bits a word must carry
/// (`mask`) and the values they must have (`value`).
const TABLE: [(Class, u64, u64); 35] = [
    (Class::Mad2, 0xf800000000000000, 0x0000000000000000),
    (Class::Nmad32, 0xf800000000000000, 0x0800000000000000),
    (Class::Nmad16, 0xf800000000000000, 0x1000000000000000),
    (Class::Mad, 0xf820000000000000, 0x1820000000000000),
    (Class::Dp, 0xf820000000000000, 0x1800000000000000),
    (Class::Dual, 0xf000000000000000, 0x2000000000000000),
    (Class::Comp, 0xf800000000000000, 0x3000000000000000),
    (Class::Mov, 0xf800000000000000, 0x3800000000000000),
    (Class::Pck, 0xf800000000000000, 0x4000000000000000),
    (Class::Tst, 0xf800000000000000, 0x4800000000000000),
    (Class::Tstmsk, 0xf800000000000000, 0x7800000000000000),
    (Class::Bw, 0xc000000000000000, 0x4000000000000000),
    (Class::Sop2, 0xf800000000000000, 0x8000000000000000),
    (Class::Sop2m, 0xf800000000000000, 0x9000000000000000),
    (Class::Sop3, 0xf800000000000000, 0x8800000000000000),
    (Class::Imad8, 0xf800000000000000, 0x9800000000000000),
    (Class::Imad16, 0xf800000000000000, 0xa000000000000000),
    (Class::Imad32, 0xf800833800000000, 0xa800000000000000),
    (Class::Illegal22, 0xf800000000000000, 0xb000000000000000),
    (Class::Illegal23, 0xf800000000000000, 0xb800000000000000),
    (Class::Illegal24, 0xf800000000000000, 0xc000000000000000),
    (Class::Imad8x2, 0xf800000000000000, 0xc800000000000000),
    (Class::Imad32x2, 0xf8000c7800000000, 0xd000000000000000),
    (Class::Illegal27, 0xf800000000000000, 0xd800000000000000),
    (Class::Smp, 0xf800000000000000, 0xe000000000000000),
    (Class::Phase, 0xff70000000000000, 0xfa40000000000000),
    (Class::Nop, 0xf87001c000000000, 0xf800014000000000),
    (Class::Branch, 0xf870018000000000, 0xf800000000000000),
    (Class::Slsi, 0xff30000000000000, 0xfa10000000000000),
    (Class::Sbo, 0xff30000000000000, 0xfb10000000000000),
    (Class::Kill, 0xff3ff9fff0000000, 0xf9300006f0000000),
    (Class::Limm, 0xff30000000000000, 0xfc20000000000000),
    (Class::Depthf, 0xff30000000000000, 0xfb30000000000000),
    (Class::Spec, 0xf800000000000000, 0xf800000000000000),
    (Class::Ldst, 0xe000000000000000, 0xe000000000000000),
];

impl Class {
    /// Every class, in match order.
    pub const ALL: [Class; 35] = [
        Class::Mad2,
        Class::Nmad32,
        Class::Nmad16,
        Class::Mad,
        Class::Dp,
        Class::Dual,
        Class::Comp,
        Class::Mov,
        Class::Pck,
        Class::Tst,
        Class::Tstmsk,
        Class::Bw,
        Class::Sop2,
        Class::Sop2m,
        Class::Sop3,
        Class::Imad8,
        Class::Imad16,
        Class::Imad32,
        Class::Illegal22,
        Class::Illegal23,
        Class::Illegal24,
        Class::Imad8x2,
        Class::Imad32x2,
        Class::Illegal27,
        Class::Smp,
        Class::Phase,
        Class::Nop,
        Class::Branch,
        Class::Slsi,
        Class::Sbo,
        Class::Kill,
        Class::Limm,
        Class::Depthf,
        Class::Spec,
        Class::Ldst,
    ];

    /// The name the listing and the docs use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mad2 => "mad2",
            Self::Nmad32 => "nmad32",
            Self::Nmad16 => "nmad16",
            Self::Mad => "mad",
            Self::Dp => "dp",
            Self::Dual => "dual",
            Self::Comp => "comp",
            Self::Mov => "mov",
            Self::Pck => "pck",
            Self::Tst => "tst",
            Self::Tstmsk => "tstmsk",
            Self::Bw => "bw",
            Self::Sop2 => "sop2",
            Self::Sop2m => "sop2m",
            Self::Sop3 => "sop3",
            Self::Imad8 => "imad8",
            Self::Imad16 => "imad16",
            Self::Imad32 => "imad32",
            Self::Illegal22 => "illegal22",
            Self::Illegal23 => "illegal23",
            Self::Illegal24 => "illegal24",
            Self::Imad8x2 => "imad8x2",
            Self::Imad32x2 => "imad32x2",
            Self::Illegal27 => "illegal27",
            Self::Smp => "smp",
            Self::Phase => "phase",
            Self::Nop => "nop",
            Self::Branch => "branch",
            Self::Slsi => "slsi",
            Self::Sbo => "sbo",
            Self::Kill => "kill",
            Self::Limm => "limm",
            Self::Depthf => "depthf",
            Self::Spec => "spec",
            Self::Ldst => "ldst",
        }
    }

    /// Whether `scripts/vita-usse.py` reads this class's operands. The rest
    /// are named and counted, never guessed past.
    #[must_use]
    pub const fn operands_read(self) -> bool {
        matches!(
            self,
            Self::Mad2
                | Self::Nmad32
                | Self::Nmad16
                | Self::Mad
                | Self::Dp
                | Self::Comp
                | Self::Mov
                | Self::Pck
                | Self::Bw
                | Self::Imad16
                | Self::Imad32
                | Self::Imad32x2
                | Self::Tst
                | Self::Smp
                | Self::Phase
                | Self::Nop
                | Self::Branch
                | Self::Slsi
                | Self::Kill
                | Self::Depthf
                | Self::Spec
        )
    }
}

/// The class of `word`, or `None` when no layout matches.
#[must_use]
pub fn classify(word: u64) -> Option<Class> {
    TABLE
        .iter()
        .find(|(_, mask, value)| word & mask == *value)
        .map(|(class, _, _)| *class)
}

/// The instruction words of one code block, in order.
///
/// `program` is the bytes [`super::Program::parse`] consumed. A trailing
/// partial word cannot occur on a program that parsed, since the header's
/// counts close on 8-byte instructions.
pub fn words<'a>(program: &'a [u8], block: &Region) -> impl Iterator<Item = u64> + 'a {
    let (words, _) = program[block.start..block.end].as_chunks::<8>();
    words.iter().map(|w| u64::from_le_bytes(*w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_ordered_like_all_and_its_values_sit_inside_their_masks() {
        for (i, (class, mask, value)) in TABLE.iter().enumerate() {
            assert_eq!(Class::ALL[i], *class);
            assert_ne!(*mask, 0, "{class:?}");
            assert_eq!(value & !mask, 0, "{class:?}");
        }
        let mut names: Vec<_> = Class::ALL.iter().map(|c| c.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TABLE.len());
    }

    #[test]
    fn known_words_classify() {
        // Words from the Canvas3D vertex program (docs/formats/gxp.md).
        assert_eq!(classify(0xfa44_0700_0000_0000), Some(Class::Phase));
        assert_eq!(classify(0x08a4_4185_e084_1204), Some(Class::Nmad32));
        assert_eq!(classify(0xe084_c600_f080_97b9), Some(Class::Smp));
        assert_eq!(classify(0x4081_0d45_2000_0100), Some(Class::Pck));
        assert_eq!(classify(0x6081_0006_a0c0_0301), Some(Class::Bw));
        assert_eq!(classify(0x0080_2086_f010_6db5), Some(Class::Mad2));
        assert_eq!(classify(0xa08b_0946_a022_0088), Some(Class::Imad16));
        assert_eq!(classify(0x1084_0000_0000_0000), Some(Class::Nmad16));
        assert_eq!(classify(0xfb27_5000_a020_0000), Some(Class::Spec));
        assert_eq!(classify(0xf804_0140_0000_0000), Some(Class::Nop));
    }
}
