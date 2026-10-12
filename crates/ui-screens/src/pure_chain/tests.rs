use super::*;

fn tick(buttons: &[Button]) -> Input {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    input
}

fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn rows(events: usize) -> Vec<Row> {
    let mut rows = Vec::new();
    for tag in 0..events {
        rows.push(Row::event(
            format!("EVENT {tag}"),
            tag,
            Status::Open,
            String::new(),
        ));
    }
    rows
}

fn list(events: usize) -> Choice {
    Choice::new("CLASS".into(), "VENOM".into(), rows(events))
}

#[test]
fn the_cursor_starts_on_the_first_row_and_wraps_at_both_ends() {
    let mut list = list(4);
    assert_eq!(list.selected_row().map(|r| r.tag), Some(0));
    list.update(&mut tick(&[Button::Down]));
    assert_eq!(list.selected_row().map(|r| r.tag), Some(1));
    list.update(&mut tick(&[Button::Down]));
    assert_eq!(list.selected_row().map(|r| r.tag), Some(2));
    list.update(&mut tick(&[Button::Up]));
    list.update(&mut tick(&[Button::Up]));
    list.update(&mut tick(&[Button::Up]));
    assert_eq!(
        list.selected_row().map(|r| r.tag),
        Some(3),
        "up from the first wraps to the last"
    );
}

#[test]
fn confirm_hands_back_the_rows_tag_and_a_locked_row_confirms_too() {
    let mut list = list(2);
    let events = list.update(&mut tick(&[Button::Cross]));
    assert_eq!(events, [Event::Confirmed(0)]);
    list.replace(vec![Row::event("A", 7, Status::Locked, "LOCKED".into())]);
    assert_eq!(
        list.update(&mut tick(&[Button::Start])),
        [Event::Confirmed(7)]
    );
}

#[test]
fn circle_backs_out_and_left_and_right_do_nothing() {
    let mut list = list(1);
    assert!(list.update(&mut tick(&[Button::Right])).is_empty());
    assert!(list.update(&mut tick(&[Button::Left])).is_empty());
    assert_eq!(list.update(&mut tick(&[Button::Circle])), [Event::Back]);
}

#[test]
fn replacing_the_rows_keeps_the_cursor_on_the_same_event() {
    let mut list = list(4);
    list.update(&mut tick(&[Button::Down]));
    list.update(&mut tick(&[Button::Down]));
    assert_eq!(list.selected_row().unwrap().tag, 2);
    list.replace(rows(4));
    assert_eq!(list.context(), "VENOM");
    assert_eq!(list.selected_row().unwrap().tag, 2);
}

#[test]
fn a_click_on_a_row_selects_and_confirms_it_and_a_click_past_the_rows_does_nothing() {
    let skin = skin();
    let mut list = list(3);
    let layout = Layout::new(&skin);
    let row = layout.row_rect(1);
    let click = |x: f32, y: f32| Pointer {
        at: Some((x, y)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    };
    let events = list.pointer(&click(row[0] + 20.0, row[1] + row[3] * 0.5), &skin);
    assert_eq!(events, [Event::Moved, Event::Confirmed(1)]);
    let past = layout.row_rect(5);
    assert!(
        list.pointer(&click(past[0] + 20.0, past[1] + 3.0), &skin)
            .is_empty(),
        "below the last row is not a target"
    );
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(list.pointer(&back, &skin), [Event::Back]);
}

#[test]
fn the_wheel_moves_the_cursor_and_a_long_list_scrolls_to_keep_it_in_view() {
    let skin = skin();
    let mut list = list(40);
    let wheel = Pointer {
        scroll: 1,
        ..Pointer::default()
    };
    for _ in 0..30 {
        list.pointer(&wheel, &skin);
        list.follow(&skin);
    }
    let drawn: Vec<String> = list
        .draw(&skin)
        .into_iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    let tag = list.selected_row().unwrap().tag;
    assert!(tag >= 20, "the cursor travelled: {tag}");
    assert!(
        drawn.contains(&format!("EVENT {tag}")),
        "and the window followed it"
    );
    assert!(
        !drawn.contains(&"EVENT 0".to_string()),
        "the top has scrolled off"
    );
}

#[test]
fn a_locked_selected_row_says_what_opens_it_and_a_medal_is_inked() {
    let skin = skin();
    let mut locked = Row::event("KARBONIS", 0, Status::Locked, "LOCKED".into());
    locked.requires = "NEEDS 10 MEDALS".into();
    let gold = Row::event("VINETA K", 1, Status::Medal(Medal::Gold), "GOLD".into());
    let list = Choice::new(String::new(), "VENOM".into(), vec![locked, gold]);
    let draws = list.draw(&skin);
    assert!(
        draws
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text == "NEEDS 10 MEDALS"))
    );
    assert!(
        draws.iter().any(
            |d| matches!(d, Draw::Text { text, color, .. } if text == "GOLD" && *color == GOLD)
        )
    );
}
