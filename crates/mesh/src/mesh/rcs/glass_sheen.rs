//! The glass family's facing-ramp combine, fully traced on one material and
//! not invented for any other.
//!
//! `etched_glass_tech.rcsmaterial` block #7 (Talon's Junction's glass floor)
//! is read instruction by instruction in
//! `docs/formats/rcsmaterial.md`, "Three sampler roles identified by what
//! they bind, and the glass floor stops painting a ramp": a view-angle sheen,
//! a facing-ramp texture sampled at `dot(V, N)`, mixed with a grid texture
//! sampled at the surface's own UV, whose red channel is both an additive
//! term and the output alpha. This module is the classifier that finds every
//! material shaped exactly like it, split out of `skin.rs` under the
//! 1,000-line rule in `scripts/check-file-size.py`.
//!
//! # Settled 2026-09-17: the routing question this combine used to block on
//!
//! Two hypotheses stood in this project's own open-thread tracker:
//! either the grid's own sampler word is not the binding it appears to be, or
//! the variant this project resolves is the wrong one. **Neither.** Read
//! directly off the disc (`hd_glass_sheen_census.rs --params etched_glass_tech`
//! against `talons_junction/track.vex`), the material's own sampler table is
//!
//! ```text
//! 0x94b2b285 (no preimage, the ramp) -> dc_iridescent_gradient.gtf
//! 0x3bdc0403 (Texture1)              -> glass_etched_tech.gtf   (the grid)
//! 0x37b5db58 (lightmap)              -> None
//! ```
//!
//! and the resolved lit-race variant (`fragment@0x5ef0`) declares exactly
//! `0x3bdc0403@unit0`, `0x94b2b285@unit2`, `0x9edd3243@unit1` - both real
//! textures, at the units the traced microcode samples them. The grid binds
//! **`Texture1`, never `lightmap`**; the `lightmap`-hash entry the earlier
//! hypotheses pointed at is real but carries no path (`None`), an ordinary
//! "left at whatever the engine binds" entry per
//! `oag_rcs::rcsmodel::Material::samplers`'s own doc. A pre-correction
//! off-by-one pairing (`docs/formats/rcsmaterial.md`, "Corrected hours
//! later") is what put the grid's path beside that empty entry's hash in an
//! earlier reading; the parser was fixed, and `docs/formats/rcsmaterial.md`
//! lines 892, 1292 and 1299 still carry the stale claim and were not edited
//! for it - this section is the correction, not those lines. Confidence 95:
//! read directly off the shipped record and the shipped variant, gated the
//! way "a code-level finding about this project's own renderer" already is
//! on this same page (see "Talon's Junction's magstrip floor").
//!
//! **The real gap was downstream, in `skin::picks`.** This material's colour
//! lane traces to more than one unit (`Texel::Mixed`), so the load-time
//! albedo fallback picked the grid (the one entry not on
//! `skin::NOT_A_PICTURE`'s list) for *both* of this renderer's two texture
//! bindings - the alpha lane also resolves to the grid's own unit, so `aux`
//! collapsed onto `albedo` and the ramp, the material's own primary
//! [`oag_rcs::rcsmodel::Material::texture`], was never bound to anything.
//! Not a hash mismatch; a `Pick` collision this module's [`classify`] now
//! routes around.
//!
//! # `op3D` is not unique to this material either
//!
//! `docs/formats/rcsmaterial.md` (~line 710) reads it as seen only in
//! `etched_glass_tech`. `mag_effect_loop_opaque.rcsmaterial`'s own block #7
//! carries it twice more (`@0x43`, `@0x60`, both writing the discard
//! register `R63`) - a free correction found while tracing that material's
//! own combine for comparison, below.
//!
//! # What this module does *not* draw
//!
//! `mag_effect_loop_opaque.rcsmaterial` (Talon's Junction's magstrip floor)
//! resolves to a *different* five-unit block, not this one: a fourth real
//! texture (`ds_mag_wave_c.gtf`, unit 3), the grid and the ramp on different
//! units than here (grid at unit 1, ramp at unit 4), a fog-idiom wrapper
//! (`EX2_SAT` against a `1.44269`-scaled term, the same shape
//! `scripts/ps3-microcode.py`'s own docstring names), and an alpha lane this
//! project's decoder reads as [`fragment::Texel::Untraced`] rather than a
//! resolved unit. "The same five-sampler reflective combine as
//! `etched_glass_tech`'s" (this page's own 2026-09-13 section) is true only
//! as a family resemblance - close enough to place the routing gap, not
//! close enough to reproduce. Implementing it from here would be inventing a
//! combine this project has not read, which `CLAUDE.md` rules out; it is
//! left classified but undrawn; see [`classify`]'s doc for exactly which
//! declared-sampler shape [`RAMP_HASHES`]/[`TEXTURE1_SAMPLER`]/
//! [`PARABOLOID_REFLECTION_SAMPLER`] catches instead.
//!
//! # The population, measured before being acted on
//!
//! `hd_glass_sheen_census.rs` sweeps all four PS3 archives: **6 materials,
//! 15 chunks**, three circuits - `etched_glass_tech.rcsmaterial` (Talon's
//! Junction, `tech_de_ra`) and its sibling `etched_glass.rcsmaterial`
//! (`modesto_heights`), every one keyed on ramp hash `0x94b2b285` alone. The
//! other two facing-ramp hashes (`blue_metal_facing_ramp.gtf`,
//! `tunnel_fx_facingramp.gtf`) never co-occur with `Texture1` and
//! `paraboloidReflectionTex` in one declared set, so the classifier is
//! general (keyed on shape, not on this material's name) without pulling in
//! the `blue_metal` family, whose own combine is unread.
//!
//! # The `c` term, wired 2026-10-07
//!
//! Block #7's `ADD H4.xyz, H2, {c}` adds a per-material parameter
//! (`0x512f8e65`, `[0.26562, 0.26562, 0.26562, 0]` on `etched_glass_tech`) to
//! the ramp before it multiplies the light. [`bias`] carries it in the glow
//! table, as the magstrip floor carries its own. The reflection tint
//! (`paraboloidReflectionTex`) is still left out: no probe exists here.

use oag_rcs::rcsmodel;
use oag_rcs::{rcsmaterial, rcsmaterial::fragment};

use crate::mesh::{Emissive, slots};

use super::emissive::EMISSIVE_LIMIT;

/// The block's per-material constant `c` (`ADD H4.xyz, H2, {c}`), measured
/// `[0.26562, 0.26562, 0.26562, 0]` on `etched_glass_tech`. No preimage.
const RAMP_BIAS: u32 = 0x512f_8e65;

/// Gives every glass-sheen material its `c`, as a glow-table entry whose
/// `offset` is `c`'s first lane and whose tint is zero, so the table adds
/// nothing of its own. The magstrip floor carries its own `c` the same way.
///
/// A material that already has a table entry keeps it and is left without
/// `c`; none of Talon's Junction's does.
pub(super) fn bias(model: &rcsmodel::Model, packed: &mut [u32], table: &mut Vec<Emissive>) {
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if *word & slots::FACING_RAMP_SHEEN == 0 || *word >> slots::MATERIAL_SHIFT != 0 {
            continue;
        }
        let Some(c) = material.parameters.iter().find(|p| p.hash == RAMP_BIAS) else {
            continue;
        };
        let layer = Emissive {
            tint: [0.0; 3],
            offset: c.value[0],
            scale: 1.0,
            rate: 0.0,
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        if let Some(index) = index {
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
    }
}

/// `~crc32("Texture1")`. The grid's own sampler, and the unit the traced
/// combine's alpha comes from.
pub(super) const TEXTURE1_SAMPLER: u32 = 0x3bdc_0403;

/// `~crc32("paraboloidReflectionTex")` - an engine-supplied dual-paraboloid
/// reflection probe, confirmed by neither material's sampler table naming
/// any `.gtf` for it (`docs/formats/rcsmaterial.md`, "Five more sampler
/// names by preimage"). This renderer has no such probe and invents none;
/// the reflection term the traced combine weights by it is left out.
pub(super) const PARABOLOID_REFLECTION_SAMPLER: u32 = 0x9edd_3243;

/// The three facing-ramp hashes `skin::NOT_A_PICTURE` also carries - every
/// use disc-wide binds a texture 32 texels or less in one dimension, a
/// lookup rather than a picture.
pub(super) const RAMP_HASHES: [u32; 3] = [0x3528_1c78, 0x994b_bcf1, 0x94b2_b285];

/// Which of a material's own sampler entries hold the ramp and the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GlassSheen {
    /// Index into [`rcsmodel::Material::samplers`] of the facing-ramp entry.
    pub(super) ramp_entry: usize,
    /// Index into [`rcsmodel::Material::samplers`] of the grid entry.
    pub(super) grid_entry: usize,
}

/// Whether `material`'s resolved variant is exactly the traced glass-sheen
/// combine, and if so, which of its own sampler entries are the ramp and the
/// grid.
///
/// **Fact-based, not name-based** - every check is a property of the
/// resolved program the disc itself ships, so a material sharing this exact
/// shape is caught without naming it, and one that merely shares a ramp
/// hash with a different combine (the `blue_metal` family, unread) is not:
///
/// 1. the resolved variant declares *exactly* three samplers - [`TEXTURE1_SAMPLER`],
///    one of [`RAMP_HASHES`], [`PARABOLOID_REFLECTION_SAMPLER`] - no
///    lightmap, no normal or specular map alongside them;
/// 2. the program's colour lane is [`fragment::Texel::Mixed`] - more than one
///    unit reaches it, which a flat picture never is;
/// 3. the alpha lane traces to `Texture1`'s own declared unit, matching the
///    traced `MOV H0.w, H6.xxxx END` reading the grid's red.
#[must_use]
pub(super) fn classify(
    material: &rcsmodel::Material,
    declared: &rcsmaterial::Declared,
    program: &fragment::Program,
) -> Option<GlassSheen> {
    let ramp_hash = declared
        .samplers
        .iter()
        .map(|&(h, _)| h)
        .find(|h| RAMP_HASHES.contains(h))?;
    let has_texture1 = declared
        .samplers
        .iter()
        .any(|&(h, _)| h == TEXTURE1_SAMPLER);
    let has_paraboloid = declared
        .samplers
        .iter()
        .any(|&(h, _)| h == PARABOLOID_REFLECTION_SAMPLER);
    if !(has_texture1 && has_paraboloid && declared.samplers.len() == 3) {
        return None;
    }
    let texture1_unit = declared
        .samplers
        .iter()
        .find(|&&(h, _)| h == TEXTURE1_SAMPLER)
        .map(|&(_, u)| u)?;
    let texels = program.output_texels();
    let colour = texels[0].merge(texels[1]).merge(texels[2]);
    if colour != fragment::Texel::Mixed {
        return None;
    }
    let alpha_at_texture1 = matches!(
        texels[3],
        fragment::Texel::Unit { unit, .. } if u32::from(unit) == texture1_unit
    );
    if !alpha_at_texture1 {
        return None;
    }
    let ramp_entry = material
        .samplers
        .iter()
        .position(|&(h, ref p)| h == ramp_hash && p.is_some())?;
    let grid_entry = material
        .samplers
        .iter()
        .position(|&(h, ref p)| h == TEXTURE1_SAMPLER && p.is_some())?;
    Some(GlassSheen {
        ramp_entry,
        grid_entry,
    })
}
