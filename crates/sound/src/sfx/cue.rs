//! [`BankName`] and [`Cue`]: which `.bnk` a cue lives in, and every cue this
//! port fires.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change - the day
//! the Plasma's own three cues (`Cue::Plasma`, `Cue::PlasmaTravel`,
//! `Cue::PlasmaHitWall`) pushed the parent file over the ratchet. `Placement`
//! stays in [`super`], beside [`super::place`] and [`super::CueEvent`], which
//! read it far more than this file does.

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
    /// so this fires on exactly the edge `oag_raceplay::Race::test_speedup_pads`
    /// already arms the flare on. `docs/ghidra/functions/psp-pulse-usa/pads.md`,
    /// confidence 88.
    ///
    /// Pure's `FUN_0886b548` is the same two-branch (positional/dry)
    /// dispatcher in one function, firing the same `SPEEDUPPAD` on either
    /// branch. `docs/ghidra/functions/psp-pure-usa/dry-play-cues.md`,
    /// confidence 78.
    SpeedupPad,
    /// The perfect start: a human craft's first thrust landing inside the
    /// launch boost's perfect window.
    ///
    /// `Race_UpdateLaunchGrade` (`0x0882773c`) writes grade 2 and calls
    /// `ExhaustFlare_OnPerfectStart` (`0x08904fd4`), which arms the engine
    /// flare's boost timer to its constructor's `0.8` and plays `"TURBO"` out
    /// of `weapons.bnk` (`DAT_08ac1df8`) through the same two branches
    /// [`Self::SpeedupPad`]'s `ExhaustFlare_OnSpeedupPad` takes.
    /// `docs/ghidra/functions/psp-pulse-usa/perfect-start.md`, confidence 85,
    /// not watched.
    Turbo,
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
    /// A pickup absorbed into the pool, or Eliminator's lap refill.
    ///
    /// `Ship_PlayAbsorbFeedback` (`0x08840640`) "plays an `ABSORB` sound
    /// once" before staggering its ten spark instances. **Not a shielded
    /// contact**, which this comment used to say: its four callers are the
    /// absorb handler, `Ship_RefillLapShield` and two network callbacks, and
    /// the contact loop's shield branch plays nothing
    /// (`docs/ghidra/functions/psp-pulse-usa/shield.md`, 2026-09-16).
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
    /// is in [`super::Engine`].
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
    /// The Autopilot's own hum, held for as long as the pickup is active.
    ///
    /// `Ship_FireHeldWeapon`'s case for held id 6 opens it through
    /// `FUN_0883e9b0` - the same **dry, no-emitter** path [`Self::Blowup`]
    /// and [`Self::Disengaging`] both take - and keeps the handle at
    /// `param_1+0x58`. `Autopilot_Update`'s own `<= 0.0f` arm - the timer's
    /// expiry, not the one-second `disengaging` warning edge - is where that
    /// same handle is released: `if (entity->0x58 != 0) { Scream_StopSound();
    /// entity->0x58 = 0; }`, read directly rather than presumed. See
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md`'s
    /// "`Ship_FireHeldWeapon` opens both cues" and "`Autopilot_Update` counts
    /// it down" sections, confidence 88 - **wired 2026-09-25**, correcting
    /// that page's own earlier "no located opener" claim. `Data\Sound\hud.bnk`,
    /// three waveforms, one looping, 3.04 s total (`oag-wad sounds`, verified
    /// against `pulse-psp-usa.chd`). **Read straight off the executable's own
    /// bytes at `0x08a7b6b0` (`read_memory`, not the Ghidra auto-label)**:
    /// the string is `~AUTOPILOT`, tilde included - the label
    /// `PTR_s__AUTOPILOT_08a7b6af_1_08a7b6bc` only *looks* like an
    /// underscore/tilde split because Ghidra substitutes `_` for a
    /// non-identifier byte in an auto-generated symbol name, and its own
    /// `_1` offset points one byte past the label, landing exactly on the
    /// `~`. No trap here, unlike [`Self::LeachAttach`]'s own genuine one.
    ///
    /// **Driven off the level, `ships[0].autopilot_timer > 0.0`, the same shape
    /// [`Self::Blowup`] takes off `craft_is_exploding`** - not pushed
    /// through the cue queue - because the timer's own expiry is exactly
    /// `Autopilot_Update`'s release condition above, so a level match
    /// reproduces "open on the rising edge, close on the falling edge"
    /// without a second push site.
    Autopilot,
    /// The one-shot "autopilot engaged" line, on the same edge as
    /// [`Self::Autopilot`].
    ///
    /// The second half of the same `Ship_FireHeldWeapon` case, immediately
    /// after [`Self::Autopilot`]'s own open: `FUN_0883e9b0(param_1, speech.bnk,
    /// "autopilot_eng", 0x400, 0)`, return discarded - a one-shot, not a
    /// held loop, the same dry path [`Self::Disengaging`] (its own "about to
    /// let go" partner) already takes. `Data\Sound\speech.bnk`, two
    /// waveforms, 3.14 s total (`oag-wad sounds`, verified against
    /// `pulse-psp-usa.chd`) - `autopilot.md`'s own earlier "cue 0, 1.28 s"
    /// note was one waveform's own length, not the cue's total. Same
    /// section, same confidence 88, same 2026-09-25 date.
    Engaging,
    /// The announcer, one second before an Autopilot pickup lets go.
    ///
    /// `Autopilot_Update` (`0x08861404`) plays it on the tick the remaining
    /// time crosses `1.0` - an edge it keeps `craft+0x144` for - through the
    /// **dry, full-volume** path rather than through the craft's emitter, which
    /// is what says it is a voice line in the player's ear rather than a thing
    /// happening in the world. `docs/ghidra/functions/psp-pulse-usa/autopilot.md`,
    /// confidence 85.
    ///
    /// **Its two partners on the disc are wired too, as [`Self::Autopilot`]
    /// and [`Self::Engaging`]** - correcting this doc comment's own earlier
    /// claim that neither had a located opener; see those variants' own doc
    /// comments.
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
    /// See [`zone-mode.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md),
    /// confidence 85.
    ///
    /// **An opponent's destruction plays nothing here**, and that is the
    /// reading rather than a gap in it - whatever an opponent's explosion
    /// sounds like comes from somewhere this pass did not find.
    ///
    /// **Plays its list as it runs** (`[key-on loop, 0x15, key-on, 0x1a 0,
    /// 0x16]`): the loop held from the start and the second waveform re-keyed
    /// every 43 master ticks until the explosion ends, by
    /// [`super::repeating::Held::drive_blowup`].
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
    /// [`lock-sight.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/lock-sight.md),
    /// confidence 85.
    ///
    /// **It is a repeating list, not two blips.** `~ROCKLOCK` (`hud.bnk` cue 6)
    /// is `[0x15, guard(param 0 == 0), key-on (delay 30), guard(param 0 == 1),
    /// key-on (delay 15), 0x16]`: a beep every 30 master ticks (116 ms) while
    /// seeking, every 15 (58 ms) once locked, both binding the same 0.052 s
    /// waveform. The guard, the parameter write and the loop-back are read in
    /// `sound.md`'s "Cue parameters" section; the list runs as
    /// [`oag_formats::sblk::runner::Runner`] and is held open by
    /// [`super::repeating::Held::drive_sight`], which writes parameter 0 each
    /// frame and stops the voice when the target goes away.
    ///
    /// **A title with no measured tick** builds no program and falls back to
    /// one blip per forward edge (waveform `0` seeking, `1` locked), the
    /// behaviour this cue had before the list was run.
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
    /// [mine.md](../../../../../docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-06-minelaunch-is-a-plain-positional-craft-emitter-cue---mineradar-is-not),
    /// confidence 78 (two hops of indirection, short of the single-hop reads'
    /// 82-85).
    ///
    /// **`MINERADAR`, the second cue of the same pair, is deliberately not
    /// here.** It anchors to a *new emitter `Mine_Init` allocates for the mine
    /// entity itself*, a held, per-projectile voice - a shape nothing in
    /// [`super::CueEvent`] or [`super::SfxVoices`] can address, since every held voice this
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
    /// see [plasma.md](../../../../../docs/ghidra/functions/psp-pulse-usa/plasma.md#plasma_init-0x0885bd18-plays-plasma-and-wo_plasma_head).
    /// So this fires at the press, the same tick [`oag_weapons::projectile::plasma::CHARGE_SECONDS`]'s
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
    /// gap: [`super::SfxVoices::plasma_travel`] is an array of
    /// [`oag_weapons::projectile::MAX_PROJECTILES`] voice handles, read and
    /// written directly off the world's own projectile array every tick -
    /// the same reason [`super::Engine`] reads craft position directly rather than
    /// through a queued [`super::CueEvent`]. Never pushed through the cue queue
    /// itself; [`Self::held`] returns `true` for it so a stray push would be
    /// silently dropped rather than mis-played as a one-shot.
    PlasmaTravel,
    /// A Plasma bolt ending on a wall, or the 10 s timeout - never a craft.
    ///
    /// `Plasmas_Update`'s teardown pass (`0x0886b490`, confidence 90) plays it
    /// unconditionally - `Psys_Release_q`, `Plasma_SpawnDetonation`,
    /// `Sound_Play(1.0, p->emitter, ..., "PLASMAHITWALL", 0)`, nothing else -
    /// for a wall hit and the `10.0 < age` timeout alike; re-read at
    /// instruction level 2026-09-16 with no elision. On the bolt's own
    /// emitter, at wherever it stopped - see [`Self::PlasmaTravel`] for why
    /// that is not the firing craft.
    ///
    /// **Does not fire on a craft hit - that ending plays [`Self::PlasmaHitShip`]
    /// instead.** This port's own Plasma can also end on a craft
    /// (`Impact::struck.is_some()`, `crates/weapons/src/projectile/flight.rs`'s
    /// shared sweep-segment test), a third ending closed 2026-09-16:
    /// `Plasma_SweepCraftHit` (`0x0886afb8`) plays `PLASMAHITSHIP` on the
    /// bolt's own emitter and **clears that emitter** before pass-two's
    /// unconditional `PLASMAHITWALL` runs, so the original never plays both -
    /// see `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "a craft hit is
    /// the third ending" section. This variant's own gate in
    /// `crate::race::tick` mirrors that split rather than reusing this cue
    /// for every ending, which an earlier pass did as a **chosen, not
    /// measured** placeholder before `Plasma_SweepCraftHit` was read.
    ///
    /// **Placed at the impact point, not at a craft** - see [`Placement::Point`]
    /// and [`super::CueEvent::at_point`]. Bank data: `oag-wad sounds` reports
    /// `PLASMAHITWALL` as 4 waveforms, 0 looping.
    PlasmaHitWall,
    /// A Plasma bolt ending on a struck craft.
    ///
    /// **Measured, confidence 88 on the routing.** `Plasma_SweepCraftHit`
    /// (`0x0886afb8`), the per-tick hull-cylinder sweep every flying bolt
    /// runs, plays `Sound_Play(1.0, p->emitter, ..., "PLASMAHITSHIP", 0)` and
    /// immediately clears that emitter (`+0x5c = 0`) *before* calling
    /// `Plasma_HitCraft` and `Plasma_ApplyBlastForce` - which is why pass-two's
    /// later, unconditional [`Self::PlasmaHitWall`] never also fires on this
    /// path: the emitter it tests is already cleared. No double sound, by
    /// construction rather than by a branch on which ending this is. See
    /// `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "a craft hit is the
    /// third ending" section, `Plasma_SweepCraftHit`'s own subsection.
    ///
    /// **Confirmed in `weapons.bnk`, bank `#866` (hash `01bec824`)**, not just
    /// the executable's own string - `oag-wad sounds
    /// data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad --cue
    /// PLASMAHITSHIP` reports `cue 14, cmds 35..40, 4 waveform(s), 0 looping,
    /// 3.59s total` - the same shape [`Self::PlasmaHitWall`]'s own 4
    /// waveforms, 0 looping already carries.
    ///
    /// **Placed at the impact point, not at a craft** - the same reasoning as
    /// [`Self::PlasmaHitWall`]: the bolt has its own emitter, not the struck
    /// craft's, so this rides [`Placement::Point`] and
    /// [`super::CueEvent::at_point`] identically.
    PlasmaHitShip,
    /// A Rocket volley's own launch, once per press.
    ///
    /// `Ship_FireHeldWeapon` (`0x08844ae8`) plays `Sound_Play(1.0,
    /// *(param_1+0x50), weapons.bnk, 0, "ROCKET", 0)` on its case for held
    /// id 0 - a plain positional call, not the dry no-emitter path
    /// [`Self::Disengaging`] and [`Self::Autopilot`] take - **found in the
    /// same read that settled `QUAKELAUNCH` and the Autopilot's own two
    /// cues**, confidence 88; see
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md`'s
    /// "`Ship_FireHeldWeapon` opens both cues" section, which carries the
    /// full switch and the cross-checks (this page's own id-to-bit table
    /// already named the function; that section is what actually
    /// decompiled it). **Local player only**: the whole switch sits behind
    /// `entity+0x368 == 0`, matching this port's own `Race::spend_pickup`,
    /// which is likewise never called for an opponent - so this fires at
    /// the same point [`Self::Leach`] does for the player's own launch, not
    /// through the opponent-fire helpers weapons like the LeachBeam and the
    /// Quake also have. **Fires on the press itself, ahead of the spawn
    /// loop** - the same shape [`Self::QuakeLaunch`]'s own doc comment
    /// explains: this function's own gate never tests whether the volley
    /// actually got anywhere, so neither does this port's own push site.
    Rocket,
    /// The Rocket bolt's own travel loop, from launch to whatever ends it.
    ///
    /// `Rocket_Init` (`0x0885cdb8`) allocates a `0x70`-byte emitter, writes
    /// `+0x38 = 0x44160000` (`600.0f`, a rolloff distance - **not**
    /// [`oag_audio::Emitter::CRAFT_RADIUS`]) and starts a looping
    /// `~ROCKETTVL` on it at volume `1.0`. Confidence 85; see
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`'s "Audio, in
    /// passing" section. Held per **projectile** slot, the same shape
    /// [`Self::PlasmaTravel`] established - see
    /// [`super::travel::TravelVoices`].
    RocketTravel,
    /// A Rocket bolt ending on a wall or track.
    ///
    /// `RocketPool_Update`'s teardown pass plays `ROCKEXPLWALL` (flag `0x10`)
    /// on the same bolt emitter [`Self::RocketTravel`] plays from - never
    /// reallocated - for a wall/track hit; confidence 90. **This port's own
    /// 5.0 s pool-reap timeout never reaches this edge**:
    /// `crates/weapons/src/projectile/flight.rs`'s own Rocket-timeout branch
    /// (`kind == Weapon::Rocket && age > rocket::LIFETIME_SECONDS`) resets
    /// the slot and `continue`s *without* writing an `Impact` at all, so no
    /// `struck: None` from a timeout ever reaches this port's own impacts
    /// loop - every one it sees there is a real wall hit, settling which cue
    /// *this port's* reap plays (none) without needing to disambiguate one
    /// from a wall hit. **What the original itself plays on that same
    /// timeout is now settled too, 2026-09-25**: `RocketPool_Update` itself
    /// (`0x0886de60`) was read whole - its age-only reap sets flag `0x4` and
    /// nothing else, and the teardown's `Sound_Play` only ever branches on
    /// `0x10` vs `0x20`, so a rocket with neither set falls through and
    /// plays no cue at all. See `rocket-visuals.md`'s "What a rocket hit
    /// spends" section - the original is silent on a timeout, matching this
    /// port. Placed at the impact point ([`super::Placement::Point`]), same
    /// reasoning as [`Self::PlasmaHitWall`].
    RocketHitWall,
    /// A Rocket bolt ending on a struck craft.
    ///
    /// The same teardown pass plays `ROCKEXPLSHIP` (flag `0x20`) instead, on
    /// a craft hit; confidence 90. This port's own split reads
    /// `oag_weapons::projectile::Impact::struck`, the same field
    /// [`Self::PlasmaHitShip`] already routes on.
    RocketHitShip,
    /// The Missile leaving the rail, both the player's own press and an
    /// opponent's - `Race::fire_missile` serves both paths.
    ///
    /// `Missile_Init` (`0x0885a160`, confidence 90) plays `MISSILE` and
    /// `~MISSILETVL` in the same breath - `missile.md`'s "Three independent
    /// things say bit `0x40` is the Missile" cites both cues by name - but
    /// does not name either call's emitter argument. **Placed on the firing
    /// craft: chosen, not measured.**
    Missile,
    /// The Missile's own travel loop, from launch to whatever ends it.
    ///
    /// `~MISSILETVL` (the bank spells it with `~`; `missile.md`'s own prose
    /// calls it `_MISSILETVL`, trust the bank) is played by `Missile_Init`
    /// (`0x0885a3f8`) through an emitter the round allocates for itself
    /// (`round+0x64`, `+0x50` pointed at the round's own matrix), its radius
    /// `+0x38` written to `0x44160000` = **600.0**, the handle kept at
    /// `round+0x68`. Read 2026-09-30, confidence 88: the placement and the
    /// radius, which this port had chosen by reusing [`Self::RocketTravel`]'s,
    /// are measured. [`Self::Missile`] rides the emitter the caller passes.
    MissileTravel,
    /// A Missile glancing off a wall - **every bounce, not the final
    /// ending.**
    ///
    /// `missile.md` (around line 391) reads `Missile_Update`'s bounce branch
    /// as playing `WO_MISSILE_BOUNCE` and the `MISSILEEXPWALL` cue together,
    /// on the same edge the already-built bounce visual fires on - see
    /// `crate::race::weapons::visuals::bounced_this_tick`, which this cue
    /// shares the `bounces_before`/after comparison with. **Not the
    /// projectile's final ending**: a missile that exhausts its bounce
    /// budget and detonates on a wall (`struck: None`, `blast: true`) plays
    /// neither this nor any other cue here - the teardown plays
    /// `MISSILEEXPWALL` off bit `0x10`, whose setter is not read (see
    /// [`Self::MissileHitShip`] for the teardown itself). Placed at the
    /// bolt's own position, the `600.0` radius [`Self::MissileTravel`]
    /// measures.
    MissileHitWall,
    /// A Missile ending on a craft.
    ///
    /// `MissilePool_Update` (`0x08869588`, confidence 88, read whole
    /// 2026-09-30) opens its teardown on the destroy bit and plays exactly one
    /// of two cues off the round's own flags: `MISSILEEXPWALL` when bit `0x10`
    /// is set, **`MISSILEEXPSHIP` when bit `0x20` is** (the craft-hit bit
    /// `MissilePool_TestCraftHits` sets, the same one `CannonPool_Update`'s
    /// teardown tests), each on the round's own emitter with its radius written
    /// to `600.0` (`0x44160000`) just before. The `Sound_Play` for this one is
    /// the `lw` at `0x08869af8` (pointer cell `0x08a7c93c`, string `0x08a7c92c`).
    /// Fires from the impact loop when a Missile's [`oag_weapons::projectile::Impact`]
    /// names a struck craft, at the impact point, like [`Self::RocketHitShip`].
    MissileHitShip,
    /// A Missile that outlived its fuse.
    ///
    /// The pool's second pass tests `3.0 < age` on every live round and, still
    /// holding the round's active bit, plays the cue at pointer cell
    /// `0x08a7c950` on the round's emitter (radius `600.0`) before it sets the
    /// destroy bit. **That cell holds `"SHURIKENEXPL"`**, not a Missile name -
    /// read directly, `0x08a7c940` - and `SHURIKENEXPL` is a real cue in
    /// `weapons.bnk`. Earlier notes took the string for a misread and left it
    /// unwired; it is the disc's own data (a copy-paste in the original, but
    /// the original plays it), so it plays. Confidence 88 for the trigger; that
    /// it is a slip of the authors' is inference. Fires for the one Missile impact
    /// that struck nothing and spends no blast (`blast: false`), the fuse
    /// (`missile_ending_cue` in `race::weapons`). **On HD the call site is
    /// unread**: HD's bank carries the cue, so it plays there too, on the
    /// strength of Pulse's binary alone.
    MissileExpire,
    /// The Cannon's own round leaving the barrel.
    ///
    /// Played "at the end of `Cannon_Init`" (`0x088648ec`, confidence 80) -
    /// see `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
    /// around lines 460-474. The emitter argument is not stated.
    /// **Placed on the firing craft: chosen, not measured.**
    Cannon,
    /// A Cannon round ending on a wall or the track.
    ///
    /// `CannonPool_Update`'s despawn pass (`0x088582b0`, confidence 88)
    /// splits `CANNONEXPLWALL`/`CANNONEXPLSHIP` on the round's own `0x10`
    /// (wall) / `0x20` (craft) flags, set by `Cannon_UpdateRound`'s
    /// world-hit branch and `Cannon_MarkCraftHit` respectively -
    /// `oag_weapons::projectile::cannon`'s own module doc already mirrors
    /// this exact split. **The same timeout reasoning as
    /// [`Self::RocketHitWall`] applies**: `flight.rs`'s shared
    /// `MAX_FLIGHT_SECONDS` reap (the branch every weapon but Rocket,
    /// Missile and Plasma falls into) resets the slot with no `Impact`
    /// written, so a Cannon round that outlives its flight time is silent
    /// here too, never a false `struck: None`. **The original matches,
    /// settled 2026-09-25**: `CannonPool_Update` (`0x088582b0`) reaches the
    /// same teardown block on `1.0 < age || (flags & 4) != 0` - the age test
    /// is an `||` on the gate itself, not a bit it sets first the way the
    /// Rocket's does - but once inside, `Sound_Play` only ever branches on
    /// `0x10` vs `0x20`, so a round that ages out with neither flag set
    /// still plays nothing. Placed at the impact point; no emitter override
    /// is stated, so this keeps [`oag_audio::Emitter::CRAFT_RADIUS`].
    CannonHitWall,
    /// A Cannon round ending on a struck craft.
    ///
    /// The same despawn pass's `0x20` branch. **`CANNONEXPLSHIP` plays
    /// `CANNONEXPLWALL`'s own nine waveforms, verified rather than assumed
    /// empty**: it owns exactly one command in `Data.wad`'s weapon bank,
    /// opcode `0x05` - one of [`oag_formats::sblk::child::CHILD_OPCODES`] -
    /// whose record indexes cue 37, `CANNONEXPLWALL`, directly. So a craft
    /// hit sounds like a wall hit, by the disc's own construction. See
    /// `crates/formats/src/sblk/child.rs`'s own `cue_tree_sounds` doc
    /// comment for the mechanism.
    CannonHitShip,
    /// A Quake wave's own launch, once per press.
    ///
    /// `Ship_FireHeldWeapon`'s case for held id 2 plays `Sound_Play(1.0,
    /// *(param_1+0x50), weapons.bnk, 0, "QUAKELAUNCH", 0)` - the same
    /// positional shape [`Self::Rocket`] takes, from the same read; see that
    /// variant's own doc comment and
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md`'s
    /// "`Ship_FireHeldWeapon` opens both cues" section, confidence 88.
    /// **Fires on the button press, not on a successful launch**: this
    /// function's own gate never tests whether a wave is already in
    /// flight - that busy check is `Weapon_FireQuake`'s own, read
    /// separately in `cannon-quake-leachbeam.md` and reached later, through
    /// `Weapon_RequestFire`'s bit rather than this switch. **Wired at
    /// `Race::spend_pickup`'s own press point, ahead of its busy check**,
    /// matching that: a re-press while a wave already travels still plays
    /// this, the same "even on a re-press" shape the original's own gate has,
    /// with the pickup itself kept rather than spent (the busy check still
    /// runs, just after the cue). Local player only, the same
    /// `entity+0x368 == 0` gate [`Self::Rocket`] has - an opponent's Quake
    /// (`fire_opponent_quake`) stays silent here.
    QuakeLaunch,
    /// The Quake's travelling wave reaching a craft, once.
    ///
    /// `QUAKEHIT` (confirmed string) plays inside the per-craft update on
    /// the rising edge of the "wave reached me" latch (`entity+0x860 &
    /// 0x40`), confidence 85 -
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
    /// around lines 930-1001. This port's own edge is
    /// [`oag_weapons::projectile::quake::Wave::hit`]'s own rising edge,
    /// snapshotted before `Race::advance_quake` and compared after.
    /// **Placed on the struck craft**: the doc's own pseudocode names the
    /// argument `owner_craft_cue_slot`, but the surrounding block's
    /// shield-gate and every pending-damage field it touches are the
    /// *victim's* - the page's own naming is internally inconsistent, and
    /// this port sides with the victim rather than the shooter.
    QuakeHit,
    /// The Quake wave travelling: a held loop while its span is active.
    ///
    /// `Quake_Update` (`0x0891d268`, confidence 85), per road span: when
    /// `Quake_SampleSpan` first reports the span active it spawns `WO_QUAKE`,
    /// allocates a `SoundEmitter` pointed at the span's own matrix with its
    /// radius written to `600.0` (`0x44160000`), and plays `~QUAKETRAVEL`
    /// (pointer cell `0x08a88554`, string `0x08a88544`, `lw` at `0x0891d954`)
    /// with a handle kept per span; when the span goes inactive it releases the
    /// effect and stops the voice. So the loop lasts as long as the wave.
    ///
    /// **One voice here, not two**: the original has a voice per span (two at
    /// most, when the wave crosses a join), this port a single travelling wave.
    /// **Placed at the wave's own road midpoint**, the same point the `WO_QUAKE`
    /// visual follows; the original's is the span midpoint, which is that.
    QuakeTravel,
    /// The LeachBeam firing, locked onto somebody.
    ///
    /// `LeachBeam_InitLocked` (`0x08873d3c`, confidence 82) plays `LEACH`
    /// once on the shooter's own emitter - see
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
    /// around lines 2058-2080. **The unlocked/fizzle case is not this
    /// variant**: only `LeachBeam_InitLocked` is cited, and the page does
    /// not confirm the same cue for `LeachBeam_InitUnlocked`'s no-lock
    /// case - which is why this fires from the `Kind::Locked` arm alone, at
    /// both the places a beam can be locked and fired: `Race::spend_pickup`'s
    /// own LeachBeam arm and `Race::fire_opponent_leach_beam`.
    Leach,
    /// The LeachBeam fired with nothing to lock onto.
    ///
    /// `LeachBeam_InitUnlocked` (`0x08872da8`, confidence 85) ends with
    /// `Sound_Play(1.0, param_4, ..., "LEACHFAIL", 0)`: pointer cell
    /// `0x08a7cc20`, string `0x08a7cc14`, on the emitter it is handed, which
    /// `Weapon_FireLeachBeam` (`0x08866658`) fills from `shooter->emitter` - the
    /// same craft emitter [`Self::Leach`] rides. It is the constructor a fire
    /// with no lock (`craft+0x16c == -1`) reaches, so an unlocked shot plays
    /// this where a locked one plays `LEACH`. Fires from
    /// `Race::spend_pickup`'s unlocked arm; an opponent only fires with a lock.
    LeachFail,
    /// The LeachBeam's own body, held for as long as a **locked** beam
    /// instance exists - through its disconnect linger, not only while it
    /// is [`connected`](oag_weapons::projectile::leach_beam::Beam::connected).
    ///
    /// `LeachBeam_InitLocked` allocates a dedicated `SoundEmitter_Init`
    /// emitter at the instance's own `+0x4c`, `600.0`-unit falloff, and the
    /// page states outright that it is "carried by the beam itself" - so
    /// its lifetime is the `Instance`'s own, the same span
    /// `crate::race::weapons::advance_leach_beam_ribbon` already reads for
    /// the visual ribbon (`Kind::Locked`, whether or not the link is still
    /// connected). Bank spells it `~LEACHATTACH`; `missile.md`-style prose
    /// elsewhere calls it `_LEACHATTACH` - trust the bank, the same
    /// underscore/tilde trap [`Self::MissileTravel`] and [`Self::RocketTravel`]
    /// already carry warnings about. **The position it is heard from -
    /// chosen as the midpoint between the two craft - is not stated**; the
    /// page never resolves what the beam's own scene node tracks.
    LeachAttach,
    /// A locked LeachBeam's own pulse, once each time its ribbon's scroll
    /// cursor wraps to zero.
    ///
    /// `LeachBeam_Advance`'s pulse block re-spawns `WO_LEACHBEAM_ENERGY` at
    /// the target *and* plays `LEACHENERGY` from that same block, falloff
    /// `300.0` - both read straight off the decompile, not inferred from the
    /// effect alone;
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
    /// "Assets and cues - checked, not assumed". **Wired 2026-09-25** off
    /// [`oag_fx::beam::Ribbon::advance`]'s own pulse-edge return, which
    /// `crate::race::weapons::visuals::Race::advance_leach_beam_ribbon` now
    /// pushes a [`super::CueEvent::at_point`] on rather than only reading for
    /// the effect respawn - the edge that page's own 2026-09-23 note left
    /// "read and discarded".
    ///
    /// **Position is chosen, not measured**: the page confirms the pulse
    /// block plays `LEACHENERGY` and re-spawns `WO_LEACHBEAM_ENERGY` at the
    /// target *together*, and gives the cue's own `300.0` falloff, but does
    /// not itself show `LEACHENERGY`'s own `Sound_Play` emitter argument
    /// separately from the effect's. This reuses the same point the effect
    /// spawns at (`Ribbon::energy_point`), not [`Self::LeachAttach`]'s
    /// owner/target midpoint - the same kind of choice
    /// [`Self::ShurikenTravel`]'s own doc comment below makes for an
    /// unread emitter argument, carrying no confidence score of its own.
    LeachEnergy,
    /// A Shuriken leaving the craft.
    ///
    /// `Shuriken_Init` (`0x08877280`, confidence 88) plays `SHURIKEN` (pointer
    /// cell `0x08a7cd44`, string `0x08a7cd38`) on the emitter its caller hands it
    /// as its last argument - the firing craft's own, before it allocates the
    /// blade's - the same shape [`Self::Plasma`] takes. Earlier notes could not
    /// verify this name; the string is `"SHURIKEN"` and the cue is in
    /// `weapons.bnk` (cue 29). Fires from `Race::spend_pickup`'s Shuriken arm
    /// and `throw_opponent_shuriken`, on the craft.
    ShurikenLaunch,
    /// A Shuriken glancing off a wall.
    ///
    /// `Shuriken_Bounce` (`0x088778ac`, confidence 88) plays `SHURIKENHIT` on
    /// every bounce (`Sound_Play` at `0x08877b60`), matching the `WO_SHURIKEN_BOUNCE`
    /// visual's edge; see `crate::race::weapons::visuals::bounced_this_tick`.
    /// **On the blade's own emitter**: it loads `round+0x50`, writes `300.0`
    /// (`0x43960000`) to its `+0x38` and plays through it. This corrects the
    /// earlier "placed on the firing craft" choice, made when no bolt-owned
    /// emitter had been read.
    ShurikenHit,
    /// The Shuriken's own travel loop, from throw to whatever ends it.
    ///
    /// `Shuriken_Init` allocates a bolt-owned emitter at `round+0x50` (a
    /// `SoundEmitter_Init`, `+0x50` pointed at the round's own matrix at
    /// `round+0xf0`, `+0x38 = 300.0`) and plays `~SHURIKENTRAVEL` through it,
    /// keeping the handle at `round+0x54`. Read 2026-09-30, confidence 88; it
    /// replaces the earlier reading that Shuriken had no emitter of its own and
    /// rode the craft's. Held per **projectile slot**, at the blade's position,
    /// radius `300.0`.
    ShurikenTravel,
    /// The Repulser firing: `REPULSOR`, `weapons.bnk` cue 33, five waveforms.
    ///
    /// `Repulser_Init` (`0x08875210`, confidence 84) plays it with
    /// `Sound_Play(1.0, firer_emitter, ..)` on the emitter its caller passes -
    /// the firing craft's own (`rec->node->+0x50`). Pushed by
    /// `Race::fire_repulser` on the firer's slot. See
    /// `docs/ghidra/functions/psp-pulse-usa/repulser.md`.
    Repulsor,
    /// A Repulser wave hitting a craft: `REPULSORHIT`, `weapons.bnk` cue 35.
    ///
    /// `Repulser_HitCraft` (`0x0886d254`, confidence 84) points the Repulser's
    /// own emitter at the struck craft's node, sets its radius to `300.0`
    /// (`0x43960000`) and plays this: one waveform, 1.14 s, on both PSP discs
    /// (`sfx_ground_truth`). `oag-wad sounds` lists it with zero waveforms in its
    /// own command range, which is that listing's limit, not the bank's.
    RepulsorHit,
    /// The start-of-race voice: `"ready"`, a timeline of several waveforms.
    ///
    /// `RaceMode_SetState` (`0x08827350`) plays it when it enters state 1, the
    /// state `RaceMode_UpdateIntro_q` (`0x08829e6c`) hands the race to at the
    /// end of its intro substates - `Sound_PlayNamedInSlot(..., "ready", 0x400,
    /// 0, 0, 0)`, the dry, no-emitter path [`Self::Disengaging`] takes. Measured
    /// live on Pulse (PSP), Time Trial, a single race and Eliminator: it starts
    /// **180 ticks before [`Self::Go`]** (see `crate::race::countdown` for the
    /// ticks), with no other cue started between the two. What its words say is
    /// not identified, and the gantry's `3`, `2`, `1` start no cue of their own.
    ///
    /// **The bank is the mode's speech bank**, which `World_LoadTrack` picked:
    /// `speech.bnk` (cue 11, six waveforms, 3.73 s), `speech_elim.bnk`
    /// (Eliminator, cue 19, 3.00 s) or `speech_zone.bnk` (Zone, cue 15, 2.48
    /// s). Not one of [`Self::ALL`]: it loads only on a title whose
    /// [`oag_title::RaceDefaults::countdown_voice`] is measured, into the same
    /// maps, through [`super::Banks::load_countdown`].
    /// `docs/ghidra/functions/psp-pulse-usa/countdown-voice.md`.
    Ready,
    /// The voice that says go: `"go"`, 0.80 s in every speech bank.
    ///
    /// `RaceMode_UpdateCountdown` (`0x088274b4`) plays it in the same call that
    /// runs `Race_StartRacing` and `RaceMode_SetState(2)`, when the state's
    /// timer reaches zero. Measured live: **the frame before the first craft
    /// update that applies thrust**, which is the last gated tick here. See
    /// [`Self::Ready`] for the bank and the trigger's home.
    Go,
    /// The announcer line `"cont_elim"`, the only sound an opponent's destruction
    /// makes in the original: `FUN_08840500` (`0x08840590`) plays it dry
    /// (`Sound_PlayNamedInSlot`, `0x400`) when a non-player craft's state 6
    /// expires, in every game mode except 2, 8 and 18 (Demo, Elimination,
    /// Multiplayer Elimination), confidence 90 (three live logs and a decompile;
    /// `docs/ghidra/functions/psp-pulse-usa/zone-rest.md`). It is in the mode's
    /// speech bank, so it loads beside [`Self::Ready`] and [`Self::Go`]. Raised
    /// by `Race::tick_destroyed_craft`.
    ContElim,
    /// The HD-lineage magstrip hum, started once as a craft comes onto a strip.
    ///
    /// `Ship_StartMagstripSound` (`0x012f8c70`, Omega) starts `_magstrip01` -
    /// `~magstrip01` in HD's `shiphd.bnk`, under a `### Magstrip` label - on a
    /// group named `MagStrip_Player` (the local ship) or `MagStrip_NPC`. The
    /// per-tick update `FUN_01762e80` raises it behind an arming flag that is
    /// `1` at construction: over the strip and armed, start and disarm; on the
    /// falling edge, stop and re-arm. Shape 80, arguments 55 -
    /// `docs/ghidra/functions/ps4-omega-eu/ships-effects.md`, "2026-10-05,
    /// magstrip-omega-law lane". Raised by `Race::advance_magstrip_wake`, on a
    /// title that builds the class only.
    ///
    /// **Held** (the cue is the start; [`Self::MagstripStop`] is its end), so it
    /// never reaches the one-shot path. A waveform that did not loop would play
    /// once, and the arming flag keeps it from restarting until the craft has
    /// left the strip.
    ///
    /// **Chosen, not measured**: the sound group has no mixer counterpart, so
    /// the player's craft and a rival's differ in nothing but the slot; and the
    /// bank's `~magstrip01` is a 35-waveform tree this reader flattens, so the
    /// voice is drawn until one loops - see `sfx::magstrip`.
    Magstrip,
    /// The falling edge of [`Self::Magstrip`]: `Ship_StopMagstripSound`
    /// (`0x01312770`) sets the stop bit on the instance and on every voice of
    /// the group. An action, not a sound: it loads nothing and is not in
    /// [`Self::ALL`].
    MagstripStop,
}

mod tables;
