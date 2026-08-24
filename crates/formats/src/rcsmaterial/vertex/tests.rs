//! Encodes instructions the way NV40 stores them and reads them back, plus a
//! whole synthetic block, so the framing and the flip test are both covered
//! without a disc.

use super::*;

/// `~crc32("Uv1")`, the attribute a circuit's diffuse coordinate arrives in.
const UV1: u32 = 0x4272_14fc;

/// Builds the four dwords of one vector instruction.
///
/// Only the fields [`Instruction::decode`] reads are filled; every other bit
/// stays zero, which is what makes a test that passes here a statement about
/// those fields rather than about the whole encoding.
fn encode(vec_op: u8, input: u8, sources: [u32; 3], dest: u32, dest_is_output: bool) -> [u32; 4] {
    let mut d0 = 0u32;
    if dest_is_output {
        d0 |= 1 << 30;
    }
    let d1 = (u32::from(vec_op) << 22) | (u32::from(input) << 8) | (sources[0] >> 9);
    let d2 = ((sources[0] & 0x1ff) << 23) | ((sources[1] & 0x1_ffff) << 6) | (sources[2] >> 11);
    let d3 = ((sources[2] & 0x7ff) << 21) | ((dest & 0x1f) << 2);
    [d0, d1, d2, d3]
}

/// A source field naming vertex input register `n`, optionally negated.
fn input_source(negated: bool) -> u32 {
    SOURCE_INPUT | if negated { SOURCE_NEGATE } else { 0 }
}

/// A source field naming a constant register, which no flip test may match.
fn constant_source() -> u32 {
    3
}

/// `o[TC6]`, the varying `track_wall` writes its flipped coordinate into.
const TC6: u32 = FIRST_TEXCOORD_DEST + 6;

const ADD: u8 = 0x03;
const MOV: u8 = 0x01;
const MUL: u8 = 0x02;

fn program(instructions: Vec<[u32; 4]>) -> Program {
    Program {
        attributes: vec![(UV1, 3)],
        instructions: instructions.into_iter().map(Instruction::decode).collect(),
    }
}

#[test]
fn the_flip_track_wall_writes_is_read_as_one() {
    // ADD o[TC6].y, -v[3].yyyy, c[206].yyyy
    let flip = encode(
        ADD,
        3,
        [input_source(true), 0, constant_source()],
        TC6,
        true,
    );
    assert!(program(vec![flip]).flips(UV1));
}

#[test]
fn a_coordinate_passed_through_is_not_a_flip() {
    // MOV o[TC6].x, v[3].xxxx
    let pass = encode(MOV, 3, [input_source(false), 0, 0], TC6, true);
    assert!(!program(vec![pass]).flips(UV1));
}

#[test]
fn a_negation_into_a_temporary_is_not_a_flip() {
    let into_temp = encode(
        ADD,
        3,
        [input_source(true), 0, constant_source()],
        TC6,
        false,
    );
    assert!(!program(vec![into_temp]).flips(UV1));
}

#[test]
fn a_negation_into_the_position_is_not_a_flip() {
    let into_position = encode(ADD, 3, [input_source(true), 0, constant_source()], 0, true);
    assert!(!program(vec![into_position]).flips(UV1));
}

#[test]
fn a_negation_of_a_different_attribute_is_not_this_one_s_flip() {
    let other = encode(
        ADD,
        2,
        [input_source(true), 0, constant_source()],
        TC6,
        true,
    );
    assert!(!program(vec![other]).flips(UV1));
}

/// `MUL` reads slots 0 and 1, so a negate parked in slot 2 is a field the
/// opcode never looks at - the false positive [`source_slots`] exists to stop.
#[test]
fn a_negate_in_a_slot_the_opcode_does_not_read_is_ignored() {
    let stale = encode(MUL, 3, [0, 0, input_source(true)], TC6, true);
    assert!(!program(vec![stale]).flips(UV1));
}

#[test]
fn an_attribute_the_block_never_declares_does_not_flip() {
    let flip = encode(
        ADD,
        3,
        [input_source(true), 0, constant_source()],
        TC6,
        true,
    );
    assert!(!program(vec![flip]).flips(0xdead_beef));
}

/// The whole framing: `SHO\x08`, the eight `u16` header, an attribute table,
/// and a code stream that only decodes at one of the probed alignments.
#[test]
fn a_block_is_framed_read_and_its_attributes_recovered() {
    let attributes_at = 0x40usize;
    let program_at = 0x60usize;
    let code_at = 0x10usize;
    let mut data = vec![0u8; 0x200];
    data[..4].copy_from_slice(b"SHO\x08");
    let put16 = |data: &mut Vec<u8>, at: usize, v: u16| {
        data[at..at + 2].copy_from_slice(&v.to_be_bytes());
    };
    put16(&mut data, 8 + 2, 1); // attribute count
    put16(&mut data, 8 + 8, attributes_at as u16);
    put16(&mut data, 8 + 14, program_at as u16);
    data[attributes_at..attributes_at + 4].copy_from_slice(&UV1.to_be_bytes());
    data[attributes_at + 4..attributes_at + 8].copy_from_slice(&3u32.to_be_bytes());
    put16(&mut data, program_at, 1); // one instruction
    let flip = encode(
        ADD,
        3,
        [input_source(true), 0, constant_source()],
        TC6,
        true,
    );
    for (n, word) in flip.iter().enumerate() {
        let at = program_at + code_at + n * 4;
        data[at..at + 4].copy_from_slice(&word.to_be_bytes());
    }

    let read = Program::parse(&data, 0).expect("the block frames");
    assert_eq!(read.attributes, vec![(UV1, 3)]);
    assert_eq!(read.attribute_slot(UV1), Some(3));
    assert!(read.flips(UV1));
}

#[test]
fn something_that_is_not_a_block_reads_as_nothing() {
    assert_eq!(Program::parse(b"not a block at all", 0), None);
}
