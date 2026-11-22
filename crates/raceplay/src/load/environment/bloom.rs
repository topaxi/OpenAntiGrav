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
        feedback: 0.0,
        final_half_texel: false,
        eight_bit: false,
        zoom: None,
        store: oag_gpu::formats::SceneStore::Computed,
    })
}

/// What the race's scene target stores for `title`, told before the circuit's staging is read
/// (a model's baked colours depend on it): HD's chain stores encoded values where the title
/// blends them so and the circuit has a `.rcsmodel` to feed it, 2048's `wo_*` chain its
/// gamma-2 space, and anything else the value as computed. The same answer
/// `scene::hd_chain::store` gives once the chains are built.
pub(in crate::load) fn scene_store(
    title: &oag_title::Title,
    track_blob: &[u8],
    ribbon: bool,
) -> oag_gpu::formats::SceneStore {
    store_of(
        title,
        oag_vex::vex::geometry_is_external(track_blob) && !ribbon,
    )
}

pub(super) fn store_of(
    title: &oag_title::Title,
    has_rcsmodel: bool,
) -> oag_gpu::formats::SceneStore {
    use oag_gpu::formats::SceneStore;
    if title.looks.wo_composite.applies_everywhere() {
        SceneStore::Squared
    } else if has_rcsmodel && title.looks.blends_encoded.applies_everywhere() {
        SceneStore::Encoded
    } else if has_rcsmodel && title.looks.texels_srgb.applies_everywhere() {
        SceneStore::SrgbTexels
    } else {
        SceneStore::Computed
    }
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
        store: store_of(title, true),
        ..params
    }
}

/// `params` as a Zone race of a title with a [`oag_title::ZoneFrame`] runs them: the
/// engine's own values over the circuit's, the adaption rate and the `Tone` family kept.
/// A Detonator race takes the same table but keeps the circuit's frame contribution and
/// adaption boost, where the title has that arm
/// ([`oag_title::ZoneFrame::detonator_arm`]). Any other mode, and a title without the
/// table, keeps the circuit's.
pub(in crate::load) fn for_mode(
    params: oag_post::hd_bloom::Params,
    race: &oag_title::RaceDefaults,
    mode: oag_race::Mode,
) -> oag_post::hd_bloom::Params {
    match race.zone_frame {
        Some(zone) if mode == oag_race::Mode::Detonator && zone.detonator_arm => {
            oag_post::hd_bloom::Params {
                frame_exponent: zone.frame_exponent,
                alpha_contribution: zone.alpha_contribution,
                vertical_size: zone.vertical_size,
                horizontal_size: zone.horizontal_size,
                feedback: zone.feedback,
                ..params
            }
        }
        Some(zone) if mode == oag_race::Mode::Zone => oag_post::hd_bloom::Params {
            frame_contribution: zone.frame_contribution,
            frame_exponent: zone.frame_exponent,
            alpha_contribution: zone.alpha_contribution,
            adaption_boost: zone.adaption_boost,
            vertical_size: zone.vertical_size,
            horizontal_size: zone.horizontal_size,
            feedback: zone.feedback,
            ..params
        },
        _ => params,
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
            feedback: 0.0,
            final_half_texel: false,
            eight_bit: false,
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

    /// Each title's scene store: HD's encoded, 2048's squared, Omega's sRGB-decoded texels over
    /// the value as computed, Pulse's the value as computed.
    #[test]
    fn the_scene_store_follows_the_titles_rules() {
        use oag_gpu::formats::SceneStore;
        assert_eq!(store_of(oag_hd::TITLE, true), SceneStore::Encoded);
        assert_eq!(store_of(oag_2048::TITLE, true), SceneStore::Squared);
        assert_eq!(store_of(oag_omega::TITLE, true), SceneStore::SrgbTexels);
        assert_eq!(store_of(oag_pulse::TITLE, false), SceneStore::Computed);
    }

    /// A Zone race on HD runs the engine's own bloom block (the feedback, the sizes, the
    /// boost), any other mode and any title without the table the circuit's.
    #[test]
    fn a_zone_race_runs_the_titles_own_bloom_block() {
        let zone = for_mode(authored(), oag_hd::TITLE.race, oag_race::Mode::Zone);
        assert_eq!(zone.feedback, -0.4);
        assert_eq!((zone.vertical_size, zone.horizontal_size), (1.0, 1.0));
        assert_eq!(zone.adaption_boost, 5.0);
        assert_eq!(
            (
                zone.frame_contribution,
                zone.frame_exponent,
                zone.alpha_contribution
            ),
            (0.5, 0.1, 1.0)
        );
        // The adaption rate and the Tone family stay the circuit's.
        assert_eq!(zone.adaption_rate, authored().adaption_rate);
        assert_eq!(zone.tone_adaption_boost, authored().tone_adaption_boost);
        assert_eq!(
            for_mode(authored(), oag_hd::TITLE.race, oag_race::Mode::SingleRace),
            authored()
        );
        assert_eq!(
            for_mode(authored(), oag_pulse::TITLE.race, oag_race::Mode::Zone),
            authored()
        );
    }

    /// A Detonator race takes the table's feedback, sizes, frame exponent and alpha
    /// contribution but keeps the circuit's frame contribution and adaption boost; every
    /// value here differs from the table's, so a taken one and a kept one cannot be
    /// confused. A title without the arm, and every other mode, keeps the circuit's.
    #[test]
    fn a_detonator_race_takes_the_table_but_keeps_the_circuits_gain() {
        let circuit = oag_post::hd_bloom::Params {
            alpha_contribution: 0.7,
            frame_contribution: 0.9,
            frame_exponent: 0.8,
            horizontal_size: 2.0,
            vertical_size: 3.0,
            adaption_boost: 2.5,
            feedback: 0.3,
            ..authored()
        };
        let detonator = for_mode(circuit, oag_hd::TITLE.race, oag_race::Mode::Detonator);
        assert_eq!(detonator.feedback, -0.4);
        assert_eq!((detonator.vertical_size, detonator.horizontal_size), (1.0, 1.0));
        assert_eq!(detonator.frame_exponent, 0.1);
        assert_eq!(detonator.alpha_contribution, 1.0);
        assert_eq!(detonator.frame_contribution, 0.9);
        assert_eq!(detonator.adaption_boost, 2.5);
        for title in [oag_2048::TITLE, oag_omega::TITLE, oag_pulse::TITLE] {
            assert_eq!(for_mode(circuit, title.race, oag_race::Mode::Detonator), circuit);
        }
    }
}
