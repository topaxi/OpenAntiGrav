//! Omega's `road_alphaspec_normal_emissive_panelmap1[_mask]`: the 2048-mode road whose pixel
//! program marches the view ray into a panel map, reflects the circuit's sky cube and adds a sun
//! lobe, all gated by the diffuse map's alpha.
//!
//! The law is `docs/ghidra/functions/ps4-omega-eu/panel-displacement.md`, "The law" (85 to 88):
//!
//! ```text
//! uv'  = uv + step * t                    10-sample sphere trace into the panel map
//! spec = sun_specular * SpecStrength * Dm.a * LM.a * sat(R.L)^p * 0.0698 / max(0.0698, 0.1745 min(1, B))
//! refl = ReflectivityAmount * Dm.a * (Min + Scale (1 - N.V)^Power) * sky(R; lod 2B)
//! rgb  = Dm.rgb * light + E + spec + refl         `_mask`: (spec + refl) * ReflectionMask(uv)
//! ```
//!
//! with `B` the material's `Blurrines`, `p = SpecPower (1 - 0.75 min(1, B))`, and diffuse,
//! normal and emissive sampled at `uv'`. The sky is [`super::sky_reflection`]'s map on the scene's
//! sky binding. Three consecutive glow-table entries carry the material's numbers, the first being
//! the one the vertices name (mode [`PROGRAM_PANEL_MAP`]):
//!
//! | entry | tint | offset | scale | program[1] |
//! | --- | --- | --- | --- | --- |
//! | 0 | `ReflectivityAmount`, `Min`, `Scale` | `Power` | `2 * Blurrines` | 1 on `_mask` |
//! | 1 | `SpecStrength`, `SpecPower`, has an emissive | | | |
//! | 2 | `Depth`, `SlopeWidth`, `Aspect` | | | |
//!
//! Bindings: the normal map third ([`super::normal_map`]), `EmissiveMap` fourth, the panel map
//! fifth (a [`BlockFormat::R16Float`] picture), `ReflectionMask` sixth on `_mask`. Admitted by name and by the
//! four `Reflectivity*` parameters, on a PS4 model whose material binds its normal map (the march
//! needs the tangent frame); anything short is counted and left to the generic reading. 2048
//! ships no panel map (the page, "2048 against Omega": checked, differs).

use std::sync::Arc;

use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::{Report, decode_material_texture, normal_map, tarmac};
use crate::mesh::rcs::Textures;
use crate::mesh::{BlockFormat, Emissive, ModelTexture, PROGRAM_PANEL_MAP, Texels};

/// `~crc32("Cg_PanelDisplacement_1In1panel_map")`: the panel map's sampler.
const PANEL_MAP: u32 = 0x4736_b3bf;
/// `~crc32("EmissiveMap")`.
const EMISSIVE_MAP: u32 = 0x19d1_66c1;
/// `~crc32("ReflectionMask")`, `_mask` only.
const REFLECTION_MASK: u32 = 0x23f1_0eea;

/// The parameters the plan reads, by hash (`panel-displacement.md`, "Authored values").
const REFLECTIVITY_AMOUNT: u32 = 0x366e_9d28;
const REFLECTIVITY_MIN: u32 = 0x20ea_500d;
const REFLECTIVITY_SCALE: u32 = 0x9164_3095;
const REFLECTIVITY_POWER: u32 = 0xd6a8_14b1;
const SPEC_STRENGTH: u32 = 0x2920_cd2d;
const SPEC_POWER: u32 = 0x05fe_c07d;
const BLURRINES: u32 = 0xc09f_740a;
const DEPTH: u32 = 0x95de_1208;
const SLOPE_WIDTH: u32 = 0x4e23_1706;
const ASPECT: u32 = 0x185b_0018;

/// What [`plan`] counted, for the load report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    /// Materials drawn through the panel-map law.
    pub materials: usize,
    /// Materials of the family left to the generic reading (a missing parameter, picture or
    /// normal map, or a full glow table).
    pub unread: usize,
    /// Whether this model carries the circuit's sky reflection map (a `skycube` build).
    pub sky_reflection: bool,
}

impl Census {
    /// A clause for [`Report::describe`], empty when there is nothing to say.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        if self.materials + self.unread > 0 {
            out += &format!(
                "; {} panel-map road material(s) march their panel map and reflect the sky cube ({} left to the generic reading)",
                self.materials, self.unread
            );
        }
        if self.sky_reflection {
            out += "; the sky reflection map is skyCube.gnf's four cubes";
        }
        out
    }
}

/// Whether `material` is one of the two panel-map road families, and if so whether it is `_mask`.
fn family(material: &Material) -> Option<bool> {
    let stem = material.name.rsplit(['/', '\\']).next()?;
    let stem = stem.trim_suffix(".rcsmaterial");
    if stem.eq_ignore_ascii_case("road_alphaspec_normal_emissive_panelmap1") {
        Some(false)
    } else if stem.eq_ignore_ascii_case("road_alphaspec_normal_emissive_panelmap1_mask") {
        Some(true)
    } else {
        None
    }
}

/// The three glow-table entries of an admitted material, `None` when a parameter is missing.
fn entries(material: &Material, mask: bool, emissive: bool) -> Option<[Emissive; 3]> {
    let p = |hash| material.param(hash)?.first().copied();
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
    let row = |tint, offset, scale, program| Emissive {
        tint,
        offset,
        scale,
        rate: 0.0,
        program,
    };
    Some([
        row(
            [
                p(REFLECTIVITY_AMOUNT)?,
                p(REFLECTIVITY_MIN)?,
                p(REFLECTIVITY_SCALE)?,
            ],
            p(REFLECTIVITY_POWER)?,
            2.0 * p(BLURRINES)?,
            [PROGRAM_PANEL_MAP, flag(mask)],
        ),
        row(
            [p(SPEC_STRENGTH)?, p(SPEC_POWER)?, flag(emissive)],
            0.0,
            0.0,
            [0.0; 2],
        ),
        row([p(DEPTH)?, p(SLOPE_WIDTH)?, p(ASPECT)?], 0.0, 0.0, [0.0; 2]),
    ])
}

/// The panel map as an `R16Float` picture, or `None` where the file is not the half-float
/// surface `oag_texture::gnf::Texture::half_floats` reads.
fn panel_picture(path: &str, blob: &[u8]) -> Option<ModelTexture> {
    let texture = oag_texture::gnf::Texture::parse(blob).ok()?;
    let texels = texture.half_floats(blob).ok()?;
    Some(ModelTexture {
        label: path.to_string(),
        width: texture.width,
        height: texture.height,
        texels: Texels::Blocks {
            format: BlockFormat::R16Float,
            levels: vec![texels.iter().flat_map(|h| h.to_le_bytes()).collect()],
        },
        mip_count: None,
        lod_code: 0.0,
    })
}

/// Admits each panel-map road material, writing its three glow-table entries into `emissive`
/// and its pictures into `layers` (the binding plan the tarmac roads share).
pub(super) fn plan(
    decoded: &psp2::Model,
    (ps4, normals): (bool, &normal_map::Plan),
    layers: &mut tarmac::Plan,
    emissive: &mut Vec<Emissive>,
    textures: Textures<'_>,
    report: &mut Report,
) {
    if !ps4 {
        return;
    }
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for (index, material) in decoded.materials.iter().enumerate() {
        let Some(mask) = family(material) else {
            continue;
        };
        let mut picture = |hash: u32, panel: bool| {
            let path = material.sampler(hash).filter(|p| !p.is_empty())?;
            cache
                .entry(path.to_string())
                .or_insert_with(|| {
                    let blob = textures(path)?;
                    if panel {
                        panel_picture(path, &blob)
                    } else {
                        decode_material_texture(path, &blob, report)
                    }
                    .map(Arc::new)
                })
                .clone()
        };
        let panel = picture(PANEL_MAP, true);
        let glow = picture(EMISSIVE_MAP, false);
        let reflection_mask = if mask {
            picture(REFLECTION_MASK, false)
        } else {
            None
        };
        let rows = entries(material, mask, glow.is_some());
        let taken = layers.of(Some(index)).is_some();
        let (Some(rows), Some(panel), Some(_), false, true) = (
            rows,
            panel,
            normals.of(Some(index)),
            taken,
            !mask || reflection_mask.is_some(),
        ) else {
            crate::mesh::rcs::census::claim(
                index,
                "panel_map",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.panel.unread += 1;
            continue;
        };
        let at = emissive.windows(3).position(|w| w == rows).or_else(|| {
            (emissive.len() + 3 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.extend(rows);
                emissive.len() - 3
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            crate::mesh::rcs::census::claim(
                index,
                "panel_map",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.panel.unread += 1;
            continue;
        };
        layers.layers[index] = Some(tarmac::Layers {
            slot,
            fifth: Some(panel),
            sixth: reflection_mask,
            emissive: glow,
        });
        crate::mesh::rcs::census::claim(
            index,
            "panel_map",
            crate::mesh::rcs::census::Outcome::Drawn,
        );
        report.panel.materials += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_rcs::rcsmaterial::name_hash;

    #[test]
    fn the_hashes_are_the_names() {
        assert_eq!(PANEL_MAP, name_hash("Cg_PanelDisplacement_1In1panel_map"));
        assert_eq!(EMISSIVE_MAP, name_hash("EmissiveMap"));
        assert_eq!(REFLECTION_MASK, name_hash("ReflectionMask"));
        assert_eq!(REFLECTIVITY_AMOUNT, name_hash("ReflectivityAmount"));
        assert_eq!(REFLECTIVITY_MIN, name_hash("ReflectivityMin"));
        assert_eq!(REFLECTIVITY_SCALE, name_hash("ReflectivityScale"));
        assert_eq!(REFLECTIVITY_POWER, name_hash("ReflectivityPower"));
        assert_eq!(SPEC_STRENGTH, name_hash("SpecStrength"));
        assert_eq!(SPEC_POWER, name_hash("SpecPower"));
        assert_eq!(BLURRINES, name_hash("Blurrines"));
        assert_eq!(DEPTH, name_hash("Cg_PanelDisplacement_1In1Depth"));
        assert_eq!(
            SLOPE_WIDTH,
            name_hash("Cg_PanelDisplacement_1In1SlopeWidth")
        );
        assert_eq!(ASPECT, name_hash("Cg_PanelDisplacement_1In1Aspect"));
    }

    fn material(name: &str, params: &[(u32, f32)]) -> Material {
        Material {
            name: name.to_string(),
            technique: None,
            textures: Vec::new(),
            lightmap: None,
            params: params
                .iter()
                .map(|&(hash, v)| oag_rcs::rcsmodel::psp2::material::Param {
                    hash,
                    bits: vec![v.to_bits()],
                })
                .collect(),
            samplers: Vec::new(),
            state: None,
        }
    }

    const SOL: [(u32, f32); 10] = [
        (REFLECTIVITY_AMOUNT, 0.5),
        (REFLECTIVITY_MIN, 0.6),
        (REFLECTIVITY_SCALE, 10.0),
        (REFLECTIVITY_POWER, 4.0),
        (SPEC_STRENGTH, 5.0),
        (SPEC_POWER, 100.0),
        (BLURRINES, 0.3),
        (DEPTH, 3.5),
        (SLOPE_WIDTH, 1.8),
        (ASPECT, 4.0),
    ];

    #[test]
    fn only_the_two_panelmap1_families_are_admitted() {
        let named = |n: &str| family(&material(n, &[]));
        assert_eq!(
            named("x/materials/road_alphaspec_normal_emissive_panelmap1.rcsmaterial"),
            Some(false)
        );
        assert_eq!(
            named("x/materials/road_alphaspec_normal_emissive_panelmap1_mask.rcsmaterial"),
            Some(true)
        );
        assert_eq!(
            named("x/materials/Diffuse_Normal_Specular_panelmap2.rcsmaterial"),
            None
        );
    }

    #[test]
    fn sols_numbers_land_in_their_lanes() {
        let rows = entries(&material("road", &SOL), false, true).expect("all authored");
        assert_eq!(rows[0].tint, [0.5, 0.6, 10.0]);
        assert_eq!(rows[0].offset, 4.0);
        assert!((rows[0].scale - 0.6).abs() < 1e-6, "lod is 2 * Blurrines");
        assert_eq!(rows[0].program, [PROGRAM_PANEL_MAP, 0.0]);
        assert_eq!(rows[1].tint, [5.0, 100.0, 1.0]);
        assert_eq!(rows[2].tint, [3.5, 1.8, 4.0]);
        assert!(rows.iter().all(|r| r.rate == 0.0), "no clock");
    }

    #[test]
    fn a_missing_reflectivity_parameter_refuses_the_material() {
        let short: Vec<_> = SOL[1..].to_vec();
        assert!(entries(&material("road", &short), false, true).is_none());
    }
}
