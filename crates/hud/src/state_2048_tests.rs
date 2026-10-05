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
    let past = Readout {
        zone: 40,
        ..Readout::blank()
    };
    assert_eq!(names(&layout, &past, oag_2048::hud::ART).len(), 10);
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
