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
    /// Wall-contact ticks over the window, and over the stint alone.
    wall: u32,
    wall_stint: u32,
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
    eps: f32,
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
    let mut setup = loaded.setup;
    crate::perturb::apply(&mut setup, eps);
    Some(oag_raceplay::Race::start(setup))
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
            out.wall_stint = race.wall_contact_ticks_of(slot) - w0;
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
    eps: f32,
) -> Vec<(Branch, Branch)> {
    let mut pairs = Vec::new();
    let mut natural = 0u32;
    let mut absorb = 0.0;
    let mut window_ticks = 0;
    for seed in seeds {
        let Some(mut race) = load(image, track, class, difficulty, seed, eps) else {
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
        window_ticks = stint + TAIL;
        absorb = race
            .weapon_stats()
            .and_then(|w| w.absorb(oag_tables::weapons::Weapon::Autopilot))
            .unwrap_or(0.0);
        race.set_autopilot(true);
        // The main path is the absorb, whatever the race's default.
        race.set_opponent_autopilot(false);
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
            if tick < 900 || !tick.is_multiple_of(600) {
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
            race.set_opponent_autopilot(false);
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
    let wall_stint_a: u32 = pairs.iter().map(|(a, _)| a.wall_stint).sum();
    let wall_stint_b: u32 = pairs.iter().map(|(_, b)| b.wall_stint).sum();
    let resp_a: u32 = pairs.iter().map(|(a, _)| a.respawns).sum();
    let resp_b: u32 = pairs.iter().map(|(_, b)| b.respawns).sum();
    let shield_a: f32 = pairs.iter().map(|(a, _)| a.shield).sum::<f32>() / pairs.len() as f32;
    let shield_b: f32 = pairs.iter().map(|(_, b)| b.shield).sum::<f32>() / pairs.len() as f32;
    let n = pairs.len();
    let window_mean_a: f32 = pairs.iter().map(|(a, _)| a.window).sum::<f32>() / n as f32;
    let ms = median(&mut stint);
    let speed = window_mean_a / (window_ticks as f32 / 60.0);
    let mw = median(&mut window);
    println!(
        "SUMMARY {image} {track:?} {class} {difficulty} eps {eps:e} n {n} natural pickups {natural} | window A mean {window_mean_a:.0} | B-A stint median {ms:+.1} [{:+.1}, {:+.1}] window median {mw:+.1} [{:+.1}, {:+.1}] (q1 {:+.1} q3 {:+.1}) B faster {faster}/{n} = {:+.2} s | wall A {wall_a} B {wall_b} (stint A {wall_stint_a} B {wall_stint_b}) | respawns A {resp_a} B {resp_b} | shield mean A {shield_a:+.1} (absorbing {:+.1}) B {shield_b:+.1}",
        stint[0],
        stint[n - 1],
        window[0],
        window[n - 1],
        window[n / 4],
        window[3 * n / 4],
        mw / speed,
        shield_a + absorb,
    );
    pairs
}

fn difficulties() -> Vec<oag_ai::Difficulty> {
    match std::env::var("OAG_SWEEP_DIFFICULTY") {
        Ok(name) => vec![name.parse().expect("a difficulty")],
        Err(_) => vec![oag_ai::Difficulty::Novice, oag_ai::Difficulty::Ace],
    }
}

/// The perturbations each cell runs at: `OAG_SWEEP_GATE=1` runs
/// `perturb::GATE`'s five and a cell is judged on their median; unset, the
/// unperturbed run alone.
fn scales() -> Vec<f32> {
    if std::env::var_os("OAG_SWEEP_GATE").is_some() {
        crate::perturb::GATE.to_vec()
    } else {
        vec![0.0]
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
            for eps in scales() {
                sweep(image, *track, &class, difficulty, 1..=seeds(), 8, eps);
            }
        }
    }
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn pulse() {
    let tracks: Vec<String> = [
        "01", "02", "03", "04", "05", "06", "07", "09", "10", "13", "14", "16",
    ]
    .iter()
    .map(|id| format!(r"Data\Environments\{id}_Track\track.vex"))
    .collect();
    let tracks: Vec<Option<&str>> = tracks.iter().map(|t| Some(t.as_str())).collect();
    title("pulse-psp-usa.chd", &tracks);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn pure() {
    title("pure-psp-usa.chd", &[None]);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn hd() {
    title(
        "hdfury-ps3-eu-dec.iso",
        &[
            Some("/data/environments/12_sol_2/track.vex"),
            Some("/data/environments/talons_junction/track.vex"),
            Some("/data/environments/05_ubermall/track.vex"),
            Some("/data/environments/amphiseum/track.vex"),
        ],
    );
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

/// One whole race's field, opponents only.
#[derive(Debug, Default)]
struct Field {
    /// Mean finishing tick of the opponents that finished.
    finish: f32,
    /// Opponents that finished.
    finished: u32,
    /// Mean lap time in ticks over every completed opponent lap.
    lap: f32,
    /// Wall-contact ticks, respawns and wrecks over the field.
    wall: u32,
    respawns: u32,
    wrecked: u32,
    /// Mean field shield at each lap's start (lap 2 on), and at the end.
    shield_per_lap: Vec<f32>,
    shield_end: f32,
    /// Autopilot stints flown (rising edges of an opponent's timer).
    stints: u32,
}

fn whole_race(race: &mut oag_raceplay::Race, on: bool) -> Field {
    race.set_opponent_autopilot(on);
    race.set_autopilot(true);
    let count = race.sim.world.ship_count as usize;
    let mut out = Field::default();
    let mut timer = [0.0f32; 8];
    let mut lap = [0u32; 8];
    let mut lap_started = [0u64; 8];
    let mut laps: Vec<u64> = Vec::new();
    let mut finish = [None; 8];
    let mut shield_sum: Vec<(f32, u32)> = Vec::new();
    let mut last_stint_end = [None::<u64>; 8];
    let mut racing = [true; 8];
    // `OAG_SWEEP_WRECK_TRACE=1`: the last 400 ticks of a craft that wrecks.
    let wreck_trace = std::env::var_os("OAG_SWEEP_WRECK_TRACE").is_some();
    let mut history: Vec<std::collections::VecDeque<String>> = vec![Default::default(); 8];
    for tick in 1..=16_000u64 {
        race.tick_sim_only(&PlayerInputs::none());
        for slot in 1..count {
            let ship = &race.sim.world.ships[slot];
            if ship.autopilot_timer > timer[slot] {
                out.stints += 1;
            }
            if ship.autopilot_timer > 0.0 {
                last_stint_end[slot] = Some(tick);
            }
            let now_racing = ship.physics.craft_state == oag_physics::CraftState::Racing;
            if wreck_trace {
                let body = &ship.physics.body;
                history[slot].push_back(format!(
                    "    t {tick} shield {:.1} wall {} timer {:.2} grounded {} pos [{:.0} {:.0} {:.0}] v {:.1}",
                    ship.physics.shield,
                    race.wall_contact_ticks_of(slot),
                    ship.autopilot_timer,
                    ship.physics.grounded,
                    body.position.x,
                    body.position.y,
                    body.position.z,
                    body.linear_velocity.dot(body.forward()),
                ));
                if history[slot].len() > 400 {
                    history[slot].pop_front();
                }
            }
            if racing[slot] && !now_racing && !ship.standing.finished() {
                for line in &history[slot] {
                    println!("{line}");
                }
                println!(
                    "  WRECK on {on} slot {slot} tick {tick} last stint end {:?} ago {:?}",
                    last_stint_end[slot],
                    last_stint_end[slot].map(|t| tick - t)
                );
            }
            racing[slot] = now_racing;
            timer[slot] = ship.autopilot_timer;
            let now = ship.standing.lap;
            if now != lap[slot] {
                if lap[slot] > 0 {
                    laps.push(tick - lap_started[slot]);
                    let k = lap[slot] as usize - 1;
                    if shield_sum.len() <= k {
                        shield_sum.resize(k + 1, (0.0, 0));
                    }
                    shield_sum[k].0 += ship.physics.shield;
                    shield_sum[k].1 += 1;
                }
                lap[slot] = now;
                lap_started[slot] = tick;
            }
            if finish[slot].is_none() && ship.standing.finished() {
                finish[slot] = Some(tick);
            }
        }
        if (1..count).all(|s| {
            finish[s].is_some()
                || race.sim.world.ships[s].physics.craft_state != oag_physics::CraftState::Racing
        }) {
            break;
        }
    }
    for (slot, finished) in finish.iter().enumerate().take(count).skip(1) {
        out.wall += race.wall_contact_ticks_of(slot);
        out.respawns += race.respawns_of(slot);
        let ship = &race.sim.world.ships[slot];
        if ship.physics.craft_state != oag_physics::CraftState::Racing && finished.is_none() {
            out.wrecked += 1;
        }
        out.shield_end += ship.physics.shield / (count - 1) as f32;
    }
    let done: Vec<u64> = finish[1..count].iter().flatten().copied().collect();
    out.finished = done.len() as u32;
    out.finish = done.iter().sum::<u64>() as f32 / done.len().max(1) as f32;
    out.lap = laps.iter().sum::<u64>() as f32 / laps.len().max(1) as f32;
    out.shield_per_lap = shield_sum
        .iter()
        .map(|(s, n)| s / (*n).max(1) as f32)
        .collect();
    out
}

/// Whole races, switch off then on, same seed: the field's lap and finish
/// times, walls, respawns, wrecks and shield.
fn races(image: &str, tracks: &[Option<&str>]) {
    if std::env::var_os("OAG_SWEEP").is_none()
        || std::env::var("OAG_SWEEP_IMAGE").is_ok_and(|only| !image.contains(only.as_str()))
    {
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
            let mut lap_delta = Vec::new();
            let mut finish_delta = Vec::new();
            for seed in 1..=seeds() {
                let Some(mut off) = load(image, *track, &class, difficulty, seed, 0.0) else {
                    return;
                };
                let Some(mut on) = load(image, *track, &class, difficulty, seed, 0.0) else {
                    return;
                };
                let a = whole_race(&mut off, false);
                let b = whole_race(&mut on, true);
                println!(
                    "RACE {image} {track:?} {difficulty} seed {seed} | off lap {:.1}s finish {:.1}s ({}) wall {} resp {} wrecked {} shield/lap {:.1?} end {:.1} | on lap {:.1}s finish {:.1}s ({}) wall {} resp {} wrecked {} shield/lap {:.1?} end {:.1} stints {}",
                    a.lap / 60.0,
                    a.finish / 60.0,
                    a.finished,
                    a.wall,
                    a.respawns,
                    a.wrecked,
                    a.shield_per_lap,
                    a.shield_end,
                    b.lap / 60.0,
                    b.finish / 60.0,
                    b.finished,
                    b.wall,
                    b.respawns,
                    b.wrecked,
                    b.shield_per_lap,
                    b.shield_end,
                    b.stints,
                );
                lap_delta.push((b.lap - a.lap) / 60.0);
                finish_delta.push((b.finish - a.finish) / 60.0);
            }
            let n = lap_delta.len();
            let ml = median(&mut lap_delta);
            let mf = median(&mut finish_delta);
            println!(
                "RACESUM {image} {track:?} {difficulty} n {n} lap on-off median {ml:+.2}s [{:+.2}, {:+.2}] finish median {mf:+.2}s [{:+.2}, {:+.2}]",
                lap_delta[0],
                lap_delta[n - 1],
                finish_delta[0],
                finish_delta[n - 1]
            );
        }
    }
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn races_pulse() {
    let tracks: Vec<String> = [
        "01", "02", "03", "04", "05", "06", "07", "09", "10", "13", "14", "16",
    ]
    .iter()
    .map(|id| format!(r"Data\Environments\{id}_Track\track.vex"))
    .collect();
    let tracks: Vec<Option<&str>> = tracks.iter().map(|t| Some(t.as_str())).collect();
    races("pulse-psp-usa.chd", &tracks);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn races_others() {
    races("pure-psp-usa.chd", &[None]);
    races(
        "hdfury-ps3-eu-dec.iso",
        &[
            Some("/data/environments/12_sol_2/track.vex"),
            Some("/data/environments/talons_junction/track.vex"),
            Some("/data/environments/05_ubermall/track.vex"),
            Some("/data/environments/amphiseum/track.vex"),
        ],
    );
    races("omega-ps4-eu.pkg", &[None]);
    races("data/test-vpk/2048-PCSF00007-dlc1.vpk", &[None]);
}
