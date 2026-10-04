//! The alternate paths off the ring: where a Repulser's third wave runs.
//!
//! [`super::Course`] walks the primary chain only. A junction whose `next[1]` is
//! set splits the track: `AiTrack_StepForward` (`0x0887e174`) returns `1` there
//! and `Repulser_AdvanceWave` (`0x08876914`) sends a third wave down the
//! alternate path (`docs/ghidra/functions/psp-pulse-usa/repulser.md`, "The fork
//! wave, read"). A [`Branch`] is that alternate path, sampled exactly as the
//! ring is, with the two ring indices where it leaves and rejoins.

use oag_core::math::Vec3;
use oag_vex::track::AiTrack;

/// One alternate path, and where it meets the ring at either end.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    /// The ring index of the first sample after the split, on the primary path:
    /// a forward wave that steps onto it from before crosses the split.
    pub split: usize,
    /// The ring index of the first sample after the merge, where the branch's
    /// own exit junction hands on to the primary chain again.
    pub merge: usize,
    /// The midpoint of the track's edges at each sample, in path order.
    pub centres: Vec<Vec3>,
    /// The AI corridor's width at each sample, parallel to [`Self::centres`].
    pub corridor_widths: Vec<f32>,
}

impl Branch {
    /// How many samples the branch holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.centres.len()
    }

    /// Whether the branch has no samples (never, for one [`super::Course`]
    /// builds).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.centres.is_empty()
    }
}

/// One sampled point: position, edge midpoint, corridor width, half-widths.
pub(super) struct Sample {
    pub(super) pos: Vec3,
    pub(super) centre: Vec3,
    pub(super) corridor_width: f32,
    pub(super) half_widths: (f32, f32),
}

/// Every sample of `path`, [`super::Course::STEPS_PER_SEGMENT`] to a
/// control-point interval - the one sampling the ring and every branch share.
pub(super) fn sample_path(path: &oag_vex::track::Path) -> Vec<Sample> {
    let steps = super::Course::STEPS_PER_SEGMENT;
    let mut out = Vec::new();
    for segment in 0..path.points.len() {
        for step in 0..steps {
            let t = step as f32 / steps as f32;
            if let Some(sample) = path.sample(segment, t) {
                let pos = Vec3::from_array(sample.pos);
                let lateral = Vec3::from_array(sample.lateral);
                let left = pos - lateral * sample.half_width_left;
                let right = pos + lateral * sample.half_width_right;
                out.push(Sample {
                    pos,
                    centre: (left + right) * 0.5,
                    corridor_width: sample.ai_bound_right - sample.ai_bound_left,
                    half_widths: (sample.half_width_left, sample.half_width_right),
                });
            }
        }
    }
    out
}

/// Every alternate path that leaves the ring at one junction and rejoins it at
/// another, in junction order. `first` is each path's first ring index, `None`
/// for a path off the ring.
pub(super) fn branches(ai: &AiTrack, first: &[Option<usize>]) -> Vec<Branch> {
    let mut out = Vec::new();
    for junction in &ai.junctions {
        let (Some(primary), Some(alternate)) = (junction.next[0], junction.next[1]) else {
            continue;
        };
        if first.get(alternate).copied().flatten().is_some() {
            continue;
        }
        let Some(split) = first.get(primary).copied().flatten() else {
            continue;
        };
        let Some(path) = ai.paths.get(alternate) else {
            continue;
        };
        let Some(merge) = path
            .exit
            .and_then(|exit| ai.junctions.get(exit))
            .and_then(|exit| exit.next[0])
            .and_then(|next| first.get(next).copied().flatten())
        else {
            continue;
        };
        let samples = sample_path(path);
        if samples.is_empty() {
            continue;
        }
        out.push(Branch {
            split,
            merge,
            centres: samples.iter().map(|s| s.centre).collect(),
            corridor_widths: samples.iter().map(|s| s.corridor_width).collect(),
        });
    }
    out
}
