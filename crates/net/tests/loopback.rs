//! A server and its clients in one process over [`Loopback`], no disc: the join,
//! the start, and the state hash agreeing on every confirmed tick, with and
//! without datagram loss.

use oag_net::client::{Client, Phase, Stats};
use oag_net::host::Host;
use oag_net::link::{Clock, Conditions, Link, Loopback, Shim};
use oag_net::wire::RaceParams;
use oag_raceplay::{Race, Setup};

/// What a test client flies with: a function of the tick, so every run is the
/// same and the two clients differ.
pub(crate) type Pilot = Box<dyn FnMut(&mut Client, &mut Race) -> oag_gameplay::InputSnapshot>;

/// How a harness lets time pass between rounds.
#[derive(Clone)]
pub(crate) enum Pace {
    /// By sleeping, so a [`Shim`] on the wall clock delays by a number of ticks that depends on
    /// how loaded the machine is.
    Wall(std::time::Duration),
    /// By advancing a [`Clock`] the shims read, a round a `tick` long and never a sleep: the same
    /// inputs arrive on the same tick on any machine, and the run is as fast as the machine is.
    Virtual(Clock, std::time::Duration),
}

impl Pace {
    /// Lets `wait` pass: a handshake step or a drain, not a round.
    fn idle(&self, wait: std::time::Duration) {
        match self {
            Self::Wall(_) => std::thread::sleep(wait),
            Self::Virtual(clock, _) => clock.advance(wait),
        }
    }

    /// Lets one round of the race pass.
    fn round(&self) {
        match self {
            Self::Wall(sleep) if sleep.is_zero() => {}
            Self::Wall(sleep) => std::thread::sleep(*sleep),
            Self::Virtual(clock, tick) => clock.advance(*tick),
        }
    }
}

/// Ticks the host and every client in lockstep with the wall clock left out:
/// each round the host reads its links and runs one tick once racing, and every
/// client steps once. Returns each client's summed stats.
pub(crate) fn drive(
    host: &mut Host,
    clients: &mut [(Client, Box<Race>, Pilot)],
    ticks: u32,
    pace: &Pace,
) -> Vec<Stats> {
    drive_observed(host, clients, ticks, pace, |_| {})
}

/// [`drive`], showing `observe` the host after each of its ticks.
pub(crate) fn drive_observed(
    host: &mut Host,
    clients: &mut [(Client, Box<Race>, Pilot)],
    ticks: u32,
    pace: &Pace,
    mut observe: impl FnMut(&Host),
) -> Vec<Stats> {
    let mut totals = vec![Stats::default(); clients.len()];
    ready_all(host, clients, pace);
    host.start(0);
    while host.tick_number() < ticks {
        host.poll();
        host.tick();
        observe(host);
        for ((client, race, pilot), total) in clients.iter_mut().zip(&mut totals) {
            let own = pilot(client, race);
            client.step(race, &own);
            let s = client.take_stats();
            total.predicted += s.predicted;
            total.confirmed += s.confirmed;
            total.hash_matches += s.hash_matches;
            total.hash_mismatches += s.hash_mismatches;
            total.corrections += s.corrections;
            total.replayed += s.replayed;
            total.largest_pop = total.largest_pop.max(s.largest_pop);
            total.largest_error = total.largest_error.max(s.largest_error);
            total.error_sum += s.error_sum;
            total.error_count += s.error_count;
            total.own_pop_max = total.own_pop_max.max(s.own_pop_max);
            total.own_moved += s.own_moved;
            total.resends += s.resends;
            total.stalls += s.stalls;
            total.sped_up += s.sped_up;
            total.slowed_down += s.slowed_down;
            total.lead = s.lead;
            total.confirm_count += s.confirm_count;
            total.weapon_mismatches += s.weapon_mismatches;
            total.weapon_corrections += s.weapon_corrections;
            total.late_cues += s.late_cues;
            total.late_blasts += s.late_blasts;
            total.late_oldest = total.late_oldest.max(s.late_oldest);
            total.shots_appeared += s.shots_appeared;
            total.shots_vanished += s.shots_vanished;
            total.first_seen += s.first_seen;
            total.first_seen_latest = total.first_seen_latest.max(s.first_seen_latest);
            total.restored += s.restored;
            total.restored_gap_max = total.restored_gap_max.max(s.restored_gap_max);
            total.own_hit_added += s.own_hit_added;
            total.own_hit_removed += s.own_hit_removed;
            total.confirm_sum_ms += s.confirm_sum_ms;
        }
        pace.round();
    }
    host.end("the test is over");
    // Let the last confirmed ticks arrive.
    for _ in 0..200 {
        let mut behind = false;
        for ((client, race, _), total) in clients.iter_mut().zip(&mut totals) {
            client.step(race, &oag_gameplay::InputSnapshot::EMPTY);
            let s = client.take_stats();
            total.confirmed += s.confirmed;
            total.hash_matches += s.hash_matches;
            total.hash_mismatches += s.hash_mismatches;
            behind |= client.confirmed_tick() < host.tick_number();
        }
        if !behind {
            break;
        }
        host.poll();
        pace.idle(std::time::Duration::from_millis(2));
    }
    totals
}

/// Welcomes every client and waits until the host has them all ready.
pub(crate) fn ready_all(host: &mut Host, clients: &mut [(Client, Box<Race>, Pilot)], pace: &Pace) {
    for (client, race, _) in clients.iter_mut() {
        loop {
            host.poll();
            if client.poll_welcome().expect("welcomed").is_some() {
                break;
            }
            pace.idle(std::time::Duration::from_millis(1));
        }
        client.ready(race).expect("same start state");
    }
    let asked = std::time::Instant::now();
    while !host.all_ready() {
        host.poll();
        for (client, _, _) in clients.iter_mut() {
            client.poll();
        }
        assert!(asked.elapsed().as_secs() < 5, "{:?}", host.seat_names());
        pace.idle(std::time::Duration::from_millis(1));
    }
}

pub(crate) fn params(people: u8) -> RaceParams {
    RaceParams {
        title: String::new(),
        mode: "single_race".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 0xC0FFEE,
        people,
        ..RaceParams::default()
    }
}

pub(crate) fn headless(params: &RaceParams) -> Box<Race> {
    let mut race = Box::new(Race::start(Setup::headless(
        oag_race::Mode::SingleRace,
        params.seed,
    )));
    oag_net::race::prepare(&mut race, params).expect("prepares");
    race
}

pub(crate) fn pilot(phase: u32) -> Pilot {
    Box::new(move |_, race| {
        let tick = u32::try_from(race.core.sim.world.tick).unwrap_or(0);
        let mut buttons = oag_gameplay::input::Input::new();
        let cross = oag_gameplay::input::Button::Cross.bit();
        buttons.begin_frame(if (tick + phase) % 7 < 4 { cross } else { 0 });
        oag_gameplay::InputSnapshot {
            buttons,
            stick_x: ((tick + phase) % 11) as f32 / 10.0 - 0.5,
            ..oag_gameplay::InputSnapshot::default()
        }
    })
}

fn seated(host: &mut Host, people: u8, loss: f32) -> Vec<(Client, Box<Race>, Pilot)> {
    (0..people)
        .map(|n| {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let link: Box<dyn Link> = if loss > 0.0 {
                Box::new(Shim::new(
                    client_end,
                    Conditions {
                        one_way: web_time::Duration::ZERO,
                        loss,
                        ..Conditions::default()
                    },
                ))
            } else {
                Box::new(client_end)
            };
            let client = Client::connect(link, &format!("client {n}"));
            (client, headless(&params(people)), pilot(u32::from(n) * 3))
        })
        .collect()
}

#[test]
fn two_clients_agree_with_the_server_on_every_confirmed_tick() {
    let params = params(2);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 2, 0.0);
    let totals = drive(
        &mut host,
        &mut clients,
        300,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    for (n, total) in totals.iter().enumerate() {
        assert_eq!(total.hash_mismatches, 0, "client {n}: {total:?}");
        assert_eq!(total.hash_matches, 300, "client {n}: {total:?}");
        assert!(total.predicted >= 300, "client {n}: {total:?}");
        // Dropping the send-time ring leaves this at zero and the overlay's CONF row blank.
        assert!(total.confirm_count > 100, "client {n}: {total:?}");
        assert!(total.confirm_sum_ms > 0.0, "client {n}: {total:?}");
    }
    for (client, _, _) in &clients {
        assert_eq!(client.phase(), &Phase::Over("the test is over".into()));
    }
    // Two different pilots really reached the server: its applied inputs differ.
    let last = host.history().last().expect("ran");
    assert_ne!(last.inputs[0], last.inputs[1]);
}

/// Half the datagrams lost both ways: the server repeats a seat's last input when
/// it has none, the client asks for confirmed ticks the redundancy did not cover,
/// and the hashes still agree on every tick.
#[test]
fn half_the_datagrams_lost_and_the_hashes_still_agree() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 1, 0.5);
    let totals = drive(
        &mut host,
        &mut clients,
        400,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    let total = totals[0];
    assert_eq!(total.hash_mismatches, 0, "{total:?}");
    assert_eq!(total.hash_matches, 400, "{total:?}");
    let counts = host.take_counts()[0].expect("seated");
    assert!(counts.repeated > 0, "{counts:?}");
}

#[test]
fn a_client_with_different_content_is_refused_at_the_start() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params.clone());
    let (server_end, client_end) = Loopback::pair();
    host.join(Box::new(server_end));
    let mut client = Client::connect(Box::new(client_end), "wrong");
    host.poll();
    client.poll_welcome().expect("welcomed").expect("a welcome");
    let mut other = params.clone();
    other.seed += 1;
    let mut race = headless(&other);
    let refused = client
        .ready(&mut race)
        .expect_err("a different start was accepted");
    assert!(
        format!("{refused}").contains("different content"),
        "{refused}"
    );
    host.poll();
    assert!(!host.all_ready());
    assert!(
        host.departed(),
        "the refused seat is a departure a lobby can end the start on"
    );
}

/// A person who leaves mid-race is retired on both ends at the same tick: the record carries
/// the seat, the server's and the survivor's craft stop being a person's, and the hashes still
/// agree on every confirmed tick.
#[test]
fn a_person_who_leaves_is_retired_on_both_ends_and_the_hashes_agree() {
    use oag_gameplay::Controller;
    let params = params(2);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 2, 0.0);
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    let mut survivor_mismatches = 0;
    let mut survivor_matches = 0;
    while host.tick_number() < 300 {
        host.poll();
        host.tick();
        for (n, (client, race, pilot)) in clients.iter_mut().enumerate() {
            if n == 1 && host.tick_number() > 100 {
                continue;
            }
            let own = pilot(client, race);
            client.step(race, &own);
            if n == 1 && host.tick_number() == 100 {
                client.leave("quit");
            }
            let stats = client.take_stats();
            if n == 0 {
                survivor_mismatches += stats.hash_mismatches;
                survivor_matches += stats.hash_matches;
            }
        }
    }
    host.end("the test is over");
    assert_eq!(survivor_mismatches, 0);
    assert!(survivor_matches > 200, "{survivor_matches}");
    let history = host.history();
    assert_eq!(history[50].retired, 0);
    assert_eq!(
        history[299].retired, 0b10,
        "seat 1 is retired once it has left"
    );
    assert_eq!(host.race().core.sim.world.controllers[1], Controller::Away);
    assert_eq!(
        host.race().core.sim.world.controllers[0],
        Controller::Remote
    );
    // The survivor's own present simulation (what the game checks `finished()` on) has the
    // leaver's craft retired too, not only its confirmed state.
    let (survivor, race, _) = &mut clients[0];
    for _ in 0..50 {
        survivor.step(race, &oag_gameplay::InputSnapshot::EMPTY);
    }
    assert_eq!(
        race.core.sim.world.controllers[1],
        Controller::Away,
        "the survivor's present has the leaver retired"
    );
}

/// A client with its race and the pilot that flies it.
pub(crate) type Seated = (Client, Box<Race>, Pilot);

/// Seats `people` clients by lobby token and returns them with the tokens.
pub(crate) fn tokened(
    host: &mut Host,
    people: u8,
    race: &mut dyn FnMut() -> Box<Race>,
) -> (Vec<Seated>, Vec<String>) {
    let tokens: Vec<String> = (0..people).map(|n| format!("token-{n}")).collect();
    host.expect_seats(
        (0..people)
            .map(|n| (format!("client {n}"), tokens[usize::from(n)].clone()))
            .collect(),
    );
    let clients = (0..people)
        .map(|n| {
            let (server_end, client_end) = Loopback::pair();
            host.join(Box::new(server_end));
            let client = Client::connect_seat(
                Box::new(client_end),
                &format!("client {n}"),
                "",
                &tokens[usize::from(n)],
            );
            (client, race(), pilot(u32::from(n) * 3))
        })
        .collect();
    (clients, tokens)
}

/// Runs a race until `until` in which client 1's link closes at tick `drop_at` and a new one
/// takes the seat back at `return_at`; `observe` sees the host after each of its ticks. Returns
/// each client's hash mismatches and matches.
pub(crate) fn drop_and_return(
    host: &mut Host,
    clients: &mut [(Client, Box<Race>, Pilot)],
    tokens: &[String],
    (drop_at, return_at, until): (u32, u32, u32),
    mut observe: impl FnMut(&Host),
) -> ([u32; 2], [u32; 2]) {
    ready_all(host, clients, &Pace::Wall(std::time::Duration::ZERO));
    host.start(0);
    let mut mismatches = [0u32; 2];
    let mut matches = [0u32; 2];
    while host.tick_number() < until {
        host.poll();
        host.tick();
        let at = host.tick_number();
        observe(host);
        for (n, (client, race, pilot)) in clients.iter_mut().enumerate() {
            if n == 1 && (drop_at..return_at).contains(&at) {
                if at == drop_at {
                    client.leave("link lost");
                }
                continue;
            }
            if n == 1 && at == return_at {
                let (server_end, client_end) = Loopback::pair();
                host.join(Box::new(server_end));
                client.reconnect(Box::new(client_end), "client 1", "", &tokens[1]);
            }
            let own = pilot(client, race);
            client.step(race, &own);
            let stats = client.take_stats();
            mismatches[n] += stats.hash_mismatches;
            matches[n] += stats.hash_matches;
        }
    }
    host.end("the test is over");
    (mismatches, matches)
}

fn holding_turbo(params: &RaceParams) -> Box<Race> {
    let mut race = headless(params);
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

/// A person whose link drops at tick 600 and returns at tick 900 takes their seat back: it is
/// away on both ends in between and handed back at the first tick the host seats them again, and
/// both ends agree on every hash throughout. (The headless grid has one craft, so the craft's
/// own pace, pickup and countdown are the disc-backed test's,
/// `net_ground_truth::a_dropped_link_returns_to_its_craft_and_its_pickup`.)
#[test]
fn a_person_whose_link_drops_comes_back_to_their_seat_and_the_hashes_agree() {
    use oag_gameplay::Controller;
    let params = params(2);
    let mut host = Host::new(holding_turbo(&params), params.clone());
    let (mut clients, tokens) = tokened(&mut host, 2, &mut || holding_turbo(&params));
    let mut handed_back_at = None;
    let (mismatches, matches) =
        drop_and_return(&mut host, &mut clients, &tokens, (600, 900, 1300), |host| {
            let at = host.tick_number();
            let world = &host.race().core.sim.world;
            if at == 700 {
                assert_eq!(world.controllers[1], Controller::Away);
            }
            if handed_back_at.is_none() && at > 900 && world.ships[1].handback {
                handed_back_at = Some(at);
                assert_eq!(world.controllers[1], Controller::Remote);
                assert!(world.ships[1].autopilot_timer > 2.0, "the countdown starts");
            }
        });
    assert_eq!(mismatches, [0, 0]);
    assert!(matches[0] > 1200, "{}", matches[0]);
    assert!(
        matches[1] > 600 + 300,
        "the returning client checked the ticks it missed and the ones after: {}",
        matches[1]
    );
    let handed_back_at = handed_back_at.expect("the seat was handed back");
    assert!(
        (900..960).contains(&handed_back_at),
        "handed back at {handed_back_at}"
    );
    let history = host.history();
    assert_eq!(history[599].retired, 0);
    assert_eq!(history[600].retired, 0b10);
    assert_eq!(
        history[handed_back_at as usize].retired, 0,
        "away no longer"
    );
    assert_eq!(
        host.race().core.sim.world.controllers[1],
        Controller::Remote
    );
    let (returned, race, _) = &mut clients[1];
    for _ in 0..30 {
        returned.step(race, &oag_gameplay::InputSnapshot::EMPTY);
    }
    assert_eq!(race.core.sim.world.controllers[1], Controller::Remote);
}

/// A killed game sends no close frame: the host still holds the old link when the relaunched
/// game's token arrives. The token takes the seat over, the race never sees the person leave
/// (nothing is retired) and the hashes agree.
#[test]
fn a_token_taking_over_a_seat_whose_link_is_still_held_is_the_person_returning() {
    use oag_gameplay::Controller;
    let params = params(2);
    let mut host = Host::new(holding_turbo(&params), params.clone());
    let (mut clients, tokens) = tokened(&mut host, 2, &mut || holding_turbo(&params));
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    let (mut mismatches, mut matches) = ([0u32; 2], [0u32; 2]);
    let mut returned = false;
    while host.tick_number() < 1100 {
        host.poll();
        host.tick();
        let at = host.tick_number();
        for (n, (client, race, pilot)) in clients.iter_mut().enumerate() {
            if n == 1 && at == 600 {
                // The old server end stays in the host: nothing closed it.
                let (server_end, client_end) = Loopback::pair();
                host.join(Box::new(server_end));
                client.reconnect(Box::new(client_end), "client 1", "", &tokens[1]);
                *race = holding_turbo(&params);
                returned = true;
            }
            if n == 1 && returned && client.phase() == &Phase::Reconnecting {
                host.poll();
                if client.poll_welcome().expect("welcomed").is_some() {
                    client.ready(race).expect("the same start state");
                }
                continue;
            }
            let own = pilot(client, race);
            client.step(race, &own);
            let stats = client.take_stats();
            mismatches[n] += stats.hash_mismatches;
            matches[n] += stats.hash_matches;
        }
    }
    assert_eq!(mismatches, [0, 0]);
    assert!(
        matches[1] > 300,
        "the returned client checked ticks: {matches:?}"
    );
    assert!(
        host.history().iter().all(|tick| tick.retired == 0),
        "the race never saw the seat leave"
    );
    assert_eq!(
        host.race().core.sim.world.controllers[1],
        Controller::Remote
    );
}

/// A seat that came back to a running race and has not said ready again is loading it, and is
/// given [`oag_net::host::RELOAD_GRACE`] before it counts as silent; once ready it is held to
/// the ordinary silence.
#[test]
fn a_returning_seat_loading_the_race_is_not_silent_until_it_is_ready() {
    let params = params(1);
    let mut host = Host::new(holding_turbo(&params), params.clone());
    let (mut clients, tokens) = tokened(&mut host, 1, &mut || holding_turbo(&params));
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    host.poll();
    host.tick();
    let (client, race, _) = &mut clients[0];
    let (server_end, client_end) = Loopback::pair();
    host.join(Box::new(server_end));
    client.reconnect(Box::new(client_end), "client 0", "", &tokens[0]);
    host.poll();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let silence = web_time::Duration::from_millis(1);
    assert_eq!(host.drop_silent(silence), 0, "loading, not silent");
    assert!(client.poll_welcome().expect("welcomed").is_some());
    client.ready(race).expect("the same start state");
    host.poll();
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(host.drop_silent(silence), 1, "ready and then silent");
}

/// A seat that has sent nothing for the allowed silence is dropped, told so, and retired.
#[test]
fn a_silent_seat_is_dropped_and_told() {
    let params = params(2);
    let mut host = Host::new(headless(&params), params);
    let mut clients = seated(&mut host, 2, 0.0);
    ready_all(
        &mut host,
        &mut clients,
        &Pace::Wall(std::time::Duration::ZERO),
    );
    host.start(0);
    host.poll();
    host.tick();
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(host.drop_silent(web_time::Duration::from_millis(1)), 2);
    assert_eq!(
        host.drop_silent(web_time::Duration::from_millis(1)),
        0,
        "once"
    );
    for (client, race, _) in &mut clients {
        client.step(race, &oag_gameplay::InputSnapshot::EMPTY);
        match client.phase() {
            Phase::Over(why) => assert!(why.contains("dropped"), "{why}"),
            other => panic!("not told: {other:?}"),
        }
    }
    host.tick();
    assert_eq!(host.history()[1].retired, 0b11);
}

/// The weapon digest tells two shots of one weapon and owner apart by the tick each was fired
/// on: a present that kept a shot the server ended, while the server holds a newer one, is
/// corrected rather than taken to agree.
#[test]
fn the_weapon_digest_knows_a_shot_by_the_tick_it_was_fired_on() {
    let params = params(2);
    let digest_with_rocket_aged = |ticks: u16| {
        let mut race = headless(&params);
        let projectiles = &mut race.core.sim.world.projectiles;
        assert!(projectiles.spawn(
            oag_tables::weapons::Weapon::Rocket,
            oag_core::math::Vec3::ZERO,
            oag_core::math::Vec3::X,
            1,
        ));
        projectiles.slots[0].lifetime -= f32::from(ticks) / 60.0;
        oag_net::weapon_sync::digest(&race.core.sim)
    };
    assert_eq!(digest_with_rocket_aged(3), digest_with_rocket_aged(3));
    assert_ne!(
        digest_with_rocket_aged(3),
        digest_with_rocket_aged(40),
        "two shots fired 37 ticks apart read as one"
    );
}

/// A bare link seated and ready, driven by hand.
fn raw_seat(host: &mut Host) -> Loopback {
    let (server_end, mut mine) = Loopback::pair();
    host.join(Box::new(server_end));
    mine.send_control(&oag_net::wire::Control::Hello {
        protocol: oag_net::wire::PROTOCOL,
        name: "raw".into(),
        team: String::new(),
        token: String::new(),
    });
    host.poll();
    mine.send_control(&oag_net::wire::Control::Ready {
        fingerprint: host.fingerprint(),
    });
    host.poll();
    assert!(host.all_ready());
    mine
}

#[test]
fn a_peer_cannot_grow_a_seats_buffer_with_inputs_for_far_future_ticks() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params);
    let mut mine = raw_seat(&mut host);
    host.start(0);
    for n in 0..10_000u32 {
        mine.send(&oag_net::wire::Datagram::Inputs {
            seat: 0,
            first: n,
            inputs: vec![oag_net::wire::Held::default()],
            signature: None,
        });
    }
    host.poll();
    assert!(
        host.buffered_inputs(0) < 120,
        "{} inputs buffered",
        host.buffered_inputs(0)
    );
}

#[test]
fn a_hundred_resend_requests_in_a_second_get_one_answer() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params);
    let mut mine = raw_seat(&mut host);
    host.start(0);
    for _ in 0..5 {
        host.poll();
        host.tick();
    }
    while mine.recv_control().is_some() {}
    for _ in 0..100 {
        mine.send_control(&oag_net::wire::Control::Resend { from: 0 });
    }
    host.poll();
    let mut answers = 0;
    while let Some(message) = mine.recv_control() {
        if matches!(message, Ok(oag_net::wire::Control::Ticks(_))) {
            answers += 1;
        }
    }
    assert_eq!(answers, 1);
}

fn silent_links(host: &mut Host, n: usize) -> Vec<Loopback> {
    (0..n)
        .map(|_| {
            let (server_end, mine) = Loopback::pair();
            host.join(Box::new(server_end));
            mine
        })
        .collect()
}

#[test]
fn hundreds_of_silent_connections_are_bounded_and_the_expected_seat_still_joins() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params);
    host.expect_seats(vec![("ann".into(), "token-ann".into())]);
    let _silent = silent_links(&mut host, 300);
    host.poll();
    assert!(
        host.joining() <= oag_net::host::MAX_JOINING.min(299),
        "{} connections wait",
        host.joining()
    );
    let (server_end, mut mine) = Loopback::pair();
    host.join(Box::new(server_end));
    mine.send_control(&oag_net::wire::Control::Hello {
        protocol: oag_net::wire::PROTOCOL,
        name: "ann".into(),
        team: String::new(),
        token: "token-ann".into(),
    });
    host.poll();
    assert_eq!(host.seat_names(), vec![Some("ann".to_string())]);
}

#[test]
fn a_connection_that_never_says_hello_is_dropped_after_the_hello_timeout() {
    let params = params(1);
    let mut host = Host::new(headless(&params), params);
    host.set_hello_timeout(std::time::Duration::from_millis(50));
    let _silent = silent_links(&mut host, 10);
    host.poll();
    assert_eq!(host.joining(), 10);
    std::thread::sleep(std::time::Duration::from_millis(120));
    host.poll();
    assert_eq!(host.joining(), 0);
}

struct Captured;

static CAPTURED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

impl log::Log for Captured {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        CAPTURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("{}", record.args()));
    }
    fn flush(&self) {}
}

#[test]
fn ten_thousand_malformed_datagrams_from_one_seat_log_at_most_a_line_a_second() {
    let _ = log::set_logger(&Captured);
    log::set_max_level(log::LevelFilter::Warn);
    let params = params(1);
    let mut host = Host::new(headless(&params), params);
    let mut mine = raw_seat(&mut host);
    host.start(0);
    CAPTURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
    let started = std::time::Instant::now();
    for _ in 0..10_000 {
        mine.send_datagram(vec![0xFF; 5]);
    }
    host.poll();
    let lines = CAPTURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .filter(|line| line.starts_with("net: seat 0:"))
        .count();
    let seconds = usize::try_from(started.elapsed().as_secs()).unwrap_or(usize::MAX);
    assert!(
        (1..=seconds + 1).contains(&lines),
        "{lines} lines in {seconds} s"
    );
}
