//! What the `.bnk` sound-bank reader in [`super`] is asserted to do: the
//! `SBlk` descriptors, the PS-ADPCM waveforms behind them, and the containers
//! it refuses.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `sblk.rs`: the tests are 261 lines, well past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// Builds a bank by hand. No game data in any test.
fn bank(cues: u16, commands: u16, waveforms: u16, blocks: usize, name: &str) -> Vec<u8> {
    let cue_bytes = usize::from(cues) * CUE_LEN;
    let command_bytes = usize::from(commands) * COMMAND_LEN;
    let command_offset = SBLK_HEADER_LEN + cue_bytes;
    let name_offset = command_offset + command_bytes;
    let block_len = name_offset + 8;
    let waveform_len = blocks * ADPCM_BLOCK_LEN;

    let mut block = vec![0u8; block_len];
    block[..4].copy_from_slice(MAGIC);
    block[4..8].copy_from_slice(&VERSION.to_le_bytes());
    block[0x16..0x18].copy_from_slice(&cues.to_le_bytes());
    block[0x18..0x1a].copy_from_slice(&commands.to_le_bytes());
    block[0x1a..0x1c].copy_from_slice(&waveforms.to_le_bytes());
    block[0x1c..0x20].copy_from_slice(&(SBLK_HEADER_LEN as u32).to_le_bytes());
    block[0x20..0x24].copy_from_slice(&(command_offset as u32).to_le_bytes());
    block[0x24..0x28].copy_from_slice(&20544u32.to_le_bytes());
    block[0x28..0x2c].copy_from_slice(&(waveform_len as u32).to_le_bytes());
    block[0x2c..0x30].copy_from_slice(&(waveform_len as u32).to_le_bytes());
    block[0x38..0x3c].copy_from_slice(&(name_offset as u32).to_le_bytes());
    let bytes = name.as_bytes();
    let take = bytes.len().min(7);
    block[name_offset..name_offset + take].copy_from_slice(&bytes[..take]);

    let mut out = VERSION.to_le_bytes().to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    let first = (HEADER_LEN + 2 * SECTION_LEN) as u32;
    out.extend_from_slice(&first.to_le_bytes());
    out.extend_from_slice(&(block_len as u32).to_le_bytes());
    out.extend_from_slice(&(first + block_len as u32).to_le_bytes());
    out.extend_from_slice(&(waveform_len as u32).to_le_bytes());
    out.extend_from_slice(&block);
    // A flat, in-spec waveform: filter 0, shift 9, flag 0.
    for _ in 0..blocks {
        out.push(0x09);
        out.push(0x00);
        out.extend_from_slice(&[0x11u8; 14]);
    }
    out
}

/// Builds a bank carrying a command table, a parameter block and a name
/// table. Hand-assembled, like [`bank`]: no game data in any test.
///
/// `sounds` is `(opcode, waveform offset, waveform length)` per command,
/// laid out contiguously in the parameter block; `names` is the name table.
/// The first two hash buckets are both aimed at entry 0 so that walking
/// them yields each name once rather than twice.
fn scored_bank(sounds: &[(u8, u32, u32)], names: &[(&str, u16)]) -> Vec<u8> {
    let cues = u16::try_from(names.len()).expect("few names");
    let cue_bytes = usize::from(cues) * CUE_LEN;
    let command_offset = SBLK_HEADER_LEN + cue_bytes;
    let parameter_offset = command_offset + sounds.len() * COMMAND_LEN;
    let name_offset = parameter_offset + sounds.len() * DESCRIPTOR_LEN;
    let entries_at = name_offset + 0x98;
    let block_len = entries_at + (names.len() + 1) * NAME_ENTRY_LEN;
    let waveform_len = sounds
        .iter()
        .map(|&(_, offset, length)| offset + length)
        .max()
        .unwrap_or(0) as usize;

    let mut block = vec![0u8; block_len];
    block[..4].copy_from_slice(MAGIC);
    block[4..8].copy_from_slice(&VERSION.to_le_bytes());
    block[0x08..0x0c].copy_from_slice(&HAS_NAME_TABLE.to_le_bytes());
    block[0x16..0x18].copy_from_slice(&cues.to_le_bytes());
    let commands = u16::try_from(sounds.len()).expect("few commands");
    block[0x18..0x1a].copy_from_slice(&commands.to_le_bytes());
    block[0x1a..0x1c].copy_from_slice(&commands.to_le_bytes());
    block[0x1c..0x20].copy_from_slice(&(SBLK_HEADER_LEN as u32).to_le_bytes());
    block[0x20..0x24].copy_from_slice(&(command_offset as u32).to_le_bytes());
    block[0x28..0x2c].copy_from_slice(&(waveform_len as u32).to_le_bytes());
    block[0x2c..0x30].copy_from_slice(&(waveform_len as u32).to_le_bytes());
    block[0x34..0x38].copy_from_slice(&(parameter_offset as u32).to_le_bytes());
    block[0x38..0x3c].copy_from_slice(&(name_offset as u32).to_le_bytes());

    for (index, &(opcode, offset, length)) in sounds.iter().enumerate() {
        let operand = u32::try_from(index * DESCRIPTOR_LEN).expect("small bank");
        let at = command_offset + index * COMMAND_LEN;
        block[at..at + 4].copy_from_slice(&(u32::from(opcode) << 24 | operand).to_le_bytes());
        let record = parameter_offset + index * DESCRIPTOR_LEN;
        block[record + 0x10..record + 0x14].copy_from_slice(&offset.to_le_bytes());
        block[record + 0x14..record + 0x18].copy_from_slice(&length.to_le_bytes());
    }

    block[name_offset + 8..name_offset + 12].copy_from_slice(&0x98u32.to_le_bytes());
    for bucket in 0..64usize {
        // Every bucket but the first two lands on the terminator, which is
        // what an empty bucket looks like on disc.
        let head = u16::try_from(if bucket < 2 { 0 } else { names.len() }).expect("few names");
        let at = name_offset + NAME_BUCKETS_AT + bucket * 2;
        block[at..at + 2].copy_from_slice(&head.to_le_bytes());
    }
    for (index, &(name, cue)) in names.iter().enumerate() {
        let at = entries_at + index * NAME_ENTRY_LEN;
        block[at..at + name.len()].copy_from_slice(name.as_bytes());
        block[at + 0x10..at + 0x12].copy_from_slice(&cue.to_le_bytes());
    }

    let mut out = VERSION.to_le_bytes().to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    let first = (HEADER_LEN + 2 * SECTION_LEN) as u32;
    out.extend_from_slice(&first.to_le_bytes());
    out.extend_from_slice(&(block_len as u32).to_le_bytes());
    out.extend_from_slice(&(first + block_len as u32).to_le_bytes());
    out.extend_from_slice(&(waveform_len as u32).to_le_bytes());
    out.extend_from_slice(&block);
    out.resize(out.len() + waveform_len, 0);
    out
}

#[test]
fn a_bank_parses_and_its_sections_close() {
    let data = bank(6, 10, 7, 4, "FRNTEND");
    let parsed = Bank::parse(&data).expect("parse");
    assert_eq!(parsed.name, "FRNTEND");
    assert_eq!(parsed.cue_count, 6);
    assert_eq!(parsed.command_count, 10);
    assert_eq!(parsed.waveform_count, 7);
    assert_eq!(parsed.cues.len(), 6 * CUE_LEN);
    assert_eq!(parsed.commands.len(), 10 * COMMAND_LEN);
    assert_eq!(parsed.adpcm_blocks(), 4);
    assert!(looks_like_bank(&data));
}

#[test]
fn a_name_longer_than_the_field_is_truncated() {
    let data = bank(1, 1, 1, 1, "basilico");
    assert_eq!(Bank::parse(&data).expect("parse").name, "basilic");
}

#[test]
fn disagreeing_waveform_sizes_are_refused() {
    let mut data = bank(1, 1, 1, 2, "x");
    // The second copy of the size, inside the SBlk header.
    let at = HEADER_LEN + 2 * SECTION_LEN + 0x2c;
    data[at..at + 4].copy_from_slice(&16u32.to_le_bytes());
    assert!(matches!(
        Bank::parse(&data),
        Err(Error::SizeDisagreement { .. })
    ));
}

#[test]
fn a_gap_between_the_sections_is_refused() {
    let mut data = bank(1, 1, 1, 2, "x");
    let at = HEADER_LEN + SECTION_LEN; // the second section's offset
    let offset = u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
    data[at..at + 4].copy_from_slice(&(offset + 16).to_le_bytes());
    assert_eq!(Bank::parse(&data), Err(Error::BadSections));
}

#[test]
fn a_truncated_blob_is_refused() {
    let data = bank(1, 1, 1, 2, "x");
    assert!(Bank::parse(&data[..data.len() - 1]).is_err());
    assert!(Bank::parse(&data[..4]).is_err());
}

#[test]
fn a_flat_block_decodes_to_the_documented_sample_count() {
    let data = bank(1, 1, 1, 3, "x");
    let parsed = Bank::parse(&data).expect("parse");
    let pcm = decode_adpcm(parsed.waveforms);
    assert_eq!(pcm.len(), 3 * ADPCM_BLOCK_SAMPLES);
}

#[test]
fn the_predictor_history_carries_across_a_block_boundary() {
    // Filter 1 with a zero residual just decays the previous sample, so the
    // second block's first output has to depend on the first block's last.
    let mut stream = vec![0x09u8, 0x00];
    stream.extend_from_slice(&[0x77u8; 14]);
    stream.extend_from_slice(&[0x10u8, 0x00]);
    stream.extend_from_slice(&[0x00u8; 14]);
    let pcm = decode_adpcm(&stream);
    assert_eq!(pcm.len(), 2 * ADPCM_BLOCK_SAMPLES);
    assert_ne!(pcm[ADPCM_BLOCK_SAMPLES], 0, "history did not carry");
}

#[test]
fn pcm16_skips_the_header_and_reads_big_endian() {
    let mut data = vec![0u8; PCM16_HEADER_LEN];
    data.extend_from_slice(&1i16.to_be_bytes());
    data.extend_from_slice(&(-2i16).to_be_bytes());
    data.extend_from_slice(&32767i16.to_be_bytes());
    assert_eq!(decode_pcm16(&data), vec![1, -2, 32767]);
}

#[test]
fn pcm16_ignores_a_trailing_odd_byte() {
    let mut data = vec![0u8; PCM16_HEADER_LEN];
    data.extend_from_slice(&7i16.to_be_bytes());
    data.push(0xff);
    assert_eq!(decode_pcm16(&data), vec![7]);
}

#[test]
fn pcm16_on_a_span_shorter_than_the_header_decodes_to_nothing() {
    assert_eq!(decode_pcm16(&[0u8; 4]), Vec::<i16>::new());
}

#[test]
fn the_key_on_commands_resolve_to_their_waveform_spans() {
    let data = scored_bank(
        &[(0x01, 0, 272), (0x09, 272, 912), (0x01, 1184, 848)],
        &[("HORN", 0)],
    );
    let bank = Bank::parse(&data).expect("parse");
    let sounds = bank.sounds();
    assert_eq!(sounds.len(), 3);
    assert_eq!(sounds[0].opcode, 0x01);
    assert_eq!(sounds[1].opcode, 0x09, "0x09 binds a waveform too");
    assert_eq!(
        sounds
            .iter()
            .map(|s| (s.offset, s.length))
            .collect::<Vec<_>>(),
        [(0, 272), (272, 912), (1184, 848)]
    );
    // The command index, so a caller can tell which two commands shared a
    // waveform when a bank reuses one.
    assert_eq!(sounds[2].command, 2);
}

#[test]
fn a_command_that_is_not_a_key_on_binds_nothing() {
    // 0x05 and 0x16 are both opcodes the shipped banks use, and neither
    // reaches the handler that resolves a descriptor.
    let data = scored_bank(&[(0x05, 0, 16), (0x01, 16, 32), (0x16, 48, 16)], &[]);
    let sounds = Bank::parse(&data).expect("parse").sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!((sounds[0].offset, sounds[0].length), (16, 32));
}

#[test]
fn a_descriptor_outside_the_block_is_skipped_rather_than_panicking() {
    let mut data = scored_bank(&[(0x01, 0, 16)], &[]);
    // Point the one command's operand past the end of the section.
    let block_at = HEADER_LEN + 2 * SECTION_LEN;
    let at = block_at + SBLK_HEADER_LEN;
    data[at..at + 4].copy_from_slice(&(0x0100_0000u32 | 0x00ff_ffff).to_le_bytes());
    assert!(Bank::parse(&data).expect("parse").sounds().is_empty());
}

#[test]
fn the_name_table_walks_to_every_entry_once() {
    let data = scored_bank(
        &[(0x01, 0, 16)],
        &[("SPEEDUPPAD", 3), ("~ENGINE", 0), ("MESSAGE", 7)],
    );
    let bank = Bank::parse(&data).expect("parse");
    let names = bank.sound_names();
    // Two buckets aim at entry 0, so a walk that did not deduplicate would
    // return six.
    assert_eq!(names.len(), 3);
    assert_eq!(names[0].name, "SPEEDUPPAD");
    assert_eq!(names[0].cue, 3);
    assert_eq!(names[2].cue, 7);
}

#[test]
fn a_bank_without_the_name_table_bit_reports_no_names() {
    let mut data = scored_bank(&[(0x01, 0, 16)], &[("HORN", 0)]);
    let at = HEADER_LEN + 2 * SECTION_LEN + 0x08;
    data[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
    let bank = Bank::parse(&data).expect("parse");
    assert_eq!(bank.flags & HAS_NAME_TABLE, 0);
    assert!(bank.sound_names().is_empty());
}

#[test]
fn the_spec_checks_agree_with_the_ranges_they_document() {
    assert!(adpcm_block_is_in_spec(&[0x4c, 0]));
    assert!(!adpcm_block_is_in_spec(&[0x5c, 0]), "filter 5");
    assert!(!adpcm_block_is_in_spec(&[0x4d, 0]), "shift 13");
    assert!(adpcm_flag_is_defined(&[0, 7]));
    assert!(!adpcm_flag_is_defined(&[0, 8]));
}
