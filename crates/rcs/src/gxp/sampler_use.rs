//! Which samplers of a Vita material's fragment programs carry its colour and
//! which only feed its lighting, read off the programs' declared interface.
//!
//! **No instruction is decoded.** A `.rcsmaterial` holds one fragment program
//! per permutation of one shader source, and the compiler declares a sampler
//! only in a permutation that reads it. The smallest permutation declares the
//! constant ambient colour and the fog colour and no light
//! (`constantAmbientColour` without any `liveLighting*` uniform): the surface
//! with the lights off. A sampler that every lit permutation declares and that
//! one never does is therefore not part of the surface's colour: switching
//! the lights off removes it. Over `data.psarc`'s 311 families that have the
//! ambient-only permutation, 301 declare their sampler in it; nine do not
//! (`constantdiffuse_specular_normal` and its `New`, `ds_booth_glass`, the
//! `fc15`/`fc17` water, the three `dc_seawater` ones and
//! `glass_colour_spec_trans`), and those nine are exactly constant-colour
//! shaders whose texture is a normal, a specular or a reflection lookup. See
//! `docs/formats/gxp.md`, "Which sampler carries the colour".

use super::{Category, Program};

/// The uniform every permutation with lights declares at least one of, by
/// prefix.
const LIGHT_PREFIX: &str = "liveLighting";
/// Declared by the ambient-only permutation, and by the zone ones.
const AMBIENT: &str = "constantAmbientColour";
const FOG: &str = "fogColour";
/// Zone permutations declare `zone*` uniforms and samplers and are never the
/// ambient-only one.
const ZONE_PREFIX: &str = "zone";
/// Samplers the engine binds itself (the baked atlas, the shadow passes and the
/// reflection captures). They are lit-only by construction and say nothing
/// about the material's own picture.
const ENGINE_SAMPLERS: [&str; 5] = [
    "lightmap",
    "occlusionMap",
    "shadowMap",
    "paraboloidReflectionTex",
    "screenSpaceReflectionTex",
];

fn names(p: &Program, category: Category) -> Vec<&str> {
    p.parameters
        .iter()
        .filter(|q| q.category == category)
        .map(|q| q.name.as_str())
        .collect()
}

/// What the programs of one material file say about its samplers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplerUse {
    /// Sampler names the ambient-only permutation declares, engine samplers
    /// left out, in sorted order. Empty means the surface's colour comes from
    /// no texture at all.
    pub ambient_samplers: Vec<String>,
    /// Sampler names a lit permutation declares and the ambient-only one does
    /// not: not the surface's colour, in sorted order.
    pub lighting_only: Vec<String>,
    /// The one uniform the ambient-only permutation declares besides the
    /// ambient and fog colours, where there is exactly one: the constant the
    /// surface takes its colour from when its samplers are lighting only.
    pub colour_uniform: Option<String>,
}

/// Reads `data`, a whole `.rcsmaterial` file, for its ambient-only and lit
/// fragment permutations. `None` when the file has no ambient-only fragment
/// permutation (a PS4 material, whose programs are not `GXP`, among them) or
/// no lit one, since the comparison needs both.
#[must_use]
pub fn sampler_use(data: &[u8]) -> Option<SamplerUse> {
    let fragments: Vec<Program> = super::programs(data)
        .into_iter()
        .filter_map(|(_, p)| p.ok())
        .filter(Program::is_fragment)
        .collect();
    let is_lit = |p: &&Program| {
        names(p, Category::Uniform)
            .iter()
            .any(|n| n.starts_with(LIGHT_PREFIX))
    };
    let is_zone = |p: &&Program| {
        names(p, Category::Uniform)
            .iter()
            .chain(names(p, Category::Sampler).iter())
            .any(|n| n.starts_with(ZONE_PREFIX))
    };
    let is_ambient_only =
        |p: &&Program| !is_lit(p) && !is_zone(p) && names(p, Category::Uniform).contains(&AMBIENT);
    let unlit: Vec<&Program> = fragments.iter().filter(is_ambient_only).collect();
    let lit: Vec<&Program> = fragments
        .iter()
        .filter(is_lit)
        .filter(|p| !is_zone(p))
        .collect();
    if unlit.is_empty() || lit.is_empty() {
        return None;
    }
    let unlit_samplers: Vec<&str> = unlit
        .iter()
        .flat_map(|p| names(p, Category::Sampler))
        .collect();
    let mut ambient_samplers: Vec<String> = unlit_samplers
        .iter()
        .filter(|n| !ENGINE_SAMPLERS.contains(n))
        .map(|n| (*n).to_string())
        .collect();
    ambient_samplers.sort();
    ambient_samplers.dedup();
    let mut lighting_only: Vec<String> = lit
        .iter()
        .flat_map(|p| names(p, Category::Sampler))
        .filter(|n| !unlit_samplers.contains(n) && !ENGINE_SAMPLERS.contains(n))
        .map(str::to_string)
        .collect();
    lighting_only.sort();
    lighting_only.dedup();
    let mut colours: Vec<&str> = unlit
        .iter()
        .flat_map(|p| names(p, Category::Uniform))
        .filter(|n| *n != AMBIENT && *n != FOG)
        .collect();
    colours.sort_unstable();
    colours.dedup();
    let colour_uniform = (colours.len() == 1).then(|| colours[0].to_string());
    Some(SamplerUse {
        ambient_samplers,
        lighting_only,
        colour_uniform,
    })
}
