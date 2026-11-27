//! The 3-D flyer card behind Wipeout HD's `Grid Selection`, against the disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^hd_flyer_ground_truth::/)'
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

/// The two cards `Campaign Selection` shows, as the screen's own chooser names them: Fury's, then
/// HD's (`campaignList`'s order).
fn campaign_cards(
    xml: &str,
    screens: &oag_ui::screen::Screens,
) -> Vec<oag_game::flyer::CampaignCard> {
    let chooser = oag_ui_screens::campaign::selection::Chooser::read(xml, screens, &|name| {
        oag_ui_screens::campaign::Layout::read_authored(
            screens,
            name,
            &oag_ui::language::StringTable::default(),
            oag_ui_screens::picker::FaceScales::default(),
            [1920.0, 1080.0],
            [1920.0, 1080.0],
        )
    })
    .expect("DATA06 authors the chooser");
    chooser
        .entries()
        .iter()
        .map(|entry| oag_game::flyer::CampaignCard {
            flyer: entry.card_flyer().expect("a known campaign"),
            widget: entry.card_widget().expect("a known campaign"),
        })
        .collect()
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
            gain: oag_game::flyer::FURY_GAIN,
            keyed: false,
        })
        .collect();
    let flyers = oag_game::flyer::Flyers::load(&mut archives, vec![widget], &cards);
    assert!(flyers.report.is_empty(), "{:?}", flyers.report);
    for name in &names {
        assert!(flyers.has(&flyers.grid_show(name)), "{name} did not decode");
        let [x, y, width, height] = flyers
            .card_rect(&flyers.grid_show(name), oag_display::space::Space::HD)
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
    let campaigns = campaign_cards(&xml, &screens);
    let cards: Vec<oag_game::flyer::CardSpec> = campaigns
        .iter()
        .map(|card| oag_game::flyer::CardSpec {
            flyer: card.flyer.clone(),
            side: oag_game::flyer::Side::Front,
            window: oag_game::flyer::FURY_WINDOW,
            stretch: oag_game::flyer::CAMPAIGN_STRETCH,
            gain: oag_game::flyer::CAMPAIGN_GAIN,
            keyed: true,
        })
        .collect();
    let flyers = oag_game::flyer::Flyers::load(&mut archives, widgets, &cards)
        .with_campaigns(campaigns, 1.0);
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
    let (first, second) = (Campaign::new(0), Campaign::new(1));
    let [fury, hd] = rects(first).try_into().expect("two");
    near(fury[0], 240.0, "Fury's left edge, selected");
    near(fury[0] + fury[2], 1038.0, "Fury's right edge, selected");
    near(fury[1], 212.0, "Fury's top, selected");
    near(hd[0], 1105.0, "HD's left edge, turned");
    let [_, hd] = rects(second).try_into().expect("two");
    near(hd[0], 879.0, "HD's left edge, selected");
}

/// The unfocused `Campaign Selection` card is dimmed and the focused one is
/// not, and the dimming follows the selection (`0x00214238` sets `0xffffffff`
/// on the entry `campaignList` holds and `0xff505050` on the other).
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn campaign_selections_unfocused_card_is_dimmed() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let xml = screen_xml(&mut archives);
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let widgets = flyer::read(&xml, &screens);
    let flyers = oag_game::flyer::Flyers::load(&mut archives, widgets, &[])
        .with_campaigns(campaign_cards(&xml, &screens), 1.0);
    use oag_game::flyer::{FOCUSED_TINT, UNFOCUSED_TINT};
    use oag_ui_screens::campaign::selection::Campaign;
    for (selected, fury, hd) in [
        (Campaign::new(0), FOCUSED_TINT, UNFOCUSED_TINT),
        (Campaign::new(1), UNFOCUSED_TINT, FOCUSED_TINT),
    ] {
        let tints: Vec<_> = flyers
            .selection_shows(selected)
            .into_iter()
            .map(|show| (show.flyer, show.tint))
            .collect();
        assert_eq!(
            tints,
            [
                ("fury_campaign".to_string(), fury),
                ("hd_campaign".to_string(), hd)
            ],
            "{selected:?}"
        );
    }
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
            gain: oag_game::flyer::FURY_GAIN,
            keyed: false,
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

/// The player's view of `Cell Selection`'s card, through the real binary: it is drawn into
/// the scene the Fury backdrop's bloom chain reads, under the block a screen with models
/// loads. The flat red of `09_blitzed`'s back (texel 63) leaves the chain at the page's own
/// adapted exposure plus its frame bloom, where the original's frame shows 103 and 104 at
/// the same `Safe Area Setting` of 96; over the page at the stand-in gain it drew 126, and
/// under the backdrop's own block 108 with no glow past its edge. That glow is the same
/// chain's: the bloom of the card reaches a few pixels over the black beside it and is gone
/// a dozen pixels out.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images and a GPU adapter"]
fn the_cell_page_card_is_exposed_and_bloomed_by_the_front_ends_chain() {
    let Some(image) = image() else {
        return;
    };
    let dir = oag_testdata::scratch::dir("hd-flyer-cell-chain-ground-truth");
    for empty in ["no-dlc", "no-mods", "config/oag"] {
        std::fs::create_dir_all(dir.join(empty)).expect("a temp directory");
    }
    // The card's share of the frame sets the exposure, so the safe area is the original's.
    std::fs::write(
        dir.join("config/oag/settings.toml"),
        "[graphics]\nhud_safe_area = 96\n",
    )
    .expect("a settings file");
    let out = dir.join("cell-select.png");
    let (width, height) = (960_usize, 540_usize);
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "960x540",
            "--menu-page",
            "cell-select",
        ])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("OAG_DLC", dir.join("no-dlc"))
        .env("OAG_MODS", dir.join("no-mods"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --menu-page cell-select");
    let still = oag_texture::replacer::decode_png(&std::fs::read(&out).expect("the screenshot"))
        .expect("a PNG");
    assert_eq!(
        (still.width as usize, still.height as usize),
        (width, height)
    );
    let at = |x: usize, y: usize| {
        let i = (y * width + x) * 4;
        [still.rgba[i], still.rgba[i + 1], still.rgba[i + 2]]
    };
    // A band of the card clear of every widget: right of the wordmark, above the rule.
    let mut reds: Vec<u8> = Vec::new();
    for y in 100..125 {
        for x in 360..600 {
            let [r, g, b] = at(x, y);
            assert!(
                g < 8 && b < 8,
                "({x}, {y}) is not the card's red: {r} {g} {b}"
            );
            reds.push(r);
        }
    }
    reds.sort_unstable();
    let card = reds[reds.len() / 2];
    println!("card red {card}");
    assert!(
        (96..=104).contains(&card),
        "the card's flat red is {card}: 126 is the page draw's gain, 108 the backdrop's own block, 63 the texel"
    );
    // The card's left edge on a row clear of its notches, and the black beside it.
    let row = 320;
    let edge = (0..width / 2)
        .find(|&x| at(x, row)[0] > card / 2)
        .expect("the card's left edge");
    let near = at(edge - 2, row)[0];
    let far = at(edge - 14, row)[0];
    println!("edge at {edge}: {near} two pixels out, {far} fourteen out");
    assert!(
        (3..card / 2).contains(&near),
        "no bloom past the card's edge: {near} two pixels out"
    );
    assert!(
        far <= 2,
        "the bloom does not end: {far} fourteen pixels out"
    );
}

/// The Fury card plays on its own clock: `Flyer_Item`'s render keeps the front model's time in `[3, 6)` seconds
/// (`0x001a2510`), and ours bakes the card at `Flyers::card_seconds` (`--menu-model-seconds`), so two clocks of that loop
/// must give two cards. `09_blitzed` authors 34 `Anim Transform` nodes; the arrows' nodes sit at one place at 3.0 s and
/// another at 5.5 s, the arrow quads' translations moving by more than a tenth of a unit.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_fury_card_is_a_different_picture_at_two_clocks_of_its_loop() {
    let Some(image) = image() else { return };
    let mut archives = archives(&image);
    let (model, _) = oag_game::preview::psp2_scene::model_named(
        &mut archives,
        &flyer::front_entry("09_blitzed"),
        false,
    )
    .expect("the card decodes");
    assert_eq!(model.anim_nodes.len(), 34, "the nodes the card animates");
    let (early, late) = (model.sample_anim_nodes(3.0), model.sample_anim_nodes(5.5));
    let moved = early
        .iter()
        .zip(&late)
        .filter(|(a, b)| (a[12] - b[12]).abs() > 0.1 || (a[13] - b[13]).abs() > 0.1)
        .count();
    assert!(moved >= 10, "{moved} nodes moved between 3.0 s and 5.5 s");
    assert_eq!(
        oag_game::flyer::SETTLED_SECONDS,
        3.0,
        "the card's loop starts at 3.0 s"
    );
}

/// The grid widget and a front [`oag_game::flyer::CardSpec`] for each of `names`, as the campaign screen wires them.
fn grid_cards(
    archives: &mut oag_assets::Archives,
    names: &[&str],
) -> (oag_game::flyer::Flyers, Vec<oag_game::flyer::Show>) {
    let xml = screen_xml(archives);
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let widget = flyer::read(&xml, &screens)
        .into_iter()
        .find(|w| w.name == "FlyerModel")
        .expect("the grid widget");
    let cards: Vec<oag_game::flyer::CardSpec> = names
        .iter()
        .map(|name| oag_game::flyer::CardSpec {
            flyer: (*name).to_string(),
            side: oag_game::flyer::Side::Front,
            window: if HD_CARDS.contains(name) {
                oag_game::flyer::HD_WINDOW
            } else {
                oag_game::flyer::FURY_WINDOW
            },
            stretch: 1.0,
            gain: oag_game::flyer::FURY_GAIN,
            keyed: false,
        })
        .collect();
    let flyers = oag_game::flyer::Flyers::load(archives, vec![widget], &cards);
    assert!(flyers.report.is_empty(), "{:?}", flyers.report);
    let shows = names.iter().map(|name| flyers.grid_show(name)).collect();
    (flyers, shows)
}

/// A GPU target the size a scene layer is, and the device that made it; `None` on a machine with no adapter.
struct Frame {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: wgpu::Texture,
}

impl Frame {
    const SIZE: (u32, u32) = (1280, 720);

    fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = oag_gpu::test_adapter::request(&instance)?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).ok()?;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("flyer clock test"),
            size: wgpu::Extent3d {
                width: Self::SIZE.0,
                height: Self::SIZE.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: oag_game::render::SCENE_LAYER_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
            texture_binding_view_dimension: None,
        });
        Some(Self {
            device,
            queue,
            target,
        })
    }

    /// One frame of `shows` through the scene layer, cleared first, read back.
    fn draw(&self, flyers: &oag_game::flyer::Flyers, shows: &[oag_game::flyer::Show]) -> Vec<u8> {
        let (width, height) = Self::SIZE;
        let view = self.target.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("flyer clock test clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        let mut layer = flyers.scene_layer(shows, oag_display::space::Space::HD, 1.0);
        layer(oag_game::render::SceneLayer {
            device: &self.device,
            queue: &self.queue,
            encoder: &mut encoder,
            view: &view,
            size: Self::SIZE,
        });
        let bytes = self
            .target
            .format()
            .block_copy_size(None)
            .expect("a colour format");
        let padded = (width * bytes).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flyer clock test readback"),
            size: u64::from(padded * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            self.target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the readback finishes");
        let mapped = slice.get_mapped_range().expect("mapped");
        let mut pixels = Vec::new();
        for row in mapped.chunks(padded as usize).take(height as usize) {
            pixels.extend_from_slice(&row[..(width * bytes) as usize]);
        }
        pixels
    }
}

/// How many bytes of two frames differ.
fn differing(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

/// Every card is decoded at the load and built on the GPU the first time a frame draws it, once: thirty frames of one
/// grid build one card, a second grid one more, and going back to the first builds nothing. The live menu draws through
/// the same `scene_layer`/`draw` call every frame with `card_seconds` at its default, so this is the browse path.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images and a GPU adapter"]
fn a_card_is_built_on_its_first_draw_and_once() {
    let Some(image) = image() else { return };
    let Some(frame) = Frame::new() else { return };
    let mut archives = archives(&image);
    let names: Vec<&str> = HD_CARDS.iter().chain(&FURY_CARDS).copied().collect();
    let (flyers, shows) = grid_cards(&mut archives, &names);
    assert_eq!(
        (flyers.waiting_len(), flyers.builds()),
        (16, 0),
        "a load builds no card"
    );
    let (fury, other) = (&shows[8..9], &shows[9..10]);
    let first = frame.draw(&flyers, fury);
    for _ in 0..30 {
        let again = frame.draw(&flyers, fury);
        assert_eq!(again, first, "a settled frame redraws the same");
    }
    assert!(first.iter().any(|b| *b != 0), "the card draws something");
    assert_eq!(
        (flyers.waiting_len(), flyers.builds()),
        (15, 1),
        "thirty frames of one grid build its card once"
    );
    frame.draw(&flyers, other);
    assert_eq!((flyers.waiting_len(), flyers.builds()), (14, 2));
    frame.draw(&flyers, fury);
    frame.draw(&flyers, other);
    assert_eq!(
        (flyers.waiting_len(), flyers.builds()),
        (14, 2),
        "browsing back builds nothing"
    );
}

/// `--menu-model-seconds` reaches the build: `09_blitzed` at 4.5 s is another picture than at 3.0 s, the default is the
/// 3.0 s picture, and a clock past the loop's end draws the card the wrap lands on (6.5 s is 3.5 s, which is not 6.5 s
/// unwrapped: the nodes sample their keys at the clock modulo their own 4.0 s).
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images and a GPU adapter"]
fn the_menu_model_seconds_reach_the_card_and_wrap_at_the_loops_end() {
    let Some(image) = image() else { return };
    let Some(frame) = Frame::new() else { return };
    let mut archives = archives(&image);
    let mut at = |seconds: Option<f32>| {
        let (mut flyers, shows) = grid_cards(&mut archives, &["09_blitzed"]);
        if let Some(seconds) = seconds {
            flyers.card_seconds = seconds;
        }
        frame.draw(&flyers, &shows)
    };
    let default = at(None);
    let settled = at(Some(3.0));
    let later = at(Some(4.5));
    let wrapped = at(Some(6.5));
    let same_as_wrapped = at(Some(3.5));
    let unwrapped_would_be = at(Some(2.5));
    println!(
        "bytes differing: 3.0 vs 4.5 {}, 6.5 vs 3.5 {}, 6.5 vs 2.5 {}",
        differing(&settled, &later),
        differing(&wrapped, &same_as_wrapped),
        differing(&wrapped, &unwrapped_would_be)
    );
    assert_eq!(default, settled, "no flag draws the card at 3.0 s");
    assert!(
        differing(&settled, &later) > 1000,
        "3.0 s and 4.5 s draw the same card: the clock does not reach the build"
    );
    assert_eq!(wrapped, same_as_wrapped, "6.5 s wraps to 3.5 s");
    assert!(
        differing(&wrapped, &unwrapped_would_be) > 1000,
        "the probe cannot tell a wrap from the raw clock"
    );
}

/// `--menu-page grid-select`'s Fury card (`09_blitzed`) at `--menu-model-seconds seconds` (the flag's absence for
/// `None`), as the PNG's RGBA bytes: the flag's whole path from the command line to the card's build.
fn grid_select_still(image: &std::path::Path, seconds: Option<&str>) -> Vec<u8> {
    let dir = oag_testdata::scratch::dir("hd-flyer-clock-flag-ground-truth");
    for empty in ["no-dlc", "no-mods", "config/oag"] {
        std::fs::create_dir_all(dir.join(empty)).expect("a temp directory");
    }
    let out = dir.join(format!("grid-select-{}.png", seconds.unwrap_or("default")));
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command.arg(image).args([
        "--no-audio",
        "--no-soundtrack",
        "--no-effects",
        "--size",
        "960x540",
        "--menu-page",
        "grid-select",
    ]);
    if let Some(seconds) = seconds {
        command.args(["--menu-model-seconds", seconds]);
    }
    let status = command
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("OAG_DLC", dir.join("no-dlc"))
        .env("OAG_MODS", dir.join("no-mods"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --menu-page grid-select");
    oag_texture::replacer::decode_png(&std::fs::read(&out).expect("the screenshot"))
        .expect("a PNG")
        .rgba
}

/// The same claim as the card's own test, through `oag-game`'s command line: no flag is the 3.0 s card, `4.5` is another
/// card, `6.5` is the card `3.5` is.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images and a GPU adapter"]
fn the_menu_model_seconds_flag_moves_grid_selects_card_and_wraps() {
    let Some(image) = image() else { return };
    let default = grid_select_still(&image, None);
    let settled = grid_select_still(&image, Some("3.0"));
    let later = grid_select_still(&image, Some("4.5"));
    let wrapped = grid_select_still(&image, Some("6.5"));
    let same_as_wrapped = grid_select_still(&image, Some("3.5"));
    assert_eq!(default, settled, "no flag is the 3.0 s card");
    assert!(
        differing(&settled, &later) > 5000,
        "the flag does not change the card"
    );
    assert_eq!(wrapped, same_as_wrapped, "6.5 s is the card 3.5 s is");
}
