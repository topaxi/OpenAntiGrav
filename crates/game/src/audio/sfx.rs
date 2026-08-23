//! Sound effects: the cues a race emits, and the banks they are read out of.
//!
//! [`super`] owns music and movie audio, which are streams with one source
//! each. This owns the other half: short sounds fired from *edges* in the
//! simulation, addressed the way the original addresses them - by a cue string
//! looked up in a `.bnk` sound bank.
//!
//! # Nothing here is fired on a guess
//!
//! `CLAUDE.md`'s rule for particle effects applies unchanged to sound: an
//! effect whose **trigger** is not recovered stays unwired rather than played
//! at a plausible-looking moment. Every variant of [`Cue`] names the page that
//! recovered its trigger, and the banks hold a great many cues this does not
//! fire - `WEAPONPICKUP`, `WRONGWAY`, `EXPLBIG`, `~AUTOPILOT` and the rest are
//! all sitting there, decoded and reachable, waiting for their call site to be
//! read out of the executable. `oag-wad sounds <archive>` lists them.
//!
//! The reverse also holds: **every cue whose trigger *is* recovered is wired**.
//! When a page like `shield-pickup.md` names a `Sound_Play` call site, that cue
//! belongs here, and `Cue::ALL` is the whole set of them.
//!
//! # Two things here are honest placeholders, and both are load-bearing
//!
//! **The sample rate.** [`oag_formats::sblk::ASSUMED_SAMPLE_RATE`] is not
//! recovered; see its doc comment. Everything below plays at it.
//!
//! **Which waveform of a cue sounds.** `.COLLISIONS` binds fifteen impact
//! samples and the opcode that chooses between them is one of the 43 that are
//! unread, so [`Bank::pick`] chooses with its own generator. That is a
//! deliberate approximation rather than a stand-in for missing data: the
//! fifteen alternates *are* the disc's own audio, all of them within a tenth of
//! a second of each other in length, and playing all fifteen at once - the only
//! reading that needs no choice - is the one thing that is certainly wrong.
//!
//! # Positional audio does not exist yet
//!
//! The original gives every craft its own emitter with a world position and a
//! radius (`Exhaust_UpdateEngineSound`'s `self+0x78` is a 0x70-byte record with
//! `50.0` at `+0x38`). The mixer has no panner, so **only the player's craft is
//! audible here**. An opponent crossing a pad is silent rather than played dry
//! at full volume in the middle of the field, which would be a louder error.

use std::collections::BTreeMap;
use std::sync::Arc;

use log::{info, warn};
use oag_assets::source::Archives;
use oag_audio::{Bus, Mixer, Play, Sound, VoiceId};
use oag_core::Rng;
use oag_formats::sblk;

use super::{Audio, TICK_HZ};

/// The seed the effects generator starts from.
///
/// Fixed and named, not drawn from a clock: a `--dump-audio` capture of the
/// same run has to choose the same collision alternates every time, or two
/// captures of one race could not be compared. It is **not** the race seed -
/// the simulation's generator must not be advanced by anything audio does, and
/// sharing a seed would invite exactly that. See
/// `docs/architecture/determinism.md`.
const SFX_SEED: u64 = 0x0aa9_5f10_0000_5f58;

/// The race's held voices and the generator that chooses a cue's alternate.
///
/// Split from [`Banks`], which is decoded data on the race, for the same reason
/// `oag_render::sparks` splits its pipeline from its particle state.
pub(super) struct SfxVoices {
    engine: Engine,
    /// The shield's held voice, while one is up. See [`Cue::Shield`].
    shield: Option<VoiceId>,
    rng: Rng,
}

/// Seconds in one simulation tick, which is the only clock this module has.
const DT: f32 = 1.0 / TICK_HZ as f32;

impl Audio {
    /// Plays one tick's worth of a race's sound effects.
    ///
    /// **Called from inside the fixed-timestep loop, immediately after
    /// `Race::tick`**, which is what makes a `--dump-audio` capture put a cue
    /// on the same tick a window does. Draining once per *frame* would merge
    /// two ticks' cues on a slow frame and drop none on a fast one, so a pad
    /// hit twice in one frame would sound once.
    ///
    /// **Also called on a finished race, where nothing is stepped.** The engine
    /// is a *held* voice with a spin-down law, so a race whose last tick has
    /// run still has audio to advance; skipping this would leave `~ENGINE`
    /// looping at racing pitch under the results table until the player backed
    /// out. See [`Engine::tick`].
    ///
    /// Everything device-shaped is here rather than on `Race`: the voice pool,
    /// the held voices and the generator that chooses between a cue's
    /// alternates. See `docs/architecture/adr/0018-audio-mixer-architecture.md`.
    pub fn race_tick(&mut self, race: &mut crate::race::Race) {
        let cues = race.drain_cues();
        if race.sounds().is_empty() {
            return;
        }
        let voices = self.sfx.get_or_insert_with(|| SfxVoices {
            engine: Engine::new(&mut Rng::new(SFX_SEED)),
            shield: None,
            rng: Rng::new(SFX_SEED),
        });
        let banks = race.sounds();
        let speed_kmh = race.telemetry().speed * oag_render::exhaust::SPEED_TO_KMH;
        let running = !race.finished();
        let shielded = race.shield_is_up();
        self.output.with_mixer(|mixer| {
            for cue in cues {
                // A held cue is not a one-shot and must not be fired as one -
                // `~ENGINE` reaching here would start a second engine every
                // time it was raised.
                if cue.held() {
                    continue;
                }
                let Some((sound, looping)) = banks.pick(cue, &mut voices.rng) else {
                    continue;
                };
                let play = if looping {
                    Play::looping(sound, Bus::Sfx)
                } else {
                    Play::once(sound, Bus::Sfx)
                };
                // The return is dropped deliberately: a one-shot is fired and
                // forgotten, and a refused voice is already counted by
                // `Mixer::starved`.
                let _ = mixer.play(play);
            }

            // `Shield_Activate` opens `~SHIELD` with a handle and the shield's
            // own drop releases it, so this is a *level* rather than an edge:
            // the voice exists exactly while the pickup timer is running.
            match (shielded, voices.shield) {
                (true, None) => {
                    if let Some((sound, _)) = banks.pick(Cue::Shield, &mut voices.rng) {
                        voices.shield = mixer.play(Play::looping(sound, Bus::Sfx));
                    }
                }
                (false, Some(id)) => {
                    mixer.stop(id);
                    voices.shield = None;
                }
                _ => {}
            }

            voices
                .engine
                .tick(mixer, banks, &mut voices.rng, speed_kmh, running, DT);
        });
    }

    /// Releases the held race voices, which is what leaving a race does.
    ///
    /// Called on the way back to the menus **and** on launching a race, because
    /// a relaunch replaces the whole `RaceStage` without passing through the
    /// back-out path - and a carried-over [`SfxVoices`] would hold a `VoiceId`
    /// into a pool that has since been reused, plus an engine already past its
    /// rising-edge snap.
    pub fn stop_race_sfx(&mut self) {
        if let Some(voices) = &mut self.sfx {
            self.output.with_mixer(|mixer| {
                voices.engine.stop(mixer);
                if let Some(id) = voices.shield.take() {
                    mixer.stop(id);
                }
            });
        }
        self.sfx = None;
    }
}

/// Which bank a cue lives in.
///
/// The paths are literal strings in the PSP executable, every one of which
/// hashes to a real archive entry on the PSP *and* the PS2 disc - see
/// `docs/formats/psp-audio.md`'s table. The PS2 build ships the same banks
/// under the same names in `WADS2.WAD`, which is why nothing here branches on
/// the platform: [`Archives::read_name`] searches whichever archives the source
/// turned out to have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BankName {
    /// `Data\Sound\hud.bnk`.
    Hud,
    /// `Data\Sound\ship.bnk`, or `ship_zone.bnk` in Zone mode.
    Ship,
    /// `Data\Sound\weapons.bnk`.
    Weapons,
    /// `Data\Sound\speech.bnk` - the announcer.
    ///
    /// The executable also carries `Data\Sound\speech_%s.bnk` and
    /// `speech_zone_%s.bnk` templates formatted with a language at runtime, and
    /// **no expansion of either resolves on the EU disc** - twelve language
    /// names were tried and every one missed. So this port reads the unsuffixed
    /// bank, which both discs do carry, and the localised path stays open. See
    /// `docs/formats/psp-audio.md`.
    Speech,
}

impl BankName {
    /// The archive entry to read, given whether this is a Zone race.
    ///
    /// Only [`Self::Ship`] moves. `SHIP_ZM` is a different bank with the same
    /// cue names and different audio - a nine-layer `~ENGINE` against one, and
    /// ten collision alternates against fifteen - so a Zone race that played
    /// `ship.bnk` would be quietly playing the wrong craft.
    #[must_use]
    pub fn entry(self, zone: bool) -> &'static str {
        match self {
            Self::Hud => r"Data\Sound\hud.bnk",
            Self::Ship if zone => r"Data\Sound\ship_zone.bnk",
            Self::Ship => r"Data\Sound\ship.bnk",
            Self::Weapons => r"Data\Sound\weapons.bnk",
            Self::Speech => r"Data\Sound\speech.bnk",
        }
    }
}

/// A sound the simulation asks for, by the name the original passes to
/// `Sound_Play`.
///
/// Every one of these has a recovered call site. The doc comment on each says
/// where, because that is the difference between a port and a soundalike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cue {
    /// Crossing onto a speed pad.
    ///
    /// `Ship_ApplySpeedupPad` calls `Sound_Play(..., "SPEEDUPPAD", ...)` from
    /// inside its new-pad branch, the same branch that arms the engine flare -
    /// so this fires on exactly the edge `oag_game::race::Race::test_speedup_pads`
    /// already arms the flare on. `docs/ghidra/functions/psp-pulse-usa/pads.md`,
    /// confidence 88.
    SpeedupPad,
    /// Hull against wall or track.
    ///
    /// `ShipCollisionFx_Trigger` (`0x089246b4`) fires it "once per surviving
    /// kind-0/1 call" - that is, once per contact that gets past the 0.8-second
    /// spark cooldown - so it rides the same gate the collision sparks do.
    /// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`, confidence 85.
    Collision,
    /// A contact a raised shield absorbed.
    ///
    /// `FUN_08840640`, the shield-absorb ability's own effect, "plays an
    /// `ABSORB` sound once" before staggering its ten spark instances. Same
    /// page. This is the sound of the contact the shield *ate*, which is why it
    /// fires exactly where the sparks are suppressed.
    Absorb,
    /// The engine, held for as long as the craft is running.
    ///
    /// `Exhaust_UpdateEngineSound` (`0x08904cf4`) opens the voice in its
    /// constructor and writes pitch and volume to it every tick;
    /// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`, confidence 80. The law
    /// is in [`Engine`].
    Engine,
    /// The shield, held for as long as it is up.
    ///
    /// `Shield_Activate` (`0x0883e544`) calls
    /// `Sound_PlayLooping(1.0, entity->0x50, ..., "~SHIELD", entity + 0x54)`,
    /// keeping the handle - so it runs for the pickup's duration and is
    /// released when the shield drops.
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    Shield,
    /// The announcer, on the same activation.
    ///
    /// The other half of `Shield_Activate`'s pair, one line above [`Self::Shield`]:
    /// `Sound_Play(entity, ..., "shieldactive", 0x400, 0)`. It lives in
    /// `speech.bnk` rather than `weapons.bnk`, which is what says it is a voice
    /// line and not an effect.
    ShieldActive,
}

impl Cue {
    /// Every cue this port fires, which is every one it knows how to load.
    pub const ALL: [Self; 6] = [
        Self::SpeedupPad,
        Self::Collision,
        Self::Absorb,
        Self::Engine,
        Self::Shield,
        Self::ShieldActive,
    ];

    /// The bank the cue is looked up in.
    #[must_use]
    pub fn bank(self) -> BankName {
        match self {
            Self::SpeedupPad => BankName::Hud,
            Self::Collision | Self::Engine => BankName::Ship,
            Self::Absorb | Self::Shield => BankName::Weapons,
            Self::ShieldActive => BankName::Speech,
        }
    }

    /// The string to look up in that bank's name table.
    ///
    /// # `COLLISIONS` is stored as `.COLLISIONS`
    ///
    /// The executable passes `"COLLISIONS"`; no bank on either disc holds a cue
    /// by that name, and `ship.bnk` and `ship_zone.bnk` both hold
    /// `".COLLISIONS"`. SCREAM's error strings distinguish *a sound* from *a
    /// child sound* (`"Didn't find sound named -> %s"` against `"Didn't find
    /// child sound named -> %s"`), so the leading dot is almost certainly that
    /// distinction and the lookup the game makes is the child one.
    ///
    /// **The dot is written here rather than stripped at lookup**, so that
    /// [`Bank::cue`](oag_formats::sblk::Bank::cue_named) stays the runtime's own
    /// 16-byte comparison and cannot resolve a name the original would have
    /// rejected. Recorded as a hypothesis at confidence **70**: the name is
    /// otherwise exact, it is the only candidate on either disc, and the parent
    /// lookup is not implemented - so if a child cue turns out to select among
    /// its parents rather than the other way round, this is the line that is
    /// wrong.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::SpeedupPad => "SPEEDUPPAD",
            Self::Collision => ".COLLISIONS",
            Self::Absorb => "ABSORB",
            Self::Engine => "~ENGINE",
            Self::Shield => "~SHIELD",
            Self::ShieldActive => "shieldactive",
        }
    }

    /// Whether this port keeps a handle to the voice rather than firing and
    /// forgetting it.
    ///
    /// Both of these are cues the original opens with `Sound_PlayLooping` and
    /// an out-parameter, which is what the `~` prefix marks - see
    /// `docs/architecture/adr/0018-audio-mixer-architecture.md`. The `~` is
    /// **not** the test: `~SPARKS` and `~FLYBY_DIST` also carry it and are
    /// one-shots the caller can cancel, so a held cue is one whose *holder* is
    /// written here.
    #[must_use]
    pub fn held(self) -> bool {
        matches!(self, Self::Engine | Self::Shield)
    }
}

/// One cue's decoded audio: every waveform its command run binds.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The alternates, in command order, each with **its own** loop flag from
    /// the descriptor's `+0x0e`. Never empty.
    ///
    /// Per waveform rather than per cue, because the flag is per waveform and
    /// several banked cues mix the two: `hud.bnk`'s `~BLOWUP` has one looping
    /// waveform of two and `~AIRBRAKE_MONO` one of three. Every cue this port
    /// currently fires happens to be uniform, so collapsing them would be
    /// invisible today and would silently play a loop as a one-shot the first
    /// time one of those was wired.
    pub waveforms: Vec<(Arc<Sound>, bool)>,
}

/// The sound banks a race needs, decoded and indexed by cue.
#[derive(Debug, Default, Clone)]
pub struct Banks {
    sounds: BTreeMap<Cue, Loaded>,
    /// What loading did, for the race's own report.
    pub report: Vec<String>,
}

impl Banks {
    /// Reads and decodes every cue in [`Cue::ALL`] out of `archives`.
    ///
    /// **Never fails.** A source with no sound banks - Wipeout Pure ships none,
    /// see `docs/formats/pure-status.md` - loads nothing and the race is silent,
    /// which is the honest outcome and is what [`Self::report`] says. A race
    /// that refused to start because a bank was missing would make the audio
    /// work a precondition for every other kind of work in the tree.
    #[must_use]
    pub fn load(archives: &mut Archives, zone: bool) -> Self {
        let mut sounds = BTreeMap::new();
        let mut report = Vec::new();
        let mut blobs: BTreeMap<BankName, Vec<u8>> = BTreeMap::new();

        for cue in Cue::ALL {
            let entry = cue.bank().entry(zone);
            let blob = match blobs.get(&cue.bank()) {
                Some(blob) => blob,
                None => match archives.read_name(entry) {
                    Ok(blob) => blobs.entry(cue.bank()).or_insert(blob),
                    Err(e) => {
                        report.push(format!("sfx: {entry} not read: {e}"));
                        continue;
                    }
                },
            };
            match load_cue(blob, cue) {
                Ok(loaded) => {
                    report.push(format!(
                        "sfx: {} -> {} waveform(s) from {entry}",
                        cue.name(),
                        loaded.waveforms.len()
                    ));
                    sounds.insert(cue, loaded);
                }
                // Deliberately a report line and not a fallback. Nothing is
                // substituted for a cue that will not resolve; it stays silent
                // and says so.
                Err(e) => report.push(format!("sfx: {} not loaded: {e}", cue.name())),
            }
        }
        for line in &report {
            info!("{line}");
        }
        Self { sounds, report }
    }

    /// Whether anything at all decoded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// One waveform for a cue, chosen by `rng` when the cue has alternates.
    ///
    /// `None` when the cue did not load. See the module docs for why the choice
    /// is made here rather than by the bank: the selecting opcode is unread.
    #[must_use]
    pub fn pick(&self, cue: Cue, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
        let loaded = self.sounds.get(&cue)?;
        let index = match u32::try_from(loaded.waveforms.len()) {
            Ok(len) if len > 1 => rng.below(len) as usize,
            _ => 0,
        };
        let (sound, looping) = &loaded.waveforms[index];
        Some((Arc::clone(sound), *looping))
    }
}

/// Resolves one cue in one bank blob and decodes what it binds.
fn load_cue(blob: &[u8], cue: Cue) -> anyhow::Result<Loaded> {
    let bank = sblk::Bank::parse(blob)?;
    let record = bank
        .cue_named(cue.name())
        .ok_or_else(|| anyhow::anyhow!("{:?} names no cue in {}", cue.name(), bank.name))?;
    let sounds = bank.cue_sounds(&record);
    anyhow::ensure!(
        !sounds.is_empty(),
        "{} binds no waveform: its {} command(s) are all opcodes this does not read",
        cue.name(),
        record.commands
    );

    let mut waveforms = Vec::with_capacity(sounds.len());
    for sound in &sounds {
        let data = bank.waveform(sound).ok_or_else(|| {
            anyhow::anyhow!("{} reaches outside the waveform section", cue.name())
        })?;
        waveforms.push((
            Arc::new(Sound::new(
                sblk::decode_adpcm(data),
                1,
                sblk::ASSUMED_SAMPLE_RATE,
            )?),
            sound.mode & LOOP_FLAG != 0,
        ));
    }
    Ok(Loaded { waveforms })
}

/// The descriptor flag `Scream_KeyOnVoice` passes as `sceSasSetVoice`'s loop
/// mode, from `+0x0e`.
const LOOP_FLAG: u16 = 0x40;

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
/// # The unit of `pitch` is inferred, not recovered
///
/// The law is a direct read; what the number *means* is not. It is written to a
/// SCREAM sound instance's `+0x04`, and at a standstill it is about **-1143**,
/// which is 1143 below whatever unity is. Read as **cents** - 1200 to the
/// octave, the near-universal convention - that is a playback ratio of
/// `2^(-1143/1200)` = 0.52 at rest, unity at 228.6 km/h and about 1.23 at 300,
/// which is the shape of an engine note. No other reading of a number near
/// -1143 lands anywhere sensible. Recorded as a hypothesis at confidence
/// **60**, and it is the one thing here a hardware capture would settle in a
/// second.
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

/// Cents to an octave, the unit `base` is read as. See [`Engine`].
const CENTS_PER_OCTAVE: f32 = 1200.0;

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
    pub fn tick(
        &mut self,
        mixer: &mut Mixer,
        banks: &Banks,
        rng: &mut Rng,
        speed_kmh: f32,
        on: bool,
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

        let pitch = (self.pitch / CENTS_PER_OCTAVE).exp2();
        let gain = self.intensity * ENGINE_GAIN_SPAN + ENGINE_GAIN_FLOOR;

        match self.voice {
            Some(id) if mixer.is_playing(id) => {
                mixer.set_pitch(id, pitch);
                mixer.set_gain(id, gain);
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
                    ..Play::looping(sound, Bus::Sfx)
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
