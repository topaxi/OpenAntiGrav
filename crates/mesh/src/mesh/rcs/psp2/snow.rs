//! Omega's `RT_snow1`: the snow mountain of Sebenco Climb, drawn by the law its own pixel program
//! computes (`docs/formats/omega-status.md`, "The snow program"), and the `DC_Rocks` family beside
//! it (below, "The rock").
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
//!
//! **The rock.** `DC_Rocks` (the boulders and cliffs of Sebenco Climb, which names the drift normal
//! as its detail map) is a lightmapped, tangent-framed material: pixel programs with the
//! `RockColour`, `RockColourNormal` and `RockDetailNormal` samplers, the lightmap, a cube
//! reflection and the wrap, detail and specular constants (`docs/formats/omega-status.md`, "The
//! rock program"). One entry carries `DetailNormalStrength`, `DetailScale` and `SpecStrength` (tint),
//! `SpecPower` (offset), the blur `ReflectivityBlur` (scale) and `Wrap` (`program[1]`); the colour
//! normal rides the third binding and the detail normal the fourth. A material is admitted only
//! when every submesh carries a lightmap pair and a tangent. The cube reflection is not drawn (the
//! cube is bound by the engine and was not read).

use std::sync::Arc;

use oag_rcs::gcn::{self, Stage};
use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::Report;
use super::normal_map;
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, ModelTexture, PROGRAM_OMEGA_ROCK, PROGRAM_OMEGA_SNOW, slots};

/// The five samplers' pictures, by `~crc32` of the program's sampler names less `Sampler`.
const TEXTURE1: u32 = 0x3bdc_0403;
const DRIFT: u32 = 0x6d65_2665;
const DETAIL: u32 = 0xff9d_cfe7;
const DETAIL_LARGE: u32 = 0x6555_b0e3;
const ROCK_COLOUR: u32 = 0xb90d_f195;
const ROCK_NORMAL: u32 = 0x97f1_2f2e;
const ROCK_DETAIL_NORMAL: u32 = 0xe188_475c;

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
const ROCK_DETAIL_STRENGTH: u32 = 0x3544_a0b1;
const ROCK_DETAIL_SCALE: u32 = 0xfe94_e23c;
const ROCK_BLUR: u32 = 0x3704_9fda;
const ROCK_WRAP: u32 = 0xf3f7_801c;

/// What one admitted material binds.
struct Bound {
    slot: u32,
    small: Arc<ModelTexture>,
    /// The snow's large detail normal; the rock has none.
    large: Option<Arc<ModelTexture>>,
    rock: bool,
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
        self.of(material).and_then(|b| b.large.clone())
    }

    /// The vertex a snow submesh writes, from its `NOVAColor`, `VertexColour2` and
    /// `VertexColour1` bytes: `None` for any other material.
    pub fn vertex(&self, material: Option<usize>, bytes: Attributes<'_>) -> Option<Vertex> {
        self.of(material).filter(|b| !b.rock)?;
        let unit = |b: u8| f32::from(b) / 255.0;
        let (nova, tint, mask) = bytes;
        let (nova, tint) = (nova?, tint?);
        let mask = mask.map_or(1.0, |m| unit(m[0]));
        Some(Vertex {
            colour: [unit(nova[0]), unit(nova[1]), unit(nova[2]), 1.0],
            sun_mask: unit(nova[3]),
            lightmap_texcoord: [unit(tint[0]), unit(tint[1])],
            texcoord2: [unit(tint[2]), mask],
        })
    }
}

/// The bytes a snow vertex reads: `NOVAColor`, `VertexColour2`, `VertexColour1`.
pub(super) type Attributes<'a> = (
    Option<&'a [u8; 4]>,
    Option<&'a [u8; 4]>,
    Option<&'a [u8; 4]>,
);

/// The fields [`Plan::vertex`] fills in `GpuVertex`.
pub(super) struct Vertex {
    pub colour: [f32; 4],
    pub sun_mask: f32,
    pub lightmap_texcoord: [f32; 2],
    pub texcoord2: [f32; 2],
}

/// The shape one family's pixel program declares: the samplers it must have, the ones a
/// permutation may add, and the constants it must name.
struct Shape {
    samplers: &'static [&'static str],
    other: &'static [&'static str],
    constants: &'static [&'static str],
}

/// A pixel program with exactly the five snow samplers (and, where a permutation reads them, the
/// lightmap and the shadow map) and the Fresnel and specular constants.
const SNOW: Shape = Shape {
    samplers: &[
        "Texture1Sampler",
        "DriftNormalTextureSampler",
        "snowNormDetailTextureSampler",
        "snowNormDetailLargeTextureSampler",
        "Texture2Sampler",
    ],
    other: &["lightmapSampler", "shadowMapSampler"],
    constants: &[
        "FresnelMin",
        "FresnelScale",
        "SpecTwinkleScale",
        "DiffuseWrap",
        "SpecPower",
        "shadowLightSpecularDiffuse",
    ],
};

/// A pixel program with the three rock samplers and the lightmap, the cube reflection (and, where
/// a permutation reads them, the shadow map, the distance cube and the Zone lookup) and the
/// rock's wrap, detail and specular constants.
const ROCK: Shape = Shape {
    samplers: &[
        "RockColourSampler",
        "RockColourNormalSampler",
        "RockDetailNormalSampler",
        "lightmapSampler",
        "cubeReflectionTexSampler",
    ],
    other: &[
        "shadowMapSampler",
        "cubeDistanceTexSampler",
        "zoneAnisoLookupTexSampler",
    ],
    constants: &[
        "DetailScale",
        "DetailNormalStrength",
        "ReflectivityBlur",
        "Wrap",
        "SpecPower",
        "shadowLightSpecularDiffuse",
    ],
};

/// Whether a material file has a pixel program of this shape.
fn declares(file: &[u8], shape: &Shape) -> bool {
    gcn::scan(file).iter().any(|(_, program)| {
        let Ok(program) = program else {
            return false;
        };
        if program.stage != Stage::Pixel {
            return false;
        }
        let Ok(reflection) = &program.reflection else {
            return false;
        };
        let names: Vec<&str> = reflection
            .samplers
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        shape.samplers.iter().all(|s| names.contains(s))
            && names
                .iter()
                .all(|n| shape.samplers.contains(n) || shape.other.contains(n))
            && shape.constants.iter().all(|c| program.names_constant(c))
    })
}

/// The snow material's constants as the three glow-table entries, `None` where one is unauthored.
fn entries(material: &Material) -> Option<Vec<Emissive>> {
    let value = |hash: u32| material.param(hash).and_then(|v| v.first().copied());
    let triple = |hash: u32| -> Option<[f32; 3]> {
        let v = material.param(hash)?;
        Some([*v.first()?, *v.get(1)?, *v.get(2)?])
    };
    let (strength, colour) = (value(FRESNEL_STRENGTH)?, triple(FRESNEL_COLOUR)?);
    let (spec_strength, spec_colour) = (value(SPEC_STRENGTH)?, triple(SPEC_COLOUR)?);
    let product = |k: f32, c: [f32; 3]| [k * c[0], k * c[1], k * c[2]];
    Some(vec![
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

/// The rock material's constants as one glow-table entry, `None` where one is unauthored.
fn rock_entries(material: &Material) -> Option<Vec<Emissive>> {
    let value = |hash: u32| material.param(hash).and_then(|v| v.first().copied());
    Some(vec![Emissive {
        tint: [
            value(ROCK_DETAIL_STRENGTH)?,
            value(ROCK_DETAIL_SCALE)?,
            value(SPEC_STRENGTH)?,
        ],
        offset: value(SPEC_POWER)?,
        scale: value(ROCK_BLUR)?,
        rate: 0.0,
        program: [PROGRAM_OMEGA_ROCK, value(ROCK_WRAP)?],
    }])
}

/// Admits each snow or rock material whose pictures decode and whose submeshes all carry the
/// vertex attributes its program reads, binding its first normal third (into `normals`) and
/// writing its entries.
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
        let Some(file) = textures(&material.name) else {
            continue;
        };
        let rock = if declares(&file, &SNOW) {
            false
        } else if declares(&file, &ROCK) {
            true
        } else {
            continue;
        };
        let unread = |report: &mut Report| {
            if rock {
                crate::mesh::rcs::census::claim(
                    index,
                    "rock",
                    crate::mesh::rcs::census::Outcome::NotDrawn,
                );
                report.rock_unread += 1;
            } else {
                crate::mesh::rcs::census::claim(
                    index,
                    "snow",
                    crate::mesh::rcs::census::Outcome::NotDrawn,
                );
                report.snow_unread += 1;
            }
        };
        let readable = decoded
            .submeshes
            .iter()
            .filter(|s| s.material == Some(index))
            .all(|s| {
                let n = s.positions.len();
                n > 0
                    && s.tangents.len() == n
                    && if rock {
                        s.lightmap_texcoords.len() == n
                    } else {
                        s.ps4_nova_colour.len() == n && s.ps4_vertex_colour2.len() == n
                    }
            });
        let mut picture = |hash: u32| {
            let path = material.sampler(hash).filter(|p| !p.is_empty())?;
            cache
                .entry(path.to_string())
                .or_insert_with(|| super::picture_memo::picture(path, textures, report))
                .clone()
        };
        let (first, small, large, entries) = if rock {
            (
                picture(ROCK_NORMAL),
                picture(ROCK_DETAIL_NORMAL),
                None,
                rock_entries(material),
            )
        } else {
            let large = picture(DETAIL_LARGE);
            (
                picture(DRIFT),
                picture(DETAIL),
                Some(large),
                entries(material),
            )
        };
        let (Some(first), Some(small), Some(large), Some(entries), true) = (
            first,
            small,
            large.map_or(Some(None), |l| l.map(Some)),
            entries,
            readable,
        ) else {
            unread(report);
            continue;
        };
        if material
            .sampler(if rock { ROCK_COLOUR } else { TEXTURE1 })
            .is_none()
        {
            unread(report);
            continue;
        }
        let count = entries.len();
        let slot = emissive
            .windows(count)
            .position(|w| w == entries.as_slice())
            .or_else(|| {
                (emissive.len() + count < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                    emissive.extend(entries);
                    emissive.len() - count
                })
            })
            .and_then(|f| u32::try_from(f + 1).ok());
        let Some(slot) = slot else {
            unread(report);
            continue;
        };
        normals.bind(index, first);
        out.bound[index] = Some(Bound {
            slot,
            small,
            large,
            rock,
        });
        if rock {
            crate::mesh::rcs::census::claim(
                index,
                "rock",
                crate::mesh::rcs::census::Outcome::Drawn,
            );
            report.rock_materials += 1;
        } else {
            crate::mesh::rcs::census::claim(
                index,
                "snow",
                crate::mesh::rcs::census::Outcome::Drawn,
            );
            report.snow_materials += 1;
        }
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
            (ROCK_COLOUR, "RockColour"),
            (ROCK_NORMAL, "RockColourNormal"),
            (ROCK_DETAIL_NORMAL, "RockDetailNormal"),
            (ROCK_DETAIL_STRENGTH, "DetailNormalStrength"),
            (ROCK_DETAIL_SCALE, "DetailScale"),
            (ROCK_BLUR, "ReflectivityBlur"),
            (ROCK_WRAP, "Wrap"),
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
