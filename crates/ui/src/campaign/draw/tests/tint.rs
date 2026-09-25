//! The runtime-tint findings from `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
//! "The runtime tint layer" section, split out of `tests.rs` under the
//! 1,000-line rule (`scripts/check-file-size.py`) the same way `menu/tests.rs`
//! already splits its own themes into one file each.

use super::*;

/// `Selector` draws `SELECTOR_TINT` (`0xff33a6b9`), not the XML's own
/// multiply-neutral `i="0xffffffff"` - see that constant's own doc for the
/// decompiled pulse this is one static endpoint of.
/// `data/reference/psp-campaign-screens/grid-selection-page1-grid0-unlocked.png`
/// is a real capture of it.
#[test]
fn the_selector_draws_the_measured_cyan_tint_not_the_xmls_own_white() {
    let layout = grid_layout();
    let model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 12,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
    }]);
    let layers = grid_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    );
    let selector_color = layers
        .body
        .iter()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, color, .. } if rect[2] == 42.0 && rect[3] == 43.0 => Some(*color),
            _ => None,
        })
        .expect("the Selector draws as a 42x43 sprite");
    assert_eq!(selector_color, argb_to_rgba(SELECTOR_TINT));
}

/// `Outline_x_y` draws `TIER_OUTLINE_TINT` on `Grid Selection` - both the
/// unlocked hex (`Outline_0_0`) and the locked one behind it
/// (`Outline_1_0`) get the same dim cyan-teal tile colour
/// `GridSelection_PopulateTiles` sets unconditionally; only the lock glyph
/// itself distinguishes locked from unlocked. See `TIER_OUTLINE_TINT`'s own
/// doc.
#[test]
fn outline_hexes_draw_the_measured_dim_cyan_tint() {
    let layout = grid_layout();
    let model = GridSelection::new(vec![
        GridSummary {
            name: "grid0".to_string(),
            cell_count: 8,
            max_points: 24,
            required_points: 12,
            gold_medals: 0,
            points_earned: 0,
            locked: false,
        },
        GridSummary {
            name: "grid1".to_string(),
            cell_count: 8,
            max_points: 24,
            required_points: 16,
            gold_medals: 0,
            points_earned: 0,
            locked: true,
        },
    ]);
    let layers = grid_draw_list(
        &model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    );
    let outline_colors: Vec<[f32; 4]> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, color, .. } if rect[2] == 32.0 && rect[3] == 32.0 => Some(*color),
            _ => None,
        })
        .collect();
    assert_eq!(outline_colors.len(), 2, "{outline_colors:?}");
    for color in outline_colors {
        assert_eq!(color, argb_to_rgba(TIER_OUTLINE_TINT));
    }
}

/// `up arrow`/`down arrow` grey out (`ARROW_DISABLED_TINT`) at the page
/// boundary they can no longer move past - `FUN_088de9bc`'s own per-frame
/// tint, measured off the decompile. Five grids over `GRIDS_PER_PAGE` (4)
/// makes two pages, so page 0 dims the up arrow and page 1 dims the down
/// arrow.
#[test]
fn the_page_arrows_grey_out_at_their_own_boundary() {
    let layout = grid_layout();
    let grids = (0..5)
        .map(|i| GridSummary {
            name: format!("grid{i}"),
            cell_count: 8,
            max_points: 24,
            required_points: 12,
            gold_medals: 0,
            points_earned: 0,
            locked: false,
        })
        .collect::<Vec<_>>();

    let arrow_colors = |model: &GridSelection| -> (Option<[f32; 4]>, Option<[f32; 4]>) {
        let layers = grid_draw_list(
            model,
            &layout,
            &skin(),
            &Frame::default(),
            &strings(),
            None,
            false,
            &|_| placed32(),
            &[],
        );
        let up = layers.body.iter().find_map(|draw| match draw {
            Draw::Sprite { rect, color, .. }
                if rect[2] == 17.0 && rect[3] == 14.0 && rect[1] == 84.0 =>
            {
                Some(*color)
            }
            _ => None,
        });
        let down = layers.body.iter().find_map(|draw| match draw {
            Draw::Sprite { rect, color, .. }
                if rect[2] == 17.0 && rect[3] == 14.0 && rect[1] == 167.0 =>
            {
                Some(*color)
            }
            _ => None,
        });
        (up, down)
    };

    let mut first_page = GridSelection::new(grids.clone());
    let (up, down) = arrow_colors(&first_page);
    assert_eq!(
        up.expect("up arrow draws"),
        argb_to_rgba(ARROW_DISABLED_TINT)
    );
    assert_eq!(down.expect("down arrow draws"), argb_to_rgba(0xffff_ffff));

    first_page.set_index(4); // second page (grid4, alone on page 1)
    let (up, down) = arrow_colors(&first_page);
    assert_eq!(up.expect("up arrow draws"), argb_to_rgba(0xffff_ffff));
    assert_eq!(
        down.expect("down arrow draws"),
        argb_to_rgba(ARROW_DISABLED_TINT)
    );
}
