//! A page answering a mouse or a finger: the regions it hands out line up
//! with what it draws, on every idiom, and the cursor and the events follow
//! the rules `super::super::pointer` states.

use super::*;

use crate::frontend::Placed;
use crate::menu::block::BlockArt;
use crate::menu::pointer::{self, Part, Region, hit};
use crate::pointer::Pointer;

fn hd_skin() -> Skin {
    Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        22.0,
    )
}

/// The frame a served HD archive resolves, with its block art - the same
/// fixture `tests/blocks.rs` builds.
fn hd_frame() -> Frame {
    let placed = |x: u32, width: u32, height: u32| Placed {
        x,
        y: 0,
        width,
        height,
        quad_extent: None,
        blend: None,
    };
    Frame {
        ink: Some([0.4, 0.4, 0.4, 1.0]),
        tab_selected: Some([0.54, 0.75, 0.79, 1.0]),
        blocks: Some(BlockArt {
            frame: placed(0, 64, 64),
            fill_alpha: 110.0 / 255.0,
            solid_alpha: 1.0,
            cursor: Some(placed(64, 32, 16)),
            arrow: Some(placed(96, 32, 32)),
            fury: true,
        }),
        ..Frame::default()
    }
}

/// Every row label's pen position, in page order: the `Draw::Text` whose
/// text is the entry's label, for each entry on screen.
fn label_pens(menu: &Menu, list: &[Draw]) -> Vec<(usize, (f32, f32))> {
    let page = menu.page();
    let first = menu.scroll();
    page.entries
        .iter()
        .enumerate()
        .skip(first)
        .take(menu.visible_rows())
        .map(|(row, entry)| {
            let pen = list
                .iter()
                .find_map(|draw| match draw {
                    Draw::Text { x, y, text, .. } if text == entry.label() => Some((*x, *y)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("row {row} ({}) is drawn", entry.label()));
            (row, pen)
        })
        .collect()
}

/// The `Part::Row` region for `row`, first in list order.
fn row_region(regions: &[Region], row: usize) -> Region {
    regions
        .iter()
        .copied()
        .find(|region| region.row == row && region.part == Part::Row)
        .unwrap_or_else(|| panic!("row {row} has a region: {regions:?}"))
}

/// The middle of a rect, which is always inside it.
fn centre(rect: [f32; 4]) -> (f32, f32) {
    (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5)
}

fn click_at(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

fn move_to(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        ..Pointer::default()
    }
}

/// The drift guard for a PSP column: every label a text row draws sits
/// inside that row's own region, a little in from its left edge.
#[test]
fn a_text_rows_label_is_inside_its_region() {
    let menu = Menu::new(built_in());
    let list = list(&menu, &no_bindings, None);
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    assert_eq!(regions.len(), menu.page().entries.len().min(visible()));
    for (row, pen) in label_pens(&menu, &list) {
        let region = row_region(&regions, row);
        assert!(
            crate::pointer::contains(region.rect, (pen.0 + 1.0, pen.1 + 1.0)),
            "row {row}: pen {pen:?} outside {region:?}"
        );
    }
    // And no text-row region carries an arrow: there is nothing to aim at.
    assert!(regions.iter().all(|region| region.part == Part::Row));
}

/// The same guard for HD's list rows, plus the arrows: the sprites the
/// row draws are exactly the rects the step regions test against.
#[test]
fn an_hd_list_rows_blocks_and_arrows_are_its_regions() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("display"));
    let skin = hd_skin();
    let frame = hd_frame();
    let list = draw_list(&menu, &skin, &no_bindings, &measure, None, &frame, false).flatten();
    let regions = pointer::regions(&menu, &skin, &frame, &measure);

    for (row, pen) in label_pens(&menu, &list) {
        let region = row_region(&regions, row);
        assert!(
            crate::pointer::contains(region.rect, (pen.0 + 1.0, pen.1 + 1.0)),
            "row {row}: pen {pen:?} outside {region:?}"
        );
    }

    let arrow = hd_frame().blocks.unwrap().arrow.unwrap();
    let arrow_sprites: Vec<[f32; 4]> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, uv, .. }
                if uv[1] as u32 == arrow.y && uv[2].abs() as u32 == arrow.width =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    let arrow_regions: Vec<[f32; 4]> = regions
        .iter()
        .filter(|region| region.part != Part::Row)
        .map(|region| region.rect)
        .collect();
    assert!(
        !arrow_sprites.is_empty(),
        "the display page has a choice row"
    );
    assert_eq!(arrow_sprites, arrow_regions);
}

/// And for both of HD's strips, the one with block art and the measured
/// fallback: each tab's label sits inside its tab.
#[test]
fn a_strip_tabs_label_is_inside_its_region() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let skin = hd_skin();
    for frame in [hd_frame(), Frame::default()] {
        let list = draw_list(&menu, &skin, &no_bindings, &measure, None, &frame, false).flatten();
        let regions = pointer::regions(&menu, &skin, &frame, &measure);
        assert_eq!(regions.len(), menu.page().entries.len());
        for (row, pen) in label_pens(&menu, &list) {
            let region = row_region(&regions, row);
            assert!(
                crate::pointer::contains(region.rect, (pen.0 + 1.0, pen.1 + 1.0)),
                "tab {row}: pen {pen:?} outside {region:?}"
            );
        }
        // Tabs run left to right and do not overlap.
        for pair in regions.windows(2) {
            assert!(
                pair[1].rect[0] >= pair[0].rect[0] + pair[0].rect[2],
                "{pair:?}"
            );
        }
    }
}

#[test]
fn hovering_a_row_selects_it_and_a_still_pointer_does_not() {
    let mut menu = Menu::new(fixture());
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let second = centre(row_region(&regions, 1).rect);
    assert!(menu.pointer(&move_to(second), &regions).is_empty());
    assert_eq!(menu.selected(), 1);

    // The keyboard moves the cursor back up; a pointer still resting over
    // the second row does not drag it down again.
    press(&mut menu, &[Button::Up]);
    assert_eq!(menu.selected(), 0);
    let resting = Pointer {
        at: Some(second),
        ..Pointer::default()
    };
    assert!(menu.pointer(&resting, &regions).is_empty());
    assert_eq!(menu.selected(), 0);
}

#[test]
fn a_click_activates_the_row_under_it_whatever_the_cursor_was_on() {
    let mut menu = Menu::new(fixture());
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    assert_eq!(menu.selected(), 0);
    let events = menu.pointer(&click_at(centre(row_region(&regions, 1).rect)), &regions);
    assert_eq!(events, vec![MenuEvent::Fired(Action::Quit)]);
    assert_eq!(menu.selected(), 1);

    // On a submenu row the click opens the page, as cross would.
    let events = menu.pointer(&click_at(centre(row_region(&regions, 0).rect)), &regions);
    assert!(events.is_empty());
    assert_eq!(menu.page().id, "options");
    assert_eq!(menu.depth(), 2);
}

#[test]
fn a_click_on_an_adjustable_row_steps_it_forward_and_the_arrows_step_either_way() {
    let mut menu = Menu::new(fixture());
    assert!(menu.open("options"));
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let events = menu.pointer(&click_at(centre(row_region(&regions, 0).rect)), &regions);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".into(),
            value: Value::Text("4x".into()),
        }]
    );

    // HD's arrows, on the same fixture drawn with its list blocks.
    let mut menu = Menu::new(fixture());
    assert!(menu.open("options"));
    let regions = pointer::regions(&menu, &hd_skin(), &hd_frame(), &measure);
    let back = regions
        .iter()
        .find(|region| region.row == 0 && region.part == Part::StepBack)
        .expect("a choice row has a back arrow");
    let events = menu.pointer(&click_at(centre(back.rect)), &regions);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".into(),
            value: Value::Text("16x".into()),
        }]
    );
    let forward = regions
        .iter()
        .find(|region| region.row == 0 && region.part == Part::StepForward)
        .expect("a choice row has a forward arrow");
    let events = menu.pointer(&click_at(centre(forward.rect)), &regions);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".into(),
            value: Value::Text("off".into()),
        }]
    );
}

/// An arrow sits inside its row's label block, and it is the arrow that
/// wins - a click on the glyph is a step, not an activate.
#[test]
fn an_arrow_inside_a_block_wins_the_hit() {
    let mut menu = Menu::new(fixture());
    assert!(menu.open("options"));
    let regions = pointer::regions(&menu, &hd_skin(), &hd_frame(), &measure);
    let back = regions
        .iter()
        .find(|region| region.row == 0 && region.part == Part::StepBack)
        .unwrap();
    let label = row_region(&regions, 0);
    let at = centre(back.rect);
    assert!(
        crate::pointer::contains(label.rect, at),
        "{label:?} {back:?}"
    );
    assert_eq!(
        hit(&regions, at).map(|region| region.part),
        Some(Part::StepBack)
    );
}

#[test]
fn the_wheel_walks_the_cursor_and_stops_at_the_ends() {
    let mut menu = Menu::new(fixture());
    assert!(menu.open("options"));
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let wheel = |scroll: i32| Pointer {
        scroll,
        ..Pointer::default()
    };
    assert!(menu.pointer(&wheel(1), &regions).is_empty());
    assert_eq!(menu.selected(), 1);
    menu.pointer(&wheel(5), &regions);
    assert_eq!(menu.selected(), 2, "clamped to the last row, not wrapped");
    menu.pointer(&wheel(-9), &regions);
    assert_eq!(menu.selected(), 0, "clamped to the first row, not wrapped");
}

#[test]
fn the_secondary_button_backs_out_and_closes_from_the_root() {
    let mut menu = Menu::new(fixture());
    // Walked to rather than jumped to, so there is a page to back out to.
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options");
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert!(menu.pointer(&back, &[]).is_empty());
    assert_eq!(menu.page().id, "main");
    assert_eq!(menu.pointer(&back, &[]), vec![MenuEvent::Closed]);
}

#[test]
fn a_click_off_every_row_does_nothing() {
    let mut menu = Menu::new(fixture());
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let events = menu.pointer(&click_at((-50.0, -50.0)), &regions);
    assert!(events.is_empty());
    assert_eq!(menu.selected(), 0);
    assert_eq!(menu.depth(), 1);
    // And a pointer with no position at all is no input.
    let nowhere = Pointer {
        clicked: true,
        moved: true,
        ..Pointer::default()
    };
    assert!(menu.pointer(&nowhere, &regions).is_empty());
}

/// A disabled row can be pointed at and is inert under a click, exactly as
/// it is under cross.
#[test]
fn a_disabled_row_is_selectable_and_inert_under_a_click() {
    let definition = Definition::parse(
        r#"
        version = 1
        root = "main"
        [[page]]
        id = "main"
        title = "MAIN"
        [[page.entry]]
        kind = "toggle"
        label = "VSYNC"
        setting = "vsync"
        [[page.entry]]
        kind = "choice"
        label = "LIMIT"
        setting = "limit"
        values = ["60", "120"]
        disabled_by = { setting = "vsync", value = true }
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("parses");
    let mut menu = Menu::new(definition);
    assert!(menu.seed("vsync", &Value::Flag(true)));
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let events = menu.pointer(&click_at(centre(row_region(&regions, 1).rect)), &regions);
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(menu.selected(), 1);
}

/// Clicking the last visible row of a long page moves the window the way
/// a pad step onto it does, so the row after it comes into view.
#[test]
fn a_click_on_a_long_pages_last_visible_row_scrolls_the_window() {
    let mut menu = Menu::new(built_in());
    let long = menu
        .definition()
        .pages
        .iter()
        .find(|page| page.entries.len() > visible())
        .map(|page| page.id.clone())
        .expect("a page longer than the screen");
    assert!(menu.open(&long));
    menu.set_visible_rows(visible());
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let last_shown = regions.last().unwrap();
    assert_eq!(last_shown.row, visible() - 1);
    menu.pointer(&move_to(centre(last_shown.rect)), &regions);
    assert_eq!(menu.selected(), visible() - 1);
    assert!(menu.scroll() > 0, "the window followed the cursor down");
}

/// The page with the most rows, opened - one that does not fit on screen.
fn long_page() -> (Menu, Vec<Region>, f32) {
    let mut menu = Menu::new(built_in());
    let id = menu
        .definition
        .pages
        .iter()
        .max_by_key(|page| page.entries.len())
        .map(|page| page.id.clone())
        .expect("a page");
    assert!(menu.open(&id));
    assert!(menu.page().entries.len() > visible(), "{id} must overflow");
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let pitch = regions[1].rect[1] - regions[0].rect[1];
    (menu, regions, pitch)
}

fn drag(dy: f32) -> Pointer {
    Pointer {
        drag: (0.0, dy),
        ..Pointer::default()
    }
}

#[test]
fn a_finger_drag_scrolls_the_view_and_keeps_the_cursor_inside_it() {
    let (mut menu, regions, pitch) = long_page();
    let rows = menu.page().entries.len();
    let last = rows - visible();
    assert_eq!((menu.scroll(), menu.selected()), (0, 0));

    assert!(menu.pointer(&drag(-3.0 * pitch), &regions).is_empty());
    assert_eq!(menu.scroll(), 3, "up the screen reveals rows below");
    let inside = |menu: &Menu| {
        (menu.scroll()..menu.scroll() + visible()).contains(&menu.selected())
    };
    assert!(inside(&menu), "{} in window {}", menu.selected(), menu.scroll());
    assert_eq!(menu.selected(), 4, "pushed one row past the window's top lookahead");

    menu.pointer(&drag(1.0 * pitch), &regions);
    assert_eq!(menu.scroll(), 2, "down the screen reveals rows above");
    assert!(inside(&menu));

    menu.pointer(&drag(-100.0 * pitch), &regions);
    assert_eq!(menu.scroll(), last, "stops at the end");
    assert!(inside(&menu));
    menu.pointer(&drag(100.0 * pitch), &regions);
    assert_eq!(menu.scroll(), 0, "stops at the top");
    assert!(inside(&menu));
}

#[test]
fn a_drag_carries_its_fraction_of_a_row_and_never_activates() {
    let (mut menu, regions, pitch) = long_page();
    for expected in [0, 0, 1] {
        let events = menu.pointer(&drag(-0.4 * pitch), &regions);
        assert!(events.is_empty());
        assert_eq!(menu.scroll(), expected);
    }
}

#[test]
fn a_page_that_fits_ignores_a_drag() {
    let mut menu = Menu::new(fixture());
    assert!(menu.open("options"));
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    menu.pointer(&drag(-500.0), &regions);
    assert_eq!((menu.scroll(), menu.selected()), (0, 0));
}
