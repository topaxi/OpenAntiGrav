//! Hand-built tracks for the unit tests.
//!
//! Everything here is synthetic. The point of the race layer is that it can be
//! tested without a disc, a GPU or a track file: a lap counter that only works
//! against real data cannot be debugged when it disagrees with real data.

use oag_formats::track::{AiTrack, Junction, Path, SplinePoint};

/// A control point at `pos`, with a unit frame and a two-unit half width.
#[must_use]
pub fn point(pos: [f32; 3]) -> SplinePoint {
    SplinePoint {
        pos,
        tangent: [1.0, 0.0, 0.0],
        down: [0.0, -1.0, 0.0],
        lateral: [0.0, 0.0, 1.0],
        half_width_left: 2.0,
        half_width_right: 2.0,
        ai_bound_left: 2.0,
        ai_bound_right: 2.0,
        racing_line: 0.0,
        section_id: 0,
        flags: 0,
    }
}

/// A closed 30x30 square split into `paths` equal paths, joined primary-only.
///
/// Twelve control points, so `paths` must divide twelve. The shape is a loop
/// with a length that can be reasoned about by hand, which is the only property
/// the rules tests need from a track.
///
/// # Panics
///
/// If `paths` does not divide twelve, or is zero.
#[must_use]
pub fn square_track(paths: usize) -> AiTrack {
    assert!(paths > 0 && 12 % paths == 0, "paths must divide twelve");

    let corners: Vec<[f32; 3]> = (0..12)
        .map(|i| match i / 3 {
            0 => [i as f32 * 10.0, 0.0, 0.0],
            1 => [30.0, 0.0, (i - 3) as f32 * 10.0],
            2 => [30.0 - (i - 6) as f32 * 10.0, 0.0, 30.0],
            _ => [0.0, 0.0, 30.0 - (i - 9) as f32 * 10.0],
        })
        .collect();

    let per = 12 / paths;
    let built: Vec<Path> = (0..paths)
        .map(|p| Path {
            points: corners[p * per..(p + 1) * per]
                .iter()
                .copied()
                .map(point)
                .collect(),
            max_spacing: 10.0,
            entry: Some(if p == 0 { paths - 1 } else { p - 1 }),
            exit: Some(p),
        })
        .collect();

    let junctions: Vec<Junction> = (0..paths)
        .map(|j| Junction {
            prev: [Some(j), None],
            next: [Some((j + 1) % paths), None],
        })
        .collect();

    AiTrack {
        version: 1,
        paths: built,
        junctions,
    }
}
