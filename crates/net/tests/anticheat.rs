//! A player host that cheats is caught, over real WebTransport sockets on localhost, no disc
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! "The anti-cheat path"): a host that alters the simulation is proven by the lobby's
//! re-simulation and struck, a host that forges a guest's input cannot show the guest's signed
//! datagram and is struck, two strikes from two races and two reporters ban the host for an
//! hour (one reporter twice does not), a host that races another job is refused at the welcome,
//! an honest race reports nothing, and a malformed or oversized report is refused before any
//! judging.
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
use crate::online::{Lab, Racer, fly_once, thread_runner};
use crate::online::{config, edition};
use crate::player_host::{alive, hosted_room_starts_keyed, offer_room, player_host};
use oag_net::wire::online::Refusal;

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
    let keys = [
        Arc::new(PlayerKey::generate().expect("a key")),
        Arc::new(PlayerKey::generate().expect("a key")),
    ];
    hosted_by(cheat, max_ticks, ban_file, keys)
}

/// [`hosted`] with Ann's and Bob's keys given: the same player across lobby runs.
fn hosted_by(
    cheat: Cheat,
    max_ticks: u32,
    ban_file: Option<std::path::PathBuf>,
    keys: [Arc<PlayerKey>; 2],
) -> Hosted {
    hosted_built(cheat, max_ticks, ban_file, keys, &edition().build)
}

/// [`hosted_by`] with the build Ann and Bob run given; the lobby's own is [`edition`]'s.
fn hosted_built(
    cheat: Cheat,
    max_ticks: u32,
    ban_file: Option<std::path::PathBuf>,
    keys: [Arc<PlayerKey>; 2],
    build: &str,
) -> Hosted {
    let mut lab = Lab::with_config(
        oag_net::online::server::Config {
            ban_file,
            ..config()
        },
        thread_runner(60),
    );
    lab.edition.build = build.to_string();
    if build != edition().build {
        // The hello refuses another build of a pressing the lobby races (`Rooms::connect`), so
        // the host's other build is of a pressing the lobby does not.
        lab.edition.platform = "PS2".into();
    }
    lab.server
        .set_referee(Arc::new(|params| Ok(headless(params))));
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

fn ban_file(tag: &str) -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir(&format!("anticheat-{tag}")).child("bans.txt")
}

/// The lines of `file` that start with `kind` and name `key`.
fn lines_on(file: &std::path::Path, kind: &str, key: &PlayerKey) -> Vec<String> {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let prefix = format!("{kind} {} ", oag_net::wt::hex(&key.public()));
    text.lines()
        .filter(|line| line.starts_with(&prefix))
        .map(str::to_string)
        .collect()
}

/// One race on a fresh lobby over `file` (a lobby restart between races), hosted by `host`
/// cheating with `cheat` and reported by `guest`: the room closes and the verdict is proven.
fn one_proven_case(
    file: &std::path::Path,
    cheat: Cheat,
    host: &Arc<PlayerKey>,
    guest: &Arc<PlayerKey>,
) {
    let mut race = hosted_by(
        cheat,
        100_000,
        Some(file.to_path_buf()),
        [host.clone(), guest.clone()],
    );
    let reason = race.until_closed();
    assert!(
        reason.contains("its host was caught cheating") && !reason.contains("banned"),
        "the guest is told the proof, not a ban: {reason}"
    );
    assert!(
        race.judged()
            .iter()
            .any(|(who, verdict)| who == "bob" && matches!(verdict, Verdict::Proven(_))),
        "{:?}",
        race.judged()
    );
}

/// A fresh lobby over `file`; `key` offers a room from a new connection. `Ok` when the room is
/// listed, `Err` with the refusal's text when it is refused.
fn try_to_host(file: &std::path::Path, key: &PlayerKey) -> Result<(), (Refusal, String)> {
    let mut lab = Lab::with_config(
        oag_net::online::server::Config {
            ban_file: Some(file.to_path_buf()),
            ..config()
        },
        thread_runner(60),
    );
    let who = lab.join_keyed("ann", key.public());
    let host = player_host(60);
    offer_room(&mut lab, who, &host, key);
    lab.until("a room or a refusal", |lab| {
        lab.people[who].room.is_some() || lab.people[who].refused.is_some()
    });
    lab.people[who].refused.take().map_or(Ok(()), Err)
}

#[test]
fn a_host_that_alters_its_simulation_is_proven_by_the_lobbys_re_simulation_and_struck_not_banned() {
    let file = ban_file("speed");
    let mut race = hosted(Cheat::Speed, 100_000, Some(file.to_path_buf()));
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
    let text = std::fs::read_to_string(&file).expect("the strike was written");
    let (_, skipped) = Bans::parse(&text);
    assert!(skipped.is_empty(), "{text}");
    let strikes = lines_on(&file, "strike", &race.keys[ANN]);
    assert_eq!(strikes.len(), 1, "one strike: {text}");
    let bob_hex = oag_net::wt::hex(&race.keys[BOB].public());
    assert!(
        strikes[0].contains(&format!(" by {bob_hex} ")) && strikes[0].contains(" race "),
        "the strike names the race and the reporter: {text}"
    );
    assert!(
        lines_on(&file, "key", &race.keys[ANN]).is_empty(),
        "no ban: {text}"
    );
    assert!(
        lines_on(&file, "strike", &race.keys[BOB]).is_empty(),
        "{text}"
    );
    let ann = race.keys[ANN].clone();
    drop(race);
    assert_eq!(
        try_to_host(&file, &ann),
        Ok(()),
        "one proven case bans nobody"
    );
}

#[test]
fn two_proven_cases_from_two_races_and_two_reporters_ban_the_host_for_an_hour() {
    let file = ban_file("two-reporters");
    let ann = Arc::new(PlayerKey::generate().expect("a key"));
    let (bob, cy) = (
        Arc::new(PlayerKey::generate().expect("a key")),
        Arc::new(PlayerKey::generate().expect("a key")),
    );
    one_proven_case(&file, Cheat::Speed, &ann, &bob);
    assert_eq!(try_to_host(&file, &ann), Ok(()), "one strike: still hosts");
    one_proven_case(&file, Cheat::Forge, &ann, &cy);
    let bans = lines_on(&file, "key", &ann);
    assert_eq!(bans.len(), 1, "one ban line: {bans:?}");
    assert!(bans[0].contains(" until "), "the ban expires: {bans:?}");
    let (code, text) = try_to_host(&file, &ann).expect_err("banned");
    assert_eq!(code, Refusal::Banned, "{text}");
    assert!(text.contains("caught cheating"), "the reason: {text}");
    assert!(
        text.contains("59 min left") || text.contains("1 h left"),
        "the time left: {text}"
    );
    assert_eq!(try_to_host(&file, &bob), Ok(()), "a reporter is not banned");
}

#[test]
fn two_proven_cases_reported_by_the_same_player_are_one_strikes_worth() {
    let file = ban_file("one-reporter");
    let ann = Arc::new(PlayerKey::generate().expect("a key"));
    let bob = Arc::new(PlayerKey::generate().expect("a key"));
    one_proven_case(&file, Cheat::Speed, &ann, &bob);
    one_proven_case(&file, Cheat::Forge, &ann, &bob);
    assert_eq!(lines_on(&file, "strike", &ann).len(), 2, "both written");
    assert!(lines_on(&file, "key", &ann).is_empty(), "no ban");
    assert_eq!(try_to_host(&file, &ann), Ok(()), "still hosts");
}

#[test]
fn a_host_that_forges_a_guests_input_cannot_show_the_signed_datagram_and_is_struck() {
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
        // At least one signed datagram per 10 of the 600 ticks: under load a client sends
        // fewer, larger datagrams (211 seen at a load of 18), so the count is not a rate.
        assert!(seen.inputs_signed > 60, "{} {seen:?}", racer.who);
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
        race.host.signed_inputs() > 60,
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

const OTHER_BUILD: &str = "0.2 (def)";

/// Steps until Bob's report on the host is judged, and returns the verdict.
fn bobs_verdict(race: &mut Hosted) -> Verdict {
    let started = std::time::Instant::now();
    loop {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "no ruling: {:?}",
            race.lab.server.rulings()
        );
        race.step();
        if let Some(ruling) = race.judged().into_iter().find(|(who, _)| who == "bob") {
            return ruling.1;
        }
    }
}

#[test]
fn a_host_of_another_build_than_the_lobby_is_not_judged_by_a_re_simulation_and_not_struck() {
    let file = ban_file("other-build");
    let keys = [
        Arc::new(PlayerKey::generate().expect("a key")),
        Arc::new(PlayerKey::generate().expect("a key")),
    ];
    let mut race = hosted_built(
        Cheat::Speed,
        100_000,
        Some(file.to_path_buf()),
        keys,
        OTHER_BUILD,
    );
    match bobs_verdict(&mut race) {
        Verdict::NotCheckable(why) => assert!(
            why.contains("the host runs another build") && why.contains(OTHER_BUILD),
            "{why}"
        ),
        other => panic!("a build difference is not proof: {other:?}"),
    }
    assert_eq!(race.lab.server.judgements(), 0, "no re-simulation spent");
    for _ in 0..60 {
        race.step();
    }
    assert!(race.lab.people[BOB].left.is_none(), "the room stays open");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    assert!(
        lines_on(&file, "strike", &race.keys[ANN]).is_empty(),
        "no strike: {text}"
    );
    assert!(
        lines_on(&file, "key", &race.keys[ANN]).is_empty(),
        "no ban: {text}"
    );
}

#[test]
fn a_host_of_another_build_that_hides_a_signed_input_is_still_struck() {
    let mut race = hosted_built(
        Cheat::Forge,
        100_000,
        None,
        [
            Arc::new(PlayerKey::generate().expect("a key")),
            Arc::new(PlayerKey::generate().expect("a key")),
        ],
        OTHER_BUILD,
    );
    let reason = race.until_closed();
    assert!(reason.contains("its host was caught cheating"), "{reason}");
    assert!(
        matches!(bobs_verdict(&mut race), Verdict::Proven(_)),
        "{:?}",
        race.judged()
    );
}
