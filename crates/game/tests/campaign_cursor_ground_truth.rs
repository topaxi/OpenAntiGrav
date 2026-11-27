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

use std::path::Path;

use serde_json::Value;

/// A profile directory; the game's config, data and state live under it.
struct Profile(oag_testdata::scratch::TempDir);

impl Profile {
    /// A new, empty profile owned by this test process alone. The process id keeps two tests that
    /// share a `tag` (the PS2 and PSP pickers) from wiping and rewriting each other's state log
    /// when nextest runs them at once.
    fn fresh(tag: &str) -> Self {
        let dir = oag_testdata::scratch::dir(&format!("campaign-cursor-{tag}"));
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
            .enumerate()
            .map(|(n, line)| {
                serde_json::from_str(line).unwrap_or_else(|e| {
                    panic!(
                        "{} line {}: a state line is JSON ({e}): {line:?}",
                        log.display(),
                        n + 1
                    )
                })
            })
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

/// The editor sits over the campaigns page, so `page=campaigns` holds while it is open; the save
/// is written off the render thread and the editor closes (`screen` reads `page`) only once it has
/// finished. Scripts wait on both, never on a tick count.
///
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
         press start; until page=campaigns screen=page within 600; ticks 30; \
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
         press start; until page=campaigns screen=page within 600; ticks 30; \
         select \"MY CAMPAIGN\"; press cross; until screen.id=cell; ticks 10; \
         press circle; until page=campaigns screen=page within 600; ticks 10; \
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
             press start; until page=campaigns screen=page within 600; ticks 10"
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

/// On Pulse's PS2 pressing `Cell Selection` leaves on the triangle and opens `Cell Help` on the
/// circle: its executable binds `backward` to the triangle and its `CellMode_Definition.xml`
/// authors `forward="circle"` on the `Cell Help` redirect, with `Back` drawn beside a triangle
/// (`docs/ui/campaign-screens.md`, "Back and Help on the PS2"). The PSP's buttons are the other way
/// round, and do the PSP's thing.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and pulse-psp-eu.chd"]
fn cell_selection_backs_out_on_the_pressings_own_button() {
    let (Some(ps2), Some(psp)) = (
        oag_testdata::image("pulse-ps2-eu.chd"),
        oag_testdata::image("pulse-psp-eu.chd"),
    ) else {
        return;
    };
    let profile = Profile::fresh("back-buttons");
    let level = |image: &Path, tail: &str, log: &str| {
        let states = profile.drive_on(
            image,
            &format!("{OPEN}; press cross; until screen.id=cell; ticks 20; {tail}; ticks 20"),
            log,
        );
        last_campaign(&states).0
    };
    assert_eq!(level(&ps2, "press triangle", "ps2-triangle.jsonl"), "grid");
    // A second triangle leaves Grid Selection for the main menu.
    let states = profile.drive_on(
        &ps2,
        &format!(
            "{OPEN}; press cross; until screen.id=cell; ticks 20; press triangle; \
             until screen.id=grid; ticks 20; press triangle; until screen!=campaign"
        ),
        "ps2-twice.jsonl",
    );
    assert_ne!(
        states
            .last()
            .and_then(|state| state["screen"]["kind"].as_str()),
        Some("campaign"),
        "the second triangle left the campaign"
    );
    assert_eq!(
        level(&ps2, "press circle", "ps2-circle.jsonl"),
        "cell",
        "circle opens Cell Help on the PS2, it does not leave"
    );
    assert_eq!(level(&psp, "press circle", "psp-circle.jsonl"), "grid");
    assert_eq!(
        level(&psp, "press triangle", "psp-triangle.jsonl"),
        "cell",
        "triangle opens Cell Help on the PSP, it does not leave"
    );
}

/// With `Cell Help` open a direction moves nothing, and the pressing's help button closes it again.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn cell_help_opened_on_the_ps2_circle_freezes_the_cursor_until_it_closes() {
    let Some(ps2) = oag_testdata::image("pulse-ps2-eu.chd") else {
        return;
    };
    let profile = Profile::fresh("ps2-help");
    let open = format!("{OPEN}; press cross; until screen.id=cell; ticks 20");
    let held = profile.drive_on(
        &ps2,
        &format!("{open}; press circle; ticks 10; press down; ticks 20"),
        "held.jsonl",
    );
    let (first, last) = first_and_last_cell(&held);
    assert_eq!(first, last, "Cell Help is open: down moved nothing");
    let freed = profile.drive_on(
        &ps2,
        &format!("{open}; press circle; ticks 10; press circle; ticks 10; press down; ticks 20"),
        "freed.jsonl",
    );
    let (first, last) = first_and_last_cell(&freed);
    assert_ne!(first, last, "Cell Help closed: down moves the cursor again");
}

/// The CONTROLS row MENU BACK = `circle` puts Back on the circle on Pulse's PS2, where the disc
/// binds it to the triangle: `Cell Selection` and `Grid Selection` leave on the circle, and Help,
/// which the circle opened, moves to the triangle (`docs/ui/campaign-screens.md`, "Menu back").
/// `original` is the previous test's behaviour.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn menu_back_circle_leaves_cell_selection_on_the_circle_and_moves_help_to_the_triangle() {
    let Some(ps2) = oag_testdata::image("pulse-ps2-eu.chd") else {
        return;
    };
    let profile = Profile::fresh("menu-back-circle");
    profile.menu_back("circle");
    let level = |tail: &str, log: &str| {
        let states = profile.drive_on(
            &ps2,
            &format!("{OPEN}; press cross; until screen.id=cell; ticks 20; {tail}; ticks 20"),
            log,
        );
        last_campaign(&states).0
    };
    assert_eq!(level("press circle", "circle.jsonl"), "grid");
    assert_eq!(
        level("press triangle", "triangle.jsonl"),
        "cell",
        "triangle opens Cell Help once circle is Back"
    );
    let states = profile.drive_on(
        &ps2,
        &format!(
            "{OPEN}; press cross; until screen.id=cell; ticks 20; press circle; \
             until screen.id=grid; ticks 20; press circle; until screen!=campaign"
        ),
        "twice.jsonl",
    );
    assert_ne!(
        states
            .last()
            .and_then(|state| state["screen"]["kind"].as_str()),
        Some("campaign"),
        "the second circle left the campaign"
    );
    let original = Profile::fresh("menu-back-original");
    original.menu_back("original");
    let states = original.drive_on(
        &ps2,
        &format!("{OPEN}; press cross; until screen.id=cell; ticks 20; press circle; ticks 20"),
        "original.jsonl",
    );
    assert_eq!(
        last_campaign(&states).0,
        "cell",
        "original keeps the PS2's circle on Help"
    );
}

// Back on every front-end screen: `oag_title::Title::menu_buttons` resolves the button the title,
// the pressing and the player's MENU BACK choice leave on (`docs/ui/back-and-help.md`). Pulse's PS2
// pressing leaves on the triangle where its executable binds `backward`
// (`docs/ghidra/functions/ps2-pulse-eu/input.md`, "The divergence"), the PSP and HD on the circle.
// The campaign screens are the tests above.

/// A screen a face button leaves: how to reach it, and the conditions that hold on it and once it is left.
struct Screen {
    tag: &'static str,
    reach: &'static str,
    /// The last forward steps of `reach`: from the screen Back returns to, back onto this one.
    again: &'static str,
    here: &'static str,
    away: &'static str,
    /// The screen authors a Help: the button that is not Back opens it, so it is pressed again to
    /// close it before Back is tried.
    helps: bool,
}

const RACE_PAGE: Screen = Screen {
    tag: "race-page",
    reach: "boot; select racebox; press cross; until page=race; ticks 10",
    again: "select racebox; press cross; until page=race; ticks 10",
    here: "page=race",
    away: "page!=race",
    helps: false,
};
const TRACK_PICKER: Screen = Screen {
    tag: "track-picker",
    reach: "boot; select racebox; press cross; until page=race; select launch_race; \
            press cross; until screen=picker; ticks 10",
    again: "select launch_race; press cross; until screen=picker; ticks 10",
    here: "screen.id=track-select",
    away: "screen.id!=track-select",
    helps: true,
};
const SHIP_PICKER: Screen = Screen {
    tag: "ship-picker",
    reach: "boot; select racebox; press cross; until page=race; select launch_race; \
            press cross; until screen=picker; press cross; until screen.id=ship-select; ticks 10",
    again: "press cross; until screen.id=ship-select; ticks 10",
    here: "screen.id=ship-select",
    away: "screen.id!=ship-select",
    helps: true,
};
/// The on-screen keyboard OPTIONS > PLAYER NAME opens.
const NAME_PROMPT: Screen = Screen {
    tag: "name-prompt",
    reach: "boot; select options; press cross; ticks 10; select edit_player_name; \
            press cross; until screen=prompt; ticks 10",
    again: "select edit_player_name; press cross; until screen=prompt; ticks 10",
    here: "screen=prompt",
    away: "screen!=prompt",
    helps: false,
};
/// The pause screen of a time trial, on its GAME OPTIONS page.
const PAUSE_OPTIONS: Screen = Screen {
    tag: "pause-options",
    reach: "boot; select racebox; press cross; until page=race; set race.mode=time_trial; \
            select launch_race; press cross; until screen=picker; press cross; \
            until screen.id=ship-select; press cross; until stage=race within 4000; ticks 400; \
            press start; until stage=menu; expect page=pause; ticks 5; select \"Game options\"; \
            press cross; until page=options; ticks 10",
    again: "select \"Game options\"; press cross; until page=options; ticks 10",
    here: "page=options",
    away: "page!=options",
    helps: false,
};

/// `(back, other)`: the face button the screen leaves on and the one it does not, under `choice`
/// on a pressing whose own Back is `original`.
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

/// How a folded script gets onto `screen` for each choice of MENU BACK: the first on the whole
/// `reach`, the others by applying the choice (`config`) where Back left the player and going
/// forward onto the screen again.
fn onto_the_screen_under(screen: &Screen, number: usize, choice: &str) -> String {
    if number == 0 {
        screen.reach.to_string()
    } else {
        format!("config controls.menu_back={choice}; {}", screen.again)
    }
}

/// On `screen` the face button that is not Back leaves nothing and Back leaves, under every choice of
/// MENU BACK: one launch for all three, `config` switching the choice and `expect` checking before
/// and after each.
fn leaves_on_the_resolved_button(image: &Path, screen: &Screen, original: &'static str) {
    let profile = Profile::fresh(screen.tag);
    profile.menu_back("original");
    let mut script = String::new();
    for (number, choice) in ["original", "circle", "triangle"].into_iter().enumerate() {
        let (back, other) = buttons(original, choice);
        println!(
            "{}: MENU BACK {choice}: {back} leaves, {other} does not",
            screen.tag
        );
        script.push_str(&format!(
            "{}; press {other}; ticks 30; expect {}; {}press {back}; ticks 30; expect {}; ",
            onto_the_screen_under(screen, number, choice),
            screen.here,
            if screen.helps {
                format!("press {other}; ticks 10; ")
            } else {
                String::new()
            },
            screen.away
        ));
    }
    profile.drive_on(image, &script, "folded.jsonl");
}

/// MENU BACK read from `settings.toml` at boot, which the folded tests above do not do for
/// `circle` and `triangle` (they switch to those with `config`): the RACE page's footer shows the
/// resolved Back, the button that is not Back leaves nothing and Back leaves.
fn boots_under_menu_back(image: &Path, original: &'static str, choice: &str) {
    let profile = Profile::fresh(&format!("boot-{choice}-{original}"));
    let (back, other) = buttons(original, choice);
    profile.menu_back(choice);
    profile.drive_on(
        image,
        &format!(
            "{}; ticks 20; draw; expect hints={back}; press {other}; ticks 30; expect {}; \
             press {back}; ticks 30; expect {}",
            RACE_PAGE.reach, RACE_PAGE.here, RACE_PAGE.away
        ),
        "booted.jsonl",
    );
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_boots_under_menu_back_circle() {
    let Some(image) = ps2() else { return };
    boots_under_menu_back(&image, "triangle", "circle");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_boots_under_menu_back_triangle() {
    let Some(image) = ps2() else { return };
    boots_under_menu_back(&image, "triangle", "triangle");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_boots_under_menu_back_circle() {
    let Some(image) = psp() else { return };
    boots_under_menu_back(&image, "circle", "circle");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_boots_under_menu_back_triangle() {
    let Some(image) = psp() else { return };
    boots_under_menu_back(&image, "circle", "triangle");
}

fn ps2() -> Option<std::path::PathBuf> {
    oag_testdata::image("pulse-ps2-eu.chd")
}

fn psp() -> Option<std::path::PathBuf> {
    oag_testdata::image("pulse-psp-eu.chd")
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_race_page_leaves_on_the_triangle() {
    let Some(image) = ps2() else { return };
    leaves_on_the_resolved_button(&image, &RACE_PAGE, "triangle");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_track_picker_leaves_on_the_triangle() {
    let Some(image) = ps2() else { return };
    leaves_on_the_resolved_button(&image, &TRACK_PICKER, "triangle");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_ship_picker_leaves_on_the_triangle() {
    let Some(image) = ps2() else { return };
    leaves_on_the_resolved_button(&image, &SHIP_PICKER, "triangle");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_keyboard_prompt_cancels_on_the_triangle() {
    let Some(image) = ps2() else { return };
    leaves_on_the_resolved_button(&image, &NAME_PROMPT, "triangle");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_pause_menu_page_leaves_on_the_triangle() {
    let Some(image) = ps2() else { return };
    leaves_on_the_resolved_button(&image, &PAUSE_OPTIONS, "triangle");
}

/// The PSP leaves on the circle as before, and follows MENU BACK = `triangle` without the input
/// exchange this replaced.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_race_page_still_leaves_on_the_circle() {
    let Some(image) = psp() else { return };
    leaves_on_the_resolved_button(&image, &RACE_PAGE, "circle");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_track_picker_still_leaves_on_the_circle() {
    let Some(image) = psp() else { return };
    leaves_on_the_resolved_button(&image, &TRACK_PICKER, "circle");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_keyboard_prompt_still_cancels_on_the_circle() {
    let Some(image) = psp() else { return };
    leaves_on_the_resolved_button(&image, &NAME_PROMPT, "circle");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_pause_menu_page_still_leaves_on_the_circle() {
    let Some(image) = psp() else { return };
    leaves_on_the_resolved_button(&image, &PAUSE_OPTIONS, "circle");
}

/// HD leaves on the circle as before.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_race_page_still_leaves_on_the_circle() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    leaves_on_the_resolved_button(&image, &RACE_PAGE, "circle");
}

/// A race to its first few hundred ticks from the main menu.
const TO_THE_RACE: &str = "boot; select racebox; press cross; until page=race; \
     select launch_race; press cross; until screen=picker; press cross; \
     until screen.id=ship-select; press cross; until stage=race within 4000; ticks 400; ";

/// Photo mode and the replay viewer, which read the device apart from the front end, leave on the
/// resolved Back and not on the circle: one launch inside a race (photo mode, then Escape in it)
/// and one that watches the race it recorded.
///
/// `original` is the pressing's own Back, so the button that is not Back is the one Back left over
/// (`Roles::help`: it resets the photo camera and does nothing in the viewer).
fn photo_and_viewer_leave_on_the_resolved_button(
    image: &Path,
    original: &'static str,
    choice: &str,
) {
    let profile = Profile::fresh(&format!("photo-viewer-{choice}-{original}"));
    let (back, other) = buttons(original, choice);
    println!("photo mode and the viewer, MENU BACK {choice}: {back} leaves, {other} does not");
    profile.menu_back(choice);
    // Photo mode: the button that is not Back leaves it up; Back leaves; Escape leaves on any
    // choice. The launch ends mid-race, which records the race for the viewer below.
    profile.drive_on(
        image,
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=free within 200; ticks 5; press {other}; ticks 10; expect photo=free; \
             press {back}; until photo=none within 50; \
             escape; until stage=menu; select photo_mode; press cross; \
             until photo=free within 200; ticks 5; escape; until photo=none within 50; quit"
        ),
        "photo.jsonl",
    );
    // The viewer: the recording opens, the button that is not Back changes nothing, Back leaves
    // for the REPLAYS page (`stage=menu`).
    profile.drive_on(
        image,
        &format!(
            "boot; select replays; press cross; until page=replays; select watch_replay; \
             press cross; until replay=playing within 4000; ticks 30; press {other}; ticks 10; \
             expect stage=race; press {back}; until stage=menu within 100; expect replay!=playing"
        ),
        "viewer.jsonl",
    );
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU"]
fn pulse_ps2_photo_mode_and_viewer_leave_on_the_triangle_under_original() {
    let Some(image) = ps2() else { return };
    photo_and_viewer_leave_on_the_resolved_button(&image, "triangle", "original");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU"]
fn pulse_ps2_photo_mode_and_viewer_leave_on_the_circle_under_circle() {
    let Some(image) = ps2() else { return };
    photo_and_viewer_leave_on_the_resolved_button(&image, "triangle", "circle");
}

/// The PSP leaves on the circle as before.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and a GPU"]
fn pulse_psp_photo_mode_and_viewer_still_leave_on_the_circle() {
    let Some(image) = psp() else { return };
    photo_and_viewer_leave_on_the_resolved_button(&image, "circle", "original");
}

/// On a selection screen that authors a Help, the face button Back left over opens it, Back
/// closes it without leaving, and Back then leaves, under every choice of MENU BACK: one launch
/// for all three, `config` switching the choice. `original` is the pressing's own Back, so the Help
/// button is the other one.
fn help_opens_on_the_displaced_button(image: &Path, screen: &Screen, original: &'static str) {
    let profile = Profile::fresh(&format!("{}-help", screen.tag));
    profile.menu_back("original");
    let mut script = String::new();
    for (number, choice) in ["original", "circle", "triangle"].into_iter().enumerate() {
        let (back, other) = buttons(original, choice);
        println!(
            "{}: MENU BACK {choice}: {other} opens Help, {back} closes it and then leaves",
            screen.tag
        );
        script.push_str(&format!(
            "{reach}; expect screen.help=closed; press {other}; ticks 10; \
             expect screen.help=open; expect {here}; press {back}; ticks 10; \
             expect screen.help=closed; expect {here}; press {other}; ticks 10; \
             expect screen.help=open; press {other}; ticks 10; expect screen.help=closed; \
             press {back}; ticks 30; expect {away}; ",
            reach = onto_the_screen_under(screen, number, choice),
            here = screen.here,
            away = screen.away
        ));
    }
    profile.drive_on(image, &script, "folded.jsonl");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_track_select_opens_track_help_on_the_circle() {
    let Some(image) = ps2() else { return };
    help_opens_on_the_displaced_button(&image, &TRACK_PICKER, "triangle");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_ship_select_opens_team_help_on_the_circle() {
    let Some(image) = ps2() else { return };
    help_opens_on_the_displaced_button(&image, &SHIP_PICKER, "triangle");
}

/// The PSP authors its Help on the triangle and still leaves on the circle.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_track_select_opens_track_help_on_the_triangle() {
    let Some(image) = psp() else { return };
    help_opens_on_the_displaced_button(&image, &TRACK_PICKER, "circle");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_ship_select_opens_team_help_on_the_triangle() {
    let Some(image) = psp() else { return };
    help_opens_on_the_displaced_button(&image, &SHIP_PICKER, "circle");
}

/// Assegai's own description (`MSC_TEAMDES_ASS`), the team Ship Select opens on.
const ASSEGAI_HELP: &str = "The Assegai's exceptional handling model forms the core of the United African \
     Nations' new ship, which has ably demonstrated solid all-round abilities in other areas.";

/// `Team Help`'s second paragraph is the selected team's own description (the PSP's team definition
/// authors a `helpText` per team): Assegai's on the team the
/// screen opens on, and another team's once the selection has moved (`TeamSelection_Update` writes
/// `Team Ship Help` from the team's `HelpText` on every change).
fn team_help_describes_the_selected_team(image: &Path, help: &str) {
    let profile = Profile::fresh(&format!("team-help-text-{help}"));
    profile.drive_on(
        image,
        &format!(
            "{reach}; press {help}; ticks 10; expect screen.help=open; \
             expect screen.help_text=\"{ASSEGAI_HELP}\"; press {help}; ticks 10; \
             press down; ticks 10; press {help}; ticks 10; expect screen.help=open; \
             expect screen.help_text!=\"{ASSEGAI_HELP}\"",
            reach = SHIP_PICKER.reach
        ),
        "text.jsonl",
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_psp_team_help_describes_the_selected_team() {
    let Some(image) = psp() else { return };
    team_help_describes_the_selected_team(&image, "triangle");
}

/// The PS2's team definition authors no `HelpText`, and its `TeamSelection_Update` (`0x001bd368`)
/// writes `Team Ship Help` from the team's empty field all the same: the second paragraph is blank
/// for every team.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_team_help_has_no_team_paragraph() {
    let Some(image) = ps2() else { return };
    let profile = Profile::fresh("team-help-text-ps2");
    profile.drive_on(
        &image,
        &format!(
            "{}; press circle; ticks 10; expect screen.help=open; expect screen.help_text=\"\"",
            SHIP_PICKER.reach
        ),
        "text.jsonl",
    );
}
