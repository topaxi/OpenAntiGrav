//! Byte coverage of one `.bnk` sound bank.
//!
//! A separate file from `sblk.rs`, deliberately: nothing here touches
//! `ASSUMED_SAMPLE_RATE`, opcode decoding or `crates/sound/src`, only byte
//! ranges via [`Bank`]'s public fields. See `docs/formats/psp-audio.md`.
//!
//! # What is claimed and why the name table needs its own walk
//!
//! The container header, section table, `SBlk` header, cue table and command
//! table are a [`Bank`] field or a fixed-size span (as `oag_rcs::rcsmodel::coverage`
//! does). Waveform descriptors are claimed per 24-byte record, only for those
//! [`Bank::sounds`] resolves. The name table has no closed-form length: the
//! bucket array size follows from two header words (see `Bank::sound_names`),
//! but entries are reached by walking bucket chains, so this re-walks them to
//! find where each entry sits.

use crate::coverage::Coverage;
use crate::sblk::{
    Bank, DESCRIPTOR_LEN, HAS_NAME_TABLE, HEADER_LEN, NAME_BUCKETS_AT, NAME_ENTRY_LEN,
    SBLK_HEADER_LEN, SECTION_LEN,
};

/// Offset of `haystack` within `data`, when `haystack` really is a subslice
/// of it - which every slice [`Bank`] hands back is, by construction.
fn offset_of(data: &[u8], haystack: &[u8]) -> Option<usize> {
    let base = data.as_ptr() as usize;
    let sub = haystack.as_ptr() as usize;
    (sub >= base && sub + haystack.len() <= base + data.len()).then_some(sub - base)
}

/// Coverage of one `.bnk` blob.
#[must_use]
pub fn coverage(data: &[u8]) -> Coverage {
    let mut seen = Coverage::new(data.len());
    let Ok(bank) = Bank::parse(data) else {
        return seen;
    };

    seen.claim(0, HEADER_LEN + 2 * SECTION_LEN, "the container header");

    let Some(block_at) = offset_of(data, bank.block) else {
        return seen;
    };
    seen.claim(block_at, SBLK_HEADER_LEN, "the SBlk block header");

    if let Some(at) = offset_of(data, bank.cues) {
        seen.claim(at, bank.cues.len(), "the cue table");
    }
    if let Some(at) = offset_of(data, bank.commands) {
        seen.claim(at, bank.commands.len(), "the command table");
    }
    if let Some(at) = offset_of(data, bank.waveforms) {
        seen.claim(at, bank.waveforms.len(), "the waveform data");
    }

    // The waveform descriptor table: one 24-byte record per command that
    // resolves, at `block_at + parameter_offset + descriptor`.
    for sound in bank.sounds() {
        let at = block_at + sound.descriptor as usize;
        seen.claim(at, DESCRIPTOR_LEN, "a waveform descriptor");
    }

    claim_name_table(&mut seen, &bank, block_at);

    seen
}

/// Claims the name table's fixed head, its bucket array, and every entry a
/// bucket chain reaches: [`Bank::sound_names`]'s walk, repeated for the byte
/// offsets it does not return.
fn claim_name_table(seen: &mut Coverage, bank: &Bank, block_at: usize) {
    if bank.flags & HAS_NAME_TABLE == 0 {
        return;
    }
    let names = bank.name_offset as usize;
    let Some(tail) = bank.block.get(names..) else {
        return;
    };
    if tail.len() < NAME_BUCKETS_AT {
        return;
    }
    let entries = bank.order.u32(tail, 0x08) as usize;
    let Some(bucket_bytes) = entries.checked_sub(NAME_BUCKETS_AT) else {
        return;
    };

    seen.claim(block_at + names, NAME_BUCKETS_AT, "the name block header");
    seen.claim(
        block_at + names + NAME_BUCKETS_AT,
        bucket_bytes,
        "the name bucket array",
    );

    let mut visited = Vec::new();
    for bucket in 0..bucket_bytes / 2 {
        let head = usize::from(bank.order.u16(tail, NAME_BUCKETS_AT + bucket * 2));
        let mut at = entries + head * NAME_ENTRY_LEN;
        while let Some(entry) = tail.get(at..).and_then(|t| t.get(..NAME_ENTRY_LEN)) {
            if entry[0] == 0 {
                break;
            }
            if !visited.contains(&at) {
                visited.push(at);
                seen.claim(block_at + names + at, NAME_ENTRY_LEN, "a name table entry");
            }
            at += NAME_ENTRY_LEN;
        }
    }
}

#[cfg(test)]
mod tests;
