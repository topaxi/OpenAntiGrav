//! A title's pause screen as a page: `super::super::definition::pause`.

use super::*;

use oag_title::{Origin, PauseAction, PauseMenu, PauseScreen, pause_menu::row};

const SCREENS: &PauseMenu = &PauseMenu {
    heading: "IG_PAUSE_GPAUSED",
    screens: &[
        PauseScreen {
            name: "Pause",
            rows: &[
                row("IG_PAUSE_CONT", Some(PauseAction::Resume)),
                row("OPT_GAME_OPT", Some(PauseAction::Options)),
                row("IG_PAUSE_GMAN", None),
                row("IG_PAUSE_REST", Some(PauseAction::Restart)),
                row("IG_PAUSE_QUIT", Some(PauseAction::Quit)),
            ],
        },
        PauseScreen {
            name: "Pause Tournament",
            rows: &[
                row("IG_PAUSE_CONT", Some(PauseAction::Resume)),
                row("EXT_PHOTO", Some(PauseAction::Photo)),
                row("IG_PAUSE_SKIP", None),
                row("IG_PAUSE_QUIT", Some(PauseAction::Quit)),
            ],
        },
    ],
    default_screen: "Pause",
    by_mode: &[("tournament", "Pause Tournament")],
    scrim_argb: 0xe800_0000,
    offers_photo: true,
    origin: Origin::Chosen,
};

fn words() -> crate::language::StringTable {
    crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="IG_PAUSE_GPAUSED" String="GAME PAUSED"></Entry>
             <Entry ID="IG_PAUSE_CONT" String="CONTINUE"></Entry>
             <Entry ID="OPT_GAME_OPT" String="GAME OPTIONS"></Entry>
             <Entry ID="EXT_PHOTO" String="PHOTO MODE"></Entry>
             <Entry ID="IG_PAUSE_REST" String="RESTART RACE"></Entry>
             <Entry ID="IG_PAUSE_QUIT" String="QUIT RACE"></Entry>
           </StringTable>"#,
    )
}

fn ids(definition: &Definition) -> Vec<String> {
    let page = definition.page(PAUSE_PAGE).expect("the pause page");
    page.meta
        .iter()
        .map(|meta| meta.id.clone().unwrap_or_default())
        .collect()
}

#[test]
fn the_pause_page_holds_the_rows_the_disc_authors_for_the_mode() {
    let mut definition = built_in();
    let built = definition.open_on_pause_screen(SCREENS, "time_trial", &words());
    assert_eq!(built.screen, "Pause");
    assert_eq!(built.unbuilt, ["IG_PAUSE_GMAN"]);
    assert_eq!(
        ids(&definition),
        [
            "pause_resume",
            "pause_options",
            // The screen authors no photo row: this project's row follows OPTIONS.
            PHOTO_ROW,
            "pause_restart",
            "pause_quit"
        ]
    );
    let mut menu = Menu::new(definition);
    // `Menu::new` parks PHOTO MODE until the host says a race is held.
    menu.offer(Action::PhotoMode, true);
    assert_eq!(menu.page().id, PAUSE_PAGE, "the menu opens on it");
    assert_eq!(menu.page().title, "GAME PAUSED");
    let labels: Vec<&str> = menu.page().entries.iter().map(Entry::label).collect();
    assert_eq!(
        labels,
        [
            "CONTINUE",
            "GAME OPTIONS",
            "PHOTO MODE",
            "RESTART RACE",
            "QUIT RACE"
        ]
    );

    let mut tournament = built_in();
    let built = tournament.open_on_pause_screen(SCREENS, "tournament", &words());
    assert_eq!(built.screen, "Pause Tournament");
    assert_eq!(built.unbuilt, ["IG_PAUSE_SKIP"]);
    assert_eq!(
        ids(&tournament),
        ["pause_resume", PHOTO_ROW, "pause_quit"],
        "the authored photo row is the one row, not two"
    );
}

#[test]
fn a_row_whose_disc_words_are_missing_is_left_off_and_named() {
    let mut definition = built_in();
    let none = crate::language::StringTable::default();
    let built = definition.open_on_pause_screen(SCREENS, "time_trial", &none);
    assert_eq!(
        built.unworded,
        [
            "IG_PAUSE_CONT",
            "OPT_GAME_OPT",
            "IG_PAUSE_REST",
            "IG_PAUSE_QUIT"
        ]
    );
    assert!(
        !ids(&definition).contains(&"pause_resume".to_string()),
        "no stand-in word is drawn for a row"
    );
}

#[test]
fn the_extras_photo_row_leaves_the_main_page_for_the_pause_page() {
    let mut definition = built_in_hd();
    let main = |definition: &Definition| {
        definition
            .page("main")
            .expect("the main page")
            .meta
            .iter()
            .filter(|meta| meta.id.as_deref() == Some(PHOTO_ROW))
            .count()
    };
    assert_eq!(
        main(&definition),
        1,
        "the extras attach it to the main page"
    );
    definition.open_on_pause_screen(SCREENS, "tournament", &words());
    assert_eq!(main(&definition), 0);
}

#[test]
fn start_resumes_from_the_pause_page_and_confirms_a_row_elsewhere() {
    let mut definition = built_in();
    definition.open_on_pause_screen(SCREENS, "time_trial", &words());
    let mut menu = Menu::new(definition);
    menu.set_pause_screen(true);
    assert_eq!(press(&mut menu, &[Button::Start]), [MenuEvent::Closed]);
    // `ACCEPT`, the cue the original plays for START on its pause screen.
    assert_eq!(menu.take_nav(), [crate::menu::nav::Nav::Accept]);

    // GAME OPTIONS: START there is a confirm, as the disc's `forward="start"` is the pause
    // screen's alone.
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options");
    assert!(!press(&mut menu, &[Button::Start]).contains(&MenuEvent::Closed));
}

/// The disc's pause screen authors `backward="none"`: CIRCLE on its root is silent and does
/// nothing (measured). ESCAPE, ours, and the CONTINUE row resume, each with `ACCEPT`.
#[test]
fn circle_on_the_pause_root_does_nothing_and_escape_and_continue_resume() {
    use crate::menu::nav::Nav;

    let mut definition = built_in();
    definition.open_on_pause_screen(SCREENS, "time_trial", &words());
    let mut menu = Menu::new(definition);
    menu.set_pause_screen(true);
    assert_eq!(press(&mut menu, &[Button::Circle]), []);
    assert_eq!(menu.take_nav(), [], "silent");
    assert_eq!(menu.back(), [MenuEvent::Closed]);
    assert_eq!(menu.take_nav(), [Nav::Accept]);
    // CONTINUE is a Back row.
    assert_eq!(press(&mut menu, &[Button::Cross]), [MenuEvent::Closed]);
    assert_eq!(menu.take_nav(), [Nav::Accept]);

    // Below the root CIRCLE is back as everywhere.
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options");
    let _ = menu.take_nav();
    press(&mut menu, &[Button::Circle]);
    assert_eq!(menu.page().id, PAUSE_PAGE);
    assert_eq!(menu.take_nav(), [Nav::Decline]);
}

/// Every row of the pause page has a region the pointer lands on, and a click on one fires
/// what the pad's CROSS on it does.
#[test]
fn a_click_on_a_pause_row_does_what_confirm_does() {
    let build = || {
        let mut definition = built_in();
        definition.open_on_pause_screen(SCREENS, "time_trial", &words());
        let mut menu = Menu::new(definition);
        menu.offer(Action::PhotoMode, true);
        menu
    };
    click_is_confirm(&build, &skin());
}

/// HD's own table on HD's menu tree, with its AUDIO OPTIONS row: the same equality.
#[test]
fn a_click_on_an_hd_pause_row_does_what_confirm_does() {
    let words = crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="IG_PAUSE_GPAUSED" String="GAME PAUSED"></Entry>
             <Entry ID="IG_PAUSE_CONT" String="CONTINUE"></Entry>
             <Entry ID="OPT_GAME_OPT" String="GAME OPTIONS"></Entry>
             <Entry ID="FE_AUDIO_OPT" String="AUDIO OPTIONS"></Entry>
             <Entry ID="MSC_PHOTO_PHOTOMODE" String="PHOTO MODE"></Entry>
             <Entry ID="IG_PAUSE_REST" String="RESTART RACE"></Entry>
             <Entry ID="IG_PAUSE_QUIT" String="QUIT RACE"></Entry>
           </StringTable>"#,
    );
    let build = || {
        let mut definition = built_in_hd();
        let built = definition.open_on_pause_screen(
            oag_hd::TITLE.pause_menu.expect("HD authors pause screens"),
            "time_trial",
            &words,
        );
        assert!(built.unworded.is_empty(), "{:?}", built.unworded);
        let mut menu = Menu::new(definition);
        menu.offer(Action::PhotoMode, true);
        menu
    };
    let labels: Vec<String> = build()
        .page()
        .entries
        .iter()
        .map(|entry| entry.label().to_string())
        .collect();
    assert_eq!(
        labels,
        [
            "CONTINUE",
            "GAME OPTIONS",
            "AUDIO OPTIONS",
            "PHOTO MODE",
            "RESTART RACE",
            "QUIT RACE"
        ]
    );
    let hd_skin = Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        22.0,
    );
    click_is_confirm(&build, &hd_skin);
}

fn click_is_confirm(build: &dyn Fn() -> Menu, skin: &Skin) {
    use crate::menu::pointer::{self, Part, hit};
    use crate::pointer::Pointer;

    let rows = build().page().entries.len();
    for row in 0..rows {
        let mut clicked = build();
        let regions = pointer::regions(&clicked, skin, &Frame::default(), &measure);
        let region = regions
            .iter()
            .find(|region| region.row == row && region.part == Part::Row)
            .unwrap_or_else(|| panic!("row {row} has a region"));
        let at = (region.rect[0] + 1.0, region.rect[1] + 1.0);
        assert_eq!(hit(&regions, at).map(|hit| hit.row), Some(row));
        let click = Pointer {
            at: Some(at),
            moved: true,
            clicked: true,
            ..Pointer::default()
        };
        let events = clicked.pointer(&click, &regions);

        let mut confirmed = build();
        for _ in 0..row {
            press(&mut confirmed, &[Button::Down]);
        }
        assert_eq!(events, press(&mut confirmed, &[Button::Cross]), "row {row}");
        assert!(
            !events.is_empty() || ["options", "audio"].contains(&clicked.page().id.as_str()),
            "row {row} did something"
        );
    }
}

#[test]
fn an_audio_row_opens_the_audio_page_and_the_pause_page_is_a_column_not_a_strip() {
    const AUDIO: &PauseMenu = &PauseMenu {
        heading: "IG_PAUSE_GPAUSED",
        screens: &[PauseScreen {
            name: "Pause",
            rows: &[
                row("IG_PAUSE_CONT", Some(PauseAction::Resume)),
                row("FE_AUDIO_OPT", Some(PauseAction::Audio)),
                row("IG_PAUSE_QUIT", Some(PauseAction::Quit)),
            ],
        }],
        default_screen: "Pause",
        by_mode: &[],
        scrim_argb: 0xc000_0000,
        offers_photo: false,
        origin: Origin::Chosen,
    };
    let words = crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="IG_PAUSE_GPAUSED" String="GAME PAUSED"></Entry>
             <Entry ID="IG_PAUSE_CONT" String="CONTINUE"></Entry>
             <Entry ID="FE_AUDIO_OPT" String="AUDIO OPTIONS"></Entry>
             <Entry ID="IG_PAUSE_QUIT" String="QUIT RACE"></Entry>
           </StringTable>"#,
    );
    let mut definition = built_in();
    let audio = definition
        .pages
        .iter()
        .position(|page| page.id == "audio")
        .expect("the AUDIO page");
    let built = definition.open_on_pause_screen(AUDIO, "single_race", &words);
    assert!(built.unbuilt.is_empty(), "{:?}", built.unbuilt);
    let page = definition.page(PAUSE_PAGE).expect("the pause page");
    assert!(
        matches!(page.entries[1], Entry::Submenu { target, .. } if target == audio),
        "AUDIO OPTIONS opens the AUDIO page"
    );
    assert!(
        !super::super::strip::suits(page),
        "every row is navigation, and the disc authors this screen as a column on HD"
    );
}
