//! The `SubWeapon` line: what a readout puts in it and which titles draw it.
//!
//! The widget is `Arcade_HUD.xml`'s own, copied off the USA PSP disc with
//! `just wad cat --expand`.

use super::draw::*;
use super::tests::{context, strings};
use super::*;

const SUB_WEAPON: &str = r#"
<Screen>
<Screen name="HUD">
<Text name="SubWeapon">
<Values scale="0.6" font="HUD" align="centre" x="240" y="45" CalcBlur="1" BorderColor="FEConst->HudBGColour"/>
</Text>
</Screen>
</Screen>
"#;

/// Every `HUD`-font string the frame draws, in order.
fn drawn(
    layout: &Layout,
    art: &'static oag_title::HudArt,
    sub_weapon: Option<SubWeapon>,
) -> Vec<String> {
    let strings = strings();
    let cx = Context {
        art,
        ..context(layout, &strings)
    };
    let readout = Readout {
        sub_weapon,
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

/// The Cannon's count is printed `%d` and the Autopilot's time `%1.2f`, and both reach the
/// draw list through the layout's own widget. Dropping the `"SubWeapon"` arm of `text_for`,
/// or the readout field, empties all three.
#[test]
fn the_cannons_rounds_and_the_autopilots_seconds_reach_the_sub_weapon_widget() {
    let layout = Layout::from_xml(SUB_WEAPON);
    let art = oag_pulse::hud::ART;
    assert_eq!(drawn(&layout, art, Some(SubWeapon::Rounds(30))), ["30"]);
    assert_eq!(drawn(&layout, art, Some(SubWeapon::Rounds(1))), ["1"]);
    assert_eq!(
        drawn(&layout, art, Some(SubWeapon::Autopilot(5.0))),
        ["5.00"]
    );
    assert_eq!(
        drawn(&layout, art, Some(SubWeapon::Autopilot(0.424))),
        ["0.42"]
    );
}

/// With nothing to say the original writes an empty string, which draws nothing.
#[test]
fn an_empty_sub_weapon_line_draws_nothing() {
    let layout = Layout::from_xml(SUB_WEAPON);
    assert!(drawn(&layout, oag_pulse::hud::ART, None).is_empty());
}

/// Every title but 2048 draws the line; 2048's sits in the grant announcement's cluster, whose
/// runtime visibility is unread - see `oag_title::HudArt::sub_weapon_line`.
#[test]
fn only_the_titles_that_place_the_line_with_the_held_icon_draw_it() {
    let layout = Layout::from_xml(SUB_WEAPON);
    let line = Some(SubWeapon::Rounds(12));
    for (name, art) in [
        ("Pulse", oag_pulse::hud::ART),
        ("Pure", oag_pure::hud::ART),
        ("HD", oag_hd::hud::ART),
        ("Omega", oag_omega::hud::ART),
    ] {
        // HD and Omega draw a black twin under every string (`HudArt::text_shadow`).
        let text = drawn(&layout, art, line);
        assert!(
            !text.is_empty() && text.iter().all(|text| text == "12"),
            "{name}: {text:?}"
        );
    }
    assert!(drawn(&layout, oag_2048::hud::ART, line).is_empty());
}
