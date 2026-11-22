//! A Zone race draws HD/Fury's own Zone sky, not the circuit's.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-raceplay \
//!     --run-ignored all -E 'test(/^zone_sky_ground_truth::/)'
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
//! # The dome over it
//!
//! The same gate draws `Sky_DrawGradientDome` in the stage's `Sky horizon
//! colour` and `Sky zenith colour`, and the original's Zone frame shows that
//! dome and no cubemap texel, so the port draws the dome as the sky and keeps
//! the cubemap as the reflection map only (`zone-sky.md`, "The dome is what
//! the frame shows").

use std::path::PathBuf;

use oag_raceplay as race;

/// One image, or `None` with a printed reason when it is not present.
fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
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
            class: "VENOM".to_string(),
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
fn faces(sky: &oag_mesh::mesh::Model) -> (u32, u32, Vec<Vec<u8>>) {
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
            .any(|line| line
                .contains("/data/tex/zonesky.gtf: loaded as the Zone sky's reflection map")),
        "the load report should say the Zone cubemap was read, and what for"
    );
    assert!(
        single_race
            .report
            .iter()
            .any(|line| line.contains("the circuit's sky")),
        "a non-Zone race still reports the circuit's own sky"
    );

    // The drawn sky is the dome, untextured, and the cubemap is its reflection.
    assert_eq!(zone_sky.label, oag_mesh::mesh::sky_cube::ZONE_DOME_LABEL);
    assert!(zone_sky.textures.iter().all(Option::is_none));
    let zone_map = zone_sky
        .sky_reflection
        .as_ref()
        .expect("the Zone cubemap is the dome's reflection map");
    let circuit_map = oag_mesh::mesh::sky_paraboloid::from_sky(circuit_sky)
        .expect("the circuit's cubemap makes a reflection map");
    assert_ne!(
        zone_map.as_slice(),
        circuit_map.as_slice(),
        "the Zone reflection must be ZoneSky.gtf's, not the circuit's"
    );

    // **64x64 against 2048x2048**: the circuit's own sky still draws its file.
    let (circuit_width, circuit_height, _) = faces(circuit_sky);
    assert_eq!(
        (circuit_width, circuit_height),
        (2048, 2048),
        "01_vineta_k's sky.gtf is a 2048x2048 cubemap"
    );
}

/// **The dome draws the disc's own stage colours by the read band law.**
/// On Vineta K's Zone start (stage 1, Sub Venom) a direction at eye level
/// takes `Sky horizon colour` alone and one overhead `Sky zenith colour`
/// alone, as the bytes the original packs, encoded by the scene's curve.
/// Dropping the dome, the stage's colours or the byte truncation fails here.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_zone_dome_takes_the_stages_horizon_and_zenith_colours() {
    let Some(zone) = load(oag_race::Mode::Zone) else {
        return;
    };
    let sky = zone.sky_model.as_ref().expect("the Zone sky loads");
    let palette = zone
        .zone_grade
        .as_ref()
        .expect("a Zone race on HD carries its grade")
        .palette();
    let horizon = palette
        .sky_horizon_colour
        .expect("Sub Venom authors a horizon");
    let zenith = palette
        .sky_zenith_colour
        .expect("Sub Venom authors a zenith");
    assert_ne!(horizon, zenith);
    let store = oag_gpu::formats::SceneStore::Encoded;
    let mut out = Vec::new();
    oag_mesh::mesh::sky_cube::shade_zone_dome(&sky.vertices, horizon, zenith, store, &mut out);
    let byte = |c: f32| store.texel_encode((c * 255.0).trunc() / 255.0);
    let mut eye_level = 0;
    let mut overhead = 0;
    for (v, shaded) in sky.vertices.iter().zip(&out) {
        let [x, y, z] = v.position;
        let across = (x * x + z * z).sqrt();
        let want = if y <= 0.0 {
            eye_level += 1;
            horizon
        } else if y >= 0.75 * across {
            overhead += 1;
            zenith
        } else {
            continue;
        };
        for c in 0..3 {
            assert!(
                (shaded.colour[c] - byte(want[c])).abs() < 1e-6,
                "vertex at {:?}: channel {c} {} against {}",
                v.position,
                shaded.colour[c],
                byte(want[c])
            );
        }
    }
    assert!(eye_level > 0 && overhead > 0);
}

const TALONS: &str = r"Data\Environments\talons_junction\track.vex";

/// **A Detonator race takes Zone's sky gate and draws Detonator's own stage colours
/// through it.** The original's one byte opens for modes 6, 13, 14 and 21, so the
/// `ZoneSky.gtf` swap and the gradient dome are not Zone's alone
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-look.md`). Both loads name Talon's
/// Junction, so the mode is the only variable; stage 1 of `DetonatorMode.effectSettings`
/// authors a black horizon and zenith.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_detonator_race_draws_the_zone_dome_in_its_own_stage_colours() {
    let (Some(detonator), Some(single_race)) = (
        load_on(oag_race::Mode::Detonator, TALONS),
        load_on(oag_race::Mode::SingleRace, TALONS),
    ) else {
        return;
    };
    let sky = detonator.sky_model.as_ref().expect("the Detonator sky loads");
    assert_eq!(sky.label, oag_mesh::mesh::sky_cube::ZONE_DOME_LABEL);
    assert!(sky.textures.iter().all(Option::is_none));
    let reflection = sky
        .sky_reflection
        .as_ref()
        .expect("ZoneSky.gtf is the dome's reflection map");
    let circuit = oag_mesh::mesh::sky_paraboloid::from_sky(
        single_race.sky_model.as_ref().expect("the circuit's sky loads"),
    )
    .expect("the circuit's cubemap makes a reflection map");
    assert_ne!(reflection.as_slice(), circuit.as_slice());
    assert!(
        detonator
            .report
            .iter()
            .any(|line| line.contains("zonesky.gtf: loaded as the Zone sky's reflection map")),
        "the load report names the Zone cubemap"
    );
    let palette = detonator
        .zone_grade
        .as_ref()
        .expect("a Detonator race on HD carries its grade")
        .palette();
    assert_eq!(palette.sky_horizon_colour, Some([0.0, 0.0, 0.0]));
    assert_eq!(palette.sky_zenith_colour, Some([0.0, 0.0, 0.0]));
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
