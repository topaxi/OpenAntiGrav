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
    assert_eq!(spray.mist_len(), MIST_CAPACITY);
    spray.advance(MIST_LIFE + 0.1, 0.0, &[]);
    assert!(spray.is_empty());
}

fn mist_height(spray: &Spray) -> f32 {
    let sum: f32 = spray.mist.iter().map(|puff| puff.position().y).sum();
    sum / spray.mist_len() as f32
}

#[test]
fn the_mist_rises_and_outlives_the_droplets() {
    let mut spray = Spray::default();
    spray.advance(1.0 / 60.0, 1.0, &[emitter(600.0); 4]);
    assert!(spray.mist_len() > 0);
    let born = mist_height(&spray);
    spray.advance(0.5, 0.0, &[]);
    assert_eq!(spray.len(), 0);
    assert!(spray.mist_len() > 0, "mist gone with the droplets");
    assert!(mist_height(&spray) > born + 0.5);
    spray.advance(MIST_LIFE, 0.0, &[]);
    assert_eq!(spray.mist_len(), 0);
}

#[test]
fn no_mist_on_a_dry_road_a_slow_craft_or_without_an_emitter() {
    for (speed, wet) in [(600.0, 0.0), (MIN_SPEED_KMH - 1.0, 1.0)] {
        let mut spray = Spray::default();
        for _ in 0..30 {
            spray.advance(1.0 / 60.0, wet, &[emitter(speed)]);
        }
        assert_eq!(spray.mist_len(), 0, "{speed} km/h wet {wet}");
    }
    let mut spray = Spray::default();
    spray.advance(1.0 / 60.0, 1.0, &[]);
    assert_eq!(spray.mist_len(), 0);
}

#[test]
fn mist_is_faint_and_fades_in_and_out() {
    let at = |t: f32| mist_alpha(MIST_LIFE * t);
    assert_eq!(at(0.0), 0.0);
    assert_eq!(at(1.0), 0.0);
    assert!(at(0.05) < at(MIST_FADE_IN) && at(0.9) < at(MIST_FADE_IN));
    assert!((0.0..=MIST_ALPHA).contains(&at(MIST_FADE_IN)));
    assert!(MIST_ALPHA < ALPHA);
}

#[test]
fn mist_puffs_reach_the_draw_list() {
    let mut spray = Spray::default();
    spray.advance(1.0 / 60.0, 1.0, &[emitter(600.0); 4]);
    spray.advance(0.5, 0.0, &[]);
    assert_eq!(spray.len(), 0);
    let mut out = Vec::new();
    spray.extend_vertices(&mut out, Vec3::X, Vec3::Y, [0.0, 0.0, 0.5, 0.25]);
    assert_eq!(out.len(), spray.mist_len() * 6);
}
