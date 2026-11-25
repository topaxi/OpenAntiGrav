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
//! 480x272 and Omega's at 960x540. `original` keeps those sizes at every render scale; `scaled`
//! gives the original's size at 100 and half of it per axis at 50. Drop the load's resize
//! (`load::view`) and every `scaled` row at 50 fails; resize without the value and the
//! `original` rows at 50 fail. One load per test, so the matrix is the test axis.

use oag_display::display::{ReflectionResolution, Scale};
use oag_raceplay as race;

const VINETA_K: &str = "/data/environments/01_vineta_k/track.vex";
const MOA_THERMA: &str = "/data/environments/03_track/track.vex";

const VITA_2048: &str = "data/extracted/vita/PCSF00007";
const OMEGA: &str = "data/extracted/ps4";
const VINETA_K_ON_2048: &str = r"Data\art\published\DLC1\environments\Vineta_K\track.vex";
const VINETA_K_ON_OMEGA: &str = r"Data\environments\01_vineta_k\track.vex";

fn hd_target_size(
    track: &str,
    resolution: ReflectionResolution,
    percent: u32,
) -> Option<(u32, u32)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    target_size(&image, track, resolution, percent)
}

fn target_size(
    source: &std::path::Path,
    track: &str,
    resolution: ReflectionResolution,
    percent: u32,
) -> Option<(u32, u32)> {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(track.to_string()),
        reflection_resolution: resolution,
        render_scale: Scale::try_from(percent).expect("an offered render scale"),
        ..race::Options::default()
    })
    .unwrap_or_else(|why| panic!("loading {track}: {why:#}"));
    let behind = &loaded.behind_glass;
    assert!(
        !behind.groups.is_empty() || behind.mirror.is_some(),
        "{track} builds a target"
    );
    Some(behind.size)
}

macro_rules! sizes {
    ($($name:ident: $track:ident, $resolution:ident, $percent:literal => ($w:literal, $h:literal);)*) => {$(
        #[test]
        #[ignore = "needs a decrypted PS3 disc image in data/images"]
        fn $name() {
            if let Some(size) = hd_target_size($track, ReflectionResolution::$resolution, $percent) {
                assert_eq!(size, ($w, $h));
            }
        }
    )*};
}

sizes! {
    vineta_k_original_at_100: VINETA_K, Original, 100 => (640, 360);
    vineta_k_original_at_50: VINETA_K, Original, 50 => (640, 360);
    vineta_k_scaled_at_100: VINETA_K, Scaled, 100 => (640, 360);
    vineta_k_scaled_at_50: VINETA_K, Scaled, 50 => (320, 180);
    moa_therma_original_at_100: MOA_THERMA, Original, 100 => (960, 540);
    moa_therma_original_at_50: MOA_THERMA, Original, 50 => (960, 540);
    moa_therma_scaled_at_100: MOA_THERMA, Scaled, 100 => (960, 540);
    moa_therma_scaled_at_50: MOA_THERMA, Scaled, 50 => (480, 270);
}

macro_rules! psp2_sizes {
    ($($name:ident: $source:ident, $track:ident, $resolution:ident, $percent:literal => ($w:literal, $h:literal);)*) => {$(
        #[test]
        #[ignore = "needs the extracted 2048 and Omega packages in data/extracted"]
        fn $name() {
            let Some(source) = oag_testdata::exact($source) else {
                return;
            };
            let size = target_size(&source, $track, ReflectionResolution::$resolution, $percent);
            assert_eq!(size, Some(($w, $h)));
        }
    )*};
}

psp2_sizes! {
    vita_2048_vineta_k_original_at_50: VITA_2048, VINETA_K_ON_2048, Original, 50 => (480, 272);
    vita_2048_vineta_k_scaled_at_50: VITA_2048, VINETA_K_ON_2048, Scaled, 50 => (240, 136);
    omega_vineta_k_original_at_50: OMEGA, VINETA_K_ON_OMEGA, Original, 50 => (960, 540);
    omega_vineta_k_scaled_at_50: OMEGA, VINETA_K_ON_OMEGA, Scaled, 50 => (480, 270);
}
