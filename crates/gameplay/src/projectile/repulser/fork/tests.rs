//! The fork wave on a hand-built split: a loop whose right side is two parallel
//! paths, the primary at `x = 100` and the alternate at `x = 120`, both from
//! junction 0 to junction 1. Each test fails if the fork's rule it names is
//! dropped.

use super::*;
use crate::projectile::repulser::Repulser;
use oag_tables::weapons::RepulserStats;
use oag_vex::track::{AiTrack, Junction, Path, SplinePoint};

fn point(x: f32, z: f32) -> SplinePoint {
    SplinePoint {
        pos: [x, 0.0, z],
        tangent: [1.0, 0.0, 0.0],
        down: [0.0, -1.0, 0.0],
        lateral: [0.0, 0.0, 1.0],
        progress: 0.0,
        half_width_left: 4.0,
        half_width_right: 4.0,
        ai_bound_left: -5.0,
        ai_bound_right: 5.0,
        racing_line: 0.0,
        section_id: 0,
        flags: 0,
        light_scale: [0xff; 4],
    }
}

fn path(points: Vec<SplinePoint>, entry: usize, exit: usize) -> Path {
    Path {
        points,
        max_spacing: 10.0,
        entry: Some(entry),
        exit: Some(exit),
    }
}

/// Paths: 0 along the bottom, 1 up the right side (primary), 2 back over the
/// top and down the left, 3 up the right side 20 units further out (alternate).
fn split_track() -> AiTrack {
    let bottom = (0..10).map(|i| point(i as f32 * 10.0, 0.0)).collect();
    let right = (0..10).map(|i| point(100.0, i as f32 * 10.0)).collect();
    let outer = (0..10).map(|i| point(120.0, i as f32 * 10.0)).collect();
    let mut back: Vec<SplinePoint> = (0..11)
        .map(|i| point(100.0 - i as f32 * 10.0, 100.0))
        .collect();
    back.extend((1..10).map(|i| point(0.0, 100.0 - i as f32 * 10.0)));
    AiTrack {
        version: 1,
        paths: vec![
            path(bottom, 2, 0),
            path(right, 0, 1),
            path(back, 1, 2),
            path(outer, 0, 1),
        ],
        junctions: vec![
            Junction {
                prev: [Some(0), None],
                next: [Some(1), Some(3)],
            },
            Junction {
                prev: [Some(1), Some(3)],
                next: [Some(2), None],
            },
            Junction {
                prev: [Some(2), None],
                next: [Some(0), None],
            },
        ],
    }
}

fn course() -> Course {
    Course::from_track(&split_track(), None).expect("the split loop closes")
}

#[test]
fn the_alternate_path_is_a_branch_between_the_split_and_the_merge() {
    let course = course();
    let [branch] = course.branches() else {
        panic!("one branch, got {}", course.branches().len());
    };
    assert_eq!(branch.split, 40, "path 1 starts after path 0's 40 samples");
    assert_eq!(branch.merge, 80, "path 2 starts after path 1's 40");
    assert_eq!(branch.len(), 40);
    assert!((branch.centres[0].x - 120.0).abs() < 1e-3);
}

/// `AiTrack_StepForward` returns 1 at the split with steps left; the fork
/// takes those (mode 3) down the alternate path, and does not sweep on the
/// update it spawns (`+0x140 = +0x110`).
#[test]
fn a_forward_step_across_the_split_forks_with_the_steps_left() {
    let course = course();
    let fork = Fork::crossing(&course, 30, 20).expect("crosses at 40");
    assert_eq!(fork.offset, 10);
    assert!((fork.front.point - course.branches()[0].centres[10]).length() < 1e-6);
    assert_eq!(fork.front.previous, fork.front.point);
    assert!(!fork.moved());
    assert!(Fork::crossing(&course, 10, 20).is_none(), "short of it");
}

#[test]
fn the_fork_walks_the_branch_then_rejoins_the_ring_after_the_merge() {
    let course = course();
    let mut fork = Fork::crossing(&course, 30, 20).unwrap();
    fork.advance(&course, 20);
    assert_eq!((fork.branch, fork.offset), (Some(0), 30));
    assert!(fork.moved());
    fork.advance(&course, 20);
    assert_eq!(fork.branch, None);
    assert_eq!(fork.front.index, 90, "10 past the merge");
}

fn stats() -> RepulserStats {
    RepulserStats {
        absorb: 15.0,
        damage: 30.0,
        blastforce: 40.0,
        slowdown_time: 0.8,
        blast_time: 0.8,
        wave_time: 0.8,
    }
}

/// A craft out on the alternate path is swept by the third wave alone: the
/// primary waves run 20 units away, past the 10-unit corridor. And only one
/// fork ever starts (`+0x25c`).
#[test]
fn a_craft_on_the_alternate_path_is_swept_only_by_the_fork() {
    let course = course();
    let on_branch = course.branches()[0].centres[20];
    let mut repulser = Repulser::launch(0, &stats());
    let (mut by_fork, mut by_primary) = (false, false);
    let mut first_fork = None;
    while repulser.advance(&course, Some(30), 1.0 / 60.0) {
        by_primary |= repulser.swept_by(on_branch, 10.0).is_some();
        by_fork |= repulser.swept_by_any(on_branch, 10.0).is_some();
        if let Some(fork) = repulser.fork {
            let first = *first_fork.get_or_insert(fork.branch);
            assert!(first.is_some(), "it forked onto the branch");
        }
    }
    assert!(by_fork, "the fork swept the craft on the branch");
    assert!(!by_primary, "the primary waves never did");
}
