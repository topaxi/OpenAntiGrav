//! Wipeout 2048's front end from a controller alone, against the real package:
//! each screen marks its focus from the first frame, a direction moves it,
//! Cross reaches the focused control and Circle backs out.
//! `docs/formats/2048-frontend.md`, "Controller focus".

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
        cache: std::env::temp_dir().join("oag-2048-focus-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    }
}

const ORANGE: [f32; 4] = [221.0 / 255.0, 88.0 / 255.0, 11.0 / 255.0, 1.0];

fn press_release(frontend: &mut frontend::Frontend, input: &mut Input, button: Button) {
    input.begin_frame(button.bit());
    frontend.update(1.0 / 60.0, input, None);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, input, None);
}

fn presses(frontend: &mut frontend::Frontend, input: &mut Input, buttons: &[Button]) {
    for &button in buttons {
        press_release(frontend, input, button);
    }
}

/// Every `Orange2048` fill on screen: what the pad's focus is drawn as.
fn focus(frontend: &frontend::Frontend) -> Vec<[i32; 4]> {
    frontend
        .draw_list()
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, color } if *color == ORANGE => Some(rect.map(|v| v.round() as i32)),
            _ => None,
        })
        .collect()
}

fn on(frontend: &frontend::Frontend, state: &str) {
    assert!(
        frontend.machine().is(state),
        "expected {state}, on {:?}",
        frontend.machine().current()
    );
}

/// `Boot Connect` to `Home` on the pad alone: cross through the movie and the
/// mode grid, triangle on the campaign shell.
fn to_home(frontend: &mut frontend::Frontend, input: &mut Input) {
    for state in [w2048::GAME_MODE_CHOICE, w2048::NEW_FE_SHELL] {
        let mut tick = 0u32;
        while !frontend.machine().is(state) {
            input.begin_frame(if tick.is_multiple_of(2) {
                Button::Cross.bit()
            } else {
                0
            });
            frontend.update(1.0 / 60.0, input, None);
            tick += 1;
            assert!(tick < 3600, "never reached {state}");
        }
    }
    presses(frontend, input, &[Button::Triangle]);
    on(frontend, w2048::HOME);
}

fn loaded(source: &Path) -> (frontend::Frontend, Input) {
    let loaded = boot::load(&options(source)).expect("the whole boot");
    (loaded.frontend, Input::new())
}

/// `GameModeChoice`: marked on entry, Right moves it, Cross on the focused
/// tile reaches the campaign it names.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_mode_grid_marks_and_moves_its_focus_and_cross_reaches_the_focused_mode() {
    let Some(source) = source() else { return };
    let (mut frontend, mut input) = loaded(&source);
    let mut tick = 0u32;
    while !frontend.machine().is(w2048::GAME_MODE_CHOICE) {
        input.begin_frame(if tick.is_multiple_of(2) {
            Button::Cross.bit()
        } else {
            0
        });
        frontend.update(1.0 / 60.0, &mut input, None);
        tick += 1;
        assert!(tick < 3600);
    }
    let first = focus(&frontend);
    assert_eq!(first.len(), 1, "one tile marked on entry: {first:?}");
    press_release(&mut frontend, &mut input, Button::Right);
    let second = focus(&frontend);
    assert_eq!(second.len(), 1);
    assert_ne!(first, second, "Right moves the mark");
    press_release(&mut frontend, &mut input, Button::Left);
    assert_eq!(focus(&frontend), first);
    press_release(&mut frontend, &mut input, Button::Circle);
    on(&frontend, w2048::GAME_MODE_CHOICE);
    press_release(&mut frontend, &mut input, Button::Cross);
    on(&frontend, w2048::NEW_FE_SHELL);
}

/// `Home`'s five tiles and its tick: each is reached by Right, marked,
/// activated by Cross on the destination it names, and Circle returns.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn home_reaches_each_destination_from_the_focused_tile_and_circle_comes_back() {
    let Some(source) = source() else { return };
    let (mut frontend, mut input) = loaded(&source);
    to_home(&mut frontend, &mut input);
    assert_eq!(
        focus(&frontend),
        vec![[86, 190, 140, 140]],
        "the first tile on entry"
    );
    let destinations = [
        w2048::TEAM,
        w2048::COMMUNITY_ADHOC_CHECK,
        w2048::PROFILE,
        w2048::OPTIONS_CAMERA,
        w2048::EXTRAS,
    ];
    let tiles = [86, 248, 410, 572, 734];
    for (index, destination) in destinations.into_iter().enumerate() {
        for _ in 0..index {
            press_release(&mut frontend, &mut input, Button::Right);
        }
        assert_eq!(
            focus(&frontend),
            vec![[tiles[index], 190, 140, 140]],
            "tile {index} is the only one marked"
        );
        press_release(&mut frontend, &mut input, Button::Cross);
        on(&frontend, destination);
        assert!(
            !focus(&frontend).is_empty(),
            "{destination} shows a focus on entry"
        );
        let mut back = 0;
        while !frontend.machine().is(w2048::HOME) && back < 4 {
            press_release(&mut frontend, &mut input, Button::Circle);
            back += 1;
        }
        let mut settle = 0;
        while !frontend.machine().is(w2048::HOME) && settle < 60 {
            input.begin_frame(0);
            frontend.update(1.0 / 60.0, &mut input, None);
            settle += 1;
        }
        on(&frontend, w2048::HOME);
    }
    // Left from the first tile wraps to the tick, which Cross leaves by.
    presses(&mut frontend, &mut input, &[Button::Left, Button::Cross]);
    on(&frontend, w2048::NEW_FE_SHELL);
}

/// The two-deep screens: Extras' Manual and Credits, Profile's stats arrow.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_sub_screens_mark_their_focus_and_back_out_one_step() {
    let Some(source) = source() else { return };
    let (mut frontend, mut input) = loaded(&source);
    to_home(&mut frontend, &mut input);
    presses(&mut frontend, &mut input, &[Button::Right; 4]);
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::EXTRAS);
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::EXTRAS_MANUAL);
    assert_eq!(focus(&frontend), vec![[822, 432, 122, 96]], "the back tick");
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::EXTRAS);
    let mut steps = 0;
    while focus(&frontend) != vec![[572, 190, 140, 140]] {
        presses(&mut frontend, &mut input, &[Button::Right]);
        steps += 1;
        assert!(steps < 8, "the credits tile is reached by Right");
    }
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::EXTRAS_CREDITS);
    presses(&mut frontend, &mut input, &[Button::Circle]);
    on(&frontend, w2048::EXTRAS);
    presses(&mut frontend, &mut input, &[Button::Circle]);
    on(&frontend, w2048::HOME);
    presses(
        &mut frontend,
        &mut input,
        &[Button::Right, Button::Right, Button::Cross],
    );
    on(&frontend, w2048::PROFILE);
    assert_eq!(focus(&frontend), vec![[4, 225, 64, 64]], "the stats arrow");
    presses(&mut frontend, &mut input, &[Button::Right]);
    assert_eq!(
        focus(&frontend),
        vec![[892, 225, 64, 64]],
        "Right moves to the other arrow"
    );
    presses(&mut frontend, &mut input, &[Button::Left, Button::Cross]);
    on(&frontend, w2048::PROFILE_STATS);
    presses(&mut frontend, &mut input, &[Button::Circle]);
    on(&frontend, w2048::PROFILE);
}

/// `team`: the cursor cell is the mark, a direction moves it, Cross presses the tick.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_team_grid_moves_its_cursor_and_cross_confirms() {
    let Some(source) = source() else { return };
    let (mut frontend, mut input) = loaded(&source);
    to_home(&mut frontend, &mut input);
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::TEAM);
    let start = focus(&frontend);
    assert!(!start.is_empty());
    presses(&mut frontend, &mut input, &[Button::Right, Button::Down]);
    assert_ne!(focus(&frontend), start, "a direction moves the cursor cell");
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::HOME);
}

/// The options hub and its four panels.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_options_panels_mark_the_selected_entry_and_circle_returns_to_the_hub() {
    let Some(source) = source() else { return };
    let (mut frontend, mut input) = loaded(&source);
    to_home(&mut frontend, &mut input);
    presses(&mut frontend, &mut input, &[Button::Right; 3]);
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::OPTIONS_CAMERA);
    let before = focus(&frontend);
    assert!(!before.is_empty());
    presses(&mut frontend, &mut input, &[Button::Right]);
    assert_ne!(focus(&frontend), before, "the list's mark follows the step");
    presses(&mut frontend, &mut input, &[Button::Circle]);
    on(&frontend, w2048::OPTIONS);
    let hub = focus(&frontend);
    assert_eq!(hub.len(), 1, "the hub marks one button: {hub:?}");
    presses(&mut frontend, &mut input, &[Button::Right]);
    assert_ne!(focus(&frontend), hub, "Right moves the hub's focus");
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::OPTIONS_AUDIO);
    presses(&mut frontend, &mut input, &[Button::Circle]);
    on(&frontend, w2048::OPTIONS);
}

/// The campaign map: a direction moves the selected event, Circle and Triangle leave.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_campaign_map_moves_its_selection_and_leaves_on_circle_and_triangle() {
    let Some(source) = source() else { return };
    let (mut frontend, mut input) = loaded(&source);
    let mut tick = 0u32;
    while !frontend.machine().is(w2048::NEW_FE_SHELL) {
        input.begin_frame(if tick.is_multiple_of(2) {
            Button::Cross.bit()
        } else {
            0
        });
        frontend.update(1.0 / 60.0, &mut input, None);
        tick += 1;
        assert!(tick < 3600);
    }
    let first = frontend.selected_event().map(|event| event.name.clone());
    assert!(first.is_some());
    for button in [Button::Right, Button::Down, Button::Left, Button::Up] {
        presses(&mut frontend, &mut input, &[button]);
    }
    presses(&mut frontend, &mut input, &[Button::Right]);
    assert_ne!(frontend.selected_event().map(|e| e.name.clone()), first);
    presses(&mut frontend, &mut input, &[Button::Circle]);
    on(&frontend, w2048::GAME_MODE_CHOICE);
    presses(&mut frontend, &mut input, &[Button::Cross]);
    on(&frontend, w2048::NEW_FE_SHELL);
    presses(&mut frontend, &mut input, &[Button::Triangle]);
    on(&frontend, w2048::HOME);
}
