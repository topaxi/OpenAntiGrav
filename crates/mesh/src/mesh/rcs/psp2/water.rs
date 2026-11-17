//! 2048's own sea: `fc15_effects_vscroll_water`.
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
//! Classified by the programs' declared interface, not by name. A PS4 material is never
//! matched: Omega ships `reflectplane_*` for these meshes and its programs are not GXP. Under
//! `off` nothing is laid, so the draw is the flat colour it was.

use std::sync::Arc;

use oag_rcs::gxp::sampler_use;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2;

use super::{Report, decode_material_texture, glow, normal_map, twin_blend};
use crate::mesh::rcs::Textures;
use crate::mesh::rcs::{Tap, VITA_SEA_KIND, lay, sea_block};
use crate::mesh::{ModelTexture, slots};

/// `~crc32("ColourConstant")`, `SpecularPower`.
const COLOUR_CONSTANT: u32 = 0x4d60_d566;
const SPECULAR_POWER: u32 = 0x81e0_e773;

/// What [`plan`] found, in submeshes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    /// Submeshes whose material the programs classify as 2048's sea.
    pub submeshes: usize,
    /// Of those, drawn with the taps.
    pub drawn: usize,
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
}

impl Plan {
    /// The bits a vertex of material `index` carries for its sea entry.
    pub(super) fn role(&self, index: Option<usize>) -> u32 {
        index
            .and_then(|i| self.entry.get(i))
            .map_or(0, |&e| e << slots::MATERIAL_SHIFT)
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
    };
    if ps4 {
        return plan;
    }
    let normal_map = name_hash("NormalMap");
    // Per material: whether the programs say it is a sea, and whether it was laid.
    let mut read: Vec<Option<(bool, bool)>> = vec![None; decoded.materials.len()];
    let mut pictures: std::collections::HashMap<&str, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for submesh in &decoded.submeshes {
        let Some(index) = submesh.material.filter(|&m| m < decoded.materials.len()) else {
            continue;
        };
        let material = &decoded.materials[index];
        let (is_sea, laid) = *read[index].get_or_insert_with(|| {
            let is_sea = textures(&material.name)
                .and_then(|blob| sampler_use::sampler_use(&blob))
                .is_some_and(|uses| {
                    uses.colourless()
                        && uses.time_scrolled
                        && uses.lighting_only == ["NormalMap"]
                        && uses.colour_uniform.as_deref() == Some("ColourConstant")
                });
            if !(is_sea && drawn) {
                return (is_sea, false);
            }
            let numbers = material
                .param(COLOUR_CONSTANT)
                .zip(material.param(SPECULAR_POWER));
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
            let (Some((colour, power)), Some(map)) = (numbers, map) else {
                return (true, false);
            };
            if colour.len() < 3 || power.is_empty() {
                return (true, false);
            }
            let [tap, none] = Tap::vita_sea();
            let block = sea_block(
                VITA_SEA_KIND,
                [colour[0], colour[1], colour[2]],
                0.0,
                power[0],
                [1.0; 3],
                [&tap, &none],
            );
            let Some(at) = lay(&mut glow.emissive, &block) else {
                return (true, false);
            };
            plan.entry[index] = u32::try_from(at + 1).unwrap_or(0);
            normals.bind(index, map);
            (true, true)
        });
        report.sea.submeshes += usize::from(is_sea);
        report.sea.drawn += usize::from(laid);
        report.sea.left_flat += usize::from(is_sea && drawn && !laid);
    }
    plan
}
