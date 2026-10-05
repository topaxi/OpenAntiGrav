//! The blend equation a Wipeout HD material authors, inherited by the two titles
//! whose materials do not.
//!
//! Wipeout: Omega Collection (PS4) and Wipeout 2048 (Vita) carry HD's own
//! **state word** on every material (see [`super::material::Material::state`])
//! but no factor pair: Omega's header holds none, and the one place in 2048's
//! executable that creates a fragment program (`FUN_812f6bee`,
//! `/2048/eboot-vita-2048-eu-v104.elf`) takes its `SceGxmBlendInfo` from the
//! caller, none of whose 22 call sites is the model-material pass that was
//! located. See `docs/formats/rcsmaterial.md`, "Additive families".
//!
//! **What is inherited, and why it is not a name heuristic.** The same
//! `.rcsmaterial` names exist on all three discs (`emissive_bloom`,
//! `hd_enginetrail`, `glass_texture`, ...), HD authors the pair beside the
//! state word, and HD's pair is a function of the name in all but 3 of 184
//! names. Each row below is HD's own authored pair for a name that Omega or
//! 2048 draws in mode 1 (blended), and a name HD itself authors two pairs for
//! (`basicalpha`, `lambert`, `dc_lightcone`) is left out. A name HD authors as
//! the default `SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA` is left out too: that is what
//! an unlisted name draws. **Inherited from HD, not measured on Omega or 2048**;
//! a name only those two titles have is not here, and draws alpha-over, chosen.
//! `crates/rcs/tests/hd_lineage_blend_ground_truth.rs` re-derives the table from
//! the three discs and fails when a row is dropped, added or changed.

use crate::rcsmodel::material::{
    FACTOR_ONE, FACTOR_ONE_MINUS_SRC_ALPHA, FACTOR_SRC_ALPHA, FACTOR_SRC_COLOUR, Factor,
};

/// `(material name, source factor, destination factor)` in HD's own RSX values,
/// names lower case without the `.rcsmaterial` extension, sorted for a binary search.
pub const INHERITED: &[(&str, u16, u16)] = &[
    ("cf_add_point", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("cf_glow_tube", FACTOR_ONE, FACTOR_ONE),
    ("cf_laserrail", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("cf_plasma_glow", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("cf_plasma_glow2", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("cf_plasma_glow3", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("cf_startbeam_glow", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("dc_hologramsigns", FACTOR_ONE, FACTOR_ONE),
    ("dc_hologramwithstatic", FACTOR_SRC_ALPHA, FACTOR_ONE),
    (
        "detonator_bomb_explosion_range",
        FACTOR_SRC_ALPHA,
        FACTOR_ONE,
    ),
    (
        "detonator_bomb_explosion_shockwave",
        FACTOR_SRC_COLOUR,
        FACTOR_ONE,
    ),
    ("detonator_bomb_lightrays", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("detonator_bomb_rays_plain", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("detonator_deathshell", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("detonator_emissive_bloom", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("detonator_hd_absorbinternal", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("detonator_shield_plasma", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("detonator_shield_plasma1", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("dg_zonelights1", FACTOR_SRC_ALPHA, FACTOR_SRC_COLOUR),
    ("electricity", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("emissive_bloom", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("emissive_constant", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("emissive_lights", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("empgauge_rays", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("explosion_kaleidoscopic", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("flame_test", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("glass_texture", FACTOR_ONE, FACTOR_ONE_MINUS_SRC_ALPHA),
    (
        "glass_texture_clamped",
        FACTOR_ONE,
        FACTOR_ONE_MINUS_SRC_ALPHA,
    ),
    (
        "glass_texture_customr",
        FACTOR_ONE,
        FACTOR_ONE_MINUS_SRC_ALPHA,
    ),
    ("glass_texture_n", FACTOR_ONE, FACTOR_ONE_MINUS_SRC_ALPHA),
    (
        "glass_texture_wrecked",
        FACTOR_ONE,
        FACTOR_ONE_MINUS_SRC_ALPHA,
    ),
    ("hd_absorbinternal", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_bomb_halo", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_bombfire_bloomring", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_bombfire_shockwaves_glow", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_detonator_cannonbolt_halo", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_enginetrail", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_enginetrail_bluered", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_leachbeam", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_leachbeam_ball_glow", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_leachbeam_bloomring", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_mine_beacon_rays", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_mine_halo", FACTOR_SRC_ALPHA, FACTOR_ONE),
    (
        "hd_missile_explosion_core_glow",
        FACTOR_SRC_ALPHA,
        FACTOR_ONE,
    ),
    (
        "hd_missile_explosion_lightrays_glow",
        FACTOR_SRC_ALPHA,
        FACTOR_ONE,
    ),
    (
        "hd_missile_explosion_shockwaves_glow",
        FACTOR_SRC_ALPHA,
        FACTOR_ONE,
    ),
    ("hd_muzzleflash", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_plasmahalo_glow", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_plasmaring_glow", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hd_waketrail", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hexagonalshield", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hexagonalshield_alpha", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hexagonalshield_rich", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("hologram", FACTOR_SRC_COLOUR, FACTOR_SRC_ALPHA),
    ("holographic_projector2", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("holographic_test", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("leacheffectmat", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("lightbarrierframeglow", FACTOR_SRC_ALPHA, FACTOR_ONE),
    (
        "lightbarrierpanel_new_additive",
        FACTOR_SRC_ALPHA,
        FACTOR_ONE,
    ),
    (
        "lightbarrierpanel_new_additive_rays",
        FACTOR_SRC_ALPHA,
        FACTOR_ONE,
    ),
    ("lightbarriershockwave", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("loopmaterial", FACTOR_ONE, FACTOR_ONE),
    ("mt_additive_glow", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("mt_additive_glow_outline", FACTOR_SRC_ALPHA, FACTOR_ONE),
    ("nr_billboardholographicscanlines", FACTOR_ONE, FACTOR_ONE),
    ("scanlinebillboard", FACTOR_ONE, FACTOR_ONE),
    ("scanlinebillboard_desaturate", FACTOR_ONE, FACTOR_ONE),
    ("uv_distortion", FACTOR_SRC_COLOUR, FACTOR_ONE),
    ("wes_billboardholographicscanlines", FACTOR_ONE, FACTOR_ONE),
    ("zonebattle_shield", FACTOR_SRC_ALPHA, FACTOR_ONE),
];

/// A material's own name from its `.rcsmaterial` path, as [`INHERITED`] keys it.
fn stem(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim_end_matches(".rcsmaterial")
        .to_ascii_lowercase()
}

/// The factor pair HD authors for a material of this name, or `None` where the
/// name is not one of [`INHERITED`]'s - which is not a statement that the
/// default fits it, only that HD's pair is unknown or is the default.
#[must_use]
pub fn inherited(material_path: &str) -> Option<(Factor, Factor)> {
    let stem = stem(material_path);
    let at = INHERITED
        .binary_search_by(|row| row.0.cmp(stem.as_str()))
        .ok()?;
    let (_, src, dst) = INHERITED[at];
    Some((Factor::from_rsx(src)?, Factor::from_rsx(dst)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_and_every_row_reads() {
        assert!(INHERITED.windows(2).all(|w| w[0].0 < w[1].0));
        for (name, ..) in INHERITED {
            assert!(inherited(name).is_some(), "{name}");
        }
        assert!(
            INHERITED
                .iter()
                .all(|r| (r.1, r.2) != (FACTOR_SRC_ALPHA, FACTOR_ONE_MINUS_SRC_ALPHA))
        );
    }

    #[test]
    fn a_path_reads_by_its_stem() {
        let path = "Data/art/published/shared/EMISSIVE_BLOOM.rcsmaterial";
        assert_eq!(inherited(path), Some((Factor::SrcAlpha, Factor::One)));
        assert_eq!(
            inherited("data\\x\\glass_texture.rcsmaterial"),
            Some((Factor::One, Factor::OneMinusSrcAlpha))
        );
        assert_eq!(inherited("basicalpha.rcsmaterial"), None);
        assert_eq!(inherited("nothing_like_it.rcsmaterial"), None);
    }
}
