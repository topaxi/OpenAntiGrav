//! The spectrum a capture feeds the Zone visualiser, split out of `race_capture.rs`
//! under the size ratchet.

use super::CaptureOptions;

/// The spectrum to feed `Scene::render` with: a fixed, deterministic ramp
/// under [`CaptureOptions::zone_spectrum_test`], or [`Output::spectrum`]'s
/// live one otherwise. See that field's own doc comment for why a capture
/// wants the override.
///
/// [`Output::spectrum`]: oag_audio::Output::spectrum
pub(super) fn zone_spectrum(
    options: &CaptureOptions,
    audio: &oag_sound::Audio,
) -> [f32; oag_audio::BANDS] {
    if options.zone_spectrum_test {
        std::array::from_fn(|i| {
            #[expect(clippy::cast_precision_loss, reason = "BANDS is 16, so exact")]
            let t = i as f32 / (oag_audio::BANDS - 1) as f32;
            t
        })
    } else {
        audio.output().spectrum().levels()
    }
}
