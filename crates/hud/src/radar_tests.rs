//! The radar dot law (`docs/ghidra/functions/ps4-omega-eu/hud-radar.md`, "The dots") on known
//! relative positions.

use super::radar::{Blip, DOTS, Kind, RANGE, place};
use super::*;

const HALF: [f32; 2] = [98.625, 27.0];
const FORWARD: [f32; 3] = [0.0, 0.0, -1.0];
const RIGHT: [f32; 3] = [1.0, 0.0, 0.0];

fn blip(offset: [f32; 3]) -> Blip {
    Blip::from_offset(offset, FORWARD, RIGHT).expect("a direction")
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn a_craft_dead_behind_hangs_from_the_bowls_centre_to_its_depth() {
    // Half the range behind: t = 0.5, f = -1, y = f * t * 27 * -2.
    let (offset, alpha) = place(Kind::Half, blip([0.0, 0.0, 100.0]), HALF).unwrap();
    assert!(close(offset[0], 0.0), "{offset:?}");
    assert!(close(offset[1], 27.0), "{offset:?}");
    // 255 - 127.5 * 0.5 = 191.25, truncated to 191.
    assert!(close(alpha, 191.0 / 255.0), "{alpha}");
}

#[test]
fn the_rim_is_the_bowls_width_and_depth_and_the_alpha_half() {
    let (offset, alpha) = place(Kind::Half, blip([0.0, 0.0, RANGE]), HALF).unwrap();
    assert!(close(offset[1], 54.0), "{offset:?}");
    assert!(close(alpha, 127.0 / 255.0), "{alpha}");
}

#[test]
fn right_of_the_player_lands_left_of_the_origin() {
    // Behind and to the right at 45 degrees, half the range out.
    let (offset, _) = place(Kind::Half, blip([70.710_68, 0.0, 70.710_68]), HALF).unwrap();
    assert!(offset[0] < 0.0, "{offset:?}");
    let t = 100.0 / RANGE;
    assert!(
        close(offset[0], -(t * std::f32::consts::FRAC_1_SQRT_2 * HALF[0])),
        "{offset:?}"
    );
    let left = place(Kind::Half, blip([-70.710_68, 0.0, 70.710_68]), HALF)
        .unwrap()
        .0;
    assert!(close(left[0], -offset[0]), "{left:?}");
}

#[test]
fn past_the_range_the_half_radar_hides_the_dot_and_the_full_keeps_its_floor() {
    let far = blip([0.0, 0.0, RANGE + 50.0]);
    let (offset, alpha) = place(Kind::Half, far, HALF).unwrap();
    assert!(close(offset[1], 54.0) && alpha == 0.0, "{offset:?} {alpha}");
    let ahead = blip([0.0, 0.0, -(RANGE + 50.0)]);
    let (_, floor) = place(Kind::Full, ahead, [97.875, 52.5]).unwrap();
    assert!(close(floor, 51.0 / 255.0), "{floor}");
}

#[test]
fn a_craft_ahead_is_only_on_the_full_radar_and_above_the_centre() {
    let ahead = blip([0.0, 0.0, -100.0]);
    assert_eq!(place(Kind::Half, ahead, HALF), None);
    let (offset, alpha) = place(Kind::Full, ahead, [97.875, 52.5]).unwrap();
    // f = 1, t = 0.5, y = 1 * 0.5 * 52.5 * -0.5.
    assert!(close(offset[1], -13.125), "{offset:?}");
    assert!(close(alpha, 127.0 / 255.0), "{alpha}");
    let (behind, _) = place(Kind::Full, blip([0.0, 0.0, 100.0]), [97.875, 52.5]).unwrap();
    assert!(close(behind[1], 26.25), "{behind:?}");
}

#[test]
fn a_craft_on_the_player_has_no_direction() {
    assert_eq!(Blip::from_offset([0.0; 3], FORWARD, RIGHT), None);
}

fn half_radar_layout() -> Layout {
    let sprite = |name: &str, rect: [f32; 4], centred: bool| Sprite {
        name: name.to_string(),
        rect,
        uv: [0.0, 0.0, 8.0, 8.0],
        color: [1.0, 1.0, 1.0, 1.0],
        src: "hud".to_string(),
        rotation: 0.0,
        gradient: None,
        centred,
    };
    let mut layout = Layout::default();
    layout
        .sprites
        .push(sprite("Radar", [381.75, 439.0, 197.25, 54.0], false));
    for i in 0..DOTS {
        layout.sprites.push(sprite(
            &format!("RadarDot{i}"),
            [469.125, 434.875, 21.75, 8.25],
            true,
        ));
    }
    layout
}

fn art_with_radar() -> oag_title::HudArt {
    oag_title::HudArt {
        always_on: &["Radar"],
        ..*oag_pulse::hud::ART
    }
}

#[test]
fn the_dot_of_a_slot_moves_and_fades_and_the_others_stay_hidden() {
    let layout = half_radar_layout();
    let mut readout = Readout::blank();
    readout.radar_blips[3] = Some(blip([0.0, 0.0, 100.0]));
    let dots = super::radar::dot_sprites(&layout, &art_with_radar(), &readout);
    assert_eq!(dots.len(), 1);
    assert_eq!(dots[0].name, "RadarDot3");
    assert!(close(dots[0].rect[0], 469.125) && close(dots[0].rect[1], 434.875 + 27.0));
    assert!(close(dots[0].color[3], 191.0 / 255.0));
    let none = super::radar::dot_sprites(&layout, &art_with_radar(), &Readout::blank());
    assert!(none.is_empty());
}

#[test]
fn a_title_that_does_not_draw_the_radar_draws_no_dots() {
    let mut readout = Readout::blank();
    readout.radar_blips[0] = Some(blip([0.0, 0.0, 100.0]));
    let art = oag_title::HudArt {
        always_on: &[],
        ..*oag_pulse::hud::ART
    };
    assert!(super::radar::dot_sprites(&half_radar_layout(), &art, &readout).is_empty());
}
