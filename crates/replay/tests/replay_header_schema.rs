//! A replay's `HEAD` chunk against `schemas/v1/replay-header.tosd` and against the loader
//! (ADR-0064 point 7, ADR-0087 decision 8): what the writer emits is accepted by both, and each
//! way a header can be wrong is refused by the one that should refuse it.
//!
//! The two do not agree everywhere, on purpose. The loader tolerates a key it does not know (a
//! file a later build wrote within this major) and a missing `toml-schema` line (a file written
//! before it existed); the schema is closed and states the format as written today. A row that
//! shows the asymmetry says so.

use oag_core::StateHasher;
use oag_replay::event::{Event, Kind};
use oag_replay::header::{Fork, GhostInfo, Setup};
use oag_replay::{Header, ReadError, Replay};
use oag_schema::validate::accepts;

const KIND: &str = "replay-header";

fn populated() -> Replay {
    let mut header = Header::new(
        "Wipeout Pulse",
        "Data\\Track\\track.vex",
        "time_trial",
        "Venom",
        "feisar",
        7,
    );
    header.build = "abc123".into();
    header.options.insert("scheme".into(), "classic".into());
    header
        .options
        .insert("pilot_assist".into(), "normal".into());
    header.ghost = Some(GhostInfo {
        lap: 2,
        start_tick: 600,
        lap_ticks: 3_000,
    });
    header.setup = Some(Setup {
        mode: "time_trial".into(),
        class: "Venom".into(),
        track: Some("Data\\Track\\track.vex".into()),
        team: Some("feisar".into()),
        hull_variant: Some("alt".into()),
        skin: Some("Eliminator".into()),
        opponent_teams: vec!["auricom".into(), "qirex".into()],
        opponents: true,
        no_driver_for_class: false,
        people: 1,
        views: 1,
        laps_override: Some(3),
        weapons_override: Some(true),
        eliminator_kill_target: Some(10),
        eliminator_score_target: Some(500),
        add_on_campaign: Some("DLC1".into()),
        zone_model: true,
    });
    header.fork = Some(Fork {
        tick: 4_312,
        offered_at: 4_300,
        seats: vec![0],
        parent_build: "def456".into(),
        parent_hash: "0x0123456789abcdef".into(),
    });
    let mut recorder = oag_replay::Recorder::new(header, 7);
    recorder.event(0, Kind::PilotAssist, 1);
    recorder.finish()
}

fn chunks(bytes: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 8;
    while at < bytes.len() {
        let tag: [u8; 4] = bytes[at..at + 4].try_into().unwrap();
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        out.push((tag, bytes[at + 8..at + 8 + len].to_vec()));
        at += 8 + len;
    }
    out
}

/// The `HEAD` text of a written file.
fn head(bytes: &[u8]) -> String {
    let (_, text) = chunks(bytes)
        .into_iter()
        .find(|(tag, _)| tag == b"HEAD")
        .unwrap();
    String::from_utf8(text).unwrap()
}

/// `bytes` with its `HEAD` text replaced, the checksum rewritten to match.
fn with_head(bytes: &[u8], text: &str) -> Vec<u8> {
    let mut out = bytes[..8].to_vec();
    for (tag, payload) in chunks(bytes) {
        if &tag == b"END_" {
            break;
        }
        let payload = if &tag == b"HEAD" {
            text.as_bytes().to_vec()
        } else {
            payload
        };
        out.extend_from_slice(&tag);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&payload);
    }
    let mut hasher = StateHasher::new();
    hasher.write(&out);
    let sum = hasher.finish();
    out.extend_from_slice(b"END_");
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&sum.to_le_bytes());
    out
}

/// What the loader and the schema each say of `text` as a file's header.
fn verdicts(text: &str) -> (Result<bool, ReadError>, Result<(), Vec<String>>) {
    let bytes = with_head(&populated().to_bytes(), text);
    (
        Replay::read(&bytes).map(|read| read.schema_header),
        accepts(KIND, 1, text),
    )
}

#[test]
fn what_the_writer_emits_is_accepted_by_both() {
    let text = head(&populated().to_bytes());
    assert!(
        text.starts_with("toml-schema = {"),
        "the schema line is the first line: {text}"
    );
    let (loader, schema) = verdicts(&text);
    assert_eq!(loader.expect("the loader reads it"), true);
    schema.unwrap_or_else(|e| panic!("{e:#?}\n{text}"));
}

#[test]
fn a_header_with_nothing_optional_is_accepted_by_both() {
    let mut replay = populated();
    replay.header.setup = None;
    replay.header.fork = None;
    replay.header.ghost = None;
    replay.header.options.clear();
    let text = head(&replay.to_bytes());
    let (loader, schema) = verdicts(&text);
    assert_eq!(loader.expect("the loader reads it"), true);
    schema.unwrap_or_else(|e| panic!("{e:#?}\n{text}"));
}

#[test]
fn a_header_of_another_kind_is_refused_by_both() {
    let text = head(&populated().to_bytes()).replace("/v1/replay-header.tosd", "/v1/pilot.tosd");
    let (loader, schema) = verdicts(&text);
    match loader {
        Err(ReadError::Schema(oag_schema::Error::WrongKind { found, expected })) => {
            assert_eq!((found.as_str(), expected.as_str()), ("pilot", KIND));
        }
        other => panic!("{other:?}"),
    }
    assert!(schema.is_err(), "the schema's location pattern refuses it");
}

#[test]
fn a_later_major_is_refused_by_both() {
    let text =
        head(&populated().to_bytes()).replace("/v1/replay-header.tosd", "/v2/replay-header.tosd");
    let (loader, schema) = verdicts(&text);
    match loader {
        Err(ReadError::Schema(oag_schema::Error::UnknownMajor {
            found, supported, ..
        })) => {
            assert_eq!((found, supported), (2, 1));
        }
        other => panic!("{other:?}"),
    }
    assert!(schema.is_err());
}

#[test]
fn a_header_with_no_schema_line_loads_and_says_so() {
    let text = head(&populated().to_bytes());
    let bare: String = text.lines().skip(1).collect::<Vec<_>>().join("\n");
    let (loader, schema) = verdicts(&bare);
    assert_eq!(loader.expect("an old file still loads"), false);
    schema.expect("the schema's line is optional, as every schema's is");
}

#[test]
fn a_key_a_later_build_added_loads_but_the_closed_schema_refuses_it() {
    let text = format!("{}\nadded_later = 1\n", head(&populated().to_bytes()));
    // Appended after the last table, it would belong to that table: put it on top.
    let top = text.lines().next().unwrap().to_string();
    let rest: String = text.lines().skip(1).collect::<Vec<_>>().join("\n");
    let text = format!("{top}\nadded_later = 1\n{rest}\n");
    let (loader, schema) = verdicts(&text);
    assert_eq!(loader.expect("the reader tolerates an unknown key"), true);
    assert!(schema.is_err(), "the schema is closed");
}

#[test]
fn options_is_an_open_table_of_strings() {
    let mut replay = populated();
    replay
        .header
        .options
        .insert("a key nobody listed".into(), "any value".into());
    let text = head(&replay.to_bytes());
    let (loader, schema) = verdicts(&text);
    loader.expect("loads");
    schema.unwrap_or_else(|e| panic!("{e:#?}"));
    let integer = text.replace("scheme = \"classic\"", "scheme = 3");
    assert!(
        accepts(KIND, 1, &integer).is_err(),
        "a value that is not text is refused"
    );
}

#[test]
fn a_parent_hash_that_is_not_hex_loads_but_the_schema_refuses_it() {
    let text = head(&populated().to_bytes()).replace("0x0123456789abcdef", "not a hash");
    let (loader, schema) = verdicts(&text);
    loader.expect("the loader keeps the text");
    assert!(schema.is_err());
}

#[test]
fn events_and_the_header_survive_a_round_trip() {
    let replay = populated();
    let read = Replay::from_bytes(&replay.to_bytes()).expect("reads");
    assert_eq!(read, replay);
    assert_eq!(read.events, vec![Event::new(0, 0, Kind::PilotAssist, 1)]);
}
