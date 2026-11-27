//! `tunnel_fx_noalpha`'s window lights on a lightmapped slot.
//!
//! Both plain programs of the material (`HalfBrightAmbientSunSpot0SVC0`, fragment block `0x4f80`,
//! and `HalfBrightIleLightmapSunSpot0SVC0`, `0x5d20`) sample `EmissiveTexture` (`0xb1f2a176`,
//! `tunnel_fx_emissive.gtf`) at unit 2 at the diffuse coordinate and fold it in after the light:
//!
//! ```text
//! out = (fog(light * D + spec * S + 1 - (1 - E) * (1 - X)) + TC1) / 2
//! ```
//!
//! `X` the facing-ramp term, so `E` is added unlit and ungated by the diffuse alpha. The generic
//! glow reading binds that texture second only where no lightmap took the binding
//! (`super::emissive`); on a lightmapped slot the lightmap holds it and the windows went dark. This
//! binds the emissive third and tags the slot with a [`PROGRAM_TUNNEL_EMISSIVE`] entry, which
//! `shade.wesl` adds on a plain program only: no Zone variant of the material declares the sampler
//! (`docs/rendering/hd-zone-recolour.md`, "Window lights on a plain chunk").
//!
//! Left out, named: the `X` term (facing ramp times specular map), which no path of ours draws for
//! this family, so the screen blend reduces to the add.

use std::sync::Arc;

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmodel;

use crate::mesh::{Emissive, PROGRAM_TUNNEL_EMISSIVE, TextureSlots, slots};

use super::emissive::{EMISSIVE_LIMIT, family};
use super::skin::decode_texture;
use super::{Report, Textures};

/// The family this reading covers.
const FAMILY: &str = "tunnel_fx_noalpha";
/// `EmissiveTexture`.
pub(super) const EMISSIVE: u32 = 0xb1f2_a176;
/// The circuit lightmap's sampler, which takes the second binding.
const LIGHTMAP: u32 = 0x37b5_db58;

/// The entry every bound slot shares.
const ENTRY: Emissive = Emissive {
    tint: [1.0, 1.0, 1.0],
    offset: 0.0,
    scale: 1.0,
    rate: 0.0,
    program: [PROGRAM_TUNNEL_EMISSIVE, 0.0],
};

/// Binds the emissive of each lightmapped `tunnel_fx_noalpha` slot third (returned) and points the
/// slot at the shared entry. `thirds` is what earlier families bound third: a slot holding one is
/// left alone.
pub(super) fn tunnel_emissive(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    thirds: &TextureSlots,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> TextureSlots {
    let mut bound: TextureSlots = vec![None; model.materials.len()];
    for (slot, material) in model.materials.iter().enumerate() {
        if family(material) != FAMILY {
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if *word & slots::SECOND_IS_LIGHTMAP == 0 {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(declared) = textures(&format!("/{}", material.name))
            .and_then(|blob| rcsmaterial::Declared::parse(&blob, variant.fragment.offset))
        else {
            report.tunnel_emissive_unread += 1;
            continue;
        };
        let declares = |hash: u32| declared.samplers.iter().any(|(h, _)| *h == hash);
        if !(declares(EMISSIVE) && declares(LIGHTMAP)) {
            continue;
        }
        if slots::material_index(*word) != 0 || thirds.get(slot).is_some_and(Option::is_some) {
            report.tunnel_emissive_unread += 1;
            continue;
        }
        let picture = material
            .samplers
            .iter()
            .find(|(h, p)| *h == EMISSIVE && p.is_some())
            .and_then(|(_, p)| p.clone())
            .and_then(|path| {
                textures(&format!("/{path}")).and_then(|bytes| decode_texture(&path, &bytes))
            });
        let index = table.iter().position(|seen| *seen == ENTRY).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(ENTRY);
                table.len() - 1
            })
        });
        let (Some(picture), Some(index)) = (picture, index) else {
            report.tunnel_emissive_unread += 1;
            continue;
        };
        bound[slot] = Some(Arc::new(picture));
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.tunnel_emissive_bound += 1;
    }
    bound
}
