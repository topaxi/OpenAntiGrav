//! The Zone and Eliminator tables, and the table walk they share.

use super::*;
use crate::endrace::draw::results_draw_list;
use crate::endrace::table::{highlight_y, row_of};
use crate::endrace::{Headline, LapSplit, Results};
use crate::frontend::Draw;
use crate::screen::Screens;

/// `EndRace_Definition.xml`'s `table` group in the shape the real file authors it:
/// eight `tablebg` rows, each an `<Image OffsetY>` nesting a tile and two rules, the
/// header bar, the 27 cells, the two icons and the highlight.
fn xml() -> String {
    let mut xml = String::from(
        r#"<Screen type="EndRace Results" name="EndRace Results">
<Text name="BigTopText"><Values idstring="ER_RES" x="240" y="19"></Values></Text>
<Text name="Line1"><Values string="race complete!" x="240" y="50"></Values></Text>
<Text name="table">
<Image name="zonetopline"><Values x="0" y="91" width="240" height="1" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff"></Values></Image>
"#,
    );
    for row in 1..=8 {
        let offset = 72 + 20 * row;
        let inner = if row == 1 { 0 } else { 1 };
        xml.push_str(&format!(
            r#"<Image name="tablebg{row}" OffsetY="{offset}">
<Values x="0" y="{inner}" width="480" height="19" Color1="0x9f000000" Color2="0x9f000000" Color3="0x9f000000" Color4="0x9f000000"></Values>
<Image><Values x="0" y="{inner}" width="480" height="19" U="0" V="0" TxtrWidth="960" TxtrHeight="43" Color="0x3fffffff" src="Data\FE\Images\hex_bg.mip"></Values></Image>
<Image><Values x="0" y="20" width="240" height="1" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff"></Values></Image>
<Image><Values x="240" y="20" width="240" height="1" Color1="0xffffffff" Color2="0xffffffff" Color3="0x00ffffff" Color4="0x00ffffff"></Values></Image>
</Image>
"#
        ));
    }
    xml.push_str(
        r#"<Image name="topbarcenter"><Values x="0" y="75" width="480" height="25" U="340" V="0" TxtrWidth="8" TxtrHeight="24" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
"#,
    );
    for row in 0..=8 {
        for (column, x) in [(0, 140), (1, 220), (2, 320)] {
            xml.push_str(&format!(
                "<Text name=\"lap{row}.{column}\"><Values x=\"{x}\" y=\"{}\"></Values></Text>\n",
                76 + 20 * row
            ));
        }
    }
    xml.push_str(
        r#"<Image name="perfectlap1"><Values x="352" y="95" width="16" height="16" src="Data\FE\Images\pulse_assets.mip"></Values><Text><Values idstring="MSC_PL" x="360" y="96"></Values></Text></Image>
<Image name="boostimg"><Values x="320" y="77" width="12" height="12" U="480" V="32" TxtrWidth="12" TxtrHeight="12" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
<Image name="tablehighlight"><Values x="0" y="20" width="480" height="19" RealGlow="128" Color="0x30009fff"></Values></Image>
</Text>
</Screen>"#,
    );
    xml
}

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="ER_RES" String="RESULTS"></Entry>
<Entry ID="RC_LAP" String="Lap"></Entry>
<Entry ID="PRO_TIME" String="Time"></Entry>
<Entry ID="PRO_STATS_TOT" String="Total"></Entry>
<Entry ID="ER_TT_COM" String="Time trial complete!"></Entry>
<Entry ID="ER_SL_COM" String="Speed lap complete!"></Entry>
<Entry ID="ER_DEATHS" String="Deaths:"></Entry>
<Entry ID="ER_TEAM" String="Team"></Entry>
<Entry ID="IG_HUD_KILLS" String="Kills"></Entry>
<Entry ID="ER_ELIM_COM" String="Eliminator complete - "></Entry>
<Entry ID="ER_8THP" String="8th place"></Entry>
<Entry ID="ER_ZONE_COM" String="Zone session complete!"></Entry>
<Entry ID="ER_ZONE_CLEAR" String="Total zones cleared:"></Entry>
<Entry ID="ER_PERF_ZONE" String="Perfect zones:"></Entry>
<Entry ID="ER_LAPSC" String="Laps cleared:"></Entry>
<Entry ID="MSC_DATA_PLAP" String="Perfect laps:"></Entry>
<Entry ID="ER_TOP_SPEED" String="Top speed:"></Entry>
<Entry ID="ER_ZONE_SCORE" String="Zone score:"></Entry>
<Entry ID="RC_KMH" String="KM/H"></Entry>
<Entry ID="AG_Systems" String="AG Systems"></Entry>
<Entry ID="Goteki" String="Goteki 45"></Entry>
</Strings>"#,
    )
}

fn layout() -> Layout {
    Layout::read(
        &Screens::from_xml(&xml()),
        "EndRace Results",
        &strings(),
        crate::picker::FaceScales::default(),
        [480.0, 272.0],
    )
    .expect("the fixture is a screen")
}

fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn placed() -> Option<Placed> {
    Some(Placed {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
        quad_extent: None,
        blend: None,
    })
}

fn texts(layers: &Layers) -> Vec<(String, f32, f32)> {
    layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, x, y, .. } => Some((text.clone(), *x, *y)),
            _ => None,
        })
        .collect()
}

fn text_list(layers: &Layers) -> Vec<String> {
    texts(layers).into_iter().map(|(text, _, _)| text).collect()
}

/// The rectangles of every non-text draw.
fn rects(layers: &Layers) -> Vec<[f32; 4]> {
    layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, .. }
            | Draw::GradientFill { rect, .. }
            | Draw::Sprite { rect, .. }
            | Draw::TiledSprite { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}

/// How many row-background widgets of each row are drawn: the gradient backing, the
/// tile and the two rules. The highlight is a plain fill and is asked for on its own.
fn per_row(layers: &Layers) -> [usize; 8] {
    let mut counts = [0; 8];
    for draw in &layers.body {
        let rect = match draw {
            Draw::GradientFill { rect, .. }
            | Draw::Sprite { rect, .. }
            | Draw::TiledSprite { rect, .. } => rect,
            _ => continue,
        };
        if let Some(row) = row_of(rect[1]) {
            counts[row - 1] += 1;
        }
    }
    counts
}

/// The `y` of the highlight, if one is drawn.
fn highlight(layers: &Layers) -> Option<f32> {
    layers.body.iter().find_map(|draw| match draw {
        Draw::Fill { rect, .. } if (rect[3] - 19.0).abs() < 0.01 && rect[2] == 480.0 => {
            Some(rect[1])
        }
        _ => None,
    })
}

fn craft(team: Option<&str>, kills: u32, deaths: u32, player: bool) -> EliminationRow {
    EliminationRow {
        team_name: team.map(str::to_string),
        kills,
        deaths,
        player,
    }
}

/// The live Eliminator race's eight records, in an arbitrary slot order: the player
/// (slot 0) and the seven AI. Read off `*(0x08b317b4) + 0x7d8` after the race ended.
fn live_field() -> Vec<EliminationRow> {
    vec![
        craft(Some("Assegai"), 0, 1, true),
        craft(Some("Feisar"), 2, 4, false),
        craft(Some("EG-X"), 5, 5, false),
        craft(Some("Qirex"), 3, 3, false),
        craft(Some("Goteki"), 3, 1, false),
        craft(Some("AG_Systems"), 2, 4, false),
        craft(Some("Triakis"), 1, 2, false),
        craft(Some("Piranha"), 4, 1, false),
    ]
}

/// The sort `Race_BuildEndRaceResult` runs, on the numbers PPSSPP showed: kills
/// descending, deaths ascending on a tie, the player last here because nothing
/// else has zero kills. `Feisar` and `AG Systems` tie on 2 kills and 4 deaths and
/// keep their slot order.
#[test]
fn the_field_is_ranked_by_kills_then_deaths_and_the_player_places_eighth() {
    let results = EliminationResults::new(live_field());
    let order: Vec<(u32, u32)> = results
        .rows()
        .iter()
        .map(|row| (row.kills, row.deaths))
        .collect();
    assert_eq!(
        order,
        [
            (5, 5),
            (4, 1),
            (3, 1),
            (3, 3),
            (2, 4),
            (2, 4),
            (1, 2),
            (0, 1)
        ]
    );
    assert_eq!(results.place(), Some(8));
    let teams: Vec<_> = results
        .rows()
        .iter()
        .map(|row| row.team_name.as_deref().unwrap())
        .collect();
    assert_eq!(
        teams[4..6],
        ["Feisar", "AG_Systems"],
        "an ordinary tie keeps the order the field came in"
    );
}

/// The swap the original makes when the record after is the player's, on a tie of
/// both numbers: the player moves ahead.
#[test]
fn the_player_wins_a_full_tie() {
    let results = EliminationResults::new(vec![
        craft(Some("Feisar"), 2, 1, false),
        craft(Some("Assegai"), 2, 1, true),
    ]);
    assert_eq!(results.place(), Some(1));
}

/// `Line1`, the header, and a row per craft with the team's display name, deaths in
/// the first column and kills in the third - the live frame's cell for cell.
#[test]
fn the_eliminator_table_draws_the_live_frames_cells() {
    let model = EliminationResults::new(live_field());
    let layers = elimination_results_draw_list(
        &model,
        &layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    let all = texts(&layers);
    let text = |wanted: &str| all.iter().find(|(text, _, _)| text == wanted).cloned();
    assert!(text("RESULTS").is_some(), "{all:?}");
    assert!(
        text("Eliminator complete -  8th place").is_some(),
        "the two spaces are the original's: {all:?}"
    );
    // Header: Deaths | Team | Kills at the authored columns.
    assert_eq!(text("Deaths:").map(|(_, x, _)| x), Some(140.0));
    assert_eq!(text("Team").map(|(_, x, _)| x), Some(220.0));
    assert_eq!(text("Kills").map(|(_, x, _)| x), Some(320.0));
    // Row 3 is `Goteki 45`, 3 kills and 1 death, and the folder id resolves to the
    // display name; a team with no string keeps its id.
    let row3: Vec<&str> = all
        .iter()
        .filter(|(_, _, y)| (*y - (76.0 + 20.0 * 3.0)).abs() < 0.01)
        .map(|(text, _, _)| text.as_str())
        .collect();
    assert_eq!(row3, ["1", "Goteki 45", "3"], "{all:?}");
    assert!(text("AG Systems").is_some(), "{all:?}");
    assert!(text("Assegai").is_some(), "{all:?}");
}

/// Eight rows, each with all four of its widgets, and the highlight on the player's
/// row at the step the executable uses.
#[test]
fn the_eliminator_table_shows_every_row_and_highlights_the_player() {
    let model = EliminationResults::new(live_field());
    let layers = elimination_results_draw_list(
        &model,
        &layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    assert_eq!(per_row(&layers), [4; 8]);
    assert_eq!(
        highlight(&layers),
        Some(highlight_y(8)),
        "the player is row 8"
    );
    assert!(
        !rects(&layers)
            .iter()
            .any(|rect| (rect[0], rect[1]) == (320.0, 77.0)),
        "`boostimg` belongs to the lap table alone"
    );
}

/// A field of three draws three rows' backgrounds and hides the other five - the
/// bit-`0x4` correction. Before it, every table drew all eight.
#[test]
fn a_short_field_hides_the_rows_it_does_not_use() {
    let model = EliminationResults::new(vec![
        craft(None, 1, 0, true),
        craft(None, 0, 1, false),
        craft(None, 0, 1, false),
    ]);
    let layers = elimination_results_draw_list(
        &model,
        &layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    assert_eq!(per_row(&layers), [4, 4, 4, 0, 0, 0, 0, 0]);
    // No team resolved: the cell draws absent, not a placeholder - the header's
    // own `Team` is the only text in that column.
    let team_column: Vec<String> = texts(&layers)
        .into_iter()
        .filter(|(_, x, _)| (*x - 220.0).abs() < 0.01)
        .map(|(text, _, _)| text)
        .collect();
    assert_eq!(team_column, ["Team"]);
}

fn zone(values: ZoneResults) -> Layers {
    zone_results_draw_list(
        &values,
        &layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    )
}

/// Six label/value rows, no header bar and no highlight; a statistic this build does
/// not keep draws its label and no value.
#[test]
fn the_zone_table_draws_six_rows_and_blanks_what_it_cannot_fill() {
    let layers = zone(ZoneResults {
        zones_cleared: 7,
        perfect_zones: None,
        laps_cleared: None,
        perfect_laps: None,
        top_speed_kmh: Some(812),
        score: 4321,
    });
    let all = texts(&layers);
    let labels: Vec<&str> = all
        .iter()
        .filter(|(_, x, _)| (*x - 140.0).abs() < 0.01)
        .map(|(text, _, _)| text.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            "Total zones cleared:",
            "Perfect zones:",
            "Laps cleared:",
            "Perfect laps:",
            "Top speed:",
            "Zone score:"
        ]
    );
    let values: Vec<(f32, &str)> = all
        .iter()
        .filter(|(_, x, _)| (*x - 320.0).abs() < 0.01)
        .map(|(text, _, y)| (*y, text.as_str()))
        .collect();
    // Rows 1, 5 and 6 only: the value column is the third.
    assert_eq!(
        values,
        [(96.0, "7"), (176.0, "812 KM/H"), (196.0, "4321")],
        "{all:?}"
    );
    assert!(
        all.iter()
            .any(|(text, _, _)| text == "Zone session complete!"),
        "{all:?}"
    );
    assert_eq!(per_row(&layers), [4, 4, 4, 4, 4, 4, 0, 0]);
    // The header bar is hidden and the header cells stay blank.
    assert!(
        !rects(&layers).iter().any(|rect| rect[1] == 75.0),
        "no `topbarcenter`"
    );
    assert!(
        !all.iter().any(|(_, _, y)| (*y - 76.0).abs() < 0.01),
        "{all:?}"
    );
}

/// The ordinary lap table on the same fixture: `boostimg` draws as the third
/// column's header, only the rows in use draw a background, and the highlight sits
/// on the totals row one pixel under it. Speed Lap has no totals row.
#[test]
fn the_lap_table_shows_boostimg_and_only_its_own_rows() {
    let results = |headline| Results {
        headline,
        laps: vec![
            LapSplit { lap: 1, ticks: 60 },
            LapSplit { lap: 2, ticks: 120 },
            LapSplit { lap: 3, ticks: 180 },
        ],
        total_ticks: 360,
    };
    let draw = |model: &Results| {
        results_draw_list(
            model,
            &layout(),
            &skin(),
            &Frame::default(),
            &strings(),
            None,
            false,
            &|_| placed(),
        )
    };

    let trial = draw(&results(Headline::TimeTrial));
    assert_eq!(per_row(&trial), [4, 4, 4, 4, 0, 0, 0, 0]);
    assert!(
        rects(&trial)
            .iter()
            .any(|rect| (rect[0], rect[1]) == (320.0, 77.0)),
        "`boostimg` is the third column's header icon"
    );
    assert_eq!(
        highlight(&trial),
        Some(highlight_y(4)),
        "the highlight sits on the totals row"
    );
    assert!(text_list(&trial).contains(&"Total".to_string()));

    let speed = draw(&results(Headline::SpeedLap));
    assert_eq!(per_row(&speed), [4, 4, 4, 0, 0, 0, 0, 0]);
    assert!(!text_list(&speed).contains(&"Total".to_string()));
    assert_eq!(highlight(&speed), None, "Speed Lap hides the highlight");
}

/// A race with no completed lap hides the whole `table` group.
#[test]
fn a_race_with_no_lap_hides_the_table() {
    let layers = results_draw_list(
        &Results {
            headline: Headline::NoPosition,
            laps: Vec::new(),
            total_ticks: 0,
        },
        &layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    assert!(rects(&layers).iter().all(|rect| row_of(rect[1]).is_none()));
    assert!(
        !rects(&layers)
            .iter()
            .any(|rect| rect[1] == 75.0 || rect[1] == 91.0 || rect[1] == 77.0),
        "no header bar, top rule or icon"
    );
    assert!(!text_list(&layers).contains(&"Lap".to_string()));
}
