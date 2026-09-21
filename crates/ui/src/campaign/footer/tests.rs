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

/// `text`/`y`, off either variant - `ControlTextConfirm`/`ControlTextBack`
/// author no `font=` of their own, fall to `"default"` in
/// [`NavigationLegend::read`], and since `oag_game::boot::fonts::face_atlas_slot`
/// this draws as [`Draw::FacedText`] rather than [`Draw::Text`] - see
/// [`face_role`]. `ControlTextConfirmButton`/`BackButton` author
/// `font="small"`, which still routes to [`Draw::Text`].
fn text_and_y(draw: &Draw) -> (&str, f32) {
    match draw {
        Draw::Text { text, y, .. } => (text.as_str(), *y),
        Draw::FacedText { text, y, .. } => (text.as_str(), *y),
        _ => unreachable!("NavigationLegend::draw only ever emits Text or FacedText"),
    }
}

#[test]
fn the_navigation_controller_s_four_prompts_resolve_through_the_string_table() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings)
        .expect("Skin.xml authors a NavigationController");
    let draws = legend.draw(&FaceScales::default(), &|_| 100.0);
    assert_eq!(draws.len(), 4);
    let texts: Vec<&str> = draws.iter().map(|draw| text_and_y(draw).0).collect();
    assert_eq!(texts, ["ε", "Confirm", "γ", "Back"]);
    // `Confirm`/`Back` (the words, `font`-less in the fixture) route to the
    // `Default` role; their own button glyphs (`font="small"`) do not - see
    // [`face_role`].
    assert!(matches!(draws[0], Draw::Text { .. }), "ε is font=\"small\"");
    assert!(
        matches!(draws[1], Draw::FacedText { role, .. } if role == "Default"),
        "Confirm authors no font, falls to \"default\""
    );
    assert!(matches!(draws[2], Draw::Text { .. }), "γ is font=\"small\"");
    assert!(
        matches!(draws[3], Draw::FacedText { role, .. } if role == "Default"),
        "Back authors no font, falls to \"default\""
    );
    // Confirm sits right after its own button glyph, Back likewise - the
    // same row the campaign screen's own HELP/CHANGE DIFFICULTY draws at
    // (`y="252"`), matching `CellMode_Definition.xml`'s own footer.
    for draw in &draws {
        let (_, y) = text_and_y(draw);
        assert!((y - 252.0).abs() < f32::EPSILON);
    }
}

/// `Confirm` overlapping `Back`'s own button glyph at the authored,
/// left-aligned `x="368"` was measured live
/// (`data/scratch/lane-pulse/shots/crop-legend2-zoom.png`) - this pins the
/// fix: `Confirm` draws right-aligned, ending a few pixels short of the
/// back glyph's own `x="415"`, not left-aligned at its own authored start.
#[test]
fn confirm_ends_before_the_back_glyph_rather_than_starting_at_its_own_authored_x() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("controller");
    let draws = legend.draw(&FaceScales::default(), &|_| 100.0);
    // `FE_CONFIRM` authors no `font=`, falls to `"default"`, and now draws
    // as `Draw::FacedText` - see [`face_role`] and `text_and_y`'s own doc.
    let Draw::FacedText {
        x: confirm_x,
        align: confirm_align,
        ..
    } = &draws[1]
    else {
        unreachable!("index 1 is FE_CONFIRM - see the fixture's own document order");
    };
    assert_eq!(*confirm_align, Align::Right);
    assert!(
        *confirm_x < 415.0,
        "Confirm's own right edge ({confirm_x}) must end before the back glyph's x=415, \
         or nothing was fixed"
    );
    // `ControlTextBack`'s own word is untouched - it had room in the live
    // capture already, so it still starts at its own authored x. Also
    // `FacedText` now, the same reason `FE_CONFIRM` is.
    let Draw::FacedText {
        x: back_x,
        align: back_align,
        ..
    } = &draws[3]
    else {
        unreachable!("index 3 is FE_BACK");
    };
    assert_eq!(*back_align, Align::Left);
    assert_eq!(*back_x, 435.0);
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
    let draw = ticker_draw(&layout, 0.0, &[], &FaceScales::default(), &|_| 100.0);
    assert!(draw.is_none());
}

#[test]
fn one_tip_starts_flush_against_the_viewport_s_own_left_edge_at_zero_elapsed() {
    let layout = TickerLayout {
        viewport: [85.0, 235.0, 370.0, 32.0],
        font: "small".to_string(),
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let tips = vec!["Why don't you try out the Speed Lap events?".to_string()];
    let draw =
        ticker_draw(&layout, 0.0, &tips, &FaceScales::default(), &|_| 100.0).expect("one tip");
    let Draw::Text { x, y, text, .. } = &draw else {
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
    let at_zero = ticker_draw(&layout, 0.0, &tips, &FaceScales::default(), &measure).expect("t0");
    let at_one = ticker_draw(&layout, 1.0, &tips, &FaceScales::default(), &measure).expect("t1");
    let Draw::Text { x: x0, .. } = &at_zero else {
        unreachable!()
    };
    let Draw::Text { x: x1, .. } = &at_one else {
        unreachable!()
    };
    assert!((x0 - x1 - TICKER_SPEED).abs() < f32::EPSILON);
}

/// The clip mechanism this pass added needs an index into the *flattened*
/// draw list, not a value out of `ticker_draw` alone - `CampaignStage::ticker_draw`
/// (in `oag-game`) finds it by equality against the returned `Draw`, so a
/// tip's own draw has to compare equal to itself across two calls with the
/// same inputs for that lookup to work at all. Pins the assumption directly,
/// since `Draw` deriving `PartialEq` is a fact this module leans on rather
/// than states anywhere else.
#[test]
fn the_same_inputs_produce_an_equal_draw_for_the_index_lookup_to_find() {
    let layout = TickerLayout {
        viewport: [85.0, 235.0, 370.0, 32.0],
        font: "small".to_string(),
        color: [1.0, 1.0, 1.0, 1.0],
    };
    let tips = vec!["a fixed-width tip".to_string()];
    let measure = |_: &str| 100.0;
    let a = ticker_draw(&layout, 3.0, &tips, &FaceScales::default(), &measure);
    let b = ticker_draw(&layout, 3.0, &tips, &FaceScales::default(), &measure);
    assert_eq!(a, b);
}
