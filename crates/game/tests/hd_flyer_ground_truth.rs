//! The 3-D flyer card behind Wipeout HD's `Grid Selection`, against the disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_flyer_ground_truth)'
//! ```
//!
//! What `docs/ui/campaign-screens.md`'s "The flyer behind `Grid Selection`"
//! claims of the disc, pinned: the widget's authored values, that each of the
//! base campaign's eight cards decodes through the game's own preview path
//! with every material and texture found, and that every grid's logo is an
//! archive entry of its own. The pose and the cut are **chosen, not
//! measured** and are not pinned here.

use std::path::PathBuf;

use oag_ui::campaign::flyer;

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// The base campaign's eight `FlyerName`s, `grid_00.xml`..`grid_07.xml`.
const HD_CARDS: [&str; 8] = [
    "01_uplift",
    "02_warped",
    "03_frenzy",
    "04_vertigo",
    "05_headrush",
    "06_speedfreak",
    "07_dropzone",
    "08_meltdown",
];

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn archives(image: &std::path::Path) -> oag_assets::Archives {
    oag_assets::Archives::open(&image.to_string_lossy(), oag_hd::TITLE).expect("the archives open")
}

/// `DATA06`'s copy of the screen file, which is the one the game reads.
fn screen_xml(archives: &mut oag_assets::Archives) -> String {
    let (_, blob) = archives
        .read_every_name(oag_hd::campaign::SCREEN_ENTRY)
        .into_iter()
        .find(|(label, _)| label.ends_with(oag_hd::campaign::SELECTION_SCREEN_ARCHIVE))
        .expect("DATA06 carries the screen file");
    String::from_utf8(blob).expect("plain UTF-8")
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_widget_is_authored_at_the_top_level_with_the_placeholder_model() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let xml = screen_xml(&mut archives);
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let widgets = flyer::read(&xml, &screens);
    let named: Vec<&str> = widgets.iter().map(|w| w.name.as_str()).collect();
    assert_eq!(
        named,
        [
            "FuryCampaignFlyerModel",
            "HDCampaignFlyerModel",
            "FlyerModel"
        ],
        "document order"
    );
    let grid = &widgets[2];
    assert_eq!(grid.src, r"Data\FE\Flyers\00_flyer.vex");
    assert_eq!(grid.origin, [960.0, 540.0]);
    assert_eq!(grid.rotation, [0.0, 1.5]);
    assert_eq!(grid.rotation_centre_offset_x, -60.0);
    assert_eq!(grid.ortho_scale, None);
    assert_eq!(widgets[0].rotation, [0.0, 0.6], "the `0.6f` literal parses");
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_base_card_decodes_and_every_grid_has_a_logo() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let xml = screen_xml(&mut archives);
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let widget = flyer::read(&xml, &screens)
        .into_iter()
        .find(|w| w.name == "FlyerModel")
        .expect("the grid widget");
    let names: Vec<String> = HD_CARDS.iter().map(ToString::to_string).collect();
    let flyers = oag_game::flyer::Flyers::load(&mut archives, widget, &names);
    assert!(flyers.report.is_empty(), "{:?}", flyers.report);
    for name in HD_CARDS {
        assert!(flyers.has(name), "{name} did not decode");
        assert!(
            archives.read_name(&flyer::logo_entry(name)).is_ok(),
            "{name}'s logo is not an archive entry"
        );
    }
}

/// The card is a flat quad set: the widest card is 115 units across and no
/// vertex of any base card is more than a few units off the plane. A Fury card
/// is authored at a third of the width and 28 units deep, which is why none is
/// drawn - see `oag_game::campaign::load_hd`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_base_card_is_flat_and_a_fury_card_is_not() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let depth = |archives: &mut oag_assets::Archives, name: &str| {
        let model = oag_game::preview::model(archives, &flyer::front_entry(name))
            .unwrap_or_else(|error| panic!("{name}: {error:#}"));
        let (lo, hi) = model
            .vertices
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), v| {
                (lo.min(v.position[2]), hi.max(v.position[2]))
            });
        hi - lo
    };
    assert!(depth(&mut archives, "01_uplift") < 10.0);
    assert!(depth(&mut archives, "09_Blitzed") > 20.0);
}
