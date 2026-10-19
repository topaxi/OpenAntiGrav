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
}
