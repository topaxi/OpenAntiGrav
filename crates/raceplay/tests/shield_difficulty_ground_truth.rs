//! A race's craft start on the shield slot of the difficulty being raced.
//!
//! **`#[ignore]`d and never run in CI.** It needs the HD disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay shield_difficulty_ground_truth:: --run-ignored all
//! ```
//!
//! `<Misc>` authors three shield slots, `easyshield`, `mediumshield` and `hardshield`, and plain
//! `shield` fills any the file leaves out (`docs/formats/handling-stats.md`, "Which difficulty
//! picks the shield slot"). HD authors `easyshield` for every team and neither of the others, so
//! a Novice field starts above an Elite one and Elite and Ace start on `shield`. The expected
//! values are read out of the team's own `handlingstats.xml` here, not written down.

use oag_raceplay as race;
use oag_raceplay::Race;
use oag_tables::handling::{self, Stats};

const IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";
const TRACK: &str = r"Data\Environments\Talons_Junction\track.vex";

fn started(
    image: &std::path::Path,
    difficulty: oag_ai::Difficulty,
    skill_level: Option<u8>,
) -> (Race, Vec<String>) {
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(TRACK.to_string()),
        difficulty,
        skill_level,
        // The pools do not read the track's art; the derived ribbon skips the `.rcsmodel` decode
        // that is most of an HD load.
        ribbon: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    let teams = loaded.setup.slot_teams.clone();
    (Race::start(loaded.setup), teams)
}

/// `team`'s `<Misc>`, as its own file authors it.
fn authored(image: &std::path::Path, team: &str) -> handling::Misc {
    let archives = oag_hd::open(&image.display().to_string()).expect("opening HD's archives");
    authored_in(archives, oag_hd::TITLE, team)
}

fn authored_in(
    mut archives: oag_assets::Archives,
    title: &oag_title::Title,
    team: &str,
) -> handling::Misc {
    let name = handling::entry_name_in(title.race.handling_dir_for(team), team);
    let blob = archives
        .read_name(&name)
        .unwrap_or_else(|e| panic!("reading {name}: {e}"));
    let stats: Stats = handling::from_blob(&blob).unwrap_or_else(|e| panic!("{name}: {e}"));
    stats.misc
}

/// Every active craft's starting pool against `expected(its team's <Misc>)`, and how many craft
/// started on a pool that differs from plain `shield`.
fn check_pools(
    image: &std::path::Path,
    race: &Race,
    teams: &[String],
    label: &str,
    expected: impl Fn(&handling::Misc) -> f32,
) -> usize {
    let mut off_shield = 0;
    for (slot, team) in teams.iter().enumerate() {
        let ship = &race.sim.world.ships[slot];
        if !ship.active {
            continue;
        }
        let misc = authored(image, team);
        let want = expected(&misc);
        assert_eq!(
            ship.handling.dimensions.shield, want,
            "{label}: slot {slot} ({team}) pool maximum"
        );
        assert_eq!(
            ship.physics.shield, want,
            "{label}: slot {slot} ({team}) starts full"
        );
        off_shield += usize::from(want != misc.shield);
    }
    off_shield
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_novice_field_starts_on_each_teams_easyshield() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let (race, teams) = started(&image, oag_ai::Difficulty::Novice, None);
    let moved = check_pools(&image, &race, &teams, "Novice", |misc| {
        misc.easyshield.expect("HD authors easyshield")
    });
    assert!(
        moved > 0,
        "no craft's easyshield differs from its shield: the check proves nothing"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn skilled_elite_and_ace_fields_keep_the_plain_shield() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    for difficulty in [
        oag_ai::Difficulty::Skilled,
        oag_ai::Difficulty::Elite,
        oag_ai::Difficulty::Ace,
    ] {
        let (race, teams) = started(&image, difficulty, None);
        let moved = check_pools(&image, &race, &teams, difficulty.name(), |misc| {
            assert!(
                misc.mediumshield.is_none() && misc.hardshield.is_none(),
                "HD authors a medium or hard slot; this test's premise is gone"
            );
            misc.shield
        });
        assert_eq!(moved, 0, "{}: a craft left plain shield", difficulty.name());
    }
}

/// A campaign cell names its own rung, which wins over the race box's difficulty.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_launch_that_names_a_rung_takes_it_over_the_difficulty() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let (race, teams) = started(&image, oag_ai::Difficulty::Ace, Some(0));
    let easy = check_pools(&image, &race, &teams, "rung 0 under Ace", |misc| {
        misc.easyshield.expect("HD authors easyshield")
    });
    assert!(easy > 0);
    let (race, teams) = started(&image, oag_ai::Difficulty::Novice, Some(1));
    let medium = check_pools(&image, &race, &teams, "rung 1 under Novice", |misc| {
        misc.shield
    });
    assert_eq!(medium, 0);
}

/// Omega authors the same `easyshield`, but nobody has measured which difficulty its executable
/// reads, so a Novice field starts where an Elite one does and the loader report says so.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_keeps_the_medium_slot_until_it_is_measured() {
    let Some(image) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let load = |difficulty| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            mode: oag_race::Mode::SingleRace,
            difficulty,
            ribbon: true,
            ..race::Options::default()
        })
        .expect("loading the race");
        let report = loaded.report.clone();
        let teams = loaded.setup.slot_teams.clone();
        let race = Race::start(loaded.setup);
        let pools: Vec<f32> = (0..teams.len())
            .filter(|&slot| race.sim.world.ships[slot].active)
            .map(|slot| race.sim.world.ships[slot].handling.dimensions.shield)
            .collect();
        (pools, report)
    };
    let (novice, report) = load(oag_ai::Difficulty::Novice);
    let (elite, quiet) = load(oag_ai::Difficulty::Elite);
    assert_eq!(novice, elite, "Omega's Novice field left the Medium slot");
    assert!(
        report.iter().any(|line| line.starts_with("shield: ")),
        "the loader report does not name the held slot: {report:?}"
    );
    assert!(
        !quiet.iter().any(|line| line.starts_with("shield: ")),
        "an Elite race has nothing to report: {quiet:?}"
    );
}

/// Pulse reads the difficulty into the slot too: the same `Ship_SetShield` indexes the same three
/// slots, and its Novice field starts on `easyshield`.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulses_novice_field_starts_on_each_teams_easyshield() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = image.display().to_string();
    let load = |difficulty| {
        let loaded = race::load(&race::Options {
            source: source.clone(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            difficulty,
            ribbon: true,
            ..race::Options::default()
        })
        .expect("loading the race");
        let teams = loaded.setup.slot_teams.clone();
        (Race::start(loaded.setup), teams)
    };
    for (difficulty, easy) in [
        (oag_ai::Difficulty::Novice, true),
        (oag_ai::Difficulty::Elite, false),
    ] {
        let (race, teams) = load(difficulty);
        let mut moved = 0;
        for (slot, team) in teams.iter().enumerate() {
            if !race.sim.world.ships[slot].active {
                continue;
            }
            let misc = authored_in(
                oag_pulse::open(&source).expect("opening Pulse's archives"),
                oag_pulse::TITLE,
                team,
            );
            let want = if easy {
                misc.easyshield.expect("Pulse authors easyshield")
            } else {
                misc.shield
            };
            assert_eq!(
                race.sim.world.ships[slot].handling.dimensions.shield,
                want,
                "{}: slot {slot} ({team})",
                difficulty.name()
            );
            moved += usize::from(want != misc.shield);
        }
        assert_eq!(moved > 0, easy, "{}", difficulty.name());
    }
}
