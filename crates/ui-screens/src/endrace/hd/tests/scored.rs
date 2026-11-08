use super::*;
use crate::endrace::FieldScore;

const XML: &str = r#"
<Screen type="EndRace Results" name="EndRace Results">
<Item OffsetX="375" OffsetY="368">
<Block name="GridHead1"><Values x="0" y="24" IDString="IG_HUD_POS"></Values></Block>
<Block name="GridHead2"><Values x="505" y="24" IDString="IG_HUD_TIME"></Values></Block>
<Block name="GridHead3"><Values x="0" y="24" width="0" IDString="X"></Values></Block>
<Text name="Grid0.0"><Values x="0" y="0"></Values></Text>
<Text name="Grid1.0"><Values x="0" y="0"></Values></Text>
<Text name="Grid2.0"><Values x="0" y="0"></Values></Text>
<Text name="Grid0.1"><Values x="0" y="0"></Values></Text>
<Text name="Grid1.1"><Values x="0" y="0"></Values></Text>
<Text name="Grid2.1"><Values x="0" y="0"></Values></Text>
<Text name="Grid0.2"><Values x="0" y="0"></Values></Text>
<Text name="Grid2.2"><Values x="0" y="0"></Values></Text>
</Item>
<Text name="Line1"><Values string="race complete!" x="586" y="50"></Values></Text>
</Screen>
"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="IG_HUD_POS" String="POS"></Entry>
<Entry ID="ER_TEAM" String="TEAM"></Entry>
<Entry ID="ER_POINTS" String="POINTS"></Entry>
<Entry ID="IG_HUD_KILLS" String="KILLS"></Entry>
<Entry ID="ER_DEATHS" String="DEATHS:"></Entry>
<Entry ID="ER_2PLACE" String="2ND PLACE"></Entry>
<Entry ID="Feisar" String="FEISAR"></Entry>
<Entry ID="Harimau" String="HARIMAU"></Entry>
<Entry ID="Goteki" String="GOTEKI 45"></Entry>
</Strings>"#,
    )
}

fn layout() -> Layout {
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

fn row(index: u8, team: &str, points: u32, kills: u32, player: bool) -> FieldRow {
    FieldRow {
        place: index + 1,
        time_ticks: None,
        player,
        seat: None,
        name: None,
        team: Some(team.to_string()),
        score: Some(FieldScore {
            points,
            kills,
            deaths: 0,
        }),
    }
}

fn model(scores: [u32; 3]) -> FieldResults {
    FieldResults {
        loyalty: None,
        headline: Headline::Position(2),
        rows: vec![
            row(0, "Harimau", scores[0], 6, false),
            row(1, "Feisar", scores[1], 6, true),
            row(2, "Goteki", scores[2], 2, false),
        ],
    }
}

fn draw(model: &FieldResults) -> Vec<Draw> {
    hd_results_draw_list(
        model,
        &layout(),
        &skin(),
        &Frame::default(),
        &strings(),
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
fn a_points_table_draws_the_points_column_in_the_row_order_it_was_given() {
    let draws = draw(&model([202, 177, 176]));
    let points: Vec<(f32, f32)> = ["202", "177", "176"]
        .iter()
        .map(|points| text_at(&draws, points).expect(points))
        .collect();
    assert!(
        points.iter().all(|at| (at.0 - (375.0 + 556.0)).abs() < 1.0),
        "{points:?}"
    );
    assert!(
        points[0].1 < points[1].1 && points[1].1 < points[2].1,
        "{points:?}"
    );
    assert!(text_at(&draws, "POINTS").is_some(), "the POINTS header");
    assert!(text_at(&draws, "TEAM").is_some(), "the TEAM header");
    assert!(
        text_at(&draws, "TIME").is_none(),
        "no TIME header on a points table"
    );
}

#[test]
fn a_points_table_names_the_person_their_place_kills_and_deaths() {
    let draws = draw(&model([202, 177, 176]));
    assert!(
        draws.iter().any(|draw| matches!(draw,
            Draw::Text { text, .. } if text.trim_end() == "2ND PLACE - KILLS: 6 DEATHS: 0")),
        "{draws:?}"
    );
}

#[test]
fn rows_on_the_same_points_share_a_number() {
    let draws = draw(&model([177, 177, 90]));
    let ranks: Vec<&str> = draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, x, .. }
                if (*x - (375.0 + 40.0)).abs() < 1.0 && text.parse::<u32>().is_ok() =>
            {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(ranks, ["1", "1", "3"]);
}

#[test]
fn a_race_without_scores_draws_no_points_column() {
    let mut unscored = model([202, 177, 176]);
    for row in &mut unscored.rows {
        row.score = None;
        row.time_ticks = Some(600);
    }
    let draws = draw(&unscored);
    assert!(text_at(&draws, "POINTS").is_none());
    assert!(text_at(&draws, "202").is_none());
}

/// With several people in the race the original's filler writes `ER_RACE_COM2` and no one
/// person's line (`0x00229bf0`, `+0xe4 >= 2`): ours names no one's kills either.
#[test]
fn several_people_have_no_one_persons_kills_line() {
    let mut split = model([202, 177, 176]);
    split.headline = Headline::Several;
    let draws = draw(&split);
    assert!(
        !draws.iter().any(|draw| matches!(draw,
            Draw::Text { text, .. } if text.contains("KILLS"))),
        "{draws:?}"
    );
}
