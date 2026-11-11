//! Starting an effect late: [`Stage::pre_age`].

use super::*;

impl Stage {
    /// Ages one fired effect by `steps` ticks of `dt` without drawing them: an effect a
    /// network client starts late, because a correction's replay is where it first learned of
    /// the event, begins as old as the event is. Only `playing`'s own instance moves; a handle
    /// that no longer names a fired instance does nothing.
    pub fn pre_age(&mut self, playing: Playing, steps: u32, dt: f32, rng: &mut Rng) {
        let Some(instance) = self
            .instances
            .get_mut(usize::from(playing.index))
            .filter(|i| i.generation == playing.generation && !i.attached)
        else {
            return;
        };
        let Some(effect) = instance.effect.clone() else {
            return;
        };
        for _ in 0..steps {
            instance
                .system
                .advance(&effect, dt, instance.anchor, instance.up, rng);
        }
    }

    /// [`Self::pre_age`] for an attached effect whose owner has been travelling: ages it by
    /// `steps` ticks of `dt` with its anchor walked in a straight line from where it was
    /// attached to `to`, as [`Self::follow`] and [`Self::advance`] would have over those
    /// ticks. What it put out on the way lies behind it, and a part that plays once at the
    /// start (a launch flash) is as far through as the owner is old. A handle that names no
    /// attached instance does nothing.
    pub fn pre_age_riding(
        &mut self,
        playing: Playing,
        steps: u32,
        dt: f32,
        to: Vec3,
        rng: &mut Rng,
    ) {
        let Some(instance) = self.get_mut(playing) else {
            return;
        };
        let Some(effect) = instance.effect.clone() else {
            return;
        };
        let from = instance.anchor;
        for step in 1..=steps {
            #[expect(clippy::cast_precision_loss, reason = "a few dozen steps")]
            let along = step as f32 / steps as f32;
            instance.anchor = from + (to - from) * along;
            instance
                .system
                .advance(&effect, dt, instance.anchor, instance.up, rng);
        }
        instance.anchor = to;
    }
}
