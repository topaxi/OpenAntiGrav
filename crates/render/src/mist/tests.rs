use super::*;

fn frame(position: Vec3, right: Vec3, up: Vec3, back: Vec3) -> Frame {
    Frame {
        position,
        right,
        up,
        back,
    }
}

/// A camera matrix as the original stores it (axes as columns, `-eye` in the
/// fourth row), as our frame.
fn from_original(m: [f32; 16]) -> Frame {
    frame(
        -Vec3::new(m[12], m[13], m[14]),
        Vec3::new(m[0], m[4], m[8]),
        Vec3::new(m[1], m[5], m[9]),
        Vec3::new(m[2], m[6], m[10]),
    )
}

const FORT_GALE: Config = Config {
    tex_scale: 3.1,
    aspect: 0.5625,
    display_scale: 0.8,
};

#[test]
fn a_left_turn_steps_the_yaw_up_as_read_live() {
    // Fort Gale, Assegai, holding left: the back axis `CameraMotion_Sample`
    // kept (`S+0xd0`) and last frame's (`S+0xdc`), and the yaw step it wrote,
    // `0.019054651`.
    let now = Vec3::new(0.967_018_4, 0.099_970_29, -0.234_267_6);
    let then = Vec3::new(0.971_311, 0.099_924_07, -0.215_801_06);
    let at = Vec3::ZERO;
    let motion = Motion::between(
        &frame(at, Vec3::X, Vec3::Y, then),
        &frame(at, Vec3::X, Vec3::Y, now),
        Vec3::ZERO,
        0.0,
    );
    assert!((motion.dyaw - 0.019_054_651).abs() < 1e-5, "{motion:?}");
    assert!((motion.dpitch - -4.645_437e-5).abs() < 2e-6, "{motion:?}");
    // And holding right, from the same run: `-0.021068335`.
    let now = Vec3::new(-0.593_664_9, 0.105_439_86, -0.797_774_7);
    let then = Vec3::new(-0.610_351_8, 0.105_253_91, -0.785_106_5);
    let motion = Motion::between(
        &frame(at, Vec3::X, Vec3::Y, then),
        &frame(at, Vec3::X, Vec3::Y, now),
        Vec3::ZERO,
        0.0,
    );
    assert!((motion.dyaw - -0.021_068_335).abs() < 1e-5, "{motion:?}");
}

#[test]
fn a_still_camera_moves_by_the_drift_as_read_live() {
    // The start line before the craft moved: the camera matrix and the mist's
    // drift as read, and `CameraMotion_Sample`'s `(dx, dy)` of
    // `(-0.0871, 0.3016)` - `dt` is `1/60`. The craft was already creeping,
    // so `dz` (`0.041`) is not asserted and the others carry a tolerance.
    let camera = from_original([
        0.004_726_154,
        -0.098_501_53,
        0.995_125_65,
        0.0,
        5.687_529e-5,
        0.995_136_8,
        0.098_502_36,
        0.0,
        -0.999_988_85,
        -0.000_408_939_23,
        0.004_708_772,
        0.0,
        -113.620_705,
        -114.651_79,
        -560.407_96,
        22.286_331,
    ]);
    let drift = Vec3::new(1.097_295_8, 18.0, -5.291_108_6);
    let motion = Motion::between(&camera, &camera, drift, 1.0 / 60.0);
    assert!((motion.dy - 0.301_635_66).abs() < 0.01, "{motion:?}");
    assert!((motion.dx - -0.087_082_86).abs() < 0.01, "{motion:?}");
}

#[test]
fn driving_forward_zooms_the_layers_in() {
    let camera = frame(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z);
    // The camera looks down `-back`, so forward is `-Z`.
    let ahead = frame(Vec3::new(0.0, 0.0, -2.0), Vec3::X, Vec3::Y, Vec3::Z);
    let motion = Motion::between(&camera, &ahead, Vec3::ZERO, 1.0 / 60.0);
    assert!((motion.dz - 2.0).abs() < 1e-6, "{motion:?}");
    assert_eq!(
        (motion.dx, motion.dy, motion.dyaw, motion.roll),
        (0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn the_roll_is_signed_by_the_right_axis() {
    let level = frame(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z);
    assert_eq!(camera_roll(&level), 0.0);
    let (sin, cos) = 0.3f32.sin_cos();
    // Rolled about the back axis, the right axis tipping up.
    let rolled = frame(
        Vec3::ZERO,
        Vec3::new(cos, sin, 0.0),
        Vec3::new(-sin, cos, 0.0),
        Vec3::Z,
    );
    assert!((camera_roll(&rolled) - -0.3).abs() < 1e-5);
    let other = frame(
        Vec3::ZERO,
        Vec3::new(cos, -sin, 0.0),
        Vec3::new(sin, cos, 0.0),
        Vec3::Z,
    );
    assert!((camera_roll(&other) - 0.3).abs() < 1e-5);
}

#[test]
fn the_angle_steps_wrap_across_pi() {
    assert!((wrap(PI + 0.1) - (-PI + 0.1)).abs() < 1e-5);
    assert!((wrap(-PI - 0.1) - (PI - 0.1)).abs() < 1e-5);
    assert_eq!(wrap(0.25), 0.25);
}

#[test]
fn the_uv_extents_match_the_live_vertices() {
    // Outpost 7 read live: at `t = 0.684` the `u` extent was `1.060` and the
    // `v` extent `0.596`, `tan(fov / 2) = 0.6249`, no roll.
    let mut rng = Rng::new(1);
    let mut layers = Layers::new(&mut rng);
    let motion = Motion {
        dz: 0.316 / ZOOM_RATE,
        ..Motion::default()
    };
    layers.step(&FORT_GALE, &motion, 0.3, &mut rng);
    assert!((layers.t[0] - 0.684).abs() < 1e-4);
    let quad = layers.quads(FORT_GALE.display_scale * 0.6249)[0];
    let u_extent = quad[2][0] - quad[0][0];
    let v_extent = quad[1][1] - quad[0][1];
    assert!((u_extent - 1.060).abs() < 1e-3, "{u_extent}");
    assert!((v_extent - 0.596).abs() < 1e-3, "{v_extent}");
}

#[test]
fn the_layers_crossfade_half_a_phase_apart() {
    let mut rng = Rng::new(2);
    let mut layers = Layers::new(&mut rng);
    let motion = Motion {
        dz: 0.1 / ZOOM_RATE,
        ..Motion::default()
    };
    layers.step(&FORT_GALE, &motion, 0.3, &mut rng);
    // Layer 0: phase 0.1, t 0.9, fade 0.2. Layer 1: phase 0.6, t 0.4, fade 0.8.
    assert!(
        (layers.alpha[0] - 0.2 * 0.3).abs() < 1e-5,
        "{:?}",
        layers.alpha
    );
    assert!(
        (layers.alpha[1] - 0.8 * 0.3).abs() < 1e-5,
        "{:?}",
        layers.alpha
    );
    // The GE gets bytes: 0.24 * 255 = 61.2, truncated to 61.
    assert_eq!(layers.alphas()[1], 61.0 / 255.0);
}

#[test]
fn a_zoom_that_restarts_rolls_new_offsets() {
    let mut rng = Rng::new(3);
    let mut layers = Layers::new(&mut rng);
    let small = Motion {
        dz: 0.1 / ZOOM_RATE,
        ..Motion::default()
    };
    layers.step(&FORT_GALE, &small, 0.3, &mut rng);
    let before = layers.offset;
    layers.step(&FORT_GALE, &small, 0.3, &mut rng);
    assert_eq!(layers.offset, before, "a small step keeps them");
    // Layer 1 goes from phase 0.7 to 0.1 past the wrap: `t` jumps 0.3 -> 0.9.
    // Layer 0 goes from 0.2 to 0.6, a step of 0.4 in `t`, and keeps its own.
    let wrap = Motion {
        dz: 0.4 / ZOOM_RATE,
        ..Motion::default()
    };
    layers.step(&FORT_GALE, &wrap, 0.3, &mut rng);
    assert_ne!(layers.offset[1], before[1]);
    assert_eq!(layers.offset[0], before[0]);
}

#[test]
fn a_turn_scrolls_the_texture_and_a_roll_turns_it() {
    let mut rng = Rng::new(4);
    let mut still = Layers::new(&mut rng);
    let mut turned = still.clone();
    let mut rolled = still.clone();
    still.step(&FORT_GALE, &Motion::default(), 0.3, &mut rng.clone());
    let turn = Motion {
        dyaw: 0.02,
        ..Motion::default()
    };
    turned.step(&FORT_GALE, &turn, 0.3, &mut rng.clone());
    // Layer 0 at `t = 1`: `u` moves by `dyaw * Aspect * t * TexScale`.
    let du = turned.rect[0].u0 - still.rect[0].u0;
    assert!((du - 0.02 * 0.5625 * 3.1).abs() < 1e-5, "{du}");
    let roll = Motion {
        roll: 0.5,
        ..Motion::default()
    };
    rolled.step(&FORT_GALE, &roll, 0.3, &mut rng.clone());
    assert_eq!(rolled.rect, still.rect);
    assert_ne!(rolled.quads(0.5), still.quads(0.5));
}

#[test]
fn the_first_corner_is_the_bottom_right() {
    let quads = [[[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]; 2];
    let vertices = pipeline::vertices(&quads, [0.25, 0.5]);
    assert_eq!(vertices[0].position, [1.0, -1.0]);
    assert_eq!(vertices[0].texcoord, [0.0, 0.0]);
    assert_eq!(vertices[5].position, [-1.0, 1.0]);
    assert_eq!(vertices[5].texcoord, [1.0, 1.0]);
    assert_eq!(vertices[6].alpha, 0.5);
}
