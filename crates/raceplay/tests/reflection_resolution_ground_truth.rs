//! `graphics.reflection_resolution` sizes the screen-space glass and sea target.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted PS3 disc image under
//! `data/images/` and the extracted 2048 and Omega packages under `data/extracted/` - see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^reflection_resolution_ground_truth::/)'
//! ```
//!
//! HD's Vineta K tunnel glass reads the behind-the-glass target (640x360) and Moa Therma's sea
//! the planar reflection target (960x540); 2048's Vineta K reads the refraction target at
//! 480x272 and Omega's at 960x540. `original` keeps those sizes, `half` and `quarter` take that
//! fraction of each axis. The load has no render scale to read, so the size cannot follow it.
//! Drop the load's `resolution` (`load::view`) and every `half` and `quarter` row fails. One
//! load per test, so the matrix is the test axis.

use oag_display::display::ReflectionResolution;
use oag_raceplay as race;

const VINETA_K: &str = "/data/environments/01_vineta_k/track.vex";
const MOA_THERMA: &str = "/data/environments/03_track/track.vex";

const VITA_2048: &str = "data/extracted/vita/PCSF00007";
const OMEGA: &str = "data/extracted/ps4";
const VINETA_K_ON_2048: &str = r"Data\art\published\DLC1\environments\Vineta_K\track.vex";
const VINETA_K_ON_OMEGA: &str = r"Data\environments\01_vineta_k\track.vex";

fn hd_target_size(track: &str, resolution: ReflectionResolution) -> Option<(u32, u32)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    target_size(&image, track, resolution)
}

fn target_size(
    source: &std::path::Path,
    track: &str,
    resolution: ReflectionResolution,
) -> Option<(u32, u32)> {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(track.to_string()),
        reflection_resolution: resolution,
        ..race::Options::default()
    })
    .unwrap_or_else(|why| panic!("loading {track}: {why:#}"));
    let behind = &loaded.behind_glass;
    assert!(
        !behind.groups.is_empty() || behind.mirror.is_some(),
        "{track} builds a target"
    );
    Some(behind.drawn_size())
}

macro_rules! sizes {
    ($($name:ident: $track:ident, $resolution:ident => ($w:literal, $h:literal);)*) => {$(
        #[test]
        #[ignore = "needs a decrypted PS3 disc image in data/images"]
        fn $name() {
            if let Some(size) = hd_target_size($track, ReflectionResolution::$resolution) {
                assert_eq!(size, ($w, $h));
            }
        }
    )*};
}

sizes! {
    vineta_k_original: VINETA_K, Original => (640, 360);
    vineta_k_half: VINETA_K, Half => (320, 180);
    vineta_k_quarter: VINETA_K, Quarter => (160, 90);
    moa_therma_original: MOA_THERMA, Original => (960, 540);
    moa_therma_half: MOA_THERMA, Half => (480, 270);
    moa_therma_quarter: MOA_THERMA, Quarter => (240, 135);
}

macro_rules! psp2_sizes {
    ($($name:ident: $source:ident, $track:ident, $resolution:ident => ($w:literal, $h:literal);)*) => {$(
        #[test]
        #[ignore = "needs the extracted 2048 and Omega packages in data/extracted"]
        fn $name() {
            let Some(source) = oag_testdata::exact($source) else {
                return;
            };
            let size = target_size(&source, $track, ReflectionResolution::$resolution);
            assert_eq!(size, Some(($w, $h)));
        }
    )*};
}

psp2_sizes! {
    vita_2048_vineta_k_original: VITA_2048, VINETA_K_ON_2048, Original => (480, 272);
    vita_2048_vineta_k_half: VITA_2048, VINETA_K_ON_2048, Half => (240, 136);
    vita_2048_vineta_k_quarter: VITA_2048, VINETA_K_ON_2048, Quarter => (120, 68);
    omega_vineta_k_original: OMEGA, VINETA_K_ON_OMEGA, Original => (960, 540);
    omega_vineta_k_half: OMEGA, VINETA_K_ON_OMEGA, Half => (480, 270);
    omega_vineta_k_quarter: OMEGA, VINETA_K_ON_OMEGA, Quarter => (240, 135);
}
