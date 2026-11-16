use super::*;

#[test]
fn alpha_is_full_to_twenty_units_half_at_forty_and_gone_at_sixty() {
    assert_eq!(alpha(1.0, 0.0, true), 1.0);
    assert_eq!(alpha(1.0, 20.0, true), 1.0);
    assert!((alpha(1.0, 40.0, true) - 0.5).abs() < 1.0e-6);
    assert_eq!(alpha(1.0, 60.0, true), 0.0);
    assert_eq!(alpha(1.0, 500.0, true), 0.0);
}

#[test]
fn alpha_is_the_lesser_of_the_fade_and_the_range() {
    assert!((alpha(0.25, 10.0, true) - 0.25).abs() < 1.0e-6);
    assert!((alpha(0.25, 40.0, true) - 0.25).abs() < 1.0e-6);
    assert!((alpha(0.75, 40.0, true) - 0.5).abs() < 1.0e-6);
    assert_eq!(alpha(0.0, 10.0, true), 0.0);
}

#[test]
fn the_option_off_forces_alpha_to_zero_at_any_distance() {
    for distance in [0.0, 5.0, 20.0, 40.0, 59.0] {
        assert_eq!(alpha(1.0, distance, false), 0.0, "{distance}");
    }
}

#[test]
fn the_fade_rises_at_one_point_two_a_second_and_stops_at_one() {
    let mut fade = 0.0;
    for _ in 0..30 {
        fade = step_fade(fade, 1.0 / 60.0);
    }
    assert!((fade - 0.6).abs() < 1.0e-4, "half a second: {fade}");
    for _ in 0..120 {
        fade = step_fade(fade, 1.0 / 60.0);
    }
    assert_eq!(fade, 1.0);
}

#[test]
fn the_anchor_climbs_two_units_as_the_fade_completes() {
    assert_eq!(raise(0.0), 0.0);
    assert_eq!(raise(0.5), 1.0);
    assert_eq!(raise(1.0), 2.0);
}
