//! A server and its clients in one process over [`Loopback`], no disc: the join,
//! the start, and the state hash agreeing on every confirmed tick, with and
//! without datagram loss.

use oag_net::client::{Client, Phase, Stats};
use oag_net::host::Host;
use oag_net::link::{Conditions, Link, Loopback, Shim};
use oag_net::wire::RaceParams;
use oag_raceplay::{Race, Setup};

/// What a test client flies with: a function of the tick, so every run is the
/// same and the two clients differ.
pub(crate) type Pilot = Box<dyn FnMut(&mut Client, &mut Race) -> oag_gameplay::InputSnapshot>;

/// Ticks the host and every client in lockstep with the wall clock left out:
/// each round the host reads its links and runs one tick once racing, and every
/// client steps once. Returns each client's summed stats.
pub(crate) fn drive(
    host: &mut Host,
    clients: &mut [(Client, Box<Race>, Pilot)],
    ticks: u32,
    sleep: std::time::Duration,
) -> Vec<Stats> {
    let mut totals = vec![Stats::default(); clients.len()];
    for (client, race, _) in clients.iter_mut() {
        loop {
            host.poll();
            if client.poll_welcome().expect("welcomed").is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
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
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    host.start(0);
    while host.tick_number() < ticks {
        host.poll();
        host.tick();
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
            total.resends += s.resends;
            total.stalls += s.stalls;
            total.sped_up += s.sped_up;
            total.slowed_down += s.slowed_down;
            total.lead = s.lead;
        }
        if !sleep.is_zero() {
            std::thread::sleep(sleep);
        }
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
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    totals
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
    let totals = drive(&mut host, &mut clients, 300, std::time::Duration::ZERO);
    for (n, total) in totals.iter().enumerate() {
        assert_eq!(total.hash_mismatches, 0, "client {n}: {total:?}");
        assert_eq!(total.hash_matches, 300, "client {n}: {total:?}");
        assert!(total.predicted >= 300, "client {n}: {total:?}");
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
    let totals = drive(&mut host, &mut clients, 400, std::time::Duration::ZERO);
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
