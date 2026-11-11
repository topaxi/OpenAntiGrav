//! A race's craft start on the shield slot of the difficulty being raced.
//!
//! **`#[ignore]`d and never run in CI.** It needs the HD disc image; the Omega, 2048 and Pulse
//! tests need theirs.
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

/// `team`'s `<Misc>` out of `title`'s archives, `None` where it ships no handling file.
fn try_authored(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    team: &str,
) -> Option<handling::Misc> {
    let name = handling::entry_name_in(title.race.handling_dir_for(team), team);
    let blob = archives.read_name(&name).ok()?;
    let stats: Stats = handling::from_blob(&blob).unwrap_or_else(|e| panic!("{name}: {e}"));
    Some(stats.misc)
}

/// Every active craft's pool against `expected(its team's <Misc>)`, read from `title`'s own
/// files, how many differ from plain `shield`, and the loader report.
fn pools_in(
    source: &str,
    title: &oag_title::Title,
    loaded: race::Loaded,
    label: &str,
    expected: impl Fn(&handling::Misc) -> f32,
) -> (usize, Vec<String>) {
    let report = loaded.report.clone();
    let teams = loaded.setup.slot_teams.clone();
    let race = Race::start(loaded.setup);
    let mut archives = oag_assets::Archives::open(source, title).expect("opening the archives");
    let mut moved = 0;
    for (slot, team) in teams.iter().enumerate() {
        let ship = &race.sim.world.ships[slot];
        if !ship.active {
            continue;
        }
        // A slot whose team ships no handling file flies the player's tuning (the loader says so).
        let misc = [team, &teams[0]]
            .into_iter()
            .find_map(|team| try_authored(&mut archives, title, team))
            .expect("the player's own handling file reads");
        let want = expected(&misc);
        assert_eq!(
            ship.handling.dimensions.shield, want,
            "{label}: slot {slot} ({team}) pool maximum"
        );
        assert_eq!(
            ship.physics.shield, want,
            "{label}: slot {slot} ({team}) starts full"
        );
        moved += usize::from(want != misc.shield);
    }
    (moved, report)
}

fn no_held_slot_line(report: &[String], label: &str) {
    assert!(
        !report.iter().any(|line| line.starts_with("shield: ")),
        "{label}: the loader report still names a held slot: {report:?}"
    );
}

/// Omega's `Ship_SetShield` (`0x012f3360`) clamps to `stats + 0x98 + skill * 4`, the three
/// `<Misc>` slots (`docs/ghidra/functions/ps4-omega-eu/craft-laws.md`), so its Novice field
/// starts on `easyshield` and an Elite one on plain `shield`.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omegas_novice_field_starts_on_each_teams_easyshield() {
    let Some(image) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let source = image.display().to_string();
    let load = |difficulty| {
        race::load(&race::Options {
            source: source.clone(),
            mode: oag_race::Mode::SingleRace,
            difficulty,
            ribbon: true,
            ..race::Options::default()
        })
        .expect("loading the race")
    };
    let (moved, report) = pools_in(
        &source,
        oag_omega::TITLE,
        load(oag_ai::Difficulty::Novice),
        "Omega Novice",
        |misc| misc.easyshield.expect("Omega authors easyshield"),
    );
    assert!(
        moved > 0,
        "no Omega craft's easyshield differs from its shield"
    );
    no_held_slot_line(&report, "Omega Novice");
    let (moved, report) = pools_in(
        &source,
        oag_omega::TITLE,
        load(oag_ai::Difficulty::Elite),
        "Omega Elite",
        |misc| misc.shield_for(1),
    );
    assert_eq!(moved, 0, "an Omega Elite craft left plain shield");
    no_held_slot_line(&report, "Omega Elite");
}

fn vita_2048_source() -> Option<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path.display().to_string());
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// 2048's `Ship_SetShield` (`0x811bb524`) makes the same read
/// (`docs/ghidra/functions/vita-2048-eu-v104/craft-laws.md`): a Novice race box starts on
/// `easyshield`.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_novice_field_starts_on_each_teams_easyshield() {
    let Some(source) = vita_2048_source() else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: source.clone(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        ribbon: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    let (moved, report) = pools_in(&source, oag_2048::TITLE, loaded, "2048 Novice", |misc| {
        misc.shield_for(0)
    });
    assert!(
        moved > 0,
        "no 2048 craft's easyshield differs from its shield"
    );
    no_held_slot_line(&report, "2048 Novice");
}

/// A 2048 campaign event races at its own level, `1`, whatever the race box's difficulty says:
/// `GameModeBase_Apply` (`0x812b0d52`) copies it to the global the shield read indexes with.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_event_starts_on_its_own_level_not_the_difficulty() {
    let Some(source) = vita_2048_source() else {
        return;
    };
    let options = race::Options {
        source: source.clone(),
        difficulty: oag_ai::Difficulty::Novice,
        seed: Some(1),
        ..race::Options::default()
    };
    let loaded = race::load_event(&options, "2048 - Event 4").expect("the event loads");
    let easy_differs = std::cell::Cell::new(0);
    let (_, report) = pools_in(
        &source,
        oag_2048::TITLE,
        loaded,
        "2048 - Event 4 under Novice",
        |misc| {
            easy_differs
                .set(easy_differs.get() + usize::from(misc.shield_for(0) != misc.shield_for(1)));
            misc.shield_for(1)
        },
    );
    assert!(
        easy_differs.get() > 0,
        "no craft of the event has an easyshield of its own: the check proves nothing"
    );
    no_held_slot_line(&report, "2048 - Event 4");
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
