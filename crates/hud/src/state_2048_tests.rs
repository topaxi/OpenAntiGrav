//! 2048's state-gated widgets: [`crate::dialect_2048::state_sprites`].

use super::*;

fn image(name: &str, x: f32, w: f32) -> String {
    format!(
        r#"<Image name="{name}"><Values x="{x}" y="0" width="{w}" height="10" U="0" V="0"
           TxtrWidth="{w}" TxtrHeight="10" Color="0xFFFFFFFF"
           Src="Data\XML\2048_hud\Texture\hud_2048.gxt"/></Image>"#
    )
}

fn layout() -> Layout {
    let mut xml = String::from("<Screen>");
    for i in 0..5 {
        xml += &image(&format!("SpeedBar{i}"), 10.0 * i as f32, 10.0);
    }
    xml += &image("ThrustBar", 0.0, 200.0);
    xml += &image("PilotAssist", 300.0, 20.0);
    for i in 0..10 {
        xml += &image(&format!("ZoneLight{i}"), 400.0 + i as f32, 1.0);
    }
    xml += &image("ManualShield", 500.0, 20.0);
    xml += "</Screen>";
    Layout::from_xml(&xml)
}

fn names(layout: &Layout, readout: &Readout, art: &oag_title::HudArt) -> Vec<String> {
    dialect_2048::state_sprites(layout, art, readout)
        .into_iter()
        .map(|s| s.name)
        .collect()
}

#[test]
fn a_speed_segment_lights_at_its_multiple_of_140_kmh() {
    let layout = layout();
    let art = oag_2048::hud::ART;
    let at = |kmh: f32| {
        names(
            &layout,
            &Readout {
                speed_kmh: kmh,
                ..Readout::blank()
            },
            art,
        )
    };
    assert!(at(139.9).is_empty());
    assert_eq!(at(140.0), ["SpeedBar0"]);
    assert_eq!(at(300.0), ["SpeedBar0", "SpeedBar1"]);
    assert_eq!(at(700.0).len(), 5);
}

#[test]
fn the_thrust_bar_crops_to_its_chase_value() {
    let layout = layout();
    let readout = Readout {
        thrust_chase_percent: 50.0,
        ..Readout::blank()
    };
    let sprites = dialect_2048::state_sprites(&layout, oag_2048::hud::ART, &readout);
    let bar = sprites.iter().find(|s| s.name == "ThrustBar").unwrap();
    assert_eq!(bar.rect[2], 100.0);
    assert_eq!(bar.uv[2], 100.0);
    assert!(names(&layout, &Readout::blank(), oag_2048::hud::ART).is_empty());
}

#[test]
fn zone_lights_fill_from_the_last_index_one_per_zone() {
    let layout = layout();
    let readout = Readout {
        zone: 2,
        ..Readout::blank()
    };
    assert_eq!(
        names(&layout, &readout, oag_2048::hud::ART),
        ["ZoneLight8", "ZoneLight9"]
    );
    let at = |zone: u32| {
        names(
            &layout,
            &Readout {
                zone,
                ..Readout::blank()
            },
            oag_2048::hud::ART,
        )
        .len()
    };
    assert_eq!(at(0), 0);
    assert_eq!(
        at(1),
        10,
        "X == 0 lights every dash in the decompiled branch"
    );
    assert_eq!(at(5), 5);
    assert_eq!(at(10), 10);
    assert_eq!(at(11), 0, "9u - X wraps past zone 10");
}

#[test]
fn the_pilot_assist_icon_follows_its_flag() {
    let layout = layout();
    let on = Readout {
        pilot_assist: true,
        ..Readout::blank()
    };
    assert_eq!(names(&layout, &on, oag_2048::hud::ART), ["PilotAssist"]);
}

/// A quarter of the Shield left draws the bottom quarter of `ManualShield`'s box and source
/// rect; none left draws nothing. Fails if the widget is stretched, cropped from the bottom, or
/// drawn with no Shield up, and if a title with no `shield_drain_widget` draws it.
#[test]
fn the_running_shield_drains_its_widget_from_the_top() {
    let layout = layout();
    let drain = |art: &oag_title::HudArt, fraction: f32| {
        let readout = Readout {
            shield_drain_fraction: fraction,
            ..Readout::blank()
        };
        dialect_2048::shield_drain(&layout, art, &readout)
    };
    let full = drain(oag_2048::hud::ART, 1.0).expect("up at full");
    let quarter = drain(oag_2048::hud::ART, 0.25).expect("up at a quarter");
    assert_eq!(full.rect, [500.0, 0.0, 20.0, 10.0]);
    assert_eq!(quarter.rect, [500.0, 7.5, 20.0, 2.5]);
    assert_eq!(quarter.uv, [0.0, 7.5, 20.0, 2.5]);
    assert!(drain(oag_2048::hud::ART, 0.0).is_none());
    assert!(drain(oag_omega::hud::ART_2048, 0.25).is_some());
    assert!(drain(oag_pulse::hud::ART, 1.0).is_none());
}

#[test]
fn no_other_title_draws_these() {
    let layout = layout();
    let loud = Readout {
        speed_kmh: 900.0,
        thrust_chase_percent: 100.0,
        zone: 5,
        pilot_assist: true,
        ..Readout::blank()
    };
    assert!(names(&layout, &loud, oag_pulse::hud::ART).is_empty());
}

const ZONE_SCORE: &str = r#"
<Screen>
<Text name="ScoreTxt"><Values idstring="IG_HUD_SCORE" font="HUD" align="centre" vertalign="top" x="480" y="20"/></Text>
<Text name="Score"><Values string="" font="HUD" align="left" vertalign="top" x="480" y="60"/></Text>
</Screen>
"#;

/// The Zone `SCORE` value is centred under `SCORE`: `FUN_811538c2` writes the left-anchored
/// widget's x as `ScoreTxt`'s x minus half the width of the value's digits. Fails if 2048's
/// `Score` stays left-anchored at the authored x (the value hangs off to the right of `SCORE`),
/// or if another title's `Score` is moved.
#[test]
fn the_2048_zone_score_value_is_centred_under_its_caption() {
    use super::draw::*;
    use super::tests::{context, strings};
    let layout = Layout::from_xml(ZONE_SCORE);
    let strings = strings();
    let aligns = |art: &'static oag_title::HudArt| {
        let cx = Context {
            art,
            ..context(&layout, &strings)
        };
        let readout = Readout {
            zone: 5,
            score: 1780,
            ..Readout::blank()
        };
        draw_list(&cx, &readout)
            .hud_text
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, x, align, .. } if text == "1780" => Some((x, align)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(aligns(oag_2048::hud::ART), [(480.0, Align::Centre)]);
    assert_eq!(aligns(oag_pulse::hud::ART), [(480.0, Align::Left)]);
}

/// With nothing held and the icon's height still above zero, 2048 keeps drawing the last weapon's
/// icon at the slot with its height scaled by `hud+0x224` and the frame at full width
/// (`Hud_UpdatePickupIcon`, not-held branch). Fails if the release draws nothing, or if another
/// title draws the leftover.
#[test]
fn the_2048_icon_collapses_in_height_after_release_with_its_frame_up() {
    use super::draw::*;
    use oag_tables::weapons::Weapon;
    let layout = Layout::from_xml(super::tests::SAMPLE_2048_PICKUP);
    let src = layout.sprite("PickupIcon").expect("icon").src.clone();
    let sheet = crate::sprite::Sheet::placed_at(&[(
        src.as_str(),
        crate::sprite::Placed {
            x: 0,
            y: 0,
            width: 512,
            height: 512,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        },
    )]);
    let strings = super::tests::strings();
    let sprites = |art: &'static oag_title::HudArt, high: f32| {
        let cx = Context {
            art,
            sheet: &sheet,
            ..super::tests::context(&layout, &strings)
        };
        let readout = Readout {
            pickup: None,
            pickup_releasing: Some(Weapon::Missile),
            pickup_pop: Some([1.0, high]),
            pickup_fly: Some(0.0),
            ..Readout::blank()
        };
        draw_list(&cx, &readout)
            .sprites
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, .. } => Some(rect),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let full = sprites(oag_2048::hud::ART, 1.0);
    let half = sprites(oag_2048::hud::ART, 0.5);
    assert_eq!(full.len(), 2, "frame and icon: {full:?}");
    assert_eq!(half.len(), 2);
    assert_eq!(half[0], full[0], "the frame holds its full width");
    assert_eq!(half[1][2], full[1][2], "the icon keeps its width");
    assert_eq!(half[1][3], full[1][3] * 0.5, "and loses half its height");
    assert_eq!(sprites(oag_2048::hud::ART, 0.0)[1][3], 0.0, "flat at zero");
    assert!(sprites(oag_pulse::hud::ART, 0.5).is_empty());
}
