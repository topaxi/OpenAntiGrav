//! The two-person screens: the draw lists, and the borrowed layouts built from the
//! PSP's own single screens.

use super::*;
use crate::endrace::borrow::{self, PS2_GRID};
use crate::endrace::{Headline, LapSplit, Loyalty, Rewards, split_rewards_draw_list};
use oag_ui::frontend::Draw;
use oag_ui::screen::Screens;

/// The PSP's own two single screens in miniature: what [`borrow`] reads from.
const PSP: &str = r#"
<Screen type="EndRace Results" name="EndRace Results">
<Text name="ContinueButton"><Values idstring="FE_CONFIRM_BUTTON" x="348" y="254"></Values></Text>
</Screen>
<Screen type="EndRace Rewards" name="EndRace Rewards">
<Image><Values width="480" height="272" Color="0x6f000000"></Values></Image>
<Image name="BGgradient"><Values x="0" y="55" width="480" height="80" Color1="0x9f000000" Color2="0x9f000000" Color3="0x9f000000" Color4="0x9f000000"></Values></Image>
<Image><Values x="0" y="55" width="240" height="1" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff"></Values></Image>
<Image name="LoyaltyImg"><Values x="80" y="100" width="32" height="32" src="Data\FE\Images\reward_icons.mip"></Values></Image>
<Text name="BigPos"><Values string="1" x="96" y="62"></Values></Text>
<Text><Values idstring="ER_REWARD" align="centre" font="menu" x="240" y="19"></Values></Text>
<Text name="RewardLine1"><Values idstring="ER_MEDAL_AWARD" x="130" y="67"></Values></Text>
<Text name="RewardLine2"><Values idstring="ER_LOY" x="130" y="107"></Values></Text>
<Text name="RewardLoyaltyActive"><Values string="line 2" x="270" y="107"></Values></Text>
<Text name="loyaltynum"><Values string="0" x="340" y="98"></Values></Text>
<Image name="loyaltybar"><Values x="340" y="116" width="124" height="10" Color="0xffffffff" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
<Text name="ContinueButton"><Values idstring="FE_CONFIRM_BUTTON" x="348" y="252"></Values></Text>
<Redirect name="EndRaceRewardsMenuRedirect"><Values backward="none"></Values><Default goto="Race End Save"></Default></Redirect>
</Screen>
"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="ER_REWARD" String="REWARDS"></Entry>
<Entry ID="ER_1STP" String="1ST PLACE!"></Entry>
<Entry ID="ER_2NDP" String="2ND PLACE!"></Entry>
<Entry ID="RC_LAP" String="Lap"></Entry>
<Entry ID="PRO_TIME" String="Time"></Entry>
<Entry ID="PRO_STATS_TOT" String="Total"></Entry>
<Entry ID="ER_NMA" String="No medal awarded"></Entry>
<Entry ID="ER_LOY" String="Loyalty:"></Entry>
<Entry ID="ER_POINTS" String="Points"></Entry>
<Entry ID="ER_TOT_LOY" String="Total loyalty:"></Entry>
</Strings>"#,
    )
}

fn single(name: &str) -> Layout {
    Layout::read(
        &Screens::from_xml(PSP),
        name,
        &strings(),
        crate::picker::FaceScales::default(),
        [480.0, 272.0],
    )
    .unwrap()
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

fn person(place: u8, ticks: &[u32]) -> Results {
    Results {
        headline: Headline::Position(place),
        laps: ticks
            .iter()
            .enumerate()
            .map(|(index, ticks)| LapSplit {
                lap: index as u32 + 1,
                ticks: *ticks,
                boosts: Some(2),
            })
            .collect(),
        total_ticks: ticks.iter().map(|t| u64::from(*t)).sum(),
    }
}

fn texts(layers: &Layers) -> Vec<(String, f32)> {
    layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, y, .. } => Some((text.clone(), *y)),
            _ => None,
        })
        .collect()
}

fn borrowed_results() -> Layout {
    borrow::results_split(&single("EndRace Results"), [480.0, 272.0]).unwrap()
}

/// Each person's table carries their own laps and headline, the second below the first,
/// and nothing of the one-person screen's own headline.
#[test]
fn each_person_gets_their_own_table_and_headline() {
    let people = [person(2, &[3600, 3000]), person(1, &[3500, 3100, 2900])];
    let layers = split_results_draw_list(
        &people,
        &borrowed_results(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    let texts = texts(&layers);
    let at = |wanted: &str| -> Vec<f32> {
        texts
            .iter()
            .filter(|(text, _)| text == wanted)
            .map(|(_, y)| *y)
            .collect()
    };
    assert_eq!(at("2ND PLACE!").len(), 1, "{texts:?}");
    assert_eq!(at("1ST PLACE!").len(), 1);
    assert!(
        at("2ND PLACE!")[0] < at("1ST PLACE!")[0],
        "person 0 is on top"
    );
    // The two tables' header rows: a `Lap` each, the second lower.
    let laps = at("Lap");
    assert_eq!(laps.len(), 2, "{texts:?}");
    assert!(laps[0] < laps[1]);
    // Person 0 ran two laps and person 1 three: their lap numbers are in their own table.
    let threes: Vec<_> = at("3");
    assert_eq!(threes.len(), 1, "only person 1 has a third lap: {texts:?}");
    assert!(
        threes[0] > laps[1],
        "that third lap is in the second table: {texts:?}"
    );
    assert!(
        !texts.iter().any(|(text, _)| text == "race complete!"),
        "the template headline never leaks: {texts:?}"
    );
}

/// A table with no lap is hidden whole, as the one-person screen hides its own.
#[test]
fn a_person_with_no_lap_has_no_table() {
    let people = [person(1, &[3000]), person(2, &[])];
    let layers = split_results_draw_list(
        &people,
        &borrowed_results(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    assert_eq!(
        texts(&layers)
            .iter()
            .filter(|(text, _)| text == "Lap")
            .count(),
        1
    );
}

/// The borrowed table is the PS2's scaled to the PSP's grid: the second person's table
/// starts `110 + 96` PS2 rows after the first's, times 272/448.
#[test]
fn the_borrowed_tables_sit_where_the_ps2_puts_them_scaled() {
    let layout = borrowed_results();
    let top = |name: &str| {
        layout
            .screen
            .fills
            .iter()
            .find(|fill| fill.name.as_deref() == Some(name))
            .map(|fill| fill.y + fill.own_offset.1)
            .unwrap()
    };
    let scale = 272.0 / 448.0;
    assert!((top("tablebg1_0") - (152.0 - 96.0) * scale).abs() < 0.01);
    assert!((top("tablebg1_1") - (152.0 + 110.0) * scale).abs() < 0.01);
    assert!((top("tablebg2_1") - top("tablebg1_1") - (25.0 * scale + 2.0 * scale)).abs() < 0.01);
}

/// The chain the PS2 authors, Results Split -> Rewards Split -> Race End Save, is on the
/// borrowed screens too.
#[test]
fn the_borrowed_screens_carry_the_authored_redirects() {
    let goto = |layout: &Layout| -> Vec<String> {
        layout
            .screen
            .redirects
            .iter()
            .filter_map(|redirect| redirect.goto.clone())
            .collect()
    };
    assert_eq!(
        goto(&borrowed_results()),
        ["EndRace Rewards Split", "Race End Save"]
    );
    let rewards = borrow::rewards_split(&single("EndRace Rewards"), [480.0, 272.0]).unwrap();
    assert_eq!(goto(&rewards), ["Race End Save"]);
}

/// The Rewards panel is stamped once a person, at the PS2's offsets scaled, and each draws
/// its own team and loyalty.
#[test]
fn each_person_gets_their_own_rewards_panel() {
    let layout = borrow::rewards_split(&single("EndRace Rewards"), [480.0, 272.0]).unwrap();
    let names: Vec<_> = layout
        .screen
        .texts
        .iter()
        .filter_map(|text| text.name.as_deref())
        .collect();
    for wanted in [
        "RewardLine2_0",
        "RewardLine2_1",
        "loyaltynum_0",
        "loyaltynum_1",
    ] {
        assert!(names.contains(&wanted), "{wanted} in {names:?}");
    }
    assert!(!names.contains(&"RewardLine2"), "the single panel is gone");
    let panel = |team: &str, award, total| Rewards {
        medal: None,
        campaign: false,
        loyalty: Some(Loyalty {
            team_name: team.to_string(),
            award,
            total,
        }),
    };
    let layers = split_rewards_draw_list(
        &[panel("Assegai", 90, 90), panel("Feisar", 60, 1200)],
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    let texts = texts(&layers);
    let at = |wanted: &str| -> Vec<f32> {
        texts
            .iter()
            .filter(|(text, _)| text == wanted)
            .map(|(_, y)| *y)
            .collect()
    };
    assert_eq!(at("Assegai Loyalty:").len(), 1, "{texts:?}");
    assert_eq!(at("Feisar Loyalty:").len(), 1);
    assert!(at("Assegai Loyalty:")[0] < at("Feisar Loyalty:")[0]);
    assert_eq!(at("Total loyalty: 90").len(), 1);
    assert_eq!(at("Total loyalty: 1200").len(), 1);
    assert_eq!(at("REWARDS").len(), 2, "a header a panel: {texts:?}");
    // One person alone draws one panel.
    let one = split_rewards_draw_list(
        &[panel("Assegai", 90, 90)],
        &layout,
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    assert_eq!(
        texts_of(&one)
            .iter()
            .filter(|text| *text == "REWARDS")
            .count(),
        1
    );
}

fn texts_of(layers: &Layers) -> Vec<String> {
    texts(layers).into_iter().map(|(text, _)| text).collect()
}

/// The borrowed screens are expressed in the grid they are asked for: the PS2's own grid
/// keeps the PS2's numbers.
#[test]
fn the_grid_asked_for_is_the_grid_drawn_in() {
    let ps2 = borrow::results_split(&single("EndRace Results"), PS2_GRID).unwrap();
    let top = ps2
        .screen
        .fills
        .iter()
        .find(|fill| fill.name.as_deref() == Some("tablebg1_1"))
        .map(|fill| fill.y + fill.own_offset.1)
        .unwrap();
    assert!((top - (152.0 + 110.0)).abs() < 0.01);
}
