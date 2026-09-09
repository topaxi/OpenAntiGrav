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
//! [`Course`] therefore walks the **primary** chain and nothing else. Alternate
//! paths are shortcuts and are deliberately off the ring: on `05_Track`, the one
//! shipped track with a genuine split, the two branches "share both endpoints, so
//! they are two lines over the same stretch rather than a geographic detour", so
//! a ship on the shortcut still projects onto the ring at a sensible distance.

use oag_core::math::Vec3;
use oag_vex::track::AiTrack;

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

    /// How far along the track the start line sits from the authored grid slot.
    ///
    /// **The grid slot is not the start line, and it is not close to it.** The
    /// slot is *upstream*: a captured time trial's craft begins **137.9 units
    /// along the slot's own forward**, plus 22.4 across the track and 0.8 up. Those
    /// three numbers are asserted in `the_authored_slot_matches_the_captured_start`
    /// (`crates/game/tests/race_ground_truth.rs`), which also pins the slot's
    /// heading to within 1.2 degrees of the craft's - so this is a real separation
    /// along the track, not a misread matrix.
    ///
    /// Putting the line at the slot instead makes a lap tick over partway down the
    /// starting straight rather than at the end of it, which is exactly how it was
    /// first noticed - on Moa Therma, by driving one.
    ///
    /// Measured **along the ring**, not in a straight line, so it follows a
    /// curving start straight.
    ///
    /// Confidence **65**, and the two halves of that are worth separating.
    ///
    /// - **The distance is measured**, on one capture of one circuit: Talon's
    ///   Junction.
    /// - **That it generalises is observed, not measured.** Moa Therma - a
    ///   different circuit, no capture - counts its laps in the right place with
    ///   this constant, checked by driving it and watching the counter. That rules
    ///   out the offset being a property of Talon's Junction alone, which was the
    ///   live worry, but "the counter ticks where it looks like it should" is a
    ///   player's eye rather than an instrument, and it would not catch a
    ///   per-track offset that happens to be close on both.
    ///
    /// So: no longer suspected of being circuit-specific, still a single
    /// hard number standing in for whatever the original computes. Whatever lays
    /// the grid out is unread code (`docs/formats/track.md`, "How a ship gets its
    /// grid slot").
    ///
    /// What would retire it: that code, or a capture on a second circuit.
    /// [`Self::path_boundaries`] landing on the line across circuits was a third
    /// candidate and is dead - see its own doc comment.
    ///
    /// # Which side of the engine/title seam this is on
    ///
    /// **Neither, yet, and that is the finding rather than an omission.** The
    /// three constants stage 6 of the engine/title split
    /// ([ADR-0022](../../../docs/architecture/adr/0022-title-packages.md)) looked
    /// at each land somewhere definite - the per-team parameters on the disc, the
    /// force-law literals in Pulse's code, Zone's scoring in Pulse's code - and
    /// this one lands nowhere, because **it stands in for a computation nobody
    /// has read**. If the grid layout turns out to be authored, this is disc data
    /// and belongs to no crate at all; if it is code, it is Pulse's and belongs
    /// beside the other recovered literals. Confidence 65 is exactly the reason
    /// it cannot be filed: a number at 65 placed in a title package would be
    /// asserting it is title-specific, which is one of the two answers still open.
    ///
    /// So it stays here, and the seam paragraph in `oag_physics::params` does not
    /// claim it. A constant whose provenance is unknown is worth less filed
    /// wrongly than left where the note explaining it is.
    pub const START_LINE_OFFSET: f32 = 137.9;

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
        let mut max_half_width = 0.0f32;
        for &path_index in &ring {
            let path = ai.paths.get(path_index)?;
            for segment in 0..path.points.len() {
                for step in 0..Self::STEPS_PER_SEGMENT {
                    let t = step as f32 / Self::STEPS_PER_SEGMENT as f32;
                    if let Some(sample) = path.sample(segment, t) {
                        positions.push(Vec3::from_array(sample.pos));
                        paths.push(u16::try_from(path_index).unwrap_or(u16::MAX));
                        max_half_width = max_half_width
                            .max(sample.half_width_left)
                            .max(sample.half_width_right);
                    }
                }
            }
        }
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
        };
        if let Some(start) = start_near
            && let Some((slot, _)) = course.nearest_global(start)
        {
            // The slot is upstream of the line by [`Self::START_LINE_OFFSET`],
            // so walk that far along the ring rather than measuring from the slot.
            course.start_index = course.advance(slot, Self::START_LINE_OFFSET);
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
    pub fn locate(&self, position: Vec3, hint: Option<usize>) -> Option<Located> {
        let (index, offset) = match hint {
            Some(hint) if hint < self.positions.len() => {
                let (index, offset) = self.nearest_within(position, hint);
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

    /// Ring indices where one path hands over to the next.
    ///
    /// **Was a lead on whether the start line is authored data; it is not.** A
    /// boundary landing on the visible start line across circuits would have let
    /// [`Self::START_LINE_OFFSET`] - a single-capture constant applied to all 40
    /// circuits - be replaced by something at the traversal's own confidence 88.
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
