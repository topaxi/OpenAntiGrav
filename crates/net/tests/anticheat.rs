//! A player host that cheats is caught, over real WebTransport sockets on localhost, no disc
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! "The anti-cheat path"): a host that alters the simulation is proven by the lobby's
//! re-simulation and banned, a host that forges a guest's input cannot show the guest's signed
//! datagram and is banned, a host that races another job is refused at the welcome, an honest
//! race reports nothing, and a malformed or oversized report is refused before any judging.
//!
//! The hosts cheat through `oag_net::host::Cheat` (the `test-cheats` feature this crate's tests
//! turn on). The test forwards a client's report and the host's answer to a challenge the way
//! a hosting game does (`oag_game::online`).

use std::sync::Arc;

use oag_net::client::Phase;
use oag_net::fair::Verdict;
use oag_net::host::Cheat;
use oag_net::online::hostkey::PlayerKey;
use oag_net::online::moderation::Bans;
use oag_net::online::player_host::PlayerHost;
use oag_net::online::server::Ruling;
use web_time::Duration;

use crate::loopback::headless;
use crate::online::{Lab, Racer, config, fly_once, thread_runner};
use crate::player_host::{alive, hosted_room_starts_keyed, player_host};

const ANN: usize = 0;
const BOB: usize = 1;

/// Ann hosts a room on her own machine, cheating with `cheat`; Bob joins; both are handed the
/// race and dial it with their keys. The lobby re-simulates with the race loader the host uses.
struct Hosted {
    lab: Lab,
    host: PlayerHost,
    racers: Vec<Racer>,
    room: u32,
    keys: [Arc<PlayerKey>; 2],
    beat: std::time::Instant,
}

fn hosted(cheat: Cheat, max_ticks: u32, ban_file: Option<std::path::PathBuf>) -> Hosted {
    let mut lab = Lab::with_config(
        oag_net::online::server::Config {
            ban_file,
            ..config()
        },
        thread_runner(60),
    );
    lab.server
        .set_referee(Arc::new(|params| Ok(headless(params))));
    let keys = [
        Arc::new(PlayerKey::generate().expect("a key")),
        Arc::new(PlayerKey::generate().expect("a key")),
    ];
    assert_eq!(lab.join_keyed("ann", keys[ANN].public()), ANN);
    assert_eq!(lab.join_keyed("bob", keys[BOB].public()), BOB);
    let mut host = player_host(max_ticks);
    host.sign_with(keys[ANN].clone());
    host.set_cheat(cheat);
    let room = hosted_room_starts_keyed(&mut lab, ANN, BOB, &mut host, &keys[ANN]);
    let racers = [ANN, BOB]
        .into_iter()
        .map(|who| {
            let handoff = lab.people[who].take_handoff().expect("handed off");
            assert_eq!(handoff.host_key, keys[ANN].public(), "the host's key");
            let mut racer = Racer::dial(who, &lab, &handoff);
            racer
                .client
                .guard(handoff.params, handoff.host_key, Some(keys[who].clone()));
            racer
        })
        .collect();
    Hosted {
        lab,
        host,
        racers,
        room,
        keys,
        beat: std::time::Instant::now(),
    }
}

impl Hosted {
    /// One pass: the host's sign of life, the racers, each racer's report sent to the lobby in
    /// parts, and the host's answers to the lobby's challenges.
    fn step(&mut self) {
        alive(&mut self.lab, ANN, self.room, &mut self.beat);
        fly_once(&mut self.lab, &mut self.racers);
        for racer in &mut self.racers {
            if let Some(proof) = racer.client.take_report() {
                self.lab.people[racer.who].queue_report(self.room, &proof.encode());
            }
        }
        for (room, seat, tick) in self.lab.people[ANN].take_challenges() {
            let datagram = self.host.show_input(seat, tick);
            self.lab.people[ANN].show_input(room, seat, tick, datagram);
        }
    }

    /// Steps until Bob is told the lobby closed the room, and returns why.
    fn until_closed(&mut self) -> String {
        let started = std::time::Instant::now();
        loop {
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "the room was never closed; rulings: {:?}; hashes {:?}; watch {:?}",
                self.lab.server.rulings(),
                self.racers
                    .iter()
                    .map(|r| (r.matches, r.mismatches))
                    .collect::<Vec<_>>(),
                self.racers
                    .iter()
                    .map(|r| r.client.watch_stats())
                    .collect::<Vec<_>>()
            );
            self.step();
            if let Some(reason) = self.lab.people[BOB].left.take() {
                return reason;
            }
        }
    }

    fn judged(&self) -> Vec<(String, Verdict)> {
        self.lab
            .server
            .rulings()
            .iter()
            .filter_map(|ruling| match ruling {
                Ruling::Judged {
                    reporter, verdict, ..
                } => Some((reporter.clone(), verdict.clone())),
                Ruling::Refused { .. } => None,
            })
            .collect()
    }
}

fn ban_file(tag: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("anticheat-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir.join("bans.txt")
}

#[test]
fn a_host_that_alters_its_simulation_is_proven_by_the_lobbys_re_simulation_and_banned() {
    let file = ban_file("speed");
    let mut race = hosted(Cheat::Speed, 100_000, Some(file.clone()));
    let reason = race.until_closed();
    assert!(
        reason.contains("its host was caught cheating"),
        "the guest is told why: {reason}"
    );
    let bob = race.judged();
    let proven = bob
        .iter()
        .find(|(who, _)| who == "bob")
        .expect("Bob's report was judged");
    match &proven.1 {
        Verdict::Proven(why) => assert!(why.contains("its own inputs give"), "{why}"),
        other => panic!("not proven: {other:?}"),
    }
    assert_eq!(race.lab.server.judgements(), 1, "one re-simulation");
    let text = std::fs::read_to_string(&file).expect("the ban was written");
    let (bans, skipped) = Bans::parse(&text);
    assert!(skipped.is_empty(), "{text}");
    let ban = bans
        .key_ban(&race.keys[ANN].public())
        .expect("the host's key is banned");
    assert!(ban.reason.starts_with("caught cheating"), "{text}");
    assert!(ban.since.starts_with("20"), "dated: {text}");
    assert!(
        bans.key_ban(&race.keys[BOB].public()).is_none(),
        "the guest is not banned"
    );
}

#[test]
fn a_host_that_forges_a_guests_input_cannot_show_the_signed_datagram_and_is_banned() {
    let mut race = hosted(Cheat::Forge, 100_000, None);
    let reason = race.until_closed();
    assert!(
        reason.contains("its host was caught cheating"),
        "the guest is told why: {reason}"
    );
    let judged = race.judged();
    assert!(
        judged.iter().any(|(who, verdict)| who == "bob"
            && matches!(verdict, Verdict::Proven(why) if why.contains("seat 1 at tick 300"))),
        "{judged:?}"
    );
    assert_eq!(
        race.lab.server.judgements(),
        0,
        "a forgery needs no re-simulation"
    );
}

#[test]
fn a_host_that_races_another_job_than_the_lobby_issued_is_refused_at_the_welcome() {
    let mut race = hosted(Cheat::Seed, 100_000, None);
    let started = std::time::Instant::now();
    let refusal = loop {
        assert!(started.elapsed() < Duration::from_secs(30), "no welcome");
        alive(&mut race.lab, ANN, race.room, &mut race.beat);
        race.lab.pump();
        match race.racers[1].client.poll_welcome() {
            Ok(Some(welcome)) => panic!("welcomed into another race: {:?}", welcome.params),
            Ok(None) => {}
            Err(e) => break e.to_string(),
        }
    };
    assert!(
        refusal.contains("a race the lobby did not issue"),
        "{refusal}"
    );
}

#[test]
fn an_honest_hosted_race_reports_nothing_bans_nobody_and_has_no_bad_hash() {
    let mut race = hosted(Cheat::None, 600, None);
    let started = std::time::Instant::now();
    while race
        .racers
        .iter()
        .any(|r| !matches!(r.client.phase(), Phase::Over(_)))
    {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the race never ended"
        );
        race.step();
    }
    for _ in 0..50 {
        race.step();
    }
    for racer in &race.racers {
        assert_eq!(racer.mismatches, 0, "person {}'s hashes", racer.who);
        let seen = racer.client.watch_stats();
        assert!(seen.checkpoints_verified >= 9, "{} {seen:?}", racer.who);
        assert!(seen.inputs_signed > 300, "{} {seen:?}", racer.who);
        assert_eq!(
            (
                seen.checkpoints_unsigned,
                seen.checkpoints_differ,
                seen.reports
            ),
            (0, 0, 0),
            "{}",
            racer.who
        );
    }
    assert!(
        race.host.signed_inputs() > 300,
        "the host applied {} signed inputs",
        race.host.signed_inputs()
    );
    assert!(race.lab.server.rulings().is_empty(), "no report");
    assert!(race.lab.people[BOB].left.is_none(), "the room stays");
}

#[test]
fn a_malformed_or_oversized_report_is_refused_without_judging_and_the_race_goes_on() {
    let mut race = hosted(Cheat::None, 100_000, None);
    let started = std::time::Instant::now();
    while race.racers[1].matches < 120 {
        assert!(started.elapsed() < Duration::from_secs(30), "not racing");
        race.step();
    }
    let room = race.room;
    // More parts than the race can have run, then a whole report that is noise.
    race.lab.people[BOB].report_part(room, 0, u16::MAX, vec![0; 16]);
    race.lab.people[BOB].report_part(room, 0, 1, b"OAGP not a proof at all".to_vec());
    let started = std::time::Instant::now();
    while race.lab.server.rulings().len() < 2 {
        assert!(started.elapsed() < Duration::from_secs(10), "not ruled");
        race.step();
    }
    let whys: Vec<&str> = race
        .lab
        .server
        .rulings()
        .iter()
        .map(|ruling| match ruling {
            Ruling::Refused { why, .. } => why.as_str(),
            Ruling::Judged { .. } => panic!("judged: {ruling:?}"),
        })
        .collect();
    assert!(
        whys[0].contains("more than the race can have run"),
        "{whys:?}"
    );
    assert!(whys[1].contains("malformed"), "{whys:?}");
    assert_eq!(race.lab.server.judgements(), 0, "no re-simulation");
    let before = race.racers[1].matches;
    for _ in 0..60 {
        race.step();
    }
    assert!(race.racers[1].matches > before, "the race goes on");
    assert!(race.lab.people[BOB].left.is_none(), "the room stays open");
    assert_eq!(race.racers[1].mismatches, 0);
}

#[test]
fn a_guest_that_claims_a_forgery_of_an_input_it_sent_is_dismissed_and_bans_nobody() {
    let mut race = hosted(Cheat::None, 100_000, None);
    race.racers[1].client.lie_from(240);
    let started = std::time::Instant::now();
    let judged = loop {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "no ruling: {:?}",
            race.lab.server.rulings()
        );
        race.step();
        if let Some(ruling) = race.judged().into_iter().find(|(who, _)| who == "bob") {
            break ruling.1;
        }
    };
    match judged {
        Verdict::Dismissed(why) => assert!(why.contains("showed seat 1's signed input"), "{why}"),
        other => panic!("an honest host is not convicted: {other:?}"),
    }
    for _ in 0..60 {
        race.step();
    }
    assert!(race.lab.people[BOB].left.is_none(), "the room stays open");
    assert_eq!(race.lab.server.judgements(), 0, "no re-simulation");
}
