//! The screen drawn from its layout file, off a hand-built title.
//!
//! The file under test is the shipped `LoadingScreen_HD.toml`, so a number moved in it moves
//! these; its own numbers (and the addresses they were read at) are pinned by
//! `oag_hud::loading_layout`'s tests. The disc-backed half, with the real art and string table,
//! is `tests/loading_screen_ground_truth.rs`.

use super::tests::progress;
use super::*;
use oag_hud::loading_layout::LoadingLayouts;

const HD_FILE: &str = include_str!("../../../../assets/fallback/Data/XML/LoadingScreen_HD.toml");

/// The renderer is given the file's own grid, so a draw's units are the file's.
const GX: f32 = 1.0;
const GY: f32 = 1.0;

/// The labels the HD file names, as the disc's table would resolve them.
fn strings() -> Vec<(String, String)> {
    [
        ("FE_FEATURE_IMAGE", "IMAGE"),
        ("FE_FEATURE_DESC", "DESC"),
        ("FE_PROG_BAR", "BAR"),
        ("FE_MODE_ICON", "ICON"),
    ]
    .map(|(id, text)| (id.to_string(), text.to_string()))
    .to_vec()
}

fn art() -> Art {
    Art {
        sheet: oag_hud::sprite::Sheet::default(),
        illustrations: vec![[0.0, 0.0, 64.0, 32.0]],
        title_arrow: Some([0.0, 0.0, 8.0, 8.0]),
        subtitle_arrow: Some([0.0, 0.0, 8.0, 8.0]),
        rule: Some([0.0, 0.0, 8.0, 8.0]),
        corner: Some([0.0, 0.0, 8.0, 8.0]),
        dot: Some([0.0, 0.0, 8.0, 8.0]),
        square: Some([0.0, 0.0, 8.0, 8.0]),
        mode_stills: None,
    }
}

fn assets(layout: Option<&str>) -> Assets {
    Assets {
        caption: Some("LOADING...".to_string()),
        wave: false,
        art: Some(art()),
        features: vec![Feature {
            slot: 2,
            heading: Some("DESCRIPTION".to_string()),
            title: Some("PILOT ASSIST".to_string()),
            description: "Pilot Assist can aid your navigation.".to_string(),
        }],
        layout: layout.map(|text| LoadingLayouts::parse(text).unwrap()),
        layout_strings: strings(),
        ..Assets::default()
    }
}

/// The assets of the HD title: its layout file, one feature, every mark.
pub(super) fn hd_screen_assets() -> Assets {
    assets(Some(HD_FILE))
}

/// A screen of the HD title.
pub(super) fn hd_screen() -> Screen {
    Screen::new(&hd_screen_assets(), AUTHORED_LINE_HEIGHT, 0, None)
}

fn texts(list: &[Draw]) -> Vec<String> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } | Draw::FacedText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn tiled(list: &[Draw]) -> Vec<([f32; 4], [f32; 2], [f32; 4])> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::TiledSprite {
                rect,
                repeat,
                color,
                ..
            } => Some((*rect, *repeat, *color)),
            _ => None,
        })
        .collect()
}

fn race(screen: &Screen) -> Vec<Draw> {
    screen.draw_list(Phase::Race, &Progress::default(), &Atlas::build())
}

/// The layout draws every text it names: the caption with the circuit, the feature's name, its
/// heading and its prose.
///
/// The regression this pins is a silent one: a second branch through `draw_list` once dropped
/// the heading while still drawing its marker, which reads as a screen always meant to start
/// with a picture.
#[test]
fn the_layout_draws_its_caption_its_name_its_heading_and_its_prose() {
    let state = Progress {
        current: Some("Talon's Junction".to_string()),
        ..Progress::default()
    };
    let list = hd_screen().draw_list(Phase::Race, &state, &Atlas::build());
    let drawn = texts(&list);
    for wanted in ["LOADING... Talon's Junction", "PILOT ASSIST", "DESCRIPTION"] {
        assert!(
            drawn.iter().any(|line| line == wanted),
            "{wanted}: {drawn:?}"
        );
    }
    assert!(
        drawn
            .iter()
            .any(|line| line.starts_with("Pilot Assist can")),
        "{drawn:?}"
    );
    // the caption is in the `Title` face, the rest in the primary one
    assert!(list.iter().any(|draw| matches!(
        draw,
        Draw::FacedText { role: "Title", text, .. } if text.starts_with("LOADING...")
    )));
}

/// The feature's name sits over the picture and its heading over the prose, in one row.
#[test]
fn the_name_is_left_of_the_heading_in_one_row() {
    let list = race(&hd_screen());
    let at = |wanted: &str| {
        list.iter().find_map(|draw| match draw {
            Draw::Text { x, y, text, .. } if text == wanted => Some((*x, *y)),
            _ => None,
        })
    };
    let name = at("PILOT ASSIST").expect("a name");
    let heading = at("DESCRIPTION").expect("a heading");
    assert!(name.0 < heading.0, "{name:?} {heading:?}");
    assert!((name.1 - heading.1).abs() < 0.01, "one row");
}

/// The lit columns are whole columns of the fraction, and the two runs tile the bar exactly: a
/// fill that drifted off the grid would cut a dot in half.
#[test]
fn the_fill_lights_whole_columns_and_the_runs_tile_the_bar() {
    for (done, lit) in [(1, 83.0), (0, 0.0)] {
        let list = hd_screen().draw_list(
            Phase::Prefetch,
            &Progress {
                total: 2,
                done,
                ..Progress::default()
            },
            &Atlas::build(),
        );
        let runs = tiled(&list);
        let columns: f32 = runs.iter().map(|(_, repeat, _)| repeat[0]).sum();
        assert_eq!(columns, 166.0, "{runs:?}");
        let lit_columns = runs
            .iter()
            .find(|(_, _, color)| color[..3] == BAR_FILL[..3])
            .map_or(0.0, |(_, repeat, _)| repeat[0]);
        assert_eq!(lit_columns, lit, "{runs:?}");
    }
}

/// The bar is the original's 166 columns by 30 rows of square tiles, and the grid is as tall
/// as those rows in pixels.
#[test]
fn the_bar_is_thirty_dot_rows_of_square_tiles() {
    let list = race(&hd_screen());
    let (rect, repeat, _) = tiled(&list)[0];
    assert!((repeat[1] - 30.0).abs() < f32::EPSILON, "{repeat:?}");
    // 8 grid pixels a tile, in screen units, both ways
    assert!((rect[3] - 30.0 * 8.0 * GY).abs() < 0.01, "{rect:?}");
    assert!((rect[0] - 168.0 * GX).abs() < 0.01, "{rect:?}");
    let right: f32 = tiled(&list).iter().map(|(r, ..)| r[2]).sum::<f32>() + rect[0];
    assert!(
        (right - 1496.0 * GX).abs() < 0.01,
        "the bar ends at 1496: {right}"
    );
}

/// No dot art means no bar: the layout skips what the title cannot resolve and does not stand
/// in a flat rectangle for it.
#[test]
fn no_dot_art_draws_no_bar_and_no_stand_in() {
    let mut screen = hd_screen();
    screen.dot = None;
    let list = race(&screen);
    assert!(tiled(&list).is_empty());
    assert!(
        !list
            .iter()
            .any(|draw| matches!(draw, Draw::Fill { rect, .. } if rect[3] < 1080.0)),
        "only the ground is a fill"
    );
}

/// Every label the layout names is drawn, each behind a bullet; one the string table did not
/// carry is left out rather than drawn as its id. The brand is the layout's own literal.
#[test]
fn the_labels_are_drawn_behind_their_bullets_and_a_missing_one_is_skipped() {
    let mut assets = assets(Some(HD_FILE));
    assets.layout_strings.retain(|(id, _)| id != "FE_PROG_BAR");
    let screen = Screen::new(&assets, AUTHORED_LINE_HEIGHT, 0, None);
    let list = race(&screen);
    let drawn = texts(&list);
    for wanted in ["WIPEOUT\u{ae} HD", "IMAGE", "DESC", "ICON"] {
        assert!(
            drawn.iter().any(|line| line == wanted),
            "{wanted} in {drawn:?}"
        );
    }
    assert!(
        !drawn
            .iter()
            .any(|line| line == "BAR" || line.contains("PROG")),
        "{drawn:?}"
    );
    let bullets = list
        .iter()
        .filter(|draw| {
            matches!(draw, Draw::Sprite { rect, .. }
                if (rect[2] - 16.0 * GX).abs() < 0.01 && (rect[3] - 16.0 * GY).abs() < 0.01)
        })
        .count();
    // the four labels, and the bullet-sized corner brackets are 16 too: 4 corners x 4 panels
    assert_eq!(
        bullets,
        4 + 16,
        "one bullet per label drawn, and the brackets"
    );
}

/// A bracket on the right or bottom edge is pulled back by the layout's `back`, and drawn
/// mirrored; the top left one is drawn as it is.
#[test]
fn the_corner_brackets_sit_at_the_panels_corners_and_mirror() {
    let list = race(&hd_screen());
    // the feature image panel: rect 160..950 x 155..655, 16 square, back 12
    let at = |x: f32, y: f32| {
        list.iter().find_map(|draw| match draw {
            Draw::Sprite { rect, uv, .. }
                if (rect[0] - x * GX).abs() < 0.01 && (rect[1] - y * GY).abs() < 0.01 =>
            {
                Some(*uv)
            }
            _ => None,
        })
    };
    let top_left = at(160.0, 155.0).expect("top left");
    assert!(
        top_left[2] > 0.0 && top_left[3] < 0.0,
        "mirrored down: {top_left:?}"
    );
    let bottom_right = at(950.0 - 12.0, 655.0 - 12.0).expect("bottom right, pulled back 12");
    assert!(
        bottom_right[2] < 0.0 && bottom_right[3] > 0.0,
        "mirrored across only: {bottom_right:?}"
    );
}

/// The Safe Area scales the grid about its centre: the top rule's left end at 160 is 192 at 96 %.
#[test]
fn the_safe_area_scales_the_grid_about_its_centre() {
    let mut screen = hd_screen();
    let rule = |screen: &Screen| {
        race(screen).into_iter().find_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[1] < 400.0 && rect[2] > 100.0 => Some(rect),
            _ => None,
        })
    };
    let whole = rule(&screen).expect("the rule");
    assert!((whole[0] - 160.0 * GX).abs() < 0.01 && (whole[2] - 1600.0 * GX).abs() < 0.01);
    screen.set_safe_area(0.96);
    let inset = rule(&screen).expect("the rule");
    assert!((inset[0] - 192.0 * GX).abs() < 0.01, "{inset:?}");
    assert!((inset[1] - 127.2 * GY).abs() < 0.01, "{inset:?}");
    assert!((inset[2] - 1536.0 * GX).abs() < 0.01, "{inset:?}");
}

/// The counts sit under the bar, below its last row, and only while something is counted.
#[test]
fn the_counts_sit_under_the_bar_and_only_while_counted() {
    let screen = hd_screen();
    let counted_row = |list: &[Draw]| {
        list.iter().find_map(|draw| match draw {
            Draw::Text { y, text, .. } if text.contains("/ 2") => Some(*y),
            _ => None,
        })
    };
    let list = screen.draw_list(Phase::Prefetch, &progress(1, 2), &Atlas::build());
    assert!(
        counted_row(&list).is_some_and(|y| y > 948.0 * GY),
        "{:?}",
        counted_row(&list)
    );
    assert!(
        counted_row(&race(&screen)).is_none(),
        "a race load counts nothing"
    );
}

/// A title whose layout file did not load draws the ground and nothing else, and says so at
/// load: no stand-in layout.
#[test]
fn a_title_with_art_and_no_layout_draws_only_the_ground() {
    let screen = Screen::new(&assets(None), AUTHORED_LINE_HEIGHT, 0, None);
    assert_eq!(race(&screen).len(), 1);
}

/// A layout restricted to a mode answers only that mode's race.
#[test]
fn a_layout_for_one_mode_does_not_answer_another() {
    let only_zone = HD_FILE.replace(
        "grid = [1920, 1080]",
        "grid = [1920, 1080]\nmodes = [\"time_trial\"]",
    );
    let single = Screen::for_mode(
        &assets(Some(&only_zone)),
        AUTHORED_LINE_HEIGHT,
        0,
        None,
        Some(3),
    );
    assert_eq!(race(&single).len(), 1, "single race is not time trial");
    let trial = Screen::for_mode(
        &assets(Some(&only_zone)),
        AUTHORED_LINE_HEIGHT,
        0,
        None,
        Some(5),
    );
    assert!(race(&trial).len() > 20);
}

/// Lit columns of a race load's bar, `0` while no column is lit.
fn lit_in_race(screen: &Screen) -> f32 {
    tiled(&race(screen))
        .iter()
        .find(|(_, _, color)| color[..3] == BAR_FILL[..3])
        .map_or(0.0, |(_, repeat, _)| repeat[0])
}

/// HD's bar follows the load's own stages: a first target of a fifth that it eases up to and
/// holds, and each stage the load reports moves the target on.
#[test]
fn a_stage_driven_bar_holds_at_its_target_until_the_load_reports_a_stage() {
    let mut screen = hd_screen();
    screen.fill = Some(fill::Fill::new(oag_hd::loading::PROGRESSION));
    for _ in 0..100 {
        screen.advance(false);
    }
    assert!(
        (lit_in_race(&screen) - 10.0).abs() < f32::EPSILON,
        "0.1 a frame"
    );
    for _ in 0..1000 {
        screen.advance(false);
    }
    assert!(
        (lit_in_race(&screen) - 33.0).abs() < f32::EPSILON,
        "held at 20 %"
    );
    for stage in 1..=4 {
        screen.set_load_stage(stage);
        for _ in 0..2000 {
            screen.advance(false);
        }
        let expected =
            (oag_hd::loading::PROGRESSION.milestones[usize::from(stage) - 1] * 166.0).floor();
        assert!(
            (lit_in_race(&screen) - expected).abs() < f32::EPSILON,
            "stage {stage}: {} lit against {expected}",
            lit_in_race(&screen)
        );
    }
    screen.advance(true);
    assert!(
        (lit_in_race(&screen) - 166.0).abs() < f32::EPSILON,
        "full once ready"
    );
}

/// A title that names no progression keeps the time estimate: it grows every tick, never fills
/// the bar while the wait is on, and fills it the moment the wait is over.
#[test]
fn a_title_with_no_progression_keeps_the_time_estimate() {
    let mut screen = hd_screen();
    assert!(screen.fill.is_none());
    assert_eq!(lit_in_race(&screen), 0.0, "nothing to show at tick zero");
    let mut last = 0.0;
    for _ in 0..4 {
        for _ in 0..60 {
            screen.advance(false);
        }
        let lit = lit_in_race(&screen);
        assert!(lit > last, "the bar moves every second: {lit} after {last}");
        assert!(lit < 166.0, "and never completes on its own: {lit}");
        last = lit;
    }
    // four seconds is `ESTIMATE_TICKS`: two thirds of the bar
    let expected = ((1.0 - (-1.0f32).exp()) * 166.0).floor();
    assert!((last - expected).abs() <= 1.0, "{last} against {expected}");
    screen.advance(true);
    assert!(
        (lit_in_race(&screen) - 166.0).abs() < f32::EPSILON,
        "full once ready"
    );
}

/// The mode icon is a video draw in the panel right of the bar, once the stage says a picture
/// is in the planes, and not before, and not while fading.
#[test]
fn the_mode_icon_is_a_video_in_its_panel_only_once_it_has_a_picture() {
    use oag_ui::frontend::Video;
    let video = |screen: &Screen| {
        race(screen).into_iter().find_map(|draw| match draw {
            Draw::Video { rect, source, .. } => Some((rect, source)),
            _ => None,
        })
    };
    let mut screen = hd_screen();
    assert!(video(&screen).is_none(), "no picture yet: no green quad");
    screen.set_mode_icon(true);
    let (rect, source) = video(&screen).expect("the icon is drawn");
    assert_eq!(source, Video::ModeIcon);
    assert!((rect[0] - 1510.0 * GX).abs() < 0.01, "{rect:?}");
    assert!(
        (rect[2] - 250.0 * GX).abs() < 0.01 && (rect[3] - 250.0 * GY).abs() < 0.01,
        "{rect:?}"
    );
    screen.advance(true);
    assert!(video(&screen).is_none(), "gone once the screen fades");
}

/// A mode the title maps no clip for gets none.
#[test]
fn only_a_mapped_mode_has_a_clip() {
    let assets = Assets {
        mode_icons: vec![ModeIcon {
            mode: 3,
            entry: "x.bik".to_string(),
            blob: Vec::new(),
        }],
        ..Assets::default()
    };
    assert!(super::icon::clip(&assets, Some(3)).is_some());
    assert!(super::icon::clip(&assets, Some(5)).is_none());
    assert!(super::icon::clip(&assets, None).is_none());
}

/// The HD screen with its feature picture a 1024x512 sheet of one opaque `ground` colour, the
/// size and shape of the disc's own illustrations.
fn hd_screen_with_picture(ground: [u8; 4]) -> Screen {
    let mut assets = hd_screen_assets();
    let image = oag_hud::sprite::DecodedImage {
        src: "feature".to_string(),
        width: 1024,
        height: 512,
        rgba: ground.repeat(1024 * 512),
        quad_extent: None,
        quad_centre: None,
        blend: None,
    };
    let sheet = oag_hud::sprite::Sheet::build_with(&[], vec![image], &mut Vec::new());
    let placed = sheet.get("feature").expect("the picture is in the sheet");
    let art = assets.art.as_mut().expect("the fixture has art");
    art.illustrations = vec![[
        placed.x as f32,
        placed.y as f32,
        placed.width as f32,
        placed.height as f32,
    ]];
    art.sheet = sheet;
    Screen::new(&assets, AUTHORED_LINE_HEIGHT, 0, None)
}

/// The feature picture's draw: the one big sprite, `[x0, y0, x1, y1]` in the grid.
fn picture(screen: &Screen) -> [f32; 4] {
    let found: Vec<[f32; 4]> = race(screen)
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] >= 200.0 && rect[3] >= 200.0 => {
                Some([rect[0], rect[1], rect[0] + rect[2], rect[1] + rect[3]])
            }
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "one picture: {found:?}");
    found[0]
}

/// The screen shapes a player has: a 16:10 Deck, 16:9, 21:9 and a phone's 20:9.
const SHAPES: [(&str, f32); 4] = [
    ("1280x800", 1280.0 / 800.0),
    ("1920x1080", 1920.0 / 1080.0),
    ("2560x1080", 2560.0 / 1080.0),
    ("20:9", 20.0 / 9.0),
];

/// A picture whose ground shows (HD's white) is cut to its panel, 160 to 950 by 155 to 655 in
/// the grid, on every screen shape: drawn whole it covers the description beside it. Without
/// the cut the picture reaches x 1076 and y 670.
#[test]
fn a_picture_with_a_showing_ground_lies_inside_its_panel_on_every_screen_shape() {
    let white = [255, 255, 255, 255];
    for (shape, aspect) in SHAPES {
        let mut screen = hd_screen_with_picture(white);
        screen.set_surface_aspect(aspect);
        let [x0, y0, x1, y1] = picture(&screen);
        assert!(
            x0 >= 160.0 && y0 >= 155.0 && x1 <= 950.0 && y1 <= 655.0,
            "{shape}: the picture is at {x0},{y0} to {x1},{y1}"
        );
        assert!(
            x1 - x0 > 500.0 && y1 - y0 > 300.0,
            "{shape}: most of it stays, not a sliver: {x0},{y0} to {x1},{y1}"
        );
    }
}

/// A picture whose ground is the screen's own shows nothing outside its panel, so it is drawn
/// as the layout places it: the cut would change nothing a player sees and would move the
/// sampling.
#[test]
fn a_picture_whose_ground_is_the_screens_own_is_left_as_placed() {
    let ground = [5, 8, 13, 255];
    for (shape, aspect) in SHAPES {
        let mut screen = hd_screen_with_picture(ground);
        screen.set_surface_aspect(aspect);
        assert_eq!(
            picture(&screen),
            [180.0, 190.0, 1076.0, 670.0],
            "{shape}: as the layout places it"
        );
    }
}

const OMEGA_FILE: &str =
    include_str!("../../../../assets/fallback/Data/XML/LoadingScreen_Omega.toml");

/// The mode icon's still is drawn at the panel from the picture its mode names, in the accent,
/// and a mode that names none (or a title with no stills) draws nothing there.
#[test]
fn a_mode_still_is_drawn_in_the_panel_for_the_mode_that_names_it() {
    let mut omega = assets(Some(OMEGA_FILE));
    omega.art.as_mut().unwrap().mode_stills = Some(super::assets::ModeStills {
        modes: vec![(5, Some([0.0, 0.0, 16.0, 16.0])), (0xd, None)],
        otherwise: Some([16.0, 0.0, 16.0, 16.0]),
    });
    let drawn = |omega: &Assets, mode: Option<u32>| {
        let screen = Screen::for_mode(omega, AUTHORED_LINE_HEIGHT, 0, None, mode);
        race(&screen)
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, uv, color } if rect[2] == 230.0 && rect[3] == 234.0 => {
                    Some((rect, uv, color))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let named = drawn(&omega, Some(5));
    assert_eq!(named.len(), 1, "{named:?}");
    assert_eq!((named[0].0[0], named[0].0[1]), (1518.0, 708.0));
    let other = drawn(&omega, Some(3));
    assert_eq!(other.len(), 1);
    assert_ne!(named[0].1, other[0].1, "each takes its own picture");
    assert!(
        drawn(&omega, Some(0xd)).is_empty(),
        "a mode with no picture draws none"
    );
    // HD's file has no still and names no `@mode-icon`: the sheet of a title without stills
    // draws nothing in the panel
    omega.art.as_mut().unwrap().mode_stills = None;
    assert!(drawn(&omega, Some(5)).is_empty());
}
