//! The two titles' menu skins, checked against each other.
//!
//! Needs no disc image and runs in CI, unlike almost everything else that
//! compares these two: the thing under test is the two title packages'
//! constants, and the risk it guards is a future edit quietly giving Pure a
//! Pulse number because Pure declares fewer of them.
//!
//! What each number *is* is checked against the disc by
//! `menu_layout_ground_truth.rs`, which does need an image.

use oag_title::MenuSkin;

fn pulse() -> &'static MenuSkin {
    oag_pulse::TITLE.menu
}

fn pure() -> &'static MenuSkin {
    oag_pure::TITLE.menu
}

/// Every layout number the two discs both state, they state differently.
///
/// Measured off `pulse-psp-usa.chd` and `pure-psp-eu.chd`: `MenuXOffset` 50
/// against 21, `MenuScale` 1.0 against 1.15, `TitleScale` 1.0 against 0.97,
/// `TitleYOffset` 0 against 20. If any of these ever come to match, the likely
/// cause is one table being filled in from the other rather than from a disc.
#[test]
fn the_two_titles_disagree_on_every_number_they_both_state() {
    assert_ne!(pulse().menu_x, pure().menu_x, "MenuXOffset");
    assert_ne!(pulse().menu_scale, pure().menu_scale, "MenuScale");
    assert_ne!(pulse().title_y, pure().title_y, "TitleYOffset");
    assert_ne!(pulse().title_scale, pure().title_scale, "TitleScale");
    assert_ne!(pulse().text, pure().text, "TextColor");
    assert_ne!(pulse().title, pure().title, "TitleColor");
}

/// Pure states less, and the gaps are `None` rather than filled in.
///
/// `docs/formats/pure-status.md` holds `oag-pure` to what has actually been
/// measured. Pure's disc carries no `MainMenu_Definition.xml`, so it has no
/// first row, no row pitch and no menu font role to state, and no capture of a
/// Pure menu cursor exists.
#[test]
fn pure_leaves_unread_fields_empty_rather_than_borrowing_pulses() {
    assert!(pure().first_row_y.is_none(), "Pure has no menu definition");
    assert!(pure().row_extra_leading.is_none(), "never captured");
    assert!(pure().menu_font.is_none(), "three roles, none confirmed");
    assert!(pure().selected.is_none(), "never captured");
}

/// Pulse states all four, because all four were read or measured.
#[test]
fn pulse_states_what_its_disc_and_its_capture_gave() {
    assert_eq!(
        pulse().first_row_y,
        Some(32.0),
        "MainMenu_Definition menu y"
    );
    assert_eq!(pulse().row_extra_leading, Some(6.0), "measured pitch rule");
    assert_eq!(pulse().menu_font, Some("menu"), "the row widget's font");
    assert!(pulse().selected.is_some(), "measured off a capture");
}

/// The pitch rule reproduces all four menus it was measured on.
///
/// `menu` is a 22-pixel face and `small` a 17-pixel one, per
/// `docs/formats/fnt.md`; the original draws them 28 and 23 apart. The `extra`
/// argument is irrelevant here because Pulse states its own.
#[test]
fn pulses_pitch_rule_matches_the_captures() {
    assert!((pulse().row_pitch(22.0, 0.0) - 28.0).abs() < 0.001);
    assert!((pulse().row_pitch(17.0, 0.0) - 23.0).abs() < 0.001);
}
