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

/// A four-probe rig of round numbers, not a title's.
const RIG: HoverRig = HoverRig {
    probes: Sourced::measured(&[
        [-1.0, -1.0, -2.0],
        [-1.0, -1.0, 2.0],
        [1.0, -1.0, -2.0],
        [1.0, -1.0, 2.0],
    ]),
    spring_share: Sourced::measured(0.15),
    cast_every_probe: Sourced::measured(true),
    along_hit_normal: Sourced::measured(true),
    quarter_sum_normal: Sourced::measured(true),
};

#[test]
fn a_title_without_a_rig_flies_pulses_two_point_law() {
    assert_eq!(physics_rig(None), Rig::TWO_POINT);
}

#[test]
fn a_titles_rig_reaches_the_physics_field_for_field() {
    let rig = physics_rig(Some(&RIG));
    assert_eq!(
        rig.offsets(),
        &[
            Vec3::new(-1.0, -1.0, -2.0),
            Vec3::new(-1.0, -1.0, 2.0),
            Vec3::new(1.0, -1.0, -2.0),
            Vec3::new(1.0, -1.0, 2.0),
        ]
    );
    assert_eq!(rig.spring_share, 0.15);
    assert!(
        !rig.derive_rear,
        "cast_every_probe turns the derived rear hit off"
    );
    assert!(rig.along_normal);
    assert_eq!(rig.normal_mean, NormalMean::QuarterSum);
}

/// HD's craft laws reach the tensor (`(17.333, 24, 17.333)`, `Ship_Construct`'s mass `1.0`); a
/// title with none keeps Pulse's tensor to the bit.
#[test]
fn a_titles_craft_laws_build_its_inertia_and_none_keeps_pulses() {
    let pulse = craft_inertia(None);
    assert_eq!(pulse, oag_physics::forces::ship_inertia());
    let hd = craft_inertia(Some(&oag_hd::race::CRAFT_LAWS));
    assert!((hd.y - 24.0).abs() < 1.0e-3, "I_yy {}", hd.y);
    assert!((hd.x - 17.333).abs() < 1.0e-3, "I_xx {}", hd.x);
}
