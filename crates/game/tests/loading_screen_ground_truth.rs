//! What each title actually puts on screen while it loads, read off its own
//! disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^loading_screen_ground_truth::/)'
//! ```
//!
//! The PS3 image has to be layer-1 decrypted first - `scripts/ps3iso.py
//! decrypt`, `docs/formats/ps3-disc.md`.
//!
//! # What is being established
//!
//! Three titles give three different answers, which is why
//! [`oag_title::Loading`] is an axis at all, and each answer is asserted
//! against the disc rather than against the table that states it:
//!
//! 1. **Pulse** ships the tips plugin and the glow strip, and no full-screen
//!    still.
//! 2. **Pure** ships none of it, and the loader says so instead of failing.
//! 3. **Wipeout HD** ships five illustrated features in two stylings and a
//!    caption, and **not** the tips plugin or the strip - which is the finding
//!    rather than a gap: its executable still carries `PI_LoadingScreen` and
//!    asks for `Data\Plugins\loading`, and the running game answers
//!    `FileSystem::Open FAILED` for it on every boot. See
//!    `docs/formats/hd-loading.md`.
//!
//! The third is the one worth pinning hardest, because it is the one a future
//! change could silently "fix" by pointing HD at Pulse's entry names and
//! getting a wave over a screen the disc never authored.

use std::path::{Path, PathBuf};

use oag_game::loading::Assets;
use oag_ui::language::StringTable;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// The chosen language's table **and** the entry it came from, which is what
/// `Assets::load` needs to reach the other copies of it.
fn table_and_entries(source: &Path) -> (StringTable, Option<String>) {
    let opened =
        oag_source::title::open_source(&source.display().to_string(), Vec::new(), Vec::new())
            .expect("the source opens");
    let mut archives = opened.archives;
    let mut report = Vec::new();
    let plugins: &[&str] = opened.title.front_end.map_or(&[], |fe| fe.language_plugins);
    let languages =
        oag_ui::language::load::load_languages(&mut archives, plugins, None, &mut report);
    let chosen = oag_ui::language::load::chosen_language(&languages, Some("English"));
    let entries = chosen.and_then(|language| language.entries.clone());
    (
        oag_ui::language::load::load_strings(
            &mut archives,
            &languages,
            Some("English"),
            &mut report,
        ),
        entries,
    )
}

/// Every feature illustration HD names is on the disc, in both stylings, and
/// decodes.
///
/// The list is `oag_hd::loading::FEATURES`, which is the order the executable
/// names the ten images in. All ten are asserted rather than the one this build
/// draws, because the styling axis is only real if both halves are shipped -
/// and it is the axis `settings.display.front_end_style` offers.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_feature_illustration_is_on_the_disc_in_both_stylings() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("the PS3 source opens");

    let styles = oag_hd::loading::FEATURES;
    assert_eq!(styles.len(), 2, "HD and Fury");
    assert_eq!(styles[0].name, "HD");
    assert_eq!(styles[1].name, "FURY");
    for style in styles {
        assert_eq!(style.features.len(), 5, "{} has five", style.name);
        for feature in style.features {
            let blob = archives
                .read_name(feature.image)
                .unwrap_or_else(|e| panic!("{} {}: {e}", style.name, feature.image));
            let gtf = oag_texture::gtf::Gtf::parse(&blob)
                .unwrap_or_else(|e| panic!("{} {}: {e}", style.name, feature.image));
            let texture = gtf.only().expect("one texture per file");
            assert!(
                texture.width >= 256 && texture.height >= 256,
                "{} {} is {}x{}, too small to be an illustration",
                style.name,
                feature.image,
                texture.width,
                texture.height
            );
        }
    }

    // The two stylings are the same five features in the same order, differing
    // only by the `_fury` suffix - which is what makes swapping them a styling
    // rather than a different screen.
    for (hd, fury) in styles[0].features.iter().zip(styles[1].features) {
        assert_eq!(hd.title, fury.title);
        assert_eq!(hd.description, fury.description);
        assert_eq!(
            fury.image,
            hd.image.replace(".gtf", "_fury.gtf"),
            "the Fury art is the same name with a suffix"
        );
    }
}

/// HD's loading screen is a caption, an illustrated feature and no wave.
///
/// The absences are the finding, so they are asserted directly: a future change
/// that pointed HD at Pulse's entry names would draw a procedural wave over a
/// stand-in glow strip on a title whose own screen is right there in its
/// executable, and every other test here would still pass.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_draws_a_feature_and_a_caption_and_no_wave() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let source = image.display().to_string();
    let (table, entries) = table_and_entries(&image);
    let assets = Assets::load(&source, &table, entries.as_deref(), None);
    for note in &assets.notes {
        println!("{note}");
    }

    assert!(!assets.wave, "HD ships no glow strip and no tips plugin");
    assert!(assets.tips.is_empty(), "{:?}", assets.tips);
    assert_eq!(
        assets.caption.as_deref(),
        Some("LOADING..."),
        "the caption is `FE_LOADINGDOT` out of the disc's own English table"
    );

    // The default styling is the one the menus are drawn in (see
    // `an_unpicked_style_is_the_one_the_menus_are_drawn_in`), and the feature is the one
    // both observations of the running game showed.
    let drawn = oag_hd::loading::FEATURES
        .iter()
        .find(|style| Some(style.name) == assets.style)
        .expect("the style the assets hold is one HD ships");
    // **All five**, because the original draws one per screen and this is read
    // once per disc.
    assert_eq!(assets.features.len(), 5, "HD ships five features");
    let art = assets.art.as_ref().expect("HD ships art");
    assert_eq!(
        art.illustrations.len(),
        5,
        "and five illustrations for them"
    );
    for entry in drawn.features.iter().map(|f| f.image) {
        assert!(
            art.sheet.get(entry).is_some(),
            "{entry} is in the one sheet the screen draws from"
        );
    }
    // The layout is the fallback layer's own file, found through the archives, and every
    // role it names resolves against the real art: nothing in it is skipped.
    let layouts = assets
        .layout
        .as_ref()
        .expect("the fallback's LoadingScreen_HD.toml loads");
    let layout = layouts
        .for_race("single_race", 16.0 / 9.0)
        .expect("it answers a single race");
    assert_eq!(layout.grid, [1920.0, 1080.0]);
    for element in &layout.elements {
        if let Some(role) = element.asset.as_deref() {
            let resolved = match role {
                "rule" => art.rule.is_some(),
                "corner" => art.corner.is_some(),
                "dot" => art.dot.is_some(),
                "square" => art.square.is_some(),
                "title-arrow" => art.title_arrow.is_some(),
                "subtitle-arrow" => art.subtitle_arrow.is_some(),
                "@feature" => true,
                other => panic!(
                    "{}: an asset role this build does not know: {other}",
                    element.id
                ),
            };
            assert!(
                resolved,
                "{}: the art role {role} has no texture on the disc",
                element.id
            );
        }
        if let Some(id) = element.string_id.as_deref() {
            assert!(
                assets.layout_strings.iter().any(|(known, _)| known == id),
                "{}: {id} is not in the disc's English table",
                element.id
            );
        }
    }
    // The marks the original frames the screen with are in the same sheet.
    assert!(art.title_arrow.is_some());
    assert!(art.subtitle_arrow.is_some());
    assert!(art.rule.is_some());
    assert!(art.corner.is_some());
    assert!(art.dot.is_some());
    assert!(art.square.is_some(), "the label bullet is in the sheet too");
    // The dot is the 8 by 8 tile the original's 166 by 30 grid is made of.
    let dot = art.dot.expect("checked above");
    assert_eq!((dot[2], dot[3]), (8.0, 8.0), "dot.gtf is an 8x8 tile");

    // The four labels are the game's own string ids.
    let labels = assets.labels.as_ref().expect("HD draws its labels");
    assert_eq!(labels.feature_image.as_deref(), Some("FEATURE IMAGE"));
    assert_eq!(
        labels.feature_description.as_deref(),
        Some("FEATURE DESCRIPTION")
    );
    assert_eq!(labels.progression_bar.as_deref(), Some("PROGRESSION BAR"));
    assert_eq!(labels.mode_icon.as_deref(), Some("MODE ICON"));

    // And the screen draws them with the bar as a grid of those dots: 166
    // columns in all, the lit ones being the load's fraction of them.
    let screen = oag_game::loading::Screen::for_mode(&assets, 33.0, 0, None, Some(3));
    let list = screen.draw_list(
        oag_game::loading::Phase::Prefetch,
        &oag_game::prefetch::Progress {
            total: 2,
            done: 1,
            ..oag_game::prefetch::Progress::default()
        },
        &oag_ui::font::Atlas::build(),
    );
    let columns: f32 = list
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::TiledSprite { repeat, .. } => Some(repeat[0]),
            _ => None,
        })
        .sum();
    assert_eq!(columns, 166.0, "the bar is 166 dot columns");
    let lit = list.iter().find_map(|draw| match draw {
        oag_ui::frontend::Draw::TiledSprite { repeat, color, .. }
            if color[3] >= 0.99 && (color[0] - 172.0 / 255.0).abs() < 0.01 =>
        {
            Some(repeat[0])
        }
        _ => None,
    });
    assert_eq!(
        lit,
        Some(83.0),
        "HD_Blue dots, half of 166, lit from the left"
    );

    let pilot = assets
        .features
        .iter()
        .find(|feature| feature.title.as_deref() == Some("PILOT ASSIST"))
        .expect("Pilot Assist is one of them");
    assert!(
        pilot
            .description
            .starts_with("Pilot Assist can aid your navigation"),
        "{:?}",
        pilot.description
    );

    // **Two of the five descriptions are in one copy of the string table
    // only**, and it is the same copy that carries all 28 circuit names - the
    // finding `oag_ui::language::CircuitNames` records, arriving a second
    // time from a different direction. Pinned both ways: the served table has
    // three of the five, and the loader still offers five.
    let missing: Vec<&str> = ["FE_ABSORB_INST", "FE_FLIP_INST"]
        .into_iter()
        .filter(|id| table.get(id).is_none())
        .collect();
    assert_eq!(
        missing,
        ["FE_ABSORB_INST", "FE_FLIP_INST"],
        "the served copy is missing the two Fury mechanics' prose"
    );
    assert!(
        table.get("FE_PA_INST").is_some(),
        "and carries the three that are not Fury's"
    );

    // The four colours are the source's own, and `HD_BG` is the served
    // archive's rather than a value spelled in this repository.
    let palette = assets.palette.expect("HD names four globals");
    assert_eq!(palette.background[3], 1.0, "the ground is opaque");
    assert!(
        palette.dim[3] < 1.0,
        "the one translucent colour is translucent: {:?}",
        palette.dim
    );

    // Asking for Fury's styling gets Fury's art and the same words.
    let fury = Assets::load(&source, &table, entries.as_deref(), Some("FURY"));
    let fury_art = fury.art.as_ref().expect("Fury ships art");
    assert!(
        fury_art
            .sheet
            .get(r"Data\FE\Images\Pilot_Assist_fury.gtf")
            .is_some(),
        "asking for Fury's styling gets Fury's art"
    );
    assert_eq!(fury.features.len(), assets.features.len());
    assert_eq!(fury.style, Some(oag_hd::loading::STYLE_FURY));

    // And the two entries a wave would need really are absent, so this reads as
    // "cut" rather than "not looked for". The running game agrees: its own
    // `TTY.log` carries `FileSystem::Open FAILED` for the first of them.
    let archives = oag_hd::open(&source).expect("the PS3 source opens");
    for entry in [
        oag_pulse::loading::TIPS_ENTRY,
        oag_pulse::loading::GLOW_STRIP_ENTRY,
    ] {
        assert!(
            archives.locate(entry).is_none(),
            "{entry} is on an HD archive after all; the cut finding needs re-reading"
        );
    }
}

/// An unpicked style is the style the menus are drawn in, and the whole screen follows it:
/// the pictures, the mode icon's clip and the four globals.
///
/// The menus never read `display.front_end_style`: they are in the style of the archive the
/// source serves (`oag_game::boot`'s block art). The loading deck used to take HD's first
/// entry instead, so a player who never touched OPTIONS saw HD's white art on Fury's black
/// page. Taking the first entry again fails the first assertion.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_unpicked_style_is_the_one_the_menus_are_drawn_in() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let source = image.display().to_string();
    let menus_are_fury = super::hd_menu_ground_truth::shell(&image)
        .frame
        .blocks
        .expect("the nine-patch decoded, so the frame carries block art")
        .fury;
    let menus = oag_hd::loading::FEATURES
        .iter()
        .find(|style| style.dark_ground == menus_are_fury)
        .expect("HD ships a style for the menus' ground");
    let (table, entries) = table_and_entries(&image);

    let unpicked = Assets::load(&source, &table, entries.as_deref(), None);
    assert_eq!(
        unpicked.style,
        Some(menus.name),
        "the deck and the menus agree"
    );
    assert_eq!(
        menus.name,
        oag_hd::loading::STYLE_FURY,
        "DATA00 serves Fury"
    );

    let art = unpicked.art.as_ref().expect("HD ships art");
    for feature in menus.features {
        assert!(
            art.sheet.get(feature.image).is_some(),
            "{} is in the sheet",
            feature.image
        );
    }
    let icons: Vec<&str> = unpicked
        .mode_icons
        .iter()
        .map(|i| i.entry.as_str())
        .collect();
    let expected: Vec<&str> = menus.mode_icons.iter().map(|(_, entry)| *entry).collect();
    assert_eq!(
        icons, expected,
        "the mode icon's clips are the menus' style's"
    );
    let palette = unpicked.palette.expect("HD names four globals");
    assert!(
        palette.background[..3].iter().sum::<f32>() < 1.5,
        "the globals are the served black page's: {:?}",
        palette.background
    );

    // The same as asking for it by name, and a different pick is honoured.
    let named = Assets::load(&source, &table, entries.as_deref(), Some(menus.name));
    assert_eq!(named.style, unpicked.style);
    assert_eq!(
        named.features.len(),
        unpicked.features.len(),
        "the same prose behind either"
    );
    let other = oag_hd::loading::FEATURES
        .iter()
        .find(|style| style.name != menus.name)
        .expect("HD ships two styles");
    let picked = Assets::load(&source, &table, entries.as_deref(), Some(other.name));
    assert_eq!(picked.style, Some(other.name));

    // And the pictures of either style land in their panel on every screen shape.
    features_lie_inside_their_panel(&unpicked, menus.name);
    features_lie_inside_their_panel(&picked, other.name);
}

/// Every feature picture of `assets`, on a 16:10, 16:9, 21:9 and 20:9 screen: what shows of it
/// (its texels that differ from the screen's ground) is inside the feature panel, 160 to 950 by
/// 155 to 655 in HD's 1920x1080 grid. A picture cut at the panel's edge passes; so does Fury's,
/// whose quad overhangs in the screen's own black.
fn features_lie_inside_their_panel(assets: &Assets, style: &str) {
    const PANEL: [f32; 4] = [160.0, 155.0, 950.0, 655.0];
    let art = assets.art.as_ref().expect("HD ships art");
    let ground = assets.palette.expect("HD names four globals").background;
    let differs = |texel: [u8; 4]| {
        texel[3] != 0
            && (0..3)
                .any(|channel| (f32::from(texel[channel]) - ground[channel] * 255.0).abs() > 16.0)
    };
    let mut shown = 0;
    for draw in 0..40 {
        for aspect in [1280.0 / 800.0, 16.0 / 9.0, 2560.0 / 1080.0, 20.0 / 9.0] {
            let mut screen = oag_game::loading::Screen::for_mode(assets, 33.0, draw, None, Some(8));
            screen.set_surface_aspect(aspect);
            let list = screen.draw_list(
                oag_game::loading::Phase::Prefetch,
                &oag_game::prefetch::Progress::default(),
                &oag_ui::font::Atlas::build(),
            );
            for item in &list {
                let oag_ui::frontend::Draw::Sprite { rect, uv, .. } = item else {
                    continue;
                };
                if rect[2] < 200.0 || rect[3] < 200.0 {
                    continue;
                }
                let ink = art
                    .sheet
                    .ink_within(*uv, differs)
                    .expect("a feature picture shows something");
                let (kx, ky) = (rect[2] / uv[2], rect[3] / uv[3]);
                let seen = [
                    rect[0] + (ink[0] - uv[0]) * kx,
                    rect[1] + (ink[1] - uv[1]) * ky,
                    rect[0] + (ink[0] + ink[2] - uv[0]) * kx,
                    rect[1] + (ink[1] + ink[3] - uv[1]) * ky,
                ];
                assert!(
                    seen[0] >= PANEL[0] - 0.5
                        && seen[1] >= PANEL[1] - 0.5
                        && seen[2] <= PANEL[2] + 0.5
                        && seen[3] <= PANEL[3] + 0.5,
                    "{style}, aspect {aspect:.2}, draw {draw}: shows at {seen:?}, panel {PANEL:?}"
                );
                shown += 1;
            }
        }
    }
    assert!(
        shown >= 160,
        "{style}: a picture on every screen, saw {shown}"
    );
}

/// Pulse still gets exactly what it always got: tips, a real strip, no still.
///
/// The regression guard for the title axis itself. Routing the loader through
/// `oag_title::Loading` was meant to leave the one title that had a loading
/// screen exactly where it was.
/// The mode icon's clip for a single race is on the disc in both stylings,
/// and is a square Bink file the movie cache can make frames of.
/// Dropping `FeatureStyle::mode_icons` or the read in `Assets::load` leaves
/// `mode_icons` empty and fails here.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_single_race_mode_icon_clip_is_on_the_disc_in_both_stylings() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let source = image.display().to_string();
    let (table, entries) = table_and_entries(&image);
    for style in ["HD", "FURY"] {
        let assets = Assets::load(&source, &table, entries.as_deref(), Some(style));
        let clip = oag_game::loading::icon::clip(&assets, Some(3))
            .unwrap_or_else(|| panic!("{style}: no clip for mode 3: {:?}", assets.notes));
        let format = oag_game::loading::icon::format(clip).expect("a Bink header");
        // The two stylings' clips are different files: the Fury one is 400
        // square, the base one 250.
        let side = if style == "FURY" { 400 } else { 250 };
        assert_eq!((format.width, format.height), (side, side), "{style}");
        let cache = std::env::temp_dir().join("oag-loading-icon-gt");
        let movie = oag_game::loading::icon::open(clip, &cache)
            .unwrap_or_else(|e| panic!("{style}: {e:#}"));
        assert!(movie.frame_count > 1, "{style}");
        assert!(
            oag_game::loading::icon::clip(&assets, Some(5)).is_none(),
            "an unmapped mode draws no icon"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_still_gets_its_tips_and_its_own_strip() {
    let Some(image) = image("pulse-psp-eu.chd") else {
        return;
    };
    let source = image.display().to_string();
    let (table, entries) = table_and_entries(&image);
    let assets = Assets::load(&source, &table, entries.as_deref(), None);
    for note in &assets.notes {
        println!("{note}");
    }

    assert!(assets.wave, "Pulse authors the wave");
    assert!(
        assets.tips.len() >= 20,
        "the disc's own loading tips: {}",
        assets.tips.len()
    );
    assert!(
        assets.features.is_empty(),
        "Pulse ships no illustrated features"
    );
    assert!(assets.art.is_none(), "and so draws no illustration");
    assert!(
        assets.caption.is_none(),
        "Pulse names no caption id, so the heading stands"
    );
    // The disc's own strip rather than the stand-in, which `Assets::load`
    // reports by name when it has to fall back.
    assert!(
        !assets
            .notes
            .iter()
            .any(|note| note.contains("authored stand-in strip")),
        "{:?}",
        assets.notes
    );
}

/// Pure authors no loading screen, and the loader says which of the two that is.
///
/// "This title has none" and "this title's assets would not read" are different
/// facts and used to produce the same silence. The note is the difference.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_authors_no_loading_screen_and_says_so() {
    let Some(image) = image("pure-psp-eu.chd") else {
        return;
    };
    let source = image.display().to_string();
    let (table, entries) = table_and_entries(&image);
    let assets = Assets::load(&source, &table, entries.as_deref(), None);
    for note in &assets.notes {
        println!("{note}");
    }

    assert!(!assets.wave);
    assert!(assets.tips.is_empty());
    assert!(assets.features.is_empty());
    assert!(assets.art.is_none());
    assert!(
        assets
            .notes
            .iter()
            .any(|note| note.contains("authors no loading screen of its own")),
        "{:?}",
        assets.notes
    );
}

/// The circuit load really runs on a thread, and the screen can name it while
/// it does.
///
/// **What this covers and what it does not.** The worker itself is testable
/// headlessly and is tested here: that it comes back with a `Loaded`, that its
/// snapshot carries the label a caller handed it and counts nothing, and that
/// `join` is idempotent. The *stage transition* built on it - `launch_race`
/// putting the screen up, `finish_loading` swapping it for the grid - needs a
/// window and is not covered anywhere.
///
/// Wipeout HD rather than Pulse on purpose: HD's circuits are the slow end of
/// the 1.5-to-5-second range that made the blocking load worth moving off the
/// frame thread in the first place.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_circuit_load_runs_on_a_worker_and_names_what_it_is_reading() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let options = oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(r"Data\Environments\Talons_Junction\track.vex".to_string()),
        ..oag_raceplay::Options::default()
    };
    let label = "TALON'S JUNCTION".to_string();
    let mut worker = oag_raceplay::LoadWorker::spawn(options, Some(label.clone()), None, false);

    // The screen's own view of it, before the load has necessarily finished.
    let progress: oag_game::prefetch::Progress = worker.progress().into();
    assert_eq!(progress.current.as_deref(), Some(label.as_str()));
    assert_eq!(progress.total, 0, "a race load counts nothing");
    assert_eq!(progress.done, 0);

    let loaded = worker
        .join()
        .expect("the thread started")
        .expect("the circuit reads");
    assert!(
        !loaded.setup.spline.is_empty(),
        "a real circuit came back, not an empty one"
    );
    assert!(worker.is_finished());
    assert!(
        worker.join().is_none(),
        "joining twice is a no-op rather than a panic"
    );
}

fn omega() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

/// Omega's `Title` names a loading screen, and every file it names is in the package pair and
/// decodes: the five illustrations (1024 by 512, the size the layout's picture window is cut
/// from) and the six marks.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_names_a_loading_screen_whose_files_are_on_the_disc() {
    let Some(source) = omega() else { return };
    let loading = oag_omega::TITLE
        .loading
        .expect("Omega names a loading screen");
    assert!(loading.wave.is_none(), "a still, not Pulse's wave");
    assert_eq!(loading.features.len(), 1, "one styling: WOHD_Colours");
    let mut archives = oag_omega::open(&source.display().to_string()).expect("Omega opens");
    let entries = loading.features[0]
        .features
        .iter()
        .map(|feature| (feature.image, true))
        .chain(loading.chrome.iter().map(|(_, entry)| (*entry, false)));
    let mut count = 0;
    for (entry, illustration) in entries {
        let blob = archives
            .read_name(entry)
            .unwrap_or_else(|e| panic!("{entry}: {e}"));
        let texture =
            oag_texture::gnf::Texture::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
        texture
            .decode(&blob)
            .unwrap_or_else(|e| panic!("{entry} does not decode: {e}"));
        if illustration {
            assert_eq!((texture.width, texture.height), (1024, 512), "{entry}");
        }
        count += 1;
    }
    assert_eq!(count, 11);
}

/// The screen Omega draws is built from its own files: five features whose pictures are in the
/// sheet, the six marks, the four menu colours through `WOHD_Colours`, the caption and brand out
/// of Omega's own English table, and a layout in which nothing is skipped but the mode clip.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_draws_its_loading_screen_from_its_own_files() {
    let Some(source) = omega() else { return };
    let (table, entries) = table_and_entries(&source);
    let assets = Assets::load(
        &source.display().to_string(),
        &table,
        entries.as_deref(),
        None,
    );
    for note in &assets.notes {
        println!("{note}");
    }
    assert!(!assets.wave);
    assert_eq!(assets.style, Some("HD"));
    assert_eq!(assets.caption.as_deref(), Some("LOADING..."));
    assert_eq!(assets.features.len(), 5, "all five features resolve");
    let art = assets.art.as_ref().expect("Omega ships art");
    assert_eq!(art.illustrations.len(), assets.features.len());
    for mark in [
        art.title_arrow,
        art.subtitle_arrow,
        art.rule,
        art.corner,
        art.dot,
        art.square,
    ] {
        assert!(mark.is_some(), "a mark is missing from the sheet");
    }
    assert!(
        assets
            .notes
            .iter()
            .all(|note| !note.contains("did not decode") && !note.contains("would not reverse")),
        "{:?}",
        assets.notes
    );

    // `WOHD_Colours`: white ground, and the ink it overwrites `HD_Grey` with.
    let palette = assets.palette.expect("the four globals resolve");
    assert_eq!(palette.background, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(palette.ink[..3], [1.0 / 255.0, 0.0, 1.0 / 255.0]);

    let layouts = assets.layout.as_ref().expect("Omega's layout loads");
    let layout = layouts
        .for_race("single_race", 16.0 / 9.0)
        .expect("it answers a single race");
    let brand = layout
        .elements
        .iter()
        .find(|element| element.id == "label-title")
        .and_then(|element| element.string_id.as_deref())
        .expect("the brand is a string id");
    assert_eq!(brand, "MSC_WOOM_CAPS");
    assert!(
        assets
            .layout_strings
            .iter()
            .any(|(id, text)| id == brand && text.contains("OMEGA COLLECTION")),
        "{:?}",
        assets.layout_strings
    );
    assert!(assets.mode_icons.is_empty(), "no .bik ships on Omega");

    // The mode icon is a still per mode, every one of the six pictures decoded.
    let stills = art.mode_stills.as_ref().expect("Omega names mode stills");
    assert!(stills.get(Some(3)).is_some(), "a single race has its flag");
    for mode in [4, 5, 6, 8, 10, 0x11, 0x14] {
        assert!(stills.get(Some(mode)).is_some(), "mode {mode}");
    }
    for mode in [0xd, 0xe, 0x15] {
        assert!(stills.get(Some(mode)).is_none(), "mode {mode} draws none");
    }
    assert_ne!(
        stills.get(Some(5)),
        stills.get(Some(8)),
        "one picture per mode"
    );
}

/// A `.gnf` the front end lists as bottom-up is drawn the way up HD's `.gtf` of the same stem
/// is: the loading sheet's corner, rule and square cover the pixels HD's do. (The colour
/// channels and the near-zero alpha differ: Omega re-cooked them.)
#[test]
#[ignore = "needs the decrypted PS4 package pair and the HD disc image"]
fn omega_marks_are_drawn_the_way_up_hd_draws_them() {
    let (Some(source), Some(hd)) = (omega(), image("hdfury-ps3-eu-dec.iso")) else {
        return;
    };
    let load = |path: &Path| {
        let (table, entries) = table_and_entries(path);
        Assets::load(
            &path.display().to_string(),
            &table,
            entries.as_deref(),
            None,
        )
    };
    let (omega, hd) = (load(&source), load(&hd));
    let rect_pixels = |assets: &Assets, rect: [f32; 4]| -> Vec<u8> {
        let sheet = &assets.art.as_ref().expect("art").sheet;
        let (x, y, w, h) = (
            rect[0] as usize,
            rect[1] as usize,
            rect[2] as usize,
            rect[3] as usize,
        );
        let stride = sheet.width as usize * 4;
        (y..y + h)
            .flat_map(|row| sheet.rgba[row * stride + x * 4..row * stride + (x + w) * 4].to_vec())
            .collect()
    };
    for (name, ours, theirs) in [
        (
            "corner",
            omega.art.as_ref().unwrap().corner,
            hd.art.as_ref().unwrap().corner,
        ),
        (
            "rule",
            omega.art.as_ref().unwrap().rule,
            hd.art.as_ref().unwrap().rule,
        ),
        (
            "square",
            omega.art.as_ref().unwrap().square,
            hd.art.as_ref().unwrap().square,
        ),
    ] {
        let (ours, theirs) = (ours.expect(name), theirs.expect(name));
        assert_eq!((ours[2], ours[3]), (theirs[2], theirs[3]), "{name} size");
        let mask = |assets: &Assets, rect: [f32; 4]| -> String {
            rect_pixels(assets, rect)
                .chunks_exact(4 * rect[2] as usize)
                .map(|row| {
                    row.chunks_exact(4)
                        .map(|px| if px[3] > 127 { '#' } else { '.' })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(
            mask(&omega, ours),
            mask(&hd, theirs),
            "{name}: Omega's opaque pixels are where HD's are (the corner is an L, so a flipped \
             sheet fails)"
        );
    }
}
