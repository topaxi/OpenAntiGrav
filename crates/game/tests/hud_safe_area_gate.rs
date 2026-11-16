//! `[graphics] hud_safe_area`: the `Safe Area Setting` every title carries (HD's measured, the others'
//! shared and chosen), the scale a value gives, and that the HUD's draws move in toward the middle
//! of the screen by it.

use oag_game::hud_overlay::safe_area_frame;
use oag_hud::Frame;
use oag_ui::frontend::Draw;

fn sprite(rect: [f32; 4]) -> Draw {
    Draw::Sprite {
        rect,
        uv: [0.0; 4],
        color: [1.0; 4],
    }
}

#[test]
fn hd_carries_the_discs_slider_and_every_other_title_the_shared_one_at_a_hundred() {
    let hd = oag_hd::hud::ART.safe_area.expect("HD's Safe Area Setting");
    assert_eq!(hd.origin, oag_title::Origin::Measured);
    assert_eq!(
        (
            hd.value.default_percent,
            hd.value.min_percent,
            hd.value.max_percent,
            hd.value.step_percent
        ),
        (90, 80, 100, 2),
        "additional_definition.xml: default=\"90%\" minSlide=\"80\" maxSlide=\"100\" incSlide=\"2\""
    );
    for (name, art) in [
        ("pulse", oag_pulse::hud::ART),
        ("pure", oag_pure::hud::ART),
        ("2048", oag_2048::hud::ART),
        ("omega", oag_omega::hud::ART),
    ] {
        let area = art.safe_area.expect("the shared slider");
        assert_eq!(
            area.origin,
            oag_title::Origin::Chosen,
            "{name}: authors none"
        );
        assert_eq!(area.value, oag_title::hud::SafeArea::SHARED, "{name}");
        assert_eq!(
            art.safe_area_scale(None),
            1.0,
            "{name}: drawn 1:1 until the player moves it"
        );
        assert_eq!(art.safe_area_scale(Some(90)), 0.9, "{name}");
    }
}

#[test]
fn a_value_is_held_to_the_slider() {
    let area = oag_hd::hud::ART.safe_area.expect("HD").value;
    assert_eq!(area.scale(None), 0.90);
    assert_eq!(area.scale(Some(96)), 0.96);
    assert_eq!(area.scale(Some(97)), 0.96, "snapped down to a step of two");
    assert_eq!(area.scale(Some(100)), 1.0);
    assert_eq!(area.scale(Some(10)), 0.80, "held to the slider's minimum");
    assert_eq!(area.scale(Some(200)), 1.0, "held to the slider's maximum");
}

#[test]
fn the_hud_moves_in_toward_the_centre_by_the_scale() {
    let size = (1920.0, 1080.0);
    let mut frame = Frame::default();
    frame.sprites.push(sprite([0.0, 0.0, 100.0, 50.0]));
    frame.sprites.push(sprite([910.0, 515.0, 100.0, 50.0]));
    let moved = safe_area_frame(frame.clone(), 0.9, size);
    let rects: Vec<[f32; 4]> = moved
        .sprites
        .iter()
        .map(|draw| match draw {
            Draw::Sprite { rect, .. } => *rect,
            other => panic!("not a sprite: {other:?}"),
        })
        .collect();
    for (got, want) in rects[0].iter().zip([96.0, 54.0, 90.0, 45.0]) {
        assert!(
            (got - want).abs() < 1e-3,
            "a corner pulled in and shrunk: {:?}",
            rects[0]
        );
    }
    let centre_x = rects[1][0] + rects[1][2] * 0.5;
    assert!(
        (centre_x - 960.0).abs() < 1e-3,
        "the centre stays: {centre_x}"
    );
    assert_eq!(
        safe_area_frame(frame.clone(), 1.0, size).sprites.len(),
        frame.sprites.len()
    );
}
