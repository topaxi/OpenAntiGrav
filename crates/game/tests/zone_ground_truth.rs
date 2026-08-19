//! Zone flies its own environment, and every title keeps that environment
//! somewhere different.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(zone_ground_truth)'
//! ```
//!
//! # What this file is the evidence for
//!
//! [`oag_title::ZoneCircuit`], and the claim underneath it: **Zone's look is
//! authored, not computed.** A Zone circuit is a whole separate `.vex` with its
//! own meshes, its own lights, its own `fogCube` and its own one-material
//! `Skycube`, where a race circuit's sky has five or six materials. Nothing in
//! this engine tints, desaturates or otherwise invents the Zone aesthetic,
//! because the disc already carries it - `CLAUDE.md`'s rule about not authoring
//! a stand-in for what the data holds.
//!
//! **Where that file lives is what diverges**, and it diverges in *shape* rather
//! than in spelling, which is what earns it a type instead of a constant:
//!
//! | Title | Zone's environment |
//! | --- | --- |
//! | Pulse | the race circuit's own directory, one extra `zone_`-prefixed file |
//! | Pure | `Data\Zone\NN_Zone\track.vex`, four circuits of its own |
//! | HD / Fury | `/data/environments/zone_N/track.vex`, four circuits of its own |
//!
//! Three titles, two shapes, no hole - so neither variant of the enum is
//! designed from one example, which is the bar [ADR-0022] sets.
//!
//! # What is deliberately not asserted
//!
//! **No lap time and no scoring.** The zone step, the score and the recharge are
//! `crates/race/src/zone.rs`'s and are tested there against
//! `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`. This file is about which
//! *file* a Zone race opens, and stops there.
//!
//! **Nothing about which circuit a player can reach.** Pure gates three of its
//! four Zone circuits behind medals and Pulse's own Zone menu has not been read;
//! that is progression, which is M7.
//!
//! [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md

use oag_game::{catalogue, race};
use oag_physics::SpeedClass;
use oag_title::ZoneCircuit;
use std::path::{Path, PathBuf};

/// One image, or `None` with a printed reason when it is not present.
///
/// `OAG_REQUIRE_GAME_DATA` turns absence into a failure, which is what
/// `just test-data` on a populated checkout wants: a skipped assertion and a
/// passing one read identically in the summary otherwise.
fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Every `PI_Track` a source declares as raceable, off the disc's own plugin
/// definition.
///
/// Read rather than listed, for the reason `oag_game::catalogue` gives: circuit
/// names in this repository would be shipped content.
fn race_circuits(archives: &mut oag_assets::Archives, definition: &str) -> Vec<catalogue::Track> {
    let blob = archives
        .read_name(definition)
        .expect("the game plugin definition");
    let xml = oag_formats::fexml::text(&blob).expect("the definition is not shortened");
    catalogue::tracks(&xml)
}

/// The same, for the circuits a Zone race can be run on.
///
/// **The two arrangements, dispatched here rather than inside the catalogue.**
/// A title whose Zone circuits are the race ones with a prefixed file marks them
/// with `availableInZone`; a title whose Zone circuits are its own declares them
/// `type="Zone"`, which the race listing skips. `oag_game::catalogue` offers the
/// two listings and deliberately does not choose between them - that choice
/// needs the title in hand, so it belongs to a caller like this one.
fn zone_circuits(
    archives: &mut oag_assets::Archives,
    definition: &str,
    zone: ZoneCircuit,
) -> Vec<catalogue::Track> {
    let blob = archives
        .read_name(definition)
        .expect("the game plugin definition");
    let xml = oag_formats::fexml::text(&blob).expect("the definition is not shortened");
    match zone {
        ZoneCircuit::Prefixed(_) => catalogue::tracks(&xml)
            .into_iter()
            .filter(|track| track.available_in_zone)
            .collect(),
        ZoneCircuit::Separate(_) => catalogue::tracks_of_kind(&xml, "Zone"),
    }
}

/// **The load-correctness claim, swept over every circuit Pulse declares.**
///
/// `availableInZone="true"` is not decoration and not menu data: on this title
/// the Zone environment lives beside the race one, and a circuit without the
/// attribute has no such file at all. So a Zone race on one of the eight that do
/// not declare it asks the archive for a name that hashes to nothing.
///
/// The assertion is the biconditional, both directions, on all 24 entries. A
/// one-directional check would pass on a build that simply never found any zone
/// file.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_available_in_zone_predicts_the_zone_file_on_every_circuit() {
    for name in [
        "data/images/pulse-psp-usa.chd",
        "data/images/pulse-psp-eu.chd",
        "data/images/pulse-ps2-eu.chd",
    ] {
        let Some(path) = image(name) else { continue };
        let mut archives = oag_pulse::open(&path.display().to_string()).expect("opening Pulse");
        let zone = oag_pulse::race::DEFAULTS.zone;
        let circuits = race_circuits(&mut archives, oag_pulse::names::GAME_PLUGIN_DEFINITION);
        assert!(!circuits.is_empty(), "{name}: the disc declares circuits");

        let (mut declared, mut present) = (0, 0);
        for circuit in &circuits {
            let variant = zone.variant_of(&circuit.entry_name());
            assert_ne!(
                variant,
                circuit.entry_name(),
                "{name}: {} - the zone name should differ from the race one",
                circuit.id
            );
            let on_disc = archives.locate(&variant).is_some();
            assert_eq!(
                circuit.available_in_zone,
                on_disc,
                "{name}: {} declares availableInZone={} but {variant} is {}. The \
                 attribute and the file must agree - if they ever do not, the \
                 attribute is not what selects the zone environment and \
                 oag_title::ZoneCircuit is reading the disc wrong",
                circuit.id,
                circuit.available_in_zone,
                if on_disc { "present" } else { "absent" }
            );
            declared += usize::from(circuit.available_in_zone);
            present += usize::from(on_disc);
        }

        // Both counts, so a build where *nothing* resolved cannot pass the
        // biconditional above by agreeing at zero.
        assert!(
            declared > 0 && declared < circuits.len(),
            "{name}: {declared} of {} circuits are zone-available; a run where all \
             or none were would mean the attribute is not selecting anything",
            circuits.len()
        );
        assert_eq!(declared, present);
        println!(
            "{name}: {declared} of {} circuits carry a zone environment",
            circuits.len()
        );
    }
}

/// **Every circuit `zone_tracks` offers actually loads**, geometry and all.
///
/// The listing and the load are separate claims: a name that resolves in the
/// archive directory can still be a blob the `.vex` decoder cannot account for,
/// and `docs/formats/skycube.md` counts the Zone skies as a different shape from
/// the race ones, so it is worth knowing they decode through the same path.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_zone_circuit_pulse_offers_loads_as_a_zone_race() {
    let Some(path) = image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = path.display().to_string();
    let mut archives = oag_pulse::open(&source).expect("opening Pulse");
    let zone = oag_pulse::race::DEFAULTS.zone;
    let offered = zone_circuits(
        &mut archives,
        oag_pulse::names::GAME_PLUGIN_DEFINITION,
        zone,
    );
    assert_eq!(
        offered.len(),
        16,
        "Pulse declares sixteen zone-available circuits; if this changes, the \
         disc changed or the attribute is being read differently"
    );

    // Circuits whose zone spline has a different point count from their race
    // one, as `(id, race, zone)`. See the comment on the comparison below.
    let mut moved: Vec<(String, usize, usize)> = Vec::new();

    for circuit in &offered {
        let loaded = race::load(&race::Options {
            source: source.clone(),
            class: SpeedClass::Venom,
            mode: oag_race::Mode::Zone,
            track: Some(circuit.entry_name()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {} as a zone race: {e:#}", circuit.id));

        // **How far the racing line moves, measured rather than assumed.**
        // `Course::START_LINE_OFFSET` is fitted on `16_Track`'s *race* file (see
        // HANDOVER), and a zone circuit that splines differently inherits a fit
        // that was never made for it. This started life as an equality
        // assertion because `16_Track` splines identically in both - 862 points
        // either way - and that generalised wrongly: `10_Track` does not, 848
        // against 844. So the spread is printed and the count of circuits that
        // move is asserted to be a minority, which is a claim the data supports;
        // an equality here would have been a claim it does not.
        let racing = race::load(&race::Options {
            source: source.clone(),
            class: SpeedClass::Venom,
            mode: oag_race::Mode::TimeTrial,
            track: Some(circuit.entry_name()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {} as a time trial: {e:#}", circuit.id));
        let (zone_points, race_points) =
            (loaded.setup.ai.point_count(), racing.setup.ai.point_count());
        if zone_points != race_points {
            moved.push((circuit.id.clone(), race_points, zone_points));
        }
        assert_ne!(
            loaded.track_model.indices.len(),
            racing.track_model.indices.len(),
            "{}: zone and race decoded identical geometry, so the swap did not happen",
            circuit.id
        );

        // The load resolved the *zone* file, not the race one it was handed.
        let expected = zone.variant_of(&circuit.entry_name());
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.starts_with("zone:") && line.contains(&expected)),
            "{}: the report should name the zone circuit it swapped to; was {:#?}",
            circuit.id,
            loaded.report
        );
        assert!(
            loaded.track_model.indices.len() > 3,
            "{}: the zone circuit decoded to no geometry",
            circuit.id
        );
        assert!(
            loaded.sky_model.is_some(),
            "{}: the zone circuit authors a Skycube and it should have decoded - \
             the zone skies are the one-material ones in docs/formats/skycube.md",
            circuit.id
        );
    }

    for (id, race_points, zone_points) in &moved {
        println!("{id}: spline race {race_points} point(s) / zone {zone_points}");
    }
    println!(
        "{} of {} zone circuits spline differently from their race twin",
        moved.len(),
        offered.len()
    );
    assert!(
        moved.len() * 2 < offered.len(),
        "most zone circuits now spline differently from their race twin ({} of \
         {}), which would mean the racing line is authored per environment \
         rather than shared. Course::START_LINE_OFFSET is fitted on 16_Track's \
         race file, so that fit would no longer carry - see HANDOVER",
        moved.len(),
        offered.len()
    );
}

/// **A Zone race and a race on the same circuit are different environments.**
///
/// The point of the whole change, stated as a difference rather than as a
/// presence: loading `16_Track` in Zone and in Time Trial must not produce the
/// same geometry, or the swap is not happening.
///
/// **The spline is compared too, and deliberately not asserted equal.**
/// `Course::START_LINE_OFFSET` is fitted on `16_Track`'s *race* file, so if the
/// zone variant splines differently, Zone lap timing inherits a fit that was
/// never made for it. Whichever way it comes out is printed, and the inequality
/// that is asserted is the geometry one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_zone_race_and_a_time_trial_load_different_environments() {
    let Some(path) = image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = path.display().to_string();
    let race_track = oag_pulse::race::DEFAULT_TRACK;

    let load_as = |mode| {
        race::load(&race::Options {
            source: source.clone(),
            class: SpeedClass::Venom,
            mode,
            track: Some(race_track.to_string()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {race_track} as {mode:?}: {e:#}"))
    };

    let racing = load_as(oag_race::Mode::TimeTrial);
    let zoning = load_as(oag_race::Mode::Zone);

    assert_ne!(
        racing.track_model.indices.len(),
        zoning.track_model.indices.len(),
        "the same circuit in zone and in time trial decoded identical geometry, so \
         the zone environment was never loaded"
    );
    println!(
        "16_Track: race {} indices / zone {} indices",
        racing.track_model.indices.len(),
        zoning.track_model.indices.len()
    );
    println!(
        "16_Track spline: race {} point(s) / zone {} point(s) - if these differ, \
         Course::START_LINE_OFFSET was fitted on the race file and zone lap \
         timing inherits a fit never made for it",
        racing.setup.ai.point_count(),
        zoning.setup.ai.point_count()
    );
}

/// **Pure keeps Zone somewhere else entirely**, and this is the assertion that
/// makes `ZoneCircuit` two variants rather than one string.
///
/// Four circuits declared `type="Zone"` under `Data\Zone\`, each holding a plain
/// `track.vex` - and **no `zone_track.vex` anywhere on the disc**, which is the
/// half that rules out "Pure spells Pulse's arrangement differently".
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_declares_zone_circuits_of_its_own_and_no_prefixed_file() {
    for name in [
        "data/images/pure-psp-eu.chd",
        "data/images/pure-psp-usa.chd",
    ] {
        let Some(path) = image(name) else { continue };
        let source = path.display().to_string();
        let mut archives = oag_pure::open(&source).expect("opening Pure");
        let zone = oag_pure::race::DEFAULTS.zone;
        assert!(
            matches!(zone, ZoneCircuit::Separate(_)),
            "{name}: Pure's zone circuits are its own"
        );

        let offered = zone_circuits(&mut archives, oag_pure::names::GAME_PLUGIN_DEFINITION, zone);
        assert_eq!(
            offered.len(),
            4,
            "{name}: Pure declares four type=\"Zone\" circuits; got {:?}",
            offered.iter().map(|t| &t.id).collect::<Vec<_>>()
        );
        for circuit in &offered {
            assert!(
                archives.locate(&circuit.entry_name()).is_some(),
                "{name}: {} declares {} and the disc should carry it",
                circuit.id,
                circuit.entry_name()
            );
        }
        assert!(
            offered
                .iter()
                .any(|c| c.entry_name() == oag_pure::race::DEFAULT_ZONE_TRACK),
            "{name}: the default zone circuit should be one the definition declares"
        );

        // The other half: Pulse's arrangement is absent, on race circuits and on
        // zone ones alike. Without this the four circuits above are consistent
        // with Pure carrying *both* arrangements.
        let prefixed = ZoneCircuit::Prefixed("zone_");
        for circuit in race_circuits(&mut archives, oag_pure::names::GAME_PLUGIN_DEFINITION)
            .iter()
            .chain(offered.iter())
        {
            let variant = prefixed.variant_of(&circuit.entry_name());
            assert!(
                archives.locate(&variant).is_none(),
                "{name}: {variant} resolves, so Pure does carry Pulse's prefixed \
                 arrangement after all and oag_pure::race's docs are wrong"
            );
            assert!(
                !circuit.available_in_zone,
                "{name}: {} declares availableInZone, which Pure was measured not \
                 to use",
                circuit.id
            );
        }
    }
}

/// **Pure's Zone craft is a team, not a model file** - the finding that closed
/// `oag_pure::race`'s "the Zone hull is unrecovered" item.
///
/// Two independent things are checked, because either alone would be weak: the
/// model resolves, and the directory's own `handlingstats.xml` says what the
/// team is *for*. The second is the disc naming Zone in its own words, which is
/// what lifts this above "a directory called Zone_01 exists".
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_zone_craft_is_a_team_the_definition_declares() {
    for name in [
        "data/images/pure-psp-eu.chd",
        "data/images/pure-psp-usa.chd",
    ] {
        let Some(path) = image(name) else { continue };
        let mut archives = oag_pure::open(&path.display().to_string()).expect("opening Pure");
        let team = oag_pure::race::ZONE_TEAM;

        let hull = format!(r"Data\Ships\{team}\Ship.vex");
        assert!(
            archives.locate(&hull).is_some(),
            "{name}: {hull} should be on the disc"
        );

        let stats = oag_formats::handling::entry_name(team);
        let blob = archives
            .read_name(&stats)
            .unwrap_or_else(|e| panic!("{name}: reading {stats}: {e}"));
        let xml = String::from_utf8_lossy(&blob);
        assert!(
            xml.contains("ZoneMode"),
            "{name}: {stats} should name the mode it is for - the disc writes \
             <Stats team=\"ZoneMode\">, which is what identifies this team as the \
             zone craft rather than a livery called Zone_01"
        );

        // And the neighbouring `PI_Team name="Zone"` is *not* this: it is
        // `type="Race"`, the unlockable livery. Confusing the two would put a
        // raceable team in a mode slot.
        assert_ne!(
            team, "Zone",
            "the zone-mode team is Zone_01; plain Zone is the unlockable livery"
        );
    }
}

/// **HD is shaped like Pure**, which is the third measurement and the reason
/// neither variant of the enum rests on one example.
///
/// Four `zone_N` environments, each holding a plain `track.vex`, and no prefixed
/// file beside any of the sixteen the manifest carries.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_keeps_four_zone_environments_of_its_own() {
    let Some(path) = image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let archives = oag_hd::open(&path.display().to_string()).expect("opening HD");
    let zone = oag_hd::race::DEFAULTS.zone;
    assert!(
        matches!(zone, ZoneCircuit::Separate(_)),
        "HD's zone circuits are its own"
    );
    assert!(
        archives.locate(oag_hd::race::DEFAULT_ZONE_TRACK).is_some(),
        "{} should be in the manifest",
        oag_hd::race::DEFAULT_ZONE_TRACK
    );

    let zone_environments: Vec<&&str> = oag_hd::names::ENVIRONMENTS
        .iter()
        .filter(|env| env.starts_with("zone_"))
        .collect();
    assert_eq!(
        zone_environments.len(),
        4,
        "HD ships four zone environments; got {zone_environments:?}"
    );

    let prefixed = ZoneCircuit::Prefixed("zone_");
    for environment in oag_hd::names::ENVIRONMENTS {
        let track = oag_hd::names::track(environment);
        assert!(
            archives.locate(&track).is_some(),
            "{track} should be in the manifest"
        );
        assert!(
            archives.locate(&prefixed.variant_of(&track)).is_none(),
            "{} resolves, so HD carries Pulse's prefixed arrangement too",
            prefixed.variant_of(&track)
        );
    }
}
