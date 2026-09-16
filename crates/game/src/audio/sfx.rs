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
//! # One thing here is an honest approximation, and it is load-bearing
//!
//! **The sample rate is not it any more.** Until 2026-09-16 every waveform
//! played at a placeholder 44,100 Hz; now each plays at
//! [`oag_formats::sblk::Sound::sample_rate`], the rate the original's own
//! note-to-pitch arithmetic hands `sceSasSetPitch` for an unmodulated play,
//! decoded from the descriptor's centre note and confirmed live against 190
//! breakpoint hits. What is *not* carried is the game's own pitch modulation
//! on top of it - the engine note rising with speed is a pitch offset the
//! caller adds per play, not a property of the bank, and it is a separate
//! reading (`Scream_UpdateVoicePitch`'s inputs) this module does not do yet.
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
mod cue;
mod engine;
mod track;
pub use announcer::{Announcer, ClassAnnouncer};
use banks::load_named_cue;
pub use banks::{Banks, Loaded};
pub use cue::{BankName, Cue};
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
                    // `PLASMAHITWALL` or `PLASMAHITSHIP` for an actual ending
                    // - a wall/timeout or a struck craft, never both - is a
                    // separate `CueEvent` carrying its own impact point,
                    // pushed from `crates/game/src/race/tick.rs`.
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
