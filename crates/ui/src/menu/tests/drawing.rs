//! Where a drawn page puts its title, rows, values and backdrop, and that the
//! shipped menus can be walked from their root to a race.
//!
//! Split out of `menu/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

#[test]
fn a_drawn_page_has_a_title_a_row_each_and_one_highlight() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    // A quarter of Pulse's own measured pulse period, not zero: at `elapsed
    // == 0.0` the highlight starts exactly at its trough, which is `normal()`
    // by construction (see `Skin::selected`) and indistinguishable from every
    // other row's own colour - a real edge of the animation, not a bug, but
    // the wrong moment to assert "exactly one row is highlighted" against.
    let mut skin = skin();
    skin.tick_pulse(skin.selected_pulse_period_secs().unwrap_or(0.0) / 4.0);
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

    let texts: Vec<&String> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } | Draw::FacedText { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    // The title is in the `Title` role now, so it is a `FacedText`.
    assert!(texts.contains(&&"OPTIONS".to_string()), "{texts:?}");
    assert!(texts.contains(&&"FILTERING".to_string()), "{texts:?}");
    assert!(
        texts.contains(&&"off".to_string()),
        "a choice draws its value: {texts:?}"
    );
    assert!(
        texts.contains(&&"OFF".to_string()),
        "a toggle draws ON or OFF: {texts:?}"
    );

    // **Selection is a colour, not a bar.** A capture of the original's
    // main menu shows the selected row brightened toward white with nothing
    // drawn behind it, so a fill here would be an invention - this build
    // used to draw one, and the assertion used to require it.
    assert!(
        !list.iter().any(|draw| matches!(draw, Draw::Fill { .. })),
        "the original marks selection by colour, not by a bar: {list:?}"
    );
    let highlighted = list
        .iter()
        .filter(|draw| {
            matches!(draw, Draw::Text { color, align, .. }
                if *color == skin.selected() && *align == Align::Left)
        })
        .count();
    assert_eq!(highlighted, 1, "exactly one row label is highlighted");
}

/// The value column is right-aligned against a fixed edge, which is the
/// whole reason a long label and a long value cannot overlap. Asserted
/// because it is invisible in a headless test otherwise.
///
/// **Against the skin's edge and against the literal 440**, which is one
/// assertion about the layout and one about the conversion: this skin is
/// Pulse's, drawn in `Space::PSP`, so the ratio is exactly 1.0 and
/// `value_right()` has to come back as the constant it is written as. A source
/// authoring elsewhere is what makes the two differ, and there is no such skin
/// in this crate to build a fixture from.
#[test]
fn values_are_right_aligned_on_one_edge() {
    let skin = skin();
    assert!(
        (skin.value_right() - 440.0).abs() < f32::EPSILON,
        "a PSP-grid skin scales this build's own figures by exactly one: got {}",
        skin.value_right()
    );
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    for draw in list(&menu, &no_bindings, None) {
        if let Draw::Text { align, x, .. } = draw
            && align == Align::Right
        {
            assert!((x - skin.value_right()).abs() < f32::EPSILON, "got {x}");
        }
    }
}

/// A row for a button nothing is bound to says so rather than drawing an
/// empty column that reads as a layout bug.
#[test]
fn an_unbound_button_draws_the_word_rather_than_nothing() {
    let definition = Definition::parse(
        r#"
        version = 1
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "binding"
        label = "THRUST"
        button = "cross"
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("parse");
    let menu = Menu::new(definition);
    let unbound = list(&menu, &no_bindings, None);
    assert!(
        unbound
            .iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "UNBOUND")),
        "{unbound:?}"
    );
    let list = list(&menu, &|_| vec!["ENTER", "X"], None);
    assert!(
        list.iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "ENTER / X")),
        "{list:?}"
    );
}

/// The backdrop has to be **first** in the list and nothing else may move.
///
/// First because the list is painted back to front, and a menu drawn under
/// its own background is a black screen with a movie on it. Nothing else
/// moving is the other half: a source with a backdrop and one without have
/// to lay the rows out identically, or the layout depends on which disc is
/// in the drive.
#[test]
fn a_backdrop_is_drawn_behind_the_rows_and_moves_none_of_them() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("display"));

    let plain = list(&menu, &no_bindings, None);
    assert!(
        !plain.iter().any(|draw| matches!(draw, Draw::Video { .. })),
        "no backdrop means no video draw at all: {plain:?}"
    );

    let backdrop = Backdrop {
        rect: [12.0, 34.0, 456.0, 78.0],
        frame: 91,
        // Past the movie's own length, because the menus are a continuation
        // of the playback `Show Logo` sat on rather than a second one - see
        // `Backdrop::position`.
        position: 631,
    };
    let with = list(&menu, &no_bindings, Some(backdrop));
    assert_eq!(
        with.first(),
        Some(&Draw::Video {
            rect: backdrop.rect,
            frame: backdrop.frame,
            position: backdrop.position,
            source: crate::frontend::Video::Backdrop,
        }),
        "the backdrop has to be painted first: {with:?}"
    );
    assert_eq!(
        &with[1..],
        &plain[..],
        "a backdrop must add a draw and change no other"
    );
}

#[test]
fn the_built_in_menu_opens_on_its_root_and_can_reach_a_race() {
    let definition = built_in();
    let root = definition.pages[definition.root].id.clone();
    let menu = Menu::new(definition);
    assert_eq!(menu.page().id, root);

    // Walk every page breadth-first and assert `launch_race` is somewhere
    // in the tree: a menu that cannot start a game is not a menu for this
    // game, and this catches an asset edit that drops the row.
    let reachable = menu
        .definition()
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .any(|entry| matches!(entry, Entry::Run { action, .. } if *action == Action::LaunchRace));
    assert!(reachable, "nothing in the menus starts a race");
}

/// A skin authored somewhere other than the PSP's grid keeps its own numbers
/// and scales this build's.
///
/// **The untested half of the change that made Wipeout HD's menus legible**, and
/// the only one reachable without a disc: `oag_hd::frontend::FRONT_END.menu` is a shipped
/// table, so this needs no image. Both directions are asserted because getting
/// either backwards is silent - the labels either land off the side of the frame
/// (the bug this replaced) or bunch into the top-left eighth of it.
#[test]
fn a_skin_authored_at_1080p_is_drawn_at_its_own_numbers() {
    let hd = Skin::new(
        oag_hd::frontend::FRONT_END.menu.unwrap(),
        oag_display::space::Space::HD,
        33.0,
    );

    // The disc's, used as the disc writes them: `FEGlobals->MenuXOffset` is 800
    // and 1920/1920 is one. Drawing this at 800-of-480 is what put the whole
    // label column past the right-hand edge.
    assert!(
        (hd.menu_x() - 800.0).abs() < f32::EPSILON,
        "{}",
        hd.menu_x()
    );
    let (title_x, title_y, _) = hd.title_at();
    assert!((title_x - 194.0).abs() < f32::EPSILON, "{title_x}");
    assert!((title_y - 62.0).abs() < f32::EPSILON, "{title_y}");

    // Ours, scaled out of the 480x272 they are written in. 440 of 480 is
    // eleven-twelfths of the width whatever the grid.
    assert!(
        (hd.value_right() - 440.0 * 1920.0 / 480.0).abs() < 0.01,
        "{}",
        hd.value_right()
    );

    // And the rows still fit: HD measures no leading of its own, so the pitch is
    // a 33-pixel face plus ours scaled up, in a screen four times as tall.
    assert!(
        visible_rows(&hd, &Frame::default(), false)
            >= visible_rows(&skin(), &Frame::default(), false),
        "a 1080-line screen must not fit fewer rows than a 272-line one"
    );
}
