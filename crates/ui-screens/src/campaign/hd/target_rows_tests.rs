//! `Cell Selection`'s target rows on an HD-dialect title: a campaign that names its own
//! medal tiers lists the best three the event awards, as Pulse's screen does.

use super::*;
use oag_tables::race_campaign::{MedalTier, MedalTiers, Mode};
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Skin};
use oag_ui::screen::{Image, Screen, Text};

fn tiers() -> MedalTiers {
    let tier = |name: &str, points, argb: u32| MedalTier {
        name: name.to_string(),
        points,
        argb,
        string_id: None,
    };
    MedalTiers::new(vec![
        tier("platinum", 4, 0xffe5_e4e2),
        tier("gold", 3, 0xfffa_eb38),
        tier("silver", 2, 0xffda_e3e4),
        tier("bronze", 1, 0xffdf_942f),
    ])
    .expect("a valid list")
}

fn timed_cell() -> Cell {
    Cell {
        name: "grid0_0_0".to_string(),
        track: Some("16_Track".to_string()),
        mode: Mode::TimeTrial,
        class: "Venom".to_string(),
        weapons: false,
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
        gold: 9_000,
        silver: 9_100,
        bronze: 9_200,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// The target texts and the target medal sprites (by their authored left edge) one draw lists.
fn drawn(targets: Option<[i64; 4]>) -> (Vec<String>, Vec<f32>) {
    let mut texts = Vec::new();
    let mut images = Vec::new();
    for row in 0..3 {
        texts.push(Text {
            name: Some(format!("Target{row}")),
            ..Text::default()
        });
        images.push(Image {
            name: Some(format!("Target{row} Medal")),
            src: "medal.mip".to_string(),
            x: 100.0 * (row + 1) as f32,
            y: 0.0,
            width: Some(60.0),
            height: Some(60.0),
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
        });
    }
    let layout = Layout {
        screen: Screen {
            texts,
            images,
            ..Screen::default()
        },
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
    };
    let mut model = CellSelection::new(vec![timed_cell()]).with_tiers(tiers());
    if let Some(targets) = targets {
        model = model.with_tier_targets(&|_| Some(targets.to_vec()));
    }
    let skin = Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        33.0,
    );
    let sprites = |_: &str| {
        Some(Placed {
            x: 0,
            y: 0,
            width: 60,
            height: 60,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
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
        1,
        &GridSummary::empty(),
        &CellArt {
            next_flyer: None,
            fury: true,
            medal_tier_rows: [0, 1, 2],
            track_emblems: &std::collections::HashMap::new(),
        },
        None,
        false,
        &sprites,
        &[],
    );
    let mut text_out = Vec::new();
    let mut sprite_out = Vec::new();
    for draw in &layers.body {
        match draw {
            Draw::Text { text, .. } | Draw::FacedText { text, .. } => text_out.push(text.clone()),
            Draw::Sprite { rect, .. } => sprite_out.push(rect[0]),
            _ => {}
        }
    }
    (text_out, sprite_out)
}

#[test]
fn the_target_rows_list_the_best_three_tiers_the_event_awards() {
    let (texts, medals) = drawn(Some([13_800, 15_000, 16_500, 18_000]));
    assert_eq!(texts, ["2.18.00", "2.30.00", "2.45.00"], "platinum, gold, silver");
    assert_eq!(medals, [100.0, 200.0, 300.0], "a medal beside each row");
}

#[test]
fn a_tier_the_event_does_not_award_has_no_row_and_no_medal() {
    let (texts, medals) = drawn(Some([0, 15_000, 16_500, 18_000]));
    assert_eq!(texts, ["2.30.00", "2.45.00", "3.00.00"], "gold, silver, bronze");
    let (texts, medals_one) = drawn(Some([0, 15_000, 0, 0]));
    assert_eq!(texts, ["2.30.00"]);
    assert_eq!(medals_one, [100.0], "{medals:?}");
}

#[test]
fn the_titles_own_gold_silver_bronze_rows_stand_when_the_campaign_names_no_targets() {
    let (texts, medals) = drawn(None);
    assert_eq!(texts, ["1.30.00", "1.31.00", "1.32.00"]);
    assert_eq!(medals.len(), 3);
}
