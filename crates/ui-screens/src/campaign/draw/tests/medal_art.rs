//! `[campaign-medals]`: a medal hex drawn from an atlas frame in place of the tinted swatch,
//! the frame chosen by the tier and the difficulty the medal was earned on.

use super::tiers::platinum_tiers;
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
    MedalArt::Atlas(Atlas::of_title(
        ATLAS.to_string(),
        per_difficulty,
        [0, 1, 2],
    ))
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

/// A mod's atlas of four 64 px rows (platinum, gold, silver, bronze), three difficulty blocks.
fn mod_atlas(per_difficulty: bool) -> MedalArt {
    let blocks = if per_difficulty { 3 } else { 1 };
    MedalArt::Atlas(
        Atlas::of_mod(
            ATLAS.to_string(),
            per_difficulty,
            ["platinum", "gold", "silver", "bronze"]
                .map(String::from)
                .to_vec(),
            (64, 4 * 64 * blocks),
        )
        .expect("an atlas of whole rows"),
    )
}

fn platinum_cell(medal: &str, difficulty: Difficulty, art: MedalArt) -> CellSelection {
    let tiers = platinum_tiers();
    let earned = tiers.find(medal).expect("a tier of the campaign");
    CellSelection::with_medals(vec![race_cell("grid0_0_0", "16_Track")], &|_| Some(earned))
        .with_difficulty(&|_| Some(difficulty))
        .with_tiers(tiers)
        .with_medal_art(art)
}

/// A mod's own atlas gives a tier the disc never had (platinum) its row, by the tier's name and
/// not its position, and the difficulty selects the block.
#[test]
fn a_mod_atlas_draws_the_row_its_tier_names_in_the_block_of_the_difficulty() {
    let at = atlas_placed();
    for (medal, difficulty, v) in [
        ("platinum", Difficulty::Easy, 0.0),
        ("gold", Difficulty::Easy, 64.0),
        ("bronze", Difficulty::Easy, 192.0),
        ("platinum", Difficulty::Hard, 2.0 * 256.0),
        ("silver", Difficulty::Medium, 256.0 + 128.0),
    ] {
        let draws = cell_draws(&platinum_cell(medal, difficulty, mod_atlas(true)), true);
        let wanted = at.sheet_rect([0.0, v, 64.0, 64.0]);
        assert!(
            atlas_crops(&draws).contains(&wanted),
            "{medal} on {difficulty:?}: no draw samples {wanted:?}, got {:?}",
            atlas_crops(&draws)
        );
    }
}

/// A campaign tier the mod's rows do not name keeps its colour swatch, and a disc atlas has no
/// row for a tier of a campaign's own.
#[test]
fn a_tier_no_row_names_draws_its_swatch() {
    let tiers = platinum_tiers();
    let gold_only = MedalArt::Atlas(
        Atlas::of_mod(ATLAS.to_string(), false, vec!["gold".to_string()], (64, 64)).unwrap(),
    );
    let platinum = tiers.find("platinum").unwrap();
    assert_eq!(gold_only.frame(&tiers, platinum, Difficulty::Easy), None);
    assert_eq!(
        mod_atlas(false).frame(&tiers, platinum, Difficulty::Hard),
        Some([0.0, 0.0, 64.0, 64.0]),
        "a single-block atlas ignores the difficulty"
    );
    assert_eq!(
        cell_draws(
            &platinum_cell("platinum", Difficulty::Easy, gold_only),
            true
        ),
        cell_draws(
            &platinum_cell("platinum", Difficulty::Easy, MedalArt::Swatch),
            true
        )
    );
    assert_eq!(
        atlas(true).frame(&tiers, platinum, Difficulty::Easy),
        None,
        "the disc's rows are gold, silver and bronze"
    );
}

#[test]
fn an_atlas_that_is_not_whole_rows_is_refused_by_name() {
    let rows = || vec!["gold".to_string(), "silver".to_string()];
    let error = Atlas::of_mod("a".into(), true, rows(), (64, 100)).unwrap_err();
    assert!(error.contains("6 equal rows"), "{error}");
    assert!(Atlas::of_mod("a".into(), false, Vec::new(), (64, 64)).is_err());
    assert!(Atlas::of_mod("a".into(), true, rows(), (64, 384)).is_ok());
}
