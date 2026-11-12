//! A campaign's own medal tiers (`[[medal-tier]]`): a tier above gold on Cell and Grid
//! Selection. The standard list draws as it always did (`tier_medal.rs`, `medal_art.rs`).

use oag_tables::race_campaign::MedalTier;

use super::*;
use crate::campaign::MedalArt;
use crate::campaign::medal_art::Atlas;

const PLATINUM: u32 = 0xffe5_e4e2;

fn platinum_tiers() -> MedalTiers {
    let tier = |name: &str, points, argb: u32, id: Option<&str>| MedalTier {
        name: name.to_string(),
        points,
        argb,
        string_id: id.map(str::to_string),
    };
    MedalTiers::new(vec![
        tier("platinum", 4, PLATINUM, Some("PLATINUM_WORD")),
        tier("gold", 3, 0xfffa_eb38, Some("IG_HUD_GOLD")),
        tier("silver", 2, 0xffda_e3e4, Some("IG_HUD_SILVER")),
        tier("bronze", 1, 0xffdf_942f, Some("IG_HUD_BRONZE")),
    ])
    .expect("a valid list")
}

fn cell_with(medal: Option<Medal>) -> CellSelection {
    CellSelection::with_medals(vec![race_cell("grid0_0_0", "16_Track")], &|_| medal)
        .with_difficulty(&|_| Some(Difficulty::Easy))
        .with_tiers(platinum_tiers())
}

fn sprite_colours(draws: &[Draw]) -> Vec<[f32; 4]> {
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

fn body(model: &CellSelection) -> Vec<Draw> {
    cell_draw_list(
        model,
        &cell_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    )
    .body
}

#[test]
fn a_tier_above_gold_fills_its_hexagon_with_its_own_colour() {
    let platinum = Medal::from_index(0).unwrap();
    let colours = sprite_colours(&body(&cell_with(Some(platinum))));
    assert!(colours.contains(&argb_to_rgba(PLATINUM)), "{colours:?}");
    let gold = Medal::from_index(1).unwrap();
    let colours = sprite_colours(&body(&cell_with(Some(gold))));
    assert!(
        colours.contains(&argb_to_rgba(MedalTiers::standard().argb(Medal::Gold))),
        "gold is the second tier here, and keeps gold's colour"
    );
    assert!(!colours.contains(&argb_to_rgba(PLATINUM)));
}

fn lines(model: &CellSelection) -> Vec<String> {
    let layers = cell_draw_list(
        model,
        &cell_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed32(),
        &[],
    );
    texts(&layers).into_iter().cloned().collect()
}

#[test]
fn the_points_and_the_best_line_come_from_the_tier_list() {
    let platinum = Medal::from_index(0).unwrap();
    let gold = Medal::from_index(1).unwrap();
    let lines_gold = lines(&cell_with(Some(gold)));
    assert!(lines_gold.iter().any(|t| t == "3/4"), "{lines_gold:?}");
    assert!(
        lines_gold.iter().any(|t| t.starts_with("IG_HUD_GOLD")),
        "{lines_gold:?}"
    );
    let lines_platinum = lines(&cell_with(Some(platinum)));
    assert!(
        lines_platinum.iter().any(|t| t == "4/4"),
        "{lines_platinum:?}"
    );
    assert!(
        lines_platinum
            .iter()
            .any(|t| t.starts_with("PLATINUM_WORD")),
        "{lines_platinum:?}"
    );
}

#[test]
fn a_grid_counts_gold_or_better_as_a_gold_medal() {
    let tiers = platinum_tiers();
    let grid = |names: &[&str]| oag_tables::race_campaign::Grid {
        name: "grid0".to_string(),
        required_points: 4,
        locked: false,
        group: None,
        unlock_grid: None,
        cells: names.iter().map(|n| race_cell(n, "16_Track")).collect(),
        campaign: None,
        title_color: None,
        text_color: None,
        flyer_name: None,
        billboard_name: None,
    };
    let grid = grid(&["grid0_0_0", "grid0_1_0", "grid0_2_0", "grid0_3_0"]);
    let summary = GridSummary::from_grid_with_medals(&grid, &tiers, &|name| match name {
        "grid0_0_0" => tiers.find("platinum"),
        "grid0_1_0" => tiers.find("gold"),
        "grid0_2_0" => tiers.find("silver"),
        _ => None,
    });
    assert_eq!(summary.gold_medals, 2, "platinum and gold");
    assert_eq!(summary.points_earned, 4 + 3 + 2);
    assert_eq!(summary.max_points, 16, "four cells at the best tier's 4");
}

#[test]
fn a_tier_the_atlas_has_no_row_for_draws_its_colour_and_the_others_their_frame() {
    let art = MedalArt::Atlas(Atlas {
        image: "atlas.gtf".to_string(),
        per_difficulty: false,
        tier_rows: [0, 1, 2],
    });
    let tiers = platinum_tiers();
    assert!(
        art.frame(&tiers, tiers.find("platinum").unwrap(), Difficulty::Easy)
            .is_none()
    );
    assert!(
        art.frame(&tiers, tiers.find("gold").unwrap(), Difficulty::Easy)
            .is_some()
    );
    assert!(
        art.frame(MedalTiers::standard(), Medal::Gold, Difficulty::Easy)
            .is_some()
    );
}

/// A time-trial cell: its targets in tier order, best first, `0` for a tier it does not award.
fn timed(targets: [i64; 4]) -> CellSelection {
    let mut cell = race_cell("grid0_0_0", "16_Track");
    cell.mode = oag_tables::race_campaign::Mode::TimeTrial;
    CellSelection::new(vec![cell])
        .with_tiers(platinum_tiers())
        .with_tier_targets(&|_| Some(targets.to_vec()))
}

#[test]
fn the_target_rows_list_the_three_best_tiers_the_cell_awards() {
    let rows = |model: &CellSelection| -> Vec<(usize, i64)> {
        model
            .target_rows()
            .unwrap()
            .into_iter()
            .map(|(m, t)| (m.index(), t))
            .collect()
    };
    assert_eq!(
        rows(&timed([13_800, 15_000, 16_500, 18_000])),
        [(0, 13_800), (1, 15_000), (2, 16_500)],
        "platinum, gold, silver; bronze has no row"
    );
    assert_eq!(
        rows(&timed([0, 15_000, 16_500, 18_000])),
        [(1, 15_000), (2, 16_500), (3, 18_000)],
        "no platinum on this event: gold, silver, bronze"
    );
    let plain = CellSelection::new(vec![race_cell("grid0_0_0", "16_Track")]);
    assert_eq!(plain.target_rows(), None, "the title's own rows");
}

#[test]
fn a_target_row_is_tinted_with_its_tiers_colour_and_a_row_with_no_tier_is_not_drawn() {
    let lines = |model: &CellSelection| {
        let layers = cell_draw_list(
            model,
            &cell_layout(),
            &skin(),
            &Frame::default(),
            &strings(),
            None,
            false,
            &|_| placed32(),
            &[],
        );
        let colours = sprite_colours(&layers.body);
        (
            texts(&layers).into_iter().cloned().collect::<Vec<_>>(),
            colours,
        )
    };
    let (texts, colours) = lines(&timed([13_800, 15_000, 16_500, 18_000]));
    assert!(colours.contains(&argb_to_rgba(PLATINUM)), "{colours:?}");
    assert!(
        texts.iter().any(|t| t == "2:18.00" || t.contains("2:18")),
        "{texts:?}"
    );
    let (texts, _) = lines(&timed([0, 15_000, 0, 0]));
    assert_eq!(
        texts
            .iter()
            .filter(|t| t.as_str() == "Target")
            .count(),
        1,
        "one awarded tier, one target title: {texts:?}"
    );
}
