//! Hardware-free tests for the cue timeline.
//!
//! Every bank is assembled byte by byte. The corpus half - Pulse's
//! `speech_zone.bnk`, where the shape was found - is
//! `crates/formats/tests/sblk_timeline_ground_truth.rs`.

use crate::sblk::{
    Bank, COMMAND_LEN, CUE_LEN, HAS_NAME_TABLE, HEADER_LEN, MAGIC, NAME_BUCKETS_AT, NAME_ENTRY_LEN,
    SBLK_HEADER_LEN, SECTION_LEN, VERSION,
};

use super::TICKS_PER_SECOND;

const RECORD_LEN: usize = 32;

/// One grain of a hand-built cue.
enum Grain {
    /// `0x01` key-on of `(offset, length)` at a pan angle, after `delay` ticks.
    KeyOn { offset: u32, angle: u16, delay: u32 },
    /// `0x05` child by cue index, after `delay` ticks.
    Child { cue: u32, delay: u32, volume: u32 },
    /// Any other opcode, after `delay` ticks.
    Other { opcode: u32, delay: u32 },
}

struct Spec(&'static str, i8, Vec<Grain>);

fn build(specs: &[Spec]) -> Vec<u8> {
    let cues = u16::try_from(specs.len()).unwrap();
    let grains: usize = specs.iter().map(|s| s.2.len()).sum();
    let command_offset = SBLK_HEADER_LEN + usize::from(cues) * CUE_LEN;
    let parameter_offset = command_offset + grains * COMMAND_LEN;
    let name_offset = parameter_offset + grains * RECORD_LEN;
    let entries_at = name_offset + 0x98;
    let block_len = entries_at + (specs.len() + 1) * NAME_ENTRY_LEN;
    let waveform_len = 64usize;

    let mut block = vec![0u8; block_len];
    block[..4].copy_from_slice(MAGIC);
    block[4..8].copy_from_slice(&VERSION.to_le_bytes());
    block[0x08..0x0c].copy_from_slice(&HAS_NAME_TABLE.to_le_bytes());
    block[0x16..0x18].copy_from_slice(&cues.to_le_bytes());
    let commands = u16::try_from(grains).unwrap();
    block[0x18..0x1a].copy_from_slice(&commands.to_le_bytes());
    block[0x1a..0x1c].copy_from_slice(&commands.to_le_bytes());
    block[0x1c..0x20].copy_from_slice(&(SBLK_HEADER_LEN as u32).to_le_bytes());
    block[0x20..0x24].copy_from_slice(&(command_offset as u32).to_le_bytes());
    block[0x28..0x2c].copy_from_slice(&(waveform_len as u32).to_le_bytes());
    block[0x2c..0x30].copy_from_slice(&(waveform_len as u32).to_le_bytes());
    block[0x34..0x38].copy_from_slice(&(parameter_offset as u32).to_le_bytes());
    block[0x38..0x3c].copy_from_slice(&(name_offset as u32).to_le_bytes());

    let mut command = 0usize;
    for (index, spec) in specs.iter().enumerate() {
        let cue = SBLK_HEADER_LEN + index * CUE_LEN;
        block[cue] = spec.1 as u8;
        block[cue + 0x04] = u8::try_from(spec.2.len()).unwrap();
        let byte_offset = u32::try_from(command * COMMAND_LEN).unwrap();
        block[cue + 0x08..cue + 0x0c].copy_from_slice(&byte_offset.to_le_bytes());
        for grain in &spec.2 {
            let record = parameter_offset + command * RECORD_LEN;
            let operand = u32::try_from(command * RECORD_LEN).unwrap();
            let (opcode, delay) = match *grain {
                Grain::KeyOn {
                    offset,
                    angle,
                    delay,
                } => {
                    block[record + 0x01] = 127;
                    block[record + 0x04..record + 0x06].copy_from_slice(&angle.to_le_bytes());
                    block[record + 0x10..record + 0x14].copy_from_slice(&offset.to_le_bytes());
                    block[record + 0x14..record + 0x18].copy_from_slice(&16u32.to_le_bytes());
                    (0x01u32, delay)
                }
                Grain::Child { cue, delay, volume } => {
                    block[record..record + 4].copy_from_slice(&volume.to_le_bytes());
                    block[record + 0x0c..record + 0x10].copy_from_slice(&cue.to_le_bytes());
                    (0x05, delay)
                }
                Grain::Other { opcode, delay } => (opcode, delay),
            };
            let at = command_offset + command * COMMAND_LEN;
            block[at..at + 4].copy_from_slice(&(opcode << 24 | operand).to_le_bytes());
            block[at + 4..at + 8].copy_from_slice(&delay.to_le_bytes());
            command += 1;
        }
    }

    block[name_offset + 8..name_offset + 12].copy_from_slice(&0x98u32.to_le_bytes());
    for bucket in 0..64usize {
        let head = u16::try_from(if bucket == 0 { 0 } else { specs.len() }).unwrap();
        let at = name_offset + NAME_BUCKETS_AT + bucket * 2;
        block[at..at + 2].copy_from_slice(&head.to_le_bytes());
    }
    for (index, spec) in specs.iter().enumerate() {
        let at = entries_at + index * NAME_ENTRY_LEN;
        block[at..at + spec.0.len()].copy_from_slice(spec.0.as_bytes());
        let cue = u16::try_from(index).unwrap();
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

fn key(offset: u32, angle: u16, delay: u32) -> Grain {
    Grain::KeyOn {
        offset,
        angle,
        delay,
    }
}

/// The shape Pulse's `zone_5` has: a child, the number as a +-30 degree pair,
/// then another child.
fn zone() -> Vec<u8> {
    build(&[
        Spec(
            "zone_5",
            120,
            vec![
                Grain::Child {
                    cue: 1,
                    delay: 0,
                    volume: 127,
                },
                key(32, 30, 105),
                key(32, 330, 10),
                Grain::Child {
                    cue: 2,
                    delay: 140,
                    volume: 127,
                },
            ],
        ),
        Spec("ZONE", 120, vec![key(0, 30, 0), key(0, 330, 10)]),
        Spec("CLEAR", 120, vec![key(16, 30, 0), key(16, 330, 10)]),
    ])
}

#[test]
fn a_cue_is_a_sequence_with_its_delays_accumulated() {
    let data = zone();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("zone_5").expect("cue");
    let timeline = bank.cue_timeline(&cue);
    assert!(timeline.is_complete());

    let got: Vec<(u32, u32, i32)> = timeline
        .grains
        .iter()
        .map(|g| (g.tick, g.sound.offset, g.angle))
        .collect();
    // ZONE at 0 and 10, the number at 105 and 115, CLEAR at 115 + 140 and 10
    // after: a child's own list runs from the tick its parent started it on and
    // the parent's next delay counts from the child *command*, not its end.
    assert_eq!(
        got,
        vec![
            (0, 0, 30),
            (10, 0, 330),
            (105, 32, 30),
            (115, 32, 330),
            (255, 16, 30),
            (265, 16, 330),
        ]
    );
}

#[test]
fn a_flat_set_of_the_same_cue_is_what_played_one_word_at_random() {
    // The bug this module answers: the flat walk reaches all three words.
    let data = zone();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("zone_5").expect("cue");
    let mut offsets: Vec<u32> = bank
        .cue_tree_sounds(&cue)
        .iter()
        .map(|s| s.offset)
        .collect();
    offsets.sort_unstable();
    offsets.dedup();
    assert_eq!(offsets, vec![0, 16, 32]);
}

#[test]
fn a_child_carries_its_records_volume_and_its_parents_scale() {
    let data = zone();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("zone_5").expect("cue");
    let timeline = bank.cue_timeline(&cue);
    let root = &timeline.grains[2];
    let child = &timeline.grains[0];
    assert_eq!((root.cue_volume, root.scale), (120, 1.0));
    assert_eq!(child.cue_volume, 127);
    assert!((child.scale - 120.0 / 127.0).abs() < 1e-6);
}

#[test]
fn an_opcode_the_walk_does_not_model_is_reported_not_skipped() {
    let data = build(&[Spec(
        "A",
        100,
        vec![
            Grain::Other {
                opcode: 0x1a,
                delay: 0,
            },
            key(0, 0, 0),
            Grain::Other {
                opcode: 0x19,
                delay: 0,
            },
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let timeline = bank.cue_timeline(&bank.cue_named("A").expect("cue"));
    assert_eq!(timeline.grains.len(), 1);
    assert_eq!(timeline.unread, vec![0x1a, 0x19]);
    assert!(!timeline.is_complete());
}

#[test]
fn a_child_that_resolves_to_no_cue_is_counted() {
    let data = build(&[Spec(
        "A",
        100,
        vec![Grain::Child {
            cue: 9,
            delay: 0,
            volume: 127,
        }],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let timeline = bank.cue_timeline(&bank.cue_named("A").expect("cue"));
    assert_eq!(timeline.unresolved, 1);
    assert!(!timeline.is_complete());
}

#[test]
fn a_cycle_terminates() {
    let data = build(&[
        Spec(
            "A",
            100,
            vec![
                key(0, 0, 0),
                Grain::Child {
                    cue: 1,
                    delay: 5,
                    volume: 127,
                },
            ],
        ),
        Spec(
            "B",
            100,
            vec![Grain::Child {
                cue: 0,
                delay: 0,
                volume: 127,
            }],
        ),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let timeline = bank.cue_timeline(&bank.cue_named("A").expect("cue"));
    assert_eq!(timeline.grains.len(), 1);
    assert_eq!(timeline.unresolved, 1);
}

#[test]
fn the_tick_is_the_psp_s_measured_rate() {
    // 44,100 Hz, three ticks per two 256-frame grains, checked live at 257.7,
    // 259.7 and 258.3.
    assert!((TICKS_PER_SECOND - 258.398_437_5).abs() < 1e-6);
}
