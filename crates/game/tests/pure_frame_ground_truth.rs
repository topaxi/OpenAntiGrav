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

use std::path::{Path, PathBuf};

use oag_formats::fexml;
use oag_game::screen::Screens;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pure-psp-usa.chd");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    eprintln!("skipping: {} is not present", path.display());
    None
}

fn fe_screen() -> Option<oag_game::screen::Screens> {
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
