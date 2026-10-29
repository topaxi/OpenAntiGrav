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

/// Launching a cell files the cursor on it, so what comes back from the race - or a relaunch after
/// it - opens on the cell raced. (`--drive` cannot press escape or finish a race, so the way back
/// itself, `RETURN TO GRID`, is `Session::reopen_cell_selection` and is not driven here.)
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_launched_cell_is_the_one_the_campaign_reopens_on() {
    if !pulse_present() {
        return;
    }
    let profile = Profile::fresh("race");
    let states = profile.drive(
        &format!(
            "{OPEN}; press cross; until screen.id=cell; press down; ticks 5; \
             press cross; until screen=picker; press cross; until stage=race within 3000; ticks 10"
        ),
        "a.jsonl",
    );
    let raced = last_campaign(&states).1;
    let states = profile.drive(
        &format!("{OPEN}; press cross; until screen.id=cell; ticks 10"),
        "b.jsonl",
    );
    assert_eq!(last_campaign(&states), ("cell".into(), raced));
}
