//! The boost's copy count, stamp and shifts against what a PPSSPP frame showed: 2 copies at
//! alpha `0x7f` and `0x30`, a stamp of `51` at 2 and `76` at 3, directions `(0.618, 0.786)` for the
//! speed bar at `(306, 220)`.

use super::*;
use crate::draw::Context;
use crate::{Layout, draw_list};

const SRC: &str = r"Data\HUD\Textures\hud.mip";

fn layout() -> Layout {
    let image = |name: &str, x: u32, y: u32, w: u32, h: u32| {
        format!(
            r#"<Image name="{name}"><Values x="{x}" y="{y}" width="{w}" height="{h}"
                 U="100" V="10" TxtrWidth="{w}" TxtrHeight="{h}" Src="{SRC}"/></Image>"#
        )
    };
    let mut xml = String::from("<Screen>");
    xml.push_str(&image("SpeedBarBg", 306, 220, 168, 26));
    xml.push_str(&image("SpeedBar", 306, 220, 168, 26));
    xml.push_str(
        r#"<Text name="SpeedBarText"><Values font="HUD" x="300" y="210" string="0" align="right"/></Text>"#,
    );
    xml.push_str("</Screen>");
    Layout::from_xml(&xml)
}

fn with_art<R>(art: &oag_title::HudArt, f: impl FnOnce(&Context<'_>) -> R) -> R {
    let layout = layout();
    let strings = oag_ui::language::StringTable::default();
    let sheet = crate::sprite::Sheet::placed_at(&[(
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
    )]);
    let cx = Context {
        default_border: layout.default_border(),
        tag_label: None,
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

fn boosting(phase: f32) -> Readout {
    Readout {
        speed_kmh: 500.0,
        boost_phase: phase,
        ..Readout::blank()
    }
}

#[test]
fn the_count_is_a_triangle_of_the_timer_and_one_when_no_boost_runs() {
    let art = oag_pulse::hud::ART;
    let n = |phase: f32| copies(art, &boosting(phase));
    assert_eq!(n(0.0), 1, "no boost: one draw");
    assert_eq!(n(0.5), 10, "the middle of the timer");
    assert_eq!(n(0.25), 5);
    assert_eq!(n(0.75), 5, "the triangle is symmetric");
    assert_eq!(n(0.79), 4, "`trunc(10 * 0.42)`");
    assert_eq!(n(1.0), 1, "a count of 0 draws once");
    assert_eq!(n(0.1), 2, "the live capture's n = 2");
    // `0.366 / 0.8` and `0.332 / 0.8` read `n = 9` and `8` live.
    assert_eq!(n(0.366 / 0.8), 9);
    assert_eq!(n(0.3324 / 0.8), 8);
    assert_eq!(
        copies(oag_hd::hud::ART, &boosting(0.5)),
        1,
        "a title without the law"
    );
}

#[test]
fn the_stamp_is_the_count_over_ten_below_two_copies_nothing() {
    let art = oag_pulse::hud::ART;
    let s = |phase: f32| stamp(art, &boosting(phase));
    assert_eq!(s(0.5), 255);
    assert_eq!(
        s(0.15),
        76,
        "n = 3: `trunc(0.3 * 255)`, the byte read from EDRAM"
    );
    assert_eq!(s(0.1), 51, "n = 2, the other byte read from EDRAM");
    assert_eq!(s(0.05), 0, "n = 1");
    assert_eq!(s(0.0), 0);
}

#[test]
fn a_copy_is_the_truncated_screen_offset_in_whole_pixels() {
    let base = [1808.0, 1912.0];
    let dir = [0.618, 0.786];
    let at = |i| shift(dir, i, base);
    assert_eq!(
        [at(0), at(1), at(2), at(3)],
        [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [2.0, 3.0]]
    );
    // A direction that points up and left moves less: truncation is not a round.
    let back = [-0.432, -0.902];
    assert_eq!(shift(back, 1, base), [0.0, 0.0]);
    assert_eq!(shift(back, 3, base), [-1.0, -2.0]);
}

#[test]
fn a_boost_draws_the_fill_n_times_at_alpha_over_n_and_stamps_the_mask() {
    with_art(oag_pulse::hud::ART, |cx| {
        let rest = draw_list(cx, &boosting(0.0));
        assert!(rest.glow.is_empty(), "no boost: nothing stamped");
        let fills = |frame: &crate::Frame| {
            frame
                .sprites
                .iter()
                .filter(|d| matches!(d, Draw::Sprite { rect, .. } if rect[1] >= 220.0 && rect[3] == 26.0))
                .count()
        };
        let once = fills(&rest);
        assert!(once >= 2, "the bar's background and fill draw at rest");
        let peak = draw_list(cx, &boosting(0.5));
        assert_eq!(fills(&peak), once * 10, "ten copies of each at the peak");
        assert_eq!(peak.glow.len(), 10);
        assert!(peak.glow.iter().all(|g| g.value == 255));
        let alpha = peak.sprites.iter().find_map(|d| match d {
            Draw::Sprite { color, .. } => Some(color[3]),
            _ => None,
        });
        assert_eq!(alpha, Some(25.0 / 255.0), "`trunc(255 / 10)`");
        let two = draw_list(cx, &boosting(0.1));
        assert_eq!(two.glow.len(), 2);
        assert_eq!(two.glow[0].value, 51);
        assert_eq!(two.glow[0].rect[0], 306.0 + 0.0);
        assert_eq!(
            two.glow[1].rect[0],
            306.0 + 1.0,
            "copy 1 is one pixel along (0.618, 0.786)"
        );
    });
}

#[test]
fn a_frame_with_no_boost_is_the_plain_frame() {
    with_art(oag_pulse::hud::ART, |cx| {
        assert_eq!(
            draw_list(cx, &boosting(0.0)),
            draw_list(
                cx,
                &Readout {
                    boost_phase: 0.0,
                    ..boosting(0.0)
                }
            )
        );
    });
    with_art(oag_hd::hud::ART, |cx| {
        let boosted = draw_list(cx, &boosting(0.5));
        assert_eq!(
            boosted,
            draw_list(cx, &boosting(0.0)),
            "HD draws once whatever the timer"
        );
    });
}

fn copies_of(frame: &crate::Frame, width: f32) -> Vec<[f32; 2]> {
    frame
        .sprites
        .iter()
        .filter_map(|d| match d {
            Draw::Sprite { rect, .. } if rect[2] == width => Some([rect[0], rect[1]]),
            _ => None,
        })
        .collect()
}

/// The pickup box is `Centred` at the authored `(240, 35)`: its copies step `(0, -1)` each, where
/// the drawn top-left `(207, 5)` would step `(-0.245, -0.969)` (`docs/ghidra/functions/
/// psp-pulse-usa/hud-boost-smear.md`, "The pickup icon at a boost").
#[test]
fn a_centred_widgets_copies_step_away_from_its_authored_point() {
    let layout = Layout::from_xml(
        r#"<Screen>
        <Image name="PickupBackground"><Values x="240" y="35" Centred="true" width="66" height="60"
          U="3" V="111" TxtrWidth="66" TxtrHeight="60" Src="p.mip"/></Image>
        <Image name="Off"><Values x="300" y="40" Centred="true" width="100" height="80"
          U="0" V="0" TxtrWidth="100" TxtrHeight="80" Src="p.mip"/></Image>
        <Image name="Plain"><Values x="306" y="220" width="30" height="26"
          U="0" V="0" TxtrWidth="30" TxtrHeight="26" Src="p.mip"/></Image>
        </Screen>"#,
    );
    let sprite = |rect: [f32; 4]| Draw::Sprite {
        rect,
        uv: [0.0; 4],
        color: [1.0; 4],
    };
    let mut frame = crate::Frame {
        sprites: vec![
            sprite([207.0, 5.0, 66.0, 60.0]),
            sprite([250.0, 0.0, 100.0, 80.0]),
            sprite([306.0, 220.0, 30.0, 26.0]),
        ],
        ..crate::Frame::default()
    };
    smear(
        &mut frame,
        oag_pulse::hud::ART,
        &layout,
        &boosting(0.5),
        &[],
        &[],
    );
    let pickup = copies_of(&frame, 66.0);
    assert_eq!(pickup.len(), 10);
    for (i, at) in pickup.iter().enumerate() {
        assert_eq!(
            *at,
            [207.0, 5.0 - i as f32],
            "copy {i}: exactly (0, -1) a step"
        );
    }
    // `(300, 40) - (240, 136)` normalised, not the box's top-left `(250, 0)`.
    let dir = direction([300.0, 40.0], [240.0, 136.0]);
    let off = copies_of(&frame, 100.0);
    for (i, at) in off.iter().enumerate() {
        let by = shift(dir, i as u32, [1808.0, 1912.0]);
        assert_eq!(*at, [250.0 + by[0], 0.0 + by[1]], "copy {i}");
    }
    let wrong = direction([250.0, 0.0], [240.0, 136.0]);
    assert_ne!(
        shift(dir, 9, [1808.0, 1912.0]),
        shift(wrong, 9, [1808.0, 1912.0]),
        "the two readings part by the last copy"
    );
    // A plain widget keeps its top-left as the anchor.
    let plain = copies_of(&frame, 30.0);
    let dir = direction([306.0, 220.0], [240.0, 136.0]);
    assert_eq!(plain[9], {
        let by = shift(dir, 9, [1808.0, 1912.0]);
        [306.0 + by[0], 220.0 + by[1]]
    });
}
