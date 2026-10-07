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
        audio_cache: oag_source::cache::default_audio_cache_dir(),
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
    // The intro is 99.57 s (`docs/formats/mp4.md`), longer than the 60 s
    // `drive_until` allows, so with nothing pressed it is still playing.
    assert!(
        !drive_until(&mut frontend, &[], w2048::TITLE_SCREEN),
        "with no button the intro screen has not run out inside 60 s"
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
    // `oag_video::mp4` reads the intro's length now, so the screen paces the
    // picture instead of waiting on a button with a note saying it cannot.
    assert!(
        !notes.iter().any(|note| note.contains("length is unknown")),
        "the MP4 demuxer is present, so the intro's length is known: {notes:#?}"
    );
}

/// The campaign map draws the disc's own hex tile art and mode icons, not
/// the flat colour square [`oag_ui::frontend::campaign_map::Frontend::placed`]
/// falls back to when the sheet has none of it.
///
/// **What this pins.** `oag_game::boot::sprites::load` asks for
/// `HEX_FILLED`/`HEX_OUTLINE`/`HEX_SELECT` and the four `EventIcon` names
/// through its own `extra` list (2026-09-27), the same mechanism the menu
/// blocks' nine-patch already used - but `boot::assemble`'s own
/// `Frontend::placements` used to be rebuilt by re-walking every screen's
/// `Image`/`TouchButton` `Src=`, which by construction never named an
/// `extra`-only texture at all. Nothing caught that gap until this test:
/// `sprites::load`'s own report line said "N of N decoded" (true), and
/// `oag-ui`'s unit tests boot with an empty sheet either way (see
/// `crate::frontend::campaign_map::HEX_FILLED`'s own doc comment), so both
/// looked green while the campaign map silently drew flat squares against a
/// full sheet. `assemble` now reads `Frontend::placements` straight off
/// `Sheet::entries`, which is a superset that already includes them; this
/// test is what would have caught the old, narrower rebuild.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_campaign_map_draws_the_discs_own_hex_tiles_not_flat_squares() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::NEW_FE_SHELL
    ));
    let list = frontend.draw_list();
    let hex_tiles = list
        .iter()
        .filter(|draw| matches!(draw, frontend::Draw::Sprite { uv, .. } if uv[2] == 128.0 && uv[3] == 128.0))
        .count();
    assert!(
        hex_tiles >= 2,
        "expected at least a filled and an outline hex sprite per visible \
         marker, found {hex_tiles} 128x128 sprites in {} draw(s): {list:#?}",
        list.len()
    );
    let mode_icons = list
        .iter()
        .filter(|draw| matches!(draw, frontend::Draw::Sprite { uv, .. } if uv[2] == 64.0 && uv[3] == 64.0))
        .count();
    assert!(
        mode_icons >= 1,
        "expected at least one 64x64 mode-icon sprite (race/speed/zone/combat), \
         found {mode_icons} in {} draw(s): {list:#?}",
        list.len()
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
    // The v1.04 grid reads SP, HD, FURY across the top: the HD tile is one right.
    step(&mut frontend, &mut input, Button::Right);
    step(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.chosen_mode(), Some("FE_RC_HD"));
    step(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
    assert!(
        frontend
            .take_notes()
            .iter()
            .any(|note| note.contains("FE_RC_HD is another title's campaign")),
        "an HD tile stays on the grid with its own note"
    );
    // Then FURY, and ONLINE CAMPAIGN is the first tile of the second row.
    step(&mut frontend, &mut input, Button::Right);
    step(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.chosen_mode(), Some("FE_RC_FURY"));
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

/// The map carries `SP.xml`'s own events, opens on the first season's first
/// one, and a press launches it by its own name - the name
/// `oag_raceplay::load_event` resolves.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_press_on_the_map_asks_for_the_first_events_race() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::NEW_FE_SHELL
    ));
    let events = frontend.campaign_events();
    assert_eq!(
        events.len(),
        115,
        "141 instances, 21 with no cell, 5 E3 demos"
    );
    assert!(events.iter().all(|event| !event.name.starts_with("MP_")));
    let first = frontend.selected_event().expect("a selected event");
    assert_eq!(first.name, "2048 - Event 1");
    assert!(
        first
            .detail
            .starts_with("EMPIRE CLIMB / single race / FLASH / 3 laps"),
        "{}",
        first.detail
    );
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::LAUNCH_2048
    ));
    assert!(frontend.is_finished());
    assert_eq!(
        frontend.launch(),
        Some(&oag_ui::frontend::Launch::Event(
            "2048 - Event 1".to_string()
        ))
    );
    // And the name resolves to a race the way `--event` does.
    let race = oag_raceplay::Options {
        source: source.display().to_string(),
        ..oag_raceplay::Options::default()
    };
    let resolved = oag_raceplay::load_event(&race, "2048 - Event 1").expect("the event loads");
    assert_eq!(resolved.setup.class, "FLASH");
}

/// **`M_PPLAYERSHIPMODELDATA` forces the player's own craft on some events -
/// `WOShipCreatorParams` is never authored anywhere in `SP.xml` and is not
/// the mechanism.** `"2048 - Event 4-2"` names `Qirex_Combat`
/// (`M_TEAM="Qirex2048"`, `M_LIVERY="combat"`); `"combat"` is
/// [`oag_2048::race::SHIP_TYPES`]'s own `"fighter"` slot (see that constant's
/// doc comment on the two teams spelling it differently), suffix `"1"`. An
/// ordinary event with no `M_PPLAYERSHIPMODELDATA` (`"2048 - Event 1"`, the
/// test above) leaves the caller's own team choice untouched - `load_event`
/// never reports a forced craft for it. See `docs/formats/2048-campaign.md`'s
/// "Craft choice" section.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn ship_model_data_forces_the_player_craft_on_some_events() {
    let Some(source) = source() else { return };
    let race = oag_raceplay::Options {
        source: source.display().to_string(),
        ..oag_raceplay::Options::default()
    };
    let forced = oag_raceplay::load_event(&race, "2048 - Event 4-2").expect("the event loads");
    assert!(
        forced
            .report
            .iter()
            .any(|line| line.contains(r"forces the player craft: Qirex2048\1")),
        "report: {:?}",
        forced.report
    );

    let open = oag_raceplay::load_event(&race, "2048 - Event 1").expect("the event loads");
    assert!(
        !open
            .report
            .iter()
            .any(|line| line.contains("forces the player craft")),
        "report: {:?}",
        open.report
    );
}

/// This build's two tiles are on the grid, after the authored four and the
/// tick, and ask for this build's own pages.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_race_box_and_remix_tiles_follow_the_authored_grid() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::GAME_MODE_CHOICE
    ));
    let list = frontend.draw_list();
    for label in ["RACEBOX", "REMIX"] {
        assert!(
            list.iter().any(
                |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. } if text == label)
            ),
            "{label} is on the grid"
        );
    }
    // Six tiles, the tick, then the two: seven rights land on RACEBOX.
    let mut input = Input::new();
    for _ in 0..7 {
        input.begin_frame(Button::Right.bit());
        frontend.update(1.0 / 60.0, &mut input, None);
        input.begin_frame(0);
        frontend.update(1.0 / 60.0, &mut input, None);
    }
    input.begin_frame(Button::Cross.bit());
    frontend.update(1.0 / 60.0, &mut input, None);
    assert_eq!(frontend.launch(), Some(&oag_ui::frontend::Launch::RaceBox));
}

/// Presses `button` for one tick and releases it for the next - the same
/// press-and-release pair every test in this file that drives a single
/// action uses.
fn press_release(frontend: &mut frontend::Frontend, input: &mut Input, button: Button) {
    input.begin_frame(button.bit());
    frontend.update(1.0 / 60.0, input, None);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, input, None);
}

/// `Boot Connect` to `Home`: cross through the movie and the mode grid,
/// triangle on the campaign shell.
fn drive_to_home(frontend: &mut frontend::Frontend) {
    assert!(drive_until(
        frontend,
        &[Button::Cross],
        w2048::GAME_MODE_CHOICE
    ));
    assert!(drive_until(frontend, &[Button::Cross], w2048::NEW_FE_SHELL));
    let mut input = Input::new();
    press_release(frontend, &mut input, Button::Triangle);
    assert!(frontend.machine().is(w2048::HOME), "Triangle reaches Home");
}

/// Every one of `Home`'s five destinations is now a real, loaded screen -
/// `includes::FOLLOWED` carries all five files and `wipeout2048::STATES`
/// carries their bare names, closing the "screen this build does not load"
/// gap `2048s-front-end-is-read-and-not-wired.md` recorded for all five.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn homes_five_destinations_are_all_loaded_screens() {
    let Some(source) = source() else { return };
    let shell = shell(&source);
    for name in [
        w2048::TEAM,
        w2048::PROFILE,
        w2048::PROFILE_STATS,
        w2048::OPTIONS,
        w2048::OPTIONS_CAMERA,
        w2048::OPTIONS_AUDIO,
        w2048::OPTIONS_CONTROLS,
        w2048::OPTIONS_PILOT,
        w2048::SAVE_2048_OPTIONS,
        w2048::COMMUNITY_ADHOC_CHECK,
        w2048::EXTRAS,
        w2048::EXTRAS_MANUAL,
        w2048::EXTRAS_CREDITS,
    ] {
        assert!(
            shell.screens.by_name(name).is_some(),
            "{name} is loaded: {:#?}",
            shell.report
        );
    }
    let home = shell.screens.by_name(w2048::HOME).expect("Home is loaded");
    let redirects: Vec<&str> = home
        .touch_buttons
        .iter()
        .filter_map(|button| button.redirect.as_deref())
        .collect();
    assert_eq!(
        redirects,
        vec![
            "team",
            "communityAdhocCheck",
            "profile",
            "OptionsCamera",
            "2048extras",
            "newFEshell",
        ],
        "Home's own five tiles, plus its tick"
    );
}

/// A tap on `Home`'s first tile - `Team`'s own default - reaches `team`; the
/// pad cycles both the team and the craft slot, and Circle leaves through
/// `select_button`'s own `redirect="PreviousScreen"`.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_team_screen_cycles_team_and_craft_and_leaves_the_way_it_came() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::TEAM));
    assert_eq!(
        frontend.team_choice(),
        None,
        "untouched: nothing has been chosen yet"
    );
    press_release(&mut frontend, &mut input, Button::Right);
    press_release(&mut frontend, &mut input, Button::Down);
    assert_eq!(
        frontend.team_choice(),
        Some(("Auricom2048", "4")),
        "one Right off the default team (index 0, AG_Systems2048) lands on \
         Auricom2048 (index 1); one Down off the default craft slot (index \
         2, speed, suffix \"3\") lands on prototype (index 3, suffix \"4\")"
    );
    press_release(&mut frontend, &mut input, Button::Circle);
    assert!(
        frontend.machine().is(w2048::HOME),
        "select_button's own PreviousScreen returns to Home"
    );
}

/// `Home`'s second tile is `FE_COMMUNITY`, which redirects to
/// `communityAdhocCheck` rather than to `community` itself - a real screen
/// this build now loads and draws, refusing by note like `GameModeChoice`'s
/// own network modes.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn community_is_reached_from_home_and_refuses_by_note() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    press_release(&mut frontend, &mut input, Button::Right);
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::COMMUNITY_ADHOC_CHECK));
    let list = frontend.draw_list();
    assert!(
        list.iter().any(
            |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. } if text.contains("Ad-Hoc") || text.contains("Community"))
        ),
        "the disc's own FE_COMMUNITY_UNAVAILABLE_ADHOC text draws: {list:#?}"
    );
    // The screen's own tick redirects to `Home` directly - authored, not the
    // back stack.
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::HOME));
}

/// `Home`'s fourth tile jumps straight to `OptionsCamera`, never to
/// `options` - read directly off `Definition.xml`, not a guess (see
/// `oag_2048::frontend::states::OPTIONS_CAMERA`'s own doc). The camera
/// choice cycles on Left/Right and Circle returns to the hub.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn options_camera_is_reached_directly_from_home_and_the_choice_cycles() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    for _ in 0..3 {
        press_release(&mut frontend, &mut input, Button::Right);
    }
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(
        frontend.machine().is(w2048::OPTIONS_CAMERA),
        "three Rights off Team land on Options, which redirects to OptionsCamera"
    );
    assert_eq!(frontend.camera_choice(), None, "untouched so far");
    press_release(&mut frontend, &mut input, Button::Right);
    assert_eq!(
        frontend.camera_choice(),
        Some(1),
        "one Right off the default index (0, OPT_CLOSE) lands on OPT_FAR (1)"
    );
    press_release(&mut frontend, &mut input, Button::Circle);
    assert!(
        frontend.machine().is(w2048::OPTIONS),
        "Circle returns to the hub - chosen, no widget names this button"
    );
}

/// `Home`'s third tile, `FE_PROFILE`, redirects straight to `profile` -
/// drawn generically, the same path `Community`/`Options` already prove.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn profile_is_reached_from_home() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    for _ in 0..2 {
        press_release(&mut frontend, &mut input, Button::Right);
    }
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::PROFILE));
}

/// `Home`'s fifth tile is `2048extras`, whose own `FE_MANUAL` tile is one
/// tap away at `manual3D`, and that screen's own tick leaves through
/// `redirect="PreviousScreen"` - the back stack `redirect_touch` built on
/// the way in, not `Home` by name.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn extras_and_its_manual_are_reached_from_home_and_the_manual_leaves_the_way_it_came() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    for _ in 0..4 {
        press_release(&mut frontend, &mut input, Button::Right);
    }
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::EXTRAS));
    // `2048extras`' own `<aTouchButton>` (AR Museum) is a typo the disc
    // ships, not a `<TouchButton>` this parser recognises - `Manual` is the
    // first real tile.
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::EXTRAS_MANUAL));
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(
        frontend.machine().is(w2048::EXTRAS),
        "PreviousScreen pops back to 2048extras, not to Home"
    );
}
