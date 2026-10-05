//! A Wipeout Pure disc loads a race, and a craft stays on the circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(pure_race_ground_truth)'
//! ```
//!
//! # What this is for
//!
//! `race_ground_truth.rs` is Pulse's, and every one of its assertions is a
//! measurement against Pulse's own recorded behaviour. This file is deliberately
//! much weaker: **Pure's physics is not implemented**, the race runs on Pulse's
//! force law with Pulse's `<pitch>` block standing in for the one Pure's files do
//! not author, and no lap time here means anything. What it pins is the load
//! path and the one property that makes a race a race rather than a fall.
//!
//! Four things had to be true for any of this to run, and each is asserted
//! separately so a regression names itself rather than showing up as "the craft
//! fell through the track":
//!
//! 1. The source opens as Pure at all - `race::load` used to be
//!    `oag_pulse::open_with_packs` outright and refused the disc by serial.
//! 2. The track's geometry decodes - its `.vex` is **version 4**, whose class
//!    numbering shares nothing with the version 6 the constants spell.
//! 3. Its **collision** geometry decodes, which needs `vex::classes::V4`'s
//!    floor and wall ids - recovered at confidence 88, see
//!    `crates/pure/tests/collision_classes_ground_truth.rs`.
//! 4. Its `handlingstats.xml` parses, which needs `<pitch>` to be optional.
//!
//! # What is deliberately not asserted
//!
//! No lap time, no clean-lap count, no comparison against Pulse. Pure ships a
//! five-rung speed ladder to Pulse's four, no `<pitch>`, and its own handling
//! model is unread; a lap-time bound here would be pinning this project's
//! stand-ins, not the original. It would also fail for reasons already written
//! down, which is noise. See `docs/formats/pure-status.md`.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;
use std::path::{Path, PathBuf};

/// Both Pure pressings, whichever are present.
fn images() -> Vec<(&'static str, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for (label, name) in [
        ("pure-psp-eu", "data/images/pure-psp-eu.chd"),
        ("pure-psp-usa", "data/images/pure-psp-usa.chd"),
    ] {
        let path = root.join(name);
        if path.exists() {
            found.push((label, path));
        } else {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
        }
    }
    found
}

/// Every circuit the disc's own plugin definition declares as a `Race`.
///
/// Read off the disc for the reason `oag_raceplay::catalogue` gives: a list of
/// circuit names in this repository would be shipped content. Pure declares no
/// `Reversed` entry anywhere, which is itself a difference from Pulse and is
/// asserted below rather than assumed here.
fn circuits(image: &Path) -> Vec<catalogue::Track> {
    let mut archives = oag_pure::open(&image.display().to_string()).expect("opening Pure");
    let blob = archives
        .read_name(oag_pure::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let xml = oag_tables::fexml::text(&blob).expect("Pure's XML is not shortened");
    catalogue::tracks(&xml)
}

/// The default Pure race, as `just play --race` on a Pure source resolves it.
fn load(image: &Path, track: &str) -> race::Loaded {
    race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        // Time Trial: the milestone's mode, and the one with no weapons. Pure's
        // `Weapon Pad` class id is recovered now (`vex::classes::V4::weapon_pad`,
        // 2026-08-13), but Pure's own weapon-pickup behaviour has not been
        // measured against anything, so this file still races the mode that
        // asks nothing of it.
        mode: oag_race::Mode::TimeTrial,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {track}: {e:#}"))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pure_source_opens_as_pure_and_loads_its_default_circuit() {
    for (label, image) in images() {
        let track = oag_pure::race::DEFAULT_TRACK;
        let loaded = load(&image, track);
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.contains("Wipeout Pure")),
            "{label}: the load report should name the title it opened as; was {:#?}",
            loaded.report
        );
        assert!(
            loaded.track_model.indices.len() > 3,
            "{label}: {track} decoded to no geometry - its .vex is version 4, so \
             this is the class table rather than the file"
        );
        assert!(
            loaded.liveries[0].hull.indices.len() > 3,
            "{label}: the ship decoded to no geometry"
        );
    }
}

/// Pure ships no standalone boost-plume model at all
/// (`docs/ghidra/functions/psp-pure-usa/ship-models.md`), so the load report
/// must say so as a title fact and never as a fault - see
/// `race_ground_truth.rs`'s `pulses_load_report_still_finds_its_boost_plume`
/// for the title that does have one. Asserts both halves: the fault-line
/// phrasing is gone, and the positive line is there in its place - silence
/// would pass a report that dropped the note by accident.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pure_races_load_report_names_no_boost_model_as_a_title_fact_not_a_fault() {
    for (label, image) in images() {
        let loaded = load(&image, oag_pure::race::DEFAULT_TRACK);
        assert!(
            loaded
                .report
                .iter()
                .all(|line| !line.contains("no boost plume for this team")),
            "{label}: a fault-line boost report survived: {:#?}",
            loaded.report
        );
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.contains("names no standalone boost-plume model")),
            "{label}: no positive boost-absence line in the report: {:#?}",
            loaded.report
        );
        // The "already correct, already measured" claim in pure-status.md and
        // exhaust-sound.md rests on the always-on flare quad actually being
        // the one drawn - pin the two preconditions `frame.rs` gates it on,
        // not just the boost model's own absence.
        assert!(
            loaded.liveries[0].nozzle.is_some(),
            "{label}: no Engine Flare locator on the player's own hull, so \
             nothing carries the boost-driven growth this checks for"
        );
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.contains("grabbedEngineFlare")),
            "{label}: the flare sprite texture did not load, so the quad this \
             title's boost term drives would draw untextured or not at all: {:#?}",
            loaded.report
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_pure_circuit_decodes_geometry_collision_and_a_spline() {
    for (label, image) in images() {
        let tracks = circuits(&image);
        assert!(!tracks.is_empty(), "{label}: the disc declares circuits");
        assert!(
            tracks.iter().all(|track| !track.reversed),
            "{label}: Pure declares no reversed circuit, and this build would look \
             for a track_reversed.vex that is not on the disc"
        );

        let mut checked = 0;
        for track in &tracks {
            let entry = track.entry_name();
            // A `Classic` or `Zone` entry points at a directory this build has no
            // circuit for; skipping is not a silent pass because `checked` below
            // asserts that some remained.
            let Ok(loaded) = race::load(&race::Options {
                source: image.display().to_string(),
                class: "VENOM".to_string(),
                mode: oag_race::Mode::TimeTrial,
                track: Some(entry.clone()),
                ..race::Options::default()
            }) else {
                println!("{label}: {entry} did not load, skipped");
                continue;
            };
            checked += 1;

            let triangles: usize = loaded
                .setup
                .collision
                .colliders()
                .iter()
                .map(oag_physics::TriangleSoup::triangle_count)
                .sum();
            assert!(
                triangles > 0,
                "{label}: {} has no collision geometry, so a craft would fall \
                 through it. That is what `vex::classes::V4`'s floor and wall ids \
                 exist for - see crates/pure/tests/collision_classes_ground_truth.rs",
                track.id
            );
            assert!(
                !loaded.setup.spline.is_empty(),
                "{label}: {} has no spline, so there is nothing to spawn on and no \
                 lap to time",
                track.id
            );
        }
        assert!(
            checked > 0,
            "{label}: no circuit loaded at all, so nothing above was tested"
        );
        println!("{label}: {checked} of {} circuits load", tracks.len());
    }
}

/// A craft under full throttle drives a Pure circuit, and a wall stops it.
///
/// **The only behavioural claim in this file, and a deliberately narrow one.** It
/// does not say the craft laps, or how quickly: Pure's handling is unread and its
/// `<pitch>` block is Pulse's stand-in, so a lap bound would pin a substitution
/// rather than measure the original. What it says is that the collision geometry
/// recovered for version 4 is **load-bearing**, on both of its classes.
///
/// That distinction cannot be made from the load report. A track whose floor and
/// wall ids were assigned the wrong way round reports exactly the same collider
/// and triangle counts as one that is right - the difference only appears in what
/// happens to a craft on it. So this drives one:
///
/// - **Full throttle, stick centred.** Not a driving test: a centred stick means
///   the craft runs the opening straight and then meets the outside of the first
///   corner, which is the point. Steering it round would make the distance figure
///   a statement about this test's inputs.
/// - **It travels**, which needs a floor under it. Measured at 265 units on both
///   pressings before it stops.
/// - **It stops, and stays stopped**, 16 units off the spline with the circuit's
///   half-width at over a hundred - which needs a wall beside it. A craft with no
///   wall carries straight on into scenery and keeps going.
///
/// Both bounds below are far from the measured values on purpose. This should
/// fail when the collision classes are wrong and at no other time; it is not a
/// performance ratchet, and Pure's physics being Pulse's means it could not
/// honestly be one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_craft_under_power_drives_a_pure_circuit_and_a_wall_stops_it() {
    for (label, image) in images() {
        let loaded = load(&image, oag_pure::race::DEFAULT_TRACK);
        let mut race = race::Race::start(loaded.setup);
        for slot in 1..oag_gameplay::MAX_SHIPS {
            race.sim.world.ships[slot].active = false;
        }

        // Cross is thrust; see `oag_gameplay::controls`.
        let mut input = oag_gameplay::InputSnapshot::default();
        input
            .buttons
            .begin_frame(oag_gameplay::input::Button::Cross.bit());

        let start = race.sim.world.ships[0].physics.body.position;
        let mut furthest_from_spline = 0.0f32;
        let mut travelled = 0.0f32;
        for _ in 0..1_200 {
            race.tick(&PlayerInputs::single(input));
            let at = race.sim.world.ships[0].physics.body.position;
            // Distance to the spline is the honest measure of "on the circuit":
            // these tracks climb and bank, so comparing height against the start
            // would read a hill as a fall.
            if let Some(distance) = race.spline().distance_to(at) {
                furthest_from_spline = furthest_from_spline.max(distance);
            }
            travelled = travelled.max(at.distance(start));
        }
        let resting = race.sim.world.ships[0].physics.body.position;

        println!(
            "{label}: travelled {travelled:.0} units, furthest from the spline \
             {furthest_from_spline:.1}, resting {:.1} out, respawns {}",
            race.spline().distance_to(resting).unwrap_or(-1.0),
            race.respawns()
        );

        assert!(
            travelled > 150.0,
            "{label}: the craft moved only {travelled:.0} units under full throttle, \
             so nothing is carrying it - `vex::classes::V4`'s floor id is what to \
             re-check"
        );
        // The widest half-width measured on these circuits is about 144 units, so
        // 400 sits well outside any of them and nowhere near the thousands a craft
        // in free fall covers in twenty seconds.
        // Pure's reset volumes *are* recovered now (`0x37f`, by the class-name
        // table index), so a craft that leaves does get put back. This bound is
        // therefore about the craft staying on the circuit under power, not about
        // the absence of a safety net: a respawn would show up as a *small*
        // distance here, never a large one.
        assert!(
            furthest_from_spline < 400.0,
            "{label}: the craft reached {furthest_from_spline:.1} units from the \
             spline, which is off the circuit and further than a reset respawn \
             leaves it - see `vex::classes::V4`'s floor and wall ids"
        );
    }
}

/// **VECTOR races on Pure's own authored tuning, and on nobody else's.**
///
/// This is the assertion the fifth speed class exists for. "Five rows are
/// offered" would not prove it - a row that quietly loaded `VENOM`'s numbers
/// would pass that and be exactly the invented stand-in this project forbids.
/// So the check is that the resolved parameter set **differs** from `VENOM`'s,
/// which can only be true if `Stats::class_named` reached the
/// `<Class name="VECTOR">` block rather than falling back.
///
/// The inputs a race takes per rung are checked separately, because they come
/// out of three different files and a regression in one would otherwise hide
/// behind the others:
///
/// | Input | File |
/// | --- | --- |
/// | `<Class>` tunables | `Data\Ships\<Team>\handlingstats.xml` |
/// | speed pads, gravity scale, weapon-pad debounce | `Data\XML\HandlingStats.xml` |
/// | pickup odds | `Data\XML\weaponstats.xml` |
#[test]
#[ignore = "needs a disc image in data/images/"]
fn vector_loads_pures_own_tuning_and_not_venoms() {
    for (label, image) in images() {
        let track = oag_pure::race::DEFAULT_TRACK;
        let options = |class: &str| race::Options {
            source: image.display().to_string(),
            class: class.to_string(),
            mode: oag_race::Mode::TimeTrial,
            track: Some(track.to_string()),
            ..race::Options::default()
        };

        let vector = race::load(&options("VECTOR"))
            .unwrap_or_else(|e| panic!("{label}: loading VECTOR: {e:#}"));
        let venom = race::load(&options("VENOM"))
            .unwrap_or_else(|e| panic!("{label}: loading VENOM: {e:#}"));

        // The per-team `<Class>` block. Pure authors VECTOR as the rung *below*
        // Venom, so at least one force-law parameter has to differ; asserting
        // the whole block differs rather than naming a field keeps this from
        // encoding a value.
        assert_ne!(
            vector.setup.handling, venom.setup.handling,
            "{label}: VECTOR resolved to VENOM's parameter set, which means the \
             <Class name=\"VECTOR\"> block was not reached"
        );

        // The engine-wide `<GlobalClass>`, which `Global::extra` now keeps
        // instead of discarding. Reported rather than asserted unequal: two
        // rungs authoring the same speed-pad boost is a thing a file is allowed
        // to do, and the block above already proves the lookup is name-keyed.
        println!(
            "{label}: VECTOR gravity scale {} vs VENOM {}",
            vector.setup.class_gravity_scale, venom.setup.class_gravity_scale
        );

        // The pickup odds. `<Pickupodds class="Vector">` is authored beside the
        // four, and `pickup::table_for` matches the name case-insensitively.
        let weapons = vector
            .setup
            .weapons
            .as_ref()
            .expect("Pure authors a weapon table");
        assert!(
            oag_weapons::pickup::table_for(weapons, "VECTOR").is_some(),
            "{label}: Pure authors <Pickupodds class=\"Vector\"> and it was not found"
        );

        println!("{label}: VECTOR resolves to its own tuning");
    }
}

/// The other arm: **Pulse has no fifth rung and refuses one outright.**
///
/// A Pulse source asked for `VECTOR` must fail to load rather than silently
/// race on some other rung - and the failure has to name what the file *does*
/// carry, so the next reader is not left guessing. Pulse authors a
/// `<GlobalClass name="VECTOR">` like every measured disc, which is exactly why
/// this needs asserting: the engine-wide file alone would let a rung look
/// available that no team can actually fly.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pulse_source_has_no_fifth_rung_and_says_so() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let image = root.join("data/images/pulse-psp-usa.chd");
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            image.display()
        );
        println!("skipping: {} not present", image.display());
        return;
    }

    let err = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VECTOR".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect_err("Pulse authors no <Class name=\"VECTOR\"> in any team");

    let text = format!("{err:#}");
    assert!(
        text.contains("VECTOR") && text.contains("VENOM"),
        "the failure must name the rung asked for and the ladder the file \
         carries; was {text}"
    );

    // And the ladder Pulse's own package declares is the four, so the menu
    // never offers the rung this refused in the first place.
    let ladder = oag_pulse::TITLE
        .race
        .speed_classes
        .expect("Pulse's ladder is measured");
    assert_eq!(ladder.names(), oag_title::SpeedClasses::PULSE_LADDER);
    assert!(!ladder.names().contains(&oag_title::SpeedClasses::VECTOR));
}
