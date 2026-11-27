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

use crate::loopback::{Pace, Pilot, drive, drop_and_return, tokened};

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
        laps: 0,
        no_barrel_rolls: false,
        assists: Vec::new(),
        zone_target: 0,
    }
}

/// The header a recording of this room's race opens with.
fn recording_header(params: &RaceParams) -> oag_replay::Header {
    let mut header = oag_replay::Header::new(
        &params.title,
        &params.track,
        &params.mode,
        &params.class,
        &params.team,
        params.seed,
    );
    header.options = oag_net::race::replay_options(params).expect("the room's options");
    header
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
    let clock = oag_net::link::Clock::default();
    // Every race is loaded before any link joins (the host's hello timeout is wall-clock).
    let races: Vec<_> = (0..2)
        .map(|_| oag_net::race::load(&params, &source).expect("loads").0)
        .collect();
    let mut clients: Vec<_> = races
        .into_iter()
        .zip(0..2)
        .map(|(race, n)| {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let link: Box<dyn Link> = Box::new(Shim::with_clock(
                client_end,
                Conditions {
                    one_way: web_time::Duration::from_millis(50),
                    loss: 0.0,
                    ..Conditions::default()
                },
                clock.clone(),
            ));
            let client = Client::connect(link, &format!("bot {n}"));
            let pilot: Pilot = Box::new(|client, race| client.bot_input(race));
            (client, race, pilot)
        })
        .collect();
    let ticks = 900;
    let totals = drive(
        &mut host,
        &mut clients,
        ticks,
        &Pace::Virtual(clock, std::time::Duration::from_micros(16_667)),
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
        let speed = race.core.sim.world.ships[slot]
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

/// A recording armed on a predicting client describes the run the server confirmed: the
/// predicted ticks (rewound by every correction, and jumped over when the server is ahead) are not
/// written, the confirmed ones are, in order, so the recorder's tick invariant holds under debug
/// assertions and the file re-drives to the server's own hashes.
#[test]
#[ignore = "needs a disc image"]
fn a_predicting_clients_recording_is_the_confirmed_run_and_re_drives() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let mut params = params();
    let (race, title) = oag_net::race::load(&params, &source).expect("loads");
    params.title = title.to_string();
    let mut host = Host::new(race, params.clone());
    let clock = oag_net::link::Clock::default();
    // Every race is loaded before any link joins (the host's hello timeout is wall-clock).
    let races: Vec<_> = (0..2)
        .map(|_| oag_net::race::load(&params, &source).expect("loads").0)
        .collect();
    let mut clients: Vec<_> = races
        .into_iter()
        .zip(0..2)
        .map(|(mut race, n)| {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let link: Box<dyn Link> = Box::new(Shim::with_clock(
                client_end,
                Conditions {
                    one_way: web_time::Duration::from_millis(50),
                    loss: 0.0,
                    ..Conditions::default()
                },
                clock.clone(),
            ));
            let client = Client::connect(link, &format!("bot {n}"));
            race.start_recording(recording_header(&params));
            let pilot: Pilot = Box::new(|client, race| client.bot_input(race));
            (client, race, pilot)
        })
        .collect();
    let ticks = 900;
    let totals = drive(
        &mut host,
        &mut clients,
        ticks,
        &Pace::Virtual(clock, std::time::Duration::from_micros(16_667)),
    );
    for (n, total) in totals.iter().enumerate() {
        assert!(total.corrections > 0, "client {n}: nothing was corrected");
        assert_eq!(total.hash_mismatches, 0, "client {n}");
        let replay = clients[n].1.recorded().expect("the client records");
        assert_eq!(
            replay.ticks(),
            u64::from(total.hash_matches),
            "client {n}: the recording holds the confirmed ticks, no more and no fewer"
        );
        let (mut again, _) = oag_net::race::load(&params, &source).expect("loads");
        let found = oag_replay::Verifier::run(&replay, again.core.sim.state_hash(), |_, inputs| {
            again.tick_sim_only(inputs);
            again.core.sim.state_hash()
        })
        .unwrap_or_else(|why| panic!("client {n}: the recording does not reproduce: {why:?}"));
        assert_eq!(found, replay.ticks(), "client {n}");
        // The viewer's own rebuild (WATCH REPLAY) reaches the tick-zero state the recording
        // opened with: the file carries the room, not only the launch.
        let rebuilt = oag_raceplay::replay::rebuild(&replay.header, &source)
            .unwrap_or_else(|why| panic!("client {n}: the viewer cannot rebuild it: {why:#}"));
        assert_eq!(
            rebuilt.core.sim.state_hash(),
            replay.initial_hash,
            "client {n}: the rebuilt race is not the recorded one"
        );
        // The pose track the client kept is the picture of the confirmed run: re-driving the
        // confirmed inputs through full ticks draws the same craft in the same places, so a
        // hit the prediction missed is where the server had it.
        let mut shown = rebuilt;
        shown.start_recording(replay.header.clone());
        oag_replay::Verifier::run(&replay, shown.core.sim.state_hash(), |_, inputs| {
            shown.tick(inputs);
            shown.core.sim.state_hash()
        })
        .unwrap_or_else(|why| panic!("client {n}: the viewer's re-drive diverges: {why:?}"));
        let again = shown
            .recorded()
            .and_then(|r| r.pose_track)
            .expect("pose track");
        let kept = replay
            .pose_track
            .as_ref()
            .expect("the client kept a pose track");
        assert_eq!(kept.crafts(), again.crafts(), "client {n}");
        assert_eq!(kept.last_tick(), again.last_tick(), "client {n}");
        for slot in 0..kept.crafts() {
            for tick in 0..=kept.last_tick() {
                let (a, b) = (kept.pose(slot, tick as f64), again.pose(slot, tick as f64));
                assert_eq!(a, b, "client {n}: craft {slot} at tick {tick}");
            }
        }
    }
    // The two clients hold the same run over the stretch both confirmed, whatever their
    // predictions did on the way.
    let theirs: Vec<_> = clients
        .iter()
        .map(|(_, race, _)| race.recorded().expect("the client records"))
        .collect();
    let common = theirs.iter().map(|r| r.ticks()).min().unwrap();
    assert!(common > 600, "only {common} ticks in common");
    assert_eq!(
        theirs[0].fingerprint_through(common),
        theirs[1].fingerprint_through(common),
        "the two clients of one race hold different runs"
    );
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

/// A private copy of `oag-server` for the test `tag`: the lobby execs its own `current_exe()`
/// again for each race, so the shared `target/<profile>/oag-server` must not be the file run
/// (`oag_testdata::exec`).
fn server_copy(tag: &str) -> std::path::PathBuf {
    let dest = oag_testdata::scratch::dir(&format!("net-ground-truth-{tag}"));
    oag_testdata::exec::private_copy(
        &oag_testdata::scratch::cache("binary-copies"),
        std::path::Path::new(env!("CARGO_BIN_EXE_oag-server")),
        &dest,
        || {},
    )
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
    let mut lobby = oag_testdata::exec::spawn(
        Command::new(server_copy("lobby"))
            .args(["lobby", "--image", &source])
            .args(["--listen", "127.0.0.1:0", "--race-listen", "127.0.0.1:0"])
            .args(["--countdown-ms", "200", "--max-ticks", "9000"])
            .stdout(Stdio::piped()),
    )
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
        let opened = oag_source::title::open_source(&source, Vec::new(), Vec::new())
            .expect("the image opens");
        let titles = [oag_net::wire::online::Edition {
            title: opened.title.name.to_string(),
            platform: opened.archives.layout.platform.to_string(),
            content: oag_net::discovery::NO_MODS,
            build: oag_net::BUILD_VERSION.to_string(),
        }];
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
            laps: 1,
            no_barrel_rolls: false,
            pilot_assist: false,
            weapons: false,
            zone_target: 0,
        });
        pump(&mut people, &|p| p[0].room.is_some());
        let room = people[0].room.as_ref().expect("room").id;
        people[1].join(room);
        pump(&mut people, &|p| p[1].room.is_some());
        people[0].ready("Feisar", true, oag_physics::pilot_assist::Level::Off);
        people[1].ready("Qirex", true, oag_physics::pilot_assist::Level::Off);
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
                        assert_eq!(welcome.seat, *who, "the token seats the person");
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
                    std::thread::sleep(std::time::Duration::from_micros(8_000));
                }
            }
        }
        assert_eq!(mismatches, 0);
        assert!(matches > 2_000, "{matches} confirmed ticks agreed");
        // The game steps a client every frame, over or not: the end state settles then.
        for (client, race, _) in &mut racers {
            let race = race.as_mut().expect("loaded");
            for _ in 0..3 {
                client.step(race, &oag_gameplay::InputSnapshot::EMPTY);
            }
        }
        for (client, race, who) in &racers {
            let race = race.as_ref().expect("loaded");
            assert_eq!(
                client.phase(),
                &Phase::Over("every person has finished".into()),
                "person {who} was told the race ran to the finish line, not to a limit"
            );
            assert!(
                race.core.sim.finished(),
                "person {who}'s own state has every person finished"
            );
            let board = race.core.results().expect("a finished race has a board");
            assert_eq!(board.laps_target, Some(1));
            assert!(
                board
                    .rows
                    .iter()
                    .filter(|row| row.player)
                    .all(|row| row.finished()),
                "both people are on the board as finishers"
            );
        }
        // The server's own board came in `End`: it is the same race as each client's own
        // (names are the server's alone), names both people, and is what the room holds.
        let mut boards = Vec::new();
        for (client, race, who) in &racers {
            let race = race.as_ref().expect("loaded");
            let theirs = client
                .final_board()
                .unwrap_or_else(|| panic!("person {who}: End carried no board"));
            assert!(
                oag_net::race::final_board(race, &[]).same_race_as(theirs),
                "person {who}: the server's board is this client's board"
            );
            let mut named: Vec<(u8, String)> = theirs
                .rows
                .iter()
                .filter(|row| !row.name.is_empty())
                .map(|row| (row.slot, row.name.clone()))
                .collect();
            named.sort();
            assert_eq!(named, [(0, "Ada".to_string()), (1, "Bob".to_string())]);
            boards.push(theirs.clone());
        }
        assert_eq!(boards[0], boards[1], "every person got the one board");
        // The room is open again with the reason and the board, and the slot is free.
        pump(&mut people, &|p| {
            p.iter().all(|c| {
                c.room.as_ref().is_some_and(|r| {
                    r.phase == RoomPhase::Open && r.note.contains("every person has finished")
                })
            })
        });
        for person in &people {
            let room = person.room.as_ref().expect("room");
            assert_eq!(
                room.board, boards[0],
                "the room screen shows the race's board"
            );
        }
        // A rematch from both: a second race starts under the same settings, without leaving.
        let settings = people[0].room.as_ref().expect("room").settings.clone();
        people[0].rematch();
        people[1].rematch();
        pump(&mut people, &|p| p.iter().all(LobbyClient::has_handoff));
        for person in &people {
            assert_eq!(person.room.as_ref().expect("room").settings, settings);
        }
    });
    let _ = lobby.kill();
    let _ = lobby.wait();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// A race child whose lobby is gone stops: the lobby holds the child's standard input open, so
/// when the lobby dies (SIGKILL included) the pipe closes and the child exits instead of holding
/// the race port for its whole join timeout.
#[test]
#[ignore = "needs a disc image"]
fn a_race_child_stops_when_its_lobby_is_gone() {
    use std::io::{BufRead, Write};
    use std::process::{Command, Stdio};

    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let job = oag_net::wire::online::RaceJob {
        params: RaceParams {
            title: "Wipeout Pulse".into(),
            ..params()
        },
        seats: vec![("Ada".into(), "tok".into())],
        keys: Vec::new(),
    };
    let mut child = oag_testdata::exec::spawn(
        Command::new(server_copy("race-child"))
            .args(["race", "--image", &image.display().to_string()])
            .args(["--listen", "127.0.0.1:0", "--join-timeout", "600"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped()),
    )
    .expect("the child starts");
    let mut stdin = child.stdin.take().expect("piped");
    writeln!(stdin, "{}", job.to_line()).expect("the job is sent");
    let mut lines = std::io::BufReader::new(child.stdout.take().expect("piped")).lines();
    while !lines
        .next()
        .expect("the child is up")
        .expect("a line")
        .starts_with("cert-hash ")
    {}
    assert!(
        child.try_wait().expect("polls").is_none(),
        "the child waits for its people while the lobby is there"
    );
    drop(stdin);
    wait("the child to notice the lobby is gone", || {
        child.try_wait().expect("polls").is_some()
    });
    let status = child.wait().expect("exited");
    assert_eq!(
        status.code(),
        Some(2),
        "it stops with a reason, not a crash"
    );
}

/// A retired person's craft is flown by the AI driver, not left to coast: a person who sends no
/// input never launches (speed 0), and the same craft retired 300 ticks in drives off round the
/// circuit.
#[test]
#[ignore = "needs a disc image"]
fn a_retired_person_s_craft_is_flown_by_the_ai() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let none = oag_gameplay::PlayerInputs::none();
    let flown = |retire: bool| {
        let (mut race, _) = oag_net::race::load(&params(), &source).expect("loads");
        for tick in 0..900 {
            if retire && tick == 300 {
                oag_net::race::retire(&mut race, 0b10);
            }
            race.tick_sim_only(&none);
        }
        let ship = &race.core.sim.world.ships[1].physics.body;
        (ship.linear_velocity.length(), ship.position)
    };
    let (idle_speed, idle_at) = flown(false);
    let (speed, at) = flown(true);
    eprintln!(
        "idle {idle_speed:.1}, retired {speed:.1}, apart {:.0}",
        (at - idle_at).length()
    );
    assert!(
        idle_speed < 1.0,
        "a person who sends nothing does not launch: {idle_speed}"
    );
    assert!(speed > 50.0, "the retired craft is flying at {speed}");
    assert!(
        (at - idle_at).length() > 300.0,
        "and has left the grid behind"
    );
}

/// A race of this room's, with a Turbo in each person's pickup slot.
fn holding_turbo(params: &RaceParams, source: &str) -> Box<oag_raceplay::Race> {
    let (mut race, _) = oag_net::race::load(params, source).expect("loads");
    for ship in race
        .core
        .sim
        .world
        .ships
        .iter_mut()
        .take(usize::from(params.people))
    {
        ship.pickup.weapon = Some(oag_tables::weapons::Weapon::Turbo);
    }
    race
}

/// A person whose link closes at tick 600 and returns at tick 900 flies their craft again on a
/// real circuit: the seat is away on both ends in between with the pickup it held still held, the
/// hand-back shows as an Autopilot countdown over that pickup, then the pickup is usable again,
/// both ends agree on every hash, and the returning client's recording of the confirmed run
/// (the drop and the return in it as events) re-drives to the server's hashes.
#[test]
#[ignore = "needs a disc image"]
fn a_dropped_link_returns_to_its_craft_and_its_pickup() {
    use oag_gameplay::Controller;
    use oag_tables::weapons::Weapon;
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let mut params = params();
    params.weapons = true;
    let mut host = Host::new(holding_turbo(&params, &source), params.clone());
    let (mut clients, tokens) = tokened(&mut host, 2, &mut || holding_turbo(&params, &source));
    clients[1].1.start_recording(recording_header(&params));
    for client in &mut clients {
        // The bot's flying without its trigger: it fires what it holds, and this test holds it.
        client.2 = Box::new(|client, race| {
            use oag_gameplay::input::Button;
            let mut own = client.bot_input(race);
            let mut buttons = oag_gameplay::input::Input::new();
            buttons.begin_frame(if own.buttons.is_held(Button::Cross) {
                Button::Cross.bit()
            } else {
                0
            });
            own.buttons = buttons;
            own
        });
    }
    let mut handed_back_at = None;
    let mut countdown_seen = false;
    let mut away_speed = 0.0_f32;
    let (mismatches, matches) =
        drop_and_return(&mut host, &mut clients, &tokens, (600, 900, 1500), |host| {
            let at = host.tick_number();
            let race = host.race();
            let ship = &race.core.sim.world.ships[1];
            if (700..900).contains(&at) {
                assert_eq!(
                    race.core.sim.world.controllers[1],
                    Controller::Away,
                    "tick {at}"
                );
                assert_eq!(ship.pickup.weapon, Some(Weapon::Turbo), "tick {at}");
                away_speed = away_speed.max(ship.physics.body.linear_velocity.length());
            }
            if at % 100 == 0 && at >= 500 {
                eprintln!(
                    "tick {at} seat 1 speed {:.1} {:?}",
                    ship.physics.body.linear_velocity.length(),
                    race.core.sim.world.controllers[1]
                );
            }
            if handed_back_at.is_none() && at > 900 && ship.handback {
                handed_back_at = Some(at);
            }
            if let Some(from) = handed_back_at {
                let shown = race.readout_for(1);
                if at < from + 140 {
                    countdown_seen |= shown.pickup == Some(Weapon::Autopilot)
                        && format!("{:?}", shown.sub_weapon).starts_with("Some(Autopilot(");
                    assert_eq!(ship.pickup.weapon, Some(Weapon::Turbo), "tick {at}");
                } else if at > from + 160 {
                    assert_eq!(shown.pickup, Some(Weapon::Turbo), "tick {at}");
                    assert_eq!(shown.sub_weapon, None, "tick {at}");
                    assert!(!ship.handback && ship.autopilot_timer == 0.0, "tick {at}");
                    assert_eq!(race.core.sim.world.controllers[1], Controller::Remote);
                }
            }
        });
    assert_eq!(mismatches, [0, 0]);
    assert!(matches[0] > 1400 && matches[1] > 1000, "{matches:?}");
    assert!(handed_back_at.is_some(), "the seat was never handed back");
    assert!(countdown_seen, "the HUD never showed the countdown");
    eprintln!("away top speed {away_speed:.1}, handed back at {handed_back_at:?}");

    // The pickup is usable again: the returned person fires it.
    let (_, race, _) = &mut clients[1];
    let mut square = oag_gameplay::input::Input::new();
    square.begin_frame(oag_gameplay::input::Button::Square.bit());
    let mut inputs = oag_gameplay::PlayerInputs::none();
    inputs.set(
        1,
        oag_gameplay::InputSnapshot {
            buttons: square,
            ..oag_gameplay::InputSnapshot::default()
        },
    );
    race.tick_sim_only(&inputs);
    assert_eq!(
        race.core.sim.world.ships[1].pickup.weapon, None,
        "the Turbo was fired"
    );
    assert!(race.core.sim.world.ships[1].physics.turbo_timer > 0.0);

    // The recording re-drives, drop and return included.
    let replay = clients[1].1.recorded().expect("the client records");
    let events: Vec<_> = replay
        .events
        .iter()
        .filter(|event| event.kind == oag_replay::Kind::Presence.code())
        .map(|event| (event.tick, event.value))
        .collect();
    assert_eq!(events.len(), 2, "{events:?}");
    assert_eq!(events[0], (600, 0));
    assert_eq!(events[1].1, 1);
    let mut again = holding_turbo(&params, &source);
    let found = oag_replay::Verifier::run(&replay, again.core.sim.state_hash(), |due, inputs| {
        for event in due {
            again.core.sim.apply_event(event).expect("applies");
        }
        again.tick_sim_only(inputs);
        again.core.sim.state_hash()
    })
    .unwrap_or_else(|why| panic!("the recording across a drop does not reproduce: {why:?}"));
    assert_eq!(found, replay.ticks());
}

/// A dropped person's craft must lose time to the opponent driver that flies the seat, never
/// gain: the same craft handed to the plain opponent driver at tick 300 covers more of the
/// racing line in the next 1,500 ticks, and what it held is still held at the end.
#[test]
#[ignore = "needs a disc image"]
fn an_away_craft_loses_time_against_the_opponent_driver() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let params = params();
    let none = oag_gameplay::PlayerInputs::none();
    let flown = |away: bool| {
        let mut race = holding_turbo(&params, &source);
        for tick in 0..1800 {
            if tick == 300 {
                race.set_away(1);
                if !away {
                    race.core.sim.world.controllers[1] = oag_gameplay::Controller::Ai;
                }
            }
            race.tick_sim_only(&none);
        }
        let ship = &race.core.sim.world.ships[1];
        (ship.driver.index, ship.pickup.weapon)
    };
    let (opponent, _) = flown(false);
    let (away, held) = flown(true);
    eprintln!("racing-line index after 1,500 ticks: opponent driver {opponent}, away {away}");
    assert!(
        away < opponent,
        "the away craft is no faster than an opponent: {away} against {opponent}"
    );
    assert!(away > 0, "and it is flying: {away}");
    assert_eq!(
        held,
        Some(oag_tables::weapons::Weapon::Turbo),
        "the away craft kept its pickup"
    );
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

/// The lobby's cost to judge a five-minute player-hosted race (18,000 ticks of two people on
/// Pulse's circuit 01): the load, the re-simulation, the proof's bytes and the memory, printed
/// for `docs/networking/server-roadmap.md` ("Anti-cheat: what it costs"). A sweep: it asserts
/// only that an honest race is dismissed.
#[test]
#[ignore = "a sweep: OAG_SWEEP=1, needs a disc image"]
fn sweep_what_judging_a_five_minute_hosted_race_costs_the_lobby() {
    use oag_net::fair::{Chain, Checkpoint, Claim, Proof, Verdict, judge, params_hash};
    use oag_net::wire::{Held, TickRecord};
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let source = image.display().to_string();
    let params = params();
    let rss = || -> u64 {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmRSS:"))?
                    .split_whitespace()
                    .nth(1)?
                    .parse()
                    .ok()
            })
            .unwrap_or(0)
    };
    const TICKS: u32 = 18_000;
    let (mut race, _) = oag_net::race::load(&params, &source).expect("loads");
    let fingerprint = oag_net::race::fingerprint(&race);
    let mut edges = [oag_gameplay::input::Input::new(); 2];
    let cross = oag_gameplay::input::Button::Cross.bit();
    let records: Vec<TickRecord> = (0..TICKS)
        .map(|tick| {
            let inputs: Vec<Held> = (0..2u32)
                .map(|seat| Held {
                    buttons: cross,
                    stick_x: ((tick / 30 + seat) % 9) as f32 / 8.0 - 0.5,
                    ..Held::default()
                })
                .collect();
            let mut snapshots = oag_gameplay::PlayerInputs::none();
            for (slot, held) in inputs.iter().enumerate() {
                snapshots.set(slot, held.snapshot(&mut edges[slot]));
            }
            oag_net::race::presence(&mut race, 0);
            race.tick_sim_only(&snapshots);
            TickRecord {
                hash: race.core.sim.state_hash(),
                inputs,
                retired: 0,
            }
        })
        .collect();
    drop(race);
    let key = oag_net::online::hostkey::PlayerKey::generate().expect("a key");
    let hash = params_hash(&params);
    let mut chain = Chain::new(&hash, fingerprint);
    for record in &records {
        chain.push(record);
    }
    let proof = Proof {
        claim: Claim::Desync,
        fingerprint,
        checkpoint: Checkpoint::sign(&key, &hash, &chain),
        records,
    };
    let bytes = proof.encode();
    let before = rss();
    let started = std::time::Instant::now();
    let decoded = Proof::decode(&bytes, 2, TICKS).expect("decodes");
    decoded.check_signed(&hash, &key.public()).expect("signed");
    let checked = started.elapsed();
    let started = std::time::Instant::now();
    let (mut race, _) = oag_net::race::load(&params, &source).expect("loads");
    let loaded = started.elapsed();
    let after_load = rss();
    let started = std::time::Instant::now();
    let verdict = judge(
        &mut race,
        &decoded,
        std::time::Instant::now() + std::time::Duration::from_secs(600),
    );
    let judged = started.elapsed();
    println!(
        "judge: {TICKS} ticks, proof {} bytes; decode and chain check {checked:?}; race load \
         {loaded:?}; re-simulation {judged:?}; RSS {} KiB before the load, {} KiB after it, {} \
         KiB after the re-simulation",
        bytes.len(),
        before,
        after_load,
        rss()
    );
    assert!(matches!(verdict, Verdict::Dismissed(_)), "{verdict:?}");
    // What a host pays per input datagram it takes: one Ed25519 verification.
    let inputs = decoded.records[..8]
        .iter()
        .map(|r| r.inputs[1])
        .collect::<Vec<_>>();
    let statement = oag_net::fair::input_statement(&hash, 1, 0, &inputs);
    let datagram = oag_net::wire::Datagram::Inputs {
        seat: 1,
        first: 0,
        inputs,
        signature: Some(key.sign(&statement)),
    };
    let started = std::time::Instant::now();
    for _ in 0..10_000 {
        assert!(oag_net::fair::inputs_signed(
            &datagram,
            &hash,
            1,
            &key.public()
        ));
    }
    println!(
        "verify: {:?} per signed input datagram of {} bytes",
        started.elapsed() / 10_000,
        datagram.encode().len()
    );
}
