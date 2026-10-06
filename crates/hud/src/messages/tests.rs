use super::*;

/// A table that carries the one phrase the tests raise.
fn table() -> oag_ui::language::StringTable {
    let mut strings = oag_ui::language::StringTable::default();
    strings.merge([("ER_GMA".to_string(), "Gold medal awarded".to_string())].into());
    strings
}

fn run(board: &mut MessageBoard, ticks: u32) {
    for _ in 0..ticks {
        board.advance();
    }
}

fn alpha(board: &MessageBoard, slot: usize) -> Option<f32> {
    board.lines()[slot].as_ref().map(|line| line.color[3])
}

/// The first message shows on the tick after it is raised, fades in to full
/// inside the first half second, and is gone after four seconds.
#[test]
fn a_message_fades_in_holds_then_fades_out_over_four_seconds() {
    let mut board = MessageBoard::default();
    board.push("Gold medal awarded", true);
    assert_eq!(alpha(&board, 0), None, "nothing before the first tick");
    board.advance();
    assert!(board.just_shown());
    assert_eq!(alpha(&board, 0), Some(0.0), "shown, but at alpha zero");

    run(&mut board, 27); // 0.45 s: the peak is at 0.44 s
    let early = alpha(&board, 0).expect("still up");
    assert!(early > 0.99, "full by the half second, got {early}");
    assert!(!board.just_shown());

    run(&mut board, 120); // 2.5 s in
    let mid = alpha(&board, 0).expect("still up");
    assert!(mid < early && mid > 0.0, "a slow fade, got {mid}");

    run(&mut board, 130); // past 4 s
    assert_eq!(alpha(&board, 0), None, "freed after its four seconds");
}

#[test]
fn a_good_message_is_green_and_a_bad_one_red() {
    let mut board = MessageBoard::default();
    board.push("good", true);
    board.advance();
    run(&mut board, 30);
    let green = board.lines()[0].clone().expect("up").color;
    assert!(
        green[1] > 0.9 && green[0] < 0.25 && green[2] < 0.25,
        "{green:?}"
    );

    let mut board = MessageBoard::default();
    board.push("bad", false);
    board.advance();
    run(&mut board, 30);
    let red = board.lines()[0].clone().expect("up").color;
    assert!(red[0] > 0.9 && red[1] < 0.25, "{red:?}");
}

/// Two raised together show 0.8 s apart, in the order they were raised, on
/// consecutive slots.
#[test]
fn messages_raised_together_show_eight_tenths_of_a_second_apart() {
    let mut board = MessageBoard::default();
    board.push("first", true);
    board.push("second", true);
    board.advance();
    assert!(board.lines()[0].is_some() && board.lines()[1].is_none());
    run(&mut board, 47); // 0.8 s is 48 ticks, counted from the first
    assert!(board.lines()[1].is_none(), "not yet");
    run(&mut board, 3);
    assert!(board.lines()[1].is_some(), "second is up");
    assert!(board.just_shown() || alpha(&board, 1).is_some());
}

#[test]
fn a_fifth_message_is_dropped_when_all_four_slots_are_busy() {
    let mut board = MessageBoard::default();
    for n in 0..5 {
        board.push(format!("m{n}"), true);
    }
    run(&mut board, 400);
    let shown: Vec<_> = board
        .lines()
        .iter()
        .flatten()
        .map(|line| line.text.clone())
        .collect();
    assert!(!shown.contains(&"m4".to_string()), "{shown:?}");
}

/// The slots reach the screen: `Info1` carries the line's text in its green,
/// at the board's own alpha, and a layout slot with no line draws nothing.
/// Dropping the arm in `draw::text_for` or the colour override fails this.
#[test]
fn a_raised_message_is_drawn_in_its_info_slot() {
    use crate::tests::context;
    use crate::{Draw, Layout, Readout};

    let layout = Layout::from_xml(
        r#"<Screen><Screen name="HUD">
<Text name="Info1"><Values font="HUD" scale="0.6" align="centre" vertalign="middle" x="240" y="80"/></Text>
<Text name="Info2"><Values font="HUD" scale="0.6" align="centre" vertalign="middle" x="240" y="95"/></Text>
</Screen></Screen>"#,
    );
    let strings = table();
    let mut board = MessageBoard::default();
    board.push("ER_GMA", true);
    board.advance();
    run(&mut board, 27);
    let readout = Readout {
        messages: board.lines(),
        ..Readout::blank()
    };
    let frame = crate::draw_list(&context(&layout, &strings), &readout);
    let drawn: Vec<_> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, color, .. } => Some((text.clone(), *color)),
            _ => None,
        })
        .collect();
    assert_eq!(drawn.len(), 1, "only Info1 has a line: {frame:?}");
    assert_eq!(drawn[0].0, "Gold medal awarded");
    assert!(
        drawn[0].1[1] > 0.9 && drawn[0].1[3] > 0.99,
        "{:?}",
        drawn[0].1
    );
}

/// A title that has not had the slots measured draws no line, whatever a host
/// pushed: HD's own `Info1` is a different widget.
#[test]
fn a_title_without_message_slots_draws_none() {
    use crate::tests::context;
    use crate::{Layout, Readout};

    let layout = Layout::from_xml(
        r#"<Screen><Screen name="HUD">
<Text name="Info1"><Values font="HUD" scale="0.6" x="240" y="80"/></Text>
</Screen></Screen>"#,
    );
    let strings = table();
    let mut board = MessageBoard::default();
    board.push("ER_GMA", true);
    board.advance();
    run(&mut board, 27);
    let readout = Readout {
        messages: board.lines(),
        ..Readout::blank()
    };
    let art = oag_title::HudArt {
        message_slots: false,
        ..*oag_pulse::hud::ART
    };
    let mut cx = context(&layout, &strings);
    cx.art = &art;
    assert!(crate::draw_list(&cx, &readout).hud_text.is_empty());
}

/// Pure's table has no `ER_GMA`: the line draws nothing, not the raw id.
#[test]
fn an_id_the_table_lacks_draws_nothing() {
    use crate::tests::{context, strings};
    use crate::{Layout, Readout};

    let layout = Layout::from_xml(
        r#"<Screen><Screen name="HUD">
<Text name="Info1"><Values font="HUD" scale="0.6" x="240" y="80"/></Text>
</Screen></Screen>"#,
    );
    let strings = strings();
    let mut board = MessageBoard::default();
    board.push("ER_GMA", true);
    board.advance();
    run(&mut board, 27);
    let readout = Readout {
        messages: board.lines(),
        ..Readout::blank()
    };
    assert!(
        crate::draw_list(&context(&layout, &strings), &readout)
            .hud_text
            .is_empty()
    );
}

/// The standing line is up from the first call, at full alpha, in the last slot,
/// and survives long after a banner has expired.
#[test]
fn a_standing_line_stays_in_the_last_slot_for_good() {
    let mut board = MessageBoard::default();
    assert_eq!(board.lines()[SLOTS - 1], None);
    board.set_standing("ER_BMA", [0.8, 0.5, 0.2]);
    run(&mut board, 1);
    let line = board.lines()[SLOTS - 1].clone().expect("standing is up");
    assert_eq!(line.text, "ER_BMA");
    assert_eq!(line.color, [0.8, 0.5, 0.2, 1.0]);
    run(&mut board, 3000);
    assert_eq!(board.lines()[SLOTS - 1], Some(line));
    board.set_standing("ER_SMA", [0.9, 0.9, 0.9]);
    assert_eq!(board.lines()[SLOTS - 1].clone().unwrap().text, "ER_SMA");
}

/// Banners queue in the first three slots while a standing line holds the
/// last, and the same words are not drawn twice at once.
#[test]
fn banners_leave_the_last_slot_to_the_standing_line() {
    let mut board = MessageBoard::default();
    board.set_standing("ER_SMA", [1.0; 3]);
    for text in ["a", "b", "c", "d"] {
        board.push(text, true);
    }
    run(&mut board, 1);
    let lines = board.lines();
    assert_eq!(lines[3].as_ref().map(|l| l.text.as_str()), Some("ER_SMA"));
    assert_eq!(lines[0].as_ref().map(|l| l.text.as_str()), Some("a"));
    let mut board = MessageBoard::default();
    board.set_standing("ER_SMA", [1.0; 3]);
    board.push("ER_SMA", true);
    run(&mut board, 30);
    let said = board
        .lines()
        .iter()
        .flatten()
        .filter(|l| l.text == "ER_SMA")
        .count();
    assert_eq!(
        said, 1,
        "the banner and the standing line are the same words"
    );
}
