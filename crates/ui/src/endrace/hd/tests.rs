use super::*;
use crate::endrace::{FieldRow, HdRewards, Headline, MenuOption};
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
<Text name="Target Title" OffsetX="1180" OffsetY="532">
<Values idstring="IG_HUD_TARGET" x="50" y="65"></Values>
<Item OffsetX="50" OffsetY="105">
<Text name="Target0"><Values string="value" x="80" y="0"></Values></Text>
</Item>
</Text>
<Item OffsetX="1180" OffsetY="368">
<Text name="loyalty1.1"><Values string="834 POINTS" x="39" y="72"></Values></Text>
<Text name="loyalty2"><Values string="3745" x="352" y="115"></Values></Text>
</Item>
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
<Entry ID="ER_1PLACE" String="1ST PLACE"></Entry>
<Entry ID="ER_1STP" String="1ST PLACE!"></Entry>
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
    // The cell's own authored position (`x="0" y="0"`) must never reach the
    // screen - every Grid{col}.{row} cell is repositioned onto its real,
    // computed column/row, the bug this test would have missed had it only
    // checked the text content above.
    let place_draw = layers.body.iter().find_map(|draw| match draw {
        Draw::Text { x, y, text, .. } if text == "1" => Some((*x, *y)),
        _ => None,
    });
    assert_eq!(place_draw, Some((375.0, row_y(0))), "{:?}", layers.body);
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

/// `Line1`'s own resolved text for a finishing position uses HD's own
/// `ER_{n}PLACE` idstring, not Pulse's `ER_{n}STP` - a real, measured
/// divergence between the two titles' string tables (see
/// [`hd_headline_text`]'s own doc), and the placeholder/target/loyalty
/// widgets this build chose not to draw never leak through the generic
/// fallback arm.
#[test]
fn results_uses_hds_own_place_idstring_and_never_leaks_the_target_or_loyalty_placeholders() {
    let model = FieldResults {
        headline: Headline::Position(1),
        rows: vec![FieldRow {
            place: 1,
            time_ticks: Some(60),
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
    assert!(texts.contains(&"1ST PLACE".to_string()), "{texts:?}");
    assert!(
        !texts.contains(&"1ST PLACE!".to_string()),
        "Pulse's own ER_1STP text must not draw on HD: {texts:?}"
    );
    assert!(
        !texts.contains(&"value".to_string()),
        "Target0's own literal placeholder must not leak through: {texts:?}"
    );
    assert!(
        !texts.contains(&"834 POINTS".to_string()) && !texts.contains(&"3745".to_string()),
        "the loyalty block's own literal placeholders must not leak through: {texts:?}"
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

/// `DATA02`'s own `EndRace Rewards` in miniature - every named widget the
/// real screen authors, with its own placeholder text, so a placeholder the
/// draw list forgets to exclude shows up here rather than on screen.
const REWARDS_XML: &str = r#"
<Screen type="EndRace Rewards" name="EndRace Rewards">
<Image transition="0"><Values x="-288" y="-200" width="2496" height="1480" color="0xc0000000"></Values></Image>
<Text><Values idstring="ER_REWARD" font="Title" x="480" y="240"></Values></Text>
<Image transition="0"><Values x="480" y="292" width="960" height="4" color="0xffffffff"></Values></Image>
<Image name="MedalImg"><Values x="600" y="360" width="32" height="32" Color="0xff8AC0CA"></Values></Image>
<Image name="LoyaltyImg"><Values x="600" y="530" width="32" height="32" Color="0xff8AC0CA"></Values></Image>
<Text name="BigPos"><Values string="1" align="centre" font="title" x="610" y="370"></Values></Text>
<Text name="RewardLine1"><Values idstring="ER_MEDAL_AWARD" font="title" x="670" y="360"></Values></Text>
<Text name="RewardLine2"><Values string="test" font="title" x="670" y="530"></Values></Text>
<Text name="RewardLoyaltyPoints"><Values string="points!" font="title" x="1040" y="530"></Values></Text>
<Text name="RewardLoyaltyActive"><Values string="line 2" font="title" x="805" y="600"></Values></Text>
<Slider name="loyaltybar" default="0"><Values idstring="ER_TOT_LOY" x="805" y="600"></Values></Slider>
<Text name="ControlTextConfirm"><Values idstring="FE_CONFIRM" x="426" y="865"></Values></Text>
<Text name="EndRaceCountDown"><Values string="" x="960" y="40"></Values></Text>
</Screen>
"#;

fn rewards_strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="ER_REWARD" String="REWARDS"></Entry>
<Entry ID="ER_MEDAL_AWARD" String="MEDAL AWARDED:"></Entry>
<Entry ID="ER_GMA" String="GOLD MEDAL AWARDED"></Entry>
<Entry ID="ER_NMA" String="NO MEDAL AWARDED"></Entry>
<Entry ID="ER_TOT_LOY" String="TOTAL LOYALTY:"></Entry>
<Entry ID="FE_CONFIRM" String="CONFIRM"></Entry>
</Strings>"#,
    )
}

fn rewards_layers(model: &HdRewards) -> crate::menu::Layers {
    let strings = rewards_strings();
    let layout = Layout::read_authored(
        &Screens::from_xml(REWARDS_XML),
        "EndRace Rewards",
        &strings,
        crate::picker::FaceScales::default(),
        [1920.0, 1080.0],
        [1920.0, 1080.0],
    )
    .unwrap();
    hd_rewards_draw_list(
        model,
        &layout,
        &skin(),
        &Frame::default(),
        &strings,
        None,
        false,
        &|_| None,
    )
}

fn has_fill_at(layers: &crate::menu::Layers, x: f32, y: f32) -> bool {
    layers
        .body
        .iter()
        .any(|draw| matches!(draw, Draw::Fill { rect, .. } if rect[0] == x && rect[1] == y))
}

/// The place, not `BigPos`'s own authored `"1"`, and the medal tier on a
/// campaign race - with none of the loyalty row's placeholders and neither
/// src-less icon tile.
#[test]
fn rewards_draws_the_place_and_tier_and_no_loyalty_placeholder() {
    let layers = rewards_layers(&HdRewards {
        place: Some(3),
        medal: Some(oag_tables::race_campaign::Medal::Gold),
        campaign: true,
    });
    let drawn = texts(&layers);
    assert!(drawn.contains(&"REWARDS".to_string()), "{drawn:?}");
    assert!(drawn.contains(&"3".to_string()), "{drawn:?}");
    assert!(!drawn.contains(&"1".to_string()), "{drawn:?}");
    assert!(
        drawn.contains(&"GOLD MEDAL AWARDED".to_string()),
        "{drawn:?}"
    );
    assert!(drawn.contains(&"CONFIRM".to_string()), "{drawn:?}");
    for leak in [
        "test",
        "points!",
        "line 2",
        "TOTAL LOYALTY:",
        "MEDAL AWARDED:",
        "",
    ] {
        assert!(
            !drawn.contains(&leak.to_string()),
            "{leak:?} leaked: {drawn:?}"
        );
    }
    assert!(!has_fill_at(&layers, 600.0, 360.0), "MedalImg never draws");
    assert!(
        !has_fill_at(&layers, 600.0, 530.0),
        "LoyaltyImg never draws"
    );
}

/// No place draws neither `BigPos` nor its tile; no campaign cell draws no
/// medal line at all.
#[test]
fn rewards_without_a_place_or_a_campaign_draws_only_the_authored_labels() {
    let layers = rewards_layers(&HdRewards {
        place: None,
        medal: None,
        campaign: false,
    });
    let drawn = texts(&layers);
    assert_eq!(drawn, vec!["REWARDS".to_string(), "CONFIRM".to_string()]);
    assert!(!has_fill_at(&layers, 600.0, 360.0));
}

#[test]
fn rewards_on_a_campaign_race_with_no_medal_says_so() {
    let layers = rewards_layers(&HdRewards {
        place: Some(8),
        medal: None,
        campaign: true,
    });
    assert!(texts(&layers).contains(&"NO MEDAL AWARDED".to_string()));
}
