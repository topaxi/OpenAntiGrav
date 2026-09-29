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
        cache: std::env::temp_dir().join("oag-2048-event-card-ground-truth"),
        audio_cache: boot::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
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
