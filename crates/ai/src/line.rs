//! The line an opponent drives, as plain world-space points.

use oag_core::math::Vec3;

/// A closed run of world-space points, in driving order.
///
/// **The caller builds it.** On a real track that means the authored racing
/// line, each spline sample's `pos - HOVER_LIFT * down + racing_line * lateral`,
/// but nothing here knows that, which is what keeps this crate free of the asset
/// types and testable against a circle. See `docs/gameplay/ai.md`.
///
/// Closed rather than open: a race goes round, so index arithmetic wraps and
/// there is no end to fall off.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Line {
    points: Vec<Vec3>,
}

impl Line {
    /// Wraps a list of points. Order is driving order.
    #[must_use]
    pub fn new(points: Vec<Vec3>) -> Self {
        Self { points }
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
        let count = self.points.len();
        if count == 0 {
            return (0, Vec3::ZERO, 0.0);
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
                return (at, point, travelled + step * fraction);
            }
            travelled += step;
            at = next;
        }
        (at, self.points[at], travelled)
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
        let (first, a, _) = self.ahead(index, span);
        let (second, b, _) = self.ahead(first, span);
        let (_, c, _) = self.ahead(second, span);
        let into = b - a;
        let out_of = c - b;
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
        // `acos` of a dot product that rounding can push outside `-1..=1`.
        let turned = into.dot(out_of).clamp(-1.0, 1.0).acos();
        turned / travelled
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

    #[test]
    fn a_straight_line_has_no_curvature() {
        let line = Line::new(
            (0..16)
                .map(|step| Vec3::new(0.0, 0.0, 10.0 * step as f32))
                .collect(),
        );
        assert!(line.curvature(0, 10.0).abs() < 1e-6);
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
