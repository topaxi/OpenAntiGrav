# A destroyed craft: `Ship_SetState` states 4 to 8

| | |
| --- | --- |
| **Binary** | `PS3_GAME/USRDIR/EBOOT.elf` (HD/Fury, BCES-00664), Ghidra `/hdfury/EBOOT-ps3-hdfury-eu.elf` |
| **Twins** | 2048 `/2048/eboot-vita-2048-eu-v104.elf`, Omega `/omega/eboot-ps4-omega-eu.bin` |
| **Subsystem** | ship state machine, wreck model, death effects, destroy camera, wreck collision |
| **Related** | [craft-flags.md](craft-flags.md) (`Ship_SetState` row, `+0x6a64` bits), [renderer.md](renderer.md) (`Ship_ReloadModelForSkin`, `Ship_SyncActiveModelPointer`), [particle-triggers.md](particle-triggers.md) (item 5), [physics.md](physics.md), [mp-time-trial-collision.md](mp-time-trial-collision.md) (the `+0x70` pair byte), [mode-manager.md](mode-manager.md) (mode ids), [rumble.md](rumble.md); Pulse's shape: [ship-wreck-model.md](../psp-pulse-usa/ship-wreck-model.md), [wreck-collision.md](../psp-pulse-usa/wreck-collision.md), [camera.md](../psp-pulse-usa/camera.md) ("The destroy camera") |

Read 2026-11-22, `hd-wreck-re`. Static: the bridge-connected Ghidra, every
TOC-relative load resolved against the function's own TOC (`0x008ad4d8` for every
function on this page, `scripts/ps3-toc.py toc`), the jump tables decoded by hand
because Ghidra ends `Ship_SetState` at its `bctr` (`0x000e1ee8`) and decompiles
none of the cases. The function really ends at `0x000e2ec4` (the next OPD function
starts at `0x000e2ec8`). Live check: not run, see "Measured on RPCS3: not run".

**Question.** HD draws no wreck: `Looks::hull_wreck` is unread. What does a
destroyed HD craft draw, which effects fire where and when, what the camera, HUD
and sound do, and whether the wreck still collides.

**Answer in one line.** HD swaps to `shipwreck.vex` **at state 4** (the moment of
the blast, not 0.5 s later as on Pulse), throws `WO_SHIP_EXPLOSION` at the hull
node with `EXPLBIG_PC`/`~SHIP_DESTROYED` and a burning loop, kicks the body into a
tumble on a separate compound collider, cuts to the destroy camera, and keeps the
wreck in the physics world until it has lain still for 6 s (or 20 s pass), when
state 6 takes it out of the world and out of the pair test.

## The census: every title ships a wreck per ship directory

Read with `scripts/psarc.py list` on the decrypted HD image and on every
`.psarc` of the Omega merged tree and 2048's (`PCSF00007`) base, DLC and v1.04
patch.

| Title | Wreck files | Where |
| --- | --- | --- |
| HD/Fury | `shipwreck.vex` and `shipwreck.rcsmodel` in **every one of the 37 ship directories** (12 teams x base, `_c1`, `_n1`, plus `detonator`, `zone`); `/data/ships/mirage/` appears in two archives | `DATA00`/`DATA02.PSARC`, `/data/ships/<team>/` |
| HD/Fury | wreck textures per livery: `<team>_wrecked*.gtf`, `*glass*wrecked.gtf`; material `/data/materials/ships/glass_texture_wrecked.rcsmaterial`; Zone `zone_tp_wrecked.gtf` per `zoneship_<team>` | `DATA02.PSARC` |
| 2048 | the same 36 HD directories under `data/art/published/hdships/<Team>[_c1/_n1]/` (`_n1` twice, base and patch), `Detonator`; plus 2048's own `Ships/<team>2048/<1..4>/` for `ag_systems`, `auricom`, `feisar`, `piranha`, `qirex` (20 sets) | `data.psarc`, `data1/2.psarc` |
| Omega | the HD set under `Data/art/published/hdships/` plus `tigron`, `tigron_x`, `Detonator_VR`; 2048's `ships/<team>2048/<1..4>/` and `VR_2048` (24 sets) | `data00`..`data09.psarc` (merged) |

So the wreck is **a model file per team on all three titles**, never the hull with a
material change. The hull's own textures carry a `_wrecked` variant on HD; which
material the wreck model binds is the wreck file's own (not read here; the
renderer reads `.rcsmodel` materials already).

`Ship_ReloadModelForSkin` (`0x000dbdc8`, [renderer.md](renderer.md)) loads
`'%s\shipwreck.vex'` into `ship+0x6ae4`, beside the hull `+0x6ae0` and the two LODs
`+0x6ae8`/`+0x6aec`. `ship+0x6adc` is the live model.

## The state machine

`Ship_SetState(ship, state)` (`0x000e1e20`) first calls `Craft_SetState` on the
body `+0x5fac` (not for the role-1 craft), clears `body+0x264` bit `0x400`, runs
one of ten cases through the table at **`0x000e1eec`** (TOC slot `0x008a8ff4`), and
always ends at `0x000e1f48`: `ship+0x5fa4 = state` (`0x000e1f88`), plus a network
event `0x00677a78(slot, state)` for a role-0/2 craft with `+0x6a64 & 0x800`.

| State | Case arm | Per-tick arm (dispatcher `0x000eb2bc`, table `0x000eb2e4`) |
| ---: | --- | --- |
| 4 | `0x000e20e8` | `0x000ed000` -> `Ship_UpdateExploding` (`0x000e3d18`) |
| 5 | `0x000e2088` | `0x000ed010` -> `Ship_UpdateWrecked` (`0x000e3768`) |
| 6 | `0x000e1fd0` | `0x000ed018`, inline |
| 7 | `0x000e2590` | `0x000ecf0c`, inline |
| 8 | `0x000e254c` | `0x000ecf50` -> `Ship_UpdateResetting` (`0x000e2f48`) |
| 0, 1, 2, 3, 9 | `0x000e265c`, `0x000e1f14`, `0x000e266c`, `0x000e2428`, `0x000e25e8` | not this page |

The per-tick dispatcher is in `FUN_000eadb8` (`lwz r3,0x5fa4(r31)`; `cmplwi 8`;
`bctr` at `0x000eb2e0`); the three helpers have no direct `bl` that Ghidra sees
because the dispatcher's arms are undisassembled. A raw scan of the text for `bl`
encodings finds exactly `0x000ed000 -> 0x000e3d18`, `0x000ed010 -> 0x000e3768`,
`0x000ecf58 -> 0x000e2f48`.

**Entry.** `Ship_SetState(ship, 4)` is called from `Ship_ApplyDamage_q` three
times (`0x000e817c`, `0x000e8358`, `0x000e8860`) and from `0x000e2f1c`, `0x000e4180`,
`0x000e6814`. Every `li r4` before a call was read (28 sites). The ship's timer is
`+0x6a78` (counts down), `+0x6a88` the time in state (counts up), `+0x6a7c` a
second timer.

**The modes.** `r30` in `Ship_SetState` is `g_GameState+0xe0` (zero when the debug
byte `*0x009384e1` is set). Ids per [mode-manager.md](mode-manager.md): 8 and 20 are
Eliminator (SP, MP), 2 Demo, 16 to 21 multiplayer, 6, 13, 14 have no ModeManager
(6 is Zone on 2048's model loader, 14 Zone or Detonator). Masks used below:
`0x206040` = {6, 13, 14, 21}, `0x3f2101` = {0, 8, 13, 16 to 21}, `0x4040` = {6, 14}.
"Ordinary race" below means a single-player race in none of these (Arcade 3/9/12,
Tournament 4, Time Trial 5/10, and the other default ids).

## The ready-to-wire table

Role: `ship+0x628c`, `0` the local player, `2` an AI opponent, `1` a remote craft
([ai-controls.md](ai-controls.md)). Conf. is the rubric's, static unless the cell
says "live".

| # | State, when | What happens | Address | Conf. | 2048 | Omega |
| ---: | --- | --- | --- | ---: | --- | --- |
| 1 | 4, on entry | **The wreck replaces the hull**: `Ship_SelectWreckModel` points `+0x6adc` at `+0x6ae4` (`shipwreck.vex`), clears visible bit `4` on the hull and both LODs, sets it on the wreck unless `+0x5f42`, regathers the node lists. Before it, the then-live model's `+0x34 \|= 2`. Not in modes 6 and 14. | `0x000e22c8` (and `0x000e2dbc` for Demo) | 88 | **ported**: `FUN_811c6f72` (slot `+0x6134`), case 4 after `+0x60b8 = 0.3`; every team's wreck loads (`omega_2048_shield_colour_ground_truth::every_2048_team_loads_a_wreck`) | **ported**: `FUN_01301260` (slot `+0x7288`), case 4; every team and every 2048-era ship loads one (`every_omega_team_loads_a_wreck_and_so_does_every_2048_era_ship`) |
| 2 | 4, on entry | **The blast**: `Ship_PlayDestroyExplosion` spawns `WO_SHIP_EXPLOSION` (modes 13 and 21, whose model set is `<team>_n1` on 2048's loader: `WO_NITRO_SHIP_DEATH`) once, with the live model's node matrix (`*(model+0x38)`): its three rows as the rotation and **translation `t - 4.0 * row1`**, 4 units back along the node's second row (`0x000d9810`-`0x000d9874`: `lvlx v1,0,r9` at `0x000d9824` splats `4.0` from `0x008a8c54` = TOC `-0x4884`, `vmaddfp` then `vsubfp` off the `+0x30` row) | `0x000d98fc`, `0x000d9acc` | 80 | **ported**: `FUN_811bc2bc` names both; `WO_SHIP_EXPLOSION` thrown. The `_n1` model set's `WO_NITRO_SHIP_DEATH` (modes 13 and 21) is not wired | **ported**: `FUN_012f7a60` names both; same, `WO_NITRO_SHIP_DEATH` not wired |
| 3 | 4, on entry | **Sound**: a new emitter (`0xb0` bytes, `+0x5ef8`) rides the body matrix (`body+0x1d0`). Local player: bank cue `EXPLBIG_PC` (`Sound_PlayBankCue`, flags `0x400`) and the looped `~SHIP_DESTROYED` into handle `+0x7a34`, distance params set `0x1c`. Others: `EXPLBIG`, params `0x1d`, no handle. Then (ordinary races only, not modes 8, 13 or MP) `~SHIP_ONFIRE` on the craft's own emitter `+0x5ee0`, handle `+0x7a30`, volume 1.0. Modes 6 and 14 play bank cue `shipdestroy` instead and skip all of this. | `0x000d9778`, `0x000d97e4`, `0x000d9a18`, `0x000e2358`, `0x000e299c` | 82 | checked, applies, not wired: same five strings, same branch shape; a `--dry-run` of the extracted `PCSF00007` with the four names added to the load reports every 2048 race cue "names no cue" (not only these), so whether they resolve is unread and `DestroyedCraft::HD_BLOWUP` keeps `~BLOWUP` there | checked, differs: `ShipDestroyed_Player` / `ShipDestroyed_NPC` (Wwise) beside `EXPLBIG`, `_SHIP_ONFIRE` (sic); no `.bnk` carries their ids (Z5), so not wired and `~BLOWUP` -> `_NGP_ShipDeath` stays |
| 4 | 4, on entry | **Camera shake** (local only): `0x000a3528(camera[pad], 0.3, 0.4, 3)`, the same arguments as Pulse's `Camera_ArmShake(0.3, 0.4, camera, 3)` at its state 5 | `0x000d976c` | 65 | not read | not read |
| 5 | 4, on entry | **Destroy camera** (local craft, or a remote one `RaceManager` `0x00055e30` says is watched; not if the camera is already in mode 2): HUD node `*0x009879dc` loses bits `1\|2\|4` of `+0x34` (hidden), `Camera_SetSubject(cam, ship, 1)` (station from `ship+0x7984` else picked near the body, field `fov + rand`, mode 10), `cam+0x34 \|= 6`, then `Camera_SetMode(cam, 8)`; modes 6 and 14 instead `0x000a9b18(cam, subject)`, `+0x6ad4 = 0`, mode 2. Single player also `0x000884d8(g_Hud)`, `*0xabc170+0x5c = 0` and `RaceManager+0x2dc4 = 4.5`. With `cam+0x34 & 6`, `cam+0x1ec` this craft and mode 8, the engine-flare gate in [engine-trail.md](engine-trail.md) (`0x002a0b84`, modes `{2, 6, 7, 8, 11}`) skips the subject's flare quad: no flare on the player's wreck (65, the two reads joined, not run). The view itself (mode 8's per-frame law) is read in [destroy-camera.md](destroy-camera.md): Pulse's mode 5 station view with substepped easing, random rate factors and a look-at wobble, handed back by the camera's 8 s clock. | `0x000e21e4`-`0x000e2298`, `0x000e2c6c` | 78 | checked, differs: same calls, `Camera_SetMode(cam, 9)` (`FUN_8121963a`), not 8 | checked, differs: mode 9 (`FUN_01382720`), like 2048 |
| 6 | 4, on entry | **The wreck tumbles on its own collider**: `CollisionWorld_ReplaceObject(g_CollisionWorld, +0x6948, +0x694c)` swaps the hull box for the compound collider `Ship_Construct` builds at `+0x694c` (`0x160` bytes, type 5, child shapes sized off the hull box, `0x000ded30`-`0x000defb0`); `body+0x4cc = +0x694c`; `Body_SetBoxInertia(body, 1, 12, 6, 12)`; `body+4 = 1`; `*(body)+0x40 = 0`. `Ship_ApplyDeathKick_q` throws a random impulse at the hull node (tuning `0x008c15e0 + 0x18..0x34`: 4000, 8000, 200, 3500, 1, 15, 16, 2000). The swap, the inertia and `~SHIP_ONFIRE` share one path: single-player modes (`<= 0xf`) other than 8 and 13. Modes 8 and 13 set `body+0x4c8 \|= 2` instead; the MP modes do neither. The kick runs in every mode but 6 and 14. | `0x000e237c`-`0x000e23dc`, `0x000e2338` | 80 | checked, applies, not wired: `FUN_811d267a(+0x5f84, +0x5f8c)`, `FUN_811d35a8(1.0, .., 6.0, body)` | ported law, other code: inverse inertia written inline as `1/15, 1/24, 1/15`, which is `Body_SetBoxInertia(1, 12, 6, 12)` evaluated; collider `+0x70d8` into `+0x4c8` |
| 7 | 4, on entry | **Rumble**: own pad `Rumble_QueueOnPad(.., pad, 0x3c)` (`Ship_Exploding_Player_Rumble`, 60); an opponent's blast queues `0x3b` (59, `Ship_Exploding_Rumble`) on every pad, positioned at the live model. Not in Demo. | `0x000e2408`, `0x000e2acc` | 85 | checked, differs: no rumble call (Vita) | checked, applies, not wired: `FUN_0174a880(.., 0x3c)` and the `0x3b` loop |
| 8 | 4, on entry | **Out of the race**: `+0x6a64 \|= 0x1000` (the bit [eliminator-flip.md](eliminator-flip.md) and [detonator-mode.md](detonator-mode.md) read), deaths `+0x6f78 += 1`, announcer `cont_elim` (`Sound_QueueAnnouncerCue`) unless Eliminator, Demo or `RaceManager+0x2cd5`; `0x000d0128` (local: two rumble writes; `0x0011a0e8` on `+0x6f60`) | `0x000e2418`, `0x000e2320` | 85 | checked, applies, not wired | checked, applies, not wired (`+0x71f5 \|= 0x10`) |
| 9 | 4 -> 5 | **0.3 s**: case 4 sets `+0x6a78 = 0.3` (`0x3e99999a`, TOC `-0x49f0`); `Ship_UpdateExploding` subtracts the frame time and enters 5 at `<= 0` | `0x000e21a0`, `0x000e3d18` | 85 | **ported**: `+0x60b8 = 0.3` | **ported**: `+0x7208 = 0.3` |
| 10 | 5, on entry | Timer `+0x6a78 = 2.0` (modes `0x3f2101`), `2.5` (modes 6, 14) or `2.0` plus **the death plume**: `ShipDamageFx_Update(flare +0x5f70, shield, 1)` in every other mode (its `param_4 != 0` branch, `0x002a1840`-`0x002a191c`: outside mode 14 it spawns **`WO_SHIP_DEATH_DAMAGE_PLUME`** once, fourcc `0x4c504457`, attached (`r7 = 1`) at the flare anchor `object+0x1c0` that `0x002a0268` rebuilds each tick, then sets the death flag `object+0x2b4 = 1`, which moves that anchor from `1.5` to `2.0` along the flare node's Z ([particle-triggers.md](particle-triggers.md), "The smoke's anchor"); mode 14 branches to `0x002a1920`, unread). On the 4 -> 5 edge `Ship_UpdateExploding` also calls `0x00104f18` (outside the `0x206040` modes) or `0x00105450` (a craft whose entry names `"nitro"`) on `+0x7e40` with the body matrix (unread) | `0x000e2088`, `0x000e20d8`, `0x000e3d18` | 75 | same masks and timers, `FUN_81271a50(.., 1)` | same masks `0x3f2101`/`0x4040`, `FUN_016f3ef0(shield, +0x64c8, 1)` |
| 11 | 5 -> 6 or 8 | `Ship_UpdateWrecked`: settled is `\|body velocity\|^2 < 0.5` and `body+0x210 == 2`. Leave when (time in state `> 6.0` and settled) or time `> 20.0` (tuning `+0x12c`, `+0x130`; file values), to **6**, or to **8** in modes 8, 13 and the MP modes 16, 17, 20, 21 with `+0x6a64 & 0x800`. A respawning mode also leaves at the 2.0 s timer. Local player after 6 s: `0x003048c0(6)` if `0x00304610() < 6`. | `0x000e37d0`-`0x000e3928` | 78 | not read | not read |
| 12 | 6, on entry | **The wreck leaves the world**: `+0x6a78 = 0`; `PhysicsWorld_RemoveBody(*0x00992ce0, body)` and `*(body+0x4cc)+0x70 = 0`, the pair byte both craft colliders need ([mp-time-trial-collision.md](mp-time-trial-collision.md)); stops `~SHIP_DESTROYED` (`+0x7a34`, `0x002fea88`) and frees the emitter `+0x5ef8` (`0x002fd528`). **The model is not touched: the wreck stays drawn.** Modes 6 and 14 instead hide the live model, the engine flare `+0x6af4` and the death shell `+0x6afc` and play their own sequence: [zone-destroy.md](zone-destroy.md). | `0x000e2710`-`0x000e277c` | 82 | **ported**: `*(+0x5f84+0x434)+0x30 = 0`, `FUN_811d44f8` | **ported**: `*(+0x70c8+0x4c8)+0x40 = 0`, `FUN_01320730` |
| 13 | 6 -> 7 | Only in MP modes (`> 0xf`) without `+0x6a64 & 0x800`; a single-player wreck stays in 6 to the race's end | `0x000ed018` | 80 | not read | not read |
| 14 | 7 | `*(body+0x4cc)+0x70 = 0`; `+0x6a64 \|= 0x1000` by mode; per tick, back to 1 when `g_GameState+0x4e4[slot]` is set (an MP rejoin) | `0x000e2590`, `0x000ecf0c` | 65 | same shape | same shape |
| 15 | 8, on entry | Timer `0.8` (others) or `1.0` (local); stops `~SHIP_ONFIRE` and `~SHIP_DESTROYED`, frees the emitter | `0x000e254c` | 82 | `0.8`/`1.0` | `0.8`/`1.0` |
| 16 | 8 -> 1 | `Ship_UpdateResetting`, at `<= 0`: `body+0x4c8 &= ~2`; local plays bank cue `RESET`; **`Ship_SyncActiveModelPointer` puts the hull back**; local HUD node `+0x34 \|= 2`, camera bit `2` cleared; placed at a respawn pose (`+0x62f0 + 0x60 * slot`); `Ship_SetState(ship, 1)`; shield refilled by mode; `+0x6a64 &= ~0x1000` | `0x000e2f48` | 78 | `RESET` in case 3, same timers | `RESET` in case 3 |

### Wired on HD (2026-11-22, `hd-wreck-wire`)

Not run against RPCS3 (see "Measured on RPCS3: not run"): every row below is the static
reading wired and tested on ours. The data is `oag_title::destroyed::DestroyedCraft::HD`
(`Title::destroyed`), the effects are HD's `Trigger::WreckExplosion` and `Trigger::WreckPlume`,
the wreck rule is `Looks::hull_wreck`. 2048 and Omega run `DestroyedCraft::HD` too, **inherited from HD** (`Sourced::inherited`, the twins' static read), below.

| Row | Status on HD | Where |
| ---: | --- | --- |
| 1 | wired: `shipwreck.vex` (+ `.rcsmodel`) of the craft's own directory, drawn from state 4; not in Zone (modes 6 and 14, row 12's Zone half) | `oag_livery::wreck`, `scene::wreck::live_model_for` |
| 2 | wired: `WO_SHIP_EXPLOSION` on entering state 4, `4.0` rows behind the model, no Pulse shockwave | `craft_flash`, `wreck_fx::throw_wreck_explosion` |
| 3 | wired 2026-11-23 (`hd-wreck-sound`): see "Sound on HD" below | `oag_raceplay::destroyed_cues`, `oag_sound::sfx::slot_held` |
| 4 | not wired (65) | open |
| 5 | not wired: HD keeps the existing destroy camera, **inherited from Pulse** (`DestroyedCraft::camera`); mode 8's view is read and ready to wire ([destroy-camera.md](destroy-camera.md), "Ready to wire") | open |
| 6 | **not wired**: the compound collider and death kick (open items 1 and 2) | open |
| 7 | not wired | open |
| 8, 13, 15, 16 | the Eliminator route is the existing Pulse mechanism, unchanged and with Pulse's timers (`raceplay::eliminator`) | open for HD's own timers |
| 9 | wired: `0.3` s | `DestroyedCraft::explosion_secs` |
| 10 | wired: `WO_SHIP_DEATH_DAMAGE_PLUME` at the flare anchor (`2.0` along Z once dead) on entering state 5, not in the Eliminator | `damage_fx::throw_death_plume` |
| 11 | wired: `6.0` s then speed squared under `0.5`, or `20.0` s; the `body+0x210 == 2` half is **not read** and is not tested | `oag_physics::damage::advance_state` |
| 12 | wired: state 6 is solid to no craft, the body is no longer integrated (opponents; the player's wreck ends the race) and the wreck stays drawn | `solid_to_craft`, `RaceSim::wreck_left_world` |
| 14 | not wired (65) | open |

### Sound on HD (2026-11-23, `hd-wreck-sound`)

Row 3, row 12's stop and row 15's stop, off `Title::destroyed.sound` (`DestroyedSound::Ship`, HD
only; Pulse and Pure keep `~BLOWUP`, and HD no longer plays `~BLOWUP`, which its executable never
names). `RaceSim::raise_destroyed_cues` runs once a tick after the states settle and raises edges:

| State edge | Cues (slot = the craft) | Our state |
| --- | --- | --- |
| 4, local player | `EXPLBIG_PC`, then held `~SHIP_DESTROYED`, then held `~SHIP_ONFIRE` | `Racing` to `Destroyed` |
| 4, any other craft | `EXPLBIG`, then held `~SHIP_ONFIRE` (no `~SHIP_DESTROYED`) | same |
| 6 | stops `~SHIP_DESTROYED`; the fire burns on | `Eliminated` with its timer at `0` (`in_state_6`) |
| 8 | stops what is open | the craft back to `Racing` (Eliminator only; an ordinary wreck never reaches it) |

The Eliminator plays the blast and `~SHIP_DESTROYED` but no `~SHIP_ONFIRE` (row 3's mode gate) and
holds the loop to state 8; Zone and Detonator (modes 6 and 14) raise nothing here (`shipdestroy` is
[zone-destroy.md](zone-destroy.md)'s). Cues are an output, never `World` state. Held voices are
`oag_sound::sfx::slot_held`, the magstrip hum's mechanism generalised to a start and stop cue pair.
**Chosen, not measured:** the two loops are drawn like the magstrip's (one draw that loops), a flat
pick, not the original's list (the thread `hd-race-cues-are-guarded-lists-played-as-one-flat-pick`,
open item 2: held cues need a runner); `EXPLBIG` is a 40-waveform tree picked uniformly, `EXPLBIG_PC`
plays its 10-voice timeline; the multiplayer gate (modes 16 to 21) is not modelled. Placed on the craft
(`Placement::Craft`), the player's own too; whether the original's `0x400` flag makes the player's blast
dry is unread. Not heard against the original (no live RPCS3). Cue order of an HD opponent destroyed
(`--race --opponents --force-wreck 100:7`, `RUST_LOG=oag_sound=trace`): `ShipExplosion` on slot 7, held
`ShipOnFire` opened on slot 7. The player (`--force-wreck 100:0`): `ShipExplosionPlayer` (10 voices),
`ShipDestroyed`, `ShipOnFire`. Tests: `raceplay::tests::hd_destroyed_cues`, and on the disc
`hd_destroyed_sound_ground_truth` (cue edges for both, and the mixer's voice count with and without the
loops at the four states).

### Wired on 2048 and Omega (2026-11-23, `wreck-2048-omega`)

`Title::destroyed` is `DestroyedCraft::HD` on both (inherited from HD), `Looks::hull_wreck` is on
everywhere, and each names `WO_SHIP_EXPLOSION` (`Trigger::WreckExplosion`) and
`WO_SHIP_DEATH_DAMAGE_PLUME` (`Trigger::WreckPlume`) in its effects table; Pulse's two per-locator
wreck effects stay unset, as on HD. Rows 1, 2, 9, 10 and 12 follow HD's wiring and carry its status
above; rows 3 to 7, 11's `body+0x210` half and 14 are open exactly as on HD. The destroy camera is
HD's inherited one: the twins' mode 9 (`Camera_SetMode` `0x8121963a`, `0x01382720`) is read in
[destroy-camera.md](destroy-camera.md) and not wired, so `DestroyedCraft::camera` stays labelled inherited from Pulse. Not run against Vita3K or shadPS4.
Our frames (`--race --force-wreck 40:7 --opponents`, Altima on 2048, Modesto Heights on Omega,
960x540, ticks 60 and 100): an orange fireball with white smoke puffs and sparks at the blast, the
wreck lying inside it, so the HD black-ball picture is not seen here. On 2048 the tick-60 blast has a hard vertical
edge on its right (a clipped sprite), not looked into.

**Unconfirmed by play or a frame of the original:** rows 1 and 12 (the wreck appears at the blast and stays to the race's end) are the code reading only; the maintainer does not remember it either.

Ours' frames (`--race --force-wreck 60:7`, Talon's Junction grid, 1280x720): the hull is charred
the tick state 4 begins, a white flash covers it for about 0.1 s, a black smoke ball with sparks
follows, then the scorched wreck lies with a bright plume at its tail.

**Against Pulse.** Pulse swaps to the wreck at state 5, 0.5 s after the blast, throws
`WO_SHIP_FXNODE_EXPLO` and `WO_SHIP_DEATH_SPARKS` at every wreck locator then, and
`WO_SHIP_EXPLOSION` 1.5 s later at state 6 ([ship-wreck-model.md](../psp-pulse-usa/ship-wreck-model.md)).
HD has neither node effect string, so **HD's wreck sequence is one blast at the
swap**, then the death plume, then nothing until the wreck is taken out of the
world. HD's states do not map onto `CraftState::Eliminated` as Pulse's do: on HD
the wreck is live from state 4.

## Functions

| Address | Name | Conf. | Evidence |
| --- | --- | ---: | --- |
| `0x000db400` | `Ship_SelectWreckModel` | 88 | instruction for instruction `Ship_SyncActiveModelPointer` (`0x000db090`) with `+0x6ae4` and `+0x6ae0` exchanged; `+0x6ae4` is where `Ship_ReloadModelForSkin` loads `shipwreck.vex`; called in `Ship_SetState` case 4; 2048 and Omega twins in their case 4 |
| `0x000d96c8` | `Ship_PlayDestroyExplosion` | 82 | the only function naming `~SHIP_DESTROYED`; `EXPLBIG_PC`/`EXPLBIG`; `WO_SHIP_EXPLOSION`/`WO_NITRO_SHIP_DEATH` (strings through its own TOC); the 2048 and Omega twins name the same set |
| `0x000d44d0` | `Ship_ApplyDeathKick_q` | 60 | random draws (`0x0028c660`, `Libc_Rand`) over tuning `+0x18..+0x34`, then `0x000f6348` with the live model's node position and `0x000f5dd8` on the body; formula not decoded |
| `0x000e3d18` | `Ship_UpdateExploding` | 82 | state-4 arm of the per-tick dispatcher; `+0x6a78 -= dt`, `Ship_SetState(5)` at `<= 0` |
| `0x000e3768` | `Ship_UpdateWrecked` | 78 | state-5 arm; the settle and timeout rule, `Ship_SetState(6)`/`(8)` |
| `0x000e2f48` | `Ship_UpdateResetting` | 78 | state-8 arm; `RESET`, `Ship_SyncActiveModelPointer`, `Ship_SetState(1)` |
| `0x000abc20` | `Camera_SetMode` | 80 | `+0x44 = +0x40; +0x40 = mode` between two twelve-way tables (leave, enter); Pulse's `Camera_SetMode` keeps the same pair |
| `0x000acab0` | `Camera_SetSubject` | 72 | `+0x1ec = craft`; the craft's own station `+0x7984` or a pick near `body+0x200` (`0x000ac7a0`); mode 10; starting field `+0x290` or `fov + rand`; Pulse's `0x0888058c` does the same with `+0xc3c` |
| `0x00033578` | `CollisionWorld_ReplaceObject` | 80 | finds `old` in the `+0xf764` list (stride 8, count `+0x17764`) and writes `new` in its place |
| `0x000f76e8` | `PhysicsWorld_RemoveBody` | 75 | finds the body in `+0x48` (`0xa0` slots), moves the last entry in, `+0x2c8 -= 1` |

`Ship_SetState` (`0x000e1e20`) is named at 65 in [craft-flags.md](craft-flags.md)'s
section ("the table not read"); this page reads the table, and the 2048 twin
`FUN_811c711e` and Omega's `FUN_012ff700` have the same ten cases with the same
timers, masks and strings: by the rubric that is 88.

Twins named here (rows in each binary's `names.tsv`):

| Binary | Address | Name | Conf. | Evidence |
| --- | --- | --- | ---: | --- |
| 2048 v1.04 | `0x811c711e` | `Ship_SetState` | 82 | ten-way switch on `param_4`, state word `+0x5638`; case 4 `+0x60b8 = 0.3`, `shipdestroy`, `cont_elim`, `~SHIP_ONFIRE`, `+0x60a4 \|= 0x1000`; case 8 `0.8`/`1.0` |
| 2048 v1.04 | `0x811c6f72` | `Ship_SelectWreckModel` | 78 | live model `+0x612c` set to `+0x6134` with the visible bit `4` moved, HD's shape; called in case 4 only |
| 2048 v1.04 | `0x811bc2bc` | `Ship_PlayDestroyExplosion` | 78 | names `EXPLBIG_PC`, `~SHIP_DESTROYED`, `EXPLBIG`, `WO_NITRO_SHIP_DEATH`, `WO_SHIP_EXPLOSION` |
| Omega | `0x012ff700` | `Ship_SetState` | 82 | the same ten cases; state word `+0x6510`; case 4 `+0x7208 = 0.3`, rumble `0x3c`/`0x3b`, `_SHIP_ONFIRE`; case 5 masks `0x3f2101`, `0x4040` |
| Omega | `0x01301260` | `Ship_SelectWreckModel` | 78 | live model `+0x7280` set to `+0x7288`, bit `4` moved; case 4 only |
| Omega | `0x012f7a60` | `Ship_PlayDestroyExplosion` | 78 | names `ShipDestroyed_Player`, `ShipDestroyed_NPC`, `EXPLBIG`, `EXPLBIG_PC`, `WO_NITRO_SHIP_DEATH`, `WO_SHIP_EXPLOSION` |

2048 and Omega also carry a second, 2048-native ship family (2048 `0x812c7e2c`,
Omega `0x0176fe30`) that names `WO_SHIP_EXPLOSION_PLAYER`; which craft uses it is
not read here.

## Measured on RPCS3: not run

2026-11-22, not reached, two attempts. The host's RPCS3 is the packaged
`rpcs3-bin 0.0.42-19777` (installed 2026-08-17); the 0.0.43-20574 build every HD
save state since 2026-11-09 was made on is no longer installed. (1)
`emu-restore-state.sh hd-wreck-re .../hd-racebox-talons-venom-midrace-absorb.SAVESTAT.zst`:
RPCS3 aborts the load in `ppu_load_prx` (`Verification failed (object: 0x0)`,
`PPUModule.cpp:1895`). (2) A cold boot, `OAG_WEAPONS_ON=1 emu-hd-race-walk.sh hd-wreck-re 8 0 0`
(GDB stub off, as that script sets it): the guard reports `STALLED at boot after 240s`,
with no screen ever recognised (`screen: ?`). Whether 0.0.42 boots the game at all
there, or only prints none of the lines the screen tracker reads, was not
looked at. Everything above is static, cross-checked against the 2048 and Omega twins.

**The read recipe**, for a build that boots: attach to a `serve` in a Racebox race
with weapons on, write each opponent's shield (`ship+0x5fa0`, `ship+0x628c == 2`,
crafts at `RaceManager+0xe8`) to `0.05`, hold `cross`, and poll per craft every
change of `+0x5fa4`, of the live model `+0x6adc` against `+0x6ae0`/`+0x6ae4`, of
the collider `*(ship+0x6944)+0x4cc` against `+0x6948`/`+0x694c` and of its
`+0x70` byte, with the physics frame counter `*0x00992ce0 + 4` and the timers
`+0x6a78`/`+0x6a88`. Rows 1, 6, 9, 11 and 12 predict: state 4 with `wreck` and
`wreckcol` on one frame, state 5 about 18 frames later (0.3 s), state 6 once the
time in state passes 6.0 with the wreck still, the pair byte `0` from then on, and
the live model still `wreck`.

## Open

1. **`Ship_ApplyDeathKick_q`'s formula**: the impulse and the torque from the eight
   tuning words; decompile `0x000d44d0` with `0x000f6348` and `0x000f5dd8`.
2. **The wreck collider's child shapes**: `Ship_Construct` `0x000ded10`-`0x000defb0`
   builds a `0x160`-byte compound and adds `0x90`-byte children (radius at `+0x88`,
   `0x00039c90`); count and placement unread.
3. ~~**Camera mode 8's view** (HD) against Pulse's mode 5 law~~ Read 2026-11-23:
   [destroy-camera.md](destroy-camera.md) (enter entry 8 at `0x000abd14` writes the field `35.0`).
4. **The `+0x7e40` call on the 4 -> 5 edge** (`0x00104f18`, `0x00105450`). Pointer (2026-11-23,
   [zone-destroy.md](zone-destroy.md), "Functions"): `+0x7e40` is the debris manager and `0x00104f18` is
   `DebrisManager_SpawnShipDebris`, which throws **19 `Ship_Damage%d` pieces** at the body matrix with
   random velocities, those past a tuning count with `WO_DEBRIS_FIRE`; `0x00105450` is its nitro
   variant (32 pieces). So an ordinary HD death very likely throws debris at 0.3 s, and "one blast at
   the swap, then the death plume, then nothing" above is incomplete (72, decompile only; the table
   is not changed here).
5. ~~Zone (modes 6 and 14, rows 1, 3, 5, 12)~~ Read 2026-11-23: [zone-destroy.md](zone-destroy.md)
   (mode 6 is Zone, mode 14 Detonator; the shell's `0.4` term is its animation time, not an alpha).
