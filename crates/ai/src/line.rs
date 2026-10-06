//! The line an opponent drives, as plain world-space points, and the corridor
//! it is allowed to drift inside.

use oag_core::math::Vec3;

pub mod shift;

/// How much road before a gap counts as the run-up to a takeoff, in units.
///
/// **Chosen, not measured**, no confidence score. `05_Track`'s first jump has a
/// crest bend (curvature 0.0126, target 124 against a 127 craft) 20 to 30
/// samples before the first unsupported one; a driver that reads it as a corner
/// coasts off and leaves the ground 1 to 5 units a second short of the clearing
/// speed. 75 covers that crest, the one gap this was looked at on.
pub const TAKEOFF_RUNUP: f32 = 75.0;

/// The fewest unsupported samples in a row that make a takeoff rather than a
/// seam or lip the craft skims over.
///
/// **Chosen, not measured.** Marking every gap cost 20 to 40 wall-contact ticks
/// a lap on the 5-sample gap on `06_Track` and the 12-sample one on `01_Track`;
/// the jumps that matter are 50 samples and up.
pub const TAKEOFF_MIN_RUN: usize = 40;

/// How much room a craft has either side of the line, and which way "aside" is.
///
/// **Both bounds are relative to the line**, so `left` is at most zero and
/// `right` at least zero. The disc stores absolute offsets in the sample's
/// lateral axis alongside the racing line's own offset; the caller subtracts.
/// See `docs/formats/track.md`.
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
    /// How far along the line it actually is: the *travelled* distance, which
    /// differs from the one asked for once the line curves.
    pub travelled: f32,
    /// The corridor there, when the line carries one.
    pub corridor: Option<Frame>,
}

/// A closed run of world-space points, in driving order.
///
/// **The caller builds it**: on a real track the authored racing line, each
/// spline sample's `pos - HOVER_LIFT * down + racing_line * lateral`. Nothing
/// here knows that, which keeps this crate free of asset types and testable
/// against a circle (`docs/gameplay/ai.md`). Closed, so index arithmetic wraps.
///
/// A line may carry the **corridor** around it, one [`Frame`] per point: what a
/// driver spends on not driving the ideal line (see [`crate::Personality`]).
/// Without one every craft drives the same line; `Line::new` is that case.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Line {
    points: Vec<Vec3>,
    /// Parallel to [`Self::points`], or empty; [`Line::with_corridor`] drops a
    /// mismatched one rather than half-using it.
    corridor: Vec<Frame>,
    /// Parallel to [`Self::points`], or empty: `true` where the track has no
    /// surface under the line. See [`Self::with_unsupported`].
    unsupported: Vec<bool>,
    /// Parallel to [`Self::points`], or empty: `true` on the [`TAKEOFF_RUNUP`]
    /// of road before each run of unsupported samples. See [`Self::is_takeoff`].
    takeoff: Vec<bool>,
}

impl Line {
    /// Wraps a list of points. Order is driving order. No corridor.
    #[must_use]
    pub fn new(points: Vec<Vec3>) -> Self {
        Self {
            points,
            corridor: Vec::new(),
            unsupported: Vec::new(),
            takeoff: Vec::new(),
        }
    }

    /// Wraps a list of points and the corridor around them.
    ///
    /// A corridor of the wrong length is **dropped**, not truncated: a short one
    /// would silently give the back of the track no room and read as a driver
    /// bug.
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
            takeoff: Vec::new(),
        }
    }

    /// Marks the points with no track surface under them.
    ///
    /// # Chosen, not measured (maintainer decision, 2026-09-29)
    ///
    /// [`Self::curvature`] reads a chord touching one of these as straight: a
    /// craft over a gap is airborne and steers nothing. At `01_Track`'s lip
    /// (samples 31-42, about 60 degrees down onto a lower floor) the pitch read
    /// as a 0.075 rad/unit bend and our Ace braked from 127 u/s to about 20. The
    /// original's field, logged live in PPSSPP, crossed it 21 times of 21 at
    /// 69-111 u/s. The original's AI is not copied: opponents obey the player's
    /// physics and drive smarter. Narrowed to gaps because discounting *every*
    /// pitch change killed three more `ai_clean_lap_gate` rows. See
    /// `docs/gameplay/leaving-the-track.md`.
    ///
    /// A mask of the wrong length is dropped, like [`Self::with_corridor`]'s.
    #[must_use]
    pub fn with_unsupported(mut self, unsupported: Vec<bool>) -> Self {
        self.unsupported = if unsupported.len() == self.points.len() {
            unsupported
        } else {
            Vec::new()
        };
        self.takeoff = self.takeoff_runups();
        self
    }

    /// The [`TAKEOFF_RUNUP`] of road before each run of unsupported samples.
    ///
    /// Walked backwards from the run's first sample summing segment lengths (a
    /// distance, since samples are unevenly spaced). Wraps.
    fn takeoff_runups(&self) -> Vec<bool> {
        let n = self.points.len();
        let mut marked = vec![false; if self.unsupported.is_empty() { 0 } else { n }];
        for start in 0..marked.len() {
            let previous = (start + n - 1) % n;
            if !self.unsupported[start] || self.unsupported[previous] {
                continue;
            }
            let run = (0..n)
                .take_while(|step| self.unsupported[(start + step) % n])
                .count();
            if run < TAKEOFF_MIN_RUN {
                continue;
            }
            let mut at = start;
            let mut walked = 0.0;
            while walked < TAKEOFF_RUNUP {
                let back = (at + n - 1) % n;
                if back == start || self.unsupported[back] {
                    break;
                }
                walked += (self.points[at] - self.points[back]).length();
                marked[back] = true;
                at = back;
            }
        }
        marked
    }

    /// Whether `index` lies on the run-up to a gap: the road a craft is still
    /// on before it leaves the ground, wrapping.
    #[must_use]
    pub fn is_takeoff(&self, index: usize) -> bool {
        !self.takeoff.is_empty() && self.takeoff[index % self.takeoff.len()]
    }

    /// Whether a takeoff run-up ([`Self::is_takeoff`]) starts within
    /// `distance` of travel ahead of `index`, `index` itself included.
    #[must_use]
    pub fn takeoff_within(&self, index: usize, distance: f32) -> bool {
        let n = self.takeoff.len();
        if n == 0 {
            return false;
        }
        let mut at = index % n;
        let mut travelled = 0.0;
        for _ in 0..n {
            if self.takeoff[at] {
                return true;
            }
            let next = (at + 1) % n;
            travelled += (self.points[next] - self.points[at]).length();
            if travelled > distance {
                return false;
            }
            at = next;
        }
        false
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

    /// The point at `index`, wrapping; the origin on an empty line rather than a
    /// panic, so a track with no samples leaves an opponent sitting still.
    #[must_use]
    pub fn point(&self, index: usize) -> Vec3 {
        if self.points.is_empty() {
            return Vec3::ZERO;
        }
        self.points[index % self.points.len()]
    }

    /// The nearest point to `position`, searched only near `around`.
    ///
    /// Windowed, for the reason `Spline::nearest_windowed` gives: a track passes
    /// near itself, so a global search lets the progress index jump between
    /// stacked sections (and costs the whole table per craft per tick). The
    /// window reaches further forward than back. Ties go to the earlier
    /// candidate, so the result cannot depend on iteration order.
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
    /// **Interpolated onto the final segment**, not snapped: samples are about
    /// 2.5 units apart, and a snapped aim point steps by that each time the walk
    /// crosses one, which is a step in the commanded turn rate. The returned
    /// distance is the *travelled* one, which a caller converting an offset into
    /// a curvature must divide by. Gives up after one lap, so coincident points
    /// cannot spin forever.
    #[must_use]
    pub fn ahead(&self, index: usize, distance: f32) -> (usize, Vec3, f32) {
        let (at, _, _, point, travelled) = self.walk(index, distance);
        (at, point, travelled)
    }

    /// [`Self::ahead`], plus the corridor at the point it lands on.
    ///
    /// **Interpolated onto the same fraction of the same segment**, for
    /// [`Self::ahead`]'s reason: a stepping bound goes straight into the
    /// commanded turn rate.
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
            // Renormalised: the blend shortens a vector wherever the track turns.
            lateral: (here.lateral + (there.lateral - here.lateral) * fraction).normalize_or_zero(),
            left: here.left + (there.left - here.left) * fraction,
            right: here.right + (there.right - here.right) * fraction,
        })
    }

    /// The corridor at sample `index`, wrapping; `None` on a line without one.
    #[must_use]
    pub fn corridor_at(&self, index: usize) -> Option<Frame> {
        let count = self.corridor.len();
        (count > 0).then(|| self.corridor[index % count])
    }

    /// Walks `distance` along the line from `index`.
    ///
    /// Returns the segment it ended on (`at`, `next`), the `fraction` along it,
    /// the interpolated point, and the *travelled* distance (see
    /// [`Self::ahead`]). Gives up after one lap.
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
    /// A target from the curvature at *one* point brakes only once that sample
    /// is inside the corner, too late on a long entry; the worst case over the
    /// window slows for the tightest thing in sight.
    #[must_use]
    pub fn max_curvature(&self, index: usize, distance: f32, span: f32) -> f32 {
        self.max_curvature_stepped(index, distance, span, span)
    }

    /// [`Self::max_curvature`] with the **chord** and the **walk step**
    /// separated.
    ///
    /// There they are one number: `curvature(at, span)` then advance `span`, so
    /// one constant sets both each reading's **resolution** and the window's
    /// **sampling density**, and the two effects cannot be told apart. The value
    /// axis is exhausted (`11` still best, `8` turns a field floor red:
    /// `docs/gameplay/ai.md`, "Tuning sweep tables"); **a shorter chord resolves
    /// the apex better and costs more than it buys**. This entry point lets the
    /// halves be measured apart; see `oag_ai::Tuning::curvature_chord`.
    ///
    /// Passing the same value for `chord` and `step` is exactly
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
    /// Three points (here, `span` ahead, `span` past that) give a turned angle
    /// over a travelled distance. Measured *ahead* because a corner has to be
    /// seen before it is entered. Zero on a straight, a degenerate line, or
    /// fewer than three points.
    ///
    /// # A valley is not a corner
    ///
    /// **The bend is the turn a craft steers plus the crest it may leave the
    /// ground over**, not any pitch change (see [`bend_angle`]). A concave line
    /// presses the craft into the road and asks nothing of its yaw, so braking
    /// for it only costs speed. `05_Track` runs a 70 units/s crest lip after
    /// such a bend, and a craft that braked 7 units/s at the foot arrived at 69
    /// and clipped it, where 71 and up cleared it.
    ///
    /// **Chosen, not measured**, board in `docs/gameplay/ai.md` ("A valley is
    /// not a corner"). It revisits `64da876e`, which discounted *every* pitch
    /// change and was narrowed to gaps in `2c1a3d4f` because it took crests
    /// faster everywhere; keeping the convex half is what that lacked.
    #[must_use]
    pub fn curvature(&self, index: usize, span: f32) -> f32 {
        if self.points.len() < 3 {
            return 0.0;
        }
        // **All three walks run from `index`, not from each other's landing
        // index.** [`Self::ahead`] returns the index of the *segment* landed in,
        // so chaining restarts at that segment's start; where a segment is longer
        // than `span` all three land in it, the second and third points coincide
        // and the zero outgoing chord would make `acos(0)` a right-angle turn.
        // `06_Track` has a 14.78-unit segment where two authored paths meet, and
        // chaining read that straight seam as 25.9 radians per unit; the AI held
        // 1.2 units/s and never recovered. Pinned by
        // `a_long_segment_is_still_straight`.
        let (at_a, a, _) = self.ahead(index, span);
        let (at_b, b, _) = self.ahead(index, span * 2.0);
        let (at_c, c, _) = self.ahead(index, span * 3.0);
        // Over a gap the craft is flying, not steering: see
        if self.is_unsupported(at_a) || self.is_unsupported(at_b) || self.is_unsupported(at_c) {
            return 0.0;
        }
        // On the run-up to a gap the crest is the launch ramp: only yaw counts.
        let launches = self.is_takeoff(at_a) || self.is_takeoff(at_b) || self.is_takeoff(at_c);
        let into = b - a;
        let out_of = c - b;
        // A chord of no length has no direction: `normalize_or_zero` would hand
        // `acos` a dot of zero, a right angle from nothing. Every circuit has a
        // pair of samples 0.11 units apart.
        if into.length() <= f32::EPSILON || out_of.length() <= f32::EPSILON {
            return 0.0;
        }
        // **The turn happens over the distance between the chords' midpoints,
        // half their total length.** Dividing by all of it reads a circle as half
        // as curved and lets every craft into every corner at 1.41 times the
        // speed it can hold. Pinned by `curvature_approximates_one_over_the_radius`.
        let travelled = (into.length() + out_of.length()) * 0.5;
        if travelled <= f32::EPSILON {
            return 0.0;
        }
        let into = into.normalize_or_zero();
        let out_of = out_of.normalize_or_zero();
        // `oag_core::math::acos`, not `f32::acos`: this angle reaches the world
        // hash and the platform's own `acos` need not be correctly rounded. The
        // clamp is ours: a dot of unit vectors can leave `-1..=1` by rounding and
        // `acos(1.0000001)` is `NaN`.
        bend_angle(into, out_of, !launches) / travelled
    }

    /// Which way the line bends over `span`, positive toward `lateral`.
    ///
    /// **Direction, not curvature** ([`Line::curvature`] is the magnitude): the
    /// chord turn projected across the line, with no `acos`.
    #[must_use]
    pub fn bend(&self, index: usize, span: f32, lateral: Vec3) -> f32 {
        if self.points.len() < 3 {
            return 0.0;
        }
        // All three walks from the same origin, so the chords are `span` apart
        // even where one segment is longer than `span`.
        let (_, a, _) = self.ahead(index, span);
        let (_, b, _) = self.ahead(index, span * 2.0);
        let (_, c, _) = self.ahead(index, span * 3.0);
        let into = (b - a).normalize_or_zero();
        let out_of = (c - b).normalize_or_zero();
        (out_of - into).dot(lateral)
    }
}

/// How far the line turns between two unit chords, in radians: the yaw a craft
/// has to steer plus the **convex** pitch it may leave the road over.
///
/// `hypot(yaw, crest)`: `yaw` is the angle between the chords flattened onto the
/// ground plane (world Y up), `crest` how far the climb angle *falls* from the
/// first chord to the second. Concave pitch contributes nothing; see
/// [`Line::curvature`]. A chord steeper than sixty degrees has no ground heading
/// worth reading, so it falls back to the plain angle between the chords.
fn bend_angle(into: Vec3, out_of: Vec3, crest_counts: bool) -> f32 {
    let flat_in = Vec3::new(into.x, 0.0, into.z);
    let flat_out = Vec3::new(out_of.x, 0.0, out_of.z);
    if flat_in.length() <= 0.5 || flat_out.length() <= 0.5 {
        // `oag_core::math::acos` and not `f32::acos`: see [`Line::curvature`].
        return oag_core::math::acos(into.dot(out_of).clamp(-1.0, 1.0));
    }
    let yaw = oag_core::math::acos(
        flat_in
            .normalize_or_zero()
            .dot(flat_out.normalize_or_zero())
            .clamp(-1.0, 1.0),
    );
    // The climb angle of a unit chord is `pi/2 - acos(y)`; the `pi/2` cancels.
    let climb_in = -oag_core::math::acos(into.y.clamp(-1.0, 1.0));
    let climb_out = -oag_core::math::acos(out_of.y.clamp(-1.0, 1.0));
    let crest = if crest_counts {
        (climb_in - climb_out).max(0.0)
    } else {
        0.0
    };
    (yaw * yaw + crest * crest).sqrt()
}

#[cfg(test)]
mod tests;
