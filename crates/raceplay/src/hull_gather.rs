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
#[derive(Debug)]
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

impl HullGather {
    /// The hulls projected from this tick's poses exactly: what a draw at alpha
    /// `1.0` shows.
    #[must_use]
    pub fn at_tick(race: &Race, hulls: &[Option<Occluder>], axis: Vec3) -> Self {
        let (vertices, slots) = if hulls.iter().any(Option::is_some) {
            let placements = race.shadow_placements_of(|slot| race.ship_tick_body(slot));
            project(race, hulls, axis, &placements, |slot| {
                race.ship_tick_model(slot)
            })
        } else {
            (Vec::new(), Vec::new())
        };
        Self {
            generation: race.pose_generations().current,
            models: (0..usize::from(race.ship_count()))
                .map(|slot| race.ship_tick_model(slot))
                .collect(),
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
        project(race, hulls, axis, &placements, |slot| {
            race.ship_model_matrix_of(slot)
        })
        .0
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
        for (slot, range) in &current.slots {
            let now = &current.vertices[range.clone()];
            let Some(from) = race.ship_swept_from(*slot) else {
                out.extend_from_slice(now);
                continue;
            };
            if previous.models.get(*slot) != Some(&from) {
                return None;
            }
            let (_, before) = previous.slots.iter().find(|(s, _)| s == slot)?;
            let before = &previous.vertices[before.clone()];
            if before.len() != now.len() {
                return None;
            }
            out.extend(before.iter().zip(now).map(|(a, b)| lerp(a, b, alpha)));
        }
        Some(out)
    }
}

/// A vertex between two of the same hull's projections: the position, the
/// plane's normal and the strength carried in the colour move, the rest is
/// constant across a hull.
fn lerp(a: &GpuVertex, b: &GpuVertex, alpha: f32) -> GpuVertex {
    let mix = |a: f32, b: f32| a + (b - a) * alpha;
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

/// Projects each placed craft's authored hull onto the surface under it, with the
/// craft at `model(slot)`, and returns the vertices with which slot owns which
/// range.
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
) -> (Vec<GpuVertex>, Vec<(usize, Range<usize>)>) {
    let mut vertices = Vec::new();
    let mut slots = Vec::new();
    for placement in placements {
        let Some(Some(hull)) = hulls.get(placement.silhouette) else {
            continue;
        };
        let first = vertices.len();
        oag_render::shadow::hull_triangles(
            &oag_render::shadow::Cast {
                hull,
                model: model(placement.silhouette),
                axis,
                contact: placement.contact,
                normal: placement.normal,
                strength: placement.strength,
            },
            &mut vertices,
        );
        oag_render::shadow::conform_to_floor(&mut vertices, first, |at, normal| {
            race.floor_above(at, normal)
        });
        slots.push((placement.silhouette, first..vertices.len()));
    }
    (vertices, slots)
}

#[cfg(test)]
mod tests;
