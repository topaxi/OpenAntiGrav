//! HD's `HDR and Bloom` block, read for [`oag_post::hd_bloom::Params`].

use oag_tables::envsettings::{
    BLOOM_ADAPTION_BOOST, BLOOM_ADAPTION_RATE, BLOOM_ALPHA_CONTRIBUTION, BLOOM_FRAME_CONTRIBUTION,
    BLOOM_FRAME_EXPONENT, BLOOM_HORIZONTAL_SIZE, BLOOM_VERTICAL_SIZE, TONE_ADAPTION_BOOST,
    TONE_DARKENING_CLAMP, TONE_MAXIMUM_BRIGHTNESS,
};

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
    let staged = staged_envsettings(archives, track, report)?;
    let env = &staged.merged;
    let name = staged.label();
    let Some(params) = hd_bloom_params(env) else {
        report.push(format!(
            "{name}: no complete HDR and Bloom block even carrying the front end's own; the \
             race draws without the read bloom chain"
        ));
        return None;
    };
    let carried = staged.carried(&BLOOM_KEYS);
    let provenance = if carried.is_empty() {
        String::new()
    } else {
        format!(
            " ({} carried from the front end's {FRONT_END_ENVSETTINGS})",
            carried.join(", ")
        )
    };
    report.push(format!(
        "{name}: bloom gate alpha x{}, lum^{} \
         x{} faded by adaptation (rate {}, boost \
         {}), blur steps {}/{}, exposure \
         {} - min(adapted x{}, \
         {}) - formulas read from the executable's own \
         FunkLayerBloom microcode and its PPU chain runner{provenance}",
        params.alpha_contribution,
        params.frame_exponent,
        params.frame_contribution,
        params.adaption_rate,
        params.adaption_boost,
        params.horizontal_size,
        params.vertical_size,
        params.tone_maximum_brightness,
        params.tone_adaption_boost,
        params.tone_darkening_clamp,
    ));
    Some(params)
}

/// The `HDR and Bloom` keys [`hd_bloom_params`] needs, for naming what a circuit
/// carried from the front end.
const BLOOM_KEYS: [&str; 10] = [
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

/// A table's `HDR and Bloom` block as the chain's parameters, or `None` if a key
/// is missing. A front-end backdrop reads the same block as a race.
pub fn hd_bloom_params(
    env: &oag_tables::envsettings::EnvSettings,
) -> Option<oag_post::hd_bloom::Params> {
    Some(oag_post::hd_bloom::Params {
        alpha_contribution: env.scalar(BLOOM_ALPHA_CONTRIBUTION)?,
        frame_contribution: env.scalar(BLOOM_FRAME_CONTRIBUTION)?,
        frame_exponent: env.scalar(BLOOM_FRAME_EXPONENT)?,
        horizontal_size: env.scalar(BLOOM_HORIZONTAL_SIZE)?,
        vertical_size: env.scalar(BLOOM_VERTICAL_SIZE)?,
        adaption_rate: env.scalar(BLOOM_ADAPTION_RATE)?,
        adaption_boost: env.scalar(BLOOM_ADAPTION_BOOST)?,
        tone_adaption_boost: env.scalar(TONE_ADAPTION_BOOST)?,
        tone_darkening_clamp: env.scalar(TONE_DARKENING_CLAMP)?,
        tone_maximum_brightness: env.scalar(TONE_MAXIMUM_BRIGHTNESS)?,
        final_half_texel: false,
        zoom: None,
        store: oag_gpu::formats::SceneStore::Computed,
    })
}

/// `params` with what the title decides: its zoom ring, what its scene target stores and
/// whether its final composite reads half a texel off.
pub(in crate::load) fn for_title(
    params: oag_post::hd_bloom::Params,
    title: &oag_title::Title,
) -> oag_post::hd_bloom::Params {
    oag_post::hd_bloom::Params {
        zoom: title.race.zoom_ring.map(crate::zoom::tuning),
        final_half_texel: title.looks.final_composite_half_texel.applies_everywhere(),
        store: if title.looks.blends_encoded.applies_everywhere() {
            oag_gpu::formats::SceneStore::Encoded
        } else {
            oag_gpu::formats::SceneStore::Computed
        },
        ..params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authored() -> oag_post::hd_bloom::Params {
        oag_post::hd_bloom::Params {
            alpha_contribution: 1.0,
            frame_contribution: 1.0,
            frame_exponent: 1.0,
            horizontal_size: 1.0,
            vertical_size: 1.0,
            adaption_rate: 0.1,
            adaption_boost: 1.0,
            tone_adaption_boost: 1.0,
            tone_darkening_clamp: 1.0,
            tone_maximum_brightness: 1.0,
            final_half_texel: false,
            zoom: None,
            store: oag_gpu::formats::SceneStore::Computed,
        }
    }

    /// HD's race chain reads the final composite half a texel off, and a title whose
    /// rule is unread does not.
    #[test]
    fn the_half_texel_follows_the_titles_rule() {
        assert!(for_title(authored(), oag_hd::TITLE).final_half_texel);
        assert!(!for_title(authored(), oag_pulse::TITLE).final_half_texel);
    }
}
