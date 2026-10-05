//! The track as a closed loop with a distance along it.
//!
//! # Why this exists
//!
//! Nothing in the original tells us where a lap begins. `gate` (`.vex` class
//! `0x3ca`) has no runtime class registration, and no lap or split logic was
//! found anywhere in the executable - see
//! `docs/formats/track.md#where-is-lap-counting`. The track file does carry a
//! `Start Position`, but that is a **grid slot**, not a start line: it sits 3.3
//! and 20.5 units off the spline's own centreline, and a real time trial was
//! measured starting 139.7 units away from it.
//!
//! So a lap is defined here rather than recovered, and this module is where that
//! definition lives. See `docs/gameplay/lap-counting.md` for the confidence score
//! and for what would retire it.
//!
//! # What is recovered
//!
//! The *shape* of the loop is not invented. `AiTrack` stores paths joined by
//! 2-in/2-out junctions, and the original's own traversal - recovered at
//! confidence 88, `docs/formats/track.md:365-381` - follows `next_primary` and
//! only takes `next_alternate` when an `excluded_path` argument says to:
//!
//! ```text
//! Path *next = path->exit->next_primary;
//! if (path->exit->next_alternate && index(next) == excluded_path)
//!     next = path->exit->next_alternate;   // branch selection
//! ```
//!
//! [`Course`] therefore walks the **primary** chain. Alternate paths are kept
//! beside it twice over: as [`Branch`]es, the narrow view a Repulser's fork
//! wave walks, and as [`Route`]s, every way from a ring fork back to the ring,
//! which is what an opponent drives (`docs/gameplay/ai.md`, "Branch choice at a
//! fork").
//!
//! **An alternate is a real detour, not a second line over the same stretch.**
//! That used to be written here of `05_Track`, and measured 2026-10-05
//! (`cargo run -p oag-game --example fork_survey`) it is wrong: Pulse's three
//! alternates (05, 07, 14) stray 63 to 241 units from the ring, and 2048's up
//! to 300 (`subway`). A craft that far out is beyond [`Course::locate`]'s
//! reacquire distance, so `locate` searches the routes near its hint as well
//! and reads progress off the ring span a route stands in for - see
//! [`Course::locate_on_route`].

use oag_core::math::Vec3;
use oag_vex::track::AiTrack;

pub mod branch;
pub mod route;
pub use branch::Branch;
pub use route::Route;

/// A track walked into a closed ring, with cumulative distance along it.
///
/// A flat `Vec` scanned in order, not a spatial index: the nearest-point
/// comparison feeds simulation state, so the order it happens in must not vary
/// between runs, and ties go to the earlier point. Same rule, and the same
/// reason, as `Spline::nearest` in the composition root. See
/// `docs/architecture/determinism.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Course {
    /// Ring points, in travel order.
    positions: Vec<Vec3>,
    /// Which path each point came from, parallel to [`Self::positions`].
    paths: Vec<u16>,
    /// Distance from `positions[0]` to each point, parallel to it and
    /// non-decreasing. Element 0 is always `0.0`.
    distance: Vec<f32>,
    /// The full way round, including the closing step from the last point back
    /// to the first.
    length: f32,
    /// The point distance is measured from - the start line.
    start_index: usize,
    /// The widest half-width anywhere on the ring.
    max_half_width: f32,
    /// The midpoint of the track's two edges at each ring point, parallel to
    /// [`Self::positions`]: `pos - lateral * half_width_left` and
    /// `pos + lateral * half_width_right`, averaged. Where the original puts a
    /// Repulser wave (`Repulser_AdvanceWave`, `0x08876914`).
    centres: Vec<Vec3>,
    /// The AI corridor's width at each ring point, `ai_bound_right -
    /// ai_bound_left`, parallel to [`Self::positions`]. The lateral bound of a
    /// Repulser wave's sweep (`RepulserPool_SweepTargets`, `0x0886d5c8`).
    corridor_widths: Vec<f32>,
    /// The alternate paths off the ring - see [`branch`].
    branches: Vec<Branch>,
    /// Every way round every fork - see [`route`].
    routes: Vec<Route>,
}

/// Where a position sits on the course.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Located {
    /// Index of the nearest ring point.
    pub index: usize,
    /// Distance from the start line, in track units, in `0.0..length`.
    pub progress: f32,
    /// How far the position is from that ring point.
    ///
    /// A ship well inside this is on the track; a large value means the nearest
    /// point is not meaningful and the caller should not trust `progress`.
    pub offset: f32,
}

impl Course {
    /// Samples per control-point interval.
    ///
    /// The same four `Spline::from_track` and `oag_render::track` use, so the
    /// ring, the locator table and the drawn ribbon all agree about where the
    /// track is.
    pub const STEPS_PER_SEGMENT: usize = 4;

    /// Ring points searched either side of the hint before giving up on it.
    ///
    /// At four samples per control-point interval and roughly six units between
    /// control points, 64 points is around 96 units of track either side - far
    /// more than a ship covers in one tick at any speed class, and still a
    /// hundredth of the scan a full pass costs.
    pub const WINDOW: usize = 64;

    /// How many track half-widths past the nearest windowed point counts as
    /// "the window lost it", triggering a full scan.
    ///
    /// The scale comes from the track's own widest half-width rather than from a
    /// number somebody picked, so it travels between circuits.
    pub const REACQUIRE_HALF_WIDTHS: f32 = 4.0;

    /// How far ahead of the authored grid slot the start line is, in track
    /// units along the spline's tangent at the slot.
    ///
    /// **Recovered from the original, not measured.** `RaceManager_Construct`
    /// (`0x08829124`) locates the `Start Position` node on the spline, steps
    /// `154.0` units along that sample's tangent in a straight line, locates
    /// *that* point on the spline, and keeps its arc position as the line the
    /// lap counter compares against
    /// (`docs/ghidra/functions/psp-pulse-usa/race-progress.md`). The literal
    /// is `0x431a0000` in the binary and the same on every circuit, which is
    /// why the previous single-capture constant (137.9 units, fitted on
    /// Talon's Junction) happened to work on Moa Therma too: it was standing in
    /// for this.
    ///
    /// The two numbers are not the same quantity. 137.9 is where the original
    /// *spawns* the craft relative to the slot
    /// (`the_authored_slot_matches_the_captured_start`,
    /// `crates/game/tests/race_ground_truth.rs`); the line is a few units
    /// further on, so a craft starts behind it and its first crossing starts the
    /// race rather than counting a lap - the original's `started` flag, this
    /// crate's [`RaceState`]'s lap gate.
    ///
    /// # Which side of the engine/title seam this is on
    ///
    /// Pulse's code: a literal in `RaceManager_Construct`, like the force-law
    /// literals `oag_physics::params` files under the title. It stays here
    /// rather than in `oag-pulse` only because `Course` is the one place that
    /// can apply it; the number is the title's.
    pub const START_LINE_ADVANCE: f32 = 154.0;

    /// Walks a decoded spline graph into a closed ring.
    ///
    /// `start_near` is the track's authored `Start Position`, when it has one.
    /// The nearest ring point to it becomes distance zero. Pass `None` and the
    /// ring's own first point is used instead, which claims nothing beyond "the
    /// loop starts somewhere".
    ///
    /// Returns `None` when the primary chain does not close - an open track, or
    /// a graph this walk does not understand. `None` is deliberate rather than a
    /// best-effort ring: a lap counter running on a course that is not a loop
    /// would produce plausible, wrong numbers.
    #[must_use]
    pub fn from_track(ai: &AiTrack, start_near: Option<Vec3>) -> Option<Self> {
        let ring = primary_ring(ai)?;

        let mut positions = Vec::new();
        let mut paths = Vec::new();
        let mut centres = Vec::new();
        let mut corridor_widths = Vec::new();
        let mut max_half_width = 0.0f32;
        let mut first = vec![None; ai.paths.len()];
        for &path_index in &ring {
            let path = ai.paths.get(path_index)?;
            first[path_index] = Some(positions.len());
            for sample in branch::sample_path(path) {
                positions.push(sample.pos);
                centres.push(sample.centre);
                corridor_widths.push(sample.corridor_width);
                paths.push(u16::try_from(path_index).unwrap_or(u16::MAX));
                max_half_width = max_half_width
                    .max(sample.half_widths.0)
                    .max(sample.half_widths.1);
            }
        }
        let branches = branch::branches(ai, &first);
        let routes = route::routes(ai, &ring, &first, &positions);
        if positions.len() < 2 {
            return None;
        }

        let mut distance = Vec::with_capacity(positions.len());
        let mut running = 0.0f32;
        distance.push(running);
        for pair in positions.windows(2) {
            running += (pair[1] - pair[0]).length();
            distance.push(running);
        }
        // The closing step back to the first point is part of the way round, but
        // is not a table entry: there is no point to attach it to.
        let last = positions.len() - 1;
        let length = running + (positions[0] - positions[last]).length();
        // Not `!(length > 0.0)`: written out so the NaN case is deliberate rather
        // than a side effect of negating a partial order. A degenerate track whose
        // points all coincide, or one carrying a NaN coordinate, has no course.
        if !length.is_finite() || length <= 0.0 {
            return None;
        }

        let mut course = Self {
            positions,
            paths,
            distance,
            length,
            start_index: 0,
            max_half_width,
            centres,
            corridor_widths,
            branches,
            routes,
        };
        if let Some(start) = start_near
            && let Some((slot, _)) = course.nearest_global(start)
            && let Some(tangent) = course.tangent(slot)
        {
            // The original's own rule: the slot projected onto the spline,
            // advanced [`Self::START_LINE_ADVANCE`] along that point's tangent in
            // a straight line, projected onto the spline again.
            let ahead = course.positions[slot] + tangent * Self::START_LINE_ADVANCE;
            if let Some((line, _)) = course.nearest_global(ahead) {
                course.start_index = line;
            }
        }
        Some(course)
    }

    /// How many points the ring holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether the ring is empty. Never true for a `Course` that exists -
    /// [`Self::from_track`] refuses one - but clippy asks for it beside `len`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// The full way round, in track units.
    #[must_use]
    pub fn length(&self) -> f32 {
        self.length
    }

    /// The ring point distance is measured from.
    #[must_use]
    pub fn start_index(&self) -> usize {
        self.start_index
    }

    /// The widest half-width anywhere on the ring.
    #[must_use]
    pub fn max_half_width(&self) -> f32 {
        self.max_half_width
    }

    /// Which path the point at `index` came from.
    #[must_use]
    pub fn path_of(&self, index: usize) -> Option<u16> {
        self.paths.get(index).copied()
    }

    /// The ring point at `index`.
    #[must_use]
    pub fn position(&self, index: usize) -> Option<Vec3> {
        self.positions.get(index).copied()
    }

    /// The midpoint of the track's two edges at `index` - see [`Self::centres`].
    #[must_use]
    pub fn centre(&self, index: usize) -> Option<Vec3> {
        self.centres.get(index).copied()
    }

    /// The alternate paths off the ring, in junction order - see [`Branch`].
    #[must_use]
    pub fn branches(&self) -> &[Branch] {
        &self.branches
    }

    /// Every way round every fork, in ring order of the fork - see [`Route`].
    #[must_use]
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// The AI corridor's width at `index` - see [`Self::corridor_widths`].
    #[must_use]
    pub fn corridor_width(&self, index: usize) -> Option<f32> {
        self.corridor_widths.get(index).copied()
    }

    /// The ring's own direction of travel at `index`: the normalised step to
    /// the next point, wrapping at the end back to the first.
    ///
    /// `None` for an out-of-range index or for the degenerate case of two
    /// coincident points - a ring built by [`Self::from_track`] never has
    /// either, but a caller holding a stale index from before a track swap
    /// could.
    #[must_use]
    pub fn tangent(&self, index: usize) -> Option<Vec3> {
        let count = self.positions.len();
        if count == 0 || index >= count {
            return None;
        }
        let next = self.positions[(index + 1) % count];
        (next - self.positions[index]).try_normalize()
    }

    /// Distance from the start line to the ring point at `index`.
    #[must_use]
    pub fn progress_at(&self, index: usize) -> Option<f32> {
        let raw = self.distance.get(index)?;
        let start = self.distance.get(self.start_index)?;
        Some((raw - start).rem_euclid(self.length))
    }

    /// Locates `position` on the ring.
    ///
    /// `hint` is the index this caller got last tick. The search starts there and
    /// widens to the whole ring only when the windowed answer is implausibly far
    /// away, which is what keeps a track that passes over itself from reading as
    /// a lap: the bridge above is hundreds of points away in the table but a few
    /// units away in space, and a global scan would happily jump to it.
    #[must_use]
    ///
    /// **A craft on a route off the ring is located on that route.** Near a
    /// fork the routes whose stretch the hint is in are searched too, and when
    /// one is nearer than the ring its sample's progress is read off the ring
    /// span it stands in for - see [`Self::locate_on_route`]. Without it a
    /// craft on a detour that strays further than the reacquire distance loses
    /// its fix and a global scan can put it a lap away (`park` read 4,388
    /// units off; `branch_progress_ground_truth.rs`).
    pub fn locate(&self, position: Vec3, hint: Option<usize>) -> Option<Located> {
        let (index, offset) = match hint {
            Some(hint) if hint < self.positions.len() => {
                let (index, offset) = self.nearest_within(position, hint);
                if let Some(on_route) = self.locate_on_route(position, hint)
                    && on_route.offset < offset
                {
                    return Some(on_route);
                }
                if offset > self.max_half_width * Self::REACQUIRE_HALF_WIDTHS {
                    self.nearest_global(position)?
                } else {
                    (index, offset)
                }
            }
            _ => self.nearest_global(position)?,
        };
        Some(Located {
            index,
            progress: self.progress_at(index)?,
            offset,
        })
    }

    /// The nearest sample on any route whose stretch `around` is in, located
    /// as a point on the ring.
    ///
    /// A route stands in for the ring from its `split` to its `merge`, so its
    /// sample at fraction `f` of its own length reads the ring's progress at
    /// fraction `f` of that span, and its index is the ring point there.
    /// Continuous at both ends by construction. **Chosen, not measured**: the
    /// original reads the point's own authored progress (`+0x40`) instead, a
    /// field this ring does not use (see `docs/gameplay/lap-counting.md`), so
    /// within a route the two can differ by how unevenly the artist spread it.
    ///
    /// A stretch is the ring from [`Self::WINDOW`] before `split` to
    /// [`Self::WINDOW`] past `merge`, so a hint still on the ring as the craft
    /// leaves it, or already back as it rejoins, finds the route.
    #[must_use]
    pub fn locate_on_route(&self, position: Vec3, around: usize) -> Option<Located> {
        let count = self.positions.len();
        let mut best: Option<(usize, usize, f32)> = None;
        for (r, route) in self.routes.iter().enumerate() {
            let from = (route.split + count - Self::WINDOW.min(count)) % count;
            let span = (route.merge + count - from) % count + Self::WINDOW;
            if (around + count - from) % count > span {
                continue;
            }
            for (i, point) in route.positions.iter().enumerate() {
                let distance = (*point - position).length();
                if best.is_none_or(|(_, _, previous)| distance < previous) {
                    best = Some((r, i, distance));
                }
            }
        }
        let (r, i, offset) = best?;
        let route = &self.routes[r];
        let ring_span =
            (self.distance[route.merge] - self.distance[route.split]).rem_euclid(self.length);
        let along = route.fraction(i) * ring_span;
        let raw = self.distance[route.split] + along;
        let start = self.distance[self.start_index];
        Some(Located {
            index: self.advance(route.split, along),
            progress: (raw - start).rem_euclid(self.length),
            offset,
        })
    }

    /// Ring indices where one path hands over to the next.
    ///
    /// **Was a lead on whether the start line is authored data; it is not.** A
    /// boundary landing on the visible start line across circuits would have let
    /// the single-capture constant this crate used before
    /// [`Self::START_LINE_ADVANCE`] was recovered be replaced by something at the
    /// traversal's own confidence 88.
    /// Checked on all 40 Pulse circuit files (2026-08-30): the nearest boundary to
    /// the true start line ranges 11.6 to 1878.4 units, median 648, with only 4 of
    /// 40 under 100 units. A path split is authored for its own reasons and is not
    /// evidence of a start line; see `docs/gameplay/lap-counting.md` for the sweep.
    /// Still reported in the load report because it costs nothing and is
    /// occasionally useful for reading a track's own split structure.
    #[must_use]
    pub fn path_boundaries(&self) -> Vec<usize> {
        let count = self.paths.len();
        (0..count)
            .filter(|&i| self.paths[i] != self.paths[(i + count - 1) % count])
            .collect()
    }

    /// The paths a lap drives, in the order it drives them.
    ///
    /// **Not the same as the track's paths, and that is the whole point of
    /// exposing it.** A split authors both branches as paths and the primary
    /// chain walks one of them, so on `05_Track`, `14_Track` and `07_Track` the
    /// file holds three paths and a lap drives paths 0 and 2. Anything that
    /// wants "the track, once round" - the line an opponent follows, say - has
    /// to ask this rather than walk the file, or it splices a stretch the lap
    /// never drives into the middle of the lap. See `docs/gameplay/ai.md`.
    ///
    /// Deduplicated by neighbour rather than by set, because the ring is
    /// contiguous per path by construction and a set would need an iteration
    /// order that simulation state must not depend on.
    #[must_use]
    pub fn path_order(&self) -> Vec<u16> {
        let mut order: Vec<u16> = Vec::new();
        for &path in &self.paths {
            if order.last() != Some(&path) {
                order.push(path);
            }
        }
        order
    }

    /// The ring point `by` units further along than `from`.
    ///
    /// Walks point to point rather than doing arithmetic on the cumulative table,
    /// because the table does not include the closing step back to point zero and
    /// a walk crosses it without a special case. Wraps, and stops early if it ever
    /// gets all the way round - a `by` longer than the lap is a caller bug, not a
    /// reason to loop forever.
    #[must_use]
    pub fn advance(&self, from: usize, by: f32) -> usize {
        let count = self.positions.len();
        if count == 0 {
            return from;
        }
        let mut index = from % count;
        let mut walked = 0.0f32;
        for _ in 0..count {
            let next = (index + 1) % count;
            let step = (self.positions[next] - self.positions[index]).length();
            if walked + step > by {
                // Whichever end of this segment lands nearer the target.
                return if by - walked < walked + step - by {
                    index
                } else {
                    next
                };
            }
            walked += step;
            index = next;
        }
        index
    }

    /// The nearest ring point to `position`, scanning everything.
    fn nearest_global(&self, position: Vec3) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (index, point) in self.positions.iter().enumerate() {
            let distance = (*point - position).length();
            // Strictly nearer, so a tie keeps the earlier point.
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best
    }

    /// The nearest ring point within [`Self::WINDOW`] either side of `around`.
    ///
    /// The window **wraps**, because the ring does: a ship on the start line is a
    /// few points from both ends of the table.
    fn nearest_within(&self, position: Vec3, around: usize) -> (usize, f32) {
        let count = self.positions.len();
        let span = Self::WINDOW.min(count / 2);
        let mut best: Option<(usize, f32)> = None;
        for step in 0..=(span * 2) {
            // `+ count` keeps the subtraction on the non-negative side before the
            // modulo, which `usize` needs and which also makes the wrap explicit.
            let index = (around + count + step - span) % count;
            let distance = (self.positions[index] - position).length();
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.unwrap_or((around, f32::INFINITY))
    }
}

/// The paths of the primary chain, in travel order, or `None` if it never closes.
///
/// Every path is tried as a starting point, in file order, because path 0 is not
/// guaranteed to be on the primary chain at all: on `05_Track` paths 0 and 1 are
/// the two sides of a split and only one of them is primary. Trying them in file
/// order keeps the answer the same on every run, which a `HashSet` walk would
/// not.
///
/// **The longest closing chain wins, not the first one found.** A malformed
/// graph can contain a short cycle that closes perfectly well while covering
/// almost none of the track - a path whose exit junction points back at itself
/// is a one-path ring, and taking the first hit would accept it and race on a
/// tenth of the circuit. Ties go to the lowest starting path, so the answer does
/// not depend on iteration luck.
fn primary_ring(ai: &AiTrack) -> Option<Vec<usize>> {
    // A single path with no junctions is the whole track and closes on itself.
    // Not a shipped case on Pulse, but a driveable-ribbon build is exactly this
    // and there is no reason for it to have no lap counter.
    if ai.paths.len() == 1 && ai.junctions.is_empty() {
        return Some(vec![0]);
    }

    let mut best: Option<Vec<usize>> = None;
    for start in 0..ai.paths.len() {
        let Some(ring) = walk_from(ai, start) else {
            continue;
        };
        if best.as_ref().is_none_or(|found| ring.len() > found.len()) {
            best = Some(ring);
        }
    }
    best
}

/// Follows `next_primary` from `start` until it returns there.
fn walk_from(ai: &AiTrack, start: usize) -> Option<Vec<usize>> {
    let mut seen = vec![false; ai.paths.len()];
    let mut ring = Vec::new();
    let mut current = start;
    loop {
        if seen.get(current).copied()? {
            // Back somewhere we have already been, and it is not the start: this
            // chain runs into a loop that does not include where we set off.
            return None;
        }
        seen[current] = true;
        ring.push(current);

        let exit = ai.paths.get(current)?.exit?;
        let next = ai.junctions.get(exit)?.next[0]?;
        if next == start {
            return Some(ring);
        }
        current = next;
    }
}

#[cfg(test)]
mod tests;
