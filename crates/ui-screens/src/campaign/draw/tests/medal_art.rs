//! `[campaign-medals]`: a medal hex drawn from an atlas frame in place of the tinted swatch,
//! the frame chosen by the tier and the difficulty the medal was earned on.

use super::*;
use crate::campaign::MedalArt;
use crate::campaign::medal_art::Atlas;

const ATLAS: &str = "atlas.gtf";

fn atlas_placed() -> oag_ui::frontend::Placed {
    oag_ui::frontend::Placed {
        x: 100,
        y: 200,
        width: 1024,
        height: 768,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    }
}

fn atlas(per_difficulty: bool) -> MedalArt {
    MedalArt::Atlas(Atlas {
        image: ATLAS.to_string(),
        per_difficulty,
        tier_rows: [0, 1, 2],
    })
}

/// The sprite sheet: the atlas where `has_atlas`, `placed32` for every other image.
fn sheet(has_atlas: bool) -> impl Fn(&str) -> Option<oag_ui::frontend::Placed> {
    move |src| {
        if src == ATLAS {
            has_atlas.then(atlas_placed)
        } else {
            placed32()
        }
    }
}

fn cell_draws(model: &CellSelection, has_atlas: bool) -> Vec<Draw> {
    cell_draw_list(
        model,
        &cell_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &sheet(has_atlas),
        &[],
    )
    .body
}

fn grid_draws(model: &GridSelection, has_atlas: bool) -> Vec<Draw> {
    grid_draw_list(
        model,
        &grid_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &sheet(has_atlas),
        &[],
    )
    .body
}

fn medal_cell(difficulty: Option<Difficulty>, art: MedalArt) -> CellSelection {
    CellSelection::with_medals(vec![race_cell("grid0_0_0", "16_Track")], &|_| {
        Some(Medal::Gold)
    })
    .with_difficulty(&|_| difficulty)
    .with_medal_art(art)
}

/// The atlas crop a draw samples, `[u, v, width, height]` in atlas pixels.
fn atlas_crops(draws: &[Draw]) -> Vec<[f32; 4]> {
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { uv, color, .. } if *color == [1.0; 4] => Some(*uv),
            _ => None,
        })
        .collect()
}

#[test]
fn a_cell_draws_the_block_of_the_difficulty_its_medal_was_earned_on() {
    let at = atlas_placed();
    for (difficulty, block) in [
        (Difficulty::Easy, 0.0),
        (Difficulty::Medium, 183.0),
        (Difficulty::Hard, 366.0),
    ] {
        let draws = cell_draws(&medal_cell(Some(difficulty), atlas(true)), true);
        let wanted = at.sheet_rect([0.0, block, 60.0, 60.0]);
        assert!(
            atlas_crops(&draws).contains(&wanted),
            "{difficulty:?}: no draw samples {wanted:?}"
        );
    }
}

#[test]
fn a_medal_with_no_saved_difficulty_takes_the_middle_block_and_an_atlas_with_one_block_the_first() {
    let at = atlas_placed();
    let unrecorded = cell_draws(&medal_cell(None, atlas(true)), true);
    assert!(atlas_crops(&unrecorded).contains(&at.sheet_rect([0.0, 183.0, 60.0, 60.0])));
    let flat = cell_draws(&medal_cell(Some(Difficulty::Hard), atlas(false)), true);
    assert!(atlas_crops(&flat).contains(&at.sheet_rect([0.0, 0.0, 60.0, 60.0])));
}

#[test]
fn the_frame_fills_the_hexagons_own_rect_and_takes_no_tint() {
    let draws = cell_draws(&medal_cell(Some(Difficulty::Easy), atlas(true)), true);
    let hex = hex_rect(&cell_layout().screen, 0, 0, &|_| placed32()).unwrap();
    assert!(draws.iter().any(|draw| matches!(
        draw,
        Draw::Sprite { rect, color, .. } if *rect == hex && *color == [1.0; 4]
    )));
}

/// An atlas missing from the sheet, or art left at the default, draws exactly what a cell with no
/// art did: the mod being off changes nothing.
#[test]
fn an_atlas_the_sheet_lacks_and_the_default_art_draw_the_swatch() {
    let plain = CellSelection::with_medals(vec![race_cell("grid0_0_0", "16_Track")], &|_| {
        Some(Medal::Gold)
    });
    let swatch = cell_draws(&plain, true);
    assert_eq!(
        cell_draws(&medal_cell(None, MedalArt::Swatch), true),
        swatch
    );
    assert_eq!(cell_draws(&medal_cell(None, atlas(true)), false), swatch);
    assert_ne!(cell_draws(&medal_cell(None, atlas(true)), true), swatch);
}

#[test]
fn a_tier_hex_keeps_the_titles_swatch_whatever_the_art() {
    let model = GridSelection::new(vec![GridSummary {
        name: "grid0".to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: 12,
        gold_medals: 8,
        points_earned: 24,
        locked: false,
        flyer_name: None,
    }])
    .with_medal_art(atlas(true));
    assert!(atlas_crops(&grid_draws(&model, true)).is_empty());
    let plain = GridSelection::new(model.grids().to_vec());
    assert_eq!(grid_draws(&model, true), grid_draws(&plain, true));
}
