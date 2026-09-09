//! Pulse's own frame - the top bar and the two footer strips - against the
//! disc it was read off.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # Why this is a test of its own
//!
//! `menu_layout_ground_truth.rs` checks `FEGlobals` numbers against `Skin.xml`.
//! Neither it nor `menu_skin.rs` reaches the frame at all: `Top FE
//! Screen->FE Screen`'s top bar and footer strips are not `FEGlobals` values,
//! they are `<Image>` widgets sitting three anonymous `<Screen>` levels down -
//! grouping containers `Screens::collect_widgets` did not used to recurse
//! into, so they were silently dropped rather than drawn wrong. This is the
//! test that would have caught that: it reads the same three widgets two
//! ways, off the raw XML and off `oag_game::menu::read_frame`'s own output,
//! and checks they agree.

use std::path::PathBuf;

use oag_game::frontend::{Draw, Space};
use oag_game::screen::Screens;
use oag_game::sprite::Sheet;
use oag_tables::fexml;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// The texture every widget in this frame samples its own patch of.
const SHEET_TEXTURE: &str = r"Data\FE\Images\pulse_assets.mip";

/// `Top FE Screen->FE Screen`'s three `<Image>` widgets are on `screen.images`
/// at all - the parser-level half of the ground truth. Before
/// `collect_widgets` recursed into an anonymous `Screen`, this screen's own
/// `images` was empty: the widgets exist in the XML but three `<Screen>`
/// wrappers with no `name` stood between them and `Top FE Screen->FE
/// Screen`'s own collection pass, and nothing walked through.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn fe_screens_widgets_survive_the_anonymous_screen_wrappers() {
    let Some(path) = image() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .expect("Pulse carries a front-end root");
    let xml = fexml::text(&raw).expect("it is text");
    let screens = Screens::from_xml(&xml);

    let screen = screens
        .by_name(oag_pulse::frontend::states::FE_SCREEN)
        .expect("FE Screen parses");

    assert_eq!(
        screen
            .images
            .iter()
            .filter(|i| i.src == SHEET_TEXTURE)
            .count(),
        3,
        "the top bar and the two footer strips, all three sampling {SHEET_TEXTURE}: {:#?}",
        screen.images
    );

    let top_bar = screen
        .images
        .iter()
        .find(|i| i.src == SHEET_TEXTURE && i.width == Some(224.0))
        .expect("the top bar's own Image");
    assert_eq!((top_bar.x, top_bar.y), (20.0, 0.0));
    assert_eq!(top_bar.height, Some(24.0));
    assert_eq!(
        (top_bar.u, top_bar.v),
        (None, None),
        "the top bar names no U/V - its sub-rect is TxtrWidth/TxtrHeight alone, \
         at the sheet's own origin"
    );
    assert_eq!(
        (top_bar.texture_width, top_bar.texture_height),
        (Some(224.0), Some(24.0))
    );

    let mut footer: Vec<_> = screen
        .images
        .iter()
        .filter(|i| i.src == SHEET_TEXTURE && i.width == Some(454.0))
        .collect();
    footer.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    assert_eq!(footer.len(), 2, "the two footer strips");

    let (upper, lower) = (footer[0], footer[1]);
    assert_eq!((upper.x, upper.y), (12.0, 236.0));
    assert_eq!(upper.height, Some(14.0));
    assert_eq!((upper.u, upper.v), (Some(5.0), Some(27.0)));
    assert_eq!(
        (upper.texture_width, upper.texture_height),
        (Some(454.0), Some(14.0))
    );

    assert_eq!((lower.x, lower.y), (12.0, 249.0));
    assert_eq!(lower.height, Some(14.0));
    assert_eq!((lower.u, lower.v), (Some(5.0), Some(52.0)));
    assert_eq!(
        (lower.texture_width, lower.texture_height),
        (Some(454.0), Some(14.0))
    );
}

/// `read_frame` turns those same three widgets into three `Draw::Sprite`s,
/// each sampling its own patch of the sheet rather than the sheet's own
/// top-left corner - the sub-rect half of the ground truth, and the one a
/// parser-only check of the widgets above cannot see: `frame::read` builds
/// `uv` itself, off `placed.x`/`.y` plus the widget's own `U`/`V`, and getting
/// that wrong draws a picture rather than failing to parse.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_frame_samples_its_own_patch_of_the_shared_sheet() {
    let Some(path) = image() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let raw = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .expect("Pulse carries a front-end root");
    let xml = fexml::text(&raw).expect("it is text");
    let screens = Screens::from_xml(&xml);

    let blob = archives
        .read_name(SHEET_TEXTURE)
        .expect("pulse_assets.mip is on the disc");
    let mut notes = Vec::new();
    let sheet = Sheet::build(&[(SHEET_TEXTURE.to_string(), blob)], &mut notes);
    let placed = sheet.get(SHEET_TEXTURE).expect("it decoded");

    let space = Space::of(oag_disc::Platform::Psp);
    let frame = oag_game::menu::read_frame(
        &screens,
        &sheet,
        space,
        Some(oag_pulse::frontend::states::FE_SCREEN),
        None,
    );

    assert!(!frame.is_empty(), "Pulse's own frame draws something");
    assert_eq!(frame.marks.len(), 3, "one Draw::Sprite per Image widget");

    let sprites: Vec<([f32; 4], [f32; 4])> = frame
        .marks
        .iter()
        .map(|draw| match draw {
            Draw::Sprite { rect, uv, .. } => (*rect, *uv),
            other => panic!("FE Screen's own marks are all Image widgets: {other:?}"),
        })
        .collect();

    // The top bar names no U/V, but does name its own TxtrWidth/TxtrHeight -
    // its uv is the placed origin (no U/V offset) at its own declared size,
    // not the placed texture's full 512x128.
    let top_bar = sprites
        .iter()
        .find(|(rect, _)| rect[2] == 224.0)
        .expect("the top bar's own sprite");
    assert_eq!(top_bar.0, [20.0, 0.0, 224.0, 24.0]);
    assert_eq!(top_bar.1, [placed.x as f32, placed.y as f32, 224.0, 24.0]);

    // The two footer strips both name U/V - their uv has to be the placed
    // origin plus the widget's own sub-rect, not the placed origin alone.
    let mut footer: Vec<_> = sprites
        .iter()
        .filter(|(rect, _)| rect[2] == 454.0)
        .collect();
    footer.sort_by(|a, b| a.0[1].partial_cmp(&b.0[1]).unwrap());
    assert_eq!(footer.len(), 2, "the two footer strips");

    assert_eq!(footer[0].0, [12.0, 236.0, 454.0, 14.0]);
    assert_eq!(
        footer[0].1,
        [placed.x as f32 + 5.0, placed.y as f32 + 27.0, 454.0, 14.0]
    );
    assert_eq!(footer[1].0, [12.0, 249.0, 454.0, 14.0]);
    assert_eq!(
        footer[1].1,
        [placed.x as f32 + 5.0, placed.y as f32 + 52.0, 454.0, 14.0]
    );
}
