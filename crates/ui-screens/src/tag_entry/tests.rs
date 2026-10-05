use super::*;
use oag_gameplay::input::Button;
use oag_ui::screen::TagInput;

/// A hand-built [`Geometry`], the shape `docs/formats/fexml.md`'s `TagInput`
/// section records for `Name` - three cells rather than ten, enough to
/// exercise the cursor's own range without a long fixture. The real numbers
/// are what `crates/ui-screens/tests/tag_entry_ground_truth.rs` checks against the
/// disc; this module checks the *model*, which does not care where a
/// [`Geometry`] came from.
fn geometry(length: u32) -> Geometry {
    Geometry {
        tag_input: TagInput {
            name: "Name".to_string(),
            x: 50.0,
            y: 85.0,
            length,
            color: 0xFF33_A6B9,
            scale: 2.0,
            encrypt: false,
            allow_blank: None,
            focus: Some("true".to_string()),
        },
        cells: (0..length)
            .map(|i| Fill {
                name: Some("BGgradient".to_string()),
                x: 46.0 + 30.0 * i as f32,
                y: 88.0,
                width: Some(28.0),
                height: Some(25.0),
                color: 0x2fff_ffff,
                gradient: None,
                reveal: Vec::new(),
                transition: 0.0,
            })
            .collect(),
        title: None,
        confirm: Some(Text {
            name: Some("confirm".to_string()),
            idstring: Some("FE_CONFIRM".to_string()),
            string: None,
            font: "Title".to_string(),
            x: 400.0,
            y: 90.0,
            scale: 1.0,
            color: 0xFF33_A6B9,
            align: "centre".to_string(),
            middle: false,
            start_enabled: true,
            pulse: false,
            delay: 0.0,
            wrap_width: None,
            transition: 0.0,
        }),
        bars: Vec::new(),
    }
}

const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz0123456789-";

fn labels() -> Labels {
    Labels {
        title: "RENAME PILOT".to_string(),
        confirm: "OK".to_string(),
        hint: String::new(),
    }
}

fn skin() -> oag_ui::menu::Skin {
    oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn tick(buttons: &[Button]) -> oag_gameplay::input::Input {
    let mut input = oag_gameplay::input::Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    input
}

#[test]
fn a_fresh_entry_shows_exactly_the_initial_text() {
    let entry = TagEntry::new(labels(), geometry(3), ALPHABET, "ab");
    assert_eq!(entry.text(), "ab");
    assert_eq!(entry.selected_cell(), Some(0));
}

#[test]
fn a_character_outside_the_alphabet_becomes_a_blank_cell_not_a_panic() {
    let entry = TagEntry::new(labels(), geometry(3), ALPHABET, "a_c");
    // `_` is not in `ALPHABET` here (mirroring Pulse's own real alphabet,
    // which also has none) - dropped silently rather than shown wrong.
    assert_eq!(entry.text(), "ac");
}

#[test]
fn cycling_up_from_blank_lands_on_the_alphabets_first_glyph() {
    let mut entry = TagEntry::new(labels(), geometry(3), ALPHABET, "");
    entry.cycle(true);
    assert_eq!(entry.text(), "a");
}

#[test]
fn cycling_down_from_blank_wraps_to_the_alphabets_last_glyph() {
    let mut entry = TagEntry::new(labels(), geometry(3), ALPHABET, "");
    entry.cycle(false);
    assert_eq!(entry.text(), "-");
}

#[test]
fn cycling_past_the_last_glyph_wraps_back_to_blank() {
    let mut entry = TagEntry::new(labels(), geometry(3), ALPHABET, "-");
    entry.cycle(true);
    assert_eq!(entry.text(), "");
}

#[test]
fn right_past_the_last_cell_lands_on_the_confirm_slot() {
    let mut entry = TagEntry::new(labels(), geometry(2), ALPHABET, "ab");
    entry.update(&mut tick(&[Button::Right]));
    entry.update(&mut tick(&[Button::Right]));
    assert_eq!(entry.selected_cell(), None);
    // A third press does not run off the end of the strip.
    entry.update(&mut tick(&[Button::Right]));
    assert_eq!(entry.selected_cell(), None);
}

#[test]
fn cross_on_the_confirm_slot_accepts_but_not_on_a_cell() {
    let mut entry = TagEntry::new(labels(), geometry(2), ALPHABET, "ab");
    assert_eq!(
        entry.update(&mut tick(&[Button::Cross])),
        Outcome::Pending,
        "Cross on a cell types nothing here - there is no key to press"
    );
    entry.update(&mut tick(&[Button::Right]));
    entry.update(&mut tick(&[Button::Right]));
    assert_eq!(entry.update(&mut tick(&[Button::Cross])), Outcome::Accepted);
}

#[test]
fn start_accepts_from_anywhere() {
    let mut entry = TagEntry::new(labels(), geometry(2), ALPHABET, "ab");
    assert_eq!(entry.update(&mut tick(&[Button::Start])), Outcome::Accepted);
}

#[test]
fn circle_cancels() {
    let mut entry = TagEntry::new(labels(), geometry(2), ALPHABET, "ab");
    assert_eq!(
        entry.update(&mut tick(&[Button::Circle])),
        Outcome::Cancelled
    );
}

#[test]
fn a_desk_keyboard_type_sets_the_cell_and_advances() {
    let mut entry = TagEntry::new(labels(), geometry(3), ALPHABET, "");
    entry.edit(crate::prompt::Edit::Type('z'));
    assert_eq!(entry.text(), "z");
    assert_eq!(entry.selected_cell(), Some(1));
}

#[test]
fn a_desk_keyboard_character_outside_the_alphabet_is_ignored() {
    let mut entry = TagEntry::new(labels(), geometry(3), ALPHABET, "");
    entry.edit(crate::prompt::Edit::Type('_'));
    assert_eq!(entry.text(), "");
    assert_eq!(entry.selected_cell(), Some(0), "the cursor did not advance");
}

#[test]
fn a_desk_keyboard_delete_steps_back_from_the_confirm_slot_and_clears() {
    let mut entry = TagEntry::new(labels(), geometry(2), ALPHABET, "ab");
    entry.update(&mut tick(&[Button::Right]));
    entry.update(&mut tick(&[Button::Right]));
    assert_eq!(entry.selected_cell(), None);
    entry.edit(crate::prompt::Edit::Delete);
    assert_eq!(entry.selected_cell(), Some(1));
    assert_eq!(entry.text(), "a");
}

#[test]
fn draw_puts_a_glyph_over_every_filled_cell() {
    let entry = TagEntry::new(labels(), geometry(3), ALPHABET, "ab");
    let draws = entry.draw(&skin());
    let glyphs = draws
        .iter()
        .filter(|d| matches!(d, Draw::Text { text, .. } if text == "a" || text == "b"))
        .count();
    assert_eq!(glyphs, 2, "exactly the two filled cells drew a glyph");
}

#[test]
fn pointer_click_on_a_cell_selects_it_without_changing_its_glyph() {
    let mut entry = TagEntry::new(labels(), geometry(3), ALPHABET, "ab");
    let pointer = oag_ui::pointer::Pointer {
        at: Some((60.0, 90.0)), // inside cell 1's rect: x=76..104, y=88..113
        clicked: true,
        ..Default::default()
    };
    entry.pointer(&pointer);
    assert_eq!(entry.selected_cell(), Some(0));
    assert_eq!(entry.text(), "ab", "a click only selects, never cycles");
}

#[test]
fn pointer_scroll_on_a_cell_cycles_its_glyph() {
    let mut entry = TagEntry::new(labels(), geometry(3), ALPHABET, "ab");
    let pointer = oag_ui::pointer::Pointer {
        at: Some((60.0, 90.0)),
        scroll: 1,
        ..Default::default()
    };
    entry.pointer(&pointer);
    assert_eq!(entry.text(), "bb", "cell 0's glyph cycled from a to b");
}

#[test]
fn pointer_click_on_confirm_accepts() {
    let mut entry = TagEntry::new(labels(), geometry(2), ALPHABET, "ab");
    let pointer = oag_ui::pointer::Pointer {
        at: Some((400.0, 90.0)),
        clicked: true,
        ..Default::default()
    };
    assert_eq!(entry.pointer(&pointer), Outcome::Accepted);
}

#[test]
fn a_name_longer_than_the_row_is_still_built_without_panicking() {
    // Not this module's gate - `session::pilot_editor` refuses to construct
    // one of these at all past `length`, see `docs/formats/fexml.md`. This
    // only proves the constructor itself does not index out of bounds if
    // that gate is ever bypassed.
    let entry = TagEntry::new(labels(), geometry(2), ALPHABET, "abcdef");
    assert_eq!(entry.text(), "ab");
}
