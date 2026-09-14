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
        oag_display::space::Space::HD,
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

/// Where a strip's first label's pen lands, given the anchor and the pads.
///
/// The anchor is the **tab's** top-left corner; a label sits one pad in from it
/// on each axis. See `strip`'s own "what the anchor means" note for the
/// measurement that settled which of the two the disc's `x`/`y` is.
fn first_pen(skin: &Skin) -> (f32, f32) {
    let strip = skin.strip().expect("HD authors a strip");
    let (left_pad, top_pad) = skin.tab_pad();
    (strip.x + left_pad, strip.y + top_pad)
}

/// The root page's entries run left to right from the widget's own anchor, and
/// the anchor is the first tab's own corner.
///
/// The anchor is `x="160" y="125"` off `MainMenu_Definition.xml`, in the
/// 1920x1080 grid HD authors in and this skin draws in - so the numbers come
/// through unconverted, and the test says so by asserting the literals the disc
/// writes rather than the field it read them into.
///
/// **What the literals are attached to changed on 2026-09-05.** They used to be
/// asserted of the first *label*; a calibrated capture of the real menu shows
/// the real tab's corner at `(160.5, 126.0)` against those authored `(160, 125)`,
/// so the anchor is the tab's corner and the label is inset from it. Asserting
/// the corner is the stronger claim of the two - it pins the pads' *sense* as
/// well as their size, which is exactly what was wrong before.
#[test]
fn hds_root_page_runs_left_to_right_from_its_own_anchor() {
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
        false,
    )
    .flatten();

    // The first tab's own corner is the disc's `x="160" y="125"`, exactly.
    // `strip::draw` pushes the body before the cut band, and the band carries
    // the tab's top edge, so the band is the one to read the corner off.
    let (band, ..) = list
        .iter()
        .find_map(|draw| match draw {
            Draw::ChamferedFill { rect, chamfer, .. } => Some((*rect, chamfer[1])),
            _ => None,
        })
        .expect("a tab's cut band");
    assert!(
        (band[0] - 160.0).abs() < 0.001 && (band[1] - 125.0).abs() < 0.001,
        "the first tab's corner is the authored anchor, not {:?}",
        [band[0], band[1]]
    );

    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, _, _, text)| text != menu.page().title.as_str())
        .collect();
    assert_eq!(
        rows.len(),
        5,
        "RACE, REMIX, RECORDS, OPTIONS and QUIT: {rows:?}"
    );
    let (pen_x, pen_y) = first_pen(&skin);
    for (_, y, _, text) in &rows {
        assert!(
            (y - pen_y).abs() < 0.001,
            "every entry shares the strip's pen y {pen_y}: {text} at {y}"
        );
    }
    assert!(
        (rows[0].0 - pen_x).abs() < 0.001,
        "the first entry sits one left pad in from the anchor, at {pen_x}, not {}",
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
        right < oag_display::space::Space::HD.size.0,
        "the strip ends at {right}, past the {}-wide frame",
        oag_display::space::Space::HD.size.0
    );
}

/// Every fill in draw order, as `(rect, colour)` - a strip entry's own tab
/// and, on the selected one, its underline mark.
fn fills(list: &[Draw]) -> Vec<([f32; 4], [f32; 4])> {
    list.iter()
        .filter_map(|draw| match draw {
            // Both, because a tab is one of each: the full-width part below
            // the cut is a `Fill` and the band above it a `ChamferedFill`.
            // See `strip::draw`.
            Draw::Fill { rect, color } | Draw::ChamferedFill { rect, color, .. } => {
                Some((*rect, *color))
            }
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
        false,
    )
    .flatten();

    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, y, _, _)| (y - first_pen(&skin).1).abs() < 0.001)
        .collect();
    assert_eq!(
        rows.len(),
        5,
        "RACE, REMIX, RECORDS, OPTIONS and QUIT: {rows:?}"
    );
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
    let list = draw_list(&menu, &skin, &no_bindings, &measure, None, &frame, false).flatten();

    // Five entries, two fills each (the cut band and the rest of the tab -
    // see `strip::draw`), plus one underline.
    let tabs = fills(&list);
    assert_eq!(
        tabs.len(),
        11,
        "five tabs of two fills, one underline: {tabs:?}"
    );
    let accents = tabs
        .iter()
        .filter(|(_, color)| *color == frame.tab_selected.unwrap())
        .count();
    let inks = tabs
        .iter()
        .filter(|(_, color)| *color == frame.ink.unwrap())
        .count();
    // The underline is drawn in `skin.normal()`, not either fill colour, so
    // it counts toward neither bucket and the two sum to ten rather than
    // eleven.
    assert_eq!(accents, 2, "one selected tab, two bands: {tabs:?}");
    assert_eq!(inks, 8, "four unselected tabs, two bands each: {tabs:?}");
}

/// The tab's corner is a 45-degree cut with a flat landing after it, and the
/// three pieces line up into one pentagon.
///
/// **The shape is the claim, not the constants.** Each of the three numbers
/// could be right on its own and still not join up - the earlier one-step band
/// had the cut's extent right and drew a right angle, and the diagonal built
/// and reverted on 2026-09-01 had the slope right and ran it to the corner with
/// no landing. So this asserts the relationships that make it a pentagon: the
/// band sits directly on top of the body, its right edge is short of the body's
/// by the landing, and its own cut is square (45 degrees) rather than any other
/// slope. Measured on two captures three times apart in resolution - see
/// `Skin::TAB_CHAMFER_CUT` and `docs/formats/hd-frontend.md`.
#[test]
fn the_tab_corner_is_a_45_degree_cut_followed_by_a_flat_landing() {
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
        false,
    )
    .flatten();

    // The first entry's two pieces, in the order `strip::draw` pushes them.
    let body = list
        .iter()
        .find_map(|draw| match draw {
            Draw::Fill { rect, .. } => Some(*rect),
            _ => None,
        })
        .expect("a tab body");
    let (band, chamfer) = list
        .iter()
        .find_map(|draw| match draw {
            Draw::ChamferedFill { rect, chamfer, .. } => Some((*rect, chamfer[1])),
            _ => None,
        })
        .expect("a cut band");

    let (cut, landing, height) = skin.tab_chamfer();
    assert!(cut > 0.0 && landing > 0.0, "both halves of the cut region");

    // The band sits directly on the body, sharing its left edge - one shape,
    // not two overlapping ones or two with a seam between them.
    assert!((band[0] - body[0]).abs() < 0.001, "{band:?} vs {body:?}");
    assert!(
        (band[1] + band[3] - body[1]).abs() < 0.001,
        "the band's bottom is the body's top: {band:?} vs {body:?}"
    );
    assert!(
        (band[3] - height).abs() < 0.001,
        "the band is the cut's own height: {band:?}"
    );

    // The landing: the band stops short of the body's right edge by exactly
    // it, and nothing is drawn in the gap. That gap *is* the flat top edge at
    // the lower level - it is the body's own top showing through.
    let landing_gap = (body[0] + body[2]) - (band[0] + band[2]);
    assert!(
        (landing_gap - landing).abs() < 0.001,
        "the band should stop {landing} short of the tab's right edge, not {landing_gap}"
    );

    // And the cut is 45 degrees: as far across as it is down. This is the one
    // the reverted attempt got right and the shipped one-step band did not.
    //
    // **Within 2%, not exactly, and the slack is a real grid difference rather
    // than float noise.** The constants live in this build's own 480x272 grid
    // and `Skin` converts them per axis, but 480x272 is `1.765:1` where HD's
    // 1920x1080 is `1.778:1` - so a square in one grid is 0.8% off square in
    // the other, and the cut was measured in HD's. Asserting equality here
    // failed at `9.0` against `8.93` for exactly that reason. Anything that
    // actually broke the slope - reading the landing as the cut, or the old
    // `2.6` height against a `2.25` cut - is 15% or more out and caught.
    let slope = (chamfer - band[3]).abs() / band[3];
    assert!(
        slope < 0.02,
        "a {chamfer}-wide cut over a {}-tall band is {:.1}% off 45 degrees",
        band[3],
        slope * 100.0
    );
}

/// A label narrow enough that its tab is thinner than the landing shrinks the
/// band to nothing rather than inverting it.
///
/// The tab's width follows `measure(label)`, so a short enough entry on a
/// small enough skin genuinely reaches this - and a negative width is not a
/// smaller quad, it is a back-facing one that may or may not be culled. The
/// cut itself needs no guard here: `Draw::ChamferedFill` clamps it to the
/// band's own width, so a zero-width band takes a zero-width cut.
#[test]
fn a_tab_narrower_than_the_landing_collapses_its_band_rather_than_inverting_it() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let skin = hd_skin();
    let (_, landing, _) = skin.tab_chamfer();
    let hairline = |_: &str| 0.0;
    let list = draw_list(
        &menu,
        &skin,
        &no_bindings,
        &hairline,
        None,
        &frame_with_fills(),
        false,
    )
    .flatten();

    let bands: Vec<_> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::ChamferedFill { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(bands.len(), 5, "one band per entry: {bands:?}");
    for band in &bands {
        assert!(
            band[2] >= 0.0,
            "a tab narrower than the {landing}-unit landing must not invert: {band:?}"
        );
    }
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
        false,
    )
    .flatten();

    let tabs = fills(&list);
    assert!(
        tabs.is_empty(),
        "no ink or accent, so no tab fill and no underline either: {tabs:?}"
    );
    let rows: Vec<_> = labels(&list)
        .into_iter()
        .filter(|(_, y, _, _)| (y - first_pen(&skin).1).abs() < 0.001)
        .collect();
    assert_eq!(rows.len(), 5, "the text draws regardless: {rows:?}");
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
    // RACE, REMIX, RECORDS, OPTIONS - three steps down past the entries Race
    // Remix and RECORDS added.
    press(&mut menu, &[Button::Down]);
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
        false,
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
    assert_eq!(rows.len(), 5, "{rows:?}");
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
    assert_eq!(menu.selected(), 4, "wrapping the way up and down do");
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
    // the flag says what the disc draws, `suits` says which pages. Three
    // downs past RACE, REMIX, RECORDS - see `a_page_with_a_value_column_
    // stays_a_column`'s own comment for why it is three and not two.
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    press(&mut menu, &[Button::Down]);
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
