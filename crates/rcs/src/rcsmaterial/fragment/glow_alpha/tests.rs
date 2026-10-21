use super::*;

fn insn(opcode: u8) -> Instruction {
    Instruction {
        opcode,
        dst: 0,
        dst_half: true,
        mask: 0b1000,
        saturate: false,
        input: 0,
        unit: 0,
        sources: [Source::Input; 3],
        swizzles: [[0, 1, 2, 3]; 3],
        constant: None,
        const_slot: None,
        negate: [false; 3],
        cond: 0x727,
        scale: 0,
        end: false,
    }
}

fn program(instructions: Vec<Instruction>) -> Program {
    let mut instructions = instructions;
    if let Some(last) = instructions.last_mut() {
        last.end = true;
    }
    Program {
        instructions,
        ..Program::default()
    }
}

fn none(_: u32) -> Option<[f32; 4]> {
    None
}

/// `MOV H0.w, {1}`: the neon sign's program.
#[test]
fn a_constant_written_to_w_is_a_constant() {
    let mut mov = insn(0x01);
    mov.sources[0] = Source::Constant;
    mov.constant = Some([1.0, 0.0, 0.0, 0.0]);
    mov.swizzles[0] = [0; 4];
    assert_eq!(program(vec![mov]).output_alpha(none), Lane::Constant(1.0));
}

/// A lane never written is `Unwritten`, which is not the same claim as zero.
#[test]
fn a_program_that_never_writes_w_leaves_it_unwritten() {
    let mut mov = insn(0x01);
    mov.mask = 0b0111;
    assert_eq!(program(vec![mov]).output_alpha(none), Lane::Unwritten);
}

/// `TEX H1, f[TC0] unit1; MOV H0.w, H1.wwww`: the emissive pads and lights.
#[test]
fn a_texture_channel_copied_to_w_names_its_unit_and_channel() {
    let mut tex = insn(0x17);
    tex.dst = 1;
    tex.mask = 0b1111;
    tex.unit = 1;
    let mut mov = insn(0x01);
    mov.sources[0] = Source::Register {
        index: 1,
        half: true,
    };
    mov.swizzles[0] = [3; 4];
    assert_eq!(
        program(vec![tex, mov]).output_alpha(none),
        Lane::Texel {
            unit: 1,
            channel: 3,
            scale: 1.0,
            bias: 0.0
        }
    );
}

/// `MUL H0.w, H1.w, K` scales the texel, and a patched constant takes the
/// material's value rather than the literal the file carries.
#[test]
fn a_patched_constant_takes_the_materials_value() {
    let mut tex = insn(0x17);
    tex.dst = 1;
    tex.mask = 0b1111;
    let mut mul = insn(0x02);
    mul.sources = [
        Source::Register {
            index: 1,
            half: true,
        },
        Source::Constant,
        Source::Input,
    ];
    mul.swizzles[0] = [3; 4];
    mul.swizzles[1] = [0; 4];
    mul.constant = Some([9.0, 0.0, 0.0, 0.0]);
    mul.const_slot = Some(5);
    let mut p = program(vec![tex, mul]);
    p.parameter_patches = vec![(5, 0xdead_beef)];
    let lane = p.output_alpha(|hash| (hash == 0xdead_beef).then_some([0.25, 0.0, 0.0, 0.0]));
    assert_eq!(
        lane,
        Lane::Texel {
            unit: 0,
            channel: 3,
            scale: 0.25,
            bias: 0.0
        },
        "the parameter's 0.25, not the literal 9"
    );
}

/// The output scale applies to what an instruction writes: `/2` halves it.
#[test]
fn the_output_scale_applies() {
    let mut mov = insn(0x01);
    mov.sources[0] = Source::Constant;
    mov.constant = Some([1.0, 0.0, 0.0, 0.0]);
    mov.swizzles[0] = [0; 4];
    mov.scale = 5;
    assert_eq!(program(vec![mov]).output_alpha(none), Lane::Constant(0.5));
}

/// A product of two samples, an interpolator or a dot product is not followed.
#[test]
fn an_expression_this_reading_does_not_follow_is_other() {
    let mov = insn(0x01); // reads an interpolator
    assert_eq!(program(vec![mov]).output_alpha(none), Lane::Other);
}
