//! Omega's `tarmac_markings_dirt_markingsnormal[_emis]`: a road whose diffuse picture is
//! overlaid by a dirt splat and a decal atlas.
//!
//! Read off the pixel programs (`docs/formats/omega-status.md`, "The `tarmac_*` pixel programs
//! combine four pictures", confidence 85), unlit and lit variants alike:
//!
//! ```text
//! c = diffuse(uv1_uvDirt.xy)
//! c = lerp(c, dirt.rgb,     dirt.a)        dirt     on uv1_uvDirt.zw
//! c = lerp(c, markings.rgb, markings.a)    markings on uvMarkings
//! ```
//!
//! The two layers ride the fifth (markings) and sixth (dirt) bindings and a glow-table entry of
//! mode [`PROGRAM_TARMAC`]. The coordinates: the markings pair in `texcoord2` (the shader's
//! `lightmap_texcoord.zw`), the dirt pair in `colour.xy` with `colour.zw` left at `1`, because
//! the road has no authored vertex colour and the vertex format has no room for a fourth pair.
//! The fragment stage treats the vertex colour as white on this mode.
//!
//! Admitted per material by name and by both samplers carrying a path that decodes; a material
//! one of whose submeshes lacks either pair is counted and left to the generic reading. The
//! `tarmac_dirt` families are not drawn here: their combines were read by sample order only. The
//! `_emis` member's `EmissiveTexture` rides the fourth binding and is added after the light at
//! the diffuse pair.

use std::sync::Arc;

use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::{Report, decode_material_texture};
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, ModelTexture, PROGRAM_TARMAC, slots};

/// The `Markings` sampler's hash: `~crc32("Markings")`.
const MARKINGS: u32 = 0x4a5b_d928;
/// The `Dirt` sampler's hash: `~crc32("Dirt")`.
const DIRT: u32 = 0xc7aa_b177;
/// Set (to anything) to draw the dirt layer. **Off by default**: with it on, Rockway Stadium's road
/// reads darker than the original in two of three boxes, and the cause is unread
/// (`docs/formats/omega-status.md`, "The `tarmac_*` pixel programs combine four pictures").
pub const DIRT_SWITCH: &str = "OAG_TARMAC_DIRT";
/// The `EmissiveTexture` sampler's hash: `~crc32("EmissiveTexture")`.
const EMISSIVE: u32 = 0xb1f2_a176;

/// One admitted material: its glow-table slot (entry index plus one) and its two pictures.
pub(super) struct Layers {
    pub slot: u32,
    pub markings: Arc<ModelTexture>,
    pub dirt: Arc<ModelTexture>,
    /// `EmissiveTexture`, added after the light at the diffuse coordinate; `None` on the
    /// family without one.
    pub emissive: Option<Arc<ModelTexture>>,
}

/// Per material, the layers it binds.
#[derive(Default)]
pub(super) struct Plan {
    pub layers: Vec<Option<Layers>>,
}

impl Plan {
    pub fn of(&self, material: Option<usize>) -> Option<&Layers> {
        self.layers.get(material?)?.as_ref()
    }

    pub fn any(&self) -> bool {
        self.layers.iter().any(Option::is_some)
    }

    /// The vertex bits that select this material's glow-table entry; `0` for any other.
    pub fn role(&self, material: Option<usize>) -> u32 {
        self.of(material)
            .map_or(0, |l| l.slot << slots::MATERIAL_SHIFT)
    }
}

/// The two pictures' paths, when `material` is one of the two `markings` families.
fn paths(material: &Material) -> Option<[&str; 2]> {
    let stem = material.name.rsplit(['/', '\\']).next()?;
    let stem = stem.strip_suffix(".rcsmaterial").unwrap_or(stem);
    if !stem.eq_ignore_ascii_case("tarmac_markings_dirt_markingsnormal")
        && !stem.eq_ignore_ascii_case("tarmac_markings_dirt_markingsnormal_emis")
    {
        return None;
    }
    let path = |hash| material.sampler(hash).filter(|p| !p.is_empty());
    Some([path(MARKINGS)?, path(DIRT)?])
}

/// Admits each road material whose pictures decode and whose every submesh carries both pairs,
/// writing its one glow-table entry into `emissive`. Empty wherever no submesh names both
/// `uvMarkings` and `uv1_uvDirt`, which is every Vita model.
pub(super) fn plan(
    decoded: &psp2::Model,
    emissive: &mut Vec<Emissive>,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    let dirt_weight = if std::env::var_os(DIRT_SWITCH).is_some() {
        1.0
    } else {
        0.0
    };
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for (index, material) in decoded.materials.iter().enumerate() {
        out.layers.push(None);
        let Some(paths) = paths(material) else {
            continue;
        };
        let readable = decoded
            .submeshes
            .iter()
            .filter(|s| s.material == Some(index))
            .all(|s| {
                !s.positions.is_empty()
                    && s.markings_texcoords.len() == s.positions.len()
                    && s.dirt_texcoords.len() == s.positions.len()
            });
        if !readable {
            report.tarmac_unread += 1;
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
        let [markings, dirt] = paths.map(&mut picture);
        let emissive_path = material.sampler(EMISSIVE).filter(|p| !p.is_empty());
        let emissive_picture = emissive_path.map(&mut picture);
        let (Some(markings), Some(dirt)) = (markings, dirt) else {
            report.tarmac_unread += 1;
            continue;
        };
        let has_emissive = matches!(emissive_picture, Some(Some(_)));
        let entry = Emissive {
            tint: [if has_emissive { 1.0 } else { 0.0 }, dirt_weight, 0.0],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [PROGRAM_TARMAC, 0.0],
        };
        let at = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            report.tarmac_unread += 1;
            continue;
        };
        out.layers[index] = Some(Layers {
            slot,
            markings,
            dirt,
            emissive: emissive_picture.flatten(),
        });
        report.tarmac_materials += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_rcs::rcsmaterial::name_hash;

    #[test]
    fn the_sampler_constants_are_the_hashes_of_their_names() {
        assert_eq!(MARKINGS, name_hash("Markings"));
        assert_eq!(DIRT, name_hash("Dirt"));
        assert_eq!(EMISSIVE, name_hash("EmissiveTexture"));
    }

    #[test]
    fn only_the_two_markings_families_are_admitted() {
        let named = |name: &str| Material {
            name: name.to_string(),
            technique: None,
            textures: Vec::new(),
            lightmap: None,
            params: Vec::new(),
            samplers: vec![(MARKINGS, "m.gnf".into()), (DIRT, "d.gnf".into())],
            state: None,
        };
        let admitted = |n: &str| paths(&named(n)).is_some();
        assert!(admitted(
            "x/materials/tarmac_markings_dirt_markingsnormal_emis.rcsmaterial"
        ));
        assert!(admitted(
            "x/materials/tarmac_markings_dirt_markingsnormal.rcsmaterial"
        ));
        assert!(!admitted("x/materials/tarmac_dirt.rcsmaterial"));
        assert!(!admitted("x/materials/tarmac_dirt_emissive.rcsmaterial"));
    }
}
