//! The engine note: the one cue this port holds a handle to for a whole race.
//!
//! Split out of [`super`] because it is a law with state rather than a cue
//! lookup, and because there are now eight of them - one per grid slot, each
//! with its own random note and its own emitter.

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
/// # The unit of `pitch` is 1/128 of a semitone - 1536 to the octave
///
/// Until 2026-09-16 this was read as cents (1200 to the octave) at confidence
/// 60, with "a hardware capture would settle it in a second" written beside
/// it. The capture is done and it settled it the other way. The number is
/// written to a SCREAM sound instance's `+0x04`, which `Scream_StartSound`
/// carries into the handler as the **pitch offset** and
/// `Scream_ComputeVoiceNote` (`0x089950a0`) adds to `note * 128 + fine` -
/// the same 1/128th-of-a-semitone unit the fine-tune table is indexed in
/// (`floor(32768 * 2^(i/1536))`, read off the binary) and the same `1536.0`
/// `SoundInstance_UpdateSpatial`'s Doppler term multiplies an octave ratio
/// by. Live, at a standstill, `~ENGINE`'s voice reached `sceSasSetPitch`
/// with an offset of `-1148` on its 22,050 Hz descriptor and a pitch word of
/// `0x4c3`: 13,124 Hz, a ratio of 0.595, which is `2^(-1148/1536)` and not
/// `2^(-1148/1200)` (0.515). Confidence **92** - a decompiled path, a table
/// closed form and a bit-exact live hit agreeing; see
/// `docs/ghidra/functions/psp-pulse-usa/sound.md`'s pitch section and
/// `oag_formats::sblk::pitch`.
///
/// So the engine sits at `2^(-1143/1536)` = 0.60 at rest, unity at 228.6
/// km/h and about 1.18 at 300.
#[derive(Debug)]
pub struct Engine {
    /// The per-craft random offset, in the same unit as `lag`.
    base: f32,
    /// The lagged pitch the running engine chases its target with.
    lag: f32,
    /// What is actually written to the voice.
    ///
    /// Separate from [`Self::lag`] because the original's two branches move
    /// different variables: running, `pitch = lag`; stopped, `pitch` itself
    /// winds down by [`ENGINE_SPINDOWN`] a tick while `lag` is left where it
    /// was. Folding them would make the engine spin *back up* from wherever the
    /// chase had got to the moment it restarted.
    pitch: f32,
    /// The intensity the volume is derived from, and which the visual shares.
    intensity: f32,
    /// The held voice, while one is playing.
    voice: Option<VoiceId>,
    /// Whether the law has been stepped at least once, so the first tick snaps
    /// rather than sweeping in - the original's rising-edge branch.
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
/// `i * 0.6 + 0.4` when `craft+0x368` is set - see
/// [`exhaust.md`](../../../../docs/ghidra/functions/psp-pulse-usa/exhaust.md).
/// **It rides the same hypothesis [`Placement::CraftUnlessPlayer`] does**, that
/// `+0x368` separates the local player from everyone else, which
/// `docs/.../pads.md` records at confidence 45. Applying it to slots 1-7 is
/// therefore a bet, and it is written here rather than folded silently into
/// [`ENGINE_GAIN_SPAN`] so that it is one line to undo.
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
            intensity: 0.0,
            voice: None,
            started: false,
            stopped: false,
            warned: false,
        }
    }

    /// Advances the law one tick and writes the result to the voice.
    ///
    /// `on` is the original's `engine_on`: this port maps it to "the race is
    /// still running", which is the only engine-state edge the simulation has.
    /// The original's is a craft flag, so a destroyed or respawning craft would
    /// also fall silent there and does not here - recorded rather than guessed
    /// at, because no page has read that flag.
    ///
    /// Starts the voice on the first tick the cue is available, and never
    /// restarts it: `~ENGINE` is a held loop for the life of the craft, which
    /// is what the `~` means.
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
        dt: f32,
    ) {
        if on {
            let target = self.base + speed_kmh * ENGINE_PITCH_PER_KMH;
            if self.started {
                self.lag += (target - self.lag) * ENGINE_LAG_RATE;
            } else {
                // The original's rising edge: snap, no sweep-in. Without this
                // the note slides up over the first seconds of every race,
                // which is audible and is not what the original does.
                self.lag = target;
                self.started = true;
            }
            self.pitch = self.lag;
            self.intensity = (self.intensity + dt * ENGINE_RISE).clamp(0.0, 1.0);
        } else {
            // A spin-down rather than a cut: the note falls toward the craft's
            // own base note and the volume follows it down twice as fast.
            if self.base <= self.pitch {
                self.pitch -= ENGINE_SPINDOWN;
            }
            self.intensity = (self.intensity - dt * ENGINE_FALL).clamp(0.0, 1.0);
        }

        // **The wound-down engine is released rather than left humming.** The
        // recovered volume law floors at [`ENGINE_GAIN_FLOOR`], so intensity
        // reaching zero is as quiet as the law ever gets - 40 %, which under a
        // results table is a drone rather than a fade. The original does not
        // have this problem because `ExhaustFlare_Destroy` (`0x08904540`) takes
        // the voice with it when the craft's flare is torn down; *when* that
        // happens is not recovered, so this port releases the voice at the
        // bottom of the law instead and says so rather than inventing a fade.
        if !on && self.intensity <= 0.0 {
            self.stop(mixer);
            return;
        }

        let pitch = (self.pitch / PITCH_UNITS_PER_OCTAVE).exp2();
        // Three terms, multiplied the way the original multiplies them:
        // `Exhaust_UpdateEngineSound` writes `i * 0.6 + 0.4` (`x 0.85` for
        // everyone but the player) **into the sound instance's own volume
        // field**, which is exactly `SoundEmitter_ComputeVolumeAndAngle`'s
        // `request_volume` - the term the emitter's attenuation multiplies
        // *before* `curve()`, per `positional-audio.md`'s
        // `volume = curve(atten * request_volume)`. So the law has to reach
        // [`oag_audio::Emitter::place`] as its `volume` argument, not
        // multiply the already-curved gain afterwards: `curve` is a power
        // curve, and `curve(atten) * law != curve(atten * law)`.
        //
        // **`None` here is out of range, not un-placed**, and it goes to
        // silence rather than to unity: `SoundInstance_UpdateSpatial` writes
        // volume `0` to a live instance past its radius. The *other* half of
        // the original's gate - `Sound_Play` refusing to open a voice on an
        // out-of-range emitter at all - is applied to the one-shots in
        // [`Audio::race_tick`] and deliberately not here, because a held voice
        // opened once at the grid would then never exist for a craft that
        // happened to start more than 50 units from the camera, and nothing
        // read says the original re-opens it.
        let law = self.intensity * ENGINE_GAIN_SPAN + ENGINE_GAIN_FLOOR;
        let volume = law * if quieter { OPPONENT_ENGINE_SCALE } else { 1.0 };
        let placed = oag_audio::Emitter::engine(position).place(listener, volume);
        let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));

        match self.voice {
            Some(id) if mixer.is_playing(id) => {
                mixer.set_pitch(id, pitch);
                mixer.set_gain(id, gain);
                mixer.set_pan(id, pan);
            }
            // Never re-opened once the spin-down closed it: a finished race
            // that kept calling this would otherwise restart the engine on the
            // tick after it went quiet, for ever.
            _ if self.stopped => {}
            _ => {
                let Some((sound, looping)) = banks.pick(Cue::Engine, rng) else {
                    return;
                };
                if !looping {
                    // The bank says this is not a loop, so holding it would be
                    // a voice that stops and never comes back. Latched, or the
                    // complaint is sixty lines a second for the whole race.
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

    /// Releases the voice, which is what leaving a race does.
    pub fn stop(&mut self, mixer: &mut Mixer) {
        self.stopped = true;
        if let Some(id) = self.voice.take() {
            mixer.stop(id);
        }
    }
}

#[cfg(test)]
mod tests;
