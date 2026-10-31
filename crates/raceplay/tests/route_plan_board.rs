//! The board behind "a fork route's speed plan costs what it is driven for,
//! and the AI is no worse on any route".
//!
//! **`#[ignore]`d, gated on `OAG_SWEEP`, printing rather than asserting**: it
//! loads every forked circuit of every title with data, learns its plans and
//! races them, minutes per circuit. One row per measurement, tab separated,
//! also appended to `OAG_ROUTE_BOARD_OUT` when set.
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 OAG_ROUTE_BOARD_OUT=board.tsv \
//!   cargo nextest run --release -p oag-raceplay --run-ignored all \
//!   --no-capture route_plan_board
//! ```
//!
//! Per circuit, class (`OAG_ROUTE_BOARD_CLASSES`, default all four) and seed
//! (`OAG_ROUTE_BOARD_SEEDS`, default `1,2,3`), at the AI level
//! `OAG_ROUTE_BOARD_DIFFICULTY` (default the race's); `OAG_ROUTE_BOARD_ONLY`
//! names circuits by label, `OAG_ROUTE_BOARD_WHERE` prints every wall
//! contact of a solo run:
//!
//! - `plan`: per route, whether its reference plan verified, the physics steps
//!   it took (`Race::plan_steps`), the steps each other hull's adaptation took,
//!   and the wall clock of learning every plan of the race.
//! - `solo`: per route, one craft alone, forced down it at every visit (the
//!   coin's draw overwritten), as slot 0 (the player's hull, the plans'
//!   reference where its plan verified) and as slot 1 (an opponent's hull),
//!   both on this project's driver at the race's level:
//!   ticks from moving onto the route's line to back on the ring, wall contact
//!   ticks and respawns over that stretch.
//! - `field`: a full field with the coin as it is, every craft on this
//!   project's driver, per route: traversals, mean
//!   ticks, wall contact ticks, respawns; and the field's lap spread.
//!
//! The plans are learned once per circuit and seed and copied into each run's
//! simulation before its first tick, so every run drives the same plans.

use std::io::Write as _;
use std::time::Instant;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

/// Long enough for three visits to any fork at any class: a solo run stops at
/// [`VISITS`] traversals.
const SOLO_TICKS: u32 = 60 * 300;
const VISITS: usize = 3;
/// The field window `ai_fork_split_ground_truth.rs` uses on `altima`.
const FIELD_TICKS: u32 = 60 * 210;

struct Circuit {
    label: String,
    source: String,
    track: String,
}

fn sweeping() -> bool {
    std::env::var_os("OAG_SWEEP").is_some()
}

fn seeds() -> Vec<u64> {
    std::env::var("OAG_ROUTE_BOARD_SEEDS")
        .ok()
        .map(|text| {
            text.split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect()
        })
        .unwrap_or_else(|| vec![1, 2, 3])
}

fn classes() -> Vec<String> {
    std::env::var("OAG_ROUTE_BOARD_CLASSES")
        .unwrap_or_else(|_| "VENOM,FLASH,RAPIER,PHANTOM".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect()
}

/// The race's AI level (`OAG_ROUTE_BOARD_DIFFICULTY`, `Novice` to `Ace`),
/// default the race's own default.
fn difficulty() -> oag_ai::Difficulty {
    match std::env::var("OAG_ROUTE_BOARD_DIFFICULTY").as_deref() {
        Ok("Novice") => oag_ai::Difficulty::Novice,
        Ok("Skilled") => oag_ai::Difficulty::Skilled,
        Ok("Ace") => oag_ai::Difficulty::Ace,
        Ok("Elite") => oag_ai::Difficulty::Elite,
        _ => oag_ai::Difficulty::default(),
    }
}

/// Slot 0 on the player's autopilot flown by this project's driver at the
/// race's own level, as an opponent is: the autopilot otherwise flies the
/// original's driver, which reads no speed plan.
fn fly_ours(race: &mut race::Race) {
    race.set_autopilot(true);
    race.set_autopilot_tuning(difficulty().tune(&oag_ai::Tuning::default()));
}

fn only() -> Option<String> {
    std::env::var("OAG_ROUTE_BOARD_ONLY").ok()
}

fn emit(row: &str) {
    println!("{row}");
    if let Ok(path) = std::env::var("OAG_ROUTE_BOARD_OUT")
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(file, "{row}");
    }
}

fn pulse_circuits() -> Vec<Circuit> {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return Vec::new();
    };
    let source = image.display().to_string();
    let mut archives = oag_pulse::open(&source).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    catalogue::tracks(&definition)
        .into_iter()
        .filter(|t| {
            ["05_Track", "07_Track", "14_Track"]
                .iter()
                .any(|id| t.location.ends_with(id))
        })
        .map(|t| Circuit {
            label: format!(
                "pulse/{}{}",
                t.location.rsplit(['\\', '/']).next().unwrap_or(&t.id),
                if t.reversed { "-rev" } else { "" }
            ),
            source: source.clone(),
            track: t.entry_name(),
        })
        .collect()
}

const NAMES_2048: [&str; 10] = [
    "square",
    "park",
    "tower",
    "mall",
    "bridge",
    "arena",
    "subway",
    "cathedral",
    "sol",
    "altima",
];

fn circuits_2048() -> Vec<Circuit> {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return Vec::new();
    };
    NAMES_2048
        .iter()
        .map(|name| Circuit {
            label: format!("2048/{name}"),
            source: source.display().to_string(),
            track: format!(r"Data\art\published\environments\{name}\track.vex"),
        })
        .collect()
}

fn circuits_omega() -> Vec<Circuit> {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return Vec::new();
    };
    vec![Circuit {
        label: "omega/altima".into(),
        source: source.display().to_string(),
        track: r"Data\environments2048\altima\track.vex".into(),
    }]
}

fn load(circuit: &Circuit, class: &str, seed: u64) -> Option<race::Setup> {
    let loaded = race::load(&race::Options {
        source: circuit.source.clone(),
        mode: oag_race::Mode::SingleRace,
        track: Some(circuit.track.clone()),
        seed: Some(seed),
        class: class.to_string(),
        difficulty: difficulty(),
        ..race::Options::default()
    });
    match loaded {
        Ok(loaded) => {
            let mut setup = loaded.setup;
            crate::perturb::apply(&mut setup, 0.0);
            Some(setup)
        }
        Err(error) => {
            emit(&format!(
                "error\t{}\t{class}\t{seed}\t{error:#}",
                circuit.label
            ));
            None
        }
    }
}

/// A race on `setup` driving `plans`' plans: the simulation copied before
/// the first tick, so nothing is learned again.
fn with_plans(setup: &race::Setup, plans: &race::Race) -> race::Race {
    let mut race = race::Race::start(setup.clone());
    race.sim = plans.sim.clone();
    race
}

/// Per route line (`0` the ring), what crossing a fork's stretch did.
#[derive(Default, Clone)]
struct Tally {
    visits: u32,
    ticks: u64,
    contacts: u64,
    respawns: u64,
}

struct Watch {
    /// `(line, start tick, contacts, respawns, fork)` while on a route.
    open: Option<(u16, u32, u32, u32)>,
}

/// The ring's own traversal of a fork's bypassed span is not measured (a craft
/// on the ring never changes `route`); the solo runs time the ring by forcing
/// it and timing from the switch index to the merge on the ring instead, see
/// [`solo`].
fn watch(race: &race::Race, slot: usize, tick: u32, w: &mut Watch, tally: &mut [Tally]) {
    let route = race.sim.world.ships[slot].driver.branching.route;
    let contacts = race.wall_contact_ticks_of(slot);
    let respawns = race.respawns_of(slot);
    match w.open {
        None if route != 0 => w.open = Some((route, tick, contacts, respawns)),
        Some((line, from, c0, r0)) if route != line => {
            let t = &mut tally[usize::from(line)];
            t.visits += 1;
            t.ticks += u64::from(tick - from);
            t.contacts += u64::from(contacts - c0);
            t.respawns += u64::from(respawns - r0);
            w.open = (route != 0).then_some((route, tick, contacts, respawns));
        }
        _ => {}
    }
}

/// One craft alone in `slot`, sent down route `want` (`k + 1`) at every visit
/// to its fork.
fn solo(setup: &race::Setup, plans: &race::Race, slot: usize, want: u16) -> Tally {
    let mut race = with_plans(setup, plans);
    let count = race.ship_count() as usize;
    for other in 0..count {
        if other != slot {
            race.sim.world.ships[other].active = false;
        }
    }
    if slot == 0 {
        fly_ours(&mut race);
    }
    let course = race
        .course()
        .expect("a forked circuit has a course")
        .clone();
    let pre_fork = course.routes()[usize::from(want - 1)].pre_fork;
    let mut tally = vec![Tally::default(); race.route_count() + 1];
    let mut w = Watch { open: None };
    for tick in 0..SOLO_TICKS {
        {
            let branching = &mut race.sim.world.ships[slot].driver.branching;
            if branching.route == 0 && branching.decided_at == pre_fork + 1 {
                branching.pending = want;
            }
        }
        let before = race.wall_contact_ticks_of(slot);
        race.tick(&PlayerInputs::none());
        if std::env::var_os("OAG_ROUTE_BOARD_WHERE").is_some()
            && race.wall_contact_ticks_of(slot) > before
        {
            let ship = &race.sim.world.ships[slot];
            let speed = ship.physics.body.linear_velocity.length();
            let (route, index) = (ship.driver.branching.route, ship.driver.index);
            let target = race
                .plan_of(slot)
                .map_or(f32::INFINITY, |p| p.target(index as usize));
            emit(&format!(
                "contact\tslot{slot}\troute{route}\tindex={index}\tspeed={speed:.1}\ttarget={target:.1}"
            ));
        }
        if std::env::var_os("OAG_ROUTE_BOARD_WHERE").is_some() && tick % 300 == 0 {
            let ship = &race.sim.world.ships[slot];
            emit(&format!(
                "at\tslot{slot}\ttick={tick}\troute={}\tindex={}\tlap={}\tspeed={:.1}\tstate={:?}\trespawns={}",
                ship.driver.branching.route,
                ship.driver.index,
                ship.standing.lap,
                ship.physics.body.linear_velocity.length(),
                ship.physics.craft_state,
                race.respawns_of(slot),
            ));
        }
        if std::env::var_os("OAG_ROUTE_BOARD_WHERE").is_some() {
            let ship = &race.sim.world.ships[slot];
            let route = ship.driver.branching.route;
            if w.open.map(|o| o.0) != (route != 0).then_some(route) {
                emit(&format!(
                    "switch\tslot{slot}\ttick={tick}\troute={route}\tindex={}\tlap={}\tpending={}\tdecided={}",
                    ship.driver.index,
                    ship.standing.lap,
                    ship.driver.branching.pending,
                    ship.driver.branching.decided_at
                ));
            }
        }
        watch(&race, slot, tick, &mut w, &mut tally);
        if tally[usize::from(want)].visits as usize >= VISITS {
            break;
        }
        if race.sim.world.ships[slot].physics.craft_state != oag_physics::CraftState::Racing {
            let ship = &race.sim.world.ships[slot];
            emit(&format!(
                "ended\tslot{slot}\ttick={tick}\tstate={:?}\troute={}\tindex={}\tlap={}",
                ship.physics.craft_state,
                ship.driver.branching.route,
                ship.driver.index,
                ship.standing.lap
            ));
            break;
        }
    }
    tally.swap_remove(usize::from(want))
}

fn field(setup: &race::Setup, plans: &race::Race) -> (Vec<Tally>, Vec<u32>, u32) {
    let mut race = with_plans(setup, plans);
    fly_ours(&mut race);
    let count = race.ship_count() as usize;
    let mut tally = vec![Tally::default(); race.route_count() + 1];
    let mut watches: Vec<Watch> = (0..count).map(|_| Watch { open: None }).collect();
    for tick in 0..FIELD_TICKS {
        race.tick(&PlayerInputs::none());
        for (slot, w) in watches.iter_mut().enumerate() {
            watch(&race, slot, tick, w, &mut tally);
        }
    }
    let laps = (0..count)
        .map(|s| race.sim.world.ships[s].standing.lap)
        .collect();
    let wrecked = (0..count)
        .filter(|&s| race.sim.world.ships[s].physics.craft_state != oag_physics::CraftState::Racing)
        .count() as u32;
    (tally, laps, wrecked)
}

fn board(circuit: &Circuit) {
    for class in classes() {
        for seed in seeds() {
            board_at(circuit, &class, seed);
        }
    }
}

fn board_at(circuit: &Circuit, class: &str, seed: u64) {
    {
        let Some(setup) = load(circuit, class, seed) else {
            return;
        };
        let mut plans = race::Race::start(setup.clone());
        let routes = plans.route_count();
        if routes == 0 {
            emit(&format!("noforks\t{}\t{class}\t{seed}", circuit.label));
            return;
        }
        let started = Instant::now();
        let steps = plans.plan_steps().clone();
        let seconds = started.elapsed().as_secs_f64();
        let ring = plans.speed_plan().is_some();
        for k in 0..routes {
            let hulls: Vec<String> = steps
                .hull_routes
                .iter()
                .map(|h| h.get(k).copied().unwrap_or(0).to_string())
                .collect();
            emit(&format!(
                "plan\t{}\t{class}\t{seed}\troute{k}\tclean={}\tsteps={}\thull_steps={}\tring_plan={ring}\tall_plans_s={seconds:.2}",
                circuit.label,
                steps.routes_clean.get(k).copied().unwrap_or(false),
                steps.routes.get(k).copied().unwrap_or(0),
                hulls.join(","),
            ));
        }
        for k in 0..routes {
            if !steps.routes_clean.get(k).copied().unwrap_or(false) {
                continue;
            }
            for slot in [0usize, 1] {
                let t = solo(&setup, &plans, slot, (k + 1) as u16);
                emit(&format!(
                    "solo\t{}\t{class}\t{seed}\troute{k}\tslot{slot}\tvisits={}\tticks={}\tmean={:.1}\tcontacts={}\trespawns={}",
                    circuit.label,
                    t.visits,
                    t.ticks,
                    t.ticks as f64 / f64::from(t.visits.max(1)),
                    t.contacts,
                    t.respawns
                ));
            }
        }
        let (tally, laps, wrecked) = field(&setup, &plans);
        for (line, t) in tally.iter().enumerate().skip(1) {
            emit(&format!(
                "field\t{}\t{class}\t{seed}\troute{}\tvisits={}\tmean={:.1}\tcontacts={}\trespawns={}",
                circuit.label,
                line - 1,
                t.visits,
                t.ticks as f64 / f64::from(t.visits.max(1)),
                t.contacts,
                t.respawns
            ));
        }
        emit(&format!(
            "laps\t{}\t{class}\t{seed}\tlaps={laps:?}\twrecked={wrecked}",
            circuit.label
        ));
    }
}

#[test]
#[ignore = "a sweep over every forked circuit: OAG_SWEEP=1, needs the discs"]
fn route_plan_board() {
    if !sweeping() {
        return;
    }
    let mut circuits = pulse_circuits();
    circuits.extend(circuits_omega());
    circuits.extend(circuits_2048());
    for circuit in &circuits {
        if only().is_some_and(|only| !only.split(',').any(|o| circuit.label == o)) {
            continue;
        }
        board(circuit);
    }
}

/// Where a route's reference plan fails its verification: per circuit
/// (`OAG_ROUTE_BOARD_ONLY`), class and seed, each route's learning report and
/// every failure tick of its verification replayed (`Race::probe_route_plan`),
/// with the line index read as ring or route sample. `OAG_ROUTE_PROBE_DUMP`
/// names a directory that gets every tick of every route as a TSV;
/// `OAG_ROUTE_PROBE_STAGES=<tick>` re-steps that verification tick stage by
/// stage and prints the craft's position after each;
/// `OAG_ROUTE_PROBE_STUCK=<ticks>` counts that long without progress as a
/// stall and puts the craft back on, to find every place it parks;
/// `OAG_ROUTE_PROBE_RING` alone probes the ring's own plan instead (printed as
/// `route=18446744073709551615`, every index a ring sample).
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 OAG_ROUTE_BOARD_ONLY=pulse/07_Track \
///   OAG_ROUTE_BOARD_CLASSES=VENOM OAG_ROUTE_BOARD_SEEDS=1 \
///   cargo nextest run --release -p oag-raceplay --run-ignored all \
///   --no-capture route_verify_probe
/// ```
#[test]
#[ignore = "a probe over forked circuits: OAG_SWEEP=1, needs the discs"]
fn route_verify_probe() {
    if !sweeping() {
        return;
    }
    let dump = std::env::var("OAG_ROUTE_PROBE_DUMP").ok();
    let circuits = probe_circuits();
    for circuit in &circuits {
        if only().is_some_and(|only| !only.split(',').any(|o| circuit.label == o)) {
            continue;
        }
        for class in classes() {
            for seed in seeds() {
                let Some(setup) = load(circuit, &class, seed) else {
                    continue;
                };
                let mut race = race::Race::start(setup);
                if std::env::var_os("OAG_ROUTE_PROBE_RING").is_some()
                    && std::env::var_os("OAG_ROUTE_PROBE_FLOOR").is_none()
                {
                    let probe = race.probe_ring_plan(var_u32("OAG_ROUTE_PROBE_STUCK"));
                    report_route(
                        &race,
                        &circuit.label,
                        &class,
                        seed,
                        usize::MAX,
                        &probe,
                        dump.as_deref(),
                    );
                    continue;
                }
                for k in 0..race.route_count() {
                    let at = std::env::var("OAG_ROUTE_PROBE_STAGES")
                        .ok()
                        .and_then(|t| t.parse().ok());
                    let Some(probe) =
                        race.probe_route_plan(k, at, var_u32("OAG_ROUTE_PROBE_STUCK"))
                    else {
                        continue;
                    };
                    report_route(
                        &race,
                        &circuit.label,
                        &class,
                        seed,
                        k,
                        &probe,
                        dump.as_deref(),
                    );
                }
            }
        }
    }
}

fn where_on(probe: &race::RouteProbe, index: u32) -> String {
    let i = index as usize;
    if i < probe.kept {
        // A racing craft moves onto the route's line 320 samples short of the
        // split (`routes::SWITCH_SAMPLES`, or later at the pre-fork path's
        // start), so only that lead-in is a stretch the race drives on it.
        let zone = if i + 320 >= probe.kept {
            "lead-in"
        } else {
            "ring"
        };
        format!("{zone} {}", (probe.merge + i) % probe.ring_len.max(1))
    } else {
        format!("route {}", i - probe.kept)
    }
}

fn report_route(
    race: &race::Race,
    label: &str,
    class: &str,
    seed: u64,
    k: usize,
    probe: &race::RouteProbe,
    dump: Option<&str>,
) {
    let r = &probe.report;
    let unresolved: Vec<String> = r
        .unresolved
        .iter()
        .map(|&(i, f)| format!("{i}({}) {f:?}", where_on(probe, i)))
        .collect();
    emit(&format!(
        "route\t{label}\t{class}\t{seed}\troute={k}\tlen={}\tkept={}\tmerge={}\tsteps={}\tpasses={}\tlowerings={}\trespawns={}\tverify_failures={}\tverify_contacts={}\tverify_respawns={}\tlap={:?}\tunresolved=[{}]",
        probe.line.len(),
        probe.kept,
        probe.merge,
        r.steps,
        r.passes,
        r.lowerings,
        r.respawns,
        r.verify_failures,
        r.verify_contacts,
        r.verify_respawns,
        r.verify_lap_ticks,
        unresolved.join(", ")
    ));
    let n = probe.line.len() as i64;
    for f in probe.rows.iter().filter(|f| f.row.failure.is_some()) {
        emit(&format!(
            "fail\t{label}\t{class}\t{seed}\troute={k}\ttick={}\tlap={}\tidx={}({})\t{:?}\tspeed={:.1}\ttarget={:.1}\tceiling={:.1}\thold={}\tthrust={:.2}\tbrake={:.2}\toffset={:.2}\theight={:.2}\tair={}\tclimb={:.2}",
            f.row.tick,
            if f.progress >= n { 2 } else { 1 },
            f.row.index,
            where_on(probe, f.row.index),
            f.row.failure,
            f.row.speed,
            f.target,
            f.ceiling,
            u8::from(f.hold),
            f.thrust,
            f.brake,
            f.row.offset,
            f.row.height,
            u8::from(f.row.airborne),
            f.row.climb,
        ));
    }
    if let Some(restep) = &probe.stages {
        let (stages, wall) = (&restep.stages, &restep.wall);
        emit(&format!(
            "stage\t{label}\t{class}\t{seed}\troute={k}\tmag_lock={:?}\tescape={:?}\ttrack_sample={:?}\tmag_contact={:?}",
            restep.mag_lock, restep.escape, restep.track_sample, restep.mag_contact
        ));
        for stage in stages {
            emit(&format!(
                "stage\t{label}\t{class}\t{seed}\troute={k}\t{}\tposition={:?}\tvelocity={:?}",
                stage.name, stage.position, stage.velocity
            ));
        }
        emit(&format!(
            "stage\t{label}\t{class}\t{seed}\troute={k}\twall={wall:?}"
        ));
        if let Some(start) = stages.first() {
            use oag_physics::{Ray, Raycaster};
            let along = start.velocity.normalize_or_zero();
            let mut out = [None; 8];
            let count = race.collision().raycast_all(
                Ray::new(start.position, along, 30.0),
                None,
                true,
                &mut out,
            );
            for hit in out[..count].iter().flatten() {
                emit(&format!(
                    "stage\t{label}\t{class}\t{seed}\troute={k}\tahead\tdistance={:.3}\tpoint={:?}\tnormal={:?}\tsurface={:?}",
                    hit.distance, hit.point, hit.normal, hit.surface
                ));
            }
        }
    }
    if let Some((from, to)) = std::env::var("OAG_ROUTE_PROBE_FLOOR")
        .ok()
        .and_then(|r| {
            r.split_once("..")
                .map(|(a, b)| (a.parse().ok(), b.parse().ok()))
        })
        .and_then(|(a, b): (Option<usize>, Option<usize>)| a.zip(b))
    {
        floor_along(race, label, k, probe, from, to);
    }
    if let Some(dir) = dump {
        let name = format!(
            "{dir}/{}-{class}-{seed}-route{k}.tsv",
            label.replace('/', "_")
        );
        let mut text = String::from(
            "tick\tprogress\tindex\twhere\tspeed\ttarget\tceiling\thold\tthrust\tbrake\toffset\theight\tleft\tright\tsteer\tcontact\tair\tclimb\tpitch\tfailure\n",
        );
        for f in &probe.rows {
            text.push_str(&format!(
                "{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.3}\t{}\t{}\t{:.2}\t{:.3}\t{:?}\n",
                f.row.tick,
                f.progress,
                f.row.index,
                where_on(probe, f.row.index),
                f.row.speed,
                f.target,
                f.ceiling,
                u8::from(f.hold),
                f.thrust,
                f.brake,
                f.row.offset,
                f.row.height,
                f.row.left,
                f.row.right,
                f.row.steer,
                u8::from(f.row.contact),
                u8::from(f.row.airborne),
                f.row.climb,
                f.row.pitch,
                f.row.failure,
            ));
        }
        let _ = std::fs::write(name, text);
    }
}

/// Every surface under and over route `k`'s line points `from..to` (line
/// indices): a cast from 30 units above to 30 below, per hit its height
/// against the line point, surface and normal's Y.
fn floor_along(
    race: &race::Race,
    label: &str,
    k: usize,
    probe: &race::RouteProbe,
    from: usize,
    to: usize,
) {
    use oag_core::math::Vec3;
    use oag_physics::{Ray, Raycaster};
    for i in from..to.min(probe.line.len()) {
        let p = probe.line.point(i);
        let mut out = [None; 8];
        let count = race.collision().raycast_all(
            Ray::new(p + Vec3::Y * 30.0, -Vec3::Y, 60.0),
            None,
            true,
            &mut out,
        );
        if std::env::var_os("OAG_ROUTE_PROBE_ACROSS").is_some() {
            across(race, label, k, probe, i);
            continue;
        }
        let hits: Vec<String> = out[..count]
            .iter()
            .flatten()
            .map(|h| {
                format!(
                    "{:+.2}:{:?}:ny{:.2}:n({:.2},{:.2})",
                    h.point.y - p.y,
                    h.surface,
                    h.normal.y,
                    h.normal.x,
                    h.normal.z
                )
            })
            .collect();
        if std::env::var_os("OAG_ROUTE_PROBE_RING").is_some() {
            let ring = race.racing_line();
            let (j, d) = (0..ring.len())
                .map(|j| (j, ring.point(j).distance(p)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap_or((0, f32::NAN));
            let q = ring.point(j);
            let ring_path = race
                .ai_sample_index(j)
                .and_then(|s| race.spline().path_of(s));
            let route_path = probe
                .order
                .get(i)
                .and_then(|&s| race.spline().path_of(s as usize));
            emit(&format!(
                "ring\t{label}\troute={k}\t{i}({})\tpaths route {route_path:?} ring {ring_path:?}\tnearest ring {j}\tdistance={d:.1}\tdy={:.1}\tring=({:.1},{:.1},{:.1})",
                where_on(probe, i as u32),
                q.y - p.y,
                q.x,
                q.y,
                q.z
            ));
            continue;
        }
        let spline_index = probe.order.get(i).map_or(0, |&s| s as usize);
        let sample = race.spline().sample(spline_index);
        emit(&format!(
            "floor\t{label}\troute={k}\t{i}({})\tspline={spline_index}\tpath={:?}\tsample_y={:.1}\twidth={:?}\tai={:?}\tline_offset={:?}\tsection={:?}\tflags={:?}\tp=({:.1},{:.1},{:.1})\t{}",
            where_on(probe, i as u32),
            race.spline().path_of(spline_index),
            sample.map_or(f32::NAN, |s| s.pos[1]),
            sample.map(|s| (s.half_width_left, s.half_width_right)),
            sample.map(|s| (s.ai_bound_left, s.ai_bound_right)),
            sample.map(|s| s.racing_line),
            sample.map(|s| s.section_id),
            sample.map(|s| s.flags),
            p.x,
            p.y,
            p.z,
            hits.join(" ")
        ));
    }
}

/// Line point `i` of route `k`, across its corridor every 2 units from -30 to
/// +30: per offset, the first surface a cast from 6 units above the line down
/// to 2.5 below meets (the hull's reach; the road is 3 below the line), as
/// `W` (wall), `M` (magstrip), `F` (floor), `R` (reset) or `.` (none).
fn across(race: &race::Race, label: &str, k: usize, probe: &race::RouteProbe, i: usize) {
    use oag_core::math::Vec3;
    use oag_physics::{Ray, Raycaster, Surface};
    let p = probe.line.point(i);
    let lateral = probe
        .line
        .aim(i, 0.0)
        .corridor
        .map_or(Vec3::X, |f| f.lateral);
    let mut row = String::new();
    for o in (-30..=30).step_by(2) {
        let q = p + lateral * o as f32;
        let hit = race
            .collision()
            .raycast(Ray::new(q + Vec3::Y * 6.0, -Vec3::Y, 8.5), None, true);
        row.push(match hit.map(|h| h.surface) {
            Some(Surface::Wall) => 'W',
            Some(Surface::MagFloor) => 'M',
            Some(Surface::Floor) => 'F',
            Some(Surface::Reset) => 'R',
            None => '.',
        });
    }
    emit(&format!(
        "across\t{label}\troute={k}\t{i}({})\t{row}",
        where_on(probe, i as u32)
    ));
}

fn var_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok().and_then(|t| t.parse().ok())
}

/// One craft alone in slot 0 on the player's autopilot, which flies the
/// original's driver (`oag_ai::original`) unless `OAG_ROUTE_BOARD_OURS` is
/// set, sent down route `OAG_ROUTE_PROBE_ROUTE` (default 1, the first route)
/// at every visit to its fork. Every `OAG_ROUTE_PROBE_EVERY` ticks (default 10)
/// while on the route it prints the line index, the craft's offset across and
/// height over its line point, speed, and the respawn count.
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 OAG_ROUTE_BOARD_ONLY=pulse/07_Track \
///   OAG_ROUTE_BOARD_CLASSES=VENOM OAG_ROUTE_BOARD_SEEDS=1 \
///   cargo nextest run --release -p oag-raceplay --run-ignored all \
///   --no-capture route_original_probe
/// ```
#[test]
#[ignore = "a probe over forked circuits: OAG_SWEEP=1, needs the discs"]
fn route_original_probe() {
    if !sweeping() {
        return;
    }
    let circuits = probe_circuits();
    let want = var_u32("OAG_ROUTE_PROBE_ROUTE").map_or(1, |r| r as u16);
    let every = var_u32("OAG_ROUTE_PROBE_EVERY").unwrap_or(10);
    for circuit in &circuits {
        if only().is_some_and(|only| !only.split(',').any(|o| circuit.label == o)) {
            continue;
        }
        for class in classes() {
            for seed in seeds() {
                let Some(setup) = load(circuit, &class, seed) else {
                    continue;
                };
                let mut race = race::Race::start(setup);
                for other in 1..race.ship_count() as usize {
                    race.sim.world.ships[other].active = false;
                }
                if std::env::var_os("OAG_ROUTE_BOARD_OURS").is_some() {
                    fly_ours(&mut race);
                } else {
                    race.set_autopilot(true);
                }
                let Some(course) = race.course().cloned() else {
                    continue;
                };
                let Some(pre_fork) = course
                    .routes()
                    .get(usize::from(want - 1))
                    .map(|r| r.pre_fork)
                else {
                    continue;
                };
                let label = &circuit.label;
                for tick in 0..SOLO_TICKS {
                    {
                        let branching = &mut race.sim.world.ships[0].driver.branching;
                        if branching.route == 0 && branching.decided_at == pre_fork + 1 {
                            branching.pending = want;
                        }
                    }
                    race.tick(&PlayerInputs::none());
                    let ship = &race.sim.world.ships[0];
                    let route = ship.driver.branching.route;
                    if route == 0 || tick % every != 0 {
                        continue;
                    }
                    let index = ship.driver.index as usize;
                    let line = race.line_of(0);
                    let p = line.point(index);
                    let lateral = line
                        .aim(index, 0.0)
                        .corridor
                        .map_or(oag_core::math::Vec3::X, |f| f.lateral);
                    let b = &ship.physics.body;
                    emit(&format!(
                        "original\t{label}\t{class}\t{seed}\ttick={tick}\troute={route}\tindex={index}\toffset={:.1}\theight={:.1}\tspeed={:.1}\tair={}\trespawns={}\tlap={}\tstate={:?}",
                        (b.position - p).dot(lateral),
                        b.position.y - p.y,
                        b.linear_velocity.length(),
                        u8::from(ship.physics.time_airborne > 0.0),
                        race.respawns_of(0),
                        ship.standing.lap,
                        ship.physics.craft_state,
                    ));
                }
            }
        }
    }
}

/// The board's circuits plus Omega's re-shipped `cathedral` and `square`, for the probes
/// only (the board's own list, and so its run time, is unchanged).
fn probe_circuits() -> Vec<Circuit> {
    let mut circuits = pulse_circuits();
    circuits.extend(circuits_omega());
    if let Some(source) = oag_testdata::exact("data/extracted/ps4") {
        for name in ["cathedral", "square"] {
            circuits.push(Circuit {
                label: format!("omega/{name}"),
                source: source.display().to_string(),
                track: format!(r"Data\environments2048\{name}\track.vex"),
            });
        }
    }
    circuits.extend(circuits_2048());
    circuits
}
