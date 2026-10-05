//! What `oag_ui_screens::prompt`'s on-screen keyboard grid offers, checked against
//! `oag-game`'s own `pilots::check_name`/`MAX_NAME` - the one assertion in
//! this cluster that needs to see both crates at once, so it lives here
//! rather than with the rest of the grid's tests in `oag-ui`'s own
//! `prompt/tests.rs`.

use oag_gameplay::input::{Button, Input};
use oag_ui_screens::prompt::{CELLS, COLUMNS, Key, Keyboard, Labels, Outcome, key_at};

fn tick(buttons: &[Button]) -> Input {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    input
}

fn keyboard(initial: &str) -> Keyboard {
    Keyboard::new(
        Labels {
            title: "RENAME PILOT".into(),
            delete: "DEL".into(),
            accept: "OK".into(),
            hint: String::new(),
        },
        initial,
        oag_raceplay::pilots::MAX_NAME,
    )
}

/// Walks the cursor to `key` with the d-pad and presses Cross on it.
///
/// Deliberately by *navigation* rather than by setting the field: a cell the
/// d-pad cannot reach is a key a pad player does not have, which is the whole
/// question this module exists to answer. Panics rather than looping forever.
fn type_key(keyboard: &mut Keyboard, key: Key) -> Outcome {
    for step in 0..CELLS * 2 {
        if keyboard.selected() == key {
            return keyboard.update(&mut tick(&[Button::Cross]));
        }
        // Right wraps within the row, so a Down every full row is what walks
        // the whole grid.
        keyboard.update(&mut tick(&[Button::Right]));
        if step % COLUMNS == COLUMNS - 1 {
            keyboard.update(&mut tick(&[Button::Down]));
        }
    }
    panic!("{key:?} is not reachable with the d-pad");
}

/// The whole point of the module: a name typed with nothing but a d-pad and
/// one button, which is what a pad has.
#[test]
fn a_name_can_be_typed_with_the_d_pad_and_cross_alone() {
    let mut keyboard = keyboard("");
    for c in "ax9-".chars() {
        assert_eq!(
            type_key(&mut keyboard, Key::Char(c)),
            Outcome::Pending,
            "typing must not end the prompt"
        );
    }
    assert_eq!(keyboard.text(), "ax9-");
    // And it is a name the writer will accept, which is the reason the grid
    // offers exactly this character set.
    oag_raceplay::pilots::check_name(keyboard.text())
        .expect("the grid cannot type an invalid name");
}

/// Every character the writer allows has to be on the grid, and nothing else:
/// a grid offering a slash builds a name that is refused only at the end, and
/// a grid missing `_` shows a player a file they cannot type back in.
#[test]
fn the_grid_offers_exactly_the_characters_a_pilot_name_may_hold() {
    let offered: String = (0..CELLS)
        .filter_map(|index| match key_at(index) {
            Key::Char(c) => Some(c),
            _ => None,
        })
        .collect();
    for c in offered.chars() {
        oag_raceplay::pilots::check_name(&c.to_string())
            .unwrap_or_else(|e| panic!("the grid offers {c:?}, which a name may not hold: {e:#}"));
    }
    for c in "abcdefghijklmnopqrstuvwxyz0123456789-_".chars() {
        assert!(offered.contains(c), "the grid cannot type {c:?}");
    }
    let specials = (0..CELLS)
        .filter(|i| !matches!(key_at(*i), Key::Char(_)))
        .count();
    assert_eq!(specials, 2, "delete and accept, exactly once each");
}
