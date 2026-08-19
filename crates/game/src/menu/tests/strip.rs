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
    assert_eq!(rows.len(), 3, "RACE, OPTIONS and QUIT: {rows:?}");
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

/// A strip entry is drawn in the widget's colour, not in `FEGlobals->TextColor`.
///
/// The two differ on this disc - `0xff705070` against a white `TextColor` - so
/// this is the assertion that catches a strip wired to `Skin::normal`, which
/// would look plausible and be wrong.
#[test]
fn an_unselected_entry_takes_the_widgets_colour_rather_than_textcolor() {
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

    let authored = skin.strip().expect("HD authors a strip").color;
    let unselected: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, y, _, _)| (y - 125.0).abs() < f32::EPSILON)
        .filter(|(_, _, color, _)| *color != skin.selected())
        .collect();
    assert_eq!(
        unselected.len(),
        2,
        "one of three is selected: {unselected:?}"
    );
    for (_, _, color, text) in unselected {
        assert_eq!(color, authored, "{text} is drawn in the widget's colour");
    }
    assert_ne!(
        authored,
        skin.normal(),
        "if these ever agree, this test stops proving anything"
    );
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
    press(&mut menu, &[button::DOWN]);
    press(&mut menu, &[button::CROSS]);
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
    assert_eq!(rows.len(), 3, "{rows:?}");
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
    press(&mut menu, &[button::RIGHT]);
    assert_eq!(menu.selected(), 1, "right moves along the strip");
    press(&mut menu, &[button::LEFT]);
    assert_eq!(menu.selected(), 0, "and left moves back");
    press(&mut menu, &[button::LEFT]);
    assert_eq!(menu.selected(), 2, "wrapping the way up and down do");
}

/// Right and left are left to the values on a column, which is every page a
/// PSP title draws and every page of this build's that carries a setting.
#[test]
fn right_and_left_do_not_step_a_column() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(false);
    press(&mut menu, &[button::RIGHT]);
    assert_eq!(
        menu.selected(),
        0,
        "a column's cursor moves with up and down alone"
    );

    // And on a strip title, a page that is not a strip behaves the same way:
    // the flag says what the disc draws, `suits` says which pages.
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    press(&mut menu, &[button::DOWN]);
    press(&mut menu, &[button::CROSS]);
    assert_eq!(menu.page().id, "options");
    let selected = menu.selected();
    press(&mut menu, &[button::RIGHT]);
    assert_eq!(
        menu.selected(),
        selected,
        "the page has a value column, so right belongs to the value"
    );
}
