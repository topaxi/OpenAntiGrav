//! A person whose link drops comes back: in a room of one, and as a client process that was
//! killed and started again with nothing but the seat token.

use oag_gameplay::Controller;
use oag_net::client::{Client, Phase};
use oag_net::host::Host;
use oag_net::link::Loopback;
use oag_raceplay::Race;

use crate::loopback::{Pace, Pilot, params, pilot, ready_all, tokened};

fn holding_turbo(params: &oag_net::wire::RaceParams) -> Box<Race> {
    let mut race = crate::loopback::headless(params);
    for ship in race
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

/// A room of one whose person drops: the seat is flown by the driver as `Away` while the link
/// is gone, handed back on the return, and the client's hashes agree throughout.
#[test]
fn the_only_person_in_a_room_goes_away_and_comes_back() {
    let params = params(1);
    let mut host = Host::new(holding_turbo(&params), params.clone());
    let (mut clients, tokens) = tokened(&mut host, 1, &mut || holding_turbo(&params));
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    let (mut mismatches, mut matches) = (0, 0);
    let mut away_seen = false;
    let mut handed_back = false;
    while host.tick_number() < 1300 {
        host.poll();
        host.tick();
        let at = host.tick_number();
        let world = &host.race().sim.world;
        if (700..900).contains(&at) {
            away_seen |= world.controllers[0] == Controller::Away;
            assert!(!world.person_flown(0), "the driver flies the away seat");
        }
        handed_back |= at > 900 && world.ships[0].handback;
        let (client, race, pilot) = &mut clients[0];
        if (600..900).contains(&at) {
            if at == 600 {
                client.leave("link lost");
            }
            continue;
        }
        if at == 900 {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            client.reconnect(Box::new(client_end), "client 0", "", &tokens[0]);
        }
        let own = pilot(client, race);
        client.step(race, &own);
        let stats = client.take_stats();
        mismatches += stats.hash_mismatches;
        matches += stats.hash_matches;
    }
    host.end("the test is over");
    assert!(away_seen, "the seat was Away while the link was gone");
    assert!(handed_back, "and handed back on the return");
    assert_eq!(mismatches, 0);
    assert!(matches > 600, "{matches}");
    assert_eq!(host.race().sim.world.controllers[0], Controller::Remote);
}

/// A client process that was killed mid-race: a fresh process builds a fresh race, says hello
/// with the token it persisted, is told the same race, and catches up to the host's ticks with
/// every hash agreeing.
#[test]
fn a_fresh_process_with_the_token_takes_the_seat_and_catches_up() {
    let params = params(2);
    let mut host = Host::new(holding_turbo(&params), params.clone());
    let (mut clients, tokens) = tokened(&mut host, 2, &mut || holding_turbo(&params));
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    let mut fresh: Option<(Client, Box<Race>, Pilot)> = None;
    let (mut mismatches, mut matches) = (0, 0);
    while host.tick_number() < 1400 {
        host.poll();
        host.tick();
        let at = host.tick_number();
        for (n, (client, race, pilot)) in clients.iter_mut().enumerate() {
            if n == 1 {
                if at == 600 {
                    client.leave("process killed");
                }
                if at >= 600 {
                    continue;
                }
            }
            let own = pilot(client, race);
            client.step(race, &own);
        }
        if at == 900 {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let mut client = Client::connect_seat(Box::new(client_end), "client 1", "", &tokens[1]);
            let mut race = holding_turbo(&params);
            let asked = std::time::Instant::now();
            loop {
                host.poll();
                if client.poll_welcome().expect("welcomed").is_some() {
                    break;
                }
                assert!(asked.elapsed().as_secs() < 5, "no welcome");
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            client.ready(&mut race).expect("the same start state");
            fresh = Some((client, race, pilot(3)));
        }
        if let Some((client, race, pilot)) = fresh.as_mut() {
            let own = pilot(client, race);
            client.step(race, &own);
            let stats = client.take_stats();
            mismatches += stats.hash_mismatches;
            matches += stats.hash_matches;
        }
    }
    host.end("the test is over");
    let (client, race, _) = fresh.expect("the fresh process was started");
    assert_ne!(*client.phase(), Phase::Joining);
    assert_eq!(mismatches, 0);
    assert!(matches > 300, "caught up and checked the ticks: {matches}");
    assert!(
        race.sim.world.tick > 1200,
        "the fresh race is near the host's: {}",
        race.sim.world.tick
    );
}

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use oag_net::link::{Cutter, Link};
use oag_net::redial::{Dialed, Dialer, Identity, Redial};

/// Dials by making a loopback pair, queueing the server's end for the host and handing the
/// client's, after `refuse` dials that fail.
struct LoopbackDialer {
    to_host: Rc<RefCell<VecDeque<Box<dyn Link>>>>,
    cutters: Rc<RefCell<Vec<Cutter>>>,
    refuse: u32,
    began: u32,
    pending: bool,
}

impl Dialer for LoopbackDialer {
    fn begin(&mut self) {
        self.began += 1;
        self.pending = true;
    }
    fn poll(&mut self) -> Dialed {
        if !std::mem::take(&mut self.pending) {
            return Dialed::Pending;
        }
        if self.began <= self.refuse {
            return Dialed::Failed("connection refused".into());
        }
        let (server_end, client_end) = Loopback::pair();
        self.cutters.borrow_mut().push(client_end.cutter());
        self.to_host.borrow_mut().push_back(Box::new(server_end));
        Dialed::Link(Box::new(client_end))
    }
}

static QUICK: [std::time::Duration; 2] = [
    std::time::Duration::from_millis(1),
    std::time::Duration::from_millis(2),
];

/// Two people seated over cuttable links; returns each client's cutter.
fn seated_cuttable(
    host: &mut Host,
    params: &oag_net::wire::RaceParams,
) -> (Vec<(Client, Box<Race>, Pilot)>, Vec<String>, Vec<Cutter>) {
    let tokens: Vec<String> = (0..2).map(|n| format!("token-{n}")).collect();
    host.expect_seats(
        (0..2)
            .map(|n| (format!("client {n}"), tokens[n].clone()))
            .collect(),
    );
    let mut cutters = Vec::new();
    let clients = (0..2u32)
        .map(|n| {
            let (server_end, client_end) = Loopback::pair();
            cutters.push(client_end.cutter());
            host.join(Box::new(server_end));
            let client = Client::connect_seat(
                Box::new(client_end),
                &format!("client {n}"),
                "",
                &tokens[n as usize],
            );
            (client, holding_turbo(params), pilot(n * 3))
        })
        .collect();
    (clients, tokens, cutters)
}

/// The link is cut under a running race and the client dials again by itself, through failed
/// attempts, and ends up in its seat with every hash agreeing.
#[test]
fn a_client_whose_link_is_cut_redials_and_rejoins_without_a_relaunch() {
    let params = params(2);
    let mut host = Host::new(holding_turbo(&params), params.clone());
    let (mut clients, tokens, cutters) = seated_cuttable(&mut host, &params);
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    let to_host = Rc::new(RefCell::new(VecDeque::new()));
    let new_cutters = Rc::new(RefCell::new(Vec::new()));
    let mut dialer = LoopbackDialer {
        to_host: to_host.clone(),
        cutters: new_cutters,
        refuse: 3,
        began: 0,
        pending: false,
    };
    let identity = Identity {
        name: "client 1".into(),
        team: String::new(),
        token: tokens[1].clone(),
    };
    let mut redial = Redial::with(identity, &QUICK, std::time::Duration::from_secs(20));
    let (mut mismatches, mut matches) = ([0u32; 2], [0u32; 2]);
    let (mut lost_seen, mut away_seen) = (false, false);
    while host.tick_number() < 1500 {
        host.poll();
        host.tick();
        for link in to_host.borrow_mut().drain(..) {
            host.join(link);
        }
        let at = host.tick_number();
        if at == 600 {
            cutters[1].cut();
        }
        away_seen |= host.race().sim.world.controllers[1] == Controller::Away;
        for (n, (client, race, pilot)) in clients.iter_mut().enumerate() {
            if n == 1 {
                redial.step(client, &mut dialer, web_time::Instant::now());
                lost_seen |= matches!(client.phase(), Phase::Lost(_));
                if matches!(client.phase(), Phase::Lost(_)) {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }
            }
            let own = pilot(client, race);
            client.step(race, &own);
            let stats = client.take_stats();
            mismatches[n] += stats.hash_mismatches;
            matches[n] += stats.hash_matches;
        }
    }
    host.end("the test is over");
    assert!(
        lost_seen,
        "the cut was seen as a lost link, not an ended race"
    );
    assert!(away_seen, "the host held the seat away meanwhile");
    assert_eq!(
        dialer.began, 4,
        "three refused dials, then one that connected"
    );
    assert_eq!(clients[1].0.phase(), &Phase::Racing);
    assert_eq!(mismatches, [0, 0]);
    assert!(matches[1] > 300, "{}", matches[1]);
    assert_eq!(host.race().sim.world.controllers[1], Controller::Remote);
}

/// A seat nobody holds any more is not waited for for ever: the client gives up and says so.
#[test]
fn a_client_that_cannot_get_back_gives_up() {
    let params = params(2);
    let mut host = Host::new(holding_turbo(&params), params.clone());
    let (mut clients, tokens, cutters) = seated_cuttable(&mut host, &params);
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    cutters[1].cut();
    let mut dialer = LoopbackDialer {
        to_host: Rc::new(RefCell::new(VecDeque::new())),
        cutters: Rc::new(RefCell::new(Vec::new())),
        refuse: u32::MAX,
        began: 0,
        pending: false,
    };
    let identity = Identity {
        name: "client 1".into(),
        team: String::new(),
        token: tokens[1].clone(),
    };
    let mut redial = Redial::with(identity, &QUICK, std::time::Duration::from_millis(60));
    let (client, race, _) = &mut clients[1];
    client.step(race, &oag_gameplay::InputSnapshot::EMPTY);
    assert!(matches!(client.phase(), Phase::Lost(_)));
    let began = std::time::Instant::now();
    while !matches!(client.phase(), Phase::Over(_)) {
        redial.step(client, &mut dialer, web_time::Instant::now());
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(began.elapsed().as_secs() < 5, "it never gave up");
    }
    let Phase::Over(why) = client.phase() else {
        unreachable!()
    };
    assert!(why.contains("could not be restored"), "{why}");
    assert!(redial.attempts() > 2);
}
