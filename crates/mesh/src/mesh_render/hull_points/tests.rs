use super::*;
use oag_vex::world_lights::Cone;

fn point(at: [f32; 3]) -> WorldLight {
    WorldLight {
        position: at,
        colour: [0.388, 0.907, 1.0],
        inner_radius: 7.0,
        outer_radius: 15.0,
        cone: None,
    }
}

fn fixed(at: [f32; 3]) -> PlacedLight {
    PlacedLight::fixed(point(at))
}

fn unpack(w: f32) -> [u32; 3] {
    let w = w as u32;
    [w & 255, w >> 8 & 255, w >> 16 & 255]
}

/// Tech de Ra, read live: the player's hull at `(-58.021, -9.514, 79.939)` with
/// every scale byte at 249 was handed this light as its fourth, from the
/// `wopoint` at `(-58.024, -5.471, 89.360)`.
#[test]
fn tech_de_ras_grid_takes_the_list_the_ge_was_handed() {
    let lights = HullPointLights {
        lights: vec![fixed([-58.024, -5.471, 89.36])],
        slots: 1,
    };
    let got = lights.at([-58.021, -9.514, 79.939], 0.0, [249, 249]);
    assert_eq!(unpack(got[0][3]), [61, 142, 157]);
    // The live vector was (-0.003, 4.043, 9.421), 10.25 long.
    assert!((got[0][1] - 0.3943).abs() < 1e-3 && (got[0][2] - 0.9189).abs() < 1e-3);
    assert_eq!(got[1], [0.0; 4], "the second slot is not there");
}

#[test]
fn a_light_beyond_its_outer_radius_takes_no_slot() {
    let lights = HullPointLights {
        lights: vec![fixed([0.0, 0.0, 0.0])],
        slots: 2,
    };
    assert_eq!(lights.at([0.0, 0.0, 15.5], 0.0, [255, 255]), [[0.0; 4]; 2]);
    assert_ne!(lights.at([0.0, 0.0, 14.0], 0.0, [255, 255])[0][3], 0.0);
}

#[test]
fn inside_the_inner_radius_is_full_strength() {
    let lights = HullPointLights {
        lights: vec![fixed([0.0, 0.0, 0.0])],
        slots: 1,
    };
    // 0.388 * 255 = 98.9, so 98, at the full 256 scale.
    assert_eq!(
        unpack(lights.at([0.0, 3.0, 0.0], 0.0, [255, 255])[0][3]),
        [(98 * 255) >> 8, (231 * 255) >> 8, (255 * 255) >> 8]
    );
}

#[test]
fn the_strongest_lights_win_the_slots_and_a_tie_keeps_the_first() {
    let lights = HullPointLights {
        lights: vec![
            fixed([0.0, 0.0, 10.0]),
            fixed([0.0, 0.0, 12.0]),
            fixed([0.0, 0.0, 8.0]),
        ],
        slots: 2,
    };
    // Distances 10, 12 and 8: the third replaces the weaker second, in its place.
    let got = lights.at([0.0; 3], 0.0, [255, 255]);
    assert!((got[0][2] - 1.0).abs() < 1e-6 && (got[1][2] - 1.0).abs() < 1e-6);
    let tie = HullPointLights {
        lights: vec![
            fixed([0.0, 0.0, 10.0]),
            fixed([0.0, 10.0, 0.0]),
            fixed([10.0, 0.0, 0.0]),
        ],
        slots: 2,
    };
    let got = tie.at([0.0; 3], 0.0, [255, 255]);
    assert!(
        got[0][2] > 0.9 && got[1][1] > 0.9,
        "the third, equally strong, does not displace"
    );
}

#[test]
fn a_spot_is_dark_outside_its_cone() {
    let mut spot = point([0.0, 0.0, 0.0]);
    spot.cone = Some(Cone {
        axis: [0.0, 0.0, 1.0],
        inner_angle: 0.5,
        outer_angle: 1.0,
    });
    let lights = HullPointLights {
        lights: vec![PlacedLight::fixed(spot)],
        slots: 1,
    };
    assert_ne!(
        lights.at([0.0, 0.0, 5.0], 0.0, [255, 255])[0][3],
        0.0,
        "on the axis"
    );
    assert_eq!(
        lights.at([0.0, 0.0, -5.0], 0.0, [255, 255])[0][3],
        0.0,
        "behind it"
    );
}

#[test]
fn none_and_no_free_slot_light_nothing() {
    assert_eq!(
        HullPointLights::none().at([0.0; 3], 0.0, [255, 255]),
        [[0.0; 4]; 2]
    );
    let full = HullPointLights {
        lights: vec![fixed([0.0, 0.0, 5.0])],
        slots: 0,
    };
    assert!(full.is_empty());
}

/// The builder's returned colour takes each chosen light as colour x strength x 255 x the
/// scale byte, not shifted down: a light that reaches the craft at all saturates.
#[test]
fn a_chosen_light_adds_its_unshifted_product_to_the_pass_colour() {
    let lights = HullPointLights {
        lights: vec![fixed([0.0, 0.0, 11.0])],
        slots: 1,
    };
    // 11 units from a light of radii 7 and 15: t = 0.5, strength 0.5.
    let sum = lights.accumulated([0.0; 3], 0.0, [200, 100]);
    let want = [0.388, 0.907, 1.0].map(|c: f32| c * (0.5 * 255.0) * 200.0);
    for k in 0..3 {
        assert!((sum[k] - want[k]).abs() < 1.0, "{sum:?} against {want:?}");
    }
    assert_eq!(
        lights.accumulated([0.0, 0.0, 40.0], 0.0, [200, 100]),
        [0.0; 3]
    );
    assert_eq!(
        HullPointLights::none().accumulated([0.0; 3], 0.0, [255, 255]),
        [0.0; 3]
    );
}
