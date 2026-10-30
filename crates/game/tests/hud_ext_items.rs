//! A mod's `hud-text` and `hud-icon` in a whole-screen HUD and in a split screen: they
//! sit at their anchor inside the title's safe area, and in a split only in the view of
//! the slot that named them.

use oag_game::hud_overlay::{ViewDraw, safe_area_frame, views_frame};
use oag_hud::ext_draws::{Anchor, Item, Outputs, What};
use oag_hud::{Context, Layout, Placement, Readout, draw_list, ext_draws};
use oag_ui::frontend::Draw;

const GRID: (f32, f32) = (480.0, 272.0);

fn context<'a>(
    layout: &'a Layout,
    strings: &'a oag_ui::language::StringTable,
    sheet: &'a oag_hud::sprite::Sheet,
) -> Context<'a> {
    Context {
        layout,
        strings,
        sheet,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
        default_border: layout.default_border(),
    }
}

fn says(text: &str, anchor: Anchor) -> Outputs {
    Outputs {
        items: vec![Item {
            anchor,
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            color: [1.0; 4],
            what: What::Text(text.into()),
        }],
        ..Outputs::default()
    }
}

fn text_x(frame: &oag_hud::Frame, wanted: &str) -> Vec<f32> {
    frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { x, text, .. } if text == wanted => Some(*x),
            _ => None,
        })
        .collect()
}

#[test]
fn an_edge_anchored_item_stays_inside_the_safe_area() {
    let layout = Layout::from_xml("<Screen/>");
    let strings = oag_ui::language::StringTable::default();
    let sheet = oag_hud::sprite::Sheet::build(&[], &mut Vec::new());
    let cx = context(&layout, &strings, &sheet);
    let readout = Readout {
        ext: says("EDGE", Anchor::Right),
        ..Readout::blank()
    };
    let mut frame = draw_list(&cx, &readout);
    ext_draws::append(&mut frame, &cx, &readout.ext, GRID);
    assert_eq!(
        text_x(&frame, "EDGE"),
        [GRID.0],
        "the right edge of the grid"
    );
    let inside = safe_area_frame(frame, 0.9, GRID);
    let x = text_x(&inside, "EDGE")[0];
    assert!(
        (x - GRID.0 * 0.95).abs() < 1e-3,
        "5 % in from the edge: {x}"
    );
}

#[test]
fn in_a_split_an_item_shows_only_in_the_view_of_the_slot_that_named_it() {
    let layout = Layout::from_xml("<Screen/>");
    let strings = oag_ui::language::StringTable::default();
    let sheet = oag_hud::sprite::Sheet::build(&[], &mut Vec::new());
    let half = |left: bool| Placement {
        scale: 1.0,
        offset: [if left { 0.0 } else { GRID.0 / 2.0 }, 0.0],
        clip: [
            if left { 0.0 } else { GRID.0 / 2.0 },
            0.0,
            GRID.0 / 2.0,
            GRID.1,
        ],
    };
    let view = |slot: usize, ext: Outputs| ViewDraw {
        cx: context(&layout, &strings, &sheet),
        readout: Readout {
            ext,
            ..Readout::blank()
        },
        placement: half(slot == 0),
        sight: None,
        sight_only: false,
        items: Some((GRID.0 / 2.0, GRID.1)),
    };
    let views = [
        view(0, says("ONE", Anchor::TopLeft)),
        view(1, Outputs::default()),
    ];
    let frame = views_frame(&views);
    assert_eq!(text_x(&frame, "ONE").len(), 1, "drawn once");
    assert!(text_x(&frame, "ONE")[0] < GRID.0 / 2.0, "in view 0's half");
    assert!(text_x(&frame, "TWO").is_empty());
    let both = views_frame(&[
        view(0, says("ONE", Anchor::TopLeft)),
        view(1, says("TWO", Anchor::TopLeft)),
    ]);
    assert!(
        text_x(&both, "TWO")[0] >= GRID.0 / 2.0,
        "slot 1's lands in its own half"
    );
}
