//! The executable's box behind an HD entry: the strip and the settings rows
//! drawn through [`super::super::block`], and the easing that grows a
//! selected one.
//!
//! The strip and row tests in this directory draw HD's skin against a frame
//! with **no** block art and so pin the measured fallback; these draw against
//! a frame that has it, and pin what `HorizMenu_LayoutBlocks`, `List_Update`
//! and `Block_Render` say - every number cited on
//! `docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md`.

use super::*;

use crate::frontend::Placed;
use crate::menu::block::{Block, BlockArt};

/// Wipeout HD's own skin, in its own 1920x1080 grid.
fn hd_skin() -> Skin {
    Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        22.0,
    )
}

/// A 64x64 nine-patch placed at the sheet's origin, its underline mark and
/// arrow beside it, on the given style.
fn art(fury: bool) -> BlockArt {
    let placed = |x: u32, width: u32, height: u32| Placed {
        x,
        y: 0,
        width,
        height,
        quad_extent: None,
        blend: None,
    };
    BlockArt {
        frame: placed(0, 64, 64),
        fill_alpha: 110.0 / 255.0,
        solid_alpha: 1.0,
        cursor: Some(placed(64, 32, 16)),
        arrow: Some(placed(96, 32, 32)),
        fury,
    }
}

/// The frame a served HD archive resolves: grey and blue fills, plus art.
fn frame(fury: bool) -> Frame {
    Frame {
        ink: Some([0.4, 0.4, 0.4, 1.0]),
        tab_selected: Some([0.54, 0.75, 0.79, 1.0]),
        blocks: Some(art(fury)),
        ..Frame::default()
    }
}

/// Every fill and chamfered fill, as `(rect, colour)`, in draw order.
fn fills(list: &[Draw]) -> Vec<([f32; 4], [f32; 4])> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, color } | Draw::ChamferedFill { rect, color, .. } => {
                Some((*rect, *color))
            }
            _ => None,
        })
        .collect()
}

/// Every sprite, as `(rect, uv)`, in draw order.
pub(super) fn sprites(list: &[Draw]) -> Vec<([f32; 4], [f32; 4])> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, uv, .. } => Some((*rect, *uv)),
            _ => None,
        })
        .collect()
}

fn draw_root(menu: &Menu, fury: bool) -> Vec<Draw> {
    draw_list(
        menu,
        &hd_skin(),
        &no_bindings,
        &measure,
        None,
        &frame(fury),
        false,
    )
    .flatten()
}

/// One block's own draws, alone.
fn one_block(width: f32, landing: bool, fury: bool) -> Vec<Draw> {
    let mut out = Vec::new();
    crate::menu::block::draw(
        &Block {
            x: 100.0,
            y: 200.0,
            width,
            height: 64.0,
            color: [0.4, 0.4, 0.4, 1.0],
            landing,
        },
        &art(fury),
        (1.0, 1.0),
        &mut out,
    );
    out
}

/// A strip tab is `ItemWidth` wide, the selected one `ItemWidth + 70`, and
/// the next starts ten past the last: the executable's own layout, whose
/// five tabs and four gaps span exactly the frame rules' 1600.
#[test]
fn strip_tabs_are_the_executables_widths_and_pitch() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let list = draw_root(&menu, false);
    let strip = hd_skin().strip().expect("HD authors a strip");

    // The first fill of each block is its body, drawn at the fill inset:
    // `x + 2`, width `w - 4`. Collect each block's body by its left edge.
    let bodies: Vec<[f32; 4]> = fills(&list)
        .iter()
        .filter(|(rect, _)| (rect[1] - (strip.y + 12.0)).abs() < 0.001)
        .map(|(rect, _)| *rect)
        .collect();
    // Two passes per block, so every body appears twice.
    assert_eq!(bodies.len(), menu.page().entries.len() * 2, "{bodies:?}");
    let widths: Vec<f32> = bodies.iter().step_by(2).map(|rect| rect[2] + 4.0).collect();
    assert!((widths[0] - 368.0).abs() < 0.001, "selected: {widths:?}");
    for width in &widths[1..] {
        assert!((width - 298.0).abs() < 0.001, "unselected: {widths:?}");
    }
    let lefts: Vec<f32> = bodies.iter().step_by(2).map(|rect| rect[0] - 2.0).collect();
    assert!((lefts[0] - strip.x).abs() < 0.001, "{lefts:?}");
    for pair in lefts.windows(2) {
        let pitch = pair[1] - pair[0];
        let expected = if (pair[0] - strip.x).abs() < 0.001 {
            368.0 + 10.0
        } else {
            298.0 + 10.0
        };
        assert!((pitch - expected).abs() < 0.001, "{lefts:?}");
    }
    let five: f32 = 368.0 + 4.0 * 298.0 + 4.0 * 10.0;
    assert!(
        (five - 1600.0).abs() < f32::EPSILON,
        "the frame rules' own span"
    );
}

/// The label is ten units in from its block, in the text colour, and the
/// underline is the cursor mark at `(x + 10, y + 35)`, 32x16.
#[test]
fn strip_label_and_underline_sit_where_the_layout_puts_them() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    menu.settle();
    let skin = hd_skin();
    let list = draw_root(&menu, false);
    let strip = skin.strip().expect("HD authors a strip");

    let first_label = list
        .iter()
        .find_map(|draw| match draw {
            Draw::Text { x, color, .. } => Some((*x, *color)),
            _ => None,
        })
        .expect("a label");
    assert!(
        (first_label.0 - (strip.x + 10.0)).abs() < 0.001,
        "{first_label:?}"
    );
    assert_eq!(first_label.1, skin.normal());

    let cursor = art(false).cursor.expect("the fixture places one");
    let underline = sprites(&list)
        .into_iter()
        .find(|(_, uv)| uv[0] as u32 == cursor.x && uv[2] as u32 == cursor.width)
        .expect("the underline sprite");
    assert_eq!(
        underline.0,
        [strip.x + 10.0, strip.y + 35.0, 32.0, 16.0],
        "settled: no slide left"
    );
}

/// On the tick a page arrives the underline is a whole `ItemWidth` to the
/// right and sweeps in by five sixths a tick; and it blinks, eight on and
/// nine off.
#[test]
fn the_underline_slides_in_on_arrival_and_blinks() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    let strip = hd_skin().strip().expect("HD authors a strip");
    let cursor = art(false).cursor.expect("placed");
    let underline_x = |list: &[Draw]| {
        sprites(list)
            .into_iter()
            .find(|(_, uv)| uv[0] as u32 == cursor.x && uv[2] as u32 == cursor.width)
            .map(|(rect, _)| rect[0])
    };

    // Tick 0: at the anchor plus ten plus the whole item width.
    let at_arrival = underline_x(&draw_root(&menu, false)).expect("visible on arrival");
    assert!(
        (at_arrival - (strip.x + 10.0 + 298.0)).abs() < 0.001,
        "{at_arrival}"
    );

    // Tick 1: five sixths of the way out.
    menu.tick_focus(1.0 / 6.0);
    let after_one = underline_x(&draw_root(&menu, false)).expect("still visible");
    assert!(
        (after_one - (strip.x + 10.0 + 298.0 * 5.0 / 6.0)).abs() < 0.01,
        "{after_one}"
    );

    // Ticks 8..=16 hide it, 17 shows it again.
    for _ in 1..8 {
        menu.tick_focus(1.0 / 6.0);
    }
    assert!(
        underline_x(&draw_root(&menu, false)).is_none(),
        "tick 8 is off"
    );
    for _ in 8..17 {
        menu.tick_focus(1.0 / 6.0);
    }
    assert!(
        underline_x(&draw_root(&menu, false)).is_some(),
        "tick 17 is on"
    );
}

/// A selected block's width eases toward its target by a sixth of the
/// remaining distance a tick, and snaps when a page arrives.
#[test]
fn a_moved_cursor_grows_the_new_tab_by_a_sixth_a_tick() {
    let mut menu = Menu::new(built_in());
    menu.set_strip_layout(true);
    assert!(
        (menu.focus_of(0) - 1.0).abs() < f32::EPSILON,
        "snapped on arrival"
    );
    assert!(menu.focus_of(1).abs() < f32::EPSILON);

    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 1);
    // The move itself does not snap: the old tab is still wide.
    assert!((menu.focus_of(0) - 1.0).abs() < f32::EPSILON);

    menu.tick_focus(1.0 / 6.0);
    assert!(
        (menu.focus_of(0) - 5.0 / 6.0).abs() < 1e-6,
        "{}",
        menu.focus_of(0)
    );
    assert!(
        (menu.focus_of(1) - 1.0 / 6.0).abs() < 1e-6,
        "{}",
        menu.focus_of(1)
    );

    // Sixty ticks on, both are within a thousandth and snapped.
    for _ in 0..60 {
        menu.tick_focus(1.0 / 6.0);
    }
    assert!(menu.focus_of(0).abs() < f32::EPSILON);
    assert!((menu.focus_of(1) - 1.0).abs() < f32::EPSILON);

    // Opening a page snaps it whatever was in flight.
    press(&mut menu, &[Button::Down]);
    menu.tick_focus(1.0 / 6.0);
    assert!(menu.open("main"), "the root page");
    assert!(menu.ticks_since_arrival() == Some(0));
    for row in 0..menu.page().entries.len() {
        let expected = if row == menu.selected() { 1.0 } else { 0.0 };
        assert!((menu.focus_of(row) - expected).abs() < f32::EPSILON);
    }
}

/// The fill is drawn twice: the first pass at the swatch's alpha on both
/// styles, the second at the swatch's alpha on Fury and opaque on HD.
#[test]
fn the_fill_is_two_passes_and_the_second_is_the_style() {
    for (fury, second) in [(false, 1.0), (true, 110.0 / 255.0)] {
        let list = one_block(298.0, true, fury);
        let alphas: Vec<f32> = fills(&list).iter().map(|(_, color)| color[3]).collect();
        // Body and band per pass: four fills, alphas pass-one, pass-one,
        // pass-two, pass-two.
        assert_eq!(alphas.len(), 4, "{alphas:?}");
        assert!((alphas[0] - 110.0 / 255.0).abs() < 1e-6, "{alphas:?}");
        assert!((alphas[1] - 110.0 / 255.0).abs() < 1e-6, "{alphas:?}");
        assert!((alphas[2] - second).abs() < 1e-6, "fury {fury}: {alphas:?}");
        assert!((alphas[3] - second).abs() < 1e-6, "fury {fury}: {alphas:?}");
    }
}

/// The fill's geometry: a body inset two units from the block, and a band
/// ten tall above it whose right end is a ten-unit cut - seventeen short of
/// the fill on a landing block, one past it on a plain one - and whose left
/// end is a cut on Fury alone.
#[test]
fn the_fill_is_a_body_and_a_cut_band() {
    let expect_band = |landing: bool, fury: bool| {
        let list = one_block(298.0, landing, fury);
        let body = fills(&list)[0].0;
        assert_eq!(body, [102.0, 212.0, 294.0, 50.0], "body, landing {landing}");
        let (band, chamfer) = list
            .iter()
            .find_map(|draw| match draw {
                Draw::ChamferedFill { rect, chamfer, .. } => Some((*rect, *chamfer)),
                _ => None,
            })
            .expect("a band");
        assert!((band[1] - 202.0).abs() < 0.001 && (band[3] - 10.0).abs() < 0.001);
        let right = band[0] + band[2];
        let expected_right = if landing {
            102.0 + 294.0 - 17.0
        } else {
            102.0 + 294.0 + 1.0
        };
        assert!(
            (right - expected_right).abs() < 0.001,
            "landing {landing}: {band:?}"
        );
        let expected_left = if fury { 101.0 } else { 102.0 };
        assert!(
            (band[0] - expected_left).abs() < 0.001,
            "fury {fury}: {band:?}"
        );
        assert_eq!(
            chamfer,
            [if fury { 10.0 } else { 0.0 }, 10.0],
            "fury {fury}"
        );
    };
    expect_band(true, true);
    expect_band(true, false);
    expect_band(false, true);
    expect_band(false, false);
}

/// The border is four 40x18 corner pieces and four edges cut from the
/// nine-patch, the top-left piece and the top-right piece chosen by style
/// and landing, every UV a sixty-fourth turned into the sheet's own rows.
#[test]
fn the_border_is_eight_pieces_of_the_nine_patch() {
    let list = one_block(298.0, true, true);
    let pieces = sprites(&list);
    assert_eq!(pieces.len(), 8, "{pieces:?}");
    let corner = |rect: [f32; 4]| (rect[2] - 40.0).abs() < 0.001 && (rect[3] - 18.0).abs() < 0.001;
    assert!(
        pieces[..4].iter().all(|(rect, _)| corner(*rect)),
        "{pieces:?}"
    );
    // Bottom-left, bottom-right, top-left, top-right, in `Block_Render`'s order.
    assert_eq!(pieces[0].0[..2], [100.0, 200.0 + 64.0 - 18.0]);
    assert_eq!(
        pieces[1].0[..2],
        [100.0 + 298.0 - 40.0, 200.0 + 64.0 - 18.0]
    );
    assert_eq!(pieces[2].0[..2], [100.0, 200.0]);
    assert_eq!(pieces[3].0[..2], [100.0 + 298.0 - 40.0, 200.0]);
    // The Fury top-left is the file's chamfer turned over: file rows 0..18
    // are sheet rows 64..46.
    assert_eq!(pieces[2].1, [0.0, 64.0, 40.0, -18.0]);
    // The landing top-right is the file's stepped corner as the sheet holds
    // it: file rows 64..46 are sheet rows 0..18.
    assert_eq!(pieces[3].1, [24.0, 0.0, 40.0, 18.0]);
    // Edges: two 40-wide verticals between the corners, two 18-tall
    // horizontals between them.
    assert_eq!(pieces[4].0, [100.0, 218.0, 40.0, 64.0 - 36.0]);
    assert_eq!(
        pieces[5].0,
        [100.0 + 298.0 - 40.0, 218.0, 40.0, 64.0 - 36.0]
    );
    assert_eq!(pieces[6].0, [140.0, 200.0, 298.0 - 80.0, 18.0]);
    assert_eq!(
        pieces[7].0,
        [140.0, 200.0 + 64.0 - 18.0, 298.0 - 80.0, 18.0]
    );

    // HD's top-left is the square corner mirrored - the bottom-left piece
    // without its vertical flip - and a plain block's top-right the chamfer
    // mirrored.
    let hd = sprites(&one_block(298.0, false, false));
    assert_eq!(hd[2].1, [64.0, 64.0, -40.0, -18.0]);
    assert_eq!(hd[0].1, [64.0, 46.0, -40.0, 18.0]);
    assert_eq!(hd[3].1, [40.0, 64.0, -40.0, -18.0]);
}

/// A settings row is a 520-wide label block at the list's anchor and a
/// value block ten past it, 280 wide unselected and 340 selected, both the
/// list's height and fifty apart.
#[test]
fn settings_rows_are_a_label_block_and_a_value_block() {
    let mut menu = Menu::new(built_in());
    assert!(
        menu.open("display"),
        "the page whose choice row carries its values inline"
    );
    let skin = hd_skin();
    let list = draw_list(
        &menu,
        &skin,
        &no_bindings,
        &measure,
        None,
        &frame(true),
        false,
    )
    .flatten();
    let anchor = skin.list().expect("HD authors a list anchor");
    assert_eq!((anchor.x, anchor.y, anchor.pitch), (160.0, 170.0, 50.0));

    // Bodies at the fill inset, one per pass: a label block per row, a value
    // block per row with a value.
    let bodies: Vec<[f32; 4]> = fills(&list)
        .iter()
        .filter(|(rect, _)| (rect[3] - (40.0 - 4.0 - 10.0)).abs() < 0.001)
        .map(|(rect, _)| *rect)
        .step_by(2)
        .collect();
    let labels: Vec<&[f32; 4]> = bodies
        .iter()
        .filter(|rect| (rect[0] - (anchor.x + 2.0)).abs() < 0.001)
        .collect();
    // The window, not the page: the display page is longer than the seven
    // rows a `Menu` shows before a caller sizes it.
    let shown = menu.page().entries.len().min(menu.visible_rows());
    assert_eq!(labels.len(), shown, "{bodies:?}");
    for (row, rect) in labels.iter().enumerate() {
        assert!((rect[1] - (anchor.y + 12.0 + row as f32 * 50.0)).abs() < 0.001);
        assert!((rect[2] - (520.0 - 4.0)).abs() < 0.001);
    }
    let values: Vec<&[f32; 4]> = bodies
        .iter()
        .filter(|rect| (rect[0] - (anchor.x + 520.0 + 10.0 + 2.0)).abs() < 0.001)
        .collect();
    let with_values = menu
        .page()
        .entries
        .iter()
        .take(shown)
        .filter(|entry| entry.value().is_some())
        .count();
    assert_eq!(values.len(), with_values, "{bodies:?}");
    // The cursor starts on row 0; a value row further down is unselected.
    for rect in &values {
        assert!((rect[2] - (280.0 - 4.0)).abs() < 0.001, "{rect:?}");
    }

    // Move onto a value row and it grows to 340 once settled.
    let value_row = menu
        .page()
        .entries
        .iter()
        .position(|entry| entry.value().is_some())
        .expect("a value row");
    for _ in 0..value_row {
        press(&mut menu, &[Button::Down]);
    }
    menu.settle();
    let grown = draw_list(
        &menu,
        &skin,
        &no_bindings,
        &measure,
        None,
        &frame(true),
        false,
    )
    .flatten();
    let selected_value = fills(&grown)
        .iter()
        .find(|(rect, _)| {
            (rect[0] - (anchor.x + 532.0)).abs() < 0.001
                && (rect[1] - (anchor.y + 12.0 + value_row as f32 * 50.0)).abs() < 0.001
        })
        .map(|(rect, _)| *rect)
        .expect("the selected row's value body");
    assert!(
        (selected_value[2] - (340.0 - 4.0)).abs() < 0.001,
        "{selected_value:?}"
    );
}

/// A value row carries its two arrows at the label block's right end, the
/// left one mirrored, in the text colour.
#[test]
fn a_value_row_carries_its_arrows() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("display"));
    let skin = hd_skin();
    let list = draw_list(
        &menu,
        &skin,
        &no_bindings,
        &measure,
        None,
        &frame(true),
        false,
    )
    .flatten();
    let arrow = art(true).arrow.expect("placed");
    let arrows: Vec<([f32; 4], [f32; 4])> = sprites(&list)
        .into_iter()
        .filter(|(_, uv)| {
            uv[1] as u32 == arrow.y
                && uv[2].abs() as u32 == arrow.width
                && uv[3] as u32 == arrow.height
                && (uv[0] as u32 == arrow.x || uv[0] as u32 == arrow.x + arrow.width)
        })
        .collect();
    let label_right = 160.0 + 520.0;
    assert_eq!(arrows.len() % 2, 0, "{arrows:?}");
    assert!(!arrows.is_empty(), "the display page has a choice row");
    let (left, right) = (&arrows[0], &arrows[1]);
    assert!(
        (left.0[0] - (label_right - 18.0 - 32.0)).abs() < 0.001,
        "{left:?}"
    );
    assert!(
        left.1[2] < 0.0,
        "the left arrow is the right one mirrored: {left:?}"
    );
    assert!(
        (right.0[0] - (label_right - 30.0)).abs() < 0.001,
        "{right:?}"
    );
    assert!(right.1[2] > 0.0, "{right:?}");
}

/// The rows are budgeted by the grid they are drawn in: the `<aList>`'s own
/// anchor and pitch, not the font's - but only when the frame has the art
/// that draws them that way, since a frame without it falls back to text
/// rows at the font's pitch. HD authors `OffsetY=170` at a pitch of `50`,
/// and against a frame with no chrome of its own that is `(1080 - 170 -
/// note) / 50 = 17` rows. The disc's 33-unit face budgets 15 at its own
/// pitch, and GRAPHICS has 15 entries: the difference between a page that
/// sits still, as the original's does, and one that scrolls.
#[test]
fn the_list_is_budgeted_by_its_own_pitch_when_the_art_decoded() {
    let skin = hd_skin();
    let list = skin.list().expect("HD anchors its rows");
    assert_eq!((list.y, list.pitch), (170.0, 50.0));
    assert_eq!(visible_rows(&skin, &frame(true), false), 17);
    let by_font = visible_rows(&skin, &Frame::default(), false);
    assert!(
        (skin.row_pitch() - list.pitch).abs() > 1.0 && by_font != 17,
        "the font's pitch is a different number, so this is a real difference: {} vs {}",
        skin.row_pitch(),
        list.pitch
    );
}
