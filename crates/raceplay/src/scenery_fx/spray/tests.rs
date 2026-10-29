use super::*;

fn emitter(speed_kmh: f32) -> Emitter {
    Emitter {
        at: Vec3::ZERO,
        normal: Vec3::Y,
        forward: Vec3::Z,
        half_width: 2.0,
        speed_kmh,
    }
}

fn thrown(speed_kmh: f32, wet: f32) -> usize {
    let mut spray = Spray::default();
    for _ in 0..30 {
        spray.advance(1.0 / 60.0, wet, &[emitter(speed_kmh)]);
    }
    spray.len()
}

#[test]
fn a_dry_road_throws_nothing() {
    assert_eq!(thrown(600.0, 0.0), 0);
}

#[test]
fn a_slow_craft_throws_nothing() {
    assert_eq!(thrown(MIN_SPEED_KMH - 1.0, 1.0), 0);
}

#[test]
fn no_emitter_no_spray() {
    let mut spray = Spray::default();
    spray.advance(1.0 / 60.0, 1.0, &[]);
    assert!(spray.is_empty());
}

#[test]
fn the_amount_rises_with_speed_and_wetness() {
    assert!(thrown(200.0, 1.0) < thrown(400.0, 1.0));
    assert!(thrown(400.0, 1.0) < thrown(600.0, 1.0));
    assert!(thrown(400.0, 0.3) < thrown(400.0, 1.0));
}

#[test]
fn the_pool_is_capped_and_drains() {
    let mut spray = Spray::default();
    let many = [emitter(600.0); 40];
    for _ in 0..20 {
        spray.advance(1.0 / 60.0, 1.0, &many);
    }
    assert_eq!(spray.len(), CAPACITY);
    spray.advance(LIFE + 0.1, 0.0, &[]);
    assert!(spray.is_empty());
}

#[test]
fn droplets_draw_translucent_and_stay_above_the_road() {
    let mut spray = Spray::default();
    spray.advance(1.0 / 60.0, 1.0, &[emitter(600.0); 4]);
    spray.advance(0.1, 1.0, &[]);
    let mut out = Vec::new();
    spray.extend_vertices(&mut out, Vec3::X, Vec3::Y, [0.0, 0.0, 0.5, 0.25]);
    assert!(!out.is_empty());
    for vertex in &out {
        assert!(vertex.colour[3] <= ALPHA);
        assert!(vertex.position[1] >= -SIZE - 1e-4);
    }
}
