use crate::draw::draw_list;
use crate::tests::{context, strings};
use crate::{KillTag, Layout, Readout};
use oag_ui::frontend::Draw;

/// 2048's native `Elimination_HUD.xml` shape: a `SCORE` block, the position pair, the time
/// limit block and rows with no string.
const LAYOUT: &str = r#"
<Screen>
<Text name="ScoreTxt"><Values idstring="IG_HUD_SCORE" font="HUD" align="left" vertalign="top" scale="0.6" x="15" y="10"/></Text>
<Text name="Score"><Values font="HUD" align="left" vertalign="top" x="15" y="30"/></Text>
<Text name="PositionTxt"><Values idstring="IG_HUD_POS" font="HUD" align="right" vertalign="top" x="900" y="10"/></Text>
<Text name="Position"><Values font="HUD" align="right" vertalign="top" x="900" y="30"/></Text>
<Text name="TimeLimitTxt"><Values idstring="IG_HUD_TIME" font="HUD" align="left" vertalign="top" x="15" y="75"/></Text>
<Text name="PosTag0"><Values string="" scale="0.3" font="HUD" align="right" vertalign="middle" x="897" y="130"/></Text>
<Text name="PosTag1"><Values string="" scale="0.3" font="HUD" align="right" vertalign="middle" x="897" y="150"/></Text>
</Screen>
"#;

fn readout() -> Readout {
    Readout {
        score_rows: true,
        kill_target: 50,
        place: 3,
        kill_tags: vec![
            KillTag {
                team: "Qirex".into(),
                count: 27,
                player: false,
            },
            KillTag {
                team: "Feisar".into(),
                count: 18,
                player: true,
            },
        ],
        ..Readout::blank()
    }
}

/// `(text, scale, colour)` of everything drawn.
fn drawn(layout: &str, readout: &Readout) -> Vec<(String, f32, [f32; 4])> {
    let layout = Layout::from_xml(layout);
    let strings = strings();
    let cx = context(&layout, &strings);
    let list = draw_list(&cx, readout);
    list.hud_text
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text {
                text, scale, color, ..
            } => Some((text, scale, color)),
            _ => None,
        })
        .collect()
}

/// Fails if `Score` stays empty, if `POS` or `TIME` is drawn, or if the rows take HD's
/// `NAME  ( n )` and scale.
#[test]
fn the_header_is_the_score_over_its_target_and_the_rows_are_2048s() {
    let drawn = drawn(LAYOUT, &readout());
    let texts: Vec<&str> = drawn.iter().map(|(text, ..)| text.as_str()).collect();
    assert!(texts.contains(&"18/50"), "{texts:?}");
    assert!(texts.contains(&"Qirex (27)"), "{texts:?}");
    assert!(texts.contains(&"Feisar (18)"), "{texts:?}");
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("POS") || t.contains("3/8") || t.contains("TIME")),
        "{texts:?}"
    );
    let row = |text: &str| drawn.iter().find(|d| d.0 == text).expect("a row");
    assert!((row("Feisar (18)").1 - 0.6).abs() < 1e-6);
    assert!((row("Qirex (27)").1 - 0.5).abs() < 1e-6);
    let person = row("Feisar (18)").2;
    assert!(
        person[1] > person[0] * 4.0 && person[1] > person[2] * 4.0,
        "{person:?}"
    );
}

/// HD's score layout authors `PositionTxt2` and keeps its own header and rows.
#[test]
fn a_layout_with_a_target_line_is_left_alone() {
    let hd = LAYOUT.replace(
        "</Screen>",
        r#"<Text name="PositionTxt2"><Values idstring="IG_HUD_POS" font="HUD" x="900" y="100"/></Text></Screen>"#,
    );
    let drawn = drawn(&hd, &readout());
    let texts: Vec<&str> = drawn.iter().map(|(text, ..)| text.as_str()).collect();
    assert!(!texts.contains(&"18/50"), "{texts:?}");
    assert!(texts.iter().any(|t| t.ends_with("( 27 )")), "{texts:?}");
}
