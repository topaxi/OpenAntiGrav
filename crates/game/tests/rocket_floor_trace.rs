//! Scratch: what a rocket volley does, tick by tick, on a circuit's floor.
//!
//! `#[ignore]`d and gated on an env var, never run in CI or in
//! `just test-data` - the same shape as `weapon_floor_sweep.rs`. Flies the
//! player's craft on the autopilot, fires a Rocket volley every
//! `OAG_ROCKET_TRACE_EVERY` ticks (default 120), and re-runs each projectile's
//! own surface probe and flight sweep against the race's own
//! `CollisionWorld` every tick so that the tick a rocket stops on names *why*:
//! the probe missed and it fell, the probe found a wall, the chord from `from`
//! to `to` cut a wall, it struck a hull, or it aged out.
//!
//! ```sh
//! OAG_ROCKET_TRACE=hd:/data/environments/10_sebenco_climb/track.vex \
//!   OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!   -E 'binary(rocket_floor_trace)' --no-capture
//! ```
//!
//! `OAG_ROCKET_TRACE` is `<title>:<track>` with `title` one of `hd` or
//! `pulse`; `OAG_ROCKET_TRACE_OUT` names the per-tick log file.

use std::fmt::Write as _;
use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_game::race;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_gameplay::projectile::{
    FALL_ACCELERATION, MAX_PROJECTILES, RIDE_HEIGHT, SURFACE_PROBE_LENGTH,
};
use oag_physics::{Ray, Raycaster, Surface};
use oag_tables::weapons::Weapon;

fn image(title: &str) -> Option<PathBuf> {
    match title {
        "hd" => oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso"),
        "pulse" => oag_testdata::image("data/images/pulse-psp-eu.chd"),
        _ => None,
    }
}

fn load(title: &str, track: Option<&str>) -> Option<race::Loaded> {
    let image = image(title)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: track.map(str::to_string),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        if line.contains("collision") || line.contains("triangle") {
            println!("{line}");
        }
    }
    Some(loaded)
}

fn snapshot(bits: u32, previous: Option<&Input>) -> oag_gameplay::InputSnapshot {
    let mut buttons = previous.copied().unwrap_or_else(Input::new);
    buttons.begin_frame(bits);
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum End {
    /// The chord from `from` to `to` hit a wall.
    SweptIntoGeometry,
    /// The surface probe found a wall beneath it.
    ProbedWall,
    /// A craft's hull.
    Hull,
    /// Ran out of `rocket::LIFETIME_SECONDS`, the original's own 5.0 s.
    Expired,
    /// Gone for a reason this replica did not predict (a blast, a respawn).
    Unexplained,
}

/// One projectile's tick, replicated from `Projectiles::advance`.
#[derive(Debug, Clone, Copy)]
struct Tick {
    position: Vec3,
    surface: Vec3,
    speed: f32,
    probe: Option<(f32, f32, Surface)>,
    rideable: bool,
    sweep: Option<(f32, f32, Surface)>,
    step: f32,
    /// The nearest opponent hull the chord enters, as a distance along it.
    hull: Option<(u8, f32)>,
}

fn segment_sphere(p0: Vec3, p1: Vec3, centre: Vec3, radius: f32) -> Option<f32> {
    let d = p1 - p0;
    let m = p0 - centre;
    let a = d.dot(d);
    if a <= 0.0 {
        return None;
    }
    let b = m.dot(d);
    let c = m.dot(m) - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    if b >= 0.0 {
        return None;
    }
    let disc = b * b - a * c;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()) / a;
    (0.0..=1.0).contains(&t).then_some(t)
}

fn replicate<R: Raycaster + ?Sized>(
    projectile: &oag_gameplay::projectile::Projectile,
    dt: f32,
    world: &R,
    ships: &[oag_gameplay::Ship],
) -> Tick {
    let from = projectile.position;
    let mut to = from + projectile.velocity * dt;
    let probe = world.raycast(
        Ray::new(to, -projectile.surface, SURFACE_PROBE_LENGTH),
        None,
        false,
    );
    let mut rideable = false;
    let mut velocity = projectile.velocity;
    let mut surface = projectile.surface;
    match probe {
        Some(hit) if hit.surface.is_hoverable() => {
            rideable = true;
            surface = hit.normal;
            to = hit.point + hit.normal * RIDE_HEIGHT;
            let along = velocity - hit.normal * velocity.dot(hit.normal);
            if along.length_squared() > 1e-6 {
                velocity = along.normalize() * velocity.length();
            }
        }
        Some(_) => {}
        None => velocity -= Vec3::Y * FALL_ACCELERATION * dt,
    }
    let _ = velocity;
    let step = to - from;
    let distance = step.length();
    let sweep = if distance > 0.0 {
        let direction = step / distance;
        world
            .raycast(Ray::new(from, direction, distance), None, false)
            .map(|hit| (hit.distance, -hit.normal.dot(direction), hit.surface))
    } else {
        None
    };
    let mut hull = None;
    for (slot, ship) in ships.iter().enumerate() {
        if !ship.active || slot as u8 == projectile.owner {
            continue;
        }
        let radius = oag_gameplay::projectile::hull_radius(&ship.handling.dimensions);
        if let Some(t) = segment_sphere(from, to, ship.physics.body.position, radius) {
            let d = t * distance;
            if hull.is_none_or(|(_, best)| d < best) {
                hull = Some((slot as u8, d));
            }
        }
    }
    Tick {
        position: from,
        surface,
        speed: projectile.velocity.length(),
        hull,
        probe: probe.map(|hit| {
            (
                hit.distance,
                hit.normal.dot(projectile.surface),
                hit.surface,
            )
        }),
        rideable,
        sweep,
        step: distance,
    }
}

fn floor_winding(world: &oag_physics::CollisionWorld) -> (usize, usize, usize) {
    let (mut up, mut down, mut flat) = (0, 0, 0);
    for collider in world.colliders() {
        if !matches!(collider.surface(), Surface::Floor | Surface::MagFloor) {
            continue;
        }
        for i in 0..collider.triangle_count() {
            let Some([a, b, c]) = collider.triangle(i) else {
                continue;
            };
            let n = (b - a).cross(c - a);
            if n.length_squared() <= 0.0 {
                flat += 1;
            } else if n.y > 0.0 {
                up += 1;
            } else {
                down += 1;
            }
        }
    }
    (up, down, flat)
}

#[test]
#[ignore = "a scratch trace: set OAG_ROCKET_TRACE=<hd|pulse>:<track>, read the log"]
fn trace_a_rocket_volley_along_the_floor() {
    let Some(spec) = std::env::var_os("OAG_ROCKET_TRACE") else {
        return;
    };
    let spec = spec.to_string_lossy().to_string();
    let (title, track) = spec.split_once(':').expect("<title>:<track>");
    let track = (!track.is_empty()).then_some(track);
    let every: u32 = std::env::var("OAG_ROCKET_TRACE_EVERY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);
    let ticks: u32 = std::env::var("OAG_ROCKET_TRACE_TICKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7_200);
    let out = std::env::var("OAG_ROCKET_TRACE_OUT")
        .unwrap_or_else(|_| "/tmp/oag-drive/hd-rocket-floor/trace.log".to_string());

    let Some(loaded) = load(title, track) else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    let dt = race.dt();

    let (up, down, flat) = floor_winding(race.collision());
    let mut log = String::new();
    writeln!(
        log,
        "# {title} {} floor triangles: {up} normal.y>0, {down} normal.y<0, {flat} degenerate; \
         max half-width {:.2}",
        track.unwrap_or("(default)"),
        race.spline().max_half_width()
    )
    .unwrap();
    let stats = race.rocket_stats().expect("the disc authors a Rocket");
    let cruise = stats.speed_for_named("VENOM").unwrap_or(0.0) / 3.6;
    writeln!(
        log,
        "# rocket venom {:.0} km/h -> {:.2} units/s cruise (0.75 of it at launch), {:.3} units/tick",
        stats.speed_for_named("VENOM").unwrap_or(0.0),
        cruise,
        cruise * dt
    )
    .unwrap();

    let mut input: Option<Input> = None;
    let mut alive: [Option<(u32, Tick)>; MAX_PROJECTILES] = [None; MAX_PROJECTILES];
    let mut born: [u32; MAX_PROJECTILES] = [0; MAX_PROJECTILES];
    let mut ends: std::collections::BTreeMap<End, usize> = Default::default();
    let mut fell_ticks = 0usize;
    let mut rode_ticks = 0usize;
    let mut wall_ticks = 0usize;
    let mut lifetimes = Vec::new();
    // Warm up: let the craft get moving before the first volley.
    for tick in 0..ticks {
        let fire = tick >= 240 && tick % every == 0;
        if fire {
            race.sim.world.ships[0].pickup.weapon = Some(Weapon::Rocket);
        }
        let bits = if fire {
            Button::Cross.bit() | Button::Square.bit()
        } else {
            Button::Cross.bit()
        };
        let snap = snapshot(bits, input.as_ref());
        input = Some(snap.buttons);

        // Predict this tick for every live rocket from its pre-tick state.
        let mut predicted: [Option<Tick>; MAX_PROJECTILES] = [None; MAX_PROJECTILES];
        for (slot, p) in race.sim.world.projectiles.slots.iter().enumerate() {
            if p.kind == Some(Weapon::Rocket) {
                let count = race.sim.world.ship_count as usize;
                predicted[slot] = Some(replicate(
                    p,
                    dt,
                    race.collision(),
                    &race.sim.world.ships[..count],
                ));
            }
        }
        race.tick(&PlayerInputs::single(snap));

        let ship = race.sim.world.ships[0].physics.body.position;
        for slot in 0..MAX_PROJECTILES {
            let now = race.sim.world.projectiles.slots[slot];
            let Some(t) = predicted[slot] else {
                if now.kind == Some(Weapon::Rocket) {
                    born[slot] = tick;
                    writeln!(
                        log,
                        "t={tick} slot={slot} LAUNCH pos=({:.1},{:.1},{:.1}) v=({:.1},{:.1},{:.1}) ship=({:.1},{:.1},{:.1})",
                        now.position.x, now.position.y, now.position.z,
                        now.velocity.x, now.velocity.y, now.velocity.z,
                        ship.x, ship.y, ship.z
                    )
                    .unwrap();
                    let count = race.sim.world.ship_count as usize;
                    alive[slot] = Some((
                        tick,
                        replicate(&now, dt, race.collision(), &race.sim.world.ships[..count]),
                    ));
                }
                continue;
            };
            if t.rideable {
                rode_ticks += 1;
            } else if t.probe.is_some() {
                wall_ticks += 1;
            } else {
                fell_ticks += 1;
            }
            let probe = match t.probe {
                Some((d, cos, s)) => format!("probe=hit d={d:.2} cos={cos:.2} {s:?}"),
                None => "probe=MISS".to_string(),
            };
            let sweep = match t.sweep {
                Some((d, facing, s)) => format!("sweep=hit d={d:.2} facing={facing:.2} {s:?}"),
                None => "sweep=-".to_string(),
            };
            writeln!(
                log,
                "t={tick} slot={slot} age={} pos=({:.1},{:.1},{:.1}) n=({:.2},{:.2},{:.2}) spd={:.1} step={:.2} {} {probe} {sweep}",
                tick - born[slot],
                t.position.x, t.position.y, t.position.z,
                t.surface.x, t.surface.y, t.surface.z,
                t.speed, t.step,
                if t.rideable { "RIDE" } else if t.probe.is_some() { "WALL" } else { "FALL" },
            )
            .unwrap();
            if now.kind != Some(Weapon::Rocket) {
                let age = tick - born[slot];
                let geometry = t
                    .sweep
                    .filter(|&(_, _, surface)| !surface.is_hoverable())
                    .map(|(d, _, _)| d);
                let probe_wall = t.probe.is_some_and(|(_, _, s)| !s.is_hoverable());
                let end = match (geometry, t.hull) {
                    _ if probe_wall => End::ProbedWall,
                    (Some(g), Some((_, h))) if h < g => End::Hull,
                    (Some(_), _) => End::SweptIntoGeometry,
                    (None, Some(_)) => End::Hull,
                    _ if age as f32 * dt
                        >= oag_gameplay::projectile::ROCKET_LIFETIME_SECONDS - dt =>
                    {
                        End::Expired
                    }
                    _ => End::Unexplained,
                };
                *ends.entry(end).or_default() += 1;
                lifetimes.push(age);
                writeln!(log, "t={tick} slot={slot} END {end:?} after {age} ticks").unwrap();
                alive[slot] = None;
            }
        }
    }
    let mut summary = String::new();
    writeln!(summary, "# ends: {ends:?}").unwrap();
    writeln!(
        summary,
        "# ticks riding {rode_ticks}, probe found a wall {wall_ticks}, falling {fell_ticks}"
    )
    .unwrap();
    lifetimes.sort_unstable();
    if !lifetimes.is_empty() {
        writeln!(
            summary,
            "# lifetimes: n={} min={} median={} max={}",
            lifetimes.len(),
            lifetimes[0],
            lifetimes[lifetimes.len() / 2],
            lifetimes[lifetimes.len() - 1]
        )
        .unwrap();
    }
    writeln!(summary, "# player respawns {}", race.respawns()).unwrap();
    println!("{}", log.lines().take(2).collect::<Vec<_>>().join("\n"));
    println!("{summary}");
    std::fs::write(&out, format!("{log}{summary}")).expect("writing the log");
    println!("wrote {out}");
}
