use super::*;
use crate::endrace::{FieldRow, Headline, MenuOption};
use crate::frontend::Draw;
use crate::language::StringTable;
use crate::menu::Frame;
use crate::screen::Screens;

/// `Data\Plugins\Frontend\Gui\EndRace_Definition.xml` in miniature - the
/// same container shape the real file authors (`<Block>` headers/options,
/// `Grid{col}.{row}` cells at `x="0" y="0"`), trimmed to two rows and four
/// menu options rather than ten and eight. See
/// `docs/formats/hd-endrace-screens.md` for the real file's own numbers.
const XML: &str = r#"
<Screen type="EndRace Results" name="EndRace Results">
<Item OffsetX="375" OffsetY="368">
<Block name="GridHead1"><Values x="0" y="24" IDString="IG_HUD_POS"></Values></Block>
<Block name="GridHead2"><Values x="505" y="24" IDString="IG_HUD_TIME"></Values></Block>
<Image name="GridHighlight"><Values x="1" y="391" width="789" height="38" color="0xff0000ff"></Values></Image>
<Text name="Grid0.0"><Values x="0" y="0" scale="0.8"></Values></Text>
<Text name="Grid1.0"><Values x="0" y="0" scale="0.8"></Values></Text>
<Text name="Grid0.1"><Values x="0" y="0" scale="0.8"></Values></Text>
<Text name="Grid1.1"><Values x="0" y="0" scale="0.8"></Values></Text>
<Text name="Gridp.0"><Values IDstring="ER_PERFECT" x="0" y="0"></Values></Text>
</Item>
<Text name="Line1"><Values string="race complete!" x="586" y="50"></Values></Text>
</Screen>
<Screen type="EndRace Menu" name="EndRace Menu">
<Block name="race_again"><Values IDString="ER_RACE_AGAIN" x="375" y="235" width="520"></Values></Block>
<Block name="return_to_grid"><Values IDString="ER_RETURN_GRID" x="375" y="285" width="520"></Values></Block>
<Block name="return_to_menu"><Values IDString="ER_RETURN_MENU" x="375" y="285" width="520"></Values></Block>
<Block name="view_again"><Values IDString="ER_VIEW_AGAIN" x="375" y="335" width="520"></Values></Block>
</Screen>
"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="IG_HUD_POS" String="POS"></Entry>
<Entry ID="IG_HUD_TIME" String="TIME"></Entry>
<Entry ID="ER_PERFECT" String="PERFECT"></Entry>
<Entry ID="ER_TT_COM" String="TIME TRIAL COMPLETE!"></Entry>
<Entry ID="ER_RACE_AGAIN" String="RACE AGAIN"></Entry>
<Entry ID="ER_RETURN_GRID" String="RETURN TO GRID"></Entry>
<Entry ID="ER_RETURN_MENU" String="RETURN TO MENU"></Entry>
<Entry ID="ER_VIEW_AGAIN" String="VIEW RESULTS AGAIN"></Entry>
</Strings>"#,
    )
}

fn results_layout() -> Layout {
    Layout::read_authored(
        &Screens::from_xml(XML),
        "EndRace Results",
        &strings(),
        crate::picker::FaceScales::default(),
        [1920.0, 1080.0],
        [1920.0, 1080.0],
    )
    .unwrap()
}

fn menu_layout() -> Layout {
    Layout::read_authored(
        &Screens::from_xml(XML),
        "EndRace Menu",
        &strings(),
        crate::picker::FaceScales::default(),
        [1920.0, 1080.0],
        [1920.0, 1080.0],
    )
    .unwrap()
}

fn skin() -> crate::menu::Skin {
    crate::menu::Skin::new(
        oag_hd::frontend::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn texts(layers: &crate::menu::Layers) -> Vec<String> {
    layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn grid_slot_matches_only_the_numeric_column_dot_row_names() {
    assert_eq!(grid_slot("Grid0.0"), Some((0, 0)));
    assert_eq!(grid_slot("Grid1.9"), Some((1, 9)));
    assert_eq!(grid_slot("GridHead1"), None);
    assert_eq!(grid_slot("Gridp.0"), None);
    assert_eq!(grid_slot("Grid0.h"), None);
}

#[test]
fn grid_cell_text_reads_place_and_time_and_nothing_past_column_one() {
    let model = FieldResults {
        headline: Headline::TimeTrial,
        rows: vec![
            FieldRow {
                place: 1,
                time_ticks: Some(180),
                player: true,
            },
            FieldRow {
                place: 2,
                time_ticks: None,
                player: false,
            },
        ],
    };
    assert_eq!(grid_cell_text(0, 0, &model).as_deref(), Some("1"));
    assert!(grid_cell_text(1, 0, &model).is_some());
    assert_eq!(grid_cell_text(0, 1, &model).as_deref(), Some("2"));
    assert_eq!(grid_cell_text(1, 1, &model).as_deref(), Some("-"));
    assert_eq!(grid_cell_text(2, 0, &model), None);
    assert_eq!(grid_cell_text(0, 5, &model), None);
}

#[test]
fn row_y_steps_down_from_the_grid_s_own_measured_top() {
    let first = row_y(0);
    let second = row_y(1);
    assert_eq!(first, GRID_ROW_AREA_TOP);
    assert!(second > first);
    // Eight rows fit inside the measured area without overflowing it.
    assert!(row_y(7) < GRID_ROW_AREA_TOP + GRID_ROW_AREA_HEIGHT);
}

/// Only the two captioned columns (position, time) draw real content -
/// `Gridp.0` (`ER_PERFECT`) never leaks through, and the headline resolves
/// off the reused idstring table rather than the XML's own placeholder.
#[test]
fn results_draws_only_the_two_captioned_columns_and_the_resolved_headline() {
    let model = FieldResults {
        headline: Headline::TimeTrial,
        rows: vec![FieldRow {
            place: 1,
            time_ticks: Some(180),
            player: true,
        }],
    };
    let strings = strings();
    let layers = hd_results_draw_list(
        &model,
        &results_layout(),
        &skin(),
        &Frame::default(),
        &strings,
        None,
        false,
        &|_| None,
    );
    let texts = texts(&layers);
    assert!(texts.contains(&"POS".to_string()), "{texts:?}");
    assert!(texts.contains(&"TIME".to_string()), "{texts:?}");
    assert!(texts.contains(&"1".to_string()), "{texts:?}");
    assert!(texts.contains(&"0.03.00".to_string()), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t.contains("PERFECT")),
        "Gridp.0 is not modelled and must not draw: {texts:?}"
    );
    assert!(
        !texts.contains(&"race complete!".to_string()),
        "the XML's own placeholder must not leak through unsubstituted"
    );
}

/// `GridHighlight` repositions onto the player's own row rather than
/// drawing at its authored default (the bottom edge of the grid).
#[test]
fn results_repositions_the_highlight_onto_the_player_s_row() {
    let model = FieldResults {
        headline: Headline::TimeTrial,
        rows: vec![
            FieldRow {
                place: 1,
                time_ticks: Some(60),
                player: false,
            },
            FieldRow {
                place: 2,
                time_ticks: Some(90),
                player: true,
            },
        ],
    };
    let strings = strings();
    let layers = hd_results_draw_list(
        &model,
        &results_layout(),
        &skin(),
        &Frame::default(),
        &strings,
        None,
        false,
        &|_| None,
    );
    let highlight_y = layers.body.iter().find_map(|draw| match draw {
        Draw::Fill { rect, .. } => Some(rect[1]),
        _ => None,
    });
    assert_eq!(highlight_y, Some(row_y(1)), "{:?}", layers.body);
}

/// The menu draws only the options the model actually lists, at each
/// option's own authored `<Block>` position - never a block the screen
/// authors but the model does not carry.
#[test]
fn menu_draws_only_the_options_the_model_actually_lists() {
    let menu = EndRaceMenu::new(vec![MenuOption::ReturnToGrid, MenuOption::RaceAgain], None);
    let strings = strings();
    let layers = hd_menu_draw_list(
        &menu,
        &menu_layout(),
        &skin(),
        &Frame::default(),
        &strings,
        None,
        false,
        &|_| None,
    );
    let texts = texts(&layers);
    assert!(texts.contains(&"RETURN TO GRID".to_string()), "{texts:?}");
    assert!(texts.contains(&"RACE AGAIN".to_string()), "{texts:?}");
    assert!(
        !texts.contains(&"RETURN TO MENU".to_string()),
        "on the screen but not in the model's own option list: {texts:?}"
    );
    assert!(
        !texts.contains(&"VIEW RESULTS AGAIN".to_string()),
        "on the screen but not in the model's own option list: {texts:?}"
    );
}

#[test]
fn menu_targets_are_built_only_for_options_the_model_lists_and_the_screen_authors() {
    let menu = EndRaceMenu::new(
        vec![MenuOption::RaceAgain, MenuOption::ViewResultsAgain],
        None,
    );
    let targets = hd_menu_targets(&menu, &menu_layout());
    assert_eq!(targets.len(), 2, "{targets:?}");
    assert_eq!(targets[0].index, 0);
    assert_eq!(targets[0].rect[0], 375.0);
    assert_eq!(targets[1].index, 1);
}
