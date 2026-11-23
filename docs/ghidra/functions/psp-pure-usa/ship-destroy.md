# Pure's destroyed-craft sequence, read against Pulse's

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure, PSP USA), image base `0x08804000` |
| **Subsystem** | ship state machine, wreck model, death effects, destroy camera |
| **Compared with** | Pulse PSP: [`ship-wreck-model.md`](../psp-pulse-usa/ship-wreck-model.md), [`wreck-collision.md`](../psp-pulse-usa/wreck-collision.md), [`camera.md`](../psp-pulse-usa/camera.md) ("The destroy camera"), [`zone-mode.md`](../psp-pulse-usa/zone-mode.md) (`Ship_SetState` case 4), [`screen-flash-callers.md`](../psp-pulse-usa/screen-flash-callers.md), [`shield.md`](../psp-pulse-usa/shield.md) (states 6 and 8) |
| **Related (Pure)** | [`wreck-big-blast.md`](wreck-big-blast.md), [`wreck-collision.md`](wreck-collision.md), [`ship-models.md`](ship-models.md), [`weapon-effects.md`](weapon-effects.md), [`quake.md`](quake.md) (`ScreenFlash_Start`) |

Read 2026-11-23 (lane `pure-wreck-re`), statically, in the shared Ghidra (`/pure/BOOT-psp-pure-usa.BIN`, Pulse
`/pulse/BOOT-psp-pulse-usa.BIN` beside it). Question: Pure's wreck rides Pulse's rule today
(`crates/pure/src/effects.rs`, `hull_wreck` and the wreck triggers, `Origin::InheritedFrom("Wipeout Pulse")`). Per
state, which of Pulse's steps does Pure's own executable do, and what does it do differently?

## How it was read

`Ship_SetState` (`0x08928824`) dispatches through an **8-entry** jump table (`sltiu a0, s1, 0x8` at `0x08928860`) at
`0x08a7a6a0`, whose words are unrelocated offsets (add `0x08804000`). Arms:

| State | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Arm | `0x08928884` | `0x089288fc` | `0x08928974` | `0x089289ec` | `0x08928b4c` | `0x08928cf0` | `0x08928d94` | `0x08928de0` |

Pulse's table has nine (`0x08a7bc18`); Pure has **no state 8** (Pulse's Eliminator respawn). The decompiler cannot
follow either table, so every arm below is read in disassembly. The per-state update is a second 8-entry table at
`0x08a7a680`, dispatched from `0x08926ae4` (`sltiu a1, a0, 0x8` at `0x08926e9c`); its handlers rejoin at
`0x08926f48`:

| State | 4 | 5 | 6 | 7 |
| --- | --- | --- | --- | --- |
| Update | `0x089257b0` | `0x089258b8` (`Ship_UpdateDestroyed`) | `0x08925d7c` | none (the arm calls nothing) |

Pure's entity layout against Pulse's, as these arms use it:

| Field | Pure | Pulse |
| --- | --- | --- |
| state timer | `+0x9a8` | `+0x874` |
| live / hull / wreck model | `+0x9ec` / `+0x9f0` / `+0x9f4` | `+0x8b0` / `+0x8b4` / `+0x8b8` |
| body (its box collider at `body+0x3fc`, Pulse `+0x3bc`) | `+0x8b4` | `+0x794` |
| local player test (`0` = the player) | `+0x488` | `+0x368` |
| `~BLOWUP` handle | `+0xc14` | `+0xcac` |
| collision-FX node list | `+0xbf0` (8) | `+0xc80` (10) |
| current state | `+0xfc` | `+0x8c` |

Strings (`read_memory`): `~BLOWUP` `0x08a7a648`, `EXPLSMALL` `0x08a7a3dc`, `EXPLBIG` `0x08a7a460`, `cont_elim`
`0x08a7a4b0`, `WO_SHIP_FXNODE_EXPLO` `0x08a7a3e8`, `WO_SHIP_DEATH_SPARKS` `0x08a7a400`, `WO_SHIP_FXNODE_BIGEXPLO`
`0x08a7a468`, `WO_SHIP_EXPLOSION_DEBRIS` `0x08a7a480`, `WO_SHIP_EXPLO_SMOKE` `0x08a7a49c`,
`Data\Weapons\explosion_gaseous.vex` `0x08a48700`.

## Names

Already named elsewhere and cited here: `Ship_SetState` `0x08928824` and `Collision_BoxAgainstBox` `0x08824fbc`
([`wreck-collision.md`](wreck-collision.md)), `Ship_SpawnExplosionSmall` `0x08923db4` and `Ship_UpdateDestroyed`
`0x089258b8` ([`wreck-big-blast.md`](wreck-big-blast.md)), `Ship_ReleaseDisruptorFx` `0x08924570`
([`weapon-effects.md`](weapon-effects.md)), `ScreenFlash_Start` `0x0882e648` ([`quake.md`](quake.md)). New:

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x089257b0` | `Ship_UpdateExploding` | 82 | state 4's update (table `0x08a7a680` entry 4): `+0x9a8 -= dt`, at zero `Ship_SetState(5)` and the gaseous flash; Pulse's `0x088404c8` without the flash |
| `0x08925d7c` | `Ship_UpdateWrecked` | 76 | state 6's update (entry 6): counts `+0x9a8` down and plays `cont_elim` at the zero crossing; never leaves the state |
| `0x0885a7d8` | `ShipGaseousFlash_Construct` | 75 | the only loader of `explosion_gaseous.vex`; its two callers are `Ship_UpdateExploding` and `Ship_UpdateDestroyed` (`get_xrefs_to`) |
| `0x0885aad4` | `ShipGaseousFlash_Update` | 75 | vtable `0x08abc760` slot `+0x24`: the scale ease, the grey keyframes, retirement at age `0.5` |
| `0x08846464` | `Camera_SetMode` | 80 | writes the mode at `camera+0x1e0` and Pulse's three framing widths `35.0`, `17.0`, `50.0` for modes 4, 5, 6 |
| `0x088462f8` | `Camera_SetSubject` | 74 | `camera+0x1e4 = craft`, a station off the craft's own `+0xba0` or `0x08845d94`, mode 6, `fov + rand(-10, 20)`; Pulse's `0x0888058c` without the flag argument |
| `0x08845d94` | `Camera_PickStation` | 72 | nearest station aim to the subject with a y-plane filter, then mode 6 and the random field. The decompile shows eight parameters; the call site passes two (`camera`, `&position`) |
| `0x088467f0` | `Camera_UpdateSpectatorView` | 74 | the per-frame switch on `camera+0x1e0` through `0x08a47510`; modes 4, 5 and 6 share `0x088468e0` |

## The table

Verdict: **same** (Pure does Pulse's step, so the inherited rule is right), **differs** (with what), **unread** (address
and recipe). Confidence is static unless the row says live; the rubric caps a decompile-only reading at 84. The state
timings, the swap and row 22 were then run live ("Measured on PPSSPP" below).

| # | State / edge | Step | Pure (address) | Pulse | Verdict | Conf. |
| --- | --- | --- | --- | --- | --- | ---: |
| 1 | 4 entry | state timer | `+0x9a8 = 0.5` (`0x08928c38`) | `+0x874 = 0.5` | same; **live**: 15 stops at 30 Hz (below) | 92 |
| 2 | 4 entry | explosion cue | `~BLOWUP` through `0x089249e0(craft, bank, "~BLOWUP", 0x400, 0)`, handle at `+0xc14` (`0x08928c4c`) | `0x0883e9b0`, same arguments, handle `+0xcac` | same | 80 |
| 3 | 4 entry | Disruptor effect | `Ship_ReleaseDisruptorFx` (`0x08924570`, [`weapon-effects.md`](weapon-effects.md)) | none (Pulse's craft has no such object) | differs: Pure retires a Disruptor effect riding the craft | 80 |
| 4 | 4 entry | a node's effect stopped | the first node of class `0x08a2fab0` under the live model (`0x08a3f4ec` walk), then `0x0886acc0`: state `2`, retires the child at `+0x144` and releases the handle at `+0x158` (`0x08831d80(h, 3)`) | the same walk and `0x089046ec`, the same body, **skipped in modes 8 and `0x12`** (Eliminator) | same (Pure has no Eliminator); the node's identity is unread on both titles (Pure `Craft_Construct` also calls `0x08a2fab0`, at `0x089269f8`) | 72 |
| 5 | 4 entry, player only | HUD and camera hand-off | if `+0x488 == 0`: `0x0881ea68(*0x08a8c9d4)`, a byte `+0x5c = 0`, an object's `+0x2c &= ~6`, the active camera's shake duration `+0xe4 = 0`, subject setter `0x088462f8(camera, craft)`, `+0x2c \|= 6`, `+0x26c = 0`, `Camera_SetMode` `0x08846464(camera, 4)`, a global `+0x21c0 = 4.5` | the same sequence, mode 5, `+0x1a08 = 4.5`, gated off in the Eliminator | same | 78 |
| 6 | 4 entry, player only | destroy camera | mode **4** on Pure is Pulse's mode **5**: `0x08846464` gives 4 -> `35.0`, 5 -> `17.0`, 6 -> `50.0` at `camera+0x268` (Pulse's 5/6/7), and the spectator update `0x088467f0` sends modes 4, 5 and 6 to one arm, `0x088468e0` (table `0x08a47510`). The subject setter picks the station whose aim is nearest (`0x08845d94`, a y-plane filter, Pulse's `Camera_PickStation`), forces mode 6 (Pulse's 7) and takes `fov + rand(-10, 20)`, at least `3` | `Camera_SetSubject(camera, craft, 1)`, `Camera_PickStation`, mode 7, the same random field | same law, mode index one lower; camera fields sit 4 bytes later (`+0x1e0` mode, `+0x1e4` subject) | 78 |
| 7 | 4 -> 5 edge | timeout | `0x089257b0`: `+0x9a8 -= dt`, at or below zero `Ship_SetState(5)` | `Ship_UpdateExploding` `0x088404c8`, the same | same; **live** | 90 |
| 8 | 4 -> 5 edge | gaseous flash | the same update then builds a `0xa0` object (`0x0885a7d8`) at the body's position (`body+0x30`) under the scene root `*0x08858434`: `explosion_gaseous.vex`, billboarded to the camera, scale eased from `0.1` toward `4.0` at `0.05` per 1/60 s step, grey `255, 255, 0` at ages `0, 0.4, 0.5` written as vertex colour `0xff000000 \| grey * 0x10101` (it darkens, alpha stays 255), retired at age `0.5` (update `0x0885aad4`) | none: Pulse's edge only changes state | **differs**: Pure only, ready to wire | 75 |
| 9 | 5 entry | destroyed flag | byte `+0xb95 = 1` (`0x08928cf4`); the per-tick update skips its two `Ship_SetState(3)` routes while it is set | `+0x860 \|= 0x1000` | same role, different storage | 72 |
| 10 | 5 entry | state timer | `+0x9a8 = 1.5` | `+0x874 = 1.5` | same; **live**, 30 Hz stops ([`wreck-big-blast.md`](wreck-big-blast.md), "Read live on PPSSPP") | 85 |
| 11 | 5 entry | **wreck model swap** | inlined `Ship_SelectWreckModel`: if a wreck exists (`+0x9f4 != 0`), bit `4` moves from the hull's flag word to the wreck's, `+0x9ec = +0x9f4`, then `0x08924a6c` (the node gather), then the live model's `+0x2c \|= 2` (`0x08928d00`..`0x08928d88`) | `Ship_SelectWreckModel` `0x0883eb68`, then `\|= 2` | same, same edge (0.5 s after state 4); **live**: the wreck is live from the first state 5 stop | 90 |
| 12 | 5 entry | which wreck | `0x08927dac` loads `%s\Phantom_shipwreck.vex` when the craft's Phantom flag byte is set (and falls back to `%s\Shipwreck.vex` if that file is absent), else `%s\Shipwreck.vex`; **no wreck** when bit `2` (the `VR\Ship.vex` hull) or bit `4` of `*(0x08857580)+0xf8` is set | one `%s\%swreck.vex` per team | differs: Phantom has its own wreck; the session word's bits are unread | 78 |
| 13 | 5 entry | small blast flash and shake | `Ship_SpawnExplosionSmall` (`0x08923db4`): `ScreenFlash_Start(.., 0, body position)`, the player's shake `(0.3, 0.4, camera, 3)` (`0x0883fe14`) | the same flash kind and shake | same | 80 |
| 14 | 5 entry | small blast cue | `Sound_Play(1.0, craft+0xd4, bank, 0, "EXPLSMALL", 0)` for every craft | `EXPLSMALL_PC` through the slot path for the player, `EXPLSMALL` positional for others | differs: no `_PC` variant on Pure | 78 |
| 15 | 5 entry | `WO_SHIP_FXNODE_EXPLO`, `WO_SHIP_DEATH_SPARKS` | gather up to **8** collision-FX nodes of the live (wreck) model, spawn `FXNE` then `DESP` on each, **parented to the locator** (`0x08871e1c`, matrix `0`) | up to 10, the same pair, the same parent | same: every Pure wreck authors 3 to 7 locators (below), so the cap never bites | 82 |
| 16 | 5 entry | after each pair | `0x0893892c(*(psys_mgr+0x4694), *0x08ab4310)` looks a particle instance up by handle ([`particle-bars.md`](particle-bars.md)) and writes `90.0` to its `+0x138` and to `+0x4c` down its `+0x78` chain | none | **unread**: Pure only; read `+0x138`/`+0x4c` in `ParticleSystem_SetScaleParams`' neighbours (`0x0893bf08`) or watch the instance live | - |
| 17 | 5 -> 6 edge | timeout | `Ship_UpdateDestroyed` (`0x089258b8`): `+0x9a8 -= dt`, at zero writes `4.0`, then everything below, then `Ship_SetState(6)` (which overwrites the timer) | `Ship_UpdateDestroyed` `0x08847650` and `Ship_SpawnExplosionBig` `0x088407b0`, the same `4.0` | same timing; **live**, 45 stops at 30 Hz ([`wreck-big-blast.md`](wreck-big-blast.md)) | 85 |
| 18 | 5 -> 6 edge | big blast cue | `Sound_Play(1.0, craft+0xd4, bank, 0, "EXPLBIG", 0)` for every craft | `EXPLBIG_PC` for the player, plus `SHIP_DEST` in mode 6; `EXPLBIG` positional for others | differs: no `_PC` variant, no `SHIP_DEST` | 78 |
| 19 | 5 -> 6 edge | big flash and shake | player: `ScreenFlash_Start(.., 6, body position)`, shake `(0.8, 0.6, camera, 1)`; others: kind `0` | player: kind `7`, the same shake; others kind `0` | shake same; flash kind **unread**: Pure's table has 10 kinds (`0x08a46a78`), Pulse's 12, so compare Pure's entry 6 with Pulse's entry 7 before calling it a difference | 75 |
| 20 | 5 -> 6 edge | `WO_SHIP_FXNODE_BIGEXPLO` | for each of up to 8 locators: refresh its world matrix if dirty (`0x0889353c`), spawn `FXBI` **parented to the particle world root** (`*0x08ab4304`), placed by the locator's world matrix (`a3 = node+0x30`, disassembly `0x089259fc`..`0x08925a94`) | none | **differs**: Pure only, already wired as measured (`Trigger::WreckBigNode`); the fireballs sit in the world, they do not ride the wreck | 80 |
| 21 | 5 -> 6 edge | gaseous flash | the same `explosion_gaseous.vex` object as row 8, at the body's position (`0x08925b54`) | `Bomb_Shockwave.vex` ring (`ShipShockwave_Construct`, [`ship-shockwave.md`](../psp-pulse-usa/ship-shockwave.md)) | **differs**: Pure throws the gaseous flash, not a shockwave ring; ready to wire | 75 |
| 22 | 5 -> 6 edge | the wreck vanishes | the live model's matrix (the wreck since row 11) is scaled by `1e-4` (`0x089233f0`, `0x38d1b717`) and written (`0x08893f64`), then `Ship_SetState(6)`; the per-tick update writes the live model's matrix only while `+0xfc != 6` (`beql a0, s4, 0x08927270` at `0x08926f48`, `li s4, 0x6` at `0x08926b54`, the write `jal 0x08893f64` at `0x08927264`), so **the squash stands and the wreck is not drawn from state 6 on** | the same squash in `Ship_SpawnExplosionBig`, and the same gate in Pulse's per-state update `0x088418e0`: `li s1, 0x6` at `0x0884193c`, `beq a0(+0x8c), s1, 0x0884226c` at `0x08841e50` skips the live model's matrix write (`jal 0x08945284` at `0x08842264`). Pulse's capture row for state 6 is the transition stop, so it does not show otherwise | **same as Pulse, and both differ from ours**: `oag_raceplay::scene::wreck` draws the wreck for the whole of `Eliminated` on every title; ready to wire (hide it at the big blast). **Live** on Pure: the row stays `7.5e-05` long (`0.75 * 1e-4`) for every state 6 stop logged, and the frames show the spot empty; Pulse's half is static | 90 |
| 23 | 5 -> 6 edge | `WO_SHIP_EXPLOSION_DEBRIS`, `WO_SHIP_EXPLO_SMOKE` | spawned at the live model's world matrix (`model+0x80`, copied before the squash), parented to the world root, **no drop** along `-up` | one `WO_SHIP_EXPLOSION`, moved `4.0` down the model's up row | differs, already wired as measured (`Trigger::WreckDebris`, `WreckSmoke`) | 80 |
| 24 | 6 entry | state timer | `+0x9a8 = 2.0` for the player, `0.8` otherwise (`0x08928d94`..`0x08928dc0`) | the same | same; the player's `2.0` **live** (below, and [`wreck-big-blast.md`](wreck-big-blast.md)) | 90 |
| 25 | 6 entry | **wreck collision** | `sb zero, 0x68(*(body+0x3fc))` at `0x08928dcc`: the wreck stops colliding with craft | the same store at `0x088445d0` | same (states 4 and 5 solid, 6 and 7 not, [`wreck-collision.md`](wreck-collision.md)) | 82 |
| 26 | 6 entry | cue released | `0x08835ea0(+0xc14)` | `0x08991d94(+0xcac)`, then `Ship_MagFloorExit` | same release; no mag-floor exit, Pure has no mag floor ([`magfloor-absent.md`](magfloor-absent.md)) | 75 |
| 27 | 6 update | expiry | `0x08925d7c`: at the zero crossing plays `cont_elim` (`0x088308f0(bank, .., "cont_elim", 0x400, 0)`) unless mode 2 (with `0x08a8c4b3` clear); nothing leaves state 6 | `cont_elim` in modes other than 2, 8 and 18; a single race's wreck stays down (measured) | same (Pure has no modes 8 and 18 here) | 78 |
| 28 | 7 entry | | the collider byte only (`0x08928dec`); no update | the collider byte plus a mode-dependent destroyed bit and `0x088407b0` | differs; Pure's callers of state 7 are unread (`get_xrefs_to 0x08928824`, look for `li a1, 7`) | 70 |

### The locator census behind row 15

`Ship Collision Fx` nodes (class `0x382`) per wreck, counted with `oag_vex::vex::class_world_transforms` on the files
extracted from `pure-psp-usa.chd` (`Data.wad`):

| Team | `Shipwreck.vex` | `Phantom_shipwreck.vex` |
| --- | ---: | ---: |
| AG_Systems | 5 | 5 |
| Assegai | 5 | 4 |
| Auricom | 6 | 4 |
| Feisar | 6 | 4 |
| Harimau | 6 | 3 |
| Piranha | 7 | 3 |
| Qirex | 7 | 4 |
| Triakis | 6 | 4 |
| Medievil | 6 | - |
| Zone | 5 | - |
| Zone_01 | 5 | - |

The maximum is 7, under Pure's cap of 8 and Pulse's of 10.

## What this says about the inherited rule

- **Same, so the inheritance is right (move to measured):** the state chain 4 (0.5 s) -> 5 (1.5 s) -> 6 (2.0 s / 0.8 s,
  stays), the wreck swap on the state 5 edge (row 11), `WO_SHIP_FXNODE_EXPLO` and `WO_SHIP_DEATH_SPARKS` on the wreck's
  locators at that edge (row 15), the destroy camera (row 6), and the wreck's collision (row 25).
- **Differs, ready to wire:** the gaseous flash at the 4 -> 5 edge and again at the big blast in place of the shockwave
  ring (rows 8, 21), the Phantom wreck (row 12, once a Phantom hull is loaded at all), and the sounds (rows 14, 18).
- **Same as Pulse, missing from ours on both titles:** the wreck hidden from the big blast on (row 22).
- **Unread:** row 16 (`90.0` on the death sparks' instance), row 19's flash kind, row 28's callers, row 4's node class.

## Wired (2026-11-23, lane `pure-wreck-wire`)

| Rows | What | Where |
| --- | --- | --- |
| 1, 7, 10, 24 | the timers and edges: `DestroyedCraft::PURE` is **measured** (was inherited from Pulse) | `oag_title::destroyed`, `crates/pure/src/lib.rs` |
| 11, 15 | `hull_wreck`, `WreckNode`, `WreckSparks` are **measured** (were `InheritedFrom`) | `crates/pure/src/effects.rs` |
| 8, 21 | `explosion_gaseous.vex` thrown at the body on the 4 -> 5 edge and again at the big blast, in place of the shockwave ring (Pure has no `WreckExplosion`, so the ring was already absent). A billboard in the bomb-blast pool (`BlastKind::Gaseous`): scale `0.1` toward `4.0` at `0.05` a tick, grey `255, 255, 0` at `0, 0.4, 0.5` s (linear between, as a multiplier of the vertex colours; the original writes the colour outright), retired at `0.5` s. Loaded through `DestroyedCraft::gaseous_flash`; a disc without it draws nothing and the loader report says so | `bomb_blast.rs`, `craft_flash.rs` |
| 14, 18 | `EXPLSMALL` at state 5 and `EXPLBIG` at state 6 on every craft, no `_PC`, no `SHIP_DEST`; out of the title's ship bank (`DestroyedSound::BlowupAndBlasts`, `Cue::ShipExplosionSmall`, `Cue::ShipExplosion`). Pulse voices neither | `destroyed_cues.rs`, `load/audio.rs` |
| 22 | the wreck is not drawn from the big blast on, **Pulse and Pure** (`DestroyedCraft::wreck_hidden_at_blast`; HD keeps its wreck in view). View state set on the tick the blast fires, so a world standing still under the results (a Zone wreck, a solo headless run) hides it too | `craft_flash.rs`, `scene/wreck.rs` |
| 26 | `~BLOWUP` is released at state 6 on both titles; `craft_is_exploding` still ties it to state 4's `0.5 s` | not wired |

Not wired: row 12 (the Phantom wreck): ours never loads Pure's `Phantom.vex` hull (what selects it is a synced
ad-hoc network byte, unread), so there is no Phantom craft to wear `Phantom_shipwreck.vex`; rows 16, 19, 28 stay unread.
`ship_models_ground_truth.rs` already asserts the eight teams ship the Phantom wreck. The disc check for the rest is
`pure_race_ground_truth::a_pure_race_loads_its_destroyed_craft_pieces`. Checked against Pulse: rows 8 and 21 are Pure
only; row 22 and the hide are Pulse's static reading (`0x08841e50`) wired for both.

## Measured on PPSSPP (2026-11-23)

PPSSPP 1.20.4, software renderer, Pure USA, the Vineta Time Trial state
`data/saves/pure-psp/time-trial-vineta-vector-feisar-restart.ppst`, the player's Feisar wrecked with
`Ship_SetState(entity, 4)` from the stop `0x08926ae4` (`scripts/psp-pure-wreck-states.py`). One stop is one 30 Hz
game tick (`dt` read back as `0x3d08abd6`, `0.0334`).

| Stops since the call | State | Timer | Live model | `+0xb95` | Live row length | Live flag word |
| ---: | ---: | --- | --- | ---: | --- | --- |
| 0 | 4 | `0.5` | hull | 0 | `0.749986` | `0x5e026` |
| 14 | 4 | `0.0331` | hull | 0 | `0.749986` | `0x5e026` |
| 15 | 5 | `1.5` | **wreck** | 1 | `0.749986` | `0x5e026` |
| 60 | 6 | `2.0` | wreck | 1 | **`7.5e-05`** | `0x105e026` |
| 61 to 110 | 6 | falling `0.0334` per stop | wreck | 1 | `7.5e-05` | `0x105e026` |

So state 4 lasts **15** stops (0.5 s), state 5 **45** (1.5 s), and the wreck swap lands on the first state 5 stop, as
read. The wreck takes the hull's whole flag word, bit `4` set, as on Pulse; state 6 adds `0x1000000`, the bit Pulse's
capture also saw and left unread. From the big blast on, the wreck's matrix row is the squashed `0.75 * 1e-4` and
stays so: nothing rewrites it in state 6. The frames agree: at 40 stops (state 5) the destroy camera frames a burning
wreck far down the straight; at 75 and 85 stops (state 6) the flash and debris fill the screen and the wreck's spot is
empty.

**Trap:** `Ship_SetState` loads its timer constants through `f12`, which is also the update's `dt` argument. A first
run that restored only the integer registers after the call fed the next update `dt = 0.5` and state 4 lasted one
stop; the script restores `f12` too (`f12` read back unchanged, `0x3d08abd6`).

Confidence, rows 1, 7, 11, 22, 24: one live run, one team, one circuit. The timings are also measured on Pulse; row 22's
Pulse half is static, so it stays at 90.

## Not read

- The gaseous object's draws: the GE census in [`wreck-big-blast.md`](wreck-big-blast.md) saw an unidentified
  103-vertex additive mesh of about 21 units at the blast; `explosion_gaseous.vex`'s vertex count would confirm it.
- The AI's `0.8` s path in state 6 (a Time Trial has no rivals; `pure-single-vineta-feisar-race.ppst` does).
- What selects a Phantom craft, and the session word at `*(0x08857580)+0xf8` (row 12).
