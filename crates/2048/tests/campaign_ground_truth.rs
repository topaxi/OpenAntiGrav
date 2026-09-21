//! Reads the real `Data\xml\SP.xml` out of `PCSF00007`'s `data.psarc` and
//! checks it against the census `oag_tables::mjolnir`'s own module doc
//! comment records, plus `"2048 - Event 3"`'s specific fields - so a future
//! patch or region that reshapes the file is a loud failure here rather than
//! a doc comment nobody re-checks.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-2048 --run-ignored all \
//!     -E 'binary(campaign_ground_truth)'
//! ```

use oag_tables::mjolnir::campaign::{EventKind, typedef};

fn open() -> Option<oag_assets::Archives> {
    let source = oag_testdata::exact("data/extracted/vita/PCSF00007/base")?;
    oag_2048::open(&source.display().to_string()).ok()
}

fn sp_xml_document(archives: &mut oag_assets::Archives) -> oag_tables::mjolnir::Document {
    let bytes = archives
        .read_name(oag_2048::campaign::SP_XML)
        .unwrap_or_else(|e| panic!("reading {}: {e}", oag_2048::campaign::SP_XML));
    let text = String::from_utf8(bytes).expect("SP.xml is not valid UTF-8");
    oag_tables::mjolnir::parse(&text)
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn sp_xml_carries_288_instances_across_the_eight_typedefs_this_crate_measured() {
    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);

    assert_eq!(doc.instances.len(), 288);

    let counts = doc.typedef_counts();
    assert_eq!(
        counts,
        vec![
            (typedef::GAME_MODE_OBJECTIVE, 96),
            (typedef::RACE_A, 53),
            (typedef::RACE_B, 52),
            (typedef::ELIMINATION, 26),
            (typedef::SHIP_MODEL_DATA, 21),
            (typedef::WEAPON_SET_DEFINITION, 20),
            // Tied at 10; typedef_counts breaks ties by ascending id, and
            // 205052969 (TRACK_DEFINITION) sorts before 1018671239 (ZONE).
            (typedef::TRACK_DEFINITION, 10),
            (typedef::ZONE, 10),
        ],
        "typedef census drifted from oag_tables::mjolnir's own doc comment"
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn track_definition_names_the_same_ten_stems_the_tracks_plugin_does() {
    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);

    let mut stems: Vec<String> = oag_tables::mjolnir::campaign::tracks(&doc)
        .into_iter()
        .map(|t| t.track_name)
        .collect();
    stems.sort();

    assert_eq!(
        stems,
        vec![
            "altima",
            "arena",
            "bridge",
            "cathedral",
            "mall",
            "park",
            "sol",
            "square",
            "subway",
            "tower",
        ]
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn event_3_is_park_flash_two_laps() {
    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);

    let event = doc
        .instance_named("2048 - Event 3")
        .and_then(oag_tables::mjolnir::campaign::Event::from_instance)
        .expect("\"2048 - Event 3\" is not in SP.xml");

    assert_eq!(event.kind, EventKind::Race);
    assert_eq!(event.description.as_deref(), Some("2048_EVENT_3"));
    assert_eq!(event.speed_class, Some(1), "eClass 1 == Flash");
    assert_eq!(event.laps, Some(2));
    assert!(
        event.has_ghost_capacity,
        "\"2048 - Event 3\" is typedef::RACE_A on the real file"
    );

    let track = oag_tables::mjolnir::campaign::track_for(&doc, event.track.expect("no M_TRACKDEF"))
        .expect("M_TRACKDEF does not resolve inside SP.xml");
    assert_eq!(track.track_name, "park");
    assert_eq!(track.display_name, "METRO PARK");

    assert_eq!(
        oag_2048::campaign::event_class(&event),
        Some(oag_2048::campaign::EClass::Flash)
    );
    assert_eq!(oag_2048::campaign::engine_mode(&event), Some("single_race"));
    assert_eq!(
        oag_2048::campaign::track_vex_entry(&track),
        r"Data\art\published\environments\park\track.vex"
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_speed_lap_named_event_carries_laps_zero_and_no_other_race_event_does() {
    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);

    let events = oag_tables::mjolnir::campaign::events(&doc);
    let (zero_laps, nonzero_laps): (Vec<_>, Vec<_>) = events
        .iter()
        .filter(|e| e.kind == EventKind::Race)
        .partition(|e| e.laps == Some(0));

    assert_eq!(
        zero_laps.len(),
        40,
        "one Speed Lap event per track per class, minus Venom"
    );
    assert!(
        zero_laps.iter().all(|e| e.name.contains("Speed Lap")),
        "every laps==0 race event should be a Speed Lap by name"
    );
    assert!(
        nonzero_laps.iter().all(|e| !e.name.contains("Speed Lap")),
        "no Speed Lap-named event should carry a nonzero lap count"
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn an_elimination_event_has_no_engine_mode() {
    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);

    let elimination = oag_tables::mjolnir::campaign::events(&doc)
        .into_iter()
        .find(|e| e.kind == EventKind::Elimination)
        .expect("SP.xml carries no Elimination event to test against");

    assert_eq!(oag_2048::campaign::engine_mode(&elimination), None);
}
