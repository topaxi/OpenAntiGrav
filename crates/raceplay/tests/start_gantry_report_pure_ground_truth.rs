//! The load report says nothing about a start gantry on Wipeout Pure, because
//! there is none to name.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The test skips with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # Why this is its own file
//!
//! It belongs beside `race_ground_truth.rs`'s
//! `pulses_default_track_reports_its_own_trackstartup_xml` and its own Pulse
//! sibling `start_gantry_report_ground_truth.rs`, both of which sit at or near
//! `race_ground_truth.rs`'s frozen `check-size` ceiling - so this takes the
//! same route and carries its own `image()`/`load()` pair.
//!
//! # What it is for
//!
//! `docs/rendering/start-gantry.md`'s "Wipeout Pure" section records that
//! `crates/formats/tests/start_gantry_pure_ground_truth.rs` reads all sixteen
//! circuits' own `TrackStartup.xml` and finds no `num="8"` on any of them,
//! model or colour - so `oag_raceplay::load`'s billboard-slot-8 report line
//! (added for Pulse in `crates/raceplay/src/load.rs`) never fires here, and
//! that is the correct, title-agnostic behaviour rather than a gap: the same
//! code that names Pulse's gantry by not finding a slot 8 on Pure's default
//! circuit, says nothing, honestly, because the circuit's own manifest says
//! nothing.

//! # The hull's light list, on the same load
//!
//! The one race this file loads also carries the hull-lights claim, so a second
//! test does not pay the load again: Vineta K authors one `AmbientLight` and two
//! `DirectionalLight`s under Pure's own class ids (`0x125`, `0x12a`), and the
//! hull is lit by them as Pulse's is, not by the stand-in rig
//! (`docs/ghidra/functions/psp-pure-usa/scene-light.md`).

use std::path::PathBuf;

use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pure-psp-usa.chd")
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_default_track_names_no_gantry_and_no_billboard_location_and_lights_its_hull() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.join("\n");
    println!("{report}");

    assert!(
        !report.contains("slot 8 is the start gantry"),
        "the report names a gantry on Pure's default track, which contradicts \
         start_gantry_pure_ground_truth.rs: {report}"
    );
    assert!(
        report.contains("naming a model and"),
        "the trackstartup.xml line is missing entirely: {report}"
    );
    assert!(
        report.contains("0 naming a model"),
        "Pure's default track (01_Vineta_K) now names a billboard model - \
         re-check whether it is the gantry: {report}"
    );
    hull_lights_are_the_circuits_own(&report);
    sun_flare_is_the_circuits_own(&loaded);
}

/// Vineta K authors one `lensflare` node (`docs/rendering/lens-flare.md`): its sun direction, grey
/// colour and the circuit's own 64 x 64 `LensflareComponents.tga`. Dropping
/// `oag_pure::effects::LOOKS.lens_flare`, the load step or the plan field leaves no flare and no
/// line.
fn sun_flare_is_the_circuits_own(loaded: &race::Loaded) {
    let flare = loaded
        .setup
        .scenery_fx
        .lens_flare
        .as_ref()
        .expect("Pure's Vineta K draws a sun flare");
    let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
    assert!(near(flare.direction[0], -0.8897), "{:?}", flare.direction);
    assert!(near(flare.direction[1], 0.2193), "{:?}", flare.direction);
    assert!(near(flare.direction[2], 0.4004), "{:?}", flare.direction);
    assert!(
        flare.colour.iter().all(|&c| near(c, 0.7107)),
        "{:?}",
        flare.colour
    );
    assert_eq!((flare.atlas.width, flare.atlas.height), (64, 64));
    // A version-4 file authors no levels below the base (`oag_vex::vex::textures`).
    assert_eq!(flare.atlas.levels.len(), 1);
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.starts_with("lens flare: sun direction (-0.890, 0.219, 0.400)")),
        "{:?}",
        loaded.report
    );
}

/// Vineta K's light nodes, as `SceneLight_BuildLightingList`'s Pure twin packs
/// them at the bare scale: the ambient `(72, 78, 84)` and a first directional of
/// full white. Dropping `oag_pure::effects::LOOKS.hull_lights`, or reading the
/// lights under Pulse's class ids again, leaves the stand-in line instead.
fn hull_lights_are_the_circuits_own(report: &str) {
    let line = report
        .lines()
        .find(|l| l.starts_with("hull lights:"))
        .unwrap_or_else(|| panic!("no hull lights line: {report}"));
    assert!(
        line.starts_with("hull lights: the circuit's own, ambient [0.28235295, 0.30588236, 0.32941177, 1.0], diffuse [[1.0, 1.0, 1.0, 0.0], [0.21176471, 0.21176471, 0.18039216, 0.0]"),
        "Pure's hull is not lit by Vineta K's own light nodes: {line}"
    );
}
