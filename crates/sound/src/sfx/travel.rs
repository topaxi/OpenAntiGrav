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

use super::CueTables;
use super::layers::{self, Where};
use super::{Banks, Cue};

/// One weapon's held voices per projectile slot, empty where it does not
/// occupy the slot: one looping voice from a title's own bank, every voice the
/// event starts from a Wwise event ([`Banks::held_voices`]).
#[derive(Debug, Clone)]
pub(super) struct TravelVoices {
    voices: [Vec<VoiceId>; MAX_PROJECTILES],
}

/// Where a voice at `at` is heard from, `radius` its emitter's.
fn heard(at: Vec3, radius: f32, listener: &Listener) -> Where {
    let placed = Emitter {
        position: at.to_array(),
        radius,
        cone: None,
    }
    .place(listener, 1.0);
    Where {
        gain: placed.map_or(0.0, |p| p.gain),
        pan: placed.map(|p| p.pan),
        front: placed.map_or(1.0, |p| p.front),
    }
}

impl TravelVoices {
    /// No voices open yet.
    pub(super) fn new() -> Self {
        Self {
            voices: std::array::from_fn(|_| Vec::new()),
        }
    }

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
        let held = &mut self.voices[slot];
        // Forget a handle the pool reclaimed before the bolt ended (starved, not
        // stopped), so a later tick does not stop whatever the pool gave it next.
        let was_held = !held.is_empty();
        held.retain(|&id| mixer.is_playing(id));
        match at {
            // Reclaimed this tick: started again on the next.
            Some(_) if was_held && held.is_empty() => {}
            // The rising edge: this slot started flying (or came into range) this tick.
            Some(at) if held.is_empty() => {
                let place = heard(at, radius, listener);
                if let Some(event) = banks.held_voices(cue, rng) {
                    *held = layers::start(mixer, &event, cue.bus(), place);
                } else if let Some((sound, looping)) = banks.pick(cue, rng)
                    && looping
                {
                    held.extend(mixer.play(Play {
                        gain: place.gain,
                        pan: place.pan,
                        front: place.front,
                        ..Play::looping(sound, cue.bus())
                    }));
                }
                // Nothing loaded, or the bank says it is not a loop: the guard of
                // `Engine::tick` for `~ENGINE`.
            }
            // Still flying and still held: follow it.
            Some(at) => {
                let place = heard(at, radius, listener);
                for &id in held.iter() {
                    mixer.set_gain(id, place.gain);
                    mixer.set_pan(id, place.pan);
                    mixer.set_front(id, place.front);
                }
            }
            // The falling edge: no longer flying, or nowhere to place it. The
            // original stops a held post with no fade (`StopPlayingID(id, 0,
            // linear)`, `docs/ghidra/functions/ps4-omega-eu/wwise-cues.md`, "Held
            // loops"); the mixer's own release is the declick.
            None => {
                for id in held.drain(..) {
                    mixer.stop(id);
                }
            }
        }
    }

    /// Releases every held voice, for [`super::Audio::stop_race_sfx`].
    pub(super) fn stop_all(&mut self, mixer: &mut Mixer) {
        for held in &mut self.voices {
            for id in held.drain(..) {
                mixer.stop(id);
            }
        }
    }
}
