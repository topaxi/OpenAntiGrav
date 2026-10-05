//! Every way round a fork: the paths a craft can take from the ring, through
//! an alternate, back to the ring.
//!
//! [`super::branch::Branch`] is the narrow case the Repulser's fork wave needs,
//! one alternate path whose own exit lands straight back on the ring. A
//! [`Route`] is the general one an opponent drives:
//!
//! - **a chain**, because an alternate can be several paths long before it
//!   rejoins (Wipeout 2048's `mall` junction 0 runs paths 7 then 17);
//! - **a tree**, because an alternate can fork again before it rejoins
//!   (`square` junction 1's alternate, path 6, ends at junction 5, which forks
//!   to 2 and 8, and 8 forks again at junction 6). Each leaf is its own route.
//!
//! The original walks this the same way: `Ai_ChooseBranch` (`0x08854920`) rolls
//! a coin for each path it enters whose exit junction forks, so a route two
//! forks deep is taken with probability one quarter. [`Route::choices`] is that
//! sequence of coins, which is what lets a driver pick a route by rolling them
//! in order. See `docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`.

use oag_core::math::Vec3;
use oag_vex::track::AiTrack;

/// One way from a ring fork back to the ring.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// The ring path whose exit junction is the fork: the path a craft is on
    /// when the original decides (`Ai_ChooseBranch`'s state 0).
    pub pre_fork: u16,
    /// The ring index of the first sample after the fork on the primary side.
    pub split: usize,
    /// The ring index of the first sample of the ring path the route rejoins.
    pub merge: usize,
    /// The paths driven, in order, from the fork's alternate to the last path
    /// before the rejoin.
    pub paths: Vec<u16>,
    /// The coin at each fork along the way, `true` for the alternate. The first
    /// is the ring fork itself and is always `true`; a route with one entry
    /// forks nowhere else.
    pub choices: Vec<bool>,
    /// Sample positions, [`super::Course::STEPS_PER_SEGMENT`] to a control-point
    /// interval like the ring, in travel order.
    pub positions: Vec<Vec3>,
    /// Distance from `positions[0]` to each sample, parallel to it.
    pub distance: Vec<f32>,
    /// From the first sample to the ring's `merge` sample, the last step
    /// included.
    pub length: f32,
}

impl Route {
    /// How many samples the route holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether the route has no samples (never, for one
    /// [`super::Course`] builds).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// How far along the route `index` is, as a fraction of [`Self::length`].
    #[must_use]
    pub fn fraction(&self, index: usize) -> f32 {
        let at = self.distance.get(index).copied().unwrap_or(0.0);
        if self.length > 0.0 {
            (at / self.length).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Every route off the ring, in ring order of the fork and then depth-first,
/// primary before alternate at each nested fork - an order that depends on the
/// file alone, so it is the same on every run.
///
/// `ring` is the ring's path order, `first` each path's first ring index
/// (`None` off the ring), `positions` the ring's own samples (for the last
/// step onto the merge).
pub(super) fn routes(
    ai: &AiTrack,
    ring: &[usize],
    first: &[Option<usize>],
    positions: &[Vec3],
) -> Vec<Route> {
    let on_ring = |path: usize| first.get(path).copied().flatten();
    let mut out = Vec::new();
    for &pre in ring {
        let Some(junction) = ai.paths[pre].exit.and_then(|exit| ai.junctions.get(exit)) else {
            continue;
        };
        let (Some(primary), Some(alternate)) = (junction.next[0], junction.next[1]) else {
            continue;
        };
        let Some(split) = on_ring(primary) else {
            continue;
        };
        if on_ring(alternate).is_some() {
            continue;
        }
        let mut found = Vec::new();
        walk(
            ai,
            &on_ring,
            alternate,
            &mut Vec::new(),
            &mut vec![true],
            &mut found,
        );
        for (paths, choices, merge) in found {
            let mut route_positions = Vec::new();
            for &path in &paths {
                route_positions.extend(
                    super::branch::sample_path(&ai.paths[path])
                        .iter()
                        .map(|s| s.pos),
                );
            }
            if route_positions.is_empty() {
                continue;
            }
            let mut distance = Vec::with_capacity(route_positions.len());
            let mut running = 0.0f32;
            distance.push(running);
            for pair in route_positions.windows(2) {
                running += (pair[1] - pair[0]).length();
                distance.push(running);
            }
            let last = route_positions[route_positions.len() - 1];
            let length = running + positions.get(merge).map_or(0.0, |p| (*p - last).length());
            out.push(Route {
                pre_fork: u16::try_from(pre).unwrap_or(u16::MAX),
                split,
                merge,
                paths: paths
                    .iter()
                    .map(|&p| u16::try_from(p).unwrap_or(u16::MAX))
                    .collect(),
                choices,
                positions: route_positions,
                distance,
                length,
            });
        }
    }
    out
}

/// Depth-first from `path`, collecting `(paths, choices, merge)` for every way
/// back onto the ring. A path already in the chain is not entered again, so a
/// malformed graph with a loop off the ring ends rather than recursing forever.
fn walk(
    ai: &AiTrack,
    on_ring: &impl Fn(usize) -> Option<usize>,
    path: usize,
    chain: &mut Vec<usize>,
    choices: &mut Vec<bool>,
    out: &mut Vec<(Vec<usize>, Vec<bool>, usize)>,
) {
    if chain.contains(&path) || path >= ai.paths.len() {
        return;
    }
    chain.push(path);
    if let Some(junction) = ai.paths[path].exit.and_then(|exit| ai.junctions.get(exit)) {
        let forks = junction.next[1].is_some();
        for (k, next) in junction.next.iter().enumerate() {
            let Some(next) = *next else {
                continue;
            };
            if forks {
                choices.push(k == 1);
            }
            if let Some(merge) = on_ring(next) {
                out.push((chain.clone(), choices.clone(), merge));
            } else {
                walk(ai, on_ring, next, chain, choices, out);
            }
            if forks {
                choices.pop();
            }
        }
    }
    chain.pop();
}
