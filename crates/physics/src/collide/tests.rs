//! What the collision query surface in [`super`] is asserted to do. Split out of `collide.rs` under the
//! 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;

/// A single triangle in the `y = height` plane, wound so its normal is `+Y` under this crate's
/// assumption, large enough that a probe near the origin is well inside it.
fn floor(height: f32, surface: Surface, collider: u32) -> TriangleSoup {
    TriangleSoup::new(
        vec![
            [-100.0, height, -100.0],
            [-100.0, height, 100.0],
            [100.0, height, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        surface,
        collider,
    )
}

#[test]
fn a_downward_ray_hits_a_floor_below_it() {
    let soup = floor(0.0, Surface::Floor, 0);
    let hit = soup
        .raycast(
            Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 20.0),
            None,
            false,
        )
        .expect("a ray straight down from above a floor hits it");

    assert_eq!(hit.point.y, 0.0);
    assert_eq!(hit.distance, 10.0);
    assert_eq!(hit.surface, Surface::Floor);
    assert_eq!(hit.collider, 0);
}

/// The winding convention this crate assumes, as a test because the parser that will feed it is
/// written separately and the track data's handedness is not established.
#[test]
fn a_counter_clockwise_floor_seen_from_above_reports_an_upward_normal() {
    let soup = floor(0.0, Surface::Floor, 0);
    let hit = soup
        .raycast(
            Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, 10.0),
            None,
            false,
        )
        .expect("hit");

    assert!(hit.normal.y > 0.0, "normal was {:?}", hit.normal);
    assert!((hit.normal.length() - 1.0).abs() < 1e-6);
}

#[test]
fn a_ray_that_stops_short_of_the_floor_misses() {
    let soup = floor(0.0, Surface::Floor, 0);
    assert_eq!(
        soup.raycast(
            Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 5.0),
            None,
            false
        ),
        None
    );
}

/// The one degenerate case the force law will actually meet, because a probe
/// can settle arbitrarily close to the surface it is pushing off.
#[test]
fn a_segment_lying_in_the_triangles_plane_misses_rather_than_returning_nan() {
    let soup = floor(0.0, Surface::Floor, 0);
    let hit = soup.raycast(
        Ray::new(Vec3::new(-10.0, 0.0, 0.0), Vec3::X, 20.0),
        None,
        false,
    );
    assert_eq!(hit, None);
}

#[test]
fn a_segment_with_an_endpoint_exactly_on_the_plane_misses() {
    // Starting on the plane and heading away, and starting above and ending
    // exactly on it, are both a strict-sign-change miss.
    assert_eq!(
        segment_triangle(
            Vec3::ZERO,
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 0.0),
        ),
        None
    );
    assert_eq!(
        segment_triangle(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::ZERO,
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 0.0),
        ),
        None
    );
}

#[test]
fn a_zero_length_segment_misses_everything() {
    let soup = floor(0.0, Surface::Floor, 0);
    assert_eq!(
        soup.raycast(Ray::new(Vec3::ZERO, Vec3::NEG_Y, 0.0), None, false),
        None
    );
}

#[test]
fn a_zero_area_triangle_misses_rather_than_dividing_by_zero() {
    let hit = segment_triangle(
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::ZERO,
    );
    assert_eq!(hit, None);
}

#[test]
fn a_ray_outside_the_triangles_edges_misses() {
    let soup = floor(0.0, Surface::Floor, 0);
    assert_eq!(
        soup.raycast(
            Ray::new(Vec3::new(1000.0, 10.0, 0.0), Vec3::NEG_Y, 20.0),
            None,
            false
        ),
        None
    );
}

#[test]
fn triangles_with_out_of_range_indices_are_dropped_at_construction() {
    let soup = TriangleSoup::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2], [0, 1, 7]],
        Vec::new(),
        Surface::Floor,
        0,
    );
    assert_eq!(soup.triangle_count(), 1);
}

#[test]
fn a_mesh_without_chunk_three_reports_a_vertex_scalar_of_one() {
    let soup = floor(0.0, Surface::Floor, 0);
    let hit = soup
        .raycast(
            Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, 10.0),
            None,
            false,
        )
        .expect("hit");
    assert_eq!(hit.vertex_scalar, 1.0);
}

#[test]
fn a_mesh_with_chunk_three_reports_the_mean_of_three_vertices() {
    let soup = TriangleSoup::new(
        vec![[-1.0, 0.0, -1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
        vec![[0, 1, 2]],
        vec![0.0, 3.0, 6.0],
        Surface::Floor,
        0,
    );
    let hit = soup
        .raycast(
            Ray::new(Vec3::new(-0.5, 5.0, 0.0), Vec3::NEG_Y, 10.0),
            None,
            false,
        )
        .expect("hit");
    assert_eq!(hit.vertex_scalar, 3.0);
}

#[test]
fn a_query_skips_the_collider_it_is_told_to_skip() {
    let soup = floor(0.0, Surface::Floor, 7);
    let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 20.0);
    assert!(soup.raycast(ray, None, false).is_some());
    assert_eq!(soup.raycast(ray, Some(7), false), None);
    assert!(soup.raycast(ray, Some(6), false).is_some());
}

#[test]
fn a_query_skips_reset_colliders_unless_asked_for_them() {
    let soup = floor(0.0, Surface::Reset, 0);
    let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 20.0);
    assert_eq!(soup.raycast(ray, None, false), None);
    assert!(soup.raycast(ray, None, true).is_some());
}

#[test]
fn a_world_returns_the_nearest_of_several_colliders() {
    let mut world = CollisionWorld::new();
    world.push(floor(0.0, Surface::Floor, 0));
    world.push(floor(4.0, Surface::Floor, 1));
    world.push(floor(-4.0, Surface::Floor, 2));

    let hit = world
        .raycast(
            Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 30.0),
            None,
            false,
        )
        .expect("hit");
    assert_eq!(hit.collider, 1);
    assert_eq!(hit.point.y, 4.0);
}

#[test]
fn a_world_query_can_skip_the_nearest_collider() {
    let mut world = CollisionWorld::new();
    world.push(floor(0.0, Surface::Floor, 0));
    world.push(floor(4.0, Surface::Floor, 1));

    let hit = world
        .raycast(
            Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 30.0),
            Some(1),
            false,
        )
        .expect("hit");
    assert_eq!(hit.collider, 0);
}

#[test]
fn an_empty_world_misses() {
    let world = CollisionWorld::new();
    assert_eq!(
        world.raycast(Ray::new(Vec3::ZERO, Vec3::NEG_Y, 10.0), None, false),
        None
    );
}

#[test]
fn a_mesh_with_no_vertices_has_a_box_that_overlaps_nothing() {
    let soup = TriangleSoup::new(Vec::new(), Vec::new(), Vec::new(), Surface::Floor, 0);
    assert!(
        !soup
            .bounds()
            .overlaps(&Aabb::from_segment(Vec3::ZERO, Vec3::ONE))
    );
    assert_eq!(
        soup.raycast(Ray::new(Vec3::ZERO, Vec3::NEG_Y, 10.0), None, false),
        None
    );
}

#[test]
fn mag_floor_is_hoverable_and_wall_is_not() {
    assert!(Surface::Floor.is_hoverable());
    assert!(Surface::MagFloor.is_hoverable());
    assert!(!Surface::Wall.is_hoverable());
    assert!(!Surface::Reset.is_hoverable());
}

/// The linear scan the broadphase replaced, kept as the definition of the answer: every
/// triangle in index order, the per-triangle box reject, then the narrowphase.
fn scan(soup: &TriangleSoup, ray: Ray) -> Vec<(usize, RaycastHit)> {
    let (p0, p1) = (ray.origin, ray.end());
    let segment = Aabb::from_segment(p0, p1);
    let mut hits = Vec::new();
    for index in 0..soup.triangle_count() {
        let [a, b, c] = soup.triangle(index).expect("in range");
        let mut tri_box = Aabb::EMPTY;
        for corner in [a, b, c] {
            tri_box.expand(corner);
        }
        if !segment.overlaps(&tri_box) {
            continue;
        }
        if let Some(hit) = segment_triangle(p0, p1, a, b, c) {
            hits.push((
                index,
                RaycastHit {
                    point: p0 + (p1 - p0) * hit.t,
                    normal: hit.normal.normalize_or_zero(),
                    distance: hit.t * ray.length,
                    surface: soup.surface(),
                    vertex_scalar: soup.triangle_scalar(soup.triangles[index]),
                    collider: soup.collider(),
                    triangle: Some([a, b, c]),
                },
            ));
        }
    }
    hits
}

/// The broadphase changes which triangles are visited and nothing about what is reported:
/// nearest hit, the ascending-index list, and the cut-off when the output is short, all
/// bit-equal to the linear scan on a soup with coplanar overlaps and shared edges.
#[test]
fn the_broadphase_answers_exactly_as_the_linear_scan_does() {
    let mut rng = oag_core::rng::Rng::new(7);
    let mut at = |scale: f32| (rng.next_f32() - 0.5) * scale;
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for cell in 0..300u32 {
        let base = [at(60.0), at(8.0), at(60.0)];
        for _ in 0..3 {
            vertices.push([base[0] + at(6.0), base[1] + at(1.0), base[2] + at(6.0)]);
        }
        triangles.push([cell * 3, cell * 3 + 1, cell * 3 + 2]);
        // The same triangle again: two coplanar hits report twice.
        if cell % 17 == 0 {
            triangles.push([cell * 3, cell * 3 + 1, cell * 3 + 2]);
        }
    }
    let soup = TriangleSoup::new(vertices, triangles, vec![0.5; 900], Surface::Wall, 3);

    let mut compared = 0;
    for _ in 0..2000 {
        let origin = Vec3::new(at(60.0), at(10.0) + 4.0, at(60.0));
        let direction = Vec3::new(at(2.0), at(2.0) - 1.0, at(2.0)).normalize_or_zero();
        let ray = Ray::new(origin, direction, 3.0 + at(2.0).abs() * 10.0);
        let expected = scan(&soup, ray);

        let nearest = soup.raycast(ray, None, false);
        let first = expected
            .iter()
            .fold(None, |best: Option<&RaycastHit>, (_, hit)| match best {
                Some(b) if hit.distance >= b.distance => best,
                _ => Some(hit),
            });
        assert_eq!(nearest.as_ref(), first);

        for room in [1, 2, 8] {
            let mut out = vec![None; room];
            let found = soup.raycast_all(ray, None, false, &mut out);
            assert_eq!(found, expected.len().min(room));
            for (got, (_, want)) in out.iter().zip(&expected).take(found) {
                assert_eq!(got.as_ref(), Some(want));
            }
        }
        compared += usize::from(!expected.is_empty());
    }
    assert!(compared > 100, "only {compared} rays hit anything");
}
