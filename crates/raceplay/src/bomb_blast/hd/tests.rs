//! The HD blast's arithmetic against numbers taken from running the
//! executable's own instructions (`0x00151538`, `0x001503d8`, `0x001512f8`) in
//! a scratch PowerPC interpreter, camera `(5, 3, -20)` looking down `-Z` (back
//! axis `+Z`), a blast at the origin with up `+Y`. A change to a constant in
//! [`super`] fails here by name.

use super::*;

const EYE: Vec3 = Vec3::new(5.0, 3.0, -20.0);
const BACK: Vec3 = Vec3::Z;

fn blast_at(ticks: usize) -> HdBlast {
    let mut blast = HdBlast::new(Vec3::ZERO, Vec3::Y);
    for _ in 0..ticks {
        blast.step(1.0 / 60.0);
    }
    blast
}

fn rows(piece: Piece) -> [[f32; 3]; 3] {
    [
        piece.matrix.x_axis,
        piece.matrix.y_axis,
        piece.matrix.z_axis,
    ]
    .map(|c| c.truncate().to_array())
}

fn assert_rows(got: [[f32; 3]; 3], want: [[f32; 3]; 3], tolerance: f32, what: &str) {
    for (r, (g, w)) in got.iter().zip(&want).enumerate() {
        for k in 0..3 {
            assert!(
                (g[k] - w[k]).abs() < tolerance,
                "{what} row {r}: {got:?} against {want:?}"
            );
        }
    }
}

/// Half a second in: the fireball is the camera frame tilted by `-30 deg`
/// about the unnormalised `up x d`, scaled `curve(0.5) + cur`; the bloom disc
/// is the frame facing the viewer.
#[test]
fn the_fireball_and_the_bloom_disc_match_the_executables_own_matrices() {
    let pieces = blast_at(30).pieces(EYE, BACK);
    let fireball = pieces.fireball.expect("shown");
    assert_rows(
        rows(fireball),
        [
            [10.1434, 1.2292, 0.3164],
            [-1.2292, 8.8779, 4.9167],
            [0.3164, -4.9167, 8.957],
        ],
        0.02,
        "fireball",
    );
    assert_rows(
        rows(pieces.bloom.expect("shown")),
        [
            [-9.9449, 0.0, -2.4862],
            [-0.358, 10.1442, 1.4321],
            [2.4603, 1.4762, -9.8413],
        ],
        0.02,
        "bloom",
    );
    // The white core is the same matrix, 0.99 times, at the constant colour.
    let core = pieces.core.expect("shown");
    assert!((core.matrix.x_axis.x - fireball.matrix.x_axis.x * 0.99).abs() < 1e-3);
    assert_eq!(core.colour, 0.9);
    assert_eq!(core.matrix.w_axis, Vec3::ZERO.extend(1.0));
}

/// The clocks and the fireball's `ColourAnim`, which the shaders read:
/// `1` until 0.75 s then `1 - (age - 0.75)^2`; the bloom disc's two-piece
/// ramp; the ring's `age / 0.6` held at 1; and `|x|` then `x^(1/4)` for the
/// colour, `x = 2 (age - 0.1) / 1.4 - 1`.
#[test]
fn the_clocks_and_the_colour_follow_the_update() {
    let early = blast_at(18).pieces(EYE, BACK);
    assert!(
        (early.fireball.unwrap().clock - 1.0).abs() < 1e-6,
        "age 0.3"
    );
    assert!((early.bloom.unwrap().clock - 0.1875).abs() < 1e-3);
    assert!((early.ring.unwrap().clock - 0.5).abs() < 1e-3);

    let late = blast_at(72).pieces(EYE, BACK); // age 1.2
    assert!((late.fireball.unwrap().clock - 0.7975).abs() < 1e-3);
    assert!((late.bloom.unwrap().clock - 0.5816).abs() < 1e-3);
    assert_eq!(late.ring.unwrap().clock, 1.0);
    // x = 2 * 1.1 / 1.4 - 1 = 0.5714, ^(1/4) = 0.8695.
    assert!((late.fireball.unwrap().colour - 0.8695).abs() < 2e-3);

    let dip = blast_at(48).pieces(EYE, BACK); // age 0.8: x = -0.0
    assert!(dip.fireball.unwrap().colour < 2e-2);
}

/// The first ring grows `0.1 -> 53.33` over 0.6 s, uniformly, then holds.
#[test]
fn the_first_ring_grows_to_53_and_a_third_in_six_tenths_of_a_second() {
    let scale = |ticks: usize| {
        let ring = blast_at(ticks).pieces(EYE, BACK).ring.unwrap();
        ring.matrix.x_axis.truncate().length()
    };
    assert!((scale(1) - 1.579).abs() < 5e-3);
    assert!((scale(30) - 44.458).abs() < 5e-3);
    assert!((scale(36) - 53.33).abs() < 5e-3);
    assert!((scale(80) - 53.33).abs() < 5e-3);
}

/// From 1.5 s the first four hide; from 1.4 s the seven ripple rings pass in
/// windows, tallest in the middle, and they sit along the up axis.
#[test]
fn from_one_and_a_half_seconds_only_the_ripple_rings_are_left() {
    let before = blast_at(80).pieces(EYE, BACK); // 1.33 s
    assert!(before.fireball.is_some() && before.ripples.iter().all(Option::is_none));

    let overlap = blast_at(86).pieces(EYE, BACK); // 1.43 s: first four still shown
    assert!(overlap.fireball.is_some());
    assert!(overlap.ripples[0].is_some() && overlap.ripples[3].is_none());

    let after = blast_at(108).pieces(EYE, BACK); // 1.8 s
    assert!(after.fireball.is_none() && after.core.is_none());
    assert!(after.bloom.is_none() && after.ring.is_none());
    let size = |k: usize| {
        let m = after.ripples[k].expect("in its window").matrix;
        (
            m.x_axis.truncate().length(),
            m.y_axis.truncate().length(),
            m.w_axis.truncate().y,
        )
    };
    // (sideways, tall, height): the executable's own, 1.8 s in.
    for (k, want) in [
        (0, (1.521, 4.095, 5.33)),
        (1, (3.865, 7.375, 4.0)),
        (2, (6.192, 5.655, 2.0)),
        (3, (8.642, 4.207, 0.0)),
    ] {
        let got = size(k);
        assert!(
            (got.0 - want.0).abs() < 5e-3
                && (got.1 - want.1).abs() < 5e-3
                && (got.2 - want.2).abs() < 1e-3,
            "ring {k}: {got:?} against {want:?}"
        );
    }
    // Mirrored about the middle.
    assert!((size(0).0 - size(6).0).abs() < 1e-4 && size(0).2 == -size(6).2);

    let done = blast_at(150).pieces(EYE, BACK); // 2.5 s
    assert!(done.ripples.iter().all(Option::is_none));
}

/// A ring begins inside out: the tall axis is negative until a little before
/// 0.4 of its window (`1 - 25 (1 - u)^4 + 2` crosses zero at `u = 0.41`).
#[test]
fn a_ripple_ring_starts_inside_out() {
    let early = blast_at(86).pieces(EYE, BACK).ripples[0].expect("window opened");
    assert!(
        early.matrix.y_axis.truncate().dot(Vec3::Y) < 0.0,
        "age 1.43"
    );
    let late = blast_at(108).pieces(EYE, BACK).ripples[0].expect("window");
    assert!(late.matrix.y_axis.truncate().dot(Vec3::Y) > 0.0, "age 1.8");
}

/// `WO_BOMB_RAYS` once, the first tick past half a second; the object retires
/// the first tick past three.
#[test]
fn the_rays_come_once_at_half_a_second_and_the_blast_retires_at_three() {
    let mut blast = HdBlast::new(Vec3::ZERO, Vec3::Y);
    let mut rays_at = Vec::new();
    let mut retired_at = None;
    for tick in 1..=200usize {
        let step = blast.step(1.0 / 60.0);
        if step.rays {
            rays_at.push(tick);
        }
        if step.retire {
            retired_at = Some(tick);
            break;
        }
    }
    // Tick 30 is 0.5 in exact arithmetic and 0.50000006 in the single-precision
    // sum the executable (and this) accumulates, so it is the first past 0.5.
    assert_eq!(rays_at, vec![30]);
    assert_eq!(retired_at, Some(181), "age 3.0167");
}
