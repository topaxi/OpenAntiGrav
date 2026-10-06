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

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_raceplay as race;
use oag_raceplay::catalogue;
use oag_title::ZoneCircuit;
use std::path::PathBuf;

/// One image, or `None` with a printed reason when it is not present.
///
/// `OAG_REQUIRE_GAME_DATA` turns absence into a failure, which is what
/// `just test-data` on a populated checkout wants: a skipped assertion and a
/// passing one read identically in the summary otherwise.
fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `PI_Track` a source declares as raceable, off the disc's own plugin
/// definition.
///
/// Read rather than listed, for the reason `oag_raceplay::catalogue` gives: circuit
/// names in this repository would be shipped content.
fn race_circuits(archives: &mut oag_assets::Archives, definition: &str) -> Vec<catalogue::Track> {
    let blob = archives
        .read_name(definition)
        .expect("the game plugin definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition is not shortened");
    catalogue::tracks(&xml)
}

/// The same, for the circuits a Zone race can be run on.
///
/// **The two arrangements, dispatched through [`ZoneCircuit::menu_tracks`]
/// rather than by hand here.** A title whose Zone circuits are the race ones
/// with a prefixed file marks them with `availableInZone`; a title whose Zone
/// circuits are its own is asked by name instead, since which `type` those
/// entries carry is a per-title choice `catalogue::tracks_named` does not
/// need to know - Pure declares `type="Zone"`, HD/Fury `type="Race"` with a
/// `zone="true"` flag, and matching by name finds either. This used to make
/// that choice itself with a `match`; it is the production dispatch now, so
/// this sweep exercises the same code path a menu's CIRCUIT row does.
fn zone_circuits(
    archives: &mut oag_assets::Archives,
    definition: &str,
    zone: ZoneCircuit,
) -> Vec<catalogue::Track> {
    let blob = archives
        .read_name(definition)
        .expect("the game plugin definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition is not shortened");
    let race_tracks = catalogue::tracks(&xml);
    zone.menu_tracks(
        &race_tracks,
        |track| track.available_in_zone,
        |names| catalogue::tracks_named(&xml, names),
    )
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
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Zone,
            track: Some(circuit.entry_name()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {} as a zone race: {e:#}", circuit.id));

        // **How far the racing line moves, measured rather than assumed.**
        // The start line is derived from each file's own spline
        // (`Course::START_LINE_ADVANCE`), so a zone circuit that splines
        // differently gets its own line - this used to matter more when the
        // line was a constant fitted on `16_Track`'s race file. This started life as an equality
        // assertion because `16_Track` splines identically in both - 862 points
        // either way - and that generalised wrongly: `10_Track` does not, 848
        // against 844. So the spread is printed and the count of circuits that
        // move is asserted to be a minority, which is a claim the data supports;
        // an equality here would have been a claim it does not.
        let racing = race::load(&race::Options {
            source: source.clone(),
            class: "VENOM".to_string(),
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
         rather than shared - see HANDOVER",
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
/// A zone variant that splines differently gets its own start line from its
/// own spline (`Course::START_LINE_ADVANCE`). Whichever way it comes out is
/// printed, and the inequality that is asserted is the geometry one.
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
            class: "VENOM".to_string(),
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
        "16_Track spline: race {} point(s) / zone {} point(s)",
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
            matches!(zone, ZoneCircuit::Separate(..)),
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
        for track in oag_pure::race::ZONE_TRACKS {
            assert!(
                offered.iter().any(|c| c.entry_name() == *track),
                "{name}: {track} is in oag_pure::race::ZONE_TRACKS but not among the \
                 disc's own declared type=\"Zone\" circuits"
            );
        }

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

        let stats = oag_tables::handling::entry_name(team);
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
    let mut archives = oag_hd::open(&path.display().to_string()).expect("opening HD");
    let zone = oag_hd::race::DEFAULTS.zone;
    assert!(
        matches!(zone, ZoneCircuit::Separate(..)),
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

    // The other three of `oag_hd::race::ZONE_TRACKS`, and the pairing with
    // `25_Track`..`28_Track` those docs claim: file order in the plugin
    // definition names `zone_1`..`zone_4` in that order, which is what
    // `ZONE_TRACKS`'s own field order rests on.
    for track in oag_hd::race::ZONE_TRACKS {
        assert!(
            archives.locate(track).is_some(),
            "{track} should be in the manifest"
        );
    }
    assert_eq!(
        oag_hd::race::ZONE_TRACKS.len(),
        4,
        "HD's own zone track list should name all four environments"
    );

    // The regression this file exists to catch on HD specifically: its own
    // four are `type="Race"` with a `zone="true"` flag rather than Pure's
    // `type="Zone"`, so the menu's CIRCUIT row must be finding them by name
    // (`ZoneCircuit::Separate`'s own list), not by asking for a `type` that
    // never appears on this title's declarations.
    let offered = zone_circuits(
        &mut archives,
        oag_hd::names::FRONT_END_PLUGIN_DEFINITION,
        zone,
    );
    let own_count = 4;
    for track in oag_hd::race::ZONE_TRACKS {
        assert!(
            offered
                .iter()
                .any(|c| archives.locate(&c.entry_name()) == archives.locate(track)),
            "{track} is in oag_hd::race::ZONE_TRACKS but not among the tracks the menu \
             offers for Zone mode"
        );
    }
    assert!(
        offered[..own_count]
            .iter()
            .all(|c| oag_hd::race::ZONE_TRACKS
                .iter()
                .any(|t| archives.locate(&c.entry_name()) == archives.locate(t))),
        "HD's own four zone-exclusive circuits should come first; got {:?}",
        offered.iter().map(|t| &t.id).collect::<Vec<_>>()
    );

    // **Unverified, on `also_race_circuits` - see `oag_title::ZoneCircuit::Separate`'s
    // own docs.** A play-based lead (2026-08-28) says HD's Zone picker also
    // offers every ordinary race circuit, not just the four zone-exclusive
    // ones above; nothing in the disc's own data this codebase reads confirms
    // it, so this assertion documents the implemented behaviour rather than a
    // measured fact the way every other assertion in this file does.
    let race_tracks = race_circuits(&mut archives, oag_hd::names::FRONT_END_PLUGIN_DEFINITION);
    assert_eq!(
        offered.len(),
        race_tracks.len(),
        "HD's own four zone-exclusive circuits are also `type=\"Race\"` entries, so \
         appending the race list should add no new count beyond it: got {} offered \
         against {} race circuits",
        offered.len(),
        race_tracks.len()
    );
    for circuit in &race_tracks {
        assert!(
            offered
                .iter()
                .any(|c| archives.locate(&c.entry_name()) == archives.locate(&circuit.entry_name())),
            "{} is an ordinary race circuit but not offered for Zone mode",
            circuit.id
        );
    }
}

/// **The defect this whole file is now the regression test for, on Pure -
/// see [`zone_mode_on_hd_races_a_named_race_circuit_directly`] for the
/// unverified companion HD now takes instead.** A menu that switches to Zone
/// mode without touching the Circuit row hands `race::load` a *race*
/// circuit's entry name - and until this test existed, `ZoneCircuit::Separate`
/// passed it through unchanged, so a Zone race on Wipeout HD or Wipeout Pure
/// loaded the picked race circuit's own environment, wearing Zone's rules,
/// instead of any Zone environment at all.
///
/// Two loads: one naming a race circuit (must substitute the title's default
/// Zone circuit and say so in the report), one naming one of the title's own
/// Zone circuits directly (must load it unchanged).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn zone_mode_on_a_named_race_circuit_substitutes_a_zone_environment() {
    let cases: [(&str, &str, &str); 1] = [(
        "data/images/pure-psp-eu.chd",
        oag_pure::race::DEFAULT_TRACK,
        oag_pure::race::ZONE_TRACK_2,
    )];

    for (name, race_track, own_zone_track) in cases {
        let Some(path) = image(name) else { continue };
        let source = path.display().to_string();

        // The race circuit's own environment, so the substitution can be
        // proven to have actually swapped the geometry and not merely the
        // report line.
        let racing = race::load(&race::Options {
            source: source.clone(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::TimeTrial,
            track: Some(race_track.to_string()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("{name}: loading {race_track} as a time trial: {e:#}"));

        // The same circuit, named the same way a menu that has not touched
        // the Circuit row since picking Zone mode would - which is exactly
        // the shape of the bug.
        let zoning_the_race_circuit = race::load(&race::Options {
            source: source.clone(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Zone,
            track: Some(race_track.to_string()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("{name}: racing {race_track} in zone mode: {e:#}"));

        assert!(
            zoning_the_race_circuit
                .report
                .iter()
                .any(|line| line.starts_with("zone:") && line.contains("->")),
            "{name}: picking a race circuit in zone mode should report the \
             substitution to a zone environment: {:#?}",
            zoning_the_race_circuit.report
        );
        assert_ne!(
            racing.track_model.indices.len(),
            zoning_the_race_circuit.track_model.indices.len(),
            "{name}: racing {race_track} in zone mode decoded the same geometry \
             as the time trial does, so the substitution did not actually swap \
             the environment - ZoneCircuit::Separate::variant_of stopped \
             substituting"
        );

        // Naming one of the title's own Zone circuits directly must not be
        // rewritten.
        let zoning_a_zone_circuit = race::load(&race::Options {
            source,
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Zone,
            track: Some(own_zone_track.to_string()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("{name}: racing {own_zone_track} in zone mode: {e:#}"));
        assert!(
            zoning_a_zone_circuit
                .report
                .iter()
                .any(|line| line.starts_with("zone:") && line.contains("as named")),
            "{name}: naming one of this title's own zone circuits should not be \
             rewritten: {:#?}",
            zoning_a_zone_circuit.report
        );
    }
}

/// **Unverified, on `also_race_circuits` - see `oag_title::ZoneCircuit::Separate`'s
/// own docs for the play-based lead behind it and what does not yet back
/// it.** HD's `ZoneCircuit::variant_of` never substitutes when the flag is
/// set, so - unlike Pure above - naming an ordinary race circuit in Zone mode
/// loads that circuit's own environment unchanged, the same geometry a time
/// trial on it would decode, rather than the title's default zone-exclusive
/// circuit. Naming one of the four zone-exclusive circuits directly still
/// loads unchanged too, on the same terms Pure's does.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn zone_mode_on_hd_races_a_named_race_circuit_directly() {
    let Some(path) = image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let source = path.display().to_string();
    let race_track = oag_hd::race::DEFAULT_TRACK;

    let racing = race::load(&race::Options {
        source: source.clone(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(race_track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {race_track} as a time trial: {e:#}"));

    let zoning_the_race_circuit = race::load(&race::Options {
        source: source.clone(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        track: Some(race_track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("racing {race_track} in zone mode: {e:#}"));

    assert!(
        zoning_the_race_circuit
            .report
            .iter()
            .any(|line| line.starts_with("zone:") && line.contains("as named")),
        "HD/Fury: an ordinary race circuit named in zone mode should race \
         unchanged, not be substituted: {:#?}",
        zoning_the_race_circuit.report
    );
    assert_eq!(
        racing.track_model.indices.len(),
        zoning_the_race_circuit.track_model.indices.len(),
        "HD/Fury: racing {race_track} in zone mode should decode the same \
         geometry as the time trial, since also_race_circuits means the \
         circuit is not substituted"
    );

    let own_zone_track = oag_hd::race::ZONE_TRACK_2;
    let zoning_a_zone_circuit = race::load(&race::Options {
        source,
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        track: Some(own_zone_track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("racing {own_zone_track} in zone mode: {e:#}"));
    assert!(
        zoning_a_zone_circuit
            .report
            .iter()
            .any(|line| line.starts_with("zone:") && line.contains("as named")),
        "HD/Fury: naming one of this title's own zone circuits should not be \
         rewritten: {:#?}",
        zoning_a_zone_circuit.report
    );
}

/// **The craft axis, on the three titles that have a disc image behind them.**
///
/// The companion of the circuit sweeps above, and it splits the corpus the same
/// way: Pulse keeps the Zone hull inside the player's own team, Pure and HD give
/// Zone a ship directory of its own. Two shapes, three titles, no hole - which
/// is what licensed [`oag_title::ZoneCraft`] to be a type rather than a
/// constant, on the same terms as [`ZoneCircuit`]. 2048 and Omega are a third
/// shape, [`oag_title::ZoneCraft::OwnShipAt`], covered by
/// `zone_craft_ground_truth` since their sources are extracted packages and not
/// disc images.
///
/// Each title is asserted on both halves of its own shape, because either alone
/// would pass on a build that had quietly fallen back to the other:
///
/// - the name a Zone race resolves is **not** the one a race would have resolved;
/// - that name is on the disc and decodes to geometry.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_titles_zone_craft_resolves_and_is_not_the_race_hull() {
    let cases: [(&str, &str, oag_title::race::ShipPaths); 3] = [
        (
            "data/images/pulse-psp-usa.chd",
            oag_pulse::race::DEFAULT_TEAM,
            oag_pulse::race::DEFAULTS.ships(),
        ),
        (
            "data/images/pure-psp-eu.chd",
            oag_pure::race::DEFAULT_TEAM,
            oag_pure::race::DEFAULTS.ships(),
        ),
        (
            "data/images/hdfury-ps3-eu-dec.iso",
            oag_hd::race::DEFAULT_TEAM,
            oag_hd::race::DEFAULTS.ships(),
        ),
    ];

    for (name, team, craft) in cases {
        let Some(path) = image(name) else { continue };
        let source = path.display().to_string();
        let archives = oag_source::title::open_source(&source, Vec::new(), Vec::new())
            .expect("opening the source");

        let racing =
            oag_livery::entry::ship_entry_name(craft, team, oag_race::Mode::TimeTrial, None);
        let zoning = oag_livery::entry::ship_entry_name(craft, team, oag_race::Mode::Zone, None);
        assert_ne!(
            racing, zoning,
            "{name}: a zone race should not fly the same hull a race does"
        );
        assert!(
            archives.archives.locate(&zoning).is_some(),
            "{name}: {zoning} should be on the disc"
        );

        // Which half of the enum this title is, asserted against the *name* so a
        // title silently changing shape fails here rather than somewhere subtle.
        match craft.zone {
            oag_title::ZoneCraft::ModelsInTeam { .. } => assert!(
                zoning.contains(team),
                "{name}: this title keeps the zone hull in the player's own team, \
                 so {zoning} should still name {team}"
            ),
            oag_title::ZoneCraft::OwnShip(ship) => {
                assert!(
                    !zoning.contains(team),
                    "{name}: this title has a zone ship of its own, so the player's \
                     team {team} should not appear in {zoning}"
                );
                assert!(zoning.contains(ship), "{name}: {zoning} should name {ship}");
            }
            oag_title::ZoneCraft::PlayerShip | oag_title::ZoneCraft::OwnShipAt { .. } => {
                unreachable!(
                    "{name}: none of the three titles this sweep covers is 2048 or Omega; \
                 theirs are in zone_craft_ground_truth"
                )
            }
        }
        println!("{name}: race {racing} / zone {zoning}");
    }
}

/// **The shipped Zone handling block, and the fact that nothing reads it.**
///
/// The finding the craft enum deliberately does *not* model: all three titles
/// carry a Zone-mode craft directory whose `handlingstats.xml` opens
/// `<Stats team="ZoneMode">` and authors **no `<Class>` block**, where a team
/// file authors four or five. So the shipped answer to "what handling does a
/// Zone craft have" is one block shared by every team - which is what a player
/// would expect of a mode that replaces the throttle - and this engine flies the
/// player's own team instead, because the per-class conversion needs a class.
///
/// Asserted rather than left as a comment so that the day someone teaches the
/// handling layer to read it, this test is what tells them the file's shape.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_titles_zone_handling_is_one_classless_block() {
    // Pulse's zone *craft* is per-team, so its ZoneMode directory is not the one
    // `ZoneCraft::directory` returns - it is named here directly, which is the
    // point: the block exists on all three titles whatever shape the hull takes.
    let cases = [
        ("data/images/pulse-psp-usa.chd", "Zone_01"),
        ("data/images/pure-psp-eu.chd", "Zone_01"),
        ("data/images/hdfury-ps3-eu-dec.iso", "zone"),
    ];

    for (name, directory) in cases {
        let Some(path) = image(name) else { continue };
        let source = path.display().to_string();
        let mut opened = oag_source::title::open_source(&source, Vec::new(), Vec::new())
            .expect("opening the source");
        let entry = oag_tables::handling::entry_name(directory);
        let blob = opened
            .archives
            .read_name(&entry)
            .unwrap_or_else(|e| panic!("{name}: reading {entry}: {e}"));
        let stats = oag_tables::handling::from_blob(&blob)
            .unwrap_or_else(|e| panic!("{name}: parsing {entry}: {e}"));

        assert_eq!(
            stats.team, "ZoneMode",
            "{name}: {entry} should name the mode it is for in the disc's own words"
        );
        assert!(
            stats.classes.is_empty(),
            "{name}: {entry} authors {} <Class> block(s). If this ever stops being \
             zero the file can drive oag_gameplay::handling_for directly, and the \
             divergence oag_title::ZoneCraft documents can be closed",
            stats.classes.len()
        );
        println!("{name}: {entry} is team=ZoneMode with no <Class>");
    }
}

/// **Zone's exhaust lights up with the accelerate button never touched, the
/// same way the original does.**
///
/// Zone mode's engine force comes from `Environment::auto_speed`
/// (`crates/physics/src/forces.rs`), which replaces the throttle entirely at
/// the physics layer - a Zone craft accelerates with nothing held down. Before
/// 2026-08-31 `Race::advance_exhausts` (`crates/raceplay/src/effects.rs`) fed
/// the exhaust's `intensity` ramp from `ship.thrust` regardless of mode - the
/// raw accelerate-button state `crates/physics/src/controls.rs` writes every
/// tick - so a Zone craft flown with nothing held (which nothing in the mode
/// requires) kept `intensity` pinned at zero and drew no flare or trail at
/// all, even at 374 km/h. **Confirmed live against the original**: the trail
/// is visible with no button held, so this was a real divergence, not an
/// inherited quirk.
///
/// The fix mirrors `oag_physics::engine::engine`'s own four-corner gate -
/// `grounded > 0.0` - which is the same condition the force law already uses
/// to decide whether the auto-speed thrust applies this tick, rather than
/// inventing a second one.
///
/// This test is the regression: intensity has to come up substantially with
/// nothing held, on the same order the accelerate-held control case reaches,
/// while the craft's speed is driven by `auto_speed`. It was identical either
/// way until 2026-10-02: Zone's auto-speed now carries the launch multiplier, so
/// 28 ticks after GO a craft that coasted (`normalMul`, 1.4) is ahead of one
/// that held accelerate (`stallMul`, 1.2) by the original's ratio, 1.159 read
/// live at the same age on a Zone engine.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn zone_speed_is_automatic_and_exhaust_intensity_now_follows_it_too() {
    let Some(path) = image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = path.display().to_string();

    let loaded = race::load(&race::Options {
        source: source.clone(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        ..race::Options::default()
    })
    .expect("loading the zone race");

    const TICKS: u32 = 300;

    // Control: accelerate held throughout, the same shape
    // `race_ground_truth.rs`'s driving tests use. This is the sanity check
    // that the mechanism itself works in this port at all.
    let mut held_race = race::Race::start(loaded.setup.clone());
    let mut held = race::HeldButtons::new(Button::Cross.bit());
    for _ in 0..TICKS {
        held_race.tick(&PlayerInputs::single(held.snapshot()));
    }
    let held_intensity = held_race.exhaust_of(0).intensity();
    let held_speed = held_race.exhaust_of(0).speed_kmh();
    assert!(
        held_intensity > 0.5,
        "holding accelerate through {TICKS} ticks should have driven the \
         exhaust intensity up: {held_intensity} at {held_speed} km/h"
    );

    // The case in question: accelerate never pressed. Zone's auto-speed does
    // not need it, so this is a normal way to play the mode.
    let mut coasting_race = race::Race::start(loaded.setup);
    for _ in 0..TICKS {
        coasting_race.tick(&PlayerInputs::none());
    }
    let coasting_intensity = coasting_race.exhaust_of(0).intensity();
    let coasting_speed = coasting_race.exhaust_of(0).speed_kmh();

    println!(
        "held: intensity {held_intensity} at {held_speed} km/h; \
         coasting: intensity {coasting_intensity} at {coasting_speed} km/h"
    );

    assert!(
        coasting_speed > 100.0,
        "auto_speed should have the craft moving fast with nothing held: {coasting_speed} km/h"
    );
    assert!(
        held_speed > 100.0,
        "auto_speed should have the held craft moving fast too: {held_speed} km/h"
    );
    let ratio = coasting_speed / held_speed;
    assert!(
        (ratio - 1.159).abs() < 0.03,
        "28 ticks after GO the coasting craft should lead the held one by the launch \
         grades' ratio, 1.159 on the original: held {held_speed} km/h, coasting \
         {coasting_speed} km/h, ratio {ratio}"
    );
    assert!(
        coasting_intensity > 0.3,
        "exhaust intensity should have come up substantially with accelerate \
         never pressed, the same way the original's flare and trail are lit \
         with nothing held: {coasting_intensity} at {coasting_speed} km/h \
         (held case reached {held_intensity})"
    );
}
