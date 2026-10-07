use super::*;
use oag_gameplay::input::Input;

fn entry() -> KeyEntry {
    KeyEntry::new(Path::new("data/images/hdfury-ps3-eu.iso"), true)
}

const KEY: &str = "00112233445566778899aabbccddeeff";

#[test]
fn typing_keeps_hex_folds_case_and_stops_at_thirty_two() {
    let mut e = entry();
    for c in "zZ-0Aq_f".chars() {
        e.type_digit(c);
    }
    assert_eq!(e.digits, "0af");
    for _ in 0..50 {
        e.type_digit('1');
    }
    assert_eq!(e.len(), DIGITS);
    e.delete();
    assert_eq!(e.len(), DIGITS - 1);
}

#[test]
fn a_whole_pasted_key_replaces_the_buffer_in_any_common_form() {
    for form in [
        KEY,
        "0x00112233445566778899AABBCCDDEEFF",
        "00112233 44556677 8899aabb ccddeeff\n",
    ] {
        let mut e = entry();
        e.type_digit('f');
        e.paste(form);
        assert_eq!(e.digits, KEY, "{form}");
    }
}

#[test]
fn a_partial_paste_appends_and_prose_is_refused_whole() {
    let mut e = entry();
    e.paste("ab cd-ef");
    assert_eq!(e.digits, "abcdef");
    e.paste("key: 12 (the disc key)");
    assert_eq!(e.digits, "abcdef", "prose must not be mined for digits");
    assert!(e.message().unwrap().contains("NO KEY"));
}

#[test]
fn a_short_buffer_is_refused_with_the_count_and_never_checked_or_stored() {
    let mut e = entry();
    e.paste("abcd");
    let out = e.submit_with(|_| panic!("must not check"), |_| panic!("must not store"));
    assert_eq!(out, Outcome::Pending);
    assert!(e.message().unwrap().contains("YOU HAVE 4"));
}

#[test]
fn a_key_that_does_not_open_the_disc_is_reported_and_not_stored() {
    let mut e = entry();
    e.paste(KEY);
    let out = e.submit_with(|_| false, |_| panic!("must not store"));
    assert_eq!(out, Outcome::Pending);
    assert!(e.message().unwrap().contains("DOES NOT OPEN"));
}

#[test]
fn a_key_that_opens_the_disc_is_stored_once_and_the_prompt_closes() {
    let mut e = entry();
    e.paste(KEY);
    let mut stored = 0;
    let out = e.submit_with(
        |_| true,
        |_| {
            stored += 1;
            Ok(())
        },
    );
    assert_eq!((out, stored), (Outcome::Stored, 1));
}

#[test]
fn nothing_the_prompt_draws_contains_the_key() {
    let mut e = entry();
    e.paste(KEY);
    let _ = e.submit_with(|_| false, |_| Ok(()));
    // The buffer is on screen while it is typed - that is the point of a prompt
    // - but the message and hint lines never echo it.
    for d in e.draw() {
        if let Draw::Text { text, .. } = d {
            if text.starts_with("00112233") {
                continue;
            }
            assert!(!text.contains("8899aabb"), "{text}");
        }
    }
}

#[test]
fn every_cell_is_reachable_by_pointer_and_a_click_presses_it() {
    let mut e = entry();
    for i in 0..e.cells().len() {
        let [x, y, w, h] = e.rect(i);
        assert_eq!(e.cell_at((x + w / 2.0, y + h / 2.0)), Some(i), "cell {i}");
    }
    assert_eq!(e.cell_at((0.0, 0.0)), None);
    // Click the cell for 'b'.
    let [x, y, w, h] = e.rect(11);
    let click = oag_ui::pointer::Pointer {
        at: Some((x + w / 2.0, y + h / 2.0)),
        moved: true,
        clicked: true,
        ..Default::default()
    };
    assert_eq!(e.pointer(&click), Outcome::Pending);
    assert_eq!(e.digits, "b");
    // The secondary button backs out, as circle does.
    let back = oag_ui::pointer::Pointer {
        back: true,
        at: Some((1.0, 1.0)),
        ..Default::default()
    };
    assert_eq!(e.pointer(&back), Outcome::Cancelled);
}

#[test]
fn without_a_clipboard_there_is_no_paste_cell() {
    let e = KeyEntry::new(Path::new("x.iso"), false);
    assert!(!e.cells().contains(&Cell::Paste));
    assert!(
        e.draw()
            .iter()
            .all(|d| !matches!(d, Draw::Text { text, .. } if text == "PASTE"))
    );
    assert!(entry().cells().contains(&Cell::Paste));
}

#[test]
fn the_pad_walks_the_grid_types_and_cancels() {
    let mut e = entry();
    let mut input = Input::default();
    input.inject_press(Button::Right);
    assert_eq!(e.update(&mut input), Outcome::Pending);
    input.inject_press(Button::Cross);
    assert_eq!(e.update(&mut input), Outcome::Pending);
    assert_eq!(e.digits, "1");
    input.inject_press(Button::Down);
    e.update(&mut input);
    assert_eq!(e.selected(), Cell::Hex('9'), "second row, same column");
    input.inject_press(Button::Down);
    e.update(&mut input);
    // The action row has fewer cells; the column lands on one of them.
    assert_eq!(e.selected(), Cell::Clear);
    input.inject_press(Button::Circle);
    assert_eq!(e.update(&mut input), Outcome::Cancelled);
}

#[test]
fn every_line_fits_the_screen() {
    let mut e = entry();
    e.paste(KEY);
    let _ = e.submit_with(|_| false, |_| Ok(()));
    for d in e.draw() {
        if let Draw::Text { x, scale, text, .. } = d {
            // 6 units a glyph in the 5x7 face at scale 1.
            assert!(x + text.len() as f32 * 6.0 * scale <= SCREEN.0, "{text}");
        }
    }
}
