//! 2048's own seas: `fc15_effects_vscroll_water` and the `fc17` pair.
//!
//! The material's programs declare one sampler, `NormalMap`, that only the lit permutations
//! read, a constant `ColourConstant`, `SpecularPower`, and a vertex program that declares
//! `time` ([`oag_rcs::gxp::sampler_use`]). Its fragment program (`docs/rendering/water.md`,
//! "2048 (Vita)") decodes the map as `2t - 1` on all three channels into the chunk's tangent
//! frame, lights it as `(ambient + diffuse * sat(N.L)) * ColourConstant` (the uniform as it is,
//! not squared as a picture is), and adds `liveLighting0specular * t.a * sat(R.L)^SpecularPower`,
//! `R` the view reflected about the mapped normal. The vertex program adds `0.05 * time` to the
//! first coordinate's `u`.
//! `liveLighting0specular` is drawn as the sun's diffuse colour (chosen, not measured: 2048's
//! envsettings author no sun specular colour).
//!
//! The sea rides HD's tap entry (`mesh::rcs::water::sea_block`, kind
//! [`VITA_SEA_KIND`]) with the second tap at weight zero, the map bound third as the PS4's
//! normal maps are (`psp2::normal_map`), and the tangent the declaration carries.
//!
//! **`fc17`** (Park's `fc17_effects_vscroll_water`, Altima's blended
//! `fc17_effects_vscroll_wateredge_vertexcol`, `docs/rendering/water.md`, "2048 (Vita)") shares
//! that frame and light, takes its colour from `Watercolour` (as it is, like the pond's),
//! multiplies the highlight by `SpecularColour` with no gloss from the map's alpha, and scrolls
//! `v` by `Speed * time`. The edge adds an alpha chain over `Opacity`, the map's alpha and the
//! two vertex colour lanes `VertexColour1.x` and `.y`, which ride the vertex's second coordinate
//! pair.
//!
//! **Omega's reflective seas** (`reflectplane_mid`, `_edge`, `_pond`, `_dc_seawater_new`,
//! `_dc_seawater_tow`; `docs/rendering/water.md`, "Omega (PS4)") replace those materials on the
//! same meshes. Their PS4 programs ([`oag_rcs::gcn::sea`]) fetch two wave normal maps,
//! `NormalMap` at `(u + K Speed t, v)` and `NormalMap1` at `(K Speed t - u, v)`, decode each
//! `.ag` as HD's sea does, light `Watercolour`, add a Phong highlight in `SpecularColour`
//! (times `SpecularStrength` where the program declares it) and the planar reflection weighted
//! by [`Fresnel`]. The edge's alpha is `VertexColour1.y * Opacity`. The planar target is not
//! drawn in this engine, so the sky at the mapped normal stands in for it (chosen, not
//! measured, as HD's sea).
//!
//! **Omega's Zone models** keep 2048's blended edge, recompiled
//! ([`oag_rcs::gcn::scrolled_edge`]). A Zone race draws it through its Zone permutation: the
//! Zone colour, the vertex normal's light, and the same alpha chain over the map's alpha
//! scrolled `v + Speed t`. That reading is what is laid (kind [`VITA_EDGE_KIND`], the taps
//! weighing nothing in the normal, no highlight); its lit permutation, which adds
//! `Reflectivity * cubeReflectionTex`, is read and not drawn.
//!
//! Classified by the programs' declared interface, not by name: a Vita material by its GXP
//! programs, a PS4 material by its GCN programs. Under `off` nothing is laid, so the draw is
//! the flat colour it was.

use std::sync::Arc;

use oag_rcs::gxp::sampler_use;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2;

use super::{Report, glow, normal_map, twin_blend};
use crate::mesh::Emissive;
use crate::mesh::rcs::Textures;
use crate::mesh::rcs::{
    Fresnel, OMEGA_EDGE_KIND, OMEGA_SEA_KIND, Tap, VITA_EDGE_KIND, VITA_SEA_KIND, VITA_WATER_KIND,
    lay, reflective_block, sea_block,
};
use crate::mesh::{ModelTexture, slots};

/// `~crc32("ColourConstant")`, `SpecularPower`.
const COLOUR_CONSTANT: u32 = 0x4d60_d566;
const SPECULAR_POWER: u32 = 0x81e0_e773;
/// `~crc32("Watercolour")`, `SpecularColour`, `Speed`, `Opacity`.
const WATERCOLOUR: u32 = 0xfe51_d093;
const SPECULAR_COLOUR: u32 = 0x370a_63cb;
const SPEED: u32 = 0x3118_2e0d;
const OPACITY: u32 = 0x73bf_ab05;
/// `~crc32` of `NormalMap1`, `SpecularStrength` and the four `Reflectivity*` weights.
const NORMAL_MAP1: u32 = 0xda28_4baf;
const SPECULAR_STRENGTH: u32 = 0x0374_6fa5;
const REFLECTIVITY_MIN: u32 = 0x20ea_500d;
const REFLECTIVITY_SCALE: u32 = 0x9164_3095;
const REFLECTIVITY_POWER: u32 = 0xd6a8_14b1;
const REFLECTIVITY_STRENGTH: u32 = 0xd6c9_5600;

/// Which sea a material's programs declare: one of 2048's three, or Omega's.
#[derive(Debug, Clone, PartialEq)]
enum Law {
    /// `fc15`: `ColourConstant`, the highlight weighted by the map's alpha.
    Pond,
    /// `fc17`, opaque: `Watercolour` and `SpecularColour`, no gloss.
    Water,
    /// `fc17`'s blended edge: [`Law::Water`] plus the alpha chain.
    Edge,
    /// Omega's reflective sea, as its PS4 programs declare it.
    Reflective(oag_rcs::gcn::Sea),
    /// Omega's copy of [`Law::Edge`] on its Zone models, drawn as its Zone permutation reads
    /// it: the scrolled map's alpha in the chain, no mapped normal and no highlight.
    ZoneEdge,
}

impl Law {
    fn kind(&self) -> f32 {
        match self {
            Self::Pond => VITA_SEA_KIND,
            Self::Water => VITA_WATER_KIND,
            Self::Edge | Self::ZoneEdge => VITA_EDGE_KIND,
            Self::Reflective(sea) if sea.opacity => OMEGA_EDGE_KIND,
            Self::Reflective(_) => OMEGA_SEA_KIND,
        }
    }

    /// Whether the material's vertices carry `VertexColour1.xy` in their second coordinate
    /// pair for the alpha.
    fn blended_edge(&self) -> bool {
        match self {
            Self::Edge | Self::ZoneEdge => true,
            Self::Reflective(sea) => sea.opacity,
            Self::Pond | Self::Water => false,
        }
    }

    /// Reads the programs' interface: a time-scrolled surface whose only material sampler is
    /// `NormalMap`, told apart by what its ambient-only permutation declares.
    fn of(uses: &sampler_use::SamplerUse) -> Option<Self> {
        if !uses.time_scrolled {
            return None;
        }
        let colour = uses.colour_uniform.as_deref();
        if uses.colourless() && uses.lighting_only == ["NormalMap"] {
            return match colour {
                Some("ColourConstant") => Some(Self::Pond),
                Some("Watercolour") => Some(Self::Water),
                _ => None,
            };
        }
        (uses.ambient_samplers == ["NormalMap"]
            && uses.lighting_only.is_empty()
            && uses.ambient_uniforms == ["Opacity", "Watercolour"])
        .then_some(Self::Edge)
    }
}

/// What [`plan`] found, in submeshes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    /// Submeshes whose material the programs classify as 2048's sea.
    pub submeshes: usize,
    /// Of those, drawn with the taps.
    pub drawn: usize,
    /// Of those drawn, the `fc17` batches: opaque water and the blended edge.
    pub fc17_drawn: usize,
    /// Of those, the blended edge.
    pub edge_drawn: usize,
    /// Of those, left flat: a normal map or numbers unread, or the glow table, the third
    /// binding or a glow entry already taken.
    pub left_flat: usize,
    /// Submeshes whose PS4 programs classify the material as Omega's reflective sea; not
    /// counted in [`Self::submeshes`].
    pub reflective: usize,
    /// Of those, drawn with the two taps.
    pub reflective_drawn: usize,
    /// Of those drawn, the blended edge.
    pub reflective_edge_drawn: usize,
    /// Submeshes of Omega's copy of 2048's blended edge (its Zone models), and of those the
    /// ones drawn with the alpha chain; counted in none of the above.
    pub zone_edge: usize,
    pub zone_edge_drawn: usize,
}

impl Census {
    /// A line for [`Report::describe`], empty when no material is a sea.
    pub fn describe(&self) -> String {
        let mut line = String::new();
        if self.submeshes != 0 {
            line += &format!(
                "; 2048's sea: {} of {} submesh(es) draw their scrolled normal map ({} left flat)",
                self.drawn, self.submeshes, self.left_flat
            );
        }
        if self.reflective != 0 {
            line += &format!(
                "; Omega's reflective sea: {} of {} submesh(es) draw their two taps, the sky standing in for the planar reflection ({} blended edge)",
                self.reflective_drawn, self.reflective, self.reflective_edge_drawn
            );
        }
        if self.zone_edge != 0 {
            line += &format!(
                "; Omega's Zone water edge: {} of {} submesh(es) blend by the scrolled map's alpha chain",
                self.zone_edge_drawn, self.zone_edge
            );
        }
        line
    }
}

/// Each sea material's glow-table entry, plus one; `0` for a material that is not drawn as one.
#[derive(Default)]
pub(super) struct Plan {
    entry: Vec<u32>,
    edge: Vec<bool>,
}

impl Plan {
    /// The bits a vertex of material `index` carries for its sea entry.
    pub(super) fn role(&self, index: Option<usize>) -> u32 {
        index
            .and_then(|i| self.entry.get(i))
            .map_or(0, |&e| e << slots::MATERIAL_SHIFT)
    }

    /// Whether material `index` is the blended edge, whose vertices carry
    /// `VertexColour1.xy` in their second coordinate pair.
    pub(super) fn edge(&self, index: Option<usize>) -> bool {
        index
            .and_then(|i| self.edge.get(i))
            .copied()
            .unwrap_or(false)
    }
}

/// Reads each material's programs, and for a sea lays its entry and binds its normal map.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    drawn: bool,
    glow: &mut glow::Plan,
    (twin, normals): (&twin_blend::Plan, &mut normal_map::Plan),
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut plan = Plan {
        entry: vec![0; decoded.materials.len()],
        edge: vec![false; decoded.materials.len()],
    };
    let normal_map = name_hash("NormalMap");
    // One reading per material file: instances share them, and a PS4 file is some ninety
    // programs, so only a file that names the reflection's weight is decoded at all.
    let mut read: std::collections::HashMap<&str, Option<Law>> = std::collections::HashMap::new();
    let laws: Vec<Option<Law>> = decoded
        .materials
        .iter()
        .map(|m| {
            read.entry(m.name.as_str())
                .or_insert_with(|| {
                    let blob = textures(&m.name)?;
                    if !ps4 {
                        return sampler_use::sampler_use(&blob).and_then(|uses| Law::of(&uses));
                    }
                    let names = |marker: &[u8]| blob.windows(marker.len()).any(|w| w == marker);
                    if names(b"ReflectivityStrength") {
                        return oag_rcs::gcn::sea(&blob).map(Law::Reflective);
                    }
                    (names(b"Watercolour") && names(b"VertexColour1") && names(b"Opacity"))
                        .then(|| oag_rcs::gcn::scrolled_edge(&blob))
                        .flatten()
                        .map(|_| Law::ZoneEdge)
                })
                .clone()
        })
        .collect();
    // Per material: whether it was laid.
    let mut laid_before: Vec<Option<bool>> = vec![None; decoded.materials.len()];
    let mut pictures: std::collections::HashMap<&str, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for submesh in &decoded.submeshes {
        let Some(index) = submesh.material.filter(|&m| m < decoded.materials.len()) else {
            continue;
        };
        let Some(law) = &laws[index] else {
            continue;
        };
        let material = &decoded.materials[index];
        let laid = *laid_before[index].get_or_insert_with(|| {
            if !drawn {
                return false;
            }
            let picture = material
                .samplers
                .iter()
                .find(|(hash, _)| *hash == normal_map)
                .map(|(_, path)| path.as_str());
            let free = glow.role(Some(index)) == 0 && twin.of(Some(index)).is_none();
            let map = picture.filter(|_| free).and_then(|path| {
                pictures
                    .entry(path)
                    .or_insert_with(|| super::picture_memo::picture(path, textures, report))
                    .clone()
            });
            let (Some(block), Some(map)) = (block(law, material), map) else {
                return false;
            };
            let Some(at) = lay(&mut glow.emissive, &block) else {
                return false;
            };
            plan.entry[index] = u32::try_from(at + 1).unwrap_or(0);
            plan.edge[index] = law.blended_edge();
            normals.bind(index, map);
            true
        });
        crate::mesh::rcs::census::claim(
            index,
            "sea",
            if laid {
                crate::mesh::rcs::census::Outcome::Drawn
            } else {
                crate::mesh::rcs::census::Outcome::NotDrawn
            },
        );
        match law {
            Law::Reflective(sea) => {
                report.sea.reflective += 1;
                report.sea.reflective_drawn += usize::from(laid);
                report.sea.reflective_edge_drawn += usize::from(laid && sea.opacity);
                continue;
            }
            Law::ZoneEdge => {
                report.sea.zone_edge += 1;
                report.sea.zone_edge_drawn += usize::from(laid);
                continue;
            }
            Law::Pond | Law::Water | Law::Edge => {}
        }
        report.sea.submeshes += 1;
        report.sea.drawn += usize::from(laid);
        report.sea.fc17_drawn += usize::from(laid && *law != Law::Pond);
        report.sea.edge_drawn += usize::from(laid && *law == Law::Edge);
        report.sea.left_flat += usize::from(drawn && !laid);
    }
    plan
}

/// The glow-table entries of one sea material, or `None` where it does not author what its
/// programs read.
fn block(law: &Law, material: &psp2::material::Material) -> Option<Vec<Emissive>> {
    let scalar = |hash| material.param(hash).and_then(|v| v.first().copied());
    let rgb = |hash| {
        let value = material.param(hash)?;
        let &[r, g, b] = value.get(..3)? else {
            return None;
        };
        Some([r, g, b])
    };
    let power = scalar(SPECULAR_POWER)?;
    match law {
        Law::Pond => {
            let taps = Tap::vita_sea();
            Some(sea_block(
                law.kind(),
                rgb(COLOUR_CONSTANT)?,
                0.0,
                power,
                [1.0; 3],
                [&taps[0], &taps[1]],
            ))
        }
        Law::Water | Law::Edge => {
            let taps = Tap::vita_water(scalar(SPEED)?);
            Some(sea_block(
                law.kind(),
                rgb(WATERCOLOUR)?,
                scalar(OPACITY).unwrap_or(1.0),
                power,
                rgb(SPECULAR_COLOUR)?,
                [&taps[0], &taps[1]],
            ))
        }
        Law::ZoneEdge => {
            // The Zone permutation reads the map's alpha alone (`dmask:0x8`), lights the vertex
            // normal and has no highlight: both taps weigh nothing in the normal, and the first
            // still places the alpha fetch at `(u, v + Speed t)`.
            let mut taps = Tap::vita_water(scalar(SPEED)?);
            taps[0].weight = 0.0;
            Some(sea_block(
                law.kind(),
                rgb(WATERCOLOUR)?,
                scalar(OPACITY)?,
                power,
                [0.0; 3],
                [&taps[0], &taps[1]],
            ))
        }
        Law::Reflective(sea) => {
            // The law reads one map through both samplers only where the material binds one
            // file to both, which every shipped instance does; two files would need a second
            // binding, so such a material is left flat.
            let path = |hash| {
                material
                    .samplers
                    .iter()
                    .find(|(h, _)| *h == hash)
                    .map(|(_, path)| path.as_str())
            };
            if path(name_hash("NormalMap"))? != path(NORMAL_MAP1)? {
                return None;
            }
            let strength = if sea.specular_strength {
                scalar(SPECULAR_STRENGTH)?
            } else {
                1.0
            };
            let opacity = if sea.opacity { scalar(OPACITY)? } else { 1.0 };
            let taps = Tap::reflective(sea.taps.rate, sea.taps.signs, scalar(SPEED)?);
            Some(reflective_block(
                law.kind(),
                rgb(name_hash(&sea.colour))?,
                opacity,
                power,
                rgb(SPECULAR_COLOUR)?.map(|c| c * strength),
                [&taps[0], &taps[1]],
                Fresnel {
                    min: scalar(REFLECTIVITY_MIN)?,
                    scale: scalar(REFLECTIVITY_SCALE)?,
                    power: scalar(REFLECTIVITY_POWER)?,
                    strength: scalar(REFLECTIVITY_STRENGTH)?,
                },
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parameter_hashes_are_their_names() {
        assert_eq!(NORMAL_MAP1, name_hash("NormalMap1"));
        assert_eq!(SPECULAR_STRENGTH, name_hash("SpecularStrength"));
        assert_eq!(REFLECTIVITY_MIN, name_hash("ReflectivityMin"));
        assert_eq!(REFLECTIVITY_SCALE, name_hash("ReflectivityScale"));
        assert_eq!(REFLECTIVITY_POWER, name_hash("ReflectivityPower"));
        assert_eq!(REFLECTIVITY_STRENGTH, name_hash("ReflectivityStrength"));
        assert_eq!(name_hash("Watercolour1"), 0x3826_9314);
        assert_eq!(WATERCOLOUR, name_hash("Watercolour"));
        assert_eq!(OPACITY, name_hash("Opacity"));
        assert_eq!(SPEED, name_hash("Speed"));
    }
}
