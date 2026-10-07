//! HD's sky is tinted by the circuit's own `Lighting.Sky colour`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(hd_sky_tint_ground_truth)'
//! ```
//!
//! The original's sky draw passes `Lighting.Sky colour` as the cube's vertex
//! colour (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "HD's sky is
//! tinted by Sky colour"). Sol 2 authors 255 and Talon's Junction 128, and the
//! matched captures put Sol 2's sky at twice ours with zero offset while
//! Talon's already matched, so 128 is the neutral byte
//! ([`oag_mesh::mesh::sky_cube::SKY_COLOUR_NEUTRAL`]). Dropping the tint from
//! the loader leaves both circuits at 1.0 and fails the Sol 2 assertion.

use oag_raceplay as race;

fn sky_tint(track: &str, mode: oag_race::Mode) -> Option<[f32; 3]> {
    let path = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let sky = loaded.sky_model.as_ref().expect("the circuit's sky loads");
    let first = sky.vertices.first().expect("the sky has vertices").colour;
    assert!(
        sky.vertices.iter().all(|v| v.colour == first),
        "one tint over all 24 corners"
    );
    Some([first[0], first[1], first[2]])
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn sol_2_doubles_its_sky_and_talons_junction_leaves_it() {
    let (Some(sol), Some(talons)) = (
        sky_tint(
            r"Data\Environments\12_sol_2\track.vex",
            oag_race::Mode::SingleRace,
        ),
        sky_tint(
            r"Data\Environments\talons_junction\track.vex",
            oag_race::Mode::SingleRace,
        ),
    ) else {
        return;
    };
    assert_eq!(sol, [255.0 / 128.0; 3], "Sol 2 authors Sky colour 255");
    assert_eq!(talons, [1.0; 3], "Talon's Junction authors Sky colour 128");
}

/// The Zone branch of the original's sky draw does not take the circuit's
/// colour, so a Zone sky stays untinted whatever the circuit authors.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_zone_sky_is_not_tinted() {
    let Some(tint) = sky_tint(
        r"Data\Environments\12_sol_2\track.vex",
        oag_race::Mode::Zone,
    ) else {
        return;
    };
    assert_eq!(tint, [1.0; 3]);
}
