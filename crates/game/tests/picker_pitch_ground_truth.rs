//! The language picker's rows step by one line of the disc's own `Default`
//! face on HD and Omega, and not by Pulse's 13-unit table.
//!
//! The picker's `<Menu>` authors no pitch (x, y, scale, colour, alignment
//! only), so the step is an inference - one line of the widget's own font -
//! but its input, the face height, is read off the disc. Dropping
//! `oag_title::BootProfile::picker_from_loaded_faces` or the
//! `set_picker_line_height` call in `boot::load_shell` puts the step back at
//! 13 x the menu scale and fails here.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(picker_pitch_ground_truth)'
//! ```

use oag_game::boot;
use oag_game::input::{Button, Input};
use oag_ui::frontend::{Draw, states};

fn load(source: &str) -> Option<boot::Boot> {
    let source = oag_testdata::exact(source)?;
    let options = boot::Options {
        language: None,
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-picker-pitch-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    Some(boot::load(&options).expect("loading the boot sequence"))
}

/// The `y` of every language row, in list order, and the `Default` face's
/// line height at the widget's own scale.
fn rows_and_pitch(source: &str) -> Option<(Vec<f32>, f32)> {
    let loaded = load(source)?;
    let mut frontend = loaded.frontend;
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(1.0 / 60.0, &mut input, None);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, &mut input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
    let names: Vec<&str> = loaded
        .languages
        .iter()
        .map(|l| l.native_name.as_str())
        .collect();
    let ys: Vec<f32> = frontend
        .draw_list()
        .iter()
        .filter_map(|d| match d {
            Draw::Text { text, y, x, .. } if names.contains(&text.as_str()) && *x > 500.0 => {
                Some(*y)
            }
            _ => None,
        })
        .collect();
    let scale = frontend
        .language_screen()
        .and_then(|s| s.menu.as_ref())
        .map_or(1.0, |m| m.scale);
    Some((ys, loaded.font.line_height * scale))
}

/// The picker's confirm prompt's glyph draws in the `Buttons` face, not as a
/// plain `Draw::Text` in a face that has no such codepoint (an empty box).
fn check_confirm_glyph(source: &str) {
    let Some(loaded) = load(source) else { return };
    let mut frontend = loaded.frontend;
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(1.0 / 60.0, &mut input, None);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, &mut input, None);
    let glyph = loaded.strings.get_or_id("FE_CONFIRM_BUTTON").to_string();
    assert_eq!(glyph.chars().count(), 1, "{glyph:?}");
    let draws = frontend.draw_list();
    assert!(
        draws.iter().any(|d| matches!(
            d,
            Draw::FacedText { role, text, .. } if *role == "Buttons" && *text == glyph
        )),
        "{source}: no Buttons-face draw of {glyph:?}"
    );
    assert!(
        !draws
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if *text == glyph)),
        "{source}: the glyph also draws as plain text"
    );
}

fn check(source: &str) {
    let Some((ys, pitch)) = rows_and_pitch(source) else {
        return;
    };
    assert!(ys.len() >= 10, "{source}: only {} rows drawn", ys.len());
    assert!(pitch > 20.0, "{source}: face height {pitch} is not HD's");
    for pair in ys.windows(2) {
        assert!(
            (pair[1] - pair[0] - pitch).abs() < 0.01,
            "{source}: rows step {} not {pitch}",
            pair[1] - pair[0]
        );
    }
}

#[test]
#[ignore = "needs the extracted HD data"]
fn hd_picker_rows_step_by_the_default_face() {
    check("data/extracted/ps3/hdfury-eu");
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_picker_rows_step_by_the_default_face() {
    check("data/extracted/ps4");
}

#[test]
#[ignore = "needs the extracted HD data"]
fn hd_picker_confirm_glyph_draws_in_the_buttons_face() {
    check_confirm_glyph("data/extracted/ps3/hdfury-eu");
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_picker_confirm_glyph_draws_in_the_buttons_face() {
    check_confirm_glyph("data/extracted/ps4");
}
