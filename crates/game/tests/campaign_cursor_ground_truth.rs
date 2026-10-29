//! The Race Campaign reopens where the player left off, per campaign, through the real session
//! (`oag-game --drive`, no window) on Pulse's disc.
//!
//! Each test is one or two launches of the binary on the **same** profile, so a launch reads what
//! the one before it wrote. The game reports the campaign's level (`screen.id`: `grid` or `cell`)
//! and the name under the cursor (`focus`), so no picture is read.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^campaign_cursor_ground_truth::/)'
//! ```

use std::path::{Path, PathBuf};

use serde_json::Value;

/// A profile directory; the game's config, data and state live under it.
struct Profile(PathBuf);

impl Profile {
    fn fresh(tag: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("campaign-cursor-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }

    /// One launch of Pulse on `script`; the states it reported.
    fn drive(&self, script: &str, log: &str) -> Vec<Value> {
        let image = oag_testdata::image("pulse-psp-eu.chd").expect("pulse-psp-eu.chd");
        self.drive_on(&image, script, log)
    }

    /// One launch of the title at `image` on `script`; the states it reported.
    fn drive_on(&self, image: &Path, script: &str, log: &str) -> Vec<Value> {
        let log = self.0.join(log);
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
            .arg(image)
            .args(["--no-audio", "--no-intro", "--drive", script])
            .arg("--state-log")
            .arg(&log)
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("XDG_DATA_HOME", self.0.join("data"))
            .env("XDG_STATE_HOME", self.0.join("state"))
            .env("OAG_TRACKS", self.0.join("no-tracks"))
            // The repository's example mods sit on the default search path when their submodule
            // is checked out; a mod campaign puts a picker behind RACE CAMPAIGN, and this test is
            // about the disc's own campaign.
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

/// RACE CAMPAIGN is open, on `Grid Selection`.
const OPEN: &str =
    "boot; until stage=menu within 3000; select campaign; press cross; until screen=campaign";

/// The level and the name under the cursor in the last state a run reported.
fn last_campaign(states: &[Value]) -> (String, String) {
    let screen = states
        .iter()
        .rev()
        .find_map(|state| state.get("screen").filter(|s| s["kind"] == "campaign"))
        .expect("the campaign was open");
    (
        screen["id"].as_str().unwrap_or_default().to_string(),
        screen["focus"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    )
}

fn pulse_present() -> bool {
    oag_testdata::image("pulse-psp-eu.chd").is_some()
}

/// Entering `Grid Selection` lands on the grid last left, in the same launch and after a relaunch.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn grid_selection_opens_on_the_last_grid_and_survives_a_relaunch() {
    if !pulse_present() {
        return;
    }
    let profile = Profile::fresh("grid");
    // Two tiles right, back out to the main page, in again: same launch.
    let states = profile.drive(
        &format!(
            "{OPEN}; expect screen.id=grid; expect focus=grid0; press right; press right; \
             expect focus=grid2; press circle; until screen!=campaign; \
             select campaign; press cross; until screen=campaign; ticks 10"
        ),
        "a.jsonl",
    );
    assert_eq!(last_campaign(&states), ("grid".into(), "grid2".into()));
    // A new process, the same profile.
    let states = profile.drive(&format!("{OPEN}; ticks 10"), "b.jsonl");
    assert_eq!(
        last_campaign(&states),
        ("grid".into(), "grid2".into()),
        "the grid survives quitting and relaunching"
    );
}

/// Opening the grid last left lands on the cell last left, in the same launch and after a relaunch.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn cell_selection_opens_on_the_last_cell_and_survives_a_relaunch() {
    if !pulse_present() {
        return;
    }
    let profile = Profile::fresh("cell");
    let states = profile.drive(
        &format!(
            "{OPEN}; press cross; until screen.id=cell; press down; \
             press circle; until screen.id=grid; expect focus=grid0; \
             press cross; until screen.id=cell; ticks 10"
        ),
        "a.jsonl",
    );
    let (level, moved_to) = last_campaign(&states);
    assert_eq!(level, "cell");
    // The default cell is not where the cursor was left.
    let first = states
        .iter()
        .find_map(|state| {
            let screen = state.get("screen")?;
            (screen["id"] == "cell").then(|| screen["focus"]["id"].as_str().map(String::from))
        })
        .flatten()
        .expect("Cell Selection opened");
    assert_ne!(first, moved_to, "the cursor moved off the default cell");
    // The process ended with Cell Selection open; a new one lands on it.
    let states = profile.drive(
        &format!("{OPEN}; press cross; until screen.id=cell; ticks 10"),
        "b.jsonl",
    );
    assert_eq!(
        last_campaign(&states),
        ("cell".into(), moved_to),
        "the cell survives quitting and relaunching"
    );
}

/// The position is filed under its campaign in the records file.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_position_is_filed_in_the_records_file() {
    if !pulse_present() {
        return;
    }
    let profile = Profile::fresh("file");
    profile.drive(
        &format!("{OPEN}; press right; press circle; until screen!=campaign"),
        "a.jsonl",
    );
    let text = std::fs::read_to_string(profile.0.join("config/oag/records.toml"))
        .expect("the records file");
    assert!(
        text.contains("[[campaign_cursor]]") && text.contains("grid = \"grid1\""),
        "{text}"
    );
}

/// The first cell `Cell Selection` reported, and the last: the cursor moved between them.
fn first_and_last_cell(states: &[Value]) -> (String, String) {
    let first = states
        .iter()
        .find_map(|state| {
            let screen = state.get("screen")?;
            (screen["id"] == "cell").then(|| screen["focus"]["id"].as_str().map(String::from))
        })
        .flatten()
        .expect("Cell Selection opened");
    (first, last_campaign(states).1)
}

/// Back from a race lands on the cell raced - through the real path (launch, Escape, which is
/// the pause menu's QUIT RACE) - and so does a relaunch after a launch.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_cell_raced_is_the_one_the_campaign_reopens_on() {
    if !pulse_present() {
        return;
    }
    let profile = Profile::fresh("race");
    let states = profile.drive(
        &format!(
            "{OPEN}; press cross; until screen.id=cell; press down; ticks 5; \
             press cross; until screen=picker; press cross; until stage=race within 3000; ticks 10; \
             escape; until screen.id=cell within 600; ticks 10"
        ),
        "a.jsonl",
    );
    let (default, back) = first_and_last_cell(&states);
    // The cell under the cursor when the launch left the menus.
    let launch = states
        .iter()
        .position(|state| state["stage"] == "race")
        .expect("the race launched");
    let raced = last_campaign(&states[..launch]).1;
    assert_ne!(raced, default, "the launch was off the default cell");
    assert_eq!(back, raced, "back from the race lands on the cell raced");
    // Leaving the race saved its result off the frame thread; the file has it, beside the cursor
    // the launch filed before it (two saves in a row, neither lost).
    let text = std::fs::read_to_string(profile.0.join("config/oag/records.toml"))
        .expect("the records file");
    assert!(
        text.contains("[[records]]") && text.contains("[[campaign_cursor]]"),
        "{text}"
    );
    // Escape ended the process on the cell; a relaunch lands on it too.
    let states = profile.drive(
        &format!("{OPEN}; press cross; until screen.id=cell; ticks 10"),
        "b.jsonl",
    );
    assert_eq!(last_campaign(&states), ("cell".into(), back));
}

/// The Single Player chain's screen id and the name under the cursor in the last state it
/// reported.
fn last_chain(states: &[Value]) -> (String, String) {
    let screen = states
        .iter()
        .rev()
        .find_map(|state| state.get("screen").filter(|s| s["kind"] == "pure-chain"))
        .expect("the chain was open");
    (
        screen["id"].as_str().unwrap_or_default().to_string(),
        screen["focus"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    )
}

/// Pure's SINGLE PLAYER chain reopens on the event last started, after a relaunch.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn pure_single_player_reopens_on_the_event_last_started() {
    let Some(image) = oag_testdata::image("pure-psp-eu.chd") else {
        return;
    };
    let profile = Profile::fresh("pure");
    let open = "boot; until stage=menu within 3000; select campaign; press cross; \
                until screen=pure-chain";
    // SINGLE RACE, class, league CLASSIC, the second track, then the ship screen starts it.
    let states = profile.drive_on(
        &image,
        &format!(
            "{open}; press cross; until screen.id=class; press cross; until screen.id=league; \
             press down; press down; press cross; until screen.id=track; press down; ticks 3; \
             press cross; until screen=picker; ticks 5"
        ),
        "a.jsonl",
    );
    let (level, raced) = last_chain(&states);
    assert_eq!(level, "track");
    assert_ne!(raced, "KARBONIS", "the cursor was off the first track");
    let states = profile.drive_on(&image, &format!("{open}; ticks 10"), "b.jsonl");
    assert_eq!(
        last_chain(&states),
        ("track".into(), raced),
        "the chain reopens on the track raced after a relaunch"
    );
}

/// 2048's campaign map opens with the cursor on the event last started, after a relaunch.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_map_reopens_on_the_event_last_started() {
    let Some(image) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let profile = Profile::fresh("map2048");
    // Event 1 passed, so Event 2 is open; a fresh profile locks every event but the first.
    let mut store = oag_game::records::Store::default();
    store.record_campaign(
        "wipeout 2048",
        "2048 - Event 1",
        Some(oag_game::records::Medal::Bronze),
        None,
    );
    let records = profile.0.join("config/oag/records.toml");
    std::fs::create_dir_all(records.parent().expect("a parent")).expect("a config directory");
    std::fs::write(
        &records,
        toml::to_string_pretty(&store).expect("serialises"),
    )
    .expect("a seeded records file");
    let to_map = "until frontend=\"Boot Intro Movie\" within 3000; ticks 20; press start; \
                  until frontend=TitleScreen within 3000; press start; \
                  until frontend=GameModeChoice within 3000; press cross; \
                  until frontend=newFEshell within 3000; ticks 60";
    let map_event = |states: &[Value]| {
        states
            .iter()
            .rev()
            .find_map(|state| state["map_event"].as_str().map(String::from))
            .expect("the map reported its cursor")
    };
    // Right to Event 2; its card opens on cross and cross launches it.
    let states = profile.drive_on(
        &image,
        &format!(
            "{to_map}; expect map_event=\"2048 - Event 1\"; press right; ticks 40; \
                  expect map_event=\"2048 - Event 2\"; press cross; ticks 60; press cross; \
                  until stage=loading within 600"
        ),
        "a.jsonl",
    );
    assert_eq!(map_event(&states), "2048 - Event 2");
    let states = profile.drive_on(&image, &format!("{to_map}; ticks 10"), "b.jsonl");
    assert_eq!(
        map_event(&states),
        "2048 - Event 2",
        "the map reopens on the event started after a relaunch"
    );
}
