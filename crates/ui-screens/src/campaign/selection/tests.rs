//! `CampaignSelection`'s own reading/stepping/confirm/pointer behaviour, on the same
//! synthetic-fixture terms `crates/ui-screens/src/campaign/tests.rs` already gives
//! `GridSelection`/`CellSelection`. The screens themselves, read off both discs, are
//! pinned in `crates/game/tests/omega_campaign_screens_ground_truth.rs` and
//! `crates/hd/tests/campaign_selection_ground_truth.rs`.

use super::*;
use oag_gameplay::input::{Button, Input};
use oag_ui::pointer::Pointer;

/// HD's shape: two entries, Fury first, each target screen paging eight grids.
const HD_XML: &str = r#"
<Screen type="CampaignSelection" name="Campaign Selection">
<Text name="ScreenTitle"><Values idstring="FE_RC_SELECT" x="180" y="80" color="0xffaaaaaa"></Values></Text>
<List name="campaignList"><Values x="159" y="-500"></Values>
<Entry String="FE_RC_FURY"></Entry><Entry String="FE_RC_HD"></Entry></List>
<Redirect>
<Entry item="campaignList" equals="FE_RC_HD" goto="Grid Selection"></Entry>
<Entry item="campaignList" equals="FE_RC_FURY" goto="Grid Selection Fury"></Entry>
<Default goto="To Be Done"></Default></Redirect>
<MiniText><Values x="160" y="140" idstring="FE_CAMPSEL_MODES" color="0xff646464"></Values></MiniText>
<Bracket><Values x="160" y="170" Width="1600" Height="780"></Values></Bracket>
<Image name="MedalImageHD"><Values x="1095" y="750" src="medal.mip"></Values>
<MiniText name="HDGoldMedalsMiniText"><Values x="1395" y="710" idstring="RC_GM" color="0xff646464"></Values></MiniText>
<Text name="NumMedalsTextHD"><Values string="b" x="1460" y="770"></Values></Text></Image>
<Image name="MedalImageFury"><Values x="415" y="750" src="medal.mip"></Values>
<MiniText name="FuryGoldMedalsMiniText"><Values x="755" y="710" idstring="RC_GM" color="0xff000000"></Values></MiniText>
<Text name="NumMedalsTextFury"><Values string="a" x="820" y="770"></Values></Text></Image>
</Screen>
<Screen name="Wrapper">
<Screen type="FlyerSelection" name="Grid Selection"><List name="flyerlist">
<Entry String="grid0"></Entry><Entry String="grid1"></Entry></List></Screen>
<Screen type="FlyerSelection" name="Grid Selection Fury"><List name="flyerlist">
<Entry String="grid2"></Entry><Entry String="grid3"></Entry><Entry String="grid4"></Entry></List></Screen>
</Screen>
"#;

/// Omega's shape: three entries, 2048 first, with a rank block where the others have a medal count.
const OMEGA_XML: &str = r#"
<Screen type="CampaignSelection" name="Campaign Selection">
<Text name="ScreenTitle"><Values idstring="FE_RC_SELECT" x="180" y="80" color="0xffaaaaaa"></Values></Text>
<List name="campaignList"><Values x="159" y="-500"></Values>
<Entry String="FE_RC"></Entry><Entry String="FE_RC_HD"></Entry><Entry String="FE_RC_FURY"></Entry></List>
<Redirect>
<Entry item="campaignList" equals="FE_RC_HD" goto="Grid Selection"></Entry>
<Entry item="campaignList" equals="FE_RC_FURY" goto="Grid Selection Fury"></Entry>
<Entry item="campaignList" equals="FE_RC" goto="Grid Selection 2048"></Entry>
<Default goto="To Be Done"></Default></Redirect>
<MiniText><Values x="160" y="140" idstring="FE_CAMPSEL_MODES" color="0xff646464"></Values></MiniText>
<Bracket><Values x="160" y="170" Width="1590" Height="780"></Values></Bracket>
<Image name="MedalImageHD"><Values x="1095" y="750" src="medal.mip"></Values>
<MiniText name="HDGoldMedalsMiniText"><Values x="1075" y="710" idstring="RC_GM" color="0xff646464"></Values></MiniText>
<Text name="NumMedalsTextHD"><Values string="b" x="1170" y="760"></Values></Text></Image>
<Image name="MedalImageFury"><Values x="1415" y="750" src="medal.mip"></Values>
<MiniText name="FuryGoldMedalsMiniText"><Values x="1395" y="710" idstring="RC_GM" color="0xff000000"></Values></MiniText>
<Text name="NumMedalsTextFury"><Values string="a" x="1490" y="760"></Values></Text></Image>
<Image name="RankItem"><MiniText name="2048GoldMedalsMiniText"><Values x="885" y="710" idstring="CO_RANK" color="0xff000000"></Values></MiniText>
<Text name="RankText"><Values string="99" x="953" y="769"></Values></Text></Image>
</Screen>
<Screen name="Wrapper">
<Screen type="FlyerSelection" name="Grid Selection"><List name="flyerlist">
<Entry String="grid0"></Entry><Entry String="grid1"></Entry></List></Screen>
<Screen type="FlyerSelection" name="Grid Selection Fury"><List name="flyerlist">
<Entry String="grid2"></Entry><Entry String="grid3"></Entry></List></Screen>
<Screen type="FlyerSelection" name="Grid Selection 2048"><List name="flyerlist">
<Entry String="grid4"></Entry><Entry String="grid5"></Entry><Entry String="grid6"></Entry></List></Screen>
</Screen>
"#;

fn layout_of(xml: &str, name: &str) -> Option<Layout> {
    Layout::read_authored(
        &Screens::from_xml(xml),
        name,
        &StringTable::default(),
        crate::picker::FaceScales::default(),
        [1920.0, 1080.0],
        [1920.0, 1080.0],
    )
}

fn chooser(xml: &str) -> Chooser {
    Chooser::read(xml, &Screens::from_xml(xml), &|name| layout_of(xml, name))
        .expect("the fixture authors the screen")
}

fn hd() -> Chooser {
    chooser(HD_XML)
}

fn omega() -> Chooser {
    chooser(OMEGA_XML)
}

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

fn step(model: &mut CampaignSelection, button: Button) {
    let mut input = Input::default();
    press(&mut input, button);
    model.update(&mut input);
}

#[test]
fn the_entries_their_targets_and_their_grids_come_off_the_screen() {
    let ids = |chooser: &Chooser| {
        chooser
            .entries()
            .iter()
            .map(|entry| (entry.id.clone(), entry.screen.clone(), entry.grids.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(&hd()),
        [
            ("FE_RC_FURY".into(), "Grid Selection Fury".into(), 2..5),
            ("FE_RC_HD".into(), "Grid Selection".into(), 0..2),
        ]
    );
    assert_eq!(
        ids(&omega()),
        [
            ("FE_RC".into(), "Grid Selection 2048".into(), 4..7),
            ("FE_RC_HD".into(), "Grid Selection".into(), 0..2),
            ("FE_RC_FURY".into(), "Grid Selection Fury".into(), 2..4),
        ]
    );
    assert_eq!(omega().campaign_of(5), Some(Campaign::new(0)));
    assert_eq!(omega().campaign_of(3), Some(Campaign::new(2)));
    assert_eq!(omega().campaign_of(9), None);
}

/// A screen whose redirect or target grids are not all there reads as no chooser at all, so a
/// source with a partial copy opens straight on a grid rather than on a chooser that dead-ends.
#[test]
fn an_entry_with_no_target_screen_leaves_no_chooser() {
    let broken = OMEGA_XML.replace(r#"goto="Grid Selection 2048""#, r#"goto="Nowhere""#);
    assert!(
        Chooser::read(&broken, &Screens::from_xml(&broken), &|name| layout_of(
            &broken, name
        ))
        .is_none()
    );
    let no_list = OMEGA_XML.replace(
        r#"<List name="flyerlist">
<Entry String="grid4"></Entry>"#,
        r#"<List name="other">
<Entry String="grid4"></Entry>"#,
    );
    assert!(
        Chooser::read(&no_list, &Screens::from_xml(&no_list), &|name| layout_of(
            &no_list, name
        ))
        .is_none()
    );
}

#[test]
fn an_entry_names_its_card_by_the_tag_its_widgets_use() {
    let tags: Vec<_> = omega()
        .entries()
        .iter()
        .map(|entry| (entry.card_widget(), entry.card_flyer()))
        .collect();
    assert_eq!(
        tags,
        [
            (
                Some("2048CampaignFlyerModel".to_string()),
                Some("2048_campaign".to_string())
            ),
            (
                Some("HDCampaignFlyerModel".to_string()),
                Some("hd_campaign".to_string())
            ),
            (
                Some("FuryCampaignFlyerModel".to_string()),
                Some("fury_campaign".to_string())
            ),
        ]
    );
}

#[test]
fn the_default_selection_is_the_lists_first_entry() {
    assert_eq!(CampaignSelection::new(hd()).selected(), Campaign::new(0));
    assert_eq!(CampaignSelection::new(omega()).selected(), Campaign::new(0));
}

#[test]
fn right_steps_through_every_entry_and_clamps_at_both_ends() {
    let mut model = CampaignSelection::new(omega());
    step(&mut model, Button::Left);
    assert_eq!(
        model.selected(),
        Campaign::new(0),
        "left at the first entry is clamped"
    );
    step(&mut model, Button::Right);
    assert_eq!(model.selected(), Campaign::new(1));
    step(&mut model, Button::Right);
    assert_eq!(model.selected(), Campaign::new(2));
    step(&mut model, Button::Right);
    assert_eq!(
        model.selected(),
        Campaign::new(2),
        "right at the last entry is clamped"
    );
    step(&mut model, Button::Left);
    assert_eq!(model.selected(), Campaign::new(1));

    let mut model = CampaignSelection::new(hd());
    step(&mut model, Button::Right);
    step(&mut model, Button::Right);
    assert_eq!(model.selected(), Campaign::new(1), "HD's list has two");
}

#[test]
fn cross_confirms_and_circle_backs_out() {
    let mut model = CampaignSelection::new(omega());
    let mut input = Input::default();
    press(&mut input, Button::Cross);
    assert_eq!(model.update(&mut input), vec![Event::Confirmed]);

    let mut input = Input::default();
    press(&mut input, Button::Circle);
    assert_eq!(model.update(&mut input), vec![Event::Back]);
}

#[test]
fn a_click_on_a_column_selects_it_then_confirms_on_a_second_click() {
    let mut model = CampaignSelection::new(omega());
    let third = (1500.0, 500.0);
    assert_eq!(model.pointer(&hover(third), &[]), vec![Event::Moved]);
    assert_eq!(model.selected(), Campaign::new(2));
    assert_eq!(model.pointer(&click(third), &[]), vec![Event::Confirmed]);
}

#[test]
fn a_click_on_the_already_selected_first_column_confirms_it_directly() {
    let mut model = CampaignSelection::new(omega());
    assert_eq!(
        model.pointer(&click((400.0, 500.0)), &[]),
        vec![Event::Confirmed]
    );
}

#[test]
fn pointer_back_fires_regardless_of_where_the_pointer_is() {
    let mut model = CampaignSelection::new(omega());
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(model.pointer(&back, &[]), vec![Event::Back]);
}

/// Each entry's outline is one equal column of the bracket, whatever the count.
#[test]
fn the_selector_outline_stays_inside_its_own_column_of_the_bracket() {
    for chooser in [hd(), omega()] {
        let count = chooser.entries().len();
        let [x, y, width, height] = chooser.bracket();
        for index in 0..count {
            #[allow(clippy::cast_precision_loss, reason = "two or three")]
            let (low, high) = (
                x + width * index as f32 / count as f32,
                x + width * (index + 1) as f32 / count as f32,
            );
            for draw in &selector_outline(&chooser, Campaign::new(index)) {
                let Draw::Fill { rect, .. } = draw else {
                    panic!("selector_outline must draw plain fills, not {draw:?}");
                };
                assert!(
                    rect[0] >= low - 0.01 && rect[0] + rect[2] <= high + 0.01,
                    "column {index} of {count}: {rect:?}"
                );
                assert!(rect[1] >= y && rect[1] + rect[3] <= y + height);
            }
        }
    }
}

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="FE_RC_SELECT" String="CAMPAIGN SELECT"></Entry>
<Entry ID="FE_CAMPSEL_MODES" String="CAMPAIGN MODES"></Entry>
<Entry ID="FE_RC_FURY" String="FURY CAMPAIGN"></Entry>
<Entry ID="FE_RC_HD" String="HD CAMPAIGN"></Entry>
<Entry ID="FE_RC" String="CAMPAIGN"></Entry>
<Entry ID="RC_GM" String="GOLD MEDALS"></Entry>
<Entry ID="CO_RANK" String="RANK"></Entry>
</Strings>"#,
    )
}

fn list(xml: &str, selected: usize, cards: bool) -> Vec<Draw> {
    let model = CampaignSelection::at(chooser(xml), Campaign::new(selected));
    let skin = Skin::new(
        oag_hd::frontend::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let gold = vec![(0, 80), (0, 87), (0, 53)];
    draw_list(
        &model,
        &layout_of(xml, SCREEN).expect("the screen is in the fixture"),
        &skin,
        &Frame::default(),
        &strings(),
        &gold,
        7,
        None,
        false,
        &|_| None,
        &[],
        cards,
    )
    .body
}

/// The four fills of the selected column's outline (the headings' bullets are fills too).
fn outline_fills(draws: &[Draw]) -> usize {
    draws
        .iter()
        .filter(|draw| {
            matches!(draw, Draw::Fill { rect, .. }
                if rect[2] == SELECTOR_BORDER || rect[3] == SELECTOR_BORDER)
        })
        .count()
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
/// selected column, every campaign's name and every counter.
#[test]
fn without_cards_the_stand_ins_and_every_counter_draw() {
    let draws = list(HD_XML, 0, false);
    let words = texts(&draws);
    for want in ["FURY CAMPAIGN", "HD CAMPAIGN", "0 / 80", "0 / 87"] {
        assert!(words.contains(&want), "{want} in {words:?}");
    }
    assert_eq!(outline_fills(&draws), 4, "the four sides of the outline");
}

/// With the cards drawn the outline and the names are gone - the turned card
/// and the card's own wordmark say the same - and only the selected
/// campaign's counter shows, on every selection.
#[test]
fn with_cards_only_the_selected_campaigns_counter_draws() {
    for (selected, shown, hidden) in [(0, "0 / 80", "0 / 87"), (1, "0 / 87", "0 / 80")] {
        let draws = list(HD_XML, selected, true);
        let words = texts(&draws);
        assert!(words.contains(&shown), "{shown} in {words:?}");
        assert!(!words.contains(&hidden), "{hidden} in {words:?}");
        assert!(!words.contains(&"FURY CAMPAIGN") && !words.contains(&"HD CAMPAIGN"));
        assert_eq!(
            words.iter().filter(|w| **w == "GOLD MEDALS").count(),
            1,
            "one label: {words:?}"
        );
        assert_eq!(outline_fills(&draws), 0, "no outline");
    }
}

/// Where the second campaign is the player's rank rather than a medal count, the selected 2048
/// card shows `RANK` and the number and neither medal counter, and the others show their counter
/// and no rank.
#[test]
fn the_2048_entry_draws_a_rank_where_the_others_draw_medals() {
    let draws = list(OMEGA_XML, 0, true);
    let words = texts(&draws);
    assert!(words.contains(&"RANK") && words.contains(&"7"), "{words:?}");
    assert!(!words.contains(&"GOLD MEDALS"), "{words:?}");
    assert!(!words.iter().any(|w| w.contains(" / ")), "{words:?}");

    let draws = list(OMEGA_XML, 1, true);
    let words = texts(&draws);
    assert!(words.contains(&"0 / 87"), "{words:?}");
    assert!(words.contains(&"GOLD MEDALS"), "{words:?}");
    assert!(
        !words.contains(&"RANK") && !words.contains(&"7"),
        "{words:?}"
    );
}

/// With the cards drawn, the hit follows their rectangles, not equal thirds of the bracket: a wide
/// selected card keeps a pointer over its right part, and the selected card wins an overlap.
#[test]
fn the_pointer_follows_the_drawn_cards_and_the_selected_one_wins_an_overlap() {
    let cards = [
        [190.0, 195.0, 805.0, 710.0],
        [1017.0, 215.0, 245.0, 700.0],
        [900.0, 215.0, 600.0, 700.0],
    ];
    let mut model = CampaignSelection::new(omega());
    assert_eq!(
        model.pointer(&hover((800.0, 500.0)), &cards),
        vec![],
        "inside the wide 2048 card"
    );
    assert_eq!(
        model.selected(),
        Campaign::new(0),
        "the equal-thirds edge at 692 does not select HD"
    );
    assert_eq!(
        model.pointer(&hover((950.0, 500.0)), &cards),
        vec![],
        "overlap of 2048 and Fury: 2048 wins"
    );
    assert_eq!(
        model.pointer(&hover((1100.0, 500.0)), &cards),
        vec![Event::Moved]
    );
    assert_eq!(
        model.selected(),
        Campaign::new(1),
        "HD, tested before Fury in list order"
    );
    assert_eq!(
        model.pointer(&click((1100.0, 500.0)), &cards),
        vec![Event::Confirmed]
    );
}
