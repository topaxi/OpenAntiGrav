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
    let texts: Vec<(f32, f32, f32, Align, [f32; 4], &str)> = list
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
            } => Some((*x, *y, *scale, *align, *color, text.as_str())),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&(105.0, 30.0, 1.0, Align::Left, BLUE, "LOADING...")),
        "{texts:?}"
    );
    assert!(
        texts.contains(&(51.0, 38.0, 0.6, Align::Centre, WHITE, "0%")),
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
