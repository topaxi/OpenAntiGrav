//! A networked race on a real circuit, in one process: a server and two clients
//! over [`Loopback`] behind a 100 ms round trip ([`Shim`]), the clients flown by
//! the player's own driver (`Client::bot_input`), and the rest of the grid
//! the AI. The claims: every confirmed tick's state hash agrees on both clients,
//! the prediction is wrong often enough that corrections happen, and the
//! corrections land where the server is.
//!
//! The round trip is paced on the wall clock (the shim delays by real time), so
//! this takes the race's length in seconds.

use oag_net::client::Client;
use oag_net::host::Host;
use oag_net::link::{Conditions, Link, Loopback, Shim};
use oag_net::wire::RaceParams;

use crate::loopback::{Pilot, drive};

const IMAGE: &str = "pulse-psp-eu.chd";

fn params() -> RaceParams {
    RaceParams {
        title: String::new(),
        track: "Data\\Environments\\01_Track\\track.vex".into(),
        team: String::new(),
        class: "VENOM".into(),
        mode: "single_race".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 0x5EED,
        people: 2,
        opponents: true,
        weapons: false,
    }
}

#[test]
#[ignore = "needs a disc image"]
fn two_bot_clients_race_a_real_circuit_at_100_ms_and_agree_with_the_server() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let mut params = params();
    let (race, title) = oag_net::race::load(&params, &source).expect("loads");
    params.title = title.to_string();
    let mut host = Host::new(race, params.clone());
    let mut clients: Vec<_> = (0..2)
        .map(|n| {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let link: Box<dyn Link> = Box::new(Shim::new(
                client_end,
                Conditions {
                    one_way: web_time::Duration::from_millis(50),
                    loss: 0.0,
                },
            ));
            let client = Client::connect(link, &format!("bot {n}"));
            let (race, _) = oag_net::race::load(&params, &source).expect("loads");
            let pilot: Pilot = Box::new(|client, race| client.bot_input(race));
            (client, race, pilot)
        })
        .collect();
    let ticks = 900;
    let totals = drive(
        &mut host,
        &mut clients,
        ticks,
        std::time::Duration::from_micros(16_667),
    );
    println!("server, per seat: {:?}", host.take_counts());
    for (n, total) in totals.iter().enumerate() {
        println!("client {n}: {total:?}");
        assert_eq!(total.hash_mismatches, 0, "client {n}");
        assert_eq!(total.hash_matches, ticks, "client {n}");
        assert!(
            total.corrections > 0,
            "client {n}: nothing was ever mispredicted"
        );
    }
    // Both people really raced: their craft left the grid.
    let race = host.race();
    for slot in 0..2 {
        let speed = race.sim.world.ships[slot]
            .physics
            .body
            .linear_velocity
            .length();
        assert!(
            speed > 10.0,
            "slot {slot} at {speed} units/s after {ticks} ticks"
        );
    }
}

/// A LAN race the way the menus run it, in one process and over real sockets: a hosting game
/// ([`oag_net::lan::Hosted`], its own player on the in-process link), a browser that finds it
/// by discovery, a second player that connects with the hash the answer carries, START, and the
/// race the two of them run: seats in arrival order, the host seat 0, the head count fixed at
/// START, and every confirmed tick's hash agreeing.
#[test]
#[ignore = "needs a disc image"]
fn a_lan_host_is_found_joined_started_and_raced_by_two_over_real_sockets() {
    use oag_net::client::Phase;
    use oag_net::discovery::Browser;
    use oag_net::lan::{Config, Hosted};
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let mut params = params();
    params.title = "Wipeout Pulse".into();
    params.people = 1;
    // A UDP port of this test's own.
    let discovery_port = {
        let probe = std::net::UdpSocket::bind("127.0.0.1:0").expect("a free port");
        probe.local_addr().expect("bound").port()
    };
    let (hosted, own_link) = Hosted::spawn(Config {
        source: source.clone(),
        params,
        name: "Ada".into(),
        track_label: "Talon".into(),
        port: 0,
        discovery_port,
    })
    .expect("hosts");
    let mut ada = Client::connect(Box::new(own_link) as Box<dyn Link>, "Ada");

    let mut browser = Browser::on(discovery_port).expect("browser");
    let mut found = None;
    wait("the host to be found", || {
        found = browser.poll().into_iter().next();
        found.is_some()
    });
    let found = found.expect("found");
    assert_eq!(found.advert.name, "Ada");
    assert_eq!(found.advert.title, "Wipeout Pulse");
    assert_eq!(found.advert.track, "Talon");
    assert_eq!(
        found.advert.hash,
        hosted.hash(),
        "the answer carries the pin"
    );
    assert_eq!(found.addr.port(), hosted.port());

    let link = oag_net::wt::connect(&found.addr.to_string(), found.advert.hash).expect("connects");
    let mut bob = Client::connect(Box::new(link) as Box<dyn Link>, "Bob");
    wait("both players in the lobby", || {
        hosted.status().names == ["Ada", "Bob"]
    });
    assert!(
        ada.poll_welcome().expect("no refusal").is_none(),
        "a welcome came before START"
    );
    hosted.start();

    let mut welcomes = Vec::new();
    for client in [&mut ada, &mut bob] {
        let mut welcome = None;
        wait("the welcome", || {
            welcome = client.poll_welcome().expect("welcomed");
            welcome.is_some()
        });
        welcomes.push(welcome.expect("welcome"));
    }
    assert_eq!(welcomes[0].seat, 0, "the host is seat 0");
    assert_eq!(welcomes[1].seat, 1);
    assert_eq!(
        welcomes[0].params.people, 2,
        "the head count was fixed at START"
    );

    let mut races: Vec<_> = (0..2)
        .map(|_| {
            oag_net::race::load(&welcomes[0].params, &source)
                .expect("loads")
                .0
        })
        .collect();
    ada.ready(&mut races[0]).expect("same start state");
    bob.ready(&mut races[1]).expect("same start state");
    wait("the start", || {
        ada.poll();
        bob.poll();
        ada.phase() == &Phase::Racing && bob.phase() == &Phase::Racing
    });
    let (mut matches, mut mismatches) = (0, 0);
    for _ in 0..180 {
        for (client, race) in [&mut ada, &mut bob].into_iter().zip(&mut races) {
            let own = client.bot_input(race);
            client.step(race, &own);
            let stats = client.take_stats();
            matches += stats.hash_matches;
            mismatches += stats.hash_mismatches;
        }
        std::thread::sleep(std::time::Duration::from_micros(16_667));
    }
    assert_eq!(mismatches, 0);
    assert!(matches > 200, "{matches} confirmed ticks agreed");
}

/// Polls `done` until it holds, failing after three minutes (a load on a busy machine).
fn wait(what: &str, mut done: impl FnMut() -> bool) {
    let started = std::time::Instant::now();
    while !done() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(180),
            "timed out waiting for {what}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
