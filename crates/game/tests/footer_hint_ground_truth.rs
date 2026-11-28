//! A click on a key or button hint presses that button, through the real session
//! (`oag-game --drive`, no window) on real discs.
//!
//! `click hint <button>` draws a frame, finds the hint on screen that presses the button, moves
//! the mouse to the middle of it and clicks (`oag_game::drive::script`). The game reports the
//! buttons its hints press as the state key `hints`, so a script can say what is on screen before
//! it clicks. Each test clicks and then reads what the screen did: Back leaves, Help opens Help,
//! Confirm confirms. A hint that is not under the pointer, or a click that is not routed to the
//! button, leaves the screen where it was, which the `expect` after it catches.
//!
//! The glyph a footer draws is the button the click presses: on Pulse's PS2 the disc authors
//! `Back` on the triangle and the player's MENU BACK choice may put it on the circle, in which
//! case the footer draws the glyphs exchanged and so does the click
//! (`docs/ui/back-and-help.md`, "Footers").
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^footer_hint_ground_truth::/)'
//! ```

use std::path::{Path, PathBuf};

use serde_json::Value;

/// A profile directory; the game's config, data and state live under it.
struct Profile(oag_testdata::scratch::TempDir);

impl Profile {
    /// A new, empty profile owned by this test process alone, so two tests sharing a `tag` do
    /// not wipe each other's logs.
    fn fresh(tag: &str) -> Self {
        let dir = oag_testdata::scratch::dir(&format!("footer-hint-{tag}"));
        Self(dir)
    }

    /// Writes `[controls] menu_back = <choice>` into this profile's settings.
    fn menu_back(&self, choice: &str) {
        let dir = self.0.join("config/oag");
        std::fs::create_dir_all(&dir).expect("a config directory");
        std::fs::write(
            dir.join("settings.toml"),
            format!("[controls]\nmenu_back = \"{choice}\"\n"),
        )
        .expect("settings.toml");
    }

    /// One launch of the title at `image` on `script`, with `extra` arguments; the states it
    /// reported.
    fn drive(&self, image: &Path, extra: &[&str], script: &str, log: &str) -> Vec<Value> {
        let log = self.0.join(log);
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
            .arg(image)
            .args(extra)
            .args([
                "--no-audio",
                "--no-soundtrack",
                "--no-effects",
                "--no-intro",
                "--drive",
                script,
            ])
            .arg("--state-log")
            .arg(&log)
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("XDG_DATA_HOME", self.0.join("data"))
            .env("XDG_STATE_HOME", self.0.join("state"))
            .env("OAG_TRACKS", self.0.join("no-tracks"))
            // The repository's example mods put a picker behind RACE CAMPAIGN and a REMIX row in
            // the racebox; these tests are about the disc's own screens.
            .env("OAG_MODS", self.0.join("no-mods"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .output()
            .expect("running oag-game");
        assert!(
            output.status.success(),
            "{:?}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).expect("a state line is JSON"))
            .collect()
    }
}

fn ps2() -> Option<PathBuf> {
    oag_testdata::image("pulse-ps2-eu.chd")
}

fn psp() -> Option<PathBuf> {
    oag_testdata::image("pulse-psp-eu.chd")
}

/// `(back, help)`: the face buttons Back and Help are on under `choice` on a pressing whose own
/// Back is `original` and whose Help is on the other face button.
fn buttons(original: &'static str, choice: &str) -> (&'static str, &'static str) {
    let other = |button: &'static str| {
        if button == "circle" {
            "triangle"
        } else {
            "circle"
        }
    };
    match choice {
        "circle" => ("circle", "triangle"),
        "triangle" => ("triangle", "circle"),
        _ => (original, other(original)),
    }
}

const TRACK_SELECT: &str = "boot; select racebox; press cross; until page=race; \
     select launch_race; press cross; until screen=picker; ticks 30";
/// The last steps of [`TRACK_SELECT`], from the race page Back returns to.
const TRACK_SELECT_AGAIN: &str = "select launch_race; press cross; until screen=picker; ticks 30";
const SHIP_SELECT: &str = "boot; select racebox; press cross; until page=race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; ticks 30";
/// The last steps of [`SHIP_SELECT`], from the Track Select Back returns to.
const SHIP_SELECT_AGAIN: &str = "press cross; until screen.id=ship-select; ticks 30";
const RACE_PAGE: &str = "boot; select racebox; press cross; until page=race; ticks 30";

/// Pulse's Track Select, under every MENU BACK choice: the hint that depicts Help opens it, the
/// one that depicts Back leaves, and each is the button the choice put there. One launch for all
/// three: `config` switches the choice and the race page's LAUNCH RACE opens Track Select again.
fn track_select_hints(image: &Path, original: &'static str, tag: &str) {
    let profile = Profile::fresh(tag);
    profile.menu_back("original");
    let mut script = String::new();
    for (number, choice) in ["original", "circle", "triangle"].into_iter().enumerate() {
        let (back, help) = buttons(original, choice);
        println!("Track Select, MENU BACK {choice}: {help} opens Help, {back} leaves");
        script.push_str(&format!(
            "{onto}; draw; expect hints={back}; expect hints={help}; expect hints=cross; \
             click hint {help}; ticks 10; expect screen.help=open; expect screen.id=track-select; \
             press {help}; ticks 10; expect screen.help=closed; \
             click hint {back}; ticks 30; expect screen.id!=track-select; ",
            onto = if number == 0 {
                TRACK_SELECT.to_string()
            } else {
                format!("config controls.menu_back={choice}; {TRACK_SELECT_AGAIN}")
            }
        ));
    }
    profile.drive(image, &[], &script, "folded.jsonl");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_track_select_hints_follow_menu_back() {
    let Some(image) = ps2() else { return };
    track_select_hints(&image, "triangle", "ps2-track");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_track_select_hints_follow_menu_back() {
    let Some(image) = psp() else { return };
    track_select_hints(&image, "circle", "psp-track");
}

/// Ship Select authors a Help of its own, which the hint opens on the button the choice put it on.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_ship_select_hints_follow_menu_back() {
    let Some(image) = ps2() else { return };
    let profile = Profile::fresh("ps2-ship");
    profile.menu_back("original");
    let mut script = String::new();
    for (number, choice) in ["original", "circle"].into_iter().enumerate() {
        let (back, help) = buttons("triangle", choice);
        println!("Ship Select, MENU BACK {choice}: {help} opens Team Help, {back} leaves");
        script.push_str(&format!(
            "{onto}; draw; expect hints={back}; expect hints={help}; \
             click hint {help}; ticks 10; expect screen.help=open; expect screen.id=ship-select; \
             press {help}; ticks 10; expect screen.help=closed; \
             click hint {back}; ticks 30; expect screen.id!=ship-select; ",
            onto = if number == 0 {
                SHIP_SELECT.to_string()
            } else {
                format!("config controls.menu_back={choice}; {SHIP_SELECT_AGAIN}")
            }
        ));
    }
    profile.drive(&image, &[], &script, "folded.jsonl");
}

/// Confirm on a picker's footer is the cross: it takes the track to Ship Select, and on Ship
/// Select it picks the team and starts the race.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_confirm_hint_confirms_on_both_pickers() {
    let Some(image) = ps2() else { return };
    let profile = Profile::fresh("ps2-confirm");
    profile.drive(
        &image,
        &[],
        &format!(
            "{TRACK_SELECT}; click hint cross; until screen.id=ship-select within 300; ticks 30; \
             click hint cross; until stage=race within 4000"
        ),
        "confirm.jsonl",
    );
}

/// The settings pages and menu pages draw `Confirm` and, past the root, `Back`: Back leaves the
/// racebox page for the main page, Confirm on the main page opens the row under the cursor.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_menu_page_hints_confirm_and_leave() {
    let Some(image) = ps2() else { return };
    let profile = Profile::fresh("ps2-page");
    profile.menu_back("original");
    let mut script = String::new();
    for (number, choice) in ["original", "circle"].into_iter().enumerate() {
        let (back, _) = buttons("triangle", choice);
        script.push_str(&format!(
            "{onto}; select racebox; ticks 30; \
             draw; expect hints=cross; click hint cross; until page=race; ticks 30; \
             draw; expect hints={back}; click hint {back}; until page=main; expect page=main; ",
            onto = if number == 0 {
                "boot; until stage=menu within 3000".to_string()
            } else {
                format!("config controls.menu_back={choice}")
            }
        ));
    }
    profile.drive(&image, &[], &script, "folded.jsonl");
}

/// The footer of the racebox page, on a title with Help authored nowhere: the Back hint leaves
/// the page on the PSP's circle.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_race_page_back_hint_leaves() {
    let Some(image) = psp() else { return };
    let profile = Profile::fresh("psp-page");
    profile.drive(
        &image,
        &[],
        &format!("{RACE_PAGE}; click hint circle; ticks 30; expect page!=race"),
        "back.jsonl",
    );
}

/// Cell Selection's footer carries Help, AI difficulty, Confirm and Back: Help opens Cell Help
/// on the button the pressing and the choice put it on (with it open a direction moves nothing),
/// and Back then leaves for Grid Selection. One launch for both choices: `config` switches the
/// choice on Grid Selection, and the cursor is put back where it started, since a campaign reopens
/// where the cursor was left.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_cell_selection_hints_follow_menu_back() {
    let Some(image) = ps2() else { return };
    let profile = Profile::fresh("ps2-cell");
    profile.menu_back("original");
    let mut script = String::new();
    for (number, choice) in ["original", "circle"].into_iter().enumerate() {
        let (back, help) = buttons("triangle", choice);
        println!("Cell Selection, MENU BACK {choice}: {help} opens Cell Help, {back} leaves");
        script.push_str(&format!(
            "{onto}; ticks 60; draw; expect hints={help}; expect hints=square; \
             click hint {help}; ticks 20; press down; ticks 20; expect focus=grid0_3_1; \
             press {help}; ticks 20; press down; ticks 20; expect focus!=grid0_3_1; \
             press up; ticks 20; expect focus=grid0_3_1; \
             click hint {back}; until screen.id=grid within 100; ",
            onto = if number == 0 {
                "boot; until stage=menu within 3000; select campaign; press cross; \
                 until screen=campaign; press cross; until screen.id=cell"
                    .to_string()
            } else {
                format!("config controls.menu_back={choice}; press cross; until screen.id=cell")
            }
        ));
    }
    profile.drive(&image, &[], &script, "folded.jsonl");
}

/// The pause menu's root draws `Confirm` alone (Back is silent there): clicking it takes the row
/// under the cursor, CONTINUE, and the race resumes. A page under it draws `Back`, which leaves
/// for the root.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_pause_menu_hints_confirm_and_leave() {
    let Some(image) = ps2() else { return };
    let profile = Profile::fresh("ps2-pause");
    let to_the_pause = "boot; select racebox; press cross; until page=race; \
         set race.mode=time_trial; select launch_race; press cross; until screen=picker; \
         press cross; until screen.id=ship-select; press cross; until stage=race within 4000; \
         ticks 400; press start; until stage=menu; expect page=pause; ticks 40";
    profile.drive(
        &image,
        &[],
        &format!(
            "{to_the_pause}; select \"Game options\"; click hint cross; until page=options; ticks 40; \
             click hint triangle; until page=pause; ticks 40; select continue; \
             click hint cross; until stage=race within 300"
        ),
        "pause.jsonl",
    );
}

/// `EndRace Menu` draws `Confirm`: a click on it takes the option under the cursor, RETURN TO
/// MENUS, and the race is left. The player is flown for it, as `photo_mode_ground_truth` does.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_end_of_race_menu_confirm_hint_confirms() {
    let Some(image) = psp() else { return };
    let profile = Profile::fresh("psp-endrace");
    profile.drive(
        &image,
        &["--autopilot"],
        "boot; select racebox; press cross; until page=race; select launch_race; \
         press cross; until screen=picker; press cross; until screen.id=ship-select; \
         press cross; until stage=race within 4000; ticks 400; \
         ticks 8000; until endrace=photo within 2000; ticks 100; press cross; ticks 100; \
         press cross; ticks 100; draw; expect hints=cross; click hint cross; \
         until stage=menu within 300",
        "endrace.jsonl",
    );
}

/// Photo mode's key legend, on Pulse's PS2: `Back` is the triangle under `original` and the
/// circle under MENU BACK `circle`, and the camera reset is the button Back displaced. A click on
/// the reset hint keeps the mode open, a click on the Back hint leaves it. One race for both
/// choices: leaving photo mode returns to it, and `config` switches the choice there.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU"]
fn pulse_ps2_photo_mode_hints_follow_menu_back() {
    let Some(image) = ps2() else { return };
    let profile = Profile::fresh("ps2-photo");
    profile.menu_back("original");
    let mut script = String::from(
        "boot; select racebox; press cross; until page=race; select launch_race; \
         press cross; until screen=picker; press cross; until screen.id=ship-select; \
         press cross; until stage=race within 4000; ticks 400; ",
    );
    for (number, choice) in ["original", "circle"].into_iter().enumerate() {
        let (back, help) = buttons("triangle", choice);
        println!("Photo mode, MENU BACK {choice}: {back} leaves, {help} resets the camera");
        if number > 0 {
            script.push_str(&format!("config controls.menu_back={choice}; "));
        }
        script.push_str(&format!(
            "escape; until stage=menu; select photo_mode; press cross; \
             until photo=free within 200; ticks 5; draw; expect hints={back}; expect hints={help}; \
             expect hints=cross; click hint {help}; ticks 10; expect photo=free; \
             click hint cross; ticks 10; expect photo=free; \
             click hint {back}; until photo=none within 50; "
        ));
    }
    profile.drive(&image, &[], &script, "folded.jsonl");
}

/// The replay viewer's key legend: a click on `CROSS PLAY` pauses the recording that is playing,
/// and a second resumes it.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn pulse_psp_replay_viewer_play_hint_toggles_playback() {
    let Some(image) = psp() else { return };
    let profile = Profile::fresh("psp-viewer");
    profile.drive(
        &image,
        &[],
        "boot; select racebox; press cross; until page=race; select launch_race; \
         press cross; until screen=picker; press cross; until screen.id=ship-select; \
         press cross; until stage=race within 4000; ticks 400; escape; until stage=menu; quit",
        "record.jsonl",
    );
    profile.drive(
        &image,
        &[],
        "boot; select replays; press cross; until page=replays; select watch_replay; \
         press cross; until replay=playing within 4000; ticks 30; \
         draw; expect hints=cross; expect hints=square; \
         click hint cross; ticks 10; expect replay=paused; \
         click hint cross; ticks 10; expect replay=playing",
        "viewer.jsonl",
    );
}

/// Wipeout HD's Track Select footer: the Back hint leaves, as the circle does there, and the
/// d-pad's `NAVIGATION` glyph is not a hint.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_track_select_hints_confirm_and_leave() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let profile = Profile::fresh("hd-track");
    let state = profile.drive(
        &image,
        &[],
        &format!(
            "{TRACK_SELECT}; ticks 60; draw; expect hints=circle; expect hints=cross; \
             click hint circle; ticks 60; expect screen.id!=track-select"
        ),
        "back.jsonl",
    );
    let hints = state
        .iter()
        .filter_map(|state| state["hints"].as_str())
        .find(|hints| hints.contains("circle"))
        .expect("the footer's hints were reported");
    assert!(
        !hints.contains("up") && !hints.contains("down") && !hints.contains("left"),
        "the d-pad glyph is four controls, not a hint: {hints}"
    );
}

/// Pure's PSP executable binds `backward` and `cancel` to the triangle
/// (`docs/ghidra/functions/psp-pure-usa/input.md`), and its footers draw the triangle beside `BACK`
/// in the `Stats` face. Back leaves on the triangle and not the circle, the footer's `BACK` hint
/// leaves, and under MENU BACK `circle` the footer draws the circle and the hint presses it. One
/// launch for both: `config` switches the choice where Back left the player.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn pure_back_is_the_triangle_and_its_footer_hint_presses_the_resolved_button() {
    let Some(image) = oag_testdata::image("pure-psp-eu.chd") else {
        return;
    };
    let profile = Profile::fresh("pure");
    profile.menu_back("original");
    let mut script = String::new();
    for (number, choice) in ["original", "circle"].into_iter().enumerate() {
        let (back, other) = buttons("triangle", choice);
        println!("Pure race page, MENU BACK {choice}: {back} leaves, {other} does not");
        script.push_str(&format!(
            "{onto}; select racebox; press cross; until page=race; ticks 60; \
             press {other}; ticks 40; expect page=race; draw; expect hints={back}; \
             expect hints=cross; click hint {back}; ticks 40; expect page!=race; ",
            onto = if number == 0 {
                "boot; until stage=menu within 3000".to_string()
            } else {
                format!("config controls.menu_back={choice}")
            }
        ));
    }
    profile.drive(&image, &[], &script, "folded.jsonl");
}

/// The name prompt's hint line (Pulse's own `TagInput` row) is a click target: `CROSS/START
/// ACCEPT` is the cross and the Back word cancels, on the Back button the pressing resolved to.
/// (The keyboard grid's own line, which adds square and start, is
/// `app::overlay::tests::a_keyboard_prompt_offers_its_hint_line_as_clicks`.)
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and pulse-ps2-eu.chd"]
fn the_name_prompts_hint_line_accepts_and_cancels() {
    for (image, original, tag) in [
        (psp(), "circle", "name-psp"),
        (ps2(), "triangle", "name-ps2"),
    ] {
        let Some(image) = image else {
            return;
        };
        let (back, _) = buttons(original, "original");
        let profile = Profile::fresh(tag);
        profile.drive(
            &image,
            &[],
            &format!(
                "boot; until stage=menu within 3000; select options; press cross; ticks 30; \
                 select edit_player_name; press cross; ticks 30; draw; expect screen=prompt; \
                 expect hints={back}; expect hints=cross; click hint {back}; ticks 30; \
                 expect screen=page"
            ),
            "name.jsonl",
        );
    }
}
