//! [`NavigationLegend`]/[`TickerLayout`] against a miniature `Skin.xml`,
//! trimmed to the same shape the real file nests the two in: several
//! anonymous `<Screen>` wrappers deep, so a test that only worked one level
//! down would miss the real gap.

use super::*;
use crate::language::StringTable;
use oag_tables::fexml::parse;

const XML: &str = r#"
<Screen>
<Screen name="FE Screen">
<Screen>
<NavigationController name="NavigationController">
<Text name="ControlTextConfirmButton">
<Values idstring="FE_CONFIRM_BUTTON" font="small" x="348" y="252" color="0xffffffff"></Values>
</Text>
<Text name="ControlTextConfirm">
<Values idstring="FE_CONFIRM" x="368" y="252" color="0xffffffff"></Values>
</Text>
<Text name="ControlTextBackButton">
<Values idstring="FE_BACK_BUTTON" font="small" x="415" y="252" color="0xffffffff"></Values>
</Text>
<Text name="ControlTextBack">
<Values idstring="FE_BACK" x="435" y="252" color="0xffffffff"></Values>
</Text>
</NavigationController>
<Screen>
<TextInfo delay="1.6">
<Values type="tag" x="30" y="235" scale="1.0" color="FEGlobals->BackgroundColor"></Values>
<Text name="NewsItemTagText"></Text>
</TextInfo>
<Viewport name="TextInfoIsAlwaysLast" delay="1" OffsetX="85" OffsetY="235">
<Values width="370" height="32"></Values>
<TextInfo delay="1.6">
<Values type="bar" x="0" y="0" scale="1.0" color="FEGlobals->TitleColor"></Values>
<Text name="NewsItemBarText1"></Text>
<Text name="NewsItemBarText2"></Text>
</TextInfo>
</Viewport>
</Screen>
</Screen>
</Screen>
</Screen>
"#;

const ENTRIES: &str = r#"
<Entries>
<Entry ID="FE_CONFIRM_BUTTON" String="ε"></Entry>
<Entry ID="FE_CONFIRM" String="Confirm"></Entry>
<Entry ID="FE_BACK_BUTTON" String="γ"></Entry>
<Entry ID="FE_BACK" String="Back"></Entry>
</Entries>
"#;

fn globals() -> HashMap<String, String> {
    let mut globals = HashMap::new();
    globals.insert("BackgroundColor".to_string(), "0xff000000".to_string());
    globals.insert("TitleColor".to_string(), "0xffffffff".to_string());
    globals
}

#[test]
fn the_navigation_controller_s_four_prompts_resolve_through_the_string_table() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings)
        .expect("Skin.xml authors a NavigationController");
    let draws = legend.draw(&FaceScales::default());
    assert_eq!(draws.len(), 4);
    let texts: Vec<&str> = draws
        .iter()
        .map(|draw| match draw {
            Draw::Text { text, .. } => text.as_str(),
            _ => unreachable!("NavigationLegend::draw only ever emits Draw::Text"),
        })
        .collect();
    assert_eq!(texts, ["ε", "Confirm", "γ", "Back"]);
    // Confirm sits right after its own button glyph, Back likewise - the
    // same row the campaign screen's own HELP/CHANGE DIFFICULTY draws at
    // (`y="252"`), matching `CellMode_Definition.xml`'s own footer.
    for draw in &draws {
        let Draw::Text { y, .. } = draw else {
            continue;
        };
        assert!((*y - 252.0).abs() < f32::EPSILON);
    }
}

#[test]
fn a_file_with_no_navigation_controller_answers_none() {
    let root = parse("<Screen></Screen>");
    let strings = StringTable::default();
    assert!(NavigationLegend::read(&root, &globals(), &strings).is_none());
}

#[test]
fn the_ticker_s_viewport_resolves_its_offset_size_and_globals_colour() {
    let root = parse(XML);
    let layout = TickerLayout::read(&root, &globals()).expect("Skin.xml authors the bar viewport");
    assert_eq!(layout.viewport, [85.0, 235.0, 370.0, 32.0]);
    // `FEGlobals->TitleColor` resolved to `0xffffffff` above - opaque white.
    assert_eq!(layout.color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn no_tips_draws_nothing() {
    let layout = TickerLayout {
        viewport: [85.0, 235.0, 370.0, 32.0],
        font: "small".to_string(),
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let draws = ticker_draw(&layout, 0.0, &[], &FaceScales::default(), &|_| 100.0);
    assert!(draws.is_empty());
}

#[test]
fn one_tip_starts_flush_against_the_viewport_s_own_left_edge_at_zero_elapsed() {
    let layout = TickerLayout {
        viewport: [85.0, 235.0, 370.0, 32.0],
        font: "small".to_string(),
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let tips = vec!["Why don't you try out the Speed Lap events?".to_string()];
    let draws = ticker_draw(&layout, 0.0, &tips, &FaceScales::default(), &|_| 100.0);
    assert!(!draws.is_empty());
    let Draw::Text { x, y, text, .. } = &draws[0] else {
        unreachable!("ticker_draw only ever emits Draw::Text");
    };
    assert_eq!(*x, 85.0);
    assert_eq!(*y, 235.0);
    assert_eq!(text, &tips[0]);
}

#[test]
fn scrolling_moves_the_tip_left_at_ticker_speed() {
    let layout = TickerLayout {
        viewport: [85.0, 235.0, 370.0, 32.0],
        font: "small".to_string(),
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let tips = vec!["a fixed-width tip".to_string()];
    let measure = |_: &str| 100.0;
    let at_zero = ticker_draw(&layout, 0.0, &tips, &FaceScales::default(), &measure);
    let at_one = ticker_draw(&layout, 1.0, &tips, &FaceScales::default(), &measure);
    let Draw::Text { x: x0, .. } = &at_zero[0] else {
        unreachable!()
    };
    let Draw::Text { x: x1, .. } = &at_one[0] else {
        unreachable!()
    };
    assert!((x0 - x1 - TICKER_SPEED).abs() < f32::EPSILON);
}
