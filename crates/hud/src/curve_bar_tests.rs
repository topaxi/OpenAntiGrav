//! HD's speed bar: a `<CurveBar>` the layout parser used to drop.
//!
//! The fixture is `HUD_speed_bars.xml`'s own `SpeedBarBG`/`SpeedBar` pair,
//! attributes as the disc spells them.

use super::*;
use crate::sprite::{Placed, Sheet};

const SPEED_BARS: &str = r#"
<Screen>
<Text name="SpeedBarParent" OffsetX="1406" OffsetY="822">
  <Image name="SpeedBarBG">
    <Values x="14" y="16" width="452" height="191" U="98" V="832" TxtrWidth="452" TxtrHeight="191" src="Data\HUD\Textures\HUD_Components.gtf"></Values>
  </Image>
  <CurveBar name="SpeedBar">
    <Values x="14" y="16" maxWidth="452" CurveStartPos="290" width="452" height="191" U="98" V="832" TxtrWidth="452" TxtrHeight="191" color1="0xFFFFFF0C" color2="0xFFFFFF0C" color3="0xFFFF7F0C" color4="0xFFFF7F0C" src="Data\HUD\Textures\HUD_Components.gtf"></Values>
  </CurveBar>
</Text>
</Screen>
"#;

fn sheet() -> Sheet {
    Sheet::placed_at(&[(
        r"Data\HUD\Textures\HUD_Components.gtf",
        Placed {
            x: 0,
            y: 0,
            width: 1024,
            height: 1024,
            quad_extent: None,
            blend: None,
        },
    )])
}

/// The fill's drawn rectangle and its two edge colours at `kmh`, `None` when
/// nothing is drawn for it.
fn fill_at(kmh: f32) -> Option<([f32; 4], [f32; 4], [f32; 4])> {
    let layout = Layout::from_xml(SPEED_BARS);
    let strings = oag_ui::language::StringTable::default();
    let sheet = sheet();
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_hd::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
    };
    let readout = Readout {
        speed_kmh: kmh,
        ..Readout::blank()
    };
    draw_list(&cx, &readout)
        .sprites
        .into_iter()
        .find_map(|draw| match draw {
            Draw::GradientSprite {
                rect, left, right, ..
            } => Some((rect, left, right)),
            _ => None,
        })
}

#[test]
fn a_curve_bar_parses_as_a_sprite_carrying_its_gradient() {
    let layout = Layout::from_xml(SPEED_BARS);
    let bar = layout.sprite("SpeedBar").expect("SpeedBar is dropped");
    assert_eq!(bar.rect, [1420.0, 838.0, 452.0, 191.0]);
    assert_eq!(bar.uv, [98.0, 832.0, 452.0, 191.0]);
    let [left, right] = bar.gradient.expect("color1..color4 are read");
    // ARGB `0xFFFFFF0C` is opaque yellow and `0xFFFF7F0C` orange.
    assert_eq!(left, argb_to_rgba(0xFFFF_FF0C));
    assert_eq!(right, argb_to_rgba(0xFFFF_7F0C));
    assert!(layout.sprite("SpeedBarBG").unwrap().gradient.is_none());
}

#[test]
fn the_speed_bar_fill_grows_with_speed_and_keeps_the_grade_of_the_whole_bar() {
    assert!(fill_at(0.0).is_none(), "an empty bar draws nothing");

    let (slow, _, slow_right) = fill_at(175.0).expect("a quarter full draws");
    let (fast, _, fast_right) = fill_at(525.0).expect("three quarters full draws");
    assert_eq!(slow[2], 452.0 * 0.25);
    assert_eq!(fast[2], 452.0 * 0.75);
    assert!(fast[2] > slow[2]);
    // Cropping the bar crops the grade too: the further it fills, the further
    // along the yellow-to-orange it ends, so the green channel falls.
    assert!(fast_right[1] < slow_right[1]);

    let (full, left, right) = fill_at(900.0).expect("over the top clamps to full");
    assert_eq!(full[2], 452.0);
    assert_eq!(left, argb_to_rgba(0xFFFF_FF0C));
    let orange = argb_to_rgba(0xFFFF_7F0C);
    assert!(right.iter().zip(orange).all(|(a, b)| (a - b).abs() < 1e-6));
}

const THRUST_BARS: &str = r#"
<Screen>
<Text name="SpeedBarParent" OffsetX="1406" OffsetY="822">
  <Image name="ThrustBarBG">
    <Values x="12" y="53" width="393" height="130" U="0" V="700" TxtrWidth="393" TxtrHeight="130" src="Data\HUD\Textures\HUD_Components.gtf"></Values>
  </Image>
  <CurveBar name="ThrustBar">
    <Values x="12" y="53" maxWidth="393" CurveStartPos="284" width="393" height="130" U="0" V="700" TxtrWidth="393" TxtrHeight="130" color1="0xFFB7D1FF" color2="0xFFB7D1FF" color3="0xFF072476" color4="0xFF072476" src="Data\HUD\Textures\HUD_Components.gtf"></Values>
  </CurveBar>
  <Text name="ThrustBarText"><Values font="HUD" string="0" x="100" y="60"/></Text>
</Text>
</Screen>
"#;

/// The thrust fill's width and the label text at `percent`.
fn thrust_at(percent: f32) -> (Option<f32>, Option<String>) {
    let layout = Layout::from_xml(THRUST_BARS);
    let strings = oag_ui::language::StringTable::default();
    let sheet = sheet();
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_hd::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
    };
    let readout = Readout {
        thrust_percent: percent,
        ..Readout::blank()
    };
    let list = draw_list(&cx, &readout);
    let width = list.sprites.iter().find_map(|draw| match draw {
        Draw::GradientSprite { rect, .. } => Some(rect[2]),
        _ => None,
    });
    let text = list.hud_text.iter().find_map(|draw| match draw {
        Draw::Text { text, .. } if text.ends_with('%') => Some(text.clone()),
        _ => None,
    });
    (width, text)
}

#[test]
fn the_thrust_bar_follows_the_craft_throttle_and_prints_it_truncated() {
    let (none, label) = thrust_at(0.0);
    assert_eq!(none, None, "no throttle draws no fill");
    assert_eq!(label.as_deref(), Some("0%"));

    let (half, label) = thrust_at(50.0);
    assert_eq!(half, Some(393.0 * 0.5));
    assert_eq!(label.as_deref(), Some("50%"));

    // Pilot Assist's 0.99 scale leaves 98.999992 in the original's float,
    // which its `(int)` cast prints as 98, not 99.
    let (assisted, label) = thrust_at(98.999_99);
    assert!(assisted.unwrap() > half.unwrap());
    assert_eq!(label.as_deref(), Some("98%"));
}

#[test]
fn the_speed_bar_scale_is_the_titles_own() {
    assert_eq!(oag_hd::hud::ART.speed_full_kmh, 700);
    assert_eq!(oag_pulse::hud::ART.speed_full_kmh, 600);
    assert_eq!(oag_pure::hud::ART.speed_full_kmh, 600);
    // HD's bar is half full at 350 km/h, where a 600 scale would read 58%.
    let (half, _, _) = fill_at(350.0).expect("draws");
    assert_eq!(half[2], 452.0 * 0.5);
}
