//! The pause screen drawn from the disc's widgets, off a fixture that authors the attributes the
//! real `ingame_definition.xml` does.

use super::*;
use oag_title::{Origin, PauseAction as A, PauseScreen as Screen, pause_menu::row};
use oag_ui::frontend::Placed as Sprite;
use oag_ui::menu::block::BlockArt;
use oag_ui::menu::{Action, Definition, Menu};

const XML: &str = r#"
<Screen name="InGame Main Menu">
<Image><Values x="480" y="292" width="960" height="4" color="0xffffffff"></Values></Image>
<Image><Values x="480" y="790" width="960" height="4" color="0xffffffff"></Values></Image>
<Text><Values idstring="IG_PAUSE_RESU" x="525" y="800" color="0xffffffff"></Values></Text>
</Screen>
<Screen name="InGame Pause SP Time Trial" type="Settings">
<Image><Values x="480" y="251" width="32" height="32" color="0xffffffff" src="Data\FE\Images\Title_Arrow_HD.gtf"></Values></Image>
<Text><Values idstring="IG_PAUSE_GPAUSED" font="Title" x="520" y="240"></Values></Text>
<Block name="Continue"><Values idstring="IG_PAUSE_CONT" x="480" y="310" width="520" selectable="true" arrowcolor="0xffffffff" shaped="true" Color="FEGlobals->HD_Grey" ActiveColor="FEGlobals->HD_Blue" textcolor="FEGlobals->HD_White"></Values></Block>
<List name="Pilot Assist" default="FE_OFF"><Values idstring="FE_PILOT_ASSIST" font="small" x="480" y="360" ActiveColor="FEGlobals->HD_Blue" InactiveColor="FEGlobals->HD_Grey" arrowcolor="0xffffffff"></Values><Entry idstring="FE_ON"></Entry><Entry idstring="FE_OFF"></Entry></List>
<Block name="Invites"><Values idstring="ONL_VIEW_INV" x="480" y="410" width="520" selectable="true" Color="FEGlobals->HD_Grey" ActiveColor="FEGlobals->HD_Blue" textcolor="FEGlobals->HD_White"></Values></Block>
<Block name="Quit"><Values idstring="IG_PAUSE_QUIT" x="480" y="460" width="520" selectable="true" arrowcolor="0xffffffff" shaped="true" Color="FEGlobals->HD_Grey" ActiveColor="FEGlobals->HD_Blue" textcolor="FEGlobals->HD_White"></Values></Block>
</Screen>
"#;

const BLUE: u32 = 0xff8a_c0ca;
const GREY: u32 = 0xff64_6464;

const TABLE: &PauseMenu = &PauseMenu {
    heading: "IG_PAUSE_GPAUSED",
    screens: &[Screen {
        name: "InGame Pause SP Time Trial",
        rows: &[
            row("IG_PAUSE_CONT", Some(A::Resume)),
            row("FE_PILOT_ASSIST", Some(A::PilotAssist)),
            row("ONL_VIEW_INV", Some(A::Inert)),
            row("IG_PAUSE_QUIT", Some(A::Quit)),
        ],
    }],
    default_screen: "InGame Pause SP Time Trial",
    by_mode: &[],
    scrim_argb: 0xc000_0000,
    layout: None,
    hud_under_scrim: true,
    offers_photo: false,
    origin: Origin::Chosen,
};

fn words() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="IG_PAUSE_GPAUSED" String="GAME PAUSED"></Entry>
<Entry ID="IG_PAUSE_CONT" String="CONTINUE"></Entry>
<Entry ID="FE_PILOT_ASSIST" String="PILOT ASSIST"></Entry>
<Entry ID="FE_ON" String="ON"></Entry>
<Entry ID="FE_OFF" String="OFF"></Entry>
<Entry ID="ONL_VIEW_INV" String="VIEW INVITES"></Entry>
<Entry ID="IG_PAUSE_QUIT" String="QUIT RACE"></Entry>
<Entry ID="IG_PAUSE_RESU" String="RESUME GAME"></Entry>
</Strings>"#,
    )
}

fn screen() -> PauseScreen {
    let globals = [
        ("HD_Blue", "0xff8ac0ca"),
        ("HD_Grey", "0xff646464"),
        ("HD_White", "0xffffffff"),
    ];
    let screens = Screens::from_xml_with_fallback_globals(XML, &globals);
    PauseScreen::read(
        &screens,
        "InGame Pause SP Time Trial",
        Some("InGame Main Menu"),
        &words(),
        FaceScales::default(),
        [1920.0, 1080.0],
    )
    .expect("both screens are in the fixture")
}

fn menu() -> Menu {
    let mut definition = Definition::built_in(&StringTable::default()).expect("the built-in menu");
    definition.open_on_pause_screen(TABLE, "time_trial", &words());
    let mut menu = Menu::new(definition);
    menu.offer(Action::PhotoMode, true);
    menu.set_visible_rows(12);
    menu
}

fn skin() -> Skin {
    Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        22.0,
    )
}

fn frame() -> Frame {
    Frame {
        blocks: Some(BlockArt {
            frame: Sprite {
                x: 0,
                y: 0,
                width: 64,
                height: 64,
                quad_extent: None,
                quad_centre: None,
                blend: None,
            },
            fill_alpha: 110.0 / 255.0,
            solid_alpha: 1.0,
            cursor: None,
            arrow: Some(Sprite {
                x: 96,
                y: 0,
                width: 32,
                height: 32,
                quad_extent: None,
                quad_centre: None,
                blend: None,
            }),
            fury: false,
        }),
        ..Frame::default()
    }
}

fn draws(menu: &Menu) -> Layers {
    draw_list(&screen(), TABLE, menu, &skin(), &frame(), &|_| {
        Some(Sprite {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        })
    })
}

fn tinted(draws: &[Draw], argb: u32) -> Vec<[f32; 4]> {
    let [r, g, b, _] = argb_to_rgba(argb);
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, color, .. }
                if (color[0] - r).abs() < 1e-3
                    && (color[1] - g).abs() < 1e-3
                    && (color[2] - b).abs() < 1e-3 =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect()
}

fn text_at(draws: &[Draw], text: &str) -> Option<(f32, f32)> {
    draws.iter().find_map(|draw| match draw {
        Draw::Text { x, y, text: t, .. } | Draw::FacedText { x, y, text: t, .. } if t == text => {
            Some((*x, *y))
        }
        _ => None,
    })
}

/// The chrome is the shared screen's two rules and footer words and the screen's own title, at
/// the authored positions.
#[test]
fn the_chrome_is_the_disc_s_rules_title_and_footer_at_their_authored_places() {
    let layers = draws(&menu());
    let rules: Vec<[f32; 4]> = layers
        .chrome
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(
        rules,
        [[480.0, 292.0, 960.0, 4.0], [480.0, 790.0, 960.0, 4.0]]
    );
    assert_eq!(text_at(&layers.chrome, "RESUME GAME"), Some((525.0, 800.0)));
    assert!(
        layers.chrome.iter().any(
            |draw| matches!(draw, Draw::Sprite { rect, .. } if rect[0] == 480.0 && rect[1] == 251.0)
        ),
        "the title arrow"
    );
}

/// The focused row's box is the block's `ActiveColor` (the global `HD_Blue` resolves to) and the
/// others keep `Color`; the focused one is wider than it rests and carries the marker arrow.
#[test]
fn the_focused_row_is_the_active_colour_and_wider_than_the_others() {
    let mut menu = menu();
    for _ in 0..40 {
        menu.tick_focus(1.0 / 6.0);
    }
    let body = draws(&menu).body;
    let blue = tinted(&body, BLUE);
    assert!(!blue.is_empty(), "the focused CONTINUE block is HD_Blue");
    assert!(
        blue.iter().all(|rect| rect[1] >= 310.0 && rect[1] < 350.0),
        "only the CONTINUE row: {blue:?}"
    );
    let grey = tinted(&body, GREY);
    assert!(grey.iter().any(|rect| rect[1] >= 460.0), "QUIT rests grey");
    let widest = |rects: &[[f32; 4]], top: f32, bottom: f32| {
        rects
            .iter()
            .filter(|rect| rect[1] >= top - 1.0 && rect[1] < bottom)
            .map(|rect| rect[0] + rect[2])
            .fold(0.0_f32, f32::max)
    };
    assert!(
        (widest(&blue, 310.0, 350.0) - (480.0 + 520.0 + 60.0)).abs() < 1.0,
        "580 wide once the focus has settled"
    );
    assert!((widest(&grey, 460.0, 500.0) - (480.0 + 520.0)).abs() < 1.0);
    assert!(
        body.iter().any(|draw| matches!(draw, Draw::Sprite { rect, .. } if rect[0] == 488.0 && rect[1] == 314.0 && rect[2] == 32.0)),
        "the marker arrow at X + 8, Y + 4"
    );
}

/// A list row is a label box, a value box ten units right of it and two step arrows, and its
/// value is the menu row's current entry.
#[test]
fn a_list_row_draws_its_label_value_and_step_arrows() {
    let body = draws(&menu()).body;
    assert_eq!(
        text_at(&body, "PILOT ASSIST").map(|at| at.0),
        Some(480.0 + 40.0)
    );
    assert_eq!(
        text_at(&body, "ON").map(|at| at.0),
        Some(480.0 + 520.0 + 10.0 + 13.0)
    );
    let arrows = body
        .iter()
        .filter(
            |draw| matches!(draw, Draw::Sprite { rect, .. } if rect[2] == 32.0 && rect[1] == 364.0),
        )
        .count();
    assert_eq!(arrows, 2, "left and right");
}

/// VIEW INVITES is drawn, greyed, and is no target: the pointer's regions leave it out and the
/// others are the authored boxes.
#[test]
fn the_inert_row_is_drawn_greyed_and_has_no_pointer_region() {
    let menu = menu();
    let body = draws(&menu).body;
    assert!(text_at(&body, "VIEW INVITES").is_some(), "drawn");
    let regions = regions(&screen(), TABLE, &menu, &skin());
    let rows: Vec<usize> = regions.iter().map(|region| region.row).collect();
    let invites = menu
        .page()
        .entries
        .iter()
        .position(|entry| entry.label() == "VIEW INVITES")
        .expect("the row");
    assert!(!rows.contains(&invites), "{rows:?}");
    let first = regions
        .iter()
        .find(|region| region.row == 0)
        .expect("CONTINUE");
    assert_eq!(first.rect[0..2], [480.0, 310.0]);
    let steps = regions
        .iter()
        .filter(|region| matches!(region.part, Part::StepBack | Part::StepForward))
        .count();
    assert_eq!(steps, 2, "the list's two arrows");
}

/// A screen that authors one `<Menu>` and a band of fills, as Pure's do.
mod menu_widget {
    use super::*;
    use oag_title::{PauseLayout, PauseMenuRows};

    const BAND_XML: &str = r#"
<Screen name="InGame Main Menu">
<Image><Values x="0" y="75" width="480" height="120" color="0xc0000010"></Values></Image>
<Image><Values x="0" y="75" width="480" height="1" color="0xff3192bb"></Values></Image>
<Image><Values x="0" y="175" width="480" height="1" color="0xff3192bb"></Values></Image>
<Menu name="Option"><Values align="centre" scale="1.2" vertalign="middle" x="240" y="106" wrap="true" color="0xFFFFFFFF"></Values><Entry idstring="Continue"></Entry><Entry idstring="Restart"></Entry><Entry idstring="Quit"></Entry></Menu>
<Text><Values y="175" align="centre" x="240" idstring="Press X to Confirm" color="0xFFFFFFFF"></Values></Text>
</Screen>
"#;

    const BAND: &PauseMenu = &PauseMenu {
        heading: "Paused",
        screens: &[Screen {
            name: "InGame Main Menu",
            rows: &[
                row("Continue", Some(A::Resume)),
                row("Restart", Some(A::Restart)),
                row("Quit", Some(A::Quit)),
            ],
        }],
        default_screen: "InGame Main Menu",
        by_mode: &[],
        scrim_argb: 0,
        layout: Some(PauseLayout {
            definition: "InGame_Definition.xml",
            chrome: None,
            grid: [480, 272],
            menu_rows: Some(PauseMenuRows {
                pitch: 15.0,
                unfocused_alpha: 0.5,
            }),
        }),
        hud_under_scrim: true,
        offers_photo: false,
        origin: Origin::Measured,
    };

    fn band_words() -> StringTable {
        StringTable::from_xml(
            r#"<Strings>
<Entry ID="Continue" String="CONTINUE"></Entry>
<Entry ID="Restart" String="RESTART"></Entry>
<Entry ID="Quit" String="QUIT"></Entry>
<Entry ID="Press X to Confirm" String="PRESS X TO CONFIRM"></Entry>
</Strings>"#,
        )
    }

    fn band_screen() -> PauseScreen {
        let screens = Screens::from_xml_with_fallback_globals(BAND_XML, &[]);
        PauseScreen::read(
            &screens,
            "InGame Main Menu",
            None,
            &band_words(),
            FaceScales::default(),
            [480.0, 272.0],
        )
        .expect("the screen is in the fixture")
    }

    fn band_menu() -> Menu {
        let mut definition =
            Definition::built_in(&StringTable::default()).expect("the built-in menu");
        definition.open_on_pause_screen(BAND, "single_race", &band_words());
        Menu::new(definition)
    }

    fn band_skin() -> Skin {
        Skin::new(
            oag_pure::frontend::MENU_SKIN,
            oag_display::space::Space::PSP,
            14.0,
        )
    }

    fn band_draws(menu: &Menu) -> Layers {
        draw_list(
            &band_screen(),
            BAND,
            menu,
            &band_skin(),
            &Frame::default(),
            &|_| None,
        )
    }

    fn rows_of(draws: &[Draw]) -> Vec<(String, f32, [f32; 4])> {
        draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, y, color, .. } | Draw::FacedText { text, y, color, .. } => {
                    Some((text.clone(), *y, *color))
                }
                _ => None,
            })
            .collect()
    }

    /// The band is the screen's own fills, drawn once: no chrome screen stands behind it.
    #[test]
    fn the_band_and_its_rules_are_the_screens_own_fills_drawn_once() {
        let layers = band_draws(&band_menu());
        let fills: Vec<[f32; 4]> = layers
            .chrome
            .iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(
            fills,
            [
                [0.0, 75.0, 480.0, 120.0],
                [0.0, 75.0, 480.0, 1.0],
                [0.0, 175.0, 480.0, 1.0]
            ]
        );
        assert_eq!(
            text_at(&layers.chrome, "PRESS X TO CONFIRM"),
            Some((240.0, 175.0)),
            "the footer line"
        );
    }

    /// Every row of the page is drawn down the widget, centred on its `x`, `15 * scale` apart,
    /// and the one with the cursor is at full ink while the others are dimmed.
    #[test]
    fn every_row_is_drawn_down_the_menu_widget_with_the_cursor_row_brightest() {
        let mut menu = band_menu();
        for _ in 0..40 {
            menu.tick_focus(1.0 / 6.0);
        }
        let rows = rows_of(&band_draws(&menu).body);
        let words: Vec<&str> = rows.iter().map(|(text, _, _)| text.as_str()).collect();
        assert_eq!(words, ["CONTINUE", "RESTART", "QUIT"]);
        let pitch = rows[1].1 - rows[0].1;
        assert!((pitch - 18.0).abs() < 1e-3, "15 * 1.2: {pitch}");
        assert!((rows[2].1 - rows[1].1 - pitch).abs() < 1e-3);
        assert!(
            (rows[0].2[3] - 1.0).abs() < 1e-3,
            "the cursor row: {:?}",
            rows[0].2
        );
        assert!(
            (rows[1].2[3] - 0.5).abs() < 1e-3,
            "the others: {:?}",
            rows[1].2
        );
    }

    /// A row is a target the whole band wide and one pitch tall, at the row's own place.
    #[test]
    fn every_row_has_a_pointer_region_across_the_band() {
        let menu = band_menu();
        let regions = regions(&band_screen(), BAND, &menu, &band_skin());
        assert_eq!(regions.len(), 3);
        for (n, region) in regions.iter().enumerate() {
            assert_eq!(region.row, n);
            assert_eq!(region.rect[0], 0.0);
            assert_eq!(region.rect[2], 480.0);
            assert!((region.rect[3] - 18.0).abs() < 1e-3);
            assert!((region.rect[1] - (106.0 - 9.0 + 18.0 * n as f32)).abs() < 1e-3);
        }
        for (n, y) in [(0, 106.0), (1, 125.0), (2, 142.0)] {
            let hit = oag_ui::menu::pointer::hit(&regions, (240.0, y)).map(|region| region.row);
            assert_eq!(hit, Some(n), "a pointer at y {y} is on row {n}");
        }
        assert!(
            oag_ui::menu::pointer::hit(&regions, (240.0, 60.0)).is_none(),
            "above the band is no row"
        );
    }
}
