//! Scratch board for "the AI really struggles with de Konstruct Black".
//!
//! `#[ignore]`d and gated on `OAG_SWEEP`, printing rather than asserting. It
//! races `05_Track` forward and reversed (`track.vex` and `track_reversed.vex`)
//! with a lone Ace and with the full field, and logs by line index where craft
//! touch walls, are rescued or are destroyed.

use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_raceplay as race;
use oag_raceplay::catalogue;

const TICKS: u64 = 18_000;
const CLUSTER: u32 = 50;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry(reversed: bool) -> Option<String> {
    entry_of("05_Track", reversed)
}

fn entry_of(folder: &str, reversed: bool) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    let all = catalogue::tracks(&definition);
    all.into_iter()
        .find(|track| track.location.ends_with(folder) && track.reversed == reversed)
        .map(|track| track.entry_name())
}

#[derive(Default)]
struct Tally {
    runs: u32,
    craft: u32,
    destroyed: u32,
    respawns: u32,
    contact: u32,
    charged: f32,
    clusters: Vec<(u32, f32)>,
    deaths_at: Vec<u32>,
    respawns_at: Vec<u32>,
    end_shield: f32,
}

fn run(entry: &str, class: &str, field: bool, seed: u64, tally: &mut Tally) -> Option<u32> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry.to_string()),
        seed: Some(seed),
        weapons_override: std::env::var_os("OAG_WEAPONS_OFF").map(|_| false),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    let count = race.ship_count() as usize;
    let slots: Vec<usize> = if field {
        (1..count).collect()
    } else {
        vec![
            std::env::var("OAG_LONE_SLOT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
        ]
    };
    for slot in 0..count {
        if !slots.contains(&slot) {
            race.sim.world.ships[slot].active = false;
        }
    }
    let line_len = race.racing_line().len() as u32;
    let mut charge = vec![vec![0.0f32; (line_len / CLUSTER + 2) as usize]; count];
    let mut seen_charge = vec![0.0f32; count];
    let mut seen_resp = vec![0u32; count];
    let mut dead = vec![false; count];
    let mut best_laps = 0;
    let mut tk: Vec<(usize, u64, f32, u32)> = Vec::new();
    let mut tick_no = 0u64;
    let mut rs: Vec<(usize, u64, u32)> = Vec::new();
    let mut took = vec![false; count];
    let mut landed_ok: Vec<Option<u32>> = vec![None; count];
    for _ in 0..TICKS {
        tick_no += 1;
        let at: Vec<u32> = (0..count)
            .map(|s| race.sim.world.ships[s].driver.index)
            .collect();
        race.tick(&PlayerInputs::none());
        for &s in &slots {
            if !took[s]
                && race.sim.world.ships[s].physics.grounded == 0.0
                && (100..260).contains(&at[s])
            {
                took[s] = true;
                println!(
                    "TAKEOFF seed {seed} slot {s} idx {} v {:.1} accelcap {:?}",
                    at[s],
                    race.sim.world.ships[s]
                        .physics
                        .body
                        .linear_velocity
                        .length(),
                    race.sim.world.ships[s].handling.engine.accelcap
                );
                landed_ok[s] = Some(race.respawns_of(s));
                tk.push((
                    s,
                    tick_no,
                    race.sim.world.ships[s]
                        .physics
                        .body
                        .linear_velocity
                        .length(),
                    race.respawns_of(s),
                ));
            }
            let now = race.wall_shield_charged_of(s);
            if now > seen_charge[s] {
                if let Some(c) = charge[s].get_mut((at[s] / CLUSTER) as usize) {
                    *c += now - seen_charge[s];
                }
                seen_charge[s] = now;
            }
            let r = race.respawns_of(s);
            if r != seen_resp[s] {
                seen_resp[s] = r;
                tally.respawns_at.push(at[s]);
                rs.push((s, tick_no, at[s]));
            }
            if !dead[s] && race.sim.world.ships[s].physics.craft_state != CraftState::Racing {
                dead[s] = true;
                tally.deaths_at.push(at[s]);
            }
            best_laps = best_laps.max(race.sim.world.ships[s].standing.lap);
        }
    }
    for (s, t0, v, _) in &tk {
        let short = rs.iter().any(|(rs_s, t, i)| {
            rs_s == s && *t >= *t0 && *t <= *t0 + 400 && (150..260).contains(i)
        });
        println!("OUTCOME seed {seed} slot {s} v {v:.1} short {short}");
    }
    tally.runs += 1;
    for &s in &slots {
        tally.craft += 1;
        tally.respawns += race.respawns_of(s);
        tally.contact += race.wall_contact_ticks_of(s);
        tally.charged += race.wall_shield_charged_of(s);
        tally.end_shield += race.sim.world.ships[s].physics.shield;
        if dead[s] {
            tally.destroyed += 1;
        }
        for (b, c) in charge[s].iter().enumerate() {
            if *c > 0.0 {
                match tally
                    .clusters
                    .iter_mut()
                    .find(|(k, _)| *k == b as u32 * CLUSTER)
                {
                    Some(e) => e.1 += *c,
                    None => tally.clusters.push((b as u32 * CLUSTER, *c)),
                }
            }
        }
    }
    Some(best_laps)
}

fn summary(label: &str, class: &str, t: &Tally) -> String {
    let mut clusters = t.clusters.clone();
    clusters.sort_by(|a, b| b.1.total_cmp(&a.1));
    let top: Vec<String> = clusters
        .iter()
        .take(4)
        .map(|(i, c)| format!("{i}@{c:.0}"))
        .collect();
    let mut d = t.deaths_at.clone();
    d.sort_unstable();
    let mut r = t.respawns_at.clone();
    r.sort_unstable();
    format!(
        "{label:<14} {class:<8} craft {:<3} destroyed {:<3} respawns {:<3} contact {:<6} charged {:<8.1} mean-end-shield {:<6.1} top-clusters {top:?} deaths_at {d:?} respawns_at {r:?}\n",
        t.craft,
        t.destroyed,
        t.respawns,
        t.contact,
        t.charged,
        t.end_shield / t.craft.max(1) as f32,
    )
}

fn classes() -> Vec<String> {
    std::env::var("OAG_SWEEP_CLASS")
        .unwrap_or_else(|_| "VENOM,FLASH,RAPIER,PHANTOM".into())
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .collect()
}

fn seeds() -> Vec<u64> {
    let n: u64 = std::env::var("OAG_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    (1..=n).collect()
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP, read the board"]
fn dekonstruct_board() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let mut report = String::new();
    if std::env::var_os("OAG_WEAPONS_OFF").is_some() {
        report.push_str("WEAPONS OFF\n");
    }
    for class in classes() {
        for reversed in [false, true] {
            let Some(entry) = entry(reversed) else { return };
            let name = if reversed {
                "05 White (rev)"
            } else {
                "05 Black (fwd)"
            };
            for field in [false, true] {
                let mut t = Tally::default();
                for seed in if field { seeds() } else { vec![1] } {
                    run(&entry, &class, field, seed, &mut t);
                }
                let label = format!("{name} {}", if field { "field" } else { "lone" });
                report.push_str(&summary(&label, &class, &t));
            }
        }
    }
    println!("\n{report}");
}

/// Every circuit, both directions, the field only: is de Konstruct an outlier?
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP, read the board"]
fn field_census() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let mut report = String::new();
    for class in classes() {
        for folder in [
            "01", "02", "03", "04", "05", "06", "07", "09", "10", "13", "14", "16",
        ] {
            for reversed in [false, true] {
                let Some(entry) = entry_of(&format!("{folder}_Track"), reversed) else {
                    continue;
                };
                let mut t = Tally::default();
                for seed in seeds() {
                    run(&entry, &class, true, seed, &mut t);
                }
                let label = format!("{folder}{}", if reversed { " rev" } else { " fwd" });
                report.push_str(&summary(&label, &class, &t));
            }
        }
    }
    println!("\n{report}");
}

/// A per-craft trace of the first ticks, for reading where a field goes wrong.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn field_trace() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(entry) = entry(false) else { return };
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".into());
    let lone = std::env::var_os("OAG_LONE").is_some();
    let from: u64 = std::env::var("OAG_FROM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let to: u64 = std::env::var("OAG_TO")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(900);
    let image = image().unwrap();
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class,
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        seed: Some(1),
        weapons_override: std::env::var_os("OAG_WEAPONS_OFF").map(|_| false),
        ..race::Options::default()
    })
    .unwrap();
    let mut race = race::Race::start(loaded.setup);
    let count = race.ship_count() as usize;
    for slot in 0..count {
        if slot == 0
            || (lone
                && slot
                    != std::env::var("OAG_LONE_SLOT")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(1))
        {
            race.sim.world.ships[slot].active = false;
        }
    }
    for tick in 0..to {
        race.tick(&PlayerInputs::none());
        if tick >= from && tick % 6 == 0 {
            let mut line = format!("t{tick:<5}");
            for s in 1..count {
                let sh = &race.sim.world.ships[s];
                if !sh.active {
                    continue;
                }
                let off = race
                    .ai_sample_for(s, sh.driver.index as usize)
                    .map_or(0.0, |sm| {
                        let lat = oag_core::math::Vec3::from_array(sm.lateral);
                        (sh.physics.body.position - oag_core::math::Vec3::from_array(sm.pos))
                            .dot(lat)
                    });
                line.push_str(&format!(
                    " | {:>4} o{:>6.1} v{:>5.1} t{:.2} b{:.2} g{:.1} s{:>5.1} r{}",
                    sh.driver.index,
                    off,
                    sh.physics.body.linear_velocity.length(),
                    sh.physics.thrust,
                    sh.physics.brake,
                    sh.physics.grounded,
                    sh.physics.shield,
                    race.respawns_of(s)
                ));
            }
            println!("{line}");
        }
    }
}

#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn line_profile() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(entry) = entry(false) else { return };
    let image = image().unwrap();
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".into(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        ..race::Options::default()
    })
    .unwrap();
    let race = race::Race::start(loaded.setup);
    let line = race.racing_line();
    let from: usize = std::env::var("OAG_FROM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(480);
    let to: usize = std::env::var("OAG_TO")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(660);
    for i in (from..to).step_by(4) {
        let k = line.max_curvature(i, 6.0, 11.0);
        let t = if k > 1e-6 {
            (260.0f32 / k).sqrt().min(1.556 / k)
        } else {
            f32::INFINITY
        };
        let fr = line.aim(i, 0.0).corridor;
        println!(
            "PROFILE idx {i} k11 {k:.4} target {t:.1} left {:?} right {:?} line {:?}",
            fr.map(|f| f.left),
            fr.map(|f| f.right),
            line.point(i)
        );
    }
}
