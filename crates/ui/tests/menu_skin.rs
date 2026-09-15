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
    oag_pulse::FRONT_END.menu
}

fn pure() -> &'static MenuSkin {
    oag_pure::FRONT_END.menu
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
/// measured. Both of these stayed `None` for different reasons - see
/// `oag_pure::frontend::MENU_SKIN`'s own comments: nothing has ever captured a
/// Pure menu's row pitch; and the menus turned out to name no font role of
/// their own, so there is none to state rather than one still unread.
#[test]
fn pure_leaves_unread_fields_empty_rather_than_borrowing_pulses() {
    assert!(pure().row_extra_leading.is_none(), "never captured");
    assert!(
        pure().menu_font.is_none(),
        "Pure's menus draw in the default face - its language plugins declare no \
         Menu slot at all"
    );
}

/// Pure's `Main Menu` capture (2026-08-25) filled in two fields a single
/// earlier `Title Screen` capture never reached - `background`, off
/// `BackgroundController`'s own `src`-less `BackgroundImage`, and `selected`,
/// off `SINGLE PLAYER` against its unselected siblings. Pinned here because a
/// title with no measured `selected` used to mean a real, visible bug on
/// Pure specifically: this build's own substitute is white, and once
/// `background` started drawing a white screen under it, `selected: None`
/// was a white row on white.
#[test]
fn pures_main_menu_capture_measured_its_background_and_its_selected_row() {
    assert_eq!(pure().background, Some(0xFFFF_FFFF));
    assert!(pure().selected.is_some(), "measured off SINGLE PLAYER");
    assert_ne!(
        pure().selected,
        Some(0xFFFF_FFFF),
        "this build's own OUR_SELECTED substitute is white too - a title whose \
         measured selected happened to equal it would pass by coincidence \
         rather than by having been measured at all"
    );
}

/// Pure's first row is its own measurement, not Pulse's number.
///
/// Read across the whole GUI tree because this disc has no
/// `MainMenu_Definition.xml` to read one screen out of: 19 of its 33 `<Menu>`
/// widgets say `y="45"`, and no other value reaches four. Pinned apart from
/// `pulse_states_what_its_disc_and_its_capture_gave` so that the two cannot
/// quietly converge.
#[test]
fn pures_first_row_is_measured_off_its_own_gui_tree() {
    assert_eq!(pure().first_row_y, Some(45.0), "19 of 33 <Menu> widgets");
    assert_ne!(
        pure().first_row_y,
        pulse().first_row_y,
        "the two discs disagree here, so borrowing would have been wrong"
    );
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

/// Only Pulse's highlight pulse is measured - a 2026-09-05 frame-accurate
/// PPSSPP capture, 33 presented frames trough to trough across four cycles.
/// HD authors no oscillation anywhere (a whole-front-end census) and Pure's
/// own selected row is simply unmeasured for one, so both must stay `None`
/// rather than borrow Pulse's period the way `pure_leaves_unread_fields_empty`
/// already guards other fields against.
#[test]
fn only_pulse_states_a_measured_highlight_pulse() {
    assert_eq!(pulse().selected_pulse_period_secs, Some(1.1));
    assert_eq!(
        pure().selected_pulse_period_secs,
        None,
        "unmeasured on Pure"
    );
    assert_eq!(
        oag_hd::frontend::MENU_SKIN.selected_pulse_period_secs,
        None,
        "HD's front end authors no oscillation anywhere"
    );
}

/// Only Pulse's per-row subtitle geometry is measured - `MainMenu_Definition
/// .xml`'s seven `helptext` widgets, none of which Pure or HD have an
/// equivalent capture for. `None` on both is a measurement, the same rule
/// [`only_pulse_states_a_measured_highlight_pulse`] already guards its own
/// axis with.
#[test]
fn only_pulse_states_measured_help_text_geometry() {
    let help_text = pulse().help_text.expect("measured off Main Menu");
    assert!((help_text.offset_y - 18.0).abs() < f32::EPSILON);
    assert_eq!(help_text.color, 0xFFFF_FFFF);
    assert!(pure().help_text.is_none(), "unmeasured on Pure");
    assert!(
        oag_hd::frontend::MENU_SKIN.help_text.is_none(),
        "HD authors no per-row help text either"
    );
}

/// One title draws a strip and two draw columns, and each says so itself.
///
/// The third title is here because this axis is the one where HD is not simply
/// *more* than the PSP titles: it lays its main menu out on the other axis
/// entirely. Both `None`s are measurements - four pressings across two titles
/// and two consoles author no `<HorizMenu>` anywhere, see `oag_title::MenuStrip`
/// - so this fails the day one of them is filled in from HD's numbers.
#[test]
fn only_hd_authors_a_horizontal_strip() {
    assert!(pulse().strip.is_none(), "Pulse's main menu is a column");
    assert!(pure().strip.is_none(), "Pure's are too");

    let strip = oag_hd::frontend::MENU_SKIN
        .strip
        .expect("HD's main menu is a <HorizMenu>");
    assert!((strip.x - 160.0).abs() < f32::EPSILON);
    assert!((strip.y - 125.0).abs() < f32::EPSILON);
    assert_eq!(strip.color, 0xFF70_5070);
    // A strip title has no first row for `first_row_y` to be the top of, which
    // is the pair of readings HD's own comments make together.
    assert!(oag_hd::frontend::MENU_SKIN.first_row_y.is_none());
    // And the widget's colour is not the global, which is the mistake the
    // drawing side would make silently.
    assert_ne!(Some(strip.color), oag_hd::frontend::MENU_SKIN.text);
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
