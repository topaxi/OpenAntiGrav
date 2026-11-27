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
| 5 | wired 2026-11-23 (`destroy-cameras-wire`): `DestroyedCraft::camera` is `DestroyCamera::HD`, mode 8's station view, rates and 8 s hand-back ([destroy-camera.md](destroy-camera.md), "Wired") | `oag_raceplay::destroy_camera`, `oag_render::camera::destroy` |
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
wired as mode 9 (`Camera_SetMode` `0x8121963a`, `0x01382720`) with the hold view
([destroy-camera.md](destroy-camera.md), "Wired"). Not run against Vita3K or shadPS4.
Our frames (`--race --force-wreck 40:7 --opponents`, Altima on 2048, Modesto Heights on Omega,
960x540, ticks 60 and 100): an orange fireball with white smoke puffs and sparks at the blast, the
wreck lying inside it, so the HD black-ball picture is not seen here. On 2048 the tick-60 blast has a hard vertical
edge on its right (a clipped sprite), not looked into.

**Unconfirmed by play or a frame of the original:** rows 1 and 12 (the wreck appears at the blast and stays to the race's end) are the code reading only; the maintainer does not remember it either.

Ours' frames (`--race --force-wreck 60:7`, Talon's Junction grid, 1280x720): the hull is charred
the tick state 4 begins, a white flash covers it for about 0.1 s, a black smoke ball with sparks
follows, then the scorched wreck lies with a bright plume at its tail.

### The black blast (HD, 2026-11-23, our frames, confidence 85 for the data, the original's picture unknown)

Bisected on `--race --force-wreck 40:7` (Talon's Junction, 960x540, ticks 45 to 60) by silencing
emitters (`per_emission = (0, 0)`): the black ball at tick 54 to 60 is `WO_SHIP_EXPLOSION`'s **root
emitter** alone (alpha-over, 2 per emission, 180 ticks); muting the whole effect removes the ball
and the sparks, muting the other eight emitters leaves it. The cause is in the file's colour
table, not in a blend mode, a missing texture or the decode:

- All nine sprites resolve (`/data/psys/tex/<stem>.gtf` in DATA00 to DATA06) and decode; the root's
  is `smoke_new4_8x4`, a 1024x512 8x4 flipbook that is **fire, yellow and white-hot cells, not
  smoke grey**, with alpha. `FIREBALL_ADDITIVE`'s is a 1024x1024 8x8 orange fire flipbook.
- The root's `colour_mode` (`+0xbc`) is **3** (so are `trail` and `TRAIL_SMOKE`; the other six
  are 0), a value nothing in `docs/` reads. Its 256-entry table is white at entry 0, then
  **black with alpha 1 for entries 1 to 31**, grey at 32 and 33, white from 40. Sprite times that
  table is black for the first eighth of a particle's life (age 0.12, about 22 ticks of 180),
  which is where the ball is.
- 2048's `WO_SHIP_EXPLOSION` (`data/particles/` and `particles2048/`) authors the **same** table
  on its root emitter and the same black on `TRAIL_SMOKE`, so 2048 is not a counter-example of a
  different file: it draws a small fireball with dark smoke at the same ticks, and its sprites
  are not the white-hot flipbook. The HD root pairs a black ramp with fire art.
- Evidence it is the table: replacing `sprite = colour * texel` with `texel.rgb` in
  `psys.wesl` (a probe, not kept) draws an orange fireball with a white core at ticks 54 and 60.
  That is the sprite's own colour and **not** a fix: it ignores a table the file authors.

Two things the HD effect needs that this page cannot supply, both unread:

1. **What `colour_mode` 3 does on HD.** Read since: the table is not a colour there but a
   blend weight, and the black run means "draw entry 0", white. See "`colour_mode` 3 is a blend
   toward the scene light" below.
2. **HD's frame advance for this effect** is read and wired ([particle-render.md](particle-render.md#the-atlas-frame)).
   The white blowout listing `WO_SHIP_EXPLOSION` showed at ticks 45 to 50 was measured before
   that; the effect has been listed since, and the blowout was not re-measured.

Not shipped: no frame, shader or table was changed. Pinned: `psys_ground_truth.rs`,
`hd_ship_explosion_root_is_a_fire_flipbook_under_a_black_colour_ramp` (the sprite resolves, the
table is black for entries 1 to 31).

### `colour_mode` 3 is a blend toward the scene light (HD, 2026-11-23, confidence 84)

Read at instruction level on both of HD's particle-update paths, the PPU one and the SPU job.
They agree, and the result is not a multiply. Under mode 3 the table's red byte is a **blend
weight**. It moves each particle's colour from table entry 0 toward a light colour sampled
once per instance per update from the scene's lighting volume. Entries 1 to 31 are black, so
their weight is 0 and the particle takes entry 0, which is white: the fire flipbook draws as
authored. The black ball comes from our reading the weights as colours.

**The context.** `Psys_BuildUpdateContext` (`0x00281ec0`) fills a per-instance block. The
PPU update and the SPU job both read it: `Psys_QueueSpuUpdate` (`0x0027c6d0`) calls the builder,
then hands the block to the `MaintainParticles` SPU job (its DMA list layout is not decoded).

```text
ctx+0x18  byte   = (emitter+0xbc != 2)                         colour animates
ctx+0x19  0x400  = memcpy(emitter+0xc4)                        the table, file byte order
if emitter+0xbc == 3:                                          cmpwi 3 at 0x00281f70
    RibbonEffects_SampleLighting(1.0, instance+0x130, ctx+0xb04)  bl at 0x002821cc
else:
    ctx+0xb04 = 0                                              0x00281f9c
```

`RibbonEffects_SampleLighting` (`0x002a4280`, [rocket-trail.md](rocket-trail.md)) looks the
instance's position up in the lighting volume. It scales the sample and clamps it to
`[0, 1]`. Lane 3 is forced to `1.0`: `vsldoi`/`vsel` at `0x002a43b8`-`0x002a43c8`, constant
`*0x0079bfd0 = 1.0`. `Colour_PackFloat4` (`0x002a39c8`) packs it as
`x<<24 | y<<16 | z<<8 | w`, each lane times `255.0` (`*0x0079bf70`). The light word's low
byte is therefore always `0xff`, never 0, so mode 3 always takes the lerp.

**The law, PPU** (`Psys_UpdateParticles`, `0x0028cc10`; `r27 = r31 = ctx`, set at
`0x0028cc54` and `0x0028cd24`):

```text
0x0028ce10  lbz   ctx+0x18; 0 -> skip the colour step
0x0028ce1c  idx = trunc(age(+0x90) * 255.99997) & 0xff         (*0x008b2a70 = 0x437fffbe)
0x0028ce2c  L   = ctx+0xb04
0x0028ce48  (L & 0xff) == 0 -> 0x0028d2a8: the plain walk:
                colour(+0x74) = (colour & 0xff000000) | (table[idx] & 0x00ffffff)
otherwise:
0x0028ce4c-0x0028ce84  E0 = table[0] as a word (unindexed ctx+0x19..+0x1c)
0x0028ce58-0x0028cecc  L' = byteswap(L)                        lanes now match E0's order
0x0028ce8c  w   = byte at ctx + idx*4 + 0x1c                   (the entry's last byte)
0x0028cf28  t   = w * (1/255)                                  (*0x008b2a74 = 0x3b808081)
0x0028cf60.. per byte b of the four: out.b = trunc(E0.b + (L'.b - E0.b) * t)
            colour(+0x74) = out                                (alpha byte included)
```

The table is stored in file byte order, and HD's `.pob` writes each entry as a big-endian
`u32` whose bytes are `A, B, G, R` (`oag_pob` reverses them to RGBA). So the weight byte at
`+0x1c` is **red**, and the particle word has the same `A, B, G, R` layout. The alpha
channel block (`ctx+0x6a0`, after the colour step) then overwrites the alpha byte as it does
in every mode, when `ctx+0x6a0` is set (copied from render state `+0xc34`).

**The SPU job agrees.** The `MaintainParticles` job in the image at `0x0080a000`, read with
`scripts/ps3-spu-disasm.py 0x0080a000 0x1b80`, offsets into that range:

```text
b74..b8c   byte ctx+0x18 == 0 -> skip
b90..bbc   idx = cfltu(age * 0x437fffbe) & 0xff; L = word ctx+0xb04 (il 2820)
bb4..bc4   ceqbi on L's low byte; brhz -> 10a8 when it is not 0
bc8..c60   the plain walk: keep the top byte, RGB from the entry
10a8       brsl 0x950: byteswap(L)
10cc..1148 w = byte ctx + idx*4 + 0x1c; t = w * (1/255) (frest/fi and a Newton step on 255.0)
10d4..1144 E0 from ctx+0x19..+0x1c, unindexed
1150       brsl 0x8a8: per byte, cfltu((b - a) * t + a), a = E0, b = L'
1154..1160 store to particle +0x74
```

Two independent compilations of one source agree on every step. Which path runs depends on
`*(0x008b2964)+0x28` and on the job queue (`Psys_UpdateInstance_q`, `0x00285020`), so the
answer is the same either way. The score is 84, the ceiling for a static read. A live
`ctx+0xb04` read moves it up (below).

**The root of `WO_SHIP_EXPLOSION`.** From the table bytes in `DATA02`'s
`/data/psys/wo_ship_explosion.pob` (emitter at file `+0x80`, flags `0x0440420c`, life
`180 +- 20` ticks, rate 1.0):

| entries | file bytes `A B G R` | weight `t` | colour |
| --- | --- | --- | --- |
| 0 | `ff ff ff ff` | 1.0 | `L` |
| 1-31 | `ff 00 00 00` | 0 | entry 0: white, `(255, 255, 255, 255)` |
| 32-40 | `ff 1a1a1a` .. `ff e6e6e6` | 0.10 .. 0.90 | white blended toward `L` |
| 41-255 | `.. ff ff ff`, `A` falling `ff` -> `00` | 1.0 | `L` (the authored `A` column is never read) |

So, by life fraction:

- **0:** index 0, weight 1, `L`. A 160-200 tick particle's first update already puts it at
  age `1/180`, index 1 (white). On this path the spawn writes only the alpha byte (`stb 0`,
  `0x00289cb4`). The RGB store at `0x0028a3b8` is mode 2's random entry. So index 0 is
  never what a frame shows. The first drawn colour is **white**.
- **1/8:** `trunc(0.125 * 255.99997) = 31`, weight 0, **white**: the flipbook's own fire.
- **1/2:** index 127, weight 1, **`L`**: the flipbook tinted by the scene light at the
  blast. Blended over life fractions 0.125 to 0.16, about 6 ticks.

`L`'s value is unread: the lighting volume behind `RibbonEffects_SampleLighting` is open in
[rocket-trail.md](rocket-trail.md), whose live rocket samples on one circuit ran from
`0xbaffff` near the grid to `0x2b5d7b` in the dark. The original's blast is therefore
white-hot fire for its first eighth, then the same flipbook in the scene's light colour. It is
never the table's black. How dark it gets later depends on `L`, which is unread.

**Read recipe for `L`** (raises the 84): RPCS3 with the patched build, a breakpoint at
`0x002821d4` (just after the `SampleLighting` call, `bl` at `0x002821cc`, in
`Psys_BuildUpdateContext`), and `--force-wreck` on a known circuit. Read the word at
`r28 + 0xb04` (the context).
It runs on both update paths.

**Wired (`pob-colour-mode-wire`, 2026-11-23).** `oag_title::Effects::colour_walk` names each title's reading
(`oag_hd`: `LightBlend`; `oag_2048`, `oag_omega`: `Reversed`; Pulse and Pure keep `Forward`), and
`oag_fx::psys::Effect::with_colour_law` applies it to the emitters that walk their table, at load. The player's
three walks index with `255.99997` as the original does; Pulse's and Pure's forward walk is untouched. Pinned by
`colour::tests` (the root's life 0, 1/8 and 1/2, with a non-white `L`),
`psys_ground_truth.rs::hd_ship_explosion_root_draws_entry_zero_through_its_black_run_under_the_light_blend`,
and `hd_effect_sprites.rs::the_loader_gives_hds_ship_explosion_the_light_blend`.

**`L` is chosen, not measured.** Our lit ribbons use no lighting volume either: `rocket_smoke::UNLIT_RGB` (white,
the bright end of the live `0xbaffff` to `0x2b5d7b` span) stands in, and `oag_fx::psys::LIGHT` is that
constant. With a white `L` the blend returns white at every weight, so HD's root is white for its whole life
until the read recipe above gives a real `L`. Never fit it to a frame.

Frames (ours, `--race --opponents --force-wreck 40:7`, Talon's Junction, 960x540): `WO_SHIP_EXPLOSION` is
now in `oag_hd::effects::SPRITES` (pinned in `hd_effect_sprites.rs`), so the root draws its `smoke_new4_8x4`
flipbook. Tick 45 a white-hot flash, 54 and 60 an orange fireball with a white core, 75 the fireball
spreading with the sparks; no white blowout (the earlier one was the table read as colours). Unlisted, the
procedural soft ball drew white where it was black.
2048 (Altima, same command): at tick 54 the dark smoke ball becomes white fire (reversed walk: indices 255
down), the last eighth of the life is black by the table (not framed: tick 205 shows only the plume, the same before and after). Census, in `psys_2048_ground_truth.rs` and
`psys_omega_ground_truth.rs`: 2048 authors modes 0, 2 and 3 only (31 mode-3 emitters in 12 effects: the
ship explosion and its player twin, `WO_ROCKET_EXPLO`, `WO_ROCKET_EXPLO_TRACK`, `WO_ROCKET_FLARE`, `WO_MINE_EXPLO`,
`WO_CANNON_SPARKS`, `WO_DAMAGE_MODERATE`, `WO_DEBRIS_SPARKS`, `WO_GLOBE`, `WO_GLOBE_STATIC`,
`WO_SHIP_DEATH_DAMAGE_PLUME`) and no mode 1; Omega's two archives author 29 and 27 mode-3 emitters and no mode 1.
**Omega: ported by reader.** `Data\particles2048` (the 2048-layout set its 2048 reader `FUN_016e06b0` plays)
walks reversed: `oag_title::Effects::colour_walk_in`. `Data\particles` keeps the plain forward walk the HD-layout
reader `FUN_016ddf70` reads (no lerp; confidence 55, and which reader plays which directory is inferred from
the directories' names and the race's `effect_dir_by_circuit`, not read). Omega's HD-era mode-3 effects
therefore stay forward, with no light blend either.

**The frame advance** runs in the same update, after the colour step:
[particle-render.md](particle-render.md#the-atlas-frame). The root's flags have no `0x40`, so it
steps at a rate; its record authors spawn `[0, 10]` and holds on frame 31.

**2048: checked, differs.** 2048's per-particle update `Psys_UpdateParticle` (`0x812ab114`,
v1.04) has no lighting branch. `s17 = 1 - p[+0x6c] / p[+0x70]` (`vdiv` at `0x812ab1b0`, `vsub`
at `0x812ab1b8`), where `+0x6c` is the countdown the function decrements at its end and
`+0x70` the lifetime. So `s17` is the age fraction. At `0x812ab1f2`-`0x812ab23a`:

```text
mode = record+0x870; mode == 2 -> no walk
f = age (1 - remaining/lifetime)
mode != 0 -> f = 1 - f                                      0x812ab1fe..0x812ab204
idx = trunc(f * 255.99997) & 0xff; RGB bytes = record+0x470 + idx*4, bytes 0..2
```

So 2048 plays mode 3, and mode 1, as the walk **reversed**. The same table gives white for
most of the life (indices 255 down to 41), the grey ramp at life fractions 0.84 to 0.875, and
black for the last eighth. That is 2048's fire that ends in dark smoke, from the same file, with
no light term. Omega's copy of this reader agrees (below), so two binaries read the same
logic. Confidence 65: the runtime record's `+0x870` and `+0x470` are taken to be the file's
`+0xbc` and `+0xc4`. That mapping is unread, because no direct store to `+0x870` exists (the
record is presumably filled in bulk). Our player walks forward on every title, so on 2048
and Omega every emitter with mode 1 or 3 is affected, not only this blast. HD's mode 1 is
the plain forward walk.

**Omega: checked, differs.** Omega carries both readers. `FUN_016e06b0` is 2048's.
`xmm5 = remaining / lifetime` (`0x016e06f6`) and `xmm10 = 1 - xmm5` (`0x016e0737`).
`+0x870` is read at `0x016e08dc`: 0 walks with `xmm3 = xmm10`, the age (`0x016e08d1`); 2
skips; any other value walks with `xmm5`, reversed. That is the same law as 2048. `FUN_016ddf70` reads an HD-layout record (table at `+0xc4`,
`0x016de807`). It makes only the `mode != 2` test (`cmp r13d, 2`, `0x016de77e`) before the
plain walk (keep the top byte, `& 0xffffff`, `0x016de7e4`-`0x016de817`). No light lerp
appears there. Whether Omega's HD-layout path is reached for HD's ship explosion is not read
(confidence 55 for "Omega dropped HD's mode 3 lerp").

**Against Pulse.** Pulse swaps to the wreck at state 5, 0.5 s after the blast, throws
`WO_SHIP_FXNODE_EXPLO` and `WO_SHIP_DEATH_SPARKS` at every wreck locator then, and
`WO_SHIP_EXPLOSION` 1.5 s later at state 6 ([ship-wreck-model.md](../psp-pulse-usa/ship-wreck-model.md)).
HD has neither node effect string, so **HD's wreck sequence is one blast at the
swap**, then the death plume and the 19-piece debris throw ("The debris throw"), then nothing until the wreck is taken out of the
world. HD's states do not map onto `CraftState::Eliminated` as Pulse's do: on HD
the wreck is live from state 4.

## The debris throw (2026-11-24, `hd-death-debris`, static, confidence per line)

`Ship_UpdateExploding` (`0x000e3d18`), on the 4 -> 5 edge and outside the `0x206040` modes (Zone, Detonator
and modes 13 and 21), calls `DebrisManager_SpawnShipDebris(mgr = ship+0x7e40, matrix = body+0x1d0,
velocity = body+0x190)` (`0x000e3e18`, `lvx v2` of the body's `+0x190`), or `DebrisManager_SpawnNitroDebris`
(`0x00105450`) when the craft's `+0x6298` entry names `"nitro"` (`0x000e3ef0`). It runs in the Eliminator too.
Read from the disassembly (Ghidra truncates `Debris_Launch` at its `bl 0x00327500`, so every address below is
an instruction address).

| Part | Law | Address | Conf. |
| --- | --- | --- | ---: |
| Source | one model per piece: `Data/Ships/Ship_Damage/Part_%d.vex` (+ `.rcsmodel`), `%d` = 1..19 (nitro: `Ship_Damage_Nitro`, part `i/4+1`, 8 parts); the manager loads them in `DebrisManager_Construct` | `0x00105ba8`-`0x00105e18`, strings `0x00782e20`, `0x00782e58` | 85 |
| Count | 19 (`cmpdi r25, 0x13`); the previous set is released first (`0x0010c1f8` per piece) | `0x001050e4`, `0x00105014` | 90 |
| Refusal | the manager built for Zone (`mgr+0x30c`) prints `"Ship has only zone debris to spawn"` and returns | `0x00104fe0` | 85 |
| Spawn point | `matrix.row3 + 0 * ...`: the vector at `0x00992d10` (written once by the static init `0x00104760` from `0x00782d20`) is `(0, 0, -5, 0)`, so `p = position - 5 * row2`; our body's forward is `-Z`, so **5 units ahead of the craft** | `0x00105118`-`0x00105148`, `0x0010479c` | 85 |
| Orientation | the piece's node takes the body matrix rows (unit) with `p` as translation; the part vertices are in their own space | `0x00105348`, `0x0010cb08`-`0x0010cb60` | 75 |
| Velocity | `v = body_velocity * 1.0 + S * D`, `D = 25 * (A * row0 + B * row1 - row2)` (tuning `0x008c17d0 +0x24 = 25.0`, `+0x1c`, `+0x20 = 1.0`), `A`, `B` = `rand30 * 2^-29 - 1` in `[-1, 1)` (`Libc_Rand` `0x0043c280` returns the low 30 bits, `rldicl r3,r0,0,0x22`), `S = 1 + 0.95 * C` (`+0x08 = 0.95`, `C` the third draw). Our rows: `row0` right, `row1` up, `-row2` forward | `0x0010514c`-`0x0010527c` | 85 |
| Torque | `cross(row1, v) * 100` (`+0x0c = 100.0`), stored at record `+0x20`, which `Debris_Launch` copies to `body+0x1c0` (the torque slot of a body laid out velocity `0x190`, spin `0x1a0`, force `0x1b0`, torque `0x1c0`, matrix `0x1d0`). Whether it is an impulse or an accumulator, and the piece's inertia (`Body_SetBoxInertia(1, r, r, r)` with `r = 0.5 * lane 3 of 0x00677968(node+0x204c)`), are unread | `0x001052a4`-`0x00105304`, `0x0010c9ec`-`0x0010ca08` | 70 (the size: unread) |
| Gravity | record `+0x40` = `(0, -9.81, 0, 0) * 2.0` (`0x00782d60`, tuning `+0x10`), copied to the object at `+0x60` (`0x0010ca5c`); how the update applies it is not decoded | `0x001052a0`, `0x0010ca60`-`0x0010ca70` | 50 |
| Fire | pieces with index `> 16` (tuning word `0x10`, `cmplw r0, r24; blt`): zero-based 17 and 18, the last two, set the record's `+0x58` byte, and `Debris_Launch` attaches `WO_DEBRIS_FIRE` to the node (`0x0010cba0`-`0x0010ccb4`, fourcc `0x46424544`) | `0x00105310`-`0x00105354` | 82 |
| Life | `Debris_Update` (`0x0010d580`, per piece from `DebrisManager_Update` `0x001046b0`, vtable slot 2 of `0x00875238`): `+0x104 += dt`; a piece with `+0x10e == 0` (the ship's `r5 = 0`) is hidden (node `+0x34 &= ~4`) and its body removed from the world once `+0x104` exceeds tuning `0x008c1810 + 0xc` = **10.0 s**. The branches on body state `+0x210` (2, 3, 4) and the settled test (`speed^2 < 0.5`) are not decoded | `0x0010d7b0`-`0x0010d878` | 62 |
| Collision | `Debris_Launch` makes a type-7 body and adds it to the physics and collision worlds (`+0x70 = 1`), so a piece lands on the track | `0x0010c9a0`, `0x0010ca38`, `0x0010ca94` | 72 |

**Nitro** (`0x00105450`): 32 pieces, spawn offset `(0, 1, 0)` (`0x00992d10 + 0x10`), `D = 25 * (A * row0 + B * row1 + C * row2)`
with its own draws, the fade flag `+0x10f = 1` and no fire (`stb 0, 0x208`): read for shape only, **not wired**.

**Ours** (`oag_title::destroyed::ShipDebris::HD`, `oag_raceplay::ship_debris`, `oag_livery::ship_debris`): the
count, the source, the spawn point, the velocity law, the torque's direction, the fire and the 10 s life are the
reading above. **Chosen, not measured:** the pull `19.62` along world down (the record's vector, applied as a
plain acceleration), the floor plane the pieces rest on under the craft, the spin rate `6` rad/s (the torque's
size needs the inertia), and that a piece collides with nothing but that plane. At most four throws are in the
air at once (the oldest is reused). Test: `raceplay::tests::ship_debris` (19 pieces on the 4 -> 5 edge and not
before, two burning, gone at 10 s, none on Pulse, all forward). Our frames: `--race --opponents --force-wreck
40:1` on HD, the debris thrown at tick 58 (0.3 s after the blast), pieces lying on the track at 1.5 s with one
still burning.

**2048 and Omega: not checkable here.** Both inherit `DestroyedCraft::HD`, so they ask for
`Data\Ships\Ship_Damage\Part_N` and say in the loader report which piece is missing; their own layout is
unread. Neither package opens on this machine: the `.pkg`'s PFS layer needs a console-derived key
(`docs/formats/vita-package.md`), and `pkg2zip` leaves `data.psarc` encrypted. `FUN_811a468a` and
`FUN_012cf870` (`zone-destroy.md`) are their `SpawnZoneDebris`; the ship variant was not looked up.

## Functions

| Address | Name | Conf. | Evidence |
| --- | --- | ---: | --- |
| `0x000db400` | `Ship_SelectWreckModel` | 88 | instruction for instruction `Ship_SyncActiveModelPointer` (`0x000db090`) with `+0x6ae4` and `+0x6ae0` exchanged; `+0x6ae4` is where `Ship_ReloadModelForSkin` loads `shipwreck.vex`; called in `Ship_SetState` case 4; 2048 and Omega twins in their case 4 |
| `0x00281ec0` | `Psys_BuildUpdateContext` | 84 | fills the update block both particle paths read: `ctx+0x18 = (+0xbc != 2)`, the table copied to `ctx+0x19`, `RibbonEffects_SampleLighting` into `ctx+0xb04` only when `+0xbc == 3` (else 0); called by `Psys_UpdateInstance_q` and `Psys_QueueSpuUpdate` alike ("`colour_mode` 3 is a blend toward the scene light") |
| `0x0027c6d0` | `Psys_QueueSpuUpdate` | 72 | under 256 queued: builds the context, names `MaintainPariclesSpu pre-baking jobs...`, queues a job over the context, the pool and the instance, returns 1; 0 sends the caller to the PPU update |
| `0x00285020` | `Psys_UpdateInstance` | 65 | per instance: SPU job when `*(0x008b2964)+0x28` and `Psys_QueueSpuUpdate` succeed, else `Psys_BuildUpdateContext` then `Psys_UpdateParticles`; then the bounds copy-back and collision |
| `0x002a39c8` | `Colour_PackFloat4` | 85 | clamps a float4 to `[0, 1]`, times `255.0` (`*0x0079bf70`), packs `x<<24 \| y<<16 \| z<<8 \| w` |
| `0x000d96c8` | `Ship_PlayDestroyExplosion` | 82 | the only function naming `~SHIP_DESTROYED`; `EXPLBIG_PC`/`EXPLBIG`; `WO_SHIP_EXPLOSION`/`WO_NITRO_SHIP_DEATH` (strings through its own TOC); the 2048 and Omega twins name the same set |
| `0x000d44d0` | `Ship_ApplyDeathKick_q` | 60 | random draws (`0x0028c660`, `Libc_Rand`) over tuning `+0x18..+0x34`, then `0x000f6348` with the live model's node position and `0x000f5dd8` on the body; formula not decoded |
| `0x00104f18` | `DebrisManager_SpawnShipDebris` | 85 | "The debris throw": 19 `Part_N` pieces, the velocity, torque and fire law; the Zone refusal string |
| `0x00105450` | `DebrisManager_SpawnNitroDebris` | 75 | its twin for a craft whose entry names `"nitro"`: 32 pieces, `(0, 1, 0)` spawn offset, no fire, fade flag |
| `0x0010c958` | `Debris_Launch` | 78 | body type 7, velocity into `body+0x190`, torque into `body+0x1c0`, the physics and collision worlds, `WO_DEBRIS_FIRE` when the record's `+0x58` is set |
| `0x0010d580` | `Debris_Update` | 70 | per piece: `+0x104 += dt`, hidden and removed from the world past tuning `+0xc` (10.0 s); the body-state branches are not decoded |
| `0x001046b0` | `DebrisManager_Update` | 74 | vtable slot 2 of `0x00875238`: `Debris_Update(piece, dt)` for each of `mgr+0x310` pieces at `mgr+0x314` |
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
4. ~~The `+0x7e40` call on the 4 -> 5 edge~~ Read 2026-11-24, "The debris throw" above (nitro variant, the body-state branches of `Debris_Update`, the torque's size and the gravity's application stay open). Earlier pointer: (`0x00104f18`, `0x00105450`). Pointer (2026-11-23,
   [zone-destroy.md](zone-destroy.md), "Functions"): `+0x7e40` is the debris manager and `0x00104f18` is
   `DebrisManager_SpawnShipDebris`, which throws **19 `Ship_Damage%d` pieces** at the body matrix with
   random velocities, those past a tuning count with `WO_DEBRIS_FIRE`; `0x00105450` is its nitro
   variant (32 pieces). So an ordinary HD death very likely throws debris at 0.3 s, and "one blast at
   the swap, then the death plume, then nothing" above is incomplete (72, decompile only; the table
   is not changed here).
5. ~~Zone (modes 6 and 14, rows 1, 3, 5, 12)~~ Read 2026-11-23: [zone-destroy.md](zone-destroy.md)
   (mode 6 is Zone, mode 14 Detonator; the shell's `0.4` term is its animation time, not an alpha).
