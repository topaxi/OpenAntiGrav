//! What each screen puts on the display list: the logo and its press-start
//! line, the picker rows, the video quad, and the near-black lift.
//!
//! Split out of `frontend/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

#[test]
fn show_logo_draws_the_pulse_logo_and_its_press_start_line() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);
    assert!(frontend.machine().is(states::SHOW_LOGO));

    let draws = frontend.draw_list();
    // The `Text` widget's own coordinates, colour and alignment, from the
    // USA disc's `Skin.xml`. `y` is 220 there and 230 on the EU disc, which
    // drops the `BOOT_LEGAL` line above it.
    let text = draws
        .iter()
        .find_map(|d| match d {
            Draw::Text {
                x,
                y,
                align,
                color,
                text,
                ..
            } => Some((*x, *y, *align, *color, text.clone())),
            _ => None,
        })
        .expect("Show Logo draws its text widget");
    assert_eq!(text.0, 460.0);
    assert_eq!(text.1, 220.0);
    assert_eq!(text.2, Align::Right);
    assert_eq!(text.3, argb_to_rgba(0x7fff_ffff));
    assert_eq!(
        text.4, "BOOT_PRESS_START",
        "with no string table loaded the id stands in for its own string"
    );

    // The logo itself needs a sprite sheet, which these tests do not build.
    // What must hold either way is that the screen's `Image` was found and
    // nothing else crept in: a black fill, then the text.
    assert!(matches!(draws[0], Draw::Fill { .. }));
    assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
}

/// `BOOT_LEGAL`'s `Viewport`-derived `wrap_width` survives `draw_screen`
/// unwrapped - the actual line-splitting is `crate::render`'s job, since only
/// it holds the font metrics to split correctly. This is its own fixture
/// rather than [`XML`]'s, which deliberately omits the `Viewport` (see its own
/// comment) to keep the shared boot-sequence tests free of it.
#[test]
fn a_widthlimited_text_carries_its_wrap_width_into_the_draw_list() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen type="FEMain" name="Top FE Screen">
    <Screen name="FE Screen">
      <Screen name="Show Logo">
        <Viewport>
          <Values x="40" y="0" height="480" width="400"></Values>
          <Text name="USLegalText" delay="0" transition="0">
            <Values align="left" idstring="BOOT_LEGAL" font="small" scale="0.7" x="40" y="250" widthlimited="true" color="0x7FFFFFFF"></Values>
          </Text>
        </Viewport>
      </Screen>
    </Screen>
  </Screen>
</Screen>
"#,
    );
    let frontend = Frontend::new(
        screens,
        StringTable::default(),
        Vec::new(),
        Vec::new(),
        0,
        false,
    );
    let draws = frontend.draw_screen("Show Logo");
    let legal = draws
        .iter()
        .find_map(|d| match d {
            Draw::Text {
                text, wrap_width, ..
            } if text == "BOOT_LEGAL" => Some(*wrap_width),
            _ => None,
        })
        .expect("Show Logo draws BOOT_LEGAL");
    assert_eq!(legal, Some(400.0));
}

#[test]
fn circle_does_not_select() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);

    input.begin_frame(Button::Circle.bit());
    frontend.update(FRAME, &mut input, None);
    assert_eq!(
        frontend.chosen(),
        None,
        "cancel is circle, activate is cross"
    );
}

#[test]
fn the_discs_own_redirect_is_reported_not_followed() {
    let frontend = frontend(300);
    assert_eq!(frontend.language_auto_redirect(), Some("LogoFMV"));
}

#[test]
fn the_picker_draws_one_row_per_language() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);

    let draws = frontend.draw_list();
    let rows: Vec<&String> = draws
        .iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert!(rows.iter().any(|t| *t == "Français"), "{rows:?}");
    assert!(rows.iter().any(|t| *t == "Deutsch"), "{rows:?}");
    assert!(rows.iter().any(|t| *t == "English"), "{rows:?}");
}

#[test]
fn the_intro_draws_no_video_quad_without_a_picture() {
    let frontend = frontend(300);
    let draws = frontend.draw_list();
    assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
}

#[test]
fn the_intro_draws_a_video_quad_when_there_is_one() {
    let frontend = Frontend::new(
        Screens::from_xml(XML),
        StringTable::default(),
        languages(),
        Vec::new(),
        300,
        true,
    );
    let draws = frontend.draw_list();
    assert!(draws.iter().any(|d| matches!(d, Draw::Video { .. })));
}

#[test]
fn near_black_text_is_lifted_off_a_black_screen() {
    assert_ne!(lighten([0.0, 0.0, 0.0, 1.0]), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(lighten([1.0, 0.5, 0.2, 1.0]), [1.0, 0.5, 0.2, 1.0]);
}

#[test]
fn an_empty_language_list_does_not_panic() {
    let mut frontend = Frontend::new(
        Screens::from_xml(XML),
        StringTable::default(),
        Vec::new(),
        Vec::new(),
        300,
        false,
    );
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    assert_eq!(frontend.chosen(), None);
    let _ = frontend.draw_list();
}
