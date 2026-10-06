//! What Pulse on the PS2 draws that the other sources do not: the bloom's glow
//! mask, measured off GS dumps of the original on PCSX2 - see
//! `docs/rendering/ps2-bloom.md`.

use super::Loaded;

/// Whether this race is Pulse off a PS2 disc.
pub(super) fn is_pulse_ps2(title: &oag_title::Title, archives: &oag_assets::Archives) -> bool {
    title.looks.ps2_glow_mask.applies(archives.layout.platform)
}

/// Marks every `.vex` model the race draws as stamping the mask by the PS2's
/// rule. A no-op when `pulse_ps2` is false.
pub(super) fn finish(loaded: &mut Loaded, pulse_ps2: bool) {
    if !pulse_ps2 {
        return;
    }
    super::glow_mask::stamp_models(loaded, true);
    loaded.report.push(
        "glow mask: PS2's rule - a batch with the glow bits stamps its fragments' own alpha \
         (texel times vertex colour), every other batch stamps nothing; the bloom runs the PS2's \
         own chain"
            .into(),
    );
}
