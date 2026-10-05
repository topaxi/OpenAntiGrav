//! Byte coverage of one `.bnk` sound bank.
//!
//! **A separate file from `sblk.rs`, deliberately.** This crate's sample-rate
//! and SAS-pitch questions belong to another thread; nothing here touches
//! `ASSUMED_SAMPLE_RATE`, opcode decoding, or `crates/sound/src` - only
//! byte ranges, using [`Bank`]'s already-public fields and offsets. See
//! `docs/formats/psp-audio.md` for what each section is.
//!
//! # What is claimed and why the name table needs its own walk
//!
//! The container header, the section table, the `SBlk` block header, the cue
//! table and the command table are all either a [`Bank`] field already or a
//! fixed-size span the header states the length of - straightforward to
//! re-derive the same way `oag_rcs::rcsmodel::coverage` does. The waveform
//! descriptor table is claimed one 24-byte record at a time, only for the
//! records [`Bank::sounds`] actually resolves - a command whose descriptor
//! offset does not resolve is not claimed, matching what the decoder itself
//! skips. The name table is the one region with no closed-form length: its
//! bucket array size falls out of two header words (see
//! `Bank::sound_names`'s own comment), but the entries it points at are
//! reached by walking bucket chains, so this module re-walks them - not to
//! recover a value `Bank` does not expose, but to find where each entry
//! actually sits so it can be claimed.

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
/// bucket chain actually reaches - the same walk [`Bank::sound_names`] does,
/// repeated here because that method returns decoded `{name, cue}` pairs and
/// not the byte offsets they came from.
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
