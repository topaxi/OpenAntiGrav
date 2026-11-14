//! Anulpha Pass's walls, floor plates and chevrons on 2048 and Omega: the light multiplies the
//! albedo's alpha and its colour tints the specular, as on HD ([`crate::mesh::slots::LIT_ALPHA`]).
//!
//! Read on each title's own programs (`docs/rendering/hd-track-tone.md`, "2048 and Omega"):
//!
//! - **2048, USSE**: every lit permutation of `reversed_diffuse_specular_normal`,
//!   `cf_chevron_pulse` and `cf_chevron_pulse_time` ends `mul light, T.www` then `mad
//!   spec * T.rgb * T.rgb * pow, +that` (`reversed_diffuse_specular_normal` #3 words 21 to 25,
//!   lightmapped #16 words 20 to 31).
//! - **Omega, GCN**: `track_surface_displaced2out` (the walls and floor plates) and
//!   `cf_chevron_pulse_displacement2out` sample the albedo with `dmask:0xf`, multiply its RGB by
//!   `liveLighting0specular` and the specular power, and add `light * T.a` (`data01` `@0xf7c4`
//!   and `@0x105c4`; the patch's `data08` recompiles, `@0x10204` and `@0x10fc4`, the same).
//!
//! Both titles ship these files under Anulpha Pass alone. The chevron programs also add an unlit
//! `max(E.rgb, Colour * mix(E.rgb, wave.rgb, E.a))` off the emissive mask and the scrolled wave,
//! which is not HD's emissive and is not drawn here.

use oag_rcs::rcsmodel::psp2::material::Material;

use super::refraction::file_stem;

/// The `.rcsmaterial` stems whose lit programs light the albedo's alpha.
const FAMILIES: [&str; 5] = [
    "reversed_diffuse_specular_normal",
    "cf_chevron_pulse",
    "cf_chevron_pulse_time",
    "track_surface_displaced2out",
    "cf_chevron_pulse_displacement2out",
];

/// Whether `material`'s lit program lights its albedo's alpha. Only where the material binds a
/// picture as its albedo: the flag reads that picture's alpha as the surface.
pub(super) fn admits(material: &Material) -> bool {
    let stem = file_stem(material);
    material.diffuse_texture().is_some_and(|t| !t.is_empty())
        && FAMILIES.iter().any(|f| stem.eq_ignore_ascii_case(f))
}
