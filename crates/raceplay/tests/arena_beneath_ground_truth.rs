//! 2048's and Omega's `arena`: from about ring sample 4560 to 4790 the authored
//! line runs over a deck, and every craft drives the road 21 units under it,
//! inside the corridor, which climbs back up to the line by 4810. The race's
//! beneath rescue (`RaceSim::beneath_too_long`) must not read a craft there as on
//! a road under the circuit (`docs/gameplay/ai.md`, "A road under the
//! line is a failure").
//!
//! A field of seven per class, race to the flag: no `Beneath` respawn. And the
//! rescue that rule exists for still fires: Pulse `05_Track`'s field runs wide
//! into the pit under its upper road.

use oag_raceplay as race;
use oag_raceplay::RespawnCause;

const TICK_CAP: u32 = 30_000;

/// What one field of seven did: `Beneath` respawns per lap, every respawn's
/// cause and line index, and the field's mean shield as each lap ended and at
/// the end of the run.
struct Run {
    beneath_per_lap: Vec<u32>,
    respawns: Vec<String>,
    shield_at_lap: Vec<Vec<f32>>,
    shield_at_end: Vec<f32>,
    finished: usize,
    ticks: u32,
    laps_target: usize,
}

fn run(source: &std::path::Path, track: &str, class: &str) -> Run {
    let loaded = race::load_sim(&race::Options {
        source: source.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track.to_string()),
        seed: Some(1),
        ribbon: true,
        ..race::Options::default()
    })
    .unwrap_or_else(|error| panic!("loading {track} at {class}: {error}"));
    let mut setup = loaded.setup;
    if std::env::var_os("OAG_BENEATH_OFF").is_some() {
        setup.reset_triggers.beneath.enabled = false;
    }
    let mut race = race::Race::start(setup, oag_raceplay::ViewSetup::headless(), loaded.modules);
    let count = race.core.sim.ship_count() as usize;
    race.core.sim.world.ships[0].active = false;
    let laps_target = race.core.sim.world.laps_target().unwrap_or(3) as usize;
    let mut out = Run {
        beneath_per_lap: vec![0; laps_target],
        respawns: Vec::new(),
        shield_at_lap: vec![Vec::new(); laps_target],
        shield_at_end: Vec::new(),
        finished: 0,
        ticks: 0,
        laps_target,
    };
    let mut respawns = vec![0u32; count];
    let mut laps = vec![0u32; count];
    for tick in 0..TICK_CAP {
        race.tick(&oag_gameplay::PlayerInputs::none());
        out.ticks = tick + 1;
        let mut running = 0;
        for slot in 1..count {
            let ship = &race.core.sim.world.ships[slot];
            if !ship.active {
                continue;
            }
            let lap = ship.standing.lap;
            if lap > laps[slot] {
                let done = laps[slot] as usize;
                if (1..=laps_target).contains(&done) {
                    out.shield_at_lap[done - 1].push(ship.physics.shield);
                }
                laps[slot] = lap;
            }
            if !ship.standing.finished()
                && ship.physics.craft_state != oag_physics::CraftState::Eliminated
            {
                running += 1;
            }
            if race.core.sim.respawns_of(slot) != respawns[slot] {
                respawns[slot] = race.core.sim.respawns_of(slot);
                let cause = race.core.sim.last_respawn_cause_of(slot);
                let index = ship.driver.index;
                out.respawns
                    .push(format!("slot {slot} lap {lap} {cause:?}@{index}"));
                if cause == Some(RespawnCause::Beneath) {
                    let at = lap.max(1) as usize - 1;
                    if out.beneath_per_lap.len() <= at {
                        out.beneath_per_lap.resize(at + 1, 0);
                    }
                    out.beneath_per_lap[at] += 1;
                }
            }
        }
        if running == 0 {
            break;
        }
    }
    for slot in 1..count {
        let ship = &race.core.sim.world.ships[slot];
        if ship.active {
            out.shield_at_end.push(ship.physics.shield);
            out.finished += usize::from(ship.standing.finished());
            if !ship.standing.finished() {
                out.respawns.push(format!(
                    "unfinished slot {slot} lap {} idx {} v {:.1} airborne {:.1} state {:?} given-up {}",
                    ship.standing.lap,
                    ship.driver.index,
                    ship.physics.body.linear_velocity.length(),
                    ship.physics.time_airborne,
                    ship.physics.craft_state,
                    race.core.sim.respawn_given_up_of(slot)
                ));
            }
        }
    }
    out
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        f32::NAN
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn check(title: &str, source: &std::path::Path, track: &str, class: &str) {
    let out = run(source, track, class);
    let per_lap: Vec<String> = out
        .shield_at_lap
        .iter()
        .map(|shields| format!("{:.1}", mean(shields)))
        .collect();
    println!(
        "{title} arena {class}: laps {}, beneath per lap {:?}, finished {}/7 in {} ticks, \
         mean shield per lap [{}], at end {:.1}, respawns {:?}",
        out.laps_target,
        out.beneath_per_lap,
        out.finished,
        out.ticks,
        per_lap.join(", "),
        mean(&out.shield_at_end),
        out.respawns
    );
    assert_eq!(
        out.beneath_per_lap.iter().sum::<u32>(),
        0,
        "{title} arena {class}: a craft on the road under the line was rescued as if under the circuit"
    );
}

fn vita_2048() -> Option<std::path::PathBuf> {
    oag_testdata::exact("data/extracted/vita/PCSF00007")
}

fn omega() -> Option<std::path::PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

const TRACK_2048: &str = r"Data\art\published\environments\arena\track.vex";
const TRACK_OMEGA: &str = r"Data\environments2048\arena\track.vex";

macro_rules! arena_classes {
    ($($name:ident => $source:ident, $title:literal, $track:ident, $class:literal;)*) => {$(
        #[test]
        #[ignore = "needs the extracted game under data/extracted/"]
        fn $name() {
            let Some(source) = $source() else {
                return;
            };
            check($title, &source, $track, $class);
        }
    )*};
}

arena_classes! {
    the_2048_arena_field_is_not_rescued_from_its_own_road_at_venom => vita_2048, "2048", TRACK_2048, "VENOM";
    the_2048_arena_field_is_not_rescued_from_its_own_road_at_flash => vita_2048, "2048", TRACK_2048, "FLASH";
    the_2048_arena_field_is_not_rescued_from_its_own_road_at_rapier => vita_2048, "2048", TRACK_2048, "RAPIER";
    the_2048_arena_field_is_not_rescued_from_its_own_road_at_phantom => vita_2048, "2048", TRACK_2048, "PHANTOM";
    the_omega_arena_field_is_not_rescued_from_its_own_road_at_venom => omega, "omega", TRACK_OMEGA, "VENOM";
    the_omega_arena_field_is_not_rescued_from_its_own_road_at_flash => omega, "omega", TRACK_OMEGA, "FLASH";
    the_omega_arena_field_is_not_rescued_from_its_own_road_at_rapier => omega, "omega", TRACK_OMEGA, "RAPIER";
    the_omega_arena_field_is_not_rescued_from_its_own_road_at_phantom => omega, "omega", TRACK_OMEGA, "PHANTOM";
}

/// Scratch census: every respawn by cause, line index and tick on every
/// Pulse, 2048 and Omega layout, a field of seven at `OAG_SWEEP_CLASS`, plus
/// the final race hash. Each `Beneath` rescue also says whether the craft was
/// outside its corridor on the tick before, and whether it had left the
/// corridor to get under the line (the speed plan's latch). Prints rows
/// `CENSUS\t...`; set `OAG_SWEEP` and `OAG_TITLE` (`pulse`, `2048`, `omega`).
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn beneath_census_all_titles() {
    use oag_core::math::Vec3;
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".into());
    let title = std::env::var("OAG_TITLE").unwrap_or_else(|_| "2048".into());
    let names = [
        "square",
        "park",
        "tower",
        "mall",
        "bridge",
        "arena",
        "subway",
        "cathedral",
        "sol",
        "altima",
    ];
    let circuits: Vec<(String, String, String)> = match title.as_str() {
        "pulse" => {
            let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
                return;
            };
            let source = image.display().to_string();
            let mut archives = oag_pulse::open(&source).unwrap();
            let blob = archives
                .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
                .unwrap();
            let definition = oag_tables::fexml::expand(&blob).unwrap();
            oag_raceplay::catalogue::tracks(&definition)
                .into_iter()
                .map(|t| (format!("pulse/{}", t.id), source.clone(), t.entry_name()))
                .collect()
        }
        "2048" => {
            let Some(source) = vita_2048() else { return };
            names
                .iter()
                .map(|n| {
                    (
                        format!("2048/{n}"),
                        source.display().to_string(),
                        format!(r"Data\art\published\environments\{n}\track.vex"),
                    )
                })
                .collect()
        }
        _ => {
            let Some(source) = omega() else { return };
            names
                .iter()
                .map(|n| {
                    (
                        format!("omega/{n}"),
                        source.display().to_string(),
                        format!(r"Data\environments2048\{n}\track.vex"),
                    )
                })
                .collect()
        }
    };
    for (label, source, track) in circuits {
        let loaded = match race::load_sim(&race::Options {
            source,
            class: class.clone(),
            mode: oag_race::Mode::SingleRace,
            difficulty: oag_ai::Difficulty::Ace,
            track: Some(track),
            seed: Some(1),
            ribbon: true,
            ..race::Options::default()
        }) {
            Ok(loaded) => loaded,
            Err(error) => {
                println!("CENSUS\t{label}\t{class}\terror {error:#}");
                continue;
            }
        };
        let mut race = race::Race::start(
            loaded.setup,
            oag_raceplay::ViewSetup::headless(),
            loaded.modules,
        );
        let count = race.core.sim.ship_count() as usize;
        race.core.sim.world.ships[0].active = false;
        let mut respawns = vec![0u32; count];
        let mut wide = vec![false; count];
        let mut outside_before = vec![false; count];
        let mut wide_before = vec![false; count];
        let mut events = Vec::new();
        let mut ticks = 0;
        for tick in 0..TICK_CAP {
            race.tick(&oag_gameplay::PlayerInputs::none());
            ticks = tick + 1;
            let mut running = 0;
            for slot in 1..count {
                let ship = &race.core.sim.world.ships[slot];
                if !ship.active {
                    continue;
                }
                if !ship.standing.finished() {
                    running += 1;
                }
                if race.core.sim.respawns_of(slot) != respawns[slot] {
                    respawns[slot] = race.core.sim.respawns_of(slot);
                    let cause = race.core.sim.last_respawn_cause_of(slot);
                    let mut event = format!("{cause:?}@{}t{tick}", ship.driver.index);
                    if cause == Some(RespawnCause::Beneath) {
                        event.push_str(&format!(
                            "[out {} wide {}]",
                            outside_before[slot], wide_before[slot]
                        ));
                    }
                    events.push(event);
                    wide[slot] = false;
                }
                let line = race.core.sim.line_of(slot);
                let index = ship.driver.index as usize;
                let under = ship.physics.time_airborne <= 0.0
                    && !line.is_unsupported(index)
                    && race
                        .core
                        .sim
                        .ai_sample_for(slot, index)
                        .is_some_and(|sample| {
                            let up = -Vec3::from_array(sample.down).normalize_or_zero();
                            (line.point(index) - ship.physics.body.position).dot(up)
                                > oag_race::recovery::BENEATH_LINE
                        });
                let outside = line.aim(index, 0.0).corridor.is_some_and(|frame| {
                    let offset =
                        (ship.physics.body.position - line.point(index)).dot(frame.lateral);
                    offset < frame.left || offset > frame.right
                });
                if !under {
                    wide[slot] = outside || (wide[slot] && ship.physics.time_airborne > 0.0);
                }
                outside_before[slot] = outside;
                wide_before[slot] = wide[slot];
            }
            if running == 0 {
                break;
            }
        }
        let finished: Vec<String> = (1..count)
            .map(|slot| {
                let s = &race.core.sim.world.ships[slot].standing;
                format!("{}{}", s.lap, if s.finished() { "f" } else { "" })
            })
            .collect();
        // The field's bodies bit for bit: the race hash also covers the
        // rescue's own bookkeeping, which a change to the rescue may add to.
        let bodies = (1..count).fold(0u64, |acc, slot| {
            let body = &race.core.sim.world.ships[slot].physics.body;
            [body.position, body.linear_velocity]
                .iter()
                .flat_map(|v| v.to_array())
                .fold(acc, |acc, x| acc.rotate_left(7) ^ u64::from(x.to_bits()))
        });
        println!(
            "CENSUS\t{label}\t{class}\tticks {ticks}\tbodies {bodies:016x}\thash {:016x}\tlaps {}\t{}",
            race.core.sim.state_hash(),
            finished.join(","),
            events.join(" ")
        );
    }
}

/// The rescue still fires where a craft really is under the circuit: Pulse
/// `05_Track` forward, FLASH, where one of the field leaves the corridor and
/// drops into the pit under the upper road (slot 2 on lap 2, ring sample 350).
/// Two did with a blast drained along forward; the side drain Pulse measures
/// (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`, "The drain pushes
/// along the side row") leaves one.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_pulse_05_track_field_is_still_rescued_from_the_pit_under_its_road() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let track = oag_raceplay::catalogue::tracks(&definition)
        .into_iter()
        .find(|t| t.id == "05_Track")
        .expect("05_Track is on the disc")
        .entry_name();
    let out = run(&image, &track, "FLASH");
    println!("pulse 05_Track FLASH: respawns {:?}", out.respawns);
    assert_eq!(
        out.beneath_per_lap.iter().sum::<u32>(),
        1,
        "05_Track's pit rescues changed: {:?}",
        out.respawns
    );
}
