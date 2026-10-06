//! What blend class 8's quads are asserted to carry: the emitter's own
//! strength in the colour slots, the channel's alpha, and only when the
//! sprite is on the sheet.

use super::*;
use crate::psys::sprite::Sprite;
use crate::psys::tests::{effect, run};

const STRENGTH: f32 = 2.5;

/// One billboard emitter, blend class 8, with a strength and a palette whose
/// colour is deliberately not the strength: if the palette reached the vertex
/// the test would see it.
fn distort_effect(placed: bool) -> std::sync::Arc<Effect> {
    let mut built = (*effect("haze", true, 100.0)).clone();
    let spec = &mut built.emitters[0];
    spec.blend = Blend::Distort;
    spec.distort_strength = STRENGTH;
    *spec.palette = [[0.1, 0.2, 0.3, 0.4]; 256];
    spec.sheet_rect = placed.then_some([0.25, 0.25, 0.5, 0.5]);
    std::sync::Arc::new(built)
}

fn quads(effect: &Effect) -> (Vec<GpuVertex>, Vec<GpuVertex>, Vec<GpuVertex>) {
    let mut system = System::new();
    system.ignite(effect, Vec3::ZERO, 1.0);
    run(&mut system, effect, 5, &mut oag_core::Rng::new(3));
    assert!(system.alive_count() > 0, "the emitter spawned nothing");
    let (additive, alpha_over) = system.vertices(effect, Vec3::X, Vec3::Y);
    let mut distort = Vec::new();
    system.extend_distort_vertices(&mut distort, effect, Vec3::X, Vec3::Y);
    (additive, alpha_over, distort)
}

#[test]
fn a_distort_quad_carries_the_emitters_strength_and_not_its_palette() {
    let (additive, alpha_over, distort) = quads(&distort_effect(true));
    assert!(
        additive.is_empty() && alpha_over.is_empty(),
        "a class 8 particle drew a colour"
    );
    assert!(!distort.is_empty() && distort.len() % 6 == 0);
    for vertex in &distort {
        assert_eq!(vertex.colour[..3], [STRENGTH; 3], "kColourScale is the rgb");
        assert_eq!(vertex.colour[3], 1.0, "alpha is the channel's, 255 of 255");
        assert_eq!(vertex.normal[0], 1.0, "mapped onto the sprite's cell");
        assert!(
            (0.25..=0.5).contains(&vertex.texcoord[0])
                && (0.25..=0.5).contains(&vertex.texcoord[1]),
            "{:?} is outside the placed cell",
            vertex.texcoord
        );
    }
}

#[test]
fn a_distort_emitter_without_its_sprite_draws_nothing() {
    let (_, _, distort) = quads(&distort_effect(false));
    assert!(distort.is_empty(), "a procedural stand-in was invented");
}

#[test]
fn only_class_8_reaches_the_offset_pass() {
    let mut plain = (*distort_effect(true)).clone();
    plain.emitters[0].blend = Blend::Additive;
    let (additive, _, distort) = quads(&plain);
    assert!(!additive.is_empty());
    assert!(distort.is_empty());
}

#[test]
fn a_distort_emitter_with_a_sprite_is_not_undrawn() {
    let mut effect = (*distort_effect(true)).clone();
    assert_eq!(effect.undrawn_emitters().count(), 1, "no sprite read yet");
    effect.emitters[0].sprite = Some(Sprite {
        width: 1,
        height: 1,
        rgba: std::sync::Arc::from([128u8, 128, 128, 255]),
    });
    assert_eq!(effect.undrawn_emitters().count(), 0);
}
