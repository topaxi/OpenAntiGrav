# Omega's race cues: which event each of this port's cues posts

Lane `omega-wwise-cue-table`, 2026-11-12. Read statically in Ghidra on the **1.07 patch executable**
(`/omega/eboot-ps4-omega-eu-v107.bin`, image base `0x01000000`; every address below is the patch's),
checked against the shipped banks' own Wwise listings (`data/audio/sound/English(US)/Weapons_NGP.txt`,
`Ship_NGP.txt`, `English(US)/Speech_NGP.txt` in `data08.psarc`) and against a real race load
(`crates/raceplay/tests/omega_countdown_ground_truth.rs`,
`a_race_plays_its_posted_cues_from_their_wwise_events`). The event-name law and the request layout
are [wwise-sound.md](wwise-sound.md); this page is the per-cue table that page left open.

**The headline: 26 of this port's 43 race cues (`oag_gameplay::cue::Cue::ALL`) have a cue string the
executable posts; 10 of them load and play for every emitter today, plus `NGP_LEACH` for the player
alone.** The other 16 posted cues stop at a switch container (14), an ATRAC9 frame the decoder rejects
(1, `QUAKELAUNCH`) or an event that starts no sound this loader reads (1, `NGP_LEACH` for non-player
emitters; why is unread). The 17 cues with no post are 12 held loops, 4 legacy events the executable never posts, and
the collision cue, whose choice is unread.

## How a cue string was read (confidence 80 for the method)

The patch inlines `SoundGroup_PlayCue` into every caller (wwise-sound.md, "Emitter groups and cue
requests"). Each inlined request does the same stores into a request popped off the manager's free list
(`DAT_021aaa68 + 0x9c18`): `[req+0x60]` the bank id and `[req+0x70]` the bank name, both read as
`manager + 0x2e8 + 8 * slot` and `manager + 0x368 + 8 * slot` for a slot global, then
`lea reg, [cue string]; mov [req+0x68], reg` and `mov dword [req], 2` (post pending). Example, the plasma
launcher `FUN_0138ab50`: `SoundGroup_Create(manager, "Plasma", 0, ship+0x5f70->+0xe0)` (the player byte
comes from the firing ship's own group), then `[req+0x68] = "NGP_Plasma"` with slot `DAT_021aaa4c`.
The instruction pairs quoted below (`0x0137dcef: lea rdx, [0x018787f0]` then `0x0137dcf6: mov [rcx+0x68],
rdx`) were read with `get_assembly_context`; the slot is the nearest slot-global read before the store.

A Ghidra script swept every candidate string (each event name of the three banks' listings, the
`_PC__<bank>` suffix stripped): its xrefs, the function holding each, and the slot globals read within 60
instructions. A string present in the executable is not a post on its own: `PLASMA`, `MISSILE`, `TURBO`
(`0x01892cb9`, ...) are referenced only by `FUN_0169c3d0`, a weapon-name table, and the legacy
`PLASMA__Weapons_NGP` events they would name are never posted.

### The slots (confidence 90)

`SoundManager_Construct` in the patch (`0x01784120`, decompiled; base `0x017359c0`) loads the boot banks
and keeps each slot index in a global:

| Global | Bank |
| --- | --- |
| `DAT_021aaa40` | `frontend` |
| `DAT_021aaa44` | `speech_fe` |
| `DAT_021aaa48` | `generaltrack` |
| `DAT_021aaa4c` | `Weapons_NGP` |
| `DAT_021aaa50`, `DAT_021aaa54` | `Speech_NGP` (the same slot twice) |
| `DAT_021aaa58` | `Speech_NGP_Grid` |
| (none, slot 0) | `Ship_NGP`, loaded directly into slot 0 when not already loaded |

`crowd` is loaded and its slot not kept. `DAT_021aaa5c` is written by the track loader (`FUN_013c23c0`,
`env0_zone` in Zone); `DAT_021aaa60` by the front end (`frontend_fliers`).

## The table

`Cue` is this port's (`oag_gameplay::cue::Cue`, named by Pulse's spelling in `oag_sound::sfx::CueTables::name`).
The event is `<cue>__<slot>` from a non-player emitter and `<cue>_PC__<slot>` from the player's.
**Plays** is the race load's result (`oag_sound::sfx::Banks::load_wwise_cues`).

| `Cue` | Omega cue | Slot | Where (store site, function) | Conf. | Plays |
| --- | --- | --- | --- | ---: | --- |
| `Message` | `MESSAGE` | `Weapons_NGP` | `0x01278a5f`, `FUN_0126b030` (HUD) | 65 | yes (6 voices) |
| `Absorb` | `NGP_Absorb` | `Weapons_NGP` | `0x0131ac6c`, `FUN_0131a9e0` | 80 | yes (3) |
| `MineLaunch` | `MINELAUNCH` | `Weapons_NGP` | `0x01378398`, `FUN_01377cb0` (with `_MINERADAR`) | 80 | yes (2) |
| `Plasma` | `NGP_Plasma` | `Weapons_NGP` | `0x0138ad10`, `FUN_0138ab50` (group `"Plasma"`) | 85 | yes (13) |
| `Rocket` | `ROCKET` | `Weapons_NGP` | `0x01396e3c`, `FUN_01396480` (with `_ROCKETTVL`, `Rocket`) | 80 | yes (4) |
| `Missile` | `NGP_MissileFire` | `Weapons_NGP` | `0x0137d01a`, `FUN_0137c0c0` (wwise-sound.md) | 85 | yes (5) |
| `CannonHitShip` | `CANNONEXPLSHIP` | `Weapons_NGP` | `0x01358c74`, `FUN_01356b20` (`CannonManager`) | 75 | yes (1; player 2) |
| `LeachFail` | `NGP_LEACHFAIL` | `Weapons_NGP` | `0x0136a1c2`, `FUN_0136a0a0` | 75 | yes (4; player 3) |
| `LeachEnergy` | `NGP_LEACHENERGY` | `Weapons_NGP` | `0x0136b2df`, `FUN_0136a490` | 65 | yes (1) for non-player emitters; the player's `_PC` event is authored and starts no sound this loader reads, so the player hears nothing |
| `Repulsor` | `REPULSOR` | `Weapons_NGP` | `0x0139597b`, `FUN_013952c0` (with `_REPULSORTRAVEL`) | 75 | yes (8) |
| `Leach` | `NGP_LEACH` | `Weapons_NGP` | `0x01369bfe`, `FUN_01369a70` (with `_NGP_LEACHATTACH`) | 75 | player only: `NGP_LEACH__Weapons_NGP` starts no sound this loader reads (why is unread) |
| `SpeedupPad` | `Pad_General` | `Weapons_NGP` | `0x0173bae4`, `FUN_0173b210` | 70 | no: switch container |
| `Turbo` | `NGP_Turbo` | `Weapons_NGP` | `0x0173c679` `FUN_0173be50`; `0x012b03b9` `FUN_012afea0` | 70 | no: switch container |
| `ShieldActive` | `shieldactive` | `Speech_NGP` | `0x0133e335`, `FUN_0133e2a0` (with `_SHIELD`) | 70 | no: switch container |
| `Engaging` | `autopilot_eng` | `Speech_NGP` | `0x017ba52a` `FUN_017ba150`; `0x013159fe` `FUN_01315000` | 70 | no: switch container |
| `Disengaging` | `disengaging` | `Speech_NGP` | `0x013a7888`, `FUN_013a2cd0` | 70 | no: switch container |
| `PlasmaHitWall` | `NGP_PlasmaHitWall` | `Weapons_NGP` | `0x0138c53b`, `FUN_0138c410` | 80 | no: switch container |
| `PlasmaHitShip` | `NGP_PlasmaHitShip` | `Weapons_NGP` | `0x0138d850`, `FUN_0138c640` | 80 | no: switch container |
| `RocketHitWall` | `ROCKEXPLWALL` | `Weapons_NGP` | `0x013825f3` `FUN_01380870`; `0x01399916` `FUN_013986d0` | 75 | no: switch container |
| `RocketHitShip` | `ROCKEXPLSHIP` | `Weapons_NGP` | `0x01382a6c` `FUN_01380870`; `0x0139994c` `FUN_013986d0` | 75 | no: switch container |
| `MissileHitWall` | `MISSILEEXPWALL` | `Weapons_NGP` | `0x0137dcef`, `FUN_0137d1e0` (with `WO_MISSILE_BOUNCE`) | 80 | no: switch container |
| `Cannon` | `NGP_Cannonfire` | `Weapons_NGP` | `0x01354994`, `FUN_01353df0` (with `CannonBullet`) | 60 | no: switch container |
| `CannonHitWall` | `CANNONEXPLWALL` | `Weapons_NGP` | `0x01358bb6`, `FUN_01356b20` | 75 | no: switch container |
| `QuakeLaunch` | `QUAKELAUNCH` | `Weapons_NGP` | `0x01368666` `FUN_013684e0`; `0x017babc1` `FUN_017ba150` | 75 | no: media `405228712` fails to decode ("invalid reuse band params", frame 0) |
| `QuakeHit` | `QUAKEHIT` | `Weapons_NGP` | `0x017b6aae`, `FUN_017b1850` | 75 | no: switch container |
| `ShurikenLaunch` | `SHURIKEN` | `Weapons_NGP` | `0x0139eefd`, `FUN_0139ea70` (with `_SHURIKENTRAVEL`) | 75 | no: switch container |

Notes on single rows:

* **`Cannon` (60).** `FUN_01353df0` stores either `NGP_Cannonfire` or `NGP_CannonHeavy` at the same
  request (`0x01354994` / `0x013549e1`, beside `WO_SPENT_CARTRIDGE` and `WO_SUPER_CARTRIDGE`): the heavy
  shot is the super cartridge's. The other `CANNON` post (`0x01391a53`, `FUN_013916a0`) sits beside
  `PulseBullet` and four track names and is not this port's Cannon; `NGP_Cannonfire_LoopStart`/`Stop`
  are posted by the weapon controllers (`FUN_01315cc0`, `FUN_017bace0`) and are not a one-shot.
* **`SpeedupPad` (70).** `FUN_0173b210` posts twice: `Pad_General` on `Weapons_NGP` (`0x0173bace` reads
  the slot, `0x0173bae4` the string), then `Pad_<team>` (`Pad_Auricom`, `Pad_Feisar`, `Pad_Qirex`,
  `Pad_AGsystems`, `Pad_Piranha`, picked by `strcmp` on the team, 2048 teams mapped to the HD ones) on
  slot 0, `Ship_NGP`. Only the first is in the table; the team layer needs the emitter's team.
* **`Message` and `LeachEnergy` (65).** No slot global is read within 250 instructions of the store; the
  event exists only in `Weapons_NGP`'s listing, which is why the slot is that one.
* **The player's version.** The loader loads `<cue>_PC__<slot>` where the bank defines that event (an
  `Event` object of that hash in the slot's entries); where none is authored (the three speech cues) the
  player hears the non-player event, and where it is authored and does not load (`LeachEnergy`) the
  player hears nothing rather than a substitute. 10 cues load a player event (`Leach` only that).
  `NGP_MissileFire_PC__Weapons_NGP` is its own event: three `SetState` (`0x1204`) actions beside its relay,
  where the non-player event is a bare relay (wwise-sound.md, "One weapon sound").

## The cues with no post

| `Cue` | Why not played |
| --- | --- |
| `Engine`, `Shield`, `Autopilot`, `Blowup`, `LockOn`, `PlasmaTravel`, `RocketTravel`, `MissileTravel`, `QuakeTravel`, `LeachAttach`, `ShurikenTravel`, `Magstrip` | **Held loops.** Omega spells them with a leading `_` (Pulse's `~`, turned into `_` by `SoundGroup_PlayCue`) and keeps a handle: `_SHIELD` (`0x0133e376`), `_AUTOPILOT` (`0x01315911`), `_NGP_MissileLock` (`0x01282e3a`), `_NGP_Plasmatvl` (`0x0138ba47`), `_ROCKETTVL` (`0x01396f5e`), `_NGP_MissileTravel` (`0x0137d13e`), `_QUAKETRAVEL` (`0x0176749d`), `_NGP_LEACHATTACH` (`0x01369e5d`), `_SHURIKENTRAVEL` (`0x0139f08b`), `_magstrip01` (`0x0132b32d`, slot 0); the engine is `Ship_NGP`'s `xpress`/`xpress_low`/`xrelease` set (`FUN_0130b8c0`) and the craft destruction `_NGP_ShipDeath` (`0x0131e922`). `oag_sound::sfx::wwise_start` reads no loop flags, so each would play once: left silent. |
| `MissileHitShip` | The missile impact (`FUN_0137d1e0`) names only `MISSILEEXPWALL`; which cue a ship hit posts is unread. `MISSILEEXPSHIP__Weapons_NGP` is in the bank, its string is not in the executable. |
| `MissileExpire` (`SHURIKENEXPL`), `ShurikenHit` (`SHURIKENHIT`), `RepulsorHit` (`REPULSORHIT`) | **Never posted.** The bank keeps the legacy event; the executable holds no such string. |
| `Collision` | Posted as `_COLLISIONS_1` or `_COLLSHIELD` (shield up), with `_GRIND`/`_GRINDSHIELD` for scraping, on slot 0 (`FUN_0130b100`, `0x0130b333`, `0x0130b32c`); `_`-prefixed with a kept handle, and the choice between them is unread. |

## Limits this table meets, not fixed here

* **Switch containers** (14 cues): the child is the game's switch value (ship class, surface, player or
  not), refused at play ([wwise.md](../../../formats/wwise.md#sound-containers)).
* **Per-post draws:** a random container's child is drawn once at load, seeded by the event name, so every
  post of a cue plays the same pick.
* **Positioning:** every loaded cue plays through the existing craft placement (pan and distance from the
  emitter), not the event's own 3D parameters, which are unread ([audio-3d.md](audio-3d.md)).
* **Action kinds** `0x1204` (`SetState`) and `0x0103` are reported, not played.
* **"Starts no sound this loader reads"** (`NGP_LEACH`, `NGP_LEACHENERGY_PC`): the event's actions reach
  no sound `wwise_start` plays: a relay into another bank, a dangling target or state and stop actions
  alone; which, is unread.
* **`QUAKELAUNCH`'s media** fails in `oag_music`'s ATRAC9 decoder at frame 0.

## Cross-title

* **2048: checked, shares the cue column, differs in container.** The open Vita 1.04 executable
  (`/2048/eboot-vita-2048-eu-v104.elf`) holds `NGP_Absorb`, `MINELAUNCH`, `NGP_MissileFire`,
  `ROCKEXPLWALL`, `NGP_Plasma`, `Pad_General` and `NGP_Turbo` as strings (7 of the 8 probed; not
  `_COLLISIONS_1`). 2048 plays `SBlk` banks with hashed names, not Wwise events, so the cue strings carry
  over and the table's event column does not; not wired here.
* **HD:** plays its own `SBlk` cues (`weapons.bnk`), unchanged.

## Reproduce

```sh
cargo nextest run -p oag-raceplay --run-ignored all a_race_plays_its_posted_cues_from_their_wwise_events
RUST_LOG=info,oag_sound=debug oag-game --race --no-audio --screenshot out.png --ticks 2 data/extracted/ps4
# [DEBUG] sfx: 10 of 43 race cues play their Wwise event, 10 with a player version
```
