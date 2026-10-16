use super::*;
use oag_ui::menu::Frame;

fn skin() -> Skin {
    Skin::new(
        oag_pure::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn labels() -> Labels {
    Labels {
        title: "SPLIT SCREEN".into(),
        hint: "PRESS A BUTTON TO JOIN".into(),
        player: "PLAYER".into(),
        empty: "OPEN".into(),
        guest: None,
        keyboard: "KEYBOARD".into(),
        pad: "GAMEPAD".into(),
        settings: "ROOM SETTINGS".into(),
        max_players: "MAX PLAYERS".into(),
        laps: "LAPS".into(),
        laps_own: "OWN".into(),
        start: "START".into(),
        back: "BACK".into(),
    }
}

fn screen(seated: usize) -> Join {
    let mut join = Join::new(labels(), 8);
    join.set_seats((0..seated).map(|n| format!("PAD {n}")).collect());
    join
}

/// The centre of row `index` in the grid the skin draws in.
fn centre(skin: &Skin, index: usize) -> (f32, f32) {
    let layout = Layout::new(skin);
    (
        layout.width / 2.0,
        layout.top + (index as f32 + 0.5) * layout.line,
    )
}

fn click(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

#[test]
fn the_rows_are_the_seats_then_one_empty_seat_then_start_and_back() {
    assert_eq!(
        screen(0).rows(),
        [Row::Empty, Row::Start, Row::Back],
        "nobody yet"
    );
    assert_eq!(
        screen(2).rows(),
        [
            Row::Seat(0),
            Row::Seat(1),
            Row::Empty,
            Row::Start,
            Row::Back
        ]
    );
}

#[test]
fn a_full_grid_offers_no_empty_seat() {
    let rows = screen(8).rows();
    assert!(!rows.contains(&Row::Empty));
    assert_eq!(rows.len(), 10);
}

#[test]
fn a_click_on_the_empty_row_asks_the_keyboard_to_join() {
    let skin = skin();
    let mut join = screen(1);
    let events = join.pointer(&click(centre(&skin, 1)), &skin);
    assert!(events.contains(&Event::Join), "{events:?}");
}

#[test]
fn a_click_on_a_taken_seat_leaves_it() {
    let skin = skin();
    let mut join = screen(3);
    let events = join.pointer(&click(centre(&skin, 1)), &skin);
    assert!(events.contains(&Event::Leave(1)), "{events:?}");
}

#[test]
fn start_answers_only_with_somebody_seated() {
    let skin = skin();
    let mut nobody = screen(0);
    let at = centre(&skin, 1);
    assert!(!nobody.pointer(&click(at), &skin).contains(&Event::Start));

    let mut two = screen(2);
    let at = centre(&skin, 3);
    assert!(two.pointer(&click(at), &skin).contains(&Event::Start));
}

#[test]
fn back_is_a_click_on_back_or_the_secondary_button() {
    let skin = skin();
    let mut join = screen(1);
    let at = centre(&skin, 3);
    assert!(join.pointer(&click(at), &skin).contains(&Event::Back));
    let secondary = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(join.pointer(&secondary, &skin), [Event::Back]);
}

#[test]
fn hover_follows_a_move_and_an_idle_pointer_does_nothing() {
    let skin = skin();
    let mut join = screen(1);
    assert!(join.pointer(&Pointer::default(), &skin).is_empty());
    let at = centre(&skin, 2);
    let moved = Pointer {
        at: Some(at),
        moved: true,
        ..Pointer::default()
    };
    assert_eq!(join.pointer(&moved, &skin), [Event::Moved]);
    assert_eq!(join.hover(), Some(Row::Start));
}

#[test]
fn the_body_names_every_seat_and_its_device() {
    let join = screen(2);
    let texts: Vec<String> = join
        .body(&skin(), &Frame::default())
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    for want in [
        "PLAYER 1", "PAD 0", "PLAYER 2", "PAD 1", "PLAYER 3", "OPEN", "START",
    ] {
        assert!(
            texts.iter().any(|t| t == want),
            "{want} missing in {texts:?}"
        );
    }
}

fn right_aligned_x(join: &Join, skin: &Skin) -> f32 {
    join.body(skin, &Frame::default())
        .into_iter()
        .find_map(|draw| match draw {
            Draw::Text {
                x,
                align: Align::Right,
                ..
            } => Some(x),
            _ => None,
        })
        .expect("a seated row has a device column")
}

fn hd_skin() -> Skin {
    Skin::new(
        oag_hd::frontend::FRONT_END.menu.unwrap(),
        oag_display::space::Space::HD,
        33.0,
    )
}

fn block_frame() -> Frame {
    let placed = |x: u32, width: u32, height: u32| oag_ui::frontend::Placed {
        x,
        y: 0,
        width,
        height,
        quad_extent: None,
        blend: None,
    };
    Frame {
        ink: Some([0.4, 0.4, 0.4, 1.0]),
        tab_selected: Some([0.54, 0.75, 0.79, 1.0]),
        blocks: Some(oag_ui::menu::block::BlockArt {
            frame: placed(0, 64, 64),
            fill_alpha: 110.0 / 255.0,
            solid_alpha: 1.0,
            cursor: None,
            arrow: None,
            fury: false,
        }),
        ..Frame::default()
    }
}

/// HD's rows used to be mirrored off the strip's column (800 on a 1920 grid) with the title's
/// margin at 194, which left 320 units between a name and its seat's device, so "PLAYER 1"
/// and a pad's name drew over each other. The rows now take HD's own `<List>` geometry. Omega's
/// skin carries HD's numbers to the digit (`oag_omega::frontend::MENU_SKIN`).
#[test]
fn on_hds_grid_the_device_column_clears_the_player_names() {
    let skin = hd_skin();
    let layout = Layout::new(&skin);
    let list = skin.list().unwrap();
    assert!(
        (layout.text_x - list.x).abs() > 1.0,
        "the label is inside its block"
    );
    // "XBOX 360 CONTROLLER" at the list's own text scale is about 350 units.
    assert!(layout.right_x - layout.text_x > 700.0);
    assert!(
        layout.right_x < layout.width - 194.0,
        "inside the title's margin"
    );
    assert!((layout.line - list.pitch).abs() < f32::EPSILON);
}

/// A label and a value block per row, except BACK which has no value; and the same title with
/// a frame that has no block art (or a PSP title) draws none.
#[test]
fn a_title_that_draws_blocks_boxes_its_rows_and_a_bare_one_does_not() {
    let hd = hd_skin();
    let join = screen(2);
    assert!(join.blocks(&hd, &Frame::default()).is_empty());
    let with = join.blocks(&hd, &block_frame());
    // 2 seats + empty + START + BACK = 5 rows, 3 of them with a value block (START and BACK are buttons).
    let plain = {
        let mut one = Vec::new();
        let frame = block_frame();
        let art = frame.blocks.unwrap();
        let block = oag_ui::menu::block::Block {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
            color: [1.0; 4],
            landing: false,
        };
        oag_ui::menu::block::draw(&block, &art, (1.0, 1.0), &mut one);
        one.len()
    };
    assert_eq!(
        with.len(),
        plain * 8,
        "{} draws for eight blocks",
        with.len()
    );
    assert!(
        screen(2).blocks(&skin(), &block_frame()).is_empty(),
        "no list on a PSP skin"
    );
}

/// The PSP titles' rows and title share a margin, so mirroring it is what it always was.
#[test]
fn on_the_psp_grid_the_device_column_is_where_it_was() {
    let skin = skin();
    let (width, _) = skin.space().size;
    assert!((right_aligned_x(&screen(1), &skin) - (width - skin.menu_x())).abs() < f32::EPSILON);
}

/// Where the race box authored the head count, START waits for every seat: a click on it
/// with one of two seated does nothing, and with both it starts.
#[test]
fn start_waits_for_every_seat_when_the_head_count_is_authored() {
    let skin = skin();
    let two = |seated: usize| {
        let mut join = Join::new(labels(), 2).requiring(2);
        join.set_seats((0..seated).map(|n| format!("PAD {n}")).collect());
        join
    };
    let mut one = two(1);
    assert!(!one.can_start());
    let start = one
        .rows()
        .iter()
        .position(|row| *row == Row::Start)
        .unwrap();
    assert!(
        !one.pointer(&click(centre(&skin, start)), &skin)
            .contains(&Event::Start),
        "one of two is not a race"
    );
    let mut both = two(2);
    assert!(both.can_start());
    assert_eq!(
        both.rows(),
        [Row::Seat(0), Row::Seat(1), Row::Start, Row::Back]
    );
    let start = both
        .rows()
        .iter()
        .position(|row| *row == Row::Start)
        .unwrap();
    assert!(
        both.pointer(&click(centre(&skin, start)), &skin)
            .contains(&Event::Start)
    );
    // The generic screen is unchanged: one seat is enough.
    assert!(screen(1).can_start());
    assert!(!screen(0).can_start());
}

/// START and BACK are buttons: START used to draw the head count beside it, which read as a
/// stray number. The seats still say which device holds them.
#[test]
fn a_button_row_shows_no_value() {
    let skin = skin();
    let texts: Vec<(String, Align)> = screen(2)
        .body(&skin, &Frame::default())
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, align, .. } => Some((text, align)),
            _ => None,
        })
        .collect();
    let rights: Vec<&str> = texts
        .iter()
        .filter(|(_, align)| *align == Align::Right)
        .map(|(text, _)| text.as_str())
        .collect();
    assert_eq!(
        rights,
        ["PAD 0", "PAD 1", "OPEN"],
        "no value beside START or BACK"
    );
}

/// LAN's host list is this screen as a list: no START row, a row says what the caller named it
/// rather than "PLAYER n", a confirm on a host answers with its index, and the cursor a pad
/// moves is the highlight the pointer sets.
#[test]
fn a_list_has_no_start_names_its_own_rows_and_answers_a_confirm_with_the_index() {
    let mut list = Join::new(labels(), 8).as_list();
    list.set_seats(vec!["VINETA K".into(), "PHANTOM".into()]);
    list.set_row_labels(vec!["ADA".into(), "BOB".into()]);
    assert_eq!(
        list.rows(),
        [Row::Seat(0), Row::Seat(1), Row::Empty, Row::Back],
        "hosts, the typed address, back; never START"
    );
    let skin = skin();
    let frame = Frame::default();
    let text: Vec<String> = list
        .body(&skin, &frame)
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert!(
        text.contains(&"ADA".to_string()) && text.contains(&"BOB".to_string()),
        "{text:?}"
    );
    assert!(!text.iter().any(|t| t.starts_with("PLAYER")), "{text:?}");
    assert_eq!(list.confirm(), None, "nothing is highlighted yet");
    list.step(1);
    list.step(1);
    assert_eq!(list.confirm(), Some(Event::Leave(1)), "the second host");
    list.step(1);
    assert_eq!(list.confirm(), Some(Event::Join), "the typed address");
    list.step(1);
    assert_eq!(list.confirm(), Some(Event::Back));
    list.step(1);
    assert_eq!(list.confirm(), Some(Event::Leave(0)), "it wraps");
    list.step(-1);
    assert_eq!(list.confirm(), Some(Event::Back), "and wraps the other way");
    // The click on a host is the same answer as a confirm on it.
    let mut pointer = click(centre(&skin, 1));
    pointer.back = false;
    assert_eq!(list.pointer(&pointer, &skin).last(), Some(&Event::Leave(1)));
}

/// A list row with a note is dimmed, cannot be chosen by a confirm or a click, and its note
/// replaces the hint while the cursor is on it.
#[test]
fn a_row_with_a_note_is_dimmed_unselectable_and_says_why() {
    let mut list = Join::new(labels(), 8).as_list();
    list.set_seats(vec!["OK".into(), "NO".into()]);
    list.set_notes(
        vec![Some("TRACK".into()), Some("DIFFERENT VERSION".into())],
        vec![false, true],
    );
    let skin = skin();
    let said = |list: &Join| -> Vec<String> {
        list.body(&skin, &Frame::default())
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    };
    list.step(1);
    assert!(
        said(&list).contains(&"TRACK".to_string()),
        "a joinable row says its own line too"
    );
    assert_eq!(list.confirm(), Some(Event::Leave(0)));
    list.step(1);
    assert_eq!(list.confirm(), None, "the dimmed row answers nothing");
    assert!(
        said(&list).contains(&"DIFFERENT VERSION".to_string()),
        "the note is the hint"
    );
    assert_eq!(
        list.pointer(&click(centre(&skin, 1)), &skin).last(),
        None,
        "nor does a click"
    );
    list.step(-1);
    assert!(said(&list).contains(&"TRACK".to_string()));
    list.step(-1);
    assert!(
        said(&list).contains(&"PRESS A BUTTON TO JOIN".to_string()),
        "on BACK the hint comes back"
    );
}

#[test]
fn a_room_on_the_network_never_asks_the_second_seat_to_press_start() {
    // HD and Omega author "PLAYER 2 PRESS START TO JOIN" for local split screen; a room's
    // empty seat is somebody still to arrive over the network.
    let strings = oag_ui::language::StringTable::from_xml(
        r#"<Strings><Entry ID="FE_GUEST_JOIN" String="Player 2 press start to join"></Entry></Strings>"#,
    );
    assert!(
        Labels::read(&strings).guest.is_some(),
        "the premise: the disc authors one"
    );
    assert_eq!(Labels::read_online_room(&strings).guest, None);
    assert_eq!(Labels::read_lan_lobby(&strings).guest, None);
}

#[test]
fn only_the_room_owner_has_the_settings_and_max_players_rows() {
    let mut join = screen(2);
    assert_eq!(
        join.rows(),
        [
            Row::Seat(0),
            Row::Seat(1),
            Row::Empty,
            Row::Start,
            Row::Back
        ],
        "a guest sees neither"
    );
    join.set_owner(Some(6));
    assert_eq!(
        join.rows(),
        [
            Row::Seat(0),
            Row::Seat(1),
            Row::Empty,
            Row::Settings,
            Row::MaxPlayers,
            Row::Start,
            Row::Back
        ]
    );
    join.set_owner(None);
    assert_eq!(join.rows().len(), 5);
}

#[test]
fn the_owners_rows_answer_the_pad_and_the_pointer_alike() {
    let skin = skin();
    let mut join = screen(2);
    join.set_owner(Some(6));
    // Pad: step down onto each of them (the cursor starts on the last row going down).
    join.step(1);
    assert_eq!(join.hover(), Some(Row::Seat(0)));
    join.step(1);
    join.step(1);
    join.step(1);
    assert_eq!(join.hover(), Some(Row::Settings));
    assert_eq!(join.confirm(), Some(Event::Settings));
    assert_eq!(
        join.adjust(1),
        None,
        "left and right do nothing on ROOM SETTINGS"
    );
    join.step(1);
    assert_eq!(join.hover(), Some(Row::MaxPlayers));
    assert_eq!(join.confirm(), Some(Event::MaxPlayers(1)));
    assert_eq!(join.adjust(-1), Some(Event::MaxPlayers(-1)));
    // Pointer: the same rows by a click, on the rows as this screen draws them.
    let mut join = screen(2);
    join.set_owner(Some(6));
    let layout = join.layout(&skin);
    let row = |index: usize| {
        (
            layout.width / 2.0,
            layout.top + (index as f32 + 0.5) * layout.line,
        )
    };
    assert_eq!(
        join.pointer(&click(row(3)), &skin),
        [Event::Moved, Event::Settings]
    );
    assert_eq!(
        join.pointer(&click(row(4)), &skin),
        [Event::Moved, Event::MaxPlayers(1)]
    );
}

#[test]
fn a_rooms_size_is_how_many_seats_the_grid_has() {
    let mut join = screen(2);
    join.set_capacity(2);
    assert_eq!(
        join.rows(),
        [Row::Seat(0), Row::Seat(1), Row::Start, Row::Back],
        "full: no empty seat"
    );
    join.set_capacity(4);
    assert!(join.rows().contains(&Row::Empty));
}

#[test]
fn the_owner_of_a_full_room_still_sees_and_can_click_start_and_back() {
    // Twelve rows (eight seats, ROOM SETTINGS, MAX PLAYERS, START, BACK) ran off the picture
    // at the usual pitch on both PSP titles.
    for menu in [
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_pure::FRONT_END.menu.unwrap(),
    ] {
        let skin = Skin::new(menu, oag_display::space::Space::PSP, 22.0);
        let mut join = Join::new(labels(), 8);
        join.set_seats((0..8).map(|n| format!("P{n}")).collect());
        join.set_owner(Some(8));
        assert_eq!(join.rows().len(), 12);
        let layout = join.layout(&skin);
        let (_, height) = skin.space().size;
        let last = layout.top + 12.0 * layout.line;
        assert!(last <= height * 0.85, "the rows end at {last} of {height}");
        // The same layout answers the pointer: a click on the drawn BACK row is BACK.
        let at = (layout.width / 2.0, layout.top + 11.5 * layout.line);
        let events = join.pointer(&click(at), &skin);
        assert!(events.contains(&Event::Back), "{events:?}");
        // A screen with few rows keeps the skin's own pitch.
        let few = screen(2);
        assert_eq!(few.layout(&skin).line, Layout::new(&skin).line);
    }
}

#[test]
fn the_laps_row_is_the_owners_and_only_while_the_mode_ends_on_laps() {
    let mut join = screen(2);
    join.set_owner(Some(8));
    assert!(!join.rows().contains(&Row::Laps));
    join.set_laps(Some(0));
    let rows = join.rows();
    let at = rows.iter().position(|r| *r == Row::Laps).expect("laps row");
    assert_eq!(rows[at - 1], Row::MaxPlayers, "right under MAX PLAYERS");
    join.step(1);
    while join.hover() != Some(Row::Laps) {
        join.step(1);
    }
    assert_eq!(join.confirm(), Some(Event::Laps(1)));
    assert_eq!(join.adjust(-1), Some(Event::Laps(-1)));
    join.set_laps(None);
    assert!(!join.rows().contains(&Row::Laps));
    assert_eq!(join.hover(), None, "the cursor was on the row that went");
}
