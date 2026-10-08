use super::*;
use oag_race::COUNTDOWN_TICKS;
use oag_title::pre_race::Sourced;

const DT: f32 = 1.0 / 60.0;

const HOVER: LaunchHover = LaunchHover {
    grid_cap: Sourced::chosen(3.0),
    release_rate: Sourced::measured(1.0),
};

#[test]
fn the_clamp_is_flat_on_the_grid() {
    assert_eq!(cap_at(&HOVER, 0, DT), 3.0);
    assert_eq!(cap_at(&HOVER, COUNTDOWN_TICKS - 2, DT), 3.0);
}

#[test]
fn the_clamp_climbs_one_unit_a_second_from_the_tick_the_grid_ends() {
    let first = cap_at(&HOVER, COUNTDOWN_TICKS - 1, DT);
    assert!((first - (3.0 + DT)).abs() < 1e-6, "{first}");
    let second_later = cap_at(&HOVER, COUNTDOWN_TICKS - 1 + 59, DT);
    assert!((second_later - 4.0).abs() < 1e-5, "{second_later}");
}

#[test]
fn the_race_target_takes_over_after_two_and_a_half_seconds() {
    // 5.5 is a Venom's ride height; the clamp passes it 2.5 s after the grid ends.
    let at = |s: f32| cap_at(&HOVER, COUNTDOWN_TICKS - 1 + (s * 60.0) as u64, DT);
    assert!(at(2.4) < 5.5);
    assert!(at(2.6) > 5.5);
}
