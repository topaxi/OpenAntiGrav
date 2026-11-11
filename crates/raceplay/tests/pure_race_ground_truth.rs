//! A Wipeout Pure disc loads a race, and a craft stays on the circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-raceplay \
//!     --run-ignored all -E 'test(/^pure_race_ground_truth::/)'
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

/// Pure ships no LeachBeam (`weapons.md`: no block in its `weaponstats.xml`, no
/// string in either executable, none of Pulse's six LeachBeam entries in its
/// `Data.wad`), so a Pure race neither asks for Pulse's beam assets nor reports
/// them absent, and says why in one positive line. Dropping the `.without`
/// triggers in `oag_pure::effects` or the `authored` gate in
/// `assets::leach_beam_texture` brings the absence lines back.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pure_race_neither_loads_nor_misses_a_leachbeam() {
    for (label, image) in images() {
        let loaded = load(&image, oag_pure::race::DEFAULT_TRACK);
        let mentions: Vec<&String> = loaded
            .report
            .iter()
            .filter(|line| {
                let line = line.to_ascii_lowercase();
                line.contains("leachbeam") || line.contains("leechbeam")
            })
            .collect();
        assert_eq!(
            mentions.len(),
            1,
            "{label}: only the one line saying why no beam is loaded, got {mentions:#?}"
        );
        assert!(
            mentions[0].contains("authors no LeachBeam"),
            "{label}: {}",
            mentions[0]
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
        // four, and `pickups_for` matches the name case-insensitively.
        let weapons = vector
            .setup
            .weapons
            .as_ref()
            .expect("Pure authors a weapon table");
        assert!(
            weapons.pickups_for("VECTOR").is_some(),
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

/// Pure's Plasma blast plays its own three models: `plasma_halo`,
/// `plasma_hemisphere_noglow` and `plasma_hemisphere` resolve on the disc and
/// the race carries Pure's law (2.0 s life, the 3.5 hemisphere drop). Fails if
/// the title's `plasma_blast_pulse` row is dropped (nothing loads, no law).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pure_race_loads_its_own_plasma_blast_trio() {
    for (label, image) in images() {
        let loaded = load(&image, oag_pure::race::DEFAULT_TRACK);
        let blast = &loaded.plasma_blast_models;
        for (name, model) in [
            ("plasma_halo", &blast.halo),
            ("plasma_hemisphere_noglow", &blast.hemisphere2),
            ("plasma_hemisphere", &blast.hemisphere1),
            ("electric_halo2", &blast.disruptor_halo),
            ("disruptor_effect", &blast.disruptor_craft),
        ] {
            assert!(
                model.is_some(),
                "{label}: {name} did not load: {:#?}",
                loaded.report
            );
        }
        assert!(
            loaded.setup.disruptor_craft_law.is_some(),
            "{label}: the on-craft Disruptor law is Pure's"
        );
        let law = loaded.setup.plasma_blast_law.expect("Pure's blast law");
        assert_eq!(law.life, 2.0, "{label}");
        assert_eq!(law.hemisphere_drop, 3.5, "{label}");
        assert!(law.halo_hide_at.is_none(), "{label}");
    }
}

/// Pure spawns its own weapon effects: every trigger the executable was read
/// for (`psp-pure-usa/weapon-effects.md`, `plasma.md`) resolves to a decoded
/// `Data\Psys` effect on the disc, the Missile, Mine and Bomb group as well as
/// the Disruptor's head and explosion and the Bomb's glow. Fails if a name is
/// dropped from Pure's table or no longer decodes.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pure_race_loads_the_effects_its_weapons_play() {
    use oag_title::Trigger;
    for (label, image) in images() {
        let loaded = load(&image, oag_pure::race::DEFAULT_TRACK);
        for (group, triggers) in [
            ("Plasma", &[Trigger::PlasmaBlast, Trigger::PlasmaFlare][..]),
            (
                "Missile/Mine/Bomb",
                &[
                    Trigger::MissileFlare,
                    Trigger::MissileBounce,
                    Trigger::MissileExplo,
                    Trigger::MineExplo,
                    Trigger::BombGlow,
                    Trigger::BombSmokering,
                ][..],
            ),
            (
                "Disruptor",
                &[
                    Trigger::DisruptorHead,
                    Trigger::DisruptorExplo,
                    Trigger::DisruptorLaunch,
                ][..],
            ),
        ] {
            for trigger in triggers {
                let spec = oag_pure::TITLE
                    .effect_on(*trigger)
                    .unwrap_or_else(|| panic!("{label}: Pure's table has no {trigger:?}"));
                assert!(
                    loaded.setup.effects.get(spec.effect).is_some(),
                    "{label}: {group} {trigger:?} ({}) did not decode: {:#?}",
                    spec.effect,
                    loaded.report
                );
            }
        }
    }
}

/// The pitch-input gate is a per-title craft law (`oag_title::craft_laws::CraftLaws::pitch_gate`):
/// Pure's `Ship_UpdatePitch` (`0x0892caf8`) never calls `Ship_TestPitchAlignment`, so every Pure
/// craft flies ungated, while Pulse (no `CraftLaws`) runs the gate (`docs/physics/pure-physics.md`,
/// "Pitch is three code literals"; `docs/physics/airborne-ground-truth.md`). Checked on every slot
/// of a full field at seating, which is when the speed plan simulates the craft, and again after
/// a reset, which must keep the title's laws. Fails if the flag stops reaching the craft or
/// either title's value moves.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_crafts_fly_without_the_pitch_gate_and_pulse_crafts_with_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources: Vec<(&str, PathBuf, bool)> = images()
        .into_iter()
        .map(|(label, image)| (label, image, false))
        .collect();
    let pulse = root.join("data/images/pulse-psp-usa.chd");
    if pulse.exists() {
        sources.push(("pulse-psp-usa", pulse, true));
    } else {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            pulse.display()
        );
    }
    for (label, image, gated) in sources {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            weapons_override: Some(false),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("{label}: loading its default race: {e:#}"));
        let mut race = race::Race::start(loaded.setup);
        let count = race.sim.world.ship_count as usize;
        assert!(count > 1, "{label}: a single race seats a field");
        for slot in 0..count {
            assert_eq!(
                race.sim.world.ships[slot].physics.pitch_gate, gated,
                "{label}: slot {slot} pitch gate at seating"
            );
        }
        for slot in 0..count {
            let body = &race.sim.world.ships[slot].physics.body;
            let pose = oag_gameplay::spawn::Pose {
                position: body.position,
                orientation: body.orientation,
            };
            race.sim.world.ships[slot].place_at(pose);
            assert_eq!(
                race.sim.world.ships[slot].physics.pitch_gate, gated,
                "{label}: slot {slot} pitch gate after a reset"
            );
        }
    }
}
