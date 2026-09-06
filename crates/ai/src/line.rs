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
}

impl Line {
    /// Wraps a list of points. Order is driving order. No corridor.
    #[must_use]
    pub fn new(points: Vec<Vec3>) -> Self {
        Self {
            points,
            corridor: Vec::new(),
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
        Self { points, corridor }
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
        if self.points.len() < 3 || span <= 0.0 {
            return 0.0;
        }
        let mut worst = 0.0f32;
        let mut at = index;
        let mut travelled = 0.0;
        while travelled < distance {
            worst = worst.max(self.curvature(at, span));
            let (next, _, stepped) = self.ahead(at, span);
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
        let (_, a, _) = self.ahead(index, span);
        let (_, b, _) = self.ahead(index, span * 2.0);
        let (_, c, _) = self.ahead(index, span * 3.0);
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
mod tests {
    use super::*;

    /// A unit-ish circle, counter-clockwise in the XZ plane.
    fn circle(radius: f32, points: usize) -> Line {
        Line::new(
            (0..points)
                .map(|step| {
                    let angle = std::f32::consts::TAU * step as f32 / points as f32;
                    Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
                })
                .collect(),
        )
    }

    #[test]
    fn an_empty_line_answers_everything_with_the_origin() {
        let line = Line::default();
        assert!(line.is_empty());
        assert_eq!(line.point(7), Vec3::ZERO);
        assert_eq!(line.nearest(Vec3::ONE, 3, 20), 0);
        assert_eq!(line.ahead(0, 50.0), (0, Vec3::ZERO, 0.0));
        assert_eq!(line.curvature(0, 10.0), 0.0);
    }

    #[test]
    fn indices_wrap() {
        let line = circle(10.0, 8);
        assert_eq!(line.point(0), line.point(8));
        assert_eq!(line.point(3), line.point(11));
    }

    #[test]
    fn nearest_finds_the_point_under_a_position() {
        let line = circle(100.0, 64);
        let target = line.point(20);
        assert_eq!(line.nearest(target, 18, 16), 20);
    }

    /// The window is the whole point: a position on the far side of the ring is
    /// *not* found from here, and that is what stops a progress index jumping
    /// across a track that passes near itself.
    #[test]
    fn nearest_does_not_look_across_the_ring() {
        let line = circle(100.0, 64);
        let opposite = line.point(40);
        assert_ne!(line.nearest(opposite, 0, 8), 40);
    }

    #[test]
    fn ahead_walks_the_requested_distance() {
        let line = Line::new(vec![
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(0.0, 0.0, 20.0),
            Vec3::new(0.0, 0.0, 30.0),
        ]);
        // Interpolated: 15 units along is halfway between points 1 and 2.
        let (index, point, travelled) = line.ahead(0, 15.0);
        assert_eq!(index, 1);
        assert!((point.z - 15.0).abs() < 1e-4, "point {point:?}");
        assert!((travelled - 15.0).abs() < 1e-4, "travelled {travelled}");
        assert_eq!(line.ahead(0, 0.0).0, 0);
    }

    /// A tighter circle bends harder. The absolute value depends on the sampling,
    /// so the ordering is what is asserted.
    #[test]
    fn a_tighter_circle_reads_as_more_curved() {
        let tight = circle(50.0, 64);
        let wide = circle(400.0, 64);
        assert!(tight.curvature(0, 10.0) > wide.curvature(0, 10.0));
    }

    /// Four points a fixed distance apart, with a corridor that widens along
    /// them, so an interpolated bound is distinguishable from a snapped one.
    fn widening() -> Line {
        let points: Vec<Vec3> = (0..4)
            .map(|step| Vec3::new(0.0, 0.0, 10.0 * step as f32))
            .collect();
        let corridor = (0..4)
            .map(|step| Frame {
                lateral: Vec3::X,
                left: -(step as f32),
                right: 1.0 * step as f32,
            })
            .collect();
        Line::with_corridor(points, corridor)
    }

    #[test]
    fn a_line_can_have_no_corridor() {
        let line = Line::new(vec![Vec3::ZERO, Vec3::X]);
        assert!(!line.has_corridor());
        assert_eq!(line.aim(0, 0.5).corridor, None);
    }

    /// A corridor that does not match the points is dropped rather than
    /// half-used: a truncated one would silently give the back of the track no
    /// room, and that reads as a driver bug rather than as the wiring mistake
    /// it is.
    #[test]
    fn a_mismatched_corridor_is_dropped() {
        let line = Line::with_corridor(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![Frame::default(), Frame::default()],
        );
        assert!(!line.has_corridor());
    }

    /// **The bound is interpolated onto the same segment fraction the aim point
    /// is**, not snapped to the sample. Snapped, it steps by a whole sample's
    /// worth every time the walk crosses one, and a driver that clamps its
    /// drift against a stepping bound puts that step into the commanded turn
    /// rate.
    #[test]
    fn the_corridor_is_interpolated_rather_than_snapped() {
        let line = widening();
        let frame = line.aim(0, 15.0).corridor.expect("a corridor");
        assert!((frame.right - 1.5).abs() < 1e-4, "right {}", frame.right);
        assert!((frame.left + 1.5).abs() < 1e-4, "left {}", frame.left);
        assert!((frame.lateral - Vec3::X).length() < 1e-4);
    }

    #[test]
    fn a_frame_measures_the_room_on_the_side_being_asked_about() {
        let frame = Frame {
            lateral: Vec3::X,
            left: -2.0,
            right: 6.0,
        };
        assert_eq!(frame.room(1.0), 6.0);
        assert_eq!(frame.room(-1.0), 2.0);
        assert_eq!(frame.clamp(9.0), 6.0);
        assert_eq!(frame.clamp(-9.0), -2.0);
        assert_eq!(frame.clamp(3.0), 3.0);
    }

    #[test]
    fn a_straight_line_has_no_curvature() {
        let line = Line::new(
            (0..16)
                .map(|step| Vec3::new(0.0, 0.0, 10.0 * step as f32))
                .collect(),
        );
        assert!(line.curvature(0, 10.0).abs() < 1e-6);
    }

    /// A straight line stays straight even where one of its segments is longer
    /// than the span being asked for.
    ///
    /// **This failed at 28.6 radians per unit before the walk was rooted at
    /// `index`**, and the shape is the one every circuit on the disc ships: two
    /// authored paths meet, the samples bunch to a tenth of a unit, and one
    /// segment jumps the gap. `06_Track`'s jump is 14.78 units and is the only
    /// one on the disc longer than ten, which is why it was the only circuit an
    /// AI could be parked on by a corner that is not there.
    #[test]
    fn a_long_segment_is_still_straight() {
        let mut points = Vec::new();
        let mut x = 0.0f32;
        // Long enough that three spans past the last index tested still land
        // well short of the wrap - a `Line` is closed, and the join of an open
        // run of points is a genuine 180-degree turn that would mask this.
        for step in 0..140 {
            points.push(Vec3::new(x, 0.0, 0.0));
            // Two samples a tenth of a unit apart, then the jump: the seam.
            x += if step == 40 {
                0.1
            } else if step == 41 {
                14.78
            } else {
                1.5
            };
        }
        let line = Line::new(points);
        for span in [4.0f32, 8.0, 10.0, 14.0, 24.0] {
            for index in 36..46 {
                let measured = line.curvature(index, span);
                assert!(
                    measured < 1e-3,
                    "span {span} at index {index} measured {measured} on a straight line"
                );
            }
        }
    }

    /// A circle's curvature is `1 / radius`, and this estimator should land near
    /// it - it is a speed target's input, so being wrong by a factor would be
    /// wrong by a factor everywhere downstream.
    #[test]
    fn curvature_approximates_one_over_the_radius() {
        let line = circle(100.0, 128);
        let measured = line.curvature(0, 20.0);
        assert!(
            (measured - 0.01).abs() < 0.002,
            "measured {measured}, expected about 0.01"
        );
    }
}
