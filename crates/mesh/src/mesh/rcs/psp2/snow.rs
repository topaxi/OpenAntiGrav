//! Omega's `RT_snow1`: the snow mountain of Sebenco Climb, drawn by the law its own pixel program
//! computes (`docs/formats/omega-status.md`, "The snow program").
//!
//! The material's vertex-lit permutation (`NOVAColor`, no lightmap, no shadow map, no vertex light)
//! blends a drift normal with two detail normals, weights the picture by a Fresnel factor and a
//! wrapped sun and adds a sun lobe; `shade.wesl` computes it under a glow-table entry of mode
//! [`PROGRAM_OMEGA_SNOW`] (`omega_snow.wesl` has the listing). **Every constant is the material's
//! own**: the entry this module writes carries `FresnelStrength * FresnelColour`, `FresnelMin`,
//! `FresnelScale`, `DiffuseWrap`; the next `SpecStrength * SpecColour`, `SpecPower`,
//! `SpecTwinkleScale`, `FresnelPower`; the third `snownormDetailStrength`, `snownormDetailScale`
//! and `snownormDetailLargeScale`.
//!
//! **Which materials.** One whose pixel programs declare exactly the five snow samplers (`Texture1`,
//! `DriftNormalTexture`, `snowNormDetailTexture`, `snowNormDetailLargeTexture`, `Texture2`, with the
//! lightmap and the shadow map where a permutation reads them) and the lit constants of that
//! permutation, and which authors all thirteen constants. `RT_snow_ref` declares three more
//! samplers and keeps the generic reading. A material is admitted only when every submesh on it
//! carries `NOVAColor`, `VertexColour2` and a tangent.
//!
//! **Which permutation the race binds is unread** (confidence 70): the vertex-lit one is chosen
//! because these submeshes carry `NOVAColor` and no `lightmapUV`, and the material has no
//! other permutation with those inputs and without a vertex light.
//!
//! Binding: the drift normal rides the third binding, the small detail the fourth, the large detail
//! the fifth; the vertex carries `NOVAColor.rgb` in `colour`, its alpha in `sun_mask`,
//! `VertexColour2.rgb` in the lightmap pair and the pair after it, and `VertexColour1.x` (the drift
//! mask) in the last lane.

use std::sync::Arc;

use oag_rcs::gcn::{self, Stage};
use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::normal_map;
use super::{Report, decode_material_texture};
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, ModelTexture, PROGRAM_OMEGA_SNOW, slots};

/// The five samplers' pictures, by `~crc32` of the program's sampler names less `Sampler`.
const TEXTURE1: u32 = 0x3bdc_0403;
const DRIFT: u32 = 0x6d65_2665;
const DETAIL: u32 = 0xff9d_cfe7;
const DETAIL_LARGE: u32 = 0x6555_b0e3;

/// The material's constants, by `~crc32` of their names.
const DIFFUSE_WRAP: u32 = 0xa99e_82f7;
const FRESNEL_MIN: u32 = 0x60a4_33da;
const FRESNEL_SCALE: u32 = 0x8ba6_13e1;
const FRESNEL_POWER: u32 = 0xcc6a_37c5;
const FRESNEL_STRENGTH: u32 = 0x4ef2_426b;
const FRESNEL_COLOUR: u32 = 0x1545_ffa2;
const SPEC_STRENGTH: u32 = 0x2920_cd2d;
const SPEC_COLOUR: u32 = 0xd036_50eb;
const SPEC_POWER: u32 = 0x05fe_c07d;
const SPEC_TWINKLE: u32 = 0x6cf2_efc3;
const DETAIL_STRENGTH: u32 = 0xd9e5_2a6c;
const DETAIL_SCALE: u32 = 0xe4cc_8a2e;
const DETAIL_LARGE_SCALE: u32 = 0xbd77_f3ba;

/// What one admitted material binds.
struct Bound {
    slot: u32,
    small: Arc<ModelTexture>,
    large: Arc<ModelTexture>,
}

/// Per material, what [`plan`] admitted.
#[derive(Default)]
pub(super) struct Plan {
    bound: Vec<Option<Bound>>,
}

impl Plan {
    pub fn any(&self) -> bool {
        self.bound.iter().any(Option::is_some)
    }

    fn of(&self, material: Option<usize>) -> Option<&Bound> {
        self.bound.get(material?)?.as_ref()
    }

    /// The vertex bits that select this material's glow-table entry; `0` for any other.
    pub fn role(&self, material: Option<usize>) -> u32 {
        self.of(material)
            .map_or(0, |b| b.slot << slots::MATERIAL_SHIFT)
    }

    /// The fourth binding: the small detail normal.
    pub fn small(&self, material: Option<usize>) -> Option<Arc<ModelTexture>> {
        self.of(material).map(|b| b.small.clone())
    }

    /// The fifth binding: the large detail normal.
    pub fn large(&self, material: Option<usize>) -> Option<Arc<ModelTexture>> {
        self.of(material).map(|b| b.large.clone())
    }

    /// The vertex a snow submesh writes: `colour`, `sun_mask`, the lightmap pair and the second
    /// pair, from `NOVAColor`, `VertexColour2` and `VertexColour1`.
    pub fn vertex(
        &self,
        material: Option<usize>,
        (nova, tint, mask): (Option<&[u8; 4]>, Option<&[u8; 4]>, Option<&[u8; 4]>),
    ) -> Option<([f32; 4], f32, [f32; 2], [f32; 2])> {
        self.of(material)?;
        let unit = |b: u8| f32::from(b) / 255.0;
        let (nova, tint) = (nova?, tint?);
        let mask = mask.map_or(1.0, |m| unit(m[0]));
        Some((
            [unit(nova[0]), unit(nova[1]), unit(nova[2]), 1.0],
            unit(nova[3]),
            [unit(tint[0]), unit(tint[1])],
            [unit(tint[2]), mask],
        ))
    }
}

/// Whether a material file declares the snow program's shape: a pixel program with exactly the five
/// snow samplers (and, where a permutation reads them, the lightmap and the shadow map) and the
/// Fresnel and specular constants.
fn declares_snow(file: &[u8]) -> bool {
    const SAMPLERS: [&str; 5] = [
        "Texture1Sampler",
        "DriftNormalTextureSampler",
        "snowNormDetailTextureSampler",
        "snowNormDetailLargeTextureSampler",
        "Texture2Sampler",
    ];
    const OTHER: [&str; 2] = ["lightmapSampler", "shadowMapSampler"];
    const CONSTANTS: [&str; 6] = [
        "FresnelMin",
        "FresnelScale",
        "SpecTwinkleScale",
        "DiffuseWrap",
        "SpecPower",
        "shadowLightSpecularDiffuse",
    ];
    gcn::scan(file).into_iter().any(|(_, program)| {
        let Ok(program) = program else {
            return false;
        };
        if program.stage != Stage::Pixel {
            return false;
        }
        let Ok(reflection) = &program.reflection else {
            return false;
        };
        let names: Vec<&str> = reflection.samplers.iter().map(|s| s.name.as_str()).collect();
        SAMPLERS.iter().all(|s| names.contains(s))
            && names
                .iter()
                .all(|n| SAMPLERS.contains(n) || OTHER.contains(n))
            && CONSTANTS.iter().all(|c| program.names_constant(c))
    })
}

/// The material's constants as the three glow-table entries, `None` where one is unauthored.
fn entries(material: &Material) -> Option<[Emissive; 3]> {
    let value = |hash: u32| material.param(hash).and_then(|v| v.first().copied());
    let triple = |hash: u32| -> Option<[f32; 3]> {
        let v = material.param(hash)?;
        Some([*v.first()?, *v.get(1)?, *v.get(2)?])
    };
    let (strength, colour) = (value(FRESNEL_STRENGTH)?, triple(FRESNEL_COLOUR)?);
    let (spec_strength, spec_colour) = (value(SPEC_STRENGTH)?, triple(SPEC_COLOUR)?);
    let product = |k: f32, c: [f32; 3]| [k * c[0], k * c[1], k * c[2]];
    Some([
        Emissive {
            tint: product(strength, colour),
            offset: value(FRESNEL_MIN)?,
            scale: value(FRESNEL_SCALE)?,
            rate: 0.0,
            program: [PROGRAM_OMEGA_SNOW, value(DIFFUSE_WRAP)?],
        },
        Emissive {
            tint: product(spec_strength, spec_colour),
            offset: value(SPEC_POWER)?,
            scale: value(SPEC_TWINKLE)?,
            rate: value(FRESNEL_POWER)?,
            program: [0.0; 2],
        },
        Emissive {
            tint: [
                value(DETAIL_STRENGTH)?,
                value(DETAIL_SCALE)?,
                value(DETAIL_LARGE_SCALE)?,
            ],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [0.0; 2],
        },
    ])
}

/// Admits each snow material whose pictures decode and whose submeshes all carry the vertex
/// attributes, binding its drift normal third (into `normals`) and writing its three entries.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    (normals, emissive): (&mut normal_map::Plan, &mut Vec<Emissive>),
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan {
        bound: decoded.materials.iter().map(|_| None).collect(),
    };
    if !ps4 {
        return out;
    }
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for (index, material) in decoded.materials.iter().enumerate() {
        if !textures(&material.name).is_some_and(|file| declares_snow(&file)) {
            continue;
        }
        let readable = decoded
            .submeshes
            .iter()
            .filter(|s| s.material == Some(index))
            .all(|s| {
                let n = s.positions.len();
                n > 0
                    && s.ps4_nova_colour.len() == n
                    && s.ps4_vertex_colour2.len() == n
                    && s.tangents.len() == n
            });
        let mut picture = |hash: u32| {
            let path = material.sampler(hash).filter(|p| !p.is_empty())?;
            cache
                .entry(path.to_string())
                .or_insert_with(|| {
                    textures(path)
                        .and_then(|blob| decode_material_texture(path, &blob, report))
                        .map(Arc::new)
                })
                .clone()
        };
        let (drift, small, large) = (picture(DRIFT), picture(DETAIL), picture(DETAIL_LARGE));
        let (Some(drift), Some(small), Some(large), Some(entries), true) =
            (drift, small, large, entries(material), readable)
        else {
            report.snow_unread += 1;
            continue;
        };
        if material.sampler(TEXTURE1).is_none() {
            report.snow_unread += 1;
            continue;
        }
        let first = emissive
            .windows(3)
            .position(|w| w == entries.as_slice())
            .or_else(|| {
                (emissive.len() + 3 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                    emissive.extend(entries);
                    emissive.len() - 3
                })
            });
        let Some(slot) = first.and_then(|f| u32::try_from(f + 1).ok()) else {
            report.snow_unread += 1;
            continue;
        };
        normals.bind(index, drift);
        out.bound[index] = Some(Bound { slot, small, large });
        report.snow_materials += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_rcs::rcsmaterial::name_hash;

    #[test]
    fn the_constants_are_the_hashes_of_their_names() {
        for (hash, name) in [
            (TEXTURE1, "Texture1"),
            (DRIFT, "DriftNormalTexture"),
            (DETAIL, "snowNormDetailTexture"),
            (DETAIL_LARGE, "snowNormDetailLargeTexture"),
            (DIFFUSE_WRAP, "DiffuseWrap"),
            (FRESNEL_MIN, "FresnelMin"),
            (FRESNEL_SCALE, "FresnelScale"),
            (FRESNEL_POWER, "FresnelPower"),
            (FRESNEL_STRENGTH, "FresnelStrength"),
            (FRESNEL_COLOUR, "FresnelColour"),
            (SPEC_STRENGTH, "SpecStrength"),
            (SPEC_COLOUR, "SpecColour"),
            (SPEC_POWER, "SpecPower"),
            (SPEC_TWINKLE, "SpecTwinkleScale"),
            (DETAIL_STRENGTH, "snownormDetailStrength"),
            (DETAIL_SCALE, "snownormDetailScale"),
            (DETAIL_LARGE_SCALE, "snownormDetailLargeScale"),
        ] {
            assert_eq!(hash, name_hash(name), "{name}");
        }
    }
}
