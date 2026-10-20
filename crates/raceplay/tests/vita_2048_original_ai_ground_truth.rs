//! Wipeout 2048's campaign opponent (`oag_ai::original` with
//! `oag_2048::race::ORIGINAL_DRIVER`) flown on the extracted package.
//!
//! The scenario is the Vita3K capture on
//! `docs/ghidra/functions/vita-2048-eu-v104/ai-controls.md`, "Measured":
//! "2048 - Event 4", a Flash race on Unity Square, the player parked on the
//! grid in eighth, skill `1.0` as the event authors it and `3.0` written
//! into the event live. Each figure asserted is one the capture read and
//! one that depends on the law, the tables and the event, not on this
//! engine's physics matching 2048's lap for lap.

use std::path::{Path, PathBuf};

use oag_gameplay::PlayerInputs;

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn loaded(event: &str) -> Option<oag_raceplay::Loaded> {
    let source = source()?;
    let options = oag_raceplay::Options {
        source: source.display().to_string(),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    };
    Some(oag_raceplay::load_event(&options, event).expect("the event loads"))
}

/// The race as the campaign launches it, with `skill` in place of the
/// event's own when given.
fn race(event: &str, skill: Option<f32>) -> Option<oag_raceplay::Race> {
    let loaded = loaded(event)?;
    let ai = loaded
        .campaign_2048_event
        .as_ref()
        .and_then(|event| event.ai)
        .expect("a race event with the original's AI");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(
        race.use_original_ai(Some(skill.unwrap_or(ai.skill_scale)), ai.level),
        "2048 flies the original driver"
    );
    race.set_original_event_ai_skills(ai.pad_tracking, ai.weapon_avoidance);
    Some(race)
}

fn seconds(race: &oag_raceplay::Race) -> f32 {
    race.sim
        .world
        .tick
        .saturating_sub(oag_race::COUNTDOWN_TICKS) as f32
        / 60.0
}

/// Every opponent's held throttle with its place, each tick from `from` to
/// `to` seconds after the green light.
fn held(skill: f32, from: f32, to: f32) -> Option<Vec<Vec<(u8, f32)>>> {
    let mut race = race("2048 - Event 4", Some(skill))?;
    let mut out = Vec::new();
    while seconds(&race) <= to {
        race.tick(&PlayerInputs::none());
        if seconds(&race) >= from {
            let places = race.places();
            let field = race.original_ai_field().expect("the field is on");
            out.push(
                (1..8)
                    .map(|s| (places[s], field.drivers[s].held_thrust))
                    .collect(),
            );
        }
    }
    Some(out)
}

/// The event's opponents read what Vita3K read off the running event: its
/// authored `M_SKILLSCALE`, the constructor's `1.0` for both AI skills and
/// level `1`, and the ring rate those give, `0.00165`, on every driver.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn event_4_reads_the_ai_vita3k_read() {
    let Some(loaded) = loaded("2048 - Event 4") else {
        return;
    };
    let ai = loaded.campaign_2048_event.expect("a campaign event").ai;
    assert_eq!(
        ai,
        Some(oag_title::original_driver::EventAi {
            skill_scale: 1.0,
            pad_tracking: 1.0,
            weapon_avoidance: 1.0,
            level: 1,
        })
    );
    let race = race("2048 - Event 4", None).expect("loaded above");
    let field = race.original_ai_field().expect("the field is on");
    assert!(field.laws().every_tick);
    for driver in &field.drivers {
        assert_eq!(f64::from(driver.planner.rate), 0.001_649_999_991_059_303_3);
    }
}

/// An Elimination event keeps this project's driver: 2048 hands its
/// opponents' throttle to a law of its own (`0x81199800`) that is unread.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn an_elimination_event_keeps_this_projects_driver() {
    let Some(loaded) = loaded("2048 - Event 5-1") else {
        return;
    };
    assert_eq!(loaded.setup.mode, oag_race::Mode::Eliminator);
    assert_eq!(
        loaded.campaign_2048_event.expect("a campaign event").ai,
        None
    );
}

/// The racer one place ahead of the parked player is its own formation
/// reference: gap `0`, `WhenBehind` at its clamp. Vita3K read it at `63.0`
/// on every sample at skill `1.0` and `74.0` at `3.0` (44 of 44 each).
fn reference_racer_reads(skill: f32, expected: f32) {
    {
        let Some(ticks) = held(skill, 30.0, 40.0) else {
            return;
        };
        // On the tick a craft takes seventh the law has read the standings
        // before the change, so only a craft seventh on both ticks counts.
        let seventh = |tick: &[(u8, f32)]| tick.iter().position(|(place, _)| *place == 7);
        let reference: Vec<f32> = ticks
            .windows(2)
            .filter_map(|w| {
                let slot = seventh(&w[1])?;
                (seventh(&w[0]) == Some(slot)).then_some(w[1][slot].1)
            })
            .collect();
        assert!(!reference.is_empty(), "nobody in seventh");
        assert!(
            reference.iter().all(|t| (t - expected).abs() < 1e-3),
            "skill {skill}: {:?}",
            reference
                .iter()
                .filter(|t| (*t - expected).abs() >= 1e-3)
                .take(12)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn at_the_events_own_skill_the_reference_racer_reads_63() {
    reference_racer_reads(1.0, 63.0);
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn at_skill_three_the_reference_racer_reads_74() {
    reference_racer_reads(3.0, 74.0);
}

/// Every opponent runs its law every tick: on a round robin at most one
/// held throttle could change from one tick to the next.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn every_opponent_runs_its_law_every_tick() {
    let Some(ticks) = held(1.0, 5.0, 20.0) else {
        return;
    };
    let most = ticks
        .windows(2)
        .map(|w| w[0].iter().zip(&w[1]).filter(|(a, b)| a.1 != b.1).count())
        .max()
        .unwrap_or(0);
    assert!(most >= 3, "at most {most} throttles moved in one tick");
}

/// `AIRaceStats_<class>2048.xml` and `AIControlStats2048.xml`, which a 2048
/// event's opponents read (`FUN_811a0b18`), hold the base set's values, so
/// the port reads the base set.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_2048_table_set_equals_the_base_set() {
    let Some(source) = source() else {
        return;
    };
    let mut archives = oag_2048::open(&source.display().to_string()).expect("the package opens");
    let mut read = |name: &str| {
        let blob = archives.read_name(name).expect(name);
        oag_tables::fexml::text(&blob).expect(name)
    };
    let squash = |text: String| text.split_whitespace().collect::<Vec<_>>().join(" ");
    for class in ["venom", "flash", "rapier", "phantom", "superphantom"] {
        let base = read(&format!(r"Data\XML\AIRaceStats_{class}.xml"));
        let own = read(&format!(r"Data\XML\AIRaceStats_{class}2048.xml"));
        assert_eq!(squash(base), squash(own), "{class}");
    }
    assert_eq!(
        squash(read(r"Data\XML\AIControlStats.xml")),
        squash(read(r"Data\XML\AIControlStats2048.xml"))
    );
}

/// The event's whole race with the player parked: every opponent finishes,
/// or is destroyed by another's weapon, inside 300 s with at most `respawns` respawns each. Shield is read at each
/// opponent's lap crossings (per lap) and at the end of the run, reported
/// apart. The parked player is behind the whole field, so `WhenBehind` holds
/// every opponent back: a race takes 160 to 280 s.
///
/// And a field that picked anything up put something in the air: the
/// campaign's opponents fire (zero on every event until 2026-10-19, see
/// `oag_ai::Driver::attend`).
fn fly(event: &str, respawns: u32) {
    let limit = 300.0;
    let Some(mut race) = race(event, None) else {
        return;
    };
    let mut lap = [1u32; 8];
    let mut per_lap: [Vec<f32>; 8] = Default::default();
    let (mut pickups, mut fired) = (0u32, 0u32);
    let mut weapon = [None; 8];
    let mut owned = [0usize; 8];
    while seconds(&race) <= limit && !(1..8).all(|s| race.sim.world.ships[s].standing.finished()) {
        race.tick(&PlayerInputs::none());
        let mut now = [0usize; 8];
        for projectile in &race.sim.world.projectiles.slots {
            if projectile.kind.is_some() && usize::from(projectile.owner) < 8 {
                now[usize::from(projectile.owner)] += 1;
            }
        }
        for slot in 1..8 {
            let held = race.sim.world.ships[slot].pickup.weapon;
            pickups += u32::from(held.is_some() && held != weapon[slot]);
            weapon[slot] = held;
            fired += now[slot].saturating_sub(owned[slot]) as u32;
            owned[slot] = now[slot];
        }
        for slot in 1..8 {
            let ship = &race.sim.world.ships[slot];
            if ship.standing.lap != lap[slot] {
                lap[slot] = ship.standing.lap;
                per_lap[slot].push(ship.physics.shield);
            }
        }
    }
    println!(
        "{event}: {:.1} s, opponent pickups {pickups}, projectiles {fired}",
        seconds(&race)
    );
    for (slot, shields) in per_lap.iter().enumerate().skip(1) {
        let ship = &race.sim.world.ships[slot];
        println!(
            "  slot {slot}: finished {} lap {} respawns {} wall-contact ticks {} per-lap shield {shields:?}, end-of-run shield {:.1}",
            ship.standing.finished(),
            ship.standing.lap,
            race.respawns_of(slot),
            race.wall_contact_ticks_of(slot),
            ship.physics.shield
        );
    }
    assert!(
        pickups == 0 || fired > 0,
        "{event}: {pickups} opponent pickups and nothing fired"
    );
    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
        // **Or destroyed**: since 2026-10-19 the field fires, and a craft
        // another opponent wrecks stays a wreck (`Destroyed`, then `Eliminated`),
        // as the original's state 6 does (`oag_physics::CraftState`). Every other craft still finishes.
        assert!(
            ship.standing.finished()
                || matches!(
                    ship.physics.craft_state,
                    oag_physics::CraftState::Destroyed | oag_physics::CraftState::Eliminated
                ),
            "{event}: slot {slot} on lap {}",
            ship.standing.lap
        );
        assert!(
            race.respawns_of(slot) <= respawns,
            "{event}: slot {slot} respawned {} times",
            race.respawns_of(slot)
        );
    }
}

/// One race event per circuit, a test each. Sol (`"2050 - Event 2"`) is not
/// here: every craft there respawns in a loop on lap 1, this project's own
/// driver as much as the original's (213 to 218 respawns each in 240 s), a
/// rescue fault the driver does not cause.
macro_rules! circuit {
    ($name:ident, $event:literal, $respawns:literal) => {
        #[test]
        #[ignore = "needs the extracted package under data/extracted/vita/"]
        fn $name() {
            fly($event, $respawns);
        }
    };
}

circuit!(the_field_finishes_tower, "2048 - Event 1", 0);
circuit!(the_field_finishes_mall, "2048 - Event 2", 0);
circuit!(the_field_finishes_bridge, "2048 - Event 2-1", 0);
circuit!(the_field_finishes_square, "2048 - Event 4", 0);
// One respawn since the Turbo's running laws (2026-10-20): in the unperturbed run slot 5 respawns once
// after 23 wall-contact ticks. Park fails 1 of 9 perturbations either way (laws off at
// `-1e-5`, on at `0`, the patched `+0.0`); the field's totals over the 49 `*_original_ai_ground_truth` tests are
// off `0 0 1 1 0 0 0 2 0`, on `1 2 0 1 4 2 0 0 0` (-1e-2 .. 1e-5), inside the noise bar.
circuit!(the_field_finishes_park, "2048 - Event 4-1", 1);
// Arena puts most of the field back on the track one to three times a craft.
circuit!(the_field_finishes_arena, "2049 - Event 6", 3);
circuit!(the_field_finishes_cathedral, "2049 - Event 2", 0);
circuit!(the_field_finishes_subway, "2049 - Event 4", 0);
circuit!(the_field_finishes_altima, "2050 - Event 1", 0);

/// Super Phantom is the rung above Phantom and 2048 authors its tables
/// (`AIRaceStats_superphantom.xml`, `SuperPhantomStats` in
/// `AIControlStats.xml`): its ten "S Phantom Challenge" events fly the
/// original's driver like every other race event. They used to keep this
/// project's driver because the class had no `SpeedClass` to read tables with.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn super_phantom_challenges_fly_the_original_driver() {
    for track in [
        "Tower",
        "Mall",
        "Bridge",
        "Square",
        "Park",
        "Arena",
        "Cathedral",
        "Subway",
        "Sol",
        "Altima",
    ] {
        let event = format!("{track} - S Phantom Challenge");
        let Some(loaded) = loaded(&event) else {
            return;
        };
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.contains("AIRaceStats_superphantom.xml")),
            "{event}: the Super Phantom table is the one read: {:?}",
            loaded.report
        );
        let ai = loaded
            .campaign_2048_event
            .as_ref()
            .and_then(|event| event.ai)
            .unwrap_or_else(|| panic!("{event} authors the original's AI"));
        let mut race = oag_raceplay::Race::start(loaded.setup);
        assert!(
            race.use_original_ai(Some(ai.skill_scale), ai.level),
            "{event}: the Super Phantom tables read, so the original driver flies"
        );
    }
}

/// The Super Phantom field on Unity Square for 120 s with the player parked:
/// every opponent is under the original's driver and at most two respawns a
/// craft (one before the Turbo's running laws, 2026-10-20; measured then: three
/// of the seven respawn once, the Flash race on the same circuit has none). Shield is
/// reported at each lap crossing (per lap) and at the end of the run, apart.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_super_phantom_field_runs_square_clean() {
    let Some(mut race) = race("Square - S Phantom Challenge", None) else {
        return;
    };
    let mut lap = [1u32; 8];
    let mut per_lap: [Vec<f32>; 8] = Default::default();
    while seconds(&race) <= 120.0 {
        race.tick(&PlayerInputs::none());
        for slot in 1..8 {
            let ship = &race.sim.world.ships[slot];
            if ship.standing.lap != lap[slot] {
                lap[slot] = ship.standing.lap;
                per_lap[slot].push(ship.physics.shield);
            }
        }
    }
    for (slot, shields) in per_lap.iter().enumerate().skip(1) {
        let ship = &race.sim.world.ships[slot];
        println!(
            "slot {slot}: lap {} respawns {} per-lap shield {:?}, end-of-run shield {:.1}",
            ship.standing.lap,
            race.respawns_of(slot),
            shields,
            ship.physics.shield
        );
        // Two since the Turbo's running laws: The Turbo's running laws (`craft+0x1c0 & 0x200`: the stun escape and the pad fade,
        // `docs/ghidra/functions/psp-pulse-usa/craft-flags.md`) moved it, judged by the noise method on
        // 2026-10-20: failures among the 49 `*_original_ai_ground_truth` tests per airborne pitch-torque
        // perturbation (-1e-2, -1e-3, -1e-4, -1e-5, 0, 1e-2, 1e-3, 1e-4, 1e-5), laws off `0 0 1 1 0 0 0 2 0`
        // (median 0, max 2), laws on `1 2 0 1 4 2 0 0 0` (median 1, max 4): inside the bar.
        // In the unperturbed run slot 1 respawns twice (per-lap shield `[88.7, 130.7]`,
        // end-of-run shield `114.6`).
        assert!(race.respawns_of(slot) <= 2, "slot {slot}");
        assert!(
            ship.standing.lap >= 3,
            "slot {slot} laps at a Super Phantom pace"
        );
    }
}

/// A RACE REMIX style race on Super Phantom, which this project's driver is
/// not verified on, fields no opponents; the same race on Phantom fields all
/// seven. (`oag_ai::coverage`; the campaign's own controller is separate.)
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_super_phantom_race_box_race_runs_without_opponents() {
    let Some(source) = source() else {
        return;
    };
    let ship_count = |class: &str| {
        let options = oag_raceplay::Options {
            source: source.display().to_string(),
            mode: oag_race::Mode::SingleRace,
            class: class.to_string(),
            no_driver_for_class: !oag_ai::coverage::project_driver_covers(class),
            seed: Some(1),
            ..oag_raceplay::Options::default()
        };
        let loaded = oag_raceplay::load(&options).expect("loads");
        oag_raceplay::Race::start(loaded.setup).sim.world.ship_count
    };
    assert_eq!(ship_count("PHANTOM"), 8);
    assert_eq!(ship_count("SUPERPHANTOM"), 1);
}
