//! Pulse's speed and shield bars against a layout shaped like `TimeTrial_HUD.xml`'s: the
//! widths and the moves are `FUN_0881a3e0`'s and `Hud_UpdateEnergyBar`'s, read live on PPSSPP
//! (59 of 60 consecutive ticks matched the law; the other was the shield bar changing in the
//! same tick).

use crate::Readout;
use crate::bar_tip::{dx, text};
use crate::draw::Context;
use crate::{Draw, Layout, draw_list};

const SRC: &str = r"Data\HUD\Textures\hud.mip";

fn layout() -> Layout {
    let image = |name: &str, x: u32, y: u32, w: u32, h: u32| {
        format!(
            r#"<Image name="{name}"><Values x="{x}" y="{y}" width="{w}" height="{h}"
                 U="100" V="10" TxtrWidth="{w}" TxtrHeight="{h}" Src="{SRC}"/></Image>"#
        )
    };
    let text = |name: &str, x: u32, y: u32| {
        format!(
            r#"<Text name="{name}"><Values font="HUD" x="{x}" y="{y}" string="0" align="right"/></Text>"#
        )
    };
    let mut xml = String::from("<Screen>");
    for (name, x, y, w, h) in [
        ("SpeedBarBg", 306, 220, 168, 26),
        ("SpeedBar", 306, 220, 168, 26),
        ("SpeedBarMark", 302, 215, 9, 7),
        ("ShieldBarBg", 306, 246, 140, 12),
        ("ShieldBar", 306, 246, 140, 12),
        ("ShieldBarMark", 302, 258, 9, 7),
    ] {
        xml.push_str(&image(name, x, y, w, h));
    }
    xml.push_str(&text("SpeedBarText", 300, 210));
    xml.push_str(&text("ShieldBarText", 300, 258));
    xml.push_str("</Screen>");
    Layout::from_xml(&xml)
}

fn sheet() -> crate::sprite::Sheet {
    crate::sprite::Sheet::placed_at(&[(
        SRC,
        crate::sprite::Placed {
            x: 0,
            y: 0,
            width: 1024,
            height: 1024,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        },
    )])
}

fn with_art<R>(art: &oag_title::HudArt, f: impl FnOnce(&Context<'_>) -> R) -> R {
    let layout = layout();
    let strings = oag_ui::language::StringTable::default();
    let sheet = sheet();
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
    };
    f(&cx)
}

fn at(speed_kmh: f32, shield: f32) -> Readout {
    Readout {
        speed_kmh,
        shield,
        shield_max: 100.0,
        ..Readout::blank()
    }
}

/// The `[x, width]` of the first drawn sprite that sits on `y` and is `height` tall, and its uv.
fn drawn(cx: &Context<'_>, readout: &Readout, y: f32) -> Vec<([f32; 4], [f32; 4])> {
    draw_list(cx, readout)
        .sprites
        .iter()
        .filter_map(|d| match d {
            Draw::Sprite { rect, uv, .. } if rect[1] == y => Some((*rect, *uv)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_speed_fill_is_a_floored_width_of_the_right_part_of_its_art() {
    with_art(oag_pulse::hud::ART, |cx| {
        // Standstill: the 0.04 floor, `floor(0.04 * 168)` = 6 px, the art's right 4 %.
        let still = drawn(cx, &at(0.0, 100.0), 220.0);
        let fill = still.last().expect("the fill draws at standstill");
        assert_eq!(fill.0[2], 6.0);
        assert!(
            (fill.1[0] - (100.0 + 168.0 * 0.96)).abs() < 1e-3,
            "{:?}",
            fill.1
        );
        assert!((fill.1[2] - 168.0 * 0.04).abs() < 1e-3, "{:?}", fill.1);
        // 4.9 km/h reads as zero, as the executable's `< 5.0` does.
        assert_eq!(drawn(cx, &at(4.9, 100.0), 220.0).last().unwrap().0[2], 6.0);
        // Half of 700 km/h: 0.04 + 0.96 * 0.5 = 0.52, floor(87.36) = 87.
        let half = drawn(cx, &at(350.0, 100.0), 220.0);
        assert_eq!(half.last().unwrap().0[2], 87.0);
        // Full: the whole art.
        let full = drawn(cx, &at(900.0, 100.0), 220.0);
        let fill = full.last().unwrap();
        assert_eq!((fill.0[2], fill.1[0], fill.1[2]), (168.0, 100.0, 168.0));
    });
}

#[test]
fn the_marker_and_the_label_ride_the_tip() {
    with_art(oag_pulse::hud::ART, |cx| {
        // Authored column plus the floored tip minus six: 0 at standstill (6 - 6).
        assert_eq!(dx(cx, &at(0.0, 100.0), "SpeedBarText"), 0.0);
        assert_eq!(dx(cx, &at(0.0, 100.0), "SpeedBarMark"), 0.0);
        assert_eq!(dx(cx, &at(350.0, 100.0), "SpeedBarText"), 81.0);
        assert_eq!(dx(cx, &at(350.0, 100.0), "SpeedBarMark"), 81.0);
        // The shield: 99.6 % -> 0.04 + 0.96 * 0.996 = 0.99616, floor(139.46) = 139.
        assert_eq!(dx(cx, &at(0.0, 99.6), "ShieldBarText"), 133.0);
        assert_eq!(dx(cx, &at(0.0, 99.6), "ShieldBarMark"), 133.0);
        // The fills and anything else do not move.
        assert_eq!(dx(cx, &at(350.0, 100.0), "SpeedBar"), 0.0);
        assert_eq!(dx(cx, &at(350.0, 100.0), "Lap"), 0.0);
    });
    // A title without the law moves nothing.
    with_art(oag_hd::hud::ART, |cx| {
        assert_eq!(dx(cx, &at(350.0, 100.0), "SpeedBarText"), 0.0);
    });
}

#[test]
fn the_labels_are_truncated_and_a_crawl_reads_zero() {
    let art = oag_pulse::hud::ART;
    assert_eq!(
        text(art, &at(455.9, 99.6), "SpeedBarText", true).as_deref(),
        Some("455 kmh")
    );
    assert_eq!(
        text(art, &at(4.9, 99.6), "SpeedBarText", true).as_deref(),
        Some("0 kmh")
    );
    assert_eq!(
        text(art, &at(5.0, 99.6), "SpeedBarText", false).as_deref(),
        Some("5")
    );
    assert_eq!(
        text(art, &at(0.0, 99.6), "ShieldBarText", true).as_deref(),
        Some("99%")
    );
    assert_eq!(
        text(art, &at(0.0, 100.0), "ShieldBarText", true).as_deref(),
        Some("100%")
    );
    assert_eq!(text(art, &at(0.0, 100.0), "Lap", true), None);
    assert_eq!(
        text(oag_hd::hud::ART, &at(455.9, 99.6), "SpeedBarText", true),
        None
    );
}
