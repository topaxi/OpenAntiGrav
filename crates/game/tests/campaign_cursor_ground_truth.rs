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
        self.drive_with_mods(image, script, log, &self.0.join("no-mods"))
    }

    /// [`Self::drive_on`] with `mods` as the whole mod search path.
    fn drive_with_mods(&self, image: &Path, script: &str, log: &str, mods: &Path) -> Vec<Value> {
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
            .env("OAG_MODS", mods)
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

/// RACE CAMPAIGN is open, on `Grid Selection`. With no mod campaign Pulse has no picker: the
/// row opens the disc's campaign, and Square on it the editor.
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

/// RACE CAMPAIGN is open on a title that has no campaign picker (no mod campaign).
const OPEN_DIRECT: &str =
    "boot; until stage=menu within 3000; select campaign; press cross; until screen=campaign";

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

/// Back from a race lands on the cell raced - through the real path (launch, Escape, the pause
/// menu's QUIT RACE row) - and so does a relaunch after a launch.
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
             escape; until stage=menu; select pause_quit; press cross; \
             until screen.id=cell within 600; ticks 10"
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

/// The pulse clocks (`screen.pulse_ms`) the campaign reported on level `id`, in order.
fn pulses(states: &[Value], id: &str) -> Vec<u64> {
    states
        .iter()
        .filter_map(|state| {
            let screen = state.get("screen").filter(|s| s["kind"] == "campaign")?;
            (screen["id"] == id).then(|| screen["pulse_ms"].as_u64())?
        })
        .collect()
}

/// `Selector`'s pulse resumes where the screen was left, as the original's widget does (Pulse's
/// `0x088a3e8c` keeps its phase in the widget): `Grid Selection` through the main menu and back,
/// and `Cell Selection` through `Grid Selection` and back, each with its own phase, frozen while
/// away. A cold first entry starts at 0.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_selector_pulse_resumes_where_the_screen_was_left() {
    if !pulse_present() {
        return;
    }
    let profile = Profile::fresh("pulse");
    let states = profile.drive(
        &format!(
            "{OPEN}; ticks 40; press cross; until screen.id=cell; ticks 25; \
             press circle; until screen.id=grid; ticks 90; \
             press cross; until screen.id=cell; ticks 5; \
             press circle; until screen.id=grid; ticks 5; \
             press circle; until screen!=campaign; ticks 120; \
             select campaign; press cross; until screen=campaign; ticks 5"
        ),
        "a.jsonl",
    );
    let grid = pulses(&states, "grid");
    let cell = pulses(&states, "cell");
    assert!(
        grid[0] < 100,
        "a cold first entry starts at phase 0: {}",
        grid[0]
    );
    assert!(
        cell[0] < 100,
        "Cell's first entry starts at phase 0: {}",
        cell[0]
    );
    // Cell: left at `cell_left` after ~25 ticks, re-entered after 90 ticks on Grid.
    let cell_left = cell
        .iter()
        .copied()
        .take_while(|ms| *ms < 5000)
        .max()
        .unwrap();
    let reentry = cell
        .iter()
        .position(|ms| *ms >= 300 && *ms + 5 >= cell_left)
        .expect("Cell resumed");
    assert!(
        cell_left > 300,
        "the first Cell visit ran long enough: {cell_left}"
    );
    assert!(
        cell[reentry] >= cell_left && cell[reentry] < cell_left + 100,
        "Cell resumed at {} after leaving at {cell_left}, not restarted and not run while away",
        cell[reentry]
    );
    // Grid: its phase is not Cell's, and survives Cell's visit and the trip to the main menu.
    let grid_at_end = *grid.last().unwrap();
    assert!(
        grid_at_end > 1100,
        "Grid ran about 1.9 s in total across its visits, 2 s of main menu not counted: {grid_at_end}"
    );
    let jump = grid.windows(2).filter(|w| w[1] + 100 < w[0]).count();
    assert_eq!(jump, 0, "the Grid clock never fell back to 0: {grid:?}");
}

/// Omega's RACE CAMPAIGN opens the three-way chooser on its first entry (2048), and each of the
/// other two entries opens its own grid: HD's `grid0` and Fury's `grid8`, with Back returning to
/// the entry it came from. 2048's grids are not loaded (`grid_16`..`grid_18` author no
/// `RequiredPoints`), so confirming it stays on the chooser rather than falling into HD's cell
/// screen with nothing in it. Dropping the chooser read, the three-way list or the redirect
/// failing here is what this is for.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omegas_chooser_opens_hd_and_fury_and_keeps_2048_on_the_chooser() {
    let Some(image) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let profile = Profile::fresh("omega-chooser");
    let states = profile.drive_on(
        &image,
        &format!(
            "{OPEN_DIRECT}; press right; ticks 5; press cross; ticks 20; press circle; ticks 5; \
             press right; ticks 5; press cross; ticks 20; press circle; ticks 5; \
             press left; press left; ticks 5; press cross; ticks 30"
        ),
        "a.jsonl",
    );
    let mut walked: Vec<(String, String)> = Vec::new();
    for state in &states {
        let Some(screen) = state.get("screen").filter(|s| s["kind"] == "campaign") else {
            continue;
        };
        let at = (
            screen["id"].as_str().unwrap_or_default().to_string(),
            screen["focus"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        );
        if walked.last() != Some(&at) {
            walked.push(at);
        }
    }
    let want: Vec<(&str, &str)> = vec![
        ("selection", "FE_RC"),
        ("selection", "FE_RC_HD"),
        ("grid", "grid0"),
        ("selection", "FE_RC_HD"),
        ("selection", "FE_RC_FURY"),
        ("grid", "grid8"),
        ("selection", "FE_RC_FURY"),
        ("selection", "FE_RC_HD"),
        ("selection", "FE_RC"),
    ];
    let got: Vec<(&str, &str)> = walked
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(got, want);
}

/// A campaign built in the editor is saved as a folder mod under the user mods directory, is
/// listed in RACE CAMPAIGN's picker, and its first event launches with the mode and team the editor set.
/// Drops the editor's wiring (Square on RACE CAMPAIGN, the pad, the save, the menu rebuild) and this
/// fails.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_campaign_made_in_the_editor_is_listed_and_its_event_launches() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let profile = Profile::fresh("editor");
    // The user mods directory is `<XDG_DATA_HOME>/oag/mods`; pinned as the whole search path.
    let mods = profile.0.join("data").join("oag").join("mods");
    std::fs::create_dir_all(&mods).expect("a mods directory");
    let states = profile.drive_with_mods(
        &image,
        "boot; until stage=menu within 3000; select campaign; press square; until screen=campaign; \
         ticks 20; press cross; press down; press down; press down; press right; press circle; \
         press right; press down; press cross; press right; press circle; \
         press start; until page=campaigns within 600; ticks 30; \
         select \"MY CAMPAIGN\"; press cross; until screen.id=cell; ticks 10; press cross; \
         until stage=race within 6000",
        "editor.jsonl",
        &mods,
    );
    let saved = mods.join("my-campaign");
    let def = oag_mod::campaign::CampaignDef::parse(
        &std::fs::read_to_string(saved.join("campaigns/my-campaign.toml"))
            .expect("the campaign file"),
    )
    .expect("the written file reads back");
    assert_eq!(def.grids[0].events.len(), 2);
    assert_eq!(def.grids[0].events[0].slot, [0, 0]);
    assert!(
        oag_mod::Mod::open(&saved).is_ok(),
        "the written folder is a mod"
    );
    // The picker listed it after the save.
    let listed = states.iter().any(|state| {
        state["page"] == "campaigns"
            && state["screen"]["rows"]
                .as_array()
                .is_some_and(|rows| rows.iter().any(|row| row["label"] == "MY CAMPAIGN"))
    });
    assert!(listed, "MY CAMPAIGN is in the picker after the save");
    // And the event it launches is the one the editor set: its mode and its preset team.
    let race = states
        .iter()
        .rev()
        .find_map(|state| {
            state
                .get("race")
                .filter(|race| race["mode"] == "single_race")
        })
        .expect("a single race started");
    assert_eq!(race["team"], "Assegai");
}

/// A campaign of one grid opens straight on that grid's `Cell Selection`, `Back` there returns to
/// the picker it came from (never a one-entry `Grid Selection`), and Pulse's own sixteen-grid
/// campaign still opens `Grid Selection`. Drops the skip and the first assertion fails; drops the
/// `Back` arm and the campaign reopens on `Grid Selection`.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_one_grid_campaign_opens_on_its_cells_and_back_returns_to_the_picker() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let profile = Profile::fresh("single-grid");
    let mods = profile.0.join("data").join("oag").join("mods");
    std::fs::create_dir_all(&mods).expect("a mods directory");
    let states = profile.drive_with_mods(
        &image,
        "boot; until stage=menu within 3000; select campaign; press square; until screen=campaign; \
         ticks 20; press cross; press down; press down; press down; press right; press circle; \
         press right; press down; press cross; press right; press circle; \
         press start; until page=campaigns within 600; ticks 30; \
         select \"MY CAMPAIGN\"; press cross; until screen.id=cell; ticks 10; \
         press circle; until page=campaigns within 600; ticks 10; \
         select campaign-original; press cross; until screen.id=grid; ticks 5",
        "single.jsonl",
        &mods,
    );
    let level = |state: &Value| -> Option<String> {
        state
            .get("screen")
            .filter(|s| s["kind"] == "campaign")
            .map(|s| s["id"].as_str().unwrap_or_default().to_string())
    };
    let on_picker =
        |state: &Value| state["page"] == "campaigns" && state["screen"]["kind"] == "page";
    let first_cell = states
        .iter()
        .position(|state| level(state).as_deref() == Some("cell") && !on_picker(state))
        .expect("Cell Selection opened");
    // From the last time the picker was up before that cell, no `Grid Selection` came between.
    let picker_before = states[..first_cell]
        .iter()
        .rposition(on_picker)
        .expect("the picker was up before the cell");
    assert!(
        !states[picker_before..first_cell]
            .iter()
            .any(|state| level(state).as_deref() == Some("grid")),
        "a one-grid campaign must not show Grid Selection between the picker and its cells"
    );
    // Back from the cells returned to the picker, with no `Grid Selection` in between.
    let back_to = states[first_cell..]
        .iter()
        .position(|state| level(state).as_deref() != Some("cell") && !on_picker(state))
        .map(|at| at + first_cell);
    let picker_after = states[first_cell..]
        .iter()
        .position(on_picker)
        .map(|at| at + first_cell)
        .expect("Back returned to the campaigns picker");
    assert!(
        back_to.is_none_or(|at| at > picker_after),
        "Back from Cell Selection must reach the picker before any other campaign screen"
    );
    let (level, _) = last_campaign(&states);
    assert_eq!(
        level, "grid",
        "Pulse's own sixteen-grid campaign still opens Grid Selection"
    );
}

/// The campaign page through the real session: SELECT opens it, the pad sets the difficulty, the
/// progression and the class bounds, TRIANGLE fills the grid from the seed, START saves. The same
/// seed saved twice writes the same grid, another seed another, and the file carries the setting.
/// Drops the page, the fill or the setting's write and the first assertion fails.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_grid_filled_from_a_seed_in_the_editor_is_saved_with_its_setting() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let profile = Profile::fresh("editor-fill");
    let mods = profile.0.join("data").join("oag").join("mods");
    std::fs::create_dir_all(&mods).expect("a mods directory");
    // Hard, a ramp, the seed as it is (or one on), classes Flash to Rapier.
    // The first run has no mod campaign, so Square on RACE CAMPAIGN opens the editor; its save
    // makes the picker, and the later runs reach NEW CAMPAIGN there.
    const FIRST: &str = "select campaign; press square;";
    const LATER: &str =
        "select campaign; press cross; until page=campaigns; select campaign-editor; press cross;";
    let script = |open: &str, seed_steps: &str| {
        format!(
            "boot; until stage=menu within 3000; {open} until screen=campaign; \
             ticks 20; press select; press right; press down; press right; press down; {seed_steps} \
             press down; press right; press down; press left; press triangle; ticks 5; \
             press start; until page=campaigns within 600; ticks 10"
        )
    };
    profile.drive_with_mods(&image, &script(FIRST, ""), "fill-a.jsonl", &mods);
    profile.drive_with_mods(&image, &script(LATER, ""), "fill-b.jsonl", &mods);
    profile.drive_with_mods(
        &image,
        &script(LATER, "press right;"),
        "fill-c.jsonl",
        &mods,
    );
    let read = |folder: &str| {
        oag_mod::campaign::CampaignDef::parse(
            &std::fs::read_to_string(mods.join(folder).join("campaigns/my-campaign.toml"))
                .expect("the campaign file"),
        )
        .expect("the written file reads back")
    };
    let (a, b, c) = (
        read("my-campaign"),
        read("my-campaign-2"),
        read("my-campaign-3"),
    );
    assert_eq!(a.campaign.difficulty.as_deref(), Some("hard"));
    assert_eq!(a.campaign.progression, oag_mod::campaign::Progression::Ramp);
    assert_eq!(a.grids[0].events.len(), 32, "every hex of Pulse's grid");
    assert_eq!(a.grids, b.grids, "the same seed rebuilds the same grid");
    assert_ne!(a.grids, c.grids, "another seed builds another");
    let classes: std::collections::BTreeSet<_> = a.grids[0]
        .events
        .iter()
        .filter_map(|e| e.class.as_ref().map(|c| c.to_ascii_lowercase()))
        .collect();
    assert!(
        classes
            .iter()
            .all(|c| ["flash", "rapier"].contains(&c.as_str()))
            && classes.len() == 2,
        "{classes:?}"
    );
}
