use super::*;

const CELL: [f32; 4] = [0.0, 0.0, 0.5, 0.25];

fn draw(splashes: &Splashes) -> (usize, usize) {
    let (mut additive, mut alpha) = (Vec::new(), Vec::new());
    splashes.extend_vertices(&mut additive, &mut alpha, Vec3::X, Vec3::Y, CELL);
    (additive.len(), alpha.len())
}

#[test]
fn a_rain_hit_draws_a_ring_and_its_droplets_then_ends() {
    let mut splashes = Splashes::default();
    splashes.spawn(Vec3::new(1.0, 2.0, 3.0), Vec3::Y, false);
    assert_eq!(splashes.len(), 1);
    let (additive, alpha) = draw(&splashes);
    assert_eq!(additive, 6, "one ring");
    assert_eq!(alpha, 6 * DROPLETS, "its droplets, none fallen yet");
    splashes.advance(RAIN_LIFE + 0.01);
    assert!(splashes.is_empty());
    assert_eq!(draw(&splashes), (0, 0));
}

#[test]
fn a_snow_hit_settles_as_one_flake_that_fades_in_and_out() {
    let mut splashes = Splashes::default();
    splashes.spawn(Vec3::ZERO, Vec3::Y, true);
    let alpha_at = |splashes: &Splashes| {
        let (mut additive, mut alpha) = (Vec::new(), Vec::new());
        splashes.extend_vertices(&mut additive, &mut alpha, Vec3::X, Vec3::Y, CELL);
        assert!(alpha.is_empty());
        assert_eq!(additive.len(), 6);
        additive[0].colour[3]
    };
    assert!(alpha_at(&splashes) < 0.01, "born invisible");
    splashes.advance(SNOW_LIFE * 0.5);
    assert!(alpha_at(&splashes) > 0.8, "settled");
    splashes.advance(SNOW_LIFE * 0.45);
    assert!(alpha_at(&splashes) < 0.3, "fading out");
    splashes.advance(SNOW_LIFE);
    assert!(splashes.is_empty());
}

#[test]
fn the_pool_is_capped() {
    let mut splashes = Splashes::default();
    for _ in 0..CAPACITY * 2 {
        splashes.spawn(Vec3::ZERO, Vec3::Y, false);
    }
    assert_eq!(splashes.len(), CAPACITY);
}

#[test]
fn the_ring_lies_flat_on_the_surface_it_hit() {
    let mut splashes = Splashes::default();
    let normal = Vec3::new(0.0, 0.6, 0.8);
    splashes.spawn(Vec3::new(5.0, 0.0, 0.0), normal, false);
    let (mut additive, mut alpha) = (Vec::new(), Vec::new());
    splashes.extend_vertices(&mut additive, &mut alpha, Vec3::X, Vec3::Y, CELL);
    for corner in &additive {
        let offset = Vec3::from(corner.position) - Vec3::new(5.0, 0.0, 0.0);
        assert!((offset.dot(normal) - LIFT).abs() < 1e-4, "{offset:?}");
    }
}

#[test]
fn the_sprite_cells_stay_inside_the_placed_rectangle() {
    let mut splashes = Splashes::default();
    splashes.spawn(Vec3::ZERO, Vec3::Y, false);
    let (mut additive, mut alpha) = (Vec::new(), Vec::new());
    splashes.extend_vertices(&mut additive, &mut alpha, Vec3::X, Vec3::Y, CELL);
    for vertex in additive.iter().chain(&alpha) {
        let [u, v] = vertex.texcoord;
        assert!((CELL[0]..=CELL[2]).contains(&u) && (CELL[1]..=CELL[3]).contains(&v));
        assert_eq!(vertex.normal[0], 1.0, "marked as sampled from the sheet");
    }
}
