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
    song_line_under_scrim: true,
    offers_photo: false,
    music_list: None,
    music_list_ps2: None,
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
                scale: [1.0, 1.0],
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
                scale: [1.0, 1.0],
            }),
            marker: None,
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
            scale: [1.0, 1.0],
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
            touch: None,
        }),
        hud_under_scrim: true,
        song_line_under_scrim: true,
        offers_photo: false,
        music_list: None,
        music_list_ps2: None,
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

/// A screen of `<TouchButton>` tiles, as Wipeout 2048's pause screens are: the panel and heading in
/// the screen it is nested in, the tiles in the screen, a confirm screen nested in that.
mod touch_tiles {
    use super::*;
    use oag_core::buttons::{Button, Input};
    use oag_title::{PauseLayout, PauseTouch, pause_menu::named};
    use oag_ui::menu::PAUSE_PAGE;
    use oag_ui::menu::nav::Dir;

    const XML: &str = r#"
<Screen name="PauseTop">
<Image><Values x="0" y="15" width="960" height="510" Color="0x50ffffff"></Values></Image>
<Image><Values x="15" y="15" width="70" height="70" src="Data\FE\NewImages\Icon_pause_pausemenu.gtf"></Values></Image>
<Text><Values idstring="IG_PAUSE_GPAUSED" font="default" x="105" y="30" color="FEGlobals->Blue2048"></Values></Text>
<Screen name="PauseRace" type="Settings">
<TouchButton><Values idstring="IG_PAUSE_OPT" x="92" y="145" width="95" height="95" smallText="true" src="Data\FE\NewImages\Icon_options_pausemenu.gtf"></Values></TouchButton>
<TouchButton name="ghostButton"><Values idstring="CHOOSE_GHOST" x="92" y="275" width="95" height="95" src="Data\FE\NewImages\speedlap_ghost_icon.gtf"></Values></TouchButton>
<TouchButton name="quitButton"><Values redirect="QuitConfirm" x="560" y="430" width="120" height="95" Src="Data\FE\NewImages\Icon_Exit.gtf"></Values></TouchButton>
<TouchButton name="continueButton"><Values redirect="Race" x="820" y="430" width="120" height="95" Src="Data\FE\NewImages\Icon_Tick.gtf"></Values></TouchButton>
<Screen name="PauseGhost" type="Ghost">
<TouchButton><Values idstring="IG_PAUSE_OPT" x="92" y="145" width="95" height="95" src="Data\FE\NewImages\Icon_options_pausemenu.gtf"></Values></TouchButton>
<TouchButton><Values idstring="CHOOSE_GHOST" x="92" y="275" width="95" height="95" src="Data\FE\NewImages\speedlap_ghost_icon.gtf"></Values></TouchButton>
<Touchlist name="GhostOption2048" default="GHOST_MY_BEST"><Values string="GhostOption" x="430" y="145" width="510" height="225"></Values>
<Entry idstring="GHOST_NONE" src="Data\FE\NewImages\speedlap_NO_ghost.gtf"></Entry>
<Entry idstring="GHOST_MY_BEST" src="Data\FE\NewImages\speedlap_my_best.gtf"></Entry>
<Entry idstring="GHOST_FRIEND_CHALLENGE" src="Data\FE\NewImages\speedlap_friends_ghost.gtf"></Entry>
<Entry idstring="GHOST_GLOBAL_CHALLENGE" src="Data\FE\NewImages\speedlap_global_ghost.gtf"></Entry>
</Touchlist>
<TouchButton name="continueButton"><Values redirect="PauseRace" x="820" y="430" width="120" height="95" Src="Data\FE\NewImages\Icon_Tick.gtf"></Values></TouchButton>
</Screen>
<Screen name="QuitConfirm">
<Image><Values x="0" y="0" width="960" height="544" color="0x30000000"></Values></Image>
<Text><Values idstring="FE_QUIT" font="default" x="480" y="158" align="centre"></Values></Text>
<TouchButton><Values redirect="Kill" x="341" y="265" width="122" height="96" Src="Data\FE\NewImages\Icon_Tick.gtf"></Values></TouchButton>
<TouchButton><Values redirect="PreviousScreen" x="497" y="265" width="122" height="96" Src="Data\FE\NewImages\callout\cross.gtf"></Values></TouchButton>
</Screen>
</Screen>
</Screen>
"#;

    const BLUE: u32 = 0xff19_295d;
    const ORANGE: u32 = 0xffdd_580b;

    const TABLE: &PauseMenu = &PauseMenu {
        heading: "IG_PAUSE_GPAUSED",
        screens: &[
            Screen {
                name: "PauseRace",
                rows: &[
                    row("IG_PAUSE_OPT", Some(A::Options)),
                    row("CHOOSE_GHOST", None),
                    named("quitButton", "IG_PAUSE_QUIT", Some(A::Quit)).opening("QuitConfirm"),
                    named("continueButton", "IG_PAUSE_CONT", Some(A::Resume)),
                ],
            },
            Screen {
                name: "QuitConfirm",
                rows: &[
                    named("Kill", "FE_QUIT", Some(A::Quit)),
                    named("PreviousScreen", "FE_CANCEL", Some(A::Resume)),
                ],
            },
        ],
        default_screen: "PauseRace",
        by_mode: &[],
        scrim_argb: 0,
        layout: Some(PauseLayout {
            definition: "InGame_Definition.xml",
            chrome: None,
            grid: [960, 544],
            menu_rows: None,
            touch: Some(PauseTouch {
                tile: "Blue2048",
                cursor: "Orange2048",
                inactive: "Grey2048",
            }),
        }),
        hud_under_scrim: false,
        song_line_under_scrim: true,
        offers_photo: false,
        music_list: None,
        music_list_ps2: None,
        origin: Origin::Measured,
    };

    const GHOST_TABLE: &PauseMenu = &PauseMenu {
        screens: &[
            Screen {
                name: "PauseRace",
                rows: &[
                    row("IG_PAUSE_OPT", Some(A::Options)),
                    row("CHOOSE_GHOST", Some(A::Ghost)).opening("PauseGhost"),
                    named("continueButton", "IG_PAUSE_CONT", Some(A::Resume)),
                ],
            },
            Screen {
                name: "PauseGhost",
                rows: &[
                    row("IG_PAUSE_OPT", Some(A::Options)),
                    row("CHOOSE_GHOST", Some(A::Inert)),
                    named("GhostOption2048", "CHOOSE_GHOST", Some(A::GhostList)),
                    named("continueButton", "IG_PAUSE_CONT", Some(A::Resume)),
                ],
            },
        ],
        ..*TABLE
    };

    fn words() -> StringTable {
        StringTable::from_xml(
            r#"<Strings>
<Entry ID="IG_PAUSE_GPAUSED" String="GAME PAUSED"></Entry>
<Entry ID="IG_PAUSE_OPT" String="OPTIONS"></Entry>
<Entry ID="CHOOSE_GHOST" String="GHOST"></Entry>
<Entry ID="IG_PAUSE_QUIT" String="QUIT RACE"></Entry>
<Entry ID="IG_PAUSE_CONT" String="CONTINUE"></Entry>
<Entry ID="FE_QUIT" String="QUIT"></Entry>
<Entry ID="FE_CANCEL" String="CANCEL"></Entry>
<Entry ID="GHOST_NONE" String="NO GHOST"></Entry>
<Entry ID="GHOST_MY_BEST" String="MY BEST"></Entry>
<Entry ID="GHOST_FRIEND_CHALLENGE" String="FRIEND"></Entry>
<Entry ID="GHOST_GLOBAL_CHALLENGE" String="WORLD"></Entry>
</Strings>"#,
        )
    }

    fn read(name: &str) -> PauseScreen {
        let globals = [
            ("Blue2048", "0xff19295d"),
            ("Orange2048", "0xffdd580b"),
            ("Grey2048", "0xff808080"),
        ];
        let screens = Screens::from_xml_with_fallback_globals(XML, &globals);
        PauseScreen::read_touch(
            &screens,
            name,
            TABLE.layout.and_then(|layout| layout.touch).expect("tiles"),
            &words(),
            FaceScales::default(),
            [960.0, 544.0],
        )
        .expect("the screen is in the fixture")
    }

    fn menu() -> Menu {
        let mut definition =
            Definition::built_in(&StringTable::default()).expect("the built-in menu");
        definition.open_on_pause_screen(TABLE, "single_race", &words());
        let mut menu = Menu::new(definition);
        menu.set_visible_rows(12);
        menu
    }

    fn press(menu: &mut Menu, button: Button) -> Vec<oag_ui::menu::MenuEvent> {
        let mut input = Input::new();
        input.begin_frame(1 << button.index());
        menu.update(&mut input)
    }

    fn sprite(_: &str) -> Option<Sprite> {
        Some(Sprite {
            x: 0,
            y: 0,
            width: 40,
            height: 30,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        })
    }

    fn draw(screen: &PauseScreen, menu: &Menu) -> Layers {
        draw_list(screen, TABLE, menu, &skin(), &frame(), &sprite)
    }

    fn fills(draws: &[Draw]) -> Vec<([f32; 4], [f32; 4])> {
        draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, color } => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// Each tile of a screen that has a row is a box at its authored rectangle in the tile colour,
    /// the pad's cursor filling the focused one, with its icon at the texture's own size centred
    /// in it; the tile whose row this build has nothing behind draws nothing.
    #[test]
    fn a_tile_is_a_box_and_its_centred_icon_and_the_cursor_fills_the_focused_one() {
        let layers = draw(&read("PauseRace"), &menu());
        let blue = argb_to_rgba(BLUE);
        let orange = argb_to_rgba(ORANGE);
        assert_eq!(
            fills(&layers.body),
            [
                ([92.0, 145.0, 95.0, 95.0], orange),
                ([560.0, 430.0, 120.0, 95.0], blue),
                ([820.0, 430.0, 120.0, 95.0], blue),
            ],
            "no tile for GHOST, which this build has no row for; the first row has the cursor"
        );
        let icons: Vec<[f32; 4]> = layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(icons[0], [92.0 + 27.5, 145.0 + 32.5, 40.0, 30.0]);
        assert_eq!(icons.len(), 3);
    }

    /// A pointer over the tiles leaves them all at rest: the cursor fill is the pad's.
    #[test]
    fn a_pointer_in_use_shows_no_cursor_on_the_tiles() {
        let layers = draw_list_for(
            &read("PauseRace"),
            TABLE,
            &menu(),
            &skin(),
            &frame(),
            &sprite,
            false,
        );
        let blue = argb_to_rgba(BLUE);
        assert!(fills(&layers.body).iter().all(|(_, color)| *color == blue));
    }

    /// A tile that authors an `idstring` carries its words under the box; an icon-only tile none.
    #[test]
    fn only_a_tile_with_an_idstring_carries_words_under_it() {
        let layers = draw(&read("PauseRace"), &menu());
        let words: Vec<&str> = layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, ["OPTIONS"]);
    }

    /// `smalltext="true"` is the small label: 0.655 of the regular size, its cap line 13 under the
    /// box (measured on a Vita3K frame, `docs/ui/pause-menu.md`, "2048").
    #[test]
    fn a_small_label_stands_under_its_tile_at_the_measured_size_and_gap() {
        let layers = draw(&read("PauseRace"), &menu());
        let Some(Draw::Text { x, y, scale, .. }) = layers
            .body
            .iter()
            .find(|draw| matches!(draw, Draw::Text { text, .. } if text == "OPTIONS"))
        else {
            panic!("the label");
        };
        let scale_want = 16.0 / 27.0 * 0.655;
        assert!((scale - scale_want).abs() < 1e-4, "{scale}");
        assert_eq!(*x, 92.0 + 95.0 / 2.0, "centred on the tile");
        assert!(
            (y - (145.0 + 95.0 + 13.0 - 10.0 * scale_want)).abs() < 1e-3,
            "{y}"
        );
    }

    /// The panel, icon and heading are the nested-in screen's, behind the tiles.
    #[test]
    fn the_panel_and_heading_of_the_screen_it_is_nested_in_draw_behind_it() {
        let layers = draw(&read("PauseRace"), &menu());
        assert_eq!(
            fills(&layers.chrome),
            [([0.0, 15.0, 960.0, 510.0], argb_to_rgba(0x50ff_ffff))]
        );
        assert_eq!(text_at(&layers.chrome, "GAME PAUSED"), Some((105.0, 30.0)));
        assert!(
            layers
                .chrome
                .iter()
                .any(|draw| matches!(draw, Draw::Sprite { rect, .. } if rect[0] == 15.0))
        );
    }

    /// A tile is a pointer target at its box, and nothing else is.
    #[test]
    fn a_tile_is_a_pointer_target_at_its_authored_box() {
        let rects: Vec<[f32; 4]> = regions(&read("PauseRace"), TABLE, &menu(), &skin())
            .into_iter()
            .map(|region| region.rect)
            .collect();
        assert_eq!(
            rects,
            [
                [92.0, 145.0, 95.0, 95.0],
                [560.0, 430.0, 120.0, 95.0],
                [820.0, 430.0, 120.0, 95.0],
            ]
        );
    }

    /// QUIT asks first: its row opens the confirm page, whose tick does the quit and whose cross
    /// goes back; the page it asks on stays drawn under the confirm screen's own dimming and panel.
    #[test]
    fn quit_asks_on_the_confirm_screen_over_the_page_it_asks_on() {
        let mut menu = menu();
        let quit = menu
            .page()
            .meta
            .iter()
            .position(|meta| meta.id.as_deref() == Some("pause_quit"))
            .expect("a QUIT row");
        while menu.selected() != quit {
            press(&mut menu, Button::Down);
        }
        press(&mut menu, Button::Cross);
        assert_eq!(
            menu.page().id,
            format!("{}QuitConfirm", oag_ui::menu::SCREEN_PAGE_PREFIX),
            "QUIT opens the confirm page, it does not quit"
        );
        let ids: Vec<_> = menu
            .page()
            .meta
            .iter()
            .map(|meta| meta.id.as_deref())
            .collect();
        assert_eq!(ids, [Some("pause_quit"), Some("pause_resume")]);

        let layers = draw(&read("QuitConfirm"), &menu);
        let behind = fills(&layers.chrome);
        let blue = argb_to_rgba(BLUE);
        // The page asked on: its panel, then its tiles at rest, then the dimming over them.
        assert_eq!(behind[0].0, [0.0, 15.0, 960.0, 510.0]);
        assert_eq!(behind[1], ([92.0, 145.0, 95.0, 95.0], blue));
        assert_eq!(behind[2], ([560.0, 430.0, 120.0, 95.0], blue));
        assert_eq!(behind[3], ([820.0, 430.0, 120.0, 95.0], blue));
        assert_eq!(behind[4].0, [0.0, 0.0, 960.0, 544.0]);
        assert_eq!(
            fills(&layers.body),
            [
                ([341.0, 265.0, 122.0, 96.0], blue),
                ([497.0, 265.0, 122.0, 96.0], argb_to_rgba(ORANGE)),
            ],
            "the question opens with the cursor on the way back"
        );
        assert_eq!(text_at(&layers.chrome, "QUIT"), Some((480.0, 158.0)));

        press(&mut menu, Button::Circle);
        assert_eq!(
            menu.page().id,
            PAUSE_PAGE,
            "back from the question is the pause page"
        );

        while menu.selected() != quit {
            press(&mut menu, Button::Down);
        }
        press(&mut menu, Button::Cross);
        assert_eq!(
            menu.selected(),
            1,
            "it opens on the cross again, not where it was left"
        );
        press(&mut menu, Button::Up);
        let events = press(&mut menu, Button::Cross);
        assert_eq!(
            events,
            [oag_ui::menu::MenuEvent::Fired(Action::QuitRace)],
            "the tick quits"
        );
    }

    /// The pad moves between tiles by where they are: DOWN from OPTIONS to the tile under it, RIGHT
    /// along the foot row, nothing past an edge.
    #[test]
    fn the_pad_steps_between_tiles_by_position() {
        let screen = read("PauseRace");
        let mut menu = menu();
        let row_of = |menu: &Menu, id: &str| {
            menu.page()
                .meta
                .iter()
                .position(|meta| meta.id.as_deref() == Some(id))
                .expect(id)
        };
        let (options, quit, resume) = (
            row_of(&menu, "pause_options"),
            row_of(&menu, "pause_quit"),
            row_of(&menu, "pause_resume"),
        );
        let go = |menu: &Menu, dir| tile_neighbour(&screen, TABLE, menu, &skin(), dir);
        assert_eq!(menu.selected(), options);
        assert_eq!(go(&menu, Dir::Left), None);
        assert_eq!(go(&menu, Dir::Up), None);
        assert_eq!(
            go(&menu, Dir::Down),
            Some(quit),
            "no GHOST tile here, so the foot row"
        );
        menu.move_to_row(quit);
        assert_eq!(go(&menu, Dir::Right), Some(resume));
        assert_eq!(go(&menu, Dir::Up), Some(options));
        menu.move_to_row(resume);
        assert_eq!(go(&menu, Dir::Right), None);
        assert_eq!(go(&menu, Dir::Left), Some(quit));
    }

    /// Every texture the screen and the screens around and in it name is asked for.
    #[test]
    fn every_icon_the_screen_and_its_neighbours_name_is_asked_for() {
        let screen = read("PauseRace");
        let mut srcs: Vec<&str> = screen.sprite_srcs().collect();
        srcs.sort_unstable();
        srcs.dedup();
        assert_eq!(
            srcs,
            [
                r"Data\FE\NewImages\Icon_Exit.gtf",
                r"Data\FE\NewImages\Icon_Tick.gtf",
                r"Data\FE\NewImages\Icon_options_pausemenu.gtf",
                r"Data\FE\NewImages\Icon_pause_pausemenu.gtf",
                r"Data\FE\NewImages\speedlap_ghost_icon.gtf",
            ]
        );
    }

    fn ghost_menu() -> Menu {
        let mut definition =
            Definition::built_in(&StringTable::default()).expect("the built-in menu");
        definition.open_on_pause_screen(GHOST_TABLE, "single_race", &words());
        let mut menu = Menu::new(definition);
        menu.set_visible_rows(12);
        menu.seed(
            "display.ghost_ship_visible",
            &oag_ui::menu::Value::Text("on".into()),
        );
        menu
    }

    fn click(at: (f32, f32)) -> oag_ui::pointer::Pointer {
        oag_ui::pointer::Pointer {
            at: Some(at),
            moved: true,
            clicked: true,
            ..Default::default()
        }
    }

    /// The ghost tile opens the list on its own page, with the cursor on the list: NONE and MY
    /// BEST are the two values of `display.ghost_ship_visible`, and the friend's and the world's
    /// are drawn in the inactive colour and are no target.
    #[test]
    fn the_ghost_tile_opens_a_list_whose_two_unavailable_entries_cannot_be_chosen() {
        let mut menu = ghost_menu();
        let ghost = menu.page().row_with_id("pause_ghost").expect("a ghost row");
        while menu.selected() != ghost {
            press(&mut menu, Button::Down);
        }
        press(&mut menu, Button::Cross);
        assert_eq!(
            menu.page().id,
            format!("{}PauseGhost", oag_ui::menu::SCREEN_PAGE_PREFIX)
        );
        assert_eq!(
            menu.page().meta[menu.selected()].id.as_deref(),
            Some("pause_ghost_list"),
            "it opens on the list"
        );

        let screen = read("PauseGhost");
        let layers = draw_list(&screen, GHOST_TABLE, &menu, &skin(), &frame(), &sprite);
        let colours = fills(&layers.body);
        let (blue, orange, grey) = (
            argb_to_rgba(BLUE),
            argb_to_rgba(ORANGE),
            argb_to_rgba(0xff80_8080),
        );
        let cell = 510.0 / 4.0;
        let strip = |at: usize| [430.0 + cell * at as f32, 145.0, cell, 225.0];
        for (at, colour) in [(0, blue), (1, orange), (2, grey), (3, grey)] {
            assert!(
                colours.contains(&(strip(at), colour)),
                "entry {at} in its colour: {colours:?}"
            );
        }
        assert!(
            colours.contains(&([92.0, 275.0, 95.0, 95.0], orange)),
            "the ghost tile of the page being drawn is filled in the cursor's colour"
        );

        let regions = regions(&screen, GHOST_TABLE, &menu, &skin());
        let values: Vec<_> = regions
            .iter()
            .filter_map(|region| match region.part {
                Part::Value(at) => Some(at),
                _ => None,
            })
            .collect();
        assert_eq!(values, [0, 1], "two entries have a value to choose");

        let tap = |menu: &mut Menu, at: usize| {
            let centre = (strip(at)[0] + cell * 0.5, 250.0);
            menu.pointer(&click(centre), &regions)
        };
        assert_eq!(tap(&mut menu, 2), [], "the friend's ghost answers nothing");
        assert_eq!(tap(&mut menu, 3), [], "the world's neither");
        assert_eq!(
            tap(&mut menu, 0),
            [oag_ui::menu::MenuEvent::Changed {
                setting: "display.ghost_ship_visible".into(),
                value: oag_ui::menu::Value::Text("off".into()),
            }],
            "a tap on NO GHOST turns the ghost ship off"
        );
        assert_eq!(tap(&mut menu, 0), [], "and on the chosen one does nothing");
        assert_eq!(
            press(&mut menu, Button::Right),
            [oag_ui::menu::MenuEvent::Changed {
                setting: "display.ghost_ship_visible".into(),
                value: oag_ui::menu::Value::Text("on".into()),
            }],
            "the pad steps it back"
        );
        assert_eq!(
            tile_neighbour(&screen, GHOST_TABLE, &menu, &skin(), Dir::Right),
            None,
            "LEFT and RIGHT on the list are the menu's, they step it"
        );
        press(&mut menu, Button::Circle);
        assert_eq!(menu.page().id, PAUSE_PAGE, "back leaves the list");
    }
}
