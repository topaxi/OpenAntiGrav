//! `HD_Detonator_CannonBolt_Halo` on 2048 and Omega: the Detonator cannon's bolt.
//!
//! HD's arithmetic, read off 2048's USSE code (`oag_rcs::gxp::cannon_bolt`) or Omega's GCN code
//! (`oag_rcs::gcn::cannon_bolt`), with each title's own rim tint, and the colour tap read from
//! the band: 2048's program squares it, Omega's picture is sRGB-typed, so the shader decodes it
//! on both (`docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt on 2048 and
//! Omega"). The two pictures ride the twin bindings ([`super::twin_blend`]), the band as the
//! diffuse and the ring strip as the second, and `Uv1_Uv2`'s second pair is the noise and colour
//! coordinate, its first the ring's.

use std::sync::Arc;

use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::{self, SubMesh, material::Material};

use super::twin_blend::{Plan, Twin};
use super::{Report, picture_memo};
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, ModelTexture, PROGRAM_CANNON_BOLT};

/// The material's samplers, as the program names them.
const BAND: &str = "LightningTexture";
const RING: &str = "Texture1";
const FRESNEL_BASE: &str = "fresnelBase";

/// Whether a submesh carries the four halves the two coordinate pairs come from.
fn routable(submesh: &SubMesh) -> bool {
    let n = submesh.positions.len();
    n > 0 && submesh.packed_texcoords.len() == n
}

/// Whether the PS4 band picture is sRGB-typed, which is what makes the texture unit decode it.
fn srgb_typed(path: &str, textures: Textures<'_>) -> bool {
    textures(path)
        .and_then(|bytes| oag_texture::gnf::Texture::parse(&bytes).ok())
        .is_some_and(|t| t.channel_type == oag_texture::gnf::ChannelType::Srgb)
}

/// The glow-table entry: the program's tint, `fresnelBase`, and `1` for the decoded colour tap.
fn entry(tint: [f32; 3], fresnel: f32) -> Emissive {
    Emissive {
        tint,
        offset: fresnel,
        scale: 0.0,
        rate: 0.0,
        program: [PROGRAM_CANNON_BOLT, 1.0],
    }
}

/// Admits each material whose program reads as the bolt's and whose two pictures and
/// `fresnelBase` read, filing it in `twin` and writing its glow-table entry. A material that
/// authors both samplers and `fresnelBase` and does not read is counted, never guessed past.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    (twin, emissive): (&mut Plan, &mut Vec<Emissive>),
    textures: Textures<'_>,
    report: &mut Report,
) {
    for (index, material) in decoded.materials.iter().enumerate() {
        let Some((band, ring, fresnel)) = candidate(material) else {
            continue;
        };
        if index >= twin.twins.len() {
            continue;
        }
        let read = (|| {
            let file = textures(&material.name)?;
            let tint = if ps4 {
                oag_rcs::gcn::cannon_bolt(&file)?
            } else {
                oag_rcs::gxp::cannon_bolt::cannon_bolt(&file)?
            };
            decoded
                .submeshes
                .iter()
                .filter(|s| s.material == Some(index))
                .all(routable)
                .then_some(())?;
            (!ps4 || srgb_typed(&band, textures)).then_some(())?;
            let mut picture = |path: &str| -> Option<Arc<ModelTexture>> {
                picture_memo::picture(path, textures, report)
            };
            let strip = picture(&ring)?;
            picture(&band)?;
            Some((strip, entry(tint, fresnel)))
        })();
        let Some((strip, entry)) = read else {
            crate::mesh::rcs::census::claim(
                index,
                "cannon_bolt",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.cannon_bolt_unread += 1;
            continue;
        };
        let at = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            crate::mesh::rcs::census::claim(
                index,
                "cannon_bolt",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.cannon_bolt_unread += 1;
            continue;
        };
        twin.admit(
            index,
            Twin {
                slot,
                first: band,
                second: strip.clone(),
                wipe: strip,
                bars: None,
            },
        );
        crate::mesh::rcs::census::claim(
            index,
            "cannon_bolt",
            crate::mesh::rcs::census::Outcome::Drawn,
        );
        report.cannon_bolt_materials += 1;
    }
}

/// The band's and the strip's paths and `fresnelBase`, for a material that authors all three.
fn candidate(material: &Material) -> Option<(String, String, f32)> {
    let sampler = |name: &str| {
        material
            .sampler(name_hash(name))
            .filter(|path| !path.is_empty())
            .map(str::to_string)
    };
    let fresnel = material
        .param(name_hash(FRESNEL_BASE))
        .and_then(|v| v.first().copied())?;
    Some((sampler(BAND)?, sampler(RING)?, fresnel))
}

/// A vertex of the bolt: `(texcoord, lightmap_texcoord, texcoord2, sun_mask)` for a vertex of a
/// material this module admitted, `Uv1_Uv2`'s second pair first.
pub(super) fn vertex(
    twin: Option<&Twin>,
    emissive: &[Emissive],
    submesh: &SubMesh,
    i: usize,
) -> Option<super::electricity::RingVertex> {
    let slot = usize::try_from(twin?.slot).ok()?.checked_sub(1)?;
    (emissive.get(slot)?.program[0] == PROGRAM_CANNON_BOLT).then_some(())?;
    let &[a, b, c, d] = submesh.packed_texcoords.get(i)?;
    Some(([c, d], [a, b], [0.0, 0.0], 1.0))
}
