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
//! Per circuit and seed (`OAG_ROUTE_BOARD_SEEDS`, default `1,2,3`):
//!
//! - `plan`: per route, whether its reference plan verified, the physics steps
//!   it took (`Race::plan_steps`), the steps each other hull's adaptation took,
//!   and the wall clock of learning every plan of the race.
//! - `solo`: per route (and the ring at the same fork), one craft alone, forced
//!   down it at every visit (the coin's draw overwritten), as slot 0 on the
//!   player's autopilot (the reference hull) and as slot 1 (an adapted hull):
//!   ticks from moving onto the route's line to back on the ring, wall contact
//!   ticks and respawns over that stretch.
//! - `field`: a full field with the coin as it is, per route: traversals, mean
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
        race.set_autopilot(true);
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
        race.tick(&PlayerInputs::none());
        watch(&race, slot, tick, &mut w, &mut tally);
        if tally[usize::from(want)].visits as usize >= VISITS {
            break;
        }
        if race.sim.world.ships[slot].physics.craft_state != oag_physics::CraftState::Racing {
            break;
        }
    }
    tally.swap_remove(usize::from(want))
}

fn field(setup: &race::Setup, plans: &race::Race) -> (Vec<Tally>, Vec<u32>, u32) {
    let mut race = with_plans(setup, plans);
    race.set_autopilot(true);
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
