//! `fc07_blend_vertred_lambert_2diff`: two pictures mixed by the vertex colour's red.
//!
//! The file's nine fragment programs that sample a picture (ambient-only, lit, vertex-lit and
//! lightmapped, each with and without the vertex lights) all begin the albedo the same way:
//!
//! ```text
//! albedo = mix(DiffuseMap^2, DiffuseMap01^2, w)      mul  x = A * A
//!                                                    mad  y = B * B - x
//!                                                    mad  z = w * y + x
//! ```
//!
//! `A` is the texel of sampler resource 0 (`DiffuseMap`) and `B` that of resource 1
//! (`DiffuseMap01`), by each program's own input table, and `w` is the fourth lane of the first
//! interpolant, which every vertex program of the file writes from `VertexColour1.x`
//! (`mov o6.y, pa8.x`). The light multiplies the mix afterwards. So a vertex painted red 0 shows
//! `DiffuseMap` and one painted red 255 shows `DiffuseMap01`: Altima's rock faces turn to grass
//! and its gravel shore to earth where the artists painted red
//! (`docs/formats/gxp.md`, "`fc07_blend_vertred_lambert_2diff` mixes two pictures by the vertex
//! red").
//!
//! The instance lists `DiffuseMap` first, so "the first sampler listed"
//! ([`Material::diffuse_texture`]) already names the first picture; what it cannot say is that
//! there is a second.
//!
//! Keyed by the file's stem and the instance's two samplers. Omega's copies of the plain file
//! run the same mix on their own programs but are not admitted here (its `_new` and
//! `2diffnormal` siblings, which most of its circuits use, are unread), and
//! `fc14_blend_vertred_phong` is a different file whose programs were not read.

use super::lighting_only;
use super::material::Material;
use crate::rcsmaterial::name_hash;

/// The family file whose programs were read.
pub const FAMILY: &str = "fc07_blend_vertred_lambert_2diff";

/// The sampler shown where the weight is 0.
pub const FIRST_SAMPLER: &str = "DiffuseMap";

/// The sampler shown where the weight is 1.
pub const SECOND_SAMPLER: &str = "DiffuseMap01";

/// Which lane of `VertexColour1` is the weight: its red.
pub const WEIGHT_LANE: usize = 0;

/// What a family instance binds, by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexBlend<'m> {
    /// The `DiffuseMap` texture: the picture at weight 0.
    pub first: &'m str,
    /// The `DiffuseMap01` texture: the picture at weight 1.
    pub second: &'m str,
}

/// The instance's two pictures, when it is a family member that binds both.
#[must_use]
pub fn of(material: &Material) -> Option<VertexBlend<'_>> {
    if lighting_only::family(material) != FAMILY {
        return None;
    }
    let first = material
        .sampler(name_hash(FIRST_SAMPLER))
        .filter(|p| !p.is_empty())?;
    let second = material
        .sampler(name_hash(SECOND_SAMPLER))
        .filter(|p| !p.is_empty())?;
    Some(VertexBlend { first, second })
}

/// The weight a vertex carries, 0 to 1, from its `VertexColour1` bytes.
#[must_use]
pub fn weight(vertex_colour1: [u8; 4]) -> f32 {
    f32::from(vertex_colour1[WEIGHT_LANE]) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(name: &str, samplers: &[(&str, &str)]) -> Material {
        Material {
            name: name.to_string(),
            technique: None,
            textures: samplers.iter().map(|(_, p)| (*p).to_string()).collect(),
            lightmap: None,
            params: Vec::new(),
            samplers: samplers
                .iter()
                .map(|(n, p)| (name_hash(n), (*p).to_string()))
                .collect(),
            state: None,
        }
    }

    const FILE: &str = "data/art/published/environments/altima/materials/fc07_Blend_VertRed_lambert_2Diff.rcsmaterial";

    #[test]
    fn the_second_picture_is_diffuse_map_01_whatever_the_listing_order() {
        let listed = instance(
            FILE,
            &[
                ("DiffuseMap", "rocks01_d.gxt"),
                ("DiffuseMap01", "grass.gxt"),
            ],
        );
        let swapped = instance(
            FILE,
            &[
                ("DiffuseMap01", "grass.gxt"),
                ("DiffuseMap", "rocks01_d.gxt"),
            ],
        );
        let want = Some(VertexBlend {
            first: "rocks01_d.gxt",
            second: "grass.gxt",
        });
        assert_eq!(of(&listed), want);
        assert_eq!(of(&swapped), want);
    }

    #[test]
    fn the_weight_is_the_red_lane() {
        assert_eq!(weight([0, 255, 255, 255]), 0.0);
        assert_eq!(weight([255, 0, 0, 0]), 1.0);
        assert_eq!(weight([51, 9, 9, 9]), 0.2);
    }

    #[test]
    fn a_family_not_read_keeps_one_picture() {
        let both = [("DiffuseMap", "a.gxt"), ("DiffuseMap01", "b.gxt")];
        for name in [
            "x/materials/fc14_Blend_VertRed_Phong.rcsmaterial",
            "x/materials/fc07_Blend_VertRed_lambert_2Diff_new.rcsmaterial",
            "x/materials/fc07_Blend_VertRed_lambert_2DiffNormal.rcsmaterial",
            "x/materials/fc06_lambert_simple.rcsmaterial",
        ] {
            assert_eq!(of(&instance(name, &both)), None, "{name}");
        }
    }

    #[test]
    fn an_instance_without_the_second_sampler_is_not_a_member() {
        assert_eq!(of(&instance(FILE, &[("DiffuseMap", "a.gxt")])), None);
        let empty = [("DiffuseMap", "a.gxt"), ("DiffuseMap01", "")];
        assert_eq!(of(&instance(FILE, &empty)), None);
    }
}
