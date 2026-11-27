//! A race hosted by a player's game and listed by the lobby: real processes, a real
//! `oag-server lobby` and a real circuit on localhost (ADR-0092).
//!
//! One `oag-game --online --online-create --online-host --net-bot` makes a room whose race it
//! runs itself; the lobby checks its signed offer and its port and lists the room; a second
//! `oag-game --online` finds the room in the list, joins it and is handed the host's own
//! address. The claims: both race Pulse's circuit 01 (one lap) to the finish with no bad hash
//! and the lobby, which ran no race, is told the result; a host that is killed ends the race
//! for the guest with the lobby's reason; a host whose key is on the lobby's ban file is
//! refused with the ban's reason; a host that alters its race (the tests' own build cheats on
//! `OAG_TEST_CHEAT`) is reported by the guest, proven by the lobby's re-simulation and struck
//! with a dated line but not banned (one strike is no ban), and the guest is told why (ADR-0092, "The anti-cheat path").
//!
//! The games are spawned by `net_rejoin_ground_truth::Lab` (`CARGO_BIN_EXE_oag-game`), so this
//! module is in nextest's `gpu-vram` group like that one.

use crate::net_rejoin_ground_truth::{Lab, Proc, image};

const HOST_ARGS: [&str; 3] = ["--online-host", "--online-host-port", "0"];

/// The hosting port `who`'s log names.
fn hosting_port(lab: &Lab, who: &str) -> Option<String> {
    let text = lab.read(who);
    let rest = text.split("online: hosting on UDP port ").nth(1)?;
    Some(rest.split(',').next()?.trim().to_string())
}

/// `a` hosts a room and `b` joins it from the list; both are racing.
fn hosted_race(lab: &Lab) -> (Proc, Proc) {
    let a = lab.client_with("a", true, &HOST_ARGS);
    lab.wait("the lobby to list the hosted room", 90, |lab| {
        lab.read("a")
            .contains("online: room 1 (player-hosted): 1 in it")
    });
    let b = lab.client_with("b", false, &[]);
    lab.wait("both to be racing", 180, |lab| {
        ["a", "b"]
            .iter()
            .all(|who| lab.progress(who).is_some_and(|(tick, _, _)| tick >= 300))
    });
    (a, b)
}

#[test]
#[ignore = "needs a disc image"]
fn a_player_hosts_a_listed_room_and_a_guest_races_them_to_the_finish() {
    let Some(image) = image() else {
        return;
    };
    let lab = Lab::new("hosted", image, 20_000);
    let (mut a, mut b) = hosted_race(&lab);
    // The guest found the room in the lobby's list and was handed the host's own listener,
    // which is not a port of the lobby's.
    let port = hosting_port(&lab, "a").expect("the host says its port");
    let guest = lab.read("b");
    assert!(
        guest.contains("online: room 1 (player-hosted): 2 in it"),
        "the guest is in the marked room"
    );
    assert!(
        guest.contains(&format!("online: the race is on 127.0.0.1:{port}")),
        "the guest dials the host: {port}"
    );
    let lobby = lab.read("lobby");
    assert!(
        lobby.contains("host-log: key "),
        "the registration is logged"
    );
    assert!(
        lobby.contains("host-log: room 1 races on its host's machine"),
        "the launch went to the host"
    );
    assert!(
        !lobby.contains("lobby: room 1 races:"),
        "the lobby ran no race of its own"
    );
    for (who, game) in [("a", &mut a), ("b", &mut b)] {
        let status = game.finish(200);
        assert!(
            status.is_some_and(|s| s.success()),
            "{who} ran to the end:\n{}",
            tail(&lab.read(who))
        );
        let (_, ok, bad) = lab.progress(who).expect("its hashes are logged");
        assert_eq!((bad, ok > 1500), (0, true), "{who}: ok {ok} bad {bad}");
        assert!(
            lab.read(who)
                .contains("the server's board matches this client's"),
            "{who}'s board is the host's:\n{}",
            tail(&without_per_second_lines(&lab.read(who)))
        );
    }
    lab.wait("the lobby to be told the result", 20, |lab| {
        lab.read("lobby")
            .contains("host-log: room 1's hosted race is over: every person has finished")
    });
}

/// A wreck leaves the finisher's own predicted race finished before the server has ended it,
/// which is not where a client may stop stepping.
#[test]
#[ignore = "needs a disc image"]
fn a_finisher_whose_partner_was_wrecked_still_compares_the_board_with_the_hosts() {
    let Some(image) = image() else {
        return;
    };
    let lab = Lab::new("hosted-wreck", image, 20_000);
    // The tests' own build wrecks seat 1 on every peer alike (`oag_net::race::presence`).
    let wreck = [("OAG_TEST_WRECK", "1")];
    let mut a = lab.client_env("a", true, &HOST_ARGS, &wreck);
    lab.wait("the lobby to list the hosted room", 90, |lab| {
        lab.read("a")
            .contains("online: room 1 (player-hosted): 1 in it")
    });
    let mut b = lab.client_env("b", false, &[], &wreck);
    for (who, game) in [("a", &mut a), ("b", &mut b)] {
        let status = game.finish(200);
        let log = lab.read(who);
        let shown = tail(&without_per_second_lines(&log)).to_string();
        assert!(
            status.is_some_and(|s| s.success()),
            "{who} ran to the end:\n{shown}"
        );
        assert!(
            log.contains("the server's board matches this client's"),
            "{who}'s board is the host's:\n{shown}"
        );
        // The scenario happened: seat 1 never finished a lap, and seat 0 did.
        assert!(
            log.lines()
                .any(|l| l.contains("slot 1 (person) laps 0 finish tick None")),
            "{who} saw seat 1 wrecked:\n{shown}"
        );
        assert_eq!(lab.progress(who).map(|(_, _, bad)| bad), Some(0), "{who}");
    }
}

#[test]
#[ignore = "needs a disc image"]
fn a_killed_host_ends_the_race_for_the_guest_with_the_lobbys_reason() {
    let Some(image) = image() else {
        return;
    };
    let lab = Lab::new("hosted-killed", image, 20_000);
    let (mut a, mut b) = hosted_race(&lab);
    a.kill();
    let status = b.finish(60);
    let guest = lab.read("b");
    assert!(
        status.is_some(),
        "the guest's race ended by itself:\n{}",
        tail(&guest)
    );
    assert!(
        guest.contains(
            "online: the lobby closed the room: the host left: a player-hosted race ends with \
             its host"
        ),
        "the guest is told why:\n{}",
        tail(&guest)
    );
    let (_, _, bad) = lab.progress("b").expect("its hashes are logged");
    assert_eq!(bad, 0, "the guest's hashes up to the end");
    assert!(
        lab.read("lobby")
            .contains("host-log: room 1's host went quiet"),
        "the lobby noticed the silence"
    );
}

#[test]
#[ignore = "needs a disc image"]
fn a_host_whose_profile_key_is_on_the_ban_file_is_refused_with_the_reason() {
    let Some(image) = image() else {
        return;
    };
    // The key the game will find in its profile, made the way the game makes it.
    let scratch = oag_testdata::scratch::dir("hosted-banned-key");
    let _ = std::fs::remove_dir_all(&scratch);
    let key = oag_game::identity::load_or_create_at(&scratch.join("identity.toml"))
        .expect("a key is made");
    let bans = scratch.join("bans.txt");
    let hex: String = key.public().iter().map(|b| format!("{b:02x}")).collect();
    std::fs::write(
        &bans,
        format!("key {hex} 2026-11-18T00:00:00.000Z altered a race\n"),
    )
    .expect("the ban file");
    let bans = bans.display().to_string();
    let lab = Lab::with_lobby_args("hosted-banned", image, 20_000, &["--ban-file", &bans]);
    let profile = lab.dir.join("a").join("config").join("oag");
    std::fs::create_dir_all(&profile).expect("the profile");
    std::fs::copy(scratch.join("identity.toml"), profile.join("identity.toml")).expect("the key");
    let mut a = lab.client_with("a", true, &HOST_ARGS);
    let status = a.finish(120);
    let log = lab.read("a");
    assert!(
        status.is_some_and(|s| !s.success()),
        "the banned host does not get a room:\n{}",
        tail(&log)
    );
    assert!(
        log.contains("Banned") && log.contains("you may not host on this lobby: altered a race"),
        "it is told why:\n{}",
        tail(&log)
    );
    assert!(
        !lab.read("lobby").contains("hosts room"),
        "nothing was listed"
    );
}

#[test]
#[ignore = "needs a disc image"]
fn a_host_that_alters_its_race_is_proven_by_the_lobby_and_struck_and_the_guest_is_told_why() {
    let Some(image) = image() else {
        return;
    };
    let scratch = oag_testdata::scratch::dir("hosted-cheat-bans");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let bans = scratch.join("bans.txt");
    let bans = bans.display().to_string();
    let lab = Lab::with_lobby_args("hosted-cheat", image, 20_000, &["--ban-file", &bans]);
    // The host's game is the tests' own build, which cheats when asked (`oag_net::host::Cheat`):
    // its own craft moves further than the simulation moves it from tick 300 on.
    let a = lab.client_env("a", true, &HOST_ARGS, &[("OAG_TEST_CHEAT", "speed")]);
    lab.wait("the lobby to list the hosted room", 90, |lab| {
        lab.read("a")
            .contains("online: room 1 (player-hosted): 1 in it")
    });
    let mut b = lab.client_with("b", false, &[]);
    lab.wait("the guest to be told the host was caught", 240, |lab| {
        lab.read("b").contains("online: the lobby closed the room:")
    });
    let guest = lab.read("b");
    assert!(
        guest.contains("online: reporting the host to the lobby: Desync"),
        "the guest reported:\n{}",
        tail(&guest)
    );
    assert!(
        guest.contains("its host was caught cheating"),
        "the guest is told why:\n{}",
        tail(&guest)
    );
    let lobby = lab.read("lobby");
    assert!(
        lobby.contains("\"b\"'s report is proven") && lobby.contains("gets a strike (1 of 2"),
        "the lobby proved it:\n{}",
        tail(&lobby)
    );
    let key = oag_game::identity::load_or_create_at(
        &lab.dir
            .join("a")
            .join("config")
            .join("oag")
            .join("identity.toml"),
    )
    .expect("the host's key");
    let hex: String = key.public().iter().map(|b| format!("{b:02x}")).collect();
    let written = std::fs::read_to_string(&bans).expect("the strike was written");
    assert!(
        written
            .lines()
            .any(|line| line.starts_with(&format!("strike {hex} 20")) && line.contains(" by ")),
        "the host's key is struck with its date and reporter: {written}"
    );
    assert!(
        !written.lines().any(|line| line.starts_with("key ")),
        "one proven case bans nobody: {written}"
    );
    assert!(!lobby.contains("banned for"), "{}", tail(&lobby));
    let status = b.finish(60);
    assert!(status.is_some(), "the guest's race ended by itself");
    drop(a);
}

/// `log` without the `net: tick` line printed once a second, which crowds out of a failure
/// message the lines that say what happened.
fn without_per_second_lines(log: &str) -> String {
    log.lines()
        .filter(|line| !line.starts_with("net: tick "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tail(log: &str) -> &str {
    let from = log.len().saturating_sub(2000);
    let from = (from..log.len())
        .find(|&i| log.is_char_boundary(i))
        .unwrap_or(log.len());
    &log[from..]
}
