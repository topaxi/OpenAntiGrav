//! `BOOT_LEGAL` against the real USA disc: parsed, string-resolved, and
//! wrapped through the real `Small` font.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! `docs/architecture/frontend-boot.md` used to say drawing `BOOT_LEGAL`
//! unwrapped "would run a single line off both edges of the screen", so the
//! disc's real 118-byte copyright string is asserted here rather than
//! transcribed by hand into the doc: it is 114 *characters* (the `©` is one
//! codepoint, two UTF-8 bytes), and at the disc's own `font="small"
//! scale="0.7"` it wraps to exactly two lines against the enclosing
//! `Viewport`'s `width="400"` - both comfortably inside it, the second
//! starting at `y=261.9` on a 272-tall screen. So the fix was real wrapping,
//! not a bigger viewport or a smaller scale.

use std::path::PathBuf;

use oag_game::loading;
use oag_ui::font::Atlas;
use oag_ui::language::roles;
use oag_ui::{font, screen};

const PSP_USA: &str = "data/images/pulse-psp-usa.chd";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PSP_USA)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn boot_legal_wraps_to_two_lines_that_clear_the_screen() {
    let Some(path) = image() else { return };
    let mut archives = oag_pulse::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opens as Pulse: {e}"));

    let blob = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .expect("Skin.xml");
    let xml = if oag_tables::fexml::is_fexml(&blob) {
        oag_tables::fexml::expand(&blob).expect("expanding Skin.xml")
    } else {
        String::from_utf8(blob).expect("Skin.xml is text")
    };
    let screens = screen::Screens::from_xml(&xml);
    let logo = screens.by_name("Show Logo").expect("Show Logo screen");
    let legal = logo
        .texts
        .iter()
        .find(|t| t.idstring.as_deref() == Some("BOOT_LEGAL"))
        .expect("USLegalText/BOOT_LEGAL");

    // The `Viewport`'s own `width`, not an inset or a screen-wide guess - see
    // `Text::wrap_width`'s doc comment for why this is the one number the XML
    // gives to wrap against.
    assert_eq!(legal.wrap_width, Some(400.0));
    assert_eq!(legal.font, "small");
    assert!((legal.scale - 0.7).abs() < 1e-6);
    assert_eq!(legal.y, 250.0);

    let mut report = Vec::new();
    let languages = oag_ui::language::load::load_languages(
        &mut archives,
        oag_pulse::LANGUAGE_PLUGINS,
        None,
        &mut report,
    );
    let strings =
        oag_ui::language::load::load_strings(&mut archives, &languages, None, &mut report);
    let text = strings.get_or_id("BOOT_LEGAL");
    assert_eq!(
        text.chars().count(),
        114,
        "the disc's copyright line: {text:?}"
    );

    let font_name = languages
        .iter()
        .find_map(|l| l.font(roles::SMALL))
        .expect("a language names the Small role");
    let real_font = archives
        .read_font(font_name)
        .unwrap_or_else(|e| panic!("{font_name}: {e}"));
    let atlas = Atlas::from_font(&real_font);

    let width = legal.wrap_width.unwrap();
    let lines = loading::wrap(&atlas, text, legal.scale, width);
    assert_eq!(
        lines.len(),
        2,
        "BOOT_LEGAL's real wrap at the Viewport's width: {lines:#?}"
    );
    for line in &lines {
        let measured = font::measure(&atlas, line) * legal.scale;
        assert!(
            measured <= width,
            "{line:?} is {measured}px, over the {width}px wrap width"
        );
    }

    // The second line's top edge, top-anchored and growing downward from the
    // widget's own `y`.
    let line_height = atlas.line_height * legal.scale;
    let second_line_y = legal.y + line_height;
    assert!((second_line_y - 261.9).abs() < 0.1, "{second_line_y}");

    // **This is the near miss, measured rather than assumed.** By the
    // atlas's own nominal `line_height` (17px, the same 17 every other
    // `font="small"` role uses - `oag_ui::frontend::font_line_height`) two
    // lines from `y=250` end at 273.8, half a pixel past the PSP's 272-tall
    // screen. Real glyph ink is shorter than the line box a proportional
    // font reserves, though: the tallest cell this exact string actually
    // uses (an ascender/cap-height character, not a diacritic) is 14px, 9.8
    // scaled, so the second line's own ink bottoms out at 271.7 and clears
    // the screen by three tenths of a pixel. So the fix needed no bottom
    // anchor or shrink-to-fit - top-anchored, growing downward, is right for
    // this string on this screen. `load_strings` above pins English
    // (`preferred: None` falls through to it, see `chosen_language`), so this
    // only measures English's copyright line - it would need looping
    // `LANGUAGE_PLUGINS` to say the same for the other four.
    let tallest_glyph = text
        .chars()
        .filter_map(|c| atlas.cell(c))
        .map(|cell| cell.height)
        .max()
        .expect("BOOT_LEGAL has at least one drawable glyph");
    let ink_bottom = second_line_y + tallest_glyph as f32 * legal.scale;
    assert!(
        ink_bottom <= 272.0,
        "the second line's own ink runs to {ink_bottom}, past the 272-tall screen"
    );
}
