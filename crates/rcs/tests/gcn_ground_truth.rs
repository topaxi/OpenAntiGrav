//! `oag_rcs::gcn` against every GCN program Wipeout: Omega Collection ships: each `Shdr` tag in
//! every `.rcsmaterial` of the base and patch archives, and in both `eboot.bin`s.
//!
//! **`#[ignore]`d and never run in CI.** It needs `data/extracted/ps4/`, which this project
//! does not ship.
//!
//! ```sh
//! just test-data
//! ```
//!
//! What each archive asserts (`docs/formats/gcn.md` has the numbers and what they mean):
//!
//! - every `Shdr` tag is a program that closes (code marker on `OrbShdr`, header span, `s_endpgm`
//!   at exactly the footer's length), or is refused under a named [`gcn::Error`], counted;
//! - every opcode number the Sea Islands tables do not list is counted by format and number,
//!   so a new one fails here rather than passing as a nameless instruction;
//! - reflections and vertex-input tables that do not close are counted by error;
//! - the whole survey is a ratchet on the exact numbers.
//!
//! Plus the oracles: lines the docs printed by hand off three programs before this decoder
//! existed (`omega-status.md`, `lightmap-prelit.md`), which it must reproduce.
//!
//! One archive per test, so the sweep parallelises across tests.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_rcs::gcn::{self, Detail, Encoding, Operand, Program, Stage};

fn path(rel: &str) -> Option<PathBuf> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/ps4")
        .join(rel);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Survey {
    files: usize,
    tags: usize,
    pixel: usize,
    vertex: usize,
    other_stage: usize,
    instructions: usize,
    /// Programs with a PS4 Pro word (VOP3P, SDWA or DPP).
    pro_programs: usize,
    /// Files whose stem does not end `_<digit>`, and programs in them with a PS4 Pro word or
    /// an unlisted opcode: the plain-named files are the base console's compile.
    plain_files: usize,
    plain_pro_or_unknown: usize,
    /// `Shdr` tags refused, by error kind.
    refused: BTreeMap<String, usize>,
    /// Instructions whose opcode number no table lists, by format and number.
    unknown: BTreeMap<String, usize>,
    /// Reflections and vertex-input tables that do not close, by error kind.
    reflection: BTreeMap<String, usize>,
    vertex_inputs: BTreeMap<String, usize>,
    /// Vertex programs fetching a four-component `tangent`, by the first instruction that
    /// reads its `w` and whether that instruction applies `2w - 1` (a multiply-add with the
    /// inline 2.0 and -1.0): HD's two compiles split on exactly this.
    tangent_w: BTreeMap<String, usize>,
}

/// An error's kind: its message with the numbers taken out.
fn kind(e: &gcn::Error) -> String {
    let text = e.to_string();
    text.split(' ')
        .map(|w| {
            if w.starts_with("0x") || w.bytes().all(|b| b.is_ascii_digit()) {
                "N"
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a material's stem ends `_<digit>` (`DC_HologramWithStatic_1`).
fn is_variant(path: &str) -> bool {
    let stem = path.rsplit('/').next().unwrap_or(path);
    let stem = stem.split('.').next().unwrap_or(stem).as_bytes();
    stem.len() > 2 && stem[stem.len() - 2] == b'_' && stem[stem.len() - 1].is_ascii_digit()
}

impl Survey {
    fn add(&mut self, file: &[u8], plain: bool) {
        self.plain_files += usize::from(plain);
        for (_, parsed) in gcn::scan(file) {
            self.tags += 1;
            match parsed {
                Ok(p) => {
                    let marked = self.program(&p);
                    self.plain_pro_or_unknown += usize::from(plain && marked);
                }
                Err(e) => *self.refused.entry(kind(&e)).or_default() += 1,
            }
        }
    }

    /// Counts one program; whether it has a PS4 Pro word or an unlisted opcode.
    fn program(&mut self, p: &Program<'_>) -> bool {
        match p.stage {
            Stage::Pixel => self.pixel += 1,
            Stage::Vertex => self.vertex += 1,
            Stage::Other(_) => self.other_stage += 1,
        }
        self.instructions += p.instructions.len();
        // `Program::signed_textures` places every `2x - 1` sample's T# through the header's
        // usage records; one it cannot place is a record kind this reading does not know.
        if p.stage == Stage::Pixel {
            assert_eq!(
                p.signed_textures().unresolved,
                0,
                "a pixel program at {:#x} signs a texture no usage record places",
                p.at
            );
        }
        let mut pro = false;
        for i in &p.instructions {
            pro |= i.encoding == Encoding::Vop3p
                || matches!(i.detail, Detail::Sdwa { .. } | Detail::Dpp { .. });
            if i.name.is_none() {
                *self
                    .unknown
                    .entry(format!("{} {}", i.encoding.name(), i.op))
                    .or_default() += 1;
            }
        }
        self.pro_programs += usize::from(pro);
        let unknown = p.instructions.iter().any(|i| i.name.is_none());
        if let Err(e) = &p.reflection {
            *self.reflection.entry(kind(e)).or_default() += 1;
        }
        if let Err(e) = &p.vertex_inputs {
            *self.vertex_inputs.entry(kind(e)).or_default() += 1;
        }
        if let Some(reader) = tangent_w_reader(p) {
            *self.tangent_w.entry(reader).or_default() += 1;
        }
        pro || unknown
    }
}

/// The first instruction reading a fetched tangent's `w`, named with whether it is `2w - 1`.
fn tangent_w_reader(p: &Program<'_>) -> Option<String> {
    let (input, _) = p.input_named("tangent")?;
    if input.size < 4 {
        return Some("w not fetched".to_string());
    }
    let w = input.vgpr + 3;
    let reads = |op: &Operand| matches!(*op, Operand::Vgpr { first, count } if (first..first + count).contains(&w));
    let first = p
        .instructions
        .iter()
        .find(|i| i.src.iter().flatten().any(reads))?;
    let two = first
        .src
        .iter()
        .flatten()
        .any(|s| *s == Operand::Float(2.0));
    let minus_one = first
        .src
        .iter()
        .flatten()
        .any(|s| *s == Operand::Float(-1.0));
    let law = if two && minus_one { "2w-1" } else { "raw" };
    Some(format!("{} {law}", first.name.unwrap_or("?")))
}

fn sweep(rel: &str) -> Option<Survey> {
    let path = path(rel)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let materials: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmaterial"))
        .cloned()
        .collect();
    let mut survey = Survey::default();
    for m in materials {
        let bytes = archive.read_path(&m).expect("every entry reads");
        survey.files += 1;
        survey.add(&bytes, !is_variant(&m));
    }
    Some(survey)
}

fn holds(rel: &str, expected: &str) {
    let Some(s) = sweep(rel) else { return };
    assert!(
        s.refused.is_empty(),
        "{rel}: refused programs {:?}",
        s.refused
    );
    assert_eq!(
        s.plain_pro_or_unknown, 0,
        "{rel}: a plain-named material has a PS4 Pro word or an unlisted opcode"
    );
    assert!(
        !s.tangent_w.keys().any(|k| k.ends_with("2w-1")),
        "{rel}: a vertex program applies 2w - 1 to the tangent's w: {:?}",
        s.tangent_w
    );
    assert_eq!(format!("{s:?}"), expected, "{rel}");
}

macro_rules! archive {
    ($test:ident, $rel:literal, $expected:literal) => {
        #[test]
        #[ignore = "needs the Omega extraction"]
        fn $test() {
            holds($rel, $expected);
        }
    };
}

archive!(
    base_data00,
    "omega-eu/uroot/data00.psarc",
    "Survey { files: 2993, tags: 138837, pixel: 31332, vertex: 107505, other_stage: 0, instructions: 16862339, pro_programs: 83658, plain_files: 991, plain_pro_or_unknown: 0, refused: {}, unknown: {\"VOP1 84\": 6162, \"VOP1 85\": 3712, \"VOP1 86\": 158803, \"VOP1 87\": 4424, \"VOP1 88\": 68246, \"VOP1 91\": 3120, \"VOP2 50\": 13722, \"VOP2 51\": 6983, \"VOP2 53\": 546590, \"VOP2 55\": 32, \"VOP2 57\": 157748, \"VOP2 58\": 191566, \"VOPC 201\": 2888, \"VOPC 203\": 550, \"VOPC 204\": 80, \"VOPC 206\": 64}, reflection: {}, vertex_inputs: {}, tangent_w: {\"v_mad_mix_f32 raw\": 7282, \"v_mad_mixhi_f16 raw\": 16, \"v_mad_mixlo_f16 raw\": 24, \"v_mul_f32 raw\": 10879, \"w not fetched\": 1695} }"
);
archive!(
    base_data01,
    "omega-eu/uroot/data01.psarc",
    "Survey { files: 1601, tags: 152141, pixel: 30731, vertex: 121410, other_stage: 0, instructions: 19406731, pro_programs: 94424, plain_files: 530, plain_pro_or_unknown: 0, refused: {}, unknown: {\"VOP1 84\": 4486, \"VOP1 86\": 177967, \"VOP1 87\": 3834, \"VOP1 88\": 77275, \"VOP1 91\": 2036, \"VOP2 50\": 12398, \"VOP2 51\": 2883, \"VOP2 53\": 590849, \"VOP2 56\": 8, \"VOP2 57\": 174496, \"VOP2 58\": 212835, \"VOPC 201\": 2064, \"VOPC 203\": 1374}, reflection: {}, vertex_inputs: {}, tangent_w: {\"v_mad_mix_f32 raw\": 13392, \"v_mad_mixhi_f16 raw\": 10, \"v_mad_mixlo_f16 raw\": 30, \"v_mul_f32 raw\": 16760, \"w not fetched\": 630} }"
);
archive!(
    base_data02,
    "omega-eu/uroot/data02.psarc",
    "Survey { files: 2549, tags: 240668, pixel: 49385, vertex: 191283, other_stage: 0, instructions: 31015298, pro_programs: 149718, plain_files: 849, plain_pro_or_unknown: 0, refused: {}, unknown: {\"VOP1 84\": 8957, \"VOP1 85\": 10296, \"VOP1 86\": 283613, \"VOP1 87\": 6989, \"VOP1 88\": 122732, \"VOP1 91\": 3604, \"VOP2 50\": 22898, \"VOP2 51\": 5913, \"VOP2 53\": 939264, \"VOP2 55\": 10, \"VOP2 57\": 270277, \"VOP2 58\": 328951, \"VOPC 201\": 5388, \"VOPC 203\": 2426}, reflection: {}, vertex_inputs: {}, tangent_w: {\"v_mad_mix_f32 raw\": 20238, \"v_mul_f32 raw\": 25242, \"w not fetched\": 990} }"
);
archive!(
    base_data03,
    "omega-eu/uroot/data03.psarc",
    "Survey { files: 0, tags: 0, pixel: 0, vertex: 0, other_stage: 0, instructions: 0, pro_programs: 0, plain_files: 0, plain_pro_or_unknown: 0, refused: {}, unknown: {}, reflection: {}, vertex_inputs: {}, tangent_w: {} }"
);
archive!(
    base_data04,
    "omega-eu/uroot/data04.psarc",
    "Survey { files: 768, tags: 66112, pixel: 11314, vertex: 54798, other_stage: 0, instructions: 8168620, pro_programs: 40486, plain_files: 256, plain_pro_or_unknown: 0, refused: {}, unknown: {\"VOP1 84\": 1398, \"VOP1 85\": 2738, \"VOP1 86\": 80438, \"VOP1 87\": 1364, \"VOP1 88\": 34510, \"VOP1 91\": 226, \"VOP2 50\": 5572, \"VOP2 51\": 1706, \"VOP2 53\": 275328, \"VOP2 57\": 78840, \"VOP2 58\": 99072, \"VOPC 201\": 80, \"VOPC 203\": 392, \"VOPC 204\": 80}, reflection: {}, vertex_inputs: {}, tangent_w: {\"v_mad_mix_f32 raw\": 3220, \"v_mul_f32 raw\": 4025} }"
);
archive!(
    patch_data05,
    "omega-eu-patch/uroot/data05.psarc",
    "Survey { files: 7912, tags: 597755, pixel: 122752, vertex: 475003, other_stage: 0, instructions: 76459359, pro_programs: 368274, plain_files: 2627, plain_pro_or_unknown: 0, refused: {}, unknown: {\"VOP1 84\": 21003, \"VOP1 85\": 16746, \"VOP1 86\": 775470, \"VOP1 87\": 251696, \"VOP1 88\": 537848, \"VOP1 91\": 7434, \"VOP2 50\": 50542, \"VOP2 51\": 33801, \"VOP2 53\": 2657704, \"VOP2 55\": 644, \"VOP2 57\": 747456, \"VOP2 58\": 824492, \"VOPC 201\": 10420, \"VOPC 203\": 4742, \"VOPC 204\": 160, \"VOPC 206\": 64}, reflection: {}, vertex_inputs: {}, tangent_w: {\"v_mad_mix_f32 raw\": 44132, \"v_mad_mixhi_f16 raw\": 22, \"v_mad_mixlo_f16 raw\": 58, \"v_mul_f32 raw\": 56906, \"w not fetched\": 3315} }"
);
archive!(
    patch_data07,
    "omega-eu-patch/uroot/data07.psarc",
    "Survey { files: 0, tags: 0, pixel: 0, vertex: 0, other_stage: 0, instructions: 0, pro_programs: 0, plain_files: 0, plain_pro_or_unknown: 0, refused: {}, unknown: {}, reflection: {}, vertex_inputs: {}, tangent_w: {} }"
);
archive!(
    patch_data08,
    "omega-eu-patch/uroot/data08.psarc",
    "Survey { files: 7874, tags: 589851, pixel: 121403, vertex: 468448, other_stage: 0, instructions: 75911790, pro_programs: 363258, plain_files: 2615, plain_pro_or_unknown: 0, refused: {}, unknown: {\"VOP1 84\": 20943, \"VOP1 85\": 6450, \"VOP1 86\": 765587, \"VOP1 87\": 251533, \"VOP1 88\": 533624, \"VOP1 91\": 7452, \"VOP2 50\": 42696, \"VOP2 51\": 33830, \"VOP2 53\": 2636839, \"VOP2 55\": 674, \"VOP2 56\": 4, \"VOP2 57\": 689156, \"VOP2 58\": 737620, \"VOPC 201\": 10344, \"VOPC 203\": 4742, \"VOPC 204\": 160, \"VOPC 206\": 64}, reflection: {}, vertex_inputs: {}, tangent_w: {\"v_mad_mix_f32 raw\": 44092, \"v_mad_mixhi_f16 raw\": 40, \"v_mad_mixlo_f16 raw\": 40, \"v_mul_f32 raw\": 57006, \"w not fetched\": 3315} }"
);
archive!(
    patch_data09,
    "omega-eu-patch/uroot/data09.psarc",
    "Survey { files: 0, tags: 0, pixel: 0, vertex: 0, other_stage: 0, instructions: 0, pro_programs: 0, plain_files: 0, plain_pro_or_unknown: 0, refused: {}, unknown: {}, reflection: {}, vertex_inputs: {}, tangent_w: {} }"
);

fn eboot(rel: &str, expected: &str) {
    let Some(path) = path(rel) else { return };
    let bytes = std::fs::read(path).expect("reads");
    let mut s = Survey {
        files: 1,
        ..Survey::default()
    };
    s.add(&bytes, false);
    assert_eq!(format!("{s:?}"), expected, "{rel}");
    tone_curve_lines(&bytes);
}

/// `tonemap.md`'s curve resolve, held in both eboots (base and the 1.07 patch decode it
/// identically): `x = max(min(E * c, t1), t0)`, then `a + x (b + x (c + d x))`, per channel.
fn tone_curve_lines(eboot: &[u8]) {
    let resolve = gcn::scan(eboot)
        .into_iter()
        .filter_map(|(_, p)| p.ok())
        .find(|p| {
            p.reflection
                .as_ref()
                .is_ok_and(|r| r.constants.iter().any(|c| c.name == "m_LAvgt0t1Exposure"))
        })
        .expect("the curve resolve ships");
    assert_eq!(resolve.instructions.len(), 105);
    contains_in_order(
        &resolve.to_string(),
        &[
            "s_buffer_load_dwordx8 s[8:15], s[8:11], 0x0",
            "v_mul_f32 v0, s15, v0",
            "v_min_f32 v0, s14, v0",
            "v_max_f32 v0, s13, v0",
            "v_mad_f32 v6, v0, s11, v4",
            "v_mad_f32 v6, v6, v0, s9",
            "v_mad_f32 v0, v0, v6, s8",
        ],
    );
}

#[test]
#[ignore = "needs the Omega extraction"]
fn base_eboot() {
    eboot(
        "omega-eu/uroot/eboot.bin",
        "Survey { files: 1, tags: 203, pixel: 134, vertex: 57, other_stage: 12, instructions: 9911, pro_programs: 3, plain_files: 0, plain_pro_or_unknown: 0, refused: {}, unknown: {}, reflection: {\"reflection: an unread header word is not zero\": 8, \"the reflection of a stage-4 program is unread\": 12}, vertex_inputs: {\"N runs in the vertex header match the input slots\": 4}, tangent_w: {} }",
    );
}

#[test]
#[ignore = "needs the Omega extraction"]
fn patch_eboot() {
    eboot(
        "omega-eu-patch/uroot/eboot.bin",
        "Survey { files: 1, tags: 213, pixel: 140, vertex: 61, other_stage: 12, instructions: 10462, pro_programs: 3, plain_files: 0, plain_pro_or_unknown: 0, refused: {}, unknown: {}, reflection: {\"reflection: an unread header word is not zero\": 9, \"the reflection of a stage-4 program is unread\": 12}, vertex_inputs: {\"N runs in the vertex header match the input slots\": 4}, tangent_w: {} }",
    );
}

/// One program's text, found by its `Shdr` tag's or its code's byte offset (the docs quote
/// either).
fn program_text(rel: &str, entry: &str, offset: usize) -> Option<String> {
    let path = path(rel)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let bytes = archive.read_path(entry).expect("the entry reads");
    let program = gcn::scan(&bytes)
        .into_iter()
        .filter_map(|(_, p)| p.ok())
        .find(|p| p.at == offset || p.code_at == offset)
        .unwrap_or_else(|| panic!("no program at {offset:#x} in {entry}"));
    Some(program.to_string())
}

fn contains_in_order(text: &str, lines: &[&str]) {
    let mut rest = text;
    for line in lines {
        let at = rest
            .find(line)
            .unwrap_or_else(|| panic!("`{line}` not found in order in:\n{text}"));
        rest = &rest[at + line.len()..];
    }
}

/// `omega-status.md`'s FranelBlend pixel shader, read with a scratch decoder in 2026-09: the
/// lines carrying `(1 - N.V)^ReflectivityPower`.
#[test]
#[ignore = "needs the Omega extraction"]
fn the_franel_blend_lines_omega_status_printed() {
    let Some(text) = program_text(
        "omega-eu/uroot/data00.psarc",
        "Data/environments/15_anulpha_pass/fe/Materials/FrontEndConstantFranelBlend.rcsmaterial",
        0x323c,
    ) else {
        return;
    };
    contains_in_order(
        &text,
        &[
            "v_mad_f32 v7, -v6, v7, 1.0 clamp",
            "s_buffer_load_dword s22, s[0:3], 0x6    ; ReflectivityPower[0]",
            "v_log_f32 v7, |v7|",
            "v_mul_f32 v7, s22, v7",
            "v_exp_f32 v7, v7",
            "v_mov_b32 v12, s20",
        ],
    );
}

/// `lightmap-prelit.md`'s Ubermall pixel shader at `0xad04`: the nova prelit curve.
#[test]
#[ignore = "needs the Omega extraction"]
fn the_ubermall_prelit_lines_lightmap_prelit_printed() {
    let Some(text) = program_text(
        "omega-eu/uroot/data02.psarc",
        "Data/environments/05_ubermall/Materials/Diffuse_Normal_Specular.rcsmaterial",
        0xad04,
    ) else {
        return;
    };
    contains_in_order(
        &text,
        &[
            "s_buffer_load_dwordx16 s[16:31], s[36:39], 0x0",
            "v_log_f32 v9, |v9|",
            "v_mul_f32 v9, s18, v9",
            "v_exp_f32 v19, v9",
            "v_mad_f32 v10, v19, s16, v20",
        ],
    );
}
