//! 2048's `electricity.rcsmaterial` fragment programs: the Detonator mine's visualiser ring.
//!
//! The law is HD's (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md`, "The visualiser,
//! wired", the 2048 line): two taps of `Bass_Treble_Hits`, one of `Pitch`, two of `Electricity`,
//! the last at `uv1.y + 0.3 m`. On the Vita the two bass and treble taps are **prefetched**: the
//! fragment input records name them as texels of that sampler at interpolants 2 and 3, so the
//! code itself holds only three `smp` words. And the program squares three of its taps itself
//! (the bass tap's red, the pitch tap's red, the colour tap's rgb), where HD leaves that to the
//! texture unit.
//!
//! What is read, by its bits (the operand layouts `scripts/vita-usse.py` lists, narrowed to the
//! classes the question needs):
//!
//! - the sampler of each `smp`, through the dependent-sampler table at `+0x84`: four records per
//!   sampler, `resource * 4 + k` in the low half and the `sa` word in the high half, relative to
//!   container 19's base (on 2,577 of 2,577 fragment programs of `data.psarc`'s materials that
//!   carry the table, every record names a declared sampler);
//! - the shift: the colour tap's `v` is written by an `add.f32` of the negated `time` to a word
//!   that an `f32` `mad` writes as `m * K + X`, `K` a literal and `X` written by an `add.f32` of
//!   `time`. So `v = uv1.y + K m`, the clock added and taken off again. A literal record is
//!   `(index, bits)` in container 19, the same container;
//! - the squares: an `f16` `mul` whose two sources are the same register lanes as its result's,
//!   on the bass texel's first lane, the pitch tap's first lane and the colour tap's rgb.

use super::{Category, Program, usse};

/// The picture of the distortion and colour taps.
pub const ELECTRICITY: &str = "Electricity";
/// The pitch picture.
pub const PITCH: &str = "Pitch";
/// The bass and treble picture, fetched before the program runs.
pub const BASS_TREBLE: &str = "Bass_Treble_Hits";
/// The uniforms the law reads.
const UNIFORMS: [&str; 7] = [
    "time",
    "Tempo",
    "MasterVolume",
    "PitchOffset",
    "PitchVolume",
    "ElectricityColour",
    "FacingPower",
];
/// The container literals and dependent-sampler descriptors live in.
const SECONDARY_CONTAINER: u16 = 19;

/// What the program's code says beyond the names it declares.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElectricityRead {
    /// How far the colour tap's `v` moves per unit of the music's distortion `m`.
    pub colour_tap_shift: f32,
    /// Whether the program squares the bass tap's and the pitch tap's red and the colour tap's
    /// rgb itself.
    pub squared_taps: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bank {
    Temp,
    Output,
    Primary,
    Secondary,
    Other,
}

/// One 32-bit register word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Word(Bank, u32);

fn field(word: u64, lo: u32, n: u32) -> u32 {
    ((word >> lo) & ((1 << n) - 1)) as u32
}

/// A source 1 or 2 register's bank: `r o pa sa` without the extension bit.
fn src12_bank(sel: u32, ext: u32) -> Bank {
    match (ext, sel) {
        (0, 0) => Bank::Temp,
        (0, 1) => Bank::Output,
        (0, 2) => Bank::Primary,
        (0, 3) => Bank::Secondary,
        _ => Bank::Other,
    }
}

fn src0_bank(sel: u32) -> Bank {
    if sel == 0 { Bank::Temp } else { Bank::Primary }
}

fn dst_bank(sel: u32, ext: u32) -> Bank {
    match (ext, sel) {
        (0, 0) => Bank::Temp,
        (0, 1) => Bank::Output,
        (0, 2) => Bank::Primary,
        (1, 0) => Bank::Secondary,
        _ => Bank::Other,
    }
}

/// The lane a 4-letter swizzle reads for result lane `lane`; `None` for a constant.
fn lane_of(swizzle: &str, lane: usize) -> Option<u32> {
    "xyzw"
        .find(swizzle.as_bytes()[lane] as char)
        .map(|i| i as u32)
}

const SW4_STD: [&str; 16] = [
    "xxxx", "yyyy", "zzzz", "wwww", "xyzw", "yzww", "xyzz", "xxyz", "xyxy", "xywz", "zxyw", "zwzw",
    "yzxz", "xxyy", "xzww", "xyz1",
];
const MAD2_S0: [&str; 8] = [
    "xxxx", "yyyy", "zzzz", "wwww", "xyzw", "yzxw", "xyww", "zwxy",
];
const MAD2_S1: [&str; 8] = [
    "xxxx", "yyyy", "zzzz", "wwww", "xyzw", "xyyz", "yyww", "wyzw",
];
const MAD2_S2: [&str; 8] = [
    "xxxx", "yyyy", "zzzz", "wwww", "xyzw", "xzww", "xxyz", "xyzz",
];

/// A register number as a word index: the fields count register pairs.
fn doubled(bank: Bank, num: u32) -> Option<u32> {
    let n = (num << 1) & 0xff;
    // The top eight doubled temporaries are the internal registers.
    (bank != Bank::Temp || n < 120).then_some(n)
}

/// A scalar `f32` vector op (`nmad32`): `fn`, the written word and its two source words for the
/// one lane written, source 1's negation. `None` for another class or more than one lane.
#[derive(Debug, PartialEq, Eq)]
struct Scalar32 {
    function: u32,
    dst: Word,
    s1: Option<Word>,
    s1_neg: bool,
    s2: Option<Word>,
}

fn scalar32(word: u64) -> Option<Scalar32> {
    if usse::classify(word) != Some(usse::Class::Nmad32) {
        return None;
    }
    let lane = match field(word, 39, 4) & 3 {
        0b01 => 0,
        0b10 => 1,
        _ => return None,
    };
    let dst_bank = dst_bank(field(word, 32, 2), field(word, 51, 1));
    let dst = Word(
        dst_bank,
        doubled(dst_bank, field(word, 22, 6))? + lane as u32,
    );
    let enc = field(word, 15, 7)
        | (field(word, 34, 2) << 7)
        | (field(word, 50, 1) << 9)
        | (field(word, 53, 2) << 10);
    let s1_lane = (enc >> (3 * lane)) & 7;
    let s1_bank = src12_bank(field(word, 30, 2), field(word, 49, 1));
    let s1 = (s1_lane < 4)
        .then(|| doubled(s1_bank, field(word, 6, 6)).map(|n| Word(s1_bank, n + s1_lane)))
        .flatten();
    let s2_bank = src12_bank(field(word, 28, 2), field(word, 48, 1));
    let s2 = lane_of(SW4_STD[field(word, 44, 4) as usize], lane)
        .and_then(|l| doubled(s2_bank, field(word, 0, 6)).map(|n| Word(s2_bank, n + l)));
    Some(Scalar32 {
        function: field(word, 12, 3),
        dst,
        s1,
        s1_neg: matches!(field(word, 37, 2), 1 | 3),
        s2,
    })
}

/// An `f32` `mad2`'s written word and its three source words, `None` for a constant lane.
type Mad32 = (Word, Option<Word>, Option<Word>, Option<Word>);

/// An `f32` `mad2` writing one lane: `(dst, s0, s1, s2)` words.
fn mad32(word: u64) -> Option<Mad32> {
    if usse::classify(word) != Some(usse::Class::Mad2) || field(word, 58, 1) != 0 {
        return None;
    }
    let lane = match field(word, 39, 4) & 3 {
        0b01 => 0,
        0b10 => 1,
        _ => return None,
    };
    let dst_bank = dst_bank(field(word, 32, 2), 0);
    let dst = Word(
        dst_bank,
        doubled(dst_bank, field(word, 22, 6))? + lane as u32,
    );
    let operand = |bank: Bank, num: u32, swizzle: &str| {
        lane_of(swizzle, lane).and_then(|l| doubled(bank, num).map(|n| Word(bank, n + l)))
    };
    let s0 = operand(
        src0_bank(field(word, 34, 1)),
        field(word, 12, 6),
        MAD2_S0[(field(word, 18, 2) | (field(word, 53, 1) << 2)) as usize],
    );
    let s1 = operand(
        src12_bank(field(word, 30, 2), field(word, 49, 1)),
        field(word, 6, 6),
        MAD2_S1[(field(word, 20, 2) | (field(word, 44, 1) << 2)) as usize],
    );
    let s2 = operand(
        src12_bank(field(word, 28, 2), field(word, 48, 1)),
        field(word, 0, 6),
        MAD2_S2[field(word, 45, 3) as usize],
    );
    Some((dst, s0, s1, s2))
}

/// An `smp`: the first result word, the first coordinate word and the sampler's `sa` word.
fn sample(word: u64) -> Option<(Word, Word, u32)> {
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

/// The `f16` register half-lanes an `nmad16` `mul` squares: its result's lanes whose two sources
/// both read that same lane of the same register. `(bank, word, lane mask)`, or `None` when any
/// written lane is not a square.
fn squares16(word: u64) -> Option<(Bank, u32, u32)> {
    if usse::classify(word) != Some(usse::Class::Nmad16) || field(word, 12, 3) != 0 {
        return None;
    }
    let mask = field(word, 39, 4);
    let s1_bank = src12_bank(field(word, 30, 2), field(word, 49, 1));
    let s2_bank = src12_bank(field(word, 28, 2), field(word, 48, 1));
    let num = field(word, 6, 6);
    if s1_bank != s2_bank || num != field(word, 0, 6) || field(word, 37, 2) != 0 {
        return None;
    }
    let enc = field(word, 15, 7)
        | (field(word, 34, 2) << 7)
        | (field(word, 50, 1) << 9)
        | (field(word, 53, 2) << 10);
    let s2 = SW4_STD[field(word, 44, 4) as usize];
    for lane in 0..4 {
        if mask & (1 << lane) == 0 {
            continue;
        }
        let s1_lane = (enc >> (3 * lane)) & 7;
        if s1_lane != lane as u32 || lane_of(s2, lane) != Some(lane as u32) {
            return None;
        }
    }
    Some((s1_bank, doubled(s1_bank, num)?, mask))
}

/// The raw tables the header points at, read only as far as this module needs.
struct Tables {
    /// `(index, bits)` literal records.
    literals: Vec<(u32, u32)>,
    /// `(resource, sa word)` per dependent-sampler record.
    samplers: Vec<(u32, u32)>,
    /// Container 19's first `sa` word.
    base: u32,
    /// Primary word of each texel input record, with its sampler resource.
    texels: Vec<(u32, u32)>,
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

fn tables(b: &[u8]) -> Option<Tables> {
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
            texels.push((at, resource));
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

/// The reading of one fragment program (`bytes` its own bytes, `p` it parsed), when it has this
/// one's interface and shape; `None` for any other.
#[must_use]
pub fn program(bytes: &[u8], p: &Program) -> Option<ElectricityRead> {
    let resource = |name: &str| {
        p.parameters
            .iter()
            .find(|q| q.category == Category::Sampler && q.name == name)
            .map(|q| q.resource)
    };
    let uniform = |name: &str| {
        p.parameters
            .iter()
            .find(|q| q.category == Category::Uniform && q.name == name)
            .map(|q| q.resource)
    };
    let (electricity, pitch, bass) = (
        resource(ELECTRICITY)?,
        resource(PITCH)?,
        resource(BASS_TREBLE)?,
    );
    UNIFORMS
        .iter()
        .all(|u| uniform(u).is_some())
        .then_some(())?;
    let time = Word(Bank::Secondary, uniform("time")?);
    let t = tables(bytes)?;
    let bass_texels: Vec<u32> = t
        .texels
        .iter()
        .filter(|&&(_, r)| r == bass)
        .map(|&(w, _)| w)
        .collect();
    (bass_texels.len() == 2).then_some(())?;
    let code: Vec<u64> = usse::words(bytes, &p.primary).collect();
    let sampler_of = |sa: u32| {
        t.samplers
            .iter()
            .find(|&&(_, word)| t.base + word == sa)
            .map(|&(r, _)| r)
    };
    let mut colour_taps = Vec::new();
    let mut pitch_taps = Vec::new();
    for (k, &w) in code.iter().enumerate() {
        let Some((dst, coord, state)) = sample(w) else {
            continue;
        };
        match sampler_of(state) {
            Some(r) if r == electricity => colour_taps.push((k, dst, coord)),
            Some(r) if r == pitch => pitch_taps.push((k, dst)),
            _ => return None,
        }
    }
    let ([_, (last, colour, coord)], [(_, pitch_tap)]) =
        (colour_taps.as_slice(), pitch_taps.as_slice())
    else {
        return None;
    };
    // The last writer of `word` before instruction `before`.
    let writer = |word: Word, before: usize| {
        code[..before].iter().rposition(|&w| {
            scalar32(w).is_some_and(|s| s.dst == word) || mad32(w).is_some_and(|m| m.0 == word)
        })
    };
    let v = Word(coord.0, coord.1 + 1);
    let unclock = scalar32(code[writer(v, *last)?])?;
    (unclock.function == 1 && unclock.s1 == Some(time) && unclock.s1_neg).then_some(())?;
    let shifted = unclock.s2?;
    let at = writer(shifted, *last)?;
    let (_, _, Some(Word(Bank::Secondary, k)), Some(clocked)) = mad32(code[at])? else {
        return None;
    };
    let shift = t
        .literals
        .iter()
        .find(|&&(index, _)| t.base + index == k)
        .map(|&(_, bits)| f32::from_bits(bits))?;
    let clock = scalar32(code[writer(clocked, at)?])?;
    (clock.function == 1 && !clock.s1_neg && (clock.s1 == Some(time) || clock.s2 == Some(time)))
        .then_some(())?;
    let squared = |bank: Bank, word: u32, mask: u32| {
        code.iter()
            .filter_map(|&w| squares16(w))
            .any(|(b, n, m)| b == bank && n == word && m & mask == mask)
    };
    let squared_taps = bass_texels
        .iter()
        .any(|&w| squared(Bank::Primary, w, 0b0001))
        && squared(pitch_tap.0, pitch_tap.1, 0b0001)
        && squared(colour.0, colour.1, 0b0111);
    Some(ElectricityRead {
        colour_tap_shift: shift,
        squared_taps,
    })
}

/// The reading of a `.rcsmaterial` file, where every fragment program that declares the
/// `Electricity` sampler has this one's shape and they agree; `None` for a file with none.
#[must_use]
pub fn electricity(data: &[u8]) -> Option<ElectricityRead> {
    let mut found: Option<ElectricityRead> = None;
    for (at, parsed) in super::programs(data) {
        let Ok(p) = parsed else { continue };
        let declares = p.is_fragment()
            && p.parameters
                .iter()
                .any(|q| q.category == Category::Sampler && q.name == ELECTRICITY);
        if !declares {
            continue;
        }
        let r = program(&data[at..at + p.size], &p)?;
        if found.is_some_and(|f| f != r) {
            return None;
        }
        found = Some(r);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_tap_and_its_coordinate_read_as_listed() {
        // `smp2d.f16.f32 pa0.x, pa12.xy, sa14`.
        assert_eq!(
            sample(0xe001_8484_e001_8380),
            Some((Word(Bank::Primary, 0), Word(Bank::Primary, 12), 14))
        );
        // `smp2d.f16.f32 pa8.x, pa8.xy, sa18`.
        assert_eq!(
            sample(0xe001_8484_e101_0480),
            Some((Word(Bank::Primary, 8), Word(Bank::Primary, 8), 18))
        );
        // `add.f32 pa12.-y, -sa0.-x, pa0.-x`: the clock taken off the colour tap's `v`.
        assert_eq!(
            scalar32(0x0800_0122_e180_1000),
            Some(Scalar32 {
                function: 1,
                dst: Word(Bank::Primary, 13),
                s1: Some(Word(Bank::Secondary, 0)),
                s1_neg: true,
                s2: Some(Word(Bank::Primary, 0)),
            })
        );
        // `add.f32 pa6.x, sa0.x, pa12.y`: the clock added to `uv1.y`.
        assert_eq!(
            scalar32(0x0824_1086_e0c4_1006),
            Some(Scalar32 {
                function: 1,
                dst: Word(Bank::Primary, 6),
                s1: Some(Word(Bank::Secondary, 0)),
                s1_neg: false,
                s2: Some(Word(Bank::Primary, 13)),
            })
        );
    }

    #[test]
    fn the_shift_mad_reads_its_literal_lane() {
        // `mad.f32 pa0.x, pa0.x, sa22.y, pa6.x`.
        assert_eq!(
            mad32(0x0000_0086_e010_02c3),
            Some((
                Word(Bank::Primary, 0),
                Some(Word(Bank::Primary, 0)),
                Some(Word(Bank::Secondary, 23)),
                Some(Word(Bank::Primary, 6)),
            ))
        );
        // `mad.f16 ...` is not the `f32` shape.
        assert_eq!(mad32(0x04a0_828e_b020_0105), None);
    }

    #[test]
    fn a_square_is_one_register_times_itself() {
        // `mul.f16 pa4.x, pa4.x, pa4.x`.
        assert_eq!(
            squares16(0x1024_0086_a084_0082),
            Some((Bank::Primary, 4, 0b0001))
        );
        // `mul.f16 pa6.xyz, pa0.xyz, pa0.xyz`.
        assert_eq!(
            squares16(0x10a4_4386_a0c4_0000),
            Some((Bank::Primary, 0, 0b0111))
        );
        // `mul.f16 pa0.xyz, pa4.www, pa6.xyz` multiplies two registers.
        assert_eq!(squares16(0x10a4_4386_a02d_8083), None);
    }
}
