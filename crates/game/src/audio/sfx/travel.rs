//! [`TravelVoices`]: one weapon's own held travel loop, per **projectile**
//! slot rather than per grid slot or per race.
//!
//! Generalised out of the shape [`super::Cue::PlasmaTravel`] first built
//! directly into `Audio::race_tick`, the day the Rocket, the Missile and the
//! Shuriken each needed their own copy of it. A bolt is neither one craft nor
//! the whole race: more than one can be in the air together, and a slot index
//! is the only stable handle a `Copy` world snapshot gives a projectile - see
//! [`super::SfxVoices::plasma_travel`]'s own doc comment for the fuller
//! argument, which still applies unchanged.
//!
//! Never pushed through the cue queue itself: every travel cue's
//! [`super::Cue::held`] returns `true`, so a stray push is dropped rather than
//! mis-played as a one-shot, and this is read and written directly off the
//! world's own projectile array every tick instead.

use oag_audio::{Emitter, Listener, Mixer, Play, VoiceId};
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_weapons::projectile::{MAX_PROJECTILES, Projectile};

use super::{Banks, Cue};

/// One weapon's held voices, one per projectile slot - `None` at every index
/// that weapon does not currently occupy.
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
    ///
    /// `flying` decides which slots should be sounding this tick - `kind ==
    /// <weapon>` for the Rocket, the Missile and the Shuriken, further gated
    /// on `charge <= 0.0` for the Plasma. `position` says where that slot is
    /// heard from; `None` stops the voice the same tick a bolt actually
    /// ending would, which is what lets [`Cue::ShurikenTravel`]'s
    /// the position closure share this same tracker. `radius` is
    /// [`Cue::radius`], read once by the caller rather than per slot since it
    /// does not vary by projectile.
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

    /// One slot's voice for one tick: `at` is where it is heard from, or
    /// [`None`] when the thing it follows has gone.
    ///
    /// The body of [`Self::tick`]'s per-slot loop, so a single travelling thing
    /// that is not a projectile (the Quake's wave) can share it.
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
            // The rising edge: this slot started flying (or came into
            // range of a position) this tick.
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
                // Else nothing loaded, or the bank says this waveform is
                // not a loop - the same defensive guard `Engine::tick`
                // carries for `~ENGINE`.
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
            // The pool reclaimed the voice before the bolt itself ended -
            // starved, not stopped. Forget the stale handle so a later
            // tick does not stop whatever slot the pool gave it to next.
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
