//! 2048's own seas: `fc15_effects_vscroll_water` and the `fc17` pair.
//!
//! The material's programs declare one sampler, `NormalMap`, that only the lit permutations
//! read, a constant `ColourConstant`, `SpecularPower`, and a vertex program that declares
//! `time` ([`oag_rcs::gxp::sampler_use`]). Its fragment program (`docs/rendering/water.md`,
//! "2048 (Vita)") decodes the map as `2t - 1` on all three channels into the chunk's tangent
//! frame, lights it as `(ambient + diffuse * sat(N.L)) * ColourConstant`, and adds
//! `liveLighting0specular * t.a * sat(R.L)^SpecularPower`, `R` the view reflected about the
//! mapped normal. The vertex program adds `0.05 * time` to the first coordinate's `u`.
//! `liveLighting0specular` is drawn as the sun's diffuse colour (chosen, not measured: 2048's
//! envsettings author no sun specular colour).
//!
//! The sea rides HD's tap entry (`mesh::rcs::water::sea_block`, kind
//! [`VITA_SEA_KIND`]) with the second tap at weight zero, the map bound third as the PS4's
//! normal maps are (`psp2::normal_map`), and the tangent the declaration carries.
//!
//! **`fc17`** (Park's `fc17_effects_vscroll_water`, Altima's blended
//! `fc17_effects_vscroll_wateredge_vertexcol`, `docs/rendering/water.md`, "2048 (Vita)") shares
//! that frame and light, takes its colour from `Watercolour`, multiplies the highlight by
//! `SpecularColour` with no gloss from the map's alpha, and scrolls `v` by `Speed * time`. The
//! edge adds an alpha chain over `Opacity`, the map's alpha and the two vertex colour lanes
//! `VertexColour1.x` and `.y`, which ride the vertex's second coordinate pair.
//!
//! Classified by the programs' declared interface, not by name. A PS4 material is never
//! matched: Omega ships `reflectplane_*` for these meshes and its programs are not GXP. Under
//! `off` nothing is laid, so the draw is the flat colour it was.

use std::sync::Arc;

use oag_rcs::gxp::sampler_use;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2;

use super::{Report, decode_material_texture, glow, normal_map, twin_blend};
use crate::mesh::rcs::Textures;
use crate::mesh::rcs::{Tap, VITA_EDGE_KIND, VITA_SEA_KIND, VITA_WATER_KIND, lay, sea_block};
use crate::mesh::{ModelTexture, slots};

/// `~crc32("ColourConstant")`, `SpecularPower`.
const COLOUR_CONSTANT: u32 = 0x4d60_d566;
const SPECULAR_POWER: u32 = 0x81e0_e773;
/// `~crc32("Watercolour")`, `SpecularColour`, `Speed`, `Opacity`.
const WATERCOLOUR: u32 = 0xfe51_d093;
const SPECULAR_COLOUR: u32 = 0x370a_63cb;
const SPEED: u32 = 0x3118_2e0d;
const OPACITY: u32 = 0x73bf_ab05;

/// Which of 2048's seas a material's programs declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Law {
    /// `fc15`: `ColourConstant`, the highlight weighted by the map's alpha.
    Pond,
    /// `fc17`, opaque: `Watercolour` and `SpecularColour`, no gloss.
    Water,
    /// `fc17`'s blended edge: [`Law::Water`] plus the alpha chain.
    Edge,
}

impl Law {
    fn kind(self) -> f32 {
        match self {
            Self::Pond => VITA_SEA_KIND,
            Self::Water => VITA_WATER_KIND,
            Self::Edge => VITA_EDGE_KIND,
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
}

impl Census {
    /// A line for [`Report::describe`], empty when no material is a sea.
    pub fn describe(&self) -> String {
        if self.submeshes == 0 {
            return String::new();
        }
        format!(
            "; 2048's sea: {} of {} submesh(es) draw their scrolled normal map ({} left flat)",
            self.drawn, self.submeshes, self.left_flat
        )
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

/// Reads each Vita material's programs, and for a sea lays its entry and binds its normal map.
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
    if ps4 {
        return plan;
    }
    let normal_map = name_hash("NormalMap");
    let laws: Vec<Option<Law>> = decoded
        .materials
        .iter()
        .map(|m| {
            textures(&m.name)
                .and_then(|blob| sampler_use::sampler_use(&blob))
                .and_then(|uses| Law::of(&uses))
        })
        .collect();
    // The edge authors no `Watercolour` (Altima's does not): it borders the same circuit's
    // `fc15` pond and takes that pond's `ColourConstant`. Chosen, not measured.
    let pond_colour = decoded
        .materials
        .iter()
        .zip(&laws)
        .filter(|(_, law)| **law == Some(Law::Pond))
        .find_map(|(m, _)| m.param(COLOUR_CONSTANT));
    // Per material: whether it was laid.
    let mut laid_before: Vec<Option<bool>> = vec![None; decoded.materials.len()];
    let mut pictures: std::collections::HashMap<&str, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for submesh in &decoded.submeshes {
        let Some(index) = submesh.material.filter(|&m| m < decoded.materials.len()) else {
            continue;
        };
        let Some(law) = laws[index] else {
            continue;
        };
        let material = &decoded.materials[index];
        let laid = *laid_before[index].get_or_insert_with(|| {
            if !drawn {
                return false;
            }
            let colour_param = if law == Law::Pond {
                COLOUR_CONSTANT
            } else {
                WATERCOLOUR
            };
            let colour = material
                .param(colour_param)
                .or_else(|| (law == Law::Edge).then(|| pond_colour.clone()).flatten());
            let power = material.param(SPECULAR_POWER);
            let picture = material
                .samplers
                .iter()
                .find(|(hash, _)| *hash == normal_map)
                .map(|(_, path)| path.as_str());
            let free = glow.role(Some(index)) == 0 && twin.of(Some(index)).is_none();
            let map = picture.filter(|_| free).and_then(|path| {
                pictures
                    .entry(path)
                    .or_insert_with(|| {
                        let blob = textures(path)?;
                        decode_material_texture(path, &blob, report).map(Arc::new)
                    })
                    .clone()
            });
            let (Some(colour), Some(power), Some(map)) = (colour, power, map) else {
                return false;
            };
            if colour.len() < 3 || power.is_empty() {
                return false;
            }
            let scalar = |hash| material.param(hash).and_then(|v| v.first().copied());
            let (taps, specular, opacity) = match law {
                Law::Pond => (Tap::vita_sea(), [1.0; 3], 0.0),
                Law::Water | Law::Edge => {
                    let Some(speed) = scalar(SPEED) else {
                        return false;
                    };
                    let specular = material.param(SPECULAR_COLOUR);
                    let Some(&[r, g, b]) = specular.as_deref().and_then(|v| v.get(..3)) else {
                        return false;
                    };
                    (
                        Tap::vita_water(speed),
                        [r, g, b],
                        scalar(OPACITY).unwrap_or(1.0),
                    )
                }
            };
            let block = sea_block(
                law.kind(),
                [colour[0], colour[1], colour[2]],
                opacity,
                power[0],
                specular,
                [&taps[0], &taps[1]],
            );
            let Some(at) = lay(&mut glow.emissive, &block) else {
                return false;
            };
            plan.entry[index] = u32::try_from(at + 1).unwrap_or(0);
            plan.edge[index] = law == Law::Edge;
            normals.bind(index, map);
            true
        });
        report.sea.submeshes += 1;
        report.sea.drawn += usize::from(laid);
        report.sea.fc17_drawn += usize::from(laid && law != Law::Pond);
        report.sea.edge_drawn += usize::from(laid && law == Law::Edge);
        report.sea.left_flat += usize::from(drawn && !laid);
    }
    plan
}
