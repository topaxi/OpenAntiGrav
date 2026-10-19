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

/// The 100 ms round trip ADR-0071's table was measured at.
fn hundred_ms() -> Conditions {
    Conditions {
        one_way: web_time::Duration::from_millis(50),
        ..Conditions::default()
    }
}

fn race_with_weapons(
    people: u8,
    conditions: Conditions,
    ticks: u32,
    weapon_trigger: bool,
) -> Activity {
    race_in_mode(
        "single_race",
        people,
        conditions,
        ticks,
        weapon_trigger,
        false,
    )
}

/// `hand`: the clients fly through a `Hand`, an analog stick, instead of the driver's raw wants.
fn race_in_mode(
    mode: &str,
    people: u8,
    conditions: Conditions,
    ticks: u32,
    weapon_trigger: bool,
    hand: bool,
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
            let link: Box<dyn Link> = Box::new(Shim::new(client_end, conditions));
            let mut client = Client::connect(link, &format!("bot {n}"));
            client.set_weapon_trigger(weapon_trigger);
            client.set_bot_hand(hand.then_some(0x4841_4E44 + u64::from(n)));
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
    let per =
        |f: fn(&oag_net::client::Stats) -> u32| -> Vec<u32> { totals.iter().map(f).collect() };
    println!(
        "TABLE hand={hand} people={people} loss={} jitter_ms={} rtt_ms={}: \
         corrections {:?}, replayed {:?}, wrong-weapon-state ticks {:?}, \
         pop-in age max {:?}, bad hashes {:?}, late cues {:?}, late blasts {:?}, late oldest {:?}, \
         mean error {:?}, largest error {:?}, largest pop {:?}, own craft moved {:?}, own pop max {:?}",
        conditions.loss,
        conditions.jitter.as_millis(),
        conditions.one_way.as_millis() * 2,
        per(|t| t.corrections),
        per(|t| t.replayed),
        per(|t| t.weapon_mismatches),
        per(|t| t.oldest_appeared),
        per(|t| t.hash_mismatches),
        per(|t| t.late_cues),
        per(|t| t.late_blasts),
        per(|t| t.late_oldest),
        totals
            .iter()
            .map(|t| t.error_sum / t.error_count.max(1) as f32)
            .collect::<Vec<_>>(),
        totals.iter().map(|t| t.largest_error).collect::<Vec<_>>(),
        totals.iter().map(|t| t.largest_pop).collect::<Vec<_>>(),
        per(|t| t.own_moved),
        totals.iter().map(|t| t.own_pop_max).collect::<Vec<_>>(),
    );
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
    assert_weapons_were_used(&race_with_weapons(2, hundred_ms(), 5400, true));
}

#[test]
#[ignore = "needs a disc image"]
fn two_firing_clients_position_trigger_only_at_100_ms() {
    assert_weapons_were_used(&race_with_weapons(2, hundred_ms(), 5400, false));
}

#[test]
#[ignore = "needs a disc image"]
fn four_firing_clients_agree_with_the_server_at_100_ms() {
    assert_weapons_were_used(&race_with_weapons(4, hundred_ms(), 5400, true));
}

#[test]
#[ignore = "needs a disc image"]
fn four_firing_clients_position_trigger_only_at_100_ms() {
    assert_weapons_were_used(&race_with_weapons(4, hundred_ms(), 5400, false));
}

/// Eliminator needs weapons: two people race one over the wire, the ends agree, and the kills
/// that decide it happen on the server.
#[test]
#[ignore = "needs a disc image"]
fn an_eliminator_race_is_flown_over_the_wire_by_two() {
    let activity = race_in_mode("eliminator", 2, hundred_ms(), 5400, true, false);
    assert_weapons_were_used(&activity);
    assert!(activity.finished, "the kills ended the race: {activity:?}");
}

/// 100 ms, 5 % loss and 30 ms jitter, four clients: the worst of the sweep below, run on every
/// `just test-data` at a shorter length. The hashes must still agree on every confirmed tick.
#[test]
#[ignore = "needs a disc image"]
fn four_firing_clients_agree_through_5_percent_loss_and_jitter() {
    let conditions = Conditions {
        loss: 0.05,
        jitter: web_time::Duration::from_millis(30),
        ..hundred_ms()
    };
    assert_weapons_were_used(&race_with_weapons(4, conditions, 3600, true));
}

/// The loss and jitter table in ADR-0071's follow-up: a sweep that prints, run with `OAG_SWEEP=1`.
#[test]
#[ignore = "sweep: prints the loss and jitter table; set OAG_SWEEP=1"]
fn loss_and_jitter_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    for people in [2, 4] {
        for (loss, jitter) in [(0.0, 0), (0.01, 0), (0.05, 0), (0.0, 30), (0.05, 30)] {
            let conditions = Conditions {
                loss,
                jitter: web_time::Duration::from_millis(jitter),
                ..hundred_ms()
            };
            assert_weapons_were_used(&race_with_weapons(people, conditions, 5400, true));
        }
    }
}

/// ADR-0069 condition 3: one cell of the analog-stick table, the driver's raw wants (`hand`
/// false) and the same driver through a `Hand` (true) in one session so the two are read
/// against one machine's load. Run with `OAG_SWEEP=1`.
fn hand_cell(people: u8, loss: f32, jitter_ms: u64, hand: bool) {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let conditions = Conditions {
        loss,
        jitter: web_time::Duration::from_millis(jitter_ms),
        ..hundred_ms()
    };
    assert_weapons_were_used(&race_in_mode(
        "single_race",
        people,
        conditions,
        5400,
        true,
        hand,
    ));
}

macro_rules! hand_cells {
    ($($name:ident: $people:expr, $loss:expr, $jitter:expr, $hand:expr;)*) => {$(
        #[test]
        #[ignore = "sweep: prints one cell of the analog-stick table; set OAG_SWEEP=1"]
        fn $name() {
            hand_cell($people, $loss, $jitter, $hand);
        }
    )*};
}

hand_cells! {
    sweep_bot_2_clean: 2, 0.0, 0, false;
    sweep_hand_2_clean: 2, 0.0, 0, true;
    sweep_bot_2_lossy: 2, 0.05, 30, false;
    sweep_hand_2_lossy: 2, 0.05, 30, true;
    sweep_bot_4_clean: 4, 0.0, 0, false;
    sweep_hand_4_clean: 4, 0.0, 0, true;
    sweep_bot_4_lossy: 4, 0.05, 30, false;
    sweep_hand_4_lossy: 4, 0.05, 30, true;
}
