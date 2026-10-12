//! What the track picker's survey says about files, without any game content:
//! the committed fixture, and files that are not tracks.

use oag_raceplay::foreign::survey::{Kind, listing};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../trackfile/tests/fixtures/oval.track.toml")
}

#[test]
fn the_committed_own_format_track_lists_by_its_title_and_loads() {
    let listed = listing(&fixture());
    assert_eq!(listed.kind, Kind::Own);
    assert_eq!(listed.title, "Fixture Octagon");
    assert_eq!(listed.id, "foreign:fixture-octagon");
    assert_eq!(listed.author, None, "neither format carries one");
    assert_eq!(listed.problem, None);
}

#[test]
fn files_that_are_not_tracks_are_listed_with_a_reason() {
    let dir = std::env::temp_dir().join(format!("oag-survey-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let junk = dir.join("junk.wtrk");
    std::fs::write(&junk, b"this is not a track").unwrap();
    let listed = listing(&junk);
    assert_eq!(listed.kind, Kind::Workshop);
    assert_eq!(
        listed.title, "junk",
        "the file's stem stands in for a title"
    );
    assert_eq!(listed.problem.as_deref(), Some("not a track file"));

    let torn = dir.join("torn.wtrk");
    let mut bytes = vec![26u8];
    bytes.extend_from_slice(b"Workshop Track (Version 1)");
    std::fs::write(&torn, bytes).unwrap();
    let problem = listing(&torn).problem.expect("damaged");
    assert_ne!(problem, "not a track file", "it opens like one: {problem}");
    assert!(
        problem.chars().count() <= 47,
        "short enough for a row: {problem}"
    );

    let manifest = dir.join("bad.track.toml");
    std::fs::write(&manifest, "this = [not toml").unwrap();
    let listed = listing(&manifest);
    assert_eq!(listed.kind, Kind::Own);
    assert!(listed.problem.is_some());

    let missing = listing(&dir.join("gone.wtrk"));
    assert!(
        missing
            .problem
            .as_deref()
            .is_some_and(|p| p.starts_with("unreadable"))
    );
    std::fs::remove_dir_all(dir).ok();
}
