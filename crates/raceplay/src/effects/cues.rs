//! What a race says out loud: the cue queue, the two announcement queues, and
//! the banks that play them.
//!
//! Split out of `race/effects.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.
//!
//! It is the seam the `RaceSim`/`RaceView` split made obvious. A cue is a
//! per-tick **output** of the simulation (ADR-0018) and lives on
//! [`RaceSim`](super::super::RaceSim); the bank that turns it into a sample is
//! loaded art and lives on [`RaceView`](super::super::RaceView). Every method
//! here is one half reaching the other, which is why they stay on `Race`.

use super::*;

impl Race {
    /// Raises `.COLLISIONS` for one craft, on that craft's own re-arm.
    ///
    /// **The timer is per craft**, because the original's gate is: the cooldown
    /// lives on the effect the *craft* owns, so eight hulls scraping eight
    /// walls make eight sounds rather than sharing one 0.8-second window.
    ///
    /// It cannot share [`RaceView::sparks_cooldown`] either: a shielded contact
    /// never ignites, so that timer would sit at zero.
    ///
    /// `ShipCollisionFx_Trigger` plays `"COLLISIONS"` once per call that gets
    /// past its own 0.8-second gate, which is the same 0.8 seconds
    /// `oag_fx::sparks::COLLISION_COOLDOWN` carries - one constant in the
    /// original, read twice here.
    ///
    /// **A shielded contact is silent, and this used to raise `ABSORB` on
    /// it.** The contact loop skips `Ship_DispatchCollisionFx` - and with it
    /// the cue - behind the shield bit, and takes `ShipShield_Hit` instead,
    /// which writes four colour words and a timer and plays nothing
    /// (`shield-pickup.md`, gates `0x0884255c`/`0x08842684`). `ABSORB` is
    /// `Ship_PlayAbsorbFeedback` (`0x08840640`), whose four callers are all
    /// read as of 2026-09-16 and none is a contact: the absorb handler, the
    /// Eliminator's lap refill, and two network callbacks. The shell's own
    /// bulge is the whole of what a shielded contact shows or sounds. See
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    pub(crate) fn raise_contact_cue(&mut self, slot: usize, impact: bool, shielded: bool) {
        let Some(cooldown) = self.sim.contact_cue_cooldown.get_mut(slot) else {
            return;
        };
        *cooldown = (*cooldown - self.sim.dt).max(0.0);
        if !impact || shielded || *cooldown > 0.0 {
            return;
        }
        *cooldown = oag_fx::sparks::COLLISION_COOLDOWN;
        self.sim.cues.push(oag_sound::sfx::CueEvent::new(
            oag_sound::sfx::Cue::Collision,
            slot,
        ));
    }

    /// Takes the one-shot sound cues this tick raised, leaving the queue empty.
    ///
    /// The seam ADR-0018 asks for: cues are a per-tick *output*, so the caller
    /// that owns the mixer drains them **inside the fixed-step loop**, next to
    /// the tick that raised them. Draining once per frame instead would
    /// coalesce or duplicate them on a frame that stepped twice or none.
    ///
    /// A caller with no audio need never call this. The queue is one `Vec` and
    /// a race that is never drained grows it by at most a handful of entries a
    /// second, all of them fixed-size - but the composition root does drain it,
    /// and a headless test that does not is bounded by its own tick count.
    pub fn drain_cues(&mut self) -> Vec<oag_sound::sfx::CueEvent> {
        std::mem::take(&mut self.sim.cues)
    }

    /// The cues raised so far and not yet drained, for tests.
    #[must_use]
    pub fn pending_cues(&self) -> &[oag_sound::sfx::CueEvent] {
        &self.sim.cues
    }

    /// The decoded sound banks this race loaded.
    #[must_use]
    pub fn sounds(&self) -> &oag_sound::sfx::Banks {
        &self.view.sounds
    }

    /// The circuit's own authored sound emitters, as this race loaded them.
    #[must_use]
    pub fn track_emitters(&self) -> &oag_sound::sfx::TrackEmitters {
        &self.view.track_emitters
    }

    /// The decoded Zone milestone announcer this race loaded.
    #[must_use]
    pub fn announcer(&self) -> &oag_sound::sfx::Announcer {
        &self.view.announcer
    }

    /// Raises a Zone milestone announcement, to be drained the same tick.
    ///
    /// Whether `milestone` actually names a loaded cue is
    /// [`Self::announcer`]'s question, not this call's - see
    /// [`Self::drain_announcements`] for why the split is the same one
    /// [`Self::raise_contact_cue`] and [`Banks::pick`](oag_sound::sfx::Banks::pick)
    /// already keep.
    pub(crate) fn push_announcement(&mut self, milestone: u16) {
        self.sim.announcements.push(milestone);
    }

    /// Takes the Zone milestone numbers this tick raised, leaving the queue
    /// empty. Same per-tick-output shape as [`Self::drain_cues`].
    pub fn drain_announcements(&mut self) -> Vec<u16> {
        std::mem::take(&mut self.sim.announcements)
    }

    /// The decoded Zone speed-class announcer this race loaded.
    #[must_use]
    pub fn class_announcer(&self) -> &oag_sound::sfx::ClassAnnouncer {
        &self.view.class_announcer
    }

    /// Raises a Zone speed-class announcement, to be drained the same tick.
    ///
    /// Whether `stage` actually names a loaded cue is
    /// [`Self::class_announcer`]'s question, not this call's - same split as
    /// [`Self::push_announcement`].
    pub(crate) fn push_class_announcement(&mut self, stage: u32) {
        self.sim.class_announcements.push(stage);
    }

    /// Takes the speed-class stages this tick raised, leaving the queue
    /// empty. Same per-tick-output shape as [`Self::drain_announcements`].
    pub fn drain_class_announcements(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.sim.class_announcements)
    }
}
