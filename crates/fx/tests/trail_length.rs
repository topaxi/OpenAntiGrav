//! The PSP ribbon at its own length is exactly what it was, and the long length is a longer ribbon.
//!
//! The first half pins the vertex bytes of a curving, wrapped ring under both laws, at two
//! intensities, on the recorded ring and between two ticks (a dropped sample in play). Its
//! fingerprints were taken before the trail length became a setting.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::exhaust::{
    Exhaust, LAYERS, LONG_TRAIL_SAMPLES, SUBSTEP_DT, TRAIL_FINS, TRAIL_MAX_VERTICES_PER_CRAFT,
    TRAIL_SAMPLES, TrailHead, TrailLaw,
};
use oag_mesh::mesh::GpuVertex;

/// The vertex count, then the sum of every position, colour and texture coordinate component.
///
/// Sums rather than a byte digest: the ribbon's `exp` and `powf` are the platform's, and the last
/// bit of them is not the same on every machine. A change to any age, width or fade moves a sum by
/// far more than [`TOLERANCE`].
fn fingerprint(vertices: &[GpuVertex]) -> [f64; 10] {
    let mut out = [0.0; 10];
    out[0] = vertices.len() as f64;
    for v in vertices {
        let parts = v.position.iter().chain(&v.colour).chain(&v.texcoord);
        for (slot, part) in out[1..].iter_mut().zip(parts) {
            *slot += f64::from(*part);
        }
    }
    out
}

/// Relative, on each sum.
const TOLERANCE: f64 = 1e-4;

/// A craft that curves and changes speed: the ring wraps several times, and
/// every sample has its own direction.
fn flown(ticks: usize, throttle_until: usize) -> Exhaust {
    let mut exhaust = Exhaust::new();
    let mut rng = Rng::new(7);
    exhaust.snap(1.0, 140.0);
    let mut position = Vec3::new(0.0, 1.0, 0.0);
    for tick in 0..ticks {
        let heading = 0.02 * tick as f32;
        let forward = Vec3::new(heading.sin(), 0.0, heading.cos());
        position += forward * (2.0 + 0.01 * (tick % 17) as f32);
        let thrust = if tick < throttle_until { 1.0 } else { 0.0 };
        exhaust.advance(SUBSTEP_DT, thrust, 140.0, &mut rng);
        exhaust.push_trail(position, -forward);
    }
    exhaust
}

fn vertices(exhaust: &Exhaust, head: Option<TrailHead>, law: TrailLaw) -> Vec<GpuVertex> {
    let mut out = Vec::new();
    exhaust.extend_trail_vertices(&mut out, head, Vec3::X, Vec3::Y, law);
    out
}

fn between_ticks() -> Option<TrailHead> {
    Some(TrailHead {
        position: Vec3::new(1.0, 1.0, 400.0),
        alpha: 0.4,
    })
}

#[test]
fn the_original_length_is_byte_identical() {
    let mut seen = Vec::new();
    for throttle_until in [usize::MAX, 20] {
        let exhaust = flown(3 * TRAIL_SAMPLES + 4, throttle_until);
        for law in [TrailLaw::Ramped, TrailLaw::Baked] {
            seen.push(fingerprint(&vertices(&exhaust, None, law)));
            seen.push(fingerprint(&vertices(&exhaust, between_ticks(), law)));
        }
    }
    println!("{seen:?}");
    assert_ne!(seen[0], seen[4], "the two intensities must differ");
    for (case, (seen, golden)) in seen.iter().zip(GOLDEN).enumerate() {
        for (part, (a, b)) in seen.iter().zip(golden).enumerate() {
            assert!(
                (a - b).abs() <= TOLERANCE * b.abs().max(1.0),
                "case {case} part {part}: {a} against {b}"
            );
        }
    }
}

const GOLDEN: [[f64; 10]; 8] = [
    [
        648.0000, 10516.5904, 648.0000, 36416.8483, 56.0957, 33.5393, 71.0311, 160.6588, 3963.5467,
        986.3903,
    ],
    [
        720.0000, 10559.0876, 720.0000, 51894.0402, 69.5499, 41.5835, 88.0674, 181.3271, 4401.7821,
        1093.5892,
    ],
    [
        432.0000, 7011.0602, 432.0000, 24277.8989, 159.8171, 91.9495, 146.2436, 0.0000, 2408.3689,
        647.9935,
    ],
    [
        480.0000, 7039.3917, 480.0000, 34596.0268, 198.1481, 114.0030, 181.3190, 0.0000, 2695.4062,
        719.9928,
    ],
    [
        648.0000, 10516.5904, 648.0000, 36416.8483, 44.8568, 25.8571, 58.2348, 142.0235, 3963.5467,
        986.3903,
    ],
    [
        720.0000, 10559.0876, 720.0000, 51894.0402, 55.6154, 32.0587, 72.2020, 160.3765, 4401.7821,
        1093.5892,
    ],
    [
        432.0000, 7011.0602, 432.0000, 24277.8989, 159.8171, 91.9495, 146.2436, 0.0000, 2408.3689,
        647.9935,
    ],
    [
        480.0000, 7039.3917, 480.0000, 34596.0268, 198.1481, 114.0030, 181.3190, 0.0000, 2695.4062,
        719.9928,
    ],
];

/// The longer ribbon: more segments, and a colour past the ninth sample, where the original's
/// fade has long since reached nothing.
#[test]
fn the_long_length_is_a_longer_ribbon_with_its_own_tail() {
    let mut original = flown(LONG_TRAIL_SAMPLES + 6, usize::MAX);
    let mut long = original;
    long.set_trail_samples(LONG_TRAIL_SAMPLES);
    assert!(!long.trail_ready(), "a change of length clears the ring");
    for tick in 0..LONG_TRAIL_SAMPLES {
        long.push_trail(Vec3::new(0.0, 1.0, tick as f32 * 2.0), -Vec3::Z);
    }
    assert!(long.trail_ready());
    original.set_trail_samples(TRAIL_SAMPLES);

    let short = vertices(&original, None, TrailLaw::Ramped);
    let long = vertices(&long, None, TrailLaw::Ramped);
    let per_segment = TRAIL_FINS * 6;
    assert_eq!(short.len(), LAYERS * (TRAIL_SAMPLES - 1) * per_segment);
    assert_eq!(long.len(), LAYERS * (LONG_TRAIL_SAMPLES - 1) * per_segment);
    assert!(long.len() <= TRAIL_MAX_VERTICES_PER_CRAFT);

    // The innermost layer's segments, head first: the head is as bright as the original's, the
    // middle of the long ring is still lit where the original's fade is under a hundredth, and
    // the last segment is dark as the original's last.
    let layer = &long[2 * (LONG_TRAIL_SAMPLES - 1) * per_segment..];
    let at = |segment: usize| layer[segment * per_segment].colour[0];
    let head = vertices(&original, None, TrailLaw::Ramped)[2 * (TRAIL_SAMPLES - 1) * per_segment]
        .colour[0];
    assert!((at(0) - head).abs() < 1e-5, "{} against {head}", at(0));
    assert!(at(LONG_TRAIL_SAMPLES / 2) > 0.05 * at(0));
    assert!(at(LONG_TRAIL_SAMPLES - 2) < 0.01 * at(0));
    // Monotone: the fade never rises along the ribbon.
    assert!((1..LONG_TRAIL_SAMPLES - 1).all(|k| at(k) <= at(k - 1)));
}

/// Switching back restores the original ribbon exactly.
#[test]
fn setting_the_length_is_idempotent_and_reversible() {
    let mut exhaust = flown(3 * TRAIL_SAMPLES + 4, usize::MAX);
    let before = exhaust;
    exhaust.set_trail_samples(TRAIL_SAMPLES);
    assert_eq!(exhaust, before, "the same length leaves the ring alone");
    exhaust.set_trail_samples(LONG_TRAIL_SAMPLES);
    exhaust.set_trail_samples(TRAIL_SAMPLES);
    assert_eq!(exhaust.trail_samples_kept(), TRAIL_SAMPLES);
    assert!(!exhaust.trail_ready());
}
