//! `Cell Selection`'s hex field and unlock box, drawn from a small screen.

use super::*;
use oag_tables::race_campaign::Mode;
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Skin};
use oag_ui::screen::{Image, Screen};

fn image(name: &str, src: &str) -> Image {
    Image {
        name: Some(name.to_string()),
        src: src.to_string(),
        x: 0.0,
        y: 0.0,
        width: None,
        height: None,
        centred: false,
        color: 0xffff_ffff,
        u: None,
        v: None,
        texture_width: None,
        texture_height: None,
        auto_load: false,
        reveal: Vec::new(),
        transition: 0.0,
        start_enabled: true,
    }
}

fn cell(name: &str) -> Cell {
    Cell {
        name: name.to_string(),
        track: Some("16_Track".to_string()),
        mode: Mode::Race,
        class: "Venom".to_string(),
        weapons: true,
        damage: true,
        locked: None,
        status: None,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(3),
        ship: None,
        ship_choice: None,
        gold: 1,
        silver: 2,
        bronze: 3,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// Every `src` the draw list drew, in order, by the sprite it asked for.
fn drawn(summary: &GridSummary, next: Option<&str>) -> Vec<String> {
    let screen = Screen {
        images: vec![
            image("bBg_0_0", "black.mip"),
            image("bBg_1_0", "black.mip"),
            image("Bg_1_0", "bg.mip"),
            image("Outline_0_0", "outline.mip"),
            image("Outline_1_0", "outline.mip"),
            image("flyerlogo", r"Data\FE\Flyers\01_uplift\Logo.gtf"),
        ],
        ..Screen::default()
    };
    let layout = Layout {
        screen,
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
    };
    let model = CellSelection::new(vec![cell("grid8_0_0")]);
    let skin = Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        33.0,
    );
    let sprites = |src: &str| {
        let known = ["black.mip", "bg.mip", "outline.mip"];
        (known.contains(&src) || src.ends_with("Logo.gtf")).then_some(Placed {
            x: 0,
            y: 0,
            width: 8,
            height: 8,
            quad_extent: None,
            blend: None,
        })
    };
    let layers = hd_cell_draw_list(
        &model,
        &layout,
        &skin,
        &Frame::default(),
        &StringTable::default(),
        &CircuitNames::default(),
        0,
        8,
        summary,
        &CellArt {
            next_flyer: next,
            track_emblem: &|_| None,
        },
        None,
        false,
        &sprites,
        &[],
    );
    layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { .. } => Some("sprite".to_string()),
            _ => None,
        })
        .collect()
}

fn summary(required: u32, earned: u32) -> GridSummary {
    GridSummary {
        required_points: required,
        points_earned: earned,
        ..GridSummary::empty()
    }
}

#[test]
fn the_black_and_outline_hexes_draw_on_empty_slots_but_the_lit_one_only_on_the_cell() {
    // Two bBg (both slots), one Bg (the empty slot), one Outline (the cell's
    // own slot): the empty slot's Outline is not drawn.
    assert_eq!(drawn(&summary(0, 0), None).len(), 4);
}

#[test]
fn the_unlock_logo_draws_only_while_points_are_short_and_a_next_grid_exists() {
    let base = drawn(&summary(0, 0), Some("10_impact")).len();
    assert_eq!(drawn(&summary(12, 0), Some("10_impact")).len(), base + 1);
    assert_eq!(drawn(&summary(12, 12), Some("10_impact")).len(), base);
    assert_eq!(drawn(&summary(12, 0), None).len(), base);
}
