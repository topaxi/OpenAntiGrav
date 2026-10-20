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
    sum_escapes: Sourced::measured(true),
    probe_escape: Sourced::measured(true),
    four_corner_spring: Sourced::measured(false),
    plain_mean_normal: Sourced::measured(false),
    alignment_pitches: Sourced::measured(false),
    downforce_unscaled: Sourced::measured(false),
    snap: Sourced::measured(false),
    brakes_off: Sourced::measured(false),
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
    assert!(rig.sum_escapes);
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

/// A title's compiled-in pitch carries its grid damping into the physics' laws, and a title
/// with none keeps the handling's damping on the grid too (Pulse's).
#[test]
fn a_titles_code_pitch_carries_its_grid_damping_into_the_laws() {
    let laws = |code_pitch| CraftLaws {
        inertia_mass: Sourced::chosen(0.9),
        steer_ramp_clamped: Sourced::chosen(false),
        air_pitch_bias: Sourced::chosen(false),
        class_gravity_on_flight: Sourced::chosen(false),
        engine_output_double: Sourced::chosen(true),
        drag_ground: Sourced::chosen(-0.005),
        bank_to_yaw: Sourced::chosen(30.0),
        wall_one_sided: Sourced::chosen(false),
        wall_steer_away: Sourced::chosen(false),
        wall_point_rate_body_local: Sourced::chosen(false),
        code_pitch,
        code_sideshift: None,
        code_start_boost: None,
        pitch_gate: Sourced::chosen(true),
        eliminator_wrong_way_kill: Sourced::chosen(true),
        sideshift_grid_gate: Sourced::chosen(true),
        turbo_running_laws: Sourced::chosen(true),
        inertia_rotates: Sourced::chosen(true),
    };
    let pitch = oag_title::craft_laws::CodePitch {
        air: 0.1,
        ground: 0.4,
        damping: 1.5,
        grid_damping: 0.6,
    };
    assert_eq!(
        physics_laws(Some(&laws(Some(Sourced::measured(pitch))))).pitch_damping_grid,
        Some(0.6)
    );
    assert_eq!(physics_laws(Some(&laws(None))).pitch_damping_grid, None);
    assert_eq!(physics_laws(None).pitch_damping_grid, None);
}

/// HD's measured one-sided contact reaches the physics' laws, inherited by Omega and 2048;
/// Pulse (no laws) and Pure keep the two-sided one.
#[test]
fn the_one_sided_wall_contact_follows_the_title() {
    assert!(physics_laws(Some(&oag_hd::race::CRAFT_LAWS)).wall_one_sided);
    assert!(physics_laws(Some(&oag_omega::race::CRAFT_LAWS)).wall_one_sided);
    assert!(physics_laws(Some(&oag_2048::race::CRAFT_LAWS)).wall_one_sided);
    assert!(!physics_laws(Some(&oag_pure::race::CRAFT_LAWS)).wall_one_sided);
    assert!(!physics_laws(None).wall_one_sided);
}

/// HD's measured steer-away torque reaches the physics' laws, inherited by Omega and 2048;
/// Pulse and Pure queue none.
#[test]
fn the_wall_steer_away_torque_follows_the_title() {
    assert!(physics_laws(Some(&oag_hd::race::CRAFT_LAWS)).wall_steer_away);
    assert!(physics_laws(Some(&oag_omega::race::CRAFT_LAWS)).wall_steer_away);
    assert!(physics_laws(Some(&oag_2048::race::CRAFT_LAWS)).wall_steer_away);
    assert!(!physics_laws(Some(&oag_pure::race::CRAFT_LAWS)).wall_steer_away);
    assert!(!physics_laws(None).wall_steer_away);
}

/// HD's measured body-local point rate reaches the physics' laws, inherited by Omega; 2048
/// holds it, and Pulse and Pure keep the textbook one.
#[test]
fn the_wall_point_rate_frame_follows_the_title() {
    assert!(physics_laws(Some(&oag_hd::race::CRAFT_LAWS)).wall_point_rate_body_local);
    assert!(physics_laws(Some(&oag_omega::race::CRAFT_LAWS)).wall_point_rate_body_local);
    // Held on 2048 (`oag_2048::race::CRAFT_LAWS`'s comment).
    assert!(!physics_laws(Some(&oag_2048::race::CRAFT_LAWS)).wall_point_rate_body_local);
    assert!(!physics_laws(Some(&oag_pure::race::CRAFT_LAWS)).wall_point_rate_body_local);
    assert!(!physics_laws(None).wall_point_rate_body_local);
}
