//! Validates Wipeout 2048's boot chain and touch grids against its real
//! package.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship - `data/extracted/vita/PCSF00007/base`, the
//! decrypted EU package. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Until 2026-09-21 `boot::load_shell` refused this title by name for having
//! no `MenuSkin`-shaped menu. What replaced the refusal is a boot that walks
//! the chain `oag_2048::frontend::BOOT_PROFILE` declares - read out of the
//! root's `<LoadXML>` includes, since the root itself declares no screen -
//! and lands on the `GameModeChoice` touch grid. Everything here checks that
//! *mechanism* against the disc: that the includes resolve, that every
//! chain step is found and driven, that the grid the file authors is the one
//! `oag_title::FrontEnd::touch` names, and that the sequence gets from
//! `Boot Connect` to the campaign shell on the pad alone. **None of it is
//! evidence about what a Vita does** - the chain is `Provenance::Declared`
//! and stays so; the one Vita3K boot that corroborates it is described in
//! `docs/formats/2048-frontend.md`, not here.

use std::path::{Path, PathBuf};

use oag_2048::frontend::states as w2048;
use oag_game::boot;
use oag_gameplay::input::{Button, Input};
use oag_ui::frontend;

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
        cache: std::env::temp_dir().join("oag-2048-boot-ground-truth"),
        audio_cache: boot::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    }
}

/// The cheap half: no movie read at all.
fn shell(source: &Path) -> boot::Shell {
    let (shell, _archives, _title) =
        boot::load_shell(&options(source)).expect("2048's front end walks since 2026-09-21");
    shell
}

/// Ticks the sequence with `pressed` held down on alternate ticks - the same
/// pulse `capture::run` gives `--press` - until it reaches `state` or gives
/// up.
fn drive_until(frontend: &mut frontend::Frontend, pressed: &[Button], state: &str) -> bool {
    let mut input = Input::new();
    let mask = pressed
        .iter()
        .fold(0u32, |mask, button| mask | button.bit());
    for tick in 0..3600u32 {
        input.begin_frame(if tick.is_multiple_of(2) { mask } else { 0 });
        frontend.update(1.0 / 60.0, &mut input, None);
        if frontend.machine().is(state) {
            return true;
        }
    }
    false
}

/// The root declares no screen; the includes are where every chain step is.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn every_declared_chain_step_is_found_through_the_includes_and_driven() {
    let Some(source) = source() else { return };
    let shell = shell(&source);
    let walked: Vec<&str> = shell.walked.iter().map(|step| step.state).collect();
    assert_eq!(
        walked,
        vec![
            w2048::BOOT_CONNECT,
            w2048::BOOT_STUDIO_LOGO,
            w2048::BOOT_INTRO_MOVIE,
            w2048::LOAD_SAVE_BOOTUP,
            w2048::TITLE_SCREEN,
        ],
        "all five steps, none skipped: {:#?}",
        shell.report
    );
    for step in &walked {
        assert!(
            shell.screens.by_name(step).is_some(),
            "{step:?} is in the merged screens"
        );
    }
    assert!(
        shell
            .report
            .iter()
            .any(|line| line.contains("Bootup_Definition_EU.xml")),
        "the localised include resolved to the EU package's file: {:#?}",
        shell.report
    );
    assert_eq!(
        shell.profile.provenance,
        oag_title::Provenance::Declared,
        "one Vita3K boot is not ADR-0025's bar"
    );
}

/// The Studio Liverpool card's `<Redirect delay="4.0">` is read, not
/// transcribed.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_studio_logo_carries_its_own_four_second_delay() {
    let Some(source) = source() else { return };
    let shell = shell(&source);
    let screen = shell
        .screens
        .by_name(w2048::BOOT_STUDIO_LOGO)
        .expect("the card is loaded");
    let timed = screen
        .redirects
        .iter()
        .find(|redirect| redirect.delay.is_some())
        .expect("one redirect carries a delay");
    assert_eq!(timed.delay, Some(4.0));
    assert_eq!(timed.goto.as_deref(), Some(w2048::BOOT_INTRO_MOVIE));
}

/// The legal footer is a `DirectEmbed` include and lands on `TitleScreen`.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_title_screen_embeds_its_eu_legal_footer() {
    let Some(source) = source() else { return };
    let shell = shell(&source);
    let screen = shell
        .screens
        .by_name(w2048::TITLE_SCREEN)
        .expect("the title screen is loaded");
    let ids: Vec<&str> = screen
        .texts
        .iter()
        .filter_map(|text| text.idstring.as_deref())
        .collect();
    assert!(ids.contains(&"BOOT_PRESS_ANY"), "{ids:?}");
    assert!(ids.contains(&"BOOT_LEGAL_TRADEMARK_FULL_EU"), "{ids:?}");
    let footer = screen
        .texts
        .iter()
        .find(|text| text.idstring.as_deref() == Some("BOOT_LEGAL_TRADEMARK_FULL_EU"))
        .expect("the footer is a text widget");
    assert_eq!(footer.font, "NEOSANS_BOLD");
    assert!(
        shell.strings.get("BOOT_LEGAL_TRADEMARK_FULL_EU").is_some(),
        "the English table names the footer"
    );
}

/// The grid the file authors is the grid `oag_2048::frontend::TOUCH` names,
/// tile for tile - the compile-time table is an assertion over the parse,
/// not a second source of coordinates.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_parsed_touch_grids_match_the_title_table() {
    let Some(source) = source() else { return };
    let shell = shell(&source);
    let touch = oag_2048::frontend::TOUCH;
    for (name, table) in [
        (w2048::GAME_MODE_CHOICE, touch.game_mode_choice),
        (w2048::HOME, touch.home),
    ] {
        let screen = shell.screens.by_name(name).expect("the grid is loaded");
        let parsed: Vec<(&str, f32, f32, f32, f32)> = screen
            .touch_buttons
            .iter()
            .filter_map(|button| {
                Some((
                    button.idstring.as_deref()?,
                    button.x,
                    button.y,
                    button.width,
                    button.height,
                ))
            })
            .collect();
        let expected: Vec<(&str, f32, f32, f32, f32)> = table
            .iter()
            .map(|b| (b.id, b.x, b.y, b.width, b.height))
            .collect();
        assert_eq!(parsed, expected, "{name}");
        // Plus the unlabelled confirm tick, which the table does not carry.
        let tick = screen
            .touch_buttons
            .iter()
            .find(|button| button.idstring.is_none() && button.redirect.is_some())
            .expect("a confirm tick");
        assert_eq!(tick.redirect.as_deref(), Some(w2048::NEW_FE_SHELL));
        assert_eq!(
            (tick.x, tick.y, tick.width, tick.height),
            (822.0, 432.0, 122.0, 96.0)
        );
        for button in &screen.touch_buttons {
            let src = button.src.as_deref().expect("every button names an icon");
            assert!(
                shell.sprites.get(src).is_some(),
                "{src} decoded into the sheet (the .gtf name resolves to the .gxt entry)"
            );
        }
    }
}

/// From `Boot Connect` to the campaign shell on the pad alone: cross through
/// the movie and the title screen, two crosses on the mode grid.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_pad_walks_from_boot_connect_to_the_campaign_shell() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    assert!(frontend.machine().is(w2048::BOOT_CONNECT));

    // Nothing pressed: the network check and the card's own delay.
    assert!(drive_until(&mut frontend, &[], w2048::BOOT_INTRO_MOVIE));
    // The movie's length is unknown with no demuxer, so it waits.
    assert!(
        !drive_until(&mut frontend, &[], w2048::TITLE_SCREEN),
        "with no button the intro screen never leaves on its own"
    );
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::TITLE_SCREEN
    ));
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::GAME_MODE_CHOICE
    ));
    // First cross chooses the single-player campaign, the second confirms.
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::NEW_FE_SHELL
    ));
    assert_eq!(frontend.chosen_mode(), Some("FE_SP_CAMPAIGN"));
    assert!(!frontend.is_finished(), "no event has been tapped yet");
    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("length is unknown")),
        "the missing demuxer is said, not hidden: {notes:#?}"
    );
}

/// A network mode cannot be confirmed here, and says so.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_network_mode_stays_on_the_grid_with_a_note() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::GAME_MODE_CHOICE
    ));
    frontend.take_notes();
    let mut input = Input::new();
    let step = |frontend: &mut frontend::Frontend, input: &mut Input, button: Button| {
        input.begin_frame(button.bit());
        frontend.update(1.0 / 60.0, input, None);
        input.begin_frame(0);
        frontend.update(1.0 / 60.0, input, None);
    };
    step(&mut frontend, &mut input, Button::Right);
    step(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.chosen_mode(), Some("FE_MP_CAMPAIGN"));
    step(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
    let notes = frontend.take_notes();
    assert!(
        notes
            .iter()
            .any(|note| note.contains("network session this build does not have")),
        "{notes:#?}"
    );
}
