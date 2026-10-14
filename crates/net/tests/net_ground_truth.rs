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
