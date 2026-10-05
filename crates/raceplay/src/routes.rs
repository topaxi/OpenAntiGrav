//! An opponent at a fork: one AI line per way round, and the coin that picks
//! between them.
//!
//! **The law is recovered**: every craft flips a fair coin at every fork,
//! independently, on entering the path before it, and holds the choice to the
//! merge (`Ai_ChooseBranch`, `0x08854920`; `oag_ai::branch`). Before this, every
//! craft followed the ring and so took the same side at every fork, which is
//! what the maintainer saw.
//!
//! **How a route is driven is ours.** A route line is the ring from the
//! route's merge round to its split, then the route's own samples: a full lap,
//! so a driver's windowed search, lookahead and curvature all work on it
//! unchanged, and leaving the route is the line wrapping back to its start.
//! The driver's index is into whichever line it is on, so everything that
//! pairs `driver.index` with a line or a sample goes through
//! [`Race::line_of`] and [`Race::ai_sample_for`]. The speed plan is built for
//! the ring alone, so a craft on a route drives the corner model.

use log::{info, warn};

use super::*;

/// One route's AI line and the bookkeeping that maps it onto the ring.
#[derive(Debug, Clone)]
pub(super) struct RouteLine {
    /// The line, a full lap: ring `merge..split`, then the route.
    pub(super) line: oag_ai::Line,
    /// Which spline sample each line index is, index-parallel to [`Self::line`].
    pub(super) order: Vec<u32>,
    /// How many of the line's samples are ring: line index `i < kept` is ring
    /// index `(merge + i) % ring_len`.
    pub(super) kept: usize,
    /// The ring index line index 0 stands on.
    pub(super) merge: usize,
    /// The ring path whose fork this route leaves from.
    pub(super) pre_fork: u16,
    /// The coins that pick it - `oag_race::course::Route::choices`.
    pub(super) choices: Vec<bool>,
}

/// One line per route of `course`, built the way the ring's own line is.
///
/// Empty when there is no course or no fork. A route whose samples cannot be
/// matched one for one onto the spline table is skipped with a warning rather
/// than driven on a line that is not where the track is.
pub(super) fn route_lines(
    spline: &Spline,
    course: Option<&Course>,
    ring_order: &[u32],
    collision: &CollisionWorld,
    reach: f32,
) -> Vec<RouteLine> {
    let Some(course) = course else {
        return Vec::new();
    };
    let n = ring_order.len();
    if course.routes().is_empty() {
        return Vec::new();
    }
    if n != course.len() {
        warn!(
            "ai routes: ring line has {n} samples and the course {}, so no route is driven",
            course.len()
        );
        return Vec::new();
    }
    let mut out = Vec::new();
    for (k, route) in course.routes().iter().enumerate() {
        let kept = (route.split + n - route.merge) % n;
        if kept == 0 {
            warn!(
                "ai route {k}: leaves and rejoins at ring index {}, not driven",
                route.split
            );
            continue;
        }
        let mut order: Vec<u32> = (0..kept)
            .map(|i| ring_order[(route.merge + i) % n])
            .collect();
        let before = order.len();
        for &path in &route.paths {
            order.extend(
                (0..spline.len() as u32).filter(|&i| spline.path_of(i as usize) == Some(path)),
            );
        }
        if order.len() - before != route.len() {
            warn!(
                "ai route {k}: {} spline samples for {} route samples, not driven",
                order.len() - before,
                route.len()
            );
            continue;
        }
        let line = racing_line(spline, &order, collision, reach);
        out.push(RouteLine {
            line,
            order,
            kept,
            merge: route.merge,
            pre_fork: route.pre_fork,
            choices: route.choices.clone(),
        });
    }
    info!("ai routes: {} way(s) round {} fork(s)", out.len(), {
        let mut forks: Vec<u16> = out.iter().map(|r| r.pre_fork).collect();
        forks.dedup();
        forks.len()
    });
    out
}

/// The line a driver on `route` follows: the ring for `0`, else that route's.
///
/// A free function over the two fields rather than a method, so a caller
/// holding `&mut` on a ship can still borrow it.
pub(super) fn line_for<'a>(
    routes: &'a [RouteLine],
    ring: &'a oag_ai::Line,
    route: u16,
) -> &'a oag_ai::Line {
    route
        .checked_sub(1)
        .and_then(|k| routes.get(usize::from(k)))
        .map_or(ring, |r| &r.line)
}

impl Race {
    /// The line slot `slot`'s driver is on - the ring, or a route.
    #[must_use]
    pub fn line_of(&self, slot: usize) -> &oag_ai::Line {
        line_for(
            &self.sim.routes,
            &self.sim.racing_line,
            self.sim.world.ships[slot].driver.branching.route,
        )
    }

    /// The spline sample index of line index `ai_index` on slot `slot`'s line.
    pub(super) fn sample_index_for(&self, slot: usize, ai_index: usize) -> Option<usize> {
        let route = self.sim.world.ships[slot].driver.branching.route;
        match route
            .checked_sub(1)
            .and_then(|k| self.sim.routes.get(usize::from(k)))
        {
            Some(r) if !r.order.is_empty() => Some(r.order[ai_index % r.order.len()] as usize),
            _ => self.sample_index_of(ai_index),
        }
    }

    /// The spline sample slot `slot`'s driver stands on at `ai_index` of its
    /// own line - [`Self::ai_sample`] for a craft that may be on a route.
    #[must_use]
    pub fn ai_sample_for(&self, slot: usize, ai_index: usize) -> Option<&Sample> {
        self.sample_index_for(slot, ai_index)
            .and_then(|index| self.sim.spline.sample(index))
    }

    /// How many ways round forks this race's drivers can take.
    #[must_use]
    pub fn route_count(&self) -> usize {
        self.sim.routes.len()
    }

    /// Moves slot `slot`'s driver between the ring and the routes, before it
    /// drives this tick: the coin on entering a pre-fork path, and back onto
    /// the ring once past the merge. `Ai_ChooseBranch`'s states 0-2.
    pub(super) fn steer_branching(&mut self, slot: usize) {
        if self.sim.routes.is_empty() {
            return;
        }
        let Some(course) = self.sim.course.as_ref() else {
            return;
        };
        let n = course.len();
        if n == 0 {
            return;
        }
        let routes = &self.sim.routes;
        let driver = &mut self.sim.world.ships[slot].driver;
        let mut branching = driver.branching;
        let index = driver.index as usize;
        match branching.route.checked_sub(1).map(usize::from) {
            None => {
                let Some(path) = course.path_of(index % n) else {
                    return;
                };
                if !routes.iter().any(|r| r.pre_fork == path) {
                    branching.decided_at = 0;
                } else if branching.decided_at != path + 1 {
                    // State 0: one coin sequence per visit to the pre-fork path.
                    branching.decided_at = path + 1;
                    let candidates = routes
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.pre_fork == path)
                        .map(|(k, r)| (k, r.choices.as_slice()));
                    let mut visits = branching.visits;
                    let chosen = oag_ai::branch::choose(driver.seed, &mut visits, candidates);
                    branching.visits = visits;
                    if let Some(k) = chosen {
                        let r = &routes[k];
                        let into = (index % n + n - r.merge) % n;
                        if into < r.kept {
                            branching.route = u16::try_from(k + 1).unwrap_or(0);
                            branching.entered = false;
                            driver.index = u32::try_from(into).unwrap_or(0);
                        }
                    }
                }
            }
            Some(k) => {
                let Some(r) = routes.get(k) else {
                    branching = branching.on_ring();
                    driver.branching = branching;
                    return;
                };
                if index >= r.kept {
                    // State 2: on the route's own samples.
                    branching.entered = true;
                } else if branching.entered {
                    // Back on the ring part of the line: past the merge.
                    driver.index = u32::try_from((r.merge + index) % n).unwrap_or(0);
                    branching = branching.on_ring();
                }
            }
        }
        driver.branching = branching;
        self.recommit(slot);
    }

    /// A craft knocked off its side of a fork onto another follows the side it
    /// is on.
    ///
    /// `Ai_ChooseBranch`'s state 2 re-commit (`0x08854ae4`-`0x08854e58`): only
    /// when the craft is **off the track and moving**, score where it is on its
    /// own line and on each sibling as `|across| * 10 + along` - `across` how far
    /// outside the track's width it is as a fraction of that width, `along` its
    /// distance along the tangent from the located point - and take the sibling
    /// that scores lower. Siblings are the ring and every route leaving the same
    /// fork. The original relocates on one sibling path; a nested fork has
    /// several, and all are tried here.
    fn recommit(&mut self, slot: usize) {
        let Some(n) = self.sim.course.as_ref().map(Course::len) else {
            return;
        };
        let ship = &self.sim.world.ships[slot];
        let position = ship.physics.body.position;
        let moving = ship.physics.body.linear_velocity.length() >= 0.1;
        let branching = ship.driver.branching;
        let index = ship.driver.index as usize;
        let routes = &self.sim.routes;
        // Which fork's stretch the craft is in, and where it is now.
        let (fork, current) = match branching.route.checked_sub(1).map(usize::from) {
            Some(k) => {
                let Some(r) = routes.get(k) else { return };
                if !branching.entered || index < r.kept {
                    return;
                }
                (r.pre_fork, self.ai_sample_for(slot, index))
            }
            None => {
                let ring = index % n;
                let Some(r) = routes.iter().find(|r| {
                    let split = (r.merge + r.kept) % n;
                    (ring + n - split) % n < (r.merge + n - split) % n
                }) else {
                    return;
                };
                (r.pre_fork, self.ai_sample_for(slot, index))
            }
        };
        let Some(current) = current else { return };
        let here = fit(current, position);
        if !moving || here == 0.0 {
            return;
        }
        // Every sibling's own stretch: the ring's bypassed span, or a route's
        // own samples, each as (line, first index, end index, sample order).
        let mut best: Option<(u16, usize, f32)> = None;
        let mut consider = |route: u16, order: &[u32], from: usize, to: usize| {
            for i in from..to {
                let Some(sample) = self.sim.spline.sample(order[i % order.len()] as usize) else {
                    continue;
                };
                let score = fit(sample, position);
                if score < here && best.is_none_or(|(_, _, b)| score < b) {
                    best = Some((route, i % order.len(), score));
                }
            }
        };
        for (k, r) in routes.iter().enumerate() {
            if r.pre_fork != fork {
                continue;
            }
            let route = u16::try_from(k + 1).unwrap_or(0);
            if route != branching.route {
                consider(route, &r.order, r.kept, r.order.len());
            }
        }
        if branching.route != 0
            && let Some(r) = routes.iter().find(|r| r.pre_fork == fork)
        {
            let split = (r.merge + r.kept) % n;
            let span = (r.merge + n - split) % n;
            consider(0, &self.sim.ai_order, split, split + span);
        }
        let Some((route, at, _)) = best else { return };
        let driver = &mut self.sim.world.ships[slot].driver;
        driver.index = u32::try_from(at).unwrap_or(0);
        driver.branching.route = route;
        driver.branching.entered = route != 0;
    }
}

/// `Ai_ChooseBranch`'s placement score: how far outside the track's width
/// `position` is, as a fraction of that width, times ten, plus how far along
/// the tangent it is from the sample. `0` on the track at the sample.
fn fit(sample: &Sample, position: Vec3) -> f32 {
    let offset = position - Vec3::from_array(sample.pos);
    let along = offset.dot(Vec3::from_array(sample.tangent)).abs();
    let width = sample.half_width_left + sample.half_width_right;
    let across = if width > 0.0 {
        (offset.dot(Vec3::from_array(sample.lateral)) + sample.half_width_left) / width
    } else {
        0.0
    };
    let outside = if across < 0.0 {
        -across
    } else if across > 1.0 {
        across - 1.0
    } else {
        0.0
    };
    // The original's "on the track" is outside == 0 and along < 5.
    if outside == 0.0 && along < 5.0 {
        0.0
    } else {
        outside * 10.0 + along
    }
}
