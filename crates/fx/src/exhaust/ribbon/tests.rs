//! Both trails drawn between ticks: continuous across a tick, the recorded
//! ring at alpha 1.

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use super::super::hd::{FINS, SAMPLES, Tube};
use super::super::{Exhaust, LAYERS, SUBSTEP_DT, TRAIL_FINS, TRAIL_SAMPLES, TrailLaw};
use super::TrailHead;

/// A craft flying `+z` at two units a tick, its nozzle at `z = 2 * tick`.
fn nozzle(tick: usize) -> Vec3 {
    Vec3::new(0.0, 1.0, 2.0 * tick as f32)
}

/// The drawn nozzle at `alpha` past tick `tick - 1`.
fn head(tick: usize, alpha: f32) -> TrailHead {
    TrailHead {
        position: nozzle(tick - 1).lerp(nozzle(tick), alpha),
        alpha,
    }
}

/// The largest difference between two vertex lists of one length, over the
/// position, the colour and the texture coordinate (the latter mod 1, a
/// wrapped scroll draws the same texels).
fn largest_gap(a: &[GpuVertex], b: &[GpuVertex]) -> f32 {
    assert_eq!(
        a.len(),
        b.len(),
        "the segment count changed across the tick"
    );
    let wrapped = |d: f32| {
        let d = d.rem_euclid(1.0);
        d.min(1.0 - d)
    };
    a.iter()
        .zip(b)
        .map(|(a, b)| {
            let position = (Vec3::from(a.position) - Vec3::from(b.position)).length();
            let colour = (0..4)
                .map(|c| (a.colour[c] - b.colour[c]).abs())
                .fold(0.0, f32::max);
            let uv =
                wrapped(a.texcoord[0] - b.texcoord[0]).max(wrapped(a.texcoord[1] - b.texcoord[1]));
            position.max(colour).max(uv)
        })
        .fold(0.0, f32::max)
}

/// A craft at full thrust and steady speed, one tick of the window's order
/// (advance, then push) per sample.
fn psp(ticks: usize) -> Exhaust {
    let mut exhaust = Exhaust::new();
    let mut rng = oag_core::Rng::new(1);
    exhaust.snap(1.0, 120.0);
    for tick in 0..ticks {
        exhaust.advance(SUBSTEP_DT, 1.0, 120.0, &mut rng);
        exhaust.push_trail(nozzle(tick), -Vec3::Z);
    }
    exhaust
}

/// `vertices` cut into `layers` runs of segments of `per_segment` vertices,
/// dropping `skip` segments off the front of each run and `trim` off its end:
/// just before a tick the tail segment closes to zero length, and just after
/// it the head's opens from zero, so the same curve sits one segment apart in
/// the two lists.
fn segments(
    vertices: &[GpuVertex],
    layers: usize,
    per_segment: usize,
    skip: usize,
    trim: usize,
) -> Vec<GpuVertex> {
    let per_layer = vertices.len() / layers;
    vertices
        .chunks(per_layer)
        .flat_map(|layer| layer[skip * per_segment..layer.len() - trim * per_segment].to_vec())
        .collect()
}

fn psp_vertices(exhaust: &Exhaust, head: Option<TrailHead>) -> Vec<GpuVertex> {
    let mut out = Vec::new();
    exhaust.extend_trail_vertices(&mut out, head, Vec3::X, Vec3::Y, TrailLaw::Ramped);
    out
}

#[test]
fn the_psp_ribbon_is_continuous_across_a_tick() {
    let ticks = TRAIL_SAMPLES + 4;
    let before = psp_vertices(&psp(ticks), Some(head(ticks - 1, 0.9999)));
    let after = psp_vertices(&psp(ticks + 1), Some(head(ticks, 0.0001)));
    let per_segment = TRAIL_FINS * 6;
    let gap = largest_gap(
        &segments(&before, LAYERS, per_segment, 0, 1),
        &segments(&after, LAYERS, per_segment, 1, 0),
    );
    assert!(gap < 1e-2, "{gap}");
    // And it moves on every frame between: halfway is neither end.
    let half = psp_vertices(&psp(ticks), Some(head(ticks - 1, 0.5)));
    assert!(largest_gap(&before, &half) > 0.1);
}

#[test]
fn the_psp_ribbon_at_alpha_one_is_the_recorded_ring() {
    let exhaust = psp(TRAIL_SAMPLES + 3);
    let tick = TRAIL_SAMPLES + 2;
    let recorded = psp_vertices(&exhaust, None);
    let headed = psp_vertices(
        &exhaust,
        Some(TrailHead {
            position: nozzle(tick),
            alpha: 1.0,
        }),
    );
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&recorded),
        bytemuck::cast_slice::<_, u8>(&headed)
    );
}

/// Until a sample has fallen off the ring there is no tail to reach back to:
/// the ribbon draws the ring's own segments, never a stale point.
#[test]
fn a_fresh_psp_ring_draws_no_extra_tail() {
    let exhaust = psp(TRAIL_SAMPLES);
    let at_tick = psp_vertices(&exhaust, None).len();
    let between = psp_vertices(&exhaust, Some(head(TRAIL_SAMPLES - 1, 0.5))).len();
    assert_eq!(between, at_tick);
}

fn hd(ticks: usize) -> Tube {
    let mut tube = Tube::new();
    for tick in 0..ticks {
        tube.push(nozzle(tick), Vec3::Y, 0.5);
    }
    tube
}

fn hd_vertices(tube: &Tube, head: Option<TrailHead>) -> Vec<GpuVertex> {
    let mut out = Vec::new();
    tube.extend_vertices(&mut out, head, 1.0, 0.5, 0.0);
    out
}

#[test]
fn the_hd_tube_is_continuous_across_a_tick() {
    let ticks = SAMPLES + 4;
    let before = hd_vertices(&hd(ticks), Some(head(ticks - 1, 0.9999)));
    let after = hd_vertices(&hd(ticks + 1), Some(head(ticks, 0.0001)));
    let gap = largest_gap(
        &segments(&before, FINS, 6, 0, 1),
        &segments(&after, FINS, 6, 1, 0),
    );
    assert!(gap < 1e-2, "{gap}");
}

#[test]
fn the_hd_tube_at_alpha_one_is_the_recorded_ring() {
    let tube = hd(SAMPLES + 3);
    let tick = SAMPLES + 2;
    let recorded = hd_vertices(&tube, None);
    let headed = hd_vertices(
        &tube,
        Some(TrailHead {
            position: nozzle(tick),
            alpha: 1.0,
        }),
    );
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&recorded),
        bytemuck::cast_slice::<_, u8>(&headed)
    );
}
