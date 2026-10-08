//! Wipeout HD/Fury's campaign as a player plays it, against the disc: the grids
//! the engine reads carry the targets the front end shows, a cell's result
//! becomes a medal at the rung it was flown at, the medal persists under the
//! title's key, and the points it earns open the next grid.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_campaign_ground_truth)'
//! ```
//!
//! Omega shares `Cell`, `plan_cell`, `records::Store` and `grid_points_met`
//! with this file's subject, so each fix covered here lands on it too; its own
//! disc is `omega_campaign_launch_ground_truth.rs`.

use oag_game::campaign::launch::{self, Refusal};
use oag_game::records::{self, Store};
use oag_game::unlock::campaign_medal_of;
use oag_tables::race_campaign::{self, Cell, Difficulty, Grid, Medal, Mode};

const TITLE: &str = "wipeout hd";

struct Hd {
    /// What the engine reads: `Title::campaign.grid_archive` honoured.
    grids: Vec<Grid>,
    /// What plain archive precedence reads, the way the engine used to.
    precedence: Vec<Grid>,
    race: Vec<oag_raceplay::catalogue::Track>,
    zone: Vec<oag_raceplay::catalogue::Track>,
}

fn open() -> Option<Hd> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let options = oag_game::boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-hd-campaign-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, mut archives, title) = oag_game::boot::load_shell(&options).expect("HD boots");
    let entry = oag_hd::campaign::DEFINITION_ENTRY;
    let grids = oag_game::campaign::read_grids(&mut archives, entry, title.campaign.grid_archive)
        .expect("the grids read");
    let precedence =
        oag_game::campaign::read_grids(&mut archives, entry, None).expect("the grids read");
    Some(Hd {
        grids,
        precedence,
        race: shell.tracks,
        zone: shell.zone_tracks,
    })
}

fn cell<'a>(grids: &'a [Grid], name: &str) -> &'a Cell {
    grids
        .iter()
        .flat_map(|grid| &grid.cells)
        .find(|cell| cell.name == name)
        .unwrap_or_else(|| panic!("no cell {name}"))
}

fn to_record(medal: Medal) -> records::Medal {
    match medal {
        Medal::Gold => records::Medal::Gold,
        Medal::Silver => records::Medal::Silver,
        Medal::Bronze => records::Medal::Bronze,
    }
}

fn to_record_difficulty(difficulty: Difficulty) -> records::Difficulty {
    match difficulty {
        Difficulty::Easy => records::Difficulty::Easy,
        Difficulty::Medium => records::Difficulty::Medium,
        Difficulty::Hard => records::Difficulty::Hard,
    }
}

/// The base campaign is read off `DATA06`, whose cells carry all three rungs;
/// `DATA02`'s older flat set is `DATA06`'s hard rung on every cell that has one,
/// so reading it handed a novice the elite times.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_base_campaign_carries_three_rungs_and_the_old_flat_set_is_its_hard_one() {
    let Some(hd) = open() else { return };
    assert_eq!(hd.grids.len(), 16);
    let mut with_rungs = 0;
    for (grid, old) in hd.grids.iter().zip(&hd.precedence).take(8) {
        assert_eq!(grid.cells.len(), old.cells.len(), "{}", grid.name);
        for cell in &grid.cells {
            let flat = old
                .cells
                .iter()
                .find(|c| c.name == cell.name)
                .expect("cell");
            let Some(rungs) = cell.difficulty_targets else {
                // A finishing-place target (1st/2nd/3rd) has no rungs.
                assert!(
                    matches!(cell.mode, Mode::Race | Mode::Tournament),
                    "{}",
                    cell.name
                );
                continue;
            };
            with_rungs += 1;
            assert_eq!(
                (rungs.hard.gold, rungs.hard.silver, rungs.hard.bronze),
                (flat.gold, flat.silver, flat.bronze),
                "{}",
                cell.name
            );
            assert_ne!(rungs.easy, rungs.hard, "{}: easy is looser", cell.name);
        }
    }
    assert!(with_rungs > 40, "{with_rungs}");
    for (grid, old) in hd.grids.iter().zip(&hd.precedence).skip(8) {
        assert_eq!(grid.cells.len(), old.cells.len(), "Fury {}", grid.name);
    }
}

/// A cell's result is judged against the rung it was flown at: grid0's Time
/// Trial at 119.00 s is gold for a novice and only bronze at elite.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_result_is_judged_against_the_rung_it_was_flown_at() {
    let Some(hd) = open() else { return };
    let time_trial = cell(&hd.grids, "grid0_2_2");
    assert_eq!(time_trial.mode, Mode::TimeTrial);
    assert_eq!(
        time_trial.evaluate_medal_for_difficulty(11900, Difficulty::Easy),
        Some(Medal::Gold)
    );
    assert_eq!(
        time_trial.evaluate_medal_for_difficulty(11900, Difficulty::Hard),
        Some(Medal::Bronze)
    );
    let speed_lap = cell(&hd.grids, "grid0_3_2");
    assert_eq!(
        speed_lap.evaluate_medal_for_difficulty(3703, Difficulty::Easy),
        Some(Medal::Gold)
    );
    assert_eq!(
        speed_lap.evaluate_medal_for_difficulty(3703, Difficulty::Hard),
        Some(Medal::Silver)
    );
}

/// Fury's `Elimination` and `NitroBattle` cells author the dummy `1`/`2`/`3` and
/// a `NitroElim*` number whose unit and tier law are unmeasured: no kill count
/// scores a medal, and a launched Elimination keeps the race's own kill target
/// rather than ending on the first kill.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn an_elimination_cell_awards_nothing_it_cannot_measure() {
    let Some(hd) = open() else { return };
    let elimination = cell(&hd.grids, "grid8_3_2");
    assert_eq!(elimination.mode, Mode::Elimination);
    for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
        for kills in [1, 5, 200, 400] {
            assert_eq!(
                elimination.evaluate_medal_for_difficulty(kills, difficulty),
                None
            );
        }
    }
    let plan = launch::plan_cell(elimination, "venom", |mode, id| entry(&hd, mode, id))
        .expect("it launches");
    assert_eq!(plan.mode, oag_race::Mode::Eliminator);
    assert_eq!(plan.eliminator_kill_target, None);
}

fn entry(hd: &Hd, mode: oag_race::Mode, id: &str) -> Option<String> {
    let list = if mode == oag_race::Mode::Zone {
        &hd.zone
    } else {
        &hd.race
    };
    list.iter()
        .find(|track| track.id == id)
        .map(oag_raceplay::catalogue::Track::entry_name)
}

/// Every cell launches, or is refused for `NitroBattle`/`Detonator`, the two
/// modes with no rules here; none names a circuit the disc does not resolve.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_cell_launches_or_is_refused_for_a_mode_with_no_rules() {
    let Some(hd) = open() else { return };
    let mut planned = 0;
    let mut refused = 0;
    for cell in hd.grids.iter().flat_map(|grid| &grid.cells) {
        match launch::plan_cell(cell, "venom", |mode, id| entry(&hd, mode, id)) {
            Ok(_) => planned += 1,
            Err(Refusal::Mode(mode)) => {
                assert!(mode == "NitroBattle" || mode == "Detonator", "{mode}");
                refused += 1;
            }
            Err(other) => panic!("{}: {other}", cell.name),
        }
    }
    assert_eq!((planned, refused), (142, 25));
}

/// Medal, record, unlock: a Time Trial flown at novice scores gold, the row
/// survives `records.toml`'s own round trip under the title's key, and
/// `grid0`'s `RequiredPoints` met by those medals is what opens `grid1`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_medal_persists_and_the_points_it_earns_open_the_next_grid() {
    let Some(hd) = open() else { return };
    let grid0 = &hd.grids[0];
    assert_eq!(hd.grids[1].unlock_grid.as_deref(), Some("grid0"));
    assert_eq!(grid0.required_points, 10);

    let mut store = Store::default();
    assert!(!race_campaign::grid_points_met(
        &hd.grids,
        "grid0",
        &campaign_medal_of(&store, TITLE)
    ));

    let time_trial = cell(&hd.grids, "grid0_2_2");
    let medal = time_trial
        .evaluate_medal_for_difficulty(11900, Difficulty::Easy)
        .expect("gold at novice");
    store.record_campaign(
        TITLE,
        &time_trial.name,
        Some(to_record(medal)),
        Some(to_record_difficulty(Difficulty::Easy)),
    );
    let text = toml::to_string_pretty(&store).expect("serialises");
    let (reloaded, notes) = records::parse(&text).expect("parses back");
    assert!(notes.is_empty(), "{notes:?}");
    let row = reloaded
        .campaign_medal(TITLE, &time_trial.name)
        .expect("the row survives");
    assert_eq!(row.best_medal, Some(records::Medal::Gold));
    assert_eq!(row.best_difficulty, Some(records::Difficulty::Easy));
    assert!(
        reloaded
            .campaign_medal("wipeout pulse", &time_trial.name)
            .is_none()
    );

    let mut store = reloaded;
    let met = |store: &Store| {
        race_campaign::grid_points_met(&hd.grids, "grid0", &campaign_medal_of(store, TITLE))
    };
    for cell in grid0.cells.iter().filter(|c| c.name != time_trial.name) {
        let earned = grid0.points_earned(&campaign_medal_of(&store, TITLE));
        assert_eq!(met(&store), earned >= grid0.required_points);
        store.record_campaign(TITLE, &cell.name, Some(records::Medal::Gold), None);
    }
    assert_eq!(grid0.points_earned(&campaign_medal_of(&store, TITLE)), 18);
    assert!(met(&store));
}
