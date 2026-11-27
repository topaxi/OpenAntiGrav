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
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
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
            marker: None,
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
fn on_a_page_that_fits_the_wheel_walks_the_cursor_and_stops_at_the_ends() {
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
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
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

/// A page of `rows` switches, each its own setting, longer than any window.
fn long_switches(rows: usize) -> Menu {
    let mut toml = String::from(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        title = "MAIN"
        "#,
    );
    for row in 0..rows {
        toml.push_str(&format!(
            "[[page.entry]]\nkind = \"toggle\"\nlabel = \"ROW {row}\"\nsetting = \"test.row{row}\"\n"
        ));
    }
    let definition = Definition::parse(&toml, &crate::language::StringTable::default())
        .expect("the page must parse");
    Menu::new(definition)
}

/// The two idioms a long page is drawn in: Pulse's text rows and HD's list of
/// boxes, each with the window its own pitch gives.
fn idioms() -> [(&'static str, Skin, Frame); 2] {
    [
        ("text rows", skin(), Frame::default()),
        ("boxed list", hd_skin(), hd_frame()),
    ]
}

/// A long page in one idiom, and what a pointer sees of it right now. The
/// regions are asked for again after every gesture, as the game does each tick.
struct Long {
    menu: Menu,
    skin: Skin,
    frame: Frame,
}

impl Long {
    fn new(skin: Skin, frame: Frame) -> Self {
        let mut menu = long_switches(40);
        menu.set_visible_rows(visible_rows(&skin, &frame, false));
        Self { menu, skin, frame }
    }

    fn regions(&self) -> Vec<Region> {
        pointer::regions(&self.menu, &self.skin, &self.frame, &measure)
    }

    /// The middle of the label of the first or last row on screen.
    fn edge(&self, last: bool) -> (usize, (f32, f32)) {
        let rows: Vec<Region> = self
            .regions()
            .into_iter()
            .filter(|region| region.part == Part::Row)
            .collect();
        let region = rows[if last { rows.len() - 1 } else { 0 }];
        (region.row, centre(region.rect))
    }

    fn feed(&mut self, pointer: &Pointer) -> Vec<MenuEvent> {
        let regions = self.regions();
        self.menu.pointer(pointer, &regions)
    }

    fn at(&self) -> (usize, usize) {
        (self.menu.selected(), self.menu.scroll())
    }
}

fn flipped(events: &[MenuEvent]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            MenuEvent::Changed { setting, .. } => Some(setting.as_str()),
            _ => None,
        })
        .collect()
}

/// Hovering the row at either edge of a long page moves the cursor there and
/// nothing else: a second small move over the same point hits the same row.
#[test]
fn hovering_an_edge_row_never_scrolls_the_list() {
    for (idiom, skin, frame) in idioms() {
        let mut long = Long::new(skin, frame);
        let (bottom, at) = long.edge(true);
        assert!(bottom + 1 < 40, "{idiom}: the page overflows");
        long.feed(&move_to(at));
        assert_eq!(long.at(), (bottom, 0), "{idiom}: bottom edge");
        long.feed(&move_to((at.0 + 1.0, at.1)));
        assert_eq!(long.at(), (bottom, 0), "{idiom}: and again");

        long.feed(&wheel(4));
        assert_eq!(long.menu.scroll(), 4, "{idiom}: the wheel still scrolls");
        let (top, at) = long.edge(false);
        assert_eq!(top, 4, "{idiom}");
        long.feed(&move_to(at));
        assert_eq!(long.at(), (4, 4), "{idiom}: top edge");
    }
}

/// The report this answers: the click after a hover on an edge row must act on
/// that row, and so must the one after it.
#[test]
fn a_click_after_a_hover_on_an_edge_row_confirms_that_row() {
    for (idiom, skin, frame) in idioms() {
        let mut long = Long::new(skin, frame);
        let (bottom, at) = long.edge(true);
        let setting = format!("test.row{bottom}");
        long.feed(&move_to(at));
        let click = Pointer {
            moved: false,
            ..click_at(at)
        };
        for press in 1..=2 {
            let events = long.feed(&click);
            assert_eq!(flipped(&events), [setting.as_str()], "{idiom}: {press}");
            assert_eq!(long.at(), (bottom, 0), "{idiom}: {press}");
        }
    }
}

/// A finger's tap is a move and a click in one tick, and leaves the list too.
#[test]
fn a_tap_on_an_edge_row_changes_it_and_leaves_the_list() {
    for (idiom, skin, frame) in idioms() {
        let mut long = Long::new(skin, frame);
        let (bottom, at) = long.edge(true);
        let setting = format!("test.row{bottom}");
        for tap in 1..=2 {
            let events = long.feed(&click_at(at));
            assert_eq!(flipped(&events), [setting.as_str()], "{idiom}: {tap}");
            assert_eq!(long.at(), (bottom, 0), "{idiom}: {tap}");
        }
    }
}

/// The pad's own rule comes back the moment the pad moves: one row of
/// lookahead past the cursor, at whichever end it is heading for. Changing the
/// value of the row it is already on moves nothing.
#[test]
fn a_pad_move_from_a_hovered_edge_row_scrolls_again() {
    let mut long = Long::new(skin(), Frame::default());
    let shown = long.menu.visible_rows();
    let (bottom, at) = long.edge(true);
    long.feed(&move_to(at));
    press(&mut long.menu, &[Button::Right]);
    assert_eq!(long.at(), (bottom, 0), "a value step is no move");
    press(&mut long.menu, &[Button::Down]);
    assert_eq!(long.menu.selected(), bottom + 1);
    assert_eq!(
        long.menu.scroll(),
        bottom + 3 - shown,
        "the row after the cursor is on screen"
    );

    long.feed(&wheel(4));
    let (top, at) = long.edge(false);
    long.feed(&move_to(at));
    press(&mut long.menu, &[Button::Up]);
    assert_eq!(long.at(), (top - 1, top - 2), "the row before it too");
}

/// A row coming or going above a hovered edge row carries the pointer's claim
/// with it: the window follows the row and asks for no lookahead of its own.
#[test]
fn rows_coming_and_going_keep_a_hovered_edge_row_on_its_edge() {
    let mut long = Long::new(skin(), Frame::default());
    long.menu.definition.pages[0].entries[0] = Entry::Run {
        label: "QUIT".into(),
        action: Action::Quit,
        inputs: None,
    };
    long.menu.offer(Action::Quit, false);
    let (bottom, at) = long.edge(true);
    long.feed(&move_to(at));
    long.menu.offer(Action::Quit, true);
    assert_eq!(
        long.at(),
        (bottom + 1, 1),
        "inserted above: the same row stays last on screen"
    );

    long.feed(&wheel(4));
    let (top, at) = long.edge(false);
    long.feed(&move_to(at));
    long.menu.offer(Action::Quit, false);
    assert_eq!(long.at(), (top - 1, top - 1), "removed above: stays first");
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
    let inside =
        |menu: &Menu| (menu.scroll()..menu.scroll() + visible()).contains(&menu.selected());
    assert!(
        inside(&menu),
        "{} in window {}",
        menu.selected(),
        menu.scroll()
    );
    assert_eq!(
        menu.selected(),
        4,
        "pushed one row past the window's top lookahead"
    );

    menu.pointer(&drag(1.0 * pitch), &regions);
    assert_eq!(menu.scroll(), 2, "down the screen reveals rows above");
    assert!(inside(&menu));

    menu.pointer(&drag(-100.0 * pitch), &regions);
    assert_eq!(menu.scroll(), last, "stops at the end");
    assert!(inside(&menu));
    lift_and_settle(&mut menu, &regions);
    assert_eq!(menu.scroll(), last, "and rests there");
    menu.pointer(&drag(100.0 * pitch), &regions);
    assert_eq!(menu.scroll(), 0, "stops at the top");
    assert!(inside(&menu));
}

fn lift_and_settle(menu: &mut Menu, regions: &[Region]) {
    menu.pointer(
        &Pointer {
            released: true,
            ..Pointer::default()
        },
        regions,
    );
    for _ in 0..120 {
        menu.tick_scroll(1.0 / 60.0);
    }
}

#[test]
fn a_flick_coasts_the_list_on_and_a_touch_down_catches_it() {
    let (mut menu, regions, pitch) = long_page();
    let last = menu.page().entries.len() - visible();
    // Half a row a tick up the screen: 30 rows a second, past the cap.
    let mut gesture = Pointer {
        pressed: true,
        ..drag(-0.5 * pitch)
    };
    for _ in 0..3 {
        menu.pointer(&gesture, &regions);
        menu.tick_scroll(1.0 / 60.0);
        gesture.pressed = false;
    }
    let lifted = menu.scroll();
    assert!(lifted <= 2, "{lifted}");
    menu.pointer(
        &Pointer {
            released: true,
            ..Pointer::default()
        },
        &regions,
    );
    for _ in 0..10 {
        menu.tick_scroll(1.0 / 60.0);
    }
    let coasting = menu.scroll();
    assert!(coasting > lifted, "kept going: {lifted} then {coasting}");
    // A tap while it coasts stops it and activates nothing.
    let row = regions.iter().find(|r| r.part == Part::Row).unwrap();
    let tap = Pointer {
        at: Some((row.rect[0] + 1.0, row.rect[1] + 1.0)),
        moved: true,
        clicked: true,
        pressed: true,
        released: true,
        ..Pointer::default()
    };
    let cursor = menu.selected();
    assert!(menu.pointer(&tap, &regions).is_empty());
    assert_eq!(menu.selected(), cursor, "the tap selected nothing");
    for _ in 0..120 {
        menu.tick_scroll(1.0 / 60.0);
    }
    let rested = menu.scroll();
    assert!(rested <= coasting + 1 && rested < last, "{rested}");
}

#[test]
fn a_pad_move_stops_a_coasting_list() {
    let (mut menu, regions, pitch) = long_page();
    for _ in 0..3 {
        menu.pointer(&drag(-0.5 * pitch), &regions);
        menu.tick_scroll(1.0 / 60.0);
    }
    menu.pointer(
        &Pointer {
            released: true,
            ..Pointer::default()
        },
        &regions,
    );
    menu.tick_scroll(1.0 / 60.0);
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Down as u32);
    menu.update(&mut input);
    let (scroll, cursor) = (menu.scroll(), menu.selected());
    for _ in 0..120 {
        menu.tick_scroll(1.0 / 60.0);
    }
    assert_eq!((menu.scroll(), menu.selected()), (scroll, cursor));
}

#[test]
fn a_drag_shows_the_row_nearest_the_finger_and_never_activates() {
    let (mut menu, regions, pitch) = long_page();
    for expected in [0, 1, 1] {
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

fn wheel(scroll: i32) -> Pointer {
    Pointer {
        scroll,
        ..Pointer::default()
    }
}

/// The maintainer's report: on a long page the wheel moved the selection
/// down a list that stayed put. It moves the list now, as a drag does.
#[test]
fn on_a_long_page_the_wheel_scrolls_the_view_and_keeps_the_cursor_inside_it() {
    let (mut menu, regions, _) = long_page();
    let last = menu.page().entries.len() - visible();
    let inside =
        |menu: &Menu| (menu.scroll()..menu.scroll() + visible()).contains(&menu.selected());

    assert!(menu.pointer(&wheel(3), &regions).is_empty());
    assert_eq!(menu.scroll(), 3, "a detent down reveals a row below");
    assert_eq!(menu.selected(), 4, "pushed inside, not walked by the wheel");
    assert!(inside(&menu));

    menu.pointer(&wheel(-1), &regions);
    assert_eq!(menu.scroll(), 2);
    assert_eq!(menu.selected(), 4, "still inside, so the cursor stays put");

    menu.pointer(&wheel(100), &regions);
    assert_eq!(menu.scroll(), last, "stops at the end");
    assert!(inside(&menu));
    menu.pointer(&wheel(-100), &regions);
    assert_eq!(menu.scroll(), 0, "stops at the top");
    assert!(inside(&menu));
}

/// Touch mode makes an HD list row 50% taller and steps the next row by the
/// same, and the regions a finger lands on are those taller rects.
#[test]
fn touch_mode_makes_an_hd_list_row_half_again_as_tall() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("display"));
    let frame = hd_frame();
    let mut skin = hd_skin();
    let rects = |skin: &Skin| {
        let regions = pointer::regions(&menu, skin, &frame, &measure);
        (row_region(&regions, 0).rect, row_region(&regions, 1).rect)
    };
    let (first, second) = rects(&skin);
    skin.set_touch_rows(true);
    let (tall_first, tall_second) = rects(&skin);
    assert!((tall_first[3] - first[3] * 1.5).abs() < 1e-3);
    assert!(((tall_second[1] - tall_first[1]) - (second[1] - first[1]) * 1.5).abs() < 1e-3);
    skin.set_touch_rows(false);
    assert_eq!(rects(&skin), (first, second), "a pad brings the size back");
}
