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
        teams: Vec::new(),
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
        platform: "PSP".into(),
        build: "test".into(),
        content: 0,
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
    // Idle past QUIC's 30 s idle timeout: only the keep-alive keeps the joiner connected.
    std::thread::sleep(std::time::Duration::from_secs(35));
    // (A joiner that dropped makes the welcome below fail with the link's close reason.)
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

/// The online lobby the way the maintainer runs it: the real `oag-server lobby` process on
/// localhost, two lobby clients that make a room and press START, the real `oag-server race`
/// child it forks on a second port, both clients reconnecting to it with their seat tokens (in
/// the wrong order), a real circuit raced with the hashes agreeing, the child exiting at its
/// tick limit, and both clients back in their room with the reason.
#[test]
#[ignore = "needs a disc image"]
fn the_lobby_forks_a_real_race_and_the_clients_reconnect_to_it_and_come_back() {
    use std::io::BufRead;
    use std::process::{Command, Stdio};

    use oag_net::client::Phase;
    use oag_net::online::client::LobbyClient;
    use oag_net::wire::online::{Phase as RoomPhase, RoomSettings};
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let mut lobby = Command::new(env!("CARGO_BIN_EXE_oag-server"))
        .args(["lobby", "--image", &source])
        .args(["--listen", "127.0.0.1:0", "--race-listen", "127.0.0.1:0"])
        .args(["--countdown-ms", "200", "--max-ticks", "300"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("the lobby starts");
    let mut lines = std::io::BufReader::new(lobby.stdout.take().expect("piped")).lines();
    let (mut addr, mut hash) = (None, None);
    while addr.is_none() || hash.is_none() {
        let line = lines.next().expect("the lobby is up").expect("a line");
        if let Some(a) = line.strip_prefix("lobby listening on ") {
            addr = Some(a.to_string());
        } else if let Some(h) = line.strip_prefix("lobby cert-hash ") {
            hash = Some(oag_net::wt::parse_hash(h).expect("hash"));
        }
    }
    let (addr, hash) = (addr.expect("addr"), hash.expect("hash"));
    // Keep draining the lobby's output so its pipe never fills.
    std::thread::spawn(move || lines.for_each(drop));
    let outcome = std::panic::catch_unwind(|| {
        let titles = ["Wipeout Pulse".to_string()];
        let mut people: Vec<LobbyClient> = ["Ada", "Bob"]
            .iter()
            .map(|name| {
                let link = oag_net::wt::connect(&addr, hash).expect("connects to the lobby");
                LobbyClient::connect(Box::new(link) as Box<dyn Link>, name, &titles, "")
            })
            .collect();
        let pump = |people: &mut Vec<LobbyClient>, done: &dyn Fn(&[LobbyClient]) -> bool| {
            let started = std::time::Instant::now();
            while !done(people) {
                assert!(started.elapsed().as_secs() < 240, "the lobby stalled");
                for person in people.iter_mut() {
                    let _ = person.poll();
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };
        pump(&mut people, &|p| p.iter().all(LobbyClient::welcomed));
        people[0].create(RoomSettings {
            title: "Wipeout Pulse".into(),
            track: params().track,
            class: "VENOM".into(),
            mode: "single_race".into(),
            max_players: 2,
            opponents: true,
        });
        pump(&mut people, &|p| p[0].room.is_some());
        let room = people[0].room.as_ref().expect("room").id;
        people[1].join(room);
        pump(&mut people, &|p| p[1].room.is_some());
        people[0].ready("Feisar", true);
        people[1].ready("Qirex", true);
        pump(&mut people, &|p| {
            p[0].room
                .as_ref()
                .is_some_and(|r| r.members.iter().all(|m| m.ready))
        });
        people[0].start();
        pump(&mut people, &|p| p.iter().all(LobbyClient::has_handoff));
        let handoffs: Vec<_> = people
            .iter_mut()
            .map(|p| p.take_handoff().expect("handoff"))
            .collect();
        let ip = addr.rsplit_once(':').expect("host:port").0.to_string();
        let mut racers: Vec<(Client, Option<Box<oag_raceplay::Race>>, usize)> = [1usize, 0]
            .iter()
            .map(|&who| {
                let h = &handoffs[who];
                let link = oag_net::wt::connect(&format!("{ip}:{}", h.port), h.cert_hash)
                    .expect("connects to the race child");
                (
                    Client::connect_seat(Box::new(link), &h.name, "", &h.token),
                    None,
                    who,
                )
            })
            .collect();
        let (mut matches, mut mismatches) = (0, 0);
        let started = std::time::Instant::now();
        while racers
            .iter()
            .any(|(c, _, _)| !matches!(c.phase(), Phase::Over(_)))
        {
            assert!(started.elapsed().as_secs() < 240, "the race stalled");
            for person in &mut people {
                let _ = person.poll();
            }
            for (client, race, who) in &mut racers {
                if race.is_none() {
                    if let Some(welcome) = client.poll_welcome().expect("welcomed") {
                        assert_eq!(
                            usize::from(welcome.seat),
                            *who,
                            "the token seats the person"
                        );
                        let mut loaded = oag_net::race::load(&welcome.params, &source)
                            .expect("loads")
                            .0;
                        client.ready(&mut loaded).expect("the same start state");
                        *race = Some(loaded);
                    }
                    continue;
                }
                let race = race.as_mut().expect("loaded");
                client.poll();
                if client.phase() == &Phase::Racing {
                    let own = client.bot_input(race);
                    client.step(race, &own);
                    let stats = client.take_stats();
                    matches += stats.hash_matches;
                    mismatches += stats.hash_mismatches;
                    std::thread::sleep(std::time::Duration::from_micros(14_000));
                }
            }
        }
        assert_eq!(mismatches, 0);
        assert!(matches > 100, "{matches} confirmed ticks agreed");
        pump(&mut people, &|p| {
            p.iter().all(|c| {
                c.room
                    .as_ref()
                    .is_some_and(|r| r.phase == RoomPhase::Open && r.note.contains("max-ticks"))
            })
        });
    });
    let _ = lobby.kill();
    let _ = lobby.wait();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
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
