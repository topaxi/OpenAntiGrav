//! The names of the `Data\Psys` effects a race plays, each with the reading that
//! says when the original plays it. **Mostly Pulse's, not all:** eight names are
//! HD's, 2048's or the PS2's and no Pulse PSP disc authors them
//! (`WO_PLASMA_LIGHTNING_*`, `WO_TRAIL_HITSHIP*`, `WO_LEACHBEAM_ABSORB`,
//! `WO_MAGSTRIP_*`, and `WO_SHIP_ENGINEFLARE` on the PSP); see
//! `docs/formats/pulse-absent-effects.md`.
//!
//! These are *names of things on the disc*. The table giving each a
//! [`Trigger`](crate::Trigger) is [`Effects::engine`](crate::Effects::engine);
//! `oag-raceplay`'s `effects` module loads and fires them, indexed by `Trigger`.
//! Which are wired is asserted against the disc by
//! `crates/raceplay/tests/psys_inventory_ground_truth.rs`.

/// The effect the original attaches to every rocket at launch.
///
/// **Recovered, confidence 72.** `Rocket_Init` (`0x0885cdb8`) spawns it through
/// `Psys_Spawn_q` with the tag `ROFL`; the string is at `0x08a7c100`. Two
/// emitters, both `oag_pob::flags::LOOPING`, so it runs as long as the rocket
/// (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`).
///
/// **No separate trail effect**: the Shuriken authors a `_HEAD` and a `_TRAIL`,
/// the rocket only this, so one emitter draws the nose glow and the streak.
pub const ROCKET_FLARE_EFFECT: &str = "WO_ROCKET_FLARE";

/// The effect the original attaches to every missile at launch.
///
/// **Recovered, confidence 90** (the reading that settles bit `0x40` as the
/// Missile, `docs/ghidra/functions/psp-pulse-usa/missile.md`). `Missile_Init`
/// (`0x0885a160`) plays this **at two anchors**, beside the two
/// `Trail_InitPreset` calls that are the missile's twin-trail signature.
///
/// Both anchors are points `Missile_Update` computes every tick, orbiting the
/// flight line at a constant rate, so no locator is needed:
/// `weapons::missile_flare_anchors` derives both from the projectile's tracked
/// position and velocity (confidence 80, formula and open points in its doc);
/// `Race::advance_projectile_flares` attaches and follows the two instances.
pub const MISSILE_FLARE_EFFECT: &str = "WO_MISSILE_HEAD";

/// The effect the original attaches to every plasma bolt at launch.
///
/// **Recovered, confidence 90.** `Plasma_Init` (`0x0885bd18`) spawns it with the
/// fourcc `PLHE` (the slot `Missile_Init` puts `MIEX` in); the name string at
/// `0x08a7c0c0` was read out of `.rodata`, one entry from the `PLASMA` cue name at
/// `0x08a7c0ac`.
///
/// **One instance, riding the craft before the bolt.** A single spawn call
/// (`Psys_Spawn_q(.., 0, 0x10, node)`) parented to the firing craft's weapon
/// node; `Plasma_Launch` (`0x0885bf84`) re-parents it to the bolt at release.
/// That is the charge-up glow: `Plasma_UpdateCharge` (`0x0885c170`) scales it by
/// the charge fraction during the wind-up second.
///
/// **This engine gets the riding and the scale ramp.**
/// `Race::advance_projectile_flares` attaches it to every live projectile of the
/// kind and follows it; a charging bolt is a live projectile reseated on the
/// nose by `Projectiles::advance`, which is the re-parenting arrived at from the
/// other end. `weapons::visuals::plasma_flare_scale` ports `Plasma_UpdateCharge`'s
/// `(1.0 - remaining) * 0.75`, applied through `psys::Stage::rescale` each tick
/// (its doc has the fraction's shape, the further `* 0.5` on `craft+0x6d`, the
/// internal/cockpit camera flag read at confidence 82, and why a released bolt's
/// flare freezes at `0.75`: `Plasma_Launch` writes no severity field). Checked
/// by screenshot on `data/images/pulse-psp-eu.chd`: the glow grows over the
/// second half of the wind-up and holds after release, no pop. Evidence:
/// `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
pub const PLASMA_FLARE_EFFECT: &str = "WO_PLASMA_HEAD";

/// The explosion a plasma bolt plays when it goes off.
///
/// **Recovered, confidence 88, direct instruction-level read.** The Plasma pool
/// walker `Plasmas_Update` (`0x0886b490`) runs a teardown pass over every entity
/// whose destroy bit (`+0x3c & 4`) is set, raised by `Plasma_Update`'s wall branch
/// and by the walker's hard `oag_weapons::projectile::plasma::MAX_FLIGHT_SECONDS`
/// reap. It calls `Plasma_SpawnDetonation` (`0x0886ac88`) at the bolt's position
/// (`entity+0xa0`), which builds a `0x170`-byte blast with `PlasmaBlast_Construct`
/// (`0x0885fd90`) whose only `Psys_Spawn_q` spawns **this** file with fourcc
/// `0x4c464c50` = `PLFL`; the name at `0x08a7c22c` was read out of `.rodata`. The
/// same teardown stops the `~PLASMATVL` loop and plays a `PLASMAHITWALL` cue
/// (`0x08a7c99b`).
///
/// **One file for every ending**, as the Missile's: the one pass runs for a wall
/// hit and a timed-out bolt alike.
///
/// **The blast's three models are recovered and not drawn here.**
/// `PlasmaBlast_Construct` also loads `Data\Weapons\pulse_plasma_halo1.vex`,
/// `pulse_plasma_hemisphere1.vex` and `pulse_plasma_hemisphere2.vex`
/// (`0x08a7c1b0`, `0x08a7c200`, `0x08a7c1d4`), orients them to the track surface
/// through `AiTrack_LocatePosition` and animates three ramps over them. This
/// engine plays the particle system only; nothing is drawn in the models' place
/// (`docs/ghidra/functions/psp-pulse-usa/plasma.md`).
pub const PLASMA_BLAST_EFFECT: &str = "WO_PLASMA_FLASH";

/// The explosion HD/Fury plays when a Plasma bolt goes off: HD's own file, not
/// [`PLASMA_BLAST_EFFECT`].
///
/// **Recovered, confidence 88.** `WeaponExplosions_Start` (`0x00127cd0`) spawns
/// fourcc `'PLED'` at the detonation point
/// (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`). The `.pob`'s internal name
/// field was read directly: `wo_plasma_lightning_expand.pob` on `DATA02` names
/// itself `WO_PLASMA_LIGHTNING_EXPAND`.
pub const PLASMA_LIGHTNING_EXPAND_EFFECT: &str = "WO_PLASMA_LIGHTNING_EXPAND";

/// What `WeaponExplosions_Collapse` plays at `blast_models::HD_BLAST_HIDE_AT_SECONDS`,
/// when it hides all three of a Plasma blast's models.
///
/// **Recovered, confidence 85** (`WeaponExplosions_Collapse`, `0x00127770`,
/// fourcc `'PLCE'`, plasma.md). Internal name confirmed as for
/// [`PLASMA_LIGHTNING_EXPAND_EFFECT`]: `wo_plasma_lightning_collapse.pob` names
/// itself `WO_PLASMA_LIGHTNING_COLLAPSE`. Played from
/// `Race::advance_plasma_blast_models`, 1.3 s after detonation, not from
/// `Race::ignite_blast`.
pub const PLASMA_LIGHTNING_COLLAPSE_EFFECT: &str = "WO_PLASMA_LIGHTNING_COLLAPSE";

/// The effect the original attaches to every blade at launch.
///
/// **Recovered, confidence 88.** `Shuriken_Init` (`0x08877280`) spawns it with
/// fourcc `SHUH` at the entity's `+0x60` anchor, name string `0x08a7cda8`.
///
/// Its frame is the blade's basis as `Shuriken_Update` rebuilds it each tick,
/// ridden by pointer, so the emitter's `+Y` is the blade's surface normal. The
/// second effect is [`SHURIKEN_TRAIL_EFFECT`].
pub const SHURIKEN_FLARE_EFFECT: &str = "WO_SHURIKEN_HEAD";

/// The effect the original attaches to every blade at launch, beside the head.
///
/// **Recovered, confidence 88.** `Shuriken_Init` (`0x08877280`) spawns it with
/// fourcc `SHUT`, name string `0x08a7cd94`, before the head, on the `+0xb0`
/// frame, flag 1 (a pointer, read live). Each `Shuriken_Update` copies the
/// blade's basis there turned `-pi/2` about its row 0 (the Rocket's flare frame),
/// so the emitter's `+Y` is the velocity
/// (`docs/ghidra/functions/psp-pulse-usa/shuriken.md`).
pub const SHURIKEN_TRAIL_EFFECT: &str = "WO_SHURIKEN_TRAIL";

/// What a blade plays on every wall it glances off.
///
/// **Recovered, confidence 88.** `Shuriken_Bounce` (`0x088778ac`) spawns it with
/// fourcc `SHBO`, name string `0x08a7cdbc`, beside a `SHURIKENHIT` cue, on the
/// branch of `Shuriken_Update` that ends a rocket.
///
/// **A burst, not a riding instance**, like [`MISSILE_BOUNCE_EFFECT`]: a bounce
/// is a moment.
pub const SHURIKEN_BOUNCE_EFFECT: &str = "WO_SHURIKEN_BOUNCE";

/// What a blade plays where it ends, whether its fuse ran out or a craft took it.
///
/// **Recovered, confidence 85** (`docs/ghidra/functions/psp-pulse-usa/shuriken.md`,
/// "What ends a shuriken"). `ShurikenPool_Update` (`0x0886ff38`) raises the
/// destroy bit at `fuse < age` (its `+0x174` against the blade's `+0x48` clock);
/// its teardown `FUN_08870c78` spawns `WO_SHURIKEN_EXPIRE` (fourcc `SHEX`) at the
/// blade's position with an identity basis, starts `ScreenFlash_Start(0)`, and
/// plays the `SHURIKENEXPL` cue. It spends no damage.
pub const SHURIKEN_EXPIRE_EFFECT: &str = "WO_SHURIKEN_EXPIRE";

/// The explosion a rocket that hits **track geometry** plays.
///
/// **Recovered, confidence 72.** Both detonating branches of `Rocket_Update`
/// (`0x0885d2a8`) spawn it, tag `ROD2`, string `0x08a7c110`. Four emitters: a
/// root glow, `fat_streaks`, a `SMOKERING` and `Fire_Emitter`.
pub const TRACK_BLAST_EFFECT: &str = "WO_ROCKET_EXPLO_TRACK";

/// The explosion a rocket that hits a **craft** plays.
///
/// **Recovered, confidence 72.** `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`),
/// reached only from `Rocket_HitCraft` (`0x0886ebdc`), tag `ROEX`, string
/// `0x08a7ca74`. Seven emitters, the largest tree on the disc: a root hanging a
/// `SMOKEMUSHROOM` off every particle, 32 pieces of `DEBRIS` a tick, a
/// `SMOKERING`, a `GLOW`, and a second per-particle pair of `FIREMUSHROOM` glows.
/// The two explosions are separately authored, so this engine plays two files.
pub const CRAFT_BLAST_EFFECT: &str = "WO_ROCKET_EXPLO";

/// The explosion a missile plays, whatever it hits and however it ends.
///
/// **Recovered, confidence 88.** `Missile_SpawnExplosion` (`0x08868d50`, fourcc
/// `MIEX`) is the pool teardown's only particle-spawning call, reached from a
/// craft hit, from the fifth wall bounce giving up, and from the
/// `SELF_DETONATE_SECONDS` timeout
/// (`docs/ghidra/functions/psp-pulse-usa/missile.md#the-blast-and-what-does-not-reach-it`).
///
/// **One file for every ending**, unlike the Rocket's track/craft split:
/// `docs/formats/pob.md`'s 35-name list has one `WO_MISSILE_EXPLO`. So
/// `Race::blast_for` does not drop the struck craft's position as
/// [`CRAFT_BLAST_DROP`] does for the Rocket: no such offset has been read for the
/// Missile and an invented one is what CLAUDE.md forbids. It plays at the impact.
pub const MISSILE_EXPLO_EFFECT: &str = "WO_MISSILE_EXPLO";

/// What a missile plays on every wall it glances off, before it finally detonates.
///
/// **Recovered, confidence 92** (the reading behind the flight model's 12.0 probe
/// length). `Missile_Update`'s wall branch mirrors the velocity, pushes off the
/// surface and fires `WO_MISSILE_BOUNCE` with the `MISSILEEXPWALL` cue up to
/// `oag_weapons::projectile::missile::MAX_BOUNCES` times before the fifth attempt
/// reaches [`MISSILE_EXPLO_EFFECT`]
/// (`docs/ghidra/functions/psp-pulse-usa/missile.md#the-flight-model-the-rockets-with-one-literal-changed`).
///
/// **A burst, not a riding instance**: `Race::ignite_missile_bounces` plays it
/// with `psys::Stage::play`, as `Race::ignite_blast` does.
pub const MISSILE_BOUNCE_EFFECT: &str = "WO_MISSILE_BOUNCE";

/// The explosion a mine plays, whenever and however it detonates.
///
/// **Recovered, confidence 90, direct instruction-level read.**
/// `Mine_SpawnExplosion` (`0x08867f1c`) is called from `MinePool_Update`'s uniform
/// teardown pass for every entity the fuse timeout **or**
/// `Weapon_PostBlastImpulse_q`'s trigger-radius sweep (`Mine_SweepCraftTrigger`)
/// marked, so a mine that runs out of time and one a craft walks into play the
/// same file. It calls `Psys_Spawn_q` with the fourcc `MIEX` (a generic
/// "explosion" tag shared with [`MISSILE_EXPLO_EFFECT`]'s spawner) and a string
/// confirmed by memory read to be `"WO_MINE_EXPLO"`
/// (`docs/ghidra/functions/psp-pulse-usa/mine.md#mine_spawnexplosion-plays-wo_mine_explo`).
///
/// **The Bomb is not this.** `Bomb_Detonate` is a distinct function on a distinct
/// pool cursor (`+0xc4`/cap 32 against the Mine's `+0x164`/`+0x64`) and plays its
/// own three-piece detonation ([`BOMB_SMOKERING_EFFECT`], `Race::blast_for`).
pub const MINE_EXPLO_EFFECT: &str = "WO_MINE_EXPLO";

/// The smoke ring one third of the Bomb's detonation plays: `BombBlast_Construct`
/// (`0x08872078`, confidence 90) spawns it by name at the blast's basis, beside
/// two `.vex` models (`bomb_blast::BombBlastModels`) the generic
/// `oag_fx::psys::Stage` cannot carry. The fourcc tag (`'BOSM'`) and the read are
/// in `docs/ghidra/functions/psp-pulse-usa/mine.md`, the bomb's teardown section.
pub const BOMB_SMOKERING_EFFECT: &str = "WO_BOMB_SMOKERING";

/// A laid Bomb's glow, a child of the Bomb's scene node for its whole life:
/// `Bomb_Init` (`0x088580d8`) on Pure spawns it with the fourcc `BOGL`
/// (`docs/ghidra/functions/psp-pure-usa/weapon-effects.md`, "Bomb", confidence 70).
/// Pure's alone: Pulse's `Bomb_Init` is not read for it.
pub const BOMB_GLOW_EFFECT: &str = "WO_BOMB_GLOW";

/// A Disruptor bolt in flight, attached to the projectile's matrix:
/// `Disruptor_Init` (`0x08859010`), fourcc `DIHE`, confidence 80. Pure's alone.
pub const DISRUPTOR_HEAD_EFFECT: &str = "WO_DISRUPTOR_HEAD";

/// A Disruptor bolt ending, at a hit or after its `10.0` s timeout:
/// `DisruptorExplo_Construct` (`0x0885a2c8`), fourcc `DIEX`, confidence 78.
/// Pure's alone.
pub const DISRUPTOR_EXPLO_EFFECT: &str = "WO_DISRUPTOR_EXPLO";

/// A Disruptor leaving the firing craft: `DisruptorPool_Fire` (`0x0884fc14`) spawns it
/// (fourcc `DILA`, mode 2, no matrix) as a child of the craft's `Ship Muzzle` node, and
/// only when the hull authors one (`*(craft+0xe0)`, `ShipMuzzle_Importer`'s node).
/// Pure's alone; `docs/ghidra/functions/psp-pure-usa/weapon-effects.md`, "Disruptor".
pub const DISRUPTOR_LAUNCH_EFFECT: &str = "WO_DISRUPTOR_LAUNCH";

/// The rays HD's Bomb blast spawns once, half a second after it starts:
/// `NormalBombBlast_Update` (`0x001503d8`) calls `Psys_Spawn_q` with the string at
/// `0x007855e8` and tag `'BORS'` at the blast's matrix, the first tick `age > 0.5`
/// (`0x150c04`..`0x150fc8`). HD alone: Pulse plays [`BOMB_SMOKERING_EFFECT`] and no
/// rays. `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, "2026-10-07".
pub const BOMB_RAYS_EFFECT: &str = "WO_BOMB_RAYS";

/// How far below a struck craft's centre its blast is drawn, in world units.
///
/// **Recovered, confidence 78.** `Rocket_HitCraft` (`0x0886ebdc`) builds the
/// explosion's position from the struck craft's with `y - 2.5`, not the impact point.
pub const CRAFT_BLAST_DROP: f32 = 2.5;

/// The engine flare particle effect the **PS2 disc** ships: two looping emitters,
/// no PSP counterpart. **No title plays it on Pulse:** the PS2 executable never
/// requests it (no path, name or hash) and a GS dump of the nozzle shows one flare
/// quad, the one `oag_fx::exhaust` draws from the `Engine Flare` locator
/// (`docs/ghidra/functions/ps2-pulse-eu/engine-trail.md`). A title whose table
/// names it plays it if its archive holds the asset (`Race::engine_flare_effect`).
pub const ENGINE_FLARE_EFFECT: &str = "WO_SHIP_ENGINEFLARE";

/// The Quake's own travelling wave: the disc's authored effect, not a stand-in.
///
/// **Read out of `.rodata` at confidence 88**, not inferred from the fourcc
/// `'QUAK'` beside it (`WO_QUAKE\0` verbatim at the name pointer's address).
/// Spawned once per wave instance, then re-positioned to the midpoint and
/// re-scaled to the track's width each frame (`Race::advance_quake_visual`;
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "What
/// `Quake_Update` builds from those two points").
pub const QUAKE_EFFECT: &str = "WO_QUAKE";

/// A Repulser's blast around the firer, from the fire.
///
/// **Recovered, confidence 84.** `Repulser_SpawnBlastEffect` (`0x088761d8`),
/// called from `Repulser_Init`, spawns this (`0x08a7cd18`, fourcc `REP3`) at the
/// entity's matrix `+0x1a0`, which `Repulser_UpdateFieldModel` rebuilds from the
/// firer's node every tick; `Repulser_Reset` releases it
/// (`docs/ghidra/functions/psp-pulse-usa/repulser.md`, `Race::advance_repulser_visual`).
pub const REPULSER_BLAST_EFFECT: &str = "WO_REPULSER_BLAST";

/// One Repulser wave, riding its centre.
///
/// **Recovered, confidence 80.** `Repulser_SpawnWaves` (`0x08876300`) spawns this
/// (`0x08a7cd2c`) twice, fourccs `REP0`/`REP1`, at the two waves' matrices
/// (`+0x60`/`+0xa0`), which `Repulser_AdvanceWave` moves every update;
/// `Repulser_Reset` releases both. A third on a junction's branch (`REP2`) is not
/// drawn: the port's waves take no branches.
pub const REPULSER_EFFECT: &str = "WO_REPULSER";

/// What a connected LeachBeam draws on the craft it is draining.
///
/// **The disc's own, with a recovered trigger**: `LeachBeam_Advance`
/// (`0x08873fa0`) spawns it at the *target's* scene node, in the block that plays
/// the `LEACHENERGY` cue, each time the beam's ribbon-scroll cursor wraps (about
/// once a second over the authored `active_time`). This engine attaches it once
/// when the link connects and follows the target, because the re-spawn cadence
/// depends on ribbon geometry this crate does not build. **Chosen, not measured**,
/// the cadence only; effect and anchor are recovered
/// (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`).
pub const LEACHBEAM_ENERGY_EFFECT: &str = "WO_LEACHBEAM_ENERGY";

/// The spark burst a Cannon round throws when it hits **track geometry**.
///
/// **Recovered, confidence 85.** `Cannon_UpdateRound` (`0x0886593c`, EU
/// `0x08865798`) raycasts each round against the track's collision mesh every tick
/// (`FUN_0883198c`, shared with the Rocket's, Missile's, Plasma's and Shuriken's
/// updates) and only when the hit type is `0` or `4` spawns this file with
/// `Psys_Spawn_q`, oriented to the hit basis at the hit point; the name string is
/// at `0x08a7c890` on USA, `0x08a7c0e0` on EU, confirmed by memory read.
///
/// **A craft hit does not reach this call**: the struck hull throws its own damage
/// sparks from `Ship_Damage` (`race::hit_sparks`); see
/// `oag_weapons::projectile::cannon`'s module doc, "The wall hit spawns a spark
/// effect; the craft hit does not". `Race::blast_for` mirrors that split:
/// `Weapon::Cannon` reaches this only when `struck` is `None`.
///
/// **A burst**, played through `Race::ignite_blast` like every detonation burst.
pub const CANNON_SPARKS_EFFECT: &str = "WO_CANNON_SPARKS";

/// What a craft *holding* a LeachBeam draws, before it fires anything.
///
/// **The disc's own, with a fully recovered trigger**: `FUN_0883f540`
/// (`0x0883f540`) spawns it on the holder's node whenever the held-weapon id is
/// `10` and the state is racing, and tears it down when either stops being true.
pub const LEACHBEAM_CHARGING_EFFECT: &str = "WO_LEACHBEAM_CHARGING";

/// Wipeout HD's own drain-trip burst: what `LeachBall_Advance`'s inlined spawn
/// (and its out-of-line, uncalled twin `LeachBeam_SpawnAbsorbEffect`, `0x00114a00`)
/// fires every time the render-side accumulator `oag_fx::beam::hd_ball` tracks
/// wraps, once a drain trip.
///
/// **The disc's own, trigger recovered, confidence 82**: fourcc `0x4541424c` and
/// the spawn call are a direct decompile of `LeachBall_Advance` (`0x00114c78`),
/// `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, "2026-09-25". **HD-only**,
/// absent from `PSP_WIRED` and `PS2_WIRED` in
/// `crates/raceplay/tests/psys_inventory_ground_truth.rs`, as
/// [`PLASMA_LIGHTNING_EXPAND_EFFECT`] is.
///
/// **Played as a one-shot burst**: the original allocates a fresh instance at every
/// wrap, the shape of `Race::ignite_blast`'s bursts. Chosen over
/// [`LEACHBEAM_ENERGY_EFFECT`]'s attach-and-follow because the decompile reads no
/// "same instance kept alive" requirement.
pub const LEACHBEAM_ABSORB_EFFECT: &str = "WO_LEACHBEAM_ABSORB";

/// What HD's Cannon throws on the craft it hits: a one-shot spark burst on the
/// hull's `Ship Collision Fx` locator nearest the contact.
///
/// **Recovered, confidence 72.** `Cannon_ApplyCraftHit` (`0x0010f730`) calls
/// `Ship_DispatchCollisionFx` with `kind` hardcoded to `1`, and
/// `ShipCollisionFx_Trigger`'s `kind == 1` branch names this effect from TOC slot
/// `0x008b3f6c` -> string `0x007a1c68`. It spawns unattached (attach flag `0`,
/// where the ship's own attached wall spark passes `1`), so it is played with
/// `psys::Stage::play`. HD-only: Pulse's Cannon spawns nothing on a craft and the
/// struck hull sparks from `Ship_Damage` (`race::hit_sparks`), so a Pulse race
/// reports it missing and fires nothing.
pub const WEAPON_SPARK_EFFECT: &str = "WO_SHIP_SPARK_DAMAGE_WEAPON";

/// HD's damage smoke below its first shield threshold (70).
///
/// **Recovered, confidence 70.** `ShipDamageFx_Update_q` (`0x002a17e8`) spawns it
/// attached at the craft's damage-effect matrix when a weapon hit leaves the
/// shield above 70 (`docs/ghidra/functions/ps3-hdfury-eu/particle-triggers.md`,
/// "The damage smoke"). A one-shot of 200 ticks (3.33 s), not looping.
pub const DAMAGE_MILD_EFFECT: &str = "WO_DAMAGE_MILD";

/// Between 40 and 70 shield left; see [`DAMAGE_MILD_EFFECT`].
pub const DAMAGE_MODERATE_EFFECT: &str = "WO_DAMAGE_MODERATE";

/// At or under 40, played with [`DAMAGE_MODERATE_EFFECT`]; see
/// [`DAMAGE_MILD_EFFECT`].
pub const DAMAGE_CRITICAL_EFFECT: &str = "WO_DAMAGE_CRITICAL";

/// The welding sparks Basilico (`01_Track`) and circuit five place on their own
/// scenery: three nodes on Basilico, six or seven on circuit five.
///
/// **Recovered, confidence 85, trigger confirmed live.** Not a code literal: the
/// circuit's `.vex` places a `ParticleSystem` node whose `Name` attribute is this
/// string, and `PsysNode_Init` (`0x089156a0`) spawns it at load on the node's
/// matrix. PPSSPP caught exactly the three Basilico loads. Played by
/// `race::scenery_fx` (`docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md`);
/// this constant names it for `super::RACE_EFFECTS`'s loader, the placement comes
/// from the disc.
pub const BLUE_WELDER_EFFECT: &str = "WO_BLUE_WELDER";

/// The steam vents circuits five and seven place: two on five, eighteen on seven.
/// **A Pulse effect**, despite the name. Same placement and evidence as
/// [`BLUE_WELDER_EFFECT`]; Outpost 7's eighteen loads were caught live too.
pub const MODESTO_STEAM_EFFECT: &str = "WO_MODESTO_STEAM_A";

/// Fort Gale's rain, its `<Weather EnvPsys>`.
///
/// **Recovered, confidence 80, played by `race::scenery_fx::weather`.** The
/// trigger is `Weather_Construct` (`0x088f184c`) off the circuit's `TrackStartup`;
/// the name here is for `super::RACE_EFFECTS`'s loader, the circuit's XML names the
/// effect it plays (`docs/ghidra/functions/psp-pulse-usa/weather.md`).
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
/// (`0x81000930`) and `FUN_810018d4` (the Zone-family test) are both false; a 2048
/// event's mode id is a CRC (`>= 0x17`), read live as `0x026886dc` for a campaign
/// Time Trial. Played by `race::magstrip_wake`.
pub const MAGSTRIP_SPARKS_EFFECT: &str = "WO_MAGSTRIP_SPARKS";

/// The Zone twin of [`MAGSTRIP_SPARKS_EFFECT`]. Which 2048 events count as a Zone
/// for `FUN_810018d4` is **chosen, not measured**: `Mode::Zone`.
pub const MAGSTRIP_ZONE_EFFECT: &str = "WO_MAGSTRIP_ZONE";

/// The effect `ShipCollisionFx_Trigger` spawns when a wall contact dealt damage: a
/// four-emitter tree (orange smoke puffs, a bright spark fountain, white `bits`
/// debris, lingering `_TRAIL` embers). Pulse's weapon hits throw the same one on
/// the struck hull. Equal to `oag_fx::sparks::DAMAGE_EFFECT`, pinned by an
/// `oag-raceplay` test.
pub const COLLISION_SPARK_EFFECT: &str = "WO_SHIP_COLL_SPARK_DAMAGE";

/// What `ShipCollisionFx_Trigger` spawns for a contact that cannot damage (its
/// record's friction is not positive): the damage file's bright fountain plus
/// `bits`. The collision cage's contacts are these. Equal to
/// `oag_fx::sparks::NO_DAMAGE_EFFECT`.
pub const NO_DAMAGE_COLLISION_SPARK_EFFECT: &str = "WO_SHIP_COLL_SPARK_NODAMAGE";

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

/// Pure's big blast at each wreck node.
pub const BIG_EXPLO_EFFECT: &str = "WO_SHIP_FXNODE_BIGEXPLO";

/// Pure's debris cloud at the wreck.
pub const EXPLOSION_DEBRIS_EFFECT: &str = "WO_SHIP_EXPLOSION_DEBRIS";

/// Pure's smoke at the wreck.
pub const EXPLO_SMOKE_EFFECT: &str = "WO_SHIP_EXPLO_SMOKE";

/// What HD plays on a craft that flies through an engine trail.
///
/// `Trail_SpawnHitEffect` (`0x002e3858`) consumes a per-trail "hit a ship" flag the
/// `Trails` SPU job raises, and `Trail_HitShipEffect` (`0x002d9ec0`) spawns this
/// system parented to the nearest of ten hull attachment nodes on the craft
/// (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`). Equal to
/// `oag_fx::exhaust::hd::TRAIL_HITSHIP_EFFECT`.
/// Equal to `oag_fx::exhaust::hd::TRAIL_HITSHIP_EFFECT`.
pub const TRAIL_HITSHIP_EFFECT: &str = "WO_TRAIL_HITSHIP";

/// The Fury-skinned variant of [`TRAIL_HITSHIP_EFFECT`], chosen by the byte that
/// turns the ribbon red (`craft + 0x7d2c`, written by `Ship_SetFuryTrailFlag`), so
/// the sparks are red exactly when the ribbon is. Equal to
/// `oag_fx::exhaust::hd::TRAIL_HITSHIP_RED_EFFECT`.
pub const TRAIL_HITSHIP_RED_EFFECT: &str = "WO_TRAIL_HITSHIP_RED";
