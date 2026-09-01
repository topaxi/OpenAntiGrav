//! The horizontal idiom: which pages get it, where it puts them, and which
//! button steps it.
//!
//! Two skins are used here where the rest of this directory uses one. Pulse's
//! is the control - its disc authors no `<HorizMenu>` at all, so every
//! assertion that a page is still a column is a real assertion about a real
//! title rather than about a fixture with a field cleared. Wipeout HD's is the
//! shipped table, so a change to the recovered numbers fails here.

use super::*;

/// Wipeout HD's own skin, in its own 1920x1080 grid.
///
/// The line height is a stand-in and says so: which face HD's rows are drawn in
/// depends on what `.fnt` the boot managed to read, and nothing in a strip's
/// layout reads it - entries advance by width, not by pitch. A number is passed
/// because `Skin::new` needs one.
fn hd_skin() -> Skin {
    Skin::new(
        oag_hd::frontend::MENU_SKIN,
        crate::frontend::Space::HD,
        22.0,
    )
}

/// Every left-aligned text in draw order, as `(x, y, colour, text)`.
fn labels(list: &[Draw]) -> Vec<(f32, f32, [f32; 4], String)> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text {
                x,
                y,
                color,
                align: Align::Left,
                text,
                ..
            } => Some((*x, *y, *color, text.clone())),
            _ => None,
        })
        .collect()
}

/// The root page's entries run left to right from the widget's own anchor.
///
/// The anchor is `x="160" y="125"` off `MainMenu_Definition.xml`, in the
/// 1920x1080 grid HD authors in and this skin draws in - so the numbers come
/// through unconverted, and the test says so by asserting the literals the disc
/// writes rather than the field it read them into.
#[test]
fn hds_root_page_runs_left_to_right_from_its_own_anchor() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let list = draw_list(
        &menu,
        &hd_skin(),
        &no_bindings,
        &measure,
        None,
        &Frame::default(),
    )
    .flatten();

    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, _, _, text)| text != menu.page().title.as_str())
        .collect();
    assert_eq!(rows.len(), 4, "RACE, REMIX, OPTIONS and QUIT: {rows:?}");
    for (_, y, _, text) in &rows {
        assert!(
            (y - 125.0).abs() < f32::EPSILON,
            "every entry shares the strip's y: {text} at {y}"
        );
    }
    assert!(
        (rows[0].0 - 160.0).abs() < f32::EPSILON,
        "the first entry sits on the anchor: {}",
        rows[0].0
    );
    // Each entry starts one measured width and one gap past the last, which is
    // the whole of the layout rule. The gap is this build's own; the widths are
    // the face's.
    let gap = hd_skin().strip_gap();
    for pair in rows.windows(2) {
        let expected = pair[0].0 + measure(&pair[0].3) + gap;
        assert!(
            (pair[1].0 - expected).abs() < 0.001,
            "{} should start at {expected}, not {}",
            pair[1].3,
            pair[1].0
        );
    }

    // And the whole strip fits the frame, which every assertion above would
    // pass without: they check the spacing *between* entries, so a strip that
    // ran off the right-hand edge would look correct to all of them. Nothing
    // wraps or scrolls a strip - see `strip`'s own docs on why there is no
    // carousel - so this is the bound that has to hold.
    let (last_x, _, _, last) = rows.last().expect("a strip has entries");
    let right = last_x + measure(last);
    assert!(
        right < crate::frontend::Space::HD.size.0,
        "the strip ends at {right}, past the {}-wide frame",
        crate::frontend::Space::HD.size.0
    );
}

/// Every fill in draw order, as `(rect, colour)` - a strip entry's own tab
/// and, on the selected one, its underline mark.
fn fills(list: &[Draw]) -> Vec<([f32; 4], [f32; 4])> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, color } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

/// A frame carrying the two colours a real boot resolves off `HD_Grey` and
/// `HD_Blue` - arbitrary numbers here, chosen only to be distinct from each
/// other and from `skin.normal()`'s white.
fn frame_with_fills() -> Frame {
    Frame {
        ink: Some([0.4, 0.4, 0.4, 1.0]),
        tab_selected: Some([0.54, 0.75, 0.79, 1.0]),
        ..Frame::default()
    }
}

/// Every strip entry is the same text colour, selected or not.
///
/// **Corrected 2026-09-01, by a capture.** This test used to assert the
/// opposite - that an unselected entry takes the widget's own literal colour
/// (`0xff705070`) rather than `FEGlobals->TextColor` - on the reasoning that a
/// widget declaring its own colour must be overriding the global one.
/// Plausible, and wrong: an RPCS3 capture of the real menu shows every entry,
/// selected or not, in the same white `TextColor`. What `0xff705070` is for
/// is still open; it is not this. See `docs/formats/hd-frontend.md`.
#[test]
fn every_entry_is_the_same_text_colour_selected_or_not() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let skin = hd_skin();
    let list = draw_list(
        &menu,
        &skin,
        &no_bindings,
        &measure,
        None,
        &frame_with_fills(),
    )
    .flatten();

    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, y, _, _)| (y - 125.0).abs() < f32::EPSILON)
        .collect();
    assert_eq!(rows.len(), 4, "RACE, REMIX, OPTIONS and QUIT: {rows:?}");
    for (_, _, color, text) in &rows {
        assert_eq!(*color, skin.normal(), "{text} should be skin.normal()");
    }

    // And the widget's own colour is still what `Skin::strip` reads - it is
    // this test's job to prove nothing reaches for it as text, not that the
    // field is gone.
    let authored = skin.strip().expect("HD authors a strip").color;
    assert!(
        rows.iter().all(|(_, _, color, _)| *color != authored),
        "nothing here should still be drawn in the widget's own colour"
    );
}

/// A selected entry's tab is the frame's accent; every other entry's is its
/// ink - the two colours a 2026-09-01 capture confirmed are exact reads of
/// `HD_Blue` and `HD_Grey` on the served archive.
#[test]
fn the_selected_tab_is_the_frames_accent_and_the_rest_are_its_ink() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let skin = hd_skin();
    let frame = frame_with_fills();
    let list = draw_list(&menu, &skin, &no_bindings, &measure, None, &frame).flatten();

    // Four entries: one selected tab, three unselected, plus the underline.
    let tabs = fills(&list);
    assert_eq!(tabs.len(), 5, "four tabs and one underline: {tabs:?}");
    let accents = tabs
        .iter()
        .filter(|(_, color)| *color == frame.tab_selected.unwrap())
        .count();
    let inks = tabs
        .iter()
        .filter(|(_, color)| *color == frame.ink.unwrap())
        .count();
    // The underline is drawn in `skin.normal()`, not either fill colour, so
    // it counts toward neither bucket and the two sum to four rather than
    // five.
    assert_eq!(accents, 1, "one selected tab: {tabs:?}");
    assert_eq!(inks, 3, "three unselected tabs: {tabs:?}");
}

/// A frame with neither colour draws no tabs and no underline - only the
/// labels still draw. The underline is gated on the tab fill rather than on
/// selection alone: a mark reading as "underlining the tab" with no tab
/// under it would be a floating dash on nothing, not the absence
/// `super::read_frame` already chose for the fill itself. Text is
/// unconditional, the same "skip rather than invent" rule a mark with no
/// placement already follows, applied here to marks with a title-package
/// field behind them and not to the one drawn regardless.
#[test]
fn with_no_frame_colours_nothing_but_the_text_draws() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let skin = hd_skin();
    let list = draw_list(
        &menu,
        &skin,
        &no_bindings,
        &measure,
        None,
        &Frame::default(),
    )
    .flatten();

    let tabs = fills(&list);
    assert!(
        tabs.is_empty(),
        "no ink or accent, so no tab fill and no underline either: {tabs:?}"
    );
    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, y, _, _)| (y - 125.0).abs() < f32::EPSILON)
        .collect();
    assert_eq!(rows.len(), 4, "the text draws regardless: {rows:?}");
}

/// A page with a value column stays a column, on a strip title too.
///
/// `options` is the case worth pinning: HD's own `Additional` screen is a
/// `<HorizMenu>` and this build hangs a `choice` off its counterpart, so the
/// page that looks most like a strip is the one that cannot be one. See
/// `strip::suits`.
#[test]
fn a_page_with_a_value_column_stays_a_column() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    // RACE, REMIX, OPTIONS - two steps down past the entry Race Remix added.
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options", "the page with the choice on it");

    let skin = hd_skin();
    let list = draw_list(
        &menu,
        &skin,
        &no_bindings,
        &measure,
        None,
        &Frame::default(),
    )
    .flatten();
    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, _, _, text)| text != menu.page().title.as_str())
        .collect();
    assert!(rows.len() > 1, "{rows:?}");
    for pair in rows.windows(2) {
        assert!(
            (pair[0].0 - pair[1].0).abs() < f32::EPSILON,
            "a column shares one x: {rows:?}"
        );
        assert!(pair[1].1 > pair[0].1, "and steps down: {rows:?}");
    }
}

/// A title that authors no strip draws a column even on a navigation page.
///
/// The control for every test above: same page, same button, Pulse's own table.
#[test]
fn a_title_that_authors_no_strip_draws_a_column() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(oag_pulse::FRONT_END.menu.strip.is_some());
    let list = list(&menu, &no_bindings, None);
    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, _, _, text)| text != menu.page().title.as_str())
        .collect();
    assert_eq!(rows.len(), 4, "{rows:?}");
    for pair in rows.windows(2) {
        assert!(pair[1].1 > pair[0].1, "the rows step down: {rows:?}");
    }
}

/// Right and left step a strip, because that is the axis it is drawn on.
#[test]
fn right_and_left_step_a_strip() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    assert_eq!(menu.selected(), 0);
    press(&mut menu, &[Button::Right]);
    assert_eq!(menu.selected(), 1, "right moves along the strip");
    press(&mut menu, &[Button::Left]);
    assert_eq!(menu.selected(), 0, "and left moves back");
    press(&mut menu, &[Button::Left]);
    assert_eq!(menu.selected(), 3, "wrapping the way up and down do");
}

/// Right and left are left to the values on a column, which is every page a
/// PSP title draws and every page of this build's that carries a setting.
#[test]
fn right_and_left_do_not_step_a_column() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(false);
    press(&mut menu, &[Button::Right]);
    assert_eq!(
        menu.selected(),
        0,
        "a column's cursor moves with up and down alone"
    );

    // And on a strip title, a page that is not a strip behaves the same way:
    // the flag says what the disc draws, `suits` says which pages.
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options");
    let selected = menu.selected();
    press(&mut menu, &[Button::Right]);
    assert_eq!(
        menu.selected(),
        selected,
        "the page has a value column, so right belongs to the value"
    );
}
