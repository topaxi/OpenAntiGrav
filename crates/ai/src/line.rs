//! The line an opponent drives, as plain world-space points, and the corridor
//! it is allowed to drift inside.

use oag_core::math::Vec3;

/// How much room a craft has either side of the line, and which way "aside" is.
///
/// **Both bounds are relative to the line itself**, so `left` is at most zero
/// and `right` at least zero. The disc stores them the other way - absolute
/// offsets in the sample's own lateral axis, alongside the racing line's own
/// offset in the same axis - and the caller subtracts, because a driver here
/// only ever asks "how far may I go from the line I am driving". See
/// `docs/formats/track.md`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    /// Unit vector across the line, pointing to the driver's **right**.
    pub lateral: Vec3,
    /// How far left of the line the corridor reaches. Zero or negative.
    pub left: f32,
    /// How far right of the line the corridor reaches. Zero or positive.
    pub right: f32,
}

impl Frame {
    /// How much room there is on the side `offset` points to, as a positive
    /// distance. Zero when there is no corridor that way.
    #[must_use]
    pub fn room(&self, offset: f32) -> f32 {
        if offset >= 0.0 {
            self.right
        } else {
            -self.left
        }
        .max(0.0)
    }

    /// `offset` cut down to what the corridor allows.
    #[must_use]
    pub fn clamp(&self, offset: f32) -> f32 {
        offset.clamp(self.left.min(0.0), self.right.max(0.0))
    }
}

/// A point on the line to aim at, and the room around it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aim {
    /// Index of the sample the point sits on or just after.
    pub index: usize,
    /// The point itself, on the line.
    pub point: Vec3,
    /// How far along the line it actually is - the *travelled* distance, which
    /// is not the distance that was asked for once the line curves.
    pub travelled: f32,
    /// The corridor there, when the line carries one.
    pub corridor: Option<Frame>,
}

/// A closed run of world-space points, in driving order.
///
/// **The caller builds it.** On a real track that means the authored racing
/// line, each spline sample's `pos - HOVER_LIFT * down + racing_line * lateral`,
/// but nothing here knows that, which is what keeps this crate free of the asset
/// types and testable against a circle. See `docs/gameplay/ai.md`.
///
/// Closed rather than open: a race goes round, so index arithmetic wraps and
/// there is no end to fall off.
///
/// A line may also carry the **corridor** around it, one [`Frame`] per point.
/// That is what a driver spends on not driving the ideal line - see
/// [`crate::Personality`] - and a line built without one simply has no room to
/// spend, so every craft on it drives the same line. `Line::new` is that case,
/// and it is the one the synthetic tests use.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Line {
    points: Vec<Vec3>,
    /// Parallel to [`Self::points`], or empty. Never any other length -
    /// [`Line::with_corridor`] drops a mismatched one rather than half-using it.
    corridor: Vec<Frame>,
    /// Parallel to [`Self::points`], or empty: `true` where the track has no
    /// surface under the line. See [`Self::with_unsupported`].
    unsupported: Vec<bool>,
}

impl Line {
    /// Wraps a list of points. Order is driving order. No corridor.
    #[must_use]
    pub fn new(points: Vec<Vec3>) -> Self {
        Self {
            points,
            corridor: Vec::new(),
            unsupported: Vec::new(),
        }
    }

    /// Wraps a list of points and the corridor around them.
    ///
    /// A corridor whose length does not match the points is **dropped**, not
    /// truncated: a half-length one would silently give the back of the track no
    /// room, which reads as a driver bug rather than as the wiring mistake it
    /// is.
    #[must_use]
    pub fn with_corridor(points: Vec<Vec3>, corridor: Vec<Frame>) -> Self {
        let corridor = if corridor.len() == points.len() {
            corridor
        } else {
            Vec::new()
        };
        Self {
            points,
            corridor,
            unsupported: Vec::new(),
        }
    }

    /// Marks the points with no track surface under them.
    ///
    /// # Chosen, not measured (maintainer decision, 2026-09-29)
    ///
    /// [`Self::curvature`] reads a chord that touches one of these as straight:
    /// a craft over a gap is airborne and steers nothing, so a bend in the line
    /// there is not a corner to brake for. At `01_Track`'s lip (samples 31-42,
    /// where the line leaves an upper deck and runs about 60 degrees down
    /// through the air onto a lower floor) the pitch read as a 0.075 rad/unit
    /// bend, and this project's Ace braked from 127 u/s to about 20 and crawled
    /// off the lip every lap. The original's field, logged live in PPSSPP,
    /// crossed it 21 times of 21 at 69-111 u/s and flew the drop. The original's
    /// AI is not being copied: the maintainer's decision is that opponents obey
    /// the player's physics and drive smarter instead. Narrowed to gaps on
    /// purpose - discounting *every* pitch change took crests faster everywhere
    /// and killed three more `ai_clean_lap_gate` rows. See
    /// `docs/gameplay/leaving-the-track.md`.
    ///
    /// A mask whose length does not match the points is dropped, for the same
    /// reason [`Self::with_corridor`] drops a mismatched corridor.
    #[must_use]
    pub fn with_unsupported(mut self, unsupported: Vec<bool>) -> Self {
        self.unsupported = if unsupported.len() == self.points.len() {
            unsupported
        } else {
            Vec::new()
        };
        self
    }

    /// Whether the track has no surface under the line at `index`, wrapping.
    #[must_use]
    pub fn is_unsupported(&self, index: usize) -> bool {
        !self.unsupported.is_empty() && self.unsupported[index % self.unsupported.len()]
    }

    /// Whether this line knows how much room there is around it.
    #[must_use]
    pub fn has_corridor(&self) -> bool {
        !self.corridor.is_empty()
    }

    /// How many points the line holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether there is no line at all - a track with no spline, or a default.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// The point at `index`, wrapping.
    ///
    /// Returns the origin on an empty line rather than panicking: a track that
    /// produced no samples should leave an opponent sitting still, not take the
    /// process down.
    #[must_use]
    pub fn point(&self, index: usize) -> Vec3 {
        if self.points.is_empty() {
            return Vec3::ZERO;
        }
        self.points[index % self.points.len()]
    }

    /// The nearest point to `position`, searched only near `around`.
    ///
    /// Windowed rather than global, for the reason `Spline::nearest_windowed`
    /// gives: a track passes near itself, so a global search lets a craft's
    /// progress index jump the gap between two stacked sections. It is also what
    /// makes eight opponents affordable - a global search is the whole table per
    /// craft per tick.
    ///
    /// The window reaches further forward than back because a craft moves
    /// forward. Ties go to the earlier candidate, so the result cannot depend on
    /// iteration order.
    #[must_use]
    pub fn nearest(&self, position: Vec3, around: usize, window: usize) -> usize {
        let count = self.points.len();
        if count == 0 {
            return 0;
        }
        let back = window / 4;
        let mut best = around % count;
        let mut best_distance = f32::INFINITY;
        for step in 0..(back + window) {
            let index = (around + count + step - back) % count;
            let distance = self.points[index].distance_squared(position);
            // Strictly nearer, so a tie keeps the earlier candidate.
            if distance < best_distance {
                best_distance = distance;
                best = index;
            }
        }
        best
    }

    /// The point `distance` further along the line: its index, itself, and how
    /// far along the line it actually is.
    ///
    /// **Interpolated onto the final segment**, not snapped to the nearest
    /// control point. A real track's samples are about 2.5 units apart, so a
    /// snapped aim point jumps by that much each time the walk crosses a sample
    /// and injects a step into whatever reads it. The steering law reads it, and
    /// a step in the aim point is a step in the commanded turn rate.
    ///
    /// The returned distance is the *travelled* one, which is what a caller
    /// converting an offset into a curvature has to divide by - the requested
    /// distance is a request, and on a curve the two differ.
    ///
    /// Gives up after one full lap, so a degenerate line whose points coincide
    /// cannot spin forever.
    #[must_use]
    pub fn ahead(&self, index: usize, distance: f32) -> (usize, Vec3, f32) {
        let (at, _, _, point, travelled) = self.walk(index, distance);
        (at, point, travelled)
    }

    /// [`Self::ahead`], plus the corridor at the point it lands on.
    ///
    /// **The corridor is interpolated onto the same fraction of the same
    /// segment as the point is**, for the reason [`Self::ahead`] interpolates
    /// the point: a bound snapped to the nearest sample steps by a sample's
    /// worth every time the walk crosses one, and a driver that clamps its
    /// drift against a stepping bound puts that step straight into the
    /// commanded turn rate. The same trap, one level further out.
    #[must_use]
    pub fn aim(&self, index: usize, distance: f32) -> Aim {
        let (at, next, fraction, point, travelled) = self.walk(index, distance);
        Aim {
            index: at,
            point,
            travelled,
            corridor: self.frame(at, next, fraction),
        }
    }

    /// The corridor `fraction` of the way from `at` to `next`.
    fn frame(&self, at: usize, next: usize, fraction: f32) -> Option<Frame> {
        if self.corridor.is_empty() {
            return None;
        }
        let here = self.corridor[at];
        let there = self.corridor[next];
        Some(Frame {
            // Renormalised after the blend, which shortens a vector wherever
            // the track turns.
            lateral: (here.lateral + (there.lateral - here.lateral) * fraction).normalize_or_zero(),
            left: here.left + (there.left - here.left) * fraction,
            right: here.right + (there.right - here.right) * fraction,
        })
    }

    /// Walks `distance` along the line from `index`.
    ///
    /// Returns the segment it ended on (`at`, `next`), how far along that
    /// segment (`fraction`), the interpolated point, and the *travelled*
    /// distance - which is what a caller converting an offset into a curvature
    /// has to divide by, since on a curve it is not the distance requested.
    ///
    /// Gives up after one full lap, so a degenerate line whose points coincide
    /// cannot spin forever.
    fn walk(&self, index: usize, distance: f32) -> (usize, usize, f32, Vec3, f32) {
        let count = self.points.len();
        if count == 0 {
            return (0, 0, 0.0, Vec3::ZERO, 0.0);
        }
        let mut at = index % count;
        let mut travelled = 0.0;
        for _ in 0..count {
            let next = (at + 1) % count;
            let step = self.points[at].distance(self.points[next]);
            if travelled + step >= distance {
                if step <= f32::EPSILON {
                    break;
                }
                let fraction = ((distance - travelled) / step).clamp(0.0, 1.0);
                let point = self.points[at] + (self.points[next] - self.points[at]) * fraction;
                return (at, next, fraction, point, travelled + step * fraction);
            }
            travelled += step;
            at = next;
        }
        (at, at, 0.0, self.points[at], travelled)
    }

    /// The sharpest bend anywhere within `distance` ahead of `index`.
    ///
    /// A speed target built on the curvature at *one* point ahead brakes for a
    /// corner only once the sample it happens to look at is inside it, which on a
    /// long entry is too late. Taking the worst case over the whole braking
    /// window means a craft slows for the tightest thing it can see.
    #[must_use]
    pub fn max_curvature(&self, index: usize, distance: f32, span: f32) -> f32 {
        self.max_curvature_stepped(index, distance, span, span)
    }

    /// The same, with the **chord** and the **walk step** separated.
    ///
    /// # They are the same number in [`Self::max_curvature`], and that is a
    /// # coupling rather than a choice
    ///
    /// The walk above takes `curvature(at, span)` and then advances `span`, so
    /// one constant sets two independent things: how long a chord each reading
    /// averages over - its **resolution** - and how densely the window is
    /// **sampled**. Shrinking it to resolve a sharp apex also multiplies the
    /// number of readings taken, and every circuit on the disc carries path
    /// seams a short chord reads badly, so the two effects arrive together and
    /// cannot be told apart from the outside.
    ///
    /// That matters because the value axis is exhausted. Swept twice when the
    /// cap was chosen and again on 2026-09-12 against the current tree: `11`
    /// is still the best solo total (`907.97`, and the only span with twelve
    /// clean laps) against `860.75` at 4, `830.42` at 6 and `827.14` at 8 -
    /// and `8` turns `opponent_weapons_ground_truth`'s worst-opponent floor
    /// red at `0.38` against `0.45`. **A shorter chord resolves the apex
    /// better and costs more than it buys.** This entry point exists so the
    /// two halves of that trade can be measured apart; see
    /// `oag_ai::Tuning::curvature_chord`.
    ///
    /// `chord` is what each reading averages over; `step` is how far the walk
    /// advances between readings. Passing the same value for both is exactly
    /// [`Self::max_curvature`].
    #[must_use]
    pub fn max_curvature_stepped(&self, index: usize, distance: f32, chord: f32, step: f32) -> f32 {
        if self.points.len() < 3 || chord <= 0.0 || step <= 0.0 {
            return 0.0;
        }
        let mut worst = 0.0f32;
        let mut at = index;
        let mut travelled = 0.0;
        while travelled < distance {
            worst = worst.max(self.curvature(at, chord));
            let (next, _, stepped) = self.ahead(at, step);
            if stepped <= f32::EPSILON {
                break;
            }
            travelled += stepped;
            at = next;
        }
        worst
    }

    /// How sharply the line bends `span` ahead of `index`, in radians per unit.
    ///
    /// Three points - here, `span` ahead, and `span` past that - give a turned
    /// angle over a travelled distance, which is curvature in the only sense the
    /// speed target needs. Measured *ahead* of the craft rather than at it,
    /// because a corner has to be seen before it is entered.
    ///
    /// Returns zero on a straight, on a degenerate line, and on any line shorter
    /// than three points.
    #[must_use]
    pub fn curvature(&self, index: usize, span: f32) -> f32 {
        if self.points.len() < 3 {
            return 0.0;
        }
        // **All three walks run from `index`, not from each other's landing
        // index.** [`Self::ahead`] returns an interpolated point but the index
        // of the *segment* it landed in, and chaining the walk off that index
        // restarts it from the segment's own start rather than from the point
        // just returned. Where a segment is longer than `span`, all three walks
        // land inside that one segment and return the same index, so the second
        // and third points coincide, the outgoing chord is the zero vector, and
        // the guard below is all that stands between that and `acos(0)` being
        // charged as a right-angle turn over a fraction of a unit. Measured on
        // `06_Track`, whose racing line has one 14.78-unit segment where two
        // authored paths meet: the old chaining read that dead-straight seam as
        // a curvature of **25.9 radians per unit** - a radius of three
        // centimetres - and an AI braking against it held 1.2 units/s and never
        // recovered, because slowing shrinks the span that produced the reading.
        // Pinned by `a_long_segment_is_still_straight`.
        let (at_a, a, _) = self.ahead(index, span);
        let (at_b, b, _) = self.ahead(index, span * 2.0);
        let (at_c, c, _) = self.ahead(index, span * 3.0);
        // Over a gap the craft is flying, not steering: see
        // [`Self::with_unsupported`].
        if self.is_unsupported(at_a) || self.is_unsupported(at_b) || self.is_unsupported(at_c) {
            return 0.0;
        }
        let into = b - a;
        let out_of = c - b;
        // A chord of no length carries no direction, and `normalize_or_zero`
        // would hand `acos` a dot of zero - which is `PI / 2`, a right angle
        // invented out of nothing. Two coincident samples are enough to produce
        // one, and every circuit on the disc has a pair 0.11 units apart.
        if into.length() <= f32::EPSILON || out_of.length() <= f32::EPSILON {
            return 0.0;
        }
        // **The turn happens over the distance between the two chords' midpoints,
        // which is half their total length, not all of it.** Dividing by the
        // whole travelled distance reads a circle as half as curved as it is,
        // and a speed target built on that lets every craft into every corner at
        // 1.41 times the speed it can hold. Pinned by
        // `curvature_approximates_one_over_the_radius`.
        let travelled = (into.length() + out_of.length()) * 0.5;
        if travelled <= f32::EPSILON {
            return 0.0;
        }
        let into = into.normalize_or_zero();
        let out_of = out_of.normalize_or_zero();
        // `oag_core::math::acos` and not `f32::acos`: this angle reaches the
        // speed target every craft brakes against, and so the world hash, and
        // the platform's own `acos` is not required to be correctly rounded.
        // The clamp is still ours - a dot of two unit vectors can leave
        // `-1..=1` by a rounding error and `acos` of `1.0000001` is `NaN`.
        let turned = oag_core::math::acos(into.dot(out_of).clamp(-1.0, 1.0));
        turned / travelled
    }

    /// Which way the line bends over `span`, positive where it bends toward
    /// `lateral`.
    ///
    /// **Direction, not curvature** - [`Line::curvature`] is the magnitude.
    /// This is the chord turn projected across the line, so it carries the sign
    /// a caller asking "which way is the inside of this corner" needs, and it
    /// does it without an `acos`.
    #[must_use]
    pub fn bend(&self, index: usize, span: f32, lateral: Vec3) -> f32 {
        if self.points.len() < 3 {
            return 0.0;
        }
        // PROBE C: all three walks from the same origin, so the chords are
        // actually `span` apart even where one segment is longer than `span`.
        let (_, a, _) = self.ahead(index, span);
        let (_, b, _) = self.ahead(index, span * 2.0);
        let (_, c, _) = self.ahead(index, span * 3.0);
        let into = (b - a).normalize_or_zero();
        let out_of = (c - b).normalize_or_zero();
        (out_of - into).dot(lateral)
    }
}

#[cfg(test)]
mod tests;
