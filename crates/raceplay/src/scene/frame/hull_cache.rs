//! The `original` tier's projected hulls, gathered once per tick rather than once
//! per drawn frame.
//!
//! The craft move once per 60 Hz tick and the window draws between ticks
//! (`docs/rendering/between-ticks.md`), so a 240 Hz window redid the same
//! gather four times over, a ray per hull vertex onto the floor each time. The
//! gather ([`HullGather`]) is now taken at the tick's own poses and kept with the
//! one before it, and a draw between the two lerps the vertices by the draw alpha.
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

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::GpuVertex;

use crate::Race;
use crate::hull_gather::HullGather;
use crate::scene::FrameWork;

/// While a draw keeps finding no previous gather to blend from, seeding the cache
/// costs a second gather for nothing (a frame rate under 60 Hz never has one), so
/// it is tried on one such draw in this many.
const SEED_EVERY: u32 = 16;

/// The last two ticks' gathers, and how long the draws have been missing the
/// previous one.
#[derive(Default, Debug)]
pub(in crate::scene) struct HullCache {
    current: Option<HullGather>,
    previous: Option<HullGather>,
    misses: u32,
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
    /// draw alpha, cast along `axis` (the title's own shadow direction).
    pub(super) fn original_hulls(&self, race: &Race, axis: Vec3) -> Vec<GpuVertex> {
        let alpha = race.draw_alpha();
        let generation = race.pose_generations();
        let models: Vec<Mat4> = (0..usize::from(race.ship_count()))
            .map(|slot| race.ship_tick_model(slot))
            .collect();
        let mut cache = self.hull_cache.borrow_mut();
        let fresh = cache.current.as_ref().is_some_and(|gather| {
            gather.generation == generation.current && gather.models == models
        });
        if !fresh {
            let predecessor = cache
                .current
                .take()
                .filter(|gather| gather.generation == generation.previous);
            let seed = alpha >= 1.0
                || predecessor.is_some()
                || cache.misses < 2
                || cache.misses.is_multiple_of(SEED_EVERY);
            if !seed {
                cache.misses += 1;
                return self.live_hulls(race, axis);
            }
            cache.misses = if predecessor.is_some() {
                0
            } else {
                cache.misses + 1
            };
            cache.previous = predecessor;
            self.count_gather();
            cache.current = Some(HullGather::at_tick(race, &self.shadow_hulls, axis));
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
            .and_then(|previous| HullGather::blend(race, previous, current, alpha))
        {
            return blended;
        }
        drop(cache);
        self.live_hulls(race, axis)
    }

    /// The hulls projected from the poses this draw blends to, as the per-frame
    /// gather did.
    fn live_hulls(&self, race: &Race, axis: Vec3) -> Vec<GpuVertex> {
        self.count_gather();
        HullGather::at_draw(race, &self.shadow_hulls, axis)
    }

    fn count_gather(&self) {
        self.frame_work.update(|w| FrameWork {
            shadow_gathers: w.shadow_gathers + 1,
            ..w
        });
    }
}
