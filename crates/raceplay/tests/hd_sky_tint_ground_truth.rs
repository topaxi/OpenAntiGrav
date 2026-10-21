//! HD's sky is tinted by the circuit's own `Lighting.Sky colour`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-raceplay \
//!     --run-ignored all -E 'test(/^hd_sky_tint_ground_truth::/)'
//! ```
//!
//! The original's sky draw carries `Lighting.Sky colour` as the cube's vertex
//! colour, byte for byte: read live off RPCS3 draw captures as
//! `(255, 255, 255, 0)` on Sol 2 and Sebenco Climb, `(128, 128, 128, 0)` on
//! Talon's Junction and `(140, 140, 140, 0)` on The Amphiseum, with a fragment
//! program that is `tex * COL0` and nothing else (`renderer.md`, "The sky law,
//! read off live draws"). So the tint is `byte / 255`
//! ([`oag_mesh::mesh::sky_cube::SKY_COLOUR_FULL`]): 255 leaves the texture
//! alone and 128 halves it in linear light. The `byte / 128` fit this replaced
//! doubled Sol 2's sky and Sebenco's; dropping the tint from the loader leaves
//! every circuit at 1.0 and fails the Talon's and Amphiseum assertions.

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
fn the_sky_tint_is_the_authored_byte_over_255() {
    for (track, byte) in [
        (r"Data\Environments\12_sol_2\track.vex", 255.0),
        (r"Data\Environments\10_sebenco_climb\track.vex", 255.0),
        (r"Data\Environments\talons_junction\track.vex", 128.0),
        (r"Data\Environments\amphiseum\track.vex", 140.0),
    ] {
        let Some(tint) = sky_tint(track, oag_race::Mode::SingleRace) else {
            return;
        };
        assert_eq!(tint, [byte / 255.0; 3], "{track} authors Sky colour {byte}");
    }
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

/// The fourth byte of `Sky colour` is the glow mask the sky writes: Metropia authors
/// `255 255 255 122` and the original's live scene target reads alpha 122 on its sky
/// (RPCS3, 2026-10-21). A Zone sky masks alpha off, so it writes none.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_sky_writes_its_fourth_byte_as_the_glow_mask() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mask = |track: &str, mode| {
        let loaded = race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode,
            track: Some(track.to_string()),
            ..race::Options::default()
        })
        .expect("loading the race");
        let sky = loaded.sky_model.as_ref().expect("the circuit's sky loads");
        assert!(sky.glow_from_program, "the sky's mask is its own program's");
        sky.vertices[0].glow
    };
    assert_eq!(
        mask(
            r"Data\Environments\02_Track\track.vex",
            oag_race::Mode::SingleRace
        ),
        122.0 / 255.0,
        "Metropia's sky"
    );
    assert_eq!(
        mask(
            r"Data\Environments\12_sol_2\track.vex",
            oag_race::Mode::SingleRace
        ),
        0.0,
        "Sol 2 authors a fourth byte of 0"
    );
    assert_eq!(
        mask(
            r"Data\Environments\02_Track\track.vex",
            oag_race::Mode::Zone
        ),
        0.0,
        "the Zone sky's branch masks alpha off"
    );
}
