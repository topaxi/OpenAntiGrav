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
    assert_eq!(
        with_port("::1"),
        "[::1]:60000",
        "a bare IPv6 literal is bracketed"
    );
    assert_eq!(with_port("[::1]"), "[::1]:60000");
    assert_eq!(with_port("[2001:db8::7]:7000"), "[2001:db8::7]:7000");
    assert_eq!(
        with_port("[::1]:x"),
        "[::1]:x:60000",
        "a bad port is not taken for one"
    );
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
    fill_room(&mut screen, &snapshot, false, &strings(), &|_| None);
    assert_eq!(screen.count(), 3);
    assert_eq!(screen.labels().hint, "THE HOST STARTS THE RACE");
    fill_room(&mut screen, &snapshot, true, &strings(), &|_| None);
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

fn snapshot_of(people: usize, max_players: u8, phase: Phase) -> Snapshot {
    let mut settings = room(1, "ada", 1, true).settings;
    settings.max_players = max_players;
    Snapshot {
        id: 1,
        settings,
        members: (0..people)
            .map(|n| Member {
                name: format!("p{n}"),
                team: String::new(),
                ready: true,
                owner: n == 0,
            })
            .collect(),
        phase,
        note: String::new(),
    }
}

#[test]
fn the_owner_gets_the_settings_rows_and_the_room_has_as_many_seats_as_its_size() {
    use oag_ui_screens::join::Row;
    let mut screen = Join::new(Labels::read_online_room(&strings()), 8);
    fill_room(
        &mut screen,
        &snapshot_of(2, 2, Phase::Open),
        true,
        &strings(),
        &|_| None,
    );
    assert!(screen.rows().contains(&Row::Settings) && screen.rows().contains(&Row::MaxPlayers));
    assert!(
        !screen.rows().contains(&Row::Empty),
        "two seats, two people: the room is full"
    );
    fill_room(
        &mut screen,
        &snapshot_of(2, 5, Phase::Open),
        true,
        &strings(),
        &|_| None,
    );
    assert!(
        screen.rows().contains(&Row::Empty),
        "a size of five leaves seats"
    );
    // A guest, or an owner once the room is queued or racing, has neither row.
    fill_room(
        &mut screen,
        &snapshot_of(2, 5, Phase::Open),
        false,
        &strings(),
        &|_| None,
    );
    assert!(!screen.rows().contains(&Row::Settings));
    fill_room(
        &mut screen,
        &snapshot_of(2, 5, Phase::Racing),
        true,
        &strings(),
        &|_| None,
    );
    assert!(!screen.rows().contains(&Row::MaxPlayers));
}

#[test]
fn a_seat_says_what_the_owner_chose_while_the_cursor_is_on_it() {
    let mut screen = Join::new(Labels::read_online_room(&strings()), 8);
    fill_room(
        &mut screen,
        &snapshot_of(2, 8, Phase::Open),
        false,
        &strings(),
        &|_| Some("VINETA K".into()),
    );
    screen.step(1);
    let hovered = oag_ui_screens::join::draw_list(
        &screen,
        &oag_ui::menu::Skin::new(
            oag_pulse::FRONT_END.menu.unwrap(),
            oag_display::space::Space::PSP,
            22.0,
        ),
        &Default::default(),
        None,
        false,
    )
    .flatten();
    let text: Vec<String> = hovered
        .iter()
        .filter_map(|d| match d {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        text.iter().any(|t| t == "VINETA K / VENOM / SINGLE RACE"),
        "{text:?}"
    );
}

#[test]
fn max_players_steps_by_one_and_wraps_between_the_fewest_and_the_most() {
    assert_eq!(
        step_max_players(8, 1, 2, 8),
        2,
        "past the most is the fewest"
    );
    assert_eq!(step_max_players(2, -1, 2, 8), 8);
    assert_eq!(step_max_players(5, 1, 3, 8), 6);
    assert_eq!(
        step_max_players(3, -1, 3, 8),
        8,
        "never below the people seated"
    );
    assert_eq!(
        step_max_players(2, 1, 2, 2),
        2,
        "a full two-seat room has one size"
    );
}

#[test]
fn a_long_note_is_never_dropped_for_the_start_instruction() {
    // What a race child that came up and then died leaves as the room's note (its exit and
    // the last line it said), longer than the line has room for with the hint after it.
    let note = "the race process exited with status 3: error: the circuit would not load";
    let mut snapshot = snapshot_of(2, 8, Phase::Open);
    snapshot.note = note.into();
    assert_eq!(room_hint(&snapshot, true, &strings()), note.to_uppercase());
    // A short note still shares the line with the instruction.
    snapshot.note = "bob joined".into();
    assert_eq!(
        room_hint(&snapshot, true, &strings()),
        "BOB JOINED - PRESS START WHEN EVERYONE IS READY"
    );
}
