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
//!
//! Four more axes, by environment, so one slice set serves every title:
//! `OAG_SWEEP_TITLE` (`pulse`, the default, `pure`, `hd`, `2048`, `omega`),
//! `OAG_SWEEP_LEVEL` (a [`oag_ai::Difficulty`] name, `ace` by default),
//! `OAG_SWEEP_TICKS` (the window, [`TICKS`] by default) and `OAG_SWEEP_ONLY`
//! (a comma list of track ids). A layout that does not load at a slice's
//! class is skipped with a line on stderr. `OAG_SWEEP_PROFILE=<bins>` also
//! writes `<slice>.profile.tsv`, where each craft spends its flying laps by
//! line-index bin ([`PROFILE_HEADER`]).
//!
//! A third scenario, `player`, is the field with slot 0 flown by the
//! operator's autopilot at the top level, so the opponents race a craft that
//! keeps up, as a human's would. `campaign_reference_laps` prints the
//! original's own reference: every campaign Speed Lap and Time Trial gold.

use std::fmt::Write as _;
use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// Five simulated minutes, the window every other AI board uses.
const TICKS: u64 = 18_000;

/// The columns. Shield is reported twice and each is labelled: `lap_shield`
/// is the mean pool lost per completed lap (wall, roll and pad moves netted),
/// and `end_shield` is the pool when the run stops.
///
/// `mean_lap_s` is over clean flying laps (no rescue); `any_lap_s` is over every
/// flying lap, so a craft that is rescued every lap still shows how slow it is.
/// `plan` says whether the race follows a verified speed plan or the corner
/// model.
const HEADER: &str = "title\tlevel\tscenario\tcircuit\tdir\tclass\tplan\tcraft\tclean_laps\t\
                      best_lap_s\tmean_lap_s\tany_lap_s\tcontact_ticks\trespawns\tdeaths\t\
                      destroyed\tfinished\tlap_shield\tend_shield";

/// `OAG_SWEEP_PROFILE`'s columns: per craft and line-index bin, the seconds a
/// flying lap spends there, the mean and least forward speed, the wall-contact
/// ticks, and the least plan target over the bin (`NaN` off a plan).
const PROFILE_HEADER: &str = "title\tlevel\tscenario\tcircuit\tdir\tclass\tcraft\tfrom\t\
                              seconds_per_lap\tmean_speed\tleast_speed\tcontact_ticks\tplan_min";

/// The title under measurement, from `OAG_SWEEP_TITLE`.
fn title() -> String {
    std::env::var("OAG_SWEEP_TITLE").unwrap_or_else(|_| "pulse".to_string())
}

/// The opponents' level, from `OAG_SWEEP_LEVEL`.
fn level() -> oag_ai::Difficulty {
    std::env::var("OAG_SWEEP_LEVEL")
        .ok()
        .map(|name| {
            oag_ai::Difficulty::from_name(&name)
                .unwrap_or_else(|| panic!("OAG_SWEEP_LEVEL={name} is no level"))
        })
        .unwrap_or(oag_ai::Difficulty::Ace)
}

fn ticks() -> u64 {
    std::env::var("OAG_SWEEP_TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(TICKS)
}

/// The source `OAG_SWEEP_TITLE` names: an image, or the extracted package
/// the title's own ground truths read.
fn image() -> Option<PathBuf> {
    match title().as_str() {
        "pulse" => oag_testdata::image("data/images/pulse-psp-usa.chd"),
        "pure" => oag_testdata::image("data/images/pure-psp-usa.chd"),
        "hd" => oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso"),
        "2048" => oag_testdata::exact("data/extracted/vita/PCSF00007"),
        "omega" => oag_testdata::exact("data/extracted/ps4"),
        other => panic!("OAG_SWEEP_TITLE={other} is no title"),
    }
}

/// `(id, entry name)` for every layout in one direction, off the title's own
/// plugin definition (the RACE REMIX page's reader, which knows each title's
/// file), narrowed by `OAG_SWEEP_ONLY`.
fn layouts(reversed: bool) -> Vec<(String, String)> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let only: Option<Vec<String>> = std::env::var("OAG_SWEEP_ONLY")
        .ok()
        .map(|v| v.split(',').map(str::to_string).collect());
    oag_game::remix::catalogue(&image.display().to_string())
        .expect("cataloguing the source")
        .tracks
        .into_iter()
        .map(|(track, _)| track)
        .filter(|track| track.reversed == reversed)
        .filter(|track| only.as_ref().is_none_or(|only| only.contains(&track.id)))
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
    flying_laps: Vec<u64>,
    /// `OAG_SWEEP_PROFILE`: per line-index bin over flying laps, `(ticks,
    /// summed forward speed, least forward speed, wall-contact ticks)`.
    bins: Vec<(u32, f32, f32, u32)>,
    contacts_before: u32,
}

/// The profile's bin width in line samples, from `OAG_SWEEP_PROFILE`, or
/// `None` when no profile is wanted.
fn profile_bin() -> Option<usize> {
    std::env::var("OAG_SWEEP_PROFILE")
        .ok()
        .map(|v| v.parse().unwrap_or(100))
}

/// Runs one layout at one class and returns one TSV row per scenario line.
/// Runs one layout at one class: its TSV rows, and its profile rows (empty
/// unless `OAG_SWEEP_PROFILE` is set).
fn run(scenario: &str, id: &str, entry: &str, reversed: bool, class: &str) -> (String, String) {
    let Some(image) = image() else {
        return (String::new(), String::new());
    };
    let Ok(loaded) = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level(),
        track: Some(entry.to_string()),
        ..race::Options::default()
    }) else {
        eprintln!("skipping {entry} at {class}: it does not load");
        return (String::new(), String::new());
    };
    let mut race = race::Race::start(loaded.setup);
    // `player`: the field with slot 0 flown by the operator's autopilot at the
    // top level, standing in for a human who keeps up; the opponents are still
    // the measured craft. Otherwise slot 0 sits out.
    if scenario == "player" {
        race.set_autopilot(true);
        race.set_autopilot_tuning(oag_ai::Difficulty::Ace.tune(&oag_ai::Tuning::default()));
    } else {
        race.sim.world.ships[0].active = false;
    }
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
    let plan = if race.speed_plan().is_some() {
        "plan"
    } else {
        "corner"
    };
    for tick in 0..ticks() {
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
            let contacts = race.wall_contact_ticks_of(slot);
            if let Some(width) = profile_bin()
                && craft.lap > 1
                && !ship.standing.finished()
            {
                let bin = ship.driver.index as usize / width;
                if craft.bins.len() <= bin {
                    craft.bins.resize(bin + 1, (0, 0.0, f32::INFINITY, 0));
                }
                let body = &ship.physics.body;
                let speed = body.linear_velocity.dot(body.forward()).max(0.0);
                let entry = &mut craft.bins[bin];
                entry.0 += 1;
                entry.1 += speed;
                entry.2 = entry.2.min(speed);
                entry.3 += contacts - craft.contacts_before;
            }
            craft.contacts_before = contacts;
            if ship.standing.lap != craft.lap {
                if craft.lap > 1 {
                    craft.flying_laps.push(tick - craft.lap_started);
                    if !craft.recovered {
                        craft.clean_laps.push(tick - craft.lap_started);
                    }
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
    let mut profile = String::new();
    if let Some(width) = profile_bin() {
        let level = level().name();
        for (craft, &slot) in crafts.iter().zip(&measured) {
            let laps = craft.flying_laps.len().max(1) as f32;
            for (bin, &(ticks, sum, least, contacts)) in craft.bins.iter().enumerate() {
                if ticks == 0 {
                    continue;
                }
                let from = bin * width;
                let plan_min = race.speed_plan().map_or(f32::NAN, |plan| {
                    (from..(from + width).min(plan.len()))
                        .map(|i| plan.target(i))
                        .fold(f32::INFINITY, f32::min)
                });
                let _ = writeln!(
                    profile,
                    "{}\t{level}\t{scenario}\t{id}\t{dir}\t{class}\t{slot}\t{from}\t{:.3}\t{:.1}\t{least:.1}\t{contacts}\t{plan_min:.1}",
                    title(),
                    ticks as f32 / 60.0 / laps,
                    sum / ticks as f32,
                );
            }
        }
    }
    let mut out = String::new();
    for (craft, &slot) in crafts.iter().zip(&measured) {
        let ship = &race.sim.world.ships[slot];
        let best = craft.clean_laps.iter().min().map(|&t| t as f32 / 60.0);
        let mean = if craft.clean_laps.is_empty() {
            None
        } else {
            Some(craft.clean_laps.iter().sum::<u64>() as f32 / craft.clean_laps.len() as f32 / 60.0)
        };
        let any = if craft.flying_laps.is_empty() {
            None
        } else {
            Some(
                craft.flying_laps.iter().sum::<u64>() as f32
                    / craft.flying_laps.len() as f32
                    / 60.0,
            )
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
            "{}\t{}\t{scenario}\t{id}\t{dir}\t{class}\t{plan}\t{slot}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{lap_shield:.2}\t{:.2}",
            title(),
            level().name(),
            craft.clean_laps.len(),
            fmt(best),
            fmt(mean),
            fmt(any),
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
    (out, profile)
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
    let mut profile = format!("{PROFILE_HEADER}\n");
    for (id, entry) in &layouts {
        let (rows, bins) = run(scenario, id, entry, reversed, class);
        table.push_str(&rows);
        profile.push_str(&bins);
    }
    print!("{table}");
    if let Some(dir) = std::env::var_os("OAG_SWEEP_OUT") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("creating the sweep output directory");
        let dir_name = if reversed { "rev" } else { "fwd" };
        let path = dir.join(format!(
            "{}-{}-{scenario}-{class}-{dir_name}.tsv",
            title(),
            level().name()
        ));
        std::fs::write(&path, table).expect("writing the slice");
        if profile_bin().is_some() {
            std::fs::write(path.with_extension("profile.tsv"), profile)
                .expect("writing the profile");
        }
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
slice_test!(speed_plan_sweep_player_venom_fwd, "player", "VENOM", false);
slice_test!(speed_plan_sweep_player_venom_rev, "player", "VENOM", true);
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

/// The original's own reference laps: every campaign `Speed Lap` and `Time
/// Trial` cell's gold target, by circuit and class, for the title
/// `OAG_SWEEP_TITLE` names. A Speed Lap gold is one lap; a Time Trial gold is
/// the whole race, printed with its lap count. Prints only.
#[test]
#[ignore = "a sweep: set OAG_SWEEP, read the table"]
fn campaign_reference_laps() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let mut opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    println!("title\tgrid\tcell\tmode\tcircuit\tclass\tlaps\tgold_s\tgold_per_lap_s");
    for index in 0..16u8 {
        let Ok(blob) = opened
            .archives
            .read_name(&oag_pulse::campaign::entry_name(index))
        else {
            continue;
        };
        let Ok(grid) = oag_tables::race_campaign::from_blob(&blob) else {
            continue;
        };
        for cell in grid.cells {
            use oag_tables::race_campaign::Mode;
            if !matches!(cell.mode, Mode::SpeedLap | Mode::TimeTrial) {
                continue;
            }
            let gold = cell.gold as f32 / 100.0;
            let laps = if cell.mode == Mode::SpeedLap {
                1
            } else {
                cell.laps.unwrap_or(1).max(1)
            };
            println!(
                "{}\t{index}\t{}\t{:?}\t{}\t{}\t{laps}\t{gold:.2}\t{:.2}",
                title(),
                cell.name,
                cell.mode,
                cell.track.as_deref().unwrap_or("-"),
                cell.class,
                gold / laps as f32
            );
        }
    }
}

/// Where a layout's plan sits at the search's floor, and what the search could
/// not resolve: for every layout `OAG_SWEEP_ONLY` names at `OAG_SWEEP_CLASS`
/// (VENOM by default), the build report's unresolved sites and every run of
/// samples whose ceiling is at or under `OAG_FLOOR` (20 by default). Prints
/// only.
#[test]
#[ignore = "a probe: set OAG_SWEEP, read the lines"]
fn plan_floor_report() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".to_string());
    let floor: f32 = std::env::var("OAG_FLOOR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20.0);
    for reversed in [false, true] {
        for (id, entry) in layouts(reversed) {
            let Ok(loaded) = race::load(&race::Options {
                source: image.display().to_string(),
                class: class.clone(),
                mode: oag_race::Mode::SingleRace,
                track: Some(entry.clone()),
                ..race::Options::default()
            }) else {
                continue;
            };
            let race = race::Race::start(loaded.setup);
            let (plan, report) = race.build_speed_plan(1);
            let mut runs: Vec<(usize, usize, f32)> = Vec::new();
            for i in 0..plan.len() {
                let ceiling = plan.ceiling(i);
                if ceiling > floor {
                    continue;
                }
                match runs.last_mut() {
                    Some(run) if run.1 + 1 == i => {
                        run.1 = i;
                        run.2 = run.2.min(ceiling);
                    }
                    _ => runs.push((i, i, ceiling)),
                }
            }
            println!(
                "{} {id} {class}: steps {} passes {} lowerings {} respawns {} verify failures {} \
                 respawns {} lap {:?}; unresolved {:?}; ceilings <= {floor}: {:?}",
                title(),
                report.steps,
                report.passes,
                report.lowerings,
                report.respawns,
                report.verify_failures,
                report.verify_respawns,
                report.verify_lap_ticks,
                report.unresolved,
                runs
            );
        }
    }
}
