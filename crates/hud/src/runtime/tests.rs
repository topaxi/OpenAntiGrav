//! HD's runtime HUD rules against a small layout shaped like its own:
//! `DamageBarBg` and its `DamageBar` sharing one source rectangle, a decoy
//! `DamageBar` elsewhere in the atlas (the pickup bar's, which `arcade_hud.xml`
//! also composes), `ShieldBarText`, and the two arcs.

use super::*;
use crate::{Layout, draw_list};

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
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        },
    )])
}

/// Runs `f` against an HD context over [`layout`].
fn with_hd<R>(art: &oag_title::HudArt, f: impl FnOnce(&Context<'_>) -> R) -> R {
    with_layout(&layout(), art, f)
}

fn with_layout<R>(
    layout: &Layout,
    art: &oag_title::HudArt,
    f: impl FnOnce(&Context<'_>) -> R,
) -> R {
    let strings = oag_ui::language::StringTable::default();
    let sheet = sheet();
    let cx = Context {
        default_border: layout.default_border(),
        tag_label: None,
        layout,
        strings: &strings,
        sheet: &sheet,
        art,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
        template_digit: None,
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
    let at = |shield_blink_phase_whole| Readout {
        shield_blink_phase_whole,
        ..readout(15.0)
    };
    // The accumulator at 0 is phase 0 (on); at 0.133 s it is phase 1 (off).
    let (on, off) = (at(0.0), at(0.133));
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

/// A hit flashes a healthy readout too, through
/// [`Readout::shield_flashing_whole`] - HD's own window, not Pulse's.
#[test]
fn a_hit_flashes_a_healthy_readout() {
    let r = Readout {
        shield_flashing_whole: true,
        ..readout(90.0)
    };
    let pulse = Readout {
        shield_flashing: true,
        ..readout(90.0)
    };
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(colour(cx, &pulse, "DamageBarBg"), Some(argb(0xFFFF_FFFF)));
    });
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(colour(cx, &r, "DamageBarBg"), Some(argb(0xFFFF_0000)));
    });
}

/// Where the flash starts in its cycle is the accumulator's, not the race
/// clock's: the same race tick draws either phase, 0.9833 s into the
/// accumulator being `floor(7.87)`, odd, so off (the stored value was read off
/// the original; the off start follows from it).
#[test]
fn the_phase_is_the_accumulators_and_not_the_race_clock() {
    let r = |race_ticks, phase| Readout {
        race_ticks,
        shield_blink_phase_whole: phase,
        ..readout(15.0)
    };
    with_hd(oag_hd::hud::ART, |cx| {
        for ticks in [0, 8, 1000] {
            assert_eq!(
                colour(cx, &r(ticks, 0.9833), "DamageBarBg"),
                Some(argb(0xFFFF_FFFF)),
                "off at {ticks} whatever the clock says"
            );
            assert_eq!(
                colour(cx, &r(ticks, 0.0), "DamageBarBg"),
                Some(argb(0xFFFF_0000))
            );
        }
    });
}

/// Absorbing is the third row of the original's table: the background stays
/// white (never the warning colour), and the fill blinks with the number -
/// the opposite of a plain flash, where the fill is solid.
#[test]
fn absorbing_blinks_the_fill_and_the_number_and_keeps_the_background_white() {
    let at = |shield_blink_phase_whole| Readout {
        shield_absorbing: true,
        shield_blink_phase_whole,
        ..readout(90.0)
    };
    let (on, off) = (at(0.0), at(0.133));
    with_hd(oag_hd::hud::ART, |cx| {
        for r in [&on, &off] {
            assert_eq!(colour(cx, r, "DamageBarBg"), Some(argb(0xFFFF_FFFF)));
        }
        assert_eq!(colour(cx, &on, "ShieldBarText"), Some(argb(0xFF16_64FF)));
        assert_eq!(colour(cx, &off, "ShieldBarText"), Some(argb(0x0016_64FF)));
    });
    let colour_of = |r: &Readout| match fill_draw(r) {
        Some(Draw::Sprite { color, .. }) => color,
        _ => panic!("a fill at 90%"),
    };
    assert_eq!(colour_of(&on), argb(0xFF16_64FF));
    assert_eq!(colour_of(&off), argb(0x0016_64FF));
    // And a low shield that is absorbing is still the absorbing row.
    let low = Readout {
        shield: 10.0,
        ..off
    };
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(colour(cx, &low, "DamageBarBg"), Some(argb(0xFFFF_FFFF)));
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

/// The pickup bar is dim with nothing to show (`Hud_UpdatePickupBar`, `0x5affffff`): the two
/// pills always, the hexagon only while no pickup is held. Pulse's art names no idle dim.
#[test]
fn the_pickup_bar_is_dim_while_idle() {
    let idle = argb(0x5aff_ffff);
    let none = readout(98.0);
    let held = Readout {
        pickup: Some(oag_tables::weapons::Weapon::ALL[0]),
        ..readout(98.0)
    };
    with_hd(oag_hd::hud::ART, |cx| {
        for name in ["PickupDamageBG", "PickupAbsorbBG", "PickupBackground"] {
            assert_eq!(colour(cx, &none, name), Some(idle), "{name}");
        }
        assert_eq!(
            colour(cx, &held, "PickupBackground"),
            Some(argb(0x80BE_1E69)),
            "a held weapon takes its table word, not the dim"
        );
        assert_eq!(colour(cx, &held, "PickupDamageBG"), Some(idle));
        assert_eq!(colour(cx, &none, "PickupFarBackground"), None);
    });
}

/// HD's held backdrop plays the fade: the table word scaled by the eased overshoot while it
/// grows, and the last weapon's word fading out after use. Fails if the draw ignores the fade
/// state or reads the layout's colour again.
#[test]
fn the_held_backdrop_plays_the_fade_on_hd() {
    use crate::pickup_fade::{Fade, scale_word};
    use oag_tables::weapons::Weapon;
    let mut fade = Fade::default();
    let step = |fade: &mut Fade, held: Option<Weapon>, ticks: u32| {
        for _ in 0..ticks {
            fade.advance(held, 1.5, 1.0 / 60.0);
        }
    };
    step(&mut fade, Some(Weapon::Missile), 10);
    let growing = Readout {
        pickup: Some(Weapon::Missile),
        pickup_fade: fade,
        ..readout(98.0)
    };
    step(&mut fade, None, 1);
    let after_use = Readout {
        pickup: None,
        pickup_fade: fade,
        ..readout(98.0)
    };
    with_hd(oag_hd::hud::ART, |cx| {
        let word = |readout: &Readout| colour(cx, readout, "PickupBackground").unwrap();
        let look = growing.pickup_fade.look(growing.pickup).unwrap();
        assert!(look.backdrop > 1.2, "mid-overshoot {}", look.backdrop);
        assert_eq!(word(&growing), argb(scale_word(0x80BE_1E69, look.backdrop)));
        assert_ne!(word(&growing), argb(0x80BE_1E69));
        assert_ne!(
            word(&after_use),
            argb(0x5aff_ffff),
            "fading out, not the idle dim"
        );
    });
}

/// Pulse's and Pure's lap counters close up on lap one (each title's `Hud_UpdateLapCounter`, `-17`):
/// the separator and the total move, the digit does not, later laps and HD (no such rule) draw as
/// authored.
#[test]
fn psp_lap_counters_close_up_on_lap_one_only() {
    let at = |lap, laps| Readout {
        lap,
        laps,
        ..readout(100.0)
    };
    for (title, art) in [("pulse", oag_pulse::hud::ART), ("pure", oag_pure::hud::ART)] {
        with_hd(art, |cx| {
            for name in ["LapOf", "Lap Outof"] {
                let dx = |lap, laps| crate::draw::lap_one_dx(cx, &at(lap, laps), name);
                assert_eq!(dx(1, 3), -17.0, "{title} {name}");
                assert_eq!(dx(2, 3), 0.0, "{title} {name}");
                assert_eq!(dx(1, 0), 0.0, "{title} {name}");
            }
            assert_eq!(
                crate::draw::lap_one_dx(cx, &at(1, 3), "Lap"),
                0.0,
                "{title}"
            );
        });
    }
    with_hd(oag_hd::hud::ART, |cx| {
        assert_eq!(crate::draw::lap_one_dx(cx, &at(1, 3), "LapOf"), 0.0);
    });
}

/// `Detonator_HUD.xml`'s `DamageBar` is the white stripes at another rectangle of its
/// background's sheet, and it is still the fill: drawn over the score box in the readout's white
/// and cropped from the top.
#[test]
fn a_fill_at_another_rectangle_of_the_background_s_sheet_is_drawn() {
    let layout = Layout::from_xml(&format!(
        r#"<Screen><Image name="DamageBarBg">
             <Values x="0" y="0" width="564" height="236" U="0" V="0" TxtrWidth="564"
                     TxtrHeight="236" Src="{SRC}"/>
             <Image name="DamageBar">
               <Values x="5" y="20" width="553" height="200" U="0" V="790" TxtrWidth="553"
                       TxtrHeight="200" color="0xFFFFFFFF" Src="{SRC}"/>
             </Image>
           </Image></Screen>"#
    ));
    let r = Readout {
        mode: oag_race::Mode::Detonator,
        ..readout(50.0)
    };
    let draw = with_layout(&layout, oag_hd::hud::ART, |cx| {
        let bg = cx.layout.sprite("DamageBarBg").expect("authored");
        shield_fill(cx, &r, bg)
    });
    let Some(Draw::Sprite { uv, color, .. }) = draw else {
        panic!("the stripes draw: {draw:?}");
    };
    assert_eq!(uv, [0.0, 890.0, 553.0, 100.0]);
    assert_eq!(color, [1.0; 4]);
}

/// [`layout`] with the countdown widget `HUD_damage_indicator.xml`'s Fury copy authors
/// between the background and the fill, a little larger than both.
fn countdown_layout() -> Layout {
    let mut layout = layout();
    let backing = Layout::from_xml(&format!(
        r#"<Screen><Image name="DamageBarShieldBg">
             <Values x="0" y="0" width="260" height="200" U="770" V="186"
                     TxtrWidth="254" TxtrHeight="194" color="0xff00ff00" Src="{SRC}"/>
           </Image></Screen>"#
    ));
    layout.sprites.extend(backing.sprites);
    layout
}

/// The Shield running with its post-hit flash on an "off" phase, so a blink
/// would show.
fn shielded() -> Readout {
    Readout {
        shield_seconds: 1.967,
        shield_flashing_whole: true,
        shield_blink_phase_whole: 0.133,
        ..readout(100.0)
    }
}

/// While a Shield runs the number is its remaining seconds to one decimal, in
/// the countdown's green, and it does not blink.
#[test]
fn a_running_shield_writes_its_seconds_over_the_percent() {
    with_layout(&countdown_layout(), oag_hd::hud::ART, |cx| {
        let r = shielded();
        assert_eq!(
            countdown_text(cx, &r, "ShieldBarText").as_deref(),
            Some("2.0")
        );
        assert_eq!(colour(cx, &r, "ShieldBarText"), Some(argb(0xFF30_FF30)));
        let spent = Readout {
            shield_seconds: 0.0,
            ..r
        };
        assert_eq!(countdown_text(cx, &spent, "ShieldBarText"), None);
        assert_eq!(colour(cx, &spent, "ShieldBarText"), Some(argb(0x0016_64FF)));
    });
}

/// The widget is drawn over the background and under the fill, only while a
/// Shield runs.
#[test]
fn a_running_shield_puts_its_widget_behind_the_fill() {
    let layout = countdown_layout();
    let widths = |r: &Readout| -> Vec<f32> {
        with_layout(&layout, oag_hd::hud::ART, |cx| {
            draw_list(cx, r)
                .sprites
                .iter()
                .filter_map(|draw| match draw {
                    Draw::Sprite { rect, uv, .. } if uv[0] == 770.0 => Some(rect[2]),
                    _ => None,
                })
                .collect()
        })
    };
    assert_eq!(widths(&shielded()), vec![254.0, 260.0, 254.0]);
    assert_eq!(widths(&readout(100.0)), vec![254.0, 254.0]);
}

/// A layout without the widget (the PSN download's) keeps the percentage.
#[test]
fn a_layout_without_the_countdown_widget_keeps_the_percent() {
    with_hd(oag_hd::hud::ART, |cx| {
        let r = shielded();
        assert_eq!(countdown_text(cx, &r, "ShieldBarText"), None);
        assert_eq!(colour(cx, &r, "ShieldBarText"), Some(argb(0x0016_64FF)));
        let bg = cx.layout.sprite("DamageBarBg").expect("authored");
        assert_eq!(countdown_backing(cx, &r, bg), None);
    });
}

/// A layout shaped like `HUD_proximity.xml`'s: the arc and its marker 100 below it.
fn proximity_layout() -> Layout {
    Layout::from_xml(&format!(
        r#"<Screen>
             <Image name="ProximityBar">
               <Values x="627" y="849" width="666" height="114" U="358" V="524"
                       TxtrWidth="666" TxtrHeight="114" Src="{SRC}"/>
             </Image>
             <Image name="ProxBackground">
               <Values x="907" y="972" width="107" height="69" U="415" V="54"
                       TxtrWidth="107" TxtrHeight="69" Src="{SRC}"/>
             </Image>
           </Screen>"#
    ))
}

fn proximity_rects(art: &oag_title::HudArt) -> Vec<[f32; 4]> {
    with_layout(&proximity_layout(), art, |cx| {
        proximity_sprites(cx)
            .iter()
            .map(|draw| match draw {
                Draw::Sprite { rect, .. } => *rect,
                other => panic!("unexpected draw {other:?}"),
            })
            .collect()
    })
}

/// The idle indicator: the arc where it is authored, the marker 20 above its own place.
#[test]
fn the_idle_proximity_arc_is_drawn_with_its_marker_raised() {
    assert_eq!(
        proximity_rects(oag_hd::hud::ART),
        [[627.0, 849.0, 666.0, 114.0], [907.0, 952.0, 107.0, 69.0]]
    );
}

/// And the whole HUD's draw list carries them: neither widget is on the always-on list.
#[test]
fn the_draw_list_carries_the_idle_proximity_arc() {
    let rects: Vec<[f32; 4]> = with_layout(&proximity_layout(), oag_hd::hud::ART, |cx| {
        draw_list(cx, &Readout::blank())
            .sprites
            .iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    });
    assert_eq!(
        rects,
        [[627.0, 849.0, 666.0, 114.0], [907.0, 952.0, 107.0, 69.0]]
    );
}

/// A title with no runtime rules draws no indicator of its own.
#[test]
fn a_title_without_the_rule_draws_no_proximity_arc() {
    let art = oag_title::HudArt {
        runtime: None,
        ..*oag_hd::hud::ART
    };
    assert!(proximity_rects(&art).is_empty());
}
