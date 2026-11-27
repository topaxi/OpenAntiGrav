//! Omega's hull programs multiply the baked occlusion colour into the light.
//!
//! A PS4 hull submesh carries `VertexColour1`, four normalised bytes (a grey occlusion bake). The
//! hull programs fetch it, export it unchanged, and the pixel program multiplies the interpolant
//! into the diffuse sample and the light: `albedo * VertexColour1.rgb * light`
//! (`docs/ghidra/functions/ps4-omega-eu/hull-tone.md`, "Recipe"). The colour is UNORM and
//! multiplied raw, never decoded as sRGB. Left unread it draws white and the hull is too bright.
//!
//! Which materials do this comes from their own programs ([`oag_rcs::gcn::input_is_factor`]): the
//! vertex program exports the input and the pixel program after it reads that parameter only as a
//! multiplicand, and one of those pixel programs takes the scene's light. The marker rides `sun_mask` ([`crate::mesh::factor_sun_mask`]) and `shade.wesl`'s
//! `colour_factor` arm is the product, the one HD's hull already takes.
//!
//! **PS4 only.** 2048's programs are GXP, whose operands nothing reads (`docs/formats/gxp.md`).

use std::collections::BTreeMap;

use oag_rcs::gcn::input_is_factor;
use oag_rcs::rcsmodel::psp2;

use super::Textures;

/// `VertexColour1` by the name the reflection gives it.
const INPUT: &str = "VertexColour1";

/// What [`plan`] and the vertex loop counted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    /// Materials whose programs take the colour as a factor.
    pub materials: usize,
    /// Materials with a colour-carrying submesh whose file could not be read.
    pub unread: usize,
    /// Submeshes drawn with the colour as a factor, by material file.
    pub submeshes: BTreeMap<String, usize>,
}

impl Census {
    /// A line for [`super::Report::describe`], empty when no material takes the colour.
    pub fn describe(&self) -> String {
        if self.materials + self.unread == 0 {
            return String::new();
        }
        format!(
            "; {} material(s) multiply VertexColour1 into the light ({} submesh(es), {} unread)",
            self.materials,
            self.submeshes.values().sum::<usize>(),
            self.unread
        )
    }
}

/// Per material, whether its programs take `VertexColour1` as a factor on the light.
#[derive(Default)]
pub(super) struct Plan {
    factor: Vec<bool>,
}

impl Plan {
    /// Whether material `index` multiplies the colour into its light.
    pub(super) fn of(&self, index: Option<usize>) -> bool {
        index.is_some_and(|i| self.factor.get(i).copied().unwrap_or(false))
    }
}

/// Reads the programs of each material of a PS4 model that carries a `VertexColour1` stream.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    textures: Textures<'_>,
    census: &mut Census,
) -> Plan {
    let mut plan = Plan {
        factor: vec![false; decoded.materials.len()],
    };
    if !ps4 {
        return plan;
    }
    let mut carrying = vec![false; decoded.materials.len()];
    for submesh in decoded.submeshes.iter().filter(|s| !s.colours.is_empty()) {
        if let Some(slot) = submesh.material.and_then(|m| carrying.get_mut(m)) {
            *slot = true;
        }
    }
    for (index, material) in decoded.materials.iter().enumerate() {
        if !carrying[index] {
            continue;
        }
        match textures(&material.name) {
            None => census.unread += 1,
            Some(blob) => {
                plan.factor[index] = input_is_factor(&blob, INPUT).is_factor();
                census.materials += usize::from(plan.factor[index]);
                if plan.factor[index] {
                    crate::mesh::rcs::census::claim(
                        index,
                        "hull_colour",
                        crate::mesh::rcs::census::Outcome::Drawn,
                    );
                }
            }
        }
    }
    plan
}

/// `colour` and `sun_mask` of one vertex that carries the colour as a factor: the bytes over 255,
/// alpha one.
pub(super) fn vertex([r, g, b, _]: [u8; 4]) -> ([f32; 4], f32) {
    (
        [
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ],
        crate::mesh::factor_sun_mask(1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vertex_carries_the_bytes_over_255_and_the_factor_marker() {
        let (colour, sun_mask) = vertex([255, 51, 0, 7]);
        assert_eq!(colour, [1.0, 0.2, 0.0, 1.0]);
        assert!(sun_mask < 0.0);
        assert_eq!(sun_mask, crate::mesh::factor_sun_mask(1.0));
    }

    #[test]
    fn a_plan_names_no_material_it_was_not_given() {
        let plan = Plan::default();
        assert!(!plan.of(None));
        assert!(!plan.of(Some(3)));
    }
}
