//! What a race says out loud: the cue queue, the two announcement queues, and
//! the banks that play them.
//!
//! Out of `effects` because the queues and the contact cue's re-arm are the
//! simulation's and run in a build with no view; the bank accessors are the
//! view's and only exist with it.
//!
//! It is the seam the `RaceSim`/`RaceView` split made obvious. A cue is a
//! per-tick **output** of the simulation (ADR-0018) and lives on
//! [`RaceSim`](super::RaceSim); the bank that turns it into a sample is
//! loaded art and lives on [`RaceView`](super::RaceView). Every method
//! here is one half reaching the other, which is why they stay on `Race`.

use super::*;
use oag_gameplay::cue::{ContactCue, ContactSurface, Cue, CueEvent};
use oag_title::contact_cue::{ContactCues, ImpulseLaw};

/// One craft's contact bookkeeping under [`ContactCues::Impulse`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ContactImpulse {
    /// This tick's largest contact impulse so far; zeroed once the cues are decided.
    pub impulse: f32,
    /// Whether that contact was another craft.
    pub craft: bool,
    /// `dot(craft forward, contact normal)` of that contact; 0 for a craft pair.
    pub dot: f32,
    /// Ticks until a collision cue may start again.
    pub hit_wait: u8,
    /// Ticks until a grind cue may start again.
    pub grind_wait: u8,
}

/// What one craft's contact is heard as this tick: the collision cue and the grind cue, either,
/// both or neither
/// (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`, "`Ship_UpdateCollisionCues`").
///
/// The wait is tested before the impulse, so a waiting craft only counts down; the two blocks
/// are independent and may both start on one tick. Another craft than a person's never grinds
/// and plays `.COLLISIONS` whatever its Shield. Clears the tick's contact once decided.
pub(crate) fn decide_contact_cues(
    state: &mut ContactImpulse,
    law: &ImpulseLaw,
    person: bool,
    shield: bool,
) -> [Option<(Cue, ContactCue)>; 2] {
    let p = state.impulse;
    let nose_on = state.dot.abs() > law.nose_on_dot;
    let surface = if state.craft {
        ContactSurface::Craft
    } else {
        ContactSurface::Track
    };
    let mut heard = [None, None];

    if state.hit_wait > 0 {
        state.hit_wait -= 1;
    } else if p > law.hit_floor && (nose_on || p > law.hit_floor_side) {
        state.hit_wait = law.hit_wait;
        let severity = if p > law.hit_large {
            2
        } else if p > law.hit_medium {
            1
        } else {
            0
        };
        let cue = if person && shield {
            Cue::CollisionShield
        } else {
            Cue::Collision
        };
        let volume = (p * law.hit_volume_per_impulse + law.hit_volume_base).min(1.0);
        heard[0] = Some((
            cue,
            ContactCue {
                severity,
                surface,
                volume,
            },
        ));
    }

    if person {
        if state.grind_wait > 0 {
            state.grind_wait -= 1;
        } else if p > law.grind_floor && !nose_on {
            state.grind_wait = law.grind_wait;
            let severity = if p > law.grind_large {
                2
            } else if p > law.grind_medium {
                1
            } else {
                0
            };
            let cue = if shield { Cue::GrindShield } else { Cue::Grind };
            let volume = (p * law.grind_volume_per_impulse + law.grind_volume_base).min(1.0);
            heard[1] = Some((
                cue,
                ContactCue {
                    severity,
                    surface,
                    volume,
                },
            ));
        }
    }

    state.impulse = 0.0;
    state.craft = false;
    state.dot = 0.0;
    heard
}

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
    /// `oag_gameplay::cue::COLLISION_COOLDOWN` carries - one constant in the
    /// original, read twice here.
    ///
    /// **A shielded contact is silent: no `ABSORB`.** The contact loop skips `Ship_DispatchCollisionFx` - and with it
    /// the cue - behind the shield bit, and takes `ShipShield_Hit` instead,
    /// which writes four colour words and a timer and plays nothing
    /// (`shield-pickup.md`, gates `0x0884255c`/`0x08842684`). `ABSORB` is
    /// `Ship_PlayAbsorbFeedback` (`0x08840640`), whose four callers are all
    /// read and none is a contact: the absorb handler, the
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
        *cooldown = oag_gameplay::cue::COLLISION_COOLDOWN;
        self.sim.cues.push(oag_gameplay::cue::CueEvent::new(
            oag_gameplay::cue::Cue::Collision,
            slot,
        ));
    }

    /// `slot`'s wall contact this tick as its sound: the title's re-armed `.COLLISIONS`
    /// ([`Self::raise_contact_cue`]) or, under the impulse law, the frame's largest contact
    /// impulse and the angle of its normal to the craft's forward axis, kept for
    /// [`Self::raise_impulse_cues`] (`Ship_UpdateEntity`'s ring walk, collision-cues.md,
    /// "Who fills them: the ring walk").
    pub(crate) fn raise_wall_cue(
        &mut self,
        slot: usize,
        wall: &oag_physics::wall::WallResponse,
        shielded: bool,
    ) {
        if matches!(self.sim.contact_cues, ContactCues::Rearmed) {
            self.raise_contact_cue(slot, wall.impact, shielded);
        } else if wall.impulse_largest > 0.0 {
            let forward = self.sim.world.ships[slot].physics.body.forward();
            self.note_contact_impulse(
                slot,
                wall.impulse_largest,
                false,
                forward.dot(wall.impulse_largest_normal),
            );
        }
    }

    /// Keeps `slot`'s largest contact of the tick under [`ContactCues::Impulse`]: a contact
    /// replaces the kept one only when strictly larger, as the ring walk's `fcmpu`/`blt`
    /// (`0x000ec514`) does.
    pub(crate) fn note_contact_impulse(
        &mut self,
        slot: usize,
        impulse: f32,
        craft: bool,
        dot: f32,
    ) {
        if !matches!(self.sim.contact_cues, ContactCues::Impulse(_)) {
            return;
        }
        let Some(state) = self.sim.contact_impulse.get_mut(slot) else {
            return;
        };
        if impulse > state.impulse {
            state.impulse = impulse;
            state.craft = craft;
            state.dot = dot;
        }
    }

    /// Decides every craft's collision and grind cues from the contacts of the tick, under
    /// [`ContactCues::Impulse`]; nothing under [`ContactCues::Rearmed`].
    ///
    /// After every contact of the tick has been noted: the craft pairs
    /// ([`Self::resolve_craft_pairs`]) and the wall contacts of every craft.
    pub(crate) fn raise_impulse_cues(&mut self) {
        let ContactCues::Impulse(law) = self.sim.contact_cues else {
            return;
        };
        for slot in 0..self.sim.world.ship_count as usize {
            if !self.sim.world.ships[slot].in_world() {
                // Nothing a craft out of the world touched is heard when it returns.
                let state = &mut self.sim.contact_impulse[slot];
                (state.impulse, state.craft, state.dot) = (0.0, false, 0.0);
                continue;
            }
            let person = self.sim.world.human_slots().any(|human| human == slot);
            let shield = self.sim.world.ships[slot].physics.shield_pickup_timer > 0.0;
            let heard = decide_contact_cues(
                &mut self.sim.contact_impulse[slot],
                &law.value,
                person,
                shield,
            );
            for (cue, contact) in heard.into_iter().flatten() {
                self.sim.cues.push(CueEvent::contact(cue, slot, contact));
            }
        }
    }

    /// Records `slot`'s wall contact for the pad: its size and which side the
    /// wall is on. **Chosen, not measured**: the original's contact measure
    /// (`ship+0x5f78` on HD, `+0x64d4` on Omega) is unread, so this stands in
    /// with the impulse the response applied
    /// (`docs/gameplay/rumble.md`, "Contact and landing").
    pub(crate) fn note_wall_contact(
        &mut self,
        slot: usize,
        wall: &oag_physics::wall::WallResponse,
    ) {
        let Some(reading) = self.sim.wall_contact.get_mut(slot) else {
            return;
        };
        *reading = wall.resolved.map(|contact| {
            let ship = &self.sim.world.ships[slot].physics.body;
            let wall_on_left = contact.normal.dot(ship.right()) > 0.0;
            crate::sim::WallContactReading {
                size: wall.impulse_sum,
                side: if wall_on_left { -1.0 } else { 1.0 },
            }
        });
    }

    /// `slot`'s wall contact this tick, for the pad.
    #[must_use]
    pub fn wall_contact_of(&self, slot: usize) -> Option<crate::sim::WallContactReading> {
        self.sim.wall_contact.get(slot).copied().flatten()
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
    pub fn drain_cues(&mut self) -> Vec<oag_gameplay::cue::CueEvent> {
        std::mem::take(&mut self.sim.cues)
    }

    /// The mods' own sounds asked for since the last drain (`play-mod-sound`), oldest
    /// first. Like [`Self::drain_cues`], an output the composition root drains.
    pub fn drain_mod_sounds(&mut self) -> Vec<oag_gameplay::cue::ModSoundEvent> {
        std::mem::take(&mut self.sim.mod_sounds)
    }

    /// The loops of the mods' own sounds that are wanted now (`play-mod-sound`'s `Loop` with
    /// no `Stop` since): what plays is brought to this each tick, so a start or a stop raised
    /// in a tick that ran silently ([`Self::tick_sim_only`]) is still heard.
    #[must_use]
    pub fn mod_sound_loops(&self) -> Vec<oag_gameplay::cue::ModSoundLoop> {
        self.sim.extensions.sound_loops()
    }

    /// The cues raised so far and not yet drained, for tests.
    #[must_use]
    pub fn pending_cues(&self) -> &[oag_gameplay::cue::CueEvent] {
        &self.sim.cues
    }

    #[cfg(feature = "view")]
    /// The decoded sound banks this race loaded.
    #[must_use]
    pub fn sounds(&self) -> &oag_sound::sfx::Banks {
        &self.view.sounds
    }

    #[cfg(feature = "view")]
    /// The circuit's own authored sound emitters, as this race loaded them.
    #[must_use]
    pub fn track_emitters(&self) -> &oag_sound::sfx::TrackEmitters {
        &self.view.track_emitters
    }

    #[cfg(feature = "view")]
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

    /// Whether a locked Leech beam is fastened to `slot`, at either end. The
    /// level the pad's `leeching_energy_rumble` holds on; which end the original
    /// rumbles is unread (`docs/gameplay/rumble.md`, "Triggers").
    #[must_use]
    pub fn leach_beam_on(&self, slot: usize) -> bool {
        self.sim.world.leach_beams_iter().any(|beam| {
            beam.kind == oag_weapons::projectile::leach_beam::Kind::Locked
                && (usize::from(beam.owner) == slot || usize::from(beam.target) == slot)
        })
    }

    #[cfg(feature = "view")]
    /// The rumble patterns this race's title ships, for the pad.
    #[must_use]
    pub fn rumble_library(&self) -> &oag_tables::rumble::Library {
        &self.view.rumble
    }

    #[cfg(feature = "view")]
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
