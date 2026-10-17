//! The tier swatch's colour, read off the grid's overall points
//! (`GridSummary::medal_tier`, `GridSelection_PopulateTiles` at
//! `0x088de630`): gold only for a perfect grid, silver from halfway between
//! `RequiredPoints` and the maximum, bronze from `RequiredPoints`, nothing
//! below it.

use super::*;

fn tier(name: &str, required: u32, earned: u32, locked: bool) -> GridSummary {
    GridSummary {
        name: name.to_string(),
        cell_count: 8,
        max_points: 24,
        required_points: required,
        gold_medals: 0,
        points_earned: earned,
        locked,
        flyer_name: None,
    }
}

fn body_colours(model: &GridSelection) -> Vec<[f32; 4]> {
    let layers = grid_draw_list(
        model,
        &grid_layout(),
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
        .filter_map(|draw| match draw {
            Draw::Sprite { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

fn medal_colours(model: &GridSelection) -> Vec<[f32; 4]> {
    let medals = [Medal::Gold, Medal::Silver, Medal::Bronze].map(|m| argb_to_rgba(medal_argb(m)));
    body_colours(model)
        .into_iter()
        .filter(|c| medals.contains(c))
        .collect()
}

/// Four tiers of 24 points with 12 required: 24 is gold, 18 (the halfway
/// mark, `(24 - 12) / 2 + 12`) silver, 12 bronze, 11 nothing. The old
/// stand-in tinted all four gold.
#[test]
fn each_tier_swatch_takes_the_colour_its_points_reach() {
    let model = GridSelection::new(vec![
        tier("grid0", 12, 24, false),
        tier("grid1", 12, 18, false),
        tier("grid2", 12, 12, false),
        tier("grid3", 12, 11, false),
    ]);
    assert_eq!(
        medal_colours(&model),
        vec![
            argb_to_rgba(medal_argb(Medal::Gold)),
            argb_to_rgba(medal_argb(Medal::Silver)),
            argb_to_rgba(medal_argb(Medal::Bronze)),
        ]
    );
}

/// A locked tier with some points but fewer than it requires draws its
/// outline alone: the points clear the lock glyph, and reach no swatch.
#[test]
fn a_tier_short_of_its_required_points_draws_neither_lock_nor_swatch() {
    let bare = body_colours(&GridSelection::new(vec![tier("grid0", 12, 0, false)]));
    let locked = body_colours(&GridSelection::new(vec![tier("grid0", 12, 0, true)]));
    assert_eq!(
        locked.len(),
        bare.len() + 1,
        "the lock glyph draws at 0 points"
    );

    let model = GridSelection::new(vec![tier("grid0", 12, 5, true)]);
    assert!(medal_colours(&model).is_empty());
    assert_eq!(body_colours(&model).len(), bare.len());
}
