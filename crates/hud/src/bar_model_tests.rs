//! [`super::bar_models::bar_model_draws`] and Pure's `SpeedL`/`SpeedR` digits, on a fixture; the
//! disc's own models are checked in `oag-raceplay`'s `pickup_icon_ground_truth`.

use super::bar_models::bar_model_draws;
use super::draw::*;
use super::*;

const PURE_GAUGE: &str = r#"
<Screen>
<Mode3D>
<Values mode="orthographic"></Values>
<Model name="Speed3D"><Values Src="Data\HUD\Bar_1.vex" x="394" y="20" z="0.0" ztest="0"></Values></Model>
<Model name="Shield3D"><Values Src="Data\HUD\Bar_3.vex" x="394" y="20" z="0.0" ztest="0"></Values></Model>
<Model name="Overlay3D"><Values Src="Data\HUD\Bar_outline_1.vex" x="394" y="20" z="-1" ztest="0"></Values></Model>
</Mode3D>
</Screen>
"#;

fn placed(x: u32, width: u32, extent: [f32; 2], centre: [f32; 2]) -> crate::sprite::Placed {
    crate::sprite::Placed {
        x,
        y: 0,
        width,
        height: extent[1] as u32,
        quad_extent: Some(extent),
        quad_centre: Some(centre),
        blend: None,
    }
}

fn gauge_draws(speed: f32, shield: f32) -> Vec<Draw> {
    let layout = Layout::from_xml(PURE_GAUGE);
    let strings = oag_ui::language::StringTable::default();
    let sheet = crate::sprite::Sheet::placed_at(&[
        (
            r"Data\HUD\Bar_1.vex",
            placed(0, 169, [169.0, 14.0], [-0.5, -6.0]),
        ),
        (
            r"Data\HUD\Bar_3.vex",
            placed(200, 166, [166.0, 9.0], [-2.0, 7.5]),
        ),
        (
            r"Data\HUD\Bar_outline_1.vex",
            placed(400, 256, [256.0, 32.0], [0.0, 0.0]),
        ),
    ]);
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_pure::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
    };
    let readout = Readout {
        speed_kmh: speed,
        shield,
        shield_max: 100.0,
        ..Readout::blank()
    };
    bar_model_draws(&cx, &readout)
}

fn rects(draws: &[Draw]) -> Vec<[f32; 4]> {
    draws
        .iter()
        .map(|draw| match draw {
            Draw::Sprite { rect, .. } => *rect,
            other => panic!("a gauge draws plain sprites: {other:?}"),
        })
        .collect()
}

#[test]
fn the_fills_are_cropped_from_the_left_and_the_outline_is_whole() {
    let half = rects(&gauge_draws(350.0, 50.0));
    assert_eq!(
        half,
        vec![
            [309.0, 239.0, 84.5, 14.0],
            [309.0, 255.0, 83.0, 9.0],
            [266.0, 236.0, 256.0, 32.0],
        ]
    );
}

#[test]
fn a_bar_at_zero_draws_nothing_and_the_outline_stays() {
    let draws = rects(&gauge_draws(0.0, 0.0));
    assert_eq!(draws, vec![[266.0, 236.0, 256.0, 32.0]]);
}

#[test]
fn a_title_with_no_bar_models_draws_no_gauge() {
    let layout = Layout::from_xml(PURE_GAUGE);
    let strings = oag_ui::language::StringTable::default();
    let sheet = crate::sprite::Sheet::default();
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
    };
    assert!(bar_model_draws(&cx, &Readout::blank()).is_empty());
}

#[test]
fn the_speed_digits_split_after_the_hundreds() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="SpeedL"><Values font="HUD" x="434" y="235"/></Text>
<Text name="SpeedR"><Values font="HUD" x="436" y="235"/></Text></Screen>"#,
    );
    let label = |name: &str| layout.label(name).expect("authored above").clone();
    let text = |name: &str, speed: f32| {
        let readout = Readout {
            speed_kmh: speed,
            ..Readout::blank()
        };
        text_for(
            &label(name),
            &readout,
            &oag_ui::language::StringTable::default(),
            true,
            false,
            None,
            true,
            false,
        )
    };
    assert_eq!(text("SpeedL", 0.0).as_deref(), Some(""));
    assert_eq!(text("SpeedR", 0.0).as_deref(), Some("0"));
    assert_eq!(text("SpeedR", 4.9).as_deref(), Some("0"));
    assert_eq!(text("SpeedR", 57.0).as_deref(), Some("57"));
    assert_eq!(text("SpeedL", 57.0).as_deref(), Some(""));
    assert_eq!(text("SpeedL", 123.4).as_deref(), Some("1"));
    assert_eq!(text("SpeedR", 123.4).as_deref(), Some("23"));
    assert_eq!(text("SpeedL", 1000.0).as_deref(), Some("10"));
}
