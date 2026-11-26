//! The anchor each title's Zone `Score` takes. HD's widget authors `ScaleX="1.3"
//! ScaleY="1.5"` and `align="left"` at its frame's origin, x 960.

use super::*;
use crate::draw::draw_list;
use crate::tests::{context, strings};
use crate::{Layout, Readout};
use oag_ui::frontend::Draw;

const SCORE: &str = r#"
<Screen>
<Text name="Score"><Values ScaleX="1.3" ScaleY="1.5" font="HUD" align="left" vertalign="middle" x="960" y="140"/></Text>
</Screen>
"#;

/// The `3` of HD's `PulseHud.fnt`: advance 52, bearing 5, ink 43
/// (`crates/raceplay/tests/hd_hud_ground_truth.rs` reads the real one).
fn three() -> TemplateDigit {
    TemplateDigit {
        cell: Cell {
            x: 0,
            y: 0,
            width: 43,
            height: 52,
            advance: 52.0,
            bearing: 5.0,
            ink: 43.0,
            extend: 0,
        },
        texel_scale: 1.0,
    }
}

/// The value's own anchor and alignment: the last draw of it, a text shadow being first.
fn placed(art: &'static oag_title::HudArt, readout: &Readout) -> Option<(f32, Align)> {
    let layout = Layout::from_xml(SCORE);
    let strings = strings();
    let cx = Context {
        art,
        template_digit: Some(three()),
        ..context(&layout, &strings)
    };
    draw_list(&cx, readout)
        .hud_text
        .iter()
        .filter_map(|draw| match draw.unstretched() {
            Draw::Text { x, align, text, .. } if text == &readout.score.to_string() => {
                Some((*x, *align))
            }
            _ => None,
        })
        .next_back()
}

fn zone(score: i32) -> Readout {
    Readout {
        zone: 5,
        score,
        ..Readout::blank()
    }
}

/// The left edge is 960 less half the width of as many `3`s as the value has digits, at the
/// horizontal scale 1.3: `3333` is 259.5 wide, `33333` 327.5. Fails if HD's art stops naming
/// the rule, if the width is taken at the vertical scale, or from the value's own digits.
#[test]
fn hd_hangs_the_zone_score_on_a_string_of_threes() {
    for (score, x) in [(1111, 830.25), (5911, 830.25), (12845, 796.25)] {
        let (got, align) = placed(oag_hd::hud::ART, &zone(score)).expect("a score");
        assert!((got - x).abs() < 0.05, "{score}: {got} vs {x}");
        assert_eq!(align, Align::Left);
    }
    let (got, _) = placed(oag_omega::hud::ART, &zone(5911)).expect("a score");
    assert!((got - 830.25).abs() < 0.05, "{got}");
}

#[test]
fn only_a_title_that_moves_it_moves_a_zone_score() {
    assert_eq!(
        placed(oag_pulse::hud::ART, &zone(1111)),
        Some((960.0, Align::Left))
    );
    assert_eq!(
        placed(oag_2048::hud::ART, &zone(1111)),
        Some((960.0, Align::Centre))
    );
}

/// An Eliminator's `Score` keeps its authored anchor.
#[test]
fn a_score_outside_a_zone_run_is_as_authored() {
    let eliminator = Readout {
        zone: 0,
        ..zone(1111)
    };
    let got = placed(oag_hd::hud::ART, &eliminator);
    assert!(got.is_none_or(|(x, _)| x == 960.0), "{got:?}");
}
