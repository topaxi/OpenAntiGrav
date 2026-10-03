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
        .find(|track| track.location.ends_with(folder) && track.reversed == reversed)
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
    for tick in 0..var("OAG_TICKS", 6000u32) {
        race.tick(&oag_gameplay::PlayerInputs::none());
        let sh = &race.sim.world.ships[slot];
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
