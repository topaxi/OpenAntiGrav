//! The online lobby end to end over real WebTransport sockets on localhost, no disc: rooms,
//! the one game slot and its queue, the hand-off, clients reconnecting to the race by seat
//! token, the race ending and its people returning to their room, and a race process that
//! crashes.

use std::sync::Arc;

use oag_net::client::{Client, Phase};
use oag_net::link::Link;
use oag_net::online::client::{Handoff, LobbyClient};
use oag_net::online::race_proc::{ChildRunner, Limits, ThreadRunner};
use oag_net::online::rooms::Config as RoomsConfig;
use oag_net::online::server::{Config, Server};
use oag_net::wire::online::{Edition, Phase as RoomPhase, RoomSettings};
use web_time::Duration;

use crate::loopback::headless;

const TITLE: &str = "Wipeout Test";

fn edition() -> Edition {
    Edition {
        title: TITLE.into(),
        platform: "PSP".into(),
        content: 0,
        build: "0.1 (abc)".into(),
    }
}

fn settings() -> RoomSettings {
    RoomSettings {
        title: TITLE.into(),
        track: String::new(),
        class: "VENOM".into(),
        mode: "single_race".into(),
        max_players: 4,
        opponents: false,
        laps: 0,
        no_barrel_rolls: false,
        pilot_assist: false,
        zone_target: 0,
    }
}

fn config() -> Config {
    Config {
        listen: "127.0.0.1:0".parse().expect("addr"),
        cert_names: Vec::new(),
        rooms: RoomsConfig {
            titles: vec![edition()],
            countdown_ms: 100,
            ..RoomsConfig::default()
        },
    }
}

fn thread_runner(max_ticks: u32) -> Box<ThreadRunner> {
    Box::new(ThreadRunner {
        load: Arc::new(|params| Ok(headless(params))),
        listen: "127.0.0.1:0".parse().expect("addr"),
        limits: Limits {
            join_timeout: Duration::from_secs(20),
            max_ticks,
        },
    })
}

/// A lobby and the people connected to it, pumped from one thread.
struct Lab {
    server: Server,
    people: Vec<LobbyClient>,
}

impl Lab {
    fn new(runner: Box<dyn oag_net::online::race_proc::Runner>) -> Self {
        Self {
            server: Server::bind(config(), runner).expect("binds"),
            people: Vec::new(),
        }
    }

    fn join(&mut self, name: &str) -> usize {
        let link = oag_net::wt::connect(
            &self.server.local_addr().to_string(),
            self.server.cert_hash(),
        )
        .expect("connects to the lobby");
        self.people.push(LobbyClient::connect(
            Box::new(link) as Box<dyn Link>,
            name,
            &[edition()],
            "",
        ));
        let who = self.people.len() - 1;
        self.until("the welcome", |lab| lab.people[who].welcomed());
        who
    }

    fn pump(&mut self) {
        self.server.poll();
        for person in &mut self.people {
            let _ = person.poll();
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    fn until(&mut self, what: &str, done: impl Fn(&Lab) -> bool) {
        let started = std::time::Instant::now();
        while !done(self) {
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "timed out waiting for {what}"
            );
            self.pump();
        }
    }

    fn phase(&self, who: usize) -> Option<RoomPhase> {
        self.people[who].room.as_ref().map(|r| r.phase)
    }

    /// `owner` makes a room, `guest` joins it, both pick a ship, the owner presses START.
    fn room_starts(&mut self, owner: usize, guest: usize, teams: [&str; 2]) {
        self.people[owner].create(settings());
        self.until("the room", |lab| lab.people[owner].room.is_some());
        let room = self.people[owner].room.as_ref().expect("room").id;
        self.people[guest].join(room);
        self.until("the join", |lab| {
            lab.people[guest]
                .room
                .as_ref()
                .is_some_and(|r| r.members.len() == 2)
        });
        self.people[owner].ready(teams[0], true, oag_physics::pilot_assist::Level::Off);
        self.people[guest].ready(teams[1], true, oag_physics::pilot_assist::Level::Off);
        self.until("both ready", |lab| {
            lab.people[owner]
                .room
                .as_ref()
                .is_some_and(|r| r.members.iter().all(|m| m.ready))
        });
        self.people[owner].start();
    }
}

/// Reconnects `who` to their race with the hand-off, and flies it to the end. Returns the
/// seat the race gave them and the state hashes that agreed and disagreed.
struct Racer {
    who: usize,
    client: Client,
    race: Option<Box<oag_raceplay::Race>>,
    seat: Option<usize>,
    matches: u32,
    mismatches: u32,
}

impl Racer {
    fn dial(who: usize, lab: &Lab, handoff: &Handoff) -> Self {
        let link = oag_net::wt::connect(
            &format!("{}:{}", lab.server.local_addr().ip(), handoff.port),
            handoff.cert_hash,
        )
        .expect("connects to the race");
        Self {
            who,
            client: Client::connect_seat(Box::new(link), &handoff.name, "", &handoff.token),
            race: None,
            seat: None,
            matches: 0,
            mismatches: 0,
        }
    }
}

/// Flies every racer until their race is over.
fn fly(lab: &mut Lab, racers: &mut [Racer]) {
    let started = std::time::Instant::now();
    while racers
        .iter()
        .any(|r| !matches!(r.client.phase(), Phase::Over(_)))
    {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the race never ended"
        );
        lab.pump();
        for racer in racers.iter_mut() {
            if racer.race.is_none() {
                if let Some(welcome) = racer.client.poll_welcome().expect("welcomed") {
                    racer.seat = Some(welcome.seat);
                    let mut race = headless(&welcome.params);
                    racer.client.ready(&mut race).expect("the same start state");
                    racer.race = Some(race);
                }
                continue;
            }
            racer.client.poll();
            if racer.client.phase() == &Phase::Racing {
                let race = racer.race.as_mut().expect("loaded");
                let own = racer.client.bot_input(race);
                racer.client.step(race, &own);
                let stats = racer.client.take_stats();
                racer.matches += stats.hash_matches;
                racer.mismatches += stats.hash_mismatches;
                std::thread::sleep(Duration::from_micros(14_000));
            }
        }
    }
}

#[test]
fn two_rooms_race_one_after_the_other_through_one_slot_and_return_to_their_rooms() {
    let mut lab = Lab::new(thread_runner(120));
    let (a1, a2, b1, b2) = (
        lab.join("a1"),
        lab.join("a2"),
        lab.join("b1"),
        lab.join("b2"),
    );
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.room_starts(b1, b2, ["Auricom", "Assegai"]);

    // Room A took the slot; room B is told it is next.
    lab.until("room A's hand-off", |lab| {
        lab.people[a1]
            .room
            .as_ref()
            .is_some_and(|r| r.phase == RoomPhase::Racing)
    });
    assert_eq!(lab.phase(b1), Some(RoomPhase::Queued { position: 1 }));
    assert_eq!(
        lab.server.racing_room(),
        Some(lab.people[a1].room.as_ref().expect("room").id)
    );
    lab.until("the hand-offs", |lab| {
        lab.people[a1].has_handoff() && lab.people[a2].has_handoff()
    });
    let handoffs: Vec<Handoff> = [a1, a2]
        .iter()
        .map(|&who| lab.people[who].take_handoff().expect("handed off"))
        .collect();
    assert_ne!(handoffs[0].token, handoffs[1].token, "a token a seat");
    assert_eq!(handoffs[0].port, handoffs[1].port, "one race");
    assert!(
        lab.people[b1].take_handoff().is_none(),
        "room B has no race yet"
    );

    // Reconnect in the wrong order: the token, not arrival, decides the seat.
    let mut racers = vec![
        Racer::dial(a2, &lab, &handoffs[1]),
        Racer::dial(a1, &lab, &handoffs[0]),
    ];
    fly(&mut lab, &mut racers);
    for racer in &racers {
        let expected = if racer.who == a1 { 0 } else { 1 };
        assert_eq!(
            racer.seat,
            Some(expected),
            "{} sits where the lobby put them",
            racer.who
        );
        assert_eq!(racer.mismatches, 0);
        assert!(
            racer.matches > 50,
            "{} confirmed ticks agreed",
            racer.matches
        );
    }

    // The slot is free: A is back in its room with the reason, and room B races next.
    lab.until("room A back open", |lab| {
        lab.phase(a1) == Some(RoomPhase::Open) && lab.phase(a2) == Some(RoomPhase::Open)
    });
    let note = lab.people[a1].room.as_ref().expect("room").note.clone();
    assert!(note.contains("max-ticks"), "{note}");
    assert!(
        lab.people[a1]
            .room
            .as_ref()
            .expect("room")
            .members
            .iter()
            .all(|m| !m.ready)
    );
    lab.until("room B's hand-off", |lab| {
        lab.people[b1].has_handoff() && lab.people[b2].has_handoff()
    });
    let handoffs: Vec<Handoff> = [b1, b2]
        .iter()
        .map(|&who| lab.people[who].take_handoff().expect("handed off"))
        .collect();
    let mut racers = vec![
        Racer::dial(b1, &lab, &handoffs[0]),
        Racer::dial(b2, &lab, &handoffs[1]),
    ];
    fly(&mut lab, &mut racers);
    assert!(racers.iter().all(|r| r.mismatches == 0 && r.matches > 50));
    lab.until("room B back open", |lab| {
        lab.phase(b1) == Some(RoomPhase::Open)
    });
    assert_eq!(lab.server.racing_room(), None);
}

#[test]
fn a_client_of_the_race_port_and_a_client_of_the_lobby_port_are_refused_by_name() {
    let lab = Lab::new(thread_runner(60));
    let mut lab = lab;
    let link = oag_net::wt::connect(&lab.server.local_addr().to_string(), lab.server.cert_hash())
        .expect("connects");
    let mut game = Client::connect(Box::new(link) as Box<dyn Link>, "lost");
    let started = std::time::Instant::now();
    let refused = loop {
        lab.pump();
        match game.poll_welcome() {
            Err(e) => break e,
            Ok(_) => assert!(started.elapsed().as_secs() < 20, "no answer"),
        }
    };
    assert!(format!("{refused}").contains("online lobby"), "{refused}");
}

#[test]
fn a_client_of_another_build_reads_the_refusal_before_the_close() {
    let mut lab = Lab::new(thread_runner(60));
    let link = oag_net::wt::connect(&lab.server.local_addr().to_string(), lab.server.cert_hash())
        .expect("connects");
    let other = Edition {
        build: "0.1 (zzz)".into(),
        ..edition()
    };
    let mut client = LobbyClient::connect(Box::new(link) as Box<dyn Link>, "Eve", &[other], "");
    let started = std::time::Instant::now();
    let (code, text) = loop {
        lab.server.poll();
        let polled = client.poll();
        if let Some(refused) = client.refused.take() {
            break refused;
        }
        assert!(
            polled.is_ok(),
            "the link closed before the refusal was read: {polled:?}"
        );
        assert!(started.elapsed().as_secs() < 20, "no answer");
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(code, oag_net::wire::online::Refusal::Version);
    assert!(
        text.contains("races only match between the same build"),
        "{text}"
    );
}

/// A link whose far end said something and then hung up: the order a refused hello arrives in
/// over a real network, where the close is read in the same poll as the refusal.
struct HungUp(oag_net::link::Loopback);

impl Link for HungUp {
    fn send_datagram(&mut self, bytes: Vec<u8>) {
        self.0.send_datagram(bytes);
    }
    fn recv_datagram(&mut self) -> Option<Vec<u8>> {
        self.0.recv_datagram()
    }
    fn send_reliable(&mut self, bytes: Vec<u8>) {
        self.0.send_reliable(bytes);
    }
    fn recv_reliable(&mut self) -> Option<Vec<u8>> {
        self.0.recv_reliable()
    }
    fn closed(&self) -> Option<String> {
        Some("closed: code=0 reason=bye".into())
    }
    fn max_datagram(&self) -> usize {
        1_200
    }
}

#[test]
fn a_refusal_that_arrives_with_the_close_is_still_read() {
    use oag_net::wire::online::{Message, Refusal};
    let (mut lobby, client_end) = oag_net::link::Loopback::pair();
    lobby.send_lobby(&Message::Rejected {
        code: Refusal::Version,
        text: "this server is build X, yours is Y".into(),
    });
    let mut client = LobbyClient::connect(Box::new(HungUp(client_end)), "Eve", &[edition()], "");
    assert!(
        client.poll().is_ok(),
        "the refusal must be handed over before the close"
    );
    let (code, text) = client.refused.take().expect("the refusal was read");
    assert_eq!(code, Refusal::Version);
    assert!(text.contains("build X"));
    assert!(
        client.poll().is_err(),
        "the close is reported on the next poll"
    );
}

/// A shell script standing in for `oag-server race`.
fn fake_race(body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("oag-lobby-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("race.sh");
    std::fs::write(&path, format!("#!/bin/sh\nread job\n{body}\n")).expect("script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

fn child_runner(program: std::path::PathBuf) -> Box<ChildRunner> {
    Box::new(ChildRunner {
        program,
        images: vec![(TITLE.to_string(), "unused".to_string())],
        listen: "127.0.0.1:0".parse().expect("addr"),
        limits: Limits::default(),
        load_timeout: Duration::from_secs(20),
    })
}

#[test]
fn a_race_process_that_comes_up_then_crashes_hands_off_then_returns_the_room_with_its_reason() {
    let script = fake_race(
        "echo 'listening on [::]:45678'\n\
         echo 'cert-hash 0101010101010101010101010101010101010101010101010101010101010101'\n\
         sleep 0.3\n\
         echo 'error: the circuit would not load' >&2\n\
         exit 3",
    );
    let mut lab = Lab::new(child_runner(script));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the hand-off", |lab| lab.people[a1].has_handoff());
    let handoff = lab.people[a1].take_handoff().expect("handed off");
    assert_eq!(handoff.port, 45678, "the port the child printed");
    assert_eq!(handoff.cert_hash, [1; 32], "the hash the child printed");
    lab.until("the room back open", |lab| {
        lab.phase(a1) == Some(RoomPhase::Open)
    });
    let note = lab.people[a2].room.as_ref().expect("room").note.clone();
    assert!(
        note.contains("exit") && note.contains("would not load"),
        "{note}"
    );
    assert_eq!(lab.server.racing_room(), None, "the slot is free again");
}

#[test]
fn a_race_that_never_comes_up_is_killed_and_the_slot_freed() {
    let script = fake_race("exec sleep 30");
    let mut lab = Lab::new(Box::new(ChildRunner {
        load_timeout: Duration::from_millis(400),
        ..*child_runner(script)
    }));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the room back open with a reason", |lab| {
        lab.phase(a1) == Some(RoomPhase::Open)
            && lab.people[a1]
                .room
                .as_ref()
                .is_some_and(|r| r.note.contains("did not come up"))
    });
}

#[test]
fn a_title_with_no_image_cannot_start_and_says_so() {
    let mut lab = Lab::new(Box::new(ChildRunner {
        images: Vec::new(),
        ..*child_runner("/bin/true".into())
    }));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the room back open with a reason", |lab| {
        lab.phase(a1) == Some(RoomPhase::Open)
            && lab.people[a1]
                .room
                .as_ref()
                .is_some_and(|r| r.note.contains("no image for"))
    });
}
