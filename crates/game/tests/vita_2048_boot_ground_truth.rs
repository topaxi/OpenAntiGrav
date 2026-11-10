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

/// From `Boot Connect` to the campaign shell on the pad alone: the movie is
/// skipped (no picture), cross through the title screen, one cross on the mode grid.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_pad_walks_from_boot_connect_to_the_campaign_shell() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    assert!(frontend.machine().is(w2048::BOOT_CONNECT));

    // Nothing pressed: the network check and the card's own delay.
    assert!(drive_until(&mut frontend, &[], w2048::BOOT_INTRO_MOVIE));
    // The intro is 99.57 s (`docs/formats/mp4.md`) and this run decodes no
    // picture, so the screen leaves by itself the way a Start press would
    // rather than sitting black for the length of the film.
    assert!(
        drive_until(&mut frontend, &[], w2048::TITLE_SCREEN),
        "an intro with no picture is skipped with no button"
    );
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::GAME_MODE_CHOICE
    ));
    // One cross chooses the single-player campaign and confirms it.
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

/// Every tile of the base campaign map sits where the original's canvas
/// camera puts it, on the reference save (Events 1 to 3 passed) at the two
/// views of the committed reference frames: the view the map opens on, and
/// the view after a tap on Event 1's tile. The expected centres are the top
/// faces measured in those frames (left and right edge, top edge plus half the
/// face), not this build's own arithmetic
/// (`docs/ghidra/functions/vita-2048-eu-v104/campaign-map-projection.md`,
/// "Evidence"). An even grid of cells puts none of them within 100 px.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_campaign_map_puts_its_tiles_where_the_canvas_camera_does() {
    use oag_ui::frontend::EarnedTier;
    use oag_ui::frontend::canvas_lattice;
    use oag_ui::pointer::Pointer;

    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    frontend.refresh_campaign_progress(|name| {
        matches!(name, "2048 - Event 1" | "2048 - Event 2" | "2048 - Event 3")
            .then_some(EarnedTier::Pass)
    });
    // The original opens on the first event of the chain that is not passed.
    assert!(frontend.select_campaign_event("2048 - Event 4"));
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::NEW_FE_SHELL
    ));
    // Where the hexagon sits in the 128 x 128 hex art: 2..116 across, 12..115 down.
    let centres = |frontend: &frontend::Frontend| -> Vec<(f32, f32)> {
        frontend
            .draw_list()
            .iter()
            .filter_map(|draw| match draw {
                frontend::Draw::Sprite { rect, uv, .. } if uv[2] == 128.0 && uv[3] == 128.0 => {
                    Some((
                        rect[0] + rect[2] * 59.0 / 128.0,
                        rect[1] + rect[3] * 63.5 / 128.0,
                    ))
                }
                _ => None,
            })
            .collect()
    };
    let assert_tiles = |frontend: &frontend::Frontend, view: &str, expected: &[(f32, f32)]| {
        let ours = centres(frontend);
        for want in expected {
            let nearest = ours
                .iter()
                .map(|at| (at.0 - want.0).abs().max((at.1 - want.1).abs()))
                .fold(f32::INFINITY, f32::min);
            assert!(
                nearest <= 4.0,
                "{view}: no tile within 4 px of {want:?} (nearest is {nearest} px off): {ours:?}"
            );
        }
    };
    // Frame 13: Event 3 (green) and Event 4 (grey, the current one).
    assert_tiles(
        &frontend,
        "the opening view",
        &[(412.0, 174.7), (527.0, 268.0)],
    );
    // The canvas lattice is under the tiles: the baked picture, repeated.
    let lattice = frontend
        .draw_list()
        .iter()
        .filter(|draw| {
            matches!(draw, frontend::Draw::Sprite { uv, .. }
                if (uv[2], uv[3])
                    == (canvas_lattice::VIEWPORT[0] as f32, canvas_lattice::VIEWPORT[1] as f32))
        })
        .count();
    assert_eq!(
        lattice, 4,
        "four copies of the lattice picture cover the view"
    );
    // A tap on Event 1's tile, whose lower half shows at the top left.
    assert!(frontend.pointer(&Pointer {
        at: Some((185.0, 20.0)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }));
    assert_eq!(
        frontend.selected_event().map(|event| event.name.as_str()),
        Some("2048 - Event 1")
    );
    // Frame 12: Events 1, 2 and 3 down the diagonal.
    assert_tiles(
        &frontend,
        "Event 1 selected",
        &[(500.0, 269.7), (614.5, 365.7), (729.5, 460.7)],
    );
}

/// The pale triangle lattice is one picture baked off `canvasTexture.gxt` with
/// the tilted camera and laid over the title screen and the game mode page.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_title_and_game_mode_pages_draw_the_baked_canvas_lattice() {
    use oag_ui::frontend::canvas_lattice;
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    let lattice = |frontend: &frontend::Frontend| {
        frontend
            .draw_list()
            .into_iter()
            .filter(|draw| {
                matches!(draw, frontend::Draw::Sprite { uv, .. }
                    if (uv[2], uv[3])
                        == (canvas_lattice::VIEWPORT[0] as f32, canvas_lattice::VIEWPORT[1] as f32))
            })
            .count()
    };
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::TITLE_SCREEN
    ));
    assert_eq!(lattice(&frontend), 1, "one lattice on the title screen");
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::GAME_MODE_CHOICE
    ));
    assert_eq!(lattice(&frontend), 1, "one lattice on the game mode page");
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
    // The v1.04 grid reads SP, HD, FURY across the top, and ONLINE CAMPAIGN is
    // the first tile of the second row. The HD and FURY tiles open a map of
    // their own when their packages are mounted, which this source has:
    // `vita_2048_dlc_menus_ground_truth` holds those.
    for _ in 0..3 {
        step(&mut frontend, &mut input, Button::Right);
    }
    step(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.chosen_mode(), Some("FE_MP_CAMPAIGN"));
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

/// This build's tiles are on the grid, after the authored six, and ask for this build's own pages.
/// A mod's tile under RACEBOX (REMIX) depends on the mods found, and is
/// `boot::tests::a_mods_row_under_racebox_is_a_tile_between_them`'s.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_race_box_tile_follows_the_authored_grid() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    assert!(drive_until(
        &mut frontend,
        &[Button::Cross],
        w2048::GAME_MODE_CHOICE
    ));
    let list = frontend.draw_list();
    assert!(
        list.iter().any(
            |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. } if text == "RACEBOX")
        ),
        "RACEBOX is on the grid"
    );
    // Six tiles (the tick is hidden), then this build's: six rights land on RACEBOX.
    let mut input = Input::new();
    for _ in 0..6 {
        input.begin_frame(Button::Right.bit());
        frontend.update(1.0 / 60.0, &mut input, None);
        input.begin_frame(0);
        frontend.update(1.0 / 60.0, &mut input, None);
    }
    input.begin_frame(Button::Cross.bit());
    frontend.update(1.0 / 60.0, &mut input, None);
    assert_eq!(
        frontend.launch(),
        Some(&oag_ui::frontend::Launch::Page("race".to_string()))
    );
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
        Some(("AG_Systems2048", "1")),
        "one Right off the starting cell (column 0 Feisar, row 0 speed) lands \
         on the second column, AG Systems; one Down on the second row, \
         fighter (suffix \"1\")"
    );
    press_release(&mut frontend, &mut input, Button::Circle);
    assert!(
        frontend.machine().is(w2048::HOME),
        "select_button's own PreviousScreen returns to Home"
    );
}

/// The team screen draws the grid's own art: five team badges and twenty
/// craft icons (`teamgrid/*.gtf`, named by no widget), not labelled tiles.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_team_screen_draws_the_grids_badges_and_craft_icons() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::TEAM));
    let sprites = frontend
        .draw_list()
        .iter()
        .filter(|draw| matches!(draw, frontend::Draw::Sprite { .. }))
        .count();
    assert!(
        sprites >= 25,
        "5 badges + 20 craft icons are sprites of the team grid: {sprites}"
    );
}

/// The Select Team panel's four rows read `<FE>` of the disc's own
/// `PI_TeamModel name="speed"` of `Feisar2048` in `Plugins/teams/Definition.xml` (8, 7, 4, 6): the orange cells
/// of each row and the number at its right, with the two header icons and the
/// two ability icons above them. Dropping the loader or the panel's draw fails
/// this.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_team_panel_draws_feisar_speeds_ratings_off_the_disc() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("team panel: 20 of 20 craft ratings read")),
        "{:#?}",
        loaded.report
    );
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::TEAM));
    let list = frontend.draw_list();
    let orange = |color: &[f32; 4]| color[0] > 0.8 && color[1] < 0.4 && color[3] == 1.0;
    let mut filled = [0usize; 4];
    let mut empty = 0;
    for draw in &list {
        if let frontend::Draw::Fill { rect, color } = draw
            && rect[2] == 22.0
            && rect[3] == 27.0
        {
            let row = ((rect[1] - 307.0) / 30.0) as usize;
            if orange(color) {
                filled[row] += 1;
            } else {
                empty += 1;
            }
        }
    }
    assert_eq!(filled, [8, 7, 4, 6], "orange cells per row");
    assert_eq!(
        empty, 40,
        "all ten cells of a row are drawn grey, the filled ones over them"
    );
    let numbers: Vec<&str> = list
        .iter()
        .filter_map(|draw| match draw {
            frontend::Draw::Text { x, text, .. } if *x == 910.0 => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(numbers, ["8", "7", "4", "6"]);
    let icons = list
        .iter()
        .filter(|draw| {
            matches!(draw, frontend::Draw::Sprite { rect, .. }
                if rect[2] == 64.0 && rect[3] == 64.0 && (rect[1] == 253.0 || rect[1] == 192.0))
        })
        .count();
    assert_eq!(icons, 4, "team, class and the two ability icons");
}

/// Every craft's Select Team hull (`<ship dir>/<team>/<n>/Ship_LOD.vex`, the file
/// the screen's own update names) builds into a hull-sized, textured mesh.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn every_select_team_hull_builds_with_its_textures() {
    let Some(source) = source() else { return };
    let (_shell, mut archives, title) =
        boot::load_shell(&options(&source)).expect("2048's front end walks");
    let roster = title.race.team_variants.expect("2048's native roster");
    let touch = title.front_end.and_then(|front_end| front_end.touch);
    assert_eq!(touch.map(|t| t.team_model_origin), Some((1400.0, 264.0)));
    let mut hulls = 0;
    for team in roster.teams {
        for variant in roster.variants {
            let entry = oag_game::team_hull::entry(title.race.ship_dir, team, variant.suffix);
            let model = oag_game::preview::psp2_scene::model(&mut archives, &entry)
                .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
            assert!(model.indices.len() / 3 > 3_000, "{entry} is a hull");
            assert!(
                model.textures.iter().flatten().count() > 0,
                "{entry} is textured"
            );
            hulls += 1;
        }
    }
    assert_eq!(hulls, 20);
}

/// The RGBA bytes of a PNG this project's own writer made (stored deflate blocks, no
/// filtering), `960 x 544`.
fn pixels_of(png: &[u8]) -> Vec<u8> {
    let mut at = 8;
    let mut zlib = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            zlib.extend_from_slice(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    let mut raw = Vec::new();
    let mut at = 2;
    loop {
        let last = zlib[at] & 1 == 1;
        let len = usize::from(u16::from_le_bytes([zlib[at + 1], zlib[at + 2]]));
        raw.extend_from_slice(&zlib[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    let stride = 1 + 960 * 4;
    assert_eq!(raw.len(), stride * 544, "an RGBA PNG of the frame's size");
    raw.chunks_exact(stride)
        .flat_map(|row| row[1..].iter().copied())
        .collect()
}

/// The headless capture of the Select Team screen carries the hull at the widget's
/// origin, (700, 132) of the 960 x 544 frame: white backdrop there without it.
/// Dropping `capture.rs`'s hull pass or `team_hull` fails this.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/ and a GPU adapter"]
fn the_select_team_capture_draws_the_hull_at_the_widgets_origin() {
    let Some(source) = source() else { return };
    let scratch = std::env::temp_dir().join(format!("oag-team-hull-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("team.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&source)
        .args([
            "--until",
            "team",
            "--press",
            "start,cross,cross,triangle,cross",
        ])
        .args([
            "--ticks",
            "40",
            "--no-audio",
            "--no-video",
            "--size",
            "960x544",
        ])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --until team");
    let pixels = pixels_of(&std::fs::read(&out).expect("reading the screenshot"));
    std::fs::remove_dir_all(&scratch).ok();
    let darkest = (-6i32..=6)
        .flat_map(|dy| (-6i32..=6).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| {
            let at = ((132 + dy) as usize * 960 + (700 + dx) as usize) * 4;
            pixels[at].min(pixels[at + 1]).min(pixels[at + 2])
        })
        .min()
        .expect("a neighbourhood");
    assert!(
        darkest < 200,
        "the hull covers the widget's origin: darkest channel {darkest}"
    );
}

/// `--press-until-reached` leaves the `--until` state settled: the capture of `TitleScreen` shows the
/// logo and the pulsing prompt, not the entry tick's footer on white. Dropping the flag's stop in
/// `capture.rs` leaves the press pulsing, the intro's state moves on or never settles, and the
/// middle of the frame stays white.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/ and a GPU adapter"]
fn the_title_capture_settles_on_the_logo_and_the_prompt() {
    let Some(source) = source() else { return };
    let scratch = std::env::temp_dir().join(format!("oag-title-settled-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("title.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&source)
        .args([
            "--until",
            "TitleScreen",
            "--press",
            "start",
            "--press-until-reached",
        ])
        .args([
            "--ticks",
            "400",
            "--no-audio",
            "--no-video",
            "--size",
            "960x544",
        ])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --until TitleScreen");
    let pixels = pixels_of(&std::fs::read(&out).expect("reading the screenshot"));
    std::fs::remove_dir_all(&scratch).ok();
    let darkest = |x0: usize, x1: usize, y0: usize, y1: usize| {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .map(|(x, y)| {
                let at = (y * 960 + x) * 4;
                pixels[at].min(pixels[at + 1]).min(pixels[at + 2])
            })
            .min()
            .expect("a region")
    };
    // White under the pale lattice, whose lines dip to about 230 once the
    // game's composite encode is applied (the capture's reach 233).
    let backdrop = darkest(0, 960, 0, 100);
    assert!(
        backdrop >= 225,
        "the title's white backdrop, not a race or a loading screen: darkest channel {backdrop}"
    );
    let logo = darkest(130, 830, 130, 290);
    assert!(
        logo < 100,
        "the navy WIPEOUT 2048 logo is drawn: darkest channel {logo}"
    );
    let prompt = darkest(215, 745, 360, 392);
    assert!(
        prompt < 245,
        "the prompt has faded in: darkest channel {prompt}"
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
    // Square (and a tap on the hint under the list) puts it back on what a first run holds.
    frontend.set_option_defaults(oag_game::settings::option_defaults());
    let hint = frontend
        .panel_reset_rect()
        .expect("the camera is off its default, so the hint is shown");
    press_release(&mut frontend, &mut input, Button::Square);
    assert_eq!(
        frontend.camera_choice(),
        Some(0),
        "the default is OPT_CLOSE"
    );
    assert_eq!(frontend.panel_reset_rect(), None, "nothing left to reset");
    press_release(&mut frontend, &mut input, Button::Square);
    assert_eq!(
        frontend.camera_choice(),
        Some(0),
        "a second press is a no-op"
    );
    press_release(&mut frontend, &mut input, Button::Right);
    assert_eq!(frontend.camera_choice(), Some(1));
    let tap = oag_ui::pointer::Pointer {
        at: Some((hint[0] + 2.0, hint[1] + 2.0)),
        moved: true,
        clicked: true,
        ..oag_ui::pointer::Pointer::default()
    };
    frontend.pointer(&tap);
    assert_eq!(frontend.camera_choice(), Some(0), "the tap resets too");
    press_release(&mut frontend, &mut input, Button::Circle);
    assert!(
        frontend.machine().is(w2048::OPTIONS),
        "Circle returns to the hub - chosen, no widget names this button"
    );
}

/// `OptionsCamera`'s second control, SCREEN SIZE: the shared slider on a title whose disc authors
/// none. It reports nothing until moved, Down focuses it, Left steps it one slider step of two
/// below the title's default 100, a tap inside its own rect steps it forward, and Square (or a tap on the
/// hint under it) puts it back on 100. Dropping `Frontend::set_screen_size` leaves the
/// control absent, so the first Left reports nothing.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn options_camera_carries_the_screen_size_control() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let area = loaded
        .title
        .hud_art
        .safe_area
        .expect("2048 offers the slider");
    let mut frontend = loaded.frontend;
    frontend.set_screen_size(area.value, None);
    frontend.set_option_defaults(oag_game::settings::option_defaults());
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    for _ in 0..3 {
        press_release(&mut frontend, &mut input, Button::Right);
    }
    press_release(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::OPTIONS_CAMERA));
    assert_eq!(frontend.screen_size_choice(), None, "untouched so far");
    // Left/Right still belong to the camera list until SCREEN SIZE is focused.
    press_release(&mut frontend, &mut input, Button::Down);
    press_release(&mut frontend, &mut input, Button::Left);
    assert_eq!(
        frontend.screen_size_choice(),
        Some(98),
        "one step of two under 100"
    );
    assert_eq!(frontend.screen_size_scale(), Some(0.98));
    assert_eq!(
        frontend.camera_choice(),
        None,
        "the camera list was not moved"
    );
    let hint = frontend
        .panel_reset_rect()
        .expect("SCREEN SIZE is off its default, so the hint is shown");
    press_release(&mut frontend, &mut input, Button::Square);
    assert_eq!(
        frontend.screen_size_choice(),
        Some(100),
        "reset to the title's default"
    );
    assert_eq!(frontend.panel_reset_rect(), None, "nothing left to reset");
    press_release(&mut frontend, &mut input, Button::Left);
    press_release(&mut frontend, &mut input, Button::Left);
    assert_eq!(frontend.screen_size_choice(), Some(96));
    let click = |frontend: &mut oag_ui::frontend::Frontend, at: [f32; 4]| {
        frontend.pointer(&oag_ui::pointer::Pointer {
            at: Some((at[0] + 2.0, at[1] + 2.0)),
            moved: true,
            clicked: true,
            ..oag_ui::pointer::Pointer::default()
        });
    };
    click(&mut frontend, hint);
    assert_eq!(
        frontend.screen_size_choice(),
        Some(100),
        "the tap on the hint resets too"
    );
    press_release(&mut frontend, &mut input, Button::Left);
    let widget = frontend.screen_size_rect().expect("the control has a rect");
    click(&mut frontend, widget);
    assert_eq!(
        frontend.screen_size_choice(),
        Some(100),
        "a tap on the widget steps it forward"
    );
    press_release(&mut frontend, &mut input, Button::Circle);
    assert!(frontend.machine().is(w2048::OPTIONS));
}

/// `OptionsPilot`'s list starts on its authored `FE_NORMAL` and reports a choice only once
/// moved: one Right is `FE_SUPER` (2), the law this build runs; see
/// `crate::main::session::frame` for how each index reaches `[controls] pilot_assist`.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn options_pilot_reports_the_players_choice() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut frontend = loaded.frontend;
    drive_to_home(&mut frontend);
    let mut input = Input::new();
    for _ in 0..3 {
        press_release(&mut frontend, &mut input, Button::Right);
    }
    press_release(&mut frontend, &mut input, Button::Cross);
    press_release(&mut frontend, &mut input, Button::Circle);
    assert!(frontend.machine().is(w2048::OPTIONS));
    // The hub's four buttons, Camera to Pilot, by whichever direction moves its focus.
    let mut reached = false;
    'search: for direction in [Button::Right, Button::Down] {
        for steps in 0..4 {
            for _ in 0..steps {
                press_release(&mut frontend, &mut input, direction);
            }
            press_release(&mut frontend, &mut input, Button::Cross);
            if frontend.machine().is(w2048::OPTIONS_PILOT) {
                reached = true;
                break 'search;
            }
            if !frontend.machine().is(w2048::OPTIONS) {
                press_release(&mut frontend, &mut input, Button::Circle);
            }
        }
    }
    assert!(reached, "OptionsPilot is reachable from the hub");
    assert_eq!(frontend.pilot_choice(), None, "untouched so far");
    press_release(&mut frontend, &mut input, Button::Right);
    assert_eq!(
        frontend.pilot_choice(),
        Some(2),
        "FE_NORMAL (1) + 1 is FE_SUPER"
    );
    press_release(&mut frontend, &mut input, Button::Right);
    assert_eq!(frontend.pilot_choice(), Some(0), "and it wraps to FE_OFF");
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
