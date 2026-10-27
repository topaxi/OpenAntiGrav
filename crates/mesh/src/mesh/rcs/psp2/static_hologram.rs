//! `dc_hologramwithstatic` on Omega's Amphiseum: HD's mode 7 law, **without HD's `/2`**.
//!
//! Omega's plain `dc_hologramwithstatic.rcsmaterial` (the `_1`/`_2` files are the PS4 Pro
//! compiles) names HD's three pictures under HD's three sampler hashes and carries HD's three
//! rates under HD's three uniform hashes, with the same values (`Speed = 1.0`, `Speed2 =
//! 4.5384`, `Alpha = 0.73802`). Its GCN pixel program, read with `oag_rcs::gcn`, is HD's
//! arithmetic instruction for instruction: `A = Texture1(u + time Speed, v)`, `C =
//! TextureGradient(u, v + time Speed2)`, `alpha = AlphaMask(u, v).x Alpha`, `colour =
//! fog(A + C)` (`docs/rendering/hd-track-tone.md`, "The Amphiseum hologram board").
//!
//! **The `/2` is not carried.** HD's program ends on one, and the shader's mode 7 applies it as
//! `scene.light.output_scale`, which the loader sets to `0.5` only for a title whose
//! `lit_output_halved` rule applies (HD). Omega leaves that rule unread, so its scene's scale
//! is `1.0` and this path drops exactly the halving with no change to the shader.
//!
//! **Inherited from HD, not measured on Omega:** the three units' gamma decoding (HD's
//! was read live off RPCS3, `SET_TEXTURE_ADDRESS`); which GCN texture slot is which unit is
//! inferred from the resource table's order (confidence 75).
//!
//! The material rides the bindings `twin_blend` already gives a two-picture material: the scan
//! mask is the diffuse, the advert the second, the static the third, the three rates one
//! glow-table entry of [`PROGRAM_STATIC_HOLOGRAM`].

use std::sync::Arc;

use oag_rcs::rcsmodel::psp2;

use super::twin_blend::{Plan, Twin};
use super::{Report, decode_material_texture};
use crate::mesh::rcs::Textures;
use crate::mesh::rcs::static_hologram::{
    ADVERT, ADVERT_RATE, ALPHA_SCALE, SCAN, STATIC, STATIC_RATE,
};
use crate::mesh::{Emissive, ModelTexture, PROGRAM_STATIC_HOLOGRAM};

/// Whether a material is the plain `dc_hologramwithstatic` by file name.
fn is_static_hologram(name: &str) -> bool {
    let stem = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = stem.strip_suffix(".rcsmaterial").unwrap_or(stem);
    stem.eq_ignore_ascii_case("dc_hologramwithstatic")
}

/// Admits each `dc_hologramwithstatic` material whose three pictures decode and whose three
/// rates are authored, filing it in `twin` and writing its glow-table entry.
pub(super) fn plan(
    decoded: &psp2::Model,
    (twin, emissive): (&mut Plan, &mut Vec<Emissive>),
    textures: Textures<'_>,
    report: &mut Report,
) {
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for (index, material) in decoded.materials.iter().enumerate() {
        if !is_static_hologram(&material.name) || index >= twin.twins.len() {
            continue;
        }
        let path = |hash| material.sampler(hash).filter(|p| !p.is_empty());
        let rate = |hash| material.param(hash).and_then(|v| v.first().copied());
        let (Some(scan), Some(advert), Some(noise)) = (path(SCAN), path(ADVERT), path(STATIC))
        else {
            report.static_hologram_unread += 1;
            continue;
        };
        let (Some(advert_rate), Some(static_rate), Some(alpha)) =
            (rate(ADVERT_RATE), rate(STATIC_RATE), rate(ALPHA_SCALE))
        else {
            report.static_hologram_unread += 1;
            continue;
        };
        let mut picture = |path: &str| {
            cache
                .entry(path.to_string())
                .or_insert_with(|| {
                    textures(path)
                        .and_then(|blob| decode_material_texture(path, &blob, report))
                        .map(Arc::new)
                })
                .clone()
        };
        let (Some(advert), Some(noise), Some(_)) = (picture(advert), picture(noise), picture(scan))
        else {
            report.static_hologram_unread += 1;
            continue;
        };
        let entry = Emissive {
            tint: [advert_rate, static_rate, alpha],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [PROGRAM_STATIC_HOLOGRAM, 0.0],
        };
        let at = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            report.static_hologram_unread += 1;
            continue;
        };
        twin.admit(
            index,
            Twin {
                slot,
                first: scan.to_string(),
                second: advert,
                wipe: noise,
                bars: None,
            },
        );
        report.static_hologram_materials += 1;
    }
}
