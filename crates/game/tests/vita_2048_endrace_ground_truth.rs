//! Wipeout 2048's race-ending pages against its real package: the file reads,
//! every texture it names decodes into the sheet, a real race's result is
//! worded from the disc's own strings, and the pages draw what that result
//! says with none of the XML's development text.
//!
//! **`#[ignore]`d and never run in CI.** Needs
//! `data/extracted/vita/PCSF00007`, the decrypted EU package
//! (`docs/architecture/adr/0006-no-copyrighted-content.md`). Run with
//! `just test-data`. The page layout is `docs/ui/endrace-2048.md`; the law the
//! tones and the medal cells come from is
//! `docs/ghidra/functions/vita-2048-eu-v104/endrace-summary.md`.

use std::path::{Path, PathBuf};

use oag_2048::campaign::{EventKind, EventObjectives, ObjectiveRule, Tier, objective_type};
use oag_game::boot;
use oag_game::endrace::touch::{Facts, Standing, finished_event, load, summary};
use oag_ui::frontend::{self, Draw};
use oag_ui_screens::endrace::touch::{self, Button, Page, Tone};
use oag_ui_screens::picker::FaceScales;

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
        cache: std::env::temp_dir().join("oag-2048-endrace-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    }
}

/// The `EndRace` pages read off the real package, and the shell they sit in.
struct Opened {
    shell: boot::Shell,
    screens: oag_game::endrace::touch::TouchScreens,
}

fn open(source: &Path) -> Opened {
    let (shell, mut archives, title) = boot::load_shell(&options(source)).expect("the shell");
    let entry = title
        .front_end
        .and_then(|front_end| front_end.endrace_entry)
        .expect("2048 names its EndRace definition");
    let globals: Vec<(&str, &str)> = shell
        .screens
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let screens = load(
        &mut archives,
        entry,
        &shell.strings,
        FaceScales::default(),
        [shell.space.size.0, shell.space.size.1],
        &shell.sprites,
        &globals,
    )
    .expect("the EndRace pages read");
    Opened { shell, screens }
}

fn draws(opened: &Opened, facts: &Facts, page: Page) -> (Vec<Draw>, touch::Summary) {
    let model = summary(facts, &opened.shell.strings);
    let sprites = &opened.screens.sprites;
    let list = touch::draw_list(
        &model,
        page,
        Button::Exit,
        &opened.screens.layouts,
        &|src| sprites.get(src),
        &opened.shell.face_scales,
        opened.shell.font.line_height,
    );
    (list, model)
}

fn texts(list: &[Draw]) -> Vec<&str> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn position(target: i64) -> ObjectiveRule {
    ObjectiveRule {
        objective_type: Some(objective_type::POSITION),
        target: Some(target),
    }
}

fn objectives(pass: i64, elite: i64) -> EventObjectives {
    EventObjectives {
        kind: EventKind::Race,
        pass: position(pass),
        elite: position(elite),
    }
}

/// The package ships the file, the title's data points at it, and every texture
/// the pages and their tiles name decodes into the sheet - the medal sheet and
/// the three objective icons among them.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_pages_read_and_every_texture_they_name_is_in_the_sheet() {
    let Some(source) = source() else { return };
    let opened = open(&source);
    let layouts = &opened.screens.layouts;
    assert!(layouts.objectives.is_some(), "ObjectiveSummary is authored");
    for layout in [&layouts.shell, &layouts.summary] {
        for image in &layout.screen.images {
            assert!(
                opened.screens.sprites.get(&image.src).is_some(),
                "{} is named by a page but is not in the sheet",
                image.src
            );
        }
    }
    for button in &layouts.shell.screen.touch_buttons {
        if let Some(src) = &button.src {
            // The Near logo is the online tile this build never draws; every
            // other tile icon must decode.
            if button.name.as_deref() != Some("NearTouchButton") {
                assert!(opened.screens.sprites.get(src).is_some(), "{src}");
            }
        }
    }
    let medals = opened
        .screens
        .sprites
        .get(r"Data\FE\NewImages\Post_Race_Medals.gtf")
        .expect("the medal sheet decodes");
    assert!(
        medals.width >= 384 && medals.height >= 128,
        "{}x{}: the pass (128) and elite (256) cells are 128 wide each",
        medals.width,
        medals.height
    );
}

/// The package holds two copies of the file (the base's 22,076 bytes and the
/// patch's 22,098); the one this build serves is the base's, the only one its
/// source mounts for 2048's front end. They differ by one attribute on a screen
/// this build never draws, so which is read changes no widget - this pins which
/// it is, so the day the patch is mounted the choice is made on purpose.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_base_copy_of_the_file_is_the_one_served() {
    let Some(source) = source() else { return };
    let (_shell, mut archives, title) = boot::load_shell(&options(&source)).expect("the shell");
    let entry = title
        .front_end
        .and_then(|front_end| front_end.endrace_entry)
        .expect("2048 names its EndRace definition");
    let sizes: Vec<usize> = archives
        .read_every_name(entry)
        .into_iter()
        .map(|(_, blob)| blob.len())
        .collect();
    assert_eq!(sizes, [22_076], "only the base copy is mounted");
    assert_eq!(archives.read_name(entry).expect("served").len(), 22_076);
}

/// A real race, flown to its end, says what it came to in the disc's own words.
/// **Dropping the wiring fails this**: with no `endrace_style` or no loader the
/// pages are empty, and with no `Facts::from_race` there is no verdict.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_real_event_says_what_it_came_to() {
    let Some(source) = source() else { return };
    let opened = open(&source);
    let race_options = oag_raceplay::Options {
        source: source.display().to_string(),
        ..oag_raceplay::Options::default()
    };
    // "FinishRaceAnyPosition" for the pass bar, a win for the elite one: the
    // autopilot finishes, so the event is at least passed.
    let facts = finished_event(&race_options, "2048 - Event 1", 30_000).expect("the event races");
    let tier = facts.tier.expect("finishing passes `2048 - Event 1`");
    let (list, model) = draws(&opened, &facts, Page::Summary);
    assert_eq!(
        model.tone,
        if tier == Tier::Elite {
            Tone::Elite
        } else {
            Tone::Pass
        }
    );
    let shown = texts(&list);
    assert!(shown.contains(&"CONGRATULATIONS!"), "{shown:?}");
    assert!(shown.contains(&"RACE SUMMARY"), "{shown:?}");
    let Standing::Place(place) = facts.standing else {
        panic!("a single race comes to a place, not {:?}", facts.standing);
    };
    assert!(
        shown.iter().any(|t| t.starts_with(&place.to_string())),
        "the place {place} is not on the page: {shown:?}"
    );
    for leaked in [
        "TOTAL_4325_XP_TEST",
        "PASS_TEST",
        "LAALA",
        "THE OBJECTIVE",
        "99",
    ] {
        assert!(!shown.contains(&leaked), "{leaked} drew");
    }
}

/// A missed bar paints the failure colour and words the failure; the objective
/// page ticks its row as failed. Fed the real race's own result against a bar
/// it did not clear.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_missed_objective_fails_in_the_disc_s_words() {
    let Some(source) = source() else { return };
    let opened = open(&source);
    let facts = Facts {
        mode: oag_race::Mode::SingleRace,
        standing: Standing::Place(5),
        objectives: Some(objectives(3, 1)),
        tier: None,
    };
    let (list, model) = draws(&opened, &facts, Page::Summary);
    assert_eq!(model.tone, Tone::Fail);
    let shown = texts(&list);
    assert!(shown.contains(&"FAIL"), "{shown:?}");
    assert!(
        list.iter().any(|d| matches!(d, Draw::Fill { color, .. }
            if *color == oag_ui::screen::argb_to_rgba(0xffcd_0102))),
        "no failure-coloured bar"
    );
    let (page, model) = draws(&opened, &facts, Page::Objectives);
    assert_eq!(model.rows.len(), 1);
    assert_eq!(model.rows[0].state, Tone::Fail);
    assert!(texts(&page).contains(&model.rows[0].text.as_str()));
}

/// The elite bar met lists the elite objective, ticked as elite; the pass bar
/// met lists the pass objective, ticked as a pass.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_objective_page_lists_the_chain_that_was_earned() {
    let Some(source) = source() else { return };
    let opened = open(&source);
    let facts = |tier| Facts {
        mode: oag_race::Mode::SingleRace,
        standing: Standing::Place(1),
        objectives: Some(objectives(3, 1)),
        tier,
    };
    let (_, elite) = draws(&opened, &facts(Some(Tier::Elite)), Page::Objectives);
    let (_, pass) = draws(&opened, &facts(Some(Tier::Pass)), Page::Objectives);
    assert_eq!(elite.rows[0].state, Tone::Elite);
    assert_eq!(pass.rows[0].state, Tone::Pass);
    assert_ne!(elite.rows[0].text, pass.rows[0].text, "two different bars");
}
