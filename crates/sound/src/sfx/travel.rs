//! [`TravelVoices`]: one weapon's held travel loop, per **projectile** slot.
//!
//! Generalised from the shape [`super::Cue::PlasmaTravel`] first built into
//! `Audio::race_tick`, when the Rocket, Missile and Shuriken each needed a copy.
//! A bolt is neither one craft nor the whole race, several can be in the air, and
//! a slot index is the only stable handle a `Copy` world snapshot gives one
//! (see [`super::SfxVoices::plasma_travel`]).
//!
//! Never pushed through the cue queue: every travel cue's [`super::Cue::held`] is
//! `true`, so a stray push is dropped, and this is driven off the world's
//! projectile array each tick.

use oag_audio::{Emitter, Listener, Mixer, Play, VoiceId};
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_weapons::projectile::{MAX_PROJECTILES, Projectile};

use super::{Banks, Cue};

/// One weapon's held voices, one per projectile slot, `None` where it does not
/// occupy the slot.
#[derive(Debug, Clone, Copy)]
pub(super) struct TravelVoices {
    voices: [Option<VoiceId>; MAX_PROJECTILES],
}

impl TravelVoices {
    /// No voices open yet.
    pub(super) fn new() -> Self {
        Self {
            voices: [None; MAX_PROJECTILES],
        }
    }

    /// One tick: starts, follows or stops every slot's voice.
    /// One tick: starts, follows or stops every slot's voice.
    ///
    /// `flying` decides which slots sound (`kind == <weapon>`, plus
    /// `charge <= 0.0` for the Plasma). `position` says where a slot is heard
    /// from; `None` stops the voice the tick a bolt ends. `radius` is
    /// [`Cue::radius`], read once by the caller.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn tick(
        &mut self,
        mixer: &mut Mixer,
        banks: &Banks,
        rng: &mut Rng,
        listener: &Listener,
        projectiles: &[Projectile; MAX_PROJECTILES],
        cue: Cue,
        radius: f32,
        flying: impl Fn(&Projectile) -> bool,
        position: impl Fn(&Projectile) -> Option<Vec3>,
    ) {
        for (slot, projectile) in projectiles.iter().enumerate() {
            let at = flying(projectile).then(|| position(projectile)).flatten();
            self.follow(slot, at, mixer, banks, rng, listener, cue, radius);
        }
    }

    /// One slot's voice for one tick: `at` is where it is heard from, [`None`]
    /// when the thing it follows has gone. The body of [`Self::tick`]'s loop, so
    /// a non-projectile (the Quake's wave) can share it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn follow(
        &mut self,
        slot: usize,
        at: Option<Vec3>,
        mixer: &mut Mixer,
        banks: &Banks,
        rng: &mut Rng,
        listener: &Listener,
        cue: Cue,
        radius: f32,
    ) {
        match (at, self.voices[slot]) {
            // The rising edge: this slot started flying (or came into range) this tick.
            (Some(at), None) => {
                if let Some((sound, looping)) = banks.pick(cue, rng)
                    && looping
                {
                    let placed = Emitter {
                        position: at.to_array(),
                        radius,
                        cone: None,
                    }
                    .place(listener, 1.0);
                    let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));
                    self.voices[slot] = mixer.play(Play {
                        gain,
                        pan,
                        ..Play::looping(sound, cue.bus())
                    });
                }
                // Nothing loaded, or the bank says it is not a loop: the guard of
                // `Engine::tick` for `~ENGINE`.
            }
            // Still flying and still held: follow it.
            (Some(at), Some(id)) if mixer.is_playing(id) => {
                let placed = Emitter {
                    position: at.to_array(),
                    radius,
                    cone: None,
                }
                .place(listener, 1.0);
                let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));
                mixer.set_gain(id, gain);
                mixer.set_pan(id, pan);
            }
            // The pool reclaimed the voice before the bolt ended (starved, not
            // stopped): forget the stale handle so a later tick does not stop
            // whatever slot the pool gave it next.
            (Some(_), Some(_)) => self.voices[slot] = None,
            // The falling edge: no longer flying, or nowhere to place it.
            (None, Some(id)) => {
                mixer.stop(id);
                self.voices[slot] = None;
            }
            (None, None) => {}
        }
    }

    /// Releases every held voice, for [`super::Audio::stop_race_sfx`].
    pub(super) fn stop_all(&mut self, mixer: &mut Mixer) {
        for voice in &mut self.voices {
            if let Some(id) = voice.take() {
                mixer.stop(id);
            }
        }
    }
}
