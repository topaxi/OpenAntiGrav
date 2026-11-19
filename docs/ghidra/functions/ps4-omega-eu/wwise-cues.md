# Omega's race cues: which event each of this port's cues posts

Lane `omega-wwise-cue-table`, 2026-11-12. Read statically in Ghidra on the **1.07 patch executable**
(`/omega/eboot-ps4-omega-eu-v107.bin`, image base `0x01000000`; every address below is the patch's),
checked against the shipped banks' own Wwise listings (`data/audio/sound/English(US)/Weapons_NGP.txt`,
`Ship_NGP.txt`, `English(US)/Speech_NGP.txt` in `data08.psarc`) and against a real race load
(`crates/raceplay/tests/omega_countdown_ground_truth.rs`,
`a_race_plays_its_posted_cues_from_their_wwise_events`). The event-name law and the request layout
are [wwise-sound.md](wwise-sound.md); this page is the per-cue table that page left open.

**The headline: 26 of this port's 43 race cues (`oag_gameplay::cue::Cue::ALL`) have a cue string the
executable posts; 12 of them load and play for every emitter today, plus `NGP_LEACH` for the player
alone.** The other 14 posted cues stop at a switch container on a group whose value is unread (8), an
ATRAC9 frame the decoder rejects (5: `QUAKELAUNCH` and four hits reached through `GAME_VR`'s `No` case)
or an event that starts no sound this loader reads (1, `NGP_LEACH` for non-player emitters; why is
unread). Of the 12 held cues, **9 play and are held** (shield, autopilot, magstrip, shuriken travel,
leach attach, and the Plasma, Rocket, Missile and Quake travel cues through `GAME_VR`; "Held loops"
below), the lock-on stops at a switch container, the death at an ATRAC9 frame, and the engine is unread.
The load report reads `21 of 43 race cues play their Wwise event` (15 before `GAME_VR` was read). The other 5 cues are 4 legacy events the executable never posts and the collision cue, whose
choice is unread.

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
| `PlasmaHitShip` | `NGP_PlasmaHitShip` | `Weapons_NGP` | `0x0138d850`, `FUN_0138c640` | 80 | yes (6; player 5), through `GAME_VR` |
| `MissileHitWall` | `MISSILEEXPWALL` | `Weapons_NGP` | `0x0137dcef`, `FUN_0137d1e0` (with `WO_MISSILE_BOUNCE`) | 80 | yes (1), through `GAME_VR` |
| `SpeedupPad` | `Pad_General` | `Weapons_NGP` | `0x0173bae4`, `FUN_0173b210` | 70 | no: switch container, group `3202674372` (name not found among the executable's strings) |
| `Turbo` | `NGP_Turbo` | `Weapons_NGP` | `0x0173c679` `FUN_0173be50`; `0x012b03b9` `FUN_012afea0` | 70 | no: switch container, group `3167274579` (name not found) |
| `ShieldActive` | `shieldactive` | `Speech_NGP` | `0x0133e335`, `FUN_0133e2a0` (with `_SHIELD`) | 70 | no: switch container, group `VoiceReg__TeamName` |
| `Engaging` | `autopilot_eng` | `Speech_NGP` | `0x017ba52a` `FUN_017ba150`; `0x013159fe` `FUN_01315000` | 70 | no: switch container, group `VoiceReg__TeamName` |
| `Disengaging` | `disengaging` | `Speech_NGP` | `0x013a7888`, `FUN_013a2cd0` | 70 | no: switch container, group `VoiceReg__TeamName` |
| `PlasmaHitWall` | `NGP_PlasmaHitWall` | `Weapons_NGP` | `0x0138c53b`, `FUN_0138c410` | 80 | no: past `GAME_VR`, media `144136475` fails to decode (player's `573010160`) |
| `RocketHitWall` | `ROCKEXPLWALL` | `Weapons_NGP` | `0x013825f3` `FUN_01380870`; `0x01399916` `FUN_013986d0` | 75 | no: past `GAME_VR`, media `845662642` fails to decode (player's `504708181`) |
| `RocketHitShip` | `ROCKEXPLSHIP` | `Weapons_NGP` | `0x01382a6c` `FUN_01380870`; `0x0139994c` `FUN_013986d0` | 75 | no: past `GAME_VR`, media `483130367` fails to decode |
| `Cannon` | `NGP_Cannonfire` | `Weapons_NGP` | `0x01354994`, `FUN_01353df0` (with `CannonBullet`) | 60 | no: switch container, group `GAME_MODE`; the player's event starts no sound this loader reads |
| `CannonHitWall` | `CANNONEXPLWALL` | `Weapons_NGP` | `0x01358bb6`, `FUN_01356b20` | 75 | no: switch container, group `GAME_MODE` |
| `QuakeLaunch` | `QUAKELAUNCH` | `Weapons_NGP` | `0x01368666` `FUN_013684e0`; `0x017babc1` `FUN_017ba150` | 75 | no: media `405228712` fails to decode ("invalid reuse band params", frame 0) |
| `QuakeHit` | `QUAKEHIT` | `Weapons_NGP` | `0x017b6aae`, `FUN_017b1850` | 75 | no: past `GAME_VR`, media `144136475` fails to decode (player's `483130367`) |
| `ShurikenLaunch` | `SHURIKEN` | `Weapons_NGP` | `0x0139eefd`, `FUN_0139ea70` (with `_SHURIKENTRAVEL`) | 75 | no: switch container, group `814711502` (name not found) |

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

## Held loops (lane `omega-wwise-held-loops`, 2026-11-12)

Omega spells a held cue with a leading `_` (`SoundGroup_PlayCue` turns Pulse's `~` into `_`) and passes a
handle slot: the request's address is stored in the owner (the shield's at ship `+0x5f10`, the magstrip's
at `+0x5f30`), the post's flags are `0x100001` (an end-of-event callback), and `SoundManager_Update`
(patch `0x01780190`) writes the playing id `PostEvent` returns to the request's `+0x5c` and sets its `+0x7c`
byte while it plays.

### How a held cue stops (confidence 75)

Every release read does the same three steps on the request: find it in its group's list, set flag
`0x10` (a pending post is dropped), and when it has a playing id and `+0x7c` is set, call
`FUN_0109c140(playing_id, 0, 4)`. `FUN_0109c140` queues engine message `0x1f` carrying those three values
(`[msg+8]`, `[msg+0xc]`, `[msg+0x10]`): the shape of Wwise's `StopPlayingID(id, transition ms, curve)`,
and `0, 4` are its default arguments (no transition, the linear curve). Two independent sites:

| Site | What it releases |
| --- | --- |
| `FUN_01339e90` | the magstrip hum: the request at ship `+0x5f30`, then every request on the group at `+0x5f38`, then the group (the same role as the base's `Ship_StopMagstripSound`, `0x01312770`, whose patch address `patch-v107-addresses.md` leaves unresolved: a hypothesis) |
| `FUN_0130b8c0` (the ship's sound update) | the engine's four held requests at `param_2[0xc95]`, `[0xc97]`, `[0xc98]`, ... before posting a new band |
| `SoundManager_Update` (patch `0x01780190`), a group being torn down | every request on the group |

**So a held cue stops with no fade**: the audible end is Wwise's own declick, which this port's
`oag_audio::Mixer::stop` release ramp stands for. No `Stop` action and no `_Stop` event pair is involved
for the 12 (the cannon's `NGP_Cannonfire_LoopStart`/`LoopStop` pair is the exception, not one of this
port's cues). What is not read: whether `FUN_0109c140` is the SDK's `StopPlayingID` by name (its message
type `0x1f` is not checked against the SDK; 75 rests on the argument shape and the call sites).

### What each held event authors

Each held event opens with a kind-`0x0103` action on its own target (a stop scoped to the game object,
read from the action kind's low byte, so a re-post replaces the last), then its `Play`. The loops are
marked by the sounds' and containers' own loop counts ([wwise.md](../../../formats/wwise.md#loops)).
Read over the 1.07 banks (`data08`), each with its `_PC` twin; **Plays** is the race load and
`omega_held_loops_ground_truth`:

| `Cue` | Event | What its `Play` reaches | Plays |
| --- | --- | --- | --- |
| `Shield` | `_SHIELD`, `_SHIELD_PC` (relays) | a layer of three one-shot picks, a layer of five sounds at loop `0` | yes: 8 voices, 5 loop (the player's event: the shield path is the player's) |
| `Autopilot` | `_AUTOPILOT`, `_AUTOPILOT_PC` (relays) | one-shot layers, sounds at loop `0`, a continuous sequence at loop `0`; the player's: loop counts `10` and `2`, a continuous sequence at `200` | yes: 8 voices (player 15), loops held |
| `Magstrip` | `_magstrip01` on `Ship_NGP` (slot 0) | three continuous random containers at loop `0`, two sounds at loop `0`, one one-shot | yes: 6 voices a craft, 5 loop |
| `ShurikenTravel` | `_SHURIKENTRAVEL` | a continuous sequence at loop `0`, a sound at loop `0` | yes: 3 voices, all loop |
| `LeachAttach` | `_NGP_LEACHATTACH` (relay) | a layer of sounds and a one-shot pick, **no loop authored** (13.7 s of media) | yes, once: held as the handle, stopped on release, not restarted |
| `PlasmaTravel`, `RocketTravel`, `MissileTravel`, `QuakeTravel` | `_NGP_Plasmatvl`, `_ROCKETTVL`, `_NGP_MissileTravel`, `_QUAKETRAVEL` | a **switch container** on `GAME_VR` ("The `GAME_VR` state" below), whose `No` case reaches one random pick or layer | yes, through `GAME_VR`: Plasma 1 voice, Rocket 2, Missile 3, Quake 2 (player 1). Rocket's `No` branch authors no loop; Plasma's, Missile's and the player's Quake loops are cut by a `Stop` in the same event and laid out as media (4.45 s, 13.2 to 22.3 s, 7.9 s), so only the non-player Quake loops while held |
| `LockOn` | `_NGP_MissileLock` | a switch container on `VoiceReg__HUDTargetingState` (`3202746158`; the executable keeps the values `NoTarget`, `Tracking`, `Locked`, `LostLock` beside it) | no: the HUD's targeting value is unread |
| `Blowup` | `_NGP_ShipDeath` | a switch container on `GAME_VR` on its first `Play` | no: past `GAME_VR`, media `483130367` fails to decode (player's `573010160`) |
| `Engine` | `~Ship<n>_<band>` | `FUN_0130b8c0` builds `~Ship1_%s` .. `~Ship5_%s` by team (`Feisar`/`Triakis` 1, `Qirex`/`Goteki_45` 2, `Piranha`/`Mirage`/`Harimau` 3, `Auricom`/`Assegai` 4, `AG_Systems`/`EGX`/`Icaras` 5) over bands `LowAcc_%d`, `MidDec_G_%d`, `vHighAcc_%d`, ... and steers `CONTROLLERID_VELOCITY`, `_THRUST`, `_ACCELERATION`, `_ENGINEON` | no: unread. `xpress`/`xpress_low`/`xrelease` are not the loop: `xpress__Ship_NGP` has no action, `xpress_low_PC` one seek, `xrelease` a one-shot pick |

**9 of 12 held cues play.** `GAME_VR` (`2068003018`) unlocked the four travel cues; the death behind it
stops at the ATRAC9 decoder.

What the port does with them (`oag_sound::sfx`): the existing held paths (`TravelVoices`, the magstrip's
per-slot voices, the shield's and autopilot's latches, the leach's held voice) start every voice the event
starts (`Banks::held_voices`), keep them while the game holds the state and stop them on release. A Wwise
title grows the voice pool to HD's 128 (eight craft's magstrip is 48 voices). Chosen, not read: the
travel voices play the non-player event (a projectile's owner is not passed to the sound path), the
magstrip plays the non-player event from every craft, and a continuous random container repeats its first
draw.

## The `GAME_VR` state (lane `omega-silent-cues`, 2026-11-19; confidence 85)

Switch group `2068003018` is `name_hash("GAME_VR")`, its default `1668749452` is `"No"` and its other
case `979470758` is `"Yes"`. It is a **state** group (`group_type` 1), and the game sets it.

Read statically off the 1.07 patch eboot's bytes (`data/extracted/ps4/omega-eu-patch/uroot/eboot.bin`;
the patch program was not in the shared Ghidra project, so with capstone: address = file offset
`- 0x5430 + 0x01000000`, calibrated on the `MISSILEEXPWALL` store at `0x0137dcef` above). The strings
`GAME_VR` (`0x0189e266`), `Yes` (`0x0189e26e`) and `No` (`0x0189e2ba`) are loaded at two sites, and both
pass them to `0x017c9b60`:

| Site | What it does |
| --- | --- |
| `SoundManager_Construct` (patch `0x01784120`), `0x01784949` | `al = [0x02212d2b]` (the headset flag, owned by the `sceHmd*` code, audio-3d.md); `[mgr+0x2b828] = al`; flag set: `0x017c9b60("GAME_VR", "Yes")`, then a `Gameplay_CAMERA` state by a camera index; flag clear: `0x017849bd lea rsi, "No"` then `0x017849e4 call 0x017c9b60("GAME_VR", "No")` |
| `SoundManager_Update` (patch `0x01780190`), `0x017801d8` | the same, only when the flag differs from `[mgr+0x2b828]` |

`0x017c9b60` builds `"Set_State__" + group + "__" + state` on the stack (`movabs rdx, "Set_Stat"`,
`0x017c9b77`; `"e__"`; the group; `"__"`; the state) and posts it as an event on game object 2 through
`0x010937d0`, printing an error string on failure: so on a flat screen the sound manager posts
`Set_State__GAME_VR__No` once, when it is built, before any race cue.

The banks agree from their side. All 36 switch containers on `GAME_VR` in the patch's `data08` banks are
state containers defaulting to `No` with exactly the cases `None`, `No` and `Yes`, and **each `None`
case lists the same child as its `No` case**: a flat screen hears the same branch whether or not the
state was ever set (`crates/formats/tests/wwise_ground_truth.rs`, `patch_data08_banks`,
`switch_game_vr_flat: 36`). The base archives (`data00`, `data05`) have none, and the base eboot has no
`GAME_VR` string: the VR states are the patch's.

Why 85 and not more: two static sites agree with each other and with the banks' authored `None` = `No`,
but the headset flag being clear on a flat screen is inferred from its owner, not traced live (shadPS4
has no memory readout). `0x017c9b60` is not given a name: `names.tsv` replays into the base program,
which has no counterpart (`Set_State__` is in its strings, unreferenced), and the patch program has no
names applied (patch-v107-addresses.md).

**How the port plays it.** `oag_title::WwiseCues::states` carries the values a title holds its state
groups at; Omega's is `GAME_VR = No`. `oag_sound::sfx::wwise_start` plays a switch container on a listed
group by every child of the value's case (the default's when the value has no case), and refuses one on
any other group by name: the default is never played for a group whose value is unread. The countdown
and the event start pass no states and are unchanged.

### The groups still refused

| Group | Cues | What the value is |
| --- | --- | --- |
| `GAME_MODE` (`1071299879`) | `Cannon`, `CannonHitWall` | the game mode; where the executable sets it is unread |
| `VoiceReg__TeamName` (`382442581`) | `ShieldActive`, `Engaging`, `Disengaging` | the player's team, for the announcer's voice; unread |
| `VoiceReg__HUDTargetingState` (`3202746158`) | `LockOn` | `NoTarget`/`Tracking`/`Locked`/`LostLock`, per frame from the HUD; unread |
| `3202674372`, `3167274579`, `814711502` | `SpeedupPad`, `Turbo`, `ShurikenLaunch` | not found by hashing every printable string of the patch eboot |

The names were found by hashing every printable run of the patch eboot with `name_hash`.

## The cues with no post

| `Cue` | Why not played |
| --- | --- |
| `Engine` | **Held, unread.** The engine posts `~Ship<n>_<band>` cues from `FUN_0130b8c0` ("Held loops" above). The other 11 held cues are posted (store sites: `_SHIELD` `0x0133e376`, `_AUTOPILOT` `0x01315911`, `_NGP_MissileLock` `0x01282e3a`, `_NGP_Plasmatvl` `0x0138ba47`, `_ROCKETTVL` `0x01396f5e`, `_NGP_MissileTravel` `0x0137d13e`, `_QUAKETRAVEL` `0x0176749d`, `_NGP_LEACHATTACH` `0x01369e5d`, `_SHURIKENTRAVEL` `0x0139f08b`, `_magstrip01` `0x0132b32d` on slot 0, `_NGP_ShipDeath` `0x0131e922`) and their rows are in "Held loops". |
| `MissileHitShip` | The missile impact (`FUN_0137d1e0`) names only `MISSILEEXPWALL`; which cue a ship hit posts is unread. `MISSILEEXPSHIP__Weapons_NGP` is in the bank, its string is not in the executable. |
| `MissileExpire` (`SHURIKENEXPL`), `ShurikenHit` (`SHURIKENHIT`), `RepulsorHit` (`REPULSORHIT`) | **Never posted.** The bank keeps the legacy event; the executable holds no such string. |
| `Collision` | Posted as `_COLLISIONS_1` or `_COLLSHIELD` (shield up), with `_GRIND`/`_GRINDSHIELD` for scraping, on slot 0 (`FUN_0130b100`, `0x0130b333`, `0x0130b32c`); `_`-prefixed with a kept handle, and the choice between them is unread. |

## Limits this table meets, not fixed here

* **Switch containers** on a group other than `GAME_VR` (8 one-shot cues, the lock-on): refused at play
  ("The groups still refused" above).
* **ATRAC9 frames** (`QUAKELAUNCH` and, past `GAME_VR`, `PlasmaHitWall`, `RocketHitWall`, `RocketHitShip`,
  `QuakeHit` and the death): `oag_music::wem::decode` rejects frame 0 ("invalid gradient boundary",
  "invalid scale factor mode", "scale factor out of bounds", "invalid reuse band params").
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

* **2048, `GAME_VR`: checked, differs.** Neither the base nor the 1.04 Vita executable holds a `GAME_VR`
  or `Set_State` string; 2048 plays `SBlk` banks and has no Wwise state to hold.
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
# [DEBUG] sfx: 21 of 43 race cues play their Wwise event, 21 with a player version
cargo nextest run -p oag-game --run-ignored all omega_held_loops_ground_truth::
# The magstrip held and released in a race (Talons Junction has strips; Tech De Ra, the default, has none):
RUST_LOG=info,oag_sound=trace oag-game --race --mode single_race --autopilot \
  --track 'Data\environments\talons_junction\track.vex' --ticks 2400 \
  --screenshot out.png --dump-audio out.wav data/extracted/ps4
# [TRACE] sfx: held Magstrip opened on slot 2, 6 voice(s)   (all eight craft, 6 voices each)
# [TRACE] sfx: held Magstrip released on slot 1, 5 voice(s) (the one-shot layer has ended)
```
