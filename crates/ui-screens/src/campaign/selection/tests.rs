//! `CampaignSelection`'s own stepping/confirm/pointer behaviour, on the same
//! synthetic-fixture terms `crates/ui-screens/src/campaign/tests.rs` already gives
//! `GridSelection`/`CellSelection`. See the module doc for the RPCS3
//! measurements this model is built from.

use super::*;
use oag_gameplay::input::{Button, Input};
use oag_ui::pointer::Pointer;

fn press(input: &mut Input, button: Button) {
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
}

fn hover(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        ..Pointer::default()
    }
}

fn click(at: (f32, f32)) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

#[test]
fn the_default_selection_is_fury_matching_the_measured_default() {
    let model = CampaignSelection::new();
    assert_eq!(model.selected(), Campaign::Fury);
}

#[test]
fn right_moves_from_fury_to_the_base_hd_campaign() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Right);
    let events = model.update(&mut input);
    assert_eq!(events, vec![Event::Moved]);
    assert_eq!(model.selected(), Campaign::Hd);
}

#[test]
fn left_at_fury_is_a_no_op_clamped_not_wrapping() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Left);
    model.update(&mut input);
    assert_eq!(
        model.selected(),
        Campaign::Fury,
        "left at the list's own first entry must not wrap to Hd - measured on RPCS3"
    );
}

#[test]
fn right_at_hd_is_also_clamped() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Right);
    model.update(&mut input);
    assert_eq!(model.selected(), Campaign::Hd);
    let mut input = Input::default();
    press(&mut input, Button::Right);
    model.update(&mut input);
    assert_eq!(model.selected(), Campaign::Hd, "no third entry to move to");
}

#[test]
fn cross_confirms_and_circle_backs_out() {
    let mut model = CampaignSelection::new();
    let mut input = Input::default();
    press(&mut input, Button::Cross);
    assert_eq!(model.update(&mut input), vec![Event::Confirmed]);

    let mut input = Input::default();
    press(&mut input, Button::Circle);
    assert_eq!(model.update(&mut input), vec![Event::Back]);
}

#[test]
fn each_campaign_grid_range_and_screen_name_matches_the_disc() {
    assert_eq!(
        Campaign::Fury.grid_range(),
        oag_hd::campaign::FURY_GRID_RANGE
    );
    assert_eq!(Campaign::Fury.grid_screen_name(), "Grid Selection Fury");
    assert_eq!(Campaign::Hd.grid_range(), oag_hd::campaign::HD_GRID_RANGE);
    assert_eq!(Campaign::Hd.grid_screen_name(), "Grid Selection");
}

#[test]
fn a_click_on_the_right_half_selects_hd_then_confirms_on_a_second_click() {
    let mut model = CampaignSelection::new();
    let right_half = (1500.0, 500.0);
    assert_eq!(model.pointer(&hover(right_half)), vec![Event::Moved]);
    assert_eq!(model.selected(), Campaign::Hd);
    assert_eq!(model.pointer(&click(right_half)), vec![Event::Confirmed]);
}

#[test]
fn a_click_on_the_already_selected_left_half_confirms_fury_directly() {
    let mut model = CampaignSelection::new();
    let left_half = (400.0, 500.0);
    assert_eq!(model.pointer(&click(left_half)), vec![Event::Confirmed]);
}

#[test]
fn pointer_back_fires_regardless_of_where_the_pointer_is() {
    let mut model = CampaignSelection::new();
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(model.pointer(&back), vec![Event::Back]);
}

#[test]
fn each_campaign_carries_its_own_disc_entry_id() {
    assert_eq!(Campaign::Fury.entry_id(), "FE_RC_FURY");
    assert_eq!(Campaign::Hd.entry_id(), "FE_RC_HD");
}

/// Pins the bug the advisor caught in the first version of this screen: a
/// left-aligned entry name at the gold-medal label's own `x` ran past the
/// Bracket's own midpoint and was cut by [`selector_outline`]'s own border.
/// Each name is now drawn with [`Align::Centre`] at its own half's centre, so
/// the anchor itself sitting inside that half is what the render actually
/// needs - the align makes the drawn extent symmetric about it.
#[test]
fn each_entry_names_own_anchor_sits_inside_its_own_half_of_the_bracket() {
    let [x, _y, width, _height] = BRACKET_RECT;
    let half = width / 2.0;
    let (fury_x, _) = FURY_ENTRY_NAME_POSITION;
    let (hd_x, _) = HD_ENTRY_NAME_POSITION;
    assert!(
        fury_x >= x && fury_x <= x + half,
        "Fury's own entry name anchor must sit inside the Bracket's left half"
    );
    assert!(
        hd_x >= x + half && hd_x <= x + width,
        "Hd's own entry name anchor must sit inside the Bracket's right half"
    );
}

#[test]
fn the_selector_outline_sits_in_the_left_half_of_the_bracket_for_fury() {
    let draws = selector_outline(Campaign::Fury);
    let [x, y, width, height] = BRACKET_RECT;
    let half = width / 2.0;
    for draw in &draws {
        let Draw::Fill { rect, .. } = draw else {
            panic!("selector_outline must draw plain fills, not {draw:?}");
        };
        assert!(
            rect[0] >= x && rect[0] + rect[2] <= x + half,
            "Fury's own outline must stay within the Bracket's left half: {rect:?}"
        );
        assert!(rect[1] >= y && rect[1] + rect[3] <= y + height);
    }
}

#[test]
fn the_selector_outline_moves_to_the_right_half_for_hd() {
    let draws = selector_outline(Campaign::Hd);
    let [x, _y, width, _height] = BRACKET_RECT;
    let half = width / 2.0;
    for draw in &draws {
        let Draw::Fill { rect, .. } = draw else {
            panic!("selector_outline must draw plain fills, not {draw:?}");
        };
        assert!(
            rect[0] >= x + half && rect[0] + rect[2] <= x + width,
            "Hd's own outline must stay within the Bracket's right half: {rect:?}"
        );
    }
}

/// `Campaign Selection` in miniature: the title, and the two medal counters.
const XML: &str = r#"
<Screen type="CampaignSelection" name="Campaign Selection">
<Text name="ScreenTitle"><Values idstring="FE_RC_SELECT" x="180" y="80" color="0xffaaaaaa"></Values></Text>
<Text name="NumMedalsTextFury"><Values string="a" x="820" y="770"></Values></Text>
<Text name="NumMedalsTextHD"><Values string="b" x="1460" y="770"></Values></Text>
</Screen>
"#;

fn list(selected: Campaign, cards: bool) -> Vec<Draw> {
    let strings = oag_ui::language::StringTable::from_xml(
        r#"<Strings>
<Entry ID="FE_RC_SELECT" String="CAMPAIGN SELECT"></Entry>
<Entry ID="FE_CAMPSEL_MODES" String="CAMPAIGN MODES"></Entry>
<Entry ID="FE_RC_FURY" String="FURY CAMPAIGN"></Entry>
<Entry ID="FE_RC_HD" String="HD CAMPAIGN"></Entry>
<Entry ID="RC_GM" String="GOLD MEDALS"></Entry>
</Strings>"#,
    );
    let layout = Layout::read_authored(
        &oag_ui::screen::Screens::from_xml(XML),
        "Campaign Selection",
        &strings,
        crate::picker::FaceScales::default(),
        [1920.0, 1080.0],
        [1920.0, 1080.0],
    )
    .expect("the screen is in the fixture");
    let skin = Skin::new(
        oag_hd::frontend::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    draw_list(
        &CampaignSelection::at(selected),
        &layout,
        &skin,
        &Frame::default(),
        &strings,
        (0, 80),
        (0, 87),
        None,
        false,
        &|_| None,
        &[],
        cards,
    )
    .body
}

fn texts(draws: &[Draw]) -> Vec<&str> {
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } | Draw::FacedText { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// With no card drawn the screen keeps its stand-ins: the outline round the
/// selected half, the two campaign names and both medal counters.
#[test]
fn without_cards_the_stand_ins_and_both_counters_draw() {
    let draws = list(Campaign::Fury, false);
    let words = texts(&draws);
    for want in ["FURY CAMPAIGN", "HD CAMPAIGN", "0 / 80", "0 / 87"] {
        assert!(words.contains(&want), "{want} in {words:?}");
    }
    assert_eq!(
        draws
            .iter()
            .filter(|d| matches!(d, Draw::Fill { .. }))
            .count(),
        4,
        "the four sides of the outline"
    );
}

/// With the cards drawn the outline and the names are gone - the turned card
/// and the card's own wordmark say the same - and only the selected
/// campaign's medal counter shows, on both selections.
#[test]
fn with_cards_only_the_selected_campaigns_counter_draws() {
    for (selected, shown, hidden) in [
        (Campaign::Fury, "0 / 80", "0 / 87"),
        (Campaign::Hd, "0 / 87", "0 / 80"),
    ] {
        let draws = list(selected, true);
        let words = texts(&draws);
        assert!(words.contains(&shown), "{shown} in {words:?}");
        assert!(!words.contains(&hidden), "{hidden} in {words:?}");
        assert!(!words.contains(&"FURY CAMPAIGN") && !words.contains(&"HD CAMPAIGN"));
        assert_eq!(
            words.iter().filter(|w| **w == "GOLD MEDALS").count(),
            1,
            "one label: {words:?}"
        );
        assert!(
            !draws.iter().any(|d| matches!(d, Draw::Fill { .. })),
            "no outline"
        );
    }
}
