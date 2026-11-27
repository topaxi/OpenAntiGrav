//! The windowed game's half of getting back into a race: `NetPlay` redials by itself after its
//! link is cut, writes the seat down when the race starts and clears it when the race ends, and
//! a new process rejoins from what the killed one wrote. A host and its clients in one process
//! over loopback links, no disc.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use oag_game::net_play::rejoin::Rejoin;
use oag_game::net_play::{NetPlay, Pending};
use oag_game::rejoin_record;
use oag_gameplay::Controller;
use oag_net::host::Host;
use oag_net::link::{Cutter, Link, Loopback};
use oag_net::redial::{Dialed, Dialer};
use oag_net::wire::RaceParams;
use oag_raceplay::{Race, SimSetup};

fn params() -> RaceParams {
    RaceParams {
        title: String::new(),
        mode: "single_race".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 0xC0FFEE,
        people: 2,
        ..RaceParams::default()
    }
}

fn race(params: &RaceParams) -> Box<Race> {
    let mut race = Box::new(Race::start(
        SimSetup::headless(oag_race::Mode::SingleRace, params.seed),
        oag_raceplay::ViewSetup::headless(),
        oag_raceplay::extensions::Prepared::default(),
    ));
    oag_net::race::prepare(&mut race, params).expect("prepares");
    race
}

fn rejoin(n: usize) -> Rejoin {
    Rejoin {
        addr: "127.0.0.1:1".into(),
        cert_hash: [9; 32],
        room: 0,
        team: String::new(),
        name: format!("client {n}"),
        token: format!("token-{n}"),
    }
}

/// Dials by making a loopback pair for the host, after `refuse` dials that fail.
struct LoopbackDialer {
    to_host: Rc<RefCell<VecDeque<Box<dyn Link>>>>,
    refuse: u32,
    began: Rc<RefCell<u32>>,
    pending: bool,
}

impl Dialer for LoopbackDialer {
    fn begin(&mut self) {
        *self.began.borrow_mut() += 1;
        self.pending = true;
    }
    fn poll(&mut self) -> Dialed {
        if !std::mem::take(&mut self.pending) {
            return Dialed::Pending;
        }
        if *self.began.borrow() <= self.refuse {
            return Dialed::Failed("connection refused".into());
        }
        let (server_end, client_end) = Loopback::pair();
        self.to_host.borrow_mut().push_back(Box::new(server_end));
        Dialed::Link(Box::new(client_end))
    }
}

struct Room {
    host: Host,
    params: RaceParams,
    cutters: Vec<Cutter>,
    nets: Vec<(NetPlay, Box<Race>)>,
}

/// Two bot clients seated on the host by token, joined and started.
fn room(files: [&std::path::Path; 2], dialers: [Option<Box<dyn Dialer>>; 2]) -> Room {
    let params = params();
    let mut host = Host::new(race(&params), params.clone());
    host.expect_seats(vec![
        ("client 0".into(), "token-0".into()),
        ("client 1".into(), "token-1".into()),
    ]);
    let mut cutters = Vec::new();
    let mut nets = Vec::new();
    for (n, dialer) in dialers.into_iter().enumerate() {
        let (server_end, client_end) = Loopback::pair();
        cutters.push(client_end.cutter());
        host.join(Box::new(server_end));
        let mut pending = Pending::new(
            Box::new(client_end),
            &format!("client {n}"),
            "",
            &format!("token-{n}"),
            true,
            "test",
            Some((rejoin(n), None)),
        );
        if let Some(dialer) = dialer {
            pending = pending.with_dialer(dialer);
        }
        let net = loop {
            host.poll();
            if let Some(net) = pending.poll().expect("welcomed") {
                break net;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        net.record_in(files[n].to_path_buf());
        let mut own = race(&params);
        net.begin_join(&mut own, "").expect("same start state");
        nets.push((net, own));
    }
    while !host.all_ready() {
        host.poll();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    host.start(0);
    for (net, _) in &nets {
        while !net.poll_start().expect("started") {
            host.poll();
        }
    }
    Room {
        host,
        params,
        cutters,
        nets,
    }
}

fn scratch(name: &str) -> oag_testdata::scratch::TempDir {
    let dir = oag_testdata::scratch::dir(&format!("net-play-rejoin-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One round: the host ticks, every client takes its step. `dead` is a client that is not
/// running (its process is gone).
fn round(room: &mut Room, dead: Option<usize>, to_host: &Rc<RefCell<VecDeque<Box<dyn Link>>>>) {
    room.host.poll();
    room.host.tick();
    for link in to_host.borrow_mut().drain(..) {
        room.host.join(link);
    }
    for (n, (net, race)) in room.nets.iter_mut().enumerate() {
        if dead == Some(n) {
            continue;
        }
        net.step(race, oag_gameplay::InputSnapshot::EMPTY);
    }
}

/// The link under a running race is cut: the client dials again by itself, through refused
/// attempts, is seated, and its hashes agree with the server's from then on.
#[test]
fn a_cut_link_is_redialled_by_the_game_without_a_relaunch() {
    let dir = scratch("redial");
    let to_host = Rc::new(RefCell::new(VecDeque::new()));
    let began = Rc::new(RefCell::new(0));
    let dialer = LoopbackDialer {
        to_host: to_host.clone(),
        refuse: 2,
        began: began.clone(),
        pending: false,
    };
    let mut room = room(
        [&dir.join("a.toml"), &dir.join("b.toml")],
        [None, Some(Box::new(dialer))],
    );
    let (mut was_reconnecting, mut away) = (false, false);
    let mut overlay = None;
    while room.host.tick_number() < 1600 {
        round(&mut room, None, &to_host);
        if room.host.tick_number() == 600 {
            room.cutters[1].cut();
        }
        was_reconnecting |= room.nets[1].0.reconnecting();
        if let Some(seconds) = room.nets[1].0.reconnect_seconds_left() {
            overlay = Some(seconds);
        }
        away |= room.host.race().core.sim.world.controllers[1] == Controller::Away;
        if room.nets[1].0.reconnecting() {
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
    }
    room.host.end("the test is over");
    let (net, _) = &room.nets[1];
    assert!(was_reconnecting, "the game saw the link go and dialled");
    assert!(away, "the host held the seat meanwhile");
    assert!(
        *began.borrow() >= 3,
        "refused twice, then through: {}",
        began.borrow()
    );
    assert!(!net.reconnecting(), "and is back");
    assert_eq!(
        net.reconnect_seconds_left(),
        None,
        "the overlay is gone after the redial"
    );
    let seconds = overlay.expect("the overlay had seconds to show while reconnecting");
    assert!((1..=30).contains(&seconds), "{seconds}");
    assert_eq!(net.over(), None);
    let (ok, bad) = net.hashes();
    assert_eq!(bad, 0);
    assert!(ok > 400, "the hashes were checked after the return: {ok}");
    assert_eq!(
        room.host.race().core.sim.world.controllers[1],
        Controller::Remote
    );
}

/// The race starting writes the seat down, a killed process leaves it, a new process builds
/// a fresh race from it and takes the seat with every hash agreeing, and the race ending
/// deletes it.
#[test]
fn a_killed_game_rejoins_from_the_seat_it_wrote_down() {
    let dir = scratch("kill");
    let (file_a, file_b) = (dir.join("a.toml"), dir.join("b.toml"));
    let mut room = room([&file_a, &file_b], [None, None]);
    let to_host = Rc::new(RefCell::new(VecDeque::new()));
    let now = rejoin_record::now();
    let written =
        rejoin_record::load_from(&file_b, "", now).expect("the race starting wrote the seat");
    assert_eq!(written.token, "token-1");
    assert_eq!(written.name, "client 1");

    while room.host.tick_number() < 600 {
        round(&mut room, None, &to_host);
    }
    // The process dies: nothing runs its drop, and its link goes with it.
    let (killed, _) = room.nets.remove(1);
    std::mem::forget(killed);
    room.cutters[1].cut();
    while room.host.tick_number() < 900 {
        room.nets.truncate(1);
        round(&mut room, None, &to_host);
    }
    assert!(
        rejoin_record::load_from(&file_b, "", rejoin_record::now()).is_some(),
        "a killed game leaves its seat behind"
    );
    assert_eq!(
        room.host.race().core.sim.world.controllers[1],
        Controller::Away
    );

    // A new process: reads the record, builds the race from nothing, says hello on the token.
    let seat = rejoin_record::load_from(&file_b, "", rejoin_record::now()).expect("a fresh seat");
    let (server_end, client_end) = Loopback::pair();
    room.host.join(Box::new(server_end));
    let mut pending = Pending::new(
        Box::new(client_end),
        &seat.name,
        &seat.team,
        &seat.token,
        true,
        "the record",
        Some((seat.clone(), None)),
    );
    let net = loop {
        room.host.poll();
        if let Some(net) = pending.poll().expect("welcomed") {
            break net;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    net.record_in(file_b.clone());
    let mut fresh = race(&room.params);
    net.begin_join(&mut fresh, "")
        .expect("the same start state");
    while !net.poll_start().expect("started") {
        room.host.poll();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    room.nets.push((net, fresh));
    while room.host.tick_number() < 1500 {
        round(&mut room, None, &to_host);
    }
    let (net, race) = &room.nets[1];
    let (ok, bad) = net.hashes();
    assert_eq!(bad, 0, "hashes agree");
    assert!(ok > 400, "caught up and checked: {ok}");
    assert!(
        race.core.sim.world.tick > 1400,
        "{}",
        race.core.sim.world.tick
    );

    // The race ends: the seat is deleted, and the offer would have nothing to show.
    room.host.end("the test is over");
    for _ in 0..30 {
        round(&mut room, None, &to_host);
    }
    assert!(!file_b.exists(), "the record is deleted when the race ends");
}

/// BACK while the game waits for its seat ends the race for this client on that call.
#[test]
fn giving_up_while_reconnecting_ends_the_race_at_once() {
    let dir = scratch("giveup");
    let to_host = Rc::new(RefCell::new(VecDeque::new()));
    let dialer = LoopbackDialer {
        to_host: to_host.clone(),
        refuse: u32::MAX,
        began: Rc::new(RefCell::new(0)),
        pending: false,
    };
    let mut room = room(
        [&dir.join("a.toml"), &dir.join("b.toml")],
        [None, Some(Box::new(dialer))],
    );
    room.cutters[1].cut();
    while !room.nets[1].0.reconnecting() {
        round(&mut room, None, &to_host);
    }
    assert_eq!(room.nets[1].0.over(), None, "still waiting for the seat");
    room.nets[1].0.give_up();
    assert!(!room.nets[1].0.reconnecting());
    let why = room.nets[1]
        .0
        .over()
        .expect("the race is over for this client");
    assert!(why.contains("left"), "{why}");
}
