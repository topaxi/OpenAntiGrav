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
        swizzles: [[0, 1, 2, 3]; 3],
        constant: None,
        const_slot: None,
        negate: [false; 3],
        cond: 0x727,
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

/// Opcodes neither source names - absent from Mesa's `nvfx_shader.h` and from
/// RPCS3's `FPOpcodes.h` alike. Both fall back to two sources, the reference
/// decoder's own behaviour.
#[test]
fn an_opcode_outside_the_table_stays_unnamed() {
    for opcode in [0x30, 0x32] {
        assert_eq!(insn(opcode).name(), None, "{opcode:#04x}");
        assert_eq!(insn(opcode).arity(), 2);
        assert!(!insn(opcode).is_texture());
    }
}

/// `RSX_FP_OPCODE_FENCB`, RPCS3's `FPOpcodes.h`: every one of its 3,115 uses
/// on the HD disc writes destination register 63, the same invariant
/// [`opcode_0x3d_is_named_fenct`] rests on.
#[test]
fn opcode_0x3e_is_named_fencb() {
    assert_eq!(insn(0x3e).name(), Some("FENCB"));
}

/// The RPCS3-only numbers that never occur in shipped HD code are named off
/// that header alone - `0x2b` is `BEM` there, and `0x39` is `NRM`, which is
/// also why `0x3b` was never the normalize.
#[test]
fn rpcs3_only_opcodes_take_rpcs3_names() {
    assert_eq!(insn(0x2b).name(), Some("BEM"));
    assert_eq!(insn(0x38).name(), Some("DP2"));
    assert_eq!(insn(0x39).name(), Some("NRM"));
}

/// `NVFX_FP_OP_OPCODE_LITEX2_NV40` - confirmed against Mesa's own
/// `nvfx_shader.h` 2026-09-03, not carried over unverified from the
/// reference script. One source operand (`nvfx_fragprog.c`'s
/// `TGSI_OPCODE_LIT` emission reads a single temporary), and absent from
/// every fragment block on this disc (`hd_litex2_census.rs`, 0 of 76,358).
#[test]
fn opcode_0x3c_is_named_lit_ex2() {
    assert_eq!(insn(0x3c).name(), Some("LIT_EX2_NV40"));
    assert_eq!(insn(0x3c).arity(), 1);
    assert!(!insn(0x3c).is_texture());
}

/// `RSX_FP_OPCODE_DIVSQ` (`rpcs3/Emu/RSX/Program/Assembler/FPOpcodes.h`,
/// GPLv2), `a / sqrt(b)` - confirmed 2026-09-25 against disc-wide use
/// (`hd_op3b_op3d_census.rs`: 183,623 uses, splitting into the
/// `DP3`-then-`op3B` normalize shape and the same-register `op3B(x,x) =
/// sqrt(x)` shape `renderer.md`'s prior `NRM` hypothesis could not explain).
/// Confidence 84. Two source operands, the reference decoder's default.
#[test]
fn opcode_0x3b_is_named_divsq() {
    assert_eq!(insn(0x3b).name(), Some("DIVSQ"));
    assert_eq!(insn(0x3b).arity(), 2);
    assert!(!insn(0x3b).is_texture());
}

/// `RSX_FP_OPCODE_FENCT` (same source), "Fence T?" - RPCS3's own hedge on the
/// exact meaning. What is confirmed, disc-wide: every one of 59,256 uses
/// writes destination register 63 (the 6-bit field's all-ones value,
/// matching `nvfx_shader.h`'s own `NV40_FP_OP_OUT_NONE` bit, independently
/// checked set on a hand sample) - zero counterexamples. Confidence 90 on
/// "writes no real destination", not on "fence" specifically.
#[test]
fn opcode_0x3d_is_named_fenct() {
    assert_eq!(insn(0x3d).name(), Some("FENCT"));
    assert_eq!(insn(0x3d).arity(), 2);
    assert!(!insn(0x3d).is_texture());
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

/// **A program that ends on `MOV H0.w, {const}` still reports its colour.**
///
/// The bug this fixes: taking the taint standing on the *final* instruction's
/// channels reports nothing whenever a program finishes by writing only alpha
/// from a constant, which is how most of Talon's Junction's lit materials end.
/// Measured over seven of them, every such program answered `0x0000` and the
/// colour written several instructions earlier was discarded.
#[test]
fn a_trailing_alpha_write_does_not_discard_the_colour() {
    let mut b = Vec::new();
    b.extend_from_slice(b"SHO\x08");
    b.extend_from_slice(&1u32.to_be_bytes());
    for v in [2u16, 0, 0, 0, 0x18, 0x18, 0x18, 0x18] {
        b.extend_from_slice(&v.to_be_bytes());
    }
    let head = b.len();
    b.extend_from_slice(&48u32.to_be_bytes());
    b.resize(head + 0x10, 0);
    b.extend_from_slice(&0x20u32.to_be_bytes());
    b.resize(head + 0x20, 0);
    let mut push = |words: [u32; 4]| {
        for w in words {
            b.extend_from_slice(&(w as u16).to_be_bytes());
            b.extend_from_slice(&((w >> 16) as u16).to_be_bytes());
        }
    };
    // ADD H0.xyz, f[TC1], f[TC1] - the colour, from interpolator 5.
    push([0x03 << 24 | 5 << 13 | 0b111 << 9, 1, 1, 0]);
    // MOV H0.w, {const} - alpha only, from an untainted source, END.
    push([0x01 << 24 | 0b1000 << 9 | 1, 2, 0, 0]);
    for _ in 0..4 {
        b.extend_from_slice(&[0u8; 4]);
    }
    let p = Program::parse(&b, 0).expect("decodes");
    assert_eq!(p.instructions.len(), 2);
    assert_eq!(
        p.output_depends_on(),
        1 << 5,
        "the trailing alpha write hid the colour"
    );
    assert_eq!(p.output_lit_by(), 1 << 5);
}

/// The inverse of [`swizzle_of`], for building a swizzled register source -
/// every test below that needs one goes through this rather than hand-packing
/// bits a second time.
fn register_source(index: u8, half: bool, lanes: [u8; 4]) -> u32 {
    let mut bits = u32::from(index) << 2;
    if half {
        bits |= 1 << 8;
    }
    for (i, &lane) in lanes.iter().enumerate() {
        bits |= u32::from(lane) << (9 + 2 * i);
    }
    bits
}

#[test]
fn swizzle_of_decodes_the_documented_bit_packing() {
    assert_eq!(
        swizzle_of(register_source(0, false, [0, 1, 2, 3])),
        [0, 1, 2, 3]
    );
    assert_eq!(
        swizzle_of(register_source(0, false, [3, 3, 3, 3])),
        [3, 3, 3, 3],
        ".wwww"
    );
    assert_eq!(
        swizzle_of(0),
        [0, 0, 0, 0],
        "no swizzle bits set decodes as .xxxx, not as the identity"
    );
}

#[test]
fn texel_merge_is_untraced_identity_mixed_absorbing_and_channel_only_on_agreement() {
    let a = Texel::Unit {
        unit: 1,
        channel: Some(2),
    };
    assert_eq!(
        Texel::Untraced.merge(a),
        a,
        "Untraced is the identity on the left"
    );
    assert_eq!(a.merge(Texel::Untraced), a, "and on the right");
    assert_eq!(Texel::Mixed.merge(a), Texel::Mixed, "Mixed absorbs");
    assert_eq!(a.merge(Texel::Mixed), Texel::Mixed);
    assert_eq!(
        a.merge(Texel::Unit {
            unit: 1,
            channel: Some(0)
        }),
        Texel::Unit {
            unit: 1,
            channel: None
        },
        "same unit, disagreeing channel: the unit survives, the channel does not"
    );
    assert_eq!(
        a.merge(Texel::Unit {
            unit: 0,
            channel: Some(2)
        }),
        Texel::Mixed,
        "different units is Mixed even when the channel happens to agree"
    );
}

/// The bug the swizzle field exists to catch: a source read `.wwww` puts its
/// **own** lane 3 in every output lane it feeds, not the lane the destination
/// happens to write. "This project drew the second [texture] for a year
/// because nothing read the first" - see [`Program::output_texels`].
#[test]
fn a_swizzled_source_reads_its_own_lane_not_the_destinations() {
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
    // TEX H1, f[TC0] unit1 - H1's four lanes are unit 1's four channels.
    push([
        0x17 << 24 | 1 << 17 | 4 << 13 | 0xf << 9 | 1 << 7 | 1 << 1,
        1,
        0,
        0,
    ]);
    // MOV H0.x, H1.wwww, END - reads unit 1's channel 3, not its channel 0.
    push([
        0x01 << 24 | 1 << 9 | 1 << 7 | 1,
        register_source(1, true, [3, 3, 3, 3]),
        0,
        0,
    ]);
    let p = Program::parse(&b, 0).expect("decodes");
    assert_eq!(p.instructions.len(), 2);
    assert_eq!(
        p.output_texels()[0],
        Texel::Unit {
            unit: 1,
            channel: Some(3)
        },
        "the .wwww swizzle should read the sampled alpha channel, not the red one"
    );
}

/// **Half and full registers are two files, not one** - the decision
/// [`Program::output_texels`]'s own doc comment stakes a measured claim on,
/// citing a real block (`talons_junction/materials/clouds.rcsmaterial`,
/// `0x1540`): writing `R2` must not disturb `H2`. Reproduced in miniature - a
/// `TEX` feeds `H2`, an unrelated `MOV` from a constant clobbers `R2`'s same
/// channel, and the final instruction reads `H2` back. Under one shared file
/// the clobber reaches `H2` and the colour comes out [`Texel::Untraced`];
/// kept apart, as here, it survives.
#[test]
fn a_half_register_and_its_same_numbered_full_register_are_different_storage() {
    let mut b = Vec::new();
    b.extend_from_slice(b"SHO\x08");
    b.extend_from_slice(&1u32.to_be_bytes());
    for v in [2u16, 0, 0, 0, 0x18, 0x18, 0x18, 0x18] {
        b.extend_from_slice(&v.to_be_bytes());
    }
    let head = b.len();
    b.extend_from_slice(&64u32.to_be_bytes());
    b.resize(head + 0x10, 0);
    b.extend_from_slice(&0x20u32.to_be_bytes());
    b.resize(head + 0x20, 0);
    let mut push = |words: [u32; 4]| {
        for w in words {
            b.extend_from_slice(&(w as u16).to_be_bytes());
            b.extend_from_slice(&((w >> 16) as u16).to_be_bytes());
        }
    };
    // TEX H2, f[TC0] unit1 - H2 now carries unit 1's sample on every lane.
    push([
        0x17 << 24 | 1 << 17 | 4 << 13 | 0xf << 9 | 1 << 7 | 2 << 1,
        1,
        0,
        0,
    ]);
    // MOV R2.x, {const} - same register ordinal, the full-precision file,
    // and it carries no texel at all.
    push([0x01 << 24 | 1 << 9 | 2 << 1, 2, 0, 0]);
    push([0, 0, 0, 0]); // the constant itself - value irrelevant, it carries no texel
    // MOV H0.x, H2.xxxx, END - reads H2 back.
    push([
        0x01 << 24 | 1 << 9 | 1 << 7 | 1,
        register_source(2, true, [0, 0, 0, 0]),
        0,
        0,
    ]);
    let p = Program::parse(&b, 0).expect("decodes");
    assert_eq!(p.instructions.len(), 3);
    assert_eq!(
        p.output_texels()[0],
        Texel::Unit {
            unit: 1,
            channel: Some(0)
        },
        "R2's write must not have clobbered H2 - they are different storage"
    );
}

/// A register operand, for the `accumulates` cases below.
fn reg(index: u8, half: bool) -> Source {
    Source::Register { index, half }
}

/// Builds the program shape Wipeout HD's emissive family ships:
/// `TEX H1 unit1`, a tint multiply, then `MAD H0.xyz, H0.wwww, H1, H0`.
fn emissive_program(combine: u8, accumulate_into_self: bool) -> Program {
    let mut tex = insn(0x17);
    tex.dst = 1;
    tex.dst_half = true;
    tex.unit = 1;

    let mut tint = insn(0x02); // MUL H1, H1, {const}
    tint.dst = 1;
    tint.dst_half = true;
    tint.sources = [reg(1, true), Source::Constant, Source::Input];

    let mut add = insn(combine);
    add.dst = 0;
    add.dst_half = true;
    add.sources = [
        reg(0, true),
        reg(1, true),
        if accumulate_into_self {
            reg(0, true)
        } else {
            reg(3, true)
        },
    ];
    if !accumulate_into_self {
        // Not merely a different addend: `MAD H0, H0, H1, H3` still names H0
        // on both sides, and that shape is a *multiply* of H0 by the sample.
        add.sources[0] = reg(2, true);
    }
    add.end = true;

    Program {
        declared: Declared::default(),
        parameter_patches: Vec::new(),
        instructions: vec![tex, tint, add],
    }
}

/// The shape the renderer keys on: a unit-1 sample folded through a tint and
/// then **added into** the register it lands in.
#[test]
fn a_sample_added_into_its_own_destination_is_an_accumulate() {
    assert!(emissive_program(0x04, true).accumulates(1), "MAD");
    assert!(emissive_program(0x03, true).accumulates(1), "ADD");
}

/// Multiplying by the sample is not adding it, however the result is used -
/// this is the distinction the whole reading rests on, and `hd_waketrail` is
/// the one material on the disc that lands here.
#[test]
fn a_multiply_is_not_an_accumulate() {
    let mut program = emissive_program(0x04, true);
    program.instructions[2].opcode = 0x02; // MUL
    assert!(!program.accumulates(1));
}

/// A `MAD` whose addend is not its own destination is a combine, not an
/// accumulate.
#[test]
fn a_mad_whose_addend_is_not_its_destination_is_not_an_accumulate() {
    assert!(!emissive_program(0x04, false).accumulates(1));
}

/// `MAD H0, H0, H1, H3` names `H0` on both sides and is still not one: it
/// *scales* `H0` by the sample. Only the third operand is added to.
#[test]
fn a_mad_that_scales_by_the_sample_is_not_an_accumulate() {
    let mut program = emissive_program(0x04, false);
    program.instructions[2].sources[0] = reg(0, true);
    assert!(!program.accumulates(1));
}

/// The question is asked of one unit, and unit 0's sample is not unit 1's.
#[test]
fn the_unit_is_part_of_the_question() {
    let program = emissive_program(0x04, true);
    assert!(program.accumulates(1));
    assert!(!program.accumulates(0), "nothing samples unit 0 here");
}

/// A register that another unit's fetch overwrites stops carrying the first
/// unit's sample - otherwise a program that samples unit 1, discards it, then
/// accumulates an unrelated unit-0 fetch would read as emissive.
#[test]
fn a_later_fetch_clobbers_the_register_it_writes() {
    let mut program = emissive_program(0x04, true);
    let mut clobber = insn(0x17);
    clobber.dst = 1;
    clobber.dst_half = true;
    clobber.unit = 0;
    program.instructions.insert(1, clobber);
    assert!(
        !program.accumulates(1),
        "unit 0's fetch took H1 over before the add"
    );
}

/// Builds a saturated `DP3` feeding `LG2` / `MUL {exponent}` / `EX2` into
/// register `reg` - the `N.H` idiom `renderer.md` reads under every
/// specular term, `pow(N.H, exponent)`.
///
/// `mask` is applied to all four instructions, not only `EX2` - real
/// shipped microcode threads a scalar through one lane throughout a chain
/// like this, and giving the others the default full mask would make them
/// count as colour writes in their own right whenever `mask` picks a
/// non-colour lane, which is not what a scalar `pow` computation is.
fn pow_chain(reg_index: u8, exponent: f32, mask: u8) -> Vec<Instruction> {
    let mut dot = insn(0x05); // DP3, arity 2 - N.H, saturated
    dot.dst = reg_index;
    dot.mask = mask;
    dot.saturate = true;

    let mut lg2 = insn(0x1d); // LG2, arity 1
    lg2.dst = reg_index;
    lg2.mask = mask;
    lg2.sources = [reg(reg_index, false), Source::Input, Source::Input];

    let mut mul = insn(0x02); // MUL, arity 2
    mul.dst = reg_index;
    mul.mask = mask;
    mul.sources = [reg(reg_index, false), Source::Constant, Source::Input];
    mul.constant = Some([exponent, 0.0, 0.0, 0.0]);

    let mut ex2 = insn(0x1c); // EX2, arity 1
    ex2.dst = reg_index;
    ex2.mask = mask;
    ex2.sources = [reg(reg_index, false), Source::Input, Source::Input];

    vec![dot, lg2, mul, ex2]
}

/// The ship's own reading: `pow(N.H, 40)` (`@0x40 LG2 / @0x43 MUL by 40 /
/// @0x46 EX2`, `renderer.md`'s "Ships have no Lambert diffuse either"),
/// multiplied into the register the program's final `MAD` writes as colour.
#[test]
fn a_pow_chain_that_reaches_the_output_returns_its_exponent() {
    let mut instructions = pow_chain(1, 40.0, 0xf);
    let mut mad = insn(0x04); // MAD H0.xyz, spec, N.L, prelit - the colour, END
    mad.dst = 0;
    mad.dst_half = true;
    mad.mask = 0b0111;
    mad.sources = [reg(1, false), Source::Input, Source::Input];
    mad.end = true;
    instructions.push(mad);

    let program = Program {
        declared: Declared::default(),
        parameter_patches: Vec::new(),
        instructions,
    };
    assert_eq!(program.specular_exponent(), Some(40.0));
    // `pow_chain`'s `DP3` is instruction index 0 - see its own doc comment.
    assert_eq!(program.specular_exponent_dp3(), Some(0));
}

/// A `pow` chain whose result nothing ever reads again is dead code, not the
/// specular term - `None`, not a guess.
#[test]
fn a_pow_chain_never_read_again_returns_none() {
    let program = Program {
        declared: Declared::default(),
        parameter_patches: Vec::new(),
        instructions: pow_chain(1, 40.0, 0xf),
    };
    assert_eq!(program.specular_exponent(), None);
}

/// **The instruction shape alone is not enough - only the chain reaching the
/// output is returned**, even when a dead `pow` chain sits later in the
/// stream and would otherwise be found first by the last-first search. This
/// is the Zone `rim^10`/`rim^5` case in miniature: the identical
/// `LG2`/`MUL`/`EX2` shape, on a register nothing downstream reads.
#[test]
fn a_dead_pow_chain_after_the_real_one_does_not_win() {
    let mut instructions = pow_chain(1, 40.0, 0xf);
    let mut mad = insn(0x04); // the real colour write, reading register 1
    mad.dst = 0;
    mad.dst_half = true;
    mad.mask = 0b0111;
    mad.sources = [reg(1, false), Source::Input, Source::Input];
    instructions.push(mad);
    // A second, unrelated chain afterward - e.g. a rim term folded only into
    // alpha, so it never touches a colour channel and never overwrites what
    // the real chain already wrote.
    instructions.extend(pow_chain(2, 10.0, 0b1000));
    let mut end = insn(0x01); // MOV H0.w, {const}, END - alpha only
    end.dst = 0;
    end.dst_half = true;
    end.mask = 0b1000;
    end.sources = [Source::Constant, Source::Input, Source::Input];
    end.constant = Some([1.0, 0.0, 0.0, 0.0]);
    end.end = true;
    instructions.push(end);

    let program = Program {
        declared: Declared::default(),
        parameter_patches: Vec::new(),
        instructions,
    };
    assert_eq!(program.specular_exponent(), Some(40.0));
}

/// **A texture-fed `pow` is not a specular term, even though it reaches the
/// output too** - `track_surface`'s own lightmap curve,
/// `pow(lightmap.rgb, k)` (`renderer.md`'s block #9), is exactly this
/// shape: the identical `LG2`/`MUL`/`EX2` instructions, fed by a `TEX`
/// result rather than a saturated dot product.
#[test]
fn a_pow_chain_fed_by_a_texture_sample_is_not_returned() {
    let mut tex = insn(0x17); // TEX H1, f[TC4] unit0 - the lightmap
    tex.dst = 1;
    tex.mask = 0xf;

    let mut lg2 = insn(0x1d);
    lg2.dst = 1;
    lg2.mask = 0xf;
    lg2.sources = [reg(1, false), Source::Input, Source::Input];

    let mut mul = insn(0x02);
    mul.dst = 1;
    mul.mask = 0xf;
    mul.sources = [reg(1, false), Source::Constant, Source::Input];
    mul.constant = Some([2.0, 0.0, 0.0, 0.0]);

    let mut ex2 = insn(0x1c);
    ex2.dst = 1;
    ex2.mask = 0xf;
    ex2.sources = [reg(1, false), Source::Input, Source::Input];

    let mut mad = insn(0x04); // + f[TC1], the colour, END
    mad.dst = 0;
    mad.dst_half = true;
    mad.mask = 0b0111;
    mad.sources = [reg(1, false), Source::Input, Source::Input];
    mad.end = true;

    let program = Program {
        declared: Declared::default(),
        parameter_patches: Vec::new(),
        instructions: vec![tex, lg2, mul, ex2, mad],
    };
    assert_eq!(
        program.specular_exponent(),
        None,
        "the LG2 reads a texture sample, not a saturated DP3 - not the N.H idiom"
    );
}

/// **`exp(N.H)` is not `pow(N.H, e)`, even fed by a saturated dot and
/// reaching the output too** - a disc-wide sweep found materials computing
/// `exp(x) = exp2(log2(e) * log2(x))` off exactly this shape, and the
/// constant's exact identity is the tell: `mesh.wgsl`'s own fog curve names
/// `log2(e)` for the identical reason.
#[test]
fn a_pow_chain_whose_exponent_is_log2_e_is_not_returned() {
    let mut instructions = pow_chain(1, std::f32::consts::LOG2_E, 0xf);
    let mut mad = insn(0x04);
    mad.dst = 0;
    mad.dst_half = true;
    mad.mask = 0b0111;
    mad.sources = [reg(1, false), Source::Input, Source::Input];
    mad.end = true;
    instructions.push(mad);

    let program = Program {
        declared: Declared::default(),
        parameter_patches: Vec::new(),
        instructions,
    };
    assert_eq!(
        program.specular_exponent(),
        None,
        "log2(e) marks exp(N.H), not a specular pow"
    );
}

/// The `log2(e)` exclusion is a value check, not a blanket rejection of
/// anything nearby - a real specular exponent that happens to be close is
/// still returned.
#[test]
fn a_pow_chain_whose_exponent_is_merely_close_to_log2_e_is_still_returned() {
    let mut instructions = pow_chain(1, 1.5, 0xf);
    let mut mad = insn(0x04);
    mad.dst = 0;
    mad.dst_half = true;
    mad.mask = 0b0111;
    mad.sources = [reg(1, false), Source::Input, Source::Input];
    mad.end = true;
    instructions.push(mad);

    let program = Program {
        declared: Declared::default(),
        parameter_patches: Vec::new(),
        instructions,
    };
    assert_eq!(program.specular_exponent(), Some(1.5));
}

/// A synthetic block carrying one parameter with `vreg = 0xffff` - patched
/// straight into the code rather than bound to a hardware register - and the
/// `fslot -> index -> offset table -> code slot` chain that names.
///
/// Verified against the real disc before this test was written:
/// `diffuse_with_specular_from_alpha.rcsmaterial`'s block at `0x7410` patches
/// `directionalLight0DirectionWorldSpace` (`0x02df31e5`) into slots `0x9` and
/// `0x10`, matching `scripts/ps3-microcode.py`'s independently-implemented
/// `fp_patch_slots` exactly - this test pins the same chain on data this
/// module owns rather than on a disc image.
fn synthetic_with_one_patch(hash: u32, patched_slot: u16) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"SHO\x08");
    b.extend_from_slice(&1u32.to_be_bytes()); // fragment
    for v in [
        2u16, 0, 1, 0, // version, 0 attributes, 1 parameter, 0 samplers
        0x18, 0x18, 0x24, 0x24, // offsets: attrs, params, samplers, program
    ] {
        b.extend_from_slice(&v.to_be_bytes());
    }
    // The one parameter record: hash, ty, count, vreg (0xffff - code-patched),
    // fslot (0x2c, the absolute offset of the index this test writes below).
    b.extend_from_slice(&hash.to_be_bytes());
    b.extend_from_slice(&0u16.to_be_bytes());
    b.extend_from_slice(&1u16.to_be_bytes());
    b.extend_from_slice(&0xffffu16.to_be_bytes());
    b.extend_from_slice(&0x2cu16.to_be_bytes());
    assert_eq!(
        b.len(),
        0x24,
        "the program sub-header starts where declared"
    );

    // Program sub-header, at +0x24 (`base`): code length, then an unrelated
    // gap this test uses to carry the patch chain's own `index` before the
    // documented `code_off` field at `+0x10`, then the entry-offset table.
    b.extend_from_slice(&0u32.to_be_bytes()); // code_len - no code, not needed
    b.resize(0x24 + 0x08, 0);
    b.extend_from_slice(&0u16.to_be_bytes()); // the index `fslot` points at
    b.resize(0x24 + 0x10, 0);
    b.extend_from_slice(&0x50u32.to_be_bytes()); // code_off, past everything
    b.extend_from_slice(&1u32.to_be_bytes()); // entries
    b.extend_from_slice(&0x20u32.to_be_bytes()); // entry_off[0], relative to base
    b.resize(0x24 + 0x20, 0);
    b.extend_from_slice(&1u16.to_be_bytes()); // this entry patches one slot
    b.extend_from_slice(&patched_slot.to_be_bytes());
    b.resize(0x24 + 0x50, 0); // code starts here, empty
    b
}

#[test]
fn a_code_patched_parameter_names_its_slot() {
    let hash = 0x02df_31e5; // directionalLight0DirectionWorldSpace
    let data = synthetic_with_one_patch(hash, 7);
    let program = Program::parse(&data, 0).expect("the block decodes");
    assert_eq!(program.patches(hash).collect::<Vec<_>>(), vec![7]);
    assert_eq!(
        program.patches(0x1234_5678).collect::<Vec<_>>(),
        Vec::<u16>::new(),
        "a hash this block never declares patches nothing"
    );
}

/// A parameter bound to a real hardware register (`vreg != 0xffff`) is not
/// code-patched at all - the chain this module walks is specifically for a
/// parameter with no register of its own.
#[test]
fn a_register_bound_parameter_is_not_in_the_patch_table() {
    let hash = 0x02df_31e5;
    let mut data = synthetic_with_one_patch(hash, 7);
    // The `vreg` field sits at the parameter record's +8, i.e. `at + 0x20`.
    data[0x20..0x22].copy_from_slice(&5u16.to_be_bytes());
    let program = Program::parse(&data, 0).expect("the block still decodes");
    assert_eq!(program.patches(hash).collect::<Vec<_>>(), Vec::<u16>::new());
}

/// The exact case `mesh::rcs::skin::roles` now checks: a `pow` chain
/// resolving to a literal `0.0`, whose constant's own code slot is the one
/// `SpecularPower` patches. [`specular_exponent_slot`] is what lets a caller
/// ask that question without re-deriving which instruction supplied the
/// literal.
///
/// [`specular_exponent_slot`]: Program::specular_exponent_slot
#[test]
fn the_zero_bucket_names_the_slot_specular_power_patches() {
    const SPECULAR_POWER: u32 = crate::rcsmaterial::SPECULAR_POWER;
    let mut instructions = pow_chain(1, 0.0, 0xf);
    // `pow_chain`'s `MUL` is instruction index 2 - see its own doc comment.
    instructions[2].const_slot = Some(9);
    let mut mad = insn(0x04);
    mad.dst = 0;
    mad.dst_half = true;
    mad.mask = 0b0111;
    mad.sources = [reg(1, false), Source::Input, Source::Input];
    mad.end = true;
    instructions.push(mad);

    let program = Program {
        declared: Declared::default(),
        parameter_patches: vec![(9, SPECULAR_POWER)],
        instructions,
    };
    assert_eq!(program.specular_exponent(), Some(0.0));
    assert_eq!(program.specular_exponent_slot(), Some(9));
    assert_eq!(program.patches(SPECULAR_POWER).collect::<Vec<_>>(), vec![9]);
}
