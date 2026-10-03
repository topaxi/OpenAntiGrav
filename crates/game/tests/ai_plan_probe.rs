//! Scratch probe for a speed plan that does not verify: drives a layout at one
//! fixed speed with the plan's own steering and prints a row per tick (line
//! index, offset from the line, corridor bounds, steering, wall contact).
//!
//! `#[ignore]`d and gated on `OAG_SWEEP`, printing rather than asserting.
//! `OAG_TRACK` (default `05_Track`), `OAG_REVERSED`, `OAG_SWEEP_CLASS`
//! (default VENOM), `OAG_SPEED` (default 15), `OAG_TICKS` (default 12000).

use oag_game::{catalogue, race};

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry_of(folder: &str, reversed: bool) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| {
            track.id == folder || (track.location.ends_with(folder) && track.reversed == reversed)
        })
        .map(|track| track.entry_name())
}

fn var<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn plan_probe() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let folder = std::env::var("OAG_TRACK").unwrap_or_else(|_| "05_Track".into());
    let reversed = std::env::var_os("OAG_REVERSED").is_some();
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".into());
    let entry = entry_of(&folder, reversed).expect("layout");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.clone(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading");
    let race = race::Race::start(loaded.setup);
    let rows = race.probe_speed_plan(1, var("OAG_SPEED", 15.0), var("OAG_TICKS", 12_000));
    if std::env::var_os("OAG_UP").is_some() {
        use oag_core::math::Vec3;
        use oag_physics::{Ray, Raycaster};
        let line = race.racing_line();
        for r in rows.iter().filter(|r| r.height < -15.0).step_by(40) {
            let up = race
                .collision()
                .raycast(Ray::new(r.position, Vec3::Y, 200.0), None, false)
                .map(|h| (h.distance, h.surface));
            let nearest = (0..line.len())
                .min_by(|&a, &b| {
                    line.point(a)
                        .distance(r.position)
                        .total_cmp(&line.point(b).distance(r.position))
                })
                .unwrap();
            println!(
                "UP tick {} idx {} h {:.1} up {:?} nearest {} at {:.1}",
                r.tick,
                r.index,
                r.height,
                up,
                nearest,
                line.point(nearest).distance(r.position)
            );
        }
        return;
    }
    println!(
        "tick\tindex\tspeed\toffset\theight\tleft\tright\tsteer\tcontact\tair\tfailure\tclimb\tpitch"
    );
    for r in rows {
        println!(
            "{}\t{}\t{:.1}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.3}\t{}\t{}\t{:?}\t{:.2}\t{:.3}",
            r.tick,
            r.index,
            r.speed,
            r.offset,
            r.height,
            r.left,
            r.right,
            r.steer,
            u8::from(r.contact),
            u8::from(r.airborne),
            r.failure,
            r.climb,
            r.pitch
        );
    }
}

/// The line against the collision under it: per sample, the first surface hit
/// casting straight down from 2 units above the line point, and every hit
/// within 150 units below.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn line_surface() {
    use oag_core::math::Vec3;
    use oag_physics::{Ray, Raycaster};
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let folder = std::env::var("OAG_TRACK").unwrap_or_else(|_| "05_Track".into());
    let reversed = std::env::var_os("OAG_REVERSED").is_some();
    let entry = entry_of(&folder, reversed).expect("layout");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".into(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading");
    let race = race::Race::start(loaded.setup);
    let line = race.racing_line();
    let world = race.collision();
    println!("index\tx\ty\tz\tunsupported\ttakeoff\tleft\tright\thits_below");
    for i in 0..line.len() {
        let p = line.point(i);
        let mut out = [None; 8];
        let ray = Ray::new(p + Vec3::Y * 2.0, -Vec3::Y, 150.0);
        let count = world.raycast_all(ray, None, false, &mut out);
        let hits: Vec<String> = out[..count]
            .iter()
            .flatten()
            .map(|h| format!("{:.1}:{:?}", h.point.y - p.y, h.surface))
            .collect();
        let fr = line.aim(i, 0.0).corridor;
        println!(
            "{i}\t{:.1}\t{:.1}\t{:.1}\t{}\t{}\t{:.2}\t{:.2}\t{}",
            p.x,
            p.y,
            p.z,
            u8::from(line.is_unsupported(i)),
            u8::from(line.is_takeoff(i)),
            fr.map_or(f32::NAN, |f| f.left),
            fr.map_or(f32::NAN, |f| f.right),
            hits.join(" ")
        );
    }
}

/// The race's own lone craft through a stretch of line: per tick, the same
/// columns `plan_probe` prints, for comparison. `OAG_FROM`/`OAG_TO` bound the
/// line index (default 130-240), `OAG_TICKS` the run.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn race_probe() {
    use oag_core::math::Vec3;
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let folder = std::env::var("OAG_TRACK").unwrap_or_else(|_| "05_Track".into());
    let reversed = std::env::var_os("OAG_REVERSED").is_some();
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".into());
    let entry = entry_of(&folder, reversed).expect("layout");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class,
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    let slot: usize = var("OAG_LONE_SLOT", 1);
    for s in 0..race.ship_count() as usize {
        if s != slot {
            race.sim.world.ships[s].active = false;
        }
    }
    let from: u32 = var("OAG_FROM", 130);
    let to: u32 = var("OAG_TO", 240);
    let line = race.racing_line().clone();
    println!("tick\tindex\tspeed\toffset\theight\tthrust\tbrake\tgrounded\tclimb\tpitch\tsteer");
    let mut respawns = 0;
    let mut last = (0u32, Vec3::ZERO, 0.0f32, 0.0f32);
    for tick in 0..var("OAG_TICKS", 6000u32) {
        race.tick(&oag_gameplay::PlayerInputs::none());
        let sh = &race.sim.world.ships[slot];
        if race.respawns_of(slot) != respawns {
            respawns = race.respawns_of(slot);
            println!(
                "RESPAWN tick {tick} prev idx {} pos {:?} air {:.2} speed {:.1} now idx {}",
                last.0, last.1, last.2, last.3, sh.driver.index
            );
        }
        last = (
            sh.driver.index,
            sh.physics.body.position,
            sh.physics.time_airborne,
            sh.physics.body.linear_velocity.length(),
        );
        let i = sh.driver.index;
        if i < from || i > to {
            continue;
        }
        let p = line.point(i as usize);
        let lat = line
            .aim(i as usize, 0.0)
            .corridor
            .map_or(Vec3::ZERO, |f| f.lateral);
        let b = &sh.physics.body;
        println!(
            "{tick}\t{i}\t{:.1}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{}\t{:.2}\t{:.3}\t{:.3}",
            b.linear_velocity.dot(b.forward()),
            (b.position - p).dot(lat),
            b.position.y - p.y,
            sh.physics.thrust,
            sh.physics.brake,
            u8::from(sh.physics.time_airborne <= 0.0),
            b.linear_velocity.y,
            b.forward().y,
            sh.physics.steer,
        );
    }
}

/// Every speed pad against the line: nearest sample, offset across it, and
/// the corridor there.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn pad_map() {
    use oag_core::math::Vec3;
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let folder = std::env::var("OAG_TRACK").unwrap_or_else(|_| "05_Track".into());
    let reversed = std::env::var_os("OAG_REVERSED").is_some();
    let entry = entry_of(&folder, reversed).expect("layout");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".into(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading");
    let race = race::Race::start(loaded.setup);
    let line = race.racing_line();
    for pad in race.speedup_pads() {
        let c = Vec3::from_array(pad.centre());
        let i = (0..line.len())
            .min_by(|&a, &b| {
                line.point(a)
                    .distance(c)
                    .total_cmp(&line.point(b).distance(c))
            })
            .unwrap();
        let fr = line.aim(i, 0.0).corridor;
        let off = fr.map_or(f32::NAN, |f| (c - line.point(i)).dot(f.lateral));
        println!(
            "pad idx {i} offset {off:.1} dist {:.1} corridor {:?}..{:?} dir {:?}",
            line.point(i).distance(c),
            fr.map(|f| f.left),
            fr.map(|f| f.right),
            pad.direction()
        );
    }
}

/// Floor height (relative to the line point) across the corridor, per sample
/// in `OAG_FROM..OAG_TO`, at offsets -12..=12 in steps of 2.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn floor_grid() {
    use oag_core::math::Vec3;
    use oag_physics::{Ray, Raycaster};
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let folder = std::env::var("OAG_TRACK").unwrap_or_else(|_| "05_Track".into());
    let reversed = std::env::var_os("OAG_REVERSED").is_some();
    let entry = entry_of(&folder, reversed).expect("layout");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".into(),
        mode: oag_race::Mode::SingleRace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading");
    let race = race::Race::start(loaded.setup);
    let line = race.racing_line();
    let from: usize = var("OAG_FROM", 140);
    let to: usize = var("OAG_TO", 240);
    let step: usize = var("OAG_STEP", 4);
    for i in (from..to).step_by(step) {
        let p = line.point(i);
        let lat = line.aim(i, 0.0).corridor.map_or(Vec3::X, |f| f.lateral);
        let mut row = format!("{i:4}");
        for o in (-12..=12).step_by(2) {
            let q = p + lat * o as f32;
            let mut out = [None; 8];
            let count = race.collision().raycast_all(
                Ray::new(q + Vec3::Y * 20.0, -Vec3::Y, 60.0),
                None,
                false,
                &mut out,
            );
            let mut tags = String::new();
            let mut top = f32::NEG_INFINITY;
            for h in out[..count].iter().flatten() {
                top = top.max(h.point.y - p.y);
                tags.push(if h.surface.is_hoverable() { 'f' } else { 'w' });
            }
            row.push_str(&format!(" {top:>6.1}{tags:<3}"));
        }
        println!("{row}");
    }
}

/// The full field: every craft's takeoff at the first jump (forward speed,
/// offset, climb, whether it was in traffic) and whether it landed on the
/// upper road, by the craft's height against the line 30 samples later.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn field_takeoffs() {
    use oag_core::math::Vec3;
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".into());
    let entry = entry_of("05_Track", false).expect("layout");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class,
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        seed: Some(var("OAG_SEED", 1)),
        weapons_override: std::env::var_os("OAG_WEAPONS_OFF").map(|_| false),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].active = false;
    let count = race.ship_count() as usize;
    let line = race.racing_line().clone();
    let lip: u32 = var("OAG_LIP", 152);
    let mut prev = vec![0u32; count];
    let mut pending: Vec<Option<(u64, f32, f32, f32)>> = vec![None; count];
    for tick in 0..var("OAG_TICKS", 18_000u64) {
        race.tick(&oag_gameplay::PlayerInputs::none());
        for s in 1..count {
            let sh = &race.sim.world.ships[s];
            if !sh.active {
                continue;
            }
            let i = sh.driver.index;
            let b = &sh.physics.body;
            let p = line.point(i as usize);
            let lat = line
                .aim(i as usize, 0.0)
                .corridor
                .map_or(Vec3::ZERO, |f| f.lateral);
            for m in [2700u32, 2780, 2830, 2860, 2920, 20, 80, 130] {
                if prev[s] < m
                    && i >= m
                    && i < m + 20
                    && std::env::var_os("OAG_MILESTONES").is_some()
                {
                    println!(
                        "MILE slot {s} tick {tick} idx {m} v {:.1} off {:.1} thrust {:.0} brakes {:.0}/{:.0} pad {:.2}",
                        b.linear_velocity.dot(b.forward()),
                        (b.position - p).dot(lat),
                        sh.physics.thrust,
                        sh.physics.airbrake_left,
                        sh.physics.airbrake_right,
                        sh.physics.pad_timer
                    );
                }
            }
            if prev[s] < lip && i >= lip && i < lip + 20 {
                pending[s] = Some((
                    tick,
                    b.linear_velocity.dot(b.forward()),
                    (b.position - p).dot(lat),
                    b.linear_velocity.y,
                ));
            }
            if let Some((t, v, o, c)) = pending[s]
                && i >= lip + 80
            {
                println!(
                    "TAKEOFF slot {s} tick {t} v {v:.1} off {o:.1} climb {c:.1} -> h {:.1} at idx {i}",
                    b.position.y - p.y
                );
                pending[s] = None;
            }
            prev[s] = i;
        }
    }
}

/// Every layout, both directions: the longest run of ticks any craft spent on
/// the ground more than `OAG_BENEATH` (default 15) below its own line, where,
/// and every respawn by cause. `OAG_SWEEP_CLASS` (default VENOM), field of
/// seven Aces unless `OAG_LONE`.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP"]
fn beneath_census() {
    use oag_core::math::Vec3;
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "VENOM".into());
    let lone = std::env::var_os("OAG_LONE").is_some();
    let threshold: f32 = var("OAG_BENEATH", 15.0);
    let mut archives = oag_pulse::open(&image.display().to_string()).unwrap();
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .unwrap();
    let definition = oag_tables::fexml::expand(&blob).unwrap();
    let only = std::env::var("OAG_ONLY").ok();
    for track in catalogue::tracks(&definition) {
        if only.as_ref().is_some_and(|o| *o != track.id) {
            continue;
        }
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: class.clone(),
            mode: oag_race::Mode::SingleRace,
            difficulty: oag_ai::Difficulty::Ace,
            track: Some(track.entry_name()),
            seed: Some(1),
            ..race::Options::default()
        })
        .unwrap();
        let mut race = race::Race::start(loaded.setup);
        let count = race.ship_count() as usize;
        for s in 0..count {
            if s == 0 || (lone && s != 1) {
                race.sim.world.ships[s].active = false;
            }
        }
        let line = race.racing_line().clone();
        let mut run = vec![0u32; count];
        let mut longest = (0u32, 0usize, 0u32);
        let mut causes: Vec<String> = Vec::new();
        let mut respawns = vec![0u32; count];
        for _ in 0..var("OAG_TICKS", 18_000u32) {
            race.tick(&oag_gameplay::PlayerInputs::none());
            for s in 1..count {
                let sh = &race.sim.world.ships[s];
                if !sh.active {
                    continue;
                }
                let i = sh.driver.index;
                let beneath = sh.physics.time_airborne <= 0.0
                    && !line.is_unsupported(i as usize)
                    && race.ai_sample(i as usize).is_some_and(|sample| {
                        let up = -Vec3::from_array(sample.down).normalize_or_zero();
                        (line.point(i as usize) - sh.physics.body.position).dot(up) > threshold
                    });
                run[s] = if beneath { run[s] + 1 } else { 0 };
                if run[s] % 100 == 99 {
                    println!(
                        "  BENEATH {} slot {s} idx {i} run {} state {:?} shield {:.1} v {:.1} lap {}",
                        track.id,
                        run[s],
                        sh.physics.craft_state,
                        sh.physics.shield,
                        sh.physics.body.linear_velocity.length(),
                        sh.standing.lap
                    );
                }
                if run[s] > longest.0 {
                    longest = (run[s], s, i);
                }
                if race.respawns_of(s) != respawns[s] {
                    respawns[s] = race.respawns_of(s);
                    causes.push(format!("{:?}@{i}", race.last_respawn_cause_of(s)));
                }
            }
        }
        println!(
            "CENSUS {} {} {class} longest-beneath {} ticks (slot {} at idx {}) respawns {:?}",
            track.id,
            if track.reversed { "rev" } else { "fwd" },
            longest.0,
            longest.1,
            longest.2,
            causes
        );
    }
}
