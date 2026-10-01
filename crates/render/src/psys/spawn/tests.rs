use super::*;

#[test]
fn a_line_spreads_along_its_frame_x_and_nowhere_else() {
    let line = Spawn::Line {
        extent: 50.0,
        depth: 0.0,
    };
    let mut rng = Rng::new(7);
    let across = Vec3::new(0.0, 0.0, 1.0);
    let mut widest = 0.0f32;
    for _ in 0..500 {
        // The Quake's co-factor on a 70-unit road: `70 / 50`.
        let offset = line.offset(1.4, Vec3::Y, across, Vec3::Y, &mut rng);
        assert_eq!(offset.x, 0.0);
        assert_eq!(offset.y, 0.0);
        assert!(offset.z.abs() <= 70.0 + 1e-3);
        widest = widest.max(offset.z.abs());
    }
    assert!(widest > 60.0, "uniform over +/-70, got at most {widest}");
}

#[test]
fn a_point_draws_nothing_from_the_generator() {
    let mut a = Rng::new(3);
    let mut b = Rng::new(3);
    let offset = Spawn::Point.offset(2.0, Vec3::X, Vec3::X, Vec3::Y, &mut a);
    assert_eq!(offset, Vec3::ZERO);
    assert_eq!(a.next_u32(), b.next_u32());
}

#[test]
fn an_exact_sphere_sits_on_its_scaled_radius() {
    let sphere = Spawn::Sphere {
        extent: 2.0,
        spread: 0.5,
        mode: 0,
    };
    let mut rng = Rng::new(1);
    let offset = sphere.offset(3.0, Vec3::Y, Vec3::X, Vec3::Y, &mut rng);
    assert!((offset - Vec3::new(0.0, 6.0, 0.0)).length() < 1e-5);
}

#[test]
fn frame_x_is_made_perpendicular_to_up() {
    let x = frame_x(Vec3::new(1.0, 1.0, 0.0), Vec3::Y);
    assert!((x - Vec3::X).length() < 1e-6);
    let fallback = frame_x(Vec3::Y, Vec3::Y);
    assert!(fallback.dot(Vec3::Y).abs() < 1e-6);
    assert!((fallback.length() - 1.0).abs() < 1e-6);
}

#[test]
fn an_exact_ring_sits_on_its_radius_in_the_frame_xz_plane() {
    let ring = Spawn::Ring {
        extent: 12.94,
        spread: 0.0,
        mode: 0,
    };
    let mut rng = Rng::new(5);
    let mut seen_x = [false; 2];
    for _ in 0..200 {
        let offset = ring.offset(1.0, Vec3::Y, Vec3::X, Vec3::Y, &mut rng);
        assert!(offset.y.abs() < 1e-5, "{offset}");
        assert!((offset.length() - 12.94).abs() < 1e-3, "{offset}");
        seen_x[usize::from(offset.x > 0.0)] = true;
    }
    assert_eq!(seen_x, [true, true], "all the way round, not one side");
}

#[test]
fn a_disc_fills_its_circle_and_stays_inside_it() {
    let disc = Spawn::Ring {
        extent: 5.12,
        spread: 0.0,
        mode: 2,
    };
    let mut rng = Rng::new(9);
    let mut inner = 0;
    for _ in 0..500 {
        let offset = disc.offset(1.0, Vec3::Y, Vec3::X, Vec3::Y, &mut rng);
        assert_eq!(offset.y, 0.0);
        assert!(offset.length() <= 5.12 + 1e-4);
        inner += usize::from(offset.length() < 2.56);
    }
    // A uniform disc has a quarter of its area inside half its radius.
    assert!((90..170).contains(&inner), "{inner} of 500");
}

#[test]
fn a_ring_under_an_aimed_velocity_flies_outward_along_its_own_heading() {
    let ring = Spawn::Ring {
        extent: 8.0,
        spread: 0.0,
        mode: 0,
    };
    let aimed = crate::psys::Direction::Aimed {
        elevation: 0.0,
        azimuth: 0.0,
        jitter: 0.0,
    };
    let mut rng = Rng::new(2);
    for _ in 0..50 {
        let (direction, offset) = place(ring, aimed, 1.0, Vec3::X, Vec3::Y, &mut rng);
        assert!(
            (direction - offset / 8.0).length() < 1e-4,
            "{direction} against {offset}"
        );
    }
    // A raised aim keeps the heading and tilts it up by the elevation.
    let raised = crate::psys::Direction::Aimed {
        elevation: 1.0,
        azimuth: 0.0,
        jitter: 0.0,
    };
    let (direction, offset) = place(ring, raised, 1.0, Vec3::X, Vec3::Y, &mut rng);
    assert!((direction.y - 1.0f32.sin()).abs() < 1e-5);
    let flat = Vec3::new(direction.x, 0.0, direction.z).normalize();
    assert!((flat - offset.normalize()).length() < 1e-4);
}
