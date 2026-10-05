//! Where the Repulser's third wave can start: `Course::branches` on every
//! circuit the disc ships.
//!
//! `Repulser_AdvanceWave` (`0x08876914`) forks a third wave at a junction with
//! an alternate path (`docs/ghidra/functions/psp-pulse-usa/repulser.md`, "The
//! fork wave, read"). A circuit with no such junction never forks, so its
//! Repulser hashes exactly as it did before the fork existed.

use std::path::PathBuf;

use oag_raceplay as race;
use oag_raceplay::catalogue;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// Every branch's two ends are on the ring and it holds samples, and the
/// circuits that carry one are exactly the ones listed.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn only_the_split_circuits_carry_a_branch() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let mut with = Vec::new();
    let mut checked = 0;
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
        let Some(course) = &loaded.setup.course else {
            continue;
        };
        checked += 1;
        for branch in course.branches() {
            assert!(!branch.is_empty(), "{}", track.id);
            assert!(branch.split < course.len() && branch.merge < course.len());
            println!(
                "{}: branch of {} samples, split at ring {}, merge at ring {} of {}",
                track.id,
                branch.len(),
                branch.split,
                branch.merge,
                course.len()
            );
        }
        if !course.branches().is_empty() {
            with.push(track.id.clone());
        }
    }
    assert!(checked > 0, "no circuit loaded");
    println!("circuits with a branch: {with:?} of {checked}");
    assert_eq!(with, EXPECTED, "the circuits with a split moved");
}

/// Read off this test's own run, 2026-10-04. `05_Track`'s branch is longer
/// than the primary path it runs beside (1,032 samples against 1,016); the
/// other three match theirs exactly.
const EXPECTED: [&str; 4] = ["05_Track", "14_Track", "07_Track", "23_Track"];
