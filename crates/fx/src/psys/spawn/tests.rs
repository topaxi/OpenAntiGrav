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
        arc: std::f32::consts::TAU,
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
        arc: std::f32::consts::TAU,
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
        arc: std::f32::consts::TAU,
    };
    let aimed = crate::psys::Direction::Aimed {
        elevation: 0.0,
        azimuth: 0.0,
        jitter: 0.0,
    };
    let mut rng = Rng::new(2);
    for _ in 0..50 {
        let (direction, offset) =
            place(ring, (aimed, None), 1.0, (Vec3::X, Vec3::Y), None, &mut rng);
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
    let (direction, offset) = place(
        ring,
        (raised, None),
        1.0,
        (Vec3::X, Vec3::Y),
        None,
        &mut rng,
    );
    assert!((direction.y - 1.0f32.sin()).abs() < 1e-5);
    let flat = Vec3::new(direction.x, 0.0, direction.z).normalize();
    assert!((flat - offset.normalize()).length() < 1e-4);
}

/// A lens droplet is born in the `XZ` rectangle and aimed from `+Z` by the live azimuth.
#[test]
fn a_rect_spawns_in_the_xz_plane_and_heads_along_the_live_azimuth() {
    let rect = Spawn::Rect {
        extent: 10.0,
        depth: 5.625,
    };
    let aimed = crate::psys::Direction::Aimed {
        elevation: 0.0,
        azimuth: 0.0,
        jitter: 0.0,
    };
    let mut rng = Rng::new(5);
    for _ in 0..50 {
        let (direction, offset) = place(
            rect,
            (aimed, Some(0.5)),
            1.0,
            (Vec3::X, Vec3::Y),
            None,
            &mut rng,
        );
        assert!(offset.x.abs() <= 10.0 && offset.z.abs() <= 5.625 && offset.y == 0.0);
        assert!((direction.x - -0.5f32.sin()).abs() < 1e-5, "{direction:?}");
        assert!((direction.z - 0.5f32.cos()).abs() < 1e-5);
    }
}

/// `ParticleSystem_AimedVelocity` (`0x088fc490`) turns the ring's heading `(x, z)` by
/// `+a`: `(x cos a - z sin a, z cos a + x sin a)`, the frame's `Z` being `X x Y`. A bead
/// born at angle `phi` flies at `phi + a` - the sense the Rect path above already plays.
#[test]
fn a_ring_beads_azimuth_turns_its_heading_the_originals_way() {
    let ring = Spawn::Ring {
        extent: 8.0,
        spread: 0.0,
        mode: 0,
        arc: std::f32::consts::TAU,
    };
    let aimed = crate::psys::Direction::Aimed {
        elevation: 0.0,
        azimuth: 0.244_346,
        jitter: 0.0,
    };
    let mut rng = Rng::new(3);
    for phi in [0.0f32, 1.0, 2.5, 4.0] {
        let (direction, offset) = place(
            ring,
            (aimed, None),
            1.0,
            (Vec3::X, Vec3::Y),
            Some(phi),
            &mut rng,
        );
        assert!(
            (offset / 8.0 - Vec3::new(phi.cos(), 0.0, phi.sin())).length() < 1e-4,
            "{offset}"
        );
        let turned = phi + 0.244_346;
        let want = Vec3::new(turned.cos(), 0.0, turned.sin());
        assert!(
            (direction - want).length() < 1e-4,
            "{direction} against {want}"
        );
    }
}

/// Shape 8 is shape 3 drawn over `pi`: every particle on the frame's `+Z` side, both
/// ends of the diameter reached. Shape 3 crosses to `-Z`.
#[test]
fn shape_eight_is_the_half_of_the_ring_on_the_frames_plus_z_side() {
    let half = Spawn::Ring {
        extent: 10.0,
        spread: 0.0,
        mode: 0,
        arc: super::arc_of(8),
    };
    let full = Spawn::Ring {
        extent: 10.0,
        spread: 0.0,
        mode: 0,
        arc: super::arc_of(3),
    };
    let mut rng = Rng::new(8);
    let mut seen_x = [false; 2];
    let mut full_below = false;
    for _ in 0..400 {
        let offset = half.offset(1.0, Vec3::Y, Vec3::X, Vec3::Y, &mut rng);
        assert!(offset.z >= -1e-4, "{offset}");
        assert!((offset.length() - 10.0).abs() < 1e-3, "{offset}");
        seen_x[usize::from(offset.x > 0.0)] = true;
        full_below |= full.offset(1.0, Vec3::Y, Vec3::X, Vec3::Y, &mut rng).z < -1.0;
    }
    assert_eq!(seen_x, [true, true], "the whole half, both quarters");
    assert!(full_below, "shape 3 is the whole ring");
}
