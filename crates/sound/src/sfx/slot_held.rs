//! A held cue's voices that ride a craft, one list per grid slot: the magstrip hum
//! ([`Cue::Magstrip`]), a destroyed craft's `~SHIP_DESTROYED` and `~SHIP_ONFIRE`.
//!
//! The start cue starts a craft's voices and its stop cue ends them; between, they
//! follow the craft, as the original's group is anchored to the ship's transform.
//! Neither reaches the one-shot path ([`Cue::held`]).

use oag_audio::{Mixer, VoiceId};
use oag_core::Rng;
use oag_core::math::Vec3;

use super::CueTables;
use super::layers::{self, Where as VoicePlace};
use super::{Banks, Cue, CueEvent, place};

/// One start cue's voices, by grid slot.
#[derive(Debug)]
pub(super) struct Voices {
    cue: Cue,
    held: [Vec<VoiceId>; oag_gameplay::MAX_SHIPS],
}

/// How loud and where `slot` is heard from. A craft out of range or with no pose
/// is silent, not refused: the hum is held and the craft may return to range.
fn heard(
    cue: Cue,
    slot: usize,
    listener: &oag_audio::Listener,
    craft: &[Option<(Vec3, f32)>],
) -> VoicePlace {
    // Placed off its craft whoever owns it, so no seat is the dry one.
    let placed = place(CueEvent::new(cue, slot), listener, craft, usize::MAX);
    VoicePlace {
        gain: placed.map_or(0.0, |p| p.gain),
        pan: placed.and_then(|p| p.pan),
        front: placed.map_or(1.0, |p| p.front),
    }
}

/// How many draws to spend finding a looping waveform.
const DRAWS: usize = 64;

/// One draw of `cue` that loops, or failing that the first draw.
///
/// Chosen, not measured. `~magstrip01` is a tree of 35 waveforms the reader flattens
/// into uniform alternates (the load report says so), about seven looping. The
/// original starts it by name and stops the group on the falling edge, so what it
/// holds is a loop; this port cannot yet tell which leaf the tree resolves to,
/// and a one-shot would end a second into a visit it should last, so it draws
/// until one loops. The destroyed craft's two loops are drawn the same way.
fn held_draw(cue: Cue, banks: &Banks, rng: &mut Rng) -> Option<Vec<layers::CueVoice>> {
    // A Wwise event marks its own loops: every layer it starts sounds.
    if let Some(event) = banks.held_voices(cue, rng) {
        return Some(event);
    }
    let first = banks.voices(cue, rng)?;
    if first.iter().all(|voice| voice.looping) {
        return Some(first);
    }
    for _ in 0..DRAWS {
        if let Some(draw) = banks.voices(cue, rng)
            && draw.iter().all(|voice| voice.looping)
        {
            return Some(draw);
        }
    }
    Some(first)
}

impl Voices {
    /// A holder of `cue`'s voices.
    pub(super) fn new(cue: Cue) -> Self {
        Self {
            cue,
            held: Default::default(),
        }
    }

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
        let Some(started) = held_draw(self.cue, banks, rng) else {
            return;
        };
        if let Some(held) = self.held.get_mut(slot) {
            *held = layers::start(
                mixer,
                &started,
                self.cue.bus(),
                heard(self.cue, slot, listener, craft),
            );
            log::trace!(
                "sfx: held {:?} opened on slot {slot}, {} voice(s)",
                self.cue,
                held.len()
            );
        }
    }

    /// Ends `slot`'s hum.
    pub(super) fn stop(&mut self, slot: usize, mixer: &mut Mixer) {
        if let Some(held) = self.held.get_mut(slot) {
            if !held.is_empty() {
                log::trace!(
                    "sfx: held {:?} released on slot {slot}, {} voice(s)",
                    self.cue,
                    held.len()
                );
            }
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
            let at = heard(self.cue, slot, listener, craft);
            for &id in held.iter() {
                mixer.set_gain(id, at.gain);
                mixer.set_pan(id, at.pan);
                mixer.set_front(id, at.front);
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

/// The start and stop cue of every held voice that rides a craft.
const PAIRS: [(Cue, Cue); 3] = [
    (Cue::Magstrip, Cue::MagstripStop),
    (Cue::ShipDestroyed, Cue::ShipDestroyedStop),
    (Cue::ShipOnFire, Cue::ShipOnFireStop),
];

/// Every [`PAIRS`] holder.
#[derive(Debug)]
pub(super) struct Set {
    voices: [Voices; PAIRS.len()],
}

impl Default for Set {
    fn default() -> Self {
        Self {
            voices: PAIRS.map(|(start, _)| Voices::new(start)),
        }
    }
}

impl Set {
    /// Acts on `cue` when it starts or stops one of the pairs; `false` for any other cue.
    pub(super) fn handle(
        &mut self,
        cue: Cue,
        slot: usize,
        mixer: &mut Mixer,
        banks: &Banks,
        rng: &mut Rng,
        listener: &oag_audio::Listener,
        craft: &[Option<(Vec3, f32)>],
    ) -> bool {
        for (voices, (start, stop)) in self.voices.iter_mut().zip(PAIRS) {
            if cue == start {
                voices.start(slot, mixer, banks, rng, listener, craft);
                return true;
            }
            if cue == stop {
                voices.stop(slot, mixer);
                return true;
            }
        }
        false
    }

    /// Moves every held voice with its craft.
    pub(super) fn follow(
        &mut self,
        mixer: &mut Mixer,
        listener: &oag_audio::Listener,
        craft: &[Option<(Vec3, f32)>],
    ) {
        for voices in &mut self.voices {
            voices.follow(mixer, listener, craft);
        }
    }

    /// Ends every held voice.
    pub(super) fn stop_all(&mut self, mixer: &mut Mixer) {
        for voices in &mut self.voices {
            voices.stop_all(mixer);
        }
    }
}
