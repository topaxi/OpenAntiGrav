use super::*;

/// Vineta K's live frame (`docs/rendering/lens-flare.md`, "Live check"): the sun at NDC
/// `(-0.4515, 0.4343)`, so `S = (-0.8026, 0.4343)`.
const NDC: [f32; 2] = [-0.4515, 0.4343];

fn live_rings() -> [Ring; RINGS] {
    rings(
        [NDC[0] * ASPECT, NDC[1]],
        0.4736,
        [0.7107; 3],
        Random {
            jitter: 1.0,
            ring: 0.7,
        },
    )
}

#[test]
fn the_ten_centres_lie_on_the_line_through_the_screen_centre_at_the_measured_ratios() {
    let ratios = [
        1.0, 0.8, 0.7, 0.55, 0.325, -0.0125, -0.51875, -1.278, -2.417, -4.126,
    ];
    for (k, (ring, ratio)) in live_rings().iter().zip(ratios).enumerate() {
        for axis in 0..2 {
            let want = NDC[axis] * ratio;
            // The ratios are read to three decimals off the vertex buffer.
            assert!(
                (ring.centre[axis] - want).abs() < 0.002 * NDC[axis].abs() + 1e-4,
                "ring {k} axis {axis}: {} vs {want}",
                ring.centre[axis]
            );
        }
    }
}

#[test]
fn ring_zero_and_ring_one_have_the_live_half_sizes() {
    let rings = live_rings();
    // Half-size in the draw's own units is in NDC y: 1.9312 and 0.2 * 0.5476 * 1.0.
    assert!((rings[0].half[1] - 1.9312).abs() < 1e-3, "{:?}", rings[0]);
    assert!(
        (rings[1].half[1] - 0.2 * 0.5476).abs() < 1e-3,
        "{:?}",
        rings[1]
    );
    // Ring 2 is the cycle's 2.0 at 1.45.
    assert!((rings[2].half[1] - 0.2 * 0.5476 * 1.45 * 2.0).abs() < 1e-3);
    // Squares on a 16:9 screen: the x half is the y half over 16/9.
    assert!((rings[1].half[0] * ASPECT - rings[1].half[1]).abs() < 1e-6);
}

#[test]
fn the_cells_are_the_measured_order() {
    let cells: Vec<u8> = live_rings().iter().map(|r| r.cell).collect();
    assert_eq!(cells, [0, 1, 1, 1, 2, 2, 3, 3, 3, 3]);
}

#[test]
fn alpha_is_the_intensity_for_ring_zero_and_scaled_by_r_for_the_rest() {
    let rings = live_rings();
    assert!((rings[0].rgba[3] - 0.4736).abs() < 1.0 / 255.0);
    assert!((rings[1].rgba[3] - 0.4736 * 0.7).abs() < 1.0 / 255.0);
}

#[test]
fn the_rings_vanish_as_the_sun_reaches_the_centre_but_ring_zero_does_not() {
    let near = rings([0.0, 0.0], 0.9, [1.0; 3], Random::FLAGGED);
    assert!(near[0].rgba[3] > 0.8);
    assert!(near[1..].iter().all(|r| r.rgba[3] == 0.0));
}

#[test]
fn the_hue_drifts_by_the_measured_amount() {
    // Modesto Heights `(1, 0.6, 0)`: G = 0.6 - 0.005 k (k + 1), read live.
    let live_g = [0.60, 0.59, 0.57, 0.54, 0.50, 0.45, 0.39, 0.32, 0.24, 0.15];
    for (k, (c, want)) in ring_colours([1.0, 0.6, 0.0]).iter().zip(live_g).enumerate() {
        assert!((c[1] - want).abs() < 0.006, "ring {k}: {c:?} vs {want}");
    }
    // A grey stays grey.
    assert!(
        ring_colours([0.7107; 3])
            .iter()
            .all(|c| c.iter().all(|v| (v - 0.7107).abs() < 1e-6))
    );
}

fn camera(look: Vec3) -> (Mat4, Mat4) {
    let view = Mat4::look_to_rh(Vec3::ZERO, look, Vec3::Y);
    let projection = Mat4::perspective_rh(60f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0);
    (view, projection)
}

#[test]
fn a_sun_in_front_of_the_camera_places_rings_and_one_behind_it_places_none() {
    let (view, projection) = camera(Vec3::NEG_Z);
    let viewport = [0.0, 0.0, 480.0, 272.0];
    let ahead = Placement::new(
        [0.0, 0.0, -1.0],
        [1.0; 3],
        view,
        projection,
        viewport,
        Random::FLAGGED,
    )
    .expect("dead ahead is visible");
    assert!(ahead.sun_pixel[0] > 239.0 && ahead.sun_pixel[0] < 241.0);
    assert!((ahead.pixel_scale - 1.0).abs() < 1e-6);
    let behind = Placement::new(
        [0.0, 0.0, 1.0],
        [1.0; 3],
        view,
        projection,
        viewport,
        Random::FLAGGED,
    );
    assert!(behind.is_none());
    // Outside the one-radian cone.
    let wide = Vec3::new(1.0, 0.0, -0.3).normalize();
    assert!(
        Placement::new(
            wide.into(),
            [1.0; 3],
            view,
            projection,
            viewport,
            Random::FLAGGED
        )
        .is_none()
    );
}

#[test]
fn a_sun_to_the_left_projects_left_and_up_stays_up() {
    let (view, projection) = camera(Vec3::NEG_Z);
    let sun = Vec3::new(-0.4, 0.2, -1.0).normalize();
    let p = Placement::new(
        sun.into(),
        [1.0; 3],
        view,
        projection,
        [0.0, 0.0, 480.0, 272.0],
        Random::FLAGGED,
    )
    .unwrap();
    assert!(p.rings[0].centre[0] < 0.0 && p.rings[0].centre[1] > 0.0);
    // Pixel y runs down: the sun above the centre is above row 136.
    assert!(p.sun_pixel[0] < 240.0 && p.sun_pixel[1] < 136.0);
    // Ring 5 sits near the centre whatever S is.
    assert!(p.rings[5].centre[0].abs() < 0.0125 * p.rings[0].centre[0].abs() + 1e-6);
}

#[test]
fn a_sun_beyond_the_sampling_margin_draws_nothing() {
    let (view, projection) = camera(Vec3::NEG_Z);
    // Inside the cone but 55 degrees off axis with a 60 degree vertical field: off the right edge.
    let sun = Vec3::new(0.82, 0.0, -0.57).normalize();
    assert!(
        Placement::new(
            sun.into(),
            [1.0; 3],
            view,
            projection,
            [0.0, 0.0, 480.0, 272.0],
            Random::FLAGGED
        )
        .is_none()
    );
}

#[test]
fn the_cone_fades_linearly() {
    assert!(intensity(1.0).unwrap() > 0.95);
    assert!(intensity(0.55).is_some_and(|i| i < 0.02));
    assert!(intensity(0.54).is_none());
    assert!(intensity(0.5).is_none());
    assert!(intensity(-0.2).is_none());
}

/// The live Vineta K camera (the compare pair's pose): its rows are a little off unit
/// length, and the original dots the raw row, so the cosine is `0.8649` and
/// `1 - acos(c)` the measured `0.4736` (`+0x6c` reads `0.47363`).
#[test]
fn the_live_camera_gives_the_measured_intensity_and_sun_position() {
    let view = Mat4::from_cols(
        Vec4::new(-0.000_859_569, 0.002_799_354_7, 0.995_379_03, 0.0),
        Vec4::new(-6.487_023e-6, 0.999_996_07, 0.096_020_25, 0.0),
        Vec4::new(-0.995_106_3, -8.936_970_5e-6, -0.000_860_430_53, 0.0),
        Vec4::new(-96.187_54, -40.560_88, -53.592_377, 1.0),
    );
    // 60 degrees vertical on 16:9.
    let projection = Mat4::perspective_rh(60f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0);
    let sun = Vec3::new(-51.826, 12.774, 23.323).normalize();
    let placement = Placement::new(
        sun.into(),
        [0.7107; 3],
        view,
        projection,
        [0.0, 0.0, 480.0, 272.0],
        Random::FLAGGED,
    )
    .expect("the sun is in the cone");
    assert!((placement.rings[0].rgba[3] - 0.4736).abs() < 1.5 / 255.0);
    // The sun's pixel: the probe the original read, `(131, 77)`.
    assert!(
        (placement.sun_pixel[0] - 131.5).abs() < 1.0,
        "{:?}",
        placement.sun_pixel
    );
    assert!(
        (placement.sun_pixel[1] - 77.0).abs() < 1.0,
        "{:?}",
        placement.sun_pixel
    );
}
