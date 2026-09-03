//! The cursor, the scrolling window a page longer than the screen shows
//! itself through, the zoom transition, and the back stack.
//!
//! Split out of `menu/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

#[test]
fn the_cursor_wraps_both_ways() {
    let mut menu = Menu::new(fixture());
    assert_eq!(menu.selected(), 0);
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 1);
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 0, "past the last row wraps to the first");
    press(&mut menu, &[Button::Up]);
    assert_eq!(menu.selected(), 1, "and back off the top wraps to the last");
}

/// A page with more rows than fit, built here rather than taken off the
/// shipped definition so the scrolling tests below say what they mean even
/// after someone adds or removes a graphics setting.
fn long_page(rows: usize) -> Definition {
    let mut text = String::from("version = 1\nroot = \"main\"\n[[page]]\nid = \"main\"\n");
    for row in 0..rows {
        text.push_str(&format!(
            "[[page.entry]]\nkind = \"action\"\nlabel = \"ROW{row}\"\naction = \"quit\"\n"
        ));
    }
    Definition::parse(&text, &crate::language::StringTable::default())
        .expect("the fixture must parse")
}

/// The labels actually on screen, top to bottom.
fn rows_drawn(menu: &Menu) -> Vec<String> {
    list(menu, &no_bindings, None)
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, x, y, .. } if x == skin().menu_x() && y >= skin().first_row_y() => {
                Some(text)
            }
            _ => None,
        })
        .collect()
}

/// The point of the whole thing: a page longer than the screen shows a
/// window into itself, and the window moves when the cursor reaches the
/// **second-last** visible row rather than the last one, so the row a
/// player is moving towards is on screen before they get to it.
#[test]
fn a_long_page_scrolls_one_row_before_the_cursor_reaches_the_bottom() {
    let mut menu = Menu::new(long_page(visible() + 3));
    assert_eq!(menu.scroll(), 0);
    assert_eq!(rows_drawn(&menu).len(), visible(), "the window is full");

    // Down to the second-last visible row. Nothing has moved yet: the last
    // one is still below the cursor, which is the lookahead.
    for _ in 0..visible() - 2 {
        press(&mut menu, &[Button::Down]);
    }
    assert_eq!(menu.selected(), visible() - 2);
    assert_eq!(menu.scroll(), 0, "the page does not move until it must");

    // One more, and it moves by one - not by a screenful.
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), visible() - 1);
    assert_eq!(menu.scroll(), 1);
    assert_eq!(rows_drawn(&menu).first().map(String::as_str), Some("ROW1"));

    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.scroll(), 2);
}

/// The zoom moves the rows and leaves the chrome where it is.
///
/// That split is the capture's, not a convenience: the original's top bar
/// and footer stay put across a page change while the row block grows.
#[test]
fn a_zoom_moves_the_body_and_not_the_chrome() {
    let menu = Menu::new(long_page(3));
    let settled = draw_list(
        &menu,
        &skin(),
        &no_bindings,
        &measure,
        None,
        &Frame::default(),
    );
    let chrome_before = first_text_at(&settled.chrome);
    let body_before = first_text_at(&settled.body);

    let zoomed = settled.clone().zoomed((150.0, 127.0), 2.0, 0.5);
    assert_eq!(
        first_text_at(&zoomed.chrome),
        chrome_before,
        "the chrome does not move"
    );
    let (x, y) = first_text_at(&zoomed.body).expect("a row");
    let (was_x, was_y) = body_before.expect("a row");
    assert!(
        (x - (150.0 + (was_x - 150.0) * 2.0)).abs() < 0.001
            && (y - (127.0 + (was_y - 127.0) * 2.0)).abs() < 0.001,
        "the body scales about the origin: {x},{y} from {was_x},{was_y}"
    );
}

/// Both layers fade, because the capture shows the chrome crossfading in
/// place rather than staying solid through a page change.
#[test]
fn a_zoom_fades_both_layers() {
    let menu = Menu::new(long_page(3));
    let faded = draw_list(
        &menu,
        &skin(),
        &no_bindings,
        &measure,
        None,
        &Frame::default(),
    )
    .zoomed((0.0, 0.0), 1.0, 0.25);
    for draw in faded.chrome.iter().chain(faded.body.iter()) {
        if let Draw::Text { color, .. } = draw {
            assert!(color[3] <= 0.25 + 0.001, "alpha not applied: {color:?}");
        }
    }
}

/// The first left-aligned text's position, for the two tests above.
fn first_text_at(draws: &[Draw]) -> Option<(f32, f32)> {
    draws.iter().find_map(|draw| match draw {
        Draw::Text { x, y, .. } => Some((*x, *y)),
        _ => None,
    })
}

/// The rows land where a capture of the original puts them.
///
/// Pinned here so an edit that drifts the layout fails a test rather than
/// quietly un-aligning the menus. The numbers are `docs/ui/menus-original.md`'s:
/// the rows' left edge measured at exactly x=50, the pitch at exactly 28 on
/// all seven rows of the main menu, and seven rows is what that menu shows.
///
/// The first row's *glyphs* land at y=40 on hardware, not at 32 - the
/// 8-pixel difference is the glyph's inset inside its 22-pixel line box,
/// which the atlas already bakes into the cell, so 32 is what this emits.
#[test]
fn the_rows_land_where_the_capture_puts_them() {
    let menu = Menu::new(long_page(7));
    let rows: Vec<(f32, f32)> = list(&menu, &no_bindings, None)
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { x, y, align, .. } if align == Align::Left && y >= 32.0 => Some((x, y)),
            _ => None,
        })
        .collect();

    assert_eq!(rows.len(), 7, "the original's main menu shows seven rows");
    assert!(
        rows.iter().all(|(x, _)| (x - 50.0).abs() < f32::EPSILON),
        "measured left edge is exactly 50: {rows:?}"
    );
    assert!(
        (rows[0].1 - 32.0).abs() < f32::EPSILON,
        "MainMenu_Definition says y=32: {rows:?}"
    );
    for pair in rows.windows(2) {
        assert!(
            (pair[1].1 - pair[0].1 - 28.0).abs() < f32::EPSILON,
            "measured pitch is exactly 28: {rows:?}"
        );
    }
}

/// The last row is reachable and is drawn at the bottom of a full window,
/// rather than the list scrolling past its own end.
#[test]
fn the_final_row_sits_at_the_bottom_of_a_full_window() {
    let rows = visible() + 3;
    let mut menu = Menu::new(long_page(rows));
    for _ in 0..rows - 1 {
        press(&mut menu, &[Button::Down]);
    }
    assert_eq!(menu.selected(), rows - 1);
    assert_eq!(menu.scroll(), rows - visible());
    let drawn = rows_drawn(&menu);
    assert_eq!(drawn.len(), visible());
    // Named from the count rather than spelled out: how many rows fit is
    // the skin's business now, so a hard-coded `ROW10` would be asserting
    // the row pitch by accident.
    assert_eq!(drawn.last().cloned(), Some(format!("ROW{}", rows - 1)));
}

/// The mirror of the rule going down, and the reason the window is stored
/// rather than recomputed from the cursor alone: stepping back up keeps the
/// row above the cursor visible and moves nothing until it has to.
#[test]
fn moving_back_up_scrolls_one_row_before_the_cursor_reaches_the_top() {
    let rows = visible() + 3;
    let mut menu = Menu::new(long_page(rows));
    for _ in 0..rows - 1 {
        press(&mut menu, &[Button::Down]);
    }
    let bottom = menu.scroll();

    // Back up to the second row of the window: still nothing to do, the row
    // above the cursor is drawn.
    for _ in 0..visible() - 2 {
        press(&mut menu, &[Button::Up]);
    }
    assert_eq!(menu.scroll(), bottom, "the window has not moved yet");
    assert_eq!(menu.selected(), bottom + 1);

    press(&mut menu, &[Button::Up]);
    assert_eq!(menu.scroll(), bottom - 1, "one row, not a screenful");
}

/// Wrapping is the one jump the cursor makes, and the window has to go with
/// it: off the last row lands on the first with the page back at the top,
/// and off the top lands on the last with it at the end.
#[test]
fn wrapping_takes_the_window_with_it() {
    let rows = visible() + 3;
    let mut menu = Menu::new(long_page(rows));
    for _ in 0..rows {
        press(&mut menu, &[Button::Down]);
    }
    assert_eq!(menu.selected(), 0, "past the last row wraps to the first");
    assert_eq!(menu.scroll(), 0);
    assert_eq!(rows_drawn(&menu).first().map(String::as_str), Some("ROW0"));

    press(&mut menu, &[Button::Up]);
    assert_eq!(menu.selected(), rows - 1);
    assert_eq!(menu.scroll(), rows - visible());
}

/// A page that fits never scrolls, whatever the cursor does on it. Most
/// pages are this one, and a window that crept off zero on them would move
/// the rows for no reason.
#[test]
fn a_page_that_fits_never_moves() {
    let mut menu = Menu::new(long_page(visible()));
    for _ in 0..visible() * 2 {
        press(&mut menu, &[Button::Down]);
        assert_eq!(menu.scroll(), 0, "on row {}", menu.selected());
    }
}

/// The check that matters for the definition this build ships: no page
/// draws a row below the bottom of the screen, on any row the cursor can
/// be on. This is the failure the scrolling exists to fix, and it is worth
/// asserting against the real asset rather than a fixture.
#[test]
fn no_shipped_page_draws_a_row_off_the_bottom_of_the_screen() {
    let definition = built_in();
    for id in definition
        .pages
        .iter()
        .map(|page| page.id.clone())
        .collect::<Vec<_>>()
    {
        let mut menu = Menu::new(built_in());
        assert!(menu.open(&id));
        for _ in 0..menu.page().entries.len() {
            for draw in list(&menu, &no_bindings, None) {
                if let Draw::Text { y, scale, .. } = draw {
                    // The tallest disc font's line height, which is what a
                    // row is drawn with; see `crate::font::Atlas`.
                    let bottom = y + 25.0 * scale;
                    assert!(
                        bottom <= crate::frontend::SCREEN.1,
                        "page {id:?} draws to {bottom} with the cursor on row {}",
                        menu.selected()
                    );
                }
            }
            press(&mut menu, &[Button::Down]);
        }
    }
}

/// Going into a page and back out again must land on the row that was
/// selected, not on the first one.
#[test]
fn the_back_stack_restores_the_cursor() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Up]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options");
    assert_eq!(menu.depth(), 2);

    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 1);
    press(&mut menu, &[Button::Circle]);
    assert_eq!(menu.page().id, "main");
    assert_eq!(menu.selected(), 0, "the row OPTIONS was entered from");

    press(&mut menu, &[Button::Cross]);
    assert_eq!(
        menu.selected(),
        1,
        "and re-entering restores the row inside it"
    );
}
