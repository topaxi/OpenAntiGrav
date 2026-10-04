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
fn an_elimination_event_maps_to_the_eliminator_mode() {
    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);

    let elimination = oag_tables::mjolnir::campaign::events(&doc)
        .into_iter()
        .find(|e| e.kind == EventKind::Elimination)
        .expect("SP.xml carries no Elimination event to test against");

    // Not "unsupported": `oag_race::Mode::Eliminator` is a real, simulated
    // mode and `--mode eliminator` loads end to end on this title (see
    // `oag_2048::campaign::engine_mode`'s own doc comment for the command
    // that proved it) - the CLI help text omitting the token from its list
    // was a stale string, not a restriction.
    assert_eq!(
        oag_2048::campaign::engine_mode(&elimination),
        Some("eliminator")
    );
}

/// [`oag_tables::mjolnir::campaign::WeaponSet::allowed_weapons`] against a
/// handful of the real file's 20 named sets - see
/// `docs/formats/2048-campaign.md`'s "The weapon set gate" section for the
/// full 20-set census this reading was pinned against, and
/// `docs/ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md` for
/// `WeaponType`'s own declaration this now decodes all eleven bits from.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_named_weapon_set_decodes_to_what_its_own_name_says() {
    use oag_tables::weapons::Weapon;

    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);
    let sets = oag_tables::mjolnir::campaign::weapon_sets(&doc);
    assert_eq!(sets.len(), 20, "SP.xml's own WeaponSetDefinition census");

    let named = |name: &str| {
        sets.iter()
            .find(|set| set.name == name)
            .unwrap_or_else(|| panic!("{name:?} names no WeaponSetDefinition in SP.xml"))
    };

    assert_eq!(
        named("Rockets Only").allowed_weapons(),
        vec![Weapon::Rocket]
    );
    assert_eq!(
        named("Leech Beam Only").allowed_weapons(),
        vec![Weapon::LeachBeam]
    );
    // Bits 8 (`Bomb`) and 9 (`Mine`) together, in `WeaponType`'s own
    // declaration order - see `WeaponSet::allowed_weapons`'s own doc comment.
    assert_eq!(
        named("Mines Only").allowed_weapons(),
        vec![Weapon::Bomb, Weapon::Mine]
    );
    assert_eq!(
        named("Cannons, Missile, Plasma").allowed_weapons(),
        vec![Weapon::Missile, Weapon::Cannon, Weapon::Plasma]
    );
    // 1023 sets bit 4 (`Shield`) alongside every other bit below LeachBeam's
    // (10) - the discriminating case against the prior reading, which left
    // bit 4 out entirely.
    assert_eq!(
        named("DemoWeapons").allowed_weapons(),
        vec![
            Weapon::Rocket,
            Weapon::Missile,
            Weapon::Quake,
            Weapon::Turbo,
            Weapon::Shield,
            Weapon::Cannon,
            Weapon::Autopilot,
            Weapon::Plasma,
            Weapon::Bomb,
            Weapon::Mine,
        ]
    );
    // 1959 = every bit except Turbo (3), Shield (4) and Autopilot (6).
    assert_eq!(
        named("EliminatorWeapons").allowed_weapons(),
        vec![
            Weapon::Rocket,
            Weapon::Missile,
            Weapon::Quake,
            Weapon::Cannon,
            Weapon::Plasma,
            Weapon::Bomb,
            Weapon::Mine,
            Weapon::LeachBeam,
        ]
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_trophy_page_is_named_for_events_that_exist_and_every_cup_shape_has_a_year() {
    use oag_2048::campaign::trophy;
    let Some(mut archives) = open() else {
        return;
    };
    let doc = sp_xml_document(&mut archives);
    let events = oag_tables::mjolnir::campaign::events(&doc);
    for name in trophy::NAMED {
        assert!(
            events.iter().any(|event| event.name == name),
            "{name} is not an SP.xml event"
        );
    }
    let mut with_page = 0;
    let mut with_art = 0;
    for event in &events {
        let shape = doc
            .instance(event.instance_id)
            .and_then(|instance| instance.field("M_BUTTONSHAPE"))
            .and_then(oag_tables::mjolnir::Field::int);
        if trophy::has_page(shape) {
            with_page += 1;
        }
        if trophy::art_for(&event.name, shape).is_some() {
            if matches!(shape, Some(9..=11)) {
                eprintln!("cup: {}", event.name);
            }
            with_art += 1;
            assert!(trophy::has_page(shape) || trophy::NAMED.contains(&event.name.as_str()));
        }
    }
    assert_eq!(
        (with_page, with_art),
        (12, 12),
        "nine named elite trophies and three cups, each with art"
    );
}
