//! The animated attributes of an emitter: `+0x93c` and the records at `+0x940`.
//!
//! `ParticleSystem_Update` (`0x088f5b9c`) walks `res+0x940`, `0xec` bytes a
//! record, **every tick**: each record is a channel block (the format
//! [`super::Channel`] reads, at the record's own `+0x00`) evaluated at the
//! emitter's normalised age (`instance+0x13c`, `1 - remaining / duration`), and
//! the value is stored into the instance field the selector at `+0xe0` names -
//! `1..6` to instance `+0x44`, `+0x48`, `+0x50`, `+0x4c`, `+0x54`, `+0x34` - after
//! which `ParticleSystem_DeriveScaledParams` runs again. Read off the decompile on
//! 2026-10-01 and the value measured live on `WO_BOMB_SMOKERING`: its one record
//! (selector `2`, a keyed channel `1 + 1 * (0.0068 .. 1)`) grows the spawn ring from
//! `12.94` units to `23.4` over the 20 ticks the emitter runs - the age-0 particles of
//! one detonation sat 13.1, 13.7, 15.0, 17.6 and 23.4 units from the centre at emitter
//! ticks 0, 1, 3, 7 and 16, where `12.94 * (1.007 + 0.993 * t / 20)` gives 13.0, 13.7,
//! 15.0, 17.5 and 23.3.
//!
//! **Selector `2` is the extent's co-factor** (`+0x2c`'s animated twin, `+0x48`):
//! it multiplies the three emitter extents and nothing else. The corpus authors
//! eleven records on eleven emitters, ten of them selector `2`; the eleventh,
//! `WO_REPULSER_BLAST`'s selector `5` (`+0x54`), is not read.

use oag_formats::ByteOrder;

use super::{Channel, Result, parse_channel};

/// Bytes of one record.
const RECORD_LEN: usize = 0xec;
/// The most records an emitter may carry - a bound on a walk driven by file bytes;
/// the corpus has one.
const MAX_RECORDS: usize = 8;

/// One animated attribute: a channel and the instance field it drives.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeAnimation {
    /// `+0xe0`: which co-factor the value is stored into. `2` is the extent's.
    pub selector: u32,
    /// The value over the emitter's normalised age.
    pub channel: Channel,
}

/// The records of the emitter whose record starts at `record`, none when the count
/// is zero, negative or past [`MAX_RECORDS`], or a record would run off `data`.
pub(super) fn parse_list(
    data: &[u8],
    order: ByteOrder,
    base: usize,
    record: &[u8],
) -> Result<Vec<AttributeAnimation>> {
    let count = order.u32(record, 0x93c) as i32;
    let pointer = order.u32(record, 0x940) as usize;
    let Ok(count) = usize::try_from(count) else {
        return Ok(Vec::new());
    };
    if count == 0 || count > MAX_RECORDS || pointer == 0 {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let start = base + pointer + index * RECORD_LEN;
        let Some(bytes) = data.get(start..start + RECORD_LEN) else {
            return Ok(Vec::new());
        };
        out.push(AttributeAnimation {
            selector: order.u32(bytes, 0xe0),
            channel: parse_channel(bytes, order, 0)?,
        });
    }
    Ok(out)
}
