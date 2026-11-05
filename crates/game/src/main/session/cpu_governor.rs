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

use log::debug;
use oag_display::display::Shadows;
use oag_mesh::mesh::ModelDetail;
use oag_present::governor::{Governor, Kind, MAX_LEVEL, Sample};

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
    /// The id the governor remembers this knob by; stable whatever else the profile has.
    fn id(self) -> usize {
        match self {
            Self::Shadows(_) => 0,
            Self::Detail(ModelDetail::High) => 1,
            Self::Detail(_) => 2,
        }
    }

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

/// The knobs the governor holds down: the first `level` of `steps` that it has not skipped.
pub(super) fn down<'a>(
    steps: &'a [Knob],
    governor: &'a Governor,
) -> impl Iterator<Item = Knob> + 'a {
    steps
        .iter()
        .copied()
        .filter(|knob| !governor.is_skipped(knob.id()))
        .take(governor.level())
}

/// `profile` with the `down` knobs applied.
pub(super) fn apply(profile: &RenderProfile, down: impl IntoIterator<Item = Knob>) -> RenderProfile {
    let mut governed = profile.clone();
    for knob in down {
        match knob {
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
        let steps = steps(&profile, self.authored_shadows());
        apply(&profile, down(&steps, &self.governor))
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
            gpu: self.governor_gpu_fresh(),
            period: self.held_hz().map(|hz| 1.0 / hz as f32),
        };
        let mut ids = [0; MAX_LEVEL];
        for (slot, knob) in ids.iter_mut().zip(&steps) {
            *slot = knob.id();
        }
        let change = self.governor.record(sample, &ids[..steps.len().min(MAX_LEVEL)]);
        let name = |id: Option<usize>| {
            id.and_then(|id| steps.iter().find(|knob| knob.id() == id))
                .map_or_else(|| "(knobs changed)".to_string(), |knob| knob.name())
        };
        if let Some(reading) = self.governor.take_reading() {
            debug!(
                "cpu governor: {} saved {:.2} ms",
                name(Some(reading.knob)),
                reading.saving * 1000.0
            );
        }
        let Some(change) = change else {
            return;
        };
        let verb = match change.kind {
            Kind::Shed => "down",
            Kind::Restore => "restored",
            Kind::Undo => "undone, it saved too little:",
            Kind::Drop => "dropped",
        };
        debug!(
            "cpu governor: {verb} {} - cpu {:.2} ms, gpu {}, budget {:.2} ms, level {} to {}",
            name(change.knob),
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
        let floor = apply(&rich, all.iter().copied());
        assert_eq!(floor.shadows, Shadows::Original);
        assert_eq!(floor.model_detail, ModelDetail::Original);
        let none = steps(&rich, Shadows::Off);
        assert_eq!(none[0], Knob::Shadows(Shadows::Off));
    }

    #[test]
    fn applying_a_level_leaves_the_players_profile_alone() {
        let rich = profile(Shadows::Mapped, ModelDetail::High);
        let governed = apply(&rich, steps(&rich, Shadows::Original).into_iter().take(1));
        assert_eq!(governed.shadows, Shadows::Original);
        assert_eq!(rich.shadows, Shadows::Mapped);
        let same = apply(&rich, []);
        assert_eq!((same.shadows, same.model_detail), (rich.shadows, rich.model_detail));
    }

    #[test]
    fn a_skipped_knob_is_left_out_of_the_knobs_held_down() {
        let rich = profile(Shadows::Mapped, ModelDetail::Maximum);
        let all = steps(&rich, Shadows::Original);
        let ids: Vec<_> = all.iter().map(|knob| knob.id()).collect();
        assert_eq!(ids, [0, 1, 2]);
        // Model detail `high` saves nothing here; the others save 0.3 of a 60 Hz period.
        let period = 1.0 / 60.0;
        let mut governor = Governor::new();
        for _ in 0..3600 {
            let held: Vec<_> = down(&all, &governor).collect();
            let saved = held.iter().filter(|k| k.id() != 1).count() as f32 * 0.3;
            let sample = Sample {
                cpu: (1.7 - saved) * period,
                gpu: Some(0.3 * period),
                period: Some(period),
            };
            governor.record(sample, &ids);
        }
        assert!(governor.is_skipped(1));
        assert_eq!(
            down(&all, &governor).collect::<Vec<_>>(),
            [Knob::Shadows(Shadows::Original), Knob::Detail(ModelDetail::Original)]
        );
        let governed = apply(&rich, down(&all, &governor));
        assert_eq!(governed.shadows, Shadows::Original);
        assert_eq!(governed.model_detail, ModelDetail::Original);
    }
}
