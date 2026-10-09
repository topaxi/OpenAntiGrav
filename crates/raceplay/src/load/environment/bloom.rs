//! HD's `HDR and Bloom` block, read for [`oag_post::hd_bloom::Params`].

use super::*;

/// The `HDR and Bloom` values a Wipeout HD circuit authors, or `None` with
/// the reason reported.
///
/// These are the parameters the engine patches into the read
/// `FunkLayerBloom` gate and blur programs - the formulas live in
/// `oag_post::hd_bloom`, every one of them the microcode's own.
///
/// **Reads through [`staged_envsettings`], so a circuit's own file only has
/// to declare what it changes.** Every Fury/DLC circuit's own file authors
/// `Tone adaption boost` and omits `Tone darkening clamp`/`Tone maximum
/// brightness`; a live read on Sol 2 found the running settings singleton
/// holding all three at the front end's own `(20, 3, 4)` regardless - see
/// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "The resolve's Fury
/// circuits carry the front end's own Tone triple". Before this, a file
/// without the *whole* ten-key set drew without the chain at all rather than
/// with the front end's carried values - the gap that left `[graphics]
/// bloom` inert on eleven of sixteen circuits (the eight Fury/DLC ones here,
/// plus `zone_2`/`zone_3`/`zone_4`, which ship no `.envsettings` at all). A
/// circuit whose merged table is still incomplete draws without the chain,
/// same as before.
pub(in crate::load) fn envsettings_bloom(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<oag_post::hd_bloom::Params> {
    use oag_tables::envsettings::{
        BLOOM_ADAPTION_BOOST, BLOOM_ADAPTION_RATE, BLOOM_ALPHA_CONTRIBUTION,
        BLOOM_FRAME_CONTRIBUTION, BLOOM_FRAME_EXPONENT, BLOOM_HORIZONTAL_SIZE, BLOOM_VERTICAL_SIZE,
        TONE_ADAPTION_BOOST, TONE_DARKENING_CLAMP, TONE_MAXIMUM_BRIGHTNESS,
    };
    const KEYS: [&str; 10] = [
        BLOOM_ALPHA_CONTRIBUTION,
        BLOOM_FRAME_CONTRIBUTION,
        BLOOM_FRAME_EXPONENT,
        BLOOM_HORIZONTAL_SIZE,
        BLOOM_VERTICAL_SIZE,
        BLOOM_ADAPTION_RATE,
        BLOOM_ADAPTION_BOOST,
        TONE_ADAPTION_BOOST,
        TONE_DARKENING_CLAMP,
        TONE_MAXIMUM_BRIGHTNESS,
    ];
    let staged = staged_envsettings(archives, track, report)?;
    let env = &staged.merged;
    let name = staged.label();
    let (
        Some(alpha_contribution),
        Some(frame_contribution),
        Some(frame_exponent),
        Some(horizontal_size),
        Some(vertical_size),
        Some(adaption_rate),
        Some(adaption_boost),
        Some(tone_adaption_boost),
        Some(tone_darkening_clamp),
        Some(tone_maximum_brightness),
    ) = (
        env.scalar(BLOOM_ALPHA_CONTRIBUTION),
        env.scalar(BLOOM_FRAME_CONTRIBUTION),
        env.scalar(BLOOM_FRAME_EXPONENT),
        env.scalar(BLOOM_HORIZONTAL_SIZE),
        env.scalar(BLOOM_VERTICAL_SIZE),
        env.scalar(BLOOM_ADAPTION_RATE),
        env.scalar(BLOOM_ADAPTION_BOOST),
        env.scalar(TONE_ADAPTION_BOOST),
        env.scalar(TONE_DARKENING_CLAMP),
        env.scalar(TONE_MAXIMUM_BRIGHTNESS),
    )
    else {
        report.push(format!(
            "{name}: no complete HDR and Bloom block even carrying the front end's own; the \
             race draws without the read bloom chain"
        ));
        return None;
    };
    let carried = staged.carried(&KEYS);
    let provenance = if carried.is_empty() {
        String::new()
    } else {
        format!(
            " ({} carried from the front end's {FRONT_END_ENVSETTINGS})",
            carried.join(", ")
        )
    };
    report.push(format!(
        "{name}: bloom gate alpha x{alpha_contribution}, lum^{frame_exponent} \
         x{frame_contribution} faded by adaptation (rate {adaption_rate}, boost \
         {adaption_boost}), blur steps {horizontal_size}/{vertical_size}, exposure \
         {tone_maximum_brightness} - min(adapted x{tone_adaption_boost}, \
         {tone_darkening_clamp}) - formulas read from the executable's own \
         FunkLayerBloom microcode and its PPU chain runner{provenance}"
    ));
    Some(oag_post::hd_bloom::Params {
        alpha_contribution,
        frame_contribution,
        frame_exponent,
        horizontal_size,
        vertical_size,
        adaption_rate,
        adaption_boost,
        tone_adaption_boost,
        tone_darkening_clamp,
        tone_maximum_brightness,
        zoom: None,
    })
}
