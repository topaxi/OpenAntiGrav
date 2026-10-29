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
<Text name="NewsItemTagText"><Values RealGlow="64"></Values></Text>
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
    let (stamps, draws): (Vec<Draw>, Vec<Draw>) = legend
        .draw(&FaceScales::default())
        .into_iter()
        .partition(|draw| matches!(draw, Draw::GlowStamp { .. }));
    assert_eq!(draws.len(), 4);
    assert_eq!(
        stamps,
        [Draw::GlowStamp {
            rect: [29.0, 234.0, 28.0, 14.0],
            value: 64
        }],
        "NewsItemTagText's authored RealGlow=\"64\" is the footer tag's stamp"
    );
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

/// Pulse's `Confirm` draws as authored: left-aligned at `x="368"`, at the face's own scale
/// (`1.0`), like every other footer word. It once right-aligned to the back glyph and shrank
/// to fit, a workaround for a menu face wider than the original's; the face now matches the
/// original pixel for pixel, so dropping the authored x or scale is a regression (the PSP
/// `Cell Selection` pair measures the word 40 px wide at x 369, not 34 px at x 375).
#[test]
fn pulses_confirm_keeps_its_authored_x_and_full_scale() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("controller");
    // A word wider than the 47 px between the confirm and back glyphs: the old fit-to-gap path
    // would have shrunk it, and right-aligned it, at any width over about 20 px.
    let draws = legend.draw(&FaceScales::default());
    let Draw::FacedText {
        x: confirm_x,
        scale: confirm_scale,
        align: confirm_align,
        ..
    } = &draws[1]
    else {
        unreachable!("index 1 is FE_CONFIRM - see the fixture's own document order");
    };
    assert_eq!(*confirm_align, Align::Left);
    assert_eq!(*confirm_x, 368.0, "authored x, not moved to fit");
    assert!(
        (*confirm_scale - 1.0).abs() < f32::EPSILON,
        "Confirm draws at the face's own scale like Back, got {confirm_scale}"
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
        text_scale: 1.0,
        text_lines: 1,
        text_font: String::new(),
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
        text_scale: 1.0,
        text_lines: 1,
        text_font: String::new(),
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
        text_scale: 1.0,
        text_lines: 1,
        text_font: String::new(),
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
    let draws = legend.draw(&FaceScales::default());
    assert_eq!(draws.len(), 2);
    let texts: Vec<&str> = draws.iter().map(|draw| text_and_y(draw).0).collect();
    assert_eq!(texts, ["e", "CONFIRM"]);
}

/// Wipeout HD/Fury's own icon glyph (`font="buttons"`) draws now, through
/// its own `Buttons`-role atlas - see [`face_role`]'s own doc for the
/// verification this rests on (a one-off read
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
    let draws = legend.draw(&FaceScales::default());
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

/// HD's own `CONFIRM` stays left-aligned at its authored `x="534"`, well short of `BACK`'s
/// `x="720"`, as the RPCS3 captures show.
#[test]
fn hds_own_confirm_stays_left_aligned_at_its_authored_x() {
    let root = parse(HD_XML);
    let strings = StringTable::from_xml(
        r#"<Entries><Entry ID="FE_CONFIRM_BUTTON" String="e"></Entry><Entry ID="FE_CONFIRM" String="CONFIRM"></Entry><Entry ID="FE_BACK_BUTTON" String="g"></Entry><Entry ID="FE_BACK" String="BACK"></Entry></Entries>"#,
    );
    let legend =
        NavigationLegend::read(&root, &HashMap::new(), &strings).expect("controller present");
    let draws = legend.draw(&FaceScales::default());
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
    let prompts = |show_back| -> Vec<Draw> {
        legend
            .draw_gated(&FaceScales::default(), show_back)
            .into_iter()
            .filter(|draw| !matches!(draw, Draw::GlowStamp { .. }))
            .collect()
    };
    let with_back = prompts(true);
    assert_eq!(with_back.len(), 4, "unchanged from draw() when show_back");
    let without_back = prompts(false);
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
        text_scale: 1.0,
        text_lines: 1,
        text_font: String::new(),
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

fn text_x_and_extent(draw: &Draw, measure: &dyn Fn(&str) -> f32) -> (f32, f32) {
    match draw {
        Draw::Text { x, text, scale, .. } | Draw::FacedText { x, text, scale, .. } => {
            (*x, *x + measure(text) * scale)
        }
        other => panic!("expected a text, got {other:?}"),
    }
}

/// The reset prompt is one more entry on `Confirm`'s own row: the title's `Square` stand-in in
/// the glyph's face, the words in the word's, on `Confirm`'s baselines and in its faces, inside
/// the screen, clear of every authored prompt (`Back` too, which a root page does not draw),
/// with a tap rect over both. Without the glyph the words stand alone.
#[test]
fn the_reset_prompt_shares_confirms_row_and_stays_clear_of_every_prompt() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("controller");
    assert!(legend.can_model_reset_prompt());
    let measure = |text: &str| text.chars().count() as f32 * 6.0;
    let (draws, rect) = legend
        .reset_prompt(
            Some('δ'),
            "RESET TO DEFAULT",
            &FaceScales::default(),
            &measure,
            480.0,
            &[],
        )
        .expect("the fixture's legend has a Confirm pair");
    let texts: Vec<&str> = draws.iter().map(|draw| text_and_y(draw).0).collect();
    assert_eq!(texts, ["δ", "RESET TO DEFAULT"]);
    for draw in &draws {
        assert!(
            (text_and_y(draw).1 - 252.0).abs() < f32::EPSILON,
            "on Confirm's baseline"
        );
    }
    assert!(
        rect[0] >= 0.0 && rect[0] + rect[2] <= 480.0,
        "{rect:?} inside the 480 screen"
    );
    let (first, last) = (
        text_x_and_extent(&draws[0], &measure),
        text_x_and_extent(&draws[1], &measure),
    );
    assert!(first.0 < last.0, "glyph then word");
    assert!(
        (rect[0] - first.0).abs() < 0.01 && (rect[0] + rect[2] - last.1).abs() < 0.01,
        "the tap rect {rect:?} covers the drawn glyph and word"
    );
    // Pulse's footer has no room after `Back`, so it goes before `Confirm`'s glyph (x=348),
    // and so never lands on `Back`, which sits from x=415.
    assert!(last.1 <= 348.0, "ends before Confirm: {last:?}");
    let (words_only, _) = legend
        .reset_prompt(None, "RESET", &FaceScales::default(), &|_| 30.0, 480.0, &[])
        .expect("words alone");
    assert_eq!(words_only.len(), 1);
}

/// On a wide footer the prompt follows the legend's order, after the last authored prompt, with
/// the legend's own spacing, and still inside the page.
#[test]
fn the_reset_prompt_goes_after_back_where_the_footer_has_room() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("controller");
    let measure = |text: &str| text.chars().count() as f32 * 6.0;
    let (draws, rect) = legend
        .reset_prompt(None, "RESET", &FaceScales::default(), &measure, 960.0, &[])
        .expect("a Confirm pair");
    // Back's word starts at 435 and ends 24 on: the prompt starts after it by the 5 the legend
    // leaves between Confirm's word (368 + 42 wide, as authored) and Back's glyph at 415.
    assert!((rect[0] - (435.0 + 24.0 + 5.0)).abs() < 0.01, "{rect:?}");
    assert!(rect[0] + rect[2] <= 960.0);
    assert_eq!(draws.len(), 1);
}

/// No `Confirm` pair, nothing to model the prompt on.
#[test]
fn the_reset_prompt_needs_a_confirm_pair() {
    assert!(
        NavigationLegend::default()
            .reset_prompt(None, "RESET", &FaceScales::default(), &|_| 30.0, 480.0, &[])
            .is_none()
    );
}

/// A mark the frame draws beside the legend closes the gap it covers; with no gap left on the
/// row the prompt takes the line above rather than draw over the mark. A strip behind the
/// legend is its own backdrop and blocks nothing.
#[test]
fn the_reset_prompt_keeps_clear_of_the_frames_own_marks() {
    let root = parse(XML);
    let strings = StringTable::from_xml(ENTRIES);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("controller");
    let measure = |text: &str| text.chars().count() as f32 * 6.0;
    let prompt = |blocked: &[[f32; 4]]| {
        legend
            .reset_prompt(
                None,
                "RESET TO DEFAULT",
                &FaceScales::default(),
                &measure,
                480.0,
                blocked,
            )
            .expect("a Confirm pair")
            .1
    };
    let strip = [80.0, 236.0, 380.0, 32.0];
    let free = prompt(&[strip]);
    assert!(
        (free[1] - 252.0).abs() < f32::EPSILON,
        "a backdrop strip leaves the row: {free:?}"
    );
    let mark = [100.0, 250.0, 150.0, 12.0];
    let lifted = prompt(&[mark]);
    assert!(
        lifted[1] < 252.0,
        "no gap beside the mark holds it: {lifted:?}"
    );
    assert!(lifted[0] >= 0.0 && lifted[0] + lifted[2] <= 480.0);
}

/// The glyph half of a pair is told by the widget's own name, not by its idstring: Pure's
/// button string is not called `*_BUTTON`, and a legend that missed it had no pair to model the
/// reset prompt on.
#[test]
fn a_button_half_is_known_by_its_widget_name_not_its_idstring() {
    let xml = XML.replace("FE_CONFIRM_BUTTON", "FE_CONFIRM_ICON");
    let root = parse(&xml);
    let entries = ENTRIES.replace("FE_CONFIRM_BUTTON", "FE_CONFIRM_ICON");
    let strings = StringTable::from_xml(&entries);
    let legend = NavigationLegend::read(&root, &globals(), &strings).expect("controller");
    assert!(legend.can_model_reset_prompt());
}

/// Pure's skin names `Back button` where its string table keys `Back Button`: the prompt still
/// resolves to the table's glyph, and does not draw the id as text.
#[test]
fn a_prompt_idstring_differing_only_in_case_still_resolves() {
    let root = parse(
        r#"<Root><NavigationController name="NavigationController">
<Text name="ControlTextBackButton"><Values idstring="Back button" font="scroll" x="365" y="251"></Values></Text>
<Text name="ControlTextBack"><Values idstring="Back" font="scroll" scale="0.7" x="381" y="253"></Values></Text>
</NavigationController></Root>"#,
    );
    let strings = StringTable::from_xml(
        r#"<Entries><Entry ID="Back Button" String="g"></Entry><Entry ID="Back" String="Back"></Entry></Entries>"#,
    );
    let legend =
        NavigationLegend::read(&root, &HashMap::new(), &strings).expect("controller present");
    let draws = legend.draw(&FaceScales::default());
    let texts: Vec<&str> = draws.iter().map(|draw| text_and_y(draw).0).collect();
    assert_eq!(texts, ["g", "Back"]);
}

/// Pure's `TextInfo` authors three `Text` lines and names no scale: the description wraps to the
/// viewport's width, stops at three lines, and sits at the viewport's own left edge in its colour.
#[test]
fn a_description_wraps_into_the_text_infos_lines_and_no_more() {
    let root = parse(
        r#"<Screen name="Skin"><Viewport name="TextInfoIsAlwaysLast" OffsetX="21" OffsetY="217">
<Values width="427" height="32"></Values>
<TextInfo delay="1.6"><Values y="1" font="scroll" scale="FEGlobals->TextInfoScale" color="0xFFFAB41A"></Values>
<Text Name="Text0"></Text><Text Name="Text1"></Text><Text Name="Text2"></Text></TextInfo></Viewport></Screen>"#,
    );
    let layout = TickerLayout::read(&root, &HashMap::new()).expect("the viewport");
    assert_eq!(
        layout.text_lines, 3,
        "the three Text children are its lines"
    );
    assert_eq!(layout.text_scale, DESCRIPTION_SCALE);
    let words = vec!["word"; 40].join(" ");
    let draws = description_draws(&layout, &words, None, &|text| text.len() as f32 * 10.0);
    assert_eq!(draws.len(), 3, "cut at the authored line count");
    let ys: Vec<f32> = draws.iter().map(|draw| text_and_y(draw).1).collect();
    assert_eq!(ys[0], 217.0);
    assert!(ys[1] > ys[0] && ys[2] > ys[1], "{ys:?}");
    assert!(description_draws(&layout, "  ", None, &|_| 1.0).is_empty());
}

/// The description wraps whole and is then revealed across the lines: at 0.5 s only the first
/// line's prefix and the cursor draw, the second line waits for the first, and settled it all draws.
#[test]
fn a_description_types_across_its_wrapped_lines_and_loops() {
    let root = parse(
        r#"<Screen name="Skin"><Viewport name="TextInfoIsAlwaysLast" OffsetX="21" OffsetY="217">
<Values width="100" height="32"></Values>
<TextInfo delay="1.6"><Values y="1" font="scroll" color="0xFFFAB41A"></Values>
<Text Name="Text0"></Text><Text Name="Text1"></Text><Text Name="Text2"></Text></TextInfo></Viewport></Screen>"#,
    );
    let layout = TickerLayout::read(&root, &HashMap::new()).expect("the viewport");
    let measure = |text: &str| text.len() as f32 * 10.0;
    let text = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj";
    let texts = |since: Option<f32>| -> Vec<String> {
        description_draws(&layout, text, since, &measure)
            .iter()
            .map(|draw| text_and_y(draw).0.to_string())
            .collect()
    };
    let settled = texts(None);
    assert!(settled.len() > 1, "wrapped over several lines: {settled:?}");
    let early = texts(Some(typein::START + 0.2));
    assert_eq!(
        early.len(),
        1,
        "the second line waits for the first: {early:?}"
    );
    assert!(early[0].ends_with('_'), "a cursor while typing: {early:?}");
    let typed = early[0].trim_end_matches('_');
    assert!(settled[0].starts_with(typed) && typed.len() < settled[0].len());
    let total = settled.iter().map(|line| line.len() + 1).sum::<usize>() - 1;
    let done = typein::START + total as f32 / typein::CHARS_PER_SECOND + 0.1;
    let whole: Vec<String> = texts(Some(done))
        .into_iter()
        .map(|line| line.trim_end_matches('_').to_string())
        .collect();
    assert_eq!(whole, settled, "held whole once typed");
    assert!(
        texts(Some(0.0)).is_empty(),
        "nothing before the start delay"
    );
    let round = typein::START + total as f32 / typein::CHARS_PER_SECOND + typein::HOLD;
    assert_eq!(
        texts(Some(round + 0.2)).len(),
        1,
        "typed again from the first line"
    );
}
