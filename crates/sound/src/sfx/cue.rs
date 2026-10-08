//! [`BankName`] and [`Cue`]: which `.bnk` a cue lives in, and every cue this
//! port fires. Split out of [`super`] under the 1,000-line rule; `Placement`
//! stays in [`super`] beside [`super::place`] and [`super::CueEvent`].

/// A sound the simulation asks for, by the name the original passes to
/// `Sound_Play`.
///
/// Every one has a recovered call site, cited on each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cue {
    /// Crossing onto a speed pad.
    ///
    /// `Ship_ApplySpeedupPad` calls `Sound_Play(..., "SPEEDUPPAD", ...)` in its
    /// new-pad branch, the same edge that arms the engine flare
    /// (`oag_raceplay::Race::test_speedup_pads`).
    /// `docs/ghidra/functions/psp-pulse-usa/pads.md`, confidence 88. Pure's
    /// `FUN_0886b548` fires the same cue on both branches:
    /// `docs/ghidra/functions/psp-pure-usa/dry-play-cues.md`, confidence 78.
    SpeedupPad,
    /// The perfect start: a human craft's first thrust inside the launch boost's
    /// perfect window.
    ///
    /// `Race_UpdateLaunchGrade` (`0x0882773c`) writes grade 2 and calls
    /// `ExhaustFlare_OnPerfectStart` (`0x08904fd4`), which arms the flare's boost
    /// timer to `0.8` and plays `"TURBO"` from `weapons.bnk` (`DAT_08ac1df8`)
    /// like [`Self::SpeedupPad`]. `docs/ghidra/functions/psp-pulse-usa/perfect-start.md`,
    /// confidence 85, not watched.
    Turbo,
    /// Hull against wall or track.
    ///
    /// `ShipCollisionFx_Trigger` (`0x089246b4`) fires it once per contact past
    /// the 0.8 s spark cooldown, so it rides the sparks' gate.
    /// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`, confidence 85.
    /// Pure's `ShipCollisionFx_Trigger` (`0x0888e340`) matches (`.COLLISIONS` in
    /// `SHIP_CL`/`SHIP_ZM`): `docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`,
    /// confidence 82, the only one of the nine cues checked on a second title;
    /// the module doc's confidence-50 bet covers the rest.
    Collision,
    /// A pickup absorbed into the pool, or Eliminator's lap refill.
    ///
    /// `Ship_PlayAbsorbFeedback` (`0x08840640`) plays an `ABSORB` sound once
    /// before staggering ten spark instances. Not a shielded contact: its four
    /// callers are the absorb handler, `Ship_RefillLapShield` and two network
    /// callbacks (`docs/ghidra/functions/psp-pulse-usa/shield.md`, 2026-09-16).
    /// Pure's `FUN_08925e20` is the same with eight sparks, not ten:
    /// `docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`, confidence 80.
    Absorb,
    /// The engine, held while the craft runs.
    ///
    /// `Exhaust_UpdateEngineSound` (`0x08904cf4`) opens the voice in its
    /// constructor and writes pitch and volume every tick;
    /// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`, confidence 80. The law
    /// is in [`super::Engine`]. Pure's `ExhaustFlare_Init` opens the same
    /// `~ENGINE` voice with two constants (`-1143.0` base pitch, `0.01` lerp)
    /// matching bit-for-bit; the per-tick write is unchecked there:
    /// `docs/ghidra/functions/psp-pure-usa/exhaust-sound.md`, confidence 82.
    Engine,
    /// The shield, held while it is up.
    ///
    /// `Shield_Activate` (`0x0883e544`) calls `Sound_PlayLooping(1.0,
    /// entity->0x50, ..., "~SHIELD", entity + 0x54)` and keeps the handle.
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`. Pure's
    /// `FUN_0892425c` fires the same cue in the same order after `ShieldActive`:
    /// `docs/ghidra/functions/psp-pure-usa/shield-sound.md`, confidence 80.
    Shield,
    /// The announcer, on the same activation.
    ///
    /// The other half of `Shield_Activate`'s pair: `Sound_Play(entity, ...,
    /// "shieldactive", 0x400, 0)`. In `speech.bnk`, so a voice line. Pure fires
    /// the same cue at `0x400`, first of the pair, through a deeper call chain
    /// (unread): `docs/ghidra/functions/psp-pure-usa/shield-sound.md`, confidence 80.
    ShieldActive,
    /// The Autopilot's hum, held while the pickup is active.
    ///
    /// `Ship_FireHeldWeapon`'s held-id-6 case opens it through `FUN_0883e9b0`
    /// (the dry, no-emitter path [`Self::Blowup`] and [`Self::Disengaging`]
    /// take) and keeps the handle at `param_1+0x58`. `Autopilot_Update`'s
    /// `<= 0.0f` arm (timer expiry, not the one-second warning edge) releases
    /// it: `if (entity->0x58 != 0) { Scream_StopSound(); entity->0x58 = 0; }`.
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md` ("`Ship_FireHeldWeapon`
    /// opens both cues", "`Autopilot_Update` counts it down"), confidence 88,
    /// wired 2026-09-25 (correcting that page's "no located opener").
    /// `Data\Sound\hud.bnk`, three waveforms, one looping, 3.04 s total
    /// (`oag-wad sounds`, `pulse-psp-usa.chd`). The string is `~AUTOPILOT`,
    /// read from the executable bytes at `0x08a7b6b0`; Ghidra's auto-label
    /// `PTR_s__AUTOPILOT_08a7b6af_1_08a7b6bc` only looks split because it
    /// substitutes `_` for the non-identifier `~`.
    ///
    /// Driven off the level `ships[0].autopilot_timer > 0.0`, as [`Self::Blowup`]
    /// is off `craft_is_exploding`, not the cue queue: the timer's expiry is the
    /// release condition, so the level reproduces open-on-rise, close-on-fall.
    Autopilot,
    /// The one-shot "autopilot engaged" line, on the same edge as [`Self::Autopilot`].
    ///
    /// Second half of the same `Ship_FireHeldWeapon` case:
    /// `FUN_0883e9b0(param_1, speech.bnk, "autopilot_eng", 0x400, 0)`, return
    /// discarded (one-shot, dry path like [`Self::Disengaging`]).
    /// `Data\Sound\speech.bnk`, two waveforms, 3.14 s total (`oag-wad sounds`,
    /// `pulse-psp-usa.chd`); `autopilot.md`'s "cue 0, 1.28 s" was one waveform.
    /// Confidence 88, 2026-09-25.
    Engaging,
    /// The announcer, one second before an Autopilot pickup lets go.
    ///
    /// `Autopilot_Update` (`0x08861404`) plays it when the remaining time crosses
    /// `1.0` (an edge kept at `craft+0x144`) through the dry, full-volume path
    /// rather than the craft's emitter, so a voice line in the player's ear.
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md`, confidence 85. Its
    /// partners are [`Self::Autopilot`] and [`Self::Engaging`]. Pure's
    /// `FUN_0884c794` fires the same cue through the chain `ShieldActive` uses;
    /// its threshold is unread: `docs/ghidra/functions/psp-pure-usa/dry-play-cues.md`,
    /// confidence 78.
    Disengaging,
    /// The player's own craft blowing up.
    ///
    /// `Ship_SetState`'s case 4 (`0x0884430c`, via the nine-entry jump table at
    /// `0x08a7bc18`) arms the `0.5 s` state timer and plays `~BLOWUP` through
    /// `FUN_0883e9b0` (dry, volume `0x400`), keeping the handle at
    /// `craft+0xcac`: held, and the player's alone (`FUN_0883e9b0` returns
    /// without playing when `craft+0x368` is non-zero). See
    /// [`zone-mode.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md),
    /// confidence 85. An opponent's destruction plays nothing here; where its
    /// sound comes from was not found.
    ///
    /// Plays its list as it runs (`[key-on loop, 0x15, key-on, 0x1a 0, 0x16]`):
    /// the loop held from the start, the second waveform re-keyed every 43
    /// master ticks, by [`super::repeating::Held::drive_blowup`].
    ///
    /// The one cue of nine with no confirmed Pure trigger: the string and the
    /// disc cue exist but six search methods found no call site;
    /// `docs/ghidra/functions/psp-pure-usa/blowup-sound-open.md` records them.
    Blowup,
    /// The lock-on reticle, seeking and then locked.
    ///
    /// `HudSight_UpdateTone` (`0x0881b34c`) opens one `~ROCKLOCK` voice the first
    /// frame the reticle has anything and switches a parameter between `0`
    /// (seeking) and `1` (locked), stopping the voice when the target goes. See
    /// [`lock-sight.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/lock-sight.md),
    /// confidence 85.
    ///
    /// A repeating list, not two blips: `~ROCKLOCK` (`hud.bnk` cue 6) is
    /// `[0x15, guard(param 0 == 0), key-on (delay 30), guard(param 0 == 1),
    /// key-on (delay 15), 0x16]`, a beep every 30 master ticks (116 ms)
    /// seeking and every 15 (58 ms) locked, on one 0.052 s waveform (`sound.md`,
    /// "Cue parameters"). It runs as [`oag_formats::sblk::runner::Runner`], held
    /// by [`super::repeating::Held::drive_sight`], which writes parameter 0 each
    /// frame. A title with no measured tick builds no program and falls back to
    /// one blip per forward edge (waveform `0` seeking, `1` locked).
    ///
    /// Pure's `HudSight_UpdateTone` is a near line-for-line match (same
    /// three-state toggle, `0x400` volume, dry-chain middle hop called directly):
    /// `docs/ghidra/functions/psp-pure-usa/lockon-sound.md`, confidence 82.
    LockOn,
    /// A mine leaving the back of a craft, one per charge of a cluster.
    ///
    /// `Weapon_DropMines` (`0x088675cc`) calls `Mine_Init` (`0x08859ac8`) per
    /// charge; its first of two cue plays is `MINELAUNCH`. The emitter argument
    /// traces through two pointer hops to `+0x50`, the firing craft's emitter
    /// (as [`Self::Collision`] and [`Self::Shield`] read it). See
    /// [mine.md](../../../../../docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-06-minelaunch-is-a-plain-positional-craft-emitter-cue---mineradar-is-not),
    /// confidence 78 (two hops, short of single-hop reads' 82-85).
    ///
    /// `MINERADAR`, the second cue, is deliberately not here: it anchors to a
    /// new per-mine emitter `Mine_Init` allocates, a held per-projectile voice
    /// [`super::CueEvent`] and [`super::SfxVoices`] cannot address (held voices
    /// are keyed by grid slot). `BOMBLAUNCH`/`~BOMBRADAR` stay unwired too:
    /// `Weapon_FireBomb` (`0x08863a20`) never calls the play function.
    MineLaunch,
    /// The Plasma's wind-up, at the press rather than at release.
    ///
    /// `Plasma_Init` (`0x0885bd18`, confidence 90) plays it off the firing
    /// craft's own emitter, not the bolt's (`Sound_Play(1.0, craft->emitter, ...,
    /// "PLASMA", 0)` runs before the bolt's emitter is constructed at its
    /// `+0x5c`); decompiled 2026-09-16, see
    /// [plasma.md](../../../../../docs/ghidra/functions/psp-pulse-usa/plasma.md#plasma_init-0x0885bd18-plays-plasma-and-wo_plasma_head).
    /// So it fires when [`oag_weapons::projectile::plasma::CHARGE_SECONDS`]'s
    /// wind-up starts, not at [`Self::PlasmaTravel`]'s release edge. A plain
    /// one-shot: `PLASMA`, 3 waveforms, 0 looping, `weapons.bnk` (`oag-wad sounds`).
    Plasma,
    /// The Plasma bolt's travel loop, from release to whatever ends it.
    ///
    /// `Plasma_Launch` (`0x0885bf84`, confidence 90) starts `Sound_Play(1.0,
    /// p->emitter, ..., "~PLASMATVL", &p->pose)` on the bolt's own emitter,
    /// pointed at the bolt's matrix (`p->emitter->node = &p->matrix`, from
    /// `Plasma_Init`'s decompile 2026-09-16), so it is heard from where the bolt
    /// is, unlike [`Self::Plasma`]. `~PLASMATVL`: one waveform, 1 looping,
    /// `weapons.bnk`.
    ///
    /// Held per projectile slot, not grid slot: the gap `mine.md`'s `MINERADAR`
    /// note names. [`super::SfxVoices::plasma_travel`] holds
    /// [`oag_weapons::projectile::MAX_PROJECTILES`] handles driven off the
    /// world's projectile array each tick, as [`super::Engine`] reads craft
    /// position directly. Never pushed through the cue queue; [`Self::held`] is
    /// `true` so a stray push is dropped rather than played as a one-shot.
    PlasmaTravel,
    /// A Plasma bolt ending on a wall, or the 10 s timeout, never a craft.
    ///
    /// `Plasmas_Update`'s teardown pass (`0x0886b490`, confidence 90) plays it
    /// unconditionally (`Psys_Release_q`, `Plasma_SpawnDetonation`,
    /// `Sound_Play(1.0, p->emitter, ..., "PLASMAHITWALL", 0)`) for a wall hit and
    /// the `10.0 < age` timeout alike (instruction-level reread 2026-09-16), on
    /// the bolt's own emitter where it stopped.
    ///
    /// A craft hit plays [`Self::PlasmaHitShip`] instead: `Plasma_SweepCraftHit`
    /// (`0x0886afb8`) plays `PLASMAHITSHIP` and clears the emitter before pass
    /// two, so the original never plays both (`docs/ghidra/functions/psp-pulse-usa/plasma.md`,
    /// "a craft hit is the third ending"). The gate in `oag_raceplay::tick`
    /// mirrors that split; an earlier pass reused this cue as a **chosen, not
    /// measured** placeholder. A craft ending is
    /// `Impact::struck.is_some()` (`crates/weapons/src/projectile/flight.rs`).
    ///
    /// Placed at the impact point ([`Placement::Point`],
    /// [`super::CueEvent::at_point`]). `PLASMAHITWALL`: 4 waveforms, 0 looping.
    PlasmaHitWall,
    /// A Plasma bolt ending on a struck craft.
    ///
    /// Measured, confidence 88 on the routing. `Plasma_SweepCraftHit`
    /// (`0x0886afb8`), the per-tick hull-cylinder sweep, plays `Sound_Play(1.0,
    /// p->emitter, ..., "PLASMAHITSHIP", 0)` and clears the emitter (`+0x5c = 0`)
    /// before `Plasma_HitCraft` and `Plasma_ApplyBlastForce`, so pass two's
    /// [`Self::PlasmaHitWall`] never also fires, by construction. See
    /// `docs/ghidra/functions/psp-pulse-usa/plasma.md` ("a craft hit is the third
    /// ending", `Plasma_SweepCraftHit` subsection).
    ///
    /// In `weapons.bnk`, bank `#866` (hash `01bec824`): `oag-wad sounds
    /// data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad --cue PLASMAHITSHIP`
    /// reports `cue 14, cmds 35..40, 4 waveform(s), 0 looping, 3.59s total`.
    ///
    /// Placed at the impact point like [`Self::PlasmaHitWall`] (the bolt has its
    /// own emitter), via [`Placement::Point`] and [`super::CueEvent::at_point`].
    PlasmaHitShip,
    /// A Rocket volley's launch, once per press.
    ///
    /// `Ship_FireHeldWeapon` (`0x08844ae8`) plays `Sound_Play(1.0,
    /// *(param_1+0x50), weapons.bnk, 0, "ROCKET", 0)` in its held-id-0 case, a
    /// positional call, not the dry path of [`Self::Disengaging`] and
    /// [`Self::Autopilot`]. Confidence 88, found in the same read as
    /// `QUAKELAUNCH` and the Autopilot cues:
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md` ("`Ship_FireHeldWeapon`
    /// opens both cues", full switch and cross-checks). Local player only: the
    /// switch sits behind `entity+0x368 == 0`, matching `Race::spend_pickup`, so
    /// it fires where [`Self::Leach`] does. It fires on the press, ahead of the
    /// spawn loop, as [`Self::QuakeLaunch`] explains: the gate never tests whether
    /// the volley got anywhere.
    Rocket,
    /// The Rocket bolt's travel loop, from launch to whatever ends it.
    ///
    /// `Rocket_Init` (`0x0885cdb8`) allocates a `0x70`-byte emitter, writes
    /// `+0x38 = 0x44160000` (`600.0f`, a rolloff distance, not
    /// [`oag_audio::Emitter::CRAFT_RADIUS`]) and starts a looping `~ROCKETTVL`
    /// at volume `1.0`. Confidence 85;
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md` ("Audio, in
    /// passing"). Held per projectile slot like [`Self::PlasmaTravel`]
    /// ([`super::travel::TravelVoices`]).
    RocketTravel,
    /// A Rocket bolt ending on a wall or track.
    ///
    /// `RocketPool_Update`'s teardown plays `ROCKEXPLWALL` (flag `0x10`) on the
    /// bolt emitter [`Self::RocketTravel`] plays from; confidence 90. This port's
    /// 5.0 s pool-reap timeout never reaches this edge:
    /// `crates/weapons/src/projectile/flight.rs` resets the slot without writing
    /// an `Impact`, so every impact seen here is a real wall hit. The original is
    /// silent on a timeout too (settled 2026-09-25): `RocketPool_Update`
    /// (`0x0886de60`) sets only flag `0x4` on an age reap and the teardown's
    /// `Sound_Play` branches only on `0x10` vs `0x20`
    /// (`rocket-visuals.md`, "What a rocket hit spends"). Placed at the impact
    /// point ([`super::Placement::Point`]) like [`Self::PlasmaHitWall`].
    RocketHitWall,
    /// A Rocket bolt ending on a struck craft.
    ///
    /// The same teardown plays `ROCKEXPLSHIP` (flag `0x20`); confidence 90. Routed
    /// on `oag_weapons::projectile::Impact::struck`, as [`Self::PlasmaHitShip`].
    RocketHitShip,
    /// The Missile leaving the rail, the player's press or an opponent's
    /// (`Race::fire_missile` serves both).
    ///
    /// `Missile_Init` (`0x0885a160`, confidence 90) plays `MISSILE` and
    /// `~MISSILETVL` together (`missile.md`, "Three independent things say bit
    /// `0x40` is the Missile") without naming the emitter. Placed on the firing
    /// craft: chosen, not measured.
    Missile,
    /// The Missile's travel loop, from launch to whatever ends it.
    ///
    /// `~MISSILETVL` (the bank spells it with `~`; `missile.md` prose says
    /// `_MISSILETVL`, trust the bank) is played by `Missile_Init` (`0x0885a3f8`)
    /// on an emitter the round allocates (`round+0x64`, `+0x50` at the round's
    /// matrix, radius `+0x38 = 0x44160000` = 600.0), handle at `round+0x68`.
    /// Read 2026-09-30, confidence 88: placement and radius, which this port
    /// had chosen by reusing [`Self::RocketTravel`]'s, are measured.
    /// [`Self::Missile`] rides the caller's emitter.
    MissileTravel,
    /// A Missile glancing off a wall: every bounce, not the final ending.
    ///
    /// `missile.md` (around line 391) reads `Missile_Update`'s bounce branch as
    /// playing `WO_MISSILE_BOUNCE` and `MISSILEEXPWALL` together, on the edge of
    /// the bounce visual (`oag_raceplay::weapons::visuals::flares::bounced_this_tick`). A
    /// missile that exhausts its bounces and detonates (`struck: None`,
    /// `blast: true`) plays no cue here: the teardown plays `MISSILEEXPWALL` off
    /// bit `0x10`, whose setter is unread (see [`Self::MissileHitShip`]). Placed
    /// at the bolt's position, radius `600.0` ([`Self::MissileTravel`]).
    MissileHitWall,
    /// A Missile ending on a craft.
    ///
    /// `MissilePool_Update` (`0x08869588`, confidence 88, read whole 2026-09-30)
    /// plays one of two cues off the round's flags: `MISSILEEXPWALL` for bit
    /// `0x10`, `MISSILEEXPSHIP` for `0x20` (the craft-hit bit
    /// `MissilePool_TestCraftHits` sets, as `CannonPool_Update` tests), on the
    /// round's emitter with radius `600.0`. The `Sound_Play` is the `lw` at
    /// `0x08869af8` (pointer cell `0x08a7c93c`, string `0x08a7c92c`). Fires when a
    /// Missile's [`oag_weapons::projectile::Impact`] names a struck craft, at the
    /// impact point, like [`Self::RocketHitShip`].
    MissileHitShip,
    /// A Missile that outlived its fuse.
    ///
    /// The pool's second pass tests `3.0 < age` and plays the cue at pointer cell
    /// `0x08a7c950` on the round's emitter (radius `600.0`) before setting the
    /// destroy bit. That cell holds `"SHURIKENEXPL"` (read at `0x08a7c940`), a
    /// real `weapons.bnk` cue: a copy-paste slip in the original, but the
    /// original plays it, so it plays. Confidence 88 for the trigger; that it is
    /// a slip is inference. Fires for the Missile impact that struck nothing and
    /// spends no blast (`blast: false`), the fuse (`missile_ending_cue` in
    /// `oag_raceplay::weapons`). On HD the call site is unread; HD's bank carries the
    /// cue, so it plays there on Pulse's binary alone.
    MissileExpire,
    /// The Cannon's round leaving the barrel.
    ///
    /// Played at the end of `Cannon_Init` (`0x088648ec`, confidence 80),
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` around
    /// lines 460-474; the emitter is not stated. Placed on the firing craft:
    /// chosen, not measured.
    Cannon,
    /// A Cannon round ending on a wall or the track.
    ///
    /// `CannonPool_Update`'s despawn pass (`0x088582b0`, confidence 88) splits
    /// `CANNONEXPLWALL`/`CANNONEXPLSHIP` on the round's `0x10` (wall) / `0x20`
    /// (craft) flags, set by `Cannon_UpdateRound` and `Cannon_MarkCraftHit`
    /// (mirrored in `oag_weapons::projectile::cannon`). As for
    /// [`Self::RocketHitWall`], `flight.rs`'s `MAX_FLIGHT_SECONDS` reap writes no
    /// `Impact`, so an aged-out round is silent. The original matches (settled
    /// 2026-09-25): `CannonPool_Update` enters the teardown on
    /// `1.0 < age || (flags & 4) != 0`, but `Sound_Play` branches only on `0x10`
    /// vs `0x20`. Placed at the impact point; no emitter override is stated, so
    /// [`oag_audio::Emitter::CRAFT_RADIUS`] stays.
    CannonHitWall,
    /// A Cannon round ending on a struck craft.
    ///
    /// The despawn pass's `0x20` branch. `CANNONEXPLSHIP` plays
    /// `CANNONEXPLWALL`'s nine waveforms, verified: it owns one command in
    /// `Data.wad`'s weapon bank, opcode `0x05` (one of
    /// [`oag_formats::sblk::child::CHILD_OPCODES`]), whose record indexes cue 37
    /// directly. See `cue_tree_sounds` in `crates/formats/src/sblk/child.rs`.
    CannonHitShip,
    /// A Quake wave's launch, once per press.
    ///
    /// `Ship_FireHeldWeapon`'s held-id-2 case plays `Sound_Play(1.0,
    /// *(param_1+0x50), weapons.bnk, 0, "QUAKELAUNCH", 0)`, positional like
    /// [`Self::Rocket`] from the same read
    /// (`docs/ghidra/functions/psp-pulse-usa/autopilot.md`, confidence 88). It
    /// fires on the press, not a successful launch: the busy check is
    /// `Weapon_FireQuake`'s own (`cannon-quake-leachbeam.md`), reached through
    /// `Weapon_RequestFire`'s bit. Wired at `Race::spend_pickup`'s press point
    /// ahead of its busy check, so a re-press while a wave travels still plays
    /// it and the pickup is kept. Local player only (`entity+0x368 == 0`, as
    /// [`Self::Rocket`]); an opponent's Quake (`fire_opponent_quake`) is silent.
    QuakeLaunch,
    /// The Quake's wave reaching a craft, once.
    ///
    /// `QUAKEHIT` plays in the per-craft update on the rising edge of the "wave
    /// reached me" latch (`entity+0x860 & 0x40`), confidence 85,
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` around
    /// lines 930-1001. This port's edge is [`oag_weapons::projectile::quake::Wave::hit`]
    /// rising, snapshotted before `Race::advance_quake`. Placed on the struck
    /// craft: the page names the argument `owner_craft_cue_slot` but the shield
    /// gate and damage fields around it are the victim's, so this sides with the
    /// victim.
    QuakeHit,
    /// The Quake wave travelling: a held loop while its span is active.
    ///
    /// `Quake_Update` (`0x0891d268`, confidence 85), per road span: when
    /// `Quake_SampleSpan` first reports it active it spawns `WO_QUAKE`, allocates
    /// a `SoundEmitter` at the span's matrix with radius `600.0` (`0x44160000`)
    /// and plays `~QUAKETRAVEL` (pointer cell `0x08a88554`, string `0x08a88544`,
    /// `lw` at `0x0891d954`), handle kept per span; on inactive it releases the
    /// effect and stops the voice.
    ///
    /// One voice here, not two (the original has one per span, two at a join).
    /// Placed at the wave's road midpoint, where the `WO_QUAKE` visual follows.
    QuakeTravel,
    /// The LeachBeam firing, locked onto somebody.
    ///
    /// `LeachBeam_InitLocked` (`0x08873d3c`, confidence 82) plays `LEACH` once on
    /// the shooter's emitter
    /// (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` around
    /// lines 2058-2080). Not the unlocked case, which the page does not confirm
    /// for the same cue, so it fires from `Kind::Locked` only, in
    /// `Race::spend_pickup`'s LeachBeam arm and `Race::fire_opponent_leach_beam`.
    Leach,
    /// The LeachBeam fired with nothing to lock onto.
    ///
    /// `LeachBeam_InitUnlocked` (`0x08872da8`, confidence 85) ends with
    /// `Sound_Play(1.0, param_4, ..., "LEACHFAIL", 0)` (pointer cell
    /// `0x08a7cc20`, string `0x08a7cc14`) on the emitter `Weapon_FireLeachBeam`
    /// (`0x08866658`) fills from `shooter->emitter`, the one [`Self::Leach`]
    /// rides. A fire with no lock (`craft+0x16c == -1`) reaches it. Fires from
    /// `Race::spend_pickup`'s unlocked arm; an opponent only fires with a lock.
    LeachFail,
    /// The LeachBeam's body, held while a **locked** beam instance exists,
    /// through its disconnect linger and not only while
    /// [`connected`](oag_weapons::projectile::leach_beam::Beam::connected).
    ///
    /// `LeachBeam_InitLocked` allocates a dedicated `SoundEmitter_Init` emitter at
    /// the instance's `+0x4c` (falloff `600.0`) "carried by the beam itself", so
    /// its life is the `Instance`'s, the span
    /// `oag_raceplay::weapons::visuals::advance_leach_beam_ribbon` reads for the ribbon.
    /// The bank spells it `~LEACHATTACH` (prose says `_LEACHATTACH`; trust the
    /// bank, the trap [`Self::MissileTravel`] warns of). Position, the midpoint
    /// between the two craft, is chosen, not measured: the page never resolves
    /// what the beam's scene node tracks.
    LeachAttach,
    /// A locked LeachBeam's pulse, once each time its ribbon's scroll cursor
    /// wraps to zero.
    ///
    /// `LeachBeam_Advance`'s pulse block re-spawns `WO_LEACHBEAM_ENERGY` at the
    /// target and plays `LEACHENERGY` (falloff `300.0`), both read off the
    /// decompile: `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
    /// "Assets and cues - checked, not assumed". Wired 2026-09-25 off
    /// [`oag_fx::beam::Ribbon::advance`]'s pulse edge, pushed as a
    /// [`super::CueEvent::at_point`] in
    /// `oag_raceplay::weapons::visuals::advance_leach_beam_ribbon`.
    ///
    /// Position is chosen, not measured: the page does not show `LEACHENERGY`'s
    /// emitter argument apart from the effect's, so it uses the effect's point
    /// (`Ribbon::energy_point`), not [`Self::LeachAttach`]'s midpoint, like
    /// [`Self::ShurikenTravel`]'s unread emitter.
    LeachEnergy,
    /// A Shuriken leaving the craft.
    ///
    /// `Shuriken_Init` (`0x08877280`, confidence 88) plays `SHURIKEN` (pointer
    /// cell `0x08a7cd44`, string `0x08a7cd38`, `weapons.bnk` cue 29) on the
    /// caller's emitter, the firing craft's, before it allocates the blade's,
    /// like [`Self::Plasma`]. Fires from `Race::spend_pickup`'s Shuriken arm and
    /// `throw_opponent_shuriken`.
    ShurikenLaunch,
    /// A Shuriken glancing off a wall.
    ///
    /// `Shuriken_Bounce` (`0x088778ac`, confidence 88) plays `SHURIKENHIT` on
    /// every bounce (`Sound_Play` at `0x08877b60`), on the edge of the
    /// `WO_SHURIKEN_BOUNCE` visual (`oag_raceplay::weapons::visuals::flares::bounced_this_tick`).
    /// On the blade's own emitter: it loads `round+0x50`, writes `300.0`
    /// (`0x43960000`) to `+0x38` and plays through it, correcting an earlier
    /// "placed on the firing craft" choice.
    ShurikenHit,
    /// The Shuriken's travel loop, from throw to whatever ends it.
    ///
    /// `Shuriken_Init` allocates a bolt-owned emitter at `round+0x50`
    /// (`SoundEmitter_Init`, pointed at the round's matrix at `round+0xf0`,
    /// `+0x38 = 300.0`) and plays `~SHURIKENTRAVEL` through it, handle at
    /// `round+0x54`. Read 2026-09-30, confidence 88, replacing the reading that
    /// it rode the craft's emitter. Held per projectile slot at the blade's
    /// position, radius `300.0`.
    ShurikenTravel,
    /// The Repulser firing: `REPULSOR`, `weapons.bnk` cue 33, five waveforms.
    ///
    /// `Repulser_Init` (`0x08875210`, confidence 84) plays it with
    /// `Sound_Play(1.0, firer_emitter, ..)` on the firing craft's emitter
    /// (`rec->node->+0x50`). Pushed by `Race::fire_repulser` on the firer's slot.
    /// `docs/ghidra/functions/psp-pulse-usa/repulser.md`.
    Repulsor,
    /// A Repulser wave hitting a craft: `REPULSORHIT`, `weapons.bnk` cue 35.
    ///
    /// `Repulser_HitCraft` (`0x0886d254`, confidence 84) points the Repulser's
    /// emitter at the struck craft's node, sets radius `300.0` (`0x43960000`) and
    /// plays this: one waveform, 1.14 s, on both PSP discs (`sfx_ground_truth`).
    /// `oag-wad sounds` lists zero waveforms in its command range, a limit of
    /// that listing, not the bank.
    RepulsorHit,
    /// The start-of-race voice: `"ready"`, a timeline of several waveforms.
    ///
    /// `RaceMode_SetState` (`0x08827350`) plays it entering state 1, which
    /// `RaceMode_UpdateIntro_q` (`0x08829e6c`) hands the race to after its intro
    /// substates: `Sound_PlayNamedInSlot(..., "ready", 0x400, 0, 0, 0)`, the dry
    /// path of [`Self::Disengaging`]. Measured live on Pulse (PSP), Time Trial,
    /// a single race and Eliminator: it starts 180 ticks before [`Self::Go`]
    /// (`oag_raceplay::countdown`) with no other cue between. Its words are not
    /// identified, and the gantry's `3`, `2`, `1` start no cue.
    ///
    /// The bank is the mode's speech bank, picked by `World_LoadTrack`:
    /// `speech.bnk` (cue 11, six waveforms, 3.73 s), `speech_elim.bnk`
    /// (Eliminator, cue 19, 3.00 s) or `speech_zone.bnk` (Zone, cue 15, 2.48 s).
    /// Not in [`Self::ALL`]: it loads only on a title whose
    /// [`oag_title::RaceDefaults::countdown_voice`] is measured, through
    /// [`super::Banks::load_countdown`].
    /// `docs/ghidra/functions/psp-pulse-usa/countdown-voice.md`.
    Ready,
    /// The voice that says go: `"go"`, 0.80 s in every speech bank.
    ///
    /// `RaceMode_UpdateCountdown` (`0x088274b4`) plays it in the call that runs
    /// `Race_StartRacing` and `RaceMode_SetState(2)` when the timer reaches zero.
    /// Measured live: the frame before the first craft update that applies
    /// thrust, the last gated tick here. See [`Self::Ready`].
    Go,
    /// The announcer line `"cont_elim"`, the only sound an opponent's destruction
    /// makes in the original: `FUN_08840500` (`0x08840590`) plays it dry
    /// (`Sound_PlayNamedInSlot`, `0x400`) when a non-player craft's state 6
    /// expires, in every mode except 2, 8 and 18 (Demo, Elimination, Multiplayer
    /// Elimination); confidence 90 (three live logs and a decompile,
    /// `docs/ghidra/functions/psp-pulse-usa/zone-rest.md`). In the mode's speech
    /// bank, loaded beside [`Self::Ready`]. Raised by `Race::tick_destroyed_craft`.
    ContElim,
    /// The HUD's message line sounding: `"MESSAGE"` in `hud.bnk` (0.34 s), played
    /// dry (`Sound_PlayNamedInSlot`, `0x400`) by `Hud_UpdateMessages`
    /// (`0x0881f148`) the tick a slot starts showing. Decompiled, not yet heard
    /// against a live frame: `docs/ghidra/functions/psp-pulse-usa/hud-messages.md`.
    /// Raised by `Race::tick` off [`oag_hud::messages::MessageBoard::just_shown`].
    Message,
    /// The HD-lineage magstrip hum, started once as a craft comes onto a strip.
    ///
    /// `Ship_StartMagstripSound` (`0x012f8c70`, Omega) starts `_magstrip01`
    /// (`~magstrip01` in HD's `shiphd.bnk`, under a `### Magstrip` label) on a
    /// group named `MagStrip_Player` or `MagStrip_NPC`. The per-tick update
    /// `FUN_01762e80` raises it behind an arming flag that is `1` at
    /// construction: over the strip and armed, start and disarm; on the falling
    /// edge, stop and re-arm. Shape 80, arguments 55 -
    /// `docs/ghidra/functions/ps4-omega-eu/ships-effects.md`, "2026-10-05,
    /// magstrip-omega-law lane". Raised by `Race::advance_magstrip_wake`, on a
    /// title that builds the class only.
    ///
    /// Held: the cue is the start and [`Self::MagstripStop`] the end, so it never
    /// reaches the one-shot path; the arming flag stops a restart until the craft
    /// leaves the strip.
    ///
    /// Chosen, not measured: the sound group has no mixer counterpart, so the
    /// player's craft and a rival's differ only in slot, and `~magstrip01` is a
    /// 35-waveform tree this reader flattens, so the voice is drawn until one
    /// loops (see `sfx::magstrip`).
    Magstrip,
    /// The falling edge of [`Self::Magstrip`]: `Ship_StopMagstripSound`
    /// (`0x01312770`) sets the stop bit on the instance and every group voice. An
    /// action, not a sound: loads nothing, not in [`Self::ALL`].
    MagstripStop,
    /// The front end's cursor moving up a list: `"UPDOWN"` in Pulse's
    /// `frontend.bnk`, `"navUp"` in HD's.
    ///
    /// Pulse: `Sound_PlayNamedInSlot(ui, bank slot 0, "UPDOWN", 0x400, 0, 0, 0)`,
    /// the dry no-emitter path, from `FUN_088a3b1c` (grid move that landed),
    /// `FUN_088a4e60`, `FUN_088a9db4`/`FUN_088a9e7c`, `FUN_088b528c`,
    /// `FUN_088cae88`, `GridSelection_Update` and `TrackSelection_Update`
    /// (`docs/ghidra/functions/psp-pulse-usa/menu-sounds.md`, confidence 88).
    /// Three waveforms on Pulse, one on Pure. HD plays a cue per direction
    /// (`docs/ghidra/functions/ps3-hdfury-eu/menu-sounds.md`). The four move
    /// cues and the two step cues are one role each so a title can name a
    /// cue per direction; the name is [`oag_title::MenuCues`]'s, not
    /// [`Self::name`]'s. Not in [`Self::ALL`]: they load through
    /// [`super::MenuSfx`] from [`oag_title::SoundBanks::frontend`].
    MenuUp,
    /// The cursor moving down. See [`Self::MenuUp`].
    MenuDown,
    /// The cursor moving left. See [`Self::MenuUp`].
    MenuLeft,
    /// The cursor moving right. See [`Self::MenuUp`].
    MenuRight,
    /// A value stepped left: Pulse's `"LEFTRIGHT"` (`FUN_088aaae4`,
    /// `FUN_088cae88` and `FUN_088e4fd0` play it on a row's value changing),
    /// HD's `"navLeft"`. See [`Self::MenuUp`].
    MenuStepLeft,
    /// A value stepped right. See [`Self::MenuStepLeft`].
    MenuStepRight,
    /// A confirmed choice: `"ACCEPT"`. `ConfirmButton_Update` plays it when
    /// the button's redirect resolves, `FUN_088ed05c` the same on the track
    /// screen; a widget's `+0x269` flag suppresses it. HD plays `accept` or,
    /// in the Fury style, `accept_fury`. See [`Self::MenuUp`].
    MenuAccept,
    /// Back, or a refused move: `"DECLINE"`. Played for the back button
    /// (`ConfirmButton_Update`'s secondary-button branch), for a confirm the
    /// button's own check refuses, and for a grid move onto an empty cell
    /// (`FUN_088a3b1c`) - there it takes the place of [`Self::MenuUp`],
    /// it does not follow it. HD's is `reject` or `reject_fury`. See [`Self::MenuUp`].
    MenuDecline,
    /// A line of text typing in: `"TELETYPE"`, one waveform at 48,051 Hz.
    /// `FUN_088b6514` plays it per character, pitched by the glyph's measured
    /// width (`(width / 100 - 1) * 0.2 * 90`). Nothing in this port types text
    /// in, so it is declared with its name and bank and neither loaded nor
    /// fired: it is not in [`Self::FRONT_END`].
    MenuTeletype,
}

mod tables;
