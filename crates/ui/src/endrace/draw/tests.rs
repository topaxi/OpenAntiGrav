//! Draw-list tests: everything that needs a [`crate::menu::Layers`] out of
//! [`results_draw_list`]/[`rewards_draw_list`]/[`endrace_menu_draw_list`].
//! Split from `endrace/tests.rs`'s model-level tests the same way
//! `campaign/draw.rs`'s own tests are.

use super::*;
use crate::endrace::{Headline, LapSplit, MenuOption, Results, Rewards};
use crate::frontend::Draw;
use crate::language::StringTable;
use crate::menu::Frame;
use crate::screen::Screens;
use oag_tables::race_campaign::Medal;

/// `EndRace_Definition.xml` in miniature - the same container shape the
/// real file authors (`table` as a `Text`-shaped group, `lap{n}.{c}` cells
/// inside it, `Endrace Options` as a `Menu` widget), trimmed to three lap
/// rows and three menu entries rather than eight and seven.
const XML: &str = r#"
<Screen type="EndRace Results" name="EndRace Results">
<Text name="BigTopText"><Values idstring="ER_RES" x="240" y="19"></Values></Text>
<Text name="Line1"><Values string="race complete!" x="240" y="50"></Values></Text>
<Text name="table">
<Image name="tablebg1" OffsetY="92"><Values x="0" y="0" width="480" height="22" Color1="0x9f000000" Color2="0x9f000000" Color3="0x9f000000" Color4="0x9f000000"></Values></Image>
<Text name="lap0.0"><Values x="140" y="76"></Values></Text>
<Text name="lap0.1"><Values x="220" y="76"></Values></Text>
<Text name="lap0.2"><Values x="320" y="76"></Values></Text>
<Text name="lap1.0"><Values x="140" y="95"></Values></Text>
<Text name="lap1.1"><Values x="220" y="95"></Values></Text>
<Text name="lap1.2"><Values x="320" y="95"></Values></Text>
<Text name="lap2.0"><Values x="140" y="115"></Values></Text>
<Text name="lap2.1"><Values x="220" y="115"></Values></Text>
<Text name="lap2.2"><Values x="320" y="115"></Values></Text>
<Text name="lap3.0"><Values x="140" y="135"></Values></Text>
<Text name="lap3.1"><Values x="220" y="135"></Values></Text>
<Text name="lap3.2"><Values x="320" y="135"></Values></Text>
<Image name="perfectlap1"><Values x="352" y="95" width="16" height="16" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
<Image name="boostimg"><Values x="320" y="77" width="12" height="12" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
<Image name="tablehighlight"><Values x="0" y="20" width="480" height="19" RealGlow="128" Color="0x30009fff"></Values></Image>
</Text>
<Text name="ContinueButton"><Values idstring="FE_CONFIRM_BUTTON" x="348" y="254"></Values></Text>
</Screen>
<Screen type="EndRace Rewards" name="EndRace Rewards">
<Text><Values idstring="ER_REWARD" x="240" y="19"></Values></Text>
<Image name="MedalImg"><Values x="80" y="60" width="32" height="32" src="Data\FE\Images\reward_icons.mip"></Values></Image>
<Image name="LoyaltyImg"><Values x="80" y="100" width="32" height="32" src="Data\FE\Images\reward_icons.mip"></Values></Image>
<Text name="RewardLine1"><Values idstring="ER_MEDAL_AWARD" x="130" y="67"></Values></Text>
<Text name="RewardLine2"><Values idstring="ER_LOY" x="130" y="107"></Values></Text>
<Text name="RewardLoyaltyActive"><Values string="line 2" x="270" y="107"></Values></Text>
<Text name="loyaltynum"><Values string="0" x="340" y="98"></Values></Text>
<Image name="loyaltybg"><Values x="340" y="116" width="124" height="10" Color="0x7fffffff" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
<Image name="loyaltybar"><Values x="340" y="116" width="124" height="10" Color="0xffffffff" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
</Screen>
<Screen type="EndRace Menu" name="EndRace Menu">
<Menu name="Endrace Options"><Values x="50" y="60" font="title" color="0xffffffff"></Values>
<Entry string="test"></Entry>
<Entry string="test"></Entry>
<Entry string="test"></Entry>
</Menu>
<Text name="GhostLine1"><Values x="50" y="199"></Values></Text>
<Text name="GhostTime1"><Values x="270" y="199"></Values></Text>
<Text name="GhostLine2"><Values idstring="ER_NEW_GHOST" x="50" y="212"></Values></Text>
<Text name="GhostTime2"><Values x="270" y="212"></Values></Text>
</Screen>
"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="ER_RES" String="RESULTS"></Entry>
<Entry ID="ER_REWARD" String="REWARDS"></Entry>
<Entry ID="ER_TT_COM" String="TIME TRIAL COMPLETE!"></Entry>
<Entry ID="ER_1STP" String="1ST PLACE!"></Entry>
<Entry ID="RC_LAP" String="Lap"></Entry>
<Entry ID="PRO_TIME" String="Time"></Entry>
<Entry ID="PRO_STATS_TOT" String="Total"></Entry>
<Entry ID="ER_GMA" String="Gold medal awarded"></Entry>
<Entry ID="ER_NMA" String="No medal awarded"></Entry>
<Entry ID="ER_RETURN_GRID" String="RETURN TO GRID"></Entry>
<Entry ID="ER_RACE_AGAIN" String="RACE AGAIN"></Entry>
<Entry ID="ER_VIEW_AGAIN" String="VIEW RESULTS AGAIN"></Entry>
<Entry ID="ER_NEW_GHOST" String="NEW GHOST RECORD:"></Entry>
<Entry ID="FE_CONFIRM_BUTTON" String="X"></Entry>
<Entry ID="ER_LOY" String="Loyalty:"></Entry>
<Entry ID="ER_POINTS" String="Points"></Entry>
<Entry ID="ER_TOT_LOY" String="Total loyalty:"></Entry>
</Strings>"#,
    )
}

fn results_layout() -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        "EndRace Results",
        &strings(),
        crate::picker::FaceScales::default(),
        [480.0, 272.0],
    )
    .unwrap()
}

fn rewards_layout() -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        "EndRace Rewards",
        &strings(),
        crate::picker::FaceScales::default(),
        [480.0, 272.0],
    )
    .unwrap()
}

fn menu_layout() -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        "EndRace Menu",
        &strings(),
        crate::picker::FaceScales::default(),
        [480.0, 272.0],
    )
    .unwrap()
}

fn texts(layers: &crate::menu::Layers) -> Vec<String> {
    layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn sprite_count(layers: &crate::menu::Layers) -> usize {
    layers
        .body
        .iter()
        .filter(|draw| matches!(draw, Draw::Sprite { .. } | Draw::TiledSprite { .. }))
        .count()
}

/// `BigTopText`/`Line1` and the per-lap table draw off the model. The third
/// column's own values and the `perfectlap` icons never appear - see the module
/// doc on why; the `boostimg` header icon does (`modes::tests` checks it).
#[test]
fn results_draws_the_headline_and_the_lap_table_but_never_the_third_column() {
    let model = Results {
        headline: Headline::TimeTrial,
        laps: vec![
            LapSplit {
                lap: 1,
                ticks: 60,
                boosts: None,
            },
            LapSplit {
                lap: 2,
                ticks: 120,
                boosts: None,
            },
        ],
        total_ticks: 180,
    };
    let layers = results_draw_list(
        &model,
        &results_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    let texts = texts(&layers);
    assert!(texts.contains(&"RESULTS".to_string()), "{texts:?}");
    assert!(
        texts.contains(&"TIME TRIAL COMPLETE!".to_string()),
        "{texts:?}"
    );
    assert!(texts.contains(&"Lap".to_string()), "{texts:?}");
    assert!(texts.contains(&"Time".to_string()), "{texts:?}");
    assert!(texts.contains(&"1".to_string()), "{texts:?}");
    assert!(texts.contains(&"0.01.00".to_string()), "{texts:?}");
    assert!(texts.contains(&"Total".to_string()), "{texts:?}");
    assert!(texts.contains(&"0.03.00".to_string()), "{texts:?}");
    // Column 2's own header is blank, and no lap row draws anything into it
    // either - the meaning is unread, see the module doc.
    for draw in &layers.body {
        if let Draw::Text { x, .. } = draw {
            assert!(
                (*x - 320.0).abs() > 0.01,
                "nothing should draw at column 2's own x"
            );
        }
    }
}

/// A position-carrying finish resolves `Line1` to the finishing-place
/// idstring, and `Zone`/`Eliminator`'s [`Headline::Unresolved`] draws no
/// `Line1` at all rather than a guessed one.
#[test]
fn a_position_headline_resolves_and_an_unresolved_one_draws_nothing() {
    let position = Results {
        headline: Headline::Position(1),
        laps: Vec::new(),
        total_ticks: 0,
    };
    let layers = results_draw_list(
        &position,
        &results_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    assert!(texts(&layers).contains(&"1ST PLACE!".to_string()));

    let unresolved = Results {
        headline: Headline::Unresolved,
        laps: Vec::new(),
        total_ticks: 0,
    };
    let layers = results_draw_list(
        &unresolved,
        &results_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    assert!(
        !texts(&layers).contains(&"race complete!".to_string()),
        "the XML's own template string must not leak through unresolved"
    );
}

/// The one measured case: a campaign race with no medal shows the hex-dash
/// glyph and the `No medal awarded` phrase, and draws nothing in the
/// loyalty row when the model carries none - see `docs/ui/endrace-screens.md`.
#[test]
fn rewards_draws_the_no_medal_case_and_never_the_loyalty_row_absent_one() {
    let model = Rewards {
        medal: None,
        campaign: true,
        loyalty: None,
    };
    let layers = rewards_draw_list(
        &model,
        &rewards_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    let texts = texts(&layers);
    assert!(texts.contains(&"REWARDS".to_string()), "{texts:?}");
    assert!(texts.contains(&"No medal awarded".to_string()), "{texts:?}");
    let sprites = sprite_count(&layers);
    assert_eq!(
        sprites, 1,
        "MedalImg's own hex-dash glyph should draw, and nothing else - not \
         LoyaltyImg or the loyalty bar's own two sprites: {:?}",
        layers.body
    );
    assert!(
        !texts.iter().any(|t| t == "line 2" || t == "0"),
        "the loyalty row's own placeholder text must not leak through: {texts:?}"
    );
}

/// The loyalty row draws once the model carries an award -
/// `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal`'s own two numbers,
/// `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`, matching the
/// captured `"Assegai Loyalty: 90 Points"` / `"Total loyalty: 90"`.
#[test]
fn rewards_draws_the_loyalty_row_once_the_model_carries_an_award() {
    let model = Rewards {
        medal: None,
        campaign: true,
        loyalty: Some(crate::endrace::Loyalty {
            team_name: "Assegai".to_string(),
            award: 90,
            total: 90,
        }),
    };
    let layers = rewards_draw_list(
        &model,
        &rewards_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    let texts = texts(&layers);
    assert!(texts.contains(&"Assegai Loyalty:".to_string()), "{texts:?}");
    assert!(texts.contains(&"90 Points".to_string()), "{texts:?}");
    assert!(
        texts.contains(&"Total loyalty: 90".to_string()),
        "{texts:?}"
    );
    // `LoyaltyImg` plus the two loyalty-bar sprites, alongside `MedalImg`'s
    // own hex-dash glyph (no medal, campaign race).
    assert_eq!(sprite_count(&layers), 4, "{:?}", layers.body);
}

/// A non-campaign race hides `MedalImg` outright, and an earned medal
/// leaves it undrawn too - the trophy is the composition root's own layer,
/// not this crate's.
#[test]
fn rewards_hides_the_glyph_off_a_non_campaign_race_or_an_earned_medal() {
    for model in [
        Rewards {
            medal: None,
            campaign: false,
            loyalty: None,
        },
        Rewards {
            medal: Some(Medal::Gold),
            campaign: true,
            loyalty: None,
        },
    ] {
        let layers = rewards_draw_list(
            &model,
            &rewards_layout(),
            &skin(),
            &Frame::default(),
            &strings(),
            None,
            false,
            &|_| placed(),
        );
        assert_eq!(
            sprite_count(&layers),
            0,
            "{model:?} should draw no sprite at all - not the medal glyph, \
             not the loyalty icon or bar: {:?}",
            layers.body
        );
    }
}

/// `Endrace Options` draws one row per option, and the just-driven run's
/// own best lap fills `GhostTime2` - with no existing-ghost row at all.
#[test]
fn menu_draws_its_options_and_the_new_best_lap_but_no_existing_ghost_row() {
    let model = EndRaceMenu::new(
        vec![
            MenuOption::ReturnToGrid,
            MenuOption::RaceAgain,
            MenuOption::ViewResultsAgain,
        ],
        Some(2960),
    );
    let layers = endrace_menu_draw_list(
        &model,
        &menu_layout(),
        &skin(),
        &Frame::default(),
        &strings(),
        None,
        false,
        &|_| placed(),
    );
    let texts = texts(&layers);
    assert!(texts.contains(&"RETURN TO GRID".to_string()), "{texts:?}");
    assert!(texts.contains(&"RACE AGAIN".to_string()), "{texts:?}");
    assert!(
        texts.contains(&"VIEW RESULTS AGAIN".to_string()),
        "{texts:?}"
    );
    assert!(
        texts.contains(&"NEW GHOST RECORD:".to_string()),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t.starts_with("0.")), "{texts:?}");
}

fn skin() -> crate::menu::Skin {
    crate::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn placed() -> Option<crate::frontend::Placed> {
    Some(crate::frontend::Placed {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
        quad_extent: None,
        blend: None,
    })
}
