//! A LAN host's waiting room: welcomes are held until START fixes the head count, then
//! the waiting links are seated in arrival order and the race runs with every hash agreeing.

use oag_net::client::Client;
use oag_net::host::Host;
use oag_net::link::{Link, Loopback};
use oag_net::lobby::Lobby;

use crate::loopback::{headless, params, pilot};

#[test]
fn welcomes_wait_for_start_then_seat_in_arrival_order_and_the_race_agrees() {
    let mut lobby = Lobby::new();
    let mut clients = Vec::new();
    for (name, team) in [("host", "feisar"), ("guest", "qirex")] {
        let (server_end, client_end) = Loopback::pair();
        lobby.join(Box::new(server_end));
        clients.push(Client::connect_as(
            Box::new(client_end) as Box<dyn Link>,
            name,
            team,
        ));
    }
    for _ in 0..50 {
        lobby.poll();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(lobby.names(), ["host", "guest"]);
    assert_eq!(
        lobby.teams(),
        ["feisar", "qirex"],
        "each pick waits with its name"
    );
    for client in &mut clients {
        assert!(
            client.poll_welcome().expect("no refusal").is_none(),
            "a welcome came before START"
        );
    }
    // START: two people, so the race has two.
    let mut params = params(u8::try_from(lobby.len()).expect("small"));
    params.teams = lobby.teams();
    let mut host = Host::new(headless(&params), params.clone());
    for (name, link) in lobby.take() {
        host.seat(link, name).expect("a free seat");
    }
    let mut seated = Vec::new();
    for (n, mut client) in clients.into_iter().enumerate() {
        let welcome = client
            .poll_welcome()
            .expect("welcomed")
            .expect("the welcome arrived after START");
        assert_eq!(welcome.seat, n, "arrival order is seat order");
        assert_eq!(welcome.params.people, 2);
        assert_eq!(
            welcome.params.teams,
            ["feisar", "qirex"],
            "every seat learns every pick"
        );
        seated.push((
            client,
            headless(&params),
            pilot(u32::try_from(n).unwrap() * 3),
        ));
    }
    // A third person after START is refused by name, not left hanging.
    let (server_end, client_end) = Loopback::pair();
    let mut late = Client::connect(Box::new(client_end) as Box<dyn Link>, "late");
    host.join(Box::new(server_end));
    host.poll();
    let refused = late.poll_welcome().expect_err("a full host refuses");
    assert!(
        format!("{refused}").contains("every seat is taken"),
        "{refused}"
    );

    // The race they were welcomed to runs, and every confirmed tick's hash agrees.
    for (client, race, _) in &mut seated {
        client.ready(race).expect("the same start state");
    }
    let asked = std::time::Instant::now();
    while !host.all_ready() {
        host.poll();
        assert!(asked.elapsed().as_secs() < 5, "{:?}", host.seat_names());
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    host.start(0);
    let mut matches = vec![0; seated.len()];
    for _ in 0..120 {
        host.poll();
        host.tick();
        for ((client, race, pilot), count) in seated.iter_mut().zip(&mut matches) {
            let own = pilot(client, race);
            client.step(race, &own);
            let stats = client.take_stats();
            assert_eq!(stats.hash_mismatches, 0, "{stats:?}");
            *count += stats.hash_matches;
        }
    }
    assert!(matches.iter().all(|&count| count > 100), "{matches:?}");
}
