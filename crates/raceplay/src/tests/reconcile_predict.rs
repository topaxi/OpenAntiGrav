//! Section 3a's M1: how predictable a remote craft's input is, measured as what
//! predicting it with its last known input costs in corrections and in position.
//!
//! **The model.** A server runs the race on the true inputs. A client runs the
//! same race ahead: its own craft on the true input, every remote craft on that
//! craft's last input it has heard of, which is `window` ticks old (a remote input
//! reaches the client `window` ticks after the server used it). Every tick a packet
//! arrives carrying the server's state after tick `t - window + 1`; the client
//! compares it with what it had at that tick and, under the policy, restores the
//! server's state and replays the `window` ticks since through
//! [`Race::tick_sim_only`]. Both sides are in this process, so no transport is
//! simulated: only the latency, as a count of ticks, and a lossless link.
//!
//! **Two policies.** `every packet` corrects whenever the state differs at all (the
//! state hash), the strict reading of "roll back on a mismatch". `threshold d`
//! corrects only when some craft is more than `d` world units from the server's
//! (Rocket League's "large difference requires correction"); the `d` values are
//! **chosen, not measured**, picked to bracket a craft's own size.
//!
//! **Where the input comes from.** Not from a person, in the packs: no
//! recording of eight people flying one circuit exists. The packs' streams are the
//! driver's controls recorded off an all-autopilot run and replayed as
//! [`InputSnapshot`]s (`record_pack`), which is a smoother and more reactive
//! stream than a stick in a hand. The ghost files the maintainer saved are the only
//! human input there is; they are replayed alone on their circuit and as one more
//! craft in a pack.
//!
//! **Contact** is a craft pair `oag_physics::pair::resolve` resolved on the server's
//! tick (`Race::pair_contacts`, recorded for tests only); a remote craft's error
//! counts as "in contact" from that tick until `window` ticks later, the span in
//! which a stale guess about it could matter.
//!
//! **What it leaves out.** Weapons: a replayed stream never presses fire, so a pack
//! here has contact and walls but no missiles, which are section 3a's own open
//! question (option D). Packet loss, jitter and a server-side input buffer.
//!
//! A scratch sweep: `#[ignore]`d and a no-op without `OAG_SWEEP`, so `just
//! test-data` never pays for it. `OAG_SWEEP_TRACKS` limits the circuits:
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
//!     --run-ignored all --no-capture -E 'test(/reconcile_predict::/)'
//! ```
//!
//! `OAG_SWEEP_GHOSTS` names a directory of `.oagr` ghost files for the human half.
//! The harness itself is pinned by the ordinary tests at the bottom, on the fixture
//! grid.

use super::*;
use oag_gameplay::input::{Button, Input};
use oag_gameplay::{Controller, InputSnapshot, PlayerInputs};
use std::collections::VecDeque;

/// Ticks a correction replays, and the age of the input a remote craft is predicted
/// with: 50, 100 and 200 ms at 60 Hz, section 3a's windows.
const WINDOWS: [usize; 3] = [3, 6, 12];

/// Threshold policy distances in world units. **Chosen, not measured.**
const THRESHOLDS: [f32; 3] = [0.25, 1.0, 4.0];

/// Ticks raced after the countdown releases.
const RACED: usize = 1500;

/// How a client decides a packet needs a correction.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Policy {
    /// The server's state differs from the client's at all.
    Every,
    /// Some craft is more than this many units from where the server has it.
    Threshold(f32),
}

impl Policy {
    fn label(self) -> String {
        match self {
            Self::Every => "every packet".to_string(),
            Self::Threshold(d) => format!("threshold {d}"),
        }
    }
}

/// Inputs for every slot over a whole run, and which slots are remote.
pub(super) struct Scenario {
    pub(super) name: String,
    pub(super) streams: Vec<Vec<InputSnapshot>>,
    pub(super) remote: Vec<usize>,
}

impl Scenario {
    fn ticks(&self) -> usize {
        self.streams.iter().map(Vec::len).min().unwrap_or(0)
    }

    /// What the server hands every slot at tick `t`.
    pub(super) fn truth(&self, t: usize) -> PlayerInputs {
        let mut inputs = PlayerInputs::none();
        for (slot, stream) in self.streams.iter().enumerate() {
            inputs.set(slot, stream[t]);
        }
        inputs
    }

    /// What the client hands every slot at tick `t` when the remote ones are only
    /// known up to tick `known` (`None`: not at all).
    pub(super) fn guess(&self, t: usize, known: Option<usize>) -> PlayerInputs {
        let mut inputs = PlayerInputs::none();
        for (slot, stream) in self.streams.iter().enumerate() {
            let snapshot = if self.remote.contains(&slot) {
                known.map_or(InputSnapshot::EMPTY, |u| held_only(&stream[u]))
            } else {
                stream[t]
            };
            inputs.set(slot, snapshot);
        }
        inputs
    }
}

/// The last known snapshot, held: the same stick and brakes, the same buttons held,
/// and no edge, because a prediction is "nothing changed", not "pressed again".
fn held_only(last: &InputSnapshot) -> InputSnapshot {
    let mut buttons = Input::new();
    let held = last.buttons.masks()[0];
    buttons.begin_frame(held);
    buttons.begin_frame(held);
    InputSnapshot { buttons, ..*last }
}

/// A race with every grid slot flown by an input stream, none by the AI.
fn person_race(setup: &Setup) -> Race {
    let mut race = Race::start(setup.clone());
    race.set_control_scheme(oag_gameplay::ControlScheme::Veteran);
    for slot in 0..race.sim.world.ship_count as usize {
        race.sim.world.controllers[slot] = Controller::Remote;
    }
    race
}

/// What the driver asks of every craft, recorded as input streams, and how many
/// recorded ticks had a thrust that was neither 0 nor 1.
///
/// Every slot is made a person-flown slot with the Autopilot pickup's timer left
/// running, so [`Race::tick`] takes its controls from the driver and
/// `last_controls` is where they are read back. The thrust is binarised the way a
/// button is (`ship_controls` reads Cross held), and is held through the countdown
/// the way a player holds it, because the countdown's own gate zeroes the recorded
/// value and a stream that pressed Cross on the release tick would hand every craft
/// the same first mispredict.
fn record_pack(setup: &Setup, ticks: usize) -> (Vec<Vec<InputSnapshot>>, usize) {
    let mut race = Race::start(setup.clone());
    let slots = race.sim.world.ship_count as usize;
    for slot in 0..slots {
        race.sim.world.controllers[slot] = Controller::Remote;
        race.sim.world.ships[slot].autopilot_timer = 1.0e9;
    }
    let mut controls = vec![Vec::new(); slots];
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        race.drain_cues();
        for (slot, log) in controls.iter_mut().enumerate() {
            log.push(race.last_controls[slot]);
        }
    }
    let release = (0..ticks as u64)
        .find(|&tick| !oag_race::RaceState::thrust_gated(tick))
        .unwrap_or(0) as usize;
    let mut fractional = 0;
    let streams = controls
        .iter()
        .map(|log| {
            let mut input = Input::new();
            let first = log.get(release).map_or(0.0, |c| c.thrust);
            log.iter()
                .enumerate()
                .map(|(tick, c)| {
                    let thrust = if tick < release { first } else { c.thrust };
                    if thrust > 0.0 && thrust < 1.0 {
                        fractional += 1;
                    }
                    input.begin_frame(if thrust > 0.5 { Button::Cross.bit() } else { 0 });
                    InputSnapshot {
                        buttons: input,
                        stick_x: c.steer_x,
                        stick_y: -c.steer_y,
                        airbrake_left: c.airbrake_left,
                        airbrake_right: c.airbrake_right,
                    }
                    .sanitised()
                })
                .collect()
        })
        .collect();
    (streams, fractional)
}

/// Where every craft is, for comparing two sims.
type Positions = [Vec3; oag_gameplay::MAX_SHIPS];

fn positions(race: &Race) -> Positions {
    std::array::from_fn(|slot| race.sim.world.ships[slot].physics.body.position)
}

fn farthest(a: &Positions, b: &Positions, ships: usize) -> f32 {
    (0..ships)
        .map(|i| (a[i] - b[i]).length())
        .fold(0.0, f32::max)
}

/// Everything one (scenario, window, policy) run counted.
#[derive(Default)]
struct Outcome {
    corrections: usize,
    replayed: usize,
    scored: usize,
    contact_ticks: usize,
    remote_free: Vec<f32>,
    remote_contact: Vec<f32>,
    local: Vec<f32>,
    jumps: Vec<f32>,
    /// Corrections after which the client's state hash was not the server's at the
    /// tick the packet described. Always zero unless the replay is broken.
    bad_corrections: usize,
}

/// One client against one server over the scenario. Counts only ticks from
/// `scored_from` on, so the countdown's grid is not in the numbers.
fn run(
    build: fn(&Setup) -> Race,
    setup: &Setup,
    sc: &Scenario,
    window: usize,
    policy: Policy,
    scored_from: usize,
) -> Outcome {
    let mut server = build(setup);
    let mut client = build(setup);
    let ships = server.sim.world.ship_count as usize;
    let mut out = Outcome::default();
    let mut server_hist: Vec<(Positions, u64)> =
        vec![(positions(&server), server.sim.state_hash())];
    let mut client_hist = server_hist.clone();
    let mut ring: VecDeque<RaceSim> = VecDeque::new();
    ring.push_back(server.snapshot_sim());
    let mut contact_last = vec![i64::MIN / 2; ships];

    for t in 0..sc.ticks() {
        server.tick(&sc.truth(t));
        server.drain_cues();
        let touching = !server.pair_contacts.is_empty();
        for &(a, b) in &server.pair_contacts {
            contact_last[a] = t as i64;
            contact_last[b] = t as i64;
        }
        server_hist.push((positions(&server), server.sim.state_hash()));
        ring.push_back(server.snapshot_sim());
        if ring.len() > window + 1 {
            ring.pop_front();
        }

        client.tick_sim_only(&sc.guess(t, t.checked_sub(window + 1)));
        client_hist.push((positions(&client), client.sim.state_hash()));

        if let Some(u) = t.checked_sub(window) {
            let (server_pos, server_hash) = server_hist[u + 1];
            let (client_pos, client_hash) = client_hist[u + 1];
            let needed = match policy {
                Policy::Every => server_hash != client_hash,
                Policy::Threshold(d) => farthest(&server_pos, &client_pos, ships) > d,
            };
            if needed {
                let before = positions(&client);
                client.restore_sim(&ring[0]);
                if client.sim.state_hash() != server_hash {
                    out.bad_corrections += 1;
                }
                for v in (u + 1)..=t {
                    client.tick_sim_only(&sc.guess(v, Some(u)));
                    client_hist[v + 1] = (positions(&client), client.sim.state_hash());
                }
                if t >= scored_from {
                    out.corrections += 1;
                    out.replayed += window;
                    out.jumps
                        .push(farthest(&before, &positions(&client), ships));
                }
            }
        }

        if t >= scored_from {
            out.scored += 1;
            out.contact_ticks += usize::from(touching);
            let (server_pos, _) = server_hist[t + 1];
            let (client_pos, _) = client_hist[t + 1];
            for slot in 0..ships {
                let error = (server_pos[slot] - client_pos[slot]).length();
                if !sc.remote.contains(&slot) {
                    out.local.push(error);
                } else if t as i64 - contact_last[slot] <= window as i64 {
                    out.remote_contact.push(error);
                } else {
                    out.remote_free.push(error);
                }
            }
        }
    }
    out
}

fn quantile(samples: &mut [f32], q: f32) -> f32 {
    if samples.is_empty() {
        return f32::NAN;
    }
    samples.sort_by(f32::total_cmp);
    samples[((samples.len() - 1) as f32 * q).round() as usize]
}

/// `p50 / p95 / max` of a sample set.
fn spread3(samples: &mut [f32]) -> String {
    format!(
        "{:.2}/{:.2}/{:.2}",
        quantile(samples, 0.5),
        quantile(samples, 0.95),
        quantile(samples, 1.0)
    )
}

/// What the remote streams are made of, and how often the last input is not the
/// next `window` inputs: the strict-policy correction rate before any physics. Both
/// count from the countdown's release, the same ticks the run scores.
fn stream_report(sc: &Scenario) {
    let after = oag_race::COUNTDOWN_TICKS as usize;
    let mut sticks: Vec<u32> = sc
        .remote
        .iter()
        .flat_map(|&slot| {
            sc.streams[slot][after..]
                .iter()
                .map(|s| s.stick_x.to_bits())
        })
        .collect();
    let total = sticks.len().max(1);
    let full = sticks
        .iter()
        .filter(|&&bits| f32::from_bits(bits).abs() == 1.0)
        .count();
    sticks.sort_unstable();
    sticks.dedup();
    println!(
        "  {} input: {} distinct stick_x values over {total} ticks, {:.1}% of them at full deflection",
        sc.name,
        sticks.len(),
        100.0 * full as f64 / total as f64
    );
    for &window in &WINDOWS {
        let (mut changed, mut total, mut moved, mut delta) = (0usize, 0usize, 0usize, 0.0f64);
        for &slot in &sc.remote {
            let stream = &sc.streams[slot];
            for u in oag_race::COUNTDOWN_TICKS as usize..stream.len().saturating_sub(window + 1) {
                let last = &stream[u];
                let differs = |s: &InputSnapshot| {
                    s.stick_x != last.stick_x
                        || s.stick_y != last.stick_y
                        || s.airbrake_left != last.airbrake_left
                        || s.airbrake_right != last.airbrake_right
                        || s.buttons.masks()[0] != last.buttons.masks()[0]
                };
                total += 1;
                changed += usize::from(stream[u + 1..=u + window].iter().any(differs));
                moved += usize::from(differs(&stream[u + 1]));
                delta += f64::from((stream[u + 1].stick_x - last.stick_x).abs());
            }
        }
        println!(
            "  {} input, window {window}: the next {window} inputs differ from the last {:.1}% of ticks; \
             the very next differs {:.1}%; mean |dstick_x| {:.4}",
            sc.name,
            100.0 * changed as f64 / total.max(1) as f64,
            100.0 * moved as f64 / total.max(1) as f64,
            delta / total.max(1) as f64
        );
    }
}

/// Outcomes pooled across circuits, keyed by (scenario, window, policy).
#[derive(Default)]
struct Pool {
    rows: Vec<(String, usize, Policy, Vec<Outcome>)>,
}

impl Pool {
    fn add(&mut self, name: &str, window: usize, policy: Policy, outcome: Outcome) {
        if let Some(row) = self
            .rows
            .iter_mut()
            .find(|r| r.0 == name && r.1 == window && r.2 == policy)
        {
            row.3.push(outcome);
        } else {
            self.rows
                .push((name.to_string(), window, policy, vec![outcome]));
        }
    }

    fn print(&mut self) {
        println!(
            "scenario | window | policy | corrections/s mean (min-max over runs) | replayed ticks/s | \
             sim cost x | remote err free p50/p95/max | remote err in contact p50/p95/max | \
             own err p50/p95/max | correction pop p50/p95/max | contact ticks % | bad corrections"
        );
        for (name, window, policy, outcomes) in &mut self.rows {
            let rates: Vec<f64> = outcomes
                .iter()
                .map(|o| o.corrections as f64 * 60.0 / o.scored.max(1) as f64)
                .collect();
            let replayed: f64 = outcomes
                .iter()
                .map(|o| o.replayed as f64 * 60.0 / o.scored.max(1) as f64)
                .sum::<f64>()
                / outcomes.len() as f64;
            let mean = rates.iter().sum::<f64>() / rates.len() as f64;
            let low = rates.iter().copied().fold(f64::MAX, f64::min);
            let high = rates.iter().copied().fold(0.0, f64::max);
            let pool = |pick: fn(&Outcome) -> &Vec<f32>| -> Vec<f32> {
                outcomes.iter().flat_map(|o| pick(o).clone()).collect()
            };
            let mut free = pool(|o| &o.remote_free);
            let mut contact = pool(|o| &o.remote_contact);
            let mut own = pool(|o| &o.local);
            let mut jumps = pool(|o| &o.jumps);
            let contact_pct = outcomes.iter().map(|o| o.contact_ticks).sum::<usize>() as f64
                * 100.0
                / outcomes.iter().map(|o| o.scored).sum::<usize>().max(1) as f64;
            let bad: usize = outcomes.iter().map(|o| o.bad_corrections).sum();
            println!(
                "{name} | {window} | {} | {mean:.2} ({low:.2}-{high:.2}) | {replayed:.1} | {:.2} | \
                 {} | {} | {} | {} | {contact_pct:.1} | {bad}",
                policy.label(),
                1.0 + replayed / 60.0,
                spread3(&mut free),
                spread3(&mut contact),
                spread3(&mut own),
                spread3(&mut jumps),
            );
        }
    }
}

fn sweep_scenario(setup: &Setup, sc: &Scenario, pool: &mut Pool) {
    let scored_from = oag_race::COUNTDOWN_TICKS as usize;
    for &window in &WINDOWS {
        for policy in std::iter::once(Policy::Every).chain(THRESHOLDS.map(Policy::Threshold)) {
            pool.add(
                &sc.name,
                window,
                policy,
                run(person_race, setup, sc, window, policy, scored_from),
            );
        }
    }
}

fn load_circuit(
    image: &std::path::Path,
    entry: &str,
    class: &str,
    mode: Mode,
    team: Option<String>,
    seed: Option<u64>,
) -> Option<Setup> {
    crate::load(&crate::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode,
        track: Some(entry.to_string()),
        team,
        seed,
        ..crate::Options::default()
    })
    .ok()
    .map(|loaded| loaded.setup)
}

fn wanted(id: &str) -> bool {
    std::env::var("OAG_SWEEP_TRACKS").map_or(true, |only| only.contains(id))
}

fn loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn reconcile_predict_ai_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    println!("load at start: {}", loadavg());
    let mut pool = Pool::default();
    let ticks = oag_race::COUNTDOWN_TICKS as usize + RACED;
    for (id, entry) in super::reconcile_cost::circuits() {
        if !wanted(&id) {
            continue;
        }
        let setup = load_circuit(&image, &entry, "VENOM", Mode::SingleRace, None, None)
            .expect("loading the circuit");
        let (streams, fractional) = record_pack(&setup, ticks);
        let slots = streams.len();
        let hull = Race::start(setup.clone()).sim.world.ships[0]
            .handling
            .dimensions;
        println!("{id}: craft 0 hull {:?}", hull);
        println!(
            "{id}: {slots} streams of {ticks} ticks, {fractional} fractional-thrust ticks binarised; load {}",
            loadavg()
        );
        let seven = Scenario {
            name: "ai7 (1 own, 7 remote)".to_string(),
            streams: streams.clone(),
            remote: (1..slots).collect(),
        };
        stream_report(&seven);
        sweep_scenario(&setup, &seven, &mut pool);
        let one = Scenario {
            name: "ai1 (7 own, 1 remote)".to_string(),
            streams,
            remote: vec![1],
        };
        sweep_scenario(&setup, &one, &mut pool);
    }
    pool.print();
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else if path.extension().is_some_and(|ext| ext == "oagr") {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1 and OAG_SWEEP_GHOSTS, read the table"]
fn reconcile_predict_human_sweep() {
    let (Some(_), Some(dir)) = (
        std::env::var_os("OAG_SWEEP"),
        std::env::var_os("OAG_SWEEP_GHOSTS"),
    ) else {
        return;
    };
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    println!("load at start: {}", loadavg());
    let mut alone = Pool::default();
    let mut packed = Pool::default();
    for path in walk(std::path::Path::new(&dir)) {
        let replay = oag_replay::Replay::from_bytes(&std::fs::read(&path).expect("reading"))
            .expect("parsing");
        let header = &replay.header;
        let class = header.class.to_uppercase();
        let mode = Mode::from_name(&header.mode).expect("the ghost's mode");
        let stream = replay.inputs[0].clone();
        let label = format!(
            "{} {} {}",
            header.track.rsplit('\\').nth(1).unwrap_or(&header.track),
            header.mode,
            header.class
        );
        println!(
            "{label}: {} ticks of human input; load {}",
            stream.len(),
            loadavg()
        );
        let human = Scenario {
            name: format!("human alone: {label}"),
            streams: vec![stream.clone()],
            remote: vec![0],
        };
        stream_report(&human);
        let team = Some(header.team.clone());
        let Some(setup) = load_circuit(
            &image,
            &header.track,
            &class,
            mode,
            team.clone(),
            Some(header.seed),
        ) else {
            println!("  could not load {}", header.track);
            continue;
        };
        sweep_scenario(&setup, &human, &mut alone);

        let Some(pack) = load_circuit(
            &image,
            &header.track,
            &class,
            Mode::SingleRace,
            team,
            Some(header.seed),
        ) else {
            continue;
        };
        let (mut streams, _) = record_pack(&pack, stream.len());
        streams[0] = stream;
        let mixed = Scenario {
            name: format!("human in a pack: {label}"),
            streams,
            remote: vec![0],
        };
        sweep_scenario(&pack, &mixed, &mut packed);
    }
    alone.print();
    packed.print();
}

/// [`person_race`] with the off-track rescue off, so a craft that wanders off the
/// fixture's short straight is not teleported back and hidden behind a respawn.
pub(super) fn held_to_the_track(setup: &Setup) -> Race {
    without_player_rescue(person_race(setup))
}

pub(super) fn fixture() -> Setup {
    let mut handling = hulled_handling();
    handling.engine.amount = 20.0;
    handling.engine.accelcap = 1000.0;
    let mut setup = setup(handling);
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    setup
}

/// Eight streams of `ticks`, every one with Cross held when `thrust(slot, tick)`
/// (thrust moves a craft's position, which a short fixture straight's steering does
/// not).
pub(super) fn fixture_streams(
    ticks: usize,
    thrust: impl Fn(usize, usize) -> bool,
) -> Vec<Vec<InputSnapshot>> {
    (0..8)
        .map(|slot| {
            let mut input = Input::new();
            (0..ticks)
                .map(|tick| {
                    input.begin_frame(if thrust(slot, tick) {
                        Button::Cross.bit()
                    } else {
                        0
                    });
                    InputSnapshot {
                        buttons: input,
                        ..InputSnapshot::default()
                    }
                })
                .collect()
        })
        .collect()
}

/// A craft whose input never changes is predicted exactly: no packet ever needs a
/// correction once the first window has been heard, under either policy. If the
/// harness corrected anyway, every number the sweep prints would be noise.
#[test]
fn a_remote_craft_that_holds_still_is_never_corrected() {
    let sc = Scenario {
        name: "steady".to_string(),
        streams: fixture_streams(420, |_, _| true),
        remote: (1..8).collect(),
    };
    for policy in [Policy::Every, Policy::Threshold(0.0)] {
        let out = run(held_to_the_track, &fixture(), &sc, 6, policy, 300);
        assert_eq!(out.corrections, 0, "{policy:?}");
        assert!(out.scored > 100);
        assert!(
            sc.ticks() > oag_race::COUNTDOWN_TICKS as usize + 100,
            "the craft must be released"
        );
    }
}

/// A craft that changes its stick is mispredicted, every correction lands on the
/// server's exact state (`bad_corrections` counts the ones that do not), and a
/// threshold no craft can reach never corrects at all.
#[test]
fn a_changing_remote_craft_is_corrected_onto_the_servers_state() {
    let weave = |slot: usize, tick: usize| (tick / (3 + slot)).is_multiple_of(2);
    let sc = Scenario {
        name: "weave".to_string(),
        streams: fixture_streams(420, weave),
        remote: (1..8).collect(),
    };
    let every = run(held_to_the_track, &fixture(), &sc, 6, Policy::Every, 300);
    assert!(every.corrections > 5, "{} corrections", every.corrections);
    assert_eq!(every.replayed, every.corrections * 6);
    assert_eq!(
        every.bad_corrections, 0,
        "a correction missed the server's hash"
    );
    let never = run(
        held_to_the_track,
        &fixture(),
        &sc,
        6,
        Policy::Threshold(1.0e9),
        300,
    );
    assert_eq!(never.corrections, 0);
    let total = |out: &Outcome| -> f32 {
        out.remote_free
            .iter()
            .chain(&out.remote_contact)
            .sum::<f32>()
    };
    assert!(
        total(&never) > 100.0,
        "an uncorrected client must drift from the server ({})",
        total(&never)
    );
    assert!(
        total(&every) < total(&never) / 4.0,
        "correcting must bring the client close to the server ({} against {})",
        total(&every),
        total(&never)
    );
}
