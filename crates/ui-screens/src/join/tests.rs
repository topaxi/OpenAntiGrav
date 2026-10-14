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
