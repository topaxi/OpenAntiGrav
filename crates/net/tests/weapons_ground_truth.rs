//! A networked race with weapons on, on a real circuit, in one process: a server and N
//! clients over [`Loopback`] behind a round trip ([`Shim`]), every client flown by the player's
//! own driver, which fires what it holds (`Client::bot_input`), and the rest of the grid the AI.
//! The claims: weapons really fire and hit on the server, every confirmed tick's state hash
//! agrees on every client, and what the correction trigger costs is printed for ADR-0071.

use oag_net::client::Client;
use oag_net::host::Host;
use oag_net::link::{Conditions, Link, Loopback, Shim};
use oag_net::wire::RaceParams;

use crate::loopback::{Pilot, drive_observed};

const IMAGE: &str = "pulse-psp-eu.chd";

fn params(people: u8, mode: &str) -> RaceParams {
    RaceParams {
        track: "Data\\Environments\\01_Track\\track.vex".into(),
        class: "VENOM".into(),
        mode: mode.into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 0x5EED,
        people,
        opponents: true,
        weapons: true,
        ..RaceParams::default()
    }
}

/// What the server's own simulation did with weapons: proof the race had any.
#[derive(Debug, Default)]
struct Activity {
    peak_in_air: usize,
    fires: u32,
    hits: u32,
    finished: bool,
}

fn race_with_weapons(people: u8, one_way_ms: u64, ticks: u32, weapon_trigger: bool) -> Activity {
    race_in_mode("single_race", people, one_way_ms, ticks, weapon_trigger)
}

fn race_in_mode(
    mode: &str,
    people: u8,
    one_way_ms: u64,
    ticks: u32,
    weapon_trigger: bool,
) -> Activity {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return Activity::default();
    };
    let source = image.display().to_string();
    let mut params = params(people, mode);
    let (race, title) = oag_net::race::load(&params, &source).expect("loads");
    params.title = title.to_string();
    let mut host = Host::new(race, params.clone());
    let mut clients: Vec<_> = (0..people)
        .map(|n| {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let link: Box<dyn Link> = Box::new(Shim::new(
                client_end,
                Conditions {
                    one_way: web_time::Duration::from_millis(one_way_ms),
                    loss: 0.0,
                },
            ));
            let mut client = Client::connect(link, &format!("bot {n}"));
            client.set_weapon_trigger(weapon_trigger);
            let (race, _) = oag_net::race::load(&params, &source).expect("loads");
            let pilot: Pilot = Box::new(|client, race| client.bot_input(race));
            (client, race, pilot)
        })
        .collect();
    let mut activity = Activity::default();
    let mut held = [false; 8];
    let mut shield = [f32::MAX; 8];
    let totals = drive_observed(
        &mut host,
        &mut clients,
        ticks,
        std::time::Duration::from_micros(16_667),
        |host| {
            let world = &host.race().sim.world;
            activity.peak_in_air = activity.peak_in_air.max(world.projectiles.live());
            for slot in 0..usize::from(people) {
                let ship = &world.ships[slot];
                let has = ship.pickup.weapon.is_some();
                activity.fires += u32::from(held[slot] && !has);
                held[slot] = has;
                activity.hits += u32::from(ship.physics.shield < shield[slot] - 1.0);
                shield[slot] = ship.physics.shield;
            }
        },
    );
    println!("server, per seat: {:?}", host.take_counts());
    println!("server weapons: {activity:?}");
    activity.finished = host.race().finished();
    for (n, total) in totals.iter().enumerate() {
        println!("trigger={weapon_trigger} client {n}: {total:?}");
        assert_eq!(total.hash_mismatches, 0, "client {n}");
        assert_eq!(total.hash_matches, ticks, "client {n}");
    }
    activity
}

fn assert_weapons_were_used(activity: &Activity) {
    assert!(
        activity.peak_in_air > 0,
        "nothing was ever in the air: {activity:?}"
    );
    assert!(activity.fires > 0, "nobody ever fired: {activity:?}");
}

#[test]
#[ignore = "needs a disc image"]
fn two_firing_clients_agree_with_the_server_at_100_ms() {
    assert_weapons_were_used(&race_with_weapons(2, 50, 5400, true));
}

#[test]
#[ignore = "needs a disc image"]
fn two_firing_clients_position_trigger_only_at_100_ms() {
    assert_weapons_were_used(&race_with_weapons(2, 50, 5400, false));
}

#[test]
#[ignore = "needs a disc image"]
fn four_firing_clients_agree_with_the_server_at_100_ms() {
    assert_weapons_were_used(&race_with_weapons(4, 50, 5400, true));
}

#[test]
#[ignore = "needs a disc image"]
fn four_firing_clients_position_trigger_only_at_100_ms() {
    assert_weapons_were_used(&race_with_weapons(4, 50, 5400, false));
}

/// Eliminator needs weapons: two people race one over the wire, the ends agree, and the kills
/// that decide it happen on the server.
#[test]
#[ignore = "needs a disc image"]
fn an_eliminator_race_is_flown_over_the_wire_by_two() {
    let activity = race_in_mode("eliminator", 2, 50, 5400, true);
    assert_weapons_were_used(&activity);
    assert!(activity.finished, "the kills ended the race: {activity:?}");
}
