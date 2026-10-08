//! What confirming a cell on Omega's `Cell Selection` asks a race for, against
//! the disc: every cell of every grid that parses is either mapped onto a
//! circuit Omega actually ships, or refused for a mode this engine cannot run -
//! never a circuit id that resolves to nothing.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(omega_campaign_launch_ground_truth)'
//! ```

use oag_game::campaign::launch::{self, CellPlan, Refusal};
use oag_tables::race_campaign::{self, Cell};

/// Omega's raceable and Zone circuits, as the front end offers them: declared
/// by the team/track plugin definition, kept only where the archive set holds
/// the geometry. This mirrors `boot::roster::load_tracks` and
/// `load_zone_tracks` (both private to the boot), because
/// `oag_game::remix::catalogue` answers only the raceable list and the Zone
/// split is half of what a cell needs resolved.
struct Catalogue {
    race: Vec<oag_raceplay::catalogue::Track>,
    zone: Vec<oag_raceplay::catalogue::Track>,
}

impl Catalogue {
    fn entry(&self, mode: oag_race::Mode, id: &str) -> Option<String> {
        let list = if mode == oag_race::Mode::Zone {
            &self.zone
        } else {
            &self.race
        };
        list.iter()
            .find(|track| track.id == id)
            .map(oag_raceplay::catalogue::Track::entry_name)
    }
}

struct Omega {
    catalogue: Catalogue,
    /// How many of the listed grids parsed.
    grids: usize,
    /// Every cell of every parsed grid, with the grid's file index.
    cells: Vec<(usize, Cell)>,
}

fn open() -> Option<Omega> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    let mut archives = oag_omega::open(&source.display().to_string()).expect("open omega");
    let title = oag_omega::TITLE;
    let mut documents = Vec::new();
    for name in [
        title.plugin_definition,
        title
            .track_plugin_definition
            .unwrap_or(title.plugin_definition),
    ] {
        let blob = archives.read_name(name).expect("plugin definition");
        let xml = oag_tables::fexml::text(&blob).expect("expand plugin definition");
        if !documents.contains(&xml) {
            documents.push(xml);
        }
    }
    documents.extend(archives.manifests.iter().cloned());
    let offered =
        |track: &oag_raceplay::catalogue::Track| archives.locate(&track.entry_name()).is_some();
    let race_all = oag_raceplay::catalogue::all_tracks(&documents);
    let zone_all = title.race.zone.menu_tracks(
        &race_all,
        |track| track.available_in_zone,
        |names| oag_raceplay::catalogue::all_tracks_named(&documents, names),
    );
    let race: Vec<_> = race_all.iter().filter(|t| offered(t)).cloned().collect();
    let zone: Vec<_> = zone_all.into_iter().filter(|t| offered(t)).collect();

    let definition = archives
        .read_name(oag_omega::campaign::DEFINITION_ENTRY)
        .expect("grids/Definition.xml");
    let definition = oag_tables::fexml::text(&definition).expect("expand Definition.xml");
    let mut cells = Vec::new();
    let mut grids = 0;
    for (index, src) in race_campaign::definition_entries(&definition)
        .into_iter()
        .enumerate()
    {
        let Ok(blob) = archives.read_name(&src) else {
            continue;
        };
        let Ok(grid) = race_campaign::from_blob(&blob) else {
            continue;
        };
        grids += 1;
        cells.extend(grid.cells.into_iter().map(|cell| (index, cell)));
    }
    Some(Omega {
        catalogue: Catalogue { race, zone },
        grids,
        cells,
    })
}

fn plan(omega: &Omega, cell: &Cell) -> Result<CellPlan, Refusal> {
    launch::plan_cell(cell, "venom", |mode, id| omega.catalogue.entry(mode, id))
}

/// The first cell of the first grid - the one the live walk confirmed - asks
/// for exactly what it authors: a single race on `01_Track`, Venom, three
/// laps, and `01_Track` is Vineta K on Omega's own disc.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_first_cell_launches_vineta_k_as_a_venom_race_of_three_laps() {
    let Some(omega) = open() else { return };
    let (_, cell) = omega.cells.first().expect("a parsed grid");
    let plan = plan(&omega, cell).expect("the first cell launches");
    assert_eq!(plan.mode, oag_race::Mode::SingleRace);
    assert_eq!(
        plan.leg_entries,
        [r"Data\Environments\01_Vineta_K\track.vex"]
    );
    assert_eq!(plan.class, "Venom");
    assert!(!plan.class_is_fallback);
    assert_eq!(plan.laps_override, Some(3));
    assert_eq!(plan.ai_count, Some(7));
}

/// Every cell either plans or is refused for its mode. A `MissingTrack` is the
/// break this test exists for: the cell names a circuit Omega's catalogue does
/// not resolve.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn every_parsed_cell_resolves_its_circuits_or_is_refused_for_its_mode() {
    let Some(omega) = open() else { return };
    // Sixteen of nineteen grids parse (16-18 lack `RequiredPoints`); a fourth
    // silently failing shows here rather than as a shorter campaign.
    assert_eq!(omega.grids, 16);
    assert_eq!(omega.cells.len(), 167);
    let mut unresolved = Vec::new();
    let mut planned = 0;
    let mut refused = Vec::new();
    for (grid, cell) in &omega.cells {
        match plan(&omega, cell) {
            Ok(_) => planned += 1,
            Err(Refusal::Mode(mode)) => refused.push(mode),
            Err(other) => unresolved.push(format!("grid {grid} {}: {other}", cell.name)),
        }
    }
    assert!(unresolved.is_empty(), "{unresolved:#?}");
    assert_eq!(planned, 142);
    assert_eq!(refused.len(), 25);
    assert!(
        refused
            .iter()
            .all(|mode| mode == "NitroBattle" || mode == "Detonator"),
        "{refused:?}"
    );
}

/// The opponents a cell authors are the opponents its mode races with, so
/// launching without an `AICount` option changes nothing: 7 on a race, an
/// elimination and a tournament, 1 head to head, none on the solo modes.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn authored_ai_counts_are_the_modes_own_field() {
    let Some(omega) = open() else { return };
    let mut checked = 0;
    for (grid, cell) in &omega.cells {
        let Ok(plan) = plan(&omega, cell) else {
            continue;
        };
        assert_eq!(
            plan.ai_count.unwrap_or(0),
            u32::from(plan.mode.opponent_count()),
            "grid {grid} {} ({:?})",
            cell.name,
            plan.mode
        );
        checked += 1;
    }
    assert!(checked > 0);
}

/// Omega's laps rule matches the class census the HD cells follow: the modes
/// that take a lap override carry the cell's own count.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn lap_overrides_are_the_cells_own_and_only_for_the_modes_that_take_one() {
    let Some(omega) = open() else { return };
    for (grid, cell) in &omega.cells {
        let Ok(plan) = plan(&omega, cell) else {
            continue;
        };
        let takes = matches!(
            plan.mode,
            oag_race::Mode::SingleRace
                | oag_race::Mode::TimeTrial
                | oag_race::Mode::Tournament
                | oag_race::Mode::Head2Head
        );
        assert_eq!(
            plan.laps_override,
            if takes { cell.laps } else { None },
            "grid {grid} {}",
            cell.name
        );
    }
}

/// Omega re-ships HD's per-difficulty schema, `NitroElim*` and all, so the
/// same guard holds: an `Elimination` or `NitroBattle` cell with a real triple
/// awards no medal on the dummy `1`/`2`/`3`, and a launched `Elimination` keeps
/// the race's own kill target instead of ending on the first kill.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn an_elimination_cell_with_a_nitro_triple_awards_nothing_it_cannot_measure() {
    let Some(omega) = open() else { return };
    let mut unmeasured = 0;
    for (grid, cell) in &omega.cells {
        if !cell.medal_law_is_unmeasured() {
            continue;
        }
        unmeasured += 1;
        for difficulty in [
            race_campaign::Difficulty::Easy,
            race_campaign::Difficulty::Medium,
            race_campaign::Difficulty::Hard,
        ] {
            assert_eq!(
                cell.evaluate_medal_for_difficulty(5, difficulty),
                None,
                "grid {grid} {}",
                cell.name
            );
        }
        if let Ok(plan) = plan(&omega, cell) {
            assert_eq!(plan.eliminator_kill_target, None, "{}", cell.name);
        }
    }
    println!("{unmeasured} Omega cells carry an unmeasured nitro law");
    assert!(unmeasured > 0);
}
