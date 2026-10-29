//! The runtime-tint findings from `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
//! "The runtime tint layer" section, split out of `tests.rs` under the
//! 1,000-line rule (`scripts/check-file-size.py`) the same way `menu/tests.rs`
//! already splits its own themes into one file each.

use super::*;

/// `Selector` draws the pulse's cyan endpoint (`0xff33a6b9`) at clock 0, not the XML's own
/// multiply-neutral `i="0xffffffff"` - see `pulse::SelectorPulse` for the
/// decompiled pulse this is one endpoint of.
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
        flyer_name: None,
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
    assert_eq!(selector_color, argb_to_rgba(0xff33_a6b9));
}

/// The `Selector`'s quad writes the glow mask over its whole 42x43 rectangle (alpha test off at the
/// original's draw), at `80 + 150 * t` of the pulse: held at cyan the low end, at white the high one.
/// Dropping the stamp drops the front end's bloom on the cursor.
#[test]
fn the_selector_stamps_the_glow_mask_over_its_whole_quad_by_the_pulse() {
    let layout = grid_layout();
    let mut model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 12,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
        flyer_name: None,
    }]);
    let stamp_at = |model: &GridSelection| {
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
        let selector = layers
            .body
            .iter()
            .find_map(|draw| match draw {
                Draw::Sprite { rect, .. } if rect[2] == 42.0 && rect[3] == 43.0 => Some(*rect),
                _ => None,
            })
            .expect("the Selector draws as a 42x43 sprite");
        let stamps: Vec<_> = layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::GlowStamp { rect, value } => Some((*rect, *value)),
                _ => None,
            })
            .collect();
        assert_eq!(stamps.len(), 1, "one stamp, the Selector's");
        assert_eq!(stamps[0].0, selector, "over the quad, not the ring");
        stamps[0].1
    };
    assert_eq!(stamp_at(&model), 80, "held cyan");
    model.pulse_mut().set_seconds(0.35 + 0.5);
    assert_eq!(stamp_at(&model), 230, "white");
}

/// The card title is authored `pulse="true"` on the disc (`GRID 1`, `SINGLE RACE`), so after its
/// text draw the list carries a glyph-quad stamp stepping down the 220 to 70 ladder; dropping the
/// widget's authored flag or the emission drops the title's halo.
#[test]
fn a_pulsing_title_stamps_its_glyph_quads_after_its_text() {
    let layout = grid_layout();
    let mut model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 12,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
        flyer_name: None,
    }]);
    let stamp_at = |model: &GridSelection| {
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
        let at = layers
            .body
            .iter()
            .position(|draw| matches!(draw, Draw::GlowText { .. }))
            .expect("the pulsing title stamps");
        assert!(
            matches!(
                &layers.body[at - 1],
                Draw::Text { .. } | Draw::FacedText { .. }
            ),
            "the stamp follows the title's own draw"
        );
        let Draw::GlowText { value } = layers.body[at] else {
            unreachable!()
        };
        value
    };
    assert_eq!(stamp_at(&model), 220);
    model.pulse_mut().set_seconds(0.5);
    assert_eq!(stamp_at(&model), 70);
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
            flyer_name: None,
        },
        GridSummary {
            name: "grid1".to_string(),
            cell_count: 8,
            max_points: 24,
            required_points: 16,
            gold_medals: 0,
            points_earned: 0,
            locked: true,
            flyer_name: None,
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
            flyer_name: None,
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

/// The pulse moves `Selector`'s drawn colour with the model's clock; dropping
/// the wiring in either draw list leaves the colour fixed.
#[test]
fn the_selector_colour_moves_with_the_pulse_clock() {
    let layout = grid_layout();
    let grid = GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 12,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
        flyer_name: None,
    };
    let colour_at = |seconds: f32| {
        let mut model = GridSelection::new(vec![grid.clone()]);
        model.pulse_mut().set_seconds(seconds);
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
        layers
            .body
            .iter()
            .find_map(|draw| match draw {
                Draw::Sprite { rect, color, .. } if rect[2] == 42.0 && rect[3] == 43.0 => {
                    Some(*color)
                }
                _ => None,
            })
            .expect("the Selector draws as a 42x43 sprite")
    };
    assert_eq!(colour_at(0.0), argb_to_rgba(0xff33_a6b9));
    assert_eq!(colour_at(0.85), argb_to_rgba(0xffff_ffff));
    assert_eq!(colour_at(1.35), colour_at(0.0), "period is one second");
}

/// The original puts `Selector`'s top-left 5 px left of and 7 px above the selected hex's own
/// top-left (`GridController_UpdateSelectorPulse`'s `5.0` and `7.0`; GE dump: quad (60,138) over
/// the hex at (65,145)). Centring the 42x43 canvas instead puts it 1.5 px low.
#[test]
fn the_selector_sits_five_left_and_seven_above_its_hex() {
    let layout = grid_layout();
    let model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 12,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
        flyer_name: None,
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
    let selector = layers
        .body
        .iter()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 42.0 && rect[3] == 43.0 => Some(*rect),
            _ => None,
        })
        .expect("the Selector draws as a 42x43 sprite");
    let hex_there = layers.body.iter().any(|draw| match draw {
        Draw::Sprite { rect, .. } => {
            rect[2] == 32.0
                && rect[3] == 32.0
                && rect[0] == selector[0] + 5.0
                && rect[1] == selector[1] + 7.0
        }
        _ => false,
    });
    assert!(hex_there, "a 32x32 hex at the Selector's origin + (5,7)");
}
