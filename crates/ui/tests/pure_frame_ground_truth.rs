//! Pure's own frame - the rule lines, scroll arrows and squiggle-text strip on
//! `FE Screen` - against the disc it was read off.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # Why this is a test of its own
//!
//! `menu_skin.rs` and `menu_layout_ground_truth.rs` never reach `FE Screen`'s
//! own widgets - `Skin.xml`'s comment calls it "all FE screens are inside
//! another screen", and its decorations sit behind a `<BackgroundController>`
//! that `Screens::collect_widgets` did not used to recurse into, so all of
//! them - and `Main Menu`'s whole backdrop with them - were silently dropped.
//! This pins that they now survive, and that the one thing genuinely
//! unauthored - `BackgroundImage`'s own texture - still draws nothing rather
//! than a guess.

use std::path::PathBuf;

use oag_tables::fexml;
use oag_ui::screen::Screens;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pure-psp-usa.chd")
}

fn fe_screen() -> Option<oag_ui::screen::Screens> {
    let path = image()?;
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pure::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(oag_pure::names::FRONTEND_ROOT)
        .expect("Pure carries a front-end root");
    let xml = fexml::text(&raw).expect("it is text");
    Some(Screens::from_xml(&xml))
}

/// `FE Screen`'s own `<Image>` widgets - the ones sitting behind
/// `<BackgroundController>` and the bare `<Animation>` wrappers around the
/// rule lines - are on `screen.images` at all.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn fe_screens_decorations_survive_backgroundcontroller() {
    let Some(screens) = fe_screen() else {
        return;
    };
    let screen = screens
        .by_name(oag_pure::frontend::states::FE_SCREEN)
        .expect("FE Screen parses");

    // BackgroundImage and BackgroundTopRightImage both name a `width` but no
    // `src` and no `color` - collected as neither an Image (no src) nor a
    // Fill (no color), so the count below is the *decorations* alone: the
    // top/bottom rule lines, the two scroll arrows, ArrowSelect and the
    // squiggle-text strip, every one of them naming a real `src` on
    // `Data\FE\Images\FETextures.mip` or `dotline.mip`.
    let named: Vec<&str> = screen.images.iter().map(|i| i.src.as_str()).collect();
    assert!(
        !named.is_empty(),
        "FE Screen authors real decorations - none reached screen.images: {named:#?}"
    );
    assert!(
        named
            .iter()
            .all(|src| src.ends_with("FETextures.mip") || src.ends_with("dotline.mip")),
        "every widget with a real src samples one of these two textures: {named:#?}"
    );

    // BackgroundImage/BackgroundTopRightImage: real widgets, no src, no
    // colour - draw nothing rather than a guess. Confirmed here, not assumed,
    // because a future disc revision naming one would silently start drawing
    // it, which is the point of pinning "neither is an Image nor a Fill" now.
    assert!(
        screen
            .images
            .iter()
            .all(|i| i.name.as_deref() != Some("BackgroundImage")),
        "BackgroundImage names no src - it must not appear as a drawn Image"
    );
    assert!(
        screen
            .fills
            .iter()
            .all(|f| { f.width != Some(512.0) || f.height != Some(272.0) }),
        "BackgroundImage names no colour either - it must not appear as a Fill"
    );
}

/// The squiggle-text "date" patch (`x=94 y=254 width=27 height=12`,
/// `U="147" V="0"` of `FETextures.mip`) is on `FE Screen` at its authored
/// coordinates and sub-rect - one concrete, uniquely-sized widget out of the
/// decorations above.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_date_patch_is_at_its_authored_position_and_sub_rect() {
    let Some(screens) = fe_screen() else {
        return;
    };
    let screen = screens
        .by_name(oag_pure::frontend::states::FE_SCREEN)
        .expect("FE Screen parses");

    let date = screen
        .images
        .iter()
        .find(|i| i.width == Some(27.0) && i.height == Some(12.0))
        .expect("the date patch");
    assert_eq!((date.x, date.y), (94.0, 254.0));
    assert_eq!((date.u, date.v), (Some(147.0), Some(0.0)));
    assert_eq!(
        (date.texture_width, date.texture_height),
        (Some(27.0), Some(12.0))
    );
    assert!(date.src.ends_with("FETextures.mip"));
}

/// `FE Screen`'s colour-only rules reach the menu frame as marks off the real disc: the footer
/// rule at y 251 and its five ticks, which need no texture
/// (`docs/ui/selection-screens.md`, "Pure's cursor arrow and footer frame").
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_footer_rule_and_its_ticks_are_frame_marks() {
    let Some(path) = image() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pure::TITLE)
        .expect("the archives open");
    // The frame's inks (`FEGlobals->DesignColor`) are the style skin's globals, as the boot
    // reads them (`oag_game::boot::screens`'s `style_skin_globals`).
    let style = archives
        .read_name(r"Data\Skins\Default\Skin.xml")
        .expect("Pure carries its default style skin");
    let style = Screens::from_xml(&fexml::text(&style).expect("it is text")).globals;
    let style: Vec<(&str, &str)> = style
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let raw = archives
        .read_name(oag_pure::names::FRONTEND_ROOT)
        .expect("Pure carries a front-end root");
    let screens =
        Screens::from_xml_with_fallback_globals(&fexml::text(&raw).expect("it is text"), &style);
    let frame = oag_ui::menu::read_frame(
        &screens,
        &[],
        oag_display::space::Space::PSP,
        Some(oag_pure::frontend::states::FE_SCREEN),
        None,
        None,
    );
    let rects = frame.mark_rects();
    assert!(
        rects.contains(&[21.0, 251.0, 444.0, 1.0]),
        "the footer rule: {rects:?}"
    );
    let ticks: Vec<f32> = rects
        .iter()
        .filter(|rect| rect[2] == 1.0 && (rect[1] == 251.0 || rect[1] == 252.0))
        .map(|rect| rect[0])
        .collect();
    assert_eq!(ticks, vec![21.0, 59.0, 88.0, 126.0, 226.0], "{rects:?}");
    assert!(
        rects.iter().all(|rect| rect[2] < 480.0 || rect[3] < 272.0),
        "no full-screen ground among the marks: {rects:?}"
    );
}
