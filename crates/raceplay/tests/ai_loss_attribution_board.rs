//! Scratch board: who killed, or stopped, each de Konstruct opponent.
//!
//! `#[ignore]`d and gated on `OAG_SWEEP`, printing rather than asserting. It
//! replays the races `ai_dekonstruct_black_ground_truth` and
//! `ai_dekonstruct_symptoms_ground_truth` run and, for every opponent that
//! leaves racing and every dead stop, prints the source of the shield it lost:
//! a weapon blow (the nearest projectile of the tick before, by kind and
//! firer), wall charge (`wall_shield_charged_of`), a barrel roll
//! (`roll_shield_spent_of`) or the respawn cost, plus the shield history.
//! Read-only: nothing here touches state a hash reads.

use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_raceplay as race;
use oag_raceplay::catalogue;

const TICKS: u64 = 18_000;
const STOP_FROM: f32 = 60.0;
const STOP_TO: f32 = 15.0;
const STOP_WITHIN: usize = 20;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry(reversed: bool) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.location.ends_with("05_Track") && track.reversed == reversed)
        .map(|track| track.entry_name())
}

#[derive(Default, Clone)]
struct Craft {
    wall: f32,
    weapon: f32,
    roll: f32,
    respawn: f32,
    pickup: f32,
    dead_tick: Option<u64>,
    /// (tick, weapon loss, wall charge) events, kept for the last-2-s window.
    events: Vec<(u64, f32, f32, String)>,
    history: Vec<f32>,
    stop_hist: Vec<f32>,
    lap: u32,
    /// Shield on entering each lap, `(lap, shield)`.
    per_lap: Vec<(u32, f32)>,
    start: f32,
}

fn run(class: &str, seed: u64, reversed: bool) -> Option<()> {
    let entry = entry(reversed)?;
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        seed: Some(seed),
        weapons_override: std::env::var_os("OAG_WEAPONS_OFF").map(|_| false),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].active = false;
    let count = race.ship_count() as usize;
    let mut c = vec![Craft::default(); count];
    let mut prev_wall = vec![0.0f32; count];
    let mut prev_roll = vec![0.0f32; count];
    let mut prev_resp = vec![0u32; count];
    let mut prev_contact = vec![0u32; count];
    let tag = format!(
        "{class} seed {seed} {}",
        if reversed { "REV" } else { "FWD" }
    );
    for tick in 1..=TICKS {
        let before_shield: Vec<f32> = (0..count)
            .map(|s| race.sim.world.ships[s].physics.shield)
            .collect();
        let shots: Vec<_> = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|p| p.kind.is_some())
            .map(|p| (format!("{:?}", p.kind.unwrap()), p.owner, p.position))
            .collect();
        race.tick(&PlayerInputs::none());
        for s in 1..count {
            let ship = &race.sim.world.ships[s];
            let now = ship.physics.shield;
            let wall = race.wall_shield_charged_of(s);
            let roll = race.roll_shield_spent_of(s);
            let resp = race.respawns_of(s);
            let dwall = wall - prev_wall[s];
            let droll = roll - prev_roll[s];
            let dresp = resp - prev_resp[s];
            prev_wall[s] = wall;
            prev_roll[s] = roll;
            prev_resp[s] = resp;
            let drop = before_shield[s] - now;
            let racing = ship.physics.craft_state == CraftState::Racing;
            if drop > 0.0 && c[s].dead_tick.is_none() {
                let respawn_cost = if dresp > 0 { 1.0 } else { 0.0 };
                let weapon = (drop - dwall - droll - respawn_cost).max(0.0);
                let wall_part = dwall.min(drop);
                c[s].wall += wall_part;
                c[s].roll += droll.min(drop);
                c[s].respawn += respawn_cost;
                c[s].weapon += weapon;
                if weapon > 0.5 || wall_part > 0.5 {
                    // nearest foreign projectile of the tick before
                    let pos = ship.physics.body.position;
                    let near = shots
                        .iter()
                        .filter(|(_, o, _)| *o as usize != s)
                        .map(|(k, o, p)| (k.clone(), *o, (*p - pos).length()))
                        .min_by(|a, b| a.2.total_cmp(&b.2));
                    let label = match near {
                        Some((k, o, d)) if weapon > 0.5 && d < 16.0 => {
                            format!("{k} from {o} at {d:.1}")
                        }
                        _ => "unattributed".to_string(),
                    };
                    c[s].events.push((
                        tick,
                        weapon,
                        wall_part,
                        format!("idx {} {label}", ship.driver.index),
                    ));
                }
            } else if drop < 0.0 {
                c[s].pickup += -drop;
            }
            c[s].history.push(now);
            if c[s].history.len() == 1 {
                c[s].start = before_shield[s];
            }
            if ship.standing.lap != c[s].lap {
                c[s].lap = ship.standing.lap;
                let lap = c[s].lap;
                c[s].per_lap.push((lap, now));
            }
            if let Ok(spec) = std::env::var("OAG_TRACE") {
                let mut it = spec.split(':');
                let (ts, t0, t1) = (
                    it.next().and_then(|v| v.parse::<usize>().ok()),
                    it.next().and_then(|v| v.parse::<u64>().ok()),
                    it.next().and_then(|v| v.parse::<u64>().ok()),
                );
                if Some(s) == ts
                    && tick >= t0.unwrap_or(0)
                    && tick <= t1.unwrap_or(0)
                    && tick
                        % std::env::var("OAG_STEP")
                            .ok()
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(4)
                        == 0
                {
                    let b = &ship.physics.body;
                    println!(
                        "TRACE {tag} slot {s} t{tick} idx {} v {:.1} fwd {:.1} shield {now:.1} grounded {:.2} pos {:.1} {:.1} {:.1} resp {resp} contacts {}",
                        ship.driver.index,
                        b.linear_velocity.length(),
                        b.linear_velocity.dot(b.forward()),
                        ship.physics.grounded,
                        b.position.x,
                        b.position.y,
                        b.position.z,
                        race.wall_contact_ticks_of(s)
                    );
                }
            }
            if !racing && c[s].dead_tick.is_none() {
                c[s].dead_tick = Some(tick);
                let cr = &c[s];
                let from = tick.saturating_sub(120);
                let (w2, a2) = cr
                    .events
                    .iter()
                    .filter(|e| e.0 >= from)
                    .fold((0.0, 0.0), |(w, a), e| (w + e.1, a + e.2));
                let last: Vec<String> = cr
                    .events
                    .iter()
                    .rev()
                    .take(3)
                    .map(|e| format!("t{} w{:.1} a{:.1} {}", e.0, e.1, e.2, e.3))
                    .collect();
                let src = if w2 > a2 {
                    "WEAPON"
                } else if a2 > 0.0 {
                    "WALL"
                } else {
                    "OTHER"
                };
                println!(
                    "DEAD {tag} slot {s} tick {tick} idx {} {src} last2s weapon {w2:.1} wall {a2:.1} | totals weapon {:.1} wall {:.1} roll {:.1} respawn {:.1} pickup {:.1} start {:.1} per-lap-shield {:?} end-shield {now:.1} | last events {last:?}",
                    ship.driver.index,
                    cr.weapon,
                    cr.wall,
                    cr.roll,
                    cr.respawn,
                    cr.pickup,
                    cr.start,
                    cr.per_lap
                );
            }
            let body = &ship.physics.body;
            let speed = body.linear_velocity.dot(body.forward()).max(0.0);
            let contact = race.wall_contact_ticks_of(s) != prev_contact[s];
            prev_contact[s] = race.wall_contact_ticks_of(s);
            let h = &mut c[s].stop_hist;
            h.push(speed);
            let from = h.len().saturating_sub(STOP_WITHIN);
            let peak = h[from..].iter().copied().fold(0.0, f32::max);
            if contact && speed <= STOP_TO && peak >= STOP_FROM {
                let recent: Vec<String> = c[s]
                    .events
                    .iter()
                    .filter(|e| e.0 + 60 >= tick)
                    .map(|e| format!("t{} w{:.1} a{:.1} {}", e.0, e.1, e.2, e.3))
                    .collect();
                println!(
                    "STOP {tag} slot {s} tick {tick} idx {} peak {peak:.0} shield {now:.1} state {:?} events in prior 60 ticks {recent:?}",
                    ship.driver.index, ship.physics.craft_state
                );
                c[s].stop_hist.clear();
            }
        }
    }
    let dead = c.iter().skip(1).filter(|x| x.dead_tick.is_some()).count();
    let respawns: u32 = (1..count).map(|s| race.respawns_of(s)).sum();
    let end: Vec<String> = (1..count)
        .map(|s| format!("{:.0}", race.sim.world.ships[s].physics.shield))
        .collect();
    let mut laps: Vec<String> = Vec::new();
    for lap in 1..=6u32 {
        let v: Vec<f32> = c
            .iter()
            .skip(1)
            .filter_map(|x| x.per_lap.iter().find(|(l, _)| *l == lap).map(|(_, sh)| *sh))
            .collect();
        if !v.is_empty() {
            laps.push(format!(
                "lap{lap}:{:.1}(n{})",
                v.iter().sum::<f32>() / v.len() as f32,
                v.len()
            ));
        }
    }
    println!(
        "SUMMARY {tag} destroyed {dead} respawns {respawns} per-lap-entry-shield-mean {laps:?} end-of-run-shield {end:?}"
    );
    Some(())
}

#[test]
#[ignore = "a scratch board: set OAG_SWEEP, read the lines"]
fn attribute() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let cells: Vec<(String, u64)> = match std::env::var("OAG_CELLS") {
        Ok(v) => v
            .split(',')
            .filter_map(|c| {
                c.split_once(':')
                    .map(|(a, b)| (a.to_string(), b.parse().unwrap()))
            })
            .collect(),
        Err(_) => ["VENOM", "FLASH", "RAPIER", "PHANTOM"]
            .iter()
            .flat_map(|k| (1..=3).map(move |s| (k.to_string(), s)))
            .collect(),
    };
    let only_fwd = std::env::var_os("OAG_FWD_ONLY").is_some();
    for (class, seed) in cells {
        for reversed in [false, true] {
            if only_fwd && reversed {
                continue;
            }
            if run(&class, seed, reversed).is_none() {
                return;
            }
        }
    }
}
