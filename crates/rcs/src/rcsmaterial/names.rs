//! Every `.rcsmaterial`/`.rcsmodel` name-hash preimage recovered so far, in one
//! lookup rather than scattered across doc prose.
//!
//! Two disjoint namespaces, both `~crc32` under [`super::name_hash`] but
//! populated from different tables and by different techniques - see
//! `docs/formats/rcsmaterial.md`, "The sampler names, by preimage" and "The
//! parameter names, by preimage", for the evidence behind each entry:
//!
//! - [`sampler_name`] answers a hash from [`super::Declared::samplers`] - what
//!   a compiled shader program's own `SHO` header declares it is fed, scoped
//!   to `DATA00.PSARC`'s fragment blocks (**125** distinct hashes there, the
//!   count `rcsmaterial.md`'s "Open" section tracks against).
//! - [`parameter_name`] answers a hash from
//!   [`crate::rcsmodel::material::Parameter::hash`] - a material *instance*'s
//!   own authored value, scoped to `DATA00.PSARC` + `DATA02.PSARC`'s
//!   `.rcsmodel` records (**300** distinct hashes there).
//!
//! Both scopes are named constants in
//! `crates/render/examples/rcs_preimage_sweep.rs`, which is what measured
//! them and is the reproducible source for every count this module's doc
//! comments cite.
//!
//! A name landing here is a **preimage** - `~crc32(name) == hash` - not a
//! resemblance; see [`super::name_hash`]'s own doc comment for the hash
//! function and `docs/ghidra/functions/ps3-hdfury-eu/renderer.md#the-name-hash-is-crc-32`
//! for where the executable was read computing it. A hash identified only by
//! *what it binds* (a normal map, a facing ramp) rather than by a recovered
//! string stays out of this table - see `rcsmaterial.md`'s "Three sampler
//! roles identified by what they bind" for those; they are real
//! identifications and still not preimages.

use super::name_hash;

/// Every sampler-name preimage recovered so far.
///
/// The first 38 are `rcsmaterial.md`'s original sweep ("The sampler names, by
/// preimage"); the rest are later, smaller batches, each cited in that page.
/// `shadowMapTex` appears once, deliberately - the later "Five more sampler
/// names" pass rediscovered it independently, which is a self-consistency
/// check on the technique rather than a second name.
pub const KNOWN_SAMPLER_NAMES: &[&str] = &[
    // Slots by number.
    "Texture1",
    "Texture2",
    "Texture3",
    // Diffuse.
    "DiffuseTexture",
    "DiffuseTexture1",
    "diffuseTexture",
    "diffuseTexture1",
    "diffuseTexture2",
    "Diffuse",
    "diffuse",
    // Surface maps.
    "NormalTexture",
    "NormalTexture2",
    "NormalMap",
    "Normal",
    "SpecularTexture",
    "SpecMap",
    "Spec",
    // Light.
    "lightmap",
    "shadowMapTex",
    "EmissiveTexture",
    "Emissive",
    "emissive",
    "ReflectionMap",
    "EnvMap",
    "EnvMap1",
    // Coverage.
    "Alpha",
    "AlphaMask",
    "AlphaTexture",
    "BlendTexture",
    // Other.
    "Ramp",
    "RampTexture",
    "Noise",
    "Clouds",
    "Dirt",
    "Colour",
    "Colour1",
    "smokeTexture",
    "smokeTexture1",
    // The Zone circuits' own three, by preimage over `EBOOT.elf`.
    "zoneTexInner",
    "zoneTexInnerNearest",
    "zoneTexVis",
    // "Five more sampler names, by preimage over the executable".
    "paraboloidReflectionTex",
    "directionalLight0ShadowTex",
    "directionalLight0LightmapTex",
    "zoneTexOuterNearest",
    "zoneTexOuter",
    // `rcs_preimage_sweep.rs`, 2026-09-16: 13 from `EBOOT.elf`'s own string
    // pool, 2 from the generated vocabulary, plus `TextureGradient` from a
    // second pass once the first batch's own tokens joined the vocabulary.
    // See `rcsmaterial.md`'s "Open" section for the sweep's own numbers.
    "TextureGradient",
    "Pitch",
    "textureSpot0Tex",
    "textureSpot0ShadowTex",
    "Rock",
    "alpha_emissive",
    "Electricity",
    "Wave",
    "GradientColour1",
    "paraboloidIblTex",
    "ambientShadowTex",
    "textureSpot1Tex",
    "zoneAnisoPalette",
    "textureSpot1ShadowTex",
    "zoneAnisoPaletteOuter",
    "screenSpaceReflectionTex",
];

/// Every parameter-name preimage `hd_param_names.rs`'s sweep of `EBOOT.elf`
/// against `DATA00.PSARC` + `DATA02.PSARC`'s `.rcsmodel` records recovered -
/// 64 of the 300 distinct hashes there, per `rcsmaterial.md`, "The parameter
/// names, by preimage".
pub const KNOWN_PARAMETER_NAMES: &[&str] = &[
    "Colour",
    "Colour_Scalar",
    "TrailSpeed",
    "UserColourScaler",
    "Fresnel_Power",
    "Charge2",
    "Nitro",
    "Transparency",
    "scale",
    "Envmap",
    "AlphaAnim",
    "BombColour",
    "uvOffset",
    "BassHit",
    "ColourAnim",
    "alpha",
    "Speed",
    "V_Offset",
    "Shockwave_scalar",
    "Fresnel_Min",
    "auroraColour",
    "Tempo",
    "Fresnel_Scale",
    "EMP",
    "min",
    "PitchOffset",
    "BombEQColour",
    "Brightness",
    "StandardBullets",
    "BombBrightness",
    "EMP_Rays",
    "PitchVolume",
    "TrebleHit",
    "ShipZone_Colour",
    "UV_offset",
    "Charge1",
    "Power",
    "BombEQBrightness",
    "Colour_Ramp",
    "Colour_Tint",
    "offset",
    "ShieldColour",
    "Refbrightness",
    "Range",
    "auroraBrightness",
    "MineBrightness",
    "ShadowSelect",
    "V_Anim",
    "ElectricityColour",
    "W_Cycle",
    "RainbowColour",
    "Distortion",
    "MineColour",
    "Flow_Speed",
    "auroraOffset",
    "MasterVolume",
    "uvScale",
    "Alpha",
    "StandardColour",
    "speed",
    "PowerColour",
    "Alpha_Scalar",
    "PowerBullets",
    "DamageRange",
    // `rcs_preimage_sweep.rs`, 2026-09-16: all 38 from the generated
    // vocabulary (tokens of the 64 above, recombined) - none independently
    // seen in `EBOOT.elf`'s own strings, so kept at a lower confidence than
    // the block above. See `rcsmaterial.md`'s "Open" section.
    "colour1",
    "SpecPower",
    "ShadowAlpha",
    "SpecularScalar",
    "SpecularColour",
    "spec",
    "SpecScale",
    "spec_power",
    "SpecularColor",
    "DiffuseColour",
    "diffuse",
    "power",
    "NormalPower",
    "FresnelMin",
    "EmissiveColour",
    "ColourTint",
    "DiffuseColour1",
    "ShadowColour",
    "Scaler",
    "DistortionSpeed",
    "VScale",
    "Reflection",
    "SpecularPower",
    "FresnelScale",
    "scale1",
    "colour2",
    "VSpeed",
    "Spec",
    "power1",
    "Speed2",
    "FresnelPower",
    "SpecColour",
    "SmokeScale",
    "colour3",
    "noiseScale",
    "Specular",
    "min1",
    "Emissive",
    "SmokeScale1",
];

/// A sampler-name hash's preimage, if [`KNOWN_SAMPLER_NAMES`] carries one.
#[must_use]
pub fn sampler_name(hash: u32) -> Option<&'static str> {
    KNOWN_SAMPLER_NAMES
        .iter()
        .copied()
        .find(|n| name_hash(n) == hash)
}

/// A parameter-name hash's preimage, if [`KNOWN_PARAMETER_NAMES`] carries one.
#[must_use]
pub fn parameter_name(hash: u32) -> Option<&'static str> {
    KNOWN_PARAMETER_NAMES
        .iter()
        .copied()
        .find(|n| name_hash(n) == hash)
}

#[cfg(test)]
mod tests;
