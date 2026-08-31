//! A Zone race draws HD/Fury's own Zone sky, not the circuit's.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(zone_sky_ground_truth)'
//! ```
//!
//! # What this file is the evidence for
//!
//! HD/Fury's per-race environment loader picks between two cubemaps on one
//! byte: `Data/Tex/ZoneSky.gtf` by its own name, or the `sky.gtf` beside the
//! circuit. Both branches call the same loader, store into the same handle
//! slot and fall into the same sampler-state patch, so the picture is the only
//! thing that changes - a **replacement**, not a tint. Evidence and addresses
//! in `docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`.
//!
//! These tests hold the port to that, against the disc: a Zone race resolves
//! the Zone cubemap and a Single Race on the same circuit resolves the
//! circuit's, they are different pictures, and the Zone one is the small one -
//! 64x64 faces against `01_vineta_k`'s 2048x2048, the measured form of the
//! maintainer's own observation that a Zone sky reads as solid colours or
//! gradients.
//!
//! # What is deliberately not asserted
//!
//! **Nothing about `Sky horizon colour` / `Sky zenith colour`.** Those two keys
//! are cross-faded per Zone stage and reach a single-caller draw the same gate
//! byte selects, but that draw's geometry has not been read, so this port draws
//! no gradient at all and asserts nothing about one. See the same page's
//! `## What is not established`.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_physics::SpeedClass;

/// One image, or `None` with a printed reason when it is not present.
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

/// A race on **one named circuit**, so the only thing a pair of loads differs
/// in is the mode.
///
/// Naming the track matters here: HD's Zone picker offers its own arenas, so a
/// default Zone load resolves `zone_1` while a default Single Race load
/// resolves the campaign's opening circuit - and then a difference between the
/// two skies proves only that two circuits ship different art. `01_vineta_k` is
/// a racing circuit both modes accept.
fn load_on(mode: oag_race::Mode, track: &str) -> Option<race::Loaded> {
    let path = image("data/images/hdfury-ps3-eu-dec.iso")?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: SpeedClass::Venom,
            mode,
            track: Some(track.to_string()),
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

const CIRCUIT: &str = r"Data\Environments\01_vineta_k\track.vex";

fn load(mode: oag_race::Mode) -> Option<race::Loaded> {
    load_on(mode, CIRCUIT)
}

/// The six faces of a loaded sky, as raw RGBA, with their size.
fn faces(sky: &oag_render::mesh::Model) -> (u32, u32, Vec<Vec<u8>>) {
    let textures: Vec<_> = sky
        .textures
        .iter()
        .map(|t| t.as_ref().expect("every sky face carries a texture"))
        .collect();
    assert_eq!(textures.len(), 6, "a cubemap has six faces");
    let (width, height) = (textures[0].width, textures[0].height);
    let pixels = textures
        .iter()
        .map(|t| {
            assert_eq!((t.width, t.height), (width, height), "faces differ in size");
            t.to_rgba().expect("a face decodes to pixels").into_owned()
        })
        .collect();
    (width, height, pixels)
}

/// **A Zone race draws a different sky from a Single Race on the same
/// circuit, and it is the disc's own `ZoneSky.gtf`.**
///
/// **Both loads name the same circuit**, so the mode is the only variable -
/// which matters, because HD's Zone picker would otherwise send the Zone load
/// to a `zone_N` arena and a difference between two skies would prove only
/// that two circuits ship different art.
///
/// The load-bearing assertion is the **size**, not the face comparison: no
/// `sky.gtf` on this disc is 64x64, so a regression that dropped the swap
/// cannot satisfy it. The face comparison and the report string are
/// corroboration.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_zone_race_swaps_the_circuits_sky_for_the_zone_one() {
    let (Some(zone), Some(single_race)) =
        (load(oag_race::Mode::Zone), load(oag_race::Mode::SingleRace))
    else {
        return;
    };
    for line in &zone.report {
        println!("{line}");
    }
    let zone_sky = zone.sky_model.as_ref().expect("the Zone sky loads");
    let circuit_sky = single_race
        .sky_model
        .as_ref()
        .expect("the circuit's sky loads");

    assert!(
        zone.report
            .iter()
            .any(|line| line.contains("the Zone sky, drawn in place of the circuit's own")),
        "the load report should say which sky it drew"
    );
    assert!(
        single_race
            .report
            .iter()
            .any(|line| line.contains("the circuit's sky")),
        "a non-Zone race still reports the circuit's own sky"
    );

    let (zone_width, zone_height, zone_faces) = faces(zone_sky);
    let (circuit_width, circuit_height, circuit_faces) = faces(circuit_sky);

    // **64x64 against 2048x2048.** Both numbers are the files' own headers,
    // and the ratio is the measured form of "a Zone sky reads as solid colours
    // or gradients": 1,024 times fewer texels a face, 12,288 bytes of DXT1
    // against 12,582,912. **This pair of assertions is the real
    // discriminator** - no `sky.gtf` on the disc is 64x64, so a load that
    // failed to swap could not satisfy the first one.
    assert_eq!(
        (zone_width, zone_height),
        (64, 64),
        "ZoneSky.gtf is a 64x64 cubemap"
    );
    assert_eq!(
        (circuit_width, circuit_height),
        (2048, 2048),
        "01_vineta_k's sky.gtf is a 2048x2048 cubemap"
    );
    assert_ne!(
        zone_faces, circuit_faces,
        "the Zone sky must not be the circuit's"
    );
}

/// **The Zone sky is a picture, not a flat fill** - which is worth asserting
/// separately, because "reads as solid colours" is exactly the description a
/// silently-blank texture would also earn.
///
/// Five of the six faces carry hundreds of distinct colours off the disc. The
/// sixth - the nadir - genuinely is near-flat, so this asserts a majority
/// rather than all six, and it asserts the faces are not all the *same* face.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_zone_sky_carries_real_texels_on_five_of_its_six_faces() {
    let Some(zone) = load(oag_race::Mode::Zone) else {
        return;
    };
    let sky = zone.sky_model.as_ref().expect("the Zone sky loads");
    let (_, _, pixels) = faces(sky);

    let varied = pixels
        .iter()
        .filter(|face| {
            let mut colours: Vec<[u8; 4]> = face.as_chunks::<4>().0.to_vec();
            colours.sort_unstable();
            colours.dedup();
            colours.len() > 100
        })
        .count();
    assert!(
        varied >= 5,
        "five of six faces should carry real art, saw {varied}"
    );

    let mut distinct = pixels.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), 6, "no two faces of the Zone sky are equal");
}

/// **Outside HD, nothing swaps.** Pulse ships no located Zone sky, so a Zone
/// race there keeps whatever sky the track authors - the absence is a `None`
/// in the title record rather than a special case in the loader.
#[test]
fn a_title_with_no_located_zone_sky_carries_none() {
    assert_eq!(oag_pulse::race::DEFAULTS.zone_sky, None);
    assert_eq!(oag_pure::race::DEFAULTS.zone_sky, None);
    assert_eq!(oag_2048::race::DEFAULTS.zone_sky, None);
    assert_eq!(
        oag_hd::race::DEFAULTS.zone_sky,
        Some(oag_hd::race::ZONE_SKY)
    );
}
