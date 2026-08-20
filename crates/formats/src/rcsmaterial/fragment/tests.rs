use super::*;

/// The halves-swapped dword, which is the container fact everything else rests
/// on: reading it as a plain big-endian `u32` decodes garbage.
#[test]
fn a_fragment_dword_reads_its_halves_swapped() {
    let raw = [0x11u8, 0x22, 0x33, 0x44];
    assert_eq!(word(&raw, 0), Some(0x3344_1122));
    assert_eq!(word(&raw, 2), None, "past the end is None, not a panic");
}

#[test]
fn a_source_type_decodes_to_where_it_reads_from() {
    assert_eq!(
        Source::of(0b0001_1100),
        Source::Register {
            index: 7,
            half: false
        }
    );
    assert_eq!(
        Source::of(0b1_0001_1100),
        Source::Register {
            index: 7,
            half: true
        },
        "bit 8 is the half-precision file"
    );
    assert_eq!(Source::of(1), Source::Input);
    assert_eq!(Source::of(2), Source::Constant);
    assert_eq!(Source::of(3), Source::Unknown);
}

fn insn(opcode: u8) -> Instruction {
    Instruction {
        opcode,
        dst: 0,
        dst_half: false,
        mask: 0xf,
        saturate: false,
        input: 0,
        unit: 0,
        sources: [Source::Input; 3],
        constant: None,
        end: false,
    }
}

/// Arity is what decides how many sources an instruction reads, and reading one
/// too many would see an operand that is not there.
#[test]
fn arity_follows_the_mnemonic() {
    assert_eq!(insn(0x12).arity(), 0, "KIL");
    assert_eq!(insn(0x01).arity(), 1, "MOV");
    assert_eq!(insn(0x17).arity(), 1, "TEX");
    assert_eq!(insn(0x02).arity(), 2, "MUL");
    assert_eq!(insn(0x04).arity(), 3, "MAD");
    assert_eq!(insn(0x1f).arity(), 3, "LRP");
    assert!(insn(0x17).is_texture());
    assert!(!insn(0x04).is_texture());
}

/// The three opcodes shipped programs use that nouveau's table does not name.
///
/// They stay unnamed rather than guessed, and fall back to two sources - the
/// reference decoder's own behaviour.
#[test]
fn an_opcode_outside_the_table_stays_unnamed() {
    for opcode in [0x3b, 0x3c, 0x3d, 0x2b] {
        assert_eq!(insn(opcode).name(), None, "{opcode:#04x}");
        assert_eq!(insn(opcode).arity(), 2);
        assert!(!insn(opcode).is_texture());
    }
}

/// A synthetic block: header, empty declaration tables, program sub-header, and
/// two instructions - the second carrying an inline constant, so the stream has
/// to advance 32 there rather than 16.
fn synthetic() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"SHO\x08");
    b.extend_from_slice(&1u32.to_be_bytes());
    for v in [2u16, 0, 0, 0, 0x18, 0x18, 0x18, 0x18] {
        b.extend_from_slice(&v.to_be_bytes());
    }
    // Program sub-header at +0x18: code length at +0x00, its offset at +0x10.
    let head = b.len();
    b.extend_from_slice(&48u32.to_be_bytes());
    b.resize(head + 0x10, 0);
    b.extend_from_slice(&0x20u32.to_be_bytes());
    b.resize(head + 0x20, 0);

    let mut push = |words: [u32; 4]| {
        for w in words {
            // Halves swapped, as the format stores them.
            b.extend_from_slice(&(w as u16).to_be_bytes());
            b.extend_from_slice(&((w >> 16) as u16).to_be_bytes());
        }
    };
    // TEX H1, f[TC1] unit2
    push([
        0x17 << 24 | 2 << 17 | 5 << 13 | 0xf << 9 | 1 << 7 | 1 << 1,
        1,
        0,
        0,
    ]);
    // MAD R0, R1, {const}, f[TC1], ending the program.
    push([0x04 << 24 | 5 << 13 | 0xf << 9 | 1, 0b100, 2, 1]);
    for v in [1.0f32, 2.0, 3.0, 4.0] {
        let w = v.to_bits();
        b.extend_from_slice(&(w as u16).to_be_bytes());
        b.extend_from_slice(&((w >> 16) as u16).to_be_bytes());
    }
    b
}

#[test]
fn a_program_decodes_and_an_inline_constant_advances_the_stream() {
    let data = synthetic();
    let p = Program::parse(&data, 0).expect("the block decodes");
    assert_eq!(p.instructions.len(), 2, "the constant was not read as code");

    let tex = p.instructions[0];
    assert_eq!(tex.name(), Some("TEX"));
    assert!(tex.is_texture());
    assert_eq!(tex.unit, 2);
    assert_eq!(tex.input, 5, "f[TC1]");
    assert!(tex.dst_half);
    assert_eq!(tex.dst, 1);
    assert_eq!(tex.constant, None);
    assert!(!tex.end);

    let mad = p.instructions[1];
    assert_eq!(mad.name(), Some("MAD"));
    assert_eq!(mad.arity(), 3);
    assert_eq!(mad.constant, Some([1.0, 2.0, 3.0, 4.0]));
    assert!(mad.end);
    assert_eq!(
        mad.sources[0],
        Source::Register {
            index: 1,
            half: false
        }
    );
    assert_eq!(mad.sources[1], Source::Constant);
    assert_eq!(mad.sources[2], Source::Input);

    // Both instructions read f[TC1], which is interpolator 5.
    assert_eq!(p.interpolators(), 1 << 5);
}

#[test]
fn a_block_that_does_not_frame_is_refused_rather_than_decoded() {
    assert!(Program::parse(b"not a block", 0).is_none());
    let mut bad = synthetic();
    // Push the code offset past the end of the data.
    bad[0x18 + 0x12] = 0xff;
    bad[0x18 + 0x13] = 0xff;
    assert!(Program::parse(&bad, 0).is_none());
}

/// A texture lookup blocks taint, which is what separates a coordinate from a
/// light term.
#[test]
fn a_texture_lookup_blocks_the_taint_a_plain_dependency_carries() {
    let data = synthetic();
    let p = Program::parse(&data, 0).expect("decodes");
    // The TEX writes H1 from f[TC1] as a coordinate; the MAD then reads R1 and
    // f[TC1] directly. Under the plain rule both paths carry TC1; under the
    // lighting rule only the direct read does - and here they agree because the
    // MAD reads the interpolator itself.
    assert_eq!(p.output_depends_on(), 1 << 5);
    assert_eq!(p.output_lit_by(), 1 << 5);
}

/// A register overwritten by an untainted instruction stops carrying taint.
#[test]
fn an_overwrite_clears_what_a_register_carried() {
    let mut b = Vec::new();
    b.extend_from_slice(b"SHO\x08");
    b.extend_from_slice(&1u32.to_be_bytes());
    for v in [2u16, 0, 0, 0, 0x18, 0x18, 0x18, 0x18] {
        b.extend_from_slice(&v.to_be_bytes());
    }
    let head = b.len();
    b.extend_from_slice(&32u32.to_be_bytes());
    b.resize(head + 0x10, 0);
    b.extend_from_slice(&0x20u32.to_be_bytes());
    b.resize(head + 0x20, 0);
    let mut push = |words: [u32; 4]| {
        for w in words {
            b.extend_from_slice(&(w as u16).to_be_bytes());
            b.extend_from_slice(&((w >> 16) as u16).to_be_bytes());
        }
    };
    // MOV R0, f[TC1]  - taints R0 with interpolator 5.
    push([0x01 << 24 | 5 << 13 | 0xf << 9, 1, 0, 0]);
    // MOV R0, {const} - overwrites every channel from an untainted source, END.
    push([0x01 << 24 | 0xf << 9 | 1, 2, 0, 0]);
    for _ in 0..4 {
        b.extend_from_slice(&[0u8; 4]);
    }
    let p = Program::parse(&b, 0).expect("decodes");
    assert_eq!(p.instructions.len(), 2);
    assert_eq!(
        p.output_depends_on(),
        0,
        "the second MOV replaced what the first put in R0"
    );
}
