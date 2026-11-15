//! Hardware-free tests for the cue timeline.
//! Every bank is assembled byte by byte. The corpus half (Pulse's
//! `speech_zone.bnk`, where the shape was found) is
//! `crates/formats/tests/sblk_timeline_ground_truth.rs`.

use crate::sblk::{
    Bank, COMMAND_LEN, CUE_LEN, HAS_NAME_TABLE, HEADER_LEN, MAGIC, NAME_BUCKETS_AT, NAME_ENTRY_LEN,
    SBLK_HEADER_LEN, SECTION_LEN, VERSION,
};

use super::{TICKS_PER_SECOND, WalkModel};

const RECORD_LEN: usize = 32;

/// One grain of a hand-built cue.
pub(crate) enum Grain {
    /// `0x01` key-on of `(offset, length)` at a pan angle, after `delay` ticks.
    KeyOn { offset: u32, angle: u16, delay: u32 },
    /// `0x05` child by cue index, after `delay` ticks.
    Child { cue: u32, delay: u32, volume: u32 },
    /// `0x08` child by cue index, after `delay` ticks: the same record as `0x05`.
    ReplaceChild { cue: u32, delay: u32, volume: u32 },
    /// Any other opcode, after `delay` ticks.
    Other { opcode: u32, delay: u32 },
    /// An opcode with its own operand word, after `delay` ticks.
    Raw {
        opcode: u32,
        operand: u32,
        delay: u32,
    },
}

pub(crate) struct Spec(
    pub(crate) &'static str,
    pub(crate) i8,
    pub(crate) Vec<Grain>,
);

pub(crate) fn build(specs: &[Spec]) -> Vec<u8> {
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
            let mut operand = u32::try_from(command * RECORD_LEN).unwrap();
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
                Grain::ReplaceChild { cue, delay, volume } => {
                    block[record..record + 4].copy_from_slice(&volume.to_le_bytes());
                    block[record + 0x0c..record + 0x10].copy_from_slice(&cue.to_le_bytes());
                    (0x08, delay)
                }
                Grain::Other { opcode, delay } => (opcode, delay),
                Grain::Raw {
                    opcode,
                    operand: raw,
                    delay,
                } => {
                    operand = raw;
                    (opcode, delay)
                }
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

pub(crate) fn key(offset: u32, angle: u16, delay: u32) -> Grain {
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
    // after: a child's list runs from the tick its parent started it, and the
    // parent's next delay counts from the child *command*, not its end.
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

pub(crate) fn raw(opcode: u32, operand: u32, delay: u32) -> Grain {
    Grain::Raw {
        opcode,
        operand,
        delay,
    }
}

fn ticks(t: &super::Timeline) -> Vec<(u32, u32)> {
    t.grains.iter().map(|g| (g.tick, g.sound.offset)).collect()
}

/// `.COLLISIONS`' shape with a tail: a group of three single-command
/// alternates, then a key-on that always runs.
fn alternates() -> Vec<u8> {
    build(&[Spec(
        "A",
        100,
        vec![
            raw(0x19, 0x0000_0103, 0),
            key(0, 0, 0),
            key(16, 0, 0),
            key(32, 0, 0),
            key(48, 0, 20),
        ],
    )])
}

#[test]
fn an_alternate_group_runs_one_block_and_then_what_follows() {
    let data = alternates();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    for (pick, offset) in [(0u8, 0u32), (1, 16), (2, 32)] {
        let t = bank.cue_timeline_with(&cue, &[pick]);
        assert!(t.is_complete(), "{:?}", t.unread);
        assert_eq!(ticks(&t), vec![(0, offset), (20, 48)], "pick {pick}");
        assert_eq!(t.groups.len(), 1);
        assert_eq!((t.groups[0].count, t.groups[0].stride), (3, 1));
    }
    // A pick past the count wraps rather than running off the group.
    assert_eq!(ticks(&bank.cue_timeline_with(&cue, &[4]))[0], (0, 16));
}

#[test]
fn every_combination_of_alternates_is_enumerated() {
    let data = alternates();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let all = bank.cue_timelines(&cue).expect("enumerable");
    let firsts: Vec<u32> = all.iter().map(|t| t.grains[0].sound.offset).collect();
    assert_eq!(firsts, vec![0, 16, 32]);
}

#[test]
fn a_stride_of_two_takes_both_commands_of_the_alternate() {
    let data = build(&[Spec(
        "A",
        100,
        vec![
            raw(0x19, 0x0000_0202, 0),
            key(0, 0, 0),
            key(16, 0, 7),
            key(32, 0, 0),
            key(48, 0, 9),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    assert_eq!(
        ticks(&bank.cue_timeline_with(&cue, &[0])),
        vec![(0, 0), (7, 16)]
    );
    assert_eq!(
        ticks(&bank.cue_timeline_with(&cue, &[1])),
        vec![(0, 32), (9, 48)]
    );
}

#[test]
fn the_end_opcode_stops_the_list() {
    let data = build(&[Spec(
        "A",
        100,
        vec![key(0, 0, 0), raw(0x2b, 0, 0), key(16, 0, 30)],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let t = bank.cue_timeline(&bank.cue_named("A").expect("cue"));
    assert!(t.is_complete());
    assert_eq!(ticks(&t), vec![(0, 0)]);
}

#[test]
fn a_random_bend_is_shared_by_the_key_ons_after_it_and_adds_no_time() {
    let data = build(&[Spec(
        "A",
        100,
        vec![
            raw(0x14, 0, 0),
            raw(0x1b, 100, 0),
            key(0, 0, 0),
            key(16, 0, 12),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let t = bank.cue_timeline(&bank.cue_named("A").expect("cue"));
    assert!(t.is_complete());
    assert_eq!(t.bends, vec![100]);
    assert_eq!(t.passed, vec![0x14]);
    assert_eq!(ticks(&t), vec![(0, 0), (12, 16)]);
    assert!(t.grains.iter().all(|g| g.bend == Some(0)));
}

#[test]
fn a_bend_after_a_key_on_is_not_modelled() {
    let data = build(&[Spec("A", 100, vec![key(0, 0, 0), raw(0x1b, 100, 0)])]);
    let bank = Bank::parse(&data).expect("parse");
    let t = bank.cue_timeline(&bank.cue_named("A").expect("cue"));
    assert_eq!(t.unread, vec![0x1b]);
}

const GOTO_MODEL: WalkModel = WalkModel {
    goto_markers: true,
    hd_alternates: false,
    globals: None,
    vita: false,
    tail_waits: false,
    zero_waits: false,
    cue_variables: None,
};

/// Wipeout HD's `c_CLEAR`: a goto and the marker it lands on, nothing else.
fn goto_and_marker() -> Vec<u8> {
    build(&[Spec("c_CLEAR", 85, vec![raw(0x24, 0, 0), raw(0x23, 0, 0)])])
}

#[test]
fn a_goto_and_its_marker_are_unread_unless_the_walk_is_asked_to_follow_them() {
    let data = goto_and_marker();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("c_CLEAR").expect("cue");
    assert_eq!(bank.cue_timeline(&cue).unread, vec![0x24, 0x23]);
    let followed = bank.cue_timeline_modelled(&cue, &[], GOTO_MODEL);
    assert!(followed.is_complete());
    assert!(followed.grains.is_empty(), "a silent cue, by construction");
    assert_eq!(followed.passed, vec![0x23]);
}

#[test]
fn a_goto_skips_to_the_marker_with_its_id_and_counts_the_markers_own_delay() {
    // key at 0; goto #1 after 3; a jumped-over key; marker #2 (not ours);
    // marker #1 after 2 more; a key after 4 more.
    let data = build(&[Spec(
        "A",
        100,
        vec![
            key(0, 0, 0),
            raw(0x24, 0x01_0000, 3),
            key(16, 0, 0),
            raw(0x23, 0x02_0000, 0),
            raw(0x23, 0x01_0000, 2),
            key(32, 0, 4),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let t = bank.cue_timeline_modelled(&cue, &[], GOTO_MODEL);
    assert!(t.is_complete());
    assert_eq!(ticks(&t), vec![(0, 0), (9, 32)]);
}

/// The shape HD's `speech_zone.bnk` `MR_VEN` has: a `0x1a` with operand 0 on a
/// delay of 200, then two key-ons with delay 0.
fn zero_wait_then_two_keys() -> Vec<u8> {
    build(&[Spec(
        "MR_VEN",
        100,
        vec![raw(0x1a, 0, 200), key(0, 30, 0), key(16, 330, 0)],
    )])
}

const ZERO_WAIT_MODEL: WalkModel = WalkModel {
    zero_waits: true,
    ..GOTO_MODEL
};

#[test]
fn a_zero_operand_random_wait_is_unread_unless_the_walk_passes_it() {
    let data = zero_wait_then_two_keys();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("MR_VEN").expect("cue");
    assert_eq!(bank.cue_timeline(&cue).unread, vec![0x1a]);
    let passed = bank.cue_timeline_modelled(&cue, &[], ZERO_WAIT_MODEL);
    assert!(passed.is_complete());
    assert_eq!(passed.passed, vec![0x1a]);
}

#[test]
fn a_passed_zero_wait_keeps_its_own_delay_for_every_key_on_after_it() {
    // Delays are relative: the keys follow a 200 tick wait, both on one tick.
    let data = zero_wait_then_two_keys();
    let bank = Bank::parse(&data).expect("parse");
    let t = bank.cue_timeline_modelled(
        &bank.cue_named("MR_VEN").expect("cue"),
        &[],
        ZERO_WAIT_MODEL,
    );
    assert_eq!(ticks(&t), vec![(200, 0), (200, 16)]);
}

#[test]
fn a_random_wait_with_an_operand_stays_unread_even_when_zero_waits_are_passed() {
    // `rand16 % 6` extra ticks: not deterministic, so not a timeline.
    let data = build(&[Spec("A", 100, vec![raw(0x1a, 5, 10), key(0, 0, 0)])]);
    let bank = Bank::parse(&data).expect("parse");
    let t = bank.cue_timeline_modelled(&bank.cue_named("A").expect("cue"), &[], ZERO_WAIT_MODEL);
    assert_eq!(t.unread, vec![0x1a]);
    assert!(!t.is_complete());
}

#[test]
fn a_goto_with_no_marker_ends_the_cue() {
    let data = build(&[Spec(
        "A",
        100,
        vec![key(0, 0, 0), raw(0x24, 0x07_0000, 1), key(16, 0, 1)],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let t = bank.cue_timeline_modelled(&bank.cue_named("A").expect("cue"), &[], GOTO_MODEL);
    assert_eq!(ticks(&t), vec![(0, 0)]);
    assert!(t.is_complete());
}

#[test]
fn a_ninth_goto_within_one_tick_is_refused_and_a_waited_loop_terminates() {
    // The marker is above the goto, so it loops. With no delay the depth guard
    // refuses the ninth pass.
    let tight = build(&[Spec("A", 100, vec![raw(0x23, 0, 0), raw(0x24, 0, 0)])]);
    let bank = Bank::parse(&tight).expect("parse");
    let t = bank.cue_timeline_modelled(&bank.cue_named("A").expect("cue"), &[], GOTO_MODEL);
    assert_eq!(t.unread, vec![0x24]);
    // With a wait each pass the guard resets; the step bound ends the walk.
    let waited = build(&[Spec(
        "A",
        100,
        vec![raw(0x23, 0, 0), key(0, 0, 1), raw(0x24, 0, 1)],
    )]);
    let bank = Bank::parse(&waited).expect("parse");
    let t = bank.cue_timeline_modelled(&bank.cue_named("A").expect("cue"), &[], GOTO_MODEL);
    assert_eq!(t.unread, vec![0xff]);
    assert!(!t.is_complete());
}

/// `[variable, mode, immediate]` as a guard's operand word.
fn guard(variable: i8, mode: u8, immediate: u8) -> Grain {
    raw(
        0x22,
        u32::from(variable as u8) | u32::from(mode) << 8 | u32::from(immediate) << 16,
        0,
    )
}

fn with_globals(slot: usize, value: i8) -> WalkModel {
    let mut globals = [0; super::GLOBAL_SLOTS];
    globals[slot - 1] = value;
    WalkModel {
        globals: Some(globals),
        ..WalkModel::default()
    }
}

/// `TMTX_NAME`'s shape: each team is a guarded pair of key-ons ten ticks apart.
fn guarded_pairs() -> Vec<u8> {
    build(&[Spec(
        "TEAM",
        127,
        vec![
            guard(-2, 1, 1),
            key(0, 0, 0),
            guard(-2, 1, 1),
            key(16, 0, 10),
            guard(-2, 1, 2),
            key(32, 0, 0),
            guard(-2, 1, 2),
            key(48, 0, 10),
        ],
    )])
}

#[test]
fn a_global_guard_keeps_the_pair_whose_immediate_equals_the_global() {
    let data = guarded_pairs();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("TEAM").expect("cue");
    for (value, offsets) in [(1, [0, 16]), (2, [32, 48])] {
        let t = bank.cue_timeline_modelled(&cue, &[], with_globals(2, value));
        assert!(t.is_complete(), "{:?}", t.unread);
        let got: Vec<(u32, u32)> = t.grains.iter().map(|g| (g.tick, g.sound.offset)).collect();
        assert_eq!(
            got,
            vec![(0, offsets[0]), (10, offsets[1])],
            "value {value}"
        );
    }
    // A value no immediate names plays nothing, as the original's missing 10.
    let none = bank.cue_timeline_modelled(&cue, &[], with_globals(2, 10));
    assert!(none.is_complete() && none.grains.is_empty());
}

#[test]
fn a_global_guard_reads_its_own_slot_and_is_unread_without_a_model() {
    let data = guarded_pairs();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("TEAM").expect("cue");
    // The guards name slot 2; a value in slot 1 selects nothing.
    let other = bank.cue_timeline_modelled(&cue, &[], with_globals(1, 1));
    assert!(other.grains.is_empty());
    // The default walk leaves the guard unread, as before this model existed.
    assert_eq!(bank.cue_timeline(&cue).unread[0], 0x22);
}

#[test]
fn a_replacing_child_is_walked_only_under_the_global_model() {
    let data = build(&[
        Spec(
            "PARENT",
            127,
            vec![
                guard(-1, 1, 1),
                Grain::ReplaceChild {
                    cue: 1,
                    delay: 0,
                    volume: 127,
                },
                guard(-1, 1, 2),
                Grain::ReplaceChild {
                    cue: 2,
                    delay: 0,
                    volume: 127,
                },
            ],
        ),
        Spec("ONE", 127, vec![key(0, 0, 0)]),
        Spec("TWO", 127, vec![key(16, 0, 0)]),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("PARENT").expect("cue");
    let two = bank.cue_timeline_modelled(&cue, &[], with_globals(1, 2));
    assert!(two.is_complete(), "{:?}", two.unread);
    assert_eq!(two.grains.len(), 1);
    assert_eq!(two.grains[0].sound.offset, 16);
    assert!(!bank.cue_timeline(&cue).is_complete());
}

const VITA_MODEL: WalkModel = WalkModel {
    goto_markers: true,
    hd_alternates: false,
    globals: None,
    vita: true,
    tail_waits: false,
    zero_waits: false,
    cue_variables: None,
};

/// 2048's marker id is the operand's low byte: a goto to marker 2 lands on the
/// second marker there, and on the first (id byte 1 reads zero) on Pulse's and HD's.
#[test]
fn a_vita_goto_finds_its_marker_by_the_low_operand_byte() {
    let data = build(&[Spec(
        "A",
        100,
        vec![
            raw(0x24, 2, 0),
            raw(0x23, 1, 0),
            key(0, 0, 0),
            raw(0x2b, 0, 0),
            raw(0x23, 2, 0),
            key(16, 0, 0),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let vita = bank.cue_timeline_modelled(&cue, &[], VITA_MODEL);
    assert!(vita.is_complete(), "{:?}", vita.unread);
    assert_eq!(vita.grains.len(), 1);
    assert_eq!(vita.grains[0].sound.offset, 16, "marker 2, not the first");
    let other = bank.cue_timeline_modelled(&cue, &[], GOTO_MODEL);
    assert_eq!(other.grains[0].sound.offset, 0, "the first marker");
}

/// `0x2e` mode 0 draws a marker uniformly from `first..=last`: an alternate group of
/// `last - first + 1`, one timeline per marker. Any other mode is unread.
#[test]
fn a_vita_random_goto_is_an_alternate_over_its_markers() {
    let mut data = build(&[Spec(
        "A",
        100,
        vec![
            raw(0x2e, 0, 0),
            raw(0x23, 1, 0),
            key(0, 0, 0),
            raw(0x2b, 0, 0),
            raw(0x23, 2, 0),
            key(16, 0, 0),
        ],
    )]);
    let (block_at, record_at) = {
        let bank = Bank::parse(&data).expect("parse");
        (
            bank.block.as_ptr() as usize - data.as_ptr() as usize,
            bank.parameter_offset as usize,
        )
    };
    let at = block_at + record_at;
    data[at..at + 4].copy_from_slice(&[1, 2, 0, 3]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let all = bank
        .cue_timelines_modelled(&cue, VITA_MODEL)
        .expect("two combinations");
    let offsets: Vec<u32> = all.iter().map(|t| t.grains[0].sound.offset).collect();
    assert_eq!(offsets, vec![0, 16]);
    assert!(all.iter().all(|t| t.is_complete()));
    data[at + 2] = 1;
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let history = bank.cue_timeline_modelled(&cue, &[], VITA_MODEL);
    assert_eq!(
        history.unread,
        vec![0x2e],
        "mode 1 keeps a history: not modelled"
    );
}

/// `[a, b, c]` as HD's operand word (`a` in the high byte).
const fn hd(a: u8, b: u8, c: u8) -> u32 {
    (a as u32) << 16 | (b as u32) << 8 | c as u32
}

fn hd_model(variables: [i8; 4]) -> WalkModel {
    WalkModel {
        goto_markers: true,
        cue_variables: Some(variables),
        ..WalkModel::default()
    }
}

/// `weapons.bnk`'s `SPEEDUPPAD` (`docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md`,
/// "`SPEEDUPPAD`"): a body, a 100 % bend, then a switch on variable 2 over
/// three markers.
fn speedup_pad() -> Vec<u8> {
    let end = hd(0x63, 0, 0);
    build(&[Spec(
        "SPEEDUPPAD",
        120,
        vec![
            key(0, 0, 0),
            raw(0x1b, hd(100, 0, 0), 0),
            raw(0x22, hd(2, 1, 0), 0),
            raw(0x24, hd(0, 0, 0), 0),
            raw(0x22, hd(2, 1, 1), 0),
            raw(0x24, hd(1, 0, 0), 0),
            raw(0x22, hd(2, 1, 2), 0),
            raw(0x24, hd(2, 0, 0), 0),
            raw(0x24, end, 0),
            raw(0x23, hd(0, 0, 0), 0),
            key(16, 220, 0),
            key(32, 140, 10),
            raw(0x24, end, 0),
            raw(0x23, hd(1, 0, 0), 0),
            key(16, 0, 0),
            key(32, 0, 10),
            raw(0x24, end, 0),
            raw(0x23, hd(2, 0, 0), 0),
            raw(0x19, hd(2, 1, 0), 0),
            key(16, 0, 0),
            key(32, 0, 5),
            raw(0x24, end, 0),
            raw(0x23, end, 0),
        ],
    )])
}

fn grains(t: &super::Timeline) -> Vec<(u32, u32, i32)> {
    t.grains
        .iter()
        .map(|g| (g.tick, g.sound.offset, g.angle))
        .collect()
}

#[test]
fn hd_variable_zero_keys_the_body_and_a_rear_pair_under_one_bend() {
    let data = speedup_pad();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("SPEEDUPPAD").expect("cue");
    let t = bank.cue_timeline_modelled(&cue, &[], hd_model([0, 0, 0, 0]));
    assert!(t.is_complete(), "{:?}", t.unread);
    assert_eq!(grains(&t), vec![(0, 0, 0), (0, 16, 220), (10, 32, 140)]);
    assert_eq!(t.bends, vec![100]);
    assert!(
        t.grains.iter().all(|g| g.bend == Some(0)),
        "the body keyed before the bend follows it too"
    );
}

#[test]
fn hd_variable_one_centres_every_voice_and_two_alternates_between_the_pair() {
    let data = speedup_pad();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("SPEEDUPPAD").expect("cue");
    let one = bank.cue_timeline_modelled(&cue, &[], hd_model([0, 0, 1, 0]));
    assert_eq!(grains(&one), vec![(0, 0, 0), (0, 16, 0), (10, 32, 0)]);

    let two = bank
        .cue_timelines_modelled(&cue, hd_model([0, 0, 2, 0]))
        .expect("one group");
    let seen: Vec<_> = two.iter().map(grains).collect();
    assert_eq!(
        seen,
        vec![vec![(0, 0, 0), (0, 16, 0)], vec![(0, 0, 0), (5, 32, 0)]]
    );
    assert!(two.iter().all(|t| t.is_complete()));

    let three = bank.cue_timeline_modelled(&cue, &[], hd_model([0, 0, 3, 0]));
    assert_eq!(grains(&three), vec![(0, 0, 0)]);
}

#[test]
fn without_the_hd_model_the_same_cue_is_not_walked_to_its_voices() {
    let data = speedup_pad();
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("SPEEDUPPAD").expect("cue");
    let pulse = bank.cue_timeline_modelled(
        &cue,
        &[],
        WalkModel {
            goto_markers: true,
            ..WalkModel::default()
        },
    );
    assert!(
        !pulse.is_complete(),
        "Pulse's walk reads HD's guard wrongly"
    );
}

#[test]
fn a_variable_outside_the_handlers_four_is_unread() {
    let data = build(&[Spec(
        "A",
        100,
        vec![raw(0x22, hd(0xfe, 1, 0), 0), key(0, 0, 0)],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    assert_eq!(
        bank.cue_timeline_modelled(&cue, &[], hd_model([0; 4]))
            .unread,
        vec![0x22]
    );
}

#[test]
fn set_and_increment_write_this_handler_and_a_child_starts_with_a_copy() {
    // The parent sets variable 1 to 5, increments it, and starts a child that
    // guards on 6; the child's own increment is not seen by the parent.
    let data = build(&[
        Spec(
            "P",
            100,
            vec![
                raw(0x1e, hd(1, 5, 0), 0),
                raw(0x20, hd(1, 0, 0), 0),
                Grain::Child {
                    cue: 1,
                    delay: 0,
                    volume: 127,
                },
                raw(0x22, hd(1, 1, 6), 0),
                key(0, 0, 0),
                key(16, 0, 7),
            ],
        ),
        Spec(
            "C",
            100,
            vec![
                raw(0x22, hd(1, 1, 6), 0),
                key(32, 0, 0),
                raw(0x20, hd(1, 0, 0), 0),
                raw(0x22, hd(1, 1, 7), 0),
                key(48, 0, 3),
            ],
        ),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("P").expect("cue");
    let t = bank.cue_timeline_modelled(&cue, &[], hd_model([0; 4]));
    assert!(t.is_complete(), "{:?}", t.unread);
    // The child sees 6 (skips nothing, keys 32), then its own 7 (keys 48 at
    // tick 3); the parent's variable is still 6 (skips nothing past its guard).
    assert_eq!(
        grains(&t),
        vec![(0, 32, 0), (0, 0, 0), (3, 48, 0), (7, 16, 0)]
    );
}

#[test]
fn an_increment_stops_at_127() {
    let data = build(&[Spec(
        "A",
        100,
        vec![
            raw(0x1e, hd(0, 127, 0), 0),
            raw(0x20, hd(0, 0, 0), 0),
            raw(0x22, hd(0, 1, 127), 0),
            key(0, 0, 0),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let t = bank.cue_timeline_modelled(&cue, &[], hd_model([0; 4]));
    assert_eq!(grains(&t), vec![(0, 0, 0)], "127 stayed 127");
}

#[test]
fn hd_s_random_goto_is_an_alternate_over_its_marker_ids() {
    let end = hd(0x63, 0, 0);
    let data = build(&[Spec(
        "A",
        100,
        vec![
            raw(0x25, hd(4, 6, 0), 0),
            raw(0x23, hd(4, 0, 0), 0),
            key(0, 0, 0),
            raw(0x24, end, 0),
            raw(0x23, hd(5, 0, 0), 0),
            key(16, 0, 0),
            raw(0x24, end, 0),
            raw(0x23, hd(6, 0, 0), 0),
            key(32, 0, 0),
            raw(0x23, end, 0),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let all = bank
        .cue_timelines_modelled(&cue, hd_model([0; 4]))
        .expect("one group");
    assert_eq!(all.len(), 3);
    let offsets: Vec<u32> = all.iter().map(|t| t.grains[0].sound.offset).collect();
    assert_eq!(offsets, vec![0, 16, 32]);
    assert_eq!((all[0].groups[0].count, all[0].groups[0].stride), (3, 0));
}

#[test]
fn hd_s_stop_ends_the_cue_and_a_late_bend_reaches_the_voices_already_keyed() {
    let data = build(&[Spec(
        "A",
        100,
        vec![
            key(0, 0, 0),
            raw(0x1b, hd(50, 0, 0), 4),
            key(16, 0, 4),
            raw(0x18, 0, 0),
            key(32, 0, 0),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    let t = bank.cue_timeline_modelled(&cue, &[], hd_model([0; 4]));
    assert!(t.is_complete(), "{:?}", t.unread);
    assert_eq!(grains(&t), vec![(0, 0, 0), (8, 16, 0)]);
    assert_eq!(t.bends, vec![50]);
    assert!(t.grains.iter().all(|g| g.bend == Some(0)));
}

#[test]
fn hd_s_bend_percent_is_the_high_operand_byte() {
    let data = build(&[Spec(
        "A",
        100,
        vec![raw(0x1b, hd(100, 0, 0), 0), key(0, 0, 0)],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named("A").expect("cue");
    assert_eq!(
        bank.cue_timeline_modelled(&cue, &[], hd_model([0; 4]))
            .bends,
        vec![100]
    );
    assert_eq!(
        bank.cue_timeline(&cue).bends,
        vec![0],
        "Pulse reads the low byte"
    );
}

#[test]
fn collisions_ship_or_wall_follows_variable_one_when_the_replacing_child_is_walked() {
    // `.COLLISIONS`: `22 01 01 00 / 08 ship / 22 01 01 01 / 08 wall`.
    let data = build(&[
        Spec(
            ".COLLISIONS",
            100,
            vec![
                raw(0x22, hd(1, 1, 0), 0),
                Grain::ReplaceChild {
                    cue: 1,
                    delay: 0,
                    volume: 127,
                },
                raw(0x22, hd(1, 1, 1), 0),
                Grain::ReplaceChild {
                    cue: 2,
                    delay: 0,
                    volume: 127,
                },
            ],
        ),
        Spec("c_CShipShip", 100, vec![key(0, 0, 0)]),
        Spec("c_CShipWall", 100, vec![key(16, 0, 0)]),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let cue = bank.cue_named(".COLLISIONS").expect("cue");
    // The race model leaves `0x08` unread: severity is unknown, so the cue
    // keeps its flat pick.
    let race = bank.cue_timeline_modelled(&cue, &[], hd_model([0, 1, 0, 0]));
    assert!(!race.is_complete());
    let walked = WalkModel {
        globals: Some([0; super::GLOBAL_SLOTS]),
        ..hd_model([0, 1, 0, 0])
    };
    let wall = bank.cue_timeline_modelled(&cue, &[], walked);
    assert_eq!(grains(&wall), vec![(0, 16, 0)]);
    let ship = bank.cue_timeline_modelled(
        &cue,
        &[],
        WalkModel {
            cue_variables: Some([0; 4]),
            ..walked
        },
    );
    assert_eq!(grains(&ship), vec![(0, 0, 0)]);
}
