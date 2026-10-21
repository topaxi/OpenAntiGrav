use super::super::draw::draw_list;
use super::super::tests::{SAMPLE, context, strings};
use super::*;
use crate::Layout;

const LEAN: f32 = 1.5;

fn swaying() -> oag_title::HudArt {
    oag_title::HudArt {
        sway: Some(oag_title::pre_race::Sourced::measured(
            oag_title::hud::HudSway { x_per_lean: -15 },
        )),
        ..*oag_pulse::hud::ART
    }
}

/// `draw` with its x moved by `dx`, for the draws a HUD frame holds.
fn moved_x(draw: &Draw, dx: f32) -> Draw {
    let mut draw = draw.clone();
    match &mut draw {
        Draw::Sprite { rect, .. } | Draw::Fill { rect, .. } => rect[0] += dx,
        Draw::Text { x, .. } => *x += dx,
        other => panic!("a draw this test does not move: {other:?}"),
    }
    draw
}

fn lists(frame: &Frame) -> [&Vec<Draw>; 5] {
    [
        &frame.sprites,
        &frame.hud_text,
        &frame.small_text,
        &frame.default_text,
        &frame.skin_text,
    ]
}

#[test]
fn a_measured_sway_slides_every_draw_by_minus_fifteen_per_unit_of_lean() {
    let layout = Layout::from_xml(SAMPLE);
    let strings = strings();
    let art = swaying();
    let mut cx = context(&layout, &strings);
    cx.art = &art;
    let still = Readout {
        pickup: Some(oag_tables::weapons::Weapon::Rocket),
        ..Readout::blank()
    };
    let leaning = Readout {
        steer_lean_follower: LEAN,
        ..still.clone()
    };
    let before = draw_list(&cx, &still);
    let after = draw_list(&cx, &leaning);

    let count: usize = lists(&before).iter().map(|list| list.len()).sum();
    assert!(
        count >= 4,
        "the fixture draws bars, text and a pickup: {before:?}"
    );
    for (was, now) in lists(&before).into_iter().zip(lists(&after)) {
        let expected: Vec<Draw> = was.iter().map(|draw| moved_x(draw, -15.0 * LEAN)).collect();
        assert_eq!(&expected, now);
    }
}

#[test]
fn a_title_with_no_sway_leaves_the_hud_where_it_is() {
    let layout = Layout::from_xml(SAMPLE);
    let strings = strings();
    let cx = context(&layout, &strings);
    assert!(cx.art.sway.is_none(), "Pulse's HUD is fixed on the screen");
    let still = Readout::blank();
    let leaning = Readout {
        steer_lean_follower: LEAN,
        ..Readout::blank()
    };
    assert_eq!(draw_list(&cx, &still), draw_list(&cx, &leaning));
}

#[test]
fn the_y_term_is_drawn_as_zero() {
    let readout = Readout {
        steer_lean_follower: -0.7,
        ..Readout::blank()
    };
    let [x, y] = offset(&swaying(), &readout);
    assert_eq!(x, 10.5);
    assert_eq!(y, 0.0);
}

#[test]
fn the_titles_that_slide_are_hd_and_the_two_that_inherit_it() {
    let law = |art: &oag_title::HudArt| art.sway.map(|sway| (sway.value.x_per_lean, sway.origin));
    use oag_title::effects::Origin;
    assert_eq!(law(oag_hd::hud::ART), Some((-15, Origin::Measured)));
    assert_eq!(
        law(oag_omega::hud::ART),
        Some((-15, Origin::InheritedFrom("hd")))
    );
    assert_eq!(
        law(oag_2048::hud::ART),
        Some((-15, Origin::InheritedFrom("hd")))
    );
    assert_eq!(law(oag_pulse::hud::ART), None);
    assert_eq!(law(oag_pure::hud::ART), None);
}
