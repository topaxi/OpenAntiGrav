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
    let unnamed = crate::menu::read_frame(&screens(), &Sheet::default(), space, None);
    assert!(unnamed.is_empty());
    let missing = crate::menu::read_frame(
        &screens(),
        &Sheet::default(),
        space,
        Some("A Screen This Disc Has Not Got"),
    );
    assert!(missing.is_empty());
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
