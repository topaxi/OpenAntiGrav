//! The before-and-after matrix for the AI speed plan: every layout on the disc,
//! forward and reversed, at every speed class, with one craft alone and with a
//! full field of seven.
//!
//! **`#[ignore]`d, gated on `OAG_SWEEP`, printing rather than asserting**, the
//! same contract `ai_clean_lap_board.rs` keeps and for the same reason: 192 rows
//! of up to five simulated minutes is not a `just test-data` cost. The asserting
//! ratchet is `ai_clean_lap_gate.rs`. Split by class, direction and scenario so
//! nextest runs the sixteen slices in parallel rather than one matrix on one
//! core (`CLAUDE.md`'s test-budget rules):
//!
//! ```sh
//! OAG_SWEEP=1 OAG_SWEEP_OUT=data/scratch/<lane>/after OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all speed_plan_sweep
//! ```
//!
//! Each slice writes `<OAG_SWEEP_OUT>/<slice>.tsv`, one row per circuit. The
//! columns are documented on [`HEADER`].

use std::fmt::Write as _;
use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

/// Five simulated minutes, the window every other AI board uses.
const TICKS: u64 = 18_000;

/// The columns. Shield is reported twice and each is labelled: `lap_shield`
/// is the mean pool lost per completed lap (wall, roll and pad moves netted),
/// and `end_shield` is the pool when the run stops.
const HEADER: &str = "scenario\tcircuit\tdir\tclass\tcraft\tclean_laps\tbest_lap_s\tmean_lap_s\t\
                      contact_ticks\trespawns\tdeaths\tdestroyed\tfinished\tlap_shield\tend_shield";

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// `(id, entry name)` for every layout in one direction.
fn layouts(reversed: bool) -> Vec<(String, String)> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| track.reversed == reversed)
        .map(|track| (track.id.clone(), track.entry_name()))
        .collect()
}

/// What one craft did over a run.
#[derive(Default)]
struct Craft {
    clean_laps: Vec<u64>,
    lap_started: u64,
    lap: u32,
    recovered: bool,
    respawns_before: u32,
    shield_at_lap: Vec<f32>,
}

/// Runs one layout at one class and returns one TSV row per scenario line.
fn run(scenario: &str, id: &str, entry: &str, reversed: bool, class: &str) -> String {
    let Some(image) = image() else {
        return String::new();
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|error| panic!("loading {entry} at {class}: {error}"));
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].active = false;
    let measured: Vec<usize> = if scenario == "lone" {
        for slot in 2..8 {
            race.sim.world.ships[slot].active = false;
        }
        vec![1]
    } else {
        (1..race.sim.world.ship_count as usize).collect()
    };
    // Diagnostic: every measured craft a plain line-follower (seed 0, no
    // personality), to separate the plan from what a pilot does to it.
    if std::env::var_os("OAG_SWEEP_PLAIN").is_some() {
        for &slot in &measured {
            race.sim.world.ships[slot].driver.seed = 0;
        }
    }
    let mut crafts: Vec<Craft> = measured
        .iter()
        .map(|&slot| Craft {
            lap: race.sim.world.ships[slot].standing.lap,
            shield_at_lap: vec![race.sim.world.ships[slot].physics.shield],
            ..Craft::default()
        })
        .collect();
    for tick in 0..TICKS {
        for (craft, &slot) in crafts.iter_mut().zip(&measured) {
            craft.respawns_before = race.respawns_of(slot);
        }
        race.tick(&PlayerInputs::none());
        let mut live = false;
        for (craft, &slot) in crafts.iter_mut().zip(&measured) {
            if race.respawns_of(slot) != craft.respawns_before {
                craft.recovered = true;
                if std::env::var_os("OAG_SWEEP_CAUSES").is_some() {
                    let ship = &race.sim.world.ships[slot];
                    eprintln!(
                        "respawn {id} {class} slot {slot} tick {tick} index {} cause {:?}",
                        ship.driver.index,
                        race.last_respawn_cause_of(slot)
                    );
                }
            }
            let ship = &race.sim.world.ships[slot];
            if ship.standing.lap != craft.lap {
                if craft.lap > 1 && !craft.recovered {
                    craft.clean_laps.push(tick - craft.lap_started);
                }
                craft.shield_at_lap.push(ship.physics.shield);
                craft.lap_started = tick;
                craft.lap = ship.standing.lap;
                craft.recovered = false;
            }
            live |= ship.physics.craft_state == oag_physics::CraftState::Racing
                && !ship.standing.finished();
        }
        if !live {
            break;
        }
    }
    let dir = if reversed { "rev" } else { "fwd" };
    let mut out = String::new();
    for (craft, &slot) in crafts.iter().zip(&measured) {
        let ship = &race.sim.world.ships[slot];
        let best = craft.clean_laps.iter().min().map(|&t| t as f32 / 60.0);
        let mean = if craft.clean_laps.is_empty() {
            None
        } else {
            Some(craft.clean_laps.iter().sum::<u64>() as f32 / craft.clean_laps.len() as f32 / 60.0)
        };
        let lost: Vec<f32> = craft
            .shield_at_lap
            .windows(2)
            .map(|pair| pair[0] - pair[1])
            .collect();
        let lap_shield = if lost.is_empty() {
            f32::NAN
        } else {
            lost.iter().sum::<f32>() / lost.len() as f32
        };
        let fmt = |v: Option<f32>| v.map_or("-".to_string(), |v| format!("{v:.2}"));
        let _ = writeln!(
            out,
            "{scenario}\t{id}\t{dir}\t{class}\t{slot}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{lap_shield:.2}\t{:.2}",
            craft.clean_laps.len(),
            fmt(best),
            fmt(mean),
            race.wall_contact_ticks_of(slot),
            race.respawns_of(slot),
            ship.standing.deaths,
            u8::from(
                ship.physics.craft_state != oag_physics::CraftState::Racing
                    && !ship.standing.finished()
            ),
            u8::from(ship.standing.finished()),
            ship.physics.shield,
        );
    }
    out
}

fn slice(scenario: &str, class: &str, reversed: bool) {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let layouts = layouts(reversed);
    if layouts.is_empty() {
        return;
    }
    let mut table = format!("{HEADER}\n");
    for (id, entry) in &layouts {
        table.push_str(&run(scenario, id, entry, reversed, class));
    }
    print!("{table}");
    if let Some(dir) = std::env::var_os("OAG_SWEEP_OUT") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("creating the sweep output directory");
        let dir_name = if reversed { "rev" } else { "fwd" };
        let path = dir.join(format!("{scenario}-{class}-{dir_name}.tsv"));
        std::fs::write(&path, table).expect("writing the slice");
    }
}

macro_rules! slice_test {
    ($name:ident, $scenario:expr, $class:expr, $reversed:expr) => {
        #[test]
        #[ignore = "a sweep: set OAG_SWEEP (and OAG_SWEEP_OUT), read the tables"]
        fn $name() {
            slice($scenario, $class, $reversed);
        }
    };
}

slice_test!(speed_plan_sweep_lone_venom_fwd, "lone", "VENOM", false);
slice_test!(speed_plan_sweep_lone_venom_rev, "lone", "VENOM", true);
slice_test!(speed_plan_sweep_lone_flash_fwd, "lone", "FLASH", false);
slice_test!(speed_plan_sweep_lone_flash_rev, "lone", "FLASH", true);
slice_test!(speed_plan_sweep_lone_rapier_fwd, "lone", "RAPIER", false);
slice_test!(speed_plan_sweep_lone_rapier_rev, "lone", "RAPIER", true);
slice_test!(speed_plan_sweep_lone_phantom_fwd, "lone", "PHANTOM", false);
slice_test!(speed_plan_sweep_lone_phantom_rev, "lone", "PHANTOM", true);
slice_test!(speed_plan_sweep_field_venom_fwd, "field", "VENOM", false);
slice_test!(speed_plan_sweep_field_venom_rev, "field", "VENOM", true);
slice_test!(speed_plan_sweep_field_flash_fwd, "field", "FLASH", false);
slice_test!(speed_plan_sweep_field_flash_rev, "field", "FLASH", true);
slice_test!(speed_plan_sweep_field_rapier_fwd, "field", "RAPIER", false);
slice_test!(speed_plan_sweep_field_rapier_rev, "field", "RAPIER", true);
slice_test!(
    speed_plan_sweep_field_phantom_fwd,
    "field",
    "PHANTOM",
    false
);
slice_test!(speed_plan_sweep_field_phantom_rev, "field", "PHANTOM", true);
