//! [`NavigationLegend`]/[`TickerLayout`] against a miniature `Skin.xml`,
//! trimmed to the same shape the real file nests the two in: several
//! anonymous `<Screen>` wrappers deep, so a test that only worked one level
//! down would miss the real gap.

use super::*;
use oag_tables::fexml::parse;
use oag_ui::language::StringTable;

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
/// left-aligned `x="368"` was measured live - this pins the
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

/// Wipeout HD/Fury's own shape: the shared `BodgeScreenContainingNavigationController`
/// (`Data\Plugins\Frontend\Gui\Skin.xml`, real widget name, confirmed by
/// direct read) carries six `Text` children, not four - `Confirm`/`Back`
/// plus an online `ControlTextInviteButton`/`ControlTextInvite` pair
/// (`StartEnabled="false"` on disc, irrelevant here regardless), and its own
/// icon halves author `font="buttons"` rather than Pulse's `font="small"`.
/// Trimmed to the controller alone - `NavigationLegend::read` does not care
/// what encloses it - unlike `XML` above, which pins the real nesting depth
/// too.
const HD_XML: &str = r#"
<Screen name="BodgeScreenContainingNavigationController">
<NavigationController name="NavigationController">
<Text name="ControlTextConfirmButton">
<Values idstring="FE_CONFIRM_BUTTON" font="buttons" x="490" y="988" scale="0.8"></Values>
</Text>
<Text name="ControlTextConfirm">
<Values idstring="FE_CONFIRM" x="534" y="994" scale="0.8"></Values>
</Text>
<Text name="ControlTextBackButton">
<Values idstring="FE_BACK_BUTTON" font="buttons" x="720" y="988" scale="0.8"></Values>
</Text>
<Text name="ControlTextBack">
<Values idstring="FE_BACK" x="764" y="994" scale="0.8"></Values>
</Text>
<Text name="ControlTextInviteButton" StartEnabled="false">
<Values string="X" font="buttons" x="1094" y="988" scale="0.8"></Values>
</Text>
<Text name="ControlTextInvite" StartEnabled="false">
<Values idstring="FE_COMMUNITY" x="1094" y="994" scale="0.8"></Values>
</Text>
</NavigationController>
</Screen>
"#;

/// The `EndRace Results` shape: a *local* controller (one screen's own, not
/// the shared root) that also carries an online-only `RecordsCycle` pair
/// beside `Confirm` - real widget names, off `EndRace_Definition.xml`
/// directly. No `Back`
/// at all on this screen, unlike the shared Bodge controller above.
const HD_RESULTS_XML: &str = r#"
<NavigationController name="NavigationController">
<Text name="ControlTextConfirmButton">
<Values idstring="FE_CONFIRM_BUTTON" font="buttons" x="376" y="890" scale="0.8"></Values>
</Text>
<Text name="ControlTextConfirm">
<Values idstring="FE_CONFIRM" x="426" y="892"></Values>
</Text>
<Text name="RecordsCycleButton">
<Values string="d" font="buttons" x="776" y="890" scale="0.8"></Values>
</Text>
<Text name="RecordsCycle">
<Values idstring="ER_GLOB_REC" x="826" y="892"></Values>
</Text>
</NavigationController>
"#;

/// The name filter added alongside Wipeout HD/Fury reuse: only the four
/// `ControlText*` widgets `NavigationLegend` knows resolve, not "any `Text`
/// with an idstring" - `RecordsCycle` (idstring `ER_GLOB_REC`) is the widget
/// that finding exists to exclude. Two draws, not four: `Confirm`'s own icon
/// (`font="buttons"`) plus its word - `RecordsCycleButton`/`RecordsCycle`
/// drop regardless of the icon fix, by name, same as ever.
#[test]
fn a_local_controllers_online_only_records_cycle_pair_never_resolves() {
    let root = parse(HD_RESULTS_XML);
    let strings = StringTable::from_xml(
        r#"<Entries><Entry ID="FE_CONFIRM_BUTTON" String="e"></Entry><Entry ID="FE_CONFIRM" String="CONFIRM"></Entry><Entry ID="ER_GLOB_REC" String="RECORDS"></Entry></Entries>"#,
    );
    let legend =
        NavigationLegend::read(&root, &HashMap::new(), &strings).expect("controller present");
    let draws = legend.draw(&FaceScales::default(), &|_| 100.0);
    assert_eq!(draws.len(), 2);
    let texts: Vec<&str> = draws.iter().map(|draw| text_and_y(draw).0).collect();
    assert_eq!(texts, ["e", "CONFIRM"]);
}

/// Wipeout HD/Fury's own icon glyph (`font="buttons"`) draws now, through
/// its own `Buttons`-role atlas - see [`face_role`]'s own doc for the
/// verification this rests on (`oag-tools --example hd_buttons_font_probe`
/// decoding real circled cross/circle glyph art at these codepoints). The
/// four remaining prompts - `Confirm`/`Back`'s own icon-then-word pairs,
/// `Invite`'s pair excluded by the name filter regardless - resolve to all
/// four, icon before word, matching document order.
#[test]
fn wipeout_hds_own_buttons_font_icon_and_word_both_draw() {
    let root = parse(HD_XML);
    let strings = StringTable::from_xml(
        r#"<Entries><Entry ID="FE_CONFIRM_BUTTON" String="e"></Entry><Entry ID="FE_CONFIRM" String="CONFIRM"></Entry><Entry ID="FE_BACK_BUTTON" String="g"></Entry><Entry ID="FE_BACK" String="BACK"></Entry><Entry ID="FE_COMMUNITY" String="INVITE"></Entry></Entries>"#,
    );
    let legend =
        NavigationLegend::read(&root, &HashMap::new(), &strings).expect("controller present");
    let draws = legend.draw(&FaceScales::default(), &|_| 100.0);
    let texts: Vec<&str> = draws.iter().map(|draw| text_and_y(draw).0).collect();
    assert_eq!(texts, ["e", "CONFIRM", "g", "BACK"]);
    // The icon draws through the `Buttons` role, not a fallback.
    let Draw::FacedText { role, scale, .. } = &draws[0] else {
        unreachable!("FE_CONFIRM_BUTTON authors font=\"buttons\"");
    };
    assert_eq!(*role, "Buttons");
    assert!(
        (*scale - 0.8).abs() < f32::EPSILON,
        "the icon's own scale=\"0.8\" must reach the draw, got {scale}"
    );
    // And the authored `scale="0.8"` on the word half reaches the draw -
    // `face_scale`'s own font-keyed answer for `"default"` is `1.0`, so a
    // `1.0` result here would mean the widget's own `scale=` was silently
    // dropped rather than multiplied in.
    let Draw::FacedText { scale, .. } = &draws[1] else {
        unreachable!("CONFIRM authors no font, falls to \"default\" - see text_and_y's own doc");
    };
    assert!(
        (*scale - 0.8).abs() < f32::EPSILON,
        "the widget's own scale=\"0.8\" must reach the draw, got {scale}"
    );
}

/// **The Pulse-only shrink-to-fit block does not fire for HD.** Once
/// `FE_CONFIRM_BUTTON`'s `font="buttons"` no longer excludes it from
/// `prompts`, both `FE_BACK_BUTTON` and `FE_CONFIRM_BUTTON` are present and
/// the shrink/right-align block (`read`'s own `GAP`/`GLYPH_WIDTH_ESTIMATE`
/// note) would trigger on HD too if it were not gated - moving `CONFIRM`
/// off its authored `x="534"` and right-aligning it, which RPCS3 never
/// shows (the RPCS3 atlas and the clean square and triangle captures both show it
/// left-aligned, well short of `BACK`'s own `x="720"`). The gate is
/// `confirm.scale > 0.99`: HD's own `ControlTextConfirm` authors
/// `scale="0.8"`, Pulse's authors none (`1.0`).
#[test]
fn hds_own_confirm_stays_left_aligned_at_its_authored_x() {
    let root = parse(HD_XML);
    let strings = StringTable::from_xml(
        r#"<Entries><Entry ID="FE_CONFIRM_BUTTON" String="e"></Entry><Entry ID="FE_CONFIRM" String="CONFIRM"></Entry><Entry ID="FE_BACK_BUTTON" String="g"></Entry><Entry ID="FE_BACK" String="BACK"></Entry></Entries>"#,
    );
    let legend =
        NavigationLegend::read(&root, &HashMap::new(), &strings).expect("controller present");
    let draws = legend.draw(&FaceScales::default(), &|_| 100.0);
    let Draw::FacedText {
        x: confirm_x,
        align: confirm_align,
        ..
    } = &draws[1]
    else {
        unreachable!("index 1 is FE_CONFIRM - icon then word, see the fixture's document order");
    };
    assert_eq!(
        *confirm_align,
        Align::Left,
        "HD's own CONFIRM is left-aligned, not shrunk to fit"
    );
    assert_eq!(
        *confirm_x, 534.0,
        "left at its own authored x, not right-aligned to the back glyph"
    );
}

/// [`NavigationLegend::draw_gated`]'s own reason to exist: `Back` drops out
/// when asked to, `Confirm` never does.
#[test]
fn draw_gated_drops_back_but_never_confirm() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("controller");
    let with_back = legend.draw_gated(&FaceScales::default(), &|_| 100.0, true);
    assert_eq!(with_back.len(), 4, "unchanged from draw() when show_back");
    let without_back = legend.draw_gated(&FaceScales::default(), &|_| 100.0, false);
    let texts: Vec<&str> = without_back.iter().map(|draw| text_and_y(draw).0).collect();
    assert_eq!(
        texts,
        ["ε", "Confirm"],
        "both Back widgets drop, both Confirm widgets stay"
    );
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

/// A refusal line sits one row above the legend, starts where its leftmost
/// prompt does, and is drawn in the word face (`Confirm`'s), never the button
/// glyphs'. With no word prompt to take a face from there is no line.
#[test]
fn a_notice_is_one_row_above_the_legend_in_its_word_face() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("a legend");
    let notice = legend
        .notice("NOT YET", &FaceScales::default())
        .expect("a notice");
    let Draw::FacedText {
        role, x, y, text, ..
    } = notice
    else {
        panic!("the word face draws as FacedText, got {notice:?}");
    };
    assert_eq!(role, "Default");
    assert_eq!(text, "NOT YET");
    assert!((y - (252.0 - NOTICE_LIFT)).abs() < f32::EPSILON);
    assert!((x - 348.0).abs() < f32::EPSILON, "the leftmost prompt's x");

    assert!(
        NavigationLegend::default()
            .notice("NOT YET", &FaceScales::default())
            .is_none()
    );
}
