//! The border pass of an outlined HUD font: which alpha it draws at.

use super::draw::*;
use super::tests::{context, strings};
use super::*;

/// Pure's own spelling, a space before the quote (`BorderColor= "0x40000000"`).
const LABEL: &str = r#"
<Screen>
<Screen name="HUD">
<Text name="CurrentTime">
<Values scale="0.9" font="HUD" align="left" x="4" y="268" BorderColor= "0x40000000"/>
</Text>
</Screen>
</Screen>
"#;

fn border_alpha(art: &'static oag_title::HudArt) -> f32 {
    let layout = Layout::from_xml(LABEL);
    let strings = strings();
    let cx = Context {
        art,
        ..context(&layout, &strings)
    };
    let frame = draw_list(&cx, &Readout::blank());
    frame
        .hud_text
        .iter()
        .find_map(|draw| match draw {
            Draw::Text {
                border: Some(border),
                ..
            } => Some(border[3]),
            _ => None,
        })
        .expect("the label draws")
}

/// Pure's outline pass is the font's alpha times `0x66/255`, whatever the layout authors
/// (`oag_title::hud::TextOutlineAlpha`); Pulse draws the authored `0x40`. Dropping the field
/// on Pure or the `outline_colour` call leaves Pure at `0x40/255`.
#[test]
fn pures_outline_alpha_is_the_measured_byte_and_pulses_is_the_layouts() {
    assert!((border_alpha(oag_pure::hud::ART) - 102.0 / 255.0).abs() < 1e-6);
    assert!((border_alpha(oag_pulse::hud::ART) - 64.0 / 255.0).abs() < 1e-6);
}
