use super::*;

const DT: f32 = 1.0 / 60.0;

/// The PPSSPP capture's case: a craft-hit Rocket 51 units from the eye,
/// inside kind 0's full-strength radius. Green over red is `1 - t`.
#[test]
fn a_close_rocket_hit_fades_yellow_to_red_over_half_a_second() {
    let mut flash = ScreenFlash::default();
    flash.start(ROCKET_CRAFT_HIT, Vec3::new(51.0, 0.0, 0.0));
    let mut drawn = Vec::new();
    for _ in 0..40 {
        flash.advance(DT, Vec3::ZERO);
        drawn.push(flash.colour());
    }
    // The first frame drawn is one `dt` in, not at `t = 0`.
    let first = drawn[0].expect("a flash draws on its first frame");
    assert_eq!(first, [1.0, 246.0 / 255.0, 0.0, 147.0 / 255.0]);
    let t = 12.0 * DT / 0.5;
    let twelfth = drawn[11].unwrap();
    assert!((twelfth[1] - (1.0 - t)).abs() < 2.0 / 255.0, "{twelfth:?}");
    assert!(
        (twelfth[3] - 0.6 * (1.0 - t)).abs() < 2.0 / 255.0,
        "{twelfth:?}"
    );
    // 0.5 s is 30 frames; by the 31st it has reached `t = 1` and stopped.
    assert!(drawn[28].is_some());
    assert!(drawn[30..].iter().all(Option::is_none));
}

/// `ScreenFlash_DistanceFalloff`: full inside `near`, none past `far`, and
/// no falloff at all where `far` is zero.
#[test]
fn the_falloff_is_linear_between_near_and_far() {
    let at = Vec3::ZERO;
    let eye = |d: f32| Vec3::new(d, 0.0, 0.0);
    assert_eq!(ROCKET_CRAFT_HIT.falloff(at, eye(50.0)), 1.0);
    assert!((ROCKET_CRAFT_HIT.falloff(at, eye(137.5)) - 0.5).abs() < 1e-6);
    assert_eq!(ROCKET_CRAFT_HIT.falloff(at, eye(250.0)), 0.0);
    let flat = Kind { far: 0.0, ..QUAKE };
    assert_eq!(flat.falloff(at, eye(1.0e4)), 1.0);
}

/// A weaker flash does not cut a stronger one short; a stronger one does.
#[test]
fn only_a_stronger_flash_replaces_the_running_one() {
    let mut flash = ScreenFlash::default();
    flash.start(ROCKET_CRAFT_HIT, Vec3::ZERO);
    flash.advance(DT, Vec3::ZERO);
    // A Quake 290 units away: alpha 0.5 * (1 - 290/300), far weaker.
    flash.start(QUAKE, Vec3::new(290.0, 0.0, 0.0));
    flash.advance(DT, Vec3::ZERO);
    let colour = flash.colour().unwrap();
    assert!(
        colour[1] > 0.9,
        "the Rocket's yellow was replaced: {colour:?}"
    );
    // The Quake at the eye, alpha 0.5 * |(1, 0.2, 0)| against the Rocket's
    // decayed 0.58 * |(1, 0.93, 0)|: still weaker.
    flash.start(QUAKE, Vec3::ZERO);
    flash.advance(DT, Vec3::ZERO);
    assert!(flash.colour().unwrap()[1] > 0.85);
    // Once the Rocket has faded, the Quake takes over.
    for _ in 0..30 {
        flash.advance(DT, Vec3::ZERO);
    }
    flash.start(QUAKE, Vec3::ZERO);
    flash.advance(DT, Vec3::ZERO);
    let quake = flash.colour().unwrap();
    assert!((quake[1] - 0.2).abs() < 2.0 / 255.0, "{quake:?}");
}
