//! `nr_twinblend` on 2048's and Omega's circuits: HD's law, **inherited from HD**.
//!
//! 2048 (DLC1 Anulpha Pass, Chenghou Project, Metropia, Sebenco Climb and Tech de Ra) and Omega
//! (the same five circuits) re-ship HD/Fury's `nr_twinblend` material beside the same four
//! textures under the same four sampler hashes, and its vertices carry the same four-lane
//! attribute (`barsUV_blendUV`, HD's `v[8]`: bars in `.xy`, wipe in `.zw`). The programs are GXP
//! and GCN, which nothing here reads, so the draw is HD's (`super::super::twin_blend`, resolved
//! from `tech_de_ra/materials/nr_twinblend.rcsmaterial`): two pictures cross-faded by a wipe
//! scrolling at the clock rate, striped by the bars.
//!
//! **Inherited from HD, not measured on either title:** the combine, the units' decoding and the
//! rate [`INHERITED_RATE`]. The authored `time` (`19992.0` on 2048) is not a rate, as
//! `super::glow` already holds for its own materials. A material is admitted by its
//! name and by all four sampler hashes carrying a path; one whose submeshes do not all carry the
//! pair attribute, or one of whose pictures will not decode, is counted and left to the generic
//! reading.

use std::sync::Arc;

use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::{Report, decode_material_texture};
use crate::mesh::rcs::Textures;
use crate::mesh::rcs::twin_blend::{BARS, BLEND, FIRST, SECOND};
use crate::mesh::{Emissive, ModelTexture, PROGRAM_TWIN_BLEND};

/// The wipe's scroll rate, per second of the engine clock: the literal `c[207].y` of HD's
/// vertex block (`oag_rcs::rcsmaterial::vertex::Program::clock_rate` reads `1.875` from it).
/// **Inherited from HD**; `hd_twin_blend_ground_truth` pins it against HD's own reading.
pub const INHERITED_RATE: f32 = 1.875;

/// One admitted material: its glow-table slot (entry index plus one) and four decoded pictures.
pub(super) struct Twin {
    pub slot: u32,
    pub first: String,
    pub second: Arc<ModelTexture>,
    pub wipe: Arc<ModelTexture>,
    pub bars: Arc<ModelTexture>,
}

/// Per material, the twin it binds.
#[derive(Default)]
pub(super) struct Plan {
    pub twins: Vec<Option<Twin>>,
}

impl Plan {
    pub fn of(&self, material: Option<usize>) -> Option<&Twin> {
        self.twins.get(material?)?.as_ref()
    }

    pub fn any(&self) -> bool {
        self.twins.iter().any(Option::is_some)
    }
}

/// Whether a material is HD's `nr_twinblend` by name, and its four pictures' paths in the order
/// `(first, second, wipe, bars)`.
fn paths(material: &Material) -> Option<[&str; 4]> {
    let stem = material.name.rsplit(['/', '\\']).next()?;
    let stem = stem.strip_suffix(".rcsmaterial").unwrap_or(stem);
    if !stem.eq_ignore_ascii_case("nr_twinblend") {
        return None;
    }
    let path = |hash| material.sampler(hash).filter(|p| !p.is_empty());
    Some([path(FIRST)?, path(SECOND)?, path(BLEND)?, path(BARS)?])
}

/// Admits each `nr_twinblend` material whose pictures decode and whose every submesh carries the
/// pair attribute, writing its one glow-table entry into `emissive`.
pub(super) fn plan(
    decoded: &psp2::Model,
    emissive: &mut Vec<Emissive>,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for (index, material) in decoded.materials.iter().enumerate() {
        out.twins.push(None);
        let Some(paths) = paths(material) else {
            continue;
        };
        let readable = decoded
            .submeshes
            .iter()
            .filter(|s| s.material == Some(index))
            .all(|s| !s.positions.is_empty() && s.bars_blend_texcoords.len() == s.positions.len());
        if !readable {
            report.twin_blend_unread += 1;
            continue;
        }
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
        let pictures = paths.map(|p| picture(p));
        let [Some(_), Some(second), Some(wipe), Some(bars)] = pictures else {
            report.twin_blend_unread += 1;
            continue;
        };
        let entry = Emissive {
            tint: [INHERITED_RATE, 0.0, 0.0],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [PROGRAM_TWIN_BLEND, 0.0],
        };
        let at = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            report.twin_blend_unread += 1;
            continue;
        };
        out.twins[index] = Some(Twin {
            slot,
            first: paths[0].to_string(),
            second,
            wipe,
            bars,
        });
        report.twin_blend_materials += 1;
    }
    out
}
