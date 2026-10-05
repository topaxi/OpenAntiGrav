//! The magstrip hum's held voices, one list per grid slot.
//!
//! [`Cue::Magstrip`] starts a craft's voices and [`Cue::MagstripStop`] ends
//! them; between the two they follow the craft, as the original's group is
//! anchored to the ship's own transform. Neither reaches the one-shot path -
//! see [`Cue::held`].

use oag_audio::{Mixer, VoiceId};
use oag_core::Rng;
use oag_core::math::Vec3;

use super::layers::{self, Where as VoicePlace};
use super::{Banks, Cue, CueEvent, place};

#[derive(Debug, Default)]
pub(super) struct Voices {
    held: [Vec<VoiceId>; oag_gameplay::MAX_SHIPS],
}

/// How loud and where `slot` is heard from. A craft out of range, or with no
/// pose, is silent rather than refused: the hum is held, and the craft may come
/// back into range while it plays.
fn heard(slot: usize, listener: &oag_audio::Listener, craft: &[Option<(Vec3, f32)>]) -> VoicePlace {
    let placed = place(CueEvent::new(Cue::Magstrip, slot), listener, craft);
    VoicePlace {
        gain: placed.map_or(0.0, |p| p.gain),
        pan: placed.and_then(|p| p.pan),
    }
}

impl Voices {
    /// Raises `slot`'s hum, replacing any it still holds.
    pub(super) fn start(
        &mut self,
        slot: usize,
        mixer: &mut Mixer,
        banks: &Banks,
        rng: &mut Rng,
        listener: &oag_audio::Listener,
        craft: &[Option<(Vec3, f32)>],
    ) {
        self.stop(slot, mixer);
        let Some(started) = banks.voices(Cue::Magstrip, rng) else {
            return;
        };
        if let Some(held) = self.held.get_mut(slot) {
            *held = layers::start(
                mixer,
                &started,
                Cue::Magstrip.bus(),
                heard(slot, listener, craft),
            );
        }
    }

    /// Ends `slot`'s hum.
    pub(super) fn stop(&mut self, slot: usize, mixer: &mut Mixer) {
        if let Some(held) = self.held.get_mut(slot) {
            for id in held.drain(..) {
                mixer.stop(id);
            }
        }
    }

    /// Moves every held hum with its craft, and forgets one the pool took back.
    pub(super) fn follow(
        &mut self,
        mixer: &mut Mixer,
        listener: &oag_audio::Listener,
        craft: &[Option<(Vec3, f32)>],
    ) {
        for (slot, held) in self.held.iter_mut().enumerate() {
            held.retain(|&id| mixer.is_playing(id));
            if held.is_empty() {
                continue;
            }
            let at = heard(slot, listener, craft);
            for &id in held.iter() {
                mixer.set_gain(id, at.gain);
                mixer.set_pan(id, at.pan);
            }
        }
    }

    /// Ends every hum.
    pub(super) fn stop_all(&mut self, mixer: &mut Mixer) {
        for slot in 0..self.held.len() {
            self.stop(slot, mixer);
        }
    }
}
