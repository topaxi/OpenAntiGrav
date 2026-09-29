//! [`BankName`] and [`Cue`]: which `.bnk` a cue lives in, and every cue this
//! port fires.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change - the day
//! the Plasma's own three cues (`Cue::Plasma`, `Cue::PlasmaTravel`,
//! `Cue::PlasmaHitWall`) pushed the parent file over the ratchet. `Placement`
//! stays in [`super`], beside [`super::place`] and [`super::CueEvent`], which
//! read it far more than this file does.

use oag_audio::Bus;

use super::{BankName, Placement};

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
    /// the one `Banks::pick` closes: `0x19` decodes which *randomly-chosen*
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
    /// gap: [`super::SfxVoices::plasma_travel`] is an array of
    /// [`oag_gameplay::projectile::MAX_PROJECTILES`] voice handles, read and
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
    /// (`Impact::struck.is_some()`, `crates/gameplay/src/projectile/flight.rs`'s
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
    /// `crates/gameplay/src/projectile/flight.rs`'s own Rocket-timeout branch
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
    /// `oag_gameplay::projectile::Impact::struck`, the same field
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
    /// calls it `_MISSILETVL`, trust the bank) is confirmed alongside
    /// [`Self::Missile`] above at the same confidence, but neither the
    /// emitter nor its radius is stated. **Chosen, not measured: placed at
    /// the bolt's own position, reusing [`Self::RocketTravel`]'s measured
    /// `600.0` radius** rather than [`oag_audio::Emitter::CRAFT_RADIUS`],
    /// on the strength of the two weapons' shared `<WEAPON>TVL` naming and
    /// shared launch shape - not a reading of `Missile_Init` itself.
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
    /// budget and finally detonates plays neither this nor any cue this
    /// port fires - see `Impact::blast`'s own doc comment on `blast: false`
    /// for that ending, and the by-catch `missile.md` records reading its
    /// own literal as `SHURIKENEXPL` there (confidence 60, likely a
    /// copy-paste bug in the original) - left unwired rather than wired
    /// under a name this port does not trust. Placed at the bolt's own
    /// position, the same chosen `600.0` radius as [`Self::MissileTravel`].
    ///
    /// **`MISSILEEXPSHIP` is deliberately not a variant here.** The bank
    /// binds it (2 waveforms), but no call site was found on `missile.md` -
    /// an effect whose trigger is not recovered stays unwired.
    MissileHitWall,
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
    /// `oag_gameplay::projectile::cannon`'s own module doc already mirrors
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
    /// [`oag_gameplay::projectile::quake::Wave::hit`]'s own rising edge,
    /// snapshotted before `Race::advance_quake` and compared after.
    /// **Placed on the struck craft**: the doc's own pseudocode names the
    /// argument `owner_craft_cue_slot`, but the surrounding block's
    /// shield-gate and every pending-damage field it touches are the
    /// *victim's* - the page's own naming is internally inconsistent, and
    /// this port sides with the victim rather than the shooter.
    QuakeHit,
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
    /// The LeachBeam's own body, held for as long as a **locked** beam
    /// instance exists - through its disconnect linger, not only while it
    /// is [`connected`](oag_gameplay::projectile::leach_beam::Beam::connected).
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
    /// [`oag_render::beam::Ribbon::advance`]'s own pulse-edge return, which
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
    /// A Shuriken glancing off a wall.
    ///
    /// `Shuriken_Bounce` (`0x088778ac`, confidence 88) plays `SHURIKENHIT` on
    /// every bounce, matching the already-built `WO_SHURIKEN_BOUNCE`
    /// visual's edge; see `crate::race::weapons::visuals::bounced_this_tick`,
    /// which already accepts `Weapon::Shuriken`. The call's own argument is
    /// `s->travel_voice` - the same handle [`Self::ShurikenTravel`] opened -
    /// which the page reads but does not itself resolve to an emitter
    /// identity. **Placed on the firing craft: chosen, not measured**, the
    /// same reasoning as [`Self::ShurikenTravel`] below.
    ShurikenHit,
    /// The Shuriken's own travel loop, from throw to whatever ends it.
    ///
    /// `Shuriken_Init` (`0x08877280`, confidence 88) is handed
    /// `craft->emitter` directly - the same emitter
    /// [`super::Placement::Craft`] already reads for `.COLLISIONS`,
    /// `ABSORB`, `~SHIELD` and `MINELAUNCH` - rather than allocating a
    /// bolt-owned one the way `Rocket_Init`/`Missile_Init` do. **What the
    /// page does not show is the `~SHURIKENTRAVEL` play call's own emitter
    /// argument** - only that no bolt-owned `SoundEmitter_Init` exists to
    /// supply one. So riding the firing craft rather than the blade is
    /// **chosen, not measured, and carries no confidence score** - the
    /// absence of a bolt-owned emitter is what rules the alternative out,
    /// not a read of this cue's own call site. A blade thrown by a craft
    /// that has since gone inactive is heard from nowhere either way, the
    /// same silent drop [`super::place`] already gives any craft-placed cue
    /// on an empty slot.
    ShurikenTravel,
    /// The voice that counts the start in: `"ready"`, then the three, two, one
    /// its own timeline speaks.
    ///
    /// `RaceMode_SetState` (`0x08827350`) plays it when it enters state 1, the
    /// state `RaceMode_UpdateIntro_q` (`0x08829e6c`) hands the race to at the
    /// end of its intro substates - `Sound_PlayNamedInSlot(..., "ready", 0x400,
    /// 0, 0, 0)`, the dry, no-emitter path [`Self::Disengaging`] takes. Measured
    /// live on Pulse (PSP), Time Trial, a single race and Eliminator: it starts
    /// **180 ticks before [`Self::Go`]** (see `crate::race::countdown` for the
    /// ticks), with no other cue between the two - the 3, 2, 1 are inside this
    /// one cue's own timeline, not separate starts.
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
}

impl Cue {
    /// The two start-of-race cues, loaded on their own because their bank is
    /// the mode's and not the title's. See [`Self::Ready`].
    pub const COUNTDOWN: [Self; 2] = [Self::Ready, Self::Go];

    /// Every cue this port fires, which is every one it knows how to load.
    pub const ALL: [Self; 33] = [
        Self::SpeedupPad,
        Self::Collision,
        Self::Absorb,
        Self::Engine,
        Self::Shield,
        Self::ShieldActive,
        Self::Autopilot,
        Self::Engaging,
        Self::Disengaging,
        Self::Blowup,
        Self::LockOn,
        Self::MineLaunch,
        Self::Plasma,
        Self::PlasmaTravel,
        Self::PlasmaHitWall,
        Self::PlasmaHitShip,
        Self::Rocket,
        Self::RocketTravel,
        Self::RocketHitWall,
        Self::RocketHitShip,
        Self::Missile,
        Self::MissileTravel,
        Self::MissileHitWall,
        Self::Cannon,
        Self::CannonHitWall,
        Self::CannonHitShip,
        Self::QuakeLaunch,
        Self::QuakeHit,
        Self::Leach,
        Self::LeachAttach,
        Self::LeachEnergy,
        Self::ShurikenHit,
        Self::ShurikenTravel,
    ];

    /// The bank the cue is looked up in.
    #[must_use]
    pub fn bank(self) -> BankName {
        match self {
            Self::SpeedupPad | Self::Blowup | Self::LockOn | Self::Autopilot => BankName::Hud,
            Self::Collision | Self::Engine => BankName::Ship,
            Self::Absorb
            | Self::Shield
            | Self::MineLaunch
            | Self::Plasma
            | Self::PlasmaTravel
            | Self::PlasmaHitWall
            | Self::PlasmaHitShip
            | Self::Rocket
            | Self::RocketTravel
            | Self::RocketHitWall
            | Self::RocketHitShip
            | Self::Missile
            | Self::MissileTravel
            | Self::MissileHitWall
            | Self::Cannon
            | Self::CannonHitWall
            | Self::CannonHitShip
            | Self::QuakeLaunch
            | Self::QuakeHit
            | Self::Leach
            | Self::LeachAttach
            | Self::LeachEnergy
            | Self::ShurikenHit
            | Self::ShurikenTravel => BankName::Weapons,
            Self::ShieldActive | Self::Engaging | Self::Disengaging | Self::Ready | Self::Go => {
                BankName::Speech
            }
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
    /// [ADR-0027](../../../../../docs/architecture/adr/0027-three-mix-buses.md).
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
    /// `oag_formats::sblk::child`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::SpeedupPad => "SPEEDUPPAD",
            Self::Collision => ".COLLISIONS",
            Self::Absorb => "ABSORB",
            Self::Engine => "~ENGINE",
            Self::Shield => "~SHIELD",
            Self::ShieldActive => "shieldactive",
            Self::Autopilot => "~AUTOPILOT",
            Self::Engaging => "autopilot_eng",
            Self::Disengaging => "disengaging",
            Self::Ready => "ready",
            Self::Go => "go",
            Self::Blowup => "~BLOWUP",
            Self::LockOn => "~ROCKLOCK",
            Self::MineLaunch => "MINELAUNCH",
            Self::Plasma => "PLASMA",
            Self::PlasmaTravel => "~PLASMATVL",
            Self::PlasmaHitWall => "PLASMAHITWALL",
            Self::PlasmaHitShip => "PLASMAHITSHIP",
            Self::Rocket => "ROCKET",
            Self::RocketTravel => "~ROCKETTVL",
            Self::RocketHitWall => "ROCKEXPLWALL",
            Self::RocketHitShip => "ROCKEXPLSHIP",
            Self::Missile => "MISSILE",
            Self::MissileTravel => "~MISSILETVL",
            Self::MissileHitWall => "MISSILEEXPWALL",
            Self::Cannon => "CANNON",
            Self::CannonHitWall => "CANNONEXPLWALL",
            Self::CannonHitShip => "CANNONEXPLSHIP",
            Self::QuakeLaunch => "QUAKELAUNCH",
            Self::QuakeHit => "QUAKEHIT",
            Self::Leach => "LEACH",
            Self::LeachAttach => "~LEACHATTACH",
            Self::LeachEnergy => "LEACHENERGY",
            Self::ShurikenHit => "SHURIKENHIT",
            Self::ShurikenTravel => "~SHURIKENTRAVEL",
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
    /// [`Self::PlasmaTravel`], [`Self::RocketTravel`], [`Self::MissileTravel`]
    /// and [`Self::ShurikenTravel`] all join this list held **per projectile
    /// slot** rather than per grid slot or as a single race-wide handle - see
    /// [`super::travel::TravelVoices`]. [`Self::LeachAttach`] is held once for
    /// the whole race, on [`super::SfxVoices::leach_attach`], since the
    /// LeachBeam is a world-wide single instance rather than a projectile
    /// slot. [`Self::Autopilot`] joins them the same way `Blowup` already
    /// does: a level-driven handle (`ships[0].autopilot_timer > 0.0`), not a
    /// per-projectile one. None of the six is ever pushed through the cue
    /// queue, so this only guards against a stray push being mis-played.
    #[must_use]
    pub fn held(self) -> bool {
        matches!(
            self,
            Self::Engine
                | Self::Shield
                | Self::Blowup
                | Self::Autopilot
                | Self::PlasmaTravel
                | Self::RocketTravel
                | Self::MissileTravel
                | Self::LeachAttach
                | Self::ShurikenTravel
        )
    }

    /// How far this cue's voice can be heard, for a cue that carries its own
    /// emitter rather than riding a craft's.
    ///
    /// Only read by [`super::place`]'s [`Placement::Point`] arm and by the
    /// bespoke held-voice trackers ([`super::travel::TravelVoices`],
    /// [`super::SfxVoices::leach_attach`]) - every [`Placement::Craft`] cue
    /// keeps [`oag_audio::Emitter::CRAFT_RADIUS`] unconditionally, by
    /// construction, so this only matters for the handful of cues with a
    /// bolt- or beam-owned emitter.
    ///
    /// Defaults to [`oag_audio::Emitter::CRAFT_RADIUS`] - `SoundEmitter_Init`'s
    /// own default, which is what every cue here keeps unless its own call
    /// site is read overriding it. **Measured at `600.0` for the Rocket**
    /// (`Rocket_Init` writes `+0x38` directly) and **for the LeachBeam's own
    /// body** (`LeachBeam_InitLocked`'s `+0x4c` emitter). **Chosen at `600.0`
    /// for the Missile**, reusing the Rocket's measured figure rather than
    /// [`oag_audio::Emitter::CRAFT_RADIUS`] - see [`Self::MissileTravel`]'s
    /// own doc comment for why. The Cannon's own hit cues keep the default:
    /// no override is stated for them. **Measured at `300.0` for
    /// [`Self::LeachEnergy`]** - half the beam body's own `600.0`, its own
    /// separate call site.
    #[must_use]
    pub fn radius(self) -> f32 {
        match self {
            Self::RocketTravel
            | Self::RocketHitWall
            | Self::RocketHitShip
            | Self::MissileTravel
            | Self::MissileHitWall
            | Self::LeachAttach => 600.0,
            // Measured, not reused from `LeachAttach`'s `600.0` -
            // `LeachBeam_Advance`'s pulse block writes its own emitter at a
            // `300.0` falloff, half the beam body's own; see
            // `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
            // "Assets and cues - checked, not assumed".
            Self::LeachEnergy => 300.0,
            _ => oag_audio::Emitter::CRAFT_RADIUS,
        }
    }

    /// Which emitter the original plays this cue on.
    ///
    /// Read off each call site's first argument to `Sound_Play`, which is the
    /// emitter record and nothing else - see
    /// [`positional-audio.md`](../../../../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md).
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
            // `RaceMode_SetState` and `RaceMode_UpdateCountdown` both call
            // `Sound_PlayNamedInSlot` with no emitter and `0x400`: a voice in
            // the player's ear, the same shape as `Disengaging` above.
            Self::Ready | Self::Go => Placement::Unplaced,
            // Read, not assumed: case 4 hands it to the path that takes no
            // emitter and a volume of `0x400`.
            Self::Blowup => Placement::Unplaced,
            // Both halves of `Ship_FireHeldWeapon`'s case 6 go through
            // `FUN_0883e9b0` too - the same dry, no-emitter path `Blowup` and
            // `Disengaging` take - see each variant's own doc comment.
            Self::Autopilot | Self::Engaging => Placement::Unplaced,
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
            // All three ride the bolt's own emitter, which `Plasma_Init`
            // constructs pointed at the bolt's own matrix rather than any
            // craft's - see each variant's own doc comment. None is routed
            // through this function's `craft` slice: [`Self::PlasmaTravel`]
            // is a bespoke per-tick tracker keyed by projectile slot, and
            // [`Self::PlasmaHitWall`]/[`Self::PlasmaHitShip`] each carry their
            // own point on the [`CueEvent`] rather than a grid slot.
            Self::PlasmaTravel | Self::PlasmaHitWall | Self::PlasmaHitShip => Placement::Point,
            // Same shape as the Plasma's three, on the Rocket's own
            // `600.0`-radius emitter - see each variant's own doc comment and
            // [`Self::radius`].
            Self::RocketTravel | Self::RocketHitWall | Self::RocketHitShip => Placement::Point,
            // `Ship_FireHeldWeapon`'s own `Sound_Play(1.0, *(param_1+0x50),
            // weapons.bnk, 0, "ROCKET", 0)` - a plain positional call taking
            // an explicit emitter argument, the same shape [`Self::Shield`]
            // reads above, not the dry path - see this variant's own doc
            // comment.
            Self::Rocket => Placement::Craft,
            // Chosen, not measured - see each variant's own doc comment.
            Self::Missile | Self::Cannon => Placement::Craft,
            Self::MissileTravel | Self::MissileHitWall => Placement::Point,
            Self::CannonHitWall | Self::CannonHitShip => Placement::Point,
            // Same `Ship_FireHeldWeapon` switch, same positional call shape
            // as `Rocket` - see that variant's own doc comment.
            Self::QuakeLaunch => Placement::Craft,
            // Settled by cross-reading the surrounding block's shield gate
            // and pending-damage fields, all the struck craft's own - see
            // this variant's own doc comment.
            Self::QuakeHit => Placement::Craft,
            // `LeachBeam_InitLocked` plays `LEACH` on the shooter's own
            // emitter - see this variant's own doc comment.
            Self::Leach => Placement::Craft,
            // A bespoke per-tick tracker on the beam's own `600.0`-radius
            // emitter, the same shape [`Self::PlasmaTravel`] takes for a
            // projectile slot - see [`super::SfxVoices::leach_attach`] and
            // [`Self::radius`].
            Self::LeachAttach => Placement::Point,
            // Pushed with the ribbon's own `energy_point` as a
            // [`super::CueEvent::at_point`] - the pulse block's own re-spawn
            // target, not a craft's emitter - see this variant's own doc
            // comment and [`Self::radius`].
            Self::LeachEnergy => Placement::Point,
            // `Shuriken_Init` is handed `craft->emitter` directly, so both
            // ride the firing craft rather than the blade - see
            // [`Self::ShurikenTravel`]'s own doc comment.
            Self::ShurikenHit | Self::ShurikenTravel => Placement::Craft,
        }
    }
}
