//! The track's centre line as a flat table, and the AI racing line derived from
//! it: [`Spline`], the path order it is walked in, and the corridor the drivers
//! are allowed to stray into.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/spline.rs`.

use super::*;

/// The authored racing line and the corridor around it, one entry per spline
/// sample.
///
/// Five things the disc carries and this reads: the sample position, the
/// `HOVER_LIFT` the loader applies to it, `racing_line` - the lateral offset the
/// artists authored, in the sample's own `lateral` axis - and the two bounds
/// `ai_bound_left` and `ai_bound_right` that fence the AI corridor in the same
/// axis. See `docs/formats/track.md`.
///
/// **The bounds are rebased onto the line.** The disc stores all three as
/// offsets from the sample's own centre; a driver only ever asks how far it may
/// stray from the line it is driving, so what it gets is the difference. Left
/// comes out at most zero and right at least zero, which the shipped data
/// guarantees - the loader clamps the bounds to straddle the racing line by at
/// least 0.1 and all 34,261 control points already satisfy it.
///
/// The corridor is what an opponent's [`oag_ai::Personality`] spends on not
/// driving the ideal line, so how far the field spreads is the track's own
/// property and narrows where the artists narrowed it. See
/// `docs/gameplay/ai.md`.
///
/// Index-parallel to [`ai_order`] on purpose - see [`RaceSim::racing_line`].
#[must_use]
pub(super) fn racing_line(
    spline: &Spline,
    order: &[u32],
    collision: &oag_physics::CollisionWorld,
    reach: f32,
) -> oag_ai::Line {
    let samples: Vec<_> = order
        .iter()
        .filter_map(|&index| spline.sample(index as usize))
        .collect();
    let mut points: Vec<Vec3> = samples
        .iter()
        .map(|sample| {
            let down = Vec3::from_array(sample.down);
            let lateral = Vec3::from_array(sample.lateral);
            Vec3::from_array(sample.pos) - oag_vex::track::HOVER_LIFT * down
                + sample.racing_line * lateral
        })
        .collect();
    let mut corridor: Vec<oag_ai::Frame> = samples
        .iter()
        .map(|sample| oag_ai::Frame {
            // Already unit length: checked on all 34,261 control points, and
            // the B-spline blend of four unit vectors is not, which is why
            // this renormalises rather than trusting the sample.
            lateral: Vec3::from_array(sample.lateral).normalize_or_zero(),
            left: (sample.ai_bound_left - sample.racing_line).min(0.0),
            right: (sample.ai_bound_right - sample.racing_line).max(0.0),
        })
        .collect();
    // Where the track has no surface under the line, cast down the sample's
    // own normal from one probe reach above the line to one below it - what a
    // craft's hover probes reach, and the cast `race_ground_truth`'s "nothing
    // under the line" table makes. See `oag_ai::Line::with_unsupported` for
    // what a driver does with it and why (chosen, not measured).
    let ups: Vec<Vec3> = samples
        .iter()
        .map(|sample| (-Vec3::from_array(sample.down)).normalize_or_zero())
        .collect();
    let unsupported_under = |points: &[Vec3]| -> Vec<bool> {
        points
            .iter()
            .zip(&ups)
            .map(|(&point, &up)| {
                oag_physics::Raycaster::raycast(
                    collision,
                    oag_physics::Ray::new(point + up * reach, -up, reach * 2.0),
                    None,
                    false,
                )
                .is_none()
            })
            .collect()
    };
    let unsupported = unsupported_under(&points);
    // Off a magstrip on the run-up to a jump: see `takeoff_line`. The gaps are
    // re-read under the moved line, which is the one a craft flies.
    let takeoff: Vec<bool> = {
        let probe = oag_ai::Line::new(points.clone()).with_unsupported(unsupported.clone());
        (0..points.len()).map(|i| probe.is_takeoff(i)).collect()
    };
    let moved = super::takeoff_line::off_magstrips(
        &mut points,
        &mut corridor,
        &ups,
        &takeoff,
        collision,
        reach,
    );
    if moved > 0 {
        log::info!("ai line: {moved} takeoff run-up sample(s) moved off a magstrip");
    }
    let unsupported = if moved > 0 {
        unsupported_under(&points)
    } else {
        unsupported
    };
    oag_ai::Line::with_corridor(points, corridor).with_unsupported(unsupported)
}

/// Which spline samples the AI line is made of, in the order a lap drives them.
///
/// **The one place the two index spaces are related**, and the reason there are
/// two. [`Spline`] is every path the track authors, concatenated in *file*
/// order, because the hover hold and the culler have to be able to locate a
/// craft wherever it physically is - including on the branch of a split that
/// this lap does not use. A driver wants the opposite: the circuit, once round,
/// with nothing in it that a lap does not drive.
///
/// On nine of the disc's twelve circuits those are the same list and this is the
/// identity permutation. On `05_Track`, `14_Track` and `07_Track` the file holds
/// three paths and the lap ring walks two; the third is the other side of a
/// split, and file order dropped a thousand samples of it into the middle of the
/// lap, pointing the wrong way. A driver reaching the end of path 0 found its
/// next samples a kilometre away, its 48-sample window could not follow, and the
/// index stuck - those three circuits were exactly the three a lone craft never
/// got a second lap out of. See `docs/gameplay/ai.md`.
///
/// Falls back to the whole table in file order when the track has no closed
/// ring. A driveable ribbon with no lap counter still wants a line, and "every
/// sample there is" is the honest answer when nothing knows which way round is
/// forward.
#[must_use]
pub(super) fn ai_order(spline: &Spline, course: Option<&Course>) -> Vec<u32> {
    let every = || (0..spline.len() as u32).collect::<Vec<_>>();
    let Some(course) = course else {
        return every();
    };
    let mut order = Vec::with_capacity(spline.len());
    for path in course.path_order() {
        // The samples of one path, which the table holds contiguously. Scanning
        // rather than assuming the two crates resample identically: `Course` and
        // `Spline` each carry their own `STEPS_PER_SEGMENT` and a mapping built
        // on them being equal would break silently if either moved.
        order.extend(
            (0..spline.len() as u32).filter(|&index| spline.path_of(index as usize) == Some(path)),
        );
    }
    if order.is_empty() { every() } else { order }
}

/// The spline resampled into a flat table, for locating a ship on the track.
///
/// A `Vec` walked in order rather than any kind of spatial index: the nearest-sample
/// comparison feeds simulation state, so the order it happens in must not be able to
/// vary between runs, and ties go to the earlier sample. See
/// `docs/architecture/determinism.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    samples: Vec<Sample>,
    /// Which path each sample came from, parallel to [`Self::samples`].
    paths: Vec<u16>,
}

impl Spline {
    /// Samples per control-point interval.
    ///
    /// The same four `oag_render::track` draws the ribbon with, so the table and the
    /// picture agree about where the track is.
    pub const STEPS_PER_SEGMENT: usize = 4;

    /// Resamples every path of a decoded spline graph.
    #[must_use]
    pub fn from_track(ai: &AiTrack) -> Self {
        let mut samples = Vec::new();
        let mut paths = Vec::new();
        for (index, path) in ai.paths.iter().enumerate() {
            for segment in 0..path.points.len() {
                for step in 0..Self::STEPS_PER_SEGMENT {
                    let t = step as f32 / Self::STEPS_PER_SEGMENT as f32;
                    if let Some(sample) = path.sample(segment, t) {
                        samples.push(sample);
                        paths.push(index as u16);
                    }
                }
            }
        }
        Self { samples, paths }
    }

    /// How many samples the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the track produced no samples at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The first sample of the first path: where a ship is put.
    ///
    /// Not a grid slot. How the original assigns one is an open question that
    /// `oag_gameplay::spawn` deliberately leaves alone, so this is the start of the
    /// track and claims nothing more than that.
    #[must_use]
    pub fn start(&self) -> Option<&Sample> {
        self.samples.first()
    }

    /// The sample as [`oag_physics::maglock`] wants it: **lifted**, with its axis.
    ///
    /// Two conversions happen here and both are the format side's business rather
    /// than the simulation's:
    ///
    /// - `Sample::pos` is the *unlifted* disc value and the running game holds the
    ///   lifted one, because `AiTrack_LoadPathPoints` does `pos -= 3.0 * down` at
    ///   load. The hold subtracts that same lift back off to recover the surface
    ///   point, so it has to be handed the lifted form or it lands three units low.
    ///   Blending is linear, so lifting an interpolated sample and interpolating
    ///   lifted points are the same thing.
    /// - `down` is passed **raw**. The hold normalises it where it needs an axis
    ///   and reads it unnormalised where the original does, which is the mag
    ///   probe's direction.
    #[must_use]
    pub fn track_sample(sample: &Sample) -> oag_physics::TrackSample {
        let down = Vec3::from_array(sample.down);
        oag_physics::TrackSample {
            position: Vec3::from_array(sample.pos) - down * oag_vex::track::HOVER_LIFT,
            down,
        }
    }

    /// The nearest sample to `position`: its index, itself, and its distance.
    #[must_use]
    pub fn nearest(&self, position: Vec3) -> Option<(usize, &Sample, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (index, sample) in self.samples.iter().enumerate() {
            let distance = (Vec3::from_array(sample.pos) - position).length();
            // Strictly nearer, so a tie keeps the earlier sample.
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, distance)| (index, &self.samples[index], distance))
    }

    /// The nearest sample to `position` with its distance and path, skipping the
    /// path `exclude` names: Pilot Assist's located record and its fork sibling.
    #[must_use]
    pub fn nearest_with_path(
        &self,
        position: Vec3,
        exclude: Option<u16>,
    ) -> Option<(&Sample, f32, u16)> {
        let mut best: Option<(usize, f32)> = None;
        for (index, sample) in self.samples.iter().enumerate() {
            if exclude == Some(self.paths[index]) {
                continue;
            }
            let distance = (Vec3::from_array(sample.pos) - position).length();
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, distance)| (&self.samples[index], distance, self.paths[index]))
    }

    /// The nearest sample to `position`, searching only `window` samples either
    /// side of `around` in table order.
    ///
    /// **A local search whose failure mode is safe.** [`Self::nearest`] walks
    /// every sample - ~3,400 on a real track - which is affordable once a tick
    /// inside the simulation and not twice more per *frame* on top. The camera
    /// is a chase spring a few units behind the craft, so it is a few samples
    /// away in this table, and a window finds it.
    ///
    /// When it does not - the window straddles a junction, or the camera really
    /// has left the track - the answer is a sample that is too far away, which
    /// the caller turns into [`UNPLACED`] and therefore into *draw everything*.
    /// A local search that misses costs a frame of culling, never a frame of
    /// missing geometry, which is why this is allowed to be approximate.
    #[must_use]
    pub fn nearest_within(
        &self,
        position: Vec3,
        around: usize,
        window: usize,
    ) -> Option<(usize, &Sample, f32)> {
        let last = self.samples.len().checked_sub(1)?;
        let from = around.saturating_sub(window);
        let to = around.saturating_add(window).min(last);
        let mut best: Option<(usize, f32)> = None;
        for index in from..=to {
            let distance = (Vec3::from_array(self.samples[index].pos) - position).length();
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, distance)| (index, &self.samples[index], distance))
    }

    /// What `AiTrack_LocatePosition(100.0, track, &frame, position, ...)` hands
    /// `LeachBeam_KeepInTrack`: the nearest sample's frame, in the running
    /// game's **lifted** form (`pos -= 3.0 * down`, see [`Self::track_sample`]),
    /// or `None` when nothing lies within `100.0`.
    ///
    /// **Chosen, not measured:** `100.0` is the call's first argument, read as
    /// a search radius, and the frame is the nearest of four samples a segment
    /// rather than the original's own interpolation along the segment.
    #[must_use]
    pub fn tube_frame(&self, position: Vec3) -> Option<oag_fx::beam::TubeFrame> {
        /// `AiTrack_LocatePosition`'s first argument in `LeachBeam_KeepInTrack`.
        const LOCATE_RADIUS: f32 = 100.0;
        let (_, sample, distance) = self.nearest(position)?;
        if distance > LOCATE_RADIUS {
            return None;
        }
        let track = Self::track_sample(sample);
        Some(oag_fx::beam::TubeFrame {
            pos: track.position,
            down: track.down,
            lateral: Vec3::from_array(sample.lateral),
            half_width_left: sample.half_width_left,
            half_width_right: sample.half_width_right,
        })
    }

    /// Distance from `position` to the nearest sample, or `None` on an empty track.
    ///
    /// Distance to a *sample*, not to the curve: at four samples per segment the two
    /// differ by a fraction of a control-point interval, far below anything worth
    /// asserting on.
    #[must_use]
    pub fn distance_to(&self, position: Vec3) -> Option<f32> {
        self.nearest(position).map(|(_, _, distance)| distance)
    }

    /// The sample at `index`, in table order.
    #[must_use]
    pub fn sample(&self, index: usize) -> Option<&Sample> {
        self.samples.get(index)
    }

    /// Which path the sample at `index` belongs to.
    #[must_use]
    pub fn path_of(&self, index: usize) -> Option<u16> {
        self.paths.get(index).copied()
    }

    /// The widest half-width anywhere on the track, either side.
    ///
    /// A scale for "still roughly on the track" that comes from the track itself
    /// rather than from a number somebody picked.
    #[must_use]
    pub fn max_half_width(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |widest, sample| {
            widest
                .max(sample.half_width_left)
                .max(sample.half_width_right)
        })
    }
}

/// The circuit's own length in world units: the lap
/// [`oag_race::course::Course`] walks - every authored path in ring order,
/// resampled, closed back to the start.
///
/// What `Track Creation`'s `Distance(m)` row shows on the original, one
/// world unit being one metre (the HUD's own `speed * 3.6` km/h factor says
/// so). Pulse's circuits author the lap as **two open paths joined at two
/// junctions** - Talon's Junction's are 2,608 and 2,471 units long and the
/// two links between them make up the rest - so a single path's own length
/// is two thirds of a lap, and the ring is what the screen prints: `5094`,
/// `5228` and `4330` here against the `5178`, `5350` and `4419` on screen
/// for the three circuits a fresh profile offers. **That is 1.7 to 2.3 per
/// cent short, and the gap is not sampling** - the figure is the same to a
/// unit at one, four, sixteen and sixty-four steps per segment and from the
/// raw control points - so the original sums a slightly different curve
/// (the centreline rather than the racing line, most likely) or counts the
/// junction links differently. Unread. Confidence 70 that the screen's
/// number is a lap length at all in the sense measured here; this build
/// prints its own measurement rather than a stand-in, and the test pins the
/// gap at under three per cent so a change in either direction is noticed.
/// See `crates/game/tests/circuit_length_ground_truth.rs`.
///
/// # Errors
///
/// A blob that is not a `.vex`, has no `WO Track` node, whose node does not
/// parse, or whose paths do not close into a ring.
pub fn circuit_length(blob: &[u8]) -> anyhow::Result<f32> {
    use anyhow::Context;
    anyhow::ensure!(
        oag_vex::vex::has_magic(blob),
        "not a .vex file (no VEXX magic)"
    );
    let nodes = oag_vex::vex::nodes(blob).context("walking the node tree")?;
    let node = oag_vex::track::find_node(blob, &nodes).context("no WO Track node")?;
    let payload = blob
        .get(node.payload())
        .context("the WO Track payload runs past the end of the file")?;
    let ai = oag_vex::track::parse(payload).map_err(|e| anyhow::anyhow!("{e}"))?;
    let course = oag_race::course::Course::from_track(&ai, None)
        .context("the authored paths do not close into a lap")?;
    Ok(course.length())
}
