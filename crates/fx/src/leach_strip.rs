//! Wipeout HD's LeachBeam strip: the ribbon `LeachBeam_DrawStrip`
//! (`0x00153288`) builds from the target's own anchor trail.
//!
//! Recovered on RPCS3 and in a scratch interpreter, evidence and confidence per
//! claim on `docs/ghidra/functions/ps3-hdfury-eu/leach-beam-strips.md`
//! ("hd-leach-path" and "hd-leach-draw"):
//!
//! - every craft records an anchor trail, `LeachTrail_RecordAnchor`
//!   (`0x001169f8`): [`AnchorTrail`];
//! - the strip's samples are that trail walked back from the target to the
//!   shooter's progress, `BeamPath_WalkTargetHistory` (`0x00116c48`): [`walk`];
//! - then bent onto the shooter's anchor by cumulative length,
//!   `BeamPath_BendToTarget` (`0x00116090`): [`bend`];
//! - the nodes and the reveal fade, `LeachBeamStrip_BuildRibbon`
//!   (`0x001164e0`): [`nodes`], [`colour`], [`Node`];
//! - three fins like the Rocket's smoke, `RibbonBuilder_Alloc(300, 3)`:
//!   [`extend_vertices`].
//!
//! Render-side only: nothing here is simulation state or reaches a hash.
//!
//! - the wobble, `BeamPath_AddWobble` (`0x001157d0`): [`wobble`].
//!
//! **Chosen, not measured**: the progress measure is this engine's unwrapped
//! track distance (divided by the course length for the wobble) where the
//! original stores a 0..1 lap fraction with a lap fix, the same ordering; and
//! the phases start at 0 when the beam starts (the constructor zeroes them,
//! the owner's reset was not read).

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use crate::rocket_smoke::{FINS, fin_axes};

/// Records one craft's trail holds: `craft + 0x110 + i * 0x50`, `i < 300`.
pub const RING: usize = 300;

/// The newest record is replaced by a new one when the anchor has moved more
/// than this from it (`0x001169f8`, TOC `-0x37e4`).
pub const RECORD_SPACING: f32 = 3.4;

/// The anchor is the `arc_anchor_point` node's position pushed this far along
/// the node's row 1 (TOC `-0x3824`).
pub const ANCHOR_OFFSET: f32 = -1.7;

/// Samples closer than this to either end are not kept by the walk.
pub const MIN_SEPARATION: f32 = 1e-4;

/// The strip's half-width, node `+0x68`.
pub const HALF_WIDTH: f32 = 1.0;

/// `u` per world unit of strip length, node `+0x6c`.
pub const U_PER_UNIT: f32 = 0.05;

/// The up reference `RibbonNode_FromDirection` projects against.
pub const REFERENCE_UP: Vec3 = Vec3::new(0.0, -1.0, 0.0);

/// The ball's flight, state 2: `2.5 * clock` reaches 1.
pub const BALL_SECONDS: f32 = 0.4;

/// The reveal, state 3: `3.3333 * clock` reaches 1.
pub const REVEAL_SECONDS: f32 = 0.3;

/// The soft edge of the reveal in state 3 (state 4 draws with a hard edge).
pub const REVEAL_WINDOW: f32 = 0.001;

/// One record of a craft's trail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub anchor: Vec3,
    /// Distance travelled along the course, unwrapped across laps.
    pub progress: f32,
    /// The craft's body rows 0 and 1 when it was laid (`+0x10`, `+0x20`),
    /// which the wobble displaces along.
    pub row0: Vec3,
    pub row1: Vec3,
}

impl Record {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            anchor: self.anchor.lerp(other.anchor, t),
            progress: self.progress + (other.progress - self.progress) * t,
            row0: self.row0.lerp(other.row0, t),
            row1: self.row1.lerp(other.row1, t),
        }
    }
}

/// One craft's anchor trail: the newest record and up to [`RING`] - 1 before.
#[derive(Debug, Clone)]
pub struct AnchorTrail {
    records: Vec<Record>,
    head: usize,
    count: usize,
}

impl Default for AnchorTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl AnchorTrail {
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: vec![
                Record {
                    anchor: Vec3::ZERO,
                    progress: 0.0,
                    row0: Vec3::X,
                    row1: Vec3::Y,
                };
                RING
            ],
            head: 0,
            count: 0,
        }
    }

    /// `LeachTrail_RecordAnchor`: a new record when the anchor is more than
    /// [`RECORD_SPACING`] from the newest one (or none exists yet). The count
    /// saturates at `RING - 1`, as the original's does.
    pub fn record(&mut self, record: Record) {
        if self.count > 0
            && (record.anchor - self.records[self.head].anchor).length() <= RECORD_SPACING
        {
            return;
        }
        if self.count > 0 {
            self.head = (self.head + 1) % RING;
        }
        self.records[self.head] = record;
        self.count = (self.count + 1).min(RING - 1);
    }

    /// Forgets everything (a craft that respawned or was teleported).
    pub fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
    }

    /// The records, newest first.
    pub fn newest_first(&self) -> impl Iterator<Item = Record> + '_ {
        (0..self.count).map(|k| self.records[(self.head + RING - k) % RING])
    }

    /// Records held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// The anchor a craft's `arc_anchor_point` node gives: the node's position
/// pushed [`ANCHOR_OFFSET`] along its row 1.
#[must_use]
pub fn anchor_point(node_position: Vec3, node_row1: Vec3) -> Vec3 {
    node_position + node_row1 * ANCHOR_OFFSET
}

/// `BeamPath_WalkTargetHistory`: sample 0 is the walked craft's own record,
/// then its trail newest first while its progress is not below the other
/// craft's, skipping points within [`MIN_SEPARATION`] of either anchor; at the
/// first record below it one more point is the inverse lerp between the last
/// kept and that record at the other craft's progress.
///
/// The walked craft is the one **ahead**: `0x001179c8` swaps the pair by the
/// sign of a progress difference before it builds, so a beam fired at a craft
/// behind walks the shooter's own trail.
#[must_use]
pub fn walk(
    leader_trail: &AnchorTrail,
    leader: Record,
    trailer_anchor: Vec3,
    trailer_progress: f32,
) -> Vec<Record> {
    let mut samples = vec![leader];
    let mut last = leader;
    for record in leader_trail.newest_first() {
        if record.progress < trailer_progress {
            let span = last.progress - record.progress;
            if span > 0.0 {
                let t = (last.progress - trailer_progress) / span;
                samples.push(last.lerp(record, t));
            }
            return samples;
        }
        if (record.anchor - leader.anchor).length() > MIN_SEPARATION
            && (record.anchor - trailer_anchor).length() > MIN_SEPARATION
        {
            samples.push(record);
            last = record;
        }
    }
    samples
}

/// `BeamPath_BendToTarget`: each sample moves toward `shooter_anchor` by its
/// cumulative length over the total, so sample 0 stays on the target and the
/// last lands on the shooter's anchor.
pub fn bend(samples: &mut [Record], shooter_anchor: Vec3) {
    let mut cumulative = vec![0.0f32; samples.len()];
    for i in 1..samples.len() {
        cumulative[i] = cumulative[i - 1] + (samples[i].anchor - samples[i - 1].anchor).length();
    }
    let total = cumulative.last().copied().unwrap_or(0.0);
    if total <= 0.0 {
        return;
    }
    for (sample, along) in samples.iter_mut().zip(cumulative) {
        sample.anchor = sample.anchor.lerp(shooter_anchor, along / total);
    }
}

/// The wobble's three sines: `{frequency, amplitude, phase}`, the first two
/// set every call (`0x001157d0`, TOC `-0x3814..-0x3800`), the phase counting
/// up by [`WOBBLE_PHASE_STEP`] per call from the beam's start.
const WOBBLE: [(f32, f32); 3] = [(3.0, 0.3), (2.9, 0.5), (2.58, 1.2)];

/// Each phase's step per call, one call per tick (TOC `-0x3810`).
pub const WOBBLE_PHASE_STEP: f32 = 0.016_666_668;

/// Lap fractions to radians in the sines' argument (TOC `-0x37fc`).
const WOBBLE_ARC_SCALE: f32 = 150.0;

/// The three phases one call later: `BeamPath_AddWobble` adds
/// [`WOBBLE_PHASE_STEP`] to each before it uses them, once a tick.
pub fn advance_wobble(phases: &mut [f32; 3]) {
    for phase in phases.iter_mut() {
        *phase += WOBBLE_PHASE_STEP;
    }
}

/// `BeamPath_AddWobble`'s displacement, with the phases already advanced by
/// [`advance_wobble`]: moves sample `i`
/// (every sample but 0) along its row 0 by `taper * D` and along its row 1 by
/// `taper`, where `D` is the three sines summed at the lap fraction travelled
/// from the far end, and `taper` is a triangle that is 0 at both ends and 1
/// at the middle. `course_length` turns the samples' distance into the
/// original's 0..1 lap fraction.
pub fn wobble(samples: &mut [Record], phases: &[f32; 3], course_length: f32) {
    let n = samples.len();
    let length = if course_length > 0.0 {
        course_length
    } else {
        1.0
    };
    let half = n as f32 * 0.5;
    let mut arc = 0.0f32;
    for i in (1..n).rev() {
        arc += (samples[i].progress - samples[i - 1].progress) / length;
        let along = arc * WOBBLE_ARC_SCALE;
        let sum = WOBBLE
            .iter()
            .zip(phases.iter())
            .map(|(&(frequency, amplitude), &phase)| {
                ((along + phase) * frequency).sin() * amplitude
            })
            .fold(0.0f32, |total, term| total + term);
        let rise = (1.0 / half) * i as f32;
        let taper = if i > n / 2 {
            (1.0 / half) * (half - i as f32) + 1.0
        } else {
            rise
        };
        let sample = &mut samples[i];
        sample.anchor += sample.row0 * (taper * sum) + sample.row1 * taper;
    }
}

/// Where the beam is in its life, from its age in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// State 2: the ball flies, nothing of the strip is drawn.
    Ball,
    /// State 3: revealing; `reveal` goes 0 to 1.
    Revealing { reveal: f32 },
    /// State 4: held, all of it, hard edge.
    Held,
}

impl Phase {
    /// `LeachBeam_Update`'s timeline: ball [`BALL_SECONDS`], reveal
    /// [`REVEAL_SECONDS`], then held.
    #[must_use]
    pub fn at(age: f32) -> Self {
        if age < BALL_SECONDS {
            Self::Ball
        } else if age < BALL_SECONDS + REVEAL_SECONDS {
            Self::Revealing {
                reveal: (age - BALL_SECONDS) / REVEAL_SECONDS,
            }
        } else {
            Self::Held
        }
    }

    /// `(reveal, window)` for [`nodes`], or `None` while nothing draws.
    #[must_use]
    pub fn reveal(self) -> Option<(f32, f32)> {
        match self {
            Self::Ball => None,
            Self::Revealing { reveal } => Some((reveal, REVEAL_WINDOW)),
            Self::Held => Some((1.0, 0.0)),
        }
    }
}

/// The vertex colour byte of sample `index` of `count`: white where the
/// reveal has passed, black beyond it, a ramp across the window
/// (`0x001164e0`, emulated).
#[must_use]
pub fn colour(index: usize, count: usize, reveal: f32, window: f32) -> u8 {
    let frac = index as f32 / (count.max(2) - 1) as f32;
    let lo = reveal * (1.0 + window) - window;
    let hi = reveal * (1.0 + window);
    if frac <= lo {
        255
    } else if frac >= hi {
        0
    } else {
        (((hi - frac) / (hi - lo)) * 255.0) as u8
    }
}

/// One ribbon node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    pub position: Vec3,
    /// `RibbonNode_FromDirection`'s rows 0 and 1.
    pub right: Vec3,
    pub up: Vec3,
    pub u: f32,
    /// The vertex colour byte, on all four channels.
    pub shade: u8,
}

/// `RibbonNode_FromDirection`: forward is the unit direction, up the
/// reference with its forward part removed, right `up x forward`.
fn basis(forward: Vec3) -> (Vec3, Vec3) {
    let up = (REFERENCE_UP - forward * REFERENCE_UP.dot(forward)).normalize_or_zero();
    (up.cross(forward), up)
}

/// `LeachBeamStrip_BuildRibbon`'s nodes, shooter end first (`u` 0 there, the
/// last sample of the path), the target last. Empty below two samples.
#[must_use]
pub fn nodes(samples: &[Record], reveal: f32, window: f32) -> Vec<Node> {
    let samples: Vec<Vec3> = samples.iter().map(|record| record.anchor).collect();
    let count = samples.len();
    if count < 2 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(count);
    let mut u = 0.0f32;
    for index in (0..count).rev() {
        let towards = if index == 0 {
            samples[1] - samples[0]
        } else {
            samples[index] - samples[index - 1]
        };
        if index + 1 < count {
            u += U_PER_UNIT * (samples[index + 1] - samples[index]).length();
        }
        let (right, up) = basis(towards.normalize_or_zero());
        out.push(Node {
            position: samples[index],
            right,
            up,
            u,
            shade: colour(index, count, reveal, window),
        });
    }
    out
}

/// The triangle list for `nodes`: per segment per fin the pair
/// `RibbonBuilder_Flush` indexes, the same fins as the Rocket's smoke. `rgb`
/// is the vertex colour's RGB before the shade.
pub fn extend_vertices(out: &mut Vec<GpuVertex>, nodes: &[Node]) {
    let fins = |node: &Node| -> [[GpuVertex; 2]; FINS] {
        let shade = f32::from(node.shade) / 255.0;
        std::array::from_fn(|k| {
            let (side, normal) = fin_axes(k, node.right, node.up);
            let offset = side * HALF_WIDTH;
            let vertex = |position: Vec3, v: f32| GpuVertex {
                position: position.to_array(),
                normal: normal.to_array(),
                colour: [shade, shade, shade, shade],
                texcoord: [node.u, v],
                ..bytemuck::Zeroable::zeroed()
            };
            [
                vertex(node.position - offset, 1.0),
                vertex(node.position + offset, 0.0),
            ]
        })
    };
    for pair in nodes.windows(2) {
        let (a, b) = (fins(&pair[0]), fins(&pair[1]));
        for fin in 0..FINS {
            let [a0, a1] = a[fin];
            let [b0, b1] = b[fin];
            out.extend_from_slice(&[a0, a1, b0, a1, b0, b1]);
        }
    }
}

#[cfg(test)]
mod tests;
