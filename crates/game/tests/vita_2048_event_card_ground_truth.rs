//! Wipeout 2048's per-event card against its real package.
//!
//! **`#[ignore]`d and never run in CI.** Needs
//! `data/extracted/vita/PCSF00007/base`, the decrypted EU package
//! (`docs/architecture/adr/0006-no-copyrighted-content.md`). Run with
//! `just test-data`.
//!
//! What the card says is read off `SP.xml`, the language table and the
//! `NewImages` art; `docs/formats/2048-campaign.md`'s "The event card" is the
//! evidence. These tests pin that every event that names a photo, an emblem
//! or a class glyph finds it in the sprite sheet, and that the reference
//! frame's own facts (`EMPIRE CLIMB`, three laps, class C) come out.

use std::path::{Path, PathBuf};

use oag_2048::frontend::states as w2048;
use oag_game::boot;
use oag_gameplay::input::{Button, Input};
use oag_ui::frontend::{self, Draw};

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn options(source: &Path) -> boot::Options {
    boot::Options {
        language: Some("English".to_string()),
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: oag_testdata::scratch::cache("2048-event-card-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    }
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn every_photo_emblem_and_class_glyph_a_card_names_is_in_the_sprite_sheet() {
    let Some(source) = source() else { return };
    let (shell, _archives, _title) = boot::load_shell(&options(&source)).expect("the shell");
    assert!(!shell.campaign_events.is_empty());
    let mut photos = 0;
    for event in &shell.campaign_events {
        for name in [
            &event.card.photo,
            &event.card.emblem,
            &event.card.class_icon,
        ]
        .into_iter()
        .flatten()
        {
            assert!(
                shell.sprites.get(name).is_some(),
                "{}: {name} is named by its card but not in the sheet",
                event.name
            );
        }
        for name in event
            .card
            .forced_craft
            .iter()
            .flat_map(|craft| [&craft.logo, &craft.type_icon])
            .flatten()
            .chain(
                event
                    .card
                    .allowed_classes
                    .iter()
                    .map(|allowed| &allowed.icon),
            )
        {
            assert!(
                shell.sprites.get(name).is_some(),
                "{}: rules page names {name}, not in the sheet",
                event.name
            );
        }
        photos += usize::from(event.card.photo.is_some());
    }
    assert!(
        photos > 100,
        "most of the 130 events sit on a base circuit, got {photos}"
    );
    for name in frontend::CARD_TEXTURES {
        assert!(shell.sprites.get(name).is_some(), "{name} did not decode");
    }
    assert!(
        shell.sprites.height <= 8192 && shell.sprites.width <= 8192,
        "{}x{} exceeds a texture",
        shell.sprites.width,
        shell.sprites.height
    );
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_first_events_card_is_the_reference_frames_and_draws_its_photo_and_buttons() {
    let Some(source) = source() else { return };
    let boot = boot::load(&options(&source)).expect("the whole boot");
    let events = boot.frontend.campaign_events().to_vec();
    let first = events
        .iter()
        .find(|event| event.name == "2048 - Event 1")
        .expect("Event 1");
    assert_eq!(
        first.card.title, "EMPIRE CLIMB",
        "frame 12's own event card"
    );
    assert_eq!(first.card.laps, Some(3));
    assert!(first.card.has_objective);
    assert_eq!(first.card.objective.as_deref(), Some("FINISH THE EVENT"));
    assert!(
        first
            .card
            .class_icon
            .as_deref()
            .is_some_and(|name| name.ends_with("c_class.gtf")),
        "{:?}",
        first.card.class_icon
    );
    let mut frontend = boot.frontend;
    let mut input = Input::new();
    let mut reached = false;
    for tick in 0..3600u32 {
        input.begin_frame(if tick.is_multiple_of(2) {
            Button::Cross.bit()
        } else {
            0
        });
        frontend.update(1.0 / 60.0, &mut input, None);
        if frontend.machine().is(w2048::NEW_FE_SHELL) {
            reached = true;
            break;
        }
    }
    assert!(reached);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, &mut input, None);
    input.begin_frame(Button::Cross.bit());
    frontend.update(1.0 / 60.0, &mut input, None);
    assert!(frontend.event_card_open());
    let list = frontend.draw_list();
    let photo = list
        .iter()
        .filter(|draw| matches!(draw, Draw::Sprite { rect, .. } if rect[..] == [16.0, 92.0, 404.0, 334.0]))
        .count();
    assert_eq!(photo, 1, "the photo body, at frame 14's own rect");
    let buttons = list
        .iter()
        .filter(
            |draw| matches!(draw, Draw::Fill { rect, .. } if rect[1] == 432.0 && rect[2] == 122.0),
        )
        .count();
    assert_eq!(buttons, 3, "Change craft, Back, Launch");
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn every_event_that_authors_a_pass_objective_words_it() {
    let Some(source) = source() else { return };
    let (shell, _archives, _title) = boot::load_shell(&options(&source)).expect("the shell");
    let mut worded = 0;
    for event in &shell.campaign_events {
        if event.card.has_objective {
            let text = event.card.objective.as_deref();
            assert!(
                text.is_some_and(|text| !text.contains('%')),
                "{}: pass objective is {text:?}",
                event.name
            );
            worded += 1;
        }
    }
    assert!(worded >= 60, "got {worded} worded objectives");
    let by = |name: &str| {
        shell
            .campaign_events
            .iter()
            .find(|event| event.name == name)
            .and_then(|event| event.card.objective.clone())
    };
    // `SpeedLapRace`'s override: the target (13000 centiseconds) as M:SS.
    assert_eq!(by("2048 - Event 3").as_deref(), Some("BEAT 2:10"));
    // `ZoneRace`'s override.
    assert_eq!(by("2048 - Event 3-2").as_deref(), Some("ZONE TARGET : 15"));
    // Every other class falls through to `FE_SCORE_POINTS`.
    assert!(
        shell
            .campaign_events
            .iter()
            .filter_map(|event| event.card.objective.as_deref())
            .any(|text| text.starts_with("SCORE ") && text.ends_with(" POINTS")),
        "an Elimination BEAT_VALUE event words its target as points"
    );
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_kind_line_and_photo_follow_the_class_the_way_the_executable_does() {
    let Some(source) = source() else { return };
    let (shell, _archives, _title) = boot::load_shell(&options(&source)).expect("the shell");
    let find = |name: &str| {
        shell
            .campaign_events
            .iter()
            .find(|event| event.name == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    // `GameMode_SpeedLapRace` with a lap count and a `BEAT M:SS` pass: the
    // stopwatch icon and `TIME TRIAL` (`FUN_812b021a`, `M_BUTTONSHAPE` 0).
    let timed = find("2048 - Event 3");
    assert_eq!(timed.kind, frontend::EventIcon::SpeedLap);
    assert_eq!(timed.card.kind_label.as_deref(), Some("TIME TRIAL"));
    assert_eq!(
        find("2048 - Event 1").card.kind_label.as_deref(),
        Some("RACE")
    );
    // `M_DESCRIPTION` words the line under the title where the event has one
    // (`CampaignEventCard_DrawPanel`); an empty one leaves the kind.
    assert_eq!(
        find("2048 - Event 1").card.description.as_deref(),
        Some("NO WEAPONS")
    );
    assert_eq!(find("2048 - Event 3").card.description, None);
    let zone = find("2048 - Event 3-2");
    assert_eq!(zone.card.kind_label.as_deref(), Some("ZONE"));
    assert!(
        zone.card
            .photo
            .as_deref()
            .is_some_and(|photo| photo.ends_with(r"trackscreens\ZoneTower.gtf")),
        "{:?}",
        zone.card.photo
    );
    assert!(
        shell
            .campaign_events
            .iter()
            .any(|event| event.card.kind_label.as_deref() == Some("SPEED LAP")),
        "the 40 shape-5/6 events"
    );
    assert!(
        shell
            .campaign_events
            .iter()
            .any(|event| event.card.kind_label.as_deref() == Some("COMBAT")),
        "Elimination's own word"
    );
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_race_load_of_event_one_draws_its_card_a_real_tip_and_the_modes_paragraph() {
    let Some(source) = source() else { return };
    let boot = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = boot.frontend;
    let card = frontend
        .launch_event_card("2048 - Event 1", 0)
        .expect("Launch on the open card leaves it");
    assert_eq!(card.event().card.title, "EMPIRE CLIMB");

    let assets = oag_game::loading::Assets::load(
        &options(&source).source,
        &boot.strings,
        boot.entries.as_deref(),
        None,
    );
    let tips = assets.game_tips.as_ref().expect("2048 names game tips");
    for (row, tip) in tips.rows.iter().enumerate().take(34) {
        let tip = tip
            .as_ref()
            .unwrap_or_else(|| panic!("row {row}: words did not resolve"));
        assert!(!tip.title.is_empty() && !tip.body.is_empty(), "row {row}");
        assert!(
            boot.sprites.get(&tip.texture).is_some(),
            "row {row}: {} is not in the sheet",
            tip.texture
        );
    }
    assert!(
        boot.sprites
            .get(&tips.rows[0].as_ref().unwrap().question_mark)
            .is_some(),
        "the ? mark"
    );

    let mut screen = oag_game::loading::Screen::new(&assets, boot.font.line_height, 0, None);
    screen.set_card(card, assets.game_tips.as_ref(), 3);
    let atlas = boot.font.clone();
    let progress = oag_game::prefetch::Progress::default();
    let words = |screen: &oag_game::loading::Screen| -> Vec<String> {
        screen
            .draw_list(oag_game::loading::Phase::Race, &progress, &atlas)
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    };
    let tip = screen.card_tip().expect("a tip").clone();
    let first = words(&screen);
    assert!(first.contains(&"EMPIRE CLIMB".to_string()), "{first:?}");
    assert!(first.contains(&tip.header), "GAME TIP header: {first:?}");
    assert!(first.contains(&tip.title), "{first:?}");
    assert!(first.iter().any(|text| text == "LOADING..."), "{first:?}");
    assert!(first.contains(&"0%".to_string()), "{first:?}");

    screen.set_load_stage(4);
    let second = words(&screen);
    let mode = "Race Events are classic Wipeout";
    assert!(
        second.iter().any(|text| text.starts_with(mode)),
        "the mode page's paragraph: {second:?}"
    );
    assert!(!second.contains(&tip.title), "the tip is dropped");
}

/// `GameModeChoice`'s header block sits above the lattice and under its glyph, the tiles'
/// glyphs are `White2048` and their labels are drawn in `NEOSANS_BOLD`.
fn check_the_game_mode_header_glyphs_and_labels(draws: &[Draw]) {
    let white = [225.0 / 255.0, 228.0 / 255.0, 235.0 / 255.0, 1.0];
    let blue = [25.0 / 255.0, 41.0 / 255.0, 93.0 / 255.0, 1.0];
    let close = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.01);
    let header = [15.0, 15.0, 70.0, 70.0];
    let block = draws
        .iter()
        .position(|draw| {
            matches!(draw, Draw::Fill { rect, color } if *rect == header && close(*color, blue))
        })
        .expect("the header's Blue2048 block");
    let glyph = draws
        .iter()
        .position(|draw| matches!(draw, Draw::Sprite { rect, .. } if *rect == header))
        .expect("the header glyph");
    let lattice = draws
        .iter()
        .position(|draw| {
            matches!(draw, Draw::Sprite { uv, .. }
                if uv[2] == frontend::canvas_lattice::VIEWPORT[0] as f32)
        })
        .expect("the lattice");
    assert!(
        lattice < block && block < glyph,
        "{lattice} {block} {glyph}"
    );
    // Single Player Campaign's 128x128 glyph, centred in its tile.
    let single_player = [231.5, 98.5, 128.0, 128.0];
    assert!(
        draws.iter().any(|draw| matches!(draw,
            Draw::Sprite { rect, color, .. } if *rect == single_player && close(*color, white))),
        "the tile glyph is White2048"
    );
    let label = draws
        .iter()
        .find(
            |draw| matches!(draw, Draw::FacedText { text, .. } if text == "SINGLE PLAYER CAMPAIGN"),
        )
        .expect("the Single Player label in its own face");
    assert!(
        matches!(label, Draw::FacedText { role: "NEOSANS_BOLD", scale, .. } if *scale == 1.0),
        "{label:?}"
    );
}

/// The open card of the first event on screen: its description under the title, the pass
/// group centred on the panel, the leaderboard's tabs in `NEOSANS_BOLD` over an orange bar
/// and the buttons' glyphs in `White2048`.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_open_card_of_event_one_words_and_places_what_the_disc_authors() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    // The label face is the disc's own, and the shell hands it to the renderer.
    assert_eq!(
        loaded.label_face.as_ref().map(|(_, role)| *role),
        Some("NEOSANS_BOLD")
    );
    let mut frontend = loaded.frontend;
    let mut input = Input::new();
    let mut tick = 0u32;
    let mut run = |frontend: &mut frontend::Frontend, pressed: u32| {
        input.begin_frame(if tick.is_multiple_of(2) { pressed } else { 0 });
        tick += 1;
        frontend.update(1.0 / 60.0, &mut input, None);
    };
    for _ in 0..3600 {
        if frontend.machine().is(w2048::GAME_MODE_CHOICE) {
            break;
        }
        run(&mut frontend, Button::Cross.bit() | Button::Start.bit());
    }
    // Past the widgets' entry fades.
    for _ in 0..60 {
        run(&mut frontend, 0);
    }
    check_the_game_mode_header_glyphs_and_labels(&frontend.draw_list());
    for _ in 0..3600 {
        if frontend.machine().is(w2048::NEW_FE_SHELL) {
            break;
        }
        run(&mut frontend, Button::Cross.bit() | Button::Start.bit());
    }
    assert!(frontend.select_campaign_event("2048 - Event 1"));
    for _ in 0..600 {
        if frontend.event_card_open() {
            break;
        }
        run(&mut frontend, Button::Cross.bit());
    }
    assert!(frontend.event_card_open());

    let draws = frontend.draw_list();
    let texts: Vec<(&str, f32)> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, x, .. } => Some((text.as_str(), *x)),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|(text, _)| *text == "NO WEAPONS"),
        "{texts:?}"
    );
    // The original's frame starts both lines at x 574.
    for line in ["PASS", "FINISH THE EVENT"] {
        let x = texts.iter().find(|(text, _)| *text == line).expect(line).1;
        assert!((x - 574.0).abs() < 3.0, "{line} at {x}");
    }
    let white = [225.0 / 255.0, 228.0 / 255.0, 235.0 / 255.0];
    assert!(
        draws
            .iter()
            .any(|draw| matches!(draw, Draw::Sprite { rect, color, .. }
            if rect[0] == 851.0 && (color[0] - white[0]).abs() < 0.01)),
        "the Launch glyph is White2048"
    );

    for _ in 0..30 {
        if frontend.event_card_page() == Some(1) {
            break;
        }
        run(&mut frontend, Button::Right.bit());
    }
    let draws = frontend.draw_list();
    for word in ["PERSONAL", "CURRENT BEST"] {
        assert!(
            draws.iter().any(|draw| matches!(draw,
                Draw::FacedText { role: "NEOSANS_BOLD", text, scale, .. }
                    if text == word && *scale == 0.7)),
            "{word} in NEOSANS_BOLD at 0.7"
        );
    }
    let orange = [221.0 / 255.0, 88.0 / 255.0, 11.0 / 255.0];
    assert!(
        draws
            .iter()
            .any(|draw| matches!(draw, Draw::Fill { rect, color }
            if *rect == [471.0, 234.0, 422.0, 37.0] && (color[0] - orange[0]).abs() < 0.01
                && (color[1] - orange[1]).abs() < 0.01)),
        "the CURRENT BEST bar is Orange2048"
    );
}

/// 2048's front-end root authors no `NavigationController`, so there is no footer legend to flow
/// (`docs/ghidra/functions/ps4-omega-eu/navigation-legend.md`, "Wired").
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_2048_root_authors_no_footer_legend() {
    let Some(source) = source() else { return };
    let (shell, _archives, _title) = boot::load_shell(&options(&source)).expect("the shell");
    assert!(shell.nav_legend.is_none());
}
