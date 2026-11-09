//! The stand-in codepoints in `oag_pulse::prompts`, `oag_pure::prompts`,
//! `oag_hd::prompts` and `oag_omega::prompts` are the glyphs they claim to be.
//!
//! Each table row says "this codepoint draws this control". That was read off
//! the rendered glyph cells, and
//! this pins it: the cell of every stand-in in the disc's own face, hashed, so
//! a table that swaps two rows - or a face whose `ε` stops being a cross -
//! fails here rather than showing a player the wrong button. The hash is of
//! alpha bytes only, and no disc content is stored.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-ui --run-ignored all \
//!     -E 'test(/^prompt_glyphs_ground_truth::/)'
//! ```

use oag_title::prompts::{Prompt, Prompts};

/// The cells whose pictures were read: `Pulse_20.fnt`, the PS2's `Pulse_14.fnt` and
/// `Pulse_20.fnt`, `PS_BUTTONS.fnt` (HD and Omega) and Pure's `FX300ANG.fnt`,
/// `ε` the circled cross, `γ` circle, `δ` square, `β` triangle, `Λ`/`Ν` the
/// L and R boxes. (face, codepoint, FNV-1a of the cell's alpha.)
const READ: &[(&str, char, u64)] = &[
    ("Pulse_20", 'ε', 0x4e78_a1ee_83a0_e278),
    ("Pulse_20", 'γ', 0x1eec_4c80_5e08_3ca4),
    ("Pulse_20", 'δ', 0x0dcf_7f7c_985d_6f08),
    ("Pulse_20", 'β', 0x0a18_686c_832d_b108),
    ("Pulse_20", 'Λ', 0xdd58_0cf5_2689_c56e),
    ("Pulse_20", 'Ν', 0x8437_305b_ac1b_0612),
    ("ps2 Pulse_14", 'ε', 0xc6e0_902d_7627_c9f5),
    ("ps2 Pulse_14", 'γ', 0xf666_3d34_8536_afd1),
    ("ps2 Pulse_14", 'δ', 0x3eae_21e3_2233_6a2d),
    ("ps2 Pulse_14", 'β', 0x4426_417a_10f9_bb1e),
    ("ps2 Pulse_14", 'Λ', 0x167a_adf5_fa2e_3030),
    ("ps2 Pulse_14", 'Ν', 0x294b_baf1_6cfb_a392),
    ("ps2 Pulse_20", 'ε', 0x7984_0edc_f2e9_e33d),
    ("ps2 Pulse_20", 'γ', 0x533b_9c3a_d161_c097),
    ("ps2 Pulse_20", 'δ', 0x7ed6_dbdc_3535_62f1),
    ("ps2 Pulse_20", 'β', 0xab76_8d4f_d7c7_c398),
    ("ps2 Pulse_20", 'Λ', 0x6020_f9e5_0f07_3b21),
    ("ps2 Pulse_20", 'Ν', 0x6af5_959e_76ac_de2d),
    ("PS_BUTTONS", 'ε', 0x8667_ec82_e0ec_2715),
    ("PS_BUTTONS", 'γ', 0x2dac_ff1c_2c39_a4c9),
    ("PS_BUTTONS", 'δ', 0x3442_bf52_a954_b7f0),
    ("PS_BUTTONS", 'β', 0x2a21_6775_2fd4_fd3e),
    ("PS_BUTTONS", 'Λ', 0xcb98_e146_a436_42af),
    ("PS_BUTTONS", 'Ν', 0xc6b4_6428_acb5_f1e7),
    ("pure FX300ANG", 'ε', 0x8913_5a96_48e1_ff94),
    ("pure FX300ANG", 'γ', 0x4c29_5fa2_59c9_4161),
    ("pure FX300ANG", 'δ', 0xd48c_2a2c_3597_14ea),
    ("pure FX300ANG", 'β', 0xc3b7_fb23_0dac_0df2),
    ("pure FX300ANG", 'Λ', 0xa8bb_9a28_78bb_fac8),
    ("pure FX300ANG", 'Ν', 0xe507_8f31_5a51_bc99),
    ("omega PS_BUTTONS", 'ε', 0xe8c8_99be_ef4a_50f6),
    ("omega PS_BUTTONS", 'γ', 0xaec2_5828_1904_e572),
    ("omega PS_BUTTONS", 'δ', 0x72fe_59ee_68f6_1caa),
    ("omega PS_BUTTONS", 'β', 0x4a23_9d56_9d2c_066d),
    ("omega PS_BUTTONS", 'Λ', 0x526e_51f5_6fbd_9faa),
    ("omega PS_BUTTONS", 'Ν', 0x454d_922f_2333_3f8a),
];

/// FNV-1a over the alpha of `ch`'s cell, row-major.
fn digest(font: &oag_texture::fnt::Font, ch: char) -> Option<(u64, u8, u8)> {
    let glyph = font
        .glyphs
        .iter()
        .find(|glyph| u32::from(glyph.codepoint) == ch as u32)?;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for y in 0..usize::from(glyph.height) {
        for x in 0..usize::from(glyph.width) {
            let alpha = font.alpha_at(usize::from(glyph.u0) + x, usize::from(glyph.v0) + y);
            hash = (hash ^ u64::from(alpha)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    Some((hash, glyph.width, glyph.height))
}

fn read(image: &str, title: &oag_title::Title, face: &str) -> Option<oag_texture::fnt::Font> {
    let image = oag_testdata::image(image)?;
    let mut archives =
        oag_assets::Archives::open(&image.display().to_string(), title).expect("open the disc");
    Some(archives.read_font(face).expect("the face reads"))
}

/// Pulse's six, in each of its three faces: every row present, six distinct
/// cells, the four face buttons about square and the shoulders about twice as
/// wide as high.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulses_six_stand_ins_are_six_different_buttons() {
    for face in ["pulse_text", "Pulse_14", "Pulse_20"] {
        let Some(font) = read(
            "pulse-psp-eu.chd",
            oag_pulse::TITLE,
            &format!(r"Data\FE\Fonts\{face}.fnt"),
        ) else {
            return;
        };
        check(&font, oag_pulse::prompts::PROMPTS, face);
    }
}

/// The PS2 pressing shares Pulse's table, and its faces are not the PSP's: `Pulse_14` and
/// `Pulse_20` carry the six at their own sizes, `pulse_text` and the HUD faces none.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2s_six_stand_ins_are_six_different_buttons() {
    for face in ["Pulse_14", "Pulse_20"] {
        let Some(font) = read(
            "pulse-ps2-eu.chd",
            oag_pulse::TITLE,
            &format!(r"Data\FE\Fonts\{face}.fnt"),
        ) else {
            return;
        };
        check(&font, oag_pulse::prompts::PROMPTS, &format!("ps2 {face}"));
    }
}

/// HD's six, in `PS_BUTTONS.fnt`.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_six_stand_ins_are_six_different_buttons() {
    let Some(font) = read(
        "hdfury-ps3-eu-dec.iso",
        oag_hd::TITLE,
        r"Data\FE\Fonts\PS_BUTTONS.fnt",
    ) else {
        return;
    };
    check(&font, oag_hd::prompts::PROMPTS, "PS_BUTTONS");
}

/// Pure's six, in its menu face `FX300ANG.fnt`.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn pures_six_stand_ins_are_six_different_buttons() {
    let Some(font) = read(
        "pure-psp-eu.chd",
        oag_pure::TITLE,
        r"Data\FE\Fonts\FX300ANG.fnt",
    ) else {
        return;
    };
    check(&font, oag_pure::prompts::PROMPTS, "pure FX300ANG");
}

/// Omega's six, in `PS_BUTTONS.fnt` (HD's face at twice the size).
#[test]
#[ignore = "needs data/extracted/ps4"]
fn omegas_six_stand_ins_are_six_different_buttons() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.display().to_string(), oag_omega::TITLE)
        .expect("open the package");
    let font = archives
        .read_font(r"Data\FE\Fonts\PS_BUTTONS.fnt")
        .expect("the face reads");
    check(&font, oag_omega::prompts::PROMPTS, "omega PS_BUTTONS");
}

fn check(font: &oag_texture::fnt::Font, prompts: &Prompts, face: &str) {
    let mut seen = Vec::new();
    for &(stand_in, prompt) in prompts.glyphs {
        let (hash, width, height) =
            digest(font, stand_in).unwrap_or_else(|| panic!("{face}: {stand_in:?} has no glyph"));
        let square = (75..=115).contains(&(u32::from(width) * 100 / u32::from(height)));
        match prompt {
            Prompt::Cross | Prompt::Circle | Prompt::Square | Prompt::Triangle => {
                assert!(
                    square,
                    "{face}: {stand_in:?} is {width}x{height}, not a face button"
                );
            }
            Prompt::L | Prompt::R => assert!(
                u32::from(width) * 10 >= u32::from(height) * 14,
                "{face}: {stand_in:?} is {width}x{height}, not a shoulder"
            ),
            _ => {}
        }
        assert!(!seen.contains(&hash), "{face}: {stand_in:?} repeats a cell");
        seen.push(hash);
        if let Some(&(_, _, read)) = READ.iter().find(|&&(f, c, _)| f == face && c == stand_in) {
            assert_eq!(
                hash, read,
                "{face}: {stand_in:?} is not the glyph that was read"
            );
        }
        println!("{face} {stand_in} {prompt:?} {hash:016x} {width}x{height}");
    }
}
