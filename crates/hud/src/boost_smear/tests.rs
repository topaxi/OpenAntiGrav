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
