use super::*;

const DT: f32 = 1.0 / 60.0;

const HELIX: Spec = Spec {
    width: [0.08, 0.3],
    lifetime: 2.8,
    u_per_unit: 0.2,
    chains: 3,
    capacity: 350,
    phase_step: 0.06,
    radius: 0.6,
    ramp: true,
};

const STUB: Spec = Spec {
    width: [0.5, 0.0],
    lifetime: 0.3,
    u_per_unit: 0.2,
    chains: 1,
    capacity: 50,
    phase_step: 0.0,
    radius: 0.0,
    ramp: false,
};

/// The Missile explosion's pool: `RibbonEffects` pool `+0x5c`.
const STREAK: Spec = Spec {
    width: [0.0, 2.0],
    lifetime: 1.2,
    u_per_unit: 0.2,
    chains: 1,
    capacity: 350,
    phase_step: 0.06,
    radius: 0.6,
    ramp: false,
};

fn identity_ramp() -> OpacityRamp {
    let mut tga = vec![0xeeu8; crate::rocket_smoke::RAMP_OFFSET];
    tga.extend((0..=255u8).collect::<Vec<_>>());
    OpacityRamp::from_tga(&tga).expect("long enough")
}

fn flown(spec: Spec, ticks: usize) -> Ribbon {
    let mut ribbon = Ribbon::new(spec);
    for i in 0..ticks {
        ribbon.push(
            Vec3::new(0.0, 0.0, i as f32 * 3.0),
            Vec3::Z,
            Vec3::NEG_Y,
            1.0,
        );
        ribbon.advance(DT);
    }
    ribbon
}

#[test]
fn a_push_lays_one_node_per_chain_around_the_axis() {
    let mut ribbon = Ribbon::new(HELIX);
    ribbon.push(Vec3::new(1.0, 2.0, 3.0), Vec3::Z, Vec3::NEG_Y, 1.0);
    assert_eq!(ribbon.node_count(), 3);
    for chain in &ribbon.chains {
        let node = chain[0];
        let offset = node.position - Vec3::new(1.0, 2.0, 3.0);
        assert!((offset.length() - 0.6).abs() < 1e-5, "{offset}");
        assert!(offset.z.abs() < 1e-5, "stays in the plane across the axis");
    }
}

#[test]
fn chains_sit_sixty_degrees_apart_and_the_phase_steps_after_each() {
    let mut ribbon = Ribbon::new(HELIX);
    ribbon.push(Vec3::ZERO, Vec3::Z, Vec3::NEG_Y, 1.0);
    let angle = |chain: usize| {
        let p = ribbon.chains[chain][0].position;
        (-p.y).atan2(-p.x)
    };
    // Chain 0 at phase 0, chain 1 at pi/3 + 0.06, chain 2 at 2pi/3 + 0.12.
    let step = |a: usize, b: usize| (angle(b) - angle(a)).rem_euclid(2.0 * PI);
    assert!(
        (step(0, 1) - (PI / 3.0 + 0.06)).abs() < 1e-4,
        "{}",
        step(0, 1)
    );
    assert!(
        (step(1, 2) - (PI / 3.0 + 0.06)).abs() < 1e-4,
        "{}",
        step(1, 2)
    );
    assert!((ribbon.phase - 0.18).abs() < 1e-6);
}

#[test]
fn width_runs_from_birth_to_death_over_the_lifetime() {
    let mut ribbon = Ribbon::new(STUB);
    ribbon.push(Vec3::ZERO, Vec3::Z, Vec3::NEG_Y, 1.0);
    ribbon.advance(0.0);
    let node = ribbon.chains[0][0];
    assert!((node.half_width - 0.5).abs() < 1e-6);
    for _ in 0..9 {
        ribbon.advance(0.03);
    }
    // The width reads the life before the update's decrement: 0.06 s left, a fifth
    // of 0.3, so a fifth of the way from death (0) back up to birth (0.5).
    let node = ribbon.chains[0][0];
    assert!((node.half_width - 0.1).abs() < 1e-4, "{}", node.half_width);
}

#[test]
fn a_node_is_freed_one_update_after_its_life_is_spent() {
    let mut ribbon = Ribbon::new(STUB);
    ribbon.push(Vec3::ZERO, Vec3::Z, Vec3::NEG_Y, 1.0);
    let mut updates = 0;
    while !ribbon.is_empty() {
        ribbon.advance(0.125);
        updates += 1;
        assert!(updates < 20, "never freed");
    }
    // Life 0.3 runs to -0.075 over three updates, and the fourth frees it.
    assert_eq!(updates, 4);
}

#[test]
fn alpha_follows_one_minus_t_without_the_ramp_and_the_ramp_with_it() {
    let ramp = identity_ramp();
    let plain = flown(STUB, 10);
    let chain = &plain.chains[0];
    let mid = chain.len() / 2;
    let expected = chain[mid].fade;
    assert!((plain.alpha(chain, mid, &ramp) - expected).abs() < 1e-6);
    let ramped = flown(HELIX, 30);
    let chain = &ramped.chains[0];
    let mid = chain.len() / 2;
    let byte = ramp.lookup(chain[mid].fade);
    assert!((ramped.alpha(chain, mid, &ramp) - f32::from(byte) / 255.0).abs() < 1e-6);
}

#[test]
fn both_ends_of_a_chain_are_transparent() {
    let ramp = identity_ramp();
    let ribbon = flown(HELIX, 40);
    let chain = &ribbon.chains[0];
    assert_eq!(ribbon.alpha(chain, 0, &ramp), 0.0);
    assert_eq!(ribbon.alpha(chain, chain.len() - 1, &ramp), 0.0);
    assert!(ribbon.alpha(chain, chain.len() / 2, &ramp) > 0.0);
}

#[test]
fn u_is_the_running_arc_length_from_the_oldest_node() {
    let ribbon = flown(STUB, 5);
    let chain = &ribbon.chains[0];
    for (i, node) in chain.iter().enumerate() {
        assert!((node.u.unwrap() - 3.0 * i as f32).abs() < 1e-4, "{i}");
    }
}

#[test]
fn the_strip_faces_the_eye_and_is_two_widths_across() {
    let ramp = identity_ramp();
    let ribbon = flown(STUB, 6);
    let eye = Vec3::new(0.0, 10.0, 6.0);
    let mut vertices = Vec::new();
    ribbon.extend_vertices(&mut vertices, eye, &ramp);
    assert!(!vertices.is_empty());
    // A triangle list of `[l0, r0, l1, r0, l1, r1]` per segment.
    assert_eq!(vertices.len() % 6, 0);
    let l0 = Vec3::from(vertices[0].position);
    let r0 = Vec3::from(vertices[1].position);
    // The ribbon runs along z and the eye is above it: the side is across x.
    assert!((l0 - r0).normalize().abs().x > 0.99, "{}", l0 - r0);
    let node = ribbon.chains[0][1];
    assert!(((l0 - r0).length() - 2.0 * node.half_width).abs() < 1e-4);
    assert_eq!(vertices[0].texcoord[1], 0.0);
    assert_eq!(vertices[1].texcoord[1], 1.0);
}

#[test]
fn a_ribbon_outlives_what_laid_it_and_then_frees_itself() {
    let mut ribbon = flown(HELIX, 20);
    assert!(!ribbon.is_empty());
    let mut ticks = 0;
    while ribbon.advance(DT) {
        ticks += 1;
        assert!(ticks < 400, "never emptied");
    }
    assert!(ticks > 150, "freed after {ticks} ticks");
    assert!(ribbon.is_empty());
}

/// `FatLine_PushChains` gives a node `f1 * lifetime` and `FatLine_Update` divides by the
/// config's lifetime, so a streak pushed with `scale` is born at `t = scale`: width
/// `2 (1 - scale)` and alpha `1 - scale`, then widens to 2 and brightens to 1 as it dies.
#[test]
fn a_streak_node_is_born_at_t_equal_to_its_scale() {
    let ramp = identity_ramp();
    for scale in [0.5f32, 0.75, 1.0] {
        let mut ribbon = Ribbon::new(STREAK);
        for i in 0..3 {
            ribbon.push(Vec3::new(i as f32, 0.0, 0.0), Vec3::Z, Vec3::NEG_Y, scale);
        }
        ribbon.advance(0.0);
        let chain = &ribbon.chains[0];
        assert!((chain[1].life - 1.2 * scale).abs() < 1e-5);
        assert!(
            (chain[1].half_width - 2.0 * (1.0 - scale)).abs() < 1e-5,
            "width at scale {scale}: {}",
            chain[1].half_width
        );
        assert!(
            (ribbon.alpha(chain, 1, &ramp) - (1.0 - scale)).abs() < 1e-5,
            "alpha at scale {scale}"
        );
    }
}

#[test]
fn a_streak_node_widens_and_brightens_as_it_dies() {
    let ramp = identity_ramp();
    let mut ribbon = Ribbon::new(STREAK);
    for i in 0..3 {
        ribbon.push(Vec3::new(i as f32, 0.0, 0.0), Vec3::Z, Vec3::NEG_Y, 0.5);
    }
    // Life 0.6 s: after 0.3 s the middle node is at t = 0.25 of the config's 1.2 s
    // (the update reads the life before its own decrement).
    ribbon.advance(0.3);
    ribbon.advance(0.0);
    let chain = &ribbon.chains[0];
    let t = (0.6 - 0.3) / 1.2;
    assert!((chain[1].half_width - 2.0 * (1.0 - t)).abs() < 1e-4);
    assert!((ribbon.alpha(chain, 1, &ramp) - (1.0 - t)).abs() < 1e-4);
}

/// A node of scale `s` outlives the push by `1.2 s`, not the config's whole second life.
#[test]
fn a_scaled_node_dies_on_its_own_shorter_life() {
    let mut ribbon = Ribbon::new(STREAK);
    ribbon.push(Vec3::ZERO, Vec3::Z, Vec3::NEG_Y, 0.5);
    let mut updates = 0;
    while !ribbon.is_empty() {
        ribbon.advance(0.125);
        updates += 1;
        assert!(updates < 30);
    }
    // 0.6 s runs to -0.025 over five updates; the sixth frees it.
    assert_eq!(updates, 6);
}
