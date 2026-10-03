//! Where does a craft actually leave the track in this engine, and why.
//!
//! **`#[ignore]`d and `OAG_SWEEP`-gated, never run in CI or `just test-data`.**
//! A measurement pass, not a tuning pass: every test prints rather than
//! asserts, and the printed lines are machine-readable so a table can be built
//! from them without editing the test.
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all \
//!   falloff_survey_ground_truth --no-capture
//! ```
//!
//! # Two populations, labelled separately, because they hit different triggers
//!
//! - `ai`: one opponent alone on the circuit. Rescued by
//!   [`oag_game::race::RespawnCause::LostCircuit`] (8 half-widths from its own
//!   driver's sample, 90 ticks), `Stalled`, or an authored `ResetZone`.
//! - `auto`: slot 0, the player's craft, flown by the same driver. Rescued by
//!   `OffTrack` (2 half-widths from the *nearest* sample, 45 ticks) or
//!   `ResetZone`.
//!
//! Three of the four triggers are this project's own inventions, not the
//! original's (see `oag_race::recovery`), so a "fall" recorded here may be one
//! of those firing where the original has no mechanism at all.
//!
//! # How an event is classified
//!
//! **At the first tick of departure, not at the respawn tick.** A rescue
//! fires 45-90 ticks after the craft left, and reading the state at the rescue
//! reads a craft already far away - `docs/gameplay/ai.md` records two
//! conclusions reached backwards from the rescue that were both wrong. A
//! per-craft ring of the last [`RING`] ticks is kept, and on a respawn it is
//! walked backwards for the start of the unbroken run of ticks that ended in
//! it: outside the local half-width on either side if the craft ever got that
//! far out, else the start of the final unbroken airborne stretch (a craft that
//! never left its own corridor and still fell, i.e. through a floor).
//!
//! Every event line carries, at that departure tick: whether the craft was
//! airborne and for how long, its speed, which side and how far past the edge,
//! and whether the hull touched a wall in the [`WALL_LOOKBACK`] ticks before.

use std::path::PathBuf;

use oag_game::race::{self, RespawnCause};
use oag_gameplay::PlayerInputs;

const TICKS: u64 = 18_000;
const LONE: usize = 1;
const RING: usize = 1200;
const WALL_LOOKBACK: usize = 45;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

struct Circuit {
    id: String,
    entry: String,
    location: String,
    reversed: bool,
}

fn circuits_all() -> Vec<Circuit> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    oag_game::catalogue::tracks(&definition)
        .into_iter()
        .map(|track| Circuit {
            id: track.id.clone(),
            entry: track.entry_name(),
            location: track.location.clone(),
            reversed: track.reversed,
        })
        .collect()
}

#[derive(Clone, Copy, Default)]
struct Rec {
    tick: u64,
    /// Signed lateral offset over the local half-width on that side, so
    /// magnitude above 1 is outside the authored corridor. Positive is the
    /// spline's `lateral` direction.
    lat_frac: f32,
    airborne: bool,
    t_air: f32,
    speed: f32,
    wall: bool,
    index: usize,
    section: u8,
    grounded: f32,
    pos: [f32; 3],
}

#[derive(Default)]
struct RunSummary {
    ticks: u64,
    airborne_ticks: u64,
    wall_ticks: u64,
    max_lat_frac: f32,
    ticks_over_80: u64,
    events: u32,
    laps: u32,
}

fn measure(
    circuit: &Circuit,
    level: oag_ai::Difficulty,
    level_name: &str,
    population: &str,
    class: &str,
) -> Option<()> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(circuit.entry.clone()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    let slot = if population == "ai" { LONE } else { 0 };
    for other in 0..8 {
        if other != slot {
            race.sim.world.ships[other].active = false;
        }
    }
    if slot == 0 {
        race.set_autopilot(true);
        race.set_autopilot_pilot(oag_ai::Pilot::BALANCED, level);
    } else {
        race.sim.world.ships[slot].active = true;
    }

    let label = format!("{}{}", circuit.id, if circuit.reversed { "r" } else { "" });
    let mut ring: std::collections::VecDeque<Rec> = std::collections::VecDeque::new();
    let mut prev_respawns = race.respawns_of(slot);
    let mut prev_wall_counter = race.wall_contact_ticks_of(slot);
    let mut summary = RunSummary::default();

    for tick in 0..TICKS {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[slot];
        let position = ship.physics.body.position;
        let speed = ship.physics.body.linear_velocity.length();
        let t_air = ship.physics.time_airborne;
        let wall_now = if slot == 0 {
            ship.physics.wall_contact_prev
        } else {
            let counter = race.wall_contact_ticks_of(slot);
            let hit = counter != prev_wall_counter;
            prev_wall_counter = counter;
            hit
        };
        let (index, lat_frac, section) = match race.spline().nearest(position) {
            Some((index, sample, _)) => {
                let lateral = oag_core::math::Vec3::from_array(sample.lateral);
                let lat = (position - oag_core::math::Vec3::from_array(sample.pos)).dot(lateral);
                let half = if lat >= 0.0 {
                    sample.half_width_right
                } else {
                    sample.half_width_left
                };
                (index, lat / half.max(1e-3), sample.section_id)
            }
            None => (0, 0.0, 0),
        };
        let rec = Rec {
            tick,
            lat_frac,
            airborne: t_air > 0.0,
            t_air,
            speed,
            wall: wall_now,
            index,
            section,
            grounded: ship.physics.grounded,
            pos: position.to_array(),
        };
        summary.ticks += 1;
        summary.airborne_ticks += u64::from(rec.airborne);
        summary.wall_ticks += u64::from(rec.wall);
        summary.max_lat_frac = summary.max_lat_frac.max(lat_frac.abs());
        summary.ticks_over_80 += u64::from(lat_frac.abs() > 0.8);
        if ring.len() == RING {
            ring.pop_front();
        }
        ring.push_back(rec);
        let now_respawns = race.respawns_of(slot);
        if now_respawns != prev_respawns {
            prev_respawns = now_respawns;
            summary.events += 1;
            let cause = match race.last_respawn_cause_of(slot) {
                Some(RespawnCause::ResetZone) => "reset",
                Some(RespawnCause::LostCircuit) => "lost",
                Some(RespawnCause::Beneath) => "beneath",
                Some(RespawnCause::Stalled) => "stalled",
                Some(RespawnCause::OffTrack) => "offtrack",
                Some(RespawnCause::Airborne) => "airborne",
                Some(RespawnCause::Destroyed) => "wrecked",
                None => "?",
            };
            // The tick that fired the respawn also teleported the craft, so the
            // newest record is the post-teleport pose and not part of the fall.
            ring.pop_back();
            report_event(&label, level_name, population, cause, tick, &ring);
            // The ring is history from before the teleport.
            ring.clear();
        }
    }
    summary.laps = race.sim.world.ships[slot].standing.lap;
    println!(
        "SUM|{label}|{level_name}|{population}|{class}|ticks {}|laps {}|events {}|air {}|wall {}|\
         maxlat {:.2}|over80 {}",
        summary.ticks,
        summary.laps,
        summary.events,
        summary.airborne_ticks,
        summary.wall_ticks,
        summary.max_lat_frac,
        summary.ticks_over_80
    );
    Some(())
}

fn report_event(
    label: &str,
    level: &str,
    population: &str,
    cause: &str,
    tick: u64,
    ring: &std::collections::VecDeque<Rec>,
) {
    let recs: Vec<Rec> = ring.iter().copied().collect();
    if recs.is_empty() {
        println!("EV|{label}|{level}|{population}|{tick}|{cause}|empty-ring");
        return;
    }
    // Walk backwards for the start of the unbroken outside-run that ends here.
    let mut start = recs.len();
    for (at, rec) in recs.iter().enumerate().rev() {
        if rec.lat_frac.abs() > 1.0 {
            start = at;
        } else {
            break;
        }
    }
    let mut kind = "edge";
    if start == recs.len() {
        // Never outside its corridor: the start of the final airborne run.
        kind = "corridor";
        start = recs.len();
        for (at, rec) in recs.iter().enumerate().rev() {
            if rec.airborne {
                start = at;
            } else {
                break;
            }
        }
        if start == recs.len() {
            start = recs.len() - 1;
            kind = "corridor-grounded";
        }
    }
    let d = recs[start];
    let wall_before = recs[start.saturating_sub(WALL_LOOKBACK)..=start]
        .iter()
        .any(|rec| rec.wall);
    // How long it had been airborne when it departed, counting the run that
    // straddles the departure tick backwards.
    let mut air_run = 0u32;
    for rec in recs[..=start].iter().rev() {
        if rec.airborne {
            air_run += 1;
        } else {
            break;
        }
    }
    println!(
        "EV|{label}|{level}|{population}|rescue_tick {tick}|{cause}|{kind}|dep_tick {}|\
         sec {}|idx {}|side {}|lat {:.2}|speed {:.0}|air_run {air_run}|t_air {:.2}|wall45 {}|\
         dwell {}",
        d.tick,
        d.section,
        d.index,
        if d.lat_frac >= 0.0 { "R" } else { "L" },
        d.lat_frac,
        d.speed,
        d.t_air,
        u8::from(wall_before),
        tick - d.tick,
    );
    if std::env::var_os("OAG_TRACE_EVENT").is_some() {
        let from = recs.len().saturating_sub(
            std::env::var("OAG_TRACE_LEN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
        );
        for rec in &recs[from..] {
            println!(
                "TR|{label}|{level}|{population}|rescue {tick}|t {}|idx {}|sec {}|lat {:.2}|g {:.1}|\
                 air {:.2}|speed {:.0}|wall {}|pos {:.0},{:.0},{:.0}",
                rec.tick,
                rec.index,
                rec.section,
                rec.lat_frac,
                rec.grounded,
                rec.t_air,
                rec.speed,
                u8::from(rec.wall),
                rec.pos[0],
                rec.pos[1],
                rec.pos[2],
            );
        }
    }
}

fn sweep(levels: &[(&str, oag_ai::Difficulty)], populations: &[&str], class: &str, only: &[&str]) {
    let mut circuits = circuits_all();
    circuits.sort_by(|a, b| {
        a.location
            .cmp(&b.location)
            .then(a.reversed.cmp(&b.reversed))
    });
    for circuit in circuits {
        if !only.is_empty() && !only.contains(&circuit.id.as_str()) {
            continue;
        }
        for &(name, level) in levels {
            for population in populations {
                if measure(&circuit, level, name, population, class).is_none() {
                    return;
                }
            }
        }
    }
}

/// Every circuit-direction, Ace and Novice, both populations.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn where_craft_leave_the_track() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let only: Vec<String> = std::env::var("OAG_ONLY")
        .map(|value| value.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    let only: Vec<&str> = only.iter().map(String::as_str).collect();
    let class = std::env::var("OAG_CLASS").unwrap_or_else(|_| "VENOM".to_string());
    let levels: Vec<(&str, oag_ai::Difficulty)> = oag_ai::Difficulty::ALL
        .iter()
        .copied()
        .filter(|(name, _)| {
            std::env::var("OAG_TIERS").map_or(*name == "ace", |tiers| tiers.contains(name))
        })
        .collect();
    let populations: Vec<&str> = match std::env::var("OAG_POP").as_deref() {
        Ok("ai") => vec!["ai"],
        Ok("auto") => vec!["auto"],
        _ => vec!["ai", "auto"],
    };
    sweep(&levels, &populations, &class, &only);
}

/// Does every grid slot spawn over a floor, on every circuit-direction?
///
/// One craft at a time (everyone else off), 400 ticks, which is past the
/// countdown. Prints the longest unbroken airborne run and whether the craft
/// was respawned in that window, for each slot. A slot whose craft is airborne
/// for hundreds of ticks from the gantry has been spawned over nothing.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn grid_slots_spawn_over_a_floor() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let mut circuits = circuits_all();
    circuits.sort_by(|a, b| {
        a.location
            .cmp(&b.location)
            .then(a.reversed.cmp(&b.reversed))
    });
    let Some(image) = image() else { return };
    for circuit in circuits {
        let mut row = format!(
            "GRID|{}{}",
            circuit.id,
            if circuit.reversed { "r" } else { "" }
        );
        for slot in 0..8usize {
            let Ok(loaded) = race::load(&race::Options {
                source: image.display().to_string(),
                class: "VENOM".to_string(),
                mode: oag_race::Mode::SingleRace,
                difficulty: oag_ai::Difficulty::Ace,
                track: Some(circuit.entry.clone()),
                ..race::Options::default()
            }) else {
                return;
            };
            let mut race = race::Race::start(loaded.setup);
            for other in 0..8 {
                if other != slot {
                    race.sim.world.ships[other].active = false;
                }
            }
            if slot == 0 {
                race.set_autopilot(true);
            }
            let spawn = race.sim.world.ships[slot].physics.body.position;
            let spawn_lat = race
                .spline()
                .nearest(spawn)
                .map(|(_, sample, _)| {
                    let lat = (spawn - oag_core::math::Vec3::from_array(sample.pos))
                        .dot(oag_core::math::Vec3::from_array(sample.lateral));
                    let half = if lat >= 0.0 {
                        sample.half_width_right
                    } else {
                        sample.half_width_left
                    };
                    lat / half.max(1e-3)
                })
                .unwrap_or(0.0);
            let (mut run, mut longest) = (0u32, 0u32);
            for _ in 0..400 {
                race.tick(&PlayerInputs::none());
                if race.sim.world.ships[slot].physics.time_airborne > 0.0 {
                    run += 1;
                    longest = longest.max(run);
                } else {
                    run = 0;
                }
            }
            row.push_str(&format!(
                "|s{slot} lat {spawn_lat:+.2} air {longest}{}",
                if race.respawns_of(slot) > 0 {
                    " RESP"
                } else {
                    ""
                }
            ));
        }
        println!("{row}");
    }
}

fn throttle_held() -> oag_gameplay::InputSnapshot {
    let mut buttons = oag_gameplay::input::Input::new();
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// A human-error stand-in that needs no input model: put the player's craft on
/// the racing line at a sampled point, turn it `angle` degrees off the
/// tangent toward one side, give it `speed` along that heading, hold the
/// throttle and steer nothing, and watch 200 ticks.
///
/// **This is a stress test of the containment the track gives, not a
/// model of a driver.** What it answers is where a craft that is pointing the
/// wrong way at speed is held on the circuit (a wall) and where it leaves (an
/// open edge, or through a wall). Output lines are `SHOVE|...`.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn shove_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let only: Vec<String> = std::env::var("OAG_ONLY")
        .map(|value| value.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    let positions: usize = std::env::var("OAG_POSITIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    let Some(image) = image() else { return };
    let mut circuits = circuits_all();
    circuits.sort_by(|a, b| {
        a.location
            .cmp(&b.location)
            .then(a.reversed.cmp(&b.reversed))
    });
    for circuit in circuits {
        if !only.is_empty() && !only.contains(&circuit.id) {
            continue;
        }
        let Ok(loaded) = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            difficulty: oag_ai::Difficulty::Ace,
            track: Some(circuit.entry.clone()),
            ..race::Options::default()
        }) else {
            return;
        };
        let mut race = race::Race::start(loaded.setup);
        for other in 1..8 {
            race.sim.world.ships[other].active = false;
        }
        let label = format!("{}{}", circuit.id, if circuit.reversed { "r" } else { "" });
        let throttle = throttle_held();
        // Past the countdown, so the craft is allowed to move.
        for _ in 0..600 {
            race.tick(&PlayerInputs::single(throttle));
        }
        let height = oag_gameplay::spawn::spawn_height(&race.sim.world.ships[0].handling);
        let len = race.spline().len();
        let step = (len / positions).max(1);
        for index in (0..len).step_by(step) {
            let Some(sample) = race.spline().sample(index).copied() else {
                continue;
            };
            for side in [-1.0f32, 1.0] {
                for angle in [10.0f32, 25.0, 45.0] {
                    for speed in [110.0f32, 190.0] {
                        let mut pose =
                            oag_gameplay::Pose::from_sample(&sample, sample.racing_line, height);
                        let up = pose.orientation * oag_core::math::Vec3::Y;
                        let turn =
                            oag_core::math::Quat::from_axis_angle(up, side * angle.to_radians());
                        pose.orientation = turn * pose.orientation;
                        let forward = pose.orientation * oag_core::math::Vec3::NEG_Z;
                        race.sim.world.ships[0].place_at(pose);
                        race.sim.world.ships[0].physics.body.linear_velocity = forward * speed;
                        let respawns_before = race.respawns_of(0);
                        let (mut max_lat, mut out_run, mut worst_run, mut wall_ticks) =
                            (0.0f32, 0u32, 0u32, 0u32);
                        let mut crossed = 0u32;
                        let mut previous = race.sim.world.ships[0].physics.body.position;
                        for _ in 0..200 {
                            race.tick(&PlayerInputs::single(throttle));
                            let ship = &race.sim.world.ships[0];
                            wall_ticks += u32::from(ship.physics.wall_contact_prev);
                            {
                                use oag_physics::{Ray, Raycaster, Surface};
                                let now = ship.physics.body.position;
                                let delta = now - previous;
                                let length = delta.length();
                                // A respawn teleports the craft; that is not a crossing.
                                if length > 1e-3
                                    && length < 20.0
                                    && race
                                        .collision()
                                        .raycast(
                                            Ray::new(previous, delta / length, length),
                                            None,
                                            false,
                                        )
                                        .is_some_and(|hit| hit.surface == Surface::Wall)
                                {
                                    crossed += 1;
                                }
                                previous = now;
                            }
                            if let Some((_, s, _)) =
                                race.spline().nearest(ship.physics.body.position)
                            {
                                let lat = (ship.physics.body.position
                                    - oag_core::math::Vec3::from_array(s.pos))
                                .dot(oag_core::math::Vec3::from_array(s.lateral));
                                let half = if lat >= 0.0 {
                                    s.half_width_right
                                } else {
                                    s.half_width_left
                                };
                                let frac = (lat / half.max(1e-3)).abs();
                                max_lat = max_lat.max(frac);
                                if frac > 1.3 {
                                    out_run += 1;
                                    worst_run = worst_run.max(out_run);
                                } else {
                                    out_run = 0;
                                }
                            }
                        }
                        let respawned = race.respawns_of(0) - respawns_before;
                        let far = worst_run >= 30;
                        let left = respawned > 0 || far;
                        let gave_up = race.respawn_given_up_of(0);
                        println!(
                            "SHOVE|{label}|idx {index}|sec {}|side {}|ang {angle:.0}|spd {speed:.0}|\
                             left {}|wall {wall_ticks}|maxlat {max_lat:.2}|resp {respawned}|far {}|\
                             crossed {crossed}|gaveup {}|hw {:.0}",
                            sample.section_id,
                            if side < 0.0 { "L" } else { "R" },
                            u8::from(left),
                            u8::from(far),
                            u8::from(gave_up),
                            sample.half_width_left.min(sample.half_width_right),
                        );
                        // Let any respawn cooldown drain and the ship settle.
                        race.sim.world.ships[0].physics.body.linear_velocity =
                            oag_core::math::Vec3::ZERO;
                    }
                }
            }
        }
    }
}

/// What contains a craft at each side of the racing line, statically.
///
/// For every fourth spline sample, from a point on the line lifted 1.5 units:
/// a ray along `+lateral` and one along `-lateral`, three half-widths long,
/// reporting the first surface met (`Wall`, `Floor`, ...) and its distance in
/// half-widths; and, past the authored edge, whether a floor exists to hold a
/// craft that got there (a downward cast at 1.15 and 1.6 half-widths).
///
/// Per circuit the printed row counts samples by class, per side:
/// `walled` (a `Wall` within 1.2 half-widths), `far-wall` (a wall between 1.2
/// and 3), `open` (no wall within 3 half-widths). `open` is then split into
/// `shoulder` (floor at 1.15) and `void` (no floor there either): `void` is
/// the only class where a craft that leaves the corridor is not held by
/// anything at all. `EDGE|` lines carry the per-sample rows for `void` runs.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn edge_profile() {
    use oag_physics::{Ray, Raycaster, Surface};
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else { return };
    let mut circuits = circuits_all();
    circuits.sort_by(|a, b| {
        a.location
            .cmp(&b.location)
            .then(a.reversed.cmp(&b.reversed))
    });
    for circuit in circuits {
        let Ok(loaded) = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            difficulty: oag_ai::Difficulty::Ace,
            track: Some(circuit.entry.clone()),
            ..race::Options::default()
        }) else {
            return;
        };
        let race = race::Race::start(loaded.setup);
        let collision = race.collision();
        let label = format!("{}{}", circuit.id, if circuit.reversed { "r" } else { "" });
        let len = race.spline().len();
        // [side][class]: walled, far-wall, shoulder, void
        let mut counts = [[0u32; 4]; 2];
        let mut void_runs: Vec<Vec<(usize, u8, f32)>> = vec![Vec::new(), Vec::new()];
        let mut sampled = 0u32;
        for index in (0..len).step_by(4) {
            let Some(sample) = race.spline().sample(index).copied() else {
                continue;
            };
            sampled += 1;
            let up = -oag_core::math::Vec3::from_array(sample.down).normalize_or_zero();
            let lateral = oag_core::math::Vec3::from_array(sample.lateral).normalize_or_zero();
            let base = oag_core::math::Vec3::from_array(sample.pos) + up * 1.5;
            for (side_index, sign) in [(0usize, -1.0f32), (1usize, 1.0f32)] {
                let half = if sign < 0.0 {
                    sample.half_width_left
                } else {
                    sample.half_width_right
                }
                .max(1.0);
                let origin = base;
                let hit =
                    collision.raycast(Ray::new(origin, lateral * sign, half * 3.0), None, false);
                let wall_at = hit
                    .filter(|h| h.surface == Surface::Wall)
                    .map(|h| h.distance / half);
                let class = match wall_at {
                    Some(d) if d <= 1.2 => 0,
                    Some(_) => 1,
                    None => {
                        let floor_at = |k: f32| {
                            let from = oag_core::math::Vec3::from_array(sample.pos)
                                + lateral * sign * half * k
                                + up * 6.0;
                            collision
                                .raycast(Ray::new(from, -up, 40.0), None, false)
                                .is_some()
                        };
                        if floor_at(1.15) || floor_at(1.6) {
                            2
                        } else {
                            3
                        }
                    }
                };
                counts[side_index][class] += 1;
                if class == 3 {
                    void_runs[side_index].push((index, sample.section_id, half));
                }
            }
        }
        let pct = |n: u32| 100.0 * n as f32 / sampled.max(1) as f32;
        println!(
            "EDGESUM|{label}|samples {sampled}|L walled {:.0}% farwall {:.0}% shoulder {:.0}% void {:.0}%|\
             R walled {:.0}% farwall {:.0}% shoulder {:.0}% void {:.0}%",
            pct(counts[0][0]),
            pct(counts[0][1]),
            pct(counts[0][2]),
            pct(counts[0][3]),
            pct(counts[1][0]),
            pct(counts[1][1]),
            pct(counts[1][2]),
            pct(counts[1][3]),
        );
        for (side_index, runs) in void_runs.iter().enumerate() {
            let mut start: Option<usize> = None;
            let mut prev = 0usize;
            let mut sec = 0u8;
            let flush = |from: usize, to: usize, sec: u8| {
                if to - from >= 12 {
                    println!(
                        "EDGE|{label}|{}|void samples {from}..{to}|sec {sec}",
                        if side_index == 0 { "L" } else { "R" }
                    );
                }
            };
            for &(index, section, _) in runs {
                match start {
                    Some(_) if index == prev + 4 => {}
                    Some(from) => {
                        flush(from, prev, sec);
                        start = Some(index);
                        sec = section;
                    }
                    None => {
                        start = Some(index);
                        sec = section;
                    }
                }
                prev = index;
            }
            if let Some(from) = start {
                flush(from, prev, sec);
            }
        }
    }
}

/// Prints every `PI_Track` line of the plugin definition, for the
/// `collisionCageEnabled` attribute the collision docs mention.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn track_definition_attributes() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let text = format!("{definition:?}");
    for (at, _) in text.match_indices("collisionCageEnabled") {
        let from = at.saturating_sub(160);
        println!("CAGE|{}", &text[from..(at + 40).min(text.len())]);
    }
}

/// One shove trial, replayed with per-tick output, plus the pose it starts from
/// in the form `scripts/psp-drive.py place` takes, so the same trial can be
/// run in the original.
///
/// `OAG_SHOVE_TRIAL=<circuit id, r suffix for reversed>:<spline index>:<L|R>:<angle>:<speed>`.
/// Prints `PLACE|pos|tangent|up|speed` and then one `TRACE|tick|x,y,z|speed|lat`
/// line per tick of the same 200-tick run `shove_sweep` does.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1 and OAG_SHOVE_TRIAL"]
fn shove_trial_trace() {
    let Ok(spec) = std::env::var("OAG_SHOVE_TRIAL") else {
        return;
    };
    let parts: Vec<&str> = spec.split(':').collect();
    let (wanted, index, side, angle, speed): (&str, usize, f32, f32, f32) = (
        parts[0],
        parts[1].parse().expect("index"),
        if parts[2] == "L" { -1.0 } else { 1.0 },
        parts[3].parse().expect("angle"),
        parts[4].parse().expect("speed"),
    );
    let Some(image) = image() else { return };
    let circuit = circuits_all()
        .into_iter()
        .find(|c| format!("{}{}", c.id, if c.reversed { "r" } else { "" }) == wanted)
        .expect("no such circuit");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(circuit.entry.clone()),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    for other in 1..8 {
        race.sim.world.ships[other].active = false;
    }
    let throttle = throttle_held();
    for _ in 0..600 {
        race.tick(&PlayerInputs::single(throttle));
    }
    let height = oag_gameplay::spawn::spawn_height(&race.sim.world.ships[0].handling);
    let sample = race.spline().sample(index).copied().expect("sample");
    let mut pose = oag_gameplay::Pose::from_sample(&sample, sample.racing_line, height);
    let up = pose.orientation * oag_core::math::Vec3::Y;
    pose.orientation =
        oag_core::math::Quat::from_axis_angle(up, side * angle.to_radians()) * pose.orientation;
    // `OAG_ROLL=<degrees>` rolls the craft about its own forward axis, so a
    // craft lying on its flank can be placed in both engines.
    if let Some(roll) = std::env::var("OAG_ROLL")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        let forward = pose.orientation * oag_core::math::Vec3::NEG_Z;
        pose.orientation =
            oag_core::math::Quat::from_axis_angle(forward, roll.to_radians()) * pose.orientation;
    }
    if let Some(sink) = std::env::var("OAG_SINK")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        pose.position -= up * sink;
    }
    let forward = pose.orientation * oag_core::math::Vec3::NEG_Z;
    let up = pose.orientation * oag_core::math::Vec3::Y;
    println!(
        "PLACE|{:.4},{:.4},{:.4}|{:.5},{:.5},{:.5}|{:.5},{:.5},{:.5}|{speed}",
        pose.position.x,
        pose.position.y,
        pose.position.z,
        forward.x,
        forward.y,
        forward.z,
        up.x,
        up.y,
        up.z
    );
    {
        use oag_physics::{Ray, Raycaster};
        let right = pose.orientation * oag_core::math::Vec3::X;
        for (name, direction) in [
            ("down", -up),
            ("up", up),
            ("fwd", forward),
            ("back", -forward),
            ("right", right),
            ("left", -right),
        ] {
            for include_reset in [false, true] {
                match race.collision().raycast(
                    Ray::new(pose.position, direction, 120.0),
                    None,
                    include_reset,
                ) {
                    Some(h) => println!(
                        "SURF|{name}|reset {include_reset}|{:?}|d {:.2}|n {:.2},{:.2},{:.2}",
                        h.surface, h.distance, h.normal.x, h.normal.y, h.normal.z
                    ),
                    None => println!("SURF|{name}|reset {include_reset}|nothing within 120"),
                }
            }
        }
    }
    race.sim.world.ships[0].place_at(pose);
    race.sim.world.ships[0].physics.body.linear_velocity = forward * speed;
    // `OAG_VEL=x,y,z` overrides the initial velocity outright, to seed a state
    // read off the original's first recorded tick.
    if let Ok(v) = std::env::var("OAG_VEL") {
        let v: Vec<f32> = v.split(',').map(|c| c.parse().expect("OAG_VEL")).collect();
        race.sim.world.ships[0].physics.body.linear_velocity =
            oag_core::math::Vec3::new(v[0], v[1], v[2]);
    }
    // `OAG_COAST=1` releases the throttle, for comparison with a placement in
    // the original that presses nothing.
    let input = if std::env::var_os("OAG_COAST").is_some() {
        oag_gameplay::InputSnapshot::default()
    } else {
        throttle
    };
    for tick in 0..200 {
        let ship = &race.sim.world.ships[0];
        let position = ship.physics.body.position;
        let lat = race.spline().nearest(position).map_or(0.0, |(_, s, _)| {
            let lat = (position - oag_core::math::Vec3::from_array(s.pos))
                .dot(oag_core::math::Vec3::from_array(s.lateral));
            let half = if lat >= 0.0 {
                s.half_width_right
            } else {
                s.half_width_left
            };
            lat / half.max(1e-3)
        });
        println!(
            "TRACE|{tick}|{:.3},{:.3},{:.3}|{:.1}|{lat:+.2}|air {:.2}|up {:.2},{:.2},{:.2}",
            position.x,
            position.y,
            position.z,
            ship.physics.body.linear_velocity.length(),
            ship.physics.time_airborne,
            ship.physics.body.up().x,
            ship.physics.body.up().y,
            ship.physics.body.up().z,
        );
        race.tick(&PlayerInputs::single(input));
    }
}

/// A lone craft's per-tick pose over a window of spline indices, first pass only.
///
/// `OAG_WINDOW=<circuit id>:<slot>:<from index>:<to index>:<max tick>`.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1 and OAG_WINDOW"]
fn window_trace() {
    let Ok(spec) = std::env::var("OAG_WINDOW") else {
        return;
    };
    let parts: Vec<&str> = spec.split(':').collect();
    let wanted = parts[0];
    let slot: usize = parts[1].parse().expect("slot");
    let from: usize = parts[2].parse().expect("from");
    let to: usize = parts[3].parse().expect("to");
    let max_tick: u64 = parts[4].parse().expect("max tick");
    let Some(image) = image() else { return };
    let circuit = circuits_all()
        .into_iter()
        .find(|c| format!("{}{}", c.id, if c.reversed { "r" } else { "" }) == wanted)
        .expect("no such circuit");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(circuit.entry.clone()),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    for other in 0..8 {
        race.sim.world.ships[other].active = other == slot;
    }
    for tick in 0..max_tick {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[slot];
        let body = &ship.physics.body;
        let Some((index, _, _)) = race.spline().nearest(body.position) else {
            continue;
        };
        if index < from || index > to {
            continue;
        }
        let up = body.up();
        let v = body.linear_velocity;
        let below = {
            use oag_physics::{Ray, Raycaster};
            race.collision()
                .raycast(
                    Ray::new(body.position, oag_core::math::Vec3::NEG_Y, 40.0),
                    None,
                    true,
                )
                .map_or("none".to_string(), |h| {
                    format!("{:?} {:.2}", h.surface, h.distance)
                })
        };
        println!(
            "WIN|{tick}|idx {index}|below {below}|pos {:.2},{:.2},{:.2}|v {:.1},{:.1},{:.1}|up {:.2},{:.2},{:.2}|g {:.1}|air {:.2}|shield {:.1}|walls {}",
            body.position.x,
            body.position.y,
            body.position.z,
            v.x,
            v.y,
            v.z,
            up.x,
            up.y,
            up.z,
            ship.physics.grounded,
            ship.physics.time_airborne,
            ship.physics.shield,
            race.wall_contact_ticks_of(slot),
        );
    }
}
