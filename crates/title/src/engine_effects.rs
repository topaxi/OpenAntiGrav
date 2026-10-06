//! The names of the `Data\Psys` effects a race plays - the engine's own - each
//! with the reading that says when the original plays it. **Mostly Pulse's, not
//! all of it:** eight names here are HD's, 2048's or the PS2's and no Pulse PSP
//! disc authors them (`WO_PLASMA_LIGHTNING_*`, `WO_TRAIL_HITSHIP*`,
//! `WO_LEACHBEAM_ABSORB`, `WO_MAGSTRIP_*`, and `WO_SHIP_ENGINEFLARE` on the PSP).
//! Pulse's own table leaves them out - `docs/formats/pulse-absent-effects.md`.
//!
//! These are *names of things on the disc* and nothing here is code. The
//! table that gives each a [`Trigger`](crate::Trigger) is
//! [`Effects::engine`](crate::Effects::engine); the machinery that loads and
//! fires them is `oag-raceplay`'s `effects` module, which indexes by
//! `Trigger` so a typo is a compile error. Which of them are wired at all is
//! asserted against the disc itself by
//! `crates/game/tests/psys_inventory_ground_truth.rs`.
//!
//! Moved here from `oag-raceplay` (2026-10-06) so a title's table can name
//! them; the evidence on each constant came with it.

/// The effect the original attaches to every rocket at launch.
///
/// **Recovered, confidence 72.** `Rocket_Init` (`0x0885cdb8`) spawns it through
/// `Psys_Spawn_q` with the tag `ROFL`; the string is at `0x08a7c100`. Two
/// emitters, both `oag_pob::flags::LOOPING`, so it runs for as long
/// as the rocket does rather than for its authored 100 ticks - see
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
///
/// **There is no separate trail effect.** The Shuriken authors a `_HEAD` and a
/// `_TRAIL`; the rocket has only this, so the one emitter draws both the glow
/// at the nose and the streak behind it.
pub const ROCKET_FLARE_EFFECT: &str = "WO_ROCKET_FLARE";

/// The effect the original attaches to every missile at launch.
///
/// **Recovered, confidence 90** (the same reading that settles bit `0x40` as
/// the Missile - see `docs/ghidra/functions/psp-pulse-usa/missile.md`).
/// `Missile_Init` (`0x0885a160`) plays this **at two anchors**, alongside the
/// two `Trail_InitPreset` calls that are the missile's own twin-trail
/// signature.
///
/// **Both anchors are now ridden.** The "two anchors" are not fixed hull
/// locators on a missile model this project never located - they are two
/// points `Missile_Update` computes every tick, orbiting the missile's own
/// flight line at a constant rate. Nothing about them needed a `.vex` or a
/// locator: `weapons::missile_flare_anchors` derives both from the
/// projectile's own tracked position and velocity, at confidence 80. See
/// that function's doc comment for the full formula and what is still
/// unsettled about it, and `Race::advance_projectile_flares` for how the
/// two instances are attached and followed.
pub const MISSILE_FLARE_EFFECT: &str = "WO_MISSILE_HEAD";

/// The effect the original attaches to every plasma bolt at launch.
///
/// **Recovered, confidence 90.** `Plasma_Init` (`0x0885bd18`) spawns it with
/// the fourcc `PLHE` - the same slot `Missile_Init` puts `MIEX` in - and the
/// name string is at `0x08a7c0c0`, read directly out of `.rodata` rather than
/// inferred from a plausible name. It sits one entry away from the `PLASMA`
/// cue name at `0x08a7c0ac`, which the same constructor plays.
///
/// **One instance, and it rides the craft before it rides the bolt.**
/// `Plasma_Init` makes a single spawn call, unlike the Missile's pair - so
/// nothing here needs the orbiting second anchor
/// `weapons::missile_flare_anchors` derives. It is spawned **parented to the
/// firing craft's own weapon node** (`Psys_Spawn_q(.., 0, 0x10, node)`), and
/// `Plasma_Launch` (`0x0885bf84`) re-parents it to the bolt at release. That
/// is the charge-up glow: for the wind-up second the instance sits on the
/// nose and `Plasma_UpdateCharge` (`0x0885c170`) scales it by the charge
/// fraction.
///
/// **This engine gets the riding half of that for free, and, as of
/// 2026-09-16, the scale ramp too.** `Race::advance_projectile_flares`
/// attaches this to every live projectile whose kind maps here and follows
/// its position each tick; a charging bolt *is* a live projectile, reseated
/// on the craft's nose every tick by `Projectiles::advance`'s charging
/// branch, so the glow rides the craft through the wind-up and the bolt
/// after release - which is what re-parenting does in the original, arrived
/// at from the other end. `weapons::visuals::plasma_flare_scale` now ports
/// `Plasma_UpdateCharge`'s `(1.0 - remaining) * 0.75` scale, called through
/// `psys::Stage::rescale` every tick alongside the `follow`, so the glow
/// does grow as the shot charges - see that function's own doc comment for
/// the fraction's exact shape, for the further `* 0.5` on `craft+0x6d` - the
/// internal/cockpit camera flag, read 2026-09-16 at 82 and ported as the
/// function's `cockpit` argument - and for why a released bolt's
/// flare freezes at `0.75` (the wind-up's own maximum) rather than resetting
/// to `1.0`: `Plasma_Launch`'s recovered body writes no severity field, so
/// nothing in it changes what `Plasma_UpdateCharge` last wrote. Verified by
/// screenshot at `data/images/pulse-psp-eu.chd`: the glow visibly grows over
/// the second half of the 1 s wind-up and holds there once the bolt flies,
/// with no pop at release, though the first few ticks after the press are
/// hard to tell apart from the resting state
/// by eye.
///
/// Corrected 2026-09-09; the earlier "one instance, at the bolt's own
/// position" reading was written before `Plasma_Init`'s `+0x4c`/`+0x50`
/// charge fields were read.
pub const PLASMA_FLARE_EFFECT: &str = "WO_PLASMA_HEAD";

/// The explosion a plasma bolt plays when it goes off.
///
/// **Recovered, confidence 88, direct instruction-level read, 2026-09-09.**
/// The Plasma pool walker `Plasmas_Update` (`0x0886b490`) runs a teardown pass
/// over every entity whose destroy bit (`+0x3c & 4`) is set - raised by
/// `Plasma_Update`'s wall branch and by the walker's own hard
/// `oag_weapons::projectile::plasma::MAX_FLIGHT_SECONDS` reap - and that
/// pass calls `Plasma_SpawnDetonation` (`0x0886ac88`) with the bolt's own
/// position (`entity+0xa0`). That function allocates a `0x170`-byte blast
/// object and constructs it with `PlasmaBlast_Construct` (`0x0885fd90`),
/// whose only `Psys_Spawn_q` call spawns **this** file with the fourcc
/// `0x4c464c50` = `PLFL`; the name string at `0x08a7c22c` was read straight
/// out of `.rodata` rather than inferred, the `Mine_SpawnExplosion` standard.
/// The same teardown stops the `~PLASMATVL` travel loop and plays a
/// `PLASMAHITWALL` cue (`0x08a7c99b`).
///
/// **One file for every ending**, as the Missile's is: the one teardown pass
/// runs for a wall hit and for a timed-out bolt alike, so there is no
/// track/craft split to mirror the Rocket's.
///
/// **The blast's three models are recovered and are not drawn here.**
/// `PlasmaBlast_Construct` also loads `Data\Weapons\pulse_plasma_halo1.vex`,
/// `Data\Weapons\pulse_plasma_hemisphere1.vex` and
/// `Data\Weapons\pulse_plasma_hemisphere2.vex` (`0x08a7c1b0`, `0x08a7c200`,
/// `0x08a7c1d4`), orients the whole thing to the track surface through
/// `AiTrack_LocatePosition` and animates three ramps over them. This engine
/// plays the particle system and **draws none of the three models**, which is
/// an honest partial rather than a substitute: nothing is invented in their
/// place. See `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
pub const PLASMA_BLAST_EFFECT: &str = "WO_PLASMA_FLASH";

/// The explosion Wipeout HD/Fury plays when a Plasma bolt goes off - HD's
/// own file, not [`PLASMA_BLAST_EFFECT`], which is Pulse's.
///
/// **Recovered, confidence 88.** `WeaponExplosions_Start` (`0x00127cd0`)
/// spawns fourcc `'PLED'` at the detonation point -
/// `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`. The `.pob`'s own
/// internal name field was read directly, 2026-09-17: `wo_plasma_lightning_expand.pob`
/// on `DATA02` names itself `WO_PLASMA_LIGHTNING_EXPAND`, byte for byte.
pub const PLASMA_LIGHTNING_EXPAND_EFFECT: &str = "WO_PLASMA_LIGHTNING_EXPAND";

/// What `WeaponExplosions_Collapse` plays at `blast_models::HD_BLAST_HIDE_AT_SECONDS`,
/// the moment it hides all three of a Plasma blast's own models.
///
/// **Recovered, confidence 85** (`WeaponExplosions_Collapse`, `0x00127770`,
/// spawns fourcc `'PLCE'`) - see plasma.md. Internal name independently
/// confirmed 2026-09-17 the same way [`PLASMA_LIGHTNING_EXPAND_EFFECT`]'s
/// was: `wo_plasma_lightning_collapse.pob` names itself
/// `WO_PLASMA_LIGHTNING_COLLAPSE`. Played from
/// `Race::advance_plasma_blast_models`, not from `Race::ignite_blast` -
/// it fires 1.3 s after detonation, not at it.
pub const PLASMA_LIGHTNING_COLLAPSE_EFFECT: &str = "WO_PLASMA_LIGHTNING_COLLAPSE";

/// The effect the original attaches to every blade at launch.
///
/// **Recovered, confidence 88.** `Shuriken_Init` (`0x08877280`) spawns it with
/// the fourcc `SHUH` at the entity's `+0x60` anchor, name string
/// `0x08a7cda8`, read directly out of `.rodata`.
///
/// **The blade's second effect is not wired.** `WO_SHURIKEN_TRAIL` (`SHUT`,
/// `0x08a7cd94`) is attached at a *second* anchor whose basis the constructor
/// rotates by `-pi/2` about the blade, and following it needs the per-tick
/// orientation `Shuriken_Update` rebuilds - which this engine does not track,
/// because a `Projectile` carries a position and a velocity and no roll. It is
/// listed as untriggered rather than approximated with the head's own file.
/// See `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
pub const SHURIKEN_FLARE_EFFECT: &str = "WO_SHURIKEN_HEAD";

/// What a blade plays on every wall it glances off.
///
/// **Recovered, confidence 88.** `Shuriken_Bounce` (`0x088778ac`) spawns it
/// with the fourcc `SHBO`, name string `0x08a7cdbc`, alongside a
/// `SHURIKENHIT` cue - on the branch of `Shuriken_Update` that ends a rocket.
///
/// **A burst, not a riding instance**, exactly as [`MISSILE_BOUNCE_EFFECT`] is,
/// and played through the same generalised gate: a bounce is a moment.
pub const SHURIKEN_BOUNCE_EFFECT: &str = "WO_SHURIKEN_BOUNCE";

/// What a blade plays where it ends, whether its fuse ran out or a craft took it.
///
/// **Recovered, confidence 85, 2026-09-30.** `ShurikenPool_Update`
/// (`0x0886ff38`) raises the destroy bit at `fuse < age` (its `+0x174` against
/// the blade's `+0x48` clock) and its teardown, `FUN_08870c78`, spawns
/// `WO_SHURIKEN_EXPIRE` (fourcc `SHEX`) at the blade's position with an
/// identity basis, starts `ScreenFlash_Start(0)` there, and plays the
/// `SHURIKENEXPL` cue. It calls nothing that spends damage. See
/// `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
pub const SHURIKEN_EXPIRE_EFFECT: &str = "WO_SHURIKEN_EXPIRE";

/// The explosion a rocket that hits **track geometry** plays.
///
/// **Recovered, confidence 72.** Both of `Rocket_Update`'s (`0x0885d2a8`)
/// detonating branches spawn it, tag `ROD2`, string `0x08a7c110`. Four
/// emitters: a root glow, `fat_streaks`, a `SMOKERING` and `Fire_Emitter`.
pub const TRACK_BLAST_EFFECT: &str = "WO_ROCKET_EXPLO_TRACK";

/// The explosion a rocket that hits a **craft** plays.
///
/// **Recovered, confidence 72.** `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`),
/// reached only from the craft-hit path `Rocket_HitCraft` (`0x0886ebdc`), tag
/// `ROEX`, string `0x08a7ca74`. Seven emitters, the largest tree on the disc:
/// a root that hangs a `SMOKEMUSHROOM` off every particle, 32 pieces of
/// `DEBRIS` a tick, a `SMOKERING`, a `GLOW`, and a second per-particle pair of
/// `FIREMUSHROOM` glows.
///
/// That the two explosions are separately authored is confirmed rather than
/// inferred from the names, which is why this engine plays two files rather
/// than one file at two sizes.
pub const CRAFT_BLAST_EFFECT: &str = "WO_ROCKET_EXPLO";

/// The explosion a missile plays, whatever it hits and however it ends.
///
/// **Recovered, confidence 88.** `Missile_SpawnExplosion` (`0x08868d50`,
/// fourcc `MIEX`) is the pool teardown's only particle-spawning call, reached
/// from a craft hit, from the fifth wall bounce giving up, and from the
/// `SELF_DETONATE_SECONDS` timeout alike - see
/// `docs/ghidra/functions/psp-pulse-usa/missile.md#the-blast-and-what-does-not-reach-it`.
///
/// **One file for every ending, unlike the Rocket's track/craft split.** The
/// Missile authors no second explosion `.pob` the way the Rocket authors
/// `WO_ROCKET_EXPLO_TRACK` beside `WO_ROCKET_EXPLO` - `docs/formats/pob.md`'s
/// 35-name list has exactly one `WO_MISSILE_EXPLO`. So `Race::blast_for`
/// does not drop the struck craft's own position under it the way
/// [`CRAFT_BLAST_DROP`] does for the Rocket: no equivalent offset has been
/// read for the Missile, and inventing one would be exactly the kind of
/// plausible-looking number `CLAUDE.md` forbids. It plays at the impact point.
pub const MISSILE_EXPLO_EFFECT: &str = "WO_MISSILE_EXPLO";

/// What a missile plays on every wall it glances off, before it finally
/// detonates.
///
/// **Recovered, confidence 92** (the same reading as the flight model's
/// 12.0 probe length). `Missile_Update`'s wall branch mirrors the velocity,
/// pushes off the surface and "fires `WO_MISSILE_BOUNCE` with the
/// `MISSILEEXPWALL` cue" up to `oag_weapons::projectile::missile::MAX_BOUNCES`
/// times before the fifth attempt gives up and reaches
/// [`MISSILE_EXPLO_EFFECT`] instead - see
/// `docs/ghidra/functions/psp-pulse-usa/missile.md#the-flight-model-the-rockets-with-one-literal-changed`.
///
/// **A burst, not a riding instance** - `Race::ignite_missile_bounces`
/// plays it with `psys::Stage::play`, the same one-shot call
/// `Race::ignite_blast` uses, because a bounce is a moment rather than
/// something to follow.
pub const MISSILE_BOUNCE_EFFECT: &str = "WO_MISSILE_BOUNCE";

/// The explosion a mine plays, whenever and however it detonates.
///
/// **Recovered, confidence 90, direct instruction-level read.** `Mine_SpawnExplosion`
/// (`0x08867f1c`) is called from `MinePool_Update`'s uniform teardown pass for
/// every entity the fuse timeout **or** `Weapon_PostBlastImpulse_q`'s own
/// trigger_radius sweep (`Mine_SweepCraftTrigger`) marked for destruction - so a mine
/// that runs out of time and a mine a craft walks into play the same file.
/// It calls the already-named `Psys_Spawn_q` with the fourcc tag `MIEX` - a
/// generic "this is an explosion" instance tag shared with
/// [`MISSILE_EXPLO_EFFECT`]'s own spawner, not a per-weapon label - and a
/// string argument confirmed by a direct memory read to be `"WO_MINE_EXPLO"`.
/// See `docs/ghidra/functions/psp-pulse-usa/mine.md#mine_spawnexplosion-plays-wo_mine_explo`.
///
/// **The Bomb is not this.** Its own teardown, `Bomb_Detonate`, is a
/// distinct function reached from a distinct pool cursor
/// (`+0xc4`/cap 32 against the Mine's `+0x164`/`+0x64`) and it plays its own
/// three-piece detonation, not this file - see [`BOMB_SMOKERING_EFFECT`] and
/// `Race::blast_for`.
pub const MINE_EXPLO_EFFECT: &str = "WO_MINE_EXPLO";

/// The smoke ring one third of the Bomb's own detonation plays -
/// `BombBlast_Construct` (`0x08872078`, confidence 90) spawns it by name at
/// the blast's own basis, alongside two `.vex` models
/// (`bomb_blast::BombBlastModels`) this engine's generic
/// `oag_fx::psys::Stage` cannot carry. See
/// `docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-15-the-bombs-teardown-read---its-own-blast-not-the-mines`
/// for the fourcc tag (`'BOSM'`) and the read this plays back.
pub const BOMB_SMOKERING_EFFECT: &str = "WO_BOMB_SMOKERING";

/// How far below a struck craft's centre its blast is drawn, in world units.
///
/// **Recovered, confidence 78.** `Rocket_HitCraft` (`0x0886ebdc`) builds the
/// explosion's position from the struck craft's own position with `y - 2.5`, not
/// from the rocket's impact point.
pub const CRAFT_BLAST_DROP: f32 = 2.5;

/// The engine flare the **PS2** port authors as a particle effect.
///
/// Two looping emitters, and there is no PSP counterpart - the PSP release
/// authors no `Data\Psys` engine flare at all, which is why
/// `oag_fx::exhaust` draws one procedurally from the `Engine Flare`
/// locator and the behaviour measured off the PSP. Where the source *does*
/// ship one, playing it beats approximating it, so a PS2-sourced race gets
/// the asset and the procedural flare quad steps aside - see
/// `Race::engine_flare_effect`.
pub const ENGINE_FLARE_EFFECT: &str = "WO_SHIP_ENGINEFLARE";

/// The Quake's own travelling wave - the disc's authored effect, not a
/// stand-in for one.
///
/// **Read directly out of `.rodata` at confidence 88**, not inferred from the
/// fourcc `'QUAK'` alongside it - `inspect_memory_content` at the name
/// pointer's address returns the ASCII bytes `WO_QUAKE\0` verbatim. See
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s "What
/// `Quake_Update` builds from those two points" section: this is spawned once
/// per wave instance, then re-positioned to the midpoint and re-scaled to the
/// track's own width every later frame - see
/// `Race::advance_quake_visual`.
pub const QUAKE_EFFECT: &str = "WO_QUAKE";

/// A Repulser's blast around the firer, from the fire.
///
/// **Recovered, confidence 84.** `Repulser_SpawnBlastEffect` (`0x088761d8`),
/// called from `Repulser_Init`, spawns this (`0x08a7cd18`, fourcc `REP3`) at the
/// entity's own matrix `+0x1a0`, which `Repulser_UpdateFieldModel` rebuilds from
/// the firer's node every tick; `Repulser_Reset` releases it. See
/// `docs/ghidra/functions/psp-pulse-usa/repulser.md` and
/// `Race::advance_repulser_visual`.
pub const REPULSER_BLAST_EFFECT: &str = "WO_REPULSER_BLAST";

/// One Repulser wave, riding its centre.
///
/// **Recovered, confidence 80.** `Repulser_SpawnWaves` (`0x08876300`) spawns
/// this (`0x08a7cd2c`) twice, fourccs `REP0`/`REP1`, at the two waves' own
/// matrices (`+0x60`/`+0xa0`), which `Repulser_AdvanceWave` moves every update;
/// `Repulser_Reset` releases both. A third on a junction's branch (`REP2`) is
/// not drawn, because the port's waves do not take branches. See
/// `Race::advance_repulser_visual`.
pub const REPULSER_EFFECT: &str = "WO_REPULSER";

/// What a connected LeachBeam draws on the craft it is draining.
///
/// **The disc's own, with a recovered trigger** - `LeachBeam_Advance`
/// (`0x08873fa0`) spawns it at the *target's* own scene node, inside the same
/// block that plays the `LEACHENERGY` cue, each time the beam's ribbon-scroll
/// cursor wraps. That is roughly once a second over the beam's authored
/// `active_time`; this engine attaches it once when the link connects and
/// follows the target with it instead, because the re-spawn cadence is a
/// function of the ribbon geometry this crate deliberately does not build.
/// **Chosen, not measured**, and it is the cadence only - the effect and its
/// anchor are both recovered. See
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
pub const LEACHBEAM_ENERGY_EFFECT: &str = "WO_LEACHBEAM_ENERGY";

/// The spark burst a Cannon round throws when it hits **track geometry**.
///
/// **Recovered, confidence 85.** `Cannon_UpdateRound` (`0x0886593c`, EU
/// `0x08865798`) raycasts each round against the track's own collision mesh
/// every tick (`FUN_0883198c`, shared with the Rocket's, Missile's,
/// Plasma's and Shuriken's own updates) and, only when that raycast's hit
/// type is `0` or `4`, spawns this file with `Psys_Spawn_q`, oriented to the
/// hit basis and positioned at the hit point - the name string sits at
/// `0x08a7c890` on USA, `0x08a7c0e0` on EU, both confirmed by a direct memory
/// read rather than inferred.
///
/// **A craft hit does not reach this call at all** - the struck hull throws
/// its own damage sparks instead, from `Ship_Damage` (`race::hit_sparks`). See
/// `oag_weapons::projectile::cannon`'s own module doc, "The wall hit spawns
/// a spark effect; the craft hit does not", for the separate cylinder-test
/// path that applies damage and a sound cue but never calls `Psys_Spawn_q`.
/// `Race::blast_for` mirrors that split: `Weapon::Cannon` only reaches this
/// effect when `struck` is `None`.
///
/// **A burst, not a riding instance** - played through `Race::ignite_blast`,
/// the same one-shot `psys::Stage::play` every other weapon's detonation
/// burst uses, because a Cannon round's impact is a moment, not something to
/// follow.
pub const CANNON_SPARKS_EFFECT: &str = "WO_CANNON_SPARKS";

/// What a craft *holding* a LeachBeam draws, before it fires anything.
///
/// **The disc's own, with a fully recovered trigger** - `FUN_0883f540`
/// (`0x0883f540`) spawns it on the holder's own node whenever the craft's
/// held-weapon id is `10` and its state is racing, and tears it down the moment
/// either stops being true. Nothing about the cadence is chosen here: it is up
/// exactly while the pickup is held.
pub const LEACHBEAM_CHARGING_EFFECT: &str = "WO_LEACHBEAM_CHARGING";

/// Wipeout HD's own drain-trip burst: what `LeachBall_Advance`'s inlined
/// spawn (and its out-of-line, uncalled twin `LeachBeam_SpawnAbsorbEffect`,
/// `0x00114a00`) fires every time the render-side accumulator
/// `oag_fx::beam::hd_ball` tracks wraps - once a drain trip.
///
/// **The disc's own, with a recovered trigger, confidence 82** - the fourcc
/// `0x4541424c` and the spawn call are both a direct decompile of
/// `LeachBall_Advance` (`0x00114c78`); read
/// `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`'s "2026-09-25" section.
/// **Not on the PSP or PS2 disc** - HD-only, absent from `PSP_WIRED` and
/// `PS2_WIRED` in `crates/game/tests/psys_inventory_ground_truth.rs`, the
/// same footing [`PLASMA_LIGHTNING_EXPAND_EFFECT`] already has.
///
/// **Played as a one-shot burst, not attached and followed.** The original
/// allocates a fresh instance at every wrap rather than reusing one, the same
/// shape `Race::ignite_blast`'s detonation bursts already take - chosen
/// over [`LEACHBEAM_ENERGY_EFFECT`]'s attach-and-follow because nothing reads
/// a "the same instance kept alive" requirement out of the decompile the way
/// that effect's own re-spawn-on-pulse behaviour did.
pub const LEACHBEAM_ABSORB_EFFECT: &str = "WO_LEACHBEAM_ABSORB";

/// What Wipeout HD's Cannon throws on the craft it hits: a one-shot spark
/// burst on the hull's `Ship Collision Fx` locator nearest the contact.
///
/// **Recovered, confidence 72.** `Cannon_ApplyCraftHit` (`0x0010f730`) calls
/// `Ship_DispatchCollisionFx` with `kind` hardcoded to `1`, and
/// `ShipCollisionFx_Trigger`'s `kind == 1` branch names this effect from TOC
/// slot `0x008b3f6c` -> string `0x007a1c68`. The branch spawns it unattached
/// (the attach flag of the spawn call is `0`, where the ship's own attached
/// wall spark passes `1`), so it is played with `psys::Stage::play`. HD-only:
/// Pulse's Cannon spawns nothing on a craft and the struck hull sparks from
/// `Ship_Damage` instead, see `race::hit_sparks`.
///
/// Not on the PSP or PS2 disc, so a Pulse race reports it as missing and
/// fires nothing - its anchors are only built for HD.
pub const WEAPON_SPARK_EFFECT: &str = "WO_SHIP_SPARK_DAMAGE_WEAPON";

/// The welding sparks Basilico (`01_Track`) and circuit five place on their
/// own scenery: three nodes on Basilico, six or seven on circuit five.
///
/// **Recovered, confidence 85, trigger confirmed live.** Not a code literal at
/// all: the circuit's `.vex` places a `ParticleSystem` node whose `Name`
/// attribute is this string, and `PsysNode_Init` (`0x089156a0`) spawns
/// it at load on the node's own matrix. PPSSPP caught exactly the three
/// Basilico loads. Played by `race::scenery_fx` - see
/// `docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md`. This
/// constant names it for `super::RACE_EFFECTS`'s loader; the placement comes
/// from the disc.
pub const BLUE_WELDER_EFFECT: &str = "WO_BLUE_WELDER";

/// The steam vents circuits five and seven place: two on five, eighteen on
/// seven. **A Pulse effect**, despite the name: no Pure circuit is needed to
/// explain it. Same placement mechanism and evidence as
/// [`BLUE_WELDER_EFFECT`]; Outpost 7's eighteen loads were caught live too.
pub const MODESTO_STEAM_EFFECT: &str = "WO_MODESTO_STEAM_A";

/// Fort Gale's rain, its `<Weather EnvPsys>`.
///
/// **Recovered, confidence 80, played by `race::scenery_fx::weather`.** The
/// trigger is `Weather_Construct` (`0x088f184c`) off the circuit's
/// `TrackStartup`; the name here is only for `super::RACE_EFFECTS`'s loader,
/// the circuit's own XML names the effect it plays. See
/// `docs/ghidra/functions/psp-pulse-usa/weather.md`.
pub const RAIN_EFFECT: &str = "WO_RAIN";

/// Fort Gale's raindrops on the glass, its `<Weather ScreenPsys>`; same footing as
/// [`RAIN_EFFECT`], played by `race::scenery_fx::lens`.
pub const RAIN_LENS_EFFECT: &str = "WO_RAIN_LENS";

/// Outpost 7's snow, its `<Weather EnvPsys>`; same footing as [`RAIN_EFFECT`].
pub const SNOW_EFFECT: &str = "WO_SNOW";

/// What a craft plays over a magstrip on 2048, outside a Zone.
///
/// **Recovered, confidence 75.** `FUN_812c847a` (the Vita ship's derived
/// constructor) plays it at `ship+0x7598` when `GameMode_IsHdLineage`
/// (`0x81000930`) is false and `FUN_810018d4` (the Zone-family test) is false;
/// a 2048 event's mode id is a CRC (`>= 0x17`), read live as `0x026886dc` for
/// a campaign Time Trial. Played by `race::magstrip_wake`.
pub const MAGSTRIP_SPARKS_EFFECT: &str = "WO_MAGSTRIP_SPARKS";

/// The Zone twin of [`MAGSTRIP_SPARKS_EFFECT`]. Which 2048 events count as a
/// Zone for `FUN_810018d4` is **chosen, not measured**: `Mode::Zone`.
pub const MAGSTRIP_ZONE_EFFECT: &str = "WO_MAGSTRIP_ZONE";

/// The effect `ShipCollisionFx_Trigger` spawns when a wall contact dealt
/// damage: a four-emitter tree - orange smoke puffs, a bright spark fountain,
/// white `bits` debris and lingering `_TRAIL` embers. Pulse's weapon hits
/// throw the same one on the struck hull. Equal to
/// `oag_fx::sparks::DAMAGE_EFFECT`, which a `oag-raceplay` test pins.
pub const COLLISION_SPARK_EFFECT: &str = "WO_SHIP_COLL_SPARK_DAMAGE";

/// What a landed LeachBeam drain throws on the struck hull instead of
/// [`COLLISION_SPARK_EFFECT`] (`craft+0x138 == 7`).
pub const LEACHBEAM_HIT_SPARK_EFFECT: &str = "WO_SHIP_SPARK_DAMAGE_LEACHBEAM";

/// The effect every absorb plays - `Data\Psys\WO_WEAPON_ABSORB.POB` on every
/// title that has one.
pub const ABSORB_EFFECT: &str = "WO_WEAPON_ABSORB";

/// The blast at each wreck node.
pub const FXNODE_EXPLO_EFFECT: &str = "WO_SHIP_FXNODE_EXPLO";

/// The sparks beside it.
pub const DEATH_SPARKS_EFFECT: &str = "WO_SHIP_DEATH_SPARKS";

/// The big blast, 1.5 s after the state 5 edge.
pub const EXPLOSION_EFFECT: &str = "WO_SHIP_EXPLOSION";

/// What Wipeout HD plays on a craft that flies through an engine trail.
///
/// `Trail_SpawnHitEffect` (`0x002e3858`) consumes a per-trail "hit a ship"
/// flag the `Trails` SPU job raises, and `Trail_HitShipEffect` (`0x002d9ec0`)
/// spawns this system parented to the nearest of ten hull attachment nodes on
/// the craft involved. `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`.
/// Equal to `oag_fx::exhaust::hd::TRAIL_HITSHIP_EFFECT`.
pub const TRAIL_HITSHIP_EFFECT: &str = "WO_TRAIL_HITSHIP";

/// The Fury-skinned variant of [`TRAIL_HITSHIP_EFFECT`], chosen by the same
/// byte that turns the ribbon red (`craft + 0x7d2c`, what
/// `Ship_SetFuryTrailFlag` writes), so the sparks are red exactly when the
/// ribbon is. Equal to `oag_fx::exhaust::hd::TRAIL_HITSHIP_RED_EFFECT`.
pub const TRAIL_HITSHIP_RED_EFFECT: &str = "WO_TRAIL_HITSHIP_RED";
