use super::*;

/// One tick with `buttons` newly down, which is what [`Input::take`] reads -
/// the same fixture `menu/tests.rs` uses, kept local so this file does not
/// reach into that module's private helpers.
fn tick(buttons: &[Button]) -> Input {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    input
}

/// Pulse's own menu table against the 22-pixel face its menus name, the same
/// skin `menu/tests.rs` draws with.
fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu,
        crate::frontend::Space::PSP,
        22.0,
    )
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
        crate::pilots::MAX_NAME,
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
    crate::pilots::check_name(keyboard.text()).expect("the grid cannot type an invalid name");
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
        crate::pilots::check_name(&c.to_string())
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

/// Every cell has to be reachable, or the grid has keys in it that are
/// decoration. Walked with the d-pad, so a wrapping bug is what this catches.
#[test]
fn every_cell_is_reachable_with_the_d_pad() {
    let mut seen = vec![false; CELLS];
    let mut keyboard = keyboard("");
    for _ in 0..GRID_ROWS {
        for _ in 0..COLUMNS {
            seen[keyboard.cell] = true;
            keyboard.update(&mut tick(&[Button::Right]));
        }
        keyboard.update(&mut tick(&[Button::Down]));
    }
    let missed: Vec<usize> = (0..CELLS).filter(|i| !seen[*i]).collect();
    assert!(missed.is_empty(), "unreachable cells: {missed:?}");
}

/// Up wraps to the bottom row and down wraps to the top, so a player at
/// either edge is never stuck against it.
#[test]
fn the_cursor_wraps_at_every_edge() {
    let mut keyboard = keyboard("");
    keyboard.update(&mut tick(&[Button::Up]));
    assert_eq!(keyboard.cell, (GRID_ROWS - 1) * COLUMNS);
    keyboard.update(&mut tick(&[Button::Down]));
    assert_eq!(keyboard.cell, 0);
    keyboard.update(&mut tick(&[Button::Left]));
    assert_eq!(keyboard.cell, COLUMNS - 1);
    keyboard.update(&mut tick(&[Button::Right]));
    assert_eq!(keyboard.cell, 0);
}

/// Square is the pad shortcut for the delete key; both do the same thing.
#[test]
fn square_deletes_without_travelling_to_the_delete_key() {
    let mut keyboard = keyboard("winston");
    keyboard.update(&mut tick(&[Button::Square]));
    assert_eq!(keyboard.text(), "winsto");
    assert_eq!(type_key(&mut keyboard, Key::Delete), Outcome::Pending);
    assert_eq!(keyboard.text(), "winst");
}

/// Deleting an empty buffer is a no-op, not a panic - a player holding the
/// delete key does exactly this.
#[test]
fn deleting_an_empty_buffer_does_nothing() {
    let mut keyboard = keyboard("");
    keyboard.update(&mut tick(&[Button::Square]));
    assert_eq!(keyboard.text(), "");
}

/// The limit is the writer's, and the keyboard cannot build a name over it.
#[test]
fn the_buffer_stops_at_its_limit_rather_than_growing_past_it() {
    let mut keyboard = Keyboard::new(Labels::default(), "", 3);
    for _ in 0..6 {
        keyboard.edit(Edit::Type('a'));
    }
    assert_eq!(keyboard.text(), "aaa");
    // Seeding over the limit truncates rather than opening on a buffer that
    // could only ever shrink.
    assert_eq!(Keyboard::new(Labels::default(), "abcdef", 3).text(), "abc");
}

/// Circle backs out and Start accepts wherever the cursor is - the two
/// shortcuts a player reaches for without looking.
#[test]
fn circle_cancels_and_start_accepts_from_anywhere() {
    let mut keyboard = keyboard("winston");
    assert_eq!(keyboard.update(&mut tick(&[Button::Circle])), Outcome::Cancelled);
    assert_eq!(keyboard.update(&mut tick(&[Button::Start])), Outcome::Accepted);
    // Neither touches the buffer: the caller decides what to do with it, and
    // there is nothing to reset because the caller drops this.
    assert_eq!(keyboard.text(), "winston");
}

/// The accept key on the grid does what Start does, so a player who never
/// finds the shortcut still finishes.
#[test]
fn the_grids_own_accept_key_accepts() {
    let mut keyboard = keyboard("gerald");
    assert_eq!(type_key(&mut keyboard, Key::Accept), Outcome::Accepted);
}

/// **The bug an edge-consuming update exists to prevent.** A prompt that
/// peeked would see the same Cross that opened it, accept on the tick it
/// appeared, and reopen forever. `Input::take` consumes, so a second reader
/// on the same tick sees nothing.
#[test]
fn cross_is_consumed_so_nothing_downstream_sees_the_same_press() {
    let mut input = tick(&[Button::Cross]);
    let mut keyboard = keyboard("");
    keyboard.update(&mut input);
    assert!(
        !input.take(Button::Cross),
        "the menus would have re-activated the row that opened this"
    );
}

/// A note is what the caller says about the text; the model only draws it.
#[test]
fn a_note_is_carried_and_cleared_by_the_caller() {
    let mut keyboard = keyboard("aggressive");
    assert!(keyboard.note.is_none());
    keyboard.set_note(Some("replaces the built-in".into()));
    assert!(
        keyboard
            .draw(&skin())
            .iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text.contains("built-in"))),
        "a note that is set has to reach the screen"
    );
    keyboard.set_note(None);
    assert!(keyboard.note.is_none());
}

/// The buffer is on screen with a caret, so an empty name is visibly empty
/// rather than indistinguishable from a prompt that is not listening.
#[test]
fn the_buffer_is_drawn_with_a_caret() {
    let list = keyboard("winston").draw(&skin());
    assert!(
        list.iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "winston_")),
        "no buffer in the draw list"
    );
    let empty = keyboard("").draw(&skin());
    assert!(
        empty
            .iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "_"))
    );
}

fn confirm() -> Confirm {
    Confirm::new(ConfirmLabels {
        title: "DELETE PILOT".into(),
        message: "Delete winston?".into(),
        yes: "DELETE".into(),
        no: "KEEP".into(),
    })
}

/// A destructive dialog that opens on its destructive answer turns a stray
/// Cross into a deleted file.
#[test]
fn a_confirm_opens_on_no_and_a_bare_cross_cancels() {
    let mut confirm = confirm();
    assert!(!confirm.on_yes());
    assert_eq!(confirm.update(&mut tick(&[Button::Cross])), Outcome::Cancelled);
}

/// Moving to yes and confirming is the only way through.
#[test]
fn a_confirm_accepts_only_after_the_answer_is_moved_to_yes() {
    let mut confirm = confirm();
    assert_eq!(confirm.update(&mut tick(&[Button::Right])), Outcome::Pending);
    assert!(confirm.on_yes());
    assert_eq!(confirm.update(&mut tick(&[Button::Cross])), Outcome::Accepted);
}

/// Down works as well as right: two answers side by side still answer to a
/// thumb that went the other way.
#[test]
fn both_axes_move_a_confirms_answer() {
    for button in [Button::Left, Button::Right, Button::Up, Button::Down] {
        let mut confirm = confirm();
        confirm.update(&mut tick(&[button]));
        assert!(confirm.on_yes(), "{button:?} did not move the answer");
    }
}

/// Circle backs out of a confirm whichever answer is under the cursor.
#[test]
fn circle_cancels_a_confirm_even_on_yes() {
    let mut confirm = confirm();
    confirm.update(&mut tick(&[Button::Right]));
    assert!(confirm.on_yes());
    assert_eq!(confirm.update(&mut tick(&[Button::Circle])), Outcome::Cancelled);
}

/// A confirm's message reaches the screen: the built-in-restore sentence is
/// the whole reason delete asks at all, and a dialog that dropped it would be
/// worse than no dialog.
#[test]
fn a_confirms_message_and_both_answers_are_drawn() {
    let list = confirm().draw(&skin());
    for wanted in ["DELETE PILOT", "Delete winston?", "DELETE", "KEEP"] {
        assert!(
            list.iter()
                .any(|draw| matches!(draw, Draw::Text { text, .. } if text == wanted)),
            "{wanted:?} is not in the draw list"
        );
    }
}

/// Both prompts cover what is behind them: an overlay a player can still read
/// the live menu through is one they will try to use the live menu from.
#[test]
fn both_prompts_scrim_everything_behind_them() {
    for list in [keyboard("winston").draw(&skin()), confirm().draw(&skin())] {
        let Some(Draw::Fill { rect, color }) = list.first() else {
            panic!("the first draw should be the scrim");
        };
        assert!(color[3] > 0.5, "the scrim is too faint to hide a menu");
        assert!(
            rect[0] <= 0.0 && rect[1] <= 0.0 && rect[2] >= 480.0 && rect[3] >= 272.0,
            "the scrim {rect:?} does not cover the screen"
        );
    }
}
