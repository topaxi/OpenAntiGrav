//! The one-shot effect that follows its owner while it emits - see
//! [`Stage::play_riding`].
//!
//! Split out of `psys.rs`, which is past the 1,000-line rule and ratcheted.

use oag_core::math::Vec3;

use super::{Effect, Playing, Stage};

impl Stage {
    /// Fires `effect` at `point` the way [`Self::play`] does - same slot
    /// choice, never refused while any instance is unattached - but hands
    /// back a handle the caller can [`Self::follow`] until it
    /// [`Self::detach`]es it.
    ///
    /// For a **one-shot effect parented to something that moves**, which is
    /// neither of the two cases above: the absorb burst rides its locator for
    /// the third of a second its emitters run, and is then let go. Through
    /// [`Self::attach`] it was refused whenever the stage was busy - on HD a
    /// field inside each other's trails re-fires `WO_TRAIL_HITSHIP` every
    /// tick, which alone keeps fewer than [`super::RESERVED_FOR_BURSTS`] slots free -
    /// and a burst the stage refuses is a burst the player never sees. The
    /// caller owns the detach: until then the slot is held like any attached
    /// one.
    pub fn play_riding(
        &mut self,
        effect: &std::sync::Arc<Effect>,
        point: Vec3,
        scale: f32,
    ) -> Option<Playing> {
        let index = self.free_slot().or_else(|| {
            self.instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| !instance.attached)
                .min_by_key(|(_, instance)| instance.system.alive_count())
                .map(|(index, _)| index)
        })?;
        Some(self.claim(index, effect, point, scale, true))
    }

    /// Whether an attached instance's emitters are still running - see
    /// [`super::System::is_emitting`]. `false` on a stale handle.
    ///
    /// What an owner that follows a one-shot effect asks before
    /// [`Self::detach`]: once nothing emits, following moves nothing, and the
    /// slot can go back to the stage while the last particles finish.
    #[must_use]
    pub fn is_emitting(&self, playing: Playing) -> bool {
        self.instance(playing)
            .is_some_and(|instance| instance.system.is_emitting())
    }
}
