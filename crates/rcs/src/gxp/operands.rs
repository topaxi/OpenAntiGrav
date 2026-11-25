//! The USSE operand and header-table reading the 2048 program matchers share
//! ([`super::electricity`], [`super::cannon_bolt`]): register banks and words, the `smp`
//! operands, and the literal, dependent-sampler and fragment-input tables. The layouts are the
//! ones `scripts/vita-usse.py` lists (`docs/formats/gxp.md`).

use super::usse;

/// The container literals and dependent-sampler descriptors live in.
const SECONDARY_CONTAINER: u16 = 19;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bank {
    Temp,
    Output,
    Primary,
    Secondary,
    Other,
}

/// One 32-bit register word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Word(pub Bank, pub u32);

pub(super) fn field(word: u64, lo: u32, n: u32) -> u32 {
    ((word >> lo) & ((1 << n) - 1)) as u32
}

/// A source 1 or 2 register's bank: `r o pa sa` without the extension bit.
pub(super) fn src12_bank(sel: u32, ext: u32) -> Bank {
    match (ext, sel) {
        (0, 0) => Bank::Temp,
        (0, 1) => Bank::Output,
        (0, 2) => Bank::Primary,
        (0, 3) => Bank::Secondary,
        _ => Bank::Other,
    }
}

pub(super) fn src0_bank(sel: u32) -> Bank {
    if sel == 0 { Bank::Temp } else { Bank::Primary }
}

pub(super) fn dst_bank(sel: u32, ext: u32) -> Bank {
    match (ext, sel) {
        (0, 0) => Bank::Temp,
        (0, 1) => Bank::Output,
        (0, 2) => Bank::Primary,
        (1, 0) => Bank::Secondary,
        _ => Bank::Other,
    }
}

/// A register number as a word index: the fields count register pairs.
pub(super) fn doubled(bank: Bank, num: u32) -> Option<u32> {
    let n = (num << 1) & 0xff;
    // The top eight doubled temporaries are the internal registers.
    (bank != Bank::Temp || n < 120).then_some(n)
}

/// An `smp`: the first result word, the first coordinate word and the sampler's `sa` word.
pub(super) fn sample(word: u64) -> Option<(Word, Word, u32)> {
    if usse::classify(word) != Some(usse::Class::Smp) {
        return None;
    }
    let dst_bank = if field(word, 39, 1) == 1 {
        Bank::Primary
    } else {
        Bank::Temp
    };
    let coord_bank = if field(word, 50, 1) == 1 {
        Bank::Other
    } else {
        src0_bank(field(word, 34, 1))
    };
    let state_bank = src12_bank(field(word, 30, 2), field(word, 49, 1));
    (state_bank == Bank::Secondary).then_some(())?;
    Some((
        Word(dst_bank, field(word, 21, 7)),
        Word(coord_bank, doubled(coord_bank, field(word, 14, 7))?),
        doubled(state_bank, field(word, 7, 7))?,
    ))
}

/// The raw tables the header points at, read only as far as this module needs.
pub(super) struct Tables {
    /// `(index, bits)` literal records, in container 19.
    pub literals: Vec<(u32, u32)>,
    /// `(resource, sa word)` per dependent-sampler record.
    pub samplers: Vec<(u32, u32)>,
    /// Container 19's first `sa` word.
    pub base: u32,
    /// The fragment inputs fetched before the program runs.
    pub texels: Vec<Texel>,
}

/// One prefetched texel of the fragment input records.
pub(super) struct Texel {
    /// The first primary register word it lands in.
    pub word: u32,
    /// The sampler's resource.
    pub resource: u32,
    /// The interpolant that addresses it.
    pub interpolant: u32,
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn rel(b: &[u8], at: usize) -> Option<usize> {
    Some(at + le32(b, at)? as usize)
}

pub(super) fn tables(b: &[u8]) -> Option<Tables> {
    let literal_at = rel(b, 0x74)?;
    let literals = (0..le32(b, 0x70)? as usize)
        .map(|k| {
            Some((
                le32(b, literal_at + 8 * k)?,
                le32(b, literal_at + 8 * k + 4)?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    let dep_at = rel(b, 0x84)?;
    let samplers = (0..le32(b, 0x80)? as usize)
        .map(|k| {
            let at = dep_at + 4 * k;
            Some((u32::from(le16(b, at)? / 4), u32::from(le16(b, at + 2)?)))
        })
        .collect::<Option<Vec<_>>>()?;
    let container_at = rel(b, 0x94)?;
    let base = (0..le32(b, 0x90)? as usize).find_map(|k| {
        let at = container_at + 8 * k;
        (le16(b, at)? == SECONDARY_CONTAINER).then(|| le16(b, at + 4).map(u32::from))?
    })?;
    // The fragment input records: `+0x2c` points at a block whose `+0x0c` counts 16-byte records
    // at its `+0x10` (`scripts/vita-gxp.py`, `fragment_inputs`).
    let block = rel(b, 0x2c)?;
    let count = le16(b, block + 0x0c)? as usize;
    let table = rel(b, block + 0x10)?;
    let mut texels = Vec::new();
    let mut at = 0;
    for k in 0..count {
        let info = le32(b, table + 16 * k)?;
        let resource = le32(b, table + 16 * k + 4)?;
        let size = le32(b, table + 16 * k + 8)?;
        if (info >> 12) & 0xf != 0xf {
            at += ((size >> 4) & 3) + 1;
        }
        if info & 0xf != 0xf {
            texels.push(Texel {
                word: at,
                resource,
                interpolant: info & 0xf,
            });
            at += ((size >> 6) & 3) + 1;
        }
    }
    Some(Tables {
        literals,
        samplers,
        base,
        texels,
    })
}
