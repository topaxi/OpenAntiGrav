//! `graphics.reflection_resolution` sizes Wipeout HD's screen-space glass and sea target.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted PS3 disc image under
//! `data/images/` - see `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^hd_reflection_resolution_ground_truth::/)'
//! ```
//!
//! Vineta K's tunnel glass reads the behind-the-glass target (640x360) and Moa Therma's sea the
//! planar reflection target (960x540). `original` keeps those sizes at every render scale;
//! `scaled` gives the original's size at 100 and half of it per axis at 50. Drop the load's
//! resize (`load::view`) and every `scaled` row at 50 fails; resize without the value and the
//! `original` rows at 50 fail. One load per test, so the matrix is the test axis.

use oag_display::display::{ReflectionResolution, Scale};
use oag_raceplay as race;

const VINETA_K: &str = "/data/environments/01_vineta_k/track.vex";
const MOA_THERMA: &str = "/data/environments/03_track/track.vex";

fn target_size(track: &str, resolution: ReflectionResolution, percent: u32) -> Option<(u32, u32)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
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
            if let Some(size) = target_size($track, ReflectionResolution::$resolution, $percent) {
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
