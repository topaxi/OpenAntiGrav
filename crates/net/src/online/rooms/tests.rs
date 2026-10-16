use super::*;

const PULSE: &str = "Wipeout Pulse";
const HD: &str = "Wipeout HD";

fn edition(title: &str) -> Edition {
    Edition {
        title: title.into(),
        platform: "PSP".into(),
        content: 0,
    }
}

fn lobby() -> Rooms {
    Rooms::new(Config {
        titles: vec![edition(PULSE), edition(HD)],
        countdown_ms: 1_000,
        ..Config::default()
    })
}

fn settings(title: &str) -> RoomSettings {
    RoomSettings {
        title: title.into(),
        track: "01_Track".into(),
        class: "VENOM".into(),
        mode: "single_race".into(),
        max_players: 3,
        opponents: true,
    }
}

fn person(rooms: &mut Rooms, id: ClientId, name: &str, titles: &[&str]) {
    rooms
        .connect(id, name, titles.iter().map(|t| edition(t)).collect())
        .expect("a free name");
}

/// The last message `id` was sent, draining the outbox.
fn last(rooms: &mut Rooms, id: ClientId) -> Option<Message> {
    rooms
        .take_outbox()
        .into_iter()
        .rev()
        .find(|(to, _)| *to == id)
        .map(|(_, m)| m)
}

fn refusal(message: Option<Message>) -> Refusal {
    match message {
        Some(Message::Rejected { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn snapshot(message: Option<Message>) -> Snapshot {
    match message {
        Some(Message::Room(s)) => s,
        other => panic!("expected a room, got {other:?}"),
    }
}

/// Two people in a ready room, ann owning it.
fn ready_room(rooms: &mut Rooms, owner: ClientId, guest: ClientId, title: &str) -> u32 {
    rooms.handle(owner, Message::CreateRoom(settings(title)));
    let id = snapshot(last(rooms, owner)).id;
    rooms.handle(guest, Message::JoinRoom { room: id });
    for (who, team) in [(owner, "Feisar"), (guest, "Qirex")] {
        rooms.handle(
            who,
            Message::Ready {
                team: team.into(),
                ready: true,
            },
        );
    }
    rooms.take_outbox();
    id
}

#[test]
fn a_name_is_unique_and_bounded() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "Ann", &[PULSE]);
    assert!(rooms.connect(2, "ann", vec![]).is_err(), "case-blind");
    assert!(rooms.connect(2, "", vec![]).is_err());
    assert!(rooms.connect(2, &"x".repeat(25), vec![]).is_err());
    rooms.disconnect(1);
    assert!(
        rooms.connect(2, "Ann", vec![]).is_ok(),
        "freed on disconnect"
    );
}

#[test]
fn the_list_shows_only_rooms_in_a_title_the_asker_can_race() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE, HD]);
    person(&mut rooms, 2, "bob", &[PULSE]);
    rooms.handle(1, Message::CreateRoom(settings(HD)));
    rooms.handle(1, Message::Leave);
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    rooms.take_outbox();
    assert_eq!(
        rooms.list(1).len(),
        1,
        "ann sees her own, the HD room is gone with its owner"
    );
    person(&mut rooms, 3, "cy", &[HD]);
    rooms.handle(3, Message::CreateRoom(settings(HD)));
    rooms.take_outbox();
    let bobs: Vec<String> = rooms
        .list(2)
        .iter()
        .map(|r| r.settings.title.clone())
        .collect();
    assert_eq!(
        bobs,
        [PULSE],
        "bob has no HD image so he is not shown the HD room"
    );
    rooms.handle(2, Message::JoinRoom { room: 3 });
    assert_eq!(refusal(last(&mut rooms, 2)), Refusal::Content);
}

#[test]
fn a_pressing_or_a_mod_set_that_differs_from_the_servers_hides_the_room_and_refuses_the_join() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE]);
    rooms
        .connect(
            2,
            "ps2",
            vec![Edition {
                platform: "PS2".into(),
                ..edition(PULSE)
            }],
        )
        .expect("name");
    rooms
        .connect(
            3,
            "modded",
            vec![Edition {
                content: 0xBEEF,
                ..edition(PULSE)
            }],
        )
        .expect("name");
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    rooms.take_outbox();
    assert_eq!(rooms.list(1).len(), 1);
    assert!(
        rooms.list(2).is_empty(),
        "a PS2 pressing is not the server's PSP one"
    );
    assert!(
        rooms.list(3).is_empty(),
        "other mods would simulate differently"
    );
    rooms.handle(2, Message::JoinRoom { room: 1 });
    assert_eq!(refusal(last(&mut rooms, 2)), Refusal::Content);
    rooms.handle(3, Message::JoinRoom { room: 1 });
    match last(&mut rooms, 3) {
        Some(Message::Rejected {
            code: Refusal::Content,
            text,
        }) => {
            assert!(text.contains("mods"), "{text}");
        }
        other => panic!("{other:?}"),
    }
    rooms.handle(3, Message::CreateRoom(settings(PULSE)));
    assert_eq!(
        refusal(last(&mut rooms, 3)),
        Refusal::Content,
        "nor can they make one"
    );
}

#[test]
fn a_room_is_made_only_in_a_title_the_server_and_the_maker_have() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE, "Wipeout Pure"]);
    rooms.handle(1, Message::CreateRoom(settings("Wipeout Pure")));
    assert_eq!(
        refusal(last(&mut rooms, 1)),
        Refusal::BadSettings,
        "server has no Pure"
    );
    rooms.handle(1, Message::CreateRoom(settings(HD)));
    assert_eq!(
        refusal(last(&mut rooms, 1)),
        Refusal::Content,
        "ann has no HD"
    );
    let mut bad = settings(PULSE);
    bad.mode = "no_such_mode".into();
    rooms.handle(1, Message::CreateRoom(bad));
    assert_eq!(refusal(last(&mut rooms, 1)), Refusal::BadSettings);
}

#[test]
fn join_refuses_full_started_and_unknown_rooms_by_name() {
    let mut rooms = lobby();
    for (id, name) in [(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "e")] {
        person(&mut rooms, id, name, &[PULSE]);
    }
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    rooms.take_outbox();
    rooms.handle(2, Message::JoinRoom { room: 1 });
    rooms.handle(3, Message::JoinRoom { room: 1 });
    rooms.take_outbox();
    rooms.handle(4, Message::JoinRoom { room: 1 });
    assert_eq!(refusal(last(&mut rooms, 4)), Refusal::Full);
    rooms.handle(4, Message::JoinRoom { room: 99 });
    assert_eq!(refusal(last(&mut rooms, 4)), Refusal::NoSuchRoom);
    for who in [1, 2, 3] {
        rooms.handle(
            who,
            Message::Ready {
                team: "Feisar".into(),
                ready: true,
            },
        );
    }
    rooms.handle(1, Message::Start);
    rooms.take_outbox();
    rooms.handle(5, Message::JoinRoom { room: 1 });
    assert_eq!(refusal(last(&mut rooms, 5)), Refusal::Started);
}

#[test]
fn the_start_gate_wants_people_then_picks_then_the_owner() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE]);
    person(&mut rooms, 2, "bob", &[PULSE]);
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    rooms.take_outbox();
    rooms.handle(1, Message::Start);
    assert_eq!(refusal(last(&mut rooms, 1)), Refusal::NeedPlayers);
    rooms.handle(2, Message::JoinRoom { room: 1 });
    rooms.take_outbox();
    rooms.handle(1, Message::Start);
    assert_eq!(refusal(last(&mut rooms, 1)), Refusal::NotReady);
    rooms.handle(
        2,
        Message::Ready {
            team: String::new(),
            ready: true,
        },
    );
    assert_eq!(
        refusal(last(&mut rooms, 2)),
        Refusal::NotReady,
        "ready needs a pick"
    );
    for (who, team) in [(1, "Feisar"), (2, "Qirex")] {
        rooms.handle(
            who,
            Message::Ready {
                team: team.into(),
                ready: true,
            },
        );
    }
    rooms.take_outbox();
    rooms.handle(2, Message::Start);
    assert_eq!(refusal(last(&mut rooms, 2)), Refusal::NotOwner);
    rooms.handle(1, Message::Start);
    assert!(matches!(
        snapshot(last(&mut rooms, 2)).phase,
        Phase::Queued { position: 1 }
    ));
}

#[test]
fn the_owner_migrates_to_the_next_person_when_they_leave() {
    let mut rooms = lobby();
    for (id, name) in [(1, "ann"), (2, "bob"), (3, "cy")] {
        person(&mut rooms, id, name, &[PULSE]);
    }
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    rooms.handle(2, Message::JoinRoom { room: 1 });
    rooms.handle(3, Message::JoinRoom { room: 1 });
    rooms.take_outbox();
    rooms.handle(1, Message::Leave);
    let seen = snapshot(last(&mut rooms, 3));
    assert_eq!(seen.members[0].name, "bob");
    assert!(seen.members[0].owner && !seen.members[1].owner);
    assert!(
        seen.note.contains("bob has been made host"),
        "{}",
        seen.note
    );
    rooms.handle(
        2,
        Message::Edit(RoomSettings {
            max_players: 2,
            ..settings(PULSE)
        }),
    );
    let after = snapshot(last(&mut rooms, 2));
    assert_eq!(after.settings.max_players, 2, "the new owner edits");
    assert_eq!(after.members.len(), 2);
    rooms.handle(3, Message::Edit(settings(PULSE)));
    assert_eq!(refusal(last(&mut rooms, 3)), Refusal::NotOwner);
}

#[test]
fn an_edit_that_picks_content_a_member_lacks_asks_them_to_go() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE, HD]);
    person(&mut rooms, 2, "bob", &[PULSE]);
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    rooms.handle(2, Message::JoinRoom { room: 1 });
    rooms.take_outbox();
    rooms.handle(1, Message::Edit(settings(HD)));
    let out = rooms.take_outbox();
    assert!(out.iter().any(|(to, m)| *to == 2
        && matches!(m, Message::Left { reason } if reason.contains("content that you do not have"))));
    assert_eq!(rooms.room_of(2), None);
    assert_eq!(rooms.room_of(1), Some(1));
}

#[test]
fn one_game_slot_and_a_fifo_of_rooms_waiting_for_it() {
    let mut rooms = lobby();
    for (id, name) in [
        (1, "a1"),
        (2, "a2"),
        (3, "b1"),
        (4, "b2"),
        (5, "c1"),
        (6, "c2"),
    ] {
        person(&mut rooms, id, name, &[PULSE]);
    }
    let a = ready_room(&mut rooms, 1, 2, PULSE);
    let b = ready_room(&mut rooms, 3, 4, PULSE);
    let c = ready_room(&mut rooms, 5, 6, PULSE);
    rooms.handle(1, Message::Start);
    rooms.handle(3, Message::Start);
    rooms.handle(5, Message::Start);
    rooms.take_outbox();
    rooms.tick(0);
    assert_eq!(
        rooms.active(),
        Some(a),
        "first to press START takes the slot"
    );
    rooms.tick(999);
    assert!(
        rooms.take_launch().is_none(),
        "the countdown has not run out"
    );
    rooms.take_outbox();
    rooms.tick(1000);
    let launch = rooms.take_launch().expect("the countdown ran out");
    assert_eq!(launch.room, a);
    rooms.take_outbox();
    // While a races, b is told it is next and c second.
    rooms.tick(5000);
    rooms.handle(3, Message::Leave);
    rooms.take_outbox();
    assert_eq!(rooms.active(), Some(a));
    assert!(rooms.take_launch().is_none(), "b and c wait for the slot");
    rooms.race_over(a, "every person has finished");
    rooms.take_outbox();
    // b lost its owner and is below the minimum now: skipped, and c gets the slot.
    rooms.tick(6000);
    assert_eq!(rooms.active(), Some(c), "b no longer qualifies");
    let _ = b;
}

#[test]
fn the_queue_tells_each_room_its_place() {
    let mut rooms = lobby();
    for (id, name) in [
        (1, "a1"),
        (2, "a2"),
        (3, "b1"),
        (4, "b2"),
        (5, "c1"),
        (6, "c2"),
    ] {
        person(&mut rooms, id, name, &[PULSE]);
    }
    ready_room(&mut rooms, 1, 2, PULSE);
    ready_room(&mut rooms, 3, 4, PULSE);
    ready_room(&mut rooms, 5, 6, PULSE);
    rooms.handle(1, Message::Start);
    rooms.tick(0);
    rooms.take_outbox();
    rooms.handle(3, Message::Start);
    rooms.handle(5, Message::Start);
    assert!(matches!(
        snapshot(last(&mut rooms, 3)).phase,
        Phase::Queued { position: 1 }
    ));
    rooms.handle(
        5,
        Message::Ready {
            team: "Qirex".into(),
            ready: false,
        },
    );
    rooms.take_outbox();
    rooms.handle(
        5,
        Message::Ready {
            team: "Qirex".into(),
            ready: true,
        },
    );
    rooms.handle(5, Message::Start);
    assert!(matches!(
        snapshot(last(&mut rooms, 5)).phase,
        Phase::Queued { position: 2 }
    ));
}

#[test]
fn people_leaving_a_queued_room_below_the_minimum_cancel_its_start() {
    let mut rooms = lobby();
    for (id, name) in [(1, "a1"), (2, "a2"), (3, "b1"), (4, "b2")] {
        person(&mut rooms, id, name, &[PULSE]);
    }
    ready_room(&mut rooms, 1, 2, PULSE);
    let b = ready_room(&mut rooms, 3, 4, PULSE);
    rooms.handle(1, Message::Start);
    rooms.tick(0);
    rooms.handle(3, Message::Start);
    rooms.take_outbox();
    rooms.disconnect(4);
    let seen = snapshot(last(&mut rooms, 3));
    assert_eq!(seen.phase, Phase::Open, "{seen:?}");
    assert!(seen.note.contains("too few"), "{}", seen.note);
    let _ = b;
}

#[test]
fn a_race_that_ends_returns_its_people_to_the_open_room_with_the_reason() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE]);
    person(&mut rooms, 2, "bob", &[PULSE]);
    let room = ready_room(&mut rooms, 1, 2, PULSE);
    rooms.handle(1, Message::Start);
    rooms.tick(0);
    rooms.tick(1000);
    rooms.take_launch().expect("launched");
    rooms.take_outbox();
    rooms.race_over(room, "the race process exited with status 1");
    let seen = snapshot(last(&mut rooms, 2));
    assert_eq!(seen.phase, Phase::Open);
    assert!(seen.members.iter().all(|m| !m.ready), "picks are cleared");
    assert!(seen.note.contains("status 1"));
    assert_eq!(rooms.active(), None, "the slot is free");
}

#[test]
fn the_launch_carries_every_setting_and_seat_into_the_race_params() {
    let mut rooms = lobby();
    for (id, name) in [(1, "ann"), (2, "bob"), (3, "cy")] {
        person(&mut rooms, id, name, &[PULSE]);
    }
    rooms.handle(
        1,
        Message::CreateRoom(RoomSettings {
            track: "03_Track".into(),
            class: "FLASH".into(),
            mode: "time_trial".into(),
            max_players: 3,
            opponents: false,
            ..settings(PULSE)
        }),
    );
    rooms.handle(2, Message::JoinRoom { room: 1 });
    rooms.handle(3, Message::JoinRoom { room: 1 });
    for (who, team) in [(1, "Feisar"), (2, "Qirex"), (3, "Auricom")] {
        rooms.handle(
            who,
            Message::Ready {
                team: team.into(),
                ready: true,
            },
        );
    }
    rooms.handle(1, Message::Start);
    rooms.tick(0);
    rooms.tick(1000);
    let launch = rooms.take_launch().expect("launched");
    let p = &launch.params;
    assert_eq!(
        (
            p.title.as_str(),
            p.track.as_str(),
            p.class.as_str(),
            p.mode.as_str()
        ),
        (PULSE, "03_Track", "FLASH", "time_trial")
    );
    assert_eq!((p.people, p.opponents, p.weapons), (3, false, false));
    assert_eq!(
        p.teams,
        ["Feisar", "Qirex", "Auricom"],
        "seat order is join order"
    );
    assert_eq!(
        launch
            .seats
            .iter()
            .map(|(_, n)| n.as_str())
            .collect::<Vec<_>>(),
        ["ann", "bob", "cy"]
    );
}
