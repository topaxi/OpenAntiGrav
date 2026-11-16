//! Wipeout 2048's race load drawn from its shipped layout file, off a hand-built title.
//!
//! The file under test is `LoadingScreen_2048.toml` and the law is `oag_2048::loading`'s, so a
//! number moved in either moves these. The operands are
//! `docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, "The box, the bar and the meter".

use super::tests::progress;
use super::*;
use oag_hud::loading_layout::LoadingLayouts;

const FILE: &str = include_str!("../../../../assets/fallback/Data/XML/LoadingScreen_2048.toml");

const BLUE: [f32; 4] = [0.098, 0.161, 0.365, 1.0];
const WHITE: [f32; 4] = [0.882, 0.894, 0.922, 1.0];

fn screen() -> Screen {
    let assets = Assets {
        caption: Some("LOADING...".to_string()),
        palette: Some(assets::Palette {
            background: WHITE,
            ink: BLUE,
            accent: WHITE,
            dim: BLUE,
        }),
        progression: Some(oag_2048::loading::PROGRESSION),
        layout: Some(LoadingLayouts::parse(FILE).unwrap()),
        ..Assets::default()
    };
    Screen::new(&assets, AUTHORED_LINE_HEIGHT, 0, None)
}

#[derive(Debug, PartialEq)]
struct Run {
    at: [f32; 2],
    scale: f32,
    align: Align,
    color: [f32; 4],
    text: String,
}

fn list(screen: &Screen) -> Vec<Draw> {
    screen.draw_list(Phase::Race, &progress(0, 0), &Atlas::build())
}

fn readout(list: &[Draw]) -> Option<String> {
    list.iter().find_map(|draw| match draw {
        Draw::Text { text, scale, .. } if (*scale - 0.6).abs() < 1e-3 => Some(text.clone()),
        _ => None,
    })
}

#[test]
fn the_screen_is_drawn_in_the_960_by_544_grid() {
    let space = screen().space();
    assert_eq!(space.size, (960.0, 544.0));
}

#[test]
fn the_box_and_the_rule_are_blue_fills_at_the_measured_rectangles() {
    let fills: Vec<([f32; 4], [f32; 4])> = list(&screen())
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, color } => Some((*rect, *color)),
            _ => None,
        })
        .collect();
    assert!(
        fills.contains(&([16.0, 16.0, 70.0, 70.0], BLUE)),
        "{fills:?}"
    );
    assert!(
        fills.contains(&([86.0, 82.0, 338.0, 4.0], BLUE)),
        "{fills:?}"
    );
}

#[test]
fn the_caption_is_left_at_105_30_and_the_readout_centred_on_51_38_in_white() {
    let list = list(&screen());
    let texts: Vec<_> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text {
                x,
                y,
                scale,
                align,
                color,
                text,
                ..
            } => Some(Run {
                at: [*x, *y],
                scale: *scale,
                align: *align,
                color: *color,
                text: text.clone(),
            }),
            _ => None,
        })
        .collect();
    let run = |at: [f32; 2], scale: f32, align: Align, color: [f32; 4], text: &str| Run {
        at,
        scale,
        align,
        color,
        text: text.to_string(),
    };
    assert!(
        texts.contains(&run([105.0, 30.0], 1.0, Align::Left, BLUE, "LOADING...")),
        "{texts:?}"
    );
    assert!(
        texts.contains(&run([51.0, 38.0], 0.6, Align::Centre, WHITE, "0%")),
        "{texts:?}"
    );
}

#[test]
fn the_meter_is_seven_triangles_of_three_quads_at_the_measured_centres() {
    let list = list(&screen());
    let triangles = list
        .iter()
        .find_map(|draw| match draw {
            Draw::SolidTriangles { triangles, color } => Some((triangles, *color)),
            _ => None,
        })
        .expect("a meter");
    assert_eq!(triangles.0.len(), 7 * 6);
    assert_eq!(triangles.1, BLUE);
    // The row spans x 715 - 24 to 895 + 24 at full breath; none strays past it.
    let xs = triangles.0.iter().flatten().map(|p| p[0]);
    assert!(xs.clone().all(|x| (690.0..=920.0).contains(&x)));
}

#[test]
fn the_readout_follows_the_law_a_step_per_two_frames_and_ends_at_a_hundred() {
    let mut screen = screen();
    assert_eq!(readout(&list(&screen)).as_deref(), Some("0%"));
    // 300 steps at 0.115 columns of 166: 34.5 columns, 20 %.
    for _ in 0..600 {
        screen.advance(false);
    }
    assert_eq!(readout(&list(&screen)).as_deref(), Some("20%"));
    // The ceiling of 57 % holds it, the load's stages raise it.
    for _ in 0..3000 {
        screen.advance(false);
    }
    assert_eq!(readout(&list(&screen)).as_deref(), Some("57%"));
    screen.set_load_stage(3);
    for _ in 0..20_000 {
        screen.advance(false);
    }
    assert_eq!(readout(&list(&screen)).as_deref(), Some("98%"));
    screen.advance(true);
    assert_eq!(readout(&list(&screen)).as_deref(), Some("100%"));
}

#[test]
fn hd_draws_no_readout_or_meter() {
    let hd = super::layout_tests::hd_screen();
    let list = hd.draw_list(Phase::Race, &Progress::default(), &Atlas::build());
    assert!(readout(&list).is_none());
    assert!(
        !list
            .iter()
            .any(|draw| matches!(draw, Draw::SolidTriangles { .. }))
    );
}

fn event() -> oag_ui::frontend::MapEvent {
    oag_ui::frontend::MapEvent {
        name: "2048 - Event 1".to_string(),
        x: 0.0,
        y: 0.0,
        detail: String::new(),
        requires: None,
        kind: oag_ui::frontend::EventIcon::Race,
        forced_craft: None,
        refused_craft: Vec::new(),
        card: oag_ui::frontend::EventCard {
            title: "EMPIRE CLIMB".to_string(),
            mode_body: Some("Race Events are classic Wipeout.".to_string()),
            ..Default::default()
        },
        role: oag_title::MapRole::default(),
        rank_gated: false,
        rank_required: None,
    }
}

fn tips() -> assets::GameTipSet {
    let rows = (0..oag_2048::tips::TIPS.rows.len())
        .map(|row| {
            Some(oag_ui::frontend::GameTip {
                header: "GAME TIP".to_string(),
                question_mark: "q".to_string(),
                texture: format!("row{row}"),
                title: format!("TIP {row}"),
                body: "body".to_string(),
            })
        })
        .collect();
    assets::GameTipSet {
        tips: oag_2048::tips::TIPS,
        rows,
    }
}

fn with_card() -> Screen {
    let mut screen = screen();
    screen.set_card(
        oag_ui::frontend::LaunchedCard::new(event(), 0),
        Some(&tips()),
        7,
    );
    screen
}

fn texts(list: &[Draw]) -> Vec<String> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_launched_events_card_is_behind_the_box_with_a_game_tip_from_the_races_list() {
    let screen = with_card();
    let drawn = texts(&list(&screen));
    assert!(drawn.iter().any(|text| text == "EMPIRE CLIMB"), "{drawn:?}");
    let tip = screen.card_tip().expect("a tip is up");
    let row = tips()
        .rows
        .iter()
        .position(|row| row.as_ref() == Some(tip))
        .unwrap();
    assert!(
        oag_2048::tips::TIPS
            .candidates(oag_title::loading::TipClass::Race)
            .contains(&(row as u8)),
        "row {row} is not on a race's list"
    );
    assert!(
        drawn.contains(&tip.title),
        "the tip's title is on screen: {drawn:?}"
    );
    // A screen with no card draws none of it.
    let bare = texts(&list(&super::vita_tests::screen()));
    assert!(!bare.iter().any(|text| text == "EMPIRE CLIMB"));
}

#[test]
fn the_race_manager_turns_the_card_to_its_mode_page_and_it_slides_two_hundred_and_two() {
    let mut screen = with_card();
    screen.set_load_stage(3);
    assert!(
        !screen.card_on_mode_page(),
        "the loader's last milestone is not the race manager"
    );
    assert_eq!(screen.card_offset(), 0.0);
    screen.set_load_stage(4);
    assert!(screen.card_on_mode_page());
    let drawn = texts(&list(&screen));
    assert!(
        drawn.iter().any(|text| text.starts_with("Race Events")),
        "{drawn:?}"
    );
    assert!(
        !drawn.iter().any(|text| text.starts_with("TIP ")),
        "the tip is dropped: {drawn:?}"
    );
    // Two frames a step: s climbs 0.08 a step, the offset is -(1 - (1 - s)^2) * 202.
    screen.advance(false);
    screen.advance(false);
    assert!(
        (screen.card_offset() + 31.0).abs() < 0.1,
        "{}",
        screen.card_offset()
    );
    for _ in 0..40 {
        screen.advance(false);
    }
    assert!(
        (screen.card_offset() + 202.0).abs() < 1e-3,
        "{}",
        screen.card_offset()
    );
}
