//! Hardware-free tests for the child-sound grain, every bank assembled byte by
//! byte. The corpus half (1,153 indexed and 300 named children) is
//! `crates/formats/tests/sblk_child_ground_truth.rs`.

use crate::sblk::{
    Bank, COMMAND_LEN, CUE_LEN, HAS_NAME_TABLE, HEADER_LEN, MAGIC, NAME_BUCKETS_AT, NAME_ENTRY_LEN,
    SBLK_HEADER_LEN, SECTION_LEN, VERSION,
};

use super::{CHILD_RECORD_LEN, MAX_CHILD_DEPTH};

/// One grain of a hand-built bank.
enum Grain {
    /// A key-on binding `(offset, length)` in the waveform section.
    KeyOn(u32, u32),
    /// A child played by cue index.
    ByIndex(u32),
    /// A child played by name.
    ByName(&'static str),
}

/// A cue: its name, and the grains it owns.
struct Spec(&'static str, Vec<Grain>);

/// Assembles a bank whose cues own contiguous runs of the command table.
/// Assembles a bank whose cues own contiguous runs of the command table.
/// Records sit at a [`CHILD_RECORD_LEN`] stride a waveform descriptor fits
/// inside; the opcode that reaches them tells the two kinds apart, as on disc.
fn build(specs: &[Spec]) -> Vec<u8> {
    let cues = u16::try_from(specs.len()).expect("few cues");
    let grains: usize = specs.iter().map(|s| s.1.len()).sum();
    let cue_bytes = usize::from(cues) * CUE_LEN;
    let command_offset = SBLK_HEADER_LEN + cue_bytes;
    let parameter_offset = command_offset + grains * COMMAND_LEN;
    let name_offset = parameter_offset + grains * CHILD_RECORD_LEN;
    let entries_at = name_offset + 0x98;
    let block_len = entries_at + (specs.len() + 1) * NAME_ENTRY_LEN;
    let waveform_len = 16 * grains.max(1);

    let mut block = vec![0u8; block_len];
    block[..4].copy_from_slice(MAGIC);
    block[4..8].copy_from_slice(&VERSION.to_le_bytes());
    block[0x08..0x0c].copy_from_slice(&HAS_NAME_TABLE.to_le_bytes());
    block[0x16..0x18].copy_from_slice(&cues.to_le_bytes());
    let commands = u16::try_from(grains).expect("few grains");
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
        block[cue + 0x04] = u8::try_from(spec.1.len()).expect("short run");
        let byte_offset = u32::try_from(command * COMMAND_LEN).expect("small bank");
        block[cue + 0x08..cue + 0x0c].copy_from_slice(&byte_offset.to_le_bytes());

        for grain in &spec.1 {
            let record = parameter_offset + command * CHILD_RECORD_LEN;
            let operand = u32::try_from(command * CHILD_RECORD_LEN).expect("small bank");
            let opcode: u32 = match grain {
                Grain::KeyOn(offset, length) => {
                    block[record + 0x10..record + 0x14].copy_from_slice(&offset.to_le_bytes());
                    block[record + 0x14..record + 0x18].copy_from_slice(&length.to_le_bytes());
                    0x01
                }
                Grain::ByIndex(child) => {
                    block[record + 0x0c..record + 0x10].copy_from_slice(&child.to_le_bytes());
                    0x08
                }
                Grain::ByName(name) => {
                    block[record + 0x0c..record + 0x10]
                        .copy_from_slice(&super::CHILD_BY_NAME.to_le_bytes());
                    block[record + 0x10..record + 0x10 + name.len()]
                        .copy_from_slice(name.as_bytes());
                    0x05
                }
            };
            block[record..record + 4].copy_from_slice(&0x7fu32.to_le_bytes());
            let at = command_offset + command * COMMAND_LEN;
            block[at..at + 4].copy_from_slice(&(opcode << 24 | operand).to_le_bytes());
            command += 1;
        }
    }

    block[name_offset + 8..name_offset + 12].copy_from_slice(&0x98u32.to_le_bytes());
    for bucket in 0..64usize {
        let head = u16::try_from(if bucket == 0 { 0 } else { specs.len() }).expect("few cues");
        let at = name_offset + NAME_BUCKETS_AT + bucket * 2;
        block[at..at + 2].copy_from_slice(&head.to_le_bytes());
    }
    for (index, spec) in specs.iter().enumerate() {
        let at = entries_at + index * NAME_ENTRY_LEN;
        block[at..at + spec.0.len()].copy_from_slice(spec.0.as_bytes());
        let cue = u16::try_from(index).expect("few cues");
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

/// The shape Wipeout HD's `.COLLISIONS` has: a parent that binds nothing and
/// two leaves that do, one reached by index and one by name.
fn collisions() -> Vec<u8> {
    build(&[
        Spec(
            ".PARENT",
            vec![Grain::ByIndex(1), Grain::ByName("LEAF_TWO")],
        ),
        Spec("LEAF_ONE", vec![Grain::KeyOn(0, 16)]),
        Spec("LEAF_TWO", vec![Grain::KeyOn(16, 32)]),
    ])
}

#[test]
fn a_parent_binds_nothing_of_its_own_and_names_its_children() {
    let data = collisions();
    let bank = Bank::parse(&data).expect("parse");
    let parent = bank.cue_named(".PARENT").expect("the parent");

    // The cue plays and owns grains, yet `cue_sounds` is empty.
    assert!(parent.plays());
    assert_eq!(parent.commands, 2);
    assert!(bank.cue_sounds(&parent).is_empty());

    let children = bank.cue_children(&parent);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].cue, Some(1));
    assert!(children[0].name.is_empty());
    assert!(!children[0].is_named());
    assert_eq!(children[1].cue, None);
    assert_eq!(children[1].name, "LEAF_TWO");
    assert!(children[1].is_named());
    // Both forms carry the record's volume, the same field either way.
    assert!(children.iter().all(|c| c.volume == 0x7f));
    assert!(children.iter().all(super::Child::is_resolvable));
}

#[test]
fn the_tree_walk_reaches_both_leaves_in_command_order() {
    let data = collisions();
    let bank = Bank::parse(&data).expect("parse");
    let parent = bank.cue_named(".PARENT").expect("the parent");

    let sounds = bank.cue_tree_sounds(&parent);
    assert_eq!(sounds.len(), 2);
    // Sorted by command index, so the frontier's pop order cannot show.
    assert!(sounds[0].command < sounds[1].command);
    assert_eq!(sounds[0].offset, 0);
    assert_eq!(sounds[1].offset, 16);
}

#[test]
fn a_leaf_walks_to_exactly_what_it_binds_itself() {
    // On PSP, PS2 and Pure no wired cue has children, so this must equal
    // `cue_sounds` there, not merely usually agree.
    let data = collisions();
    let bank = Bank::parse(&data).expect("parse");
    for name in ["LEAF_ONE", "LEAF_TWO"] {
        let cue = bank.cue_named(name).expect("a leaf");
        assert!(bank.cue_children(&cue).is_empty());
        assert_eq!(bank.cue_tree_sounds(&cue), bank.cue_sounds(&cue));
    }
}

#[test]
fn a_cycle_terminates_and_yields_each_waveform_once() {
    // Nothing on any disc does this, but a walk trusting that would hang.
    let data = build(&[
        Spec("A", vec![Grain::KeyOn(0, 16), Grain::ByIndex(1)]),
        Spec("B", vec![Grain::KeyOn(16, 16), Grain::ByIndex(0)]),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let a = bank.cue_named("A").expect("A");
    let sounds = bank.cue_tree_sounds(&a);
    assert_eq!(sounds.len(), 2, "a cycle replayed a leaf");
}

#[test]
fn a_diamond_yields_its_shared_leaf_once() {
    let data = build(&[
        Spec("TOP", vec![Grain::ByIndex(1), Grain::ByIndex(2)]),
        Spec("LEFT", vec![Grain::ByIndex(3)]),
        Spec("RIGHT", vec![Grain::ByIndex(3)]),
        Spec("SHARED", vec![Grain::KeyOn(0, 16)]),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let top = bank.cue_named("TOP").expect("TOP");
    assert_eq!(bank.cue_tree_sounds(&top).len(), 1);
}

#[test]
fn an_index_past_the_cue_table_is_dropped_rather_than_carried() {
    // `weapons_det.bnk` on HD holds index 65 in a 55-cue bank, what SCREAM's
    // "snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d" prints about.
    let data = build(&[
        Spec("ROOT", vec![Grain::ByIndex(99)]),
        Spec("LEAF", vec![Grain::KeyOn(0, 16)]),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let root = bank.cue_named("ROOT").expect("ROOT");
    let children = bank.cue_children(&root);
    assert_eq!(children.len(), 1, "the grain itself is still reported");
    assert_eq!(children[0].cue, None);
    assert!(children[0].name.is_empty());
    assert!(!children[0].is_resolvable());
    assert!(bank.resolve_child(&children[0]).is_none());
    assert!(bank.cue_tree_sounds(&root).is_empty());
}

#[test]
fn a_name_this_bank_does_not_hold_resolves_to_nothing() {
    // `env0_det.bnk` asks for ".COLLISIONS", which lives in `shiphd.bnk`.
    // Silence, not an error: the record is well formed.
    let data = build(&[
        Spec("ROOT", vec![Grain::ByName("SOMEWHERE_ELSE")]),
        Spec("LEAF", vec![Grain::KeyOn(0, 16)]),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let root = bank.cue_named("ROOT").expect("ROOT");
    let children = bank.cue_children(&root);
    assert_eq!(children[0].name, "SOMEWHERE_ELSE");
    assert!(children[0].is_resolvable(), "the record is well formed");
    assert!(bank.resolve_child(&children[0]).is_none());
    assert!(bank.cue_tree_sounds(&root).is_empty());
}

#[test]
fn the_walk_stops_at_the_depth_cap() {
    // A chain one longer than the cap, so the last leaf is the one dropped.
    let mut specs = Vec::new();
    for step in 0..=MAX_CHILD_DEPTH {
        let child = u32::try_from(step + 1).expect("small");
        specs.push(Spec(
            "LINK",
            vec![Grain::KeyOn(0, 16), Grain::ByIndex(child)],
        ));
    }
    specs.push(Spec("END", vec![Grain::KeyOn(0, 16)]));
    let data = build(&specs);
    let bank = Bank::parse(&data).expect("parse");
    let root = bank.cue(0).expect("the root");
    // Depth 0 through MAX inclusive is MAX + 1 cues; the one past is not
    // visited, so its key-on is not collected.
    assert_eq!(bank.cue_tree_sounds(&root).len(), MAX_CHILD_DEPTH + 1);
}

#[test]
fn a_record_carrying_both_an_index_and_a_name_is_reported_as_both() {
    // The corpus assertion is "no record carries both forms"; deriving the name
    // from "the index did not resolve" would make that unfalsifiable, so this
    // reader has to be able to express the case.
    let mut data = build(&[
        Spec("ROOT", vec![Grain::ByIndex(1)]),
        Spec("LEAF", vec![Grain::KeyOn(0, 16)]),
    ]);
    let block_at = HEADER_LEN + 2 * SECTION_LEN;
    let parameter_offset = Bank::parse(&data).expect("parse").parameter_offset as usize;
    let name_at = block_at + parameter_offset + 0x10;
    data[name_at..name_at + 4].copy_from_slice(b"BOTH");

    let bank = Bank::parse(&data).expect("parse");
    let root = bank.cue_named("ROOT").expect("ROOT");
    let children = bank.cue_children(&root);
    assert_eq!(children[0].cue, Some(1), "the index is still read");
    assert_eq!(children[0].name, "BOTH", "the name is read independently");
    assert!(children[0].is_both());
    // The index wins, as the runtime's load-time fixup leaves it: the resolved
    // form, with the name what it resolved from.
    assert_eq!(bank.resolve_child(&children[0]).map(|c| c.index), Some(1));
}

#[test]
fn an_out_of_range_index_is_not_mistaken_for_a_named_child() {
    // Folding "the index did not resolve" into "read the name" would make this
    // grain look like a named child of whatever the name field held.
    let data = build(&[
        Spec("ROOT", vec![Grain::ByIndex(99)]),
        Spec("LEAF", vec![Grain::KeyOn(0, 16)]),
    ]);
    let bank = Bank::parse(&data).expect("parse");
    let root = bank.cue_named("ROOT").expect("ROOT");
    let child = &bank.cue_children(&root)[0];
    assert!(!child.is_both());
    assert!(!child.is_named());
    assert!(!child.is_resolvable());
}
