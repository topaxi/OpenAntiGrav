//! The CPU governor's side of the frame loop: what it reads each frame and which of this
//! project's own extras it turns down.
//!
//! The controller is `oag_present::governor`; this file is the glue it deliberately does not
//! have. The reading is the frame thread's CPU time without the swapchain waits (the `CPU`
//! row), the GPU time the resolution controller already reads, and the frame limiter's
//! period as the budget. The knobs are **only this project's own tiers above what a title
//! authors** (the shadow map above the title's own shadows, model detail above the authored
//! switch distances), stepped down to the authored tier and never below it. The override lives
//! in [`Session::governed_profile`], a copy handed to the draw; `settings.render_profiles` is
//! never written, so a saved row cannot drift.

use log::info;
use oag_display::display::Shadows;
use oag_mesh::mesh::ModelDetail;
use oag_present::governor::{MAX_LEVEL, Sample};

use super::Session;
use crate::settings::RenderProfile;
use crate::stage::Stage;

/// One step down: what the field is set to, in the order the governor takes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Knob {
    /// The shadow map, down to the tier a title authors (or off, where it authors none).
    Shadows(Shadows),
    /// Model detail, down one rung.
    Detail(ModelDetail),
}

impl Knob {
    fn name(self) -> String {
        match self {
            Self::Shadows(tier) => format!("shadows {tier}"),
            Self::Detail(detail) => format!("model detail {}", detail.name()),
        }
    }
}

/// The knobs a profile can give up, cheapest quality loss per CPU saved first.
///
/// `authored_shadows` is what the title draws on its own (`original`, or `off` where it
/// authors none). Nothing goes below an authored tier, and a profile already at one has no
/// step for it.
pub(super) fn steps(profile: &RenderProfile, authored_shadows: Shadows) -> Vec<Knob> {
    let mut steps = Vec::with_capacity(MAX_LEVEL);
    if profile.shadows == Shadows::Mapped {
        steps.push(Knob::Shadows(authored_shadows));
    }
    if profile.model_detail == ModelDetail::Maximum {
        steps.push(Knob::Detail(ModelDetail::High));
    }
    if profile.model_detail != ModelDetail::Original {
        steps.push(Knob::Detail(ModelDetail::Original));
    }
    steps
}

/// `profile` with the first `level` of its `steps` applied.
pub(super) fn apply(profile: &RenderProfile, steps: &[Knob], level: usize) -> RenderProfile {
    let mut governed = profile.clone();
    for knob in steps.iter().take(level) {
        match *knob {
            Knob::Shadows(tier) => governed.shadows = tier,
            Knob::Detail(detail) => governed.model_detail = detail,
        }
    }
    governed
}

impl Session {
    /// What the title authors for shadows: `original` where it is built and draws, else `off`.
    fn authored_shadows(&self) -> Shadows {
        let title = self
            .shell
            .as_ref()
            .map(|shell| shell.title)
            .or(self.race_title);
        match title {
            Some(title) if title.original_shadows.draws() => Shadows::Original,
            _ => Shadows::Off,
        }
    }

    /// The render profile the draw uses: the player's, with the CPU governor's current level
    /// applied. A copy; nothing is written back.
    pub(super) fn governed_profile(&self) -> RenderProfile {
        let profile = self.render_profile();
        let level = self.governor.level();
        if level == 0 {
            return profile;
        }
        apply(&profile, &steps(&profile, self.authored_shadows()), level)
    }

    /// One frame's CPU time (without the swapchain waits) into the governor, and one log line
    /// for each change of level.
    pub(super) fn feed_governor(&mut self, cpu_seconds: f32) {
        let racing = matches!(self.stage, Stage::Race(_));
        if !racing || !self.settings.graphics.cpu_governor {
            self.governor.reset();
            return;
        }
        let profile = self.render_profile();
        let steps = steps(&profile, self.authored_shadows());
        let sample = Sample {
            cpu: cpu_seconds,
            gpu: self.governor_gpu,
            period: self.limiter_hz().map(|hz| 1.0 / hz as f32),
        };
        let Some(change) = self.governor.record(sample, steps.len()) else {
            return;
        };
        let knob = |level: usize| steps.get(level - 1).map(|knob| knob.name());
        let (verb, what) = if change.to > change.from {
            ("down", knob(change.to))
        } else {
            ("restored", knob(change.from))
        };
        info!(
            "cpu governor: {verb} {} - cpu {:.2} ms, gpu {}, budget {:.2} ms, level {} to {}",
            what.unwrap_or_else(|| "(knobs changed)".to_string()),
            change.cpu * 1000.0,
            change
                .gpu
                .map_or_else(|| "n/a".to_string(), |g| format!("{:.2} ms", g * 1000.0)),
            change.period * 1000.0,
            change.from,
            change.to,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(shadows: Shadows, detail: ModelDetail) -> RenderProfile {
        RenderProfile {
            shadows,
            model_detail: detail,
            ..RenderProfile::default()
        }
    }

    #[test]
    fn a_profile_at_the_authored_tiers_has_nothing_to_give_up() {
        let at_rest = profile(Shadows::Original, ModelDetail::Original);
        assert!(steps(&at_rest, Shadows::Original).is_empty());
        let off = profile(Shadows::Off, ModelDetail::Original);
        assert!(steps(&off, Shadows::Original).is_empty());
    }

    #[test]
    fn extras_step_down_to_the_authored_tier_and_never_below() {
        let rich = profile(Shadows::Mapped, ModelDetail::Maximum);
        let all = steps(&rich, Shadows::Original);
        assert_eq!(
            all,
            [
                Knob::Shadows(Shadows::Original),
                Knob::Detail(ModelDetail::High),
                Knob::Detail(ModelDetail::Original),
            ]
        );
        let floor = apply(&rich, &all, 3);
        assert_eq!(floor.shadows, Shadows::Original);
        assert_eq!(floor.model_detail, ModelDetail::Original);
        let none = steps(&rich, Shadows::Off);
        assert_eq!(none[0], Knob::Shadows(Shadows::Off));
    }

    #[test]
    fn applying_a_level_leaves_the_players_profile_alone() {
        let rich = profile(Shadows::Mapped, ModelDetail::High);
        let governed = apply(&rich, &steps(&rich, Shadows::Original), 1);
        assert_eq!(governed.shadows, Shadows::Original);
        assert_eq!(rich.shadows, Shadows::Mapped);
        let same = apply(&rich, &steps(&rich, Shadows::Original), 0);
        assert_eq!((same.shadows, same.model_detail), (rich.shadows, rich.model_detail));
    }
}
