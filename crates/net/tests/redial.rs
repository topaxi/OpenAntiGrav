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
    ready_all(&mut host, &mut clients, &Pace::Wall(std::time::Duration::ZERO));
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
    ready_all(&mut host, &mut clients, &Pace::Wall(std::time::Duration::ZERO));
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
