//! Sound effects: the cues a race emits, and the banks they are read out of.
//!
//! [`super`] owns music and movie audio (streams with one source each). This
//! owns short sounds fired from *edges* in the simulation, addressed as the
//! original does: a cue string looked up in a `.bnk` sound bank.
//!
//! # Nothing here is fired on a guess
//!
//! As for particle effects (`CLAUDE.md`), a cue whose **trigger** is not
//! recovered stays unwired. Every [`Cue`] variant names the page that recovered
//! its trigger; the banks hold many cues this does not fire (`WEAPONPICKUP`,
//! `WRONGWAY`, `EXPLBIG` and more), listed by `oag-wad sounds <archive>`. The
//! reverse holds too: every cue whose trigger is recovered is wired, and
//! `Cue::ALL` is the set.
//!
//! # One caveat, per title
//!
//! Every trigger was read out of a **PSP Wipeout Pulse** executable. Cue names
//! and banks are each title's own data (Pure spells them as Pulse does, HD
//! keeps two elsewhere, see [`oag_title::SoundBanks`]). Eight of the nine now
//! cite an independently decompiled Pure trigger, structurally corroborated
//! against Pulse's, confidence 78-82; only [`Cue::Blowup`] is unconfirmed on
//! Pure, and all nine are unconfirmed on HD. That is a bet that games in one
//! series with the same cue names and middleware fire them at the same moments:
//! confidence 50, below this project's naming threshold, so written down
//! rather than implied.
//!
//! # What is approximated
//!
//! Each waveform plays at [`oag_formats::sblk::Sound::sample_rate`], the rate
//! the original's note-to-pitch arithmetic hands `sceSasSetPitch` for an
//! unmodulated play (decoded from the descriptor's centre note, confirmed live
//! against 190 breakpoint hits; a placeholder 44,100 Hz until 2026-09-16). The
//! game's own pitch modulation on top (the engine note rising with speed) is a
//! per-play offset from the caller, a separate reading
//! (`Scream_UpdateVoicePitch`'s inputs) not done here yet.
//!
//! **Which waveform of a cue sounds.** `.COLLISIONS` binds fifteen impact
//! samples and the opcode that chooses between them, `0x19`, is decoded on
//! Wipeout HD's binary and corroborated on PSP (confidence 88 both sides; PS2
//! grouped with PSP/Pure by operand byte layout, not independently checked).
//! [`Banks::pick`] matches it with a uniform draw that never repeats the
//! previous pick for a cue. The fifteen alternates are the disc's own audio,
//! within a tenth of a second of each other, so playing all at once (the only
//! choice-free reading) is certainly wrong. See
//! [`sound.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/sound.md#0x19---alternate-selection-decoded).
//!
//! On HD, `.COLLISIONS` plays `c_CShipShip` and `c_CShipWall`, each with
//! `S`/`M`/`L` children (112 waveforms walking the tree). The split is visible
//! and not wired: the child choice is guarded by opcode `0x22`, whose operand is
//! not decoded, and pairing ship-against-wall or a severity band with a suffix
//! off the names would be invented here (the refusal already made about
//! `oag_fx::sparks::severity`).
//!
//! # Positional audio
//!
//! Every craft is audible from where it is. The law (linear falloff to a
//! `200`-unit radius, a hard gate past it, the gamma volume curve, the
//! equal-power pan the disc tabulates) lives in [`oag_audio::spatial`]; the
//! listener is the **camera**, which the original copies into its sound
//! manager once a frame.
//!
//! This module adds only the wiring: which slot raised a cue ([`CueEvent`]),
//! which emitter it belongs on ([`Cue::placement`]), where each craft is this
//! tick. Not placed: [`Cue::ShieldActive`] (full volume, pan zero) and music.

use oag_audio::{Bus, Play, VoiceId};
use oag_core::Rng;
use oag_core::math::Vec3;

use super::{Audio, TICK_HZ};

mod frame;
pub use frame::RaceFrame;

mod announcer;
mod bank_name;
mod banks;
mod compose;
mod cue;
mod engine;
mod hd;
mod layers;
mod magstrip;
mod repeating;
mod track;
mod travel;
mod xfade;
pub use announcer::{Announcer, ClassAnnouncer};
pub use bank_name::BankName;
use banks::load_named_cue;
pub use banks::{Banks, Loaded};
pub use cue::Cue;
pub use engine::Engine;
pub use layers::{CueVoice, Where as VoicePlace, start as start_voices};
pub use repeating::Playing;
pub use track::TrackEmitters;
pub use track::circuit_manifest;
use travel::TravelVoices;
pub use xfade::{
    Craft as XfadeCraft, Inputs as XfadeInputs, Law as XfadeLaw, Source as XfadeSource,
    Team as XfadeTeam,
};

/// The seed the effects generator starts from.
///
/// Fixed, not drawn from a clock, so two `--dump-audio` captures of one run
/// choose the same collision alternates. Not the race seed: the simulation's
/// generator must not be advanced by anything audio does
/// (`docs/architecture/determinism.md`).
const SFX_SEED: u64 = 0x0aa9_5f10_0000_5f58;

/// The race's held voices and the generator that chooses a cue's alternate.
///
/// Split from [`Banks`] (decoded data), as `oag_fx::sparks` splits its pipeline
/// from particle state.
pub(super) struct SfxVoices {
    /// One per grid slot, each with its own random note and emitter. The
    /// original builds one in every craft's `ExhaustFlare_Init`, `base` being
    /// `rand(-127, 127) - 1143`, audible as eight engines. Drawn in slot order
    /// from one generator so a capture is reproducible.
    engines: [Engine; oag_gameplay::MAX_SHIPS],
    /// HD's crossfaded engine, one per grid slot whose team has a table, built
    /// on the first tick from [`Banks::xfade_team`]; empty on a title that plays
    /// `~ENGINE`. See [`xfade`].
    xfade: [Option<XfadeCraft>; oag_gameplay::MAX_SHIPS],
    /// Where the ear was last tick (`mgr+0x70`): the doppler term is held off
    /// when the listener jumped more than `oag_audio::LISTENER_JUMP`, a camera
    /// cut and not a velocity.
    last_listener: Option<oag_audio::Listener>,
    /// What the lock-on reticle was doing last frame, so its two blips fire on
    /// the transitions. See [`Cue::LockOn`].
    sight: oag_race::sight::State,
    /// The explosion's and the lock-on tone's repeating handles, on a title
    /// whose bank plays them as the list runs. See [`repeating::Held`].
    repeating: repeating::Held,
    /// The shield's held voice, while one is up and the alternate drawn was a
    /// loop. See [`Cue::Shield`].
    shield: Vec<VoiceId>,
    /// Whether the shield has been responded to for this activation: "a voice
    /// is held" and "this activation is handled" differ, and only the second
    /// may gate the trigger.
    shield_open: bool,
    /// `~BLOWUP`'s held voice, while the player's craft is mid-explosion.
    blowup: Vec<VoiceId>,
    /// Whether this explosion has been responded to, as [`Self::shield_open`].
    blowup_open: bool,
    /// `~AUTOPILOT`'s held voice, while the player's Autopilot is active. See
    /// [`Cue::Autopilot`].
    autopilot: Vec<VoiceId>,
    /// Whether this activation has opened [`Self::autopilot`] (and played
    /// [`Cue::Engaging`]), the latch shape of [`Self::blowup_open`].
    autopilot_open: bool,
    /// The circuit's ambience: one held voice per authored emitter in range. See
    /// [`TrackEmitters`].
    ambience: track::Ambience,
    /// `~PLASMATVL`'s held voice, one per **projectile** slot: a bolt is not
    /// the craft and several can be in the air, unlike the one-activation
    /// `Option<VoiceId>` of [`Self::shield`] (see [`Cue::PlasmaTravel`]). Driven
    /// directly off `race.sim.world.projectiles.slots` in [`Audio::race_tick`],
    /// as [`Engine`] reads craft position: a moving voice needs this tick's
    /// position. See [`TravelVoices`] for the shape the fields below share.
    plasma_travel: TravelVoices,
    /// `~ROCKETTVL`'s held voice, as [`Self::plasma_travel`]. See [`Cue::RocketTravel`].
    rocket_travel: TravelVoices,
    /// `~MISSILETVL`'s held voice, as above. See [`Cue::MissileTravel`].
    missile_travel: TravelVoices,
    /// `~SHURIKENTRAVEL`'s held voice, one per **projectile** slot: several
    /// Shurikens can be in flight and a slot index is each one's only stable
    /// handle. See [`Cue::ShurikenTravel`].
    shuriken_travel: TravelVoices,
    /// `~QUAKETRAVEL`'s held voice, slot 0 of a [`TravelVoices`] for the race's
    /// one wave. See [`Cue::QuakeTravel`].
    quake_travel: TravelVoices,
    /// `~LEACHATTACH`'s held voice, while a **locked** beam instance exists
    /// ([`Cue::LeachAttach`]). `Option<VoiceId>` because the LeachBeam is the
    /// race's single world-wide instance, not a slot in
    /// [`oag_weapons::projectile::Projectiles`] (see
    /// `oag_weapons::projectile::leach_beam`); driven directly off
    /// `race.sim.world.leach_beam` for this tick's position.
    leach_attach: Option<VoiceId>,
    /// The magstrip hum, per grid slot. See [`Cue::Magstrip`].
    magstrip: magstrip::Voices,
    rng: Rng,
}

/// Seconds in one simulation tick, the only clock this module has.
const DT: f32 = 1.0 / TICK_HZ as f32;

impl Audio {
    /// Plays one tick's worth of a race's sound effects.
    ///
    /// Called inside the fixed-timestep loop immediately after `Race::tick`, so a
    /// `--dump-audio` capture puts a cue on the same tick a window does; per
    /// frame, a slow frame would merge two ticks' cues.
    ///
    /// Also called on a finished race, where nothing is stepped: the engine is a
    /// held voice with a spin-down law, and skipping this would leave `~ENGINE`
    /// at racing pitch under the results table (see [`Engine::tick`]).
    ///
    /// Everything device-shaped lives here, not on `Race`; see
    /// `docs/architecture/adr/0018-audio-mixer-architecture.md`.
    pub fn race_tick(&mut self, frame: RaceFrame<'_>) {
        let RaceFrame {
            banks,
            emitters,
            announcer,
            class_announcer,
            cues,
            announcements,
            class_announcements,
            listener,
            craft,
            slot_teams,
            throttle,
            projectiles,
            running,
            shielded,
            exploding,
            autopilot_active,
            thrust_gated,
            sight,
            quake_point,
            leach_beam,
        } = frame;
        self.hd_race_mix(banks.mix.as_deref(), thrust_gated);
        if banks.is_empty()
            && announcer.is_empty()
            && class_announcer.is_empty()
            && emitters.omni.is_empty()
            && emitters.directional.is_empty()
        {
            return;
        }
        let voices = self.sfx.get_or_insert_with(|| {
            let mut rng = Rng::new(SFX_SEED);
            SfxVoices {
                engines: std::array::from_fn(|_| Engine::new(&mut rng)),
                xfade: std::array::from_fn(|slot| {
                    slot_teams[slot]
                        .and_then(|team| banks.xfade_team(team))
                        .map(|t| XfadeCraft::new(t).on_bus(banks.group_bus(7)))
                }),
                last_listener: None,
                sight: oag_race::sight::State::Absent,
                repeating: repeating::Held::default(),
                shield: Vec::new(),
                shield_open: false,
                blowup: Vec::new(),
                blowup_open: false,
                autopilot: Vec::new(),
                autopilot_open: false,
                ambience: track::Ambience::default().on_bus(banks.group_bus(8)),
                plasma_travel: TravelVoices::new(),
                rocket_travel: TravelVoices::new(),
                missile_travel: TravelVoices::new(),
                shuriken_travel: TravelVoices::new(),
                quake_travel: TravelVoices::new(),
                leach_attach: None,
                magstrip: magstrip::Voices::default(),
                rng: Rng::new(SFX_SEED),
            }
        });
        // `SoundManager_Update`'s camera-cut guard, one answer for every voice
        // this tick; the first tick is treated as a cut, which it is.
        let doppler_enabled = voices
            .last_listener
            .replace(listener)
            .is_some_and(|previous| !listener.jumped_from(&previous));
        self.output.with_mixer(|mixer| {
            for event in cues {
                let slot = usize::from(event.slot);
                match event.cue {
                    Cue::Magstrip => {
                        voices.magstrip.start(
                            slot,
                            mixer,
                            banks,
                            &mut voices.rng,
                            &listener,
                            &craft,
                        );
                        continue;
                    }
                    Cue::MagstripStop => {
                        voices.magstrip.stop(slot, mixer);
                        continue;
                    }
                    _ => {}
                }
                // A held cue is not a one-shot: `~ENGINE` here would start a second
                // engine each time it was raised.
                if event.cue.held() {
                    continue;
                }
                let Some(pan) = place(event, &listener, &craft) else {
                    // Out of range: the original's `Sound_Play` refuses a voice on an
                    // emitter whose out-of-range bit is latched, so it is not started
                    // rather than started silent.
                    continue;
                };
                let Some(started) = banks.voices(event.cue, &mut voices.rng) else {
                    continue;
                };
                // Handles dropped: a one-shot is fire-and-forget, and a refused
                // voice is counted by `Mixer::starved`.
                let _ = layers::start(
                    mixer,
                    &started,
                    event.cue.bus(),
                    VoicePlace {
                        gain: pan.gain,
                        pan: pan.pan,
                    },
                );
            }

            // The Zone announcer: one voice line per milestone raised this tick,
            // dry at full volume like `ShieldActive` (a line in the player's ear,
            // no emitter). `None` covers no announcer on this title and a ladder
            // that does not name this zone number, the silent degrade of
            // `Banks::pick`.
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

            // The Zone speed-class announcer: one voice line per stage raised this
            // tick (see `oag_raceplay::tick`). Unlike the milestone one, this trigger is
            // not yet read from HD's executable (`oag_title::ZoneClassAnnouncer`).
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

            // `Shield_Activate` opens `~SHIELD` with a handle and the shield's drop
            // releases it, so this is a level, not an edge. Latched on
            // `shield_open`, not the voice handle, so it fires once per
            // activation: keying on `Option<VoiceId>` would re-enter every tick
            // when no handle came back (a full pool, a non-looping alternate)
            // and repeat the sound sixty times a second.
            match (shielded, voices.shield_open) {
                (true, false) => {
                    voices.shield_open = true;
                    // The alternate's own loop flag decides, not the cue's: Pulse's
                    // `~SHIELD` is two waveforms and both loop, Pure's four of which
                    // two do, so forcing `Play::looping` would loop a one-shot.
                    if let Some(started) = banks.voices(Cue::Shield, &mut voices.rng) {
                        voices.shield =
                            layers::start(mixer, &started, Cue::Shield.bus(), VoicePlace::DRY);
                    }
                }
                (false, true) => {
                    voices.shield_open = false;
                    for id in voices.shield.drain(..) {
                        mixer.stop(id);
                    }
                }
                _ => {}
            }

            // The lock-on reticle's two blips, on the edges of its state: this
            // mixer has no equivalent of the original's cue parameter. See
            // [`Cue::LockOn`].
            let sight_played = voices
                .repeating
                .drive_sight(sight, banks, mixer, &mut voices.rng);
            if !sight_played && sight != voices.sight {
                let waveform = match sight {
                    oag_race::sight::State::Absent => None,
                    oag_race::sight::State::Seeking => Some(0),
                    oag_race::sight::State::Locked => Some(1),
                };
                // Only forward transitions blip; locked back to seeking (target
                // sliding off the nose) would chatter.
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

            // The explosion, level-and-latch like the shield: case 4 opens a
            // handle, so this is a voice's lifetime. Dry: the player's own craft
            // goes to the no-emitter path.
            let blowup_played =
                voices
                    .repeating
                    .drive_blowup(exploding, banks, mixer, &mut voices.rng);
            match (exploding && !blowup_played, voices.blowup_open) {
                (true, false) => {
                    voices.blowup_open = true;
                    if let Some(started) = banks.voices(Cue::Blowup, &mut voices.rng) {
                        voices.blowup =
                            layers::start(mixer, &started, Cue::Blowup.bus(), VoicePlace::DRY);
                    }
                }
                (false, true) => {
                    voices.blowup_open = false;
                    for id in voices.blowup.drain(..) {
                        mixer.stop(id);
                    }
                }
                _ => {}
            }

            // The Autopilot, level-and-latch like the explosion:
            // `Ship_FireHeldWeapon`'s case 6 opens a handle on `~AUTOPILOT`, and
            // `autopilot_eng` plays once on the same rising edge ([`Cue::Autopilot`],
            // [`Cue::Engaging`]). Both dry.
            match (autopilot_active, voices.autopilot_open) {
                (true, false) => {
                    voices.autopilot_open = true;
                    if let Some(started) = banks.voices(Cue::Autopilot, &mut voices.rng) {
                        voices.autopilot =
                            layers::start(mixer, &started, Cue::Autopilot.bus(), VoicePlace::DRY);
                    }
                    if let Some(started) = banks.voices(Cue::Engaging, &mut voices.rng) {
                        let _ =
                            layers::start(mixer, &started, Cue::Engaging.bus(), VoicePlace::DRY);
                    }
                }
                (false, true) => {
                    voices.autopilot_open = false;
                    for id in voices.autopilot.drain(..) {
                        mixer.stop(id);
                    }
                }
                _ => {}
            }

            // Every held travel voice, one call per weapon ([`TravelVoices`]; not
            // the shield/blowup shape, see [`SfxVoices::plasma_travel`]). Each reads
            // this tick's projectile array, not a queued `CueEvent`, as the craft's
            // engines below do. The one-shot endings (`*HITWALL`/`*HITSHIP`, never
            // both) are separate `CueEvent`s with their impact point, pushed from
            // `crates/raceplay/src/tick.rs`.
            voices.plasma_travel.tick(
                mixer,
                banks,
                &mut voices.rng,
                &listener,
                &projectiles,
                Cue::PlasmaTravel,
                Cue::PlasmaTravel.radius(),
                // The rising edge: charge reached zero this tick
                // (`Projectiles::advance`'s charging branch holds the release edge).
                |p| p.kind == Some(oag_tables::weapons::Weapon::Plasma) && p.charge <= 0.0,
                |p| Some(p.position),
            );
            voices.rocket_travel.tick(
                mixer,
                banks,
                &mut voices.rng,
                &listener,
                &projectiles,
                Cue::RocketTravel,
                Cue::RocketTravel.radius(),
                |p| p.kind == Some(oag_tables::weapons::Weapon::Rocket),
                |p| Some(p.position),
            );
            voices.missile_travel.tick(
                mixer,
                banks,
                &mut voices.rng,
                &listener,
                &projectiles,
                Cue::MissileTravel,
                Cue::MissileTravel.radius(),
                |p| p.kind == Some(oag_tables::weapons::Weapon::Missile),
                |p| Some(p.position),
            );
            // On the blade's own emitter, `300.0` radius ([`Cue::ShurikenTravel`]).
            voices.shuriken_travel.tick(
                mixer,
                banks,
                &mut voices.rng,
                &listener,
                &projectiles,
                Cue::ShurikenTravel,
                Cue::ShurikenTravel.radius(),
                |p| p.kind == Some(oag_tables::weapons::Weapon::Shuriken),
                |p| Some(p.position),
            );
            // `~QUAKETRAVEL`: one held voice at the wave's road midpoint ([`Cue::QuakeTravel`]).
            voices.quake_travel.follow(
                0,
                quake_point,
                mixer,
                banks,
                &mut voices.rng,
                &listener,
                Cue::QuakeTravel,
                Cue::QuakeTravel.radius(),
            );

            // `~LEACHATTACH`, held while a **locked** beam instance exists, which
            // outlives `Beam::connected` through the disconnect linger
            // ([`Cue::LeachAttach`]). Position is the chosen midpoint between the
            // two craft, updated every tick like the Plasma bolt above.
            let leach_locked = leach_beam
                .filter(|beam| beam.kind == oag_weapons::projectile::leach_beam::Kind::Locked);
            match (leach_locked, voices.leach_attach) {
                (Some(beam), None) => {
                    if let Some(started) = banks.voices(Cue::LeachAttach, &mut voices.rng)
                        && let Some(at) = leach_attach_point(&craft, beam)
                    {
                        let placed = oag_audio::Emitter {
                            position: at.to_array(),
                            radius: Cue::LeachAttach.radius(),
                            cone: None,
                        }
                        .place(&listener, 1.0);
                        let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));
                        // The cue keys a one-shot and a loop: the loop is the held
                        // voice this arm steers, the one-shot fires once where the
                        // beam attached.
                        let place = VoicePlace { gain, pan };
                        let (held, once): (Vec<_>, Vec<_>) =
                            started.into_iter().partition(|v| v.looping);
                        let _ = layers::start(mixer, &once, Cue::LeachAttach.bus(), place);
                        voices.leach_attach = layers::start(
                            mixer,
                            &held[..held.len().min(1)],
                            Cue::LeachAttach.bus(),
                            place,
                        )
                        .first()
                        .copied();
                    }
                }
                (Some(beam), Some(id)) if mixer.is_playing(id) => {
                    if let Some(at) = leach_attach_point(&craft, beam) {
                        let placed = oag_audio::Emitter {
                            position: at.to_array(),
                            radius: Cue::LeachAttach.radius(),
                            cone: None,
                        }
                        .place(&listener, 1.0);
                        let (gain, pan) = placed.map_or((0.0, None), |p| (p.gain, Some(p.pan)));
                        mixer.set_gain(id, gain);
                        mixer.set_pan(id, pan);
                    }
                }
                (Some(_), Some(_)) => voices.leach_attach = None,
                (None, Some(id)) => {
                    mixer.stop(id);
                    voices.leach_attach = None;
                }
                (None, None) => {}
            }

            // Every craft's engine, each off its own emitter. A craft with no pose
            // was never spawned and is skipped, not placed at the origin (eight
            // engines in a heap under the start line).
            for (slot, engine) in voices.engines.iter_mut().enumerate() {
                let Some(&(position, speed)) = craft.get(slot).and_then(Option::as_ref) else {
                    engine.stop(mixer);
                    continue;
                };
                engine.tick(
                    mixer,
                    banks,
                    &mut voices.rng,
                    speed * oag_core::math::SPEED_TO_KMH,
                    running,
                    position.to_array(),
                    &listener,
                    slot != 0,
                    doppler_enabled,
                    DT,
                );
            }

            // HD's engine: same craft and ears, a table instead of a held pitch
            // law ([`xfade`]). Its layers stay resident, beyond the PSP's 32 voices.
            if voices.xfade.iter().any(Option::is_some) {
                mixer.grow_pool(oag_audio::mixer::HD_VOICES);
            }
            for (slot, craft_xfade) in voices.xfade.iter_mut().enumerate() {
                let Some(xfade) = craft_xfade else {
                    continue;
                };
                let Some(&(position, speed)) = craft.get(slot).and_then(Option::as_ref) else {
                    xfade.stop(mixer);
                    continue;
                };
                let player = slot == 0;
                xfade.tick(
                    mixer,
                    XfadeInputs {
                        speed_field: speed
                            * oag_core::math::SPEED_TO_KMH
                            * oag_core::math::SPEED_FIELD_GAIN,
                        throttle: player.then_some(throttle),
                    },
                    running,
                    position.to_array(),
                    &listener,
                    doppler_enabled,
                    DT,
                );
            }

            voices.magstrip.follow(mixer, &listener, &craft);

            // The circuit's ambience, same ears and law, after the craft so a
            // starved pool serves the race's cues first: the original's pool is
            // hardware, so the order is chosen, not measured. The peak on the two
            // circuits `track_audio_ground_truth` flies is eight against
            // `oag_audio::mixer::MAX_VOICES`, so it has not yet mattered.
            voices.ambience.tick(
                mixer,
                emitters,
                &listener,
                &mut voices.rng,
                doppler_enabled,
                DT,
            );
        });
    }

    /// How many of the circuit's own emitters are sounding now ("is the circuit
    /// audible"), apart from `Mixer::active_voices`, which counts cues too; the
    /// claim of `crates/game/tests/track_audio_ground_truth.rs`.
    #[must_use]
    pub fn ambient_voices(&self) -> usize {
        self.sfx.as_ref().map_or(0, |voices| {
            self.output
                .with_mixer(|mixer| voices.ambience.playing(mixer))
        })
    }

    /// Releases the held race voices, on leaving a race and on launching one: a
    /// relaunch replaces the `RaceStage` without the back-out path, and a
    /// carried-over [`SfxVoices`] would hold a `VoiceId` into a reused pool and
    /// an engine past its rising-edge snap.
    pub fn stop_race_sfx(&mut self) {
        if let Some(voices) = &mut self.sfx {
            self.output.with_mixer(|mixer| {
                for engine in &mut voices.engines {
                    engine.stop(mixer);
                }
                for xfade in voices.xfade.iter_mut().flatten() {
                    xfade.stop(mixer);
                }
                for id in voices.shield.drain(..) {
                    mixer.stop(id);
                }
                voices.shield_open = false;
                for id in voices.blowup.drain(..) {
                    mixer.stop(id);
                }
                voices.blowup_open = false;
                voices.repeating.stop_all(mixer);
                for id in voices.autopilot.drain(..) {
                    mixer.stop(id);
                }
                voices.autopilot_open = false;
                voices.plasma_travel.stop_all(mixer);
                voices.rocket_travel.stop_all(mixer);
                voices.missile_travel.stop_all(mixer);
                voices.shuriken_travel.stop_all(mixer);
                voices.quake_travel.stop_all(mixer);
                if let Some(id) = voices.leach_attach.take() {
                    mixer.stop(id);
                }
                voices.magstrip.stop_all(mixer);
                voices.ambience.stop(mixer);
            });
        }
        self.sfx = None;
        self.hd.state = super::hd_mix::State::FrontEnd;
    }
}

/// [`Cue::LeachAttach`]'s position: the midpoint between shooter and target, or
/// [`None`] if either has gone from the grid.
///
/// Chosen, not measured: the page never states what the beam's scene node
/// tracks, and the two endpoints (which `oag_raceplay::weapons::visuals::leach_beam_ribbon_vertices`
/// draws between) are the only positions this port has.
fn leach_attach_point(
    craft: &[Option<(Vec3, f32)>; oag_gameplay::MAX_SHIPS],
    beam: oag_weapons::projectile::leach_beam::Beam,
) -> Option<Vec3> {
    let (owner, _) = (*craft.get(usize::from(beam.owner))?)?;
    let (target, _) = (*craft.get(usize::from(beam.target))?)?;
    Some((owner + target) * 0.5)
}

/// Where one cue is heard from, or [`None`] if out of range entirely.
///
/// The inner `Option<f32>` is the mixer's "no position" ([`oag_audio::mixer::Play::pan`]),
/// so there are three answers: placed, placed without position, refused.
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
        // An arbitrary world point ([`Placement::Point`]). `None` means the event
        // was built wrong (no [`CueEvent::at_point`]), not out of range; that
        // refusal comes from `place` below as for a craft.
        let point = event.point?;
        // Not `Emitter::craft` (always [`oag_audio::Emitter::CRAFT_RADIUS`]): some
        // cues carry a bolt- or beam-owned emitter with its own radius ([`Cue::radius`]).
        let placed = oag_audio::Emitter {
            position: point.to_array(),
            radius: event.cue.radius(),
            cone: None,
        }
        .place(listener, 1.0)?;
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
    // A cue from a slot with no craft is dropped, not played dry: a rival's
    // collision would arrive at full volume the moment a slot is emptied.
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
    /// The craft's own emitter, [`oag_audio::Emitter::CRAFT_RADIUS`] wide.
    ///
    /// `Craft_Construct_q` allocates it and never overrides the default radius,
    /// so this is every craft including the player's.
    Craft,
    /// The engine flare's emitter, a quarter as wide.
    Engine,
    /// The craft's emitter for an opponent; dry at full volume for the player.
    /// A hypothesis, stated: the original branches on `racer+0x368`, which
    /// [`pads.md`](../../../../docs/ghidra/functions/psp-pulse-usa/pads.md)
    /// records at confidence **45** as probably "is the local player", below the
    /// naming threshold, on three converging uses. Two more from positional
    /// audio: the non-positional branch plays at volume `0x400` (maximum), pan
    /// zero, which is what a player's own sound wants, and
    /// `Exhaust_UpdateEngineSound` scales the engine by `0.85` when the field is
    /// set, as a mix does to everybody else. If the field means something else,
    /// this is the line that moves.
    CraftUnlessPlayer,
    /// No emitter has been read for this cue, so it is played dry.
    Unplaced,
    /// An arbitrary world point carried on the [`CueEvent`], rather than read off
    /// a craft.
    ///
    /// Ours, not read off any call site: the original always names an emitter
    /// (an object with a scene node), never a bare position. For a noise-maker
    /// that is not a craft and has no modelled emitter: the Plasma bolt, which
    /// `Plasma_Init` gives its own emitter at the bolt's matrix ([`Cue::PlasmaTravel`],
    /// [`Cue::PlasmaHitWall`]). [`CueEvent::at_point`] is the entry point;
    /// [`Cue::PlasmaTravel`]'s held voice is placed the same way by its own
    /// tracker ([`SfxVoices::plasma_travel`]). Radius is
    /// [`oag_audio::Emitter::CRAFT_RADIUS`]: neither `Plasma_Init` nor
    /// `Plasma_Launch` writes the emitter's `+0x38`, so it keeps
    /// `SoundEmitter_Init`'s default, as the craft's does.
    Point,
}

/// A cue and the craft that raised it.
///
/// The slot is the original's own: `Sound_Play` takes an emitter first and
/// every one of these cues passes the craft's, the part this port once threw
/// away, so only the player was audible. [`Self::point`] is the same for
/// [`Placement::Point`]. No `#[derive(Eq)]`: `Vec3` holds `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CueEvent {
    /// What to play.
    pub cue: Cue,
    /// The grid slot whose emitter plays it. Slot 0 is the player. Left at `0`
    /// and unread for a [`Placement::Point`] cue built with [`Self::at_point`].
    pub slot: u8,
    /// The world position a [`Placement::Point`] cue plays at, else [`None`].
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

    /// A cue at `point` rather than at a craft ([`Placement::Point`]).
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
