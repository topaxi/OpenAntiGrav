//! The official mod Slipstream (`assets/mods/official/slipstream`, `oag.slipstream`) in a
//! real race on Pulse PSP EU's `16_Track`: the pads hand it out by its own odds, a press
//! with no room for the portal keeps the pickup, a fire is a transit that leaves the craft
//! untouchable and out of the world and puts it further along at a slower speed, a lap or
//! the finish through the jump counts on its exit tick, the AI fires it, and only one
//! portal is open at a time.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, the
//! `assets/mods/official` submodule and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/slipstream_ground_truth::/)' --no-capture
//! ```
#![cfg(feature = "extensions")]

use std::sync::Arc;

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_raceplay as race;
use oag_raceplay::transit::Distance;
use oag_tables::weapons::Weapon;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";
const MOD: &str = "assets/mods/official/slipstream";

/// The mod as the game builds its source from the manifest (`oag_game::mod_extensions`):
/// every field, so the icons, effects and look rows are the real ones.
fn slipstream() -> Option<race::extensions::Source> {
    source_of(MOD)
}

/// The mod at `dir` (relative to the repository) as the game builds its source.
pub(crate) fn source_of(dir: &str) -> Option<race::extensions::Source> {
    let manifest = oag_testdata::exact(&format!("{dir}/oag-mod.toml"))?;
    let module = Arc::new(oag_mod::Mod::open(manifest.parent().unwrap()).expect("the mod opens"));
    let extension = module.manifest.extension.clone().expect("[extension]");
    let rules = extension.rules.expect("a rules module");
    let wasm = module
        .read(&rules.module)
        .unwrap()
        .expect("the committed module");
    let files = {
        let module = module.clone();
        Arc::new(move |name: &str| {
            module
                .read(name)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("{name}: not in the mod"))
        })
    };
    let icons = module
        .manifest
        .icons
        .iter()
        .map(|(name, path)| {
            let files = files.clone();
            let file = path.clone();
            let reader: race::extensions::Reader = Arc::new(move || files(&file));
            race::extensions::IconSource {
                name: name.clone(),
                path: path.clone(),
                read: reader,
            }
        })
        .collect();
    Some(race::extensions::Source {
        mod_id: module.manifest.id.clone(),
        module: rules.module,
        titles: module.manifest.titles.clone(),
        api: extension.api,
        memory_pages: extension.memory_pages,
        state_bytes: extension.state_bytes,
        drives_ai_pickup: extension.drives_ai_pickup,
        scope: race::extensions::Scope::Rules,
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons,
        effects: module
            .manifest
            .effects
            .iter()
            .map(|(name, path)| (name.clone(), path.clone()))
            .collect(),
        shaders: module
            .manifest
            .shaders
            .iter()
            .map(|(name, path)| (name.clone(), path.clone()))
            .collect(),
        sounds: module
            .manifest
            .sounds
            .iter()
            .map(|(name, path)| (name.clone(), path.clone()))
            .collect(),
        files,
        pickup: module.manifest.pickup.clone(),
        pickup_titles: module.manifest.pickup_titles.clone(),
        pickup_odds: module.manifest.pickup_odds.clone(),
        options: Default::default(),
        channels: race::extensions::Declared {
            publishes: module.manifest.channels.publishes.clone(),
            listens: module.manifest.channels.listens.clone(),
        },
    })
}

/// A Single Race on `16_Track` with the mod on; `field` fills the grid with opponents.
fn race_on(source: &race::extensions::Source, field: bool) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        no_driver_for_class: !field,
        extensions: vec![source.clone()],
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    Some(race)
}

fn held(buttons: &[Button], pressed: &[Button]) -> PlayerInputs {
    let mut input = oag_gameplay::input::Input::new();
    let held = buttons.iter().fold(0, |bits, b| bits | b.bit());
    let pressed_bits = pressed.iter().fold(0, |bits, b| bits | b.bit());
    input.begin_frame(held & !pressed_bits);
    input.begin_frame(held);
    PlayerInputs::single(oag_gameplay::InputSnapshot {
        buttons: input,
        ..oag_gameplay::InputSnapshot::new()
    })
}

fn throttle() -> PlayerInputs {
    held(&[Button::Cross], &[])
}

fn press_fire() -> PlayerInputs {
    held(&[Button::Cross, Button::Square], &[Button::Square])
}

fn give(race: &mut race::Race, slot: usize) {
    race.sim.world.ships[slot].pickup.weapon =
        Some(Weapon::Extension(oag_tables::weapons::ModKind::FIRST));
}

fn no_fault(race: &race::Race) -> bool {
    race.extension_reports().iter().all(|r| r.faulted.is_none())
}

fn ship(race: &race::Race) -> oag_gameplay::world::Ship {
    race.sim.world.ships[0]
}

fn progress(race: &race::Race) -> f32 {
    ship(race).standing.progress.expect("a course fix")
}

fn kmh(race: &race::Race) -> f32 {
    ship(race).physics.body.linear_velocity.length() * oag_core::math::SPEED_TO_KMH
}

/// Drives with the throttle held until `done`, at most `limit` ticks.
fn drive_until(race: &mut race::Race, limit: u32, done: impl Fn(&race::Race) -> bool) {
    for _ in 0..limit {
        if done(race) {
            return;
        }
        race.tick(&throttle());
    }
    panic!("not reached in {limit} ticks");
}

/// Off the grid and `ticks` into the first lap, holding the pickup with the guest's
/// probe answered; then fires and returns the tick the transit began after.
fn fire_at(race: &mut race::Race, ticks: u32) {
    drive_until(race, 3000, |r| {
        r.sim.world.ships[0].standing.lap_start_tick.is_some()
    });
    for _ in 0..ticks {
        race.tick(&throttle());
    }
    give(race, 0);
    for _ in 0..4 {
        race.tick(&throttle());
    }
    race.tick(&press_fire());
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd, the official mods and --features extensions"]
fn a_press_where_the_portal_does_not_fit_keeps_the_pickup() {
    let Some(source) = slipstream() else { return };
    let Some(mut race) = race_on(&source, false) else {
        return;
    };
    // `16_Track` has a stretch (about tick 2000 of this drive) where the half disc 30
    // units ahead meets geometry. Hold the pickup well before it, and press once the
    // probe has read it shut on two ticks running, so the guest's last answer is "no".
    drive_until(&mut race, 3000, |r| r.sim.world.tick >= 1900);
    give(&mut race, 0);
    let mut shut = 0;
    for _ in 0..300 {
        race.tick(&throttle());
        let (fits, _) = race.clearance(0, Distance::Units(30.0), 12.0);
        shut = if fits { 0 } else { shut + 1 };
        if shut == 2 {
            break;
        }
    }
    assert_eq!(shut, 2, "the stretch the drive knows");
    race.tick(&press_fire());
    race.tick(&throttle());
    assert_eq!(ship(&race).transit, None, "no portal, no transit");
    assert_eq!(
        race.ship_pickup(),
        Some(Weapon::Extension(oag_tables::weapons::ModKind::FIRST)),
        "kept"
    );
    assert!(no_fault(&race), "a refusal is not a fault");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd, the official mods and --features extensions"]
fn a_fire_is_a_second_out_of_the_world_and_four_seconds_on_at_a_slower_speed() {
    let Some(source) = slipstream() else { return };
    let Some(mut race) = race_on(&source, false) else {
        return;
    };
    fire_at(&mut race, 400);
    let pressed = race.sim.world.tick;
    let pressed_at = progress(&race);
    assert_eq!(ship(&race).transit, None, "not on the press tick");
    // The craft keeps flying, in the world, until it reaches the portal 30 units ahead.
    let mut flying = 0;
    while ship(&race).transit.is_none() {
        assert!(ship(&race).in_world(), "still flying at {flying}");
        assert!(race.ship_active(0), "still drawn at {flying}");
        race.tick(&throttle());
        flying += 1;
        assert!(flying < 60, "never reached the portal");
    }
    let entry = ship(&race).physics.body.position;
    let before = progress(&race);
    let speed = kmh(&race);
    println!(
        "pressed at tick {pressed} progress {pressed_at:.1}; transit began tick {} progress {before:.1}",
        race.sim.world.tick
    );
    assert!(race.sim.world.tick > pressed + 3, "later than the press");
    assert!(
        before - pressed_at >= 30.0 && before - pressed_at < 30.0 + 2.0 * speed / 216.0,
        "began at the entrance: {} units on",
        before - pressed_at
    );
    assert_eq!(race.ship_pickup(), None, "spent");
    let mut out = 1;
    while ship(&race).transit.is_some() {
        assert_eq!(ship(&race).physics.body.position, entry, "frozen at {out}");
        assert!(!race.ship_active(0), "not drawn");
        assert!(!ship(&race).in_world(), "out of the world: untouchable");
        race.tick(&throttle());
        out += 1;
        assert!(out < 100, "never came back");
    }
    assert!((59..=61).contains(&out), "a second out: {out} ticks");
    let after = progress(&race);
    let exit_speed = kmh(&race);
    let expected = speed / oag_core::math::SPEED_TO_KMH * 4.0;
    println!(
        "progress {before:.0} -> {after:.0} (+{:.0}, four seconds at {speed:.0} km/h is {expected:.0}); exit speed {exit_speed:.0}",
        after - before
    );
    assert!(
        (after - before - expected).abs() < expected * 0.2,
        "four seconds further"
    );
    assert!(
        (0.8..0.97).contains(&(exit_speed / speed)),
        "a slight loss: {speed} -> {exit_speed}"
    );
    assert!(race.respawns() == 0, "no rescue");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd, the official mods and --features extensions"]
fn a_fire_over_the_line_is_a_lap_and_on_the_last_lap_a_finish_on_the_exit_tick() {
    let Some(source) = slipstream() else { return };
    let Some(mut race) = race_on(&source, false) else {
        return;
    };
    drive_until(&mut race, 3000, |r| {
        r.sim.world.ships[0].standing.lap_start_tick.is_some()
    });
    let length = race.course().expect("a course").length();
    // Near the end of the lap: a fire there crosses the line.
    drive_until(&mut race, 6000, |r| progress(r) > length - 150.0);
    give(&mut race, 0);
    for _ in 0..3 {
        race.tick(&throttle());
    }
    let lap = ship(&race).standing.lap;
    race.tick(&press_fire());
    let mut exit = None;
    for _ in 0..120 {
        race.tick(&throttle());
        if ship(&race).standing.lap > lap {
            exit = Some(race.sim.world.tick);
            break;
        }
    }
    let exit = exit.expect("the lap counts");
    assert_eq!(ship(&race).transit, None, "on the exit tick");
    assert_eq!(ship(&race).standing.lap_start_tick, Some(exit));
    assert_eq!(
        race.sim.world.race[0].lap_start_tick, exit,
        "the person's clock too"
    );

    // On to the last lap by the engine's own jump, then the mod's fire over the line.
    let target = race.sim.world.laps_target().expect("a lap target");
    while ship(&race).standing.lap < target {
        race.begin_transit(0, 2, Distance::Lap(0.9), 0.95).unwrap();
        for _ in 0..62 {
            race.tick(&throttle());
        }
    }
    // The jump landed late in the lap and the half-way gate is still shut, so drive over
    // the line (it does not finish), and round the whole last lap to its end.
    drive_until(&mut race, 6000, |r| progress(r) < 800.0);
    drive_until(&mut race, 6000, |r| progress(r) > length - 150.0);
    give(&mut race, 0);
    for _ in 0..3 {
        race.tick(&throttle());
    }
    race.tick(&press_fire());
    for _ in 0..80 {
        race.tick(&throttle());
        if ship(&race).standing.finish_tick.is_some() {
            break;
        }
    }
    let finish = ship(&race).standing.finish_tick.expect("it finishes");
    assert_eq!(finish, race.sim.world.tick, "timed on the exit tick");
    assert!(race.sim.world.race[0].finished);
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd, the official mods and --features extensions"]
fn the_ai_fires_it_and_only_one_portal_is_open_at_a_time() {
    let Some(source) = slipstream() else { return };
    let Some(mut race) = race_on(&source, true) else {
        return;
    };
    let count = usize::from(race.sim.world.ship_count);
    for slot in 1..count {
        give(&mut race, slot);
    }
    let mut was_out = vec![false; count];
    let mut transits = vec![0_u32; count];
    let mut most_out = 0;
    for _ in 0..2400 {
        race.tick(&throttle());
        let out = (1..count)
            .filter(|&s| race.sim.world.ships[s].transit.is_some())
            .count();
        most_out = most_out.max(out);
        for slot in 1..count {
            let now = race.sim.world.ships[slot].transit.is_some();
            if now && !was_out[slot] {
                transits[slot] += 1;
            }
            was_out[slot] = now;
        }
    }
    println!("transits per slot {transits:?}, most out at once {most_out}");
    assert!(
        transits.iter().sum::<u32>() >= 2,
        "the AI fires it, and the queue runs"
    );
    assert_eq!(most_out, 1, "one portal pair at a time");
    assert!(no_fault(&race), "no fault");
}

#[test]
#[ignore = "needs the Pulse, Pure, HD and 2048 images (and Omega's) and the official mods"]
fn its_share_of_the_pad_draws_is_a_fraction_of_a_turbo_on_every_titles_real_table() {
    let Some(source) = slipstream() else { return };
    let row = source.pickup_odds[0].odds;
    for (image, title) in [
        ("data/images/pulse-psp-eu.chd", "Pulse"),
        ("data/images/pure-psp-eu.chd", "Pure"),
        ("data/images/hdfury-ps3-eu-dec.iso", "HD"),
        ("data/extracted/vita/PCSF00007", "2048"),
        ("data/images/omega-ps4-eu.pkg", "Omega"),
    ] {
        let Some(image) = oag_testdata::image(image) else {
            continue;
        };
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("{title}: {e}"));
        let weapons = loaded.setup.weapons.as_ref().expect("a weapon table");
        assert!(!weapons.pickups.is_empty(), "{title}");
        for table in &weapons.pickups {
            let turbo = table
                .odds
                .iter()
                .find(|(w, _)| *w == Weapon::Turbo)
                .map(|(_, o)| *o)
                .expect("a Turbo row");
            let ai: f32 = table.odds.iter().map(|(_, o)| o.ai).sum();
            let human: f32 = table.odds.iter().map(|(_, o)| o.human).sum();
            let ai_share = row.ai / (ai + row.ai);
            let human_share = row.human / (human + row.human);
            println!(
                "{title} {}: AI {:.1} % (turbo {:.1} %), human {:.1} %",
                table.class,
                ai_share * 100.0,
                turbo.ai / (ai + row.ai) * 100.0,
                human_share * 100.0
            );
            assert!((0.03..0.15).contains(&ai_share), "{title} {}", table.class);
            assert!(
                (0.03..0.15).contains(&human_share),
                "{title} {}",
                table.class
            );
            assert!(ai_share < turbo.ai / (ai + row.ai), "less than a Turbo");
        }
    }
}
