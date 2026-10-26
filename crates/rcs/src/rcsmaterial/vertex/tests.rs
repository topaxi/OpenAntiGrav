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
        literals: Vec::new(),
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

/// `encode` for an instruction whose vector half writes temporary `temp`.
fn encode_to_temp(vec_op: u8, input: u8, sources: [u32; 3], temp: u32) -> [u32; 4] {
    let mut words = encode(vec_op, input, sources, 0, false);
    words[0] |= temp << 15;
    words
}

/// A source field naming temporary `n`.
fn temporary_source(n: u32) -> u32 {
    SOURCE_TEMPORARY | (n << 2)
}

#[test]
fn a_negation_into_a_temporary_that_never_reaches_an_output_is_not_a_flip() {
    let into_temp = encode_to_temp(ADD, 3, [input_source(true), 0, constant_source()], 3);
    assert!(!program(vec![into_temp]).flips(UV1));
}

/// `ADD R3.w, -v[3].yyyy, c[208].zzzz` then `MOV o[TC4], R3`: Amphiseum's
/// `track_surface_no_emissive`, whose pit-box floor drew with the picture upside
/// down for as long as only a direct negation counted.
#[test]
fn a_negation_copied_to_an_interpolator_through_a_temporary_is_a_flip() {
    let negate = encode_to_temp(ADD, 3, [input_source(true), 0, constant_source()], 3);
    let copy = encode(MOV, 0, [temporary_source(3), 0, 0], TC6, true);
    assert!(program(vec![negate, copy]).flips(UV1));
}

#[test]
fn a_copy_that_comes_before_the_negation_does_not_count() {
    let negate = encode_to_temp(ADD, 3, [input_source(true), 0, constant_source()], 3);
    let copy = encode(MOV, 0, [temporary_source(3), 0, 0], TC6, true);
    assert!(!program(vec![copy, negate]).flips(UV1));
}

#[test]
fn a_negation_into_one_temporary_is_not_a_copy_of_another() {
    let negate = encode_to_temp(ADD, 3, [input_source(true), 0, constant_source()], 3);
    let copy = encode(MOV, 0, [temporary_source(2), 0, 0], TC6, true);
    assert!(!program(vec![negate, copy]).flips(UV1));
}

/// Talon's `track_surface` loads the same temporary without the negate
/// (`MOV R3.w, v[3].yyyy`) and is upright.
#[test]
fn an_unnegated_load_copied_through_a_temporary_is_not_a_flip() {
    let load = encode_to_temp(MOV, 3, [input_source(false), 0, 0], 3);
    let copy = encode(MOV, 0, [temporary_source(3), 0, 0], TC6, true);
    assert!(!program(vec![load, copy]).flips(UV1));
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

/// `encode` plus the two fields [`Program::clock_rate`] reads on top: the
/// instruction's constant register and its vector write mask.
fn encode_full(
    vec_op: u8,
    (input, constant): (u8, u32),
    sources: [u32; 3],
    (dest, dest_is_output, temp): (u32, bool, u32),
    write_mask: u32,
) -> [u32; 4] {
    let mut words = encode(vec_op, input, sources, dest, dest_is_output);
    words[0] |= temp << 15;
    words[1] |= (constant & 0xff) << 12;
    words[3] |= (write_mask & 0xf) << 13;
    words
}

/// A broadcast swizzle (`.llll`) on a source field.
fn broadcast(source: u32, lane: u32) -> u32 {
    source | (lane * 0b0101_0101) << 8
}

/// `nr_twinblend`'s vertex block #3 (`@0x1240`, Tech de Ra), the two
/// instructions that matter: `MOV R0.w, c[207].yyyy` then
/// `MAD o[TC4].x, R0.wwww, c[208].xxxx, v[8].zzzz`, with `c[207]` the block's
/// literal `(0, 1.875, 0, 0)` and `c[208]` the engine's `time`.
fn twinblend(mad_constant: u32) -> Program {
    const ATTRIBUTE: u32 = 0x0e7e_a5a8;
    let mov = encode_full(
        MOV,
        (0, 207),
        [broadcast(constant_source(), 1), 0, 0],
        (0, false, 0),
        1,
    );
    let mad = encode_full(
        0x04,
        (8, mad_constant),
        [
            broadcast(temporary_source(0), 3),
            broadcast(constant_source(), 0),
            broadcast(input_source(false), 2),
        ],
        (FIRST_TEXCOORD_DEST + 4, true, 0),
        8,
    );
    Program {
        attributes: vec![(ATTRIBUTE, 8)],
        instructions: [mov, mad].into_iter().map(Instruction::decode).collect(),
        literals: vec![(207, [0.0, 1.875, 0.0, 0.0])],
    }
}

#[test]
fn the_twinblend_mask_scroll_reads_its_literal_rate() {
    let rate = twinblend(208).clock_rate(208, 0x0e7e_a5a8);
    assert_eq!(
        rate,
        Some(ClockRate {
            texcoord: 4,
            lane: 0,
            from: 2,
            rate: 1.875
        })
    );
}

#[test]
fn a_multiply_by_another_constant_is_not_the_clock() {
    assert_eq!(twinblend(209).clock_rate(208, 0x0e7e_a5a8), None);
    assert_eq!(twinblend(208).clock_rate(208, 0x1234_5678), None);
}
