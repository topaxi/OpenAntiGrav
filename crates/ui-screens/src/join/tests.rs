use super::*;

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
        keyboard: "KEYBOARD".into(),
        pad: "GAMEPAD".into(),
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
        layout.rows_y + (index as f32 + 0.5) * layout.line,
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
        .body(&skin())
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
