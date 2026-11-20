//! The materials whose vertex programs write `1 - v` into the diffuse coordinate.
//!
//! Four of 2048's material files do, in every permutation: Amphiseum's `track_surface` and
//! `track_surface_no_emissive`, Modesto Heights' `track_wall` and Tech De Ra's `track_surface`
//! (`oag_rcs::gxp::texcoord_flip`, `docs/formats/2048-material-params.md`, "The track wall that
//! writes `1 - v`"). Drawn with the file's own `v`, Modesto's wall showed the picture's
//! chevron band where the original shows its panel and hex rows. HD's circuits ship the same
//! materials and flip the same way (`skin::flips`, read off the RSX microcode). Omega's GCN
//! programs flip in three files (`oag_rcs::gcn::flips_v`, `docs/formats/omega-status.md`, "The
//! diffuse coordinate that writes `1 - v`").
//!
//! **Read per material file, off its own programs, never by name.**

use oag_rcs::rcsmodel::psp2;
use oag_rcs::{gcn, gxp};

use super::Report;
use crate::mesh::rcs::Textures;

/// Which material slots flip `v`.
#[derive(Default)]
pub(super) struct Plan {
    flipped: Vec<bool>,
}

/// Reads each material file once for the flip.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut by_file: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    let flipped: Vec<bool> = decoded
        .materials
        .iter()
        .map(|material| {
            *by_file.entry(material.name.as_str()).or_insert_with(|| {
                textures(&material.name).is_some_and(|blob| {
                    if ps4 {
                        gcn::flips_v(&blob).is_flip()
                    } else {
                        gxp::texcoord_flip::flips_v(&blob)
                    }
                })
            })
        })
        .collect();
    report.flipped_v_materials += flipped.iter().filter(|f| **f).count();
    Plan { flipped }
}

impl Plan {
    /// The diffuse coordinate of a vertex of `material`.
    pub(super) fn apply(&self, material: Option<usize>, [u, v]: [f32; 2]) -> [f32; 2] {
        if material.is_some_and(|m| self.flipped.get(m).copied().unwrap_or(false)) {
            [u, 1.0 - v]
        } else {
            [u, v]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_rcs::rcsmodel::psp2::material::Material;

    const ARCHIVE: &str = "../../data/extracted/ps4/omega-eu/uroot/data02.psarc";
    const WALL: &str = "Data/environments/modesto_heights/materials/Track_Wall.rcsmaterial";
    const SURFACE: &str = "Data/environments/modesto_heights/materials/Track_Surface.rcsmaterial";

    fn model(names: &[&str]) -> psp2::Model {
        let material = |name: &str| Material {
            name: name.to_string(),
            technique: None,
            textures: Vec::new(),
            lightmap: None,
            params: Vec::new(),
            samplers: Vec::new(),
            state: None,
        };
        psp2::Model {
            sections: Vec::new(),
            submeshes: Vec::new(),
            unpaired_pointers: 0,
            materials: names.iter().map(|n| material(n)).collect(),
            scene: psp2::nodes::Scene::default(),
        }
    }

    /// Which of `names` (entries of Omega's `data02.psarc`) `plan` flips, with the PS4 or the
    /// Vita reader. `None` without the extraction.
    fn flips(names: &[&str], ps4: bool) -> Option<Vec<bool>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ARCHIVE);
        if !path.exists() {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            return None;
        }
        let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
        let mut read = |name: &str| archive.read_path(name).ok();
        let mut report = Report::default();
        let planned = plan(&model(names), ps4, &mut read, &mut report);
        let flipped: Vec<bool> = (0..names.len())
            .map(|k| planned.apply(Some(k), [0.25, 0.25]) == [0.25, 0.75])
            .collect();
        assert_eq!(
            report.flipped_v_materials,
            flipped.iter().filter(|f| **f).count()
        );
        Some(flipped)
    }

    #[test]
    #[ignore = "needs the Omega extraction"]
    fn omegas_track_wall_flips_through_the_gcn_reader_and_its_track_surface_does_not() {
        let Some(found) = flips(&[WALL, SURFACE], true) else {
            return;
        };
        assert_eq!(found, [true, false]);
    }

    #[test]
    #[ignore = "needs the Omega extraction"]
    fn the_vita_reader_reads_nothing_in_a_gcn_file() {
        let Some(found) = flips(&[WALL], false) else {
            return;
        };
        assert_eq!(found, [false]);
    }
}
