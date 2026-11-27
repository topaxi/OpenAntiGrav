use crate::endrace::hd::hd_results_draw_list;
use crate::endrace::{DetonatorResults, FieldResults, Headline, ScoreRecord};
use oag_ui::frontend::Draw;
use oag_ui::language::StringTable;
use oag_ui::menu::Frame;
use oag_ui::screen::Screens;

use crate::endrace::Layout;

/// The real `EndRace Results` in miniature: the grid's `Item` at `(375, 368)`, six rows and the
/// score line of `Grid0`/`Grid1`, a column no Detonator row uses, and the frame.
const XML: &str = r#"
<Screen type="EndRace Results" name="EndRace Results">
<Item OffsetX="375" OffsetY="368">
<Block name="GridHead1"><Values x="0" y="24" IDString="IG_HUD_POS"></Values></Block>
<Image name="GridSideBarL"><Values x="0" y="44" width="4" height="100" color="0xff646464"></Values></Image>
<Image name="GridBottomBar"><Values x="0" y="500" width="791" height="4" color="0xff646464"></Values></Image>
<Image name="GridHighlight"><Values x="1" y="391" width="789" height="38" color="0xff0000ff"></Values></Image>
<Text name="Grid0.0"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid1.0"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid0.5"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid1.5"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid0.6"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid1.6"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid2.0"><Values x="0" y="0" scale="0.8" color="0xff646464"></Values></Text>
<Text name="Grid0.h"><Values x="0" y="0" scale="0.8" color="0xffffffff"></Values></Text>
<Text name="Grid1.h"><Values x="0" y="0" scale="0.8" color="0xffffffff"></Values></Text>
<Text name="RecValue"><Values string="NEW" x="120" y="37"></Values></Text>
<Block name="RecBlockBlue"><Values x="0" y="500" width="791" height="60" Color="0xff0000ff" ArrowColor="0xffffffff" unselectable="true"></Values></Block>
<Block name="RecBlockGrey"><Values x="0" y="500" width="791" height="60" Color="0xff777777"></Values></Block>
</Item>
<Text name="Line1"><Values string="race complete!" x="586" y="50"></Values></Text>
<Item OffsetX="1180" OffsetY="368">
<Text name="loyalty1.1"><Values string="834 POINTS" x="39" y="72"></Values></Text>
</Item>
</Screen>
"#;

fn strings(with_headline: bool) -> StringTable {
    StringTable::from_xml(&format!(
        r#"<Strings>
<Entry ID="IG_HUD_POS" String="POS"></Entry>
<Entry ID="ER_DETONATOR_STAGE" String="STAGE CLEARED"></Entry>
<Entry ID="PERCENT_MINES_CLEARED" String="MINES CLEARED"></Entry>
<Entry ID="PERCENT_BOMBS_CLEARED" String="BOMBS CLEARED"></Entry>
<Entry ID="ER_NUM_5_STREAKS" String="NUMBER OF 5 CHAINS"></Entry>
<Entry ID="ER_NUM_10_STREAKS" String="NUMBER OF 10 CHAINS"></Entry>
<Entry ID="ER_NUM_15_STREAKS" String="NUMBER OF 15 CHAINS"></Entry>
<Entry ID="ER_DETONATOR_SCORE" String="TOTAL SCORE"></Entry>
<Entry ID="ER_NSR" String="NEW SCORE RECORD"></Entry>
<Entry ID="ER_SR" String="SCORE RECORD"></Entry>
{}</Strings>"#,
        if with_headline {
            r#"<Entry ID="ER_ZDETONATOR_COM" String="DETONATOR SESSION COMPLETE!"></Entry>"#
        } else {
            ""
        }
    ))
}

fn layout() -> Layout {
    Layout::read_authored(
        &Screens::from_xml(XML),
        "EndRace Results",
        &strings(true),
        crate::picker::FaceScales::default(),
        [1920.0, 1080.0],
        [1920.0, 1080.0],
    )
    .unwrap()
}

fn skin() -> oag_ui::menu::Skin {
    oag_ui::menu::Skin::new(
        oag_hd::frontend::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn destroyed_in_stage_five() -> DetonatorResults {
    DetonatorResults {
        stage: 4,
        mines_percent: 100.0 * 230.0 / 1860.0,
        bombs: 3,
        chains: [2, 1, 0],
        score: 41_250,
        record: ScoreRecord::default(),
    }
}

fn model(results: Option<DetonatorResults>) -> FieldResults {
    FieldResults {
        detonator: results,
        loyalty: None,
        headline: Headline::Detonator,
        rows: Vec::new(),
    }
}

fn draw(results: Option<DetonatorResults>, strings: &StringTable) -> Vec<Draw> {
    hd_results_draw_list(
        &model(results),
        &layout(),
        &skin(),
        &Frame::default(),
        strings,
        None,
        false,
        &|_| None,
    )
    .body
}

fn text_at(draws: &[Draw], wanted: &str) -> Option<(f32, f32)> {
    draws.iter().find_map(|draw| match draw {
        Draw::Text { text, x, y, .. } if text == wanted => Some((*x, *y)),
        _ => None,
    })
}

#[test]
fn a_run_destroyed_in_stage_five_reads_four_of_fourteen() {
    let rows = destroyed_in_stage_five().rows();
    assert_eq!(rows[0], ("ER_DETONATOR_STAGE", "4 / 14".to_string()));
    assert_eq!(rows[1], ("PERCENT_MINES_CLEARED", "12.4%".to_string()));
    assert_eq!(rows[2], ("PERCENT_BOMBS_CLEARED", "3 / 14".to_string()));
    assert_eq!(rows[3].1, "2");
    assert_eq!(rows[4].1, "1");
    assert_eq!(rows[5], ("ER_NUM_15_STREAKS", "0".to_string()));
}

#[test]
fn a_finished_run_reads_fourteen_of_fourteen() {
    let finished = DetonatorResults {
        stage: 14,
        ..destroyed_in_stage_five()
    };
    assert_eq!(finished.rows()[0].1, "14 / 14");
}

#[test]
fn the_bombs_count_is_truncated_to_the_u16_the_original_stores() {
    let many = DetonatorResults {
        bombs: 65_537,
        ..destroyed_in_stage_five()
    };
    assert_eq!(many.rows()[2].1, "1 / 14");
}

#[test]
fn the_rows_sit_at_forty_and_five_ninety_on_the_grids_origin() {
    let draws = draw(Some(destroyed_in_stage_five()), &strings(true));
    let (label_x, label_y) = text_at(&draws, "STAGE CLEARED").expect("the stage label");
    let (value_x, value_y) = text_at(&draws, "4 / 14").expect("the stage value");
    assert_eq!((label_x, value_x), (375.0 + 40.0, 375.0 + 590.0));
    assert_eq!((label_y, value_y), (368.0 + 37.0, 368.0 + 37.0));
    let (_, last_y) = text_at(&draws, "NUMBER OF 15 CHAINS").expect("the last label");
    assert_eq!(last_y, 368.0 + 37.0 + 40.0 * 5.0);
}

#[test]
fn the_score_line_is_the_labelled_total_at_y_three_sixty() {
    let draws = draw(Some(destroyed_in_stage_five()), &strings(true));
    assert_eq!(text_at(&draws, "TOTAL SCORE"), Some((415.0, 368.0 + 360.0)));
    assert_eq!(text_at(&draws, "41250"), Some((965.0, 368.0 + 360.0)));
}

#[test]
fn rows_six_to_eight_and_the_other_columns_stay_blank() {
    let draws = draw(Some(destroyed_in_stage_five()), &strings(true));
    let texts: Vec<&str> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(!texts.contains(&"NEW"), "{texts:?}");
    // Rows 0 and 5 (label and value each), the score line, `Line1` and the record notice's text:
    // the miniature authors a row 6 and a `Grid2.0` that must stay blank.
    assert_eq!(texts.len(), 2 * 2 + 2 + 2, "{texts:?}");
}

#[test]
fn line_one_is_the_session_complete_text_and_blank_when_the_table_lacks_it() {
    let with = draw(Some(destroyed_in_stage_five()), &strings(true));
    assert!(text_at(&with, "DETONATOR SESSION COMPLETE!").is_some());
    let without = draw(Some(destroyed_in_stage_five()), &strings(false));
    assert!(text_at(&without, "ER_ZDETONATOR_COM").is_none());
    assert!(text_at(&without, "DETONATOR SESSION COMPLETE!").is_none());
}

#[test]
fn the_side_bars_and_the_bottom_bar_take_the_detonator_frame() {
    let draws = draw(Some(destroyed_in_stage_five()), &strings(true));
    let fills: Vec<[f32; 4]> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert!(
        fills.iter().any(|r| r[1] == 368.0 + 29.0 && r[3] == 347.0),
        "{fills:?}"
    );
    assert!(fills.iter().any(|r| r[1] == 368.0 + 391.0), "{fills:?}");
    assert!(
        fills.iter().all(|r| r[3] != 38.0),
        "no highlight row: {fills:?}"
    );
}

#[test]
fn the_loyalty_block_draws_beside_the_rows_as_it_does_on_every_other_mode() {
    let mut with = model(Some(destroyed_in_stage_five()));
    with.loyalty = Some(crate::endrace::HdLoyalty {
        award: 150,
        total: 4_020,
    });
    let draws = hd_results_draw_list(
        &with,
        &layout(),
        &skin(),
        &Frame::default(),
        &strings(true),
        None,
        false,
        &|_| None,
    )
    .body;
    let texts: Vec<&str> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.starts_with("150")), "{texts:?}");
}

#[test]
fn a_field_grid_model_draws_none_of_it() {
    let draws = draw(None, &strings(true));
    assert!(text_at(&draws, "STAGE CLEARED").is_none());
    assert!(text_at(&draws, "TOTAL SCORE").is_none());
}

/// The colour of the box the notice draws, where the miniature's blue and grey blocks differ:
/// every sprite of a block frame is tinted with the block's own `Color`.
fn box_colours(results: DetonatorResults) -> Vec<[u8; 3]> {
    let placed = oag_ui::frontend::Placed {
        x: 0,
        y: 0,
        width: 64,
        height: 64,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    };
    let frame = Frame {
        blocks: Some(oag_ui::menu::block::BlockArt {
            frame: placed,
            fill_alpha: 110.0 / 255.0,
            solid_alpha: 1.0,
            cursor: None,
            arrow: None,
            marker: None,
            fury: false,
        }),
        ..Frame::default()
    };
    let draws = hd_results_draw_list(
        &model(Some(results)),
        &layout(),
        &skin(),
        &frame,
        &strings(true),
        None,
        false,
        &|_| None,
    )
    .body;
    let colours: Vec<[u8; 3]> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { color, .. } => {
                let [r, g, b, _] = color.map(|c| (c * 255.0).round() as u8);
                Some([r, g, b])
            }
            _ => None,
        })
        .collect();
    assert!(!colours.is_empty(), "the box draws");
    colours
}

#[test]
fn a_new_record_draws_the_blue_box_and_the_new_score_record_text() {
    let beaten = DetonatorResults {
        record: ScoreRecord::judge(5600, 0),
        ..destroyed_in_stage_five()
    };
    let draws = draw(Some(beaten), &strings(true));
    assert!(text_at(&draws, "NEW SCORE RECORD 5600").is_some());
    let colours = box_colours(beaten);
    assert!(
        colours.iter().all(|c| *c == [0, 0, 255]),
        "blue: {colours:?}"
    );
}

#[test]
fn no_new_record_draws_the_grey_box_and_the_standing_best() {
    let kept = DetonatorResults {
        record: ScoreRecord::judge(100, 5600),
        ..destroyed_in_stage_five()
    };
    let draws = draw(Some(kept), &strings(true));
    assert!(text_at(&draws, "SCORE RECORD 5600").is_some());
    assert!(text_at(&draws, "NEW SCORE RECORD 5600").is_none());
    let colours = box_colours(kept);
    assert!(colours.iter().all(|c| *c == [0x77; 3]), "grey: {colours:?}");
}

/// An unselectable block with an `ArrowColor` carries the static marker at `(17, 13)`, 16 square;
/// the grey box, which authors none, carries none.
#[test]
fn an_unselectable_block_draws_its_marker_where_it_authors_an_arrow_colour() {
    let at = |results: DetonatorResults| -> Vec<[f32; 4]> {
        let placed = |x| oag_ui::frontend::Placed {
            x,
            y: 0,
            width: 64,
            height: 64,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        };
        let frame = Frame {
            blocks: Some(oag_ui::menu::block::BlockArt {
                frame: placed(0),
                fill_alpha: 110.0 / 255.0,
                solid_alpha: 1.0,
                cursor: None,
                arrow: None,
                marker: Some(placed(64)),
                fury: false,
            }),
            ..Frame::default()
        };
        hd_results_draw_list(
            &model(Some(results)),
            &layout(),
            &skin(),
            &frame,
            &strings(true),
            None,
            false,
            &|_| None,
        )
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 16.0 && rect[3] == 16.0 => Some(*rect),
            _ => None,
        })
        .collect()
    };
    let beaten = DetonatorResults {
        record: ScoreRecord::judge(5600, 0),
        ..destroyed_in_stage_five()
    };
    let screen = layout().screen;
    let block = screen
        .blocks
        .iter()
        .find(|block| block.name.as_deref() == Some("RecBlockBlue"))
        .expect("the blue box");
    assert!(
        at(beaten).contains(&[block.x + 17.0, block.y + 13.0, 16.0, 16.0]),
        "{:?}",
        at(beaten)
    );
    assert!(at(destroyed_in_stage_five()).is_empty());
}
