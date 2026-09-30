//! How a [`Stage`] lets go of an instance.
//!
//! The original has one function for it, `Psys_ReleaseHandle` (`0x088f3298`),
//! with two behaviours picked by its last argument (read 2026-09-30, see
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "Releasing an
//! instance by handle"):
//!
//! - **`now == 0`** sets the instance's dead bit, and the manager's next tick
//!   destroys it with every particle it still holds: [`Stage::kill`]. The
//!   LeachBeam's `WO_LEACHBEAM_ENERGY` does this, at each re-spawn
//!   (`LeachBeam_Advance`) and when the beam goes
//!   (`LeachBeam_UpdatePool`'s teardown).
//! - **`now != 0`** (`ParticleSystem_StopAndClear`) stops the emitters and
//!   frees the instance's *template* particles at once, leaving its
//!   emitters' own particles to finish their lives: [`Stage::release`]. The
//!   rocket, missile, plasma and quake releases are this one. A template can
//!   live for thousands of ticks (`WO_MISSILE_HEAD`'s `glow` for 3600,
//!   `WO_PLASMA_HEAD`'s for 65535), so a release that let them "finish"
//!   would leave a glow hanging where the projectile died.
//!
//! [`Stage::detach`] is the third case, an instance whose emitters ran out on
//! their own (an absorb burst, a hit's sparks): nothing is freed early.

use super::{Playing, Stage, System};

impl Stage {
    /// Hands an attached instance back the way the original's weapon pools do
    /// when a projectile ends: the emitters stop, the template particles go
    /// at once, the emitters' own particles finish their lives.
    ///
    /// A no-op on a stale handle.
    pub fn release(&mut self, playing: Playing) {
        if let Some(instance) = self.get_mut(playing) {
            instance.attached = false;
            instance.system.stop();
            if let Some(effect) = instance.effect.as_deref() {
                instance.system.clear_templates(effect);
            }
        }
    }

    /// Live particles across every instance playing `effect`, by identity.
    ///
    /// What a caller that owns one effect asks to see whether the instance it
    /// let go of is really gone - a [`Self::kill`] empties it, a
    /// [`Self::detach`] leaves its particles to finish.
    #[must_use]
    pub fn alive_of_effect(&self, effect: &std::sync::Arc<super::Effect>) -> usize {
        self.instances
            .iter()
            .filter(|instance| {
                instance
                    .effect
                    .as_ref()
                    .is_some_and(|held| std::sync::Arc::ptr_eq(held, effect))
            })
            .map(|instance| instance.system.alive_count())
            .sum()
    }

    /// Ends an attached instance outright: its emitters stop and every
    /// particle it holds goes with it, at once.
    ///
    /// **Chosen, not measured:** the original destroys the instance on the
    /// manager's next tick, so a frame of the old particles may still draw
    /// first; the order against the draw was not read. A no-op on a stale
    /// handle.
    pub fn kill(&mut self, playing: Playing) {
        if let Some(instance) = self.get_mut(playing) {
            instance.attached = false;
            instance.system = System::new();
        }
    }
}
