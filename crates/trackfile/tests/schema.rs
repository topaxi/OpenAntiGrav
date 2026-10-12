//! `schemas/v1/track.tosd` and the manifest reader must give the same verdict on
//! the shape of a manifest.
//!
//! One deliberate difference: a key the reader does not know is read and named as
//! unread (a file from a later revision of the same v1 schema), where the closed
//! schema refuses it. `an_unknown_key_is_the_one_place_they_differ` pins that.

use oag_trackfile::{Parts, fixture, from_parts, to_parts};

fn parts() -> Parts {
    to_parts(&fixture::oval(), "oval.glb").expect("the fixture is writable")
}

fn cases(m: &str) -> Vec<(&'static str, String, bool)> {
    let cut = |from: &str, to: &str| m.replace(from, to);
    vec![
        ("as written", m.to_owned(), true),
        (
            "another major",
            cut("/v1/track.tosd", "/v2/track.tosd"),
            false,
        ),
        ("another kind", cut("track.tosd", "pilot.tosd"), false),
        (
            "no header",
            m.split_once("[track]")
                .map(|(_, rest)| format!("[track]{rest}"))
                .unwrap(),
            false,
        ),
        (
            "old format keys",
            cut(
                "[track]\n",
                "[track]\nformat = \"oag-track\"\nversion = \"1.1\"\n",
            )
            .replace("[toml-schema]", "[unused]"),
            false,
        ),
        (
            "no counts table",
            m.split_once("[counts]").map(|(a, _)| a.to_owned()).unwrap(),
            false,
        ),
        (
            "count not an integer",
            cut("speedup_pads = 1", "speedup_pads = \"1\""),
            false,
        ),
        (
            "count negative",
            cut("speedup_pads = 1", "speedup_pads = -1"),
            false,
        ),
        (
            "empty id",
            cut("id = \"fixture-octagon\"", "id = \"\""),
            false,
        ),
        (
            "title not a string",
            cut("title = \"Fixture Octagon\"", "title = 4"),
            false,
        ),
        (
            "junction pair of three",
            cut("prev = [", "prev = [0, "),
            false,
        ),
    ]
}

#[test]
fn the_schema_and_the_reader_agree_on_every_example() {
    let p = parts();
    for (what, text, accepted) in cases(&p.manifest) {
        let schema = oag_schema::validate::accepts("track", 1, &text);
        let reader = from_parts(&text, &p.glb);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(
            reader.is_ok(),
            accepted,
            "reader, {what}: {:?}",
            reader.err()
        );
    }
}

#[test]
fn an_unknown_key_is_the_one_place_they_differ() {
    let p = parts();
    let newer = p.manifest.replace("[spline]", "future_key = 3\n\n[spline]");
    assert!(oag_schema::validate::accepts("track", 1, &newer).is_err());
    let loaded = from_parts(&newer, &p.glb).expect("the reader tolerates it");
    assert!(loaded.unread.iter().any(|u| u.contains("future_key")));
}

#[test]
fn the_committed_fixture_is_valid_and_reads() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let text = std::fs::read_to_string(format!("{dir}/oval.track.toml")).unwrap();
    oag_schema::validate::accepts("track", 1, &text).expect("schema");
    let glb = std::fs::read(format!("{dir}/oval.glb")).unwrap();
    from_parts(&text, &glb).expect("reader");
}
