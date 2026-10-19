//! A server and its clients in one process over [`Loopback`], no disc: the join,
//! the start, and the state hash agreeing on every confirmed tick, with and
//! without datagram loss.

use oag_net::client::{Client, Phase, Stats};
use oag_net::host::Host;
use oag_net::link::{Clock, Conditions, Link, Loopback, Shim};
use oag_net::wire::RaceParams;
use oag_raceplay::{Race, Setup};

/// What a test client flies with: a function of the tick, so every run is the
/// same and the two clients differ.
pub(crate) type Pilot = Box<dyn FnMut(&mut Client, &mut Race) -> oag_gameplay::InputSnapshot>;

/// How a harness lets time pass between rounds.
#[derive(Clone)]
pub(crate) enum Pace {
    /// By sleeping, so a [`Shim`] on the wall clock delays by a number of ticks that depends on
    /// how loaded the machine is.
    Wall(std::time::Duration),
    /// By advancing a [`Clock`] the shims read, a round a `tick` long and never a sleep: the same
    /// inputs arrive on the same tick on any machine, and the run is as fast as the machine is.
    Virtual(Clock, std::time::Duration),
}

impl Pace {
    /// Lets `wait` pass: a handshake step or a drain, not a round.
    fn idle(&self, wait: std::time::Duration) {
        match self {
            Self::Wall(_) => std::thread::sleep(wait),
            Self::Virtual(clock, _) => clock.advance(wait),
        }
    }

    /// Lets one round of the race pass.
    fn round(&self) {
        match self {
            Self::Wall(sleep) if sleep.is_zero() => {}
            Self::Wall(sleep) => std::thread::sleep(*sleep),
            Self::Virtual(clock, tick) => clock.advance(*tick),
        }
    }
}

/// Ticks the host and every client in lockstep with the wall clock left out:
/// each round the host reads its links and runs one tick once racing, and every
/// client steps once. Returns each client's summed stats.
pub(crate) fn drive(
    host: &mut Host,
    clients: &mut [(Client, Box<Race>, Pilot)],
    ticks: u32,
    pace: &Pace,
) -> Vec<Stats> {
    drive_observed(host, clients, ticks, pace, |_| {})
}

/// [`drive`], showing `observe` the host after each of its ticks.
pub(crate) fn drive_observed(
    host: &mut Host,
    clients: &mut [(Client, Box<Race>, Pilot)],
    ticks: u32,
    pace: &Pace,
    mut observe: impl FnMut(&Host),
) -> Vec<Stats> {
    let mut totals = vec![Stats::default(); clients.len()];
    ready_all(host, clients, pace);
    host.start(0);
    while host.tick_number() < ticks {
        host.poll();
        host.tick();
        observe(host);
        for ((client, race, pilot), total) in clients.iter_mut().zip(&mut totals) {
            let own = pilot(client, race);
            client.step(race, &own);
            let s = client.take_stats();
            total.predicted += s.predicted;
            total.confirmed += s.confirmed;
            total.hash_matches += s.hash_matches;
            total.hash_mismatches += s.hash_mismatches;
            total.corrections += s.corrections;
            total.replayed += s.replayed;
            total.largest_pop = total.largest_pop.max(s.largest_pop);
            total.largest_error = total.largest_error.max(s.largest_error);
            total.error_sum += s.error_sum;
            total.error_count += s.error_count;
            total.own_pop_max = total.own_pop_max.max(s.own_pop_max);
            total.own_moved += s.own_moved;
            total.resends += s.resends;
            total.stalls += s.stalls;
            total.sped_up += s.sped_up;
            total.slowed_down += s.slowed_down;
            total.lead = s.lead;
            total.confirm_count += s.confirm_count;
            total.weapon_mismatches += s.weapon_mismatches;
            total.weapon_corrections += s.weapon_corrections;
            total.late_cues += s.late_cues;
            total.late_blasts += s.late_blasts;
            total.late_oldest = total.late_oldest.max(s.late_oldest);
            total.shots_appeared += s.shots_appeared;
            total.shots_vanished += s.shots_vanished;
            total.oldest_appeared = total.oldest_appeared.max(s.oldest_appeared);
            total.own_hit_added += s.own_hit_added;
            total.own_hit_removed += s.own_hit_removed;
            total.confirm_sum_ms += s.confirm_sum_ms;
        }
        pace.round();
    }
    host.end("the test is over");
    // Let the last confirmed ticks arrive.
    for _ in 0..200 {
        let mut behind = false;
        for ((client, race, _), total) in clients.iter_mut().zip(&mut totals) {
            client.step(race, &oag_gameplay::InputSnapshot::EMPTY);
            let s = client.take_stats();
            total.confirmed += s.confirmed;
            total.hash_matches += s.hash_matches;
            total.hash_mismatches += s.hash_mismatches;
            behind |= client.confirmed_tick() < host.tick_number();
        }
        if !behind {
            break;
        }
        host.poll();
        pace.idle(std::time::Duration::from_millis(2));
    }
    totals
}

/// Welcomes every client and waits until the host has them all ready.
pub(crate) fn ready_all(host: &mut Host, clients: &mut [(Client, Box<Race>, Pilot)], pace: &Pace) {
    for (client, race, _) in clients.iter_mut() {
        loop {
            host.poll();
            if client.poll_welcome().expect("welcomed").is_some() {
                break;
            }
            pace.idle(std::time::Duration::from_millis(1));
        }
        client.ready(race).expect("same start state");
    }
    let asked = std::time::Instant::now();
    while !host.all_ready() {
        host.poll();
        for (client, _, _) in clients.iter_mut() {
            client.poll();
        }
        assert!(asked.elapsed().as_secs() < 5, "{:?}", host.seat_names());
        pace.idle(std::time::Duration::from_millis(1));
    }
}

pub(crate) fn params(people: u8) -> RaceParams {
    RaceParams {
        title: String::new(),
        mode: "single_race".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 0xC0FFEE,
        people,
        ..RaceParams::default()
    }
}

pub(crate) fn headless(params: &RaceParams) -> Box<Race> {
    let mut race = Box::new(Race::start(Setup::headless(
        oag_race::Mode::SingleRace,
        params.seed,
    )));
    oag_net::race::prepare(&mut race, params).expect("prepares");
    race
}

pub(crate) fn pilot(phase: u32) -> Pilot {
    Box::new(move |_, race| {
        let tick = u32::try_from(race.sim.world.tick).unwrap_or(0);
        let mut buttons = oag_gameplay::input::Input::new();
        let cross = oag_gameplay::input::Button::Cross.bit();
        buttons.begin_frame(if (tick + phase) % 7 < 4 { cross } else { 0 });
        oag_gameplay::InputSnapshot {
            buttons,
            stick_x: ((tick + phase) % 11) as f32 / 10.0 - 0.5,
            ..oag_gameplay::InputSnapshot::default()
        }
    })
}

fn seated(host: &mut Host, people: u8, loss: f32) -> Vec<(Client, Box<Race>, Pilot)> {
    (0..people)
        .map(|n| {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let link: Box<dyn Link> = if loss > 0.0 {
                Box::new(Shim::new(
                    client_end,
                    Conditions {
                        one_way: web_time::Duration::ZERO,
                        loss,
                        ..Conditions::default()
                    },
                ))
            } else {
                Box::new(client_end)
            };
            let client = Client::connect(link, &format!("client {n}"));
            (client, headless(&params(people)), pilot(u32::from(n) * 3))
        })
        .collect()
}

#[test]
fn two_clients_agree_with_the_server_on_every_confirmed_tick() {
    let params = params(2);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 2, 0.0);
    let totals = drive(
        &mut host,
        &mut clients,
        300,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    for (n, total) in totals.iter().enumerate() {
        assert_eq!(total.hash_mismatches, 0, "client {n}: {total:?}");
        assert_eq!(total.hash_matches, 300, "client {n}: {total:?}");
        assert!(total.predicted >= 300, "client {n}: {total:?}");
        // Dropping the send-time ring leaves this at zero and the overlay's CONF row blank.
        assert!(total.confirm_count > 100, "client {n}: {total:?}");
        assert!(total.confirm_sum_ms > 0.0, "client {n}: {total:?}");
    }
    for (client, _, _) in &clients {
        assert_eq!(client.phase(), &Phase::Over("the test is over".into()));
    }
    // Two different pilots really reached the server: its applied inputs differ.
    let last = host.history().last().expect("ran");
    assert_ne!(last.inputs[0], last.inputs[1]);
}

/// Half the datagrams lost both ways: the server repeats a seat's last input when
/// it has none, the client asks for confirmed ticks the redundancy did not cover,
/// and the hashes still agree on every tick.
#[test]
fn half_the_datagrams_lost_and_the_hashes_still_agree() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 1, 0.5);
    let totals = drive(
        &mut host,
        &mut clients,
        400,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    let total = totals[0];
    assert_eq!(total.hash_mismatches, 0, "{total:?}");
    assert_eq!(total.hash_matches, 400, "{total:?}");
    let counts = host.take_counts()[0].expect("seated");
    assert!(counts.repeated > 0, "{counts:?}");
}

#[test]
fn a_client_with_different_content_is_refused_at_the_start() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params.clone());
    let (server_end, client_end) = Loopback::pair();
    host.join(Box::new(server_end));
    let mut client = Client::connect(Box::new(client_end), "wrong");
    host.poll();
    client.poll_welcome().expect("welcomed").expect("a welcome");
    let mut other = params.clone();
    other.seed += 1;
    let mut race = headless(&other);
    let refused = client
        .ready(&mut race)
        .expect_err("a different start was accepted");
    assert!(
        format!("{refused}").contains("different content"),
        "{refused}"
    );
    host.poll();
    assert!(!host.all_ready());
    assert!(
        host.departed(),
        "the refused seat is a departure a lobby can end the start on"
    );
}

/// A person who leaves mid-race is retired on both ends at the same tick: the record carries
/// the seat, the server's and the survivor's craft stop being a person's, and the hashes still
/// agree on every confirmed tick.
#[test]
fn a_person_who_leaves_is_retired_on_both_ends_and_the_hashes_agree() {
    use oag_gameplay::Controller;
    let params = params(2);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 2, 0.0);
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    let mut survivor_mismatches = 0;
    let mut survivor_matches = 0;
    while host.tick_number() < 300 {
        host.poll();
        host.tick();
        for (n, (client, race, pilot)) in clients.iter_mut().enumerate() {
            if n == 1 && host.tick_number() > 100 {
                continue;
            }
            let own = pilot(client, race);
            client.step(race, &own);
            if n == 1 && host.tick_number() == 100 {
                client.leave("quit");
            }
            let stats = client.take_stats();
            if n == 0 {
                survivor_mismatches += stats.hash_mismatches;
                survivor_matches += stats.hash_matches;
            }
        }
    }
    host.end("the test is over");
    assert_eq!(survivor_mismatches, 0);
    assert!(survivor_matches > 200, "{survivor_matches}");
    let history = host.history();
    assert_eq!(history[50].retired, 0);
    assert_eq!(
        history[299].retired, 0b10,
        "seat 1 is retired once it has left"
    );
    assert_eq!(host.race().sim.world.controllers[1], Controller::Ai);
    assert_eq!(host.race().sim.world.controllers[0], Controller::Remote);
    // The survivor's own present simulation (what the game checks `finished()` on) has the
    // leaver's craft retired too, not only its confirmed state.
    let (survivor, race, _) = &mut clients[0];
    for _ in 0..50 {
        survivor.step(race, &oag_gameplay::InputSnapshot::EMPTY);
    }
    assert_eq!(
        race.sim.world.controllers[1],
        Controller::Ai,
        "the survivor's present has the leaver retired"
    );
}

/// A seat that has sent nothing for the allowed silence is dropped, told so, and retired.
#[test]
fn a_silent_seat_is_dropped_and_told() {
    let params = params(2);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 2, 0.0);
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    host.poll();
    host.tick();
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(host.drop_silent(web_time::Duration::from_millis(1)), 2);
    assert_eq!(
        host.drop_silent(web_time::Duration::from_millis(1)),
        0,
        "once"
    );
    for (client, race, _) in &mut clients {
        client.step(race, &oag_gameplay::InputSnapshot::EMPTY);
        match client.phase() {
            Phase::Over(why) => assert!(why.contains("dropped"), "{why}"),
            other => panic!("not told: {other:?}"),
        }
    }
    host.tick();
    assert_eq!(host.history()[1].retired, 0b11);
}
