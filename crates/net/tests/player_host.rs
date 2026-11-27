//! A race hosted by a player's machine and listed by the lobby, over real WebTransport sockets
//! on localhost, no disc: the signed offer and the probe, the mark in the list, the hand-off
//! to the host's own address, the race to its end with every hash agreeing, the host leaving,
//! and the ban list (ADR-0092).

use std::sync::Arc;

use oag_net::client::Phase;
use oag_net::online::hostkey::PlayerKey;
use oag_net::online::player_host::{Event, HOST_LEFT, PlayerHost};
use oag_net::online::race_proc::Limits;
use oag_net::online::rooms::HOST_LEFT_RACING;
use oag_net::wire::online::{Phase as RoomPhase, Refusal};
use web_time::Duration;

use crate::loopback::headless;
use crate::online::{Lab, Racer, config, fly_once, settings, thread_runner};

/// The host's sign of life to the lobby, once a second, as a hosting game sends it.
pub(crate) fn alive(lab: &mut Lab, owner: usize, room: u32, last: &mut std::time::Instant) {
    if last.elapsed() >= Duration::from_secs(1) {
        *last = std::time::Instant::now();
        lab.people[owner].host_up(room);
    }
}

pub(crate) fn player_host(max_ticks: u32) -> PlayerHost {
    PlayerHost::bind(
        "127.0.0.1:0".parse().expect("addr"),
        Limits {
            join_timeout: Duration::from_secs(20),
            max_ticks,
        },
    )
    .expect("the hosting port binds")
}

/// `who` offers `host`'s listener to the lobby, signed with `key`.
pub(crate) fn offer_room(lab: &mut Lab, who: usize, host: &PlayerHost, key: &PlayerKey) {
    let token = lab.people[who].token.clone().expect("welcomed");
    let offer = key.offer(&lab.server.cert_hash(), &token, host.port(), host.hash());
    lab.people[who].host_room(settings(), offer);
}

/// `owner` hosts a room on `host`, `guest` joins it, both pick, the owner presses START, the
/// host loads the lobby's job and says it is up. Returns the room.
fn hosted_room_starts(lab: &mut Lab, owner: usize, guest: usize, host: &mut PlayerHost) -> u32 {
    hosted_room_starts_keyed(
        lab,
        owner,
        guest,
        host,
        &PlayerKey::generate().expect("a key"),
    )
}

/// [`hosted_room_starts`] with the owner's offer signed by `key`.
pub(crate) fn hosted_room_starts_keyed(
    lab: &mut Lab,
    owner: usize,
    guest: usize,
    host: &mut PlayerHost,
    key: &PlayerKey,
) -> u32 {
    offer_room(lab, owner, host, key);
    lab.until("the hosted room", |lab| lab.people[owner].room.is_some());
    let room = lab.people[owner].room.as_ref().expect("room").id;
    lab.people[guest].join(room);
    lab.until("the join", |lab| {
        lab.people[guest]
            .room
            .as_ref()
            .is_some_and(|r| r.members.len() == 2)
    });
    for who in [owner, guest] {
        lab.people[who].ready("Feisar", true, oag_physics::pilot_assist::Level::Off);
    }
    lab.until("both ready", |lab| {
        lab.people[owner]
            .room
            .as_ref()
            .is_some_and(|r| r.members.iter().all(|m| m.ready))
    });
    lab.people[owner].start();
    let started = std::time::Instant::now();
    let (launched, job) = loop {
        assert!(started.elapsed() < Duration::from_secs(30), "no launch");
        lab.pump();
        if let Some(launch) = lab.people[owner].take_launch() {
            break launch;
        }
    };
    assert_eq!(launched, room);
    assert_eq!(job.seats.len(), 2, "a seat token for each member");
    host.launch(room, job, Arc::new(|params| Ok(headless(params))));
    loop {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "the host never came up"
        );
        lab.pump();
        match host.poll() {
            Some(Event::Up(up)) => {
                assert_eq!(up, room);
                break;
            }
            Some(other) => panic!("the host could not load the race: {other:?}"),
            None => {}
        }
    }
    lab.people[owner].host_up(room);
    lab.until("the hand-offs", |lab| {
        lab.people[owner].has_handoff() && lab.people[guest].has_handoff()
    });
    room
}

#[test]
fn a_signed_offer_whose_port_answers_is_listed_with_the_mark_and_the_hosts_name() {
    let mut lab = Lab::new(thread_runner(60));
    let (ann, bob, cy) = (lab.join("ann"), lab.join("bob"), lab.join("cy"));
    let host = player_host(60);
    let key = PlayerKey::generate().expect("a key");
    offer_room(&mut lab, ann, &host, &key);
    lab.until("the hosted room", |lab| lab.people[ann].room.is_some());
    assert!(lab.people[ann].room.as_ref().expect("room").player_hosted);
    // A room the server races, beside it.
    lab.people[cy].create(settings());
    lab.until("the server room", |lab| lab.people[cy].room.is_some());
    lab.people[bob].list();
    lab.until("the list", |lab| lab.people[bob].rooms.len() == 2);
    let marks: Vec<(&str, bool)> = lab.people[bob]
        .rooms
        .iter()
        .map(|room| (room.owner.as_str(), room.player_hosted))
        .collect();
    assert_eq!(marks, [("ann", true), ("cy", false)]);
    assert!(lab.people[bob].rooms.iter().all(|room| room.joinable));
}

#[test]
fn a_guest_joins_through_the_handoff_and_races_the_host_to_the_end_with_no_bad_hash() {
    let mut lab = Lab::new(thread_runner(60));
    let (ann, bob) = (lab.join("ann"), lab.join("bob"));
    let mut host = player_host(360);
    let room = hosted_room_starts(&mut lab, ann, bob, &mut host);
    assert_eq!(
        lab.server.racing_room(),
        None,
        "the server's own slot is free"
    );
    assert_eq!(lab.server.hosted_races(), 1);
    let handoffs: Vec<_> = [ann, bob]
        .iter()
        .map(|&who| lab.people[who].take_handoff().expect("handed off"))
        .collect();
    for handoff in &handoffs {
        assert_eq!(
            handoff.host, "127.0.0.1",
            "the host's address as the lobby saw it"
        );
        assert_eq!(
            (handoff.port, handoff.cert_hash),
            (host.port(), host.hash())
        );
    }
    let mut racers = [
        Racer::dial(ann, &lab, &handoffs[0]),
        Racer::dial(bob, &lab, &handoffs[1]),
    ];
    let mut beat = std::time::Instant::now();
    while racers
        .iter()
        .any(|r| !matches!(r.client.phase(), Phase::Over(_)))
    {
        alive(&mut lab, ann, room, &mut beat);
        fly_once(&mut lab, &mut racers);
    }
    for racer in &racers {
        assert_eq!(racer.mismatches, 0, "person {}'s hashes", racer.who);
        assert!(racer.matches >= 300, "{} ticks confirmed", racer.matches);
    }
    assert_eq!(
        (racers[0].seat, racers[1].seat),
        (Some(0), Some(1)),
        "the host is seat 0"
    );
    let started = std::time::Instant::now();
    let ended = loop {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the host never reported the end"
        );
        if let Some(Event::Over(over, ended)) = host.poll() {
            assert_eq!(over, room);
            break ended;
        }
        lab.pump();
    };
    lab.people[ann].host_over(room, &ended.reason, ended.board);
    lab.until("the room to be open again", |lab| {
        lab.phase(bob) == Some(RoomPhase::Open)
    });
    let back = lab.people[bob].room.as_ref().expect("still in the room");
    assert_eq!(back.note, "--max-ticks reached");
    assert_eq!(back.members.len(), 2);
    assert_eq!(lab.server.hosted_races(), 0);
}

#[test]
fn the_host_leaving_the_race_ends_it_for_the_guest_with_the_reason() {
    let mut lab = Lab::new(thread_runner(60));
    let (ann, bob) = (lab.join("ann"), lab.join("bob"));
    let mut host = player_host(100_000);
    let room = hosted_room_starts(&mut lab, ann, bob, &mut host);
    let handoff = lab.people[bob].take_handoff().expect("handed off");
    let own = lab.people[ann].take_handoff().expect("handed off");
    let mut racers = [
        Racer::dial(ann, &lab, &own),
        Racer::dial(bob, &lab, &handoff),
    ];
    // Fly until the guest has confirmed two seconds, then the host's player leaves.
    let started = std::time::Instant::now();
    let mut beat = started;
    let mut left = false;
    while !matches!(racers[1].client.phase(), Phase::Over(_)) {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the race never ended"
        );
        if !left && racers[1].client.confirmed_tick() >= 120 {
            host.end_race();
            left = true;
        }
        alive(&mut lab, ann, room, &mut beat);
        fly_once(&mut lab, &mut racers);
    }
    assert_eq!(
        racers[1].client.phase(),
        &Phase::Over(HOST_LEFT.to_string())
    );
    assert_eq!(racers[1].mismatches, 0);
}

#[test]
fn a_host_that_vanishes_closes_its_room_and_the_lobby_tells_the_guest_why() {
    let mut lab = Lab::new(thread_runner(60));
    // The host joins last, so its lobby client is the one popped.
    let (bob, ann) = (lab.join("bob"), lab.join("ann"));
    let mut host = player_host(100_000);
    hosted_room_starts(&mut lab, ann, bob, &mut host);
    drop(host);
    drop(lab.people.pop());
    lab.until("the lobby to tell the guest", |lab| {
        lab.people[bob].left.is_some()
    });
    assert_eq!(lab.people[bob].left.as_deref(), Some(HOST_LEFT_RACING));
    assert!(lab.people[bob].room.is_none());
    lab.until("the race to be forgotten", |lab| {
        lab.server.hosted_races() == 0
    });
    lab.people[bob].list();
    lab.pump();
    lab.until("an empty list", |lab| lab.people[bob].rooms.is_empty());
}

/// A machine that is killed closes nothing: the lobby hears no sign of life from it for
/// `QUIET_TIMEOUT` and closes its room, telling the guest why.
#[test]
fn a_host_that_goes_quiet_mid_race_is_taken_for_gone_within_ten_seconds() {
    let mut lab = Lab::new(thread_runner(60));
    let (ann, bob) = (lab.join("ann"), lab.join("bob"));
    let mut host = player_host(100_000);
    hosted_room_starts(&mut lab, ann, bob, &mut host);
    let quiet = std::time::Instant::now();
    lab.until("the lobby to tell the guest", |lab| {
        lab.people[bob].left.is_some()
    });
    assert_eq!(lab.people[bob].left.as_deref(), Some(HOST_LEFT_RACING));
    assert!(
        quiet.elapsed() < Duration::from_secs(10),
        "{:?}",
        quiet.elapsed()
    );
    assert_eq!(lab.server.hosted_races(), 0);
}

#[test]
fn an_offer_that_is_wrongly_signed_or_whose_port_is_closed_is_refused_by_name() {
    let mut lab = Lab::new(thread_runner(60));
    let (ann, bob) = (lab.join("ann"), lab.join("bob"));
    let host = player_host(60);
    let key = PlayerKey::generate().expect("a key");
    // Signed for another session.
    let offer = key.offer(
        &lab.server.cert_hash(),
        "not my token",
        host.port(),
        host.hash(),
    );
    lab.people[ann].host_room(settings(), offer);
    lab.until("the refusal", |lab| lab.people[ann].refused.is_some());
    let (code, text) = lab.people[ann].refused.take().expect("refused");
    assert_eq!(code, Refusal::BadHost, "{text}");
    // Asking again at once is one request too many.
    offer_room(&mut lab, ann, &host, &key);
    lab.until("the refusal", |lab| lab.people[ann].refused.is_some());
    let (code, text) = lab.people[ann].refused.take().expect("refused");
    assert_eq!(code, Refusal::BadHost);
    assert!(text.contains("every 5 s"), "{text}");
    // A port nothing listens on: bound, read for its number, and closed again.
    let port = std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|socket| socket.local_addr())
        .expect("a free port")
        .port();
    let token = lab.people[bob].token.clone().expect("welcomed");
    let offer = key.offer(&lab.server.cert_hash(), &token, port, host.hash());
    lab.people[bob].host_room(settings(), offer);
    lab.until("the refusal", |lab| lab.people[bob].refused.is_some());
    let (code, text) = lab.people[bob].refused.take().expect("refused");
    assert_eq!(code, Refusal::Unreachable, "{text}");
    assert!(text.contains(&port.to_string()), "{text}");
    assert!(lab.people[ann].room.is_none() && lab.people[bob].room.is_none());
    lab.people[bob].list();
    lab.pump();
    assert!(lab.people[bob].rooms.is_empty(), "nothing was listed");
}

#[test]
fn an_address_hosts_only_its_cap_of_rooms() {
    let mut lab = Lab::new(thread_runner(60));
    let hosts: Vec<PlayerHost> = (0..3).map(|_| player_host(60)).collect();
    let key = PlayerKey::generate().expect("a key");
    for (i, host) in hosts.iter().enumerate() {
        let who = lab.join(&format!("host{i}"));
        offer_room(&mut lab, who, host, &key);
        lab.until("an answer", |lab| {
            lab.people[who].room.is_some() || lab.people[who].refused.is_some()
        });
    }
    assert!(lab.people[0].room.is_some() && lab.people[1].room.is_some());
    let (code, text) = lab.people[2].refused.take().expect("the third is refused");
    assert_eq!(code, Refusal::Full, "{text}");
}

#[test]
fn a_banned_key_cannot_host_and_a_ban_added_later_delists_the_room() {
    let dir = oag_testdata::scratch::dir("player-host-bans");
    let file = dir.join("bans.txt");
    let banned = PlayerKey::generate().expect("a key");
    let hex = |key: &PlayerKey| oag_net::wt::hex(&key.public());
    std::fs::write(
        &file,
        format!(
            "key {} 2026-11-18T00:00:00.000Z altered a race\n",
            hex(&banned)
        ),
    )
    .expect("writes");
    let mut lab = Lab::with_config(
        oag_net::online::server::Config {
            ban_file: Some(file.clone()),
            ..config()
        },
        thread_runner(60),
    );
    let (ann, bob, cy) = (lab.join("ann"), lab.join("bob"), lab.join("cy"));
    let (first, second) = (player_host(60), player_host(60));
    offer_room(&mut lab, ann, &first, &banned);
    lab.until("the refusal", |lab| lab.people[ann].refused.is_some());
    let (code, text) = lab.people[ann].refused.take().expect("refused");
    assert_eq!(code, Refusal::Banned);
    assert!(text.contains("altered a race"), "{text}");
    // Another key hosts; the operator then bans it, and its listed room goes.
    let other = PlayerKey::generate().expect("a key");
    offer_room(&mut lab, bob, &second, &other);
    lab.until("the hosted room", |lab| lab.people[bob].room.is_some());
    lab.people[cy].list();
    lab.until("the room is listed", |lab| lab.people[cy].rooms.len() == 1);
    std::fs::write(
        &file,
        format!(
            "key {} 2026-11-18T00:00:00.000Z altered a race\nkey {} 2026-11-18T00:00:00.000Z hosted and quit\n",
            hex(&banned),
            hex(&other)
        ),
    )
    .expect("writes");
    lab.until("the room to be closed", |lab| {
        lab.people[bob].left.is_some()
    });
    let why = lab.people[bob].left.clone().expect("told why");
    assert!(why.contains("hosted and quit"), "{why}");
    lab.people[cy].list();
    lab.until("the room is delisted", |lab| {
        lab.people[cy].rooms.is_empty()
    });
}
