//! The race box's two selection screens against the disc they are read off:
//! `Track Creation` and `Team Selection` in `Selection_Definition.xml`, the
//! ratings in `Definition.xml`, and the two preview meshes each screen shows.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # What this pins that the unit tests cannot
//!
//! `oag_ui_screens::picker`'s own tests parse a hand-written miniature of the file.
//! This reads the real one through the same boot the game uses, so the two
//! parser gaps that hid the screens for a day - `<Item>`/`<LeftLayer>` not
//! being walked, and `OffsetX`/`OffsetY` not being summed - would fail here
//! rather than draw an empty panel. The numbers are the XML's own, checked
//! against `docs/ui/selection-screens.md`'s capture.

use std::path::PathBuf;

use oag_game::boot;
use oag_ui::language::StringTable;
use oag_ui_screens::picker::{Details, Entry, Kind, Picker};

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn shell(path: &std::path::Path) -> boot::Shell {
    let options = boot::Options {
        source: path.to_string_lossy().into_owned(),
        language: None,
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-selection-screens-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        // Every assertion here is about the XML and two meshes; nothing pays
        // for a transcode.
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    boot::load_shell(&options).expect("the shell loads").0
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn both_screens_read_with_their_panel_at_the_authored_offset() {
    let Some(path) = image() else {
        return;
    };
    let shell = shell(&path);
    let track = shell.track_select.as_ref().expect("Track Creation reads");
    let ship = shell.ship_select.as_ref().expect("Team Selection reads");

    // `<LeftLayer OffsetX="290" OffsetY="25">` holds `Infogradient` at
    // `y="1"`, so the panel is at (290, 26); `Team Selection`'s layer is
    // `OffsetY="45"`, so its panel is twenty lower.
    assert_eq!(track.panel, [290.0, 26.0, 170.0, 200.0]);
    assert_eq!(ship.panel, [290.0, 46.0, 170.0, 162.0]);

    // The three stat rules under `<Item name="line bg1" OffsetY="142">` and
    // its siblings: two 85-wide gradient halves each, at the layer's x.
    let rules: Vec<(f32, f32)> = track
        .screen
        .fills
        .iter()
        .filter(|fill| fill.gradient.is_some() && fill.height == Some(14.0))
        .map(|fill| (fill.x, fill.y))
        .collect();
    assert_eq!(
        rules,
        vec![
            (290.0, 167.0),
            (375.0, 167.0),
            (290.0, 182.0),
            (375.0, 182.0),
            (290.0, 197.0),
            (375.0, 197.0)
        ],
        "{rules:?}"
    );

    // The four rating bars, `<Item OffsetY="44">` and on, at x="65" y="3"
    // inside the `OffsetX="14"` group under the `OffsetY="45"` layer:
    // 290 + 14 + 65 across, 45 + 44 + 3 down.
    let bars: Vec<(String, f32, f32)> = ship
        .screen
        .images
        .iter()
        .filter(|image| image.name.as_deref().is_some_and(|n| n.ends_with(" Bar")))
        .map(|image| (image.name.clone().unwrap(), image.x, image.y))
        .collect();
    assert_eq!(
        bars,
        vec![
            ("Speed Bar".to_string(), 369.0, 92.0),
            ("Thrust Bar".to_string(), 369.0, 107.0),
            ("Handling Bar".to_string(), 369.0, 122.0),
            ("Shield Bar".to_string(), 369.0, 137.0),
            ("Loyalty Bar".to_string(), 304.0, 181.0),
        ],
        "{bars:?}"
    );
    // The livery arrows are children of the `skin` text widget.
    assert!(
        ship.screen
            .images
            .iter()
            .any(|image| image.name.as_deref() == Some("skin left arrow")),
        "{:?}",
        ship.screen.images
    );
    // Every image the two screens name is in the sheet, `hex_bg.mip` included.
    for image in track.screen.images.iter().chain(&ship.screen.images) {
        assert!(
            shell.sprites.get(&image.src).is_some(),
            "{} is not in the sprite sheet",
            image.src
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ratings_are_the_ones_the_capture_shows() {
    let Some(path) = image() else {
        return;
    };
    let shell = shell(&path);
    let rating = |id: &str| {
        shell
            .teams
            .iter()
            .find(|team| team.id == id)
            .and_then(|team| team.rating)
            .map(|r| [r.speed, r.thrust, r.handling, r.shield])
    };
    // `docs/ui/selection-screens.md`: the four bars on the 2026-09-09
    // PPSSPP capture, team by team.
    assert_eq!(rating("Assegai"), Some([8, 8, 9, 7]));
    assert_eq!(rating("Qirex"), Some([8, 7, 8, 9]));
    assert_eq!(rating("AG_Systems"), Some([7, 9, 9, 8]));
    assert_eq!(rating("Piranha"), Some([10, 6, 6, 9]));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_preview_meshes_resolve_and_are_what_the_screens_show() {
    let Some(path) = image() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    // The circuit outline: a single ribbon at track scale, not a flythrough
    // - its radius is the circuit's own, hundreds of units, on 364 vertices.
    let blob = archives
        .read_name(r"Data\Environments\16_Track\FE\forward.vex")
        .expect("the outline resolves");
    let outline = oag_mesh::mesh::build("outline", &blob).expect("it decodes");
    assert_eq!(outline.vertices.len(), 364);
    assert!(
        outline.radius > 500.0,
        "track scale, not a corridor: radius {}",
        outline.radius
    );
    // The craft: the front-end hull, a few units across.
    let blob = archives
        .read_name(r"Data\Ships\Assegai\ship_FE.vex")
        .expect("the hull resolves");
    let hull = oag_mesh::mesh::build("hull", &blob).expect("it decodes");
    assert!(hull.vertices.len() > 1000, "{}", hull.vertices.len());
    assert!(hull.radius < 20.0, "craft scale: radius {}", hull.radius);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_track_picker_draws_the_discs_own_rows() {
    let Some(path) = image() else {
        return;
    };
    let shell = shell(&path);
    let layout = shell.track_select.as_ref().expect("Track Creation reads");
    let entries: Vec<Entry> = shell
        .tracks
        .iter()
        .map(|track| Entry {
            id: track.id.clone(),
            label: shell.strings.get_or_id(&track.id).to_string(),
            details: Details::Track {
                info: ["-".into(), "-".into(), "-".into()],
                emblem: None,
                reversed: false,
            },
        })
        .collect();
    let picker = Picker::new(Kind::Track, entries, Some("16_Track"), None);
    let face = shell.menu_font.as_ref().unwrap_or(&shell.font);
    let skin = oag_ui::menu::Skin::new(shell.menu_skin, shell.space, face.line_height);
    let layers = oag_ui_screens::picker::draw_list(
        &picker,
        layout,
        &skin,
        &shell.frame,
        None,
        false,
        &|src| shell.sprites.get(src),
        &|text| oag_ui::font::measure(face, text),
    );
    let texts: Vec<String> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            // `font="default"` text is a `FacedText` in the `Default` role
            // since the picker draws it in its own face.
            oag_ui::frontend::Draw::Text { text, .. }
            | oag_ui::frontend::Draw::FacedText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    // The counter - over the disc's own circuits, no pack mounted here -
    // the three stat titles off the string table, and the circuit's own
    // name: none of them spelled in this repository.
    let counter = format!("1 / {}", shell.tracks.len());
    assert!(texts.contains(&counter), "{counter}: {texts:?}");
    for id in ["IG_HUD_DISTANCE", "IG_HUD_LAP_REC", "ER_RR"] {
        let title = shell
            .strings
            .get(id)
            .expect("the string table names the row");
        assert!(texts.iter().any(|t| t == title), "{title:?} in {texts:?}");
    }
    let sprites = layers
        .body
        .iter()
        .filter(|draw| matches!(draw, oag_ui::frontend::Draw::Sprite { .. }))
        .count();
    assert_eq!(sprites, 2, "up arrow, down arrow");
    // The hex grid is the 32x16 tile repeated, not a patch of the sheet.
    let tiled: Vec<[f32; 2]> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::TiledSprite { repeat, .. } => Some(*repeat),
            _ => None,
        })
        .collect();
    assert_eq!(tiled, vec![[340.0 / 32.0, 120.0 / 16.0]], "{tiled:?}");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_circuits_own_screen_xml_is_the_slideshow_behind_the_window() {
    let Some(path) = image() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let mut report = Vec::new();
    let (show, blobs) = oag_game::preview::slideshow(
        &mut archives,
        r"Data\Environments\16_Track",
        false,
        &[],
        &StringTable::default(),
        &mut report,
    )
    .expect("16_Track authors a screen.xml with an Info chain");
    assert!(report.is_empty(), "{report:?}");
    // Info -> Info2 -> Info3 -> Info4 -> Info3 Close -> Info2 Close, two
    // seconds apiece, one card deeper per step and back - see
    // `docs/ui/selection-screens.md`.
    let names: Vec<&str> = show.states().iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Top",
            "Info",
            "Info2",
            "Info2 Close",
            "Info3",
            "Info3 Close",
            "Info4"
        ]
    );
    assert_eq!(show.at(0.0).images.len(), 1);
    assert_eq!(show.at(7.0).name, "Info4");
    assert_eq!(show.at(7.0).images.len(), 7, "four cards and three shadows");
    assert_eq!(show.at(12.5).name, "Info");
    // Four cards and the shadow, every one on the disc and 256x128 / 128x64.
    assert_eq!(blobs.len(), 5, "{:?}", show.sources());
    let sheet = oag_hud::sprite::Sheet::build(&blobs, &mut report);
    let card = sheet
        .get(r"Data\Environments\16_Track\FE\image_01.mip")
        .expect("the first card decodes");
    assert_eq!((card.width, card.height), (256, 128));
    let shadow = sheet
        .get(r"Data\FE\Images\track_sel_shadow.mip")
        .expect("the shadow decodes");
    assert_eq!((shadow.width, shadow.height), (128, 64));
    // And the same file places the outline: the disc's own pose, carried.
    let model = show.model.as_ref().expect("the Mode3D model reads");
    assert_eq!(model.src, r"Data\Environments\16_Track\FE\forward.vex");
    assert_eq!(model.position, [0.0, -300.0, -3700.0]);
    // Zone mode has its own file and its own chain.
    let (zone, _) = oag_game::preview::slideshow(
        &mut archives,
        r"Data\Environments\16_Track",
        true,
        &[],
        &StringTable::default(),
        &mut report,
    )
    .expect("16_Track authors a screen_zone.xml");
    assert_eq!(zone.at(0.0).name, "Zone");
    assert!(
        zone.sources()[0].ends_with(r"\FE\zone_01.mip"),
        "{:?}",
        zone.sources()
    );
}

/// The PS2 pressing: the same screens on its own grid, `Team Selection`'s
/// widgets read under the PSP's names, both previews textured off their
/// sibling sets, and a five-card slideshow with the Zone chain in the same
/// file. See `docs/ui/selection-screens.md`'s PS2 section.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ps2_pressing_reads_the_same_screens_on_its_own_grid() {
    let Some(path) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let shell = shell(&path);
    let track = shell.track_select.as_ref().expect("Track Creation reads");
    let ship = shell.ship_select.as_ref().expect("Team Selection reads");
    // `<Item OffsetX="387" OffsetY="41">` on `Track Creation`, `OffsetY="74"`
    // on `Team Selection` - the PSP's 290/25 and 290/45 scaled.
    assert_eq!(track.panel, [387.0, 43.0, 227.0, 329.0]);
    assert_eq!(ship.panel, [387.0, 76.0, 227.0, 267.0]);
    assert!(
        (track.scale[0] - 4.0 / 3.0).abs() < 1e-5,
        "{:?}",
        track.scale
    );
    // Neither pressing authors a `<Menu>` on a selection screen - Pulse shows
    // the selected entry alone in named `<Text>` widgets, where Pure lists
    // every entry. So `picker`'s entry list draws nothing on either, which is
    // what keeps this shared machinery from putting a second list over
    // Pulse's single-entry display.
    assert!(track.screen.menu.is_none());
    assert!(ship.screen.menu.is_none());
    // `honey0`, `Speed Bar0`, `Infogradient0` on the disc; bare here.
    assert!(
        ship.screen
            .texts
            .iter()
            .any(|t| t.name.as_deref() == Some("honey"))
    );
    assert!(
        ship.screen
            .images
            .iter()
            .any(|i| i.name.as_deref() == Some("Speed Bar"))
    );
    assert!(
        !ship
            .screen
            .texts
            .iter()
            .any(|t| t.name.as_deref().is_some_and(|n| n.ends_with('0'))),
        "{:?}",
        ship.screen
            .texts
            .iter()
            .filter_map(|t| t.name.clone())
            .collect::<Vec<_>>()
    );

    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    for entry in [
        r"Data\Environments\16_Track\FE\forward.vex",
        r"Data\Ships\Assegai\ship_FE.vex",
    ] {
        let model = oag_game::preview::model(&mut archives, entry).expect(entry);
        assert!(!model.textures.is_empty(), "{entry} names textures");
        assert!(
            model.textures.iter().any(Option::is_some),
            "{entry}: no texture resolved off the sibling set - it would draw white"
        );
    }
    let mut report = Vec::new();
    let (show, blobs) = oag_game::preview::slideshow(
        &mut archives,
        r"Data\Environments\16_Track",
        false,
        &[],
        &StringTable::default(),
        &mut report,
    )
    .expect("the PS2 16_Track authors a screen.xml");
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(show.at(9.0).name, "Info5", "five cards on the PS2");
    assert_eq!(blobs.len(), 6, "{:?}", show.sources());
    let (zone, _) = oag_game::preview::slideshow(
        &mut archives,
        r"Data\Environments\16_Track",
        true,
        &[],
        &StringTable::default(),
        &mut report,
    )
    .expect("the Zone chain is in the same file");
    assert_eq!(zone.at(0.0).name, "Zone");
}

/// Pure: same definition file name as Pulse's, different screen name for the
/// track picker (`Track Selection`, not `Track Creation`), and **no preview
/// mesh at all** - both screens preview with the stills the selected entry's
/// own `screen.xml` authors, out of `FEData.wad`. See
/// `docs/formats/race-setup.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_reads_its_own_screen_names_and_previews_with_stills() {
    let Some(path) = oag_testdata::image("data/images/pure-psp-eu.chd") else {
        return;
    };
    let shell = shell(&path);
    let track = shell.track_select.as_ref().expect("Track Selection reads");
    let ship = shell.ship_select.as_ref().expect("Team Selection reads");
    assert_eq!(track.title, "Track Selection");
    assert_eq!(ship.title, "Team Selection");
    // And Pure's own shape: one `<Menu>` per screen and no named `<Text>`, so
    // it lists every entry where Pulse shows the selected one alone.
    for (what, layout) in [("track", track), ("ship", ship)] {
        let menu = layout
            .screen
            .menu
            .as_ref()
            .unwrap_or_else(|| panic!("{what} authors a Menu widget"));
        assert_eq!((menu.x, menu.y), (21.0, 45.0), "{what}");
        // `FEGlobals->MenuScale` and `FEGlobals->TextColor`, the latter out
        // of the style skin.
        assert!((menu.scale - 1.15).abs() < 1e-5, "{what}: {}", menu.scale);
        assert_eq!(menu.color, 0xFF11_ACD0, "{what}");
    }
    // The title says it previews with stills, and that is what stops the
    // pickers reaching for a `.vex` that is not on this disc.
    assert!(!oag_pure::FRONT_END.preview_meshes);
    assert!(oag_pulse::FRONT_END.preview_meshes);

    // The style skin's globals, which is where every colour Pure's selection
    // screens draw in lives - not in the front-end root the way Pulse's do.
    // Without them the stills tint white on a white front end and the
    // picture is not wrong but absent.
    let globals: Vec<(&str, &str)> = shell
        .screens
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    for (key, value) in [
        ("ShipColor", "0xFF99D9E8"),
        ("TrackColor", "0xFF99D9E8"),
        ("TextColor", "0xFF11ACD0"),
    ] {
        assert_eq!(
            shell.screens.globals.get(key).map(String::as_str),
            Some(value),
            "{key} comes from Data\\Skins\\Default\\Skin.xml"
        );
    }

    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pure::TITLE)
        .expect("the archives open");
    // Three stills a piece, two seconds each, cycling - `Info` (a 3/4 view),
    // `Side`, `Top`, then round again. That cycle is what a 2026-09-10 PPSSPP
    // capture read as a rotating 3D model.
    for (location, srcs) in [
        (
            r"Data\Ships\Feisar",
            [
                r"Data\Ships\Feisar\Images\feisar3D.mip",
                r"Data\Ships\Feisar\Images\feisarside.mip",
                r"Data\Ships\Feisar\Images\feisartop.mip",
            ],
        ),
        (
            r"Data\Environments\01_Vineta_K",
            [
                r"Data\Environments\01_Vineta_K\Images\Track_1.mip",
                r"Data\Environments\01_Vineta_K\Images\Track_2.mip",
                r"Data\Environments\01_Vineta_K\Images\Track_3.mip",
            ],
        ),
    ] {
        let mut report = Vec::new();
        let (show, blobs) = oag_game::preview::slideshow(
            &mut archives,
            location,
            false,
            &globals,
            &StringTable::default(),
            &mut report,
        )
        .unwrap_or_else(|e| panic!("{location} authors a screen.xml chain: {e:#}"));
        assert!(report.is_empty(), "{report:?}");
        let names: Vec<&str> = show.states().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Info", "Side", "Top"], "{location}");
        assert_eq!(show.sources(), srcs, "{location}");
        assert_eq!(show.at(0.0).name, "Info");
        assert_eq!(show.at(2.5).name, "Side");
        assert_eq!(show.at(4.5).name, "Top");
        assert_eq!(show.at(6.5).name, "Info", "the chain closes on itself");
        // One still at a time, unlike Pulse's stack of cards.
        assert_eq!(show.at(0.0).images.len(), 1);
        // Tinted out of the style skin, not left white.
        assert_eq!(show.at(0.0).images[0].color, 0xFF99_D9E8);

        assert_eq!(blobs.len(), 3);
        let sheet = oag_hud::sprite::Sheet::build(&blobs, &mut report);
        for src in srcs {
            let placed = sheet.get(src).unwrap_or_else(|| panic!("{src} decodes"));
            assert_eq!((placed.width, placed.height), (256, 128), "{src}");
        }
    }

    // Zone circuits name the chain `screen_z.xml` and still start it at
    // `Info`, where Pulse's Zone chain is a `Zone` state in `screen_zone.xml`.
    let mut report = Vec::new();
    let (zone, blobs) = oag_game::preview::slideshow(
        &mut archives,
        r"Data\Zone\01_Zone",
        true,
        &globals,
        &StringTable::default(),
        &mut report,
    )
    .expect("01_Zone authors a screen_z.xml chain");
    assert_eq!(zone.at(0.0).name, "Info");
    assert_eq!(blobs.len(), 3);

    // The two files the previews were wrongly wired to until 2026-09-10 do
    // exist - that is why the mistake was plausible - but they are the
    // in-race hull and the full racing circuit, neither of which either
    // screen draws, and neither is what the `screen.xml` chains name.
    assert!(
        archives
            .read_name(r"Data\Environments\01_Vineta_K\track.vex")
            .is_ok(),
        "the racing circuit is on disc, it is simply not the picker's preview"
    );
    assert!(
        archives.read_name(r"Data\Ships\Feisar\Ship.vex").is_ok(),
        "the in-race hull is on disc, it is simply not the picker's preview"
    );
    // And Pulse's own two names are on neither of Pure's entries, which is
    // why a fallthrough was the wrong shape rather than a harmless one.
    assert!(
        archives
            .read_name(r"Data\Ships\Feisar\ship_FE.vex")
            .is_err()
    );
    assert!(
        archives
            .read_name(r"Data\Environments\01_Vineta_K\FE\forward.vex")
            .is_err()
    );
}

/// Wipeout HD/Fury: `Team Selection` reads off `DATA06`'s own
/// `Team_Selection_Definition.xml` (`oag_ui_screens::picker::hd`), with the parent
/// `Team Selection Top Level`'s widgets under the child's name, every
/// heading resolved - `RC_NAV_TEAM` only through `DATA06`'s own
/// `entries.xml` - and each team's per-model ratings in tenths. HD's track
/// screen is read too, see the next test. See `docs/ui/campaign-screens.md`'s
/// "Wipeout HD/Fury: `Team Selection`" section.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_reads_its_own_team_selection() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let shell = shell(&path);
    let ship = shell.ship_select.as_ref().expect("Team Selection reads");
    assert_eq!(ship.title, "SHIP SELECT");
    let extra = ship.hd.as_deref().expect("HD's own layout");
    let labels: Vec<(&str, f32, f32)> = extra
        .labels
        .iter()
        .map(|label| (label.text.as_str(), label.x, label.y))
        .collect();
    assert_eq!(
        labels,
        vec![
            ("CHOOSE TEAM", 160.0, 140.0),
            ("NAVIGATE TEAM", 160.0, 336.0),
            ("SHIP MODEL", 705.0, 140.0),
            ("STATISTICS", 705.0, 833.0),
        ]
    );
    assert_eq!(ship.panel, [160.0, 170.0, 506.0, 156.0]);
    assert_eq!(ship.preview, [705.0, 170.0, 1052.0, 650.0]);
    assert_eq!(extra.logo, Some([160.0, 185.0, 512.0, 128.0]));
    let model = extra.ship_model.expect("ShipModel");
    assert_eq!(model.origin, [1220.0, 412.0]);
    for name in ["Slide_0", "Slide_1", "Slide_2", "Slide_3", "Slide_4"] {
        assert!(
            ship.screen
                .blocks
                .iter()
                .any(|block| block.name.as_deref() == Some(name)),
            "{name}"
        );
    }

    // Feisar's own `<FE>`s: `normal` for the classic hull, `concept1` for
    // `_c1` - the two numbers two RPCS3 frames of this screen print.
    let feisar = shell
        .teams
        .iter()
        .find(|team| team.id == "Feisar")
        .expect("Feisar");
    let stats = feisar.variant_stats(["", "_c1", "_n1"]);
    let values: Vec<Option<[u8; 4]>> = stats.iter().map(|s| s.map(|s| s.values())).collect();
    assert_eq!(
        values,
        vec![
            Some([70, 80, 100, 80]),
            Some([80, 85, 100, 80]),
            Some([80, 85, 100, 80]),
        ]
    );
    // And every team's logo is on the sheet.
    for team in &shell.teams {
        let src = oag_ui_screens::picker::hd::logo_src(&team.id);
        assert!(shell.sprites.get(&src).is_some(), "{src}");
    }
}

/// Wipeout HD/Fury: `Track Creation` reads off `DATA06`'s own
/// `Track_Selection_Definition.xml` (`oag_ui_screens::picker::hd::track`), merged
/// under the child's name with none of `Tournament C`'s widgets, every
/// heading and the RECORDS row labels resolved, and every circuit's own
/// `TrackSelectEmblem_Fury.gtf` on the sheet. See `docs/ui/campaign-screens.md`'s
/// "Wipeout HD/Fury: `Track Creation`" section.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_reads_its_own_track_creation() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let shell = shell(&path);
    let track = shell.track_select.as_ref().expect("Track Creation reads");
    assert_eq!(track.title, "TRACK SELECT");
    assert!(track.hd.is_none() && track.is_hd());
    let extra = track.hd_track.as_deref().expect("HD's own track layout");
    let labels: Vec<(&str, f32, f32)> = extra
        .common
        .labels
        .iter()
        .map(|label| (label.text.as_str(), label.x, label.y))
        .collect();
    assert_eq!(
        labels,
        vec![
            ("CHOOSE CIRCUIT", 160.0, 140.0),
            ("CIRCUIT MODEL", 857.0, 140.0),
            ("CIRCUIT DIRECTION", 160.0, 422.0),
            ("RACE INFORMATION", 160.0, 690.0),
        ]
    );
    assert_eq!(track.panel, [160.0, 170.0, 680.0, 242.0]);
    assert_eq!(track.preview, [856.0, 170.0, 900.0, 565.0]);
    assert_eq!(extra.emblem, Some([270.0, 178.0, 192.0, 192.0]));
    assert_eq!(extra.model.map(|model| model.origin), Some([1308.0, 440.0]));
    let grid = extra.hex_grid.expect("TrackHexSelection");
    assert_eq!((grid.columns, grid.rows), (9, 2));
    assert_eq!(extra.row_labels, ["PERSONAL", "FRIENDS", "GLOBAL"]);
    assert!(
        track.screen.texts.iter().all(|text| !text
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("TrackNumber"))),
        "Tournament C's widgets are not merged in"
    );
    // The screen's own path is live: `racebox_definition.xml` only names it.
    assert!(shell.tracks.iter().all(|track| {
        let src = oag_ui_screens::picker::hd::track::emblem_src(&track.location);
        shell.sprites.get(&src).is_some()
    }));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_kills_row_offers_the_discs_own_eliminations_list() {
    let Some(path) = image() else {
        return;
    };
    // `RaceBox_Definition.xml`'s `Eliminations` list, in authored order.
    assert_eq!(
        shell(&path).race_setup.kill_targets,
        ["5", "10", "15", "20", "25"]
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_weapons_row_offers_the_discs_own_two_states() {
    let Some(path) = image() else {
        return;
    };
    // `RaceBox_Definition.xml`'s `Weapons` list: the string id and the `value`
    // `Race_ReadSetupOptions` compares against `"On"`.
    assert_eq!(
        shell(&path).race_setup.weapons,
        [
            ("FE_ON".to_string(), "On".to_string()),
            ("FE_OFF".to_string(), "Off".to_string())
        ]
    );
}
