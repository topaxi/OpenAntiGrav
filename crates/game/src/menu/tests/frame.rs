//! What a menu frame is read as, without a disc.
//!
//! The half that needs one - that `line.gtf` and `Title_Arrow_HD.gtf` decode and
//! land in the sheet, and that the served archive's palette is what comes out -
//! is `crates/game/tests/hd_menu_ground_truth.rs`. What is here is the reading
//! itself: which widgets are taken, which are not, and what an absence produces.

use super::*;

use crate::screen::Screens;
use crate::sprite::Sheet;

/// A frame screen shaped like Wipeout HD's, in plain XML.
///
/// Not HD's file - a fixture, because this is a test about the *reading*. The
/// shape is the one the disc has: a clear naming a global, two stretched rules
/// and a mark, and a `<Text>` that must not be drawn.
const XML: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<Screen>
  <Variable global="BG"><Values String="0xff112233"></Values></Variable>
  <Variable global="Ink"><Values String="0xff445566"></Values></Variable>
  <Screen name="Top">
    <Screen name="Frame">
      <ScreenClear><Values Colour="FEGlobals->BG"></Values></ScreenClear>
      <Image><Values x="160" y="110" width="1600" height="8" color="FEGlobals->Ink" src="rule.gtf"></Values></Image>
      <Image><Values x="160" y="73" width="32" height="32" color="FEGlobals->Ink" src="mark.gtf"></Values></Image>
      <Text><Values idstring="FE_TRIAL_MODE" x="1760" y="62" color="FEGlobals->Ink"></Values></Text>
    </Screen>
  </Screen>
</Screen>"#;

fn screens() -> Screens {
    Screens::from_xml(XML)
}

/// The clear resolves its global, and the frame carries it as one fill.
#[test]
fn the_clear_is_read_and_resolved() {
    let frame = crate::menu::read_frame(
        &screens(),
        &Sheet::default(),
        crate::frontend::Space::HD,
        Some("Frame"),
        None,
    );
    let Some(Draw::Fill { rect, color }) = frame.clear else {
        panic!("the screen clears: {frame:?}");
    };
    assert_eq!(rect, [0.0, 0.0, 1920.0, 1080.0], "the whole grid");
    // `0xff112233`, as straight-alpha RGBA.
    assert!((color[0] - 17.0 / 255.0).abs() < 0.001, "{color:?}");
    assert!((color[1] - 34.0 / 255.0).abs() < 0.001, "{color:?}");
    assert!((color[2] - 51.0 / 255.0).abs() < 0.001, "{color:?}");
    assert!((color[3] - 1.0).abs() < 0.001, "{color:?}");
}

/// An image with no placement is skipped rather than drawn wrong.
///
/// The empty sheet is the case a real boot hits when a texture will not decode,
/// and a missing rule is the honest outcome: nothing is invented to stand in for
/// it, and the clear still lands.
#[test]
fn an_image_the_sheet_has_no_placement_for_is_skipped() {
    let frame = crate::menu::read_frame(
        &screens(),
        &Sheet::default(),
        crate::frontend::Space::HD,
        Some("Frame"),
        None,
    );
    assert!(frame.marks.is_empty(), "no placements, so no marks");
    assert!(frame.clear.is_some(), "and the clear is unaffected");
    assert!(frame.ink.is_none(), "with no marks there is no ink either");
}

/// A title that names no frame screen gets an empty one, and so does a name
/// that is not on this source.
#[test]
fn no_name_and_a_wrong_name_both_read_as_no_frame() {
    let space = crate::frontend::Space::HD;
    let unnamed = crate::menu::read_frame(&screens(), &Sheet::default(), space, None, None);
    assert!(unnamed.is_empty());
    let missing = crate::menu::read_frame(
        &screens(),
        &Sheet::default(),
        space,
        Some("A Screen This Disc Has Not Got"),
        None,
    );
    assert!(missing.is_empty());
}

/// The selected fill is read straight off the globals table, by name - not
/// off any widget's own marks, unlike the clear or `ink` above.
///
/// Two cases in one: the name resolves when the disc's globals carry it, and
/// `None` (no strip, or a title-package name this disc's globals do not
/// carry) reads as no highlight rather than a wrong colour.
#[test]
fn the_selected_fill_is_read_from_the_globals_table_by_name() {
    let frame = crate::menu::read_frame(
        &screens(),
        &Sheet::default(),
        crate::frontend::Space::HD,
        Some("Frame"),
        Some("Ink"),
    );
    let Some(color) = frame.tab_selected else {
        panic!("Ink is one of the fixture's own globals: {frame:?}");
    };
    // `0xff445566`, as straight-alpha RGBA - the fixture's `Ink` global.
    assert!((color[0] - 68.0 / 255.0).abs() < 0.001, "{color:?}");
    assert!((color[1] - 85.0 / 255.0).abs() < 0.001, "{color:?}");
    assert!((color[2] - 102.0 / 255.0).abs() < 0.001, "{color:?}");

    let no_name = crate::menu::read_frame(
        &screens(),
        &Sheet::default(),
        crate::frontend::Space::HD,
        Some("Frame"),
        None,
    );
    assert!(no_name.tab_selected.is_none(), "no name, nothing to read");

    let unknown_name = crate::menu::read_frame(
        &screens(),
        &Sheet::default(),
        crate::frontend::Space::HD,
        Some("Frame"),
        Some("HD_Blue"),
    );
    assert!(
        unknown_name.tab_selected.is_none(),
        "the fixture carries no HD_Blue"
    );
}

/// The screen's `<Text>` widgets are not part of the frame.
///
/// The fixture's is `FE_TRIAL_MODE`, which is the real one: HD authors the trial
/// build's text on the same screen as its rules, and a frame that drew every
/// widget would put "trial mode" across a retail menu. See `menu::frame`.
#[test]
fn the_frames_text_widgets_are_left_alone() {
    let screens = screens();
    let screen = screens.by_name("Frame").expect("the fixture has it");
    assert_eq!(screen.texts.len(), 1, "the fixture authors one");

    let frame = crate::menu::read_frame(
        &screens,
        &Sheet::default(),
        crate::frontend::Space::HD,
        Some("Frame"),
        None,
    );
    let drawn = frame.clear.iter().chain(frame.marks.iter()).count();
    assert!(
        !frame
            .clear
            .iter()
            .chain(frame.marks.iter())
            .any(|draw| matches!(draw, Draw::Text { .. })),
        "no text reaches the frame: {frame:?}"
    );
    assert_eq!(drawn, 1, "the clear alone, on an empty sheet");
}

/// [`Frame::content_bottom`] takes the lowest mark in the screen's lower
/// half - Pulse's own two footer strips, at `y=236` and `y=249` of a
/// 272-tall screen (`docs/ui/menus-original.md`'s "The top bar and the
/// footer's two strips") - and ignores the top bar sitting at `y=0`, upper
/// half or not.
#[test]
fn content_bottom_is_the_lowest_mark_below_the_midline() {
    let sprite = |y: f32| Draw::Sprite {
        rect: [12.0, y, 454.0, 14.0],
        uv: [0.0, 0.0, 0.0, 0.0],
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let frame = Frame {
        marks: vec![sprite(0.0), sprite(236.0), sprite(249.0)],
        ..Frame::default()
    };
    assert_eq!(
        frame.content_bottom(crate::frontend::Space::PSP),
        Some(236.0),
        "the higher of the two footer strips, not the top bar"
    );
}

/// A frame with nothing in the lower half - no frame at all, or one whose
/// marks sit entirely above the midline - answers `None` rather than the
/// screen's own edge, so a caller falls back to whatever it used before
/// this existed.
#[test]
fn content_bottom_is_none_with_nothing_below_the_midline() {
    assert_eq!(
        Frame::default().content_bottom(crate::frontend::Space::PSP),
        None,
        "no marks at all"
    );
    let top_bar_only = Frame {
        marks: vec![Draw::Sprite {
            rect: [20.0, 0.0, 224.0, 24.0],
            uv: [0.0, 0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }],
        ..Frame::default()
    };
    assert_eq!(
        top_bar_only.content_bottom(crate::frontend::Space::PSP),
        None,
        "the one mark is above the midline"
    );
}

/// [`Frame::backdrops`] with a race behind the menus: the full-screen mark
/// (Pulse's own `FE_SCREEN` authors exactly this shape ahead of its top bar
/// and its two footer strips) is dropped so the parked race's picture, or
/// the pause overlay over it, shows through - but the small top-bar mark
/// stays, because it is structural chrome rather than a background.
///
/// Live-reproduced, not only asserted here - a real `Escape` from a running
/// race, before and after this fix, screenshotted either side of it.
#[test]
fn backdrops_drops_a_full_screen_mark_but_keeps_a_small_one_behind_a_race() {
    let space = crate::frontend::Space::PSP;
    let full_screen = Draw::Sprite {
        rect: [0.0, 0.0, space.size.0, space.size.1],
        uv: [0.0, 0.0, 480.0, 256.0],
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let top_bar = Draw::Sprite {
        rect: [20.0, 0.0, 224.0, 24.0],
        uv: [0.0, 257.0, 224.0, 24.0],
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let frame = Frame {
        marks: vec![full_screen.clone(), top_bar.clone()],
        ..Frame::default()
    };

    let behind = frame.backdrops(space, None, None, true);
    assert_eq!(behind, vec![top_bar.clone()], "only the small mark stays");

    let ordinary = frame.backdrops(space, None, None, false);
    assert_eq!(
        ordinary,
        vec![full_screen, top_bar],
        "both marks draw exactly as `draw_list` always has, race or not"
    );
}

/// The same drop applies to the frame's own `<ScreenClear>` (or, absent one,
/// [`oag_title::MenuSkin::background`], passed in the same way) - it is
/// always full-screen by construction, so `backdrops` need not compare its
/// rect the way it does for a mark.
#[test]
fn backdrops_drops_the_clear_behind_a_race_too() {
    let space = crate::frontend::Space::PSP;
    let clear = Draw::Fill {
        rect: [0.0, 0.0, space.size.0, space.size.1],
        color: [0.1, 0.1, 0.1, 1.0],
    };
    let frame = Frame {
        clear: Some(clear.clone()),
        ..Frame::default()
    };

    assert!(
        frame.backdrops(space, None, None, true).is_empty(),
        "the clear alone would hide the race just as the mark did"
    );
    assert_eq!(frame.backdrops(space, None, None, false), vec![clear]);
}
