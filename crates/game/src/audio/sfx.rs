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
//! # One caveat, and it is per title
//!
//! Every trigger below was read out of a **PSP Wipeout Pulse** executable.
//! The cue names and the banks are each title's own data - Pure spells them as
//! Pulse does, HD keeps two of them elsewhere, see
//! [`oag_title::SoundBanks`] - but **no Pure or HD dispatch had ever been
//! looked at** until each `Cue` variant's own doc comment started citing a
//! Pure trigger too: eight of the nine now have one, each independently
//! decompiled and structurally corroborated against its Pulse counterpart,
//! confidence 78-82. Only [`Cue::Blowup`] is still unconfirmed on Pure - see
//! its own doc comment - and all nine are still unconfirmed on HD, which is
//! still a bet that games in one series with the same cue names and the same
//! middleware fire them at the same moments. Reasonable, and not a reading;
//! confidence 50, which is
//! below this project's naming threshold and so is
//! written down rather than implied.
//!
//! # Two things here are honest placeholders, and both are load-bearing
//!
//! **The sample rate.** [`oag_formats::sblk::ASSUMED_SAMPLE_RATE`] is not
//! recovered; see its doc comment. Everything below plays at it.
//!
//! **Which waveform of a cue sounds.** `.COLLISIONS` binds fifteen impact
//! samples and the opcode that chooses between them, `0x19`, is decoded on
//! Wipeout HD's binary and corroborated on PSP (confidence 88 both sides;
//! PS2 grouped with PSP/Pure by operand byte layout, not independently
//! checked): [`Banks::pick`] now matches it - a uniform draw that never
//! repeats the immediately previous pick for a cue. That is still an approximation
//! rather than a full reading, not a stand-in for missing data: the fifteen
//! alternates *are* the disc's own audio, all of them within a tenth of a
//! second of each other in length, so playing all fifteen at once - the only
//! reading that needs no choice at all - is the one thing that is certainly
//! wrong. See
//! [`sound.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/sound.md#0x19---alternate-selection-decoded).
//!
//! The same approximation now reaches one level further down on Wipeout HD,
//! where `.COLLISIONS` plays `c_CShipShip` and `c_CShipWall` and each of those
//! has `S`/`M`/`L` children - 112 waveforms once the tree is walked. **The
//! split is visible and is not wired**: which child a parent picks is guarded
//! by opcode `0x22`, whose operand is not decoded, so nothing here pairs
//! ship-against-wall or a severity band with a suffix. The game does know which
//! of those happened, and pairing them off the names would be a mapping
//! invented here rather than one read off the disc - the same refusal this file
//! already makes about `oag_render::sparks::severity`.
//!
//! # Positional audio
//!
//! Every craft is audible, from where it is. The law - linear falloff to a
//! `200`-unit radius, a hard gate past it, the gamma volume curve, and the
//! equal-power pan the disc tabulates - is recovered and lives in
//! [`oag_audio::spatial`]; the listener is the **camera**, which is what the
//! original copies into its sound manager once a frame.
//!
//! What this module adds on top is only the wiring: which slot raised a cue
//! ([`CueEvent`]), which emitter that cue belongs on ([`Cue::placement`]), and
//! where each craft is this tick. Two things are deliberately *not* placed -
//! [`Cue::ShieldActive`], whose call site plays at full volume with pan zero,
//! and music, which has no emitter at all.

use oag_audio::{Bus, Play, VoiceId};
use oag_core::Rng;
use oag_core::math::Vec3;

use super::{Audio, TICK_HZ};

mod announcer;
mod banks;
mod engine;
mod track;
pub use announcer::{Announcer, ClassAnnouncer};
use banks::load_named_cue;
pub use banks::{Banks, Loaded};
pub use engine::Engine;
pub use track::TrackEmitters;

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
    /// One per grid slot, each with its own random note and its own emitter.
    ///
    /// The original builds one in every craft's `ExhaustFlare_Init`, and its
    /// `base` is `rand(-127, 127) - 1143` - a per-craft spread that is audible
    /// as eight engines rather than one played eight times. Drawn from the same
    /// generator in slot order, so a `--dump-audio` capture of one race is the
    /// same capture twice.
    engines: [Engine; oag_gameplay::MAX_SHIPS],
    /// The shield's held voice, while one is up and the alternate drawn was a
    /// loop. See [`Cue::Shield`].
    /// What the lock-on reticle was doing last frame, so its two blips fire on
    /// the transitions rather than every frame. See [`Cue::LockOn`].
    sight: oag_race::sight::State,
    shield: Option<VoiceId>,
    /// Whether the shield has already been responded to for this activation.
    ///
    /// The latch, kept apart from [`Self::shield`] because "a voice is held"
    /// and "this activation has been handled" are different facts and only the
    /// second one may gate the trigger.
    shield_open: bool,
    /// `~BLOWUP`'s held voice, while the player's craft is mid-explosion.
    blowup: Option<VoiceId>,
    /// Whether this explosion has already been responded to, for the same
    /// reason [`Self::shield_open`] exists: the level must arm the voice once.
    blowup_open: bool,
    /// The circuit's own ambience: one held voice per authored emitter that is
    /// currently in range. See [`TrackEmitters`].
    ambience: track::Ambience,
    /// `~PLASMATVL`'s held voice, one per **projectile** slot rather than per
    /// grid slot or per race. See [`Cue::PlasmaTravel`] for why a Plasma bolt
    /// needs this rather than the `Option<VoiceId>` shape [`Self::shield`] and
    /// [`Self::blowup`] use: those are one activation per craft at a time,
    /// and a bolt is neither - it is not the craft, and more than one can be
    /// in the air together.
    ///
    /// Read and written directly off `race.sim.world.projectiles.slots` every
    /// tick in [`Audio::race_tick`], the same way [`Engine`] reads craft
    /// position directly rather than through a queued [`CueEvent`] - a moving
    /// held voice needs *this* tick's position, not a queued one. `None` at
    /// every index a bolt is not occupying; [`oag_gameplay::projectile::MAX_PROJECTILES`]
    /// entries because a slot index is the only stable handle a `Copy` world
    /// snapshot gives a projectile.
    plasma_travel: [Option<VoiceId>; oag_gameplay::projectile::MAX_PROJECTILES],
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
        let announcements = race.drain_announcements();
        let class_announcements = race.drain_class_announcements();
        if race.sounds().is_empty()
            && race.announcer().is_empty()
            && race.class_announcer().is_empty()
            && race.track_emitters().omni.is_empty()
            && race.track_emitters().directional.is_empty()
        {
            return;
        }
        let voices = self.sfx.get_or_insert_with(|| {
            let mut rng = Rng::new(SFX_SEED);
            SfxVoices {
                engines: std::array::from_fn(|_| Engine::new(&mut rng)),
                sight: oag_race::sight::State::Absent,
                shield: None,
                shield_open: false,
                blowup: None,
                blowup_open: false,
                ambience: track::Ambience::default(),
                plasma_travel: [None; oag_gameplay::projectile::MAX_PROJECTILES],
                rng: Rng::new(SFX_SEED),
            }
        });
        let banks = race.sounds();
        let emitters = race.track_emitters();
        let announcer = race.announcer();
        let class_announcer = race.class_announcer();
        let listener = listener_of(race);
        let craft = craft_positions(race);
        // An owned snapshot, the same shape `craft` is - `Projectile` is
        // `Copy` and the array is small, and copying it out avoids holding a
        // borrow of `race` across the mixer closure below. Read after
        // `Race::tick` has already run, so a bolt that ended this tick is
        // already back to `Projectile::default()` here - which is why
        // `Cue::PlasmaHitWall` carries its own impact point on the `CueEvent`
        // rather than being read off this array.
        let projectiles = race.sim.world.projectiles.slots;
        let running = !race.finished();
        let shielded = race.shield_is_up();
        let exploding = race.craft_is_exploding();
        self.output.with_mixer(|mixer| {
            for event in cues {
                // A held cue is not a one-shot and must not be fired as one -
                // `~ENGINE` reaching here would start a second engine every
                // time it was raised.
                if event.cue.held() {
                    continue;
                }
                let Some(pan) = place(event, &listener, &craft) else {
                    // Out of range: the original's `Sound_Play` refuses to open
                    // a voice on an emitter whose out-of-range bit is latched,
                    // so a rival's scrape on the far side of the circuit is
                    // *not started* rather than started silent.
                    continue;
                };
                let Some((sound, looping)) = banks.pick(event.cue, &mut voices.rng) else {
                    continue;
                };
                let bus = event.cue.bus();
                let play = if looping {
                    Play::looping(sound, bus)
                } else {
                    Play::once(sound, bus)
                };
                // The return is dropped deliberately: a one-shot is fired and
                // forgotten, and a refused voice is already counted by
                // `Mixer::starved`.
                let _ = mixer.play(Play {
                    gain: pan.gain,
                    pan: pan.pan,
                    ..play
                });
            }

            // The Zone announcer: one voice line per milestone this tick
            // raised, dry and at full volume like `ShieldActive` - a line in
            // the player's ear, not a thing happening in the world, and there
            // is no emitter to place it on in the first place. `None` covers
            // both "this title has no announcer" and "this title's ladder
            // does not name this particular zone number" - the same silent
            // degrade `Banks::pick` already gives an unresolved cue.
            for milestone in announcements {
                let Some((sound, looping)) = announcer.pick(milestone, &mut voices.rng) else {
                    continue;
                };
                let play = if looping {
                    Play::looping(sound, Bus::Speech)
                } else {
                    Play::once(sound, Bus::Speech)
                };
                let _ = mixer.play(Play {
                    gain: 1.0,
                    pan: None,
                    ..play
                });
            }

            // The Zone speed-class announcer: same shape as the milestone
            // announcer just above, one voice line per stage this tick
            // raised. See `race::tick`'s own comment for the edge this queue
            // is raised on and `oag_title::ZoneClassAnnouncer` for why this
            // trigger, unlike the milestone one, is not yet read from HD's
            // own executable.
            for stage in class_announcements {
                let Some((sound, looping)) = class_announcer.pick(stage, &mut voices.rng) else {
                    continue;
                };
                let play = if looping {
                    Play::looping(sound, Bus::Speech)
                } else {
                    Play::once(sound, Bus::Speech)
                };
                let _ = mixer.play(Play {
                    gain: 1.0,
                    pan: None,
                    ..play
                });
            }

            // `Shield_Activate` opens `~SHIELD` with a handle and the shield's
            // own drop releases it, so this is a *level* rather than an edge:
            // the voice exists exactly while the pickup timer is running.
            // Latched on `shield_open` rather than on the voice handle, so
            // this fires **once per activation** whatever happened to the
            // voice. Keying on `Option<VoiceId>` would re-enter the arm every
            // tick whenever no handle came back - a full voice pool, or a
            // non-looping alternate that is played and not held - and repeat
            // the sound sixty times a second under exactly the conditions that
            // are already going wrong.
            match (shielded, voices.shield_open) {
                (true, false) => {
                    voices.shield_open = true;
                    // **The alternate's own loop flag decides**, not the cue's.
                    // Pulse's `~SHIELD` is two waveforms and both loop; Pure's
                    // is four of which only two do, so forcing `Play::looping`
                    // here would loop a one-shot on about half the draws.
                    if let Some((sound, looping)) = banks.pick(Cue::Shield, &mut voices.rng) {
                        voices.shield = if looping {
                            mixer.play(Play::looping(sound, Cue::Shield.bus()))
                        } else {
                            let _ = mixer.play(Play::once(sound, Cue::Shield.bus()));
                            None
                        };
                    }
                }
                (false, true) => {
                    voices.shield_open = false;
                    if let Some(id) = voices.shield.take() {
                        mixer.stop(id);
                    }
                }
                _ => {}
            }

            // The lock-on reticle's two blips, on the edges of its state rather
            // than as a level: one voice per transition, because this mixer has
            // no equivalent of the original's cue parameter. See [`Cue::LockOn`].
            let sight = race.sight_state();
            if sight != voices.sight {
                let waveform = match sight {
                    oag_race::sight::State::Absent => None,
                    oag_race::sight::State::Seeking => Some(0),
                    oag_race::sight::State::Locked => Some(1),
                };
                // Only forward transitions blip. Falling back from locked to
                // seeking - the target sliding off the nose - would otherwise
                // chatter as the reticle hunted.
                let forward = matches!(
                    (voices.sight, sight),
                    (
                        oag_race::sight::State::Absent,
                        oag_race::sight::State::Seeking
                    ) | (
                        oag_race::sight::State::Seeking,
                        oag_race::sight::State::Locked
                    )
                );
                if let (Some(index), true) = (waveform, forward)
                    && let Some((sound, looping)) = banks.pick_at(Cue::LockOn, index)
                {
                    let play = if looping {
                        Play::looping(sound, Cue::LockOn.bus())
                    } else {
                        Play::once(sound, Cue::LockOn.bus())
                    };
                    let _ = mixer.play(play);
                }
                voices.sight = sight;
            }

            // The explosion, on the same level-and-latch shape the shield uses
            // and for the same reason: case 4 opens a *handle*, so this is a
            // voice's lifetime rather than a one-shot. Dry - it is the player's
            // own craft and the original hands it to the no-emitter path.
            match (exploding, voices.blowup_open) {
                (true, false) => {
                    voices.blowup_open = true;
                    if let Some((sound, looping)) = banks.pick(Cue::Blowup, &mut voices.rng) {
                        voices.blowup = if looping {
                            mixer.play(Play::looping(sound, Cue::Blowup.bus()))
                        } else {
                            let _ = mixer.play(Play::once(sound, Cue::Blowup.bus()));
                            None
                        };
                    }
                }
                (false, true) => {
                    voices.blowup_open = false;
                    if let Some(id) = voices.blowup.take() {
                        mixer.stop(id);
                    }
                }
                _ => {}
            }

            // The Plasma's own travel loop, `~PLASMATVL`, keyed by
            // *projectile* slot - see [`Cue::PlasmaTravel`] and
            // [`SfxVoices::plasma_travel`]'s own doc comments for why this
            // cannot be the shield/blowup shape above. Read directly off this
            // tick's own projectile array rather than off a queued
            // `CueEvent`, for the reason the craft's own engines below are.
            for (slot, projectile) in projectiles.iter().enumerate() {
                let flying = projectile.kind == Some(oag_tables::weapons::Weapon::Plasma)
                    && projectile.charge <= 0.0;
                match (flying, voices.plasma_travel[slot]) {
                    // The rising edge: charge just reached zero this tick -
                    // see `Projectiles::advance`'s charging branch, which is
                    // the one place this port's own release edge lives.
                    (true, None) => {
                        if let Some((sound, looping)) =
                            banks.pick(Cue::PlasmaTravel, &mut voices.rng)
                            && looping
                        {
                            let placed = oag_audio::Emitter::craft(projectile.position.to_array())
                                .place(&listener, 1.0);
                            let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));
                            voices.plasma_travel[slot] = mixer.play(Play {
                                gain,
                                pan,
                                ..Play::looping(sound, Cue::PlasmaTravel.bus())
                            });
                        }
                        // Else either nothing loaded or the bank says this
                        // waveform is not a loop - the same guard
                        // `Engine::tick` carries for `~ENGINE`. `~PLASMATVL`
                        // reads looping in every corpus checked so far
                        // (`oag-wad sounds`), so the second case is
                        // defensive rather than expected to fire.
                    }
                    // Still flying and still held: follow the bolt.
                    (true, Some(id)) if mixer.is_playing(id) => {
                        let placed = oag_audio::Emitter::craft(projectile.position.to_array())
                            .place(&listener, 1.0);
                        let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));
                        mixer.set_gain(id, gain);
                        mixer.set_pan(id, pan);
                    }
                    // The pool reclaimed the voice before the bolt itself
                    // ended - starved, not stopped. Forget the stale handle so
                    // a later tick does not stop whatever slot the pool gave
                    // it to next.
                    (true, Some(_)) => voices.plasma_travel[slot] = None,
                    // The falling edge: the bolt is no longer flying, whether
                    // because it just ended (`Projectile::default()` already
                    // reset `kind`) or - unreachably today, since nothing
                    // re-charges a flying bolt - because it started charging
                    // again. Either way the loop stops here; the one-shot
                    // `PLASMAHITWALL` for an actual ending is a separate
                    // `CueEvent` carrying its own impact point, pushed from
                    // `crates/game/src/race/tick.rs`.
                    (false, Some(id)) => {
                        mixer.stop(id);
                        voices.plasma_travel[slot] = None;
                    }
                    (false, None) => {}
                }
            }

            // Every craft's engine, each off its own emitter. A craft with no
            // pose is one the race never spawned; it is skipped rather than
            // placed at the origin, which would put eight engines in a heap
            // under the start line.
            for (slot, engine) in voices.engines.iter_mut().enumerate() {
                let Some(&(position, speed)) = craft.get(slot).and_then(Option::as_ref) else {
                    engine.stop(mixer);
                    continue;
                };
                engine.tick(
                    mixer,
                    banks,
                    &mut voices.rng,
                    speed * oag_render::exhaust::SPEED_TO_KMH,
                    running,
                    position.to_array(),
                    &listener,
                    slot != 0,
                    DT,
                );
            }

            // The circuit's own ambience, from the same ears and the same law.
            // **After the craft**, so that on a starved pool the race's own
            // cues have already taken their voices - the original's pool is
            // hardware and this one is not, so the order is this project's
            // choice and is written down as one. Measured on the two circuits
            // `track_audio_ground_truth` flies, the peak is eight against
            // `oag_audio::mixer::MAX_VOICES`, so it has not yet mattered.
            voices
                .ambience
                .tick(mixer, emitters, &listener, &mut voices.rng);
        });
    }

    /// How many of the circuit's own emitters are sounding right now.
    ///
    /// Separate from `Mixer::active_voices`, which counts the race's cues too:
    /// what this answers is "is the circuit audible", which is the claim
    /// `crates/game/tests/track_audio_ground_truth.rs` makes and the one a
    /// silent-ambience regression would break without changing any other count.
    #[must_use]
    pub fn ambient_voices(&self) -> usize {
        self.sfx.as_ref().map_or(0, |voices| {
            self.output
                .with_mixer(|mixer| voices.ambience.playing(mixer))
        })
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
                for engine in &mut voices.engines {
                    engine.stop(mixer);
                }
                if let Some(id) = voices.shield.take() {
                    mixer.stop(id);
                }
                voices.shield_open = false;
                if let Some(id) = voices.blowup.take() {
                    mixer.stop(id);
                }
                voices.blowup_open = false;
                for voice in &mut voices.plasma_travel {
                    if let Some(id) = voice.take() {
                        mixer.stop(id);
                    }
                }
                voices.ambience.stop(mixer);
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
    /// The archive entry to read, given a title's own table and whether this is
    /// a Zone race.
    ///
    /// **Which file a cue is in is a per-title fact and the cue's name is not.**
    /// All three titles spell `SPEEDUPPAD` and `.COLLISIONS` identically; HD
    /// keeps the first in `weapons.bnk` because it has no `hud.bnk` at all, and
    /// the second in `shiphd.bnk`. See [`oag_title::SoundBanks`].
    ///
    /// Only [`Self::Ship`] moves with `zone`, and on Pulse and Pure that is
    /// load-bearing: `SHIP_ZM` is a different bank with the same cue names and
    /// different audio - a nine-layer `~ENGINE` against one, ten collision
    /// alternates against fifteen - so a Zone race reading `ship.bnk` would be
    /// quietly playing the wrong craft. HD ships no separate Zone bank and its
    /// table says so by repeating itself.
    #[must_use]
    pub fn entry(self, banks: &oag_title::SoundBanks, zone: bool) -> &'static str {
        match self {
            Self::Hud => banks.hud,
            Self::Ship if zone => banks.ship_zone,
            Self::Ship => banks.ship,
            Self::Weapons => banks.weapons,
            Self::Speech => banks.speech,
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
    ///
    /// Pure's `FUN_0886b548` is the same two-branch (positional/dry)
    /// dispatcher in one function, firing the same `SPEEDUPPAD` on either
    /// branch. `docs/ghidra/functions/psp-pure-usa/dry-play-cues.md`,
    /// confidence 78.
    SpeedupPad,
    /// Hull against wall or track.
    ///
    /// `ShipCollisionFx_Trigger` (`0x089246b4`) fires it "once per surviving
    /// kind-0/1 call" - that is, once per contact that gets past the 0.8-second
    /// spark cooldown - so it rides the same gate the collision sparks do.
    /// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`, confidence 85.
    ///
    /// Pure's own `ShipCollisionFx_Trigger` (`0x0888e340`) does the same thing:
    /// same `.COLLISIONS` cue (confirmed present in its `SHIP_CL`/`SHIP_ZM`
    /// banks), same point in the same 0.8-second cooldown gate.
    /// `docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`,
    /// confidence 82 - the only one of the nine cues checked on a second
    /// title so far; the module doc's confidence-50 bet still covers the rest.
    Collision,
    /// A contact a raised shield absorbed.
    ///
    /// `FUN_08840640`, the shield-absorb ability's own effect, "plays an
    /// `ABSORB` sound once" before staggering its ten spark instances. Same
    /// page. This is the sound of the contact the shield *ate*, which is why it
    /// fires exactly where the sparks are suppressed.
    ///
    /// Pure's own counterpart (`FUN_08925e20`) does the same thing - one
    /// `Sound_Play` of the same undotted `ABSORB`, then a stagger loop into
    /// `ShipCollisionFx_Trigger` - with one real difference: the loop runs
    /// eight times, not ten. `docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`,
    /// confidence 80.
    Absorb,
    /// The engine, held for as long as the craft is running.
    ///
    /// `Exhaust_UpdateEngineSound` (`0x08904cf4`) opens the voice in its
    /// constructor and writes pitch and volume to it every tick;
    /// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`, confidence 80. The law
    /// is in [`Engine`].
    ///
    /// Pure's `ExhaustFlare_Init` opens the same `~ENGINE` voice, and two of
    /// its tuning constants (the `-1143.0` base pitch, the `0.01` lerp rate)
    /// match Pulse's bit-for-bit, not just structurally.
    /// `docs/ghidra/functions/psp-pure-usa/exhaust-sound.md`, confidence 82 -
    /// the per-tick pitch/volume write itself is unchecked on Pure's side.
    Engine,
    /// The shield, held for as long as it is up.
    ///
    /// `Shield_Activate` (`0x0883e544`) calls
    /// `Sound_PlayLooping(1.0, entity->0x50, ..., "~SHIELD", entity + 0x54)`,
    /// keeping the handle - so it runs for the pickup's duration and is
    /// released when the shield drops.
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    ///
    /// Pure's `FUN_0892425c` fires the same `~SHIELD` at the same point in the
    /// same two-cue order (after `ShieldActive`, below), with a matching
    /// handle-out-slot call shape. `docs/ghidra/functions/psp-pure-usa/shield-sound.md`,
    /// confidence 80.
    Shield,
    /// The announcer, on the same activation.
    ///
    /// The other half of `Shield_Activate`'s pair, one line above [`Self::Shield`]:
    /// `Sound_Play(entity, ..., "shieldactive", 0x400, 0)`. It lives in
    /// `speech.bnk` rather than `weapons.bnk`, which is what says it is a voice
    /// line and not an effect.
    ///
    /// Pure's counterpart fires the same undotted `shieldactive` at the same
    /// volume (`0x400`), first of the pair - but through a deeper call chain
    /// than Pulse's flat one-hop dry helper, unread past confirming it looks
    /// like sound-engine code. `docs/ghidra/functions/psp-pure-usa/shield-sound.md`,
    /// confidence 80.
    ShieldActive,
    /// The announcer, one second before an Autopilot pickup lets go.
    ///
    /// `Autopilot_Update` (`0x08861404`) plays it on the tick the remaining
    /// time crosses `1.0` - an edge it keeps `craft+0x144` for - through the
    /// **dry, full-volume** path rather than through the craft's emitter, which
    /// is what says it is a voice line in the player's ear rather than a thing
    /// happening in the world. `docs/ghidra/functions/psp-pulse-usa/autopilot.md`,
    /// confidence 85.
    ///
    /// **Its two partners on the disc are deliberately unwired**: `~AUTOPILOT`
    /// in `hud.bnk` is a held loop the handler *stops* and no located code
    /// starts, and `autopilot_eng` sits beside this one in `speech.bnk` with no
    /// call site at all. An effect whose trigger is not recovered stays silent.
    ///
    /// Pure's `FUN_0884c794` fires the same undotted `disengaging` through the
    /// same dry chain `ShieldActive` uses, at the same volume.
    /// `docs/ghidra/functions/psp-pure-usa/dry-play-cues.md`, confidence 78 -
    /// the countdown threshold itself is unread on Pure's side.
    Disengaging,
    /// The player's own craft blowing up.
    ///
    /// `Ship_SetState`'s case 4 (`0x0884430c`, reached through the nine-entry
    /// jump table at `0x08a7bc18`) arms the `0.5 s` state timer and, in the
    /// same breath, plays `~BLOWUP` through `FUN_0883e9b0` - the **dry,
    /// no-emitter** path at volume `0x400` - keeping the handle at
    /// `craft+0xcac`. It is therefore held, and it is the player's alone:
    /// `FUN_0883e9b0` returns without playing when `craft+0x368` is non-zero.
    /// See [`zone-mode.md`](../../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md),
    /// confidence 85.
    ///
    /// **An opponent's destruction plays nothing here**, and that is the
    /// reading rather than a gap in it - whatever an opponent's explosion
    /// sounds like comes from somewhere this pass did not find.
    ///
    /// **The one cue of nine with no confirmed Pure trigger.** The `~BLOWUP`
    /// string exists in Pure's executable and the cue exists on its disc,
    /// but its call site was not found - six search methods that found every
    /// other cue this thread chased all came up empty here.
    /// `docs/ghidra/functions/psp-pure-usa/blowup-sound-open.md` records
    /// what was tried, so a future pass does not repeat it.
    Blowup,
    /// The lock-on reticle, seeking and then locked.
    ///
    /// `HudSight_UpdateTone` (`0x0881b34c`) opens **one** `~ROCKLOCK` voice the
    /// first frame the reticle has anything, keeps the handle, and switches a
    /// parameter between `0` while it is seeking and `1` once it has locked -
    /// stopping the voice only when the target goes away. See
    /// [`lock-sight.md`](../../../../docs/ghidra/functions/psp-pulse-usa/lock-sight.md),
    /// confidence 85.
    ///
    /// **The bank agrees with the reading**: `~ROCKLOCK` lives in `hud.bnk`
    /// beside `SPEEDUPPAD` and `~BLOWUP`, and it binds exactly **two**
    /// waveforms, neither looping, 0.11 s apiece - which is what a parameter
    /// with two values selects between.
    ///
    /// **This port fires them as two edges rather than as one parameterised
    /// voice**, because this mixer has no cue parameters: waveform `0` on
    /// entering [`oag_race::sight::State::Seeking`] and waveform `1` on
    /// entering [`oag_race::sight::State::Locked`]. With two 0.11 s
    /// non-looping waveforms the audible result is the same pair of blips; what
    /// is lost is the original's ability to switch mid-voice, which at that
    /// length it never gets to use. Recorded rather than smoothed over.
    ///
    /// **Which waveform is which is inference, at 55.** What is read is that the
    /// parameter takes `0` while seeking and `1` once locked; that those values
    /// index the cue's two waveforms *in that order* is the obvious reading,
    /// not one taken off the bank's command list. This is a different gap from
    /// the one [`Banks::pick`] closes: `0x19` decodes which *randomly-chosen*
    /// alternate a multi-waveform cue plays, and `LockOn`'s two waveforms are
    /// never reached that way - `pick_at` selects between them by the
    /// seeking/locked parameter above, a mapping no command-list reading would
    /// recover either way. If the two turn out to be the other way round, the
    /// seeking blip and the lock chime are swapped and nothing else changes.
    /// `--sound` writes a WAV and settles it by ear.
    ///
    /// Pure's own `HudSight_UpdateTone` is a near line-for-line match: same
    /// three-state toggle, same `0x400` volume, same choice to call the
    /// dry-play chain's middle hop directly rather than through its gate
    /// helper. `docs/ghidra/functions/psp-pure-usa/lockon-sound.md`,
    /// confidence 82.
    LockOn,
    /// A mine leaving the back of a craft, one per charge of a cluster.
    ///
    /// `Weapon_DropMines` (`0x088675cc`) calls `Mine_Init` (`0x08859ac8`) once
    /// per charge; `Mine_Init` ends with two cue plays, and the first is
    /// `MINELAUNCH`. Both call sites were decompiled directly: the emitter
    /// argument `Weapon_DropMines` hands `Mine_Init` traces, through two
    /// pointer hops off the subsystem's per-craft slot, to `+0x50` - the same
    /// offset [`Self::Collision`] and [`Self::Shield`] read as the firing
    /// craft's own emitter. See
    /// [mine.md](../../../../docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-06-minelaunch-is-a-plain-positional-craft-emitter-cue---mineradar-is-not),
    /// confidence 78 (two hops of indirection, short of the single-hop reads'
    /// 82-85).
    ///
    /// **`MINERADAR`, the second cue of the same pair, is deliberately not
    /// here.** It anchors to a *new emitter `Mine_Init` allocates for the mine
    /// entity itself*, a held, per-projectile voice - a shape nothing in
    /// [`CueEvent`] or [`SfxVoices`] can address, since every held voice this
    /// engine plays is keyed by grid slot. `BOMBLAUNCH`/`~BOMBRADAR` also stay
    /// unwired: `Weapon_FireBomb` (`0x08863a20`) was decompiled end to end and
    /// never calls the play function at all. See `mine.md`'s own section for
    /// both.
    MineLaunch,
    /// The Plasma's wind-up, at the press rather than at release.
    ///
    /// `Plasma_Init` (`0x0885bd18`, confidence 90) plays it directly off the
    /// **firing craft's own emitter**, not the bolt's - `Sound_Play(1.0,
    /// craft->emitter, ..., "PLASMA", 0)` runs before the bolt's own emitter
    /// is even constructed a few lines later in the same function
    /// (`SoundEmitter_Init` on a fresh `0x70`-byte allocation, stored at the
    /// bolt's `+0x5c`). Decompiled directly 2026-09-16 to settle exactly this:
    /// see [plasma.md](../../../../docs/ghidra/functions/psp-pulse-usa/plasma.md#plasma_init-0x0885bd18-plays-plasma-and-wo_plasma_head).
    /// So this fires at the press, the same tick [`oag_gameplay::projectile::plasma::CHARGE_SECONDS`]'s
    /// wind-up starts - not at release, which is [`Self::PlasmaTravel`]'s edge.
    /// Bank data confirms it as a plain one-shot: `oag-wad sounds` reports
    /// `PLASMA` with 3 waveforms and 0 looping, in `weapons.bnk`.
    Plasma,
    /// The Plasma bolt's own travel loop, from release to whatever ends it.
    ///
    /// `Plasma_Launch` (`0x0885bf84`, confidence 90) starts it -
    /// `Sound_Play(1.0, p->emitter, ..., "~PLASMATVL", &p->pose)` - on the
    /// bolt's **own** emitter, the one `Plasma_Init` constructed and pointed
    /// at the bolt's own matrix (`p->emitter->node = &p->matrix`, read
    /// directly off `Plasma_Init`'s decompile 2026-09-16) rather than at any
    /// craft's. That is why this needs its own placement: the bolt is heard
    /// from wherever it actually is, which is not the firing craft's emitter
    /// [`Self::Plasma`] plays from a moment earlier. Bank data confirms the
    /// loop bit: `oag-wad sounds` reports `~PLASMATVL` as one waveform, 1
    /// looping, in `weapons.bnk`.
    ///
    /// **Held per *projectile* slot, not per grid slot** - the gap
    /// `docs/../mine.md`'s own `MINERADAR` note names as "nothing in
    /// `CueEvent` or `SfxVoices` can address... every held voice this engine
    /// plays is keyed by grid slot". This is the first cue built against that
    /// gap: [`SfxVoices::plasma_travel`] is an array of
    /// [`oag_gameplay::projectile::MAX_PROJECTILES`] voice handles, read and
    /// written directly off the world's own projectile array every tick -
    /// the same reason [`Engine`] reads craft position directly rather than
    /// through a queued [`CueEvent`]. Never pushed through the cue queue
    /// itself; [`Self::held`] returns `true` for it so a stray push would be
    /// silently dropped rather than mis-played as a one-shot.
    PlasmaTravel,
    /// Whatever ends a Plasma bolt's flight - a wall, a craft, or the 10 s
    /// timeout.
    ///
    /// `Plasmas_Update`'s teardown pass (`0x0886b490`, confidence 90) plays it
    /// unconditionally - `Psys_Release_q`, `Plasma_SpawnDetonation`,
    /// `Sound_Play(1.0, p->emitter, ..., "PLASMAHITWALL", 0)`, nothing else -
    /// for a wall hit and the `10.0 < age` timeout alike; re-read at
    /// instruction level 2026-09-16 with no elision. On the bolt's own
    /// emitter, at wherever it stopped - see [`Self::PlasmaTravel`] for why
    /// that is not the firing craft.
    ///
    /// **This port's own Plasma can also end on a craft**
    /// (`Impact::struck.is_some()`, `crates/gameplay/src/projectile/flight.rs`'s
    /// shared sweep-segment test), a third ending `plasma.md`'s reading never
    /// located a call site for - `Plasma_Update`'s own decompiled switch only
    /// covers the downward probe (wall/floor/none), and the travel-segment
    /// sweep against a craft is elided in the same page as "the same
    /// collision test again". The bank carries a **distinct** `PLASMAHITSHIP`
    /// cue neither this variant nor any other reads - confirmed present in
    /// `weapons.bnk` by `oag-wad sounds`, and named as the HD/Omega craft-hit
    /// cue's PSP counterpart in this thread's own cross-title table. Playing
    /// `PLASMAHITWALL` for a craft hit too is therefore **chosen, not
    /// measured**: the honest alternative to inventing which cue a craft hit
    /// actually plays is to reuse the one ending that *is* confirmed rather
    /// than guess at the other, and say so here rather than silently. See the
    /// `weapons-eight-of-thirteen-the-plasma-and-the` handover thread.
    ///
    /// **Placed at the impact point, not at a craft** - see [`Placement::Point`]
    /// and [`CueEvent::at_point`]. Bank data: `oag-wad sounds` reports
    /// `PLASMAHITWALL` as 4 waveforms, 0 looping.
    PlasmaHitWall,
}

impl Cue {
    /// Every cue this port fires, which is every one it knows how to load.
    pub const ALL: [Self; 13] = [
        Self::SpeedupPad,
        Self::Collision,
        Self::Absorb,
        Self::Engine,
        Self::Shield,
        Self::ShieldActive,
        Self::Disengaging,
        Self::Blowup,
        Self::LockOn,
        Self::MineLaunch,
        Self::Plasma,
        Self::PlasmaTravel,
        Self::PlasmaHitWall,
    ];

    /// The bank the cue is looked up in.
    #[must_use]
    pub fn bank(self) -> BankName {
        match self {
            Self::SpeedupPad | Self::Blowup | Self::LockOn => BankName::Hud,
            Self::Collision | Self::Engine => BankName::Ship,
            Self::Absorb
            | Self::Shield
            | Self::MineLaunch
            | Self::Plasma
            | Self::PlasmaTravel
            | Self::PlasmaHitWall => BankName::Weapons,
            Self::ShieldActive | Self::Disengaging => BankName::Speech,
        }
    }

    /// Which mix bus this cue's voice belongs on.
    ///
    /// **Read straight off [`Self::bank`], because the bank is where the
    /// original draws the line.** `shieldactive` sits in `speech.bnk` and not
    /// in `weapons.bnk` beside the `~SHIELD` loop it fires with, and that is
    /// already this module's own reason for calling it a voice line rather
    /// than an effect - see [`Self::ShieldActive`]. `Disengaging` agrees from
    /// the other direction: `Autopilot_Update` plays it through the dry,
    /// full-volume path instead of the craft's emitter, which is a line in the
    /// player's ear rather than a thing happening in the world.
    ///
    /// So a cue moves bus by moving bank, and nothing here holds a second list
    /// that can disagree with `bank()`. See
    /// [ADR-0027](../../../../docs/architecture/adr/0027-three-mix-buses.md).
    #[must_use]
    pub fn bus(self) -> Bus {
        match self.bank() {
            BankName::Speech => Bus::Speech,
            BankName::Hud | BankName::Ship | BankName::Weapons => Bus::Sfx,
        }
    }

    /// The string to look up in that bank's name table.
    ///
    /// # The dot in `.COLLISIONS` is the executable's own, not a lookup fix-up
    ///
    /// **Correction, 2026-09-04**: this section used to read the executable as
    /// passing bare `"COLLISIONS"` and claimed the leading dot was added here
    /// to bridge a SCREAM sound/child-sound naming split, at confidence 70.
    /// That was a misread of the call site - `ShipCollisionFx_Trigger`'s
    /// string argument is not built in place the way its three spark names
    /// are; it is loaded indirectly out of a small pointer table
    /// (`0x08924854: lw a3,0x46ec(a2)`), and the decompiler's inline literal
    /// showed the table slot's own apparent text, not the string the pointer
    /// it holds actually points at. Reading that pointer's target directly
    /// (`0x08a886e0`) gives `.COLLISIONS\0` - the dot is already in the
    /// executable's own data. See the correction paragraph in
    /// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
    ///
    /// So this is a plain, exact match: no bridging, no SCREAM child-sound
    /// theory needed, just the same name the game itself passes. It happens
    /// to be the only cue named `.COLLISIONS` on any disc, and Wipeout HD's
    /// bank independently confirms the dot is a plain naming convention, not
    /// a parent/child marker: `.COLLISIONS` is the **parent**, and the cues
    /// it plays are the plainly named `c_CShipShip` and `c_CShipWall`. See
    /// [`oag_formats::sblk::child`].
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::SpeedupPad => "SPEEDUPPAD",
            Self::Collision => ".COLLISIONS",
            Self::Absorb => "ABSORB",
            Self::Engine => "~ENGINE",
            Self::Shield => "~SHIELD",
            Self::ShieldActive => "shieldactive",
            Self::Disengaging => "disengaging",
            Self::Blowup => "~BLOWUP",
            Self::LockOn => "~ROCKLOCK",
            Self::MineLaunch => "MINELAUNCH",
            Self::Plasma => "PLASMA",
            Self::PlasmaTravel => "~PLASMATVL",
            Self::PlasmaHitWall => "PLASMAHITWALL",
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
    ///
    /// [`Self::PlasmaTravel`] joins this list held **per projectile slot**
    /// rather than per grid slot or as a single race-wide handle - see
    /// [`SfxVoices::plasma_travel`]. It is still never pushed through the cue
    /// queue, so this only guards against a stray push being mis-played.
    #[must_use]
    pub fn held(self) -> bool {
        matches!(
            self,
            Self::Engine | Self::Shield | Self::Blowup | Self::PlasmaTravel
        )
    }

    /// Which emitter the original plays this cue on.
    ///
    /// Read off each call site's first argument to `Sound_Play`, which is the
    /// emitter record and nothing else - see
    /// [`positional-audio.md`](../../../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md).
    /// A cue whose call site has not been read stays [`Placement::Unplaced`]
    /// rather than being placed somewhere plausible.
    #[must_use]
    pub fn placement(self) -> Placement {
        match self {
            // `lw a0, 0x50(a0)` at `0x08924844`, inside
            // `ShipCollisionFx_Trigger` - the craft's own emitter, with no
            // branch, so the player's hull is positional too.
            Self::Collision => Placement::Craft,
            // `FUN_08840640` is the absorb effect and sits on the same craft;
            // its emitter argument was not disassembled, so this rides
            // `Collision`'s reading of the contact path rather than its own.
            Self::Absorb => Placement::Craft,
            // `Shield_Activate`'s own doc comment above: `Sound_PlayLooping(1.0,
            // entity->0x50, ...)`, the same `+0x50` the collision path uses.
            Self::Shield => Placement::Craft,
            // Two branches, and which is taken splits on `racer+0x368` -
            // `ExhaustFlare_OnSpeedupPad` (`0x08904f74`) plays positionally off
            // `craft+0x50` on one side and dry at volume `0x400` through
            // `FUN_0883e9b0` on the other. See [`Placement::CraftUnlessPlayer`].
            Self::SpeedupPad => Placement::CraftUnlessPlayer,
            // `ExhaustFlare_Init` gives the note its own emitter at a quarter
            // of the craft radius, and [`Engine`] holds the voice, so this
            // never reaches the one-shot path.
            Self::Engine => Placement::Engine,
            // Recorded as `Sound_Play(entity, ..., "shieldactive", 0x400, 0)` -
            // full volume and pan zero, which is the shape of the *non*-emitter
            // path. Left dry, which is also what this port has always done.
            //
            // `Disengaging` joins it on a stronger footing: its call site is
            // read, and it is `FUN_0883e9b0` -> `FUN_0893a768`, the path that
            // takes no emitter at all.
            Self::ShieldActive | Self::Disengaging => Placement::Unplaced,
            // Read, not assumed: case 4 hands it to the path that takes no
            // emitter and a volume of `0x400`.
            Self::Blowup => Placement::Unplaced,
            // `HudSight_UpdateTone` opens it with a volume of `0x400` and no
            // emitter argument, which is the same dry, full-volume shape - and
            // it is a HUD sound about the player's own reticle rather than a
            // thing happening somewhere in the world.
            Self::LockOn => Placement::Unplaced,
            // Traces to the firing craft's own `+0x50` - see this variant's
            // own doc comment - so it rides the craft, not the mine.
            Self::MineLaunch => Placement::Craft,
            // `Plasma_Init` plays this off the argument it was handed
            // directly - the firing craft's own emitter - before it ever
            // constructs the bolt's own. See this variant's own doc comment.
            Self::Plasma => Placement::Craft,
            // Both ride the bolt's own emitter, which `Plasma_Init`
            // constructs pointed at the bolt's own matrix rather than any
            // craft's - see each variant's own doc comment. Neither is routed
            // through this function's `craft` slice: [`Self::PlasmaTravel`]
            // is a bespoke per-tick tracker keyed by projectile slot, and
            // [`Self::PlasmaHitWall`] carries its own point on the
            // [`CueEvent`] rather than a grid slot.
            Self::PlasmaTravel | Self::PlasmaHitWall => Placement::Point,
        }
    }
}

/// The listener, off the camera the frame is actually drawn from.
///
/// `SoundManager_Update` (`0x0893a2b0`) copies the active camera's rotation
/// rows and the negation of its `+0x70` into the sound manager once a frame.
/// The negation is there because the camera stores a *negated* eye beside a
/// world-to-camera rotation whose world axes are its columns - a split
/// `docs/.../camera.md` measured from the rendering side and this reads back
/// from the audio side. Inverting `Race::view` recovers both halves at once:
/// the camera's world matrix, whose translation is the eye and whose first
/// column is the right axis the pan projects onto.
///
/// `pub` because a circuit's own emitters are placed against the same ears the
/// craft cues are, and the two must not be allowed to disagree about where the
/// listener is - see [`TrackEmitters`].
pub fn listener_of(race: &crate::race::Race) -> oag_audio::Listener {
    let camera = race.view().inverse();
    oag_audio::Listener {
        position: camera.w_axis.truncate().to_array(),
        right: camera.x_axis.truncate().normalize_or_zero().to_array(),
    }
}

/// Every live craft's world position and speed, indexed by grid slot.
///
/// [`None`] for a slot the race did not field. Read once per tick rather than
/// per cue, because eight cues from one craft must all agree on where it was.
fn craft_positions(race: &crate::race::Race) -> [Option<(Vec3, f32)>; oag_gameplay::MAX_SHIPS] {
    std::array::from_fn(|slot| {
        let ship = race.sim.world.ships.get(slot)?;
        (slot < race.ship_count() as usize && ship.active).then(|| {
            (
                ship.physics.body.position,
                ship.physics.body.linear_velocity.length(),
            )
        })
    })
}

/// Where one cue is heard from, or [`None`] if it is out of range entirely.
///
/// The `Option<f32>` inside is the mixer's own "no position" - see
/// [`oag_audio::mixer::Play::pan`]. So this returns three answers, not two:
/// placed, placed-with-no-position, and refused.
fn place(
    event: CueEvent,
    listener: &oag_audio::Listener,
    craft: &[Option<(Vec3, f32)>],
) -> Option<Placed> {
    let dry = Placed {
        gain: 1.0,
        pan: None,
    };
    if event.cue.placement() == Placement::Point {
        // An arbitrary world point rather than a craft's own emitter - see
        // [`Placement::Point`]. `None` here means the event was built wrong
        // (a `Point` cue without [`CueEvent::at_point`]), not that the point
        // is out of range; that refusal still comes from `place` below,
        // exactly as it does for a craft.
        let point = event.point?;
        let placed = oag_audio::Emitter::craft(point.to_array()).place(listener, 1.0)?;
        return Some(Placed {
            gain: placed.gain,
            pan: Some(placed.pan),
        });
    }
    let emitter = match event.cue.placement() {
        Placement::Unplaced => return Some(dry),
        // The player's own branch, on a hypothesis the type documents.
        Placement::CraftUnlessPlayer if event.is_player() => return Some(dry),
        Placement::Craft | Placement::CraftUnlessPlayer => oag_audio::Emitter::craft,
        Placement::Engine => oag_audio::Emitter::engine,
        Placement::Point => unreachable!("handled above"),
    };
    // A cue from a slot with no craft is not a cue: dropped rather than played
    // dry, because the alternative is a rival's collision arriving at full
    // volume out of nowhere the moment a slot is emptied.
    let (position, _) = (*craft.get(usize::from(event.slot))?)?;
    let placed = emitter(position.to_array()).place(listener, 1.0)?;
    Some(Placed {
        gain: placed.gain,
        pan: Some(placed.pan),
    })
}

/// A gain and an optional stereo position, in the shape [`Play`] wants.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placed {
    gain: f32,
    pan: Option<f32>,
}

/// Which emitter a cue is heard from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// The craft's own emitter, [`oag_audio::Emitter::CRAFT_RADIUS`] wide.
    ///
    /// `Craft_Construct_q` allocates it, points it at the ship's scene node and
    /// never overrides the default radius, so this is every craft on the grid
    /// including the player's.
    Craft,
    /// The engine flare's emitter, a quarter as wide.
    Engine,
    /// The craft's emitter for an opponent; dry at full volume for the player.
    ///
    /// **This one rides a hypothesis and says so.** The original branches on
    /// `racer+0x368`, a field
    /// [`pads.md`](../../../../docs/ghidra/functions/psp-pulse-usa/pads.md)
    /// records at confidence **45** as *probably* "is the local player" - below
    /// the naming threshold, on three converging uses. Positional audio adds
    /// two more: the non-positional branch plays at volume `0x400`, the
    /// maximum, with pan zero, which is exactly what a player's own sound
    /// wants; and `Exhaust_UpdateEngineSound` scales the engine by `0.85` when
    /// the same field is *set*, which is what a mix does to everybody else. If
    /// that field turns out to mean something else, this is the line that moves.
    CraftUnlessPlayer,
    /// No emitter has been read for this cue, so it is played dry.
    Unplaced,
    /// An arbitrary world point, carried on the [`CueEvent`] itself rather
    /// than read off a craft.
    ///
    /// **Ours, not read off any call site** - the original always names an
    /// *emitter*, an object with a scene node, never a bare position; this is
    /// what a caller reaches for when the thing making the noise is not a
    /// craft and has no emitter this port models. The Plasma bolt is the
    /// first user: `Plasma_Init` constructs the bolt its **own** emitter,
    /// pointed at the bolt's own matrix rather than any craft's - see
    /// [`Cue::PlasmaTravel`] and [`Cue::PlasmaHitWall`]. [`CueEvent::at_point`]
    /// is the entry point, and [`Cue::PlasmaTravel`]'s own held voice is
    /// placed the same way by its own bespoke tracker rather than through
    /// this enum's normal draining path - see [`SfxVoices::plasma_travel`].
    /// Radius is [`oag_audio::Emitter::CRAFT_RADIUS`], because neither
    /// `Plasma_Init` nor `Plasma_Launch` ever writes the bolt's own emitter's
    /// `+0x38`, so it keeps `SoundEmitter_Init`'s default - the same one the
    /// craft's own emitter never overrides either.
    Point,
}

/// A cue and the craft that raised it.
///
/// **Which craft is not bookkeeping this port invented.** The original's
/// `Sound_Play` takes an emitter as its first argument and every one of these
/// cues passes the craft's own, so the slot is the part of the call this port
/// used to be throwing away - which is why only the player was ever audible.
///
/// [`Self::point`] is the same idea for [`Placement::Point`]: a cue not tied
/// to any craft carries its own position instead of a slot. `#[derive(Eq)]`
/// is deliberately not here any more - `Vec3` holds `f32` and does not
/// implement it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CueEvent {
    /// What to play.
    pub cue: Cue,
    /// The grid slot whose emitter plays it. Slot 0 is the player.
    ///
    /// Meaningless for a [`Placement::Point`] cue built with
    /// [`Self::at_point`] - left at `0` there, and nothing reads it for one.
    pub slot: u8,
    /// The world position a [`Placement::Point`] cue plays at, or [`None`]
    /// for every other placement.
    pub point: Option<Vec3>,
}

impl CueEvent {
    /// A cue from `slot`.
    #[must_use]
    pub fn new(cue: Cue, slot: usize) -> Self {
        Self {
            cue,
            slot: u8::try_from(slot).unwrap_or(u8::MAX),
            point: None,
        }
    }

    /// A cue at `point` rather than at a craft - see [`Placement::Point`].
    #[must_use]
    pub fn at_point(cue: Cue, point: Vec3) -> Self {
        Self {
            cue,
            slot: 0,
            point: Some(point),
        }
    }

    /// Whether the player raised it.
    #[must_use]
    pub fn is_player(self) -> bool {
        self.slot == 0
    }
}

#[cfg(test)]
mod tests;
