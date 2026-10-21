//! Scratch sweep: where along the line a lone Ace's wall contact lands, over
//! the clean-lap gate's perturbed runs. `#[ignore]`d and gated on `OAG_SWEEP`,
//! printing rather than asserting. `OAG_TRACK` (default `06_Track`),
//! `OAG_SWEEP_CLASS` (default RAPIER), `OAG_BIN` (default 10).

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

use crate::in_parallel;
use crate::perturb;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry_of(folder: &str) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.id == folder && !track.reversed)
        .map(|track| track.entry_name())
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP"]
fn contact_where() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let folder = std::env::var("OAG_TRACK").unwrap_or_else(|_| "06_Track".into());
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "RAPIER".into());
    let bin: u32 = std::env::var("OAG_BIN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let entry = entry_of(&folder).expect("layout");
    let options = race::Options {
        source: image.display().to_string(),
        class: class.clone(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        ..race::Options::default()
    };
    let jobs: Vec<(f32, f32)> = perturb::BOUND_OFFSETS
        .iter()
        .flat_map(|&o| perturb::GATE.iter().map(move |&e| (o, e)))
        .collect();
    let rows = in_parallel::map(&jobs, 16, |&(offset, eps)| {
        let mut setup = race::load(&options).expect("loading").setup;
        perturb::apply_at(&mut setup, eps, offset);
        let mut race = race::Race::start(setup);
        for slot in 2..8 {
            race.sim.world.ships[slot].active = false;
        }
        race.sim.world.ships[0].active = false;
        let yaw = oag_ai::hull_yaw_ceiling(
            &race.sim.world.ships[1].handling,
            &race.sim.world.ships[1].physics.body,
        );
        let reference_ptr = race.speed_plan().map(|p| p as *const _ as usize);
        let planned = race.plan_of(1).map_or("none".to_string(), |p| {
            format!(
                "{} share {:.3}",
                if Some(p as *const _ as usize) == reference_ptr {
                    "REF"
                } else {
                    "own"
                },
                p.share_for(Some(yaw))
            )
        });
        let mut bins: std::collections::BTreeMap<u32, (u32, f32, usize, usize)> =
            Default::default();
        let mut lap = race.sim.world.ships[1].standing.lap;
        let mut started = 0u64;
        let mut laps = Vec::new();
        let mut lap_shield = Vec::new();
        let start_shield = race.sim.world.ships[1].physics.shield;
        for tick in 0..18_000u64 {
            let before = race.wall_contact_ticks_of(1);
            race.tick(&PlayerInputs::none());
            let ship = &race.sim.world.ships[1];
            if race.wall_contact_ticks_of(1) != before {
                let speed = ship.physics.body.linear_velocity.length();
                let position = ship.physics.body.position;
                let lap_now = ship.standing.lap;
                let index = ship.driver.index;
                let spline = race.spline().nearest(position).map_or(0, |(i, _, _)| i);
                let e = bins
                    .entry(index / bin * bin + 100_000 * u32::from(lap_now > 1))
                    .or_insert((0, 0.0, usize::MAX, 0));
                e.0 += 1;
                e.1 = e.1.max(speed);
                e.2 = e.2.min(spline);
                e.3 = e.3.max(spline);
            }
            if ship.standing.lap != lap {
                lap_shield.push(ship.physics.shield);
                laps.push(tick - started);
                started = tick;
                lap = ship.standing.lap;
            }
        }
        let total = race.wall_contact_ticks_of(1);
        let end_shield = race.sim.world.ships[1].physics.shield;
        // Shield lost per completed lap, from the pool at each lap line.
        let mut pool = Vec::with_capacity(lap_shield.len() + 1);
        pool.push(start_shield);
        pool.extend(lap_shield.iter().copied());
        let lost: Vec<String> = pool
            .windows(2)
            .map(|w| format!("{:.1}", w[0] - w[1]))
            .collect();
        let text: Vec<String> = bins
            .iter()
            .map(|(k, (n, v, a, b))| format!("{k}:{n}@{v:.0}[{a}-{b}]"))
            .collect();
        format!(
            "off {offset:>8} eps {eps:>8} plan {planned} contact {total:>4} laps {laps:?} \
             shield lost per lap [{}] end {end_shield:.1}\n    {}",
            lost.join(", "),
            text.join(" ")
        )
    });
    for row in rows {
        println!("{row}");
    }
}

/// Per-tick rows over a line-index window for one `(offset, eps)` run:
/// `OAG_TRACE=offset,eps`, `OAG_RANGE=lo,hi` (default 700,980).
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP"]
fn contact_trace() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let pair = |name: &str, default: (f32, f32)| -> (f32, f32) {
        std::env::var(name)
            .ok()
            .and_then(|v| {
                let mut it = v.split(',').map(|x| x.trim().parse::<f32>().ok());
                Some((it.next()??, it.next()??))
            })
            .unwrap_or(default)
    };
    let (offset, eps) = pair("OAG_TRACE", (0.0, 0.0));
    let (lo, hi) = pair("OAG_RANGE", (700.0, 980.0));
    let folder = std::env::var("OAG_TRACK").unwrap_or_else(|_| "06_Track".into());
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| "RAPIER".into());
    let entry = entry_of(&folder).expect("layout");
    let mut setup = race::load(&race::Options {
        source: image.display().to_string(),
        class,
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        ..race::Options::default()
    })
    .expect("loading")
    .setup;
    perturb::apply_at(&mut setup, eps, offset);
    let mut race = race::Race::start(setup);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;
    let targets: Vec<(f32, f32, f32)> = {
        let Some(plan) = race.plan_of(1) else {
            println!("no plan: the corner model drives");
            return;
        };
        (0..plan.len())
            .map(|i| (plan.target(i), plan.ceiling(i), plan.pace(i)))
            .collect()
    };
    println!("tick lap idx speed target ceil pace lat steer thrust brake abL abR contact");
    for tick in 0..18_000u64 {
        let before = race.wall_contact_ticks_of(1);
        race.tick(&PlayerInputs::none());
        let contact = race.wall_contact_ticks_of(1) != before;
        let ship = &race.sim.world.ships[1];
        let index = ship.driver.index as usize;
        if (index as f32) < lo || (index as f32) > hi {
            continue;
        }
        let line = race.line_of(1);
        let p = ship.physics.body.position;
        let lat = line
            .corridor_at(index)
            .map_or(f32::NAN, |f| (p - line.point(index)).dot(f.lateral));
        let s = &ship.physics;
        let (t, c, pace) = targets
            .get(index)
            .copied()
            .unwrap_or((f32::NAN, f32::NAN, f32::NAN));
        println!(
            "{tick} {} {index} {:.1} {t:.0} {c:.0} {pace:.0} {lat:.1} {:.2} {:.2} {:.2} {:.2} {:.2} {}",
            ship.standing.lap,
            s.body.linear_velocity.length(),
            s.steer,
            s.thrust,
            s.brake,
            s.airbrake_left,
            s.airbrake_right,
            u8::from(contact)
        );
    }
}
