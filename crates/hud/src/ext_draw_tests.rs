//! What a simulation mod's `hud-*` outputs do to the draw list: the added share of a bar,
//! the shake, and the anchors of text and icons. The layout is a small Pulse-shaped one.

use super::ext_draws::{Anchor, BarExtra, Element, Item, Outputs, What};
use super::*;

const RED: [f32; 4] = [1.0, 0.1, 0.1, 1.0];

fn layout() -> Layout {
    Layout::from_xml(
        r#"<Screen>
             <Image name="SpeedBar">
               <Values x="10" y="20" width="200" height="10" U="0" V="20"
                       TxtrWidth="200" TxtrHeight="10" Color="0xFF7DEFC0"
                       Src="Data\HUD\Textures\PulseHUD.mip"/>
             </Image>
             <Image name="ShieldBar">
               <Values x="10" y="40" width="100" height="10" U="0" V="40"
                       TxtrWidth="100" TxtrHeight="10" Color="0xFF0DDFDD"
                       Src="Data\HUD\Textures\PulseHUD.mip"/>
             </Image>
             <Image name="TimeDiffIcon">
               <Values x="0" y="0" width="14" height="12" U="0" V="60"
                       TxtrWidth="28" TxtrHeight="23"
                       Src="Data\HUD\Textures\PulseHUD.mip"/>
             </Image>
           </Screen>"#,
    )
}

fn sheet() -> crate::sprite::Sheet {
    crate::sprite::Sheet::placed_at(&[(
        r"Data\HUD\Textures\PulseHUD.mip",
        crate::sprite::Placed {
            x: 0,
            y: 0,
            width: 256,
            height: 256,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        },
    )])
}

fn frame_of(readout: &Readout) -> Frame {
    let layout = layout();
    let strings = oag_ui::language::StringTable::default();
    let sheet = sheet();
    let cx = crate::tests::context(&layout, &strings);
    let cx = Context {
        sheet: &sheet,
        ..cx
    };
    draw_list(&cx, readout)
}

fn rects(frame: &Frame) -> Vec<[f32; 4]> {
    frame
        .sprites
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } | Draw::GradientSprite { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}

fn readout(speed_kmh: f32, ext: Outputs) -> Readout {
    Readout {
        speed_kmh,
        shield: 80.0,
        shield_max: 100.0,
        ext,
        ..Readout::blank()
    }
}

#[test]
fn outputs_that_ask_for_nothing_leave_the_draw_list_byte_identical() {
    let quiet = Outputs {
        tick: 1234,
        slot: 3,
        ..Outputs::default()
    };
    assert!(quiet.is_quiet());
    assert_eq!(
        frame_of(&readout(400.0, quiet)),
        frame_of(&readout(400.0, Outputs::default()))
    );
}

#[test]
fn a_bars_added_share_is_the_rest_of_the_fill_and_blends_into_the_colour() {
    let plain = frame_of(&readout(400.0, Outputs::default()));
    let extra = 0.9;
    let mut ext = Outputs::default();
    ext.bar_extra[Element::Speed as usize] = Some(BarExtra { extra, color: RED });
    let boosted = frame_of(&readout(400.0, ext));
    let (plain, boosted) = (rects(&plain), rects(&boosted));
    assert_eq!(
        boosted.len(),
        plain.len() + 1,
        "one more draw: the added share"
    );
    // The speed bar is the first sprite.
    let fill = plain[0];
    let (base, added) = (boosted[0], boosted[1]);
    assert_eq!(base[0], fill[0], "the base starts where the fill did");
    assert!(
        (base[2] - fill[2] / (1.0 + extra)).abs() <= 1.0,
        "the base is 1/(1+extra) of the fill: {} of {}",
        base[2],
        fill[2]
    );
    assert!(
        (added[0] - (base[0] + base[2])).abs() < 1e-3,
        "the share follows the base"
    );
    assert!(
        (base[2] + added[2] - fill[2]).abs() <= 1.0,
        "together they reach the fill the value had"
    );
    let share = added[2] / (base[2] + added[2]);
    assert!(
        (share - extra / (1.0 + extra)).abs() < 0.02,
        "about 47 %: {share}"
    );
    let blended = frame_of(&readout(
        400.0,
        Outputs {
            bar_extra: {
                let mut a = Outputs::default().bar_extra;
                a[Element::Speed as usize] = Some(BarExtra { extra, color: RED });
                a
            },
            ..Outputs::default()
        },
    ));
    let Draw::GradientSprite { left, right, .. } = blended.sprites[1].clone() else {
        panic!("the added share is a gradient: {:?}", blended.sprites[1]);
    };
    assert_eq!(right, RED);
    assert_eq!(left, argb_to_rgba(0xFF7D_EFC0), "from the bar's own colour");
}

#[test]
fn a_shaking_bar_moves_by_the_law_and_a_still_one_not_at_all() {
    let plain = rects(&frame_of(&readout(400.0, Outputs::default())));
    let mut still = Outputs::default();
    still.shake[Element::Energy as usize] = 0.0;
    assert_eq!(rects(&frame_of(&readout(400.0, still))), plain);
    for slot in 0..4_u8 {
        for tick in 0..300_u64 {
            let mut ext = Outputs {
                tick,
                slot,
                ..Outputs::default()
            };
            ext.shake[Element::Energy as usize] = 2.0;
            let [dx, dy] = ext.shake_offset(Element::Energy);
            assert!(
                dx != 0.0 && dy != 0.0,
                "tick {tick} slot {slot} holds still"
            );
            assert!(dx.abs() <= 2.0 && dy.abs() <= 2.0 && dx.abs() >= 1.0 && dy.abs() >= 1.0);
            let shaken = rects(&frame_of(&readout(400.0, ext)));
            // The shield bar is the second fill; the speed bar does not move.
            assert_eq!(shaken[0], plain[0]);
            assert_eq!(shaken[1][0], plain[1][0] + dx);
            assert_eq!(shaken[1][1], plain[1][1] + dy);
        }
    }
    let mut ext = Outputs::default();
    ext.shake[Element::Energy as usize] = 2.0;
    let seen: std::collections::HashSet<_> = (0..64_u64)
        .map(|tick| {
            let ext = Outputs {
                tick,
                ..ext.clone()
            };
            ext.shake_offset(Element::Energy).map(f32::to_bits)
        })
        .collect();
    assert!(seen.len() > 32, "it jitters rather than sits on one offset");
}

#[test]
fn text_and_icons_land_at_their_anchor_inside_the_grid() {
    let layout = layout();
    let strings = oag_ui::language::StringTable::default();
    let sheet = sheet();
    let cx = Context {
        sheet: &sheet,
        ..crate::tests::context(&layout, &strings)
    };
    let item = |anchor, what| Item {
        anchor,
        x: -8.0,
        y: -6.0,
        scale: 1.0,
        color: [1.0; 4],
        what,
    };
    let ext = Outputs {
        items: vec![
            item(Anchor::BottomRight, What::Text("1:02.0".into())),
            item(Anchor::Center, What::Icon("TimeDiffIcon".into())),
            item(Anchor::TopLeft, What::Icon("NoSuchSprite".into())),
        ],
        ..Outputs::default()
    };
    let mut frame = Frame::default();
    ext_draws::append(&mut frame, &cx, &ext, (480.0, 272.0));
    let texts: Vec<_> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text {
                x, y, align, text, ..
            } if text == "1:02.0" => Some((*x, *y, *align)),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts.last(),
        Some(&(472.0, 272.0 - 6.0 - 25.0, oag_ui::frontend::Align::Right)),
        "the right edge at 480 - 8, the line's top one line above 272 - 6"
    );
    assert_eq!(
        frame.sprites.len(),
        1,
        "an icon the layout lacks draws nothing"
    );
    let Draw::Sprite { rect, .. } = frame.sprites[0] else {
        panic!("a sprite");
    };
    assert_eq!(rect, [240.0 - 8.0 - 7.0, 136.0 - 6.0 - 6.0, 14.0, 12.0]);
}

#[test]
fn thrust_is_the_thrust_bar_where_a_title_has_one_and_nothing_where_it_has_not() {
    let thrust = |ext_on: bool, xml: &str| {
        let layout = Layout::from_xml(xml);
        let strings = oag_ui::language::StringTable::default();
        let sheet = sheet();
        let cx = Context {
            sheet: &sheet,
            art: oag_hd::hud::ART,
            ..crate::tests::context(&layout, &strings)
        };
        let mut ext = Outputs::default();
        if ext_on {
            ext.bar_extra[Element::Thrust as usize] = Some(BarExtra {
                extra: 0.9,
                color: RED,
            });
        }
        let readout = Readout {
            thrust_percent: 100.0,
            ext,
            ..Readout::blank()
        };
        draw_list(&cx, &readout).sprites
    };
    let bar = |name: &str| {
        format!(
            r#"<Screen><Image name="{name}"><Values x="10" y="20" width="200" height="10" U="0" V="20"
               TxtrWidth="200" TxtrHeight="10" Color="0xFF7DEFC0" Src="Data\HUD\Textures\PulseHUD.mip"/></Image></Screen>"#
        )
    };
    let with = thrust(true, &bar("ThrustBar"));
    assert_eq!(with.len(), 2, "base and added share");
    assert!(matches!(
        &with[1],
        Draw::GradientSprite { right, .. } if *right == RED
    ));
    let pulse_like = thrust(true, &bar("SpeedBar"));
    assert_eq!(
        pulse_like,
        thrust(false, &bar("SpeedBar")),
        "no thrust bar: nothing"
    );
}

#[test]
fn a_mods_own_icon_is_drawn_from_its_place_in_the_sheet_at_its_own_size() {
    let layout = layout();
    let strings = oag_ui::language::StringTable::default();
    let icon = crate::sprite::DecodedImage {
        src: "mod:test/badge".into(),
        width: 8,
        height: 4,
        rgba: vec![255; 8 * 4 * 4],
        quad_extent: None,
        quad_centre: None,
        blend: None,
    };
    let sheet = crate::sprite::Sheet::build_with(&[], vec![icon], &mut Vec::new());
    let placed = sheet.get("mod:test/badge").expect("in the sheet");
    let cx = Context {
        sheet: &sheet,
        ..crate::tests::context(&layout, &strings)
    };
    let item = |src: &str| Item {
        anchor: Anchor::TopRight,
        x: -2.0,
        y: 3.0,
        scale: 2.0,
        color: [1.0, 1.0, 0.5, 1.0],
        what: What::ModIcon(src.into()),
    };
    let ext = Outputs {
        items: vec![item("mod:test/badge"), item("mod:test/missing")],
        ..Outputs::default()
    };
    let mut frame = Frame::default();
    ext_draws::append(&mut frame, &cx, &ext, (480.0, 272.0));
    assert_eq!(frame.sprites.len(), 1, "a missing icon draws nothing");
    assert_eq!(
        frame.sprites[0],
        Draw::Sprite {
            rect: [480.0 - 2.0 - 16.0, 3.0, 16.0, 8.0],
            uv: [placed.x as f32, placed.y as f32, 8.0, 4.0],
            color: [1.0, 1.0, 0.5, 1.0],
        }
    );
}
