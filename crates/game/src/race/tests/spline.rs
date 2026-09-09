//! `Spline` and the AI line over it: locating a craft, naming a section, and
//! which samples a lap actually drives.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`; `split_track` is only read here.

use super::*;

#[test]
fn the_locator_finds_the_nearest_sample() {
    let spline = Spline::from_track(&straight_track());
    assert!(!spline.is_empty());
    assert_eq!(spline.len(), 8 * Spline::STEPS_PER_SEGMENT);
    assert_eq!(spline.path_of(0), Some(0));

    let start = *spline.start().expect("a first sample");
    let at = Vec3::from_array(start.pos);
    assert!(spline.distance_to(at).expect("a distance") < 1e-3);

    // Straight up from the first sample is still nearest that sample.
    let (index, _, distance) = spline.nearest(at + Vec3::Y * 5.0).expect("a sample");
    assert_eq!(index, 0);
    assert!((distance - 5.0).abs() < 1e-3, "{distance}");
}

#[test]
fn the_widest_half_width_comes_from_the_track() {
    let spline = Spline::from_track(&straight_track());
    assert!((spline.max_half_width() - 12.0).abs() < 1e-3);
}

/// The shape of `05_Track`, `14_Track` and `07_Track`: paths 0 and 1 are the
/// two branches of a split and path 2 is the merge, so a lap drives 0 and 2
/// and never touches 1.
fn split_track() -> AiTrack {
    let base = straight_track();
    let path = base.paths[0].clone();
    AiTrack {
        version: base.version,
        paths: vec![
            oag_vex::track::Path {
                entry: Some(0),
                exit: Some(1),
                ..path.clone()
            },
            oag_vex::track::Path {
                entry: Some(0),
                exit: Some(1),
                ..path.clone()
            },
            oag_vex::track::Path {
                entry: Some(1),
                exit: Some(0),
                ..path
            },
        ],
        junctions: vec![
            oag_vex::track::Junction {
                prev: [Some(2), None],
                next: [Some(0), Some(1)],
            },
            oag_vex::track::Junction {
                prev: [Some(0), Some(1)],
                next: [Some(2), None],
            },
        ],
    }
}

#[test]
fn a_branch_the_lap_never_drives_is_not_in_the_ai_line() {
    let ai = split_track();
    let spline = Spline::from_track(&ai);
    let course = Course::from_track(&ai, None).expect("the split's chain closes");
    let order = ai_order(&spline, Some(&course));

    let per_path = spline.len() / 3;
    assert_eq!(
        order.len(),
        2 * per_path,
        "the alternate branch is still in the line"
    );
    assert!(
        order
            .iter()
            .all(|&index| spline.path_of(index as usize) != Some(1)),
        "a sample of the branch the lap never drives reached the line"
    );
    // And in travel order rather than file order: the merge follows the
    // branch it merges from.
    assert_eq!(spline.path_of(order[0] as usize), Some(0));
    assert_eq!(spline.path_of(order[order.len() - 1] as usize), Some(2));
}

/// The nine circuits where nothing changes, and the assertion that says so:
/// file order already is travel order, so the mapping is the identity and
/// every index means exactly what it meant before `ai_order` existed.
#[test]
fn a_track_whose_lap_drives_every_path_maps_a_line_index_onto_itself() {
    let ai = straight_track();
    let spline = Spline::from_track(&ai);
    let course = Course::from_track(&ai, None).expect("this fixture does close");
    let order = ai_order(&spline, Some(&course));
    assert_eq!(order, (0..spline.len() as u32).collect::<Vec<_>>());
}

/// A track with no closed ring still gets a line - every sample, in file
/// order, because nothing knows which way round is forward.
#[test]
fn a_track_with_no_ring_still_gets_every_sample_in_file_order() {
    let spline = Spline::from_track(&straight_track());
    let order = ai_order(&spline, None);
    assert_eq!(order, (0..spline.len() as u32).collect::<Vec<_>>());
}

/// The lookup a driver's index goes through, and the wrap that makes
/// "the next sample" a lap question rather than a table one.
///
/// **Asserted against `path_of` rather than against `ai_order`**, because
/// `ai_sample` *is* a lookup through `ai_order` and comparing the two would
/// assert a function equals its own definition - true on a mapping that had
/// dropped the wrong path entirely. Which path each index lands on is a
/// claim the mapping can fail.
#[test]
fn an_ai_index_reads_the_sample_the_lap_puts_there() {
    let ai = split_track();
    let mut race = Race::start(Setup {
        ai: ai.clone(),
        spline: Spline::from_track(&ai),
        course: Course::from_track(&ai, None),
        ..setup(hulled_handling())
    });
    race.sim.world.ships[0].active = true;

    let length = race.racing_line().len();
    assert!(length > 0);
    let walked: Vec<u16> = (0..length)
        .map(|ai_index| {
            let sample = race.sample_index_of(ai_index).expect("a sample index");
            race.sim.spline.path_of(sample).expect("a path")
        })
        .collect();
    assert!(
        !walked.contains(&1),
        "a line index reads the branch the lap never drives"
    );
    // Two contiguous runs, one per path the lap drives, in travel order.
    let handovers = walked.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert_eq!(handovers, 1, "the two paths are not contiguous: {walked:?}");
    assert_eq!((walked[0], walked[length - 1]), (0, 2));

    // One past the end is the start again, and the assertion is which
    // *path* it lands on: this fixture's three paths are clones of one
    // straight, so comparing positions would hold even on a wrap into the
    // branch the lap skips.
    assert_eq!(race.sample_index_of(length), race.sample_index_of(0));
}

/// A local search finds what a whole-table search would, when the answer is
/// inside the window.
#[test]
fn a_local_search_agrees_with_a_whole_table_one_inside_its_window() {
    let spline = Spline::from_track(&straight_track());
    let target = Vec3::from_array(spline.sample(20).expect("a sample").pos);
    let (whole, _, _) = spline.nearest(target).expect("a sample");
    let (local, _, distance) = spline
        .nearest_within(target, 20, 8)
        .expect("a sample in the window");
    assert_eq!(local, whole);
    assert!(distance < 1e-3, "{distance}");
}

/// And when the answer is outside the window it returns something too far
/// away, which the caller turns into "draw everything" rather than into a
/// confident wrong section. See `Spline::nearest_within`.
#[test]
fn a_local_search_that_misses_reports_a_distance_the_caller_will_reject() {
    let spline = Spline::from_track(&straight_track());
    let far_end = Vec3::from_array(spline.sample(30).expect("a sample").pos);
    let (index, _, distance) = spline
        .nearest_within(far_end, 0, 2)
        .expect("a sample in the window");
    assert!(index <= 2, "the search stayed inside its window");
    assert!(
        distance > 12.0 * OFF_TRACK_HALF_WIDTHS,
        "a miss must be far enough to be rejected, was {distance}"
    );
}

/// **The conservative path, which nothing else exercises.** A craft on the
/// racing line names its section; one that has left the track names
/// nothing, so the visible set becomes everything.
#[test]
fn a_position_off_the_track_refuses_to_name_a_section() {
    let race = Race::start(setup(Handling::default()));
    let spline = race.spline();
    let sample = *spline.sample(4).expect("a sample");
    let on_line = Vec3::from_array(sample.pos);

    assert_eq!(
        race.section_of(4, on_line),
        sample.section_id,
        "a craft on the racing line names its own section"
    );

    // The synthetic track is 12 units wide either side, so the gate is at
    // 36. Checked either side of it rather than at one distance, so the
    // test would fail if the threshold were dropped entirely.
    let half_width = 12.0 * OFF_TRACK_HALF_WIDTHS;
    let lateral = Vec3::from_array(sample.lateral);
    assert_eq!(
        race.section_of(4, on_line + lateral * (half_width - 1.0)),
        sample.section_id,
        "still on the track at just under the gate"
    );
    assert_eq!(
        race.section_of(4, on_line + lateral * (half_width + 1.0)),
        UNPLACED,
        "just past the gate, nothing may be culled on this position's word"
    );
    assert_eq!(
        race.section_of(4, on_line + Vec3::Y * 200.0),
        UNPLACED,
        "far above the track - airborne over a gap, or fallen through"
    );
    assert_eq!(
        race.section_of(9_999, on_line),
        UNPLACED,
        "a sample index the table does not have"
    );
}
