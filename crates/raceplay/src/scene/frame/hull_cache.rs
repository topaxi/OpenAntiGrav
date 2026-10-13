//! The `original` tier's projected hulls, gathered once per tick rather than once
//! per drawn frame.
//!
//! The craft move once per 60 Hz tick and the window draws between ticks
//! (`docs/rendering/between-ticks.md`), so a 240 Hz window redid the same
//! gather four times over, a ray per hull vertex onto the floor each time. The
//! gather is now taken at the tick's own poses and kept with the one before it,
//! and a draw between the two lerps the vertices by the draw alpha.
//!
//! - **At alpha `1.0` the picture is the one the per-frame gather drew, bit for
//!   bit**: the current tick's gather is returned untouched, and it is the same
//!   function of the same poses. Every capture, trace and `--screenshot` draws
//!   there.
//! - **Between ticks it is not identical.** The per-frame gather projected the
//!   hull from the blended pose and cast every ray from there; this lerps the
//!   projected vertices of the two ticks. For a rigid hull moving a fraction of a
//!   tick the two differ by the second-order term of the rotation and by where
//!   the floor under a vertex bends inside the tick. **Chosen, not measured**
//!   as acceptable; `docs/rendering/split-screen.md`, "Cost", records the size
//!   of the difference on a real lap.
//! - **Anything the blend cannot represent is gathered live**, at the blended
//!   pose, as before: a craft that gains or loses its shadow over the tick, a
//!   previous gather that is not this tick's predecessor, and a draw whose
//!   previous tick was never gathered (a frame rate under 60 Hz runs several
//!   ticks between draws, so there is none to blend from and the cache would
//!   only double the work).
//! - **A craft whose blend is skipped** (respawn, a change of whether it is
//!   racing, a move past `SNAP_DISTANCE`: [`Race::ship_swept_from`]) draws its
//!   current gather, which is where the per-frame gather put it too.

use std::ops::Range;

use oag_core::math::Mat4;
use oag_mesh::mesh::GpuVertex;

use crate::Race;
use crate::scene::FrameWork;

/// While a draw keeps finding no previous gather to blend from, seeding the cache
/// costs a second gather for nothing (a frame rate under 60 Hz never has one), so
/// it is tried on one such draw in this many.
const SEED_EVERY: u32 = 16;

/// The last two ticks' gathers, and how long the draws have been missing the
/// previous one.
#[derive(Default, Debug)]
pub(in crate::scene) struct HullCache {
    current: Option<Gather>,
    previous: Option<Gather>,
    misses: u32,
}

/// One tick's projected hulls.
#[derive(Debug)]
struct Gather {
    /// [`Race::pose_generations`] `.current` the gather was taken at.
    generation: u64,
    /// Every drawn slot's model matrix at that tick, so a pose changed outside a
    /// tick path (which would not move the generation) is noticed.
    models: Vec<Mat4>,
    /// Which slot's hull is in which range of `vertices`, in slot order.
    slots: Vec<(usize, Range<usize>)>,
    vertices: Vec<GpuVertex>,
}

impl HullCache {
    /// Forgets both gathers, so the next draw gathers afresh. For the render
    /// bench, which draws one tick over and over and would otherwise measure a
    /// cache hit on every frame.
    pub(in crate::scene) fn forget(&mut self) {
        *self = Self::default();
    }
}

impl super::super::Scene {
    /// Throws away the kept shadow gathers, so the next frame gathers afresh as a
    /// frame on a new tick does. Only the render bench calls it.
    pub fn forget_shadow_gathers(&self) {
        self.hull_cache.borrow_mut().forget();
    }

    /// The projected hulls the `original` tier draws this frame, at the race's
    /// draw alpha.
    pub(super) fn original_hulls(&self, race: &Race) -> Vec<GpuVertex> {
        let alpha = race.draw_alpha();
        let generations = race.pose_generations();
        let models: Vec<Mat4> = (0..usize::from(race.ship_count()))
            .map(|slot| race.ship_tick_model(slot))
            .collect();
        let mut cache = self.hull_cache.borrow_mut();
        let fresh = cache.current.as_ref().is_some_and(|gather| {
            gather.generation == generations.current && gather.models == models
        });
        if !fresh {
            let predecessor = cache
                .current
                .take()
                .filter(|gather| gather.generation == generations.previous);
            let seed = alpha >= 1.0
                || predecessor.is_some()
                || cache.misses < 2
                || cache.misses % SEED_EVERY == 0;
            if !seed {
                cache.misses += 1;
                return self.live_hulls(race);
            }
            cache.misses = if predecessor.is_some() {
                0
            } else {
                cache.misses + 1
            };
            cache.previous = predecessor;
            cache.current = Some(self.gather_tick(race, generations.current, models));
        }
        let HullCache {
            current: Some(current),
            previous,
            ..
        } = &*cache
        else {
            return Vec::new();
        };
        if alpha >= 1.0 {
            return current.vertices.clone();
        }
        if let Some(blended) = previous
            .as_ref()
            .and_then(|previous| blend(race, previous, current, alpha))
        {
            return blended;
        }
        drop(cache);
        self.live_hulls(race)
    }

    /// The hulls projected from the poses this draw blends to, as the per-frame
    /// gather did.
    fn live_hulls(&self, race: &Race) -> Vec<GpuVertex> {
        self.count_gather();
        if !self.shadow_hulls.iter().any(Option::is_some) {
            return Vec::new();
        }
        let placements = race.shadow_placements();
        self.project_hulls(race, &placements, |slot| race.ship_model_matrix_of(slot))
            .0
    }

    /// The hulls projected from this tick's poses exactly.
    fn gather_tick(&self, race: &Race, generation: u64, models: Vec<Mat4>) -> Gather {
        self.count_gather();
        let (vertices, slots) = if self.shadow_hulls.iter().any(Option::is_some) {
            let placements = race.shadow_placements_of(|slot| race.ship_tick_body(slot));
            self.project_hulls(race, &placements, |slot| race.ship_tick_model(slot))
        } else {
            (Vec::new(), Vec::new())
        };
        Gather {
            generation,
            models,
            slots,
            vertices,
        }
    }

    fn count_gather(&self) {
        self.frame_work.update(|w| FrameWork {
            shadow_gathers: w.shadow_gathers + 1,
            ..w
        });
    }
}

/// The two ticks' hulls blended by `alpha`, or `None` where they cannot be
/// (see the module doc).
fn blend(race: &Race, previous: &Gather, current: &Gather, alpha: f32) -> Option<Vec<GpuVertex>> {
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

/// A vertex between two of the same hull's projections: the position, the
/// plane's normal and the strength carried in the colour move, the rest is
/// constant across a hull.
fn lerp(a: &GpuVertex, b: &GpuVertex, alpha: f32) -> GpuVertex {
    let mix = |a: f32, b: f32| a + (b - a) * alpha;
    let normal = oag_core::math::Vec3::from_array(a.normal)
        .lerp(oag_core::math::Vec3::from_array(b.normal), alpha)
        .normalize_or_zero();
    GpuVertex {
        position: std::array::from_fn(|i| mix(a.position[i], b.position[i])),
        normal: normal.to_array(),
        colour: std::array::from_fn(|i| mix(a.colour[i], b.colour[i])),
        ..*b
    }
}

impl super::super::Scene {
    /// Projects each placed craft's authored hull onto the surface under it,
    /// with the craft at `model(slot)`, and returns the vertices with which slot
    /// owns which range.
    pub(super) fn project_hulls(
        &self,
        race: &Race,
        placements: &[oag_render::shadow::Placement],
        model: impl Fn(usize) -> Mat4,
    ) -> (Vec<GpuVertex>, Vec<(usize, Range<usize>)>) {
        let mut vertices = Vec::new();
        let mut slots = Vec::new();
        // A slot whose model authors no hull contributes nothing rather than
        // falling back to a blob: a tier that silently becomes another tier is
        // how a missing feature stops being noticed.
        for placement in placements {
            let Some(Some(hull)) = self.shadow_hulls.get(placement.silhouette) else {
                continue;
            };
            let first = vertices.len();
            oag_render::shadow::hull_triangles(
                &oag_render::shadow::Cast {
                    hull,
                    model: model(placement.silhouette),
                    axis: oag_core::math::Vec3::from_array(oag_pulse::shadow::AUTHORED_AXIS),
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
}

#[cfg(test)]
mod tests;
