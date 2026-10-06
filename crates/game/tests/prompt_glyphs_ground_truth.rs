//! The stand-in codepoints in `oag_pulse::prompts` and `oag_hd::prompts` are
//! the glyphs they claim to be.
//!
//! Each table row says "this codepoint draws this control". That was read off
//! the rendered glyph cells (`oag-tools --example prompt_glyph_probe`), and
//! this pins it: the cell of every stand-in in the disc's own face, hashed, so
//! a table that swaps two rows - or a face whose `ε` stops being a cross -
//! fails here rather than showing a player the wrong button. The hash is of
//! alpha bytes only, and no disc content is stored.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(prompt_glyphs_ground_truth)'
//! ```

use oag_title::prompts::{Prompt, Prompts};

/// The cells whose pictures were read: `Pulse_20.fnt` and `PS_BUTTONS.fnt`,
/// `ε` the circled cross, `γ` circle, `δ` square, `β` triangle, `Λ`/`Ν` the
/// L and R boxes. (face, codepoint, FNV-1a of the cell's alpha.)
const READ: &[(&str, char, u64)] = &[
    ("Pulse_20", 'ε', 0x4e78_a1ee_83a0_e278),
    ("Pulse_20", 'γ', 0x1eec_4c80_5e08_3ca4),
    ("Pulse_20", 'δ', 0x0dcf_7f7c_985d_6f08),
    ("Pulse_20", 'β', 0x0a18_686c_832d_b108),
    ("Pulse_20", 'Λ', 0xdd58_0cf5_2689_c56e),
    ("Pulse_20", 'Ν', 0x8437_305b_ac1b_0612),
    ("PS_BUTTONS", 'ε', 0x8667_ec82_e0ec_2715),
    ("PS_BUTTONS", 'γ', 0x2dac_ff1c_2c39_a4c9),
    ("PS_BUTTONS", 'δ', 0x3442_bf52_a954_b7f0),
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

/// HD's three, in `PS_BUTTONS.fnt`.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_three_stand_ins_are_three_different_buttons() {
    let Some(font) = read(
        "hdfury-ps3-eu-dec.iso",
        oag_hd::TITLE,
        r"Data\FE\Fonts\PS_BUTTONS.fnt",
    ) else {
        return;
    };
    check(&font, oag_hd::prompts::PROMPTS, "PS_BUTTONS");
}

fn check(font: &oag_texture::fnt::Font, prompts: &Prompts, face: &str) {
    let mut seen = Vec::new();
    for &(stand_in, prompt) in prompts.glyphs {
        let (hash, width, height) =
            digest(font, stand_in).unwrap_or_else(|| panic!("{face}: {stand_in:?} has no glyph"));
        let square = (85..=115).contains(&(u32::from(width) * 100 / u32::from(height)));
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
