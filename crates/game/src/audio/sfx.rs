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
//! [`oag_title::SoundBanks`] - but **no Pure or HD dispatch has ever been
//! looked at**. Firing a Pulse-recovered edge on those titles is a bet that
//! games in one series with the same cue names and the same middleware fire
//! them at the same moments. Reasonable, and not a reading; confidence 50,
//! which is below this project's naming threshold and so is written down
//! rather than implied.
//!
//! # Two things here are honest placeholders, and both are load-bearing
//!
//! **The sample rate.** [`oag_formats::sblk::ASSUMED_SAMPLE_RATE`] is not
//! recovered; see its doc comment. Everything below plays at it.
//!
//! **Which waveform of a cue sounds.** `.COLLISIONS` binds fifteen impact
//! samples and the opcode that chooses between them is one of the 41 that are
//! unread, so [`Bank::pick`] chooses with its own generator. That is a
//! deliberate approximation rather than a stand-in for missing data: the
//! fifteen alternates *are* the disc's own audio, all of them within a tenth of
//! a second of each other in length, and playing all fifteen at once - the only
//! reading that needs no choice - is the one thing that is certainly wrong.
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

use std::collections::BTreeMap;
use std::sync::Arc;

use log::info;
use oag_assets::source::Archives;
use oag_audio::{Bus, Play, Sound, VoiceId};
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_formats::sblk;

use super::{Audio, TICK_HZ};

mod engine;
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
    shield: Option<VoiceId>,
    /// Whether the shield has already been responded to for this activation.
    ///
    /// The latch, kept apart from [`Self::shield`] because "a voice is held"
    /// and "this activation has been handled" are different facts and only the
    /// second one may gate the trigger.
    shield_open: bool,
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
        let voices = self.sfx.get_or_insert_with(|| {
            let mut rng = Rng::new(SFX_SEED);
            SfxVoices {
                engines: std::array::from_fn(|_| Engine::new(&mut rng)),
                shield: None,
                shield_open: false,
                rng: Rng::new(SFX_SEED),
            }
        });
        let banks = race.sounds();
        let listener = listener_of(race);
        let craft = craft_positions(race);
        let running = !race.finished();
        let shielded = race.shield_is_up();
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
                let play = if looping {
                    Play::looping(sound, Bus::Sfx)
                } else {
                    Play::once(sound, Bus::Sfx)
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
                            mixer.play(Play::looping(sound, Bus::Sfx))
                        } else {
                            let _ = mixer.play(Play::once(sound, Bus::Sfx));
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
    Disengaging,
}

impl Cue {
    /// Every cue this port fires, which is every one it knows how to load.
    pub const ALL: [Self; 7] = [
        Self::SpeedupPad,
        Self::Collision,
        Self::Absorb,
        Self::Engine,
        Self::Shield,
        Self::ShieldActive,
        Self::Disengaging,
    ];

    /// The bank the cue is looked up in.
    #[must_use]
    pub fn bank(self) -> BankName {
        match self {
            Self::SpeedupPad => BankName::Hud,
            Self::Collision | Self::Engine => BankName::Ship,
            Self::Absorb | Self::Shield => BankName::Weapons,
            Self::ShieldActive | Self::Disengaging => BankName::Speech,
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
    /// rejected. The name is otherwise exact and it is the only candidate on
    /// any disc.
    ///
    /// This comment used to add "if a child cue turns out to select among its
    /// parents rather than the other way round, this is the line that is
    /// wrong", and Wipeout HD's bank now answers it: `.COLLISIONS` is the
    /// **parent**, and the cues it plays are the plainly named `c_CShipShip`
    /// and `c_CShipWall`. So the leading dot does not mark a cue as somebody's
    /// child - it is a naming convention, and the lookup stands.
    /// See [`oag_formats::sblk::child`].
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
    /// **Never fails, and reports every cue either way.** Every title in the
    /// lineage does carry banks - a claim this project got wrong about Pure for
    /// months - but a cue can still be missing for reasons that are ordinary
    /// rather than broken: Wipeout HD has no `~ENGINE` at all, its ship audio
    /// being a per-event `c_*` set rather than a held loop. Each miss is a line
    /// in [`Self::report`] and silence, not an error and not a substitute.
    ///
    /// A race that refused to start because a bank was missing would make the
    /// audio work a precondition for every other kind of work in the tree.
    #[must_use]
    pub fn load(archives: &mut Archives, banks: &oag_title::SoundBanks, zone: bool) -> Self {
        let mut sounds = BTreeMap::new();
        let mut report = Vec::new();
        let mut blobs: BTreeMap<BankName, Vec<u8>> = BTreeMap::new();

        for cue in Cue::ALL {
            let entry = cue.bank().entry(banks, zone);
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
                Ok((loaded, skipped)) => {
                    let undecoded = if skipped == 0 {
                        String::new()
                    } else {
                        format!(", {skipped} skipped as not PS-ADPCM")
                    };
                    report.push(format!(
                        "sfx: {} -> {} waveform(s) from {entry}{undecoded}",
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
fn load_cue(blob: &[u8], cue: Cue) -> anyhow::Result<(Loaded, usize)> {
    let bank = sblk::Bank::parse(blob)?;
    let record = bank
        .cue_named(cue.name())
        .ok_or_else(|| anyhow::anyhow!("{:?} names no cue in {}", cue.name(), bank.name))?;
    // **The tree, not the cue's own run.** On the PSP, PS2 and Pure discs no
    // wired cue plays a child, so this is `cue_sounds` there and the two are
    // the same call. Wipeout HD's `.COLLISIONS` binds nothing itself and plays
    // `c_CShipShip` and `c_CShipWall`, each of which has S/M/L children of its
    // own - 112 waveforms in all. See `oag_formats::sblk::child`.
    let sounds = bank.cue_tree_sounds(&record);
    anyhow::ensure!(
        !sounds.is_empty(),
        "{} binds no waveform: its {} command(s) are all opcodes this does not read",
        cue.name(),
        record.commands
    );

    let mut waveforms = Vec::with_capacity(sounds.len());
    let mut skipped = 0;
    for sound in &sounds {
        // **Not every waveform is PS-ADPCM, and the descriptor says which.**
        // On Wipeout HD about a third set `+0x0e`'s `0x80`, a codec this
        // project has not identified; decoding one as ADPCM would produce 28
        // samples a block of noise, which is exactly the plausible-looking
        // stand-in `CLAUDE.md` forbids. Dropped, and counted so the load
        // report says so. See `oag_formats::sblk::NOT_ADPCM_FLAG`.
        if !sound.is_adpcm() {
            skipped += 1;
            continue;
        }
        let data = bank.waveform(sound).ok_or_else(|| {
            anyhow::anyhow!("{} reaches outside the waveform section", cue.name())
        })?;
        waveforms.push((
            Arc::new(Sound::new(
                sblk::decode_adpcm(data),
                1,
                sblk::ASSUMED_SAMPLE_RATE,
            )?),
            sound.is_looping(),
        ));
    }
    anyhow::ensure!(
        !waveforms.is_empty(),
        "all {skipped} of {}'s waveforms are in a codec this does not decode",
        cue.name()
    );
    Ok((Loaded { waveforms }, skipped))
}

#[cfg(test)]
mod tests;
