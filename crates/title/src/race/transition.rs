//! How a Zone stage change sweeps the world on a title whose transition has
//! been read out of its executable.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`.

/// How a Zone stage change reaches the world on a title whose transition is
/// read: a sphere growing out of the local craft, repainting stage `n - 1`
/// into stage `n` as it passes, beside a colour weight that ramps the rest of
/// the palette (fog, light rig) over the same period.
///
/// # What this is, recovered rather than chosen
///
/// Wipeout HD/Fury's `Environment_UpdateStageBlend` (`0x003da540`,
/// `ps3-hdfury-eu`) keeps four floats per entity beside the stage indices:
/// the sphere radius, its speed, its acceleration and the colour weight. On
/// the frame a stage commits it writes `radius = 0.1`, `speed = start speed`
/// and `weight = 0`; on every later frame, while the game is not paused, it
/// runs `radius += speed` (only while `radius < radius cap`), `speed +=
/// acceleration` and `weight += weight step` (clamped at `1.0`). Per call and
/// never against a timestep. The shader then selects the new stage's colours
/// inside the sphere and the old stage's outside it, with the sphere
/// re-centred on the local craft's position every frame.
///
/// Read statically from the instruction stream and the `.data` image on
/// 2026-09-15, then reproduced live on RPCS3 the same day at two independent
/// frame counts, to the tenth - see
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`,
/// passes thirty and thirty-one. The two schema keys that would author the
/// speed and acceleration (`Transition start speed`, `Transition
/// acceleration`) are developer-only and unauthored on the shipped disc, so
/// the `.data` defaults are what every player saw.
///
/// # Why this is an axis
///
/// 2048 has the same stage table shape and a different transition: its
/// `Zone_UpdateStage` fades the current stage toward the next by a factor
/// (`DAT_816c6bc8`) nothing traced writes, and its own `Growing Texture` key
/// group names a mechanism this one does not have. Its numbers are unread, so
/// it carries `None` rather than these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneTransition {
    /// The radius the sphere starts at on the frame a stage commits.
    pub start_radius: f32,
    /// What the sphere grows by on the first frame after the commit.
    pub start_speed: f32,
    /// What the growth itself gains every frame.
    pub acceleration: f32,
    /// Where the radius stops growing. The original tests `radius < cap`
    /// before adding, so it parks one step past this; the difference is one
    /// frame's growth at the end of a ten-second sweep.
    pub radius_cap: f32,
    /// What the colour weight gains every frame, from `0` at the commit to a
    /// clamp at `1.0`.
    pub weight_step: f32,
}

impl ZoneTransition {
    /// The sphere's radius `frames` frames after a commit, in the closed form
    /// of the per-frame advance above: `start_radius + start_speed * k +
    /// acceleration * k * (k - 1) / 2`, capped.
    ///
    /// With HD/Fury's numbers that is `0.1 + 0.5k + 0.05k(k - 1)`: `164.4` at
    /// `k = 53` and `3288.7` at `k = 252`, both read live off the running
    /// game. The cap is applied to the closed form rather than reproducing the
    /// one-step overshoot the iterated original ends on.
    #[must_use]
    pub fn radius_after(&self, frames: u32) -> f32 {
        let k = frames as f32;
        let radius =
            self.start_radius + self.start_speed * k + self.acceleration * k * (k - 1.0) / 2.0;
        radius.min(self.radius_cap)
    }

    /// The colour weight `frames` frames after a commit: `weight_step * k`,
    /// clamped at `1.0`. The new stage's own share of the palette cross-fade,
    /// `0` on the commit frame.
    #[must_use]
    pub fn weight_after(&self, frames: u32) -> f32 {
        (self.weight_step * frames as f32).min(1.0)
    }
}
