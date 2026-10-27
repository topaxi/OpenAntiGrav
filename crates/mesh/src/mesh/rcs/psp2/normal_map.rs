//! Omega's tangent-space normal maps, bound third like HD's hull normal map.
//!
//! A PS4 submesh whose declaration carries a `tangent` reads a normal map: which of the material's
//! samplers it is comes from the material's own pixel programs, the one decoded `2x - 1`
//! ([`oag_rcs::gcn::signed_samplers`], chosen by [`oag_rcs::rcsmodel::psp2::normal_map::choose`]).
//! Its picture rides the third binding ([`Model::pad_masks`]) and its vertices carry the
//! tangent, which switches `shade.wesl`'s `hull_normal` on (`hull_normal.wesl`).
//!
//! **The bitangent is `cross(N0, T) * w` with `w` exactly `+-1`**, which the vertex programs read
//! raw (`docs/formats/gcn.md`); packed as byte 0 or 255 it comes back through the shader's
//! `2w - 1`. A material whose third binding another family already holds (a twin-blend wipe, the
//! hologram) keeps it, and one with several candidate normal maps binds none: both counted.
//!
//! **PS4 only.** 2048's programs are GXP, whose operands nothing reads (`docs/formats/gxp.md`), so
//! a Vita build neither binds a map nor emits a tangent.

use std::sync::Arc;

use oag_core::math::{Mat4, Vec3};
use oag_rcs::gcn::signed_samplers;
use oag_rcs::rcsmodel::psp2::{self, normal_map::Choice};

use super::{Report, decode_material_texture, twin_blend};
use crate::mesh::ModelTexture;
use crate::mesh::rcs::{Textures, pack_tangent};

/// What [`plan`] counted, in submeshes that carry a tangent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    /// Submeshes of a PS4 model whose declaration carries a `tangent`.
    pub submeshes: usize,
    /// Of those, bound to the material's one normal map.
    pub bound: usize,
    /// Of those, on a material with several candidate normal maps (none bound).
    pub ambiguous: usize,
    /// Of those, on a material none of whose bound samplers any program decodes `2x - 1`.
    pub absent: usize,
    /// Of those, on a material whose file, or whose normal map, could not be read.
    pub unread: usize,
    /// Of those, on a material whose third binding another family holds.
    pub third_taken: usize,
    /// Signed samples, over the materials read, whose texture no usage record placed.
    pub unresolved_samples: usize,
    /// Each material that bound nothing, by its file, with why (`"ambiguous"`, `"none signed"`,
    /// `"unread"`).
    pub unbound: std::collections::BTreeMap<String, &'static str>,
}

impl Census {
    /// A line for [`Report::describe`], empty when no submesh carries a tangent.
    pub fn describe(&self) -> String {
        if self.submeshes == 0 {
            return String::new();
        }
        format!(
            "; {} of {} tangent-carrying submesh(es) draw with their normal map ({} on a material with several candidates, {} with none signed, {} unread, {} with the third binding taken, {} signed sample(s) unplaced)",
            self.bound,
            self.submeshes,
            self.ambiguous,
            self.absent,
            self.unread,
            self.third_taken,
            self.unresolved_samples
        )
    }
}

/// Each material's normal map, by material index.
#[derive(Default)]
pub(super) struct Plan {
    maps: Vec<Option<Arc<ModelTexture>>>,
}

impl Plan {
    /// Whether any material has a normal map.
    pub(super) fn any(&self) -> bool {
        self.maps.iter().any(Option::is_some)
    }

    /// The normal map of material `index`.
    pub(super) fn of(&self, index: Option<usize>) -> Option<&Arc<ModelTexture>> {
        self.maps.get(index?)?.as_ref()
    }

    /// `GpuVertex::tangent` for one vertex of a submesh on `material`: `0` (the shader's off
    /// switch) where that material has no normal map or the submesh no tangent.
    pub(super) fn tangent(
        &self,
        material: Option<usize>,
        authored: Option<&[f32; 4]>,
        to_world: &Mat4,
    ) -> u32 {
        let (Some(_), Some(&[x, y, z, w])) = (self.of(material), authored) else {
            return 0;
        };
        // A rigid node leaves the length alone; Omega's skeletons also scale non-uniformly.
        let t = to_world
            .transform_vector3(Vec3::new(x, y, z))
            .normalize_or_zero();
        pack_tangent(t, w.clamp(-1.0, 1.0))
    }
}

/// What one material's program and instance said, kept so a material shared by many submeshes
/// is read once.
#[derive(Clone)]
enum Outcome {
    Unread,
    Absent,
    Ambiguous,
    Bound(Arc<ModelTexture>),
}

/// Reads each tangent-carrying PS4 material's program and decodes its normal map.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    twin: &twin_blend::Plan,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut plan = Plan {
        maps: vec![None; decoded.materials.len()],
    };
    if !ps4 {
        return plan;
    }
    let mut outcomes: Vec<Option<Outcome>> = vec![None; decoded.materials.len()];
    let mut pictures: std::collections::HashMap<&str, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for submesh in decoded.submeshes.iter().filter(|s| !s.tangents.is_empty()) {
        report.normal_maps.submeshes += 1;
        let Some(index) = submesh.material.filter(|&m| m < decoded.materials.len()) else {
            report.normal_maps.unread += 1;
            continue;
        };
        if twin.of(Some(index)).is_some() {
            report.normal_maps.third_taken += 1;
            continue;
        }
        let material = &decoded.materials[index];
        if outcomes[index].is_none() {
            let outcome = match textures(&material.name) {
                None => Outcome::Unread,
                Some(blob) => {
                    let signed = signed_samplers(&blob);
                    report.normal_maps.unresolved_samples += signed.unresolved;
                    match psp2::normal_map::choose(material, &signed) {
                        Choice::Absent => Outcome::Absent,
                        Choice::Ambiguous(_) => Outcome::Ambiguous,
                        Choice::Bound(path) => pictures
                            .entry(path)
                            .or_insert_with(|| {
                                let blob = textures(path)?;
                                decode_material_texture(path, &blob, report).map(Arc::new)
                            })
                            .clone()
                            .map_or(Outcome::Unread, Outcome::Bound),
                    }
                }
            };
            if let Outcome::Bound(picture) = &outcome {
                plan.maps[index] = Some(picture.clone());
            }
            outcomes[index] = Some(outcome);
        }
        let census = &mut report.normal_maps;
        let why = match outcomes[index] {
            Some(Outcome::Bound(_)) => {
                census.bound += 1;
                continue;
            }
            Some(Outcome::Ambiguous) => {
                census.ambiguous += 1;
                "ambiguous"
            }
            Some(Outcome::Absent) => {
                census.absent += 1;
                "none signed"
            }
            _ => {
                census.unread += 1;
                "unread"
            }
        };
        census.unbound.insert(material.name.clone(), why);
    }
    plan
}
