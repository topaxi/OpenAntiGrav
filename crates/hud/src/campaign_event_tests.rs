//! 2048's `XP` and `OBJECTIVE` lines: which races draw them.
//!
//! The widgets are the names, strings and nesting of `Data\XML\2048_hud\HUD_award_xp.xml` and
//! `HUD_objectives.xml`, which `Arcade_HUD.xml` (and `Arcade_HUD_HD.xml`) load; the same
//! walk over the disc's own composed layout is `crates/raceplay/tests/vita_2048_hud_event_ground_truth.rs`.

use super::draw::*;
use super::tests::{context, strings};
use super::*;

const XP_AND_OBJECTIVE: &str = r#"
<Screen>
<Screen name="HUD">
<Text name="AwardXPParent" OffsetX="945" OffsetY="70">
<Text name="RaceXPTxt"><Values idstring="FE_XP" scale="0.6" font="HUD" align="right" x="-90" y="10"/></Text>
<Text name="RaceXP"><Values string="" scale="0.6" font="HUD" align="right" x="0" y="10"/></Text>
<Text name="AwardXP"><Values string="" scale="0.6" font="HUD" align="right" x="0" y="45"/></Text>
</Text>
<Text name="ObjectiveParent" OffsetX="15" OffsetY="130">
<Text name="ObjectiveTxt"><Values idstring="IG_HUD_OBJECTIVE" font="HUD" align="left" scale="0.6" x="0" y="0"/></Text>
</Text>
<Text name="LapTxt"><Values idstring="IG_HUD_LAP" font="HUD" x="4" y="0"/></Text>
</Screen>
</Screen>
"#;

/// Every `HUD`-font string the frame draws, in order.
fn drawn(art: &'static oag_title::HudArt, campaign_event: bool) -> Vec<String> {
    let layout = Layout::from_xml(XP_AND_OBJECTIVE);
    let strings = strings();
    let cx = Context {
        art,
        ..context(&layout, &strings)
    };
    let readout = Readout {
        campaign_event,
        ..Readout::blank()
    };
    draw_list(&cx, &readout)
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// An `SP.xml` event is being raced: the XP caption is up and `OBJECTIVE` is not. Dropping the
/// `RaceXPTxt` gate draws `FE_XP` in the next test; dropping the `ObjectiveTxt` arm draws
/// `IG_HUD_OBJECTIVE` here.
#[test]
fn a_campaign_event_draws_the_xp_caption_and_never_objective() {
    let text = drawn(oag_2048::hud::ART, true);
    assert_eq!(text, ["FE_XP", "IG_HUD_LAP"]);
}

/// The HD and Fury Campaigns' races bind no event: neither line is drawn, the rest of the HUD is.
#[test]
fn a_race_with_no_event_draws_neither_line() {
    let text = drawn(oag_2048::hud::ART, false);
    assert_eq!(text, ["IG_HUD_LAP"]);
}
