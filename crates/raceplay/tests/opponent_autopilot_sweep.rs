//! Whether an opponent on this project's driver gains by flying the
//! original's player driver while it holds an Autopilot
//! (`Race::set_opponent_autopilot`), against absorbing it as it does today.
//!
//! Paired forks: at each trigger the race is snapshotted, one opponent is
//! handed the authored Autopilot `time` in branch B and left alone in branch
//! A, and both run the stint plus a tail. The main path carries on from A.

use oag_gameplay::PlayerInputs;

/// Ticks after the stint that still count, for the handover.
const TAIL: u64 = 180;

/// What one branch did to the triggered slot over the window.
#[derive(Debug, Clone, Copy, Default)]
struct Branch {
    /// Distance raced by the end of the stint.
    stint: f32,
    /// Distance raced by the end of the window.
    window: f32,
    /// Wall-contact ticks over the window.
    wall: u32,
    /// Rescues over the window.
    respawns: u32,
    /// Shield at the end of the window minus at the trigger.
    shield: f32,
    /// Window tick of the first wall contact and of the first rescue.
    first_wall: Option<u64>,
    /// Window tick of the first rescue, with its cause.
    first_respawn: Option<(u64, Option<oag_raceplay::RespawnCause>)>,
}

fn load(
    image: &str,
    track: Option<&str>,
    class: &str,
    difficulty: oag_ai::Difficulty,
    seed: u64,
) -> Option<oag_raceplay::Race> {
    let path = if image.starts_with("data/") {
        oag_testdata::exact(image)?
    } else {
        oag_testdata::image(image)?
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: path.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        track: track.map(str::to_string),
        seed: Some(seed),
        difficulty,
        weapons_override: std::env::var_os("OAG_SWEEP_NO_WEAPONS").map(|_| false),
        ..oag_raceplay::Options::default()
    })
    .unwrap_or_else(|e| panic!("{image}: {e}"));
    Some(oag_raceplay::Race::start(loaded.setup))
}

fn run_branch(race: &mut oag_raceplay::Race, slot: usize, stint: u64, trace: bool) -> Branch {
    let d0 = race.race_distance_of(slot).unwrap_or(0.0);
    let w0 = race.wall_contact_ticks_of(slot);
    let r0 = race.respawns_of(slot);
    let s0 = race.sim.world.ships[slot].physics.shield;
    let mut out = Branch::default();
    for t in 0..stint + TAIL {
        race.tick_sim_only(&PlayerInputs::none());
        if trace {
            let ship = &race.sim.world.ships[slot];
            let body = &ship.physics.body;
            println!(
                "  t {t} pos [{:.0} {:.0} {:.0}] v {:.1} thrust {:.1} grounded {} air {:.2} idx {} wall {} timer {:.2}",
                body.position.x,
                body.position.y,
                body.position.z,
                body.linear_velocity.dot(body.forward()),
                ship.physics.thrust,
                ship.physics.grounded,
                ship.physics.time_airborne,
                ship.driver.index,
                race.wall_contact_ticks_of(slot) - w0,
                ship.autopilot_timer,
            );
        }
        if out.first_wall.is_none() && race.wall_contact_ticks_of(slot) > w0 {
            out.first_wall = Some(t);
        }
        if out.first_respawn.is_none() && race.respawns_of(slot) > r0 {
            out.first_respawn = Some((t, race.last_respawn_cause_of(slot)));
        }
        if t + 1 == stint {
            out.stint = race.race_distance_of(slot).unwrap_or(0.0) - d0;
        }
    }
    out.window = race.race_distance_of(slot).unwrap_or(0.0) - d0;
    out.wall = race.wall_contact_ticks_of(slot) - w0;
    out.respawns = race.respawns_of(slot) - r0;
    out.shield = race.sim.world.ships[slot].physics.shield - s0;
    out
}

/// The Autopilot's share of the `ai` column, per class.
fn ai_odds(race: &oag_raceplay::Race) -> Vec<(String, f32)> {
    let Some(stats) = race.weapon_stats() else {
        return Vec::new();
    };
    stats
        .pickups
        .iter()
        .map(|table| {
            let total: f32 = table.odds.iter().map(|(_, odds)| odds.ai).sum();
            let autopilot = table
                .odds
                .iter()
                .find(|(weapon, _)| *weapon == oag_tables::weapons::Weapon::Autopilot)
                .map_or(0.0, |(_, odds)| odds.ai);
            (
                table.class.clone(),
                if total > 0.0 { autopilot / total } else { 0.0 },
            )
        })
        .collect()
}

fn race_position(snapshot: &oag_raceplay::RaceSim, slot: usize) -> [i32; 3] {
    let p = snapshot.world.ships[slot].physics.body.position;
    [p.x as i32, p.y as i32, p.z as i32]
}

fn median(values: &mut [f32]) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// Runs `triggers` paired forks on each seed and prints one line a fork and a
/// summary. Returns the (A, B) pairs.
fn sweep(
    image: &str,
    track: Option<&str>,
    class: &str,
    difficulty: oag_ai::Difficulty,
    seeds: std::ops::RangeInclusive<u64>,
    triggers: usize,
) -> Vec<(Branch, Branch)> {
    let mut pairs = Vec::new();
    let mut natural = 0u32;
    for seed in seeds {
        let Some(mut race) = load(image, track, class, difficulty, seed) else {
            return pairs;
        };
        if seed == 1 {
            println!("{image} {track:?} ai odds {:?}", ai_odds(&race));
        }
        let Some(time) = race
            .weapon_stats()
            .and_then(|w| w.simple(oag_tables::weapons::Weapon::Autopilot))
            .map(|s| s.time)
            .or(std::env::var_os("OAG_SWEEP_NO_WEAPONS").map(|_| 5.0))
        else {
            println!("{image}: no Autopilot authored");
            return pairs;
        };
        let stint = (time * 60.0).round() as u64;
        race.set_autopilot(true);
        let count = race.sim.world.ship_count as usize;
        let mut held = [false; 8];
        let mut next_slot = 1;
        let mut done = 0;
        let mut tick = 0u64;
        while done < triggers && tick < 20_000 {
            race.tick_sim_only(&PlayerInputs::none());
            tick += 1;
            for (slot, was) in held.iter_mut().enumerate().take(count).skip(1) {
                let now = race.sim.world.ships[slot].pickup.weapon
                    == Some(oag_tables::weapons::Weapon::Autopilot);
                if now && !*was {
                    natural += 1;
                }
                *was = now;
            }
            if tick < 900 || tick % 600 != 0 {
                continue;
            }
            let slot = next_slot;
            next_slot = if next_slot + 1 >= count {
                1
            } else {
                next_slot + 1
            };
            let ship = &race.sim.world.ships[slot];
            if ship.physics.craft_state != oag_physics::CraftState::Racing
                || ship.standing.finished()
            {
                continue;
            }
            let snapshot = race.snapshot_sim();
            race.set_opponent_autopilot(true);
            if !race.engage_opponent_autopilot(slot) {
                // Weapons off: no table to read the time from.
                race.sim.world.ships[slot].autopilot_timer = time;
            }
            let trace = std::env::var("OAG_SWEEP_TRACE").ok() == Some(format!("{seed},{tick}"));
            let b = run_branch(&mut race, slot, stint, trace);
            race.restore_sim(&snapshot);
            if trace {
                println!("  -- A");
            }
            let a = run_branch(&mut race, slot, stint, trace);
            tick += stint + TAIL;
            println!(
                "seed {seed} tick {tick} slot {slot} A stint {:.0} window {:.0} wall {} resp {} shield {:+.1} | B stint {:.0} window {:.0} wall {} resp {} shield {:+.1}",
                a.stint,
                a.window,
                a.wall,
                a.respawns,
                a.shield,
                b.stint,
                b.window,
                b.wall,
                b.respawns,
                b.shield
            );
            if b.first_wall.is_some() || b.first_respawn.is_some() {
                println!(
                    "  B first wall {:?} first respawn {:?} (stint {stint}) at {:?}",
                    b.first_wall,
                    b.first_respawn,
                    race_position(&snapshot, slot)
                );
            }
            pairs.push((a, b));
            done += 1;
        }
    }
    if pairs.is_empty() {
        return pairs;
    }
    let mut stint: Vec<f32> = pairs.iter().map(|(a, b)| b.stint - a.stint).collect();
    let mut window: Vec<f32> = pairs.iter().map(|(a, b)| b.window - a.window).collect();
    let faster = window.iter().filter(|d| **d > 0.0).count();
    let wall_a: u32 = pairs.iter().map(|(a, _)| a.wall).sum();
    let wall_b: u32 = pairs.iter().map(|(_, b)| b.wall).sum();
    let resp_a: u32 = pairs.iter().map(|(a, _)| a.respawns).sum();
    let resp_b: u32 = pairs.iter().map(|(_, b)| b.respawns).sum();
    let shield_a: f32 = pairs.iter().map(|(a, _)| a.shield).sum::<f32>() / pairs.len() as f32;
    let shield_b: f32 = pairs.iter().map(|(_, b)| b.shield).sum::<f32>() / pairs.len() as f32;
    let n = pairs.len();
    let window_mean_a: f32 = pairs.iter().map(|(a, _)| a.window).sum::<f32>() / n as f32;
    let ms = median(&mut stint);
    let mw = median(&mut window);
    println!(
        "SUMMARY {image} {track:?} {class} {difficulty} n {n} natural pickups {natural} | window A mean {window_mean_a:.0} | B-A stint median {ms:+.1} [{:+.1}, {:+.1}] window median {mw:+.1} [{:+.1}, {:+.1}] (q1 {:+.1} q3 {:+.1}) B faster {faster}/{n} | wall A {wall_a} B {wall_b} | respawns A {resp_a} B {resp_b} | shield mean A {shield_a:+.1} B {shield_b:+.1}",
        stint[0],
        stint[n - 1],
        window[0],
        window[n - 1],
        window[n / 4],
        window[3 * n / 4],
    );
    pairs
}

fn difficulties() -> Vec<oag_ai::Difficulty> {
    match std::env::var("OAG_SWEEP_DIFFICULTY") {
        Ok(name) => vec![name.parse().expect("a difficulty")],
        Err(_) => vec![oag_ai::Difficulty::Novice, oag_ai::Difficulty::Ace],
    }
}

fn seeds() -> u64 {
    std::env::var("OAG_SWEEP_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4)
}

/// The scratch sweep, one title a test so they run side by side.
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
///     --run-ignored all opponent_autopilot_sweep --no-capture
/// ```
fn title(image: &str, tracks: &[Option<&str>]) {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".to_string());
    for difficulty in difficulties() {
        let only = std::env::var("OAG_SWEEP_TRACK").ok();
        for track in tracks {
            if let (Some(only), Some(track)) = (&only, track)
                && !track.contains(only.as_str())
            {
                continue;
            }
            sweep(image, *track, &class, difficulty, 1..=seeds(), 8);
        }
    }
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn pulse() {
    title(
        "pulse-psp-usa.chd",
        &[
            Some(r"Data\Environments\01_Track\track.vex"),
            Some(r"Data\Environments\05_Track\track.vex"),
            Some(r"Data\Environments\10_Track\track.vex"),
            Some(r"Data\Environments\14_Track\track.vex"),
        ],
    );
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn pure() {
    title("pure-psp-usa.chd", &[None]);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn hd() {
    title("hdfury-ps3-eu-dec.iso", &[None]);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn omega() {
    title("omega-ps4-eu.pkg", &[None]);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn vita_2048() {
    title("data/test-vpk/2048-PCSF00007-dlc1.vpk", &[None]);
}
