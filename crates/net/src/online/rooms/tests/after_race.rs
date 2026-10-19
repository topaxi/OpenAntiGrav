//! What a room does after its race: the board, a rematch, and removing a person.

use super::*;
use crate::wire::board::BoardRow;

fn board() -> Board {
    Board {
        rows: vec![
            BoardRow {
                place: 1,
                slot: 1,
                name: "bob".into(),
                laps: 3,
                finish_ticks: Some(5000),
                best_lap_ticks: Some(1600),
                eliminations: 0,
            },
            BoardRow {
                place: 2,
                slot: 0,
                name: "ann".into(),
                laps: 3,
                finish_ticks: Some(5100),
                best_lap_ticks: Some(1650),
                eliminations: 0,
            },
        ],
    }
}

/// Ann and Bob in a ready room, raced and over; ann owns it.
fn raced(rooms: &mut Rooms) -> u32 {
    person(rooms, 1, "ann", &[PULSE]);
    person(rooms, 2, "bob", &[PULSE]);
    let room = ready_room(rooms, 1, 2, PULSE);
    rooms.handle(1, Message::Start);
    rooms.tick(0);
    rooms.tick(1000);
    rooms.take_launch().expect("the countdown ran out");
    rooms.take_outbox();
    room
}

#[test]
fn the_board_reaches_every_member_with_the_room_afterwards() {
    let mut rooms = lobby();
    let room = raced(&mut rooms);
    rooms.race_over(room, "every person has finished", board());
    let out = rooms.take_outbox();
    for who in [1, 2] {
        let seen = out
            .iter()
            .rev()
            .find_map(|(to, m)| match m {
                Message::Room(s) if *to == who => Some(s.clone()),
                _ => None,
            })
            .expect("a room push");
        assert_eq!(seen.board, board(), "member {who} sees the server's board");
        assert_eq!(seen.phase, Phase::Open);
        assert!(seen.members.iter().all(|m| !m.ready), "picks are cleared");
    }
}

#[test]
fn a_rematch_from_the_owner_starts_once_everyone_is_ready_with_the_same_settings() {
    let mut rooms = lobby();
    let room = raced(&mut rooms);
    let before = settings(PULSE);
    // The owner asks while the race still runs; Bob has not.
    rooms.handle(1, Message::Rematch);
    rooms.take_outbox();
    rooms.race_over(room, "every person has finished", board());
    let now = snapshot(last(&mut rooms, 2));
    assert_eq!(now.phase, Phase::Open, "Bob is not ready, so it waits");
    assert!(
        now.note.starts_with("every person has finished") && now.note.contains("waiting for: bob"),
        "it keeps the reason and says who: {}",
        now.note
    );
    assert!(now.members[0].ready && !now.members[1].ready);
    assert_eq!(now.settings, before, "the settings are untouched");
    // Bob agrees: the start fires by itself.
    rooms.handle(2, Message::Rematch);
    let queued = snapshot(last(&mut rooms, 1));
    assert!(matches!(queued.phase, Phase::Queued { .. }));
    rooms.tick(10_000);
    rooms.tick(11_000);
    let launch = rooms.take_launch().expect("a second race launched");
    assert_eq!(launch.room, room);
    assert_eq!(launch.params.track, before.track);
    assert_eq!(launch.params.class, before.class);
    assert_eq!(launch.params.mode, before.mode);
    assert_eq!(
        launch.params.teams,
        ["Feisar", "Qirex"],
        "the picks were kept"
    );
}

#[test]
fn a_member_who_asked_while_racing_is_ready_when_it_ends() {
    let mut rooms = lobby();
    let room = raced(&mut rooms);
    rooms.handle(2, Message::Rematch);
    rooms.race_over(room, "every person has finished", board());
    let now = snapshot(last(&mut rooms, 1));
    assert!(!now.members[0].ready && now.members[1].ready);
    assert_eq!(
        now.phase,
        Phase::Open,
        "no owner request, so nothing starts"
    );
}

#[test]
fn a_rematch_dies_when_too_few_people_are_left() {
    let mut rooms = lobby();
    let room = raced(&mut rooms);
    rooms.handle(1, Message::Rematch);
    rooms.race_over(room, "every person has finished", board());
    rooms.take_outbox();
    rooms.handle(2, Message::Leave);
    rooms.take_outbox();
    person(&mut rooms, 3, "cy", &[PULSE]);
    rooms.handle(3, Message::JoinRoom { room });
    rooms.handle(
        3,
        Message::Ready {
            team: "Qirex".into(),
            ready: true,
            assist: 0,
        },
    );
    let now = snapshot(last(&mut rooms, 1));
    assert_eq!(
        now.phase,
        Phase::Open,
        "the owner's request was dropped with bob"
    );
}

#[test]
fn an_owner_leaving_hands_the_room_over_without_the_old_request() {
    let mut rooms = lobby();
    let room = raced(&mut rooms);
    rooms.handle(1, Message::Rematch);
    rooms.race_over(room, "every person has finished", board());
    rooms.take_outbox();
    rooms.handle(1, Message::Leave);
    let now = snapshot(last(&mut rooms, 2));
    assert!(now.members[0].owner && now.members[0].name == "bob");
    assert_eq!(now.phase, Phase::Open);
}

#[test]
fn a_rematch_needs_a_pick() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE]);
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    rooms.take_outbox();
    rooms.handle(1, Message::Rematch);
    assert_eq!(refusal(last(&mut rooms, 1)), Refusal::NotReady);
}

#[test]
fn only_the_owner_can_remove_someone_and_they_are_refused_back() {
    let mut rooms = lobby();
    person(&mut rooms, 1, "ann", &[PULSE]);
    person(&mut rooms, 2, "bob", &[PULSE]);
    person(&mut rooms, 3, "cy", &[PULSE]);
    rooms.handle(1, Message::CreateRoom(settings(PULSE)));
    let room = snapshot(last(&mut rooms, 1)).id;
    rooms.handle(2, Message::JoinRoom { room });
    rooms.handle(3, Message::JoinRoom { room });
    rooms.take_outbox();
    rooms.handle(2, Message::Kick { name: "cy".into() });
    assert_eq!(refusal(last(&mut rooms, 2)), Refusal::NotOwner);
    rooms.handle(1, Message::Kick { name: "ann".into() });
    assert_eq!(refusal(last(&mut rooms, 1)), Refusal::BadSettings);
    rooms.handle(
        1,
        Message::Kick {
            name: "nobody".into(),
        },
    );
    assert_eq!(refusal(last(&mut rooms, 1)), Refusal::NotInRoom);
    rooms.handle(1, Message::Kick { name: "CY".into() });
    let out = rooms.take_outbox();
    let told = out
        .iter()
        .find_map(|(to, m)| match m {
            Message::Left { reason } if *to == 3 => Some(reason.clone()),
            _ => None,
        })
        .expect("the removed person is told why");
    assert!(told.contains("removed"), "{told}");
    assert_eq!(rooms.room_of(3), None);
    let after = out
        .iter()
        .rev()
        .find_map(|(to, m)| match m {
            Message::Room(s) if *to == 2 => Some(s.clone()),
            _ => None,
        })
        .expect("the others see the room");
    assert_eq!(after.members.len(), 2);
    assert!(after.note.contains("cy was removed"), "{}", after.note);
    // Refused back, in the list as at the join.
    rooms.handle(3, Message::JoinRoom { room });
    assert_eq!(refusal(last(&mut rooms, 3)), Refusal::Kicked);
    assert!(!rooms.list(3)[0].joinable);
    // The ban is the room's: another room takes them, and a new session of the same name is
    // still refused.
    rooms.disconnect(3);
    person(&mut rooms, 4, "Cy", &[PULSE]);
    rooms.handle(4, Message::JoinRoom { room });
    assert_eq!(refusal(last(&mut rooms, 4)), Refusal::Kicked);
}

#[test]
fn nobody_is_removed_while_the_race_is_starting_or_running() {
    let mut rooms = lobby();
    let room = raced(&mut rooms);
    rooms.handle(1, Message::Kick { name: "bob".into() });
    assert_eq!(refusal(last(&mut rooms, 1)), Refusal::Started);
    assert_eq!(rooms.room_of(2), Some(room));
}
