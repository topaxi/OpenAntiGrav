//! The `original` tier's projected hulls at a tick's own poses, and the blend of
//! two ticks' worth.
//!
//! The pose-and-projection half of `scene/frame/hull_cache.rs`, apart from the
//! `Scene` so a test can drive it on a real race without a GPU. See there for why
//! the gather is taken per tick and what a draw between ticks does with it.

use std::ops::Range;

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::GpuVertex;
use oag_vex::shadow_occluder::Occluder;

use crate::Race;

/// One tick's projected hulls.
#[derive(Debug, Default)]
pub struct HullGather {
    /// [`Race::pose_generations`] `.current` the gather was taken at.
    pub generation: u64,
    /// Every drawn slot's model matrix at that tick, so a pose changed outside a
    /// tick path (which would not move the generation) is noticed.
    pub models: Vec<Mat4>,
    /// Which slot's hull is in which range of `vertices`, in slot order.
    pub slots: Vec<(usize, Range<usize>)>,
    pub vertices: Vec<GpuVertex>,
}

/// What [`HullGather::at_tick_with`] works in, kept from tick to tick so a gather
/// allocates nothing once warm: the projection's lists, and the lists of a gather
/// the cache has let go of, refilled as the next one.
#[derive(Debug, Default)]
pub struct GatherScratch {
    hull: oag_render::shadow::HullScratch,
    spare: Option<HullGather>,
}

impl GatherScratch {
    /// Keeps `gather`'s lists for the next [`HullGather::at_tick_with`] to fill.
    pub fn recycle(&mut self, gather: HullGather) {
        self.spare = Some(gather);
    }
}

impl HullGather {
    /// The hulls projected from this tick's poses exactly: what a draw at alpha
    /// `1.0` shows.
    #[must_use]
    pub fn at_tick(race: &Race, hulls: &[Option<Occluder>], axis: Vec3) -> Self {
        Self::at_tick_with(race, hulls, axis, &mut GatherScratch::default())
    }

    /// [`Self::at_tick`], working in `scratch` and filling its spare gather's lists.
    #[must_use]
    pub fn at_tick_with(
        race: &Race,
        hulls: &[Option<Occluder>],
        axis: Vec3,
        scratch: &mut GatherScratch,
    ) -> Self {
        let Self {
            mut models,
            mut slots,
            mut vertices,
            ..
        } = scratch.spare.take().unwrap_or_default();
        models.clear();
        slots.clear();
        vertices.clear();
        if hulls.iter().any(Option::is_some) {
            let placements = race.shadow_placements_of(|slot| race.ship_tick_body(slot));
            project(
                race,
                hulls,
                axis,
                &placements,
                |slot| race.ship_tick_model(slot),
                Gathered {
                    vertices: &mut vertices,
                    slots: &mut slots,
                    scratch: &mut scratch.hull,
                },
            );
        }
        models.extend((0..usize::from(race.ship_count())).map(|slot| race.ship_tick_model(slot)));
        Self {
            generation: race.pose_generations().current,
            models,
            slots,
            vertices,
        }
    }

    /// The hulls projected from the poses the race's draw alpha blends to: what
    /// the per-frame gather drew before the cache.
    #[must_use]
    pub fn at_draw(race: &Race, hulls: &[Option<Occluder>], axis: Vec3) -> Vec<GpuVertex> {
        if !hulls.iter().any(Option::is_some) {
            return Vec::new();
        }
        let placements = race.shadow_placements();
        let mut vertices = Vec::new();
        project(
            race,
            hulls,
            axis,
            &placements,
            |slot| race.ship_model_matrix_of(slot),
            Gathered {
                vertices: &mut vertices,
                slots: &mut Vec::new(),
                scratch: &mut oag_render::shadow::HullScratch::default(),
            },
        );
        vertices
    }

    /// The two ticks' hulls blended by `alpha`, or `None` where they cannot be
    /// (see `scene/frame/hull_cache.rs`).
    #[must_use]
    pub fn blend(
        race: &Race,
        previous: &Self,
        current: &Self,
        alpha: f32,
    ) -> Option<Vec<GpuVertex>> {
        let mut out = Vec::with_capacity(current.vertices.len());
        Self::blend_into(race, previous, current, alpha, &mut out).then_some(out)
    }

    /// [`Self::blend`] into `out`, emptied first, so a list kept from frame to frame is
    /// refilled rather than a new one made. Returns `false` where the two cannot be blended,
    /// with `out` holding no meaningful vertices.
    pub fn blend_into(
        race: &Race,
        previous: &Self,
        current: &Self,
        alpha: f32,
        out: &mut Vec<GpuVertex>,
    ) -> bool {
        out.clear();
        for (slot, range) in &current.slots {
            let now = &current.vertices[range.clone()];
            let Some(from) = race.ship_swept_from(*slot) else {
                out.extend_from_slice(now);
                continue;
            };
            if previous.models.get(*slot) != Some(&from) {
                return false;
            }
            let Some((_, before)) = previous.slots.iter().find(|(s, _)| s == slot) else {
                return false;
            };
            let before = &previous.vertices[before.clone()];
            if before.len() != now.len() {
                return false;
            }
            out.extend(before.iter().zip(now).map(|(a, b)| lerp(a, b, alpha)));
        }
        true
    }
}

/// A vertex between two of the same hull's projections: the position, the
/// plane's normal and the strength carried in the colour move, the rest is
/// constant across a hull.
fn lerp(a: &GpuVertex, b: &GpuVertex, alpha: f32) -> GpuVertex {
    let mix = |a: f32, b: f32| oag_core::math::lerp(a, b, alpha);
    let normal = Vec3::from_array(a.normal)
        .lerp(Vec3::from_array(b.normal), alpha)
        .normalize_or_zero();
    GpuVertex {
        position: std::array::from_fn(|i| mix(a.position[i], b.position[i])),
        normal: normal.to_array(),
        colour: std::array::from_fn(|i| mix(a.colour[i], b.colour[i])),
        ..*b
    }
}

/// Where [`project`] appends: the vertices, which slot owns which range of them,
/// and the lists it works in.
struct Gathered<'a> {
    vertices: &'a mut Vec<GpuVertex>,
    slots: &'a mut Vec<(usize, Range<usize>)>,
    scratch: &'a mut oag_render::shadow::HullScratch,
}

/// Projects each placed craft's authored hull onto the surface under it, with the
/// craft at `model(slot)`, appending the vertices and which slot owns which range
/// to `into`.
///
/// A slot whose model authors no hull contributes nothing rather than falling
/// back to a blob: a tier that silently becomes another tier is how a missing
/// feature stops being noticed.
fn project(
    race: &Race,
    hulls: &[Option<Occluder>],
    axis: Vec3,
    placements: &[oag_render::shadow::Placement],
    model: impl Fn(usize) -> Mat4,
    into: Gathered<'_>,
) {
    let Gathered {
        vertices,
        slots,
        scratch,
    } = into;
    for placement in placements {
        let Some(Some(hull)) = hulls.get(placement.silhouette) else {
            continue;
        };
        let first = vertices.len();
        oag_render::shadow::hull_triangles_with(
            &oag_render::shadow::Cast {
                hull,
                model: model(placement.silhouette),
                axis,
                contact: placement.contact,
                normal: placement.normal,
                strength: placement.strength,
            },
            vertices,
            scratch,
        );
        oag_render::shadow::conform_to_floor_with(
            vertices,
            first,
            |at, normal| race.floor_above(at, normal),
            scratch,
        );
        slots.push((placement.silhouette, first..vertices.len()));
    }
}

#[cfg(test)]
mod tests;
