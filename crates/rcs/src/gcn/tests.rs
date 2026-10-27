//! Words assembled by hand from the manuals' field tables (no disc bytes), decoded back.
//! The disc-backed oracles, lines the docs printed off real programs, are in
//! `tests/gcn_ground_truth.rs`.

use super::isa::{DecodeError, Encoding, Operand, decode, decode_all};

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

fn text(words: &[u32]) -> String {
    decode(&bytes(words), 0).expect("decodes").to_string()
}

/// VOP3a: `OP` [25:17], `VDST` [7:0], `ABS` [10:8], `CLAMP` 11; `SRC0..2` 9 bits each from
/// bit 32, `OMOD` [60:59], `NEG` [63:61] (section 13.3).
fn vop3(op: u32, vdst: u32, src: [u32; 3], abs: u32, neg: u32, clamp: bool) -> [u32; 2] {
    [
        0b11_0100 << 26 | op << 17 | u32::from(clamp) << 11 | abs << 8 | vdst,
        neg << 29 | src[2] << 18 | src[1] << 9 | src[0],
    ]
}

const V: u32 = 256;

#[test]
fn the_code_marker_is_a_scalar_move_with_a_literal() {
    // SOP1: [31:23] = 0x17d, SDST [22:16] = 107 (vcc_hi), OP [15:8] = 3, SSRC0 = 255.
    let w = 0x17d << 23 | 107 << 16 | 3 << 8 | 255;
    assert_eq!(w, super::CODE_MARKER);
    assert_eq!(text(&[w, 0x38]), "s_mov_b32 vcc_hi, 0x38");
}

#[test]
fn prefixes_pick_the_longest_matching_format() {
    assert_eq!(Encoding::of(0xbeeb_03ff), Some(Encoding::Sop1));
    assert_eq!(Encoding::of(0xbf81_0000), Some(Encoding::Sopp));
    assert_eq!(Encoding::of(0xbf00_0000), Some(Encoding::Sopc));
    assert_eq!(Encoding::of(0xb000_0000), Some(Encoding::Sopk));
    assert_eq!(Encoding::of(0x8000_0000), Some(Encoding::Sop2));
    assert_eq!(Encoding::of(0x7e02_0216), Some(Encoding::Vop1));
    assert_eq!(Encoding::of(0x7c00_0000), Some(Encoding::Vopc));
    assert_eq!(Encoding::of(0x3e02_0c12), Some(Encoding::Vop2));
    assert_eq!(Encoding::of(0xc304_0500), Some(Encoding::Smrd));
    assert_eq!(Encoding::of(0xd282_0005), Some(Encoding::Vop3));
    assert_eq!(Encoding::of(0xc824_0400), Some(Encoding::Vintrp));
    assert_eq!(Encoding::of(0xf080_0000), Some(Encoding::Mimg));
    assert_eq!(Encoding::of(0xf800_0000), Some(Encoding::Exp));
    assert_eq!(Encoding::of(0xcc22_4000), Some(Encoding::Vop3p));
    assert_eq!(Encoding::of(0xfc00_0000), None);
}

#[test]
fn a_vop3_carries_its_negate_absolute_and_clamp() {
    // V_MAD_F32 is VOP3 opcode 321; 242 is the inline 1.0.
    let w = vop3(321, 7, [V + 6, V + 7, 242], 0, 0b001, true);
    assert_eq!(text(&w), "v_mad_f32 v7, -v6, v7, 1.0 clamp");
    // V_LOG_F32 is VOP1 39, so VOP3 384 + 39.
    let w = vop3(384 + 39, 7, [V + 7, 0, 0], 0b001, 0, false);
    assert_eq!(text(&w), "v_log_f32 v7, |v7|");
}

#[test]
fn a_vop3_refuses_a_literal() {
    let w = vop3(321, 0, [255, V, V], 0, 0, false);
    assert_eq!(
        decode(&bytes(&w), 0),
        Err(DecodeError::LiteralInVop3 { at: 0 })
    );
}

#[test]
fn a_scalar_buffer_load_counts_its_offset_in_dwords() {
    // SMRD: [31:27] = 11000, OP [26:22] (table 7.1), SDST [21:15], SBASE [14:9] in pairs,
    // IMM 8, OFFSET [7:0].
    let w = 0b11000 << 27 | 8 << 22 | 22 << 15 | 1 << 8 | 6;
    assert_eq!(text(&[w]), "s_buffer_load_dword s22, s[0:3], 0x6");
    let w = 0b11000 << 27 | 12 << 22 | 8 << 15 | 2 << 9 | 1 << 8;
    assert_eq!(text(&[w]), "s_buffer_load_dwordx16 s[8:23], s[4:7], 0x0");
}

#[test]
fn a_vop2_literal_is_the_next_dword() {
    // VOP2: OP [30:25] = 8 (V_MUL_F32), VDST [24:17], VSRC1 [16:9], SRC0 [8:0] = 255.
    let w = 8 << 25 | 4 << 17 | 4 << 9 | 255;
    let i = decode(&bytes(&[w, 0x3fb8_aa3b]), 0).expect("decodes");
    assert_eq!(i.size, 8);
    assert_eq!(i.to_string(), "v_mul_f32 v4, 0x3fb8aa3b, v4");
    assert_eq!(
        i.src[0].and_then(Operand::as_f32),
        Some(f32::from_bits(0x3fb8_aa3b))
    );
}

#[test]
fn madmk_and_madak_always_carry_their_constant() {
    // V_MADMK_F32 (32): D = S0 * K + S1; V_MADAK_F32 (33): D = S0 * S1 + K.
    let mk = 32 << 25 | 1 << 17 | 3 << 9 | (V + 2);
    assert_eq!(
        text(&[mk, 0x3e80_0000]),
        "v_madmk_f32 v1, v2, 0x3e800000, v3"
    );
    let ak = 33 << 25 | 1 << 17 | 3 << 9 | (V + 2);
    assert_eq!(
        text(&[ak, 0x3e80_0000]),
        "v_madak_f32 v1, v2, v3, 0x3e800000"
    );
}

#[test]
fn an_interpolation_names_its_attribute_and_channel() {
    // VINTRP: [31:26] = 110010, VDST [25:18], OP [17:16], ATTR [15:10], ATTRCHAN [9:8], VSRC.
    let w = 0b11_0010 << 26 | 9 << 18 | 1 << 16 | 2 << 10 | 3 << 8 | 1;
    assert_eq!(text(&[w]), "v_interp_p2_f32 v9, v1, attr2.w");
}

#[test]
fn an_image_sample_reads_dmask_registers() {
    // MIMG: OP [24:18] = 32 (IMAGE_SAMPLE), DMASK [11:8]; VADDR, VDATA, SRSRC x4, SSAMP x4.
    let w0 = 0b11_1100 << 26 | 32 << 18 | 0xf << 8;
    let w1 = 8 << 21 | 6 << 16 | 9 << 8 | 5;
    assert_eq!(
        text(&[w0, w1]),
        "image_sample v[9:12], v5, s[24:31], s[32:35] dmask:0xf"
    );
}

#[test]
fn an_export_prints_its_target_and_flags() {
    // EXP: EN [3:0], TGT [9:4], COMPR 10, DONE 11, VM 12; VSRC0..3 a byte each.
    let w0 = 0b11_1110 << 26 | 1 << 11 | 12 << 4 | 0xf;
    let w1 = 13 << 24 | 5 << 16 | 11 << 8 | 6;
    assert_eq!(text(&[w0, w1]), "exp pos0 v6, v11, v5, v13 done");
    let w0 = 0b11_1110 << 26 | 1 << 10 | 0b0011;
    assert_eq!(text(&[w0, 1 << 8 | 2]), "exp mrt0 v2, v1, off, off compr");
}

#[test]
fn a_branch_prints_its_target() {
    // S_CBRANCH_EXECZ (SOPP 8): PC + 4 + SIMM16 * 4.
    let w = 0x17f << 23 | 8 << 16 | 6;
    assert_eq!(text(&[w]), "s_cbranch_execz 0x1c");
    assert_eq!(text(&[0x17f << 23 | 1 << 16]), "s_endpgm");
}

#[test]
fn an_unlisted_opcode_keeps_the_stream_in_step() {
    // SOP2 13 is reserved in the manual; it still decodes as one word.
    let code = bytes(&[0b10 << 30 | 13 << 23, 0x17f << 23 | 1 << 16]);
    let all = decode_all(&code).expect("in step");
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].name, None);
    assert!(all[1].is_end());
}

#[test]
fn an_unknown_prefix_stops_the_program() {
    assert_eq!(
        decode(&bytes(&[0xfc00_0000]), 0),
        Err(DecodeError::UnknownEncoding {
            at: 0,
            word: 0xfc00_0000
        })
    );
}

#[test]
fn packed_math_and_sdwa_take_a_second_dword() {
    // VOP3P (Vega 13.3.6) with the PS4 Pro's 0xCC prefix: OP [22:16] = 34, V_MAD_MIXHI_F16.
    let w = [
        0xcc << 24 | 34 << 16 | 1 << 14,
        244 << 18 | (V + 1) << 9 | V,
    ];
    assert!(text(&w).starts_with("v_mad_mixhi_f16 v0, v0, v1, 2.0"));
    // VOP1 V_MOV_B32 with SRC0 = 249: SDWA, the real source in the second dword.
    let i = decode(&bytes(&[0x7e00_02f9, 6 << 16 | 2 << 11 | 4 << 8 | 3]), 0).expect("decodes");
    assert_eq!(i.size, 8);
    assert_eq!(
        i.to_string(),
        "v_mov_b32_sdwa v0, v3 dst_sel:WORD_0 dst_unused:2 src0_sel:DWORD src1_sel:BYTE_0"
    );
}
