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
//! looked at** until [`Cue::Collision`], [`Cue::Absorb`], [`Cue::Shield`],
//! [`Cue::ShieldActive`], [`Cue::SpeedupPad`] and [`Cue::Disengaging`]'s own
//! doc comments: Pure's counterparts of `ShipCollisionFx_Trigger`, its
//! absorb-effect caller, its shield-activate function and the dry-play chain
//! shared by three more cues each fire the same cue at the same point in the
//! same gate, corroborated confidence 78-82. That is six cues on one of the
//! two other titles; firing the remaining three (`Engine`, `Blowup`,
//! `LockOn`) on Pure and all nine on HD is still a bet that games in one
//! series with the same cue names and the same middleware fire them at the
//! same moments. Reasonable, and not a reading; confidence 50, which is
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
pub use announcer::{Announcer, ClassAnnouncer};
use banks::load_named_cue;
pub use banks::{Banks, Loaded};
pub use engine::Engine;

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
    sight: crate::race::sight::State,
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
        {
            return;
        }
        let voices = self.sfx.get_or_insert_with(|| {
            let mut rng = Rng::new(SFX_SEED);
            SfxVoices {
                engines: std::array::from_fn(|_| Engine::new(&mut rng)),
                sight: crate::race::sight::State::Absent,
                shield: None,
                shield_open: false,
                blowup: None,
                blowup_open: false,
                rng: Rng::new(SFX_SEED),
            }
        });
        let banks = race.sounds();
        let announcer = race.announcer();
        let class_announcer = race.class_announcer();
        let listener = listener_of(race);
        let craft = craft_positions(race);
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
                    crate::race::sight::State::Absent => None,
                    crate::race::sight::State::Seeking => Some(0),
                    crate::race::sight::State::Locked => Some(1),
                };
                // Only forward transitions blip. Falling back from locked to
                // seeking - the target sliding off the nose - would otherwise
                // chatter as the reticle hunted.
                let forward = matches!(
                    (voices.sight, sight),
                    (
                        crate::race::sight::State::Absent,
                        crate::race::sight::State::Seeking
                    ) | (
                        crate::race::sight::State::Seeking,
                        crate::race::sight::State::Locked
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

            // Every craft's engine, each off its own emitter. A craft with no
            // pose is one the race never spawned; it is skipped rather than
            // placed at the origin, which would put eight engines in a heap
            // under the start line.
            for (slot, engine) in voices.engines.iter_mut().enumerate() {
                let Some(&(position, speed)) = craft.get(slot).and_then(Option::as_ref) else {
                    engine.stop(mixer);
                    continue;
                };
                let placed = oag_audio::Emitter::engine(position.to_array()).place(&listener, 1.0);
                engine.tick(
                    mixer,
                    banks,
                    &mut voices.rng,
                    speed * oag_render::exhaust::SPEED_TO_KMH,
                    running,
                    placed,
                    slot != 0,
                    DT,
                );
            }
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
    /// entering [`crate::race::sight::State::Seeking`] and waveform `1` on
    /// entering [`crate::race::sight::State::Locked`]. With two 0.11 s
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
    LockOn,
}

impl Cue {
    /// Every cue this port fires, which is every one it knows how to load.
    pub const ALL: [Self; 9] = [
        Self::SpeedupPad,
        Self::Collision,
        Self::Absorb,
        Self::Engine,
        Self::Shield,
        Self::ShieldActive,
        Self::Disengaging,
        Self::Blowup,
        Self::LockOn,
    ];

    /// The bank the cue is looked up in.
    #[must_use]
    pub fn bank(self) -> BankName {
        match self {
            Self::SpeedupPad | Self::Blowup | Self::LockOn => BankName::Hud,
            Self::Collision | Self::Engine => BankName::Ship,
            Self::Absorb | Self::Shield => BankName::Weapons,
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
        matches!(self, Self::Engine | Self::Shield | Self::Blowup)
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
fn listener_of(race: &crate::race::Race) -> oag_audio::Listener {
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
        let ship = race.world.ships.get(slot)?;
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
    let emitter = match event.cue.placement() {
        Placement::Unplaced => return Some(dry),
        // The player's own branch, on a hypothesis the type documents.
        Placement::CraftUnlessPlayer if event.is_player() => return Some(dry),
        Placement::Craft | Placement::CraftUnlessPlayer => oag_audio::Emitter::craft,
        Placement::Engine => oag_audio::Emitter::engine,
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
}

/// A cue and the craft that raised it.
///
/// **Which craft is not bookkeeping this port invented.** The original's
/// `Sound_Play` takes an emitter as its first argument and every one of these
/// cues passes the craft's own, so the slot is the part of the call this port
/// used to be throwing away - which is why only the player was ever audible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CueEvent {
    /// What to play.
    pub cue: Cue,
    /// The grid slot whose emitter plays it. Slot 0 is the player.
    pub slot: u8,
}

impl CueEvent {
    /// A cue from `slot`.
    #[must_use]
    pub fn new(cue: Cue, slot: usize) -> Self {
        Self {
            cue,
            slot: u8::try_from(slot).unwrap_or(u8::MAX),
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
