//! Where `Course::START_LINE_ADVANCE` puts the start line on every circuit.
//!
//! The rule is the original's own (`RaceManager_Construct`,
//! `docs/ghidra/functions/psp-pulse-usa/race-progress.md`): the authored grid
//! slot projected onto the spline, stepped 154 units along that sample's tangent
//! in a straight line, projected onto the spline again. The original bounds the
//! second projection to a 100-unit search; `Course::from_track` snaps to the
//! nearest ring point with no bound, which on a start straight that turns hard
//! inside 154 units could land the line across a hairpin instead of down the
//! road. Nothing else asserts where `start_index` went - the grid-layout tests
//! place craft off the spline, not off the course - so this does.
//!
//! The check is the arc the line sits ahead of the slot. A 154-unit chord along
//! the tangent covers a little *more* than 154 units of arc when the road
//! curves, and never much less; a wrong-side snap reads near zero or near the
//! whole lap.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_race::Course;
use oag_raceplay as race;
use oag_raceplay::catalogue;

/// The disc, or `None` on a checkout without one.
fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// How far ahead of the slot's ring point the line may sit, either way.
///
/// The lower bound is the chord itself less the ring's own sampling step; the
/// upper bound allows the start straight to curve. Both are generous on purpose:
/// the claim is "down the road, roughly a chord's length", not a geometry proof.
const ARC_MIN: f32 = 140.0;
const ARC_MAX: f32 = 200.0;

/// The start line lands a chord's length down the road from the slot, on every
/// circuit the disc ships, reversed ones included.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_start_line_is_a_chord_ahead_of_the_slot_on_every_circuit() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    let mut checked = 0;
    let mut worst: Option<(String, f32)> = None;
    for track in catalogue::tracks(&definition) {
        let Ok(loaded) = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(track.entry_name()),
            ..race::Options::default()
        }) else {
            continue;
        };
        let (Some(course), Some(slot)) = (&loaded.setup.course, &loaded.setup.start_position)
        else {
            continue;
        };
        let slot_point = course
            .locate(Vec3::from(slot.position), None)
            .expect("the slot is on the ring");
        // `progress` is measured from the line, so the slot's progress is how far
        // *behind* the line it is - the arc the line sits ahead of it.
        let ahead = course.length() - slot_point.progress;
        println!(
            "{}: line {ahead:.1} units ahead of the slot (ring point {} -> {}, {:.0}-unit lap)",
            track.id,
            slot_point.index,
            course.start_index(),
            course.length()
        );
        checked += 1;
        let excess = (ahead - Course::START_LINE_ADVANCE).abs();
        if worst.as_ref().is_none_or(|(_, w)| excess > *w) {
            worst = Some((track.id.clone(), excess));
        }
        assert!(
            (ARC_MIN..=ARC_MAX).contains(&ahead),
            "{}: the start line is {ahead:.1} units ahead of the slot along the ring; the \
             154-unit chord should put it between {ARC_MIN} and {ARC_MAX}. A value near 0 or \
             near {:.0} means the nearest-point snap landed across the track rather than down \
             it, and `Course::from_track` needs the original's 100-unit search bound.",
            track.id,
            course.length()
        );
    }
    assert!(checked >= 20, "only {checked} circuits were loadable");
    if let Some((id, excess)) = worst {
        println!("furthest from the chord length: {id}, by {excess:.1} units");
    }
}

/// The captured Talon's Junction spawn is a few units short of the line.
///
/// `crates/trace/tests/lap_capture_ground_truth.rs` measures the same thing off
/// the capture; this one pins the number the rule produces on the circuit the
/// old constant was fitted to, so a change in the ring's sampling that moved it
/// would show up here without a capture.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn talons_junction_puts_the_line_where_the_original_does() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("Talon's Junction loads");
    let course = loaded.setup.course.as_ref().expect("a course");
    let slot = loaded.setup.start_position.as_ref().expect("a slot");
    let slot_point = course
        .locate(Vec3::from(slot.position), None)
        .expect("the slot is on the ring");
    let ahead = course.length() - slot_point.progress;
    println!("16_Track: line {ahead:.1} units ahead of the slot");
    // 137.9 is the captured spawn's distance from the slot along the ring; the
    // line is ahead of the spawn by the 16.5 units the capture test measures.
    assert!(
        (150.0..=160.0).contains(&ahead),
        "16_Track's line moved to {ahead:.1} units ahead of the slot"
    );
}
