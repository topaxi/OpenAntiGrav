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
use oag_net::wire::board::{Board, BoardRow};
use oag_net::wire::online::{Edition, Phase as RoomPhase, Refusal, RoomSettings};
use web_time::Duration;

use crate::loopback::headless;

const TITLE: &str = "Wipeout Test";

pub(crate) fn edition() -> Edition {
    Edition {
        title: TITLE.into(),
        platform: "PSP".into(),
        content: 0,
        build: "0.1 (abc)".into(),
    }
}

pub(crate) fn settings() -> RoomSettings {
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
        weapons: false,
        zone_target: 0,
    }
}

pub(crate) fn config() -> Config {
    Config {
        listen: "127.0.0.1:0".parse().expect("addr"),
        cert_names: Vec::new(),
        rooms: RoomsConfig {
            titles: vec![edition()],
            countdown_ms: 100,
            ..RoomsConfig::default()
        },
        max_connections: oag_net::online::server::MAX_CONNECTIONS,
        max_per_address: oag_net::online::server::MAX_PER_ADDRESS,
        ban_file: None,
    }
}

pub(crate) fn thread_runner(max_ticks: u32) -> Box<ThreadRunner> {
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
pub(crate) struct Lab {
    pub(crate) server: Server,
    pub(crate) people: Vec<LobbyClient>,
    /// The edition every person joins with.
    pub(crate) edition: Edition,
}

impl Lab {
    pub(crate) fn new(runner: Box<dyn oag_net::online::race_proc::Runner>) -> Self {
        Self::with_config(config(), runner)
    }

    pub(crate) fn with_config(
        config: Config,
        runner: Box<dyn oag_net::online::race_proc::Runner>,
    ) -> Self {
        Self {
            server: Server::bind(config, runner).expect("binds"),
            people: Vec::new(),
            edition: edition(),
        }
    }

    pub(crate) fn join(&mut self, name: &str) -> usize {
        self.join_keyed(name, [0; 32])
    }

    /// [`Self::join`] naming the player's public `key` in the hello.
    pub(crate) fn join_keyed(&mut self, name: &str, key: [u8; 32]) -> usize {
        let link = oag_net::wt::connect(
            &self.server.local_addr().to_string(),
            self.server.cert_hash(),
        )
        .expect("connects to the lobby");
        self.people.push(LobbyClient::connect_keyed(
            Box::new(link) as Box<dyn Link>,
            name,
            std::slice::from_ref(&self.edition),
            "",
            key,
        ));
        let who = self.people.len() - 1;
        self.until("the welcome", |lab| lab.people[who].welcomed());
        who
    }

    pub(crate) fn pump(&mut self) {
        self.server.poll();
        for person in &mut self.people {
            let _ = person.poll();
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    pub(crate) fn until(&mut self, what: &str, done: impl Fn(&Lab) -> bool) {
        let started = std::time::Instant::now();
        while !done(self) {
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "timed out waiting for {what}"
            );
            self.pump();
        }
    }

    pub(crate) fn phase(&self, who: usize) -> Option<RoomPhase> {
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
pub(crate) struct Racer {
    pub(crate) who: usize,
    pub(crate) client: Client,
    race: Option<Box<oag_raceplay::Race>>,
    pub(crate) seat: Option<usize>,
    pub(crate) matches: u32,
    pub(crate) mismatches: u32,
}

impl Racer {
    pub(crate) fn dial(who: usize, lab: &Lab, handoff: &Handoff) -> Self {
        // A player-hosted race is on the address the hand-off names, a server's on the lobby's.
        let host = if handoff.host.is_empty() {
            lab.server.local_addr().ip().to_string()
        } else {
            handoff.host.clone()
        };
        let link = oag_net::wt::connect(&format!("{host}:{}", handoff.port), handoff.cert_hash)
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
pub(crate) fn fly(lab: &mut Lab, racers: &mut [Racer]) {
    let started = std::time::Instant::now();
    while racers
        .iter()
        .any(|r| !matches!(r.client.phase(), Phase::Over(_)))
    {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the race never ended"
        );
        fly_once(lab, racers);
    }
}

/// One pass of [`fly`]: the lobby, then each racer loads, or reads its link and steps.
pub(crate) fn fly_once(lab: &mut Lab, racers: &mut [Racer]) {
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

    // Room A took the slot; room B is told it is next. B's START is still on the wire when
    // `room_starts` returns, and A may already be racing by then, so each is waited for.
    lab.until("room B told it is next", |lab| {
        lab.phase(b1) == Some(RoomPhase::Queued { position: 1 })
    });
    lab.until("room A racing", |lab| {
        lab.phase(a1) == Some(RoomPhase::Racing)
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
fn fake_race(name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = fake_race_dir();
    let path = dir.join(format!("{name}.sh"));
    std::fs::write(&path, format!("#!/bin/sh\nread job\n{body}\n")).expect("script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

fn fake_race_dir() -> oag_testdata::scratch::TempDir {
    let dir = oag_testdata::scratch::dir("lobby");
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// A script waits at a gate until the test, having seen what it was waiting for, opens it:
/// a wall-clock `sleep` in the script would race the test's own pump, which a loaded machine
/// can stall past it (the child would exit before the lobby polled it, and a child that
/// exited is `Over`, never `Up`).
fn gate(name: &str) -> std::path::PathBuf {
    let path = fake_race_dir().join(format!("{name}.gate"));
    let _ = std::fs::remove_file(&path);
    path
}

/// The script's line that blocks until `gate` exists.
fn wait_at(gate: &std::path::Path) -> String {
    format!("while [ ! -e '{}' ]; do sleep 0.01; done", gate.display())
}

fn child_runner(program: std::path::PathBuf) -> Box<ChildRunner> {
    Box::new(ChildRunner {
        program,
        images: vec![(TITLE.to_string(), "unused".to_string())],
        listen: "127.0.0.1:0".parse().expect("addr"),
        limits: Limits::default(),
        load_timeout: Duration::from_secs(20),
        quiet_timeout: Duration::from_secs(60),
    })
}

#[test]
fn a_race_process_that_comes_up_then_crashes_hands_off_then_returns_the_room_with_its_reason() {
    let gate = gate("crash");
    let script = fake_race(
        "crash",
        &format!(
            "echo 'listening on [::]:45678'\n\
             echo 'cert-hash 0101010101010101010101010101010101010101010101010101010101010101'\n\
             {}\n\
             echo 'error: the circuit would not load' >&2\n\
             exit 3",
            wait_at(&gate)
        ),
    );
    let mut lab = Lab::new(child_runner(script));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the hand-off", |lab| lab.people[a1].has_handoff());
    let handoff = lab.people[a1].take_handoff().expect("handed off");
    std::fs::write(&gate, b"").expect("opens the gate: the child may crash now");
    assert_eq!(handoff.port, 45678, "the port the child printed");
    assert_eq!(handoff.cert_hash, [1; 32], "the hash the child printed");
    lab.until("the room back open for both, with the reason", |lab| {
        [a1, a2].into_iter().all(|who| {
            lab.phase(who) == Some(RoomPhase::Open)
                && lab.people[who]
                    .room
                    .as_ref()
                    .is_some_and(|r| r.note.contains("would not load"))
        })
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
    let script = fake_race("never_up", "exec sleep 30");
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
fn a_race_process_that_goes_quiet_is_stopped_and_the_slot_freed_within_ten_seconds() {
    let script = fake_race(
        "hung",
        "echo 'listening on [::]:45680'\n\
         echo 'cert-hash 0303030303030303030303030303030303030303030303030303030303030303'\n\
         exec sleep 60",
    );
    let mut lab = Lab::new(Box::new(ChildRunner {
        quiet_timeout: Duration::from_secs(2),
        ..*child_runner(script)
    }));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the hand-off", |lab| lab.people[a1].has_handoff());
    let up = std::time::Instant::now();
    lab.until("the room back open with a reason", |lab| {
        lab.phase(a1) == Some(RoomPhase::Open)
            && lab.people[a1]
                .room
                .as_ref()
                .is_some_and(|r| r.note.contains("went quiet"))
    });
    assert!(up.elapsed() < Duration::from_secs(10), "{:?}", up.elapsed());
    assert_eq!(lab.server.racing_room(), None, "the slot is free again");
}

#[test]
fn a_client_that_drops_while_its_race_loads_and_takes_its_session_over_receives_the_handoff() {
    let gate = gate("takeover");
    let script = fake_race(
        "takeover",
        &format!(
            "echo 'listening on [::]:45681'\n\
             echo 'cert-hash 0404040404040404040404040404040404040404040404040404040404040404'\n\
             while true; do echo 'health: waiting'; sleep 0.2; if [ -e '{}' ]; then exit 0; fi; done",
            gate.display()
        ),
    );
    let mut lab = Lab::new(child_runner(script));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the hand-off", |lab| lab.people[a1].has_handoff());
    let first = lab.people[a1].take_handoff().expect("handed off");
    // The link drops and the person comes back on a new one under their session token.
    let token = lab.people[a1].token.clone().expect("a session token");
    let link = oag_net::wt::connect(&lab.server.local_addr().to_string(), lab.server.cert_hash())
        .expect("connects to the lobby");
    lab.people[a1] =
        LobbyClient::connect(Box::new(link) as Box<dyn Link>, "a1", &[edition()], &token);
    lab.until("the hand-off again", |lab| lab.people[a1].has_handoff());
    assert_eq!(lab.people[a1].take_handoff(), Some(first));
    std::fs::write(&gate, b"").expect("lets the child end");
}

#[test]
fn a_session_whose_old_link_closes_before_the_new_one_says_hello_is_still_taken_over() {
    let gate = gate("takeover-close-first");
    let script = fake_race(
        "takeover_close_first",
        &format!(
            "echo 'listening on [::]:45682'\n\
             echo 'cert-hash 0505050505050505050505050505050505050505050505050505050505050505'\n\
             while true; do echo 'health: waiting'; sleep 0.2; if [ -e '{}' ]; then exit 0; fi; done",
            gate.display()
        ),
    );
    let mut lab = Lab::new(child_runner(script));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the hand-off", |lab| lab.people[a1].has_handoff());
    let first = lab.people[a1].take_handoff().expect("handed off");
    let token = lab.people[a1].token.clone().expect("a session token");
    // The old link goes first and the lobby reads the close before any new link exists.
    let (gone, _) = oag_net::link::Loopback::pair();
    lab.people[a1] = LobbyClient::connect(Box::new(gone) as Box<dyn Link>, "a1", &[edition()], "");
    for _ in 0..50 {
        lab.pump();
    }
    let link = oag_net::wt::connect(&lab.server.local_addr().to_string(), lab.server.cert_hash())
        .expect("connects to the lobby");
    lab.people[a1] =
        LobbyClient::connect(Box::new(link) as Box<dyn Link>, "a1", &[edition()], &token);
    lab.until("the hand-off again", |lab| lab.people[a1].has_handoff());
    assert_eq!(lab.people[a1].take_handoff(), Some(first));
    std::fs::write(&gate, b"").expect("lets the child end");
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

fn board() -> Board {
    Board {
        rows: vec![
            BoardRow {
                place: 1,
                slot: 1,
                name: "a2".into(),
                laps: 3,
                finish_ticks: Some(4_000),
                best_lap_ticks: Some(1_300),
                eliminations: 0,
            },
            BoardRow {
                place: 2,
                slot: 0,
                name: "a1".into(),
                laps: 3,
                finish_ticks: Some(4_100),
                best_lap_ticks: None,
                eliminations: 0,
            },
        ],
    }
}

/// A race process that comes up, waits at `first` (or, when an earlier run has been through, at
/// `later`), reports `board` and ends as a finished race.
fn finishing_race(
    name: &str,
    first: &std::path::Path,
    later: &std::path::Path,
    board: &Board,
) -> Box<ChildRunner> {
    let ran = fake_race_dir().join(format!("{name}.ran"));
    let _ = std::fs::remove_file(&ran);
    child_runner(fake_race(
        name,
        &format!(
            "echo 'listening on [::]:45679'\n\
             echo 'cert-hash 0202020202020202020202020202020202020202020202020202020202020202'\n\
             if [ -e '{ran}' ]; then {later}; else {first}; touch '{ran}'; fi\n\
             echo 'race-board {board}'\n\
             echo 'race-over every person has finished'\n\
             exit 0",
            ran = ran.display(),
            first = wait_at(first),
            later = wait_at(later),
            board = board.to_line()
        ),
    ))
}

#[test]
fn the_races_board_reaches_every_member_and_a_rematch_races_again_with_the_same_settings() {
    let (first, second) = (gate("board-1"), gate("board-2"));
    let mut lab = Lab::new(finishing_race("board", &first, &second, &board()));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.room_starts(a1, a2, ["Feisar", "Qirex"]);
    lab.until("the hand-off", |lab| {
        lab.people[a1].has_handoff() && lab.people[a2].has_handoff()
    });
    lab.people[a1].take_handoff();
    lab.people[a2].take_handoff();
    let settings = lab.people[a1].room.as_ref().expect("room").settings.clone();
    // The owner asks for a rematch while the race still runs; the guest has not.
    lab.people[a1].rematch();
    lab.until("the owner's ask is in", |lab| {
        lab.people[a2].room.as_ref().is_some_and(|r| r.rematch)
    });
    std::fs::write(&first, b"").expect("the race may end now");
    lab.until("the board in both rooms", |lab| {
        [a1, a2].into_iter().all(|who| {
            lab.people[who]
                .room
                .as_ref()
                .is_some_and(|r| !r.board.is_empty() && r.phase == RoomPhase::Open)
        })
    });
    for who in [a1, a2] {
        let room = lab.people[who].room.as_ref().expect("room");
        assert_eq!(room.board, board(), "member {who} holds the server's board");
        assert!(
            room.note.starts_with("every person has finished"),
            "{}",
            room.note
        );
    }
    // The guest agrees: the room starts again by itself, under the same settings.
    lab.people[a2].rematch();
    lab.until("a second hand-off for both", |lab| {
        lab.people[a1].has_handoff() && lab.people[a2].has_handoff()
    });
    for who in [a1, a2] {
        assert_eq!(
            lab.people[who].room.as_ref().expect("room").settings,
            settings,
            "the second race has the first race's settings"
        );
        assert!(lab.people[who].take_handoff().is_some());
    }
    std::fs::write(&second, b"").expect("the second race may end");
    lab.until("the second race over", |lab| {
        lab.phase(a1) == Some(RoomPhase::Open) && lab.server.racing_room().is_none()
    });
}

#[test]
fn a_removed_member_is_told_why_and_refused_back() {
    let mut lab = Lab::new(thread_runner(60));
    let (a1, a2) = (lab.join("a1"), lab.join("a2"));
    lab.people[a1].create(settings());
    lab.until("the room", |lab| lab.people[a1].room.is_some());
    let room = lab.people[a1].room.as_ref().expect("room").id;
    lab.people[a2].join(room);
    lab.until("both in", |lab| {
        lab.people[a1]
            .room
            .as_ref()
            .is_some_and(|r| r.members.len() == 2)
    });
    // A guest cannot remove the owner; the owner can remove the guest.
    lab.people[a2].kick("a1");
    lab.until("the refusal", |lab| lab.people[a2].refused.is_some());
    assert_eq!(
        lab.people[a2].refused.take().map(|(c, _)| c),
        Some(Refusal::NotOwner)
    );
    lab.people[a1].kick("a2");
    lab.until("a2 told", |lab| lab.people[a2].left.is_some());
    let why = lab.people[a2].left.clone().expect("a reason");
    assert!(why.contains("removed"), "{why}");
    lab.until("a1 sees the room alone", |lab| {
        lab.people[a1]
            .room
            .as_ref()
            .is_some_and(|r| r.members.len() == 1 && r.note.contains("a2 was removed"))
    });
    lab.people[a2].join(room);
    lab.until("the refusal on rejoin", |lab| {
        lab.people[a2].refused.is_some()
    });
    assert_eq!(
        lab.people[a2].refused.take().map(|(c, _)| c),
        Some(Refusal::Kicked)
    );
}

/// Dials the lobby the way a stranger would, without joining through the lab.
fn dial_lobby(lab: &Lab, name: &str) -> LobbyClient {
    let link = oag_net::wt::connect(&lab.server.local_addr().to_string(), lab.server.cert_hash())
        .expect("connects");
    LobbyClient::connect(Box::new(link) as Box<dyn Link>, name, &[edition()], "")
}

/// Pumps the lab and `extras` until every extra was refused, then checks the people who had
/// joined are still served: one makes a room.
fn extras_are_refused_and_the_first_are_still_served(
    lab: &mut Lab,
    extras: &mut [LobbyClient],
    joined: usize,
) {
    let started = std::time::Instant::now();
    while !extras.iter().all(|e| e.refused.is_some()) {
        assert!(started.elapsed() < Duration::from_secs(30), "no refusal");
        lab.pump();
        for extra in extras.iter_mut() {
            let _ = extra.poll();
        }
    }
    for extra in extras.iter() {
        let (code, text) = extra.refused.clone().expect("refused");
        assert_eq!(code, Refusal::Full, "{text}");
        assert!(!extra.welcomed());
    }
    assert_eq!(
        lab.server.connected(),
        joined,
        "the extras were never seated"
    );
    lab.people[0].create(settings());
    lab.until("a joined person's room", |lab| lab.people[0].room.is_some());
}

#[test]
fn one_address_may_hold_only_its_cap_of_lobby_connections_and_the_others_are_still_served() {
    let mut cap = config();
    cap.max_per_address = 4;
    let mut lab = Lab::with_config(cap, thread_runner(10));
    for i in 0..4 {
        lab.join(&format!("p{i}"));
    }
    let mut extras: Vec<LobbyClient> = (0..20)
        .map(|i| dial_lobby(&lab, &format!("x{i}")))
        .collect();
    extras_are_refused_and_the_first_are_still_served(&mut lab, &mut extras, 4);
}

#[test]
fn the_lobby_holds_only_its_cap_of_connections_whatever_their_addresses() {
    let mut cap = config();
    cap.max_connections = 3;
    let mut lab = Lab::with_config(cap, thread_runner(10));
    for i in 0..3 {
        lab.join(&format!("p{i}"));
    }
    let mut extras: Vec<LobbyClient> = (0..20)
        .map(|i| dial_lobby(&lab, &format!("x{i}")))
        .collect();
    extras_are_refused_and_the_first_are_still_served(&mut lab, &mut extras, 3);
}

fn transport_pair() -> (
    oag_net::wt::Listener,
    oag_net::wt::WtLink,
    oag_net::wt::WtLink,
    oag_net::wt::WtLink,
    oag_net::wt::WtLink,
) {
    use oag_net::wt::{Identity, LinkLimits, Listener, connect};
    let identity = Identity::self_signed(&[]).expect("certificate");
    let mut listener = Listener::bind(
        "127.0.0.1:0".parse().expect("addr"),
        &identity,
        LinkLimits::LOBBY,
    )
    .expect("binds");
    let addr = listener.local_addr().to_string();
    let mut first = connect(&addr, identity.hash()).expect("connects");
    first.send_reliable(vec![1]);
    let mut second = connect(&addr, identity.hash()).expect("connects");
    second.send_reliable(vec![2]);
    let mut served = Vec::new();
    let started = std::time::Instant::now();
    while served.len() < 2 {
        assert!(started.elapsed() < Duration::from_secs(10), "no sessions");
        if let Some(mut link) = listener.accept() {
            let who = loop {
                if let Some(bytes) = link.recv_reliable() {
                    break bytes[0];
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            served.push((who, link));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    served.sort_by_key(|(who, _)| *who);
    let mut served = served.into_iter().map(|(_, link)| link);
    (
        listener,
        first,
        served.next().expect("first"),
        second,
        served.next().expect("second"),
    )
}

#[test]
fn a_peer_that_never_reads_is_closed_at_the_send_queue_limit_and_the_other_is_still_served() {
    let (_listener, _idle, mut idle_server, mut good, mut good_server) = transport_pair();
    for _ in 0..400 {
        idle_server.send_reliable(vec![0; 64 << 10]);
    }
    let why = idle_server.closed().expect("the queue passed its limit");
    assert!(why.contains("does not drain"), "{why}");
    good_server.send_reliable(vec![7; 100]);
    let started = std::time::Instant::now();
    let got = loop {
        if let Some(bytes) = good.recv_reliable() {
            break bytes;
        }
        assert!(started.elapsed() < Duration::from_secs(5), "not served");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(got, vec![7; 100]);
    assert!(good_server.closed().is_none());
}

#[test]
fn a_flood_of_datagrams_is_cut_to_the_budget_and_the_other_peer_is_still_served() {
    use oag_net::wt::{DATAGRAM_BURST, DATAGRAM_RATE};
    let (_listener, mut flooder, mut flooded, mut good, mut good_server) = transport_pair();
    let started = std::time::Instant::now();
    let mut received = 0u32;
    for n in 0..3_000u32 {
        flooder.send_datagram(n.to_le_bytes().to_vec());
        if n % 20 == 19 {
            std::thread::sleep(Duration::from_millis(1));
            while flooded.recv_datagram().is_some() {
                received += 1;
            }
        }
    }
    std::thread::sleep(Duration::from_millis(300));
    while flooded.recv_datagram().is_some() {
        received += 1;
    }
    let allowed = DATAGRAM_BURST + DATAGRAM_RATE * (started.elapsed().as_secs() as u32 + 1);
    assert!(
        received <= allowed,
        "{received} datagrams passed, {allowed} allowed"
    );
    assert!(received >= 50, "the budget starved the peer: {received}");
    good.send_datagram(vec![9]);
    let t = std::time::Instant::now();
    loop {
        if good_server.recv_datagram() == Some(vec![9]) {
            break;
        }
        assert!(t.elapsed() < Duration::from_secs(5), "not served");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn sessions_nobody_takes_are_bounded_and_the_listener_keeps_accepting() {
    use oag_net::wt::{Identity, LinkLimits, Listener, connect};
    let identity = Identity::self_signed(&[]).expect("certificate");
    let mut listener = Listener::bind(
        "127.0.0.1:0".parse().expect("addr"),
        &identity,
        LinkLimits::LOBBY,
    )
    .expect("binds");
    let addr = listener.local_addr().to_string();
    let mut held = Vec::new();
    for _ in 0..300 {
        held.push(connect(&addr, identity.hash()).expect("connects"));
    }
    std::thread::sleep(Duration::from_millis(1_000));
    let mut taken = 0;
    while listener.accept().is_some() {
        taken += 1;
    }
    assert!(taken <= 256, "{taken} sessions queued");
    assert!(taken > 0);
    let mut later = connect(&addr, identity.hash()).expect("connects");
    later.send_reliable(vec![1]);
    let started = std::time::Instant::now();
    while listener.accept().is_none() {
        assert!(started.elapsed() < Duration::from_secs(5), "not accepted");
        std::thread::sleep(Duration::from_millis(1));
    }
}
