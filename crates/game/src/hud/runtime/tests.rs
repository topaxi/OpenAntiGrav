//! HD's runtime HUD rules against a small layout shaped like its own:
//! `DamageBarBg` and its `DamageBar` sharing one source rectangle, a decoy
//! `DamageBar` elsewhere in the atlas (the pickup bar's, which `arcade_hud.xml`
//! also composes), `ShieldBarText`, and the two arcs.

use super::*;
use crate::hud::{Layout, draw_list};

const SRC: &str = r"Data\HUD\Textures\HUD_Components.gtf";

fn layout() -> Layout {
    let mut xml = format!(
        r#"<Screen>
             <Image name="DamageBar">
               <Values x="0" y="200" width="78" height="56" U="453" V="648"
                       TxtrWidth="-78" TxtrHeight="56" Src="{SRC}"/>
             </Image>
             <Image name="DamageBarBg">
               <Values x="0" y="0" width="254" height="194" U="770" V="186"
                       TxtrWidth="254" TxtrHeight="194" Src="{SRC}"/>
             </Image>
             <Image name="DamageBar">
               <Values x="0" y="0" width="254" height="194" U="770" V="186"
                       TxtrWidth="254" TxtrHeight="194" color="0xFFFF0000" Src="{SRC}"/>
             </Image>
             <Text name="ShieldBarText">
               <Values font="HUDSmall" x="0" y="0" color="0x94FF0000"/>
             </Text>"#
    );
    for k in 0..7 {
        xml.push_str(&format!(
            r#"<Image name="LapBar{k}"><Values x="{k}" y="400" width="1" height="1"
                 U="339" V="{k}" TxtrWidth="1" TxtrHeight="1" Src="{SRC}"/></Image>"#
        ));
    }
    for k in 0..8 {
        xml.push_str(&format!(
            r#"<Image name="PosBar{k}"><Values x="{k}" y="500" width="1" height="1"
                 U="376" V="{k}" TxtrWidth="1" TxtrHeight="1" Src="{SRC}"/></Image>"#
        ));
    }
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
            blend: None,
        },
    )])
}

/// Runs `f` against an HD context over [`layout`].
fn with_hd<R>(art: &oag_title::HudArt, f: impl FnOnce(&Context<'_>) -> R) -> R {
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
    };
    f(&cx)
}

fn readout(shield: f32) -> Readout {
    Readout {
        shield,
        shield_max: 100.0,
        ..Readout::blank()
    }
}

fn argb(argb: u32) -> [f32; 4] {
    argb_to_rgba(argb)
}

fn fill_draw(readout: &Readout) -> Option<Draw> {
    with_hd(oag_hd::hud::ART, |cx| {
        let bg = cx.layout.sprite("DamageBarBg").expect("authored");
        shield_fill(cx, readout, bg)
    })
}

fn lit(readout: &Readout) -> Vec<String> {
    lit_segments(oag_hd::hud::RUNTIME, readout)
}

fn names(prefix: &str, ks: &[u32]) -> Vec<String> {
    ks.iter().map(|k| format!("{prefix}{k}")).collect()
}

/// Lap 1 of 3 lights the last five of seven - `talons-matched/00.png`.
#[test]
fn lap_one_of_three_lights_the_last_five_segments() {
    let r = Readout {
        lap: 1,
        laps: 3,
        ..Readout::blank()
    };
    assert_eq!(lit(&r), names("LapBar", &[2, 3, 4, 5, 6]));
}

/// The final lap lights all seven, and a race with no lap count none.
#[test]
fn the_lap_arc_fills_on_the_last_lap_and_is_absent_without_a_count() {
    let last = Readout {
        lap: 3,
        laps: 3,
        ..Readout::blank()
    };
    assert_eq!(lit(&last), names("LapBar", &[0, 1, 2, 3, 4, 5, 6]));
    let none = Readout {
        lap: 1,
        laps: 0,
        ..Readout::blank()
    };
    assert!(lit(&none).is_empty());
}

/// Eighth shows one segment and seventh two (`talons-matched/00.png`,
/// `03.png`), first all eight, and no place none.
#[test]
fn the_place_arc_counts_from_the_back_of_an_eight_segment_ring() {
    let at = |place| Readout {
        place,
        ..Readout::blank()
    };
    assert_eq!(lit(&at(8)), names("PosBar", &[0]));
    assert_eq!(lit(&at(7)), names("PosBar", &[0, 1]));
    assert_eq!(lit(&at(1)), names("PosBar", &[0, 1, 2, 3, 4, 5, 6, 7]));
    assert!(lit(&at(0)).is_empty());
}

/// The number truncates the way `fctiwz` does and carries no `%`.
#[test]
fn the_shield_number_truncates() {
    assert_eq!(shield_digits(&readout(99.6)), "99");
    assert_eq!(shield_digits(&readout(100.0)), "100");
    assert_eq!(shield_digits(&readout(7.99)), "7");
}

/// Above the threshold with no hit: the background white, the fill and the
/// number opaque `0x1664FF` - not the number's authored translucent red.
#[test]
fn a_steady_readout_is_opaque_hd_blue_over_a_white_background() {
    let r = readout(98.0);
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(colour(cx, &r, "DamageBarBg"), Some(argb(0xFFFF_FFFF)));
        assert_eq!(colour(cx, &r, "ShieldBarText"), Some(argb(0xFF16_64FF)));
        assert_eq!(colour(cx, &r, "DamageBar"), None, "drawn by shield_fill");
    });
    let Some(Draw::Sprite { color, .. }) = fill_draw(&r) else {
        panic!("a fill at 98%");
    };
    assert_eq!(color, argb(0xFF16_64FF));
}

/// The fill is the `DamageBar` sharing its background's source rectangle,
/// never the pickup bar's piece that `arcade_hud.xml` names the same.
#[test]
fn the_fill_is_the_damage_bar_over_its_own_background() {
    let Some(Draw::Sprite { uv, .. }) = fill_draw(&readout(100.0)) else {
        panic!("a full fill");
    };
    assert_eq!(uv, [770.0, 186.0, 254.0, 194.0]);
}

/// Half a shield crops the top half away and keeps the bottom edge.
#[test]
fn half_a_shield_crops_the_fill_from_the_top() {
    let Some(Draw::Sprite { rect, uv, .. }) = fill_draw(&readout(50.0)) else {
        panic!("a half fill");
    };
    let full = with_hd(oag_hd::hud::ART, |cx| {
        cx.layout.sprite("DamageBarBg").expect("authored").rect
    });
    assert_eq!(rect[1] + rect[3], full[1] + full[3], "bottom edge fixed");
    assert_eq!(rect[3], full[3] * 0.5);
    assert_eq!(uv, [770.0, 186.0 + 97.0, 254.0, 97.0]);
    assert!(fill_draw(&readout(0.0)).is_none(), "nothing at zero");
}

/// At 15 % the background flashes red and the number blinks, eight phases a
/// second, while the fill stays opaque.
#[test]
fn a_critical_readout_flashes_the_background_and_blinks_the_number() {
    let at = |race_ticks| Readout {
        race_ticks,
        ..readout(15.0)
    };
    // Tick 0 is phase 0 (on); tick 8 is 0.133 s, phase 1 (off).
    let (on, off) = (at(0), at(8));
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(colour(cx, &on, "DamageBarBg"), Some(argb(0xFFFF_0000)));
        assert_eq!(colour(cx, &on, "ShieldBarText"), Some(argb(0xFF16_64FF)));
        assert_eq!(colour(cx, &off, "DamageBarBg"), Some(argb(0xFFFF_FFFF)));
        assert_eq!(colour(cx, &off, "ShieldBarText"), Some(argb(0x0016_64FF)));
    });
    for r in [on, off] {
        let Some(Draw::Sprite { color, .. }) = fill_draw(&r) else {
            panic!("a fill at 15%");
        };
        assert_eq!(color, argb(0xFF16_64FF));
    }
}

/// A hit flashes a healthy readout too, through [`Readout::shield_flashing`].
#[test]
fn a_hit_flashes_a_healthy_readout() {
    let r = Readout {
        shield_flashing: true,
        ..readout(90.0)
    };
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(colour(cx, &r, "DamageBarBg"), Some(argb(0xFFFF_0000)));
    });
}

/// Eliminator draws the readout white - the original's mode test.
#[test]
fn eliminator_draws_the_readout_white() {
    let r = Readout {
        mode: oag_race::Mode::Eliminator,
        ..readout(80.0)
    };
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(colour(cx, &r, "ShieldBarText"), Some(argb(0xFFFF_FFFF)));
    });
}

/// A title with no runtime read overrides nothing and draws no arc.
#[test]
fn a_title_without_a_runtime_read_is_untouched() {
    let r = Readout {
        place: 8,
        ..readout(10.0)
    };
    with_hd(oag_pulse::hud::ART, |cx| {
        assert_eq!(colour(cx, &r, "DamageBarBg"), None);
        assert_eq!(colour(cx, &r, "ShieldBarText"), None);
        assert!(arc_sprites(cx, &r).is_empty());
    });
}

/// Through the whole draw list: the fill lands straight after its
/// background, so it paints over it.
#[test]
fn draw_list_paints_the_fill_straight_over_its_background() {
    let r = Readout {
        place: 8,
        ..readout(100.0)
    };
    let sprites = with_hd(oag_hd::hud::ART, |cx| draw_list(cx, &r).sprites);
    let uvs: Vec<[f32; 4]> = sprites
        .iter()
        .filter_map(|d| match d {
            Draw::Sprite { uv, .. } => Some(*uv),
            _ => None,
        })
        .collect();
    let bg = uvs
        .iter()
        .position(|uv| *uv == [770.0, 186.0, 254.0, 194.0])
        .expect("the background draws");
    assert_eq!(uvs.get(bg + 1), Some(&[770.0, 186.0, 254.0, 194.0]));
    assert!(uvs.contains(&[376.0, 0.0, 1.0, 1.0]), "PosBar0 at eighth");
}
