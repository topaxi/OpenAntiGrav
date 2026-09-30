use super::*;

const DT: f32 = 1.0 / 60.0;

/// The PPSSPP capture's case: a craft-hit Rocket 51 units from the eye,
/// inside kind 0's full-strength radius. Green over red is `1 - t`.
#[test]
fn a_close_rocket_hit_fades_yellow_to_red_over_half_a_second() {
    let mut flash = ScreenFlash::default();
    flash.start(BLAST, Vec3::new(51.0, 0.0, 0.0));
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
    assert_eq!(BLAST.falloff(at, eye(50.0)), 1.0);
    assert!((BLAST.falloff(at, eye(137.5)) - 0.5).abs() < 1e-6);
    assert_eq!(BLAST.falloff(at, eye(250.0)), 0.0);
    let flat = Kind { far: 0.0, ..QUAKE };
    assert_eq!(flat.falloff(at, eye(1.0e4)), 1.0);
}

/// A weaker flash does not cut a stronger one short; a stronger one does.
#[test]
fn only_a_stronger_flash_replaces_the_running_one() {
    let mut flash = ScreenFlash::default();
    flash.start(BLAST, Vec3::ZERO);
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

/// `ScreenFlash_Start`'s table, kind by kind: the doc page's rows, each
/// reachable by the `a1` its callers pass.
#[test]
fn the_kind_table_is_indexed_by_the_start_argument() {
    assert_eq!(KINDS[0], BLAST);
    assert_eq!(KINDS[1], PLASMA);
    assert_eq!(KINDS[3], BOMB);
    assert_eq!(KINDS[4], QUAKE);
    assert_eq!(KINDS[6], RESET);
    assert_eq!(KINDS[7], PLAYER_DESTROYED);
    assert_eq!(KINDS[8], MINE);
    for kind in KINDS {
        assert_eq!(kind.keys[0].0, 0.0, "every ramp starts at time 0");
        assert_eq!(
            kind.keys.last().unwrap().0,
            1.0,
            "and ends at time 1, where the flash stops"
        );
    }
    // Only kind 11 is alpha-over, and only it has no caller besides 5.
    assert_eq!(KINDS.iter().filter(|k| !k.additive).count(), 1);
    assert!(!KINDS[11].additive);
}

/// Kind 7 authors three keys: white, yellow at a tenth of its two seconds,
/// then red to nothing. The lerp walks the segments.
#[test]
fn the_players_destruction_ramps_through_three_keys() {
    let at = |t: f32| PLAYER_DESTROYED.colour_at(t);
    assert_eq!(at(0.0), [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(at(0.1), [1.0, 1.0, 0.0, 0.8]);
    let mid = at(0.55);
    assert!((mid[1] - 0.5).abs() < 1e-5, "{mid:?}");
    assert!((mid[3] - 0.4).abs() < 1e-5, "{mid:?}");
    // A quarter of the way to the yellow key.
    let early = at(0.025);
    assert!((early[2] - 0.75).abs() < 1e-5, "{early:?}");
    assert_eq!(at(1.0), [1.0, 0.0, 0.0, 0.0]);

    let mut flash = ScreenFlash::default();
    flash.start(PLAYER_DESTROYED, Vec3::ZERO);
    let mut frames = 0;
    loop {
        flash.advance(DT, Vec3::ZERO);
        if flash.colour().is_none() {
            break;
        }
        frames += 1;
    }
    // Two seconds at 60 Hz, less the one frame the clock reaches 1.0.
    assert!((118..=120).contains(&frames), "{frames}");
}

/// A flash with no falloff (`far == 0`) draws the same wherever the eye is.
#[test]
fn a_reset_flash_does_not_fall_off_with_distance() {
    let mut near = ScreenFlash::default();
    let mut far = ScreenFlash::default();
    near.start(RESET, Vec3::ZERO);
    far.start(RESET, Vec3::ZERO);
    near.advance(DT, Vec3::ZERO);
    far.advance(DT, Vec3::new(5000.0, 0.0, 0.0));
    assert_eq!(near.colour(), far.colour());
    // One frame in: alpha `1 - 1/60`, truncated to a byte.
    assert_eq!(near.colour().unwrap()[3], 250.0 / 255.0);
}
