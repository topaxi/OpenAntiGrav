//! `EndRace Menu`'s cursor as `Block_Update` draws it, and the race-family
//! grid `0x0022c068`/`0x0022b688` lay out - off a fixture that authors the
//! attributes the real file does (`Color`, `selectable`, `arrowcolor`,
//! `textcolor`, lower-case as `EndRace Menu` spells them).

use super::{skin, strings};
use crate::endrace::hd::{hd_menu_draw_list, hd_menu_targets, hd_results_draw_list};
use crate::endrace::{EndRaceMenu, FieldResults, FieldRow, Headline, Layout, MenuOption};
use crate::frontend::{Draw, Placed};
use crate::menu::Frame;
use crate::menu::block::{BlockArt, FOCUS_GROWTH, Focus};
use crate::screen::{Screens, argb_to_rgba};

const XML: &str = r#"
<Screen type="EndRace Menu" name="EndRace Menu">
<Block name="race_again"><Values idstring="ER_RACE_AGAIN" x="375" y="235" width="520" selectable="true" arrowcolor="0xffffffff" shaped="true" Color="0xff646464" textcolor="0xffffffff"></Values></Block>
<Block name="return_to_grid"><Values idstring="ER_RETURN_GRID" x="375" y="285" width="520" selectable="true" arrowcolor="0xffffffff" shaped="true" Color="0xff646464" textcolor="0xffffffff"></Values></Block>
</Screen>
<Screen type="EndRace Results" name="EndRace Results">
<Item OffsetX="375" OffsetY="368">
<Block name="GridHead1"><Values x="0" y="24" IDString="IG_HUD_POS" TextColor="0xffffffff" unselectable="true" width="497" shaped="true" Color="0xff646464"></Values></Block>
<Block name="GridHead2"><Values x="505" y="24" IDString="IG_HUD_TIME" TextColor="0xffffffff" unselectable="true" width="286" shaped="true" Color="0xff646464"></Values></Block>
<Image name="GridHighlight"><Values x="1" y="391" width="789" height="38" color="0xff0000ff"></Values></Image>
<Image name="GridSideBarL"><Values x="1" y="44" width="4" height="347" color="0xff646464"></Values></Image>
<Image name="GridBottomBar"><Values x="1" y="391" width="789" height="4" color="0xff646464"></Values></Image>
<Image name="GridBottomBlock"><Values x="1" y="357" width="789" height="38" color="0xff646464"></Values></Image>
<Image name="GridStrikeThrough"><Values x="20" y="391" width="750" height="2" color="0xff646464"></Values></Image>
<Text name="Grid0.0"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid1.0"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid2.0"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid0.1"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid1.1"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid2.1"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
</Item>
</Screen>
"#;

fn layout(screen: &str) -> Layout {
    Layout::read_authored(
        &Screens::from_xml(XML),
        screen,
        &strings(),
        crate::picker::FaceScales::default(),
        [1920.0, 1080.0],
        [1920.0, 1080.0],
    )
    .unwrap()
}

fn placed(x: u32, width: u32, height: u32) -> Placed {
    Placed {
        x,
        y: 0,
        width,
        height,
        quad_extent: None,
        blend: None,
    }
}

fn frame() -> Frame {
    Frame {
        blocks: Some(BlockArt {
            frame: placed(0, 64, 64),
            fill_alpha: 110.0 / 255.0,
            solid_alpha: 1.0,
            cursor: None,
            arrow: Some(placed(96, 32, 32)),
            fury: false,
        }),
        ..Frame::default()
    }
}

/// Every sprite drawn in `rgb`, ignoring alpha - the block's fill passes
/// carry the swatch's alpha, its border the tint's own.
fn sprites_in(body: &[Draw], rgb: [f32; 3]) -> Vec<[f32; 4]> {
    body.iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, color, .. }
                if (color[0] - rgb[0]).abs() < 1e-3
                    && (color[1] - rgb[1]).abs() < 1e-3
                    && (color[2] - rgb[2]).abs() < 1e-3 =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect()
}

fn rgb(argb: u32) -> [f32; 3] {
    let [r, g, b, _] = argb_to_rgba(argb);
    [r, g, b]
}

#[test]
fn focus_eases_a_sixth_of_the_way_and_blinks_eight_on_nine_off() {
    let mut focus = Focus::default();
    focus.tick(true);
    assert!((focus.fraction() - 1.0 / 6.0).abs() < 1e-6);
    let mut lit = Vec::new();
    let mut focus = Focus::default();
    for _ in 0..34 {
        focus.tick(true);
        lit.push(focus.arrow_lit());
    }
    // Counter 1..=7 lit, 8..=16 dark, then 0 lit again: 7 + 9 + 1 per period.
    assert_eq!(lit.iter().take(17).filter(|lit| **lit).count(), 8);
    assert!(!lit[7] && lit[16], "{lit:?}");
    assert!(focus.fraction() > 0.99);
    for _ in 0..60 {
        focus.tick(false);
    }
    assert!(focus.fraction() < 0.01);
}

/// The focused option's box is `ActiveColor` (unauthored, so
/// `Block_Construct`'s `0xff8ac0ca`) and the rest keep their `Color`; the
/// focused one grows by up to 60, its arrow blinks at `X + 8`, and every
/// label sits at `X + 40` in its own `TextColor` - no brightening.
#[test]
fn the_focused_option_is_its_active_colour_block_with_a_blinking_arrow() {
    let mut menu = EndRaceMenu::new(vec![MenuOption::RaceAgain, MenuOption::ReturnToGrid], None);
    for _ in 0..34 {
        menu.tick();
    }
    let draw = |menu: &EndRaceMenu| {
        hd_menu_draw_list(
            menu,
            &layout("EndRace Menu"),
            &skin(),
            &frame(),
            &strings(),
            None,
            false,
            &|_| None,
        )
        .body
    };
    let body = draw(&menu);
    let active = sprites_in(&body, rgb(0xff8a_c0ca));
    let grey = sprites_in(&body, rgb(0xff64_6464));
    assert!(
        active
            .iter()
            .all(|rect| rect[1] >= 235.0 && rect[1] < 285.0)
    );
    assert!(!active.is_empty() && grey.iter().all(|rect| rect[1] >= 285.0));
    let right = active
        .iter()
        .map(|rect| rect[0] + rect[2])
        .fold(0.0, f32::max);
    assert!(
        (right - (375.0 + 520.0 + FOCUS_GROWTH)).abs() < 1.0,
        "{right}"
    );
    // The arrow: a 32x32 white sprite at (383, 239), on the lit phase.
    let arrow = |body: &[Draw]| {
        sprites_in(body, [1.0, 1.0, 1.0])
            .into_iter()
            .any(|rect| rect == [383.0, 239.0, 32.0, 32.0])
    };
    assert!(arrow(&body));
    for _ in 0..8 {
        menu.tick();
    }
    assert!(!arrow(&draw(&menu)), "counter 8 is the dark phase");
    for text in body.iter().filter_map(|draw| match draw {
        Draw::Text { x, color, .. } => Some((*x, *color)),
        _ => None,
    }) {
        assert_eq!(text, (415.0, [1.0, 1.0, 1.0, 1.0]));
    }
}

#[test]
fn a_menu_target_is_the_blocks_own_resting_box() {
    let menu = EndRaceMenu::new(vec![MenuOption::RaceAgain, MenuOption::ReturnToGrid], None);
    let targets = hd_menu_targets(&menu, &layout("EndRace Menu"));
    assert_eq!(targets[0].rect, [375.0, 235.0, 520.0, 40.0]);
    assert_eq!(targets[1].rect, [375.0, 285.0, 520.0, 40.0]);
}

/// The race layout: rows at the grid's `96 + 45 r`, place at `x = 40`,
/// time at `545`, nothing in column 1, the player's row white, the footer
/// block and strike-through hidden, and the frame rewritten to `443`/`487`.
#[test]
fn a_race_lays_its_grid_out_as_the_executable_does() {
    let model = FieldResults {
        headline: Headline::Position(2),
        rows: vec![
            FieldRow {
                place: 1,
                time_ticks: Some(6000),
                player: false,
            },
            FieldRow {
                place: 2,
                time_ticks: None,
                player: true,
            },
        ],
    };
    let body = hd_results_draw_list(
        &model,
        &layout("EndRace Results"),
        &skin(),
        &frame(),
        &strings(),
        None,
        false,
        &|_| None,
    )
    .body;
    let text = |wanted: &str| {
        body.iter()
            .find_map(|draw| match draw {
                Draw::Text {
                    text, x, y, color, ..
                } if text == wanted => Some((*x, *y, *color)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{wanted} drew: {body:?}"))
    };
    let grey = argb_to_rgba(0xff64_6464);
    assert_eq!(text("1"), (375.0 + 40.0, 368.0 + 96.0, grey));
    assert_eq!(text("1.40.00"), (375.0 + 545.0, 368.0 + 96.0, grey));
    assert_eq!(text("2"), (375.0 + 40.0, 368.0 + 141.0, [1.0; 4]));
    assert_eq!(text("-").0, 375.0 + 545.0);
    let fills: Vec<[f32; 4]> = body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert!(
        fills.contains(&[376.0, 368.0 + 44.0, 4.0, 443.0]),
        "{fills:?}"
    );
    assert!(
        fills.contains(&[376.0, 368.0 + 487.0, 789.0, 4.0]),
        "{fills:?}"
    );
    assert!(
        fills.contains(&[376.0, 368.0 + 139.0, 789.0, 38.0]),
        "{fills:?}"
    );
    assert!(
        !fills.iter().any(|rect| rect[1] == 368.0 + 357.0),
        "{fills:?}"
    );
    assert!(!fills.iter().any(|rect| rect[3] == 2.0), "{fills:?}");
}

/// Up and Down follow the Blocks down the screen, not the shared list's
/// Pulse order, and the default focus survives the reorder.
#[test]
fn hd_steps_its_options_in_screen_order_keeping_the_focus() {
    let pulse_order = EndRaceMenu::new(vec![MenuOption::ReturnToGrid, MenuOption::RaceAgain], None);
    let mut menu = crate::endrace::hd::hd_screen_order(&pulse_order, &layout("EndRace Menu"));
    assert_eq!(
        menu.options(),
        [MenuOption::RaceAgain, MenuOption::ReturnToGrid]
    );
    assert_eq!(menu.selected(), Some(MenuOption::ReturnToGrid));
    menu.step(-1);
    assert_eq!(menu.selected(), Some(MenuOption::RaceAgain), "Up goes up");
}
