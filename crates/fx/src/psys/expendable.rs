//! Effects that give way: [`Stage::play_expendable`].
//!
//! A one-shot that a caller re-fires every tick for as long as a condition holds (a spark burst
//! on a craft flying in a trail) keeps tens of instances alive at once. Left as ordinary plays
//! they fill the stage and every other effect's recycling pass lands on whatever is thinnest,
//! which is a detonation or a pickup burst. An expendable instance is the first thing any other
//! play takes a slot from, and it never takes one from anything else.
//!
//! Split out of `psys.rs`, which is at the 1,500-line ceiling.

use oag_core::math::Vec3;

use super::{Effect, Playing, Stage};

impl Stage {
    /// [`Self::play`] for an effect that must never cost another effect its slot.
    ///
    /// Takes a free instance, else recycles the expendable one with the fewest live particles,
    /// else returns `None` and plays nothing. Any other play, and [`Self::attach`], reclaims
    /// an expendable instance before it touches anything else.
    pub fn play_expendable(
        &mut self,
        effect: &std::sync::Arc<Effect>,
        point: Vec3,
        scale: f32,
    ) -> Option<Playing> {
        let index = self.free_slot().or_else(|| self.victim(true))?;
        let playing = self.claim(index, effect, point, scale, false);
        self.instances[index].expendable = true;
        Some(playing)
    }

    /// The instance a play recycles when nothing is free: unattached, not a running held
    /// surface, expendable ones first and then the fewest live particles. With `expendable_only`
    /// an ordinary instance is never a candidate.
    pub(super) fn victim(&self, expendable_only: bool) -> Option<usize> {
        self.instances
            .iter()
            .enumerate()
            .filter(|(_, instance)| {
                !instance.attached
                    && !(instance.held && instance.system.is_running())
                    && (instance.expendable || !expendable_only)
            })
            .min_by_key(|(_, instance)| (!instance.expendable, instance.system.alive_count()))
            .map(|(index, _)| index)
    }

    /// Busy instances a play may take from at no cost to another effect.
    pub(super) fn expendable_count(&self) -> usize {
        self.instances
            .iter()
            .filter(|instance| instance.expendable && instance.busy())
            .count()
    }

    /// Live particles of `effect` across the stage.
    #[must_use]
    pub fn alive_count_of(&self, effect: &std::sync::Arc<Effect>) -> usize {
        self.instances
            .iter()
            .filter(|instance| {
                instance
                    .effect
                    .as_ref()
                    .is_some_and(|playing| std::sync::Arc::ptr_eq(playing, effect))
            })
            .map(|instance| instance.system.alive_count())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use oag_core::Rng;

    use super::super::tests::{DT, effect};
    use super::super::{MAX_INSTANCES, RESERVED_FOR_BURSTS};
    use super::*;

    /// A stage whose every instance is an expendable `spark` that has run `ticks` ticks.
    fn full_of_sparks(spark: &std::sync::Arc<Effect>, ticks: usize) -> Stage {
        let mut stage = Stage::new();
        let mut rng = Rng::new(5);
        for _ in 0..ticks {
            stage.advance(DT, &mut rng);
            spark_burst(&mut stage, spark);
        }
        stage
    }

    /// Three sparks in one tick: a spark lives about fourteen, so the stage fills within ten.
    fn spark_burst(stage: &mut Stage, spark: &std::sync::Arc<Effect>) {
        for _ in 0..3 {
            stage.play_expendable(spark, Vec3::ZERO, 1.0);
        }
    }

    /// Expendable plays recycle each other and nothing else: with the stage full of another
    /// effect the call plays nothing, and the other effect stays whole.
    #[test]
    fn an_expendable_play_never_takes_a_slot_from_another_effect() {
        let (burst, spark) = (effect("burst", false, 4.0), effect("spark", false, 4.0));
        let mut stage = Stage::new();
        for _ in 0..MAX_INSTANCES {
            stage.play(&burst, Vec3::ZERO, 1.0).expect("a free slot");
        }
        let before = stage.alive_count_of(&burst);
        assert!(stage.play_expendable(&spark, Vec3::ZERO, 1.0).is_none());
        assert_eq!(stage.alive_count_of(&burst), before);
        assert_eq!(stage.alive_count_of(&spark), 0);
    }

    #[test]
    fn expendable_plays_fill_the_stage_and_recycle_each_other() {
        let spark = effect("spark", false, 4.0);
        let stage = full_of_sparks(&spark, MAX_INSTANCES * 3);
        assert_eq!(stage.playing_count(), MAX_INSTANCES);
        assert!(stage.alive_count_of(&spark) > 0);
    }

    /// An ordinary play takes an expendable instance even when an ordinary one is thinner, so
    /// an older detonation that has thinned out is not the one cut short.
    #[test]
    fn an_ordinary_play_recycles_an_expendable_instance_before_a_thinner_ordinary_one() {
        let (burst, spark, next) = (
            effect("burst", false, 1.0),
            effect("spark", false, 4.0),
            effect("next", false, 4.0),
        );
        let mut rng = Rng::new(9);
        let mut stage = Stage::new();
        stage.play(&burst, Vec3::ZERO, 1.0).expect("old burst");
        // The old burst has one particle; the sparks that follow have more by the time the
        // stage is full, so the thinnest instance is the ordinary one.
        for _ in 0..5 {
            stage.advance(DT, &mut rng);
            for _ in 0..10 {
                stage.play_expendable(&spark, Vec3::ZERO, 1.0);
            }
        }
        assert_eq!(stage.playing_count(), MAX_INSTANCES);
        let kept = stage.alive_count_of(&burst);
        assert!(kept > 0, "the fixture lost the old burst on its own");
        stage.play(&next, Vec3::ZERO, 1.0).expect("a recycled slot");
        assert_eq!(
            stage.alive_count_of(&burst),
            kept,
            "a spark was kept over the burst"
        );
        stage.advance(DT, &mut rng);
        assert!(stage.alive_count_of(&next) > 0);
    }

    /// A flare must attach while expendable plays hold every instance: they count as free.
    #[test]
    fn attach_reclaims_expendable_instances() {
        let (spark, forever) = (effect("spark", false, 4.0), effect("forever", true, 10.0));
        let mut stage = full_of_sparks(&spark, MAX_INSTANCES * 3);
        let mut attached = Vec::new();
        while let Some(handle) = stage.attach(&forever, Vec3::ZERO, 1.0) {
            attached.push(handle);
        }
        assert_eq!(attached.len(), MAX_INSTANCES - RESERVED_FOR_BURSTS);
        assert!(stage.play(&spark, Vec3::ZERO, 1.0).is_some());
    }
}
