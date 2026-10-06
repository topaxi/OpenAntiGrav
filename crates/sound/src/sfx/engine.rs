//! The engine note: the one cue this port holds a handle to for a whole race.
//! A law with state, one per grid slot, each with its own random note and emitter.

use log::warn;
use oag_audio::{Mixer, Play, VoiceId};
use oag_core::Rng;

use super::{Banks, Cue};

/// The engine voice, and the law that drives its pitch and volume.
///
/// `Exhaust_UpdateEngineSound` (`0x08904cf4`), confidence 80:
///
/// ```text
/// base  = rand(-127, 127) - 1143                 per craft, once
/// ON :  target = base + speed_kmh * 5.0
///       lag   += (target - lag) * 0.01           substepped
///       i     += dt * 0.25
/// OFF:  if base <= pitch { pitch -= 48.0 }
///       i     -= dt * 0.5
/// clamp i to [0, 1]
/// pitch  = lag
/// volume = i * 0.6 + 0.4
/// ```
///
/// # The unit of `pitch` is 1/128 of a semitone, 1536 to the octave
///
/// Read as cents (1200 to the octave) at confidence 60 until 2026-09-16; the
/// hardware capture settled it the other way. The value goes to a SCREAM sound
/// instance's `+0x04`, which `Scream_StartSound` carries as the pitch offset and
/// `Scream_ComputeVoiceNote` (`0x089950a0`) adds to `note * 128 + fine`: the
/// unit the fine-tune table is indexed in (`floor(32768 * 2^(i/1536))`, read off
/// the binary) and the `1536.0` `SoundInstance_UpdateSpatial`'s Doppler term
/// multiplies an octave ratio by. Live, at a standstill, `~ENGINE` reached
/// `sceSasSetPitch` with offset `-1148` on its 22,050 Hz descriptor and pitch
/// word `0x4c3`: 13,124 Hz, ratio 0.595 = `2^(-1148/1536)`, not
/// `2^(-1148/1200)` (0.515). Confidence **92** (decompiled path, table closed
/// form and bit-exact live hit); see
/// `docs/ghidra/functions/psp-pulse-usa/sound.md`'s pitch section and
/// `oag_formats::sblk::pitch`.
///
/// The engine sits at `2^(-1143/1536)` = 0.60 at rest, unity at 228.6 km/h and
/// about 1.18 at 300.
#[derive(Debug)]
pub struct Engine {
    /// The per-craft random offset, in the same unit as `lag`.
    base: f32,
    lag: f32,
    /// What is actually written to the voice.
    /// What is written to the voice. Separate from [`Self::lag`] because the
    /// original's branches move different variables: running, `pitch = lag`;
    /// stopped, `pitch` winds down by [`ENGINE_SPINDOWN`] a tick and `lag` stays,
    /// else the engine would spin back up from wherever the chase had got to.
    pitch: f32,
    /// The distance this engine was heard at last tick, for the doppler term.
    doppler: oag_audio::Doppler,
    /// The intensity the volume is derived from, and which the visual shares.
    intensity: f32,
    voice: Option<VoiceId>,
    /// Whether the law has stepped once, so the first tick snaps (the original's
    /// rising-edge branch).
    started: bool,
    /// Set once the spin-down has released the voice, so it is never re-opened.
    stopped: bool,
    /// Latch on the not-looping complaint. See [`Engine::tick`].
    warned: bool,
}

/// Pitch-offset units to an octave: 12 semitones of 128 fine steps, the unit
/// `base` is in. See [`Engine`].
const PITCH_UNITS_PER_OCTAVE: f32 = 1536.0;

/// The centre of the per-craft random spread, `rand(-127, 127) - 1143`.
const ENGINE_BASE: f32 = -1143.0;

/// Half-width of that spread.
const ENGINE_SPREAD: f32 = 127.0;

/// How fast the lagged pitch chases its target, per substep.
const ENGINE_LAG_RATE: f32 = 0.01;

/// Pitch per km/h.
const ENGINE_PITCH_PER_KMH: f32 = 5.0;

/// How fast intensity rises with the engine on, per second.
const ENGINE_RISE: f32 = 0.25;

/// How fast it falls with the engine off, per second.
const ENGINE_FALL: f32 = 0.5;

/// How far the stopped engine's pitch winds down each tick, until it reaches
/// [`Engine::base`].
const ENGINE_SPINDOWN: f32 = 48.0;

/// The volume law's floor and span: `intensity * 0.6 + 0.4`.
const ENGINE_GAIN_FLOOR: f32 = 0.4;
const ENGINE_GAIN_SPAN: f32 = 0.6;

/// What everyone but the player is scaled by.
///
/// `Exhaust_UpdateEngineSound` writes `(i * 0.6 + 0.4) * 0.85` instead of
/// `i * 0.6 + 0.4` when `craft+0x368` is set
/// ([`exhaust.md`](../../../../docs/ghidra/functions/psp-pulse-usa/exhaust.md)).
/// It rides the hypothesis [`Placement::CraftUnlessPlayer`] does (`+0x368`
/// separates the local player, confidence 45 in `docs/.../pads.md`), so applying
/// it to slots 1-7 is a bet, kept apart from [`ENGINE_GAIN_SPAN`] to be one line
/// to undo.
const OPPONENT_ENGINE_SCALE: f32 = 0.85;

impl Engine {
    /// A craft's engine, with its own note picked out of `rng`.
    #[must_use]
    pub fn new(rng: &mut Rng) -> Self {
        let base = ENGINE_BASE + (rng.next_f32() * 2.0 - 1.0) * ENGINE_SPREAD;
        Self {
            base,
            lag: base,
            pitch: base,
            doppler: oag_audio::Doppler::default(),
            intensity: 0.0,
            voice: None,
            started: false,
            stopped: false,
            warned: false,
        }
    }

    /// Advances the law one tick and writes the result to the voice.
    ///
    /// `on` is the original's `engine_on`, mapped to "the race is still
    /// running", the only engine-state edge the simulation has. The original's is
    /// a craft flag, so a destroyed or respawning craft also falls silent there
    /// and not here; no page has read that flag.
    ///
    /// Starts the voice on the first tick the cue is available and never
    /// restarts it: `~ENGINE` is a held loop for the craft's life.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        mixer: &mut Mixer,
        banks: &Banks,
        rng: &mut Rng,
        speed_kmh: f32,
        on: bool,
        position: [f32; 3],
        listener: &oag_audio::Listener,
        quieter: bool,
        doppler_enabled: bool,
        dt: f32,
    ) {
        if on {
            let target = self.base + speed_kmh * ENGINE_PITCH_PER_KMH;
            if self.started {
                self.lag += (target - self.lag) * ENGINE_LAG_RATE;
            } else {
                // The original's rising edge: snap, no sweep-in (else the note
                // slides up over the first seconds of every race).
                self.lag = target;
                self.started = true;
            }
            self.pitch = self.lag;
            self.intensity = (self.intensity + dt * ENGINE_RISE).clamp(0.0, 1.0);
        } else {
            // A spin-down, not a cut: the note falls toward the craft's base and
            // the volume follows twice as fast.
            if self.base <= self.pitch {
                self.pitch -= ENGINE_SPINDOWN;
            }
            self.intensity = (self.intensity - dt * ENGINE_FALL).clamp(0.0, 1.0);
        }

        // The wound-down engine is released rather than left humming: the
        // recovered volume law floors at [`ENGINE_GAIN_FLOOR`], 40 %, a drone
        // under a results table. The original has `ExhaustFlare_Destroy`
        // (`0x08904540`) take the voice when the flare is torn down; when that
        // happens is not recovered, so this releases at the bottom of the law
        // rather than inventing a fade.
        if !on && self.intensity <= 0.0 {
            self.stop(mixer);
            return;
        }

        let pitch = (self.pitch / PITCH_UNITS_PER_OCTAVE).exp2();
        // Three terms, multiplied as the original does: `Exhaust_UpdateEngineSound`
        // writes `i * 0.6 + 0.4` (`x 0.85` for all but the player) into the
        // instance's own volume field, `SoundEmitter_ComputeVolumeAndAngle`'s
        // `request_volume`, which the attenuation multiplies *before* `curve()`
        // (`positional-audio.md`: `volume = curve(atten * request_volume)`). So the
        // law goes to [`oag_audio::Emitter::place`] as its `volume` argument:
        // `curve` is a power curve and `curve(atten) * law != curve(atten * law)`.
        //
        // `None` is out of range, not un-placed, and goes to silence, not unity:
        // `SoundInstance_UpdateSpatial` writes volume `0` to a live instance past
        // its radius. The other half of the gate (`Sound_Play` refusing to open on
        // an out-of-range emitter) is applied to one-shots in [`Audio::race_tick`]
        // and not here: a held voice opened once at the grid would never exist for
        // a craft starting over 50 units from the camera, and nothing read says
        // the original re-opens it.
        let law = self.intensity * ENGINE_GAIN_SPAN + ENGINE_GAIN_FLOOR;
        let volume = law * if quieter { OPPONENT_ENGINE_SCALE } else { 1.0 };
        let placed = oag_audio::Emitter::engine(position).place(listener, volume);
        let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));
        // The doppler term rides on the note: `SoundInstance_UpdateSpatial` adds
        // `-(dd/dt) * 0.0005 * 1536` pitch units, a multiply in ratios. The
        // player's own engine gets it too (the ear is the lagging chase camera),
        // as in the original. See `oag_audio::Doppler`.
        let doppler = match placed {
            Some(p) => self.doppler.ratio(p.distance, dt, doppler_enabled),
            None => {
                self.doppler.reset();
                1.0
            }
        };
        let pitch = pitch * doppler;

        match self.voice {
            Some(id) if mixer.is_playing(id) => {
                mixer.set_pitch(id, pitch);
                mixer.set_gain(id, gain);
                mixer.set_pan(id, pan);
            }
            // Never re-opened once the spin-down closed it: a finished race would
            // restart the engine the tick after it went quiet.
            _ if self.stopped => {}
            _ => {
                let Some((sound, looping)) = banks.pick(Cue::Engine, rng) else {
                    return;
                };
                if !looping {
                    // Not a loop per the bank, so holding it would be a voice that
                    // stops for good. Latched, or the complaint is sixty lines a second.
                    if !self.warned {
                        self.warned = true;
                        warn!("sfx: ~ENGINE is not marked looping in this bank; not held");
                    }
                    return;
                }
                self.voice = mixer.play(Play {
                    gain,
                    pitch,
                    pan,
                    ..Play::looping(sound, Cue::Engine.bus())
                });
            }
        }
    }

    /// Releases the voice, on leaving a race.
    pub fn stop(&mut self, mixer: &mut Mixer) {
        self.stopped = true;
        if let Some(id) = self.voice.take() {
            mixer.stop(id);
        }
    }
}

#[cfg(test)]
mod tests;
