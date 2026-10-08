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
//! sixteen grids' cards decodes through the game's own preview path with every
//! material and texture found and its own camera read, and that every grid's
//! logo is an archive entry of its own. The pose and the window are **chosen, not
//! measured** and are not pinned here.

use std::path::PathBuf;

use oag_ui_screens::campaign::flyer;

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

/// Fury's eight, `grid_08.xml`..`grid_15.xml`, spelled as the disc spells them
/// (`turbulance` included).
const FURY_CARDS: [&str; 8] = [
    "09_blitzed",
    "10_impact",
    "11_voltage",
    "12_turbulance",
    "13_vortex",
    "14_corruption",
    "15_nuked",
    "16_aftermath",
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
fn every_card_of_both_campaigns_decodes_and_every_grid_has_a_logo() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let xml = screen_xml(&mut archives);
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let widget = flyer::read(&xml, &screens)
        .into_iter()
        .find(|w| w.name == "FlyerModel")
        .expect("the grid widget");
    let names: Vec<String> = HD_CARDS
        .iter()
        .chain(&FURY_CARDS)
        .map(ToString::to_string)
        .collect();
    let cards: Vec<oag_game::flyer::CardSpec> = names
        .iter()
        .map(|name| oag_game::flyer::CardSpec {
            flyer: name.clone(),
            side: oag_game::flyer::Side::Front,
            window: if HD_CARDS.contains(&name.as_str()) {
                oag_game::flyer::HD_WINDOW
            } else {
                oag_game::flyer::FURY_WINDOW
            },
            stretch: 1.0,
        })
        .collect();
    let flyers = oag_game::flyer::Flyers::load(&mut archives, vec![widget], &cards);
    assert!(flyers.report.is_empty(), "{:?}", flyers.report);
    for name in &names {
        assert!(
            flyers.has(&oag_game::flyer::Flyers::grid_show(name)),
            "{name} did not decode"
        );
        let [x, y, width, height] = flyers
            .card_rect(
                &oag_game::flyer::Flyers::grid_show(name),
                oag_display::space::Space::HD,
            )
            .unwrap_or_else(|| panic!("{name} has no rectangle"));
        assert!(
            width > 400.0 && height > 300.0,
            "{name}: {width} x {height}"
        );
        assert!(x > 700.0 && y > 50.0, "{name}: at {x}, {y}");
        assert!(
            archives.read_name(&flyer::logo_entry(name)).is_ok(),
            "{name}'s logo is not an archive entry"
        );
    }
}

/// Every flyer authors the camera it was composed for, a pure translation down
/// the card's axis: 92.4957 for the base campaign's eight, 12 for Fury's and
/// for the two campaign cards. Its `+0x1c` word is the card's aspect ratio to
/// four digits - the base body is 115 by 74.8 - and `+0x20` is what separates
/// the three families.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_flyer_authors_its_camera_on_the_cards_axis() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let mut check = |name: &str, z: f32, aspect: f32, word: u32| {
        let blob = archives
            .read_name(&flyer::front_entry(name))
            .unwrap_or_else(|error| panic!("{name}: {error:#}"));
        let cameras = oag_vex::camera::cameras(&blob);
        assert_eq!(cameras.len(), 1, "{name}");
        let camera = &cameras[0];
        assert_eq!(camera.position()[0..2], [0.0, 0.0], "{name}");
        assert!(
            (camera.position()[2] - z).abs() < 1e-3,
            "{name}: {camera:?}"
        );
        assert!(
            (camera.value_1c - aspect).abs() < 1e-4,
            "{name}: {camera:?}"
        );
        assert_eq!(camera.value_20, word, "{name}");
    };
    for name in HD_CARDS {
        check(name, 92.4957, 1.5380, 0x4f15);
    }
    for name in FURY_CARDS {
        check(name, 12.0, 1.5389, 0x4a2c);
    }
    for name in ["fury_campaign", "hd_campaign"] {
        check(name, 12.0, 1.0833, 0x3621);
    }
}

/// `Campaign Selection`'s two cards land where RPCS3's frames put them, off
/// the disc's own widget numbers (`z`, `orthoScale`, `RotY`, the pivot): with
/// Fury selected its card faces front from authored column 240 to 1038 and
/// row 212 down, and `HD`'s is turned with its left edge at 1105; with `HD`
/// selected its card's left edge is at 879. Measured on
/// `campaign-settled-a` and `campaign-right`; the tolerance is the width of
/// the fit, not of the frame.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn campaign_selections_cards_land_where_rpcs3_shows_them() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let xml = screen_xml(&mut archives);
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let widgets = flyer::read(&xml, &screens);
    let cards: Vec<oag_game::flyer::CardSpec> =
        [flyer::FURY_CAMPAIGN_FLYER, flyer::HD_CAMPAIGN_FLYER]
            .map(|name| oag_game::flyer::CardSpec {
                flyer: name.to_string(),
                side: oag_game::flyer::Side::Front,
                window: oag_game::flyer::FURY_WINDOW,
                stretch: oag_game::flyer::CAMPAIGN_STRETCH,
            })
            .to_vec();
    let flyers = oag_game::flyer::Flyers::load(&mut archives, widgets, &cards);
    assert!(flyers.report.is_empty(), "{:?}", flyers.report);
    let rects = |selected| {
        let shows = flyers.selection_shows(selected);
        assert_eq!(shows.len(), 2, "both widgets are on the disc");
        shows
            .into_iter()
            .map(|show| {
                flyers
                    .card_rect(&show, oag_display::space::Space::HD)
                    .unwrap_or_else(|| panic!("{} has no rectangle", show.flyer))
            })
            .collect::<Vec<_>>()
    };
    let near = |got: f32, want: f32, what: &str| {
        assert!((got - want).abs() < 40.0, "{what}: {got} against {want}");
    };
    use oag_ui_screens::campaign::selection::Campaign;
    let [fury, hd] = rects(Campaign::Fury).try_into().expect("two");
    near(fury[0], 240.0, "Fury's left edge, selected");
    near(fury[0] + fury[2], 1038.0, "Fury's right edge, selected");
    near(fury[1], 212.0, "Fury's top, selected");
    near(hd[0], 1105.0, "HD's left edge, turned");
    let [_, hd] = rects(Campaign::Hd).try_into().expect("two");
    near(hd[0], 879.0, "HD's left edge, selected");
}

/// The placeholder is the card: 102.4 by 66.6 with a chamfered corner and
/// notches (the front face covers 6,783.6 square units of the rectangle's
/// 6,819.8), and its reflection fades from alpha `0x4c` over half the card's
/// height.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_placeholder_is_the_cards_outline_and_reflection() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let shell = oag_game::flyer::Shell::load(&mut archives).expect("the shell reads");
    assert!((shell.half_size[0] - 51.2).abs() < 0.02, "{shell:?}");
    assert!((shell.half_size[1] - 33.3).abs() < 0.02, "{shell:?}");
    let area: f32 = shell
        .outline
        .iter()
        .map(|[a, b, c]| {
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
        })
        .sum();
    assert!((area - 6783.6).abs() < 1.0, "{area}");
    assert!(
        (shell.fade.top_alpha - 76.0 / 255.0).abs() < 1e-4,
        "{shell:?}"
    );
    assert!((shell.fade.depth - 33.3).abs() < 0.05, "{shell:?}");
    assert!((shell.fade.edge_y + 33.3).abs() < 0.05, "{shell:?}");
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

/// `Cell Selection` draws the back of the grid's flyer face-on, on the
/// rectangle RPCS3's settled frame of `09_blitzed` shows (authored columns 165
/// to 1756, rows 139 to 937 of the 1920 by 1080 grid). Every grid's back
/// decodes and stands there.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_grids_back_card_stands_on_the_measured_cell_selection_rectangle() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let xml = screen_xml(&mut archives);
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let widget = flyer::read(&xml, &screens)
        .into_iter()
        .find(|w| w.name == "FlyerModel")
        .expect("the grid widget");
    let names: Vec<&str> = HD_CARDS.iter().chain(&FURY_CARDS).copied().collect();
    let cards: Vec<oag_game::flyer::CardSpec> = names
        .iter()
        .map(|name| oag_game::flyer::CardSpec {
            flyer: (*name).to_string(),
            side: oag_game::flyer::Side::Back,
            window: oag_game::flyer::BACK_WINDOW,
            stretch: oag_game::flyer::BACK_STRETCH,
        })
        .collect();
    let flyers = oag_game::flyer::Flyers::load(&mut archives, vec![widget], &cards);
    assert!(flyers.report.is_empty(), "{:?}", flyers.report);
    for name in names {
        let show = oag_game::flyer::Flyers::cell_show(name);
        assert!(flyers.has(&show), "{name}'s back did not decode");
        let rect = flyers
            .card_rect(&show, oag_display::space::Space::HD)
            .unwrap_or_else(|| panic!("{name} has no rectangle"));
        for (got, want) in rect.iter().zip([165.0, 139.0, 1591.0, 798.0]) {
            assert!((got - want).abs() < 12.0, "{name}: {rect:?}");
        }
    }
}
