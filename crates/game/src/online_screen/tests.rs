use super::*;
use oag_net::wire::online::RoomSettings;
use oag_ui_screens::join::{Event, Labels};

fn strings() -> StringTable {
    StringTable::from_xml("<Strings/>")
}

fn room(id: u32, owner: &str, members: u8, joinable: bool) -> RoomInfo {
    RoomInfo {
        id,
        owner: owner.into(),
        settings: RoomSettings {
            title: "Wipeout Pulse".into(),
            track: "Data\\Environments\\02_Track\\track.vex".into(),
            class: "venom".into(),
            mode: "single_race".into(),
            max_players: 8,
            opponents: false,
        },
        members,
        joinable,
    }
}

fn list(rooms: &[RoomInfo]) -> Join {
    let mut screen = Join::new(Labels::read_online_list(&strings()), 16).as_list();
    fill_list(&mut screen, rooms, &strings(), &|_| None);
    screen
}

#[test]
fn a_port_is_added_only_when_none_is_given() {
    assert_eq!(with_port("race.example.org"), "race.example.org:60000");
    assert_eq!(with_port(" 10.0.0.2:7000 "), "10.0.0.2:7000");
    assert_eq!(with_port("host:notaport"), "host:notaport:60000");
}

#[test]
fn every_room_is_a_row_and_a_closed_one_cannot_be_chosen() {
    let mut screen = list(&[room(1, "ada", 1, true), room(2, "bob", 8, false)]);
    assert_eq!(screen.count(), 2);
    // Down starts on the first row; confirming a joinable room asks to join row 0.
    screen.step(1);
    assert_eq!(screen.confirm(), Some(Event::Leave(0)));
    // The closed room answers nothing.
    screen.step(1);
    assert_eq!(
        screen.confirm(),
        None,
        "a room that cannot be joined is dimmed"
    );
}

#[test]
fn a_rooms_circuit_is_named_by_the_title_when_it_can_be() {
    let ada = room(1, "ada", 1, true);
    let named = |entry: &str| entry.contains("02_Track").then(|| "Metropia".to_string());
    assert_eq!(
        room_line(&ada, &strings(), &named),
        "METROPIA / VENOM / SINGLE RACE"
    );
    assert_eq!(
        room_line(&ada, &strings(), &|_| None),
        "02_TRACK / VENOM / SINGLE RACE",
        "the folder stands in when the title names nothing"
    );
    let mut closed = room(2, "bob", 8, false);
    closed.settings.track.clear();
    assert!(room_line(&closed, &strings(), &named).starts_with("NOT JOINABLE"));
}

#[test]
fn the_room_says_who_is_ready_and_where_the_room_is() {
    let members = vec![
        Member {
            name: "ada".into(),
            team: "Feisar".into(),
            ready: true,
            owner: true,
        },
        Member {
            name: "bob".into(),
            team: String::new(),
            ready: false,
            owner: false,
        },
    ];
    let snapshot = |phase, note: &str| Snapshot {
        id: 1,
        settings: room(1, "ada", 2, true).settings,
        members: members.clone(),
        phase,
        note: note.into(),
    };
    assert_eq!(member_line(&members[0], &strings()), "ADA (HOST)  READY");
    assert_eq!(member_line(&members[1], &strings()), "BOB  PICKING");
    assert_eq!(
        room_hint(&snapshot(Phase::Open, ""), true, &strings()),
        "PRESS START WHEN EVERYONE IS READY"
    );
    assert_eq!(
        room_hint(&snapshot(Phase::Open, ""), false, &strings()),
        "THE HOST STARTS THE RACE"
    );
    assert_eq!(
        room_hint(
            &snapshot(Phase::Queued { position: 2 }, ""),
            false,
            &strings()
        ),
        "ANOTHER GAME IS RACING: YOU ARE NUMBER 2 IN LINE"
    );
    assert_eq!(
        room_hint(
            &snapshot(Phase::Countdown { millis: 1200 }, ""),
            true,
            &strings()
        ),
        "THE RACE STARTS IN 2"
    );
    assert_eq!(
        room_hint(&snapshot(Phase::Open, "bob left"), true, &strings()),
        "BOB LEFT - PRESS START WHEN EVERYONE IS READY",
        "the room's own note leads"
    );
}

#[test]
fn the_room_screen_seats_everyone_in_the_room_and_leaves_the_rest_waiting() {
    let members: Vec<Member> = ["ada", "bob", "cyd"]
        .iter()
        .enumerate()
        .map(|(n, name)| Member {
            name: (*name).into(),
            team: String::new(),
            ready: true,
            owner: n == 0,
        })
        .collect();
    let snapshot = Snapshot {
        id: 1,
        settings: room(1, "ada", 3, true).settings,
        members,
        phase: Phase::Open,
        note: String::new(),
    };
    let mut screen = Join::new(Labels::read_online_room(&strings()), 8);
    fill_room(&mut screen, &snapshot, false, &strings());
    assert_eq!(screen.count(), 3);
    assert_eq!(screen.labels().hint, "THE HOST STARTS THE RACE");
    fill_room(&mut screen, &snapshot, true, &strings());
    assert_eq!(screen.labels().hint, "PRESS START WHEN EVERYONE IS READY");
}

#[test]
fn a_room_pick_without_a_ship_select_falls_back_to_the_titles_own_team_not_to_nothing() {
    let roster: Vec<oag_ui::menu::Choice> = ["qirex2048\\3", "feisar2048\\3", "auricom2048\\3"]
        .iter()
        .map(|id| oag_ui::menu::Choice::labelled(*id, *id))
        .collect();
    // The settings' stored team is not in 2048's roster, so the page left none picked.
    assert_eq!(room_team(None, "Feisar2048\\3", &roster), "feisar2048\\3");
    assert_eq!(
        room_team(None, "nobody", &roster),
        "qirex2048\\3",
        "the first, as a last resort"
    );
    assert_eq!(
        room_team(Some("auricom2048\\3"), "feisar2048\\3", &roster),
        "auricom2048\\3"
    );
    assert_eq!(room_team(None, "x", &[]), "");
}
