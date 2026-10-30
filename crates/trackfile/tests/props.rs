//! Author-defined properties and trigger volumes (`oag_trackfile::props`): a track
//! carrying them on its pads, on a trigger and on itself writes and reads back equal,
//! and what a mod could not read (a string, two names hashing alike, a list that does
//! not match its pads) is refused by name.

use oag_trackfile::props::{Props, Trigger};
use oag_trackfile::{Error, Track, fixture, from_parts, to_parts};

fn props(pairs: &[(&str, f32)]) -> Props {
    pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
}

/// The fixture with a second speed pad, properties on both, on the weapon pad, on
/// one trigger and on the track. 0.1 is not dyadic: a decimal detour would move it.
fn with_props() -> Track {
    let mut track = fixture::oval();
    let mut second = track.speedup_pads[0];
    second.to_world[12] += 40.0;
    track.speedup_pads.push(second);
    track.speedup_pad_props = vec![
        props(&[("boost", 1.5), ("lit", 1.0)]),
        props(&[("boost", 0.1)]),
    ];
    track.weapon_pad_props = vec![Props::new()];
    track.props = props(&[("laps", 3.0), ("gravity", 0.1)]);
    track.triggers = vec![Trigger {
        name: "tunnel".into(),
        to_world: second.to_world,
        min: [-20.0, -2.0, -5.0],
        max: [20.0, 12.0, 5.0],
        props: props(&[("dark", 0.1), ("gate", 2.0)]),
    }];
    track
}

#[test]
fn props_on_two_pads_a_trigger_and_the_track_round_trip_exactly() {
    let track = with_props();
    let parts = to_parts(&track, "t.glb").expect("writable");
    assert!(parts.manifest.contains("[props]"), "{}", parts.manifest);
    assert!(
        parts.manifest.contains("triggers = 1"),
        "{}",
        parts.manifest
    );
    let loaded = from_parts(&parts.manifest, &parts.glb).expect("reads");
    assert!(loaded.unread.is_empty(), "{:?}", loaded.unread);
    assert_eq!(loaded.track, track);
    assert_eq!(
        loaded.track.speedup_pad_props[1]["boost"].to_bits(),
        0.1_f32.to_bits()
    );
    oag_schema::validate::accepts("track", 1, &parts.manifest).expect("the schema takes [props]");
    // A second write of what was read is byte-identical.
    assert_eq!(to_parts(&loaded.track, "t.glb").unwrap(), parts);
}

#[test]
fn a_track_without_props_writes_none_of_their_keys() {
    let parts = to_parts(&fixture::oval(), "t.glb").unwrap();
    assert!(!parts.manifest.contains("props"), "{}", parts.manifest);
    assert!(!parts.manifest.contains("triggers"), "{}", parts.manifest);
    let json = String::from_utf8_lossy(&parts.glb);
    assert!(!json.contains("oag_props") && !json.contains("oag/triggers"));
}

#[test]
fn a_bool_reads_as_one_or_zero_and_a_string_is_refused_by_name() {
    let parts = to_parts(&with_props(), "t.glb").unwrap();
    let as_bool = parts.manifest.replace("laps = 3.0", "laps = true");
    let loaded = from_parts(&as_bool, &parts.glb).expect("a bool reads");
    assert_eq!(loaded.track.props["laps"], 1.0);
    let as_text = parts.manifest.replace("laps = 3.0", "laps = \"three\"");
    let err = from_parts(&as_text, &parts.glb).unwrap_err();
    assert!(
        matches!(&err, Error::Props { why, .. } if why.contains("\"laps\" is a string")),
        "{err}"
    );
    assert!(oag_schema::validate::accepts("track", 1, &as_text).is_err());
}

#[test]
fn what_a_mod_could_not_read_is_refused_naming_it() {
    assert_eq!(
        oag_ext_abi::track::prop_key("costarring"),
        oag_ext_abi::track::prop_key("liquid")
    );
    let mut clash = with_props();
    clash.triggers[0].props = props(&[("costarring", 1.0), ("liquid", 2.0)]);
    let err = to_parts(&clash, "t.glb").unwrap_err();
    assert!(
        matches!(&err, Error::Props { what, why } if what.contains("tunnel") && why.contains("\"costarring\" and \"liquid\"")),
        "{err}"
    );
    let mut short = with_props();
    short.speedup_pad_props.pop();
    assert!(matches!(
        to_parts(&short, "t.glb"),
        Err(Error::Props { .. })
    ));
    let mut empty = with_props();
    empty.props.insert(String::new(), 1.0);
    assert!(matches!(
        to_parts(&empty, "t.glb"),
        Err(Error::Props { .. })
    ));
    let mut crowded = with_props();
    crowded.triggers = vec![crowded.triggers[0].clone(); oag_ext_abi::track::MAX_TRIGGERS + 1];
    let err = to_parts(&crowded, "t.glb").unwrap_err();
    assert!(
        matches!(&err, Error::Props { why, .. } if why.contains("65 triggers")),
        "{err}"
    );
}
