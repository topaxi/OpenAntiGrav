//! Scratch: what a Rocket, Missile, Shuriken or Plasma fired on a steep bank does, tick by tick.
//!
//! `#[ignore]`d and gated on an env var, never run in CI or in `just test-data`, the same shape
//! as `rocket_floor_trace.rs`. The bank is the run of spline samples whose up is below
//! `OAG_LOOP_TRACE_UP_Y` (default 0.5) that holds the circuit's steepest sample.
//!
//! - `trace_a_projectile_onto_a_bank` poses the player's craft on the racing line
//!   `OAG_LOOP_TRACE_LEAD` samples (default 40) before the bank (or at sample
//!   `OAG_LOOP_TRACE_FIRST`), at `OAG_LOOP_TRACE_SPEED` units/s (default 140), holds it on that
//!   pose until the shot is in the air, and logs every tick of the projectile: position,
//!   velocity, carried normal, the nearest spline sample's lateral offset, half-widths and up,
//!   and what 6- and 12-unit probes along the carried normal find.
//! - `census_of_shots_along_a_bank` flies the player on the autopilot and fires every
//!   `OAG_LOOP_TRACE_EVERY` ticks (default 20) while the craft is on the bank, and tallies how
//!   each shot ended.
//!
//! ```sh
//! OAG_LOOP_TRACE=hd:/data/environments/03_track/track.vex:plasma \
//!   OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!   -E 'test(/^loop_floor_trace::/)' --no-capture
//! ```

use std::fmt::Write as _;

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_physics::{Ray, Raycaster};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;
use oag_weapons::projectile::{Impact, MAX_PROJECTILES, Projectile};

fn env<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn snapshot(bits: u32, previous: Option<&Input>) -> oag_gameplay::InputSnapshot {
    let mut buttons = previous.copied().unwrap_or_else(Input::new);
    buttons.begin_frame(bits);
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

fn probe<R: Raycaster + ?Sized>(world: &R, at: Vec3, down: Vec3, length: f32) -> String {
    match world.raycast(Ray::new(at, down, length), None, false) {
        Some(hit) => format!(
            "d={:.2} n=({:.2},{:.2},{:.2}) {:?}",
            hit.distance, hit.normal.x, hit.normal.y, hit.normal.z, hit.surface
        ),
        None => "MISS".to_string(),
    }
}

struct Setup {
    spec: String,
    weapon: Weapon,
    race: race::Race,
    /// The bank: first and last sample.
    bank: (usize, usize),
    log: String,
}

fn setup() -> Option<Setup> {
    let spec = std::env::var("OAG_LOOP_TRACE").ok()?;
    let mut parts = spec.splitn(3, ':');
    let title = parts.next().expect("title");
    let track = parts.next().expect("track").to_string();
    let weapon = match parts.next().expect("weapon") {
        "rocket" => Weapon::Rocket,
        "missile" => Weapon::Missile,
        "shuriken" => Weapon::Shuriken,
        "plasma" => Weapon::Plasma,
        other => panic!("unknown weapon {other}"),
    };
    let image = match title {
        "hd" => oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso"),
        "pulse" => oag_testdata::image("data/images/pulse-psp-eu.chd"),
        _ => None,
    }?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    let mut log = String::new();
    writeln!(
        log,
        "# {spec} floor_arms={:?} release={:?} cage={}",
        race.core.sim.floor_arms(),
        race.core.sim.plasma_release(),
        race.core.sim.spline().cage().is_some()
    )
    .unwrap();

    let spline = race.core.sim.spline();
    let up_of = |i: usize| -Vec3::from_array(spline.sample(i).unwrap().down).normalize();
    let below: f32 = env("OAG_LOOP_TRACE_UP_Y", 0.5);
    let lowest = (0..spline.len())
        .min_by(|&a, &b| up_of(a).y.total_cmp(&up_of(b).y))
        .unwrap();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for i in (0..spline.len()).filter(|&i| up_of(i).y < below) {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == i => *end = i,
            _ => runs.push((i, i)),
        }
    }
    for &(a, b) in &runs {
        let low = (a..=b)
            .min_by(|&x, &y| up_of(x).y.total_cmp(&up_of(y).y))
            .unwrap();
        writeln!(
            log,
            "# run {a}..={b} (up.y < {below}): lowest up.y {:.3} at {low}",
            up_of(low).y
        )
        .unwrap();
    }
    let bank = runs
        .iter()
        .copied()
        .find(|&(a, b)| (a..=b).contains(&lowest))
        .unwrap_or((lowest, lowest));
    for i in (bank.0..=bank.1).step_by(25) {
        let s = spline.sample(i).unwrap();
        let up = up_of(i);
        writeln!(
            log,
            "# sample {i} pos=({:.1},{:.1},{:.1}) up=({:.2},{:.2},{:.2}) hw=({:.1},{:.1})",
            s.pos[0], s.pos[1], s.pos[2], up.x, up.y, up.z, s.half_width_left, s.half_width_right
        )
        .unwrap();
    }
    Some(Setup {
        spec,
        weapon,
        race,
        bank,
        log,
    })
}

/// Where a point sits against the nearest spline sample: index, lateral offset, height along the
/// sample's up, the half-widths, the up.
fn locate(race: &race::Race, at: Vec3) -> (usize, f32, f32, f32, f32, Vec3) {
    let spline = race.core.sim.spline();
    let Some((i, s, _)) = spline.nearest(at) else {
        return (0, 0.0, 0.0, 0.0, 0.0, Vec3::ZERO);
    };
    let up = -Vec3::from_array(s.down).normalize();
    let lat = Vec3::from_array(s.lateral).normalize_or_zero();
    let along = at - Vec3::from_array(s.pos);
    (
        i,
        along.dot(lat),
        along.dot(up),
        s.half_width_left,
        s.half_width_right,
        up,
    )
}

/// What the pool's own step does this tick, on a copy: the impacts.
fn predict(race: &race::Race) -> [Option<Impact>; MAX_PROJECTILES] {
    let mut world = race.core.sim.world.clone();
    let count = world.ship_count as usize;
    let mut hits = vec![oag_weapons::projectile::WeaponHit::default(); world.ships.len()];
    let cage = race
        .core
        .sim
        .spline()
        .cage()
        .map(|c| c as &dyn oag_physics::cage::Containment);
    oag_weapons::projectile::step(
        &mut world.projectiles,
        &mut world.ships[..count],
        race.core.sim.dt(),
        race.core.sim.collision(),
        cage,
        race.core.sim.weapon_stats(),
        "VENOM",
        race.core.sim.flight_laws(),
        race.core.sim.own_craft_probe(),
        race.core.sim.plasma_release(),
        oag_weapons::projectile::BlastPush::Posted,
        oag_physics::DamageRules::default(),
        &mut hits,
    )
}

fn describe(race: &race::Race, tick: u32, slot: usize, p: &Projectile) -> String {
    let (ni, lat, height, hwl, hwr, nup) = locate(race, p.position);
    let world = race.core.sim.collision();
    format!(
        "t={tick} s{slot} chg={:.2} life={:.2} pos=({:.1},{:.1},{:.1}) v=({:.1},{:.1},{:.1}) |v|={:.1} n=({:.2},{:.2},{:.2}) near={ni} lat={lat:.1}/(-{hwl:.1},{hwr:.1}) up=({:.2},{:.2},{:.2}) h={height:.2} p6[{}] p12[{}] pup20[{}]",
        p.charge,
        p.lifetime,
        p.position.x,
        p.position.y,
        p.position.z,
        p.velocity.x,
        p.velocity.y,
        p.velocity.z,
        p.velocity.length(),
        p.surface.x,
        p.surface.y,
        p.surface.z,
        nup.x,
        nup.y,
        nup.z,
        probe(world, p.position, -p.surface, 6.0),
        probe(world, p.position, -p.surface, 12.0),
        probe(world, p.position, -nup, 20.0),
    )
}

fn out_path(name: &str) -> String {
    std::env::var("OAG_LOOP_TRACE_OUT").unwrap_or_else(|_| {
        oag_testdata::scratch::cache("loop-trace")
            .join(name)
            .display()
            .to_string()
    })
}

#[test]
#[ignore = "a scratch trace: set OAG_LOOP_TRACE=<hd|pulse>:<track>:<weapon>, read the log"]
fn trace_a_projectile_onto_a_bank() {
    let Some(Setup {
        spec,
        weapon,
        mut race,
        bank,
        mut log,
    }) = setup()
    else {
        return;
    };
    let lead: usize = env("OAG_LOOP_TRACE_LEAD", 40);
    let speed: f32 = env("OAG_LOOP_TRACE_SPEED", 140.0);
    let ticks: u32 = env("OAG_LOOP_TRACE_TICKS", 400);
    let lateral: f32 = env("OAG_LOOP_TRACE_LATERAL", f32::NAN);
    let len = race.core.sim.spline().len();
    let start = env("OAG_LOOP_TRACE_FIRST", (bank.0 + len - lead) % len);
    let sample = *race.core.sim.spline().sample(start).unwrap();
    let offset = if lateral.is_nan() {
        sample.racing_line
    } else {
        lateral
    };
    let pose = oag_gameplay::spawn::Pose::from_sample(&sample, offset, 3.0);
    let place = |race: &mut race::Race| {
        let craft = &mut race.core.sim.world.ships[0];
        craft.place_at(pose);
        craft.physics.body.linear_velocity = craft.physics.body.forward() * speed;
    };
    place(&mut race);
    writeln!(
        log,
        "# craft posed at sample {start} lateral {offset:.1}, {speed} u/s"
    )
    .unwrap();
    {
        let body = race.core.sim.world.ships[0].physics.body;
        let (p, f, u) = (body.position, body.forward(), body.up());
        writeln!(
            log,
            "# pose pos={:.4},{:.4},{:.4} forward={:.5},{:.5},{:.5} up={:.5},{:.5},{:.5}",
            p.x, p.y, p.z, f.x, f.y, f.z, u.x, u.y, u.z
        )
        .unwrap();
    }

    let mut input: Option<Input> = None;
    let mut born = false;
    for tick in 0..ticks {
        let fire = tick == 2;
        if fire {
            race.core.sim.world.ships[0].pickup.weapon = Some(weapon);
        }
        let bits = Button::Cross.bit() | if fire { Button::Square.bit() } else { 0 };
        let snap = snapshot(bits, input.as_ref());
        input = Some(snap.buttons);
        for (slot, impact) in predict(&race).iter().enumerate() {
            if let Some(impact) = impact
                && impact.kind == weapon
            {
                let (_, lat, height, hwl, hwr, _) = locate(&race, impact.point);
                writeln!(
                    log,
                    "t={tick} s{slot} IMPACT at ({:.1},{:.1},{:.1}) lat={lat:.1}/(-{hwl:.1},{hwr:.1}) h={height:.1} struck={:?}",
                    impact.point.x, impact.point.y, impact.point.z, impact.struck,
                )
                .unwrap();
            }
        }
        race.tick(&PlayerInputs::single(snap));
        // Held on the pose until the shot flies (the Plasma's second of charge included).
        let charging = race
            .core
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .any(|p| p.kind == Some(weapon) && p.charge > 0.0);
        if tick < 2 || charging {
            place(&mut race);
        }
        let ours: Vec<(usize, Projectile)> = race
            .core
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .enumerate()
            .filter(|(_, p)| p.kind == Some(weapon) && p.owner == 0)
            .map(|(i, p)| (i, *p))
            .collect();
        if ours.is_empty() {
            if born {
                writeln!(log, "t={tick} GONE").unwrap();
                break;
            }
            continue;
        }
        born = true;
        for (slot, p) in ours {
            writeln!(log, "{}", describe(&race, tick, slot, &p)).unwrap();
        }
    }
    let out = out_path(&format!("trace-{}.log", spec.rsplit(':').next().unwrap()));
    std::fs::write(&out, &log).expect("writing the log");
    println!("wrote {out}");
}

/// How a shot ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum End {
    /// A wall impact within 2 units of the track's edge.
    EdgeWall,
    /// Any other wall or cage impact.
    Wall,
    /// A craft's hull.
    Hull,
    /// Left the track: more than 15 units off the floor along the spline's up, or 2 beyond the
    /// edge, before it ended.
    LeftTrack,
    /// Reaped by age while on the track.
    Expired,
}

struct Shot {
    tick: u32,
    sample: usize,
    left: Option<(u32, usize, f32, f32)>,
    last: Option<usize>,
}

#[test]
#[ignore = "a scratch census: set OAG_LOOP_TRACE=<hd|pulse>:<track>:<weapon>, read the log"]
fn census_of_shots_along_a_bank() {
    let Some(Setup {
        spec,
        weapon,
        mut race,
        bank,
        mut log,
    }) = setup()
    else {
        return;
    };
    let every: u32 = env("OAG_LOOP_TRACE_EVERY", 20);
    let ticks: u32 = env("OAG_LOOP_TRACE_TICKS", 7_200);
    let lead: usize = env("OAG_LOOP_TRACE_LEAD", 40);
    race.core.sim.set_autopilot(true);
    let len = race.core.sim.spline().len();
    let from = (bank.0 + len - lead) % len;
    let window = |i: usize| {
        if from <= bank.1 {
            (from..=bank.1).contains(&i)
        } else {
            i >= from || i <= bank.1
        }
    };

    let mut shots: [Option<Shot>; MAX_PROJECTILES] = std::array::from_fn(|_| None);
    let mut ends: std::collections::BTreeMap<End, usize> = Default::default();
    let mut by_bucket: std::collections::BTreeMap<(usize, End), usize> = Default::default();
    let mut input: Option<Input> = None;
    for tick in 0..ticks {
        let at = locate(&race, race.core.sim.world.ships[0].physics.body.position).0;
        let fire = tick >= 240 && tick % every == 0 && window(at);
        if fire {
            race.core.sim.world.ships[0].pickup.weapon = Some(weapon);
        }
        let bits = Button::Cross.bit() | if fire { Button::Square.bit() } else { 0 };
        let snap = snapshot(bits, input.as_ref());
        input = Some(snap.buttons);
        let impacts = predict(&race);
        race.tick(&PlayerInputs::single(snap));
        for (slot, impact) in impacts.iter().enumerate() {
            let p = race.core.sim.world.projectiles.slots[slot];
            let mine = p.kind == Some(weapon) && p.owner == 0;
            match (&mut shots[slot], mine) {
                (None, true) => {
                    shots[slot] = Some(Shot {
                        tick,
                        sample: at,
                        left: None,
                        last: None,
                    });
                }
                (Some(shot), true) => {
                    if p.charge > 0.0 {
                        shot.sample = at;
                        continue;
                    }
                    let (i, lat, height, hwl, hwr, _) = locate(&race, p.position);
                    shot.last = Some(i);
                    if shot.left.is_none()
                        && (height.abs() > 15.0 || lat < -hwl - 2.0 || lat > hwr + 2.0)
                    {
                        shot.left = Some((tick, i, lat, height));
                    }
                }
                (Some(shot), false) => {
                    let end = match impact {
                        _ if shot.left.is_some() => End::LeftTrack,
                        Some(impact) if impact.struck.is_some() => End::Hull,
                        Some(impact) => {
                            let (_, lat, _, hwl, hwr, _) = locate(&race, impact.point);
                            if lat < -hwl + 2.0 || lat > hwr - 2.0 {
                                End::EdgeWall
                            } else {
                                End::Wall
                            }
                        }
                        None => End::Expired,
                    };
                    writeln!(
                        log,
                        "shot t={} from sample {} ended t={tick} at sample {:?}: {end:?}{}",
                        shot.tick,
                        shot.sample,
                        shot.last,
                        shot.left.map_or(String::new(), |(t, i, lat, h)| format!(
                            " (left at t={t} sample {i} lat {lat:.1} h {h:.1})"
                        ))
                    )
                    .unwrap();
                    *ends.entry(end).or_default() += 1;
                    *by_bucket.entry((shot.sample / 100 * 100, end)).or_default() += 1;
                    shots[slot] = None;
                }
                (None, false) => {}
            }
        }
    }
    writeln!(log, "# ends: {ends:?}").unwrap();
    for ((bucket, end), n) in &by_bucket {
        writeln!(
            log,
            "# from samples {bucket}..{}: {end:?} x{n}",
            bucket + 99
        )
        .unwrap();
    }
    let out = out_path(&format!("census-{}.log", spec.rsplit(':').next().unwrap()));
    std::fs::write(&out, &log).expect("writing the log");
    println!("wrote {out}");
}

/// Each `x,y,z` line of `OAG_LOOP_TRACE_POINTS` against the spline: sample, lateral, height.
#[test]
#[ignore = "a scratch locator: set OAG_LOOP_TRACE and OAG_LOOP_TRACE_POINTS=<file>"]
fn locate_recorded_points() {
    let Some(points) = std::env::var_os("OAG_LOOP_TRACE_POINTS") else {
        return;
    };
    let Some(Setup { race, .. }) = setup() else {
        return;
    };
    let text = std::fs::read_to_string(points).expect("the points file");
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let (tag, xyz) = line.split_once(' ').unwrap_or(("", line));
        let v: Vec<f32> = xyz.split(',').map(|s| s.trim().parse().unwrap()).collect();
        let at = Vec3::new(v[0], v[1], v[2]);
        let (i, lat, h, hwl, hwr, up) = locate(&race, at);
        let floor = race
            .core
            .sim
            .collision()
            .raycast(Ray::new(at, -up, 20.0), None, false);
        println!(
            "{tag} ({:.1},{:.1},{:.1}) near={i} lat={lat:.1}/(-{hwl:.1},{hwr:.1}) h={h:.2} floor={:?}",
            at.x,
            at.y,
            at.z,
            floor.map(|f| (f.distance, f.surface))
        );
    }
}

/// The bolt as `trace_a_projectile_onto_a_bank` releases it, then flown alone through
/// `projectile::step` with `OAG_LOOP_TRACE_ARMS` (`steer` or `parallel`), the pool's other laws
/// unchanged; prints the bolt's lateral offset every 10 ticks.
#[test]
#[ignore = "a scratch experiment: set OAG_LOOP_TRACE and OAG_LOOP_TRACE_ARMS"]
fn fly_the_released_bolt_with_chosen_arms() {
    let Some(arms) = std::env::var("OAG_LOOP_TRACE_ARMS").ok() else {
        return;
    };
    let arms = match arms.as_str() {
        "steer" => oag_weapons::projectile::FloorArms::SteerToRidePoint,
        _ => oag_weapons::projectile::FloorArms::ParallelToFloor,
    };
    let Some(Setup {
        weapon,
        mut race,
        bank,
        ..
    }) = setup()
    else {
        return;
    };
    let len = race.core.sim.spline().len();
    let start = env("OAG_LOOP_TRACE_FIRST", (bank.0 + len - 40) % len);
    let sample = *race.core.sim.spline().sample(start).unwrap();
    let pose = oag_gameplay::spawn::Pose::from_sample(&sample, sample.racing_line, 3.0);
    let place = |race: &mut race::Race| {
        let craft = &mut race.core.sim.world.ships[0];
        craft.place_at(pose);
        craft.physics.body.linear_velocity = craft.physics.body.forward() * 140.0;
    };
    place(&mut race);
    let mut input: Option<Input> = None;
    // Tick until the shot is out of its charge.
    for tick in 0..200 {
        let fire = tick == 2;
        if fire {
            race.core.sim.world.ships[0].pickup.weapon = Some(weapon);
        }
        let bits = Button::Cross.bit() | if fire { Button::Square.bit() } else { 0 };
        let snap = snapshot(bits, input.as_ref());
        input = Some(snap.buttons);
        race.tick(&PlayerInputs::single(snap));
        let slots = &race.core.sim.world.projectiles.slots;
        if tick > 2
            && slots
                .iter()
                .any(|p| p.kind == Some(weapon) && p.charge <= 0.0)
        {
            break;
        }
        place(&mut race);
    }
    let mut world = race.core.sim.world.clone();
    let dt = race.core.sim.dt();
    for tick in 0..200 {
        let count = world.ship_count as usize;
        let mut hits = vec![oag_weapons::projectile::WeaponHit::default(); world.ships.len()];
        let impacts = oag_weapons::projectile::step(
            &mut world.projectiles,
            &mut world.ships[..count],
            dt,
            race.core.sim.collision(),
            None,
            race.core.sim.weapon_stats(),
            "VENOM",
            oag_weapons::projectile::FlightLaws {
                floor_arms: arms,
                ..race.core.sim.flight_laws()
            },
            race.core.sim.own_craft_probe(),
            race.core.sim.plasma_release(),
            oag_weapons::projectile::BlastPush::Posted,
            oag_physics::DamageRules::default(),
            &mut hits,
        );
        let Some((slot, p)) = world
            .projectiles
            .slots
            .iter()
            .enumerate()
            .find(|(_, p)| p.kind == Some(weapon))
        else {
            println!("tick {tick}: ended {:?}", impacts.iter().flatten().next());
            break;
        };
        if tick % 5 == 0 {
            println!("{}", describe(&race, tick, slot, p));
        }
    }
}
