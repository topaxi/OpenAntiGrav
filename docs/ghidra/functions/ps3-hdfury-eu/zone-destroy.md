# The Zone end: `Ship_SetState` modes 6 (and 14) on a destroyed craft

| | |
| --- | --- |
| **Binary** | `PS3_GAME/USRDIR/EBOOT.elf` (HD/Fury, BCES-00664), Ghidra `/hdfury/EBOOT-ps3-hdfury-eu.elf` |
| **Twins** | 2048 `/2048/eboot-vita-2048-eu-v104.elf` (`Ship_SetState` `0x811c711e`), Omega `/omega/eboot-ps4-omega-eu.bin` (`Ship_SetState` `0x012ff700`) |
| **Subsystem** | ship state machine (Zone branch), death shell, Zone debris, Zone end camera, HUD, race end |
| **Related** | [ship-destroy.md](ship-destroy.md) (states 4 to 8 in an ordinary race; this page reads its rows 3 and 12's "modes 6 and 14"), [zone-craft.md](zone-craft.md), [detonator-hull.md](detonator-hull.md) (the death shell slot `+0x6afc`), [hud-start-gate.md](hud-start-gate.md) (`SPZone_Update`'s phase table), [sound.md](sound.md) (the environment bank), [camera.md](camera.md); Pulse's Zone end: [zone-mode.md](../psp-pulse-usa/zone-mode.md), [zone-rest.md](../psp-pulse-usa/zone-rest.md) |

Read 2026-11-23, `zone-destroy-re`. Static only. Method: the jump-table arms Ghidra does
not disassemble were read with capstone over the ELF, every TOC load resolved through
`scripts/ps3-toc.py` against the function's own TOC (`0x008ad4d8` for every HD function
here); the jump tables were decoded by hand. Live check: not run (see "Measured on
RPCS3: not run"). Wired: see "Wired" below.

**Mode 14 is Detonator** ([mode-manager.md](mode-manager.md), settled 2026-08-30), so the
table below is scoped to **mode 6, Zone**. Modes 6 and 14 share the `0x4040` branches of
`Ship_SetState`; where Detonator diverges it is listed under "Mode 14 diverges".

**Answer in one line.** A Zone craft never becomes a wreck: at the blast the hull stays
drawn, the bank cue `shipdestroy` plays after every other cue is stopped, the camera
holds a trackside view on the craft (mode 2); for 2.5 s the white death shell
(`zoneship_dead_whiteshell.vex`, drawn over the hull all race) plays its animation from
`0` to `1`; then the hull, flare and shell are hidden and 112 pieces of
`zoneship_dead.vex` fly off with the craft's velocity. The race hides the HUD on the
blast and opens `EndRace Results` 1.0 + 4.5 s later.

## What the wiring lane needs to know first

- **A Zone race on HD never selects `+0x6ae4` (`shipwreck.vex`).** `Ship_SelectWreckModel`
  is skipped for modes 6 and 14 (`0x000e22b4`, `0x000e22bc` branch round `0x000e22c8`). The
  Zone `shipwreck.vex` that `zone-wreck-livery` builds is loaded by the original
  (`Ship_ReloadModelForSkin` stores it at `+0x6ae4` for every mode) but never drawn in a
  Zone race. The Zone end's models are the **death shell** and the **debris model**.
- The Zone end draws no `WO_SHIP_EXPLOSION`, no death plume, no `EXPLBIG`, no
  `~SHIP_DESTROYED`, no `~SHIP_ONFIRE`, no camera shake, no collider swap and no death kick
  (all of ship-destroy.md rows 1, 2, 3, 4, 6 and 10 are skipped in mode 6; addresses per row
  below).
- The three Zone-end models ship on all three titles:

| Title | Death shell | Debris model | Where |
| --- | --- | --- | --- |
| HD/Fury | `/data/ships/zone/zoneship_dead/zoneship_dead_whiteshell.vex` (13,376 B) + `.rcsmodel` | `/data/ships/zone/zoneship_dead/zoneship_dead.vex` (42,336 B) + `.rcsmodel` | `DATA02.PSARC` |
| 2048 | `data/art/published/hdships/Zone/Zoneship_dead/Zoneship_dead_WhiteShell.vex` (27,024 B) + `.rcsmodel`, `.ma.xml` | `.../Zoneship_dead/Zoneship_dead.vex` (41,696 B) + `.rcsmodel` | `data.psarc` (PCSF00007 base) |
| Omega | `Data/art/published/hdships/zone/Zoneship_dead/Zoneship_dead_WhiteShell.vex` (27,024 B) + `.rcsmodel`, `.ma.xml`; VR copy under `HDShips/Zone_VR/` | `.../Zoneship_dead/Zoneship_dead.vex` (41,696 B) + `.rcsmodel` | `data01.psarc`, `data08.psarc` (merged tree) |

Census: `scripts/psarc.py list` on the decrypted HD image, the extracted 2048 base and
every archive of the Omega merged tree. Detonator's own shell is
`ship_deathshell.vex` on all three.

## The model slots

Read off `Ship_ReloadModelForSkin` (`0x000dbdc8`, [renderer.md](renderer.md)) and a raw
scan of the text for `0x6aXX(rN)` loads and stores.

| Slot | Model | Loaded | Evidence | Conf. |
| --- | --- | --- | --- | ---: |
| `+0x6adc` | the live model | | ship-destroy.md | 88 |
| `+0x6af0` | `%s\locators.vex` | every mode | `0x000dc294`-`0x000dc2d0` | 85 |
| `+0x6af4` | `%s\engineflare.vex` (0 if absent) | every mode | `0x000dc2cc`-`0x000dc2fc`, `0x000dcaec` | 85 |
| `+0x6afc` | the death shell: `%s\Zoneship_dead/zoneship_dead_whiteshell.vex` in mode 6 (`0x007822c0`), `ship_deathshell.vex` in mode 14; else 0 (`0x000dc28c`) | modes 6, 14 | `0x000dd4e0`-`0x000dd5d4` | 85 |

The shell is made **visible at load**: `SceneNode_SetFlags(shell, 4)` (`0x000dd548`;
`0x00683048` ORs its argument into `+0x34`, bit `4` the visible bit
[ship-destroy.md](ship-destroy.md) row 1 moves), its animation time is set to `0.0`
(`AnimNode_UpdateTransformTree(shell, 0.0)`, `0x000dd574`) and in mode 6 its material
parameters are bound through `0x000ddc48` (not read). So the white shell is drawn over the
hull for the whole race, its look carried by its animation time. While racing that time is
driven by `Ship_UpdateEntity` (`0x000eb324`-`0x000eb3e0`, `0x000ed300`): in state 0
`clamp(tuning+0x138 - ship+0x6a30, 0, 1)` (`+0x6a30` the craft's age, `+= dt` at
`0x000eaea4`), in states 1-3 and 7-9 `+0x6ad4`, which decays by `0.5 dt` to 0 and is raised
by `0.1 x f27` at `0x000ec934`-`0x000ec96c` (beside setting `+0x6a64` bits `0x400020`) and set to `1.0` by case 3 (`0x000e253c`);
states 4, 5 and 6 skip it. That in-race flash is outside this lane and is **65** (one
reading, its trigger at `0x000ec928` unread).

## The ready-to-wire table (mode 6)

**Ready to wire:** Z1, Z3-Z9 and Z12 fully; Z2 (camera mode 2's per-frame view, what the player
watches for 5.5 s), Z10 (the side calls `0x000f9610` and `0x003f5878`) and Z11 (the launch velocity)
each with one unread value and its address under "Open".

Role is `ship+0x628c` (`0` local, `2` AI, `1` remote). A Zone race has the one local craft.
"Shared" cites ship-destroy.md's row that runs unchanged in mode 6.

| # | State, when | What happens | Address | Conf. | 2048 | Omega |
| ---: | --- | --- | --- | ---: | --- | --- |
| Z1 | 4, on entry | **Shared**: `+0x6f78 += 1`; destroy camera prologue (HUD node `*0x009879dc` `+0x34 &= ~7`, `Camera_SetSubject(cam, ship, 1)`, `cam+0x34 \|= 6`; skipped if the camera is already in mode 2); single player: `0x000884d8(g_Hud)` (frees `g_Hud+0x98..+0xa8`, unread what they hold), `*0x00abc170 + 0x5c = 0`, `RaceManager+0x2dc4 = 4.5` (the race end's second wait, Z9); the `0x1000` out-of-race bit and `0x000d0128` (ship-destroy.md rows 5, 8) | `0x000e20e8`-`0x000e2244`, `0x000e2c6c`-`0x000e2c9c`, `0x000e2410`-`0x000e2420` | 85 | checked, applies, not wired: the same prologue, `0x131ac = 4.5` (`0x40900000`), `+0x60a4 \|= 0x1000` | checked, applies, not wired: same prologue, `+0x1aff0 = 4.5`, `+0x71f5 \|= 0x10` |
| Z2 | 4, on entry | **Camera mode 2, not 8**: for modes in `0x206040` except 13 and 21: `ship+0x6ad4 = 0` (the shell's in-race clock), `Camera_SetTarget_q(cam, cam+0x1ec)` (`0x000a9b18`, `cam+0x1f0 = craft`), `Camera_SetMode(cam, 2)`. Mode 2's enter, `Camera_EnterTrackView_q` (`0x000aa090`): stores the craft's body position (`body+0x200`) at `cam+0x390`, clears `cam+0x48` bit 2, and either reuses its stored station (`cam+0x3a0 > 0`: `+0x2a4 = +0x3a4`, `+0x2a0 = +0x3a8`, field `+0x288 = 35.0`, `+0x22c = +0x230 = 65.0`) or looks one up in `"AI track data"` within `10000.0` (`0x000a97f0`, read from the disassembly only: Ghidra marks `0x000a97f0` no-return and truncates the decompile). **The per-frame law of mode 2 is unread**: next read is the camera's per-frame twelve-way switch on `cam+0x40`, entry 2. | `0x000e2248`-`0x000e2298`, `0x000abe20`, `0x000aa090` | 75 (the enter: 65) | checked, applies, not wired: `+0x6124 = 0`, `FUN_81218884(cam, cam+0x1e8)`, `FUN_8121963a(cam, 2)` for the Zone family except 13 and 21 | checked, differs: mode 2 for modes 6, 13, 14, 21 (`0x8181 >> (mode-6)`) **or** when `FUN_01669270` (behind `DAT_0203a728`) or `*(DAT_01e31910+0x21c) == 0`, except 13 and 21; `+0x7278 = 0`; `FUN_01382720(cam, 2)` |
| Z3 | 4, on entry | **No wreck, no blast**: the live model gets `+0x34 \|= 2` (as in every mode) and the mode-6/14 branch jumps over `Ship_SelectWreckModel`, `Ship_PlayDestroyExplosion`, the announcer, `Ship_ApplyDeathKick_q`, `~SHIP_ONFIRE`, the collider swap and the inertia (ship-destroy.md rows 1, 2, 3, 6) | `0x000e22a0`-`0x000e22bc` -> `0x000e296c` | 88 | checked, applies, not wired: the `(iVar4 == 6 \|\| iVar4 == 0xe)` branch skips `FUN_811c6f72`, `FUN_811bc2bc`, `cont_elim`, `~SHIP_ONFIRE` | checked, applies, not wired: `(mode \| 8) == 0xe` skips `FUN_01301260` and the explosion |
| Z4 | 4, on entry | **Rumble**: local craft `Rumble_QueueOnPad(pad, 0x3c)` (`Ship_Exploding_Player_Rumble`); no `0x3b` broadcast | `0x000e2974` -> `0x000e2ca8` | 85 | checked, differs: no rumble call (Vita) | checked, applies, not wired: `FUN_0174a880(.., 0x3c)` for a local craft |
| Z5 | 4, on entry | **Sound**: `+0x6a78 = 0`; `Sound_StopAllBankCues_q(*0x00b6cc28)` (`0x002ff600` -> `0x00315c58`: for every loaded bank, stop every playing instance of every cue, `0x00315598`), then `Sound_PlayBankCue(*0x00b6cc28, *0x00b6cc50, "shipdestroy", 0x400, 0...)`, dry (no emitter). `*0x00b6cc50` is the environment bank, live-read as `env0_zone.bnk` in a Zone race ([sound.md](sound.md), conf 90); **`shipdestroy` is in `env0_zone.bnk` and `env0_det.bnk` only** (strings census of `DATA00`'s banks; `weapons.bnk` has `EXPLBIG*`, `~SHIP_*`, `~BLOWUP`, not it). Its only call site in the binary is this one (`attrib 0x00782538`). | `0x000e2978`-`0x000e29c4` | 85 | checked, applies, not wired: `FUN_812641f0(mgr)` then `"shipdestroy"` at volume 1.0; 2048's `data/audio/sound/env0_zone.bnk` names it | not checkable by census: posted by name through `FUN_01779800(2, .., "shipdestroy")`; no `.bnk` in the merged tree carries its FNV-1 id `0xb26629e7` (nor `ShipDestroyed_*`/`EXPLBIG`; the hash code checks out against `env0_zone`'s bank id `0xe5044d7b`), so the event's container is unread |
| Z6 | 4 -> 5 | **One tick**: Z5's `+0x6a78 = 0` overrides case 4's `0.3`, so `Ship_UpdateExploding` enters state 5 on its first update; the 4 -> 5 edge's debris call (`0x00104f18`) is skipped in the `0x206040` modes | `0x000e2980`, `0x000e3d18` | 85 | checked, applies, not wired: `+0x60b8 = 0` | checked, applies, not wired: `+0x7208 = 0` |
| Z7 | 5, on entry | Timer `+0x6a78 = 2.5` (`0x40200000`, mask `0x4040`); no death plume | `0x000e27ac`-`0x000e27c4` | 88 | checked, applies, not wired: case 6/`0xe` `+0x60b8 = 2.5` | checked, applies, not wired: `0x4040` -> `2.5` |
| Z8 | 5, per tick | **The shell animates**: `Ship_UpdateWrecked`'s mode switch (table `0x000e394c`, entry 6 -> `0x000e3a38`) calls `AnimNode_UpdateTransformTree(+0x6afc, 1 - max(0.4 * timer, 0))` with `timer` the remaining `+0x6a78` (so `0` at entry, `1` at 2.5 s), then takes the common settle test with the respawn flag clear and the "leave on the timer" flag set: at `timer <= 0` it zeroes the timer and calls `Ship_SetState(6)` (`0x000e38b4` -> `0x000e390c` -> `0x000e3c48` -> `0x000e3864`; the decompile drops this exit as unreachable). Without a shell the arm falls to the ordinary 6 s/20 s test. | `0x000e3a38`-`0x000e3a78`, `0x000e3c48` | 82 | not read: the arm is `Ship_UpdateWrecked`'s twin; find it by the `0.4` literal (`0x3ecccccd`) beside the 2.5 s countdown | ported law, other code: `Ship_UpdateWrecked` `0x013080f0` case 6: shell (`+0x72c8`) `+0x60 \|= 4` each tick, then the same `1 - max(0.4 t, 0)` (`FUN_0170e860`); **checked, differs** in that Omega sets the visible bit here, HD at load |
| Z9 | race, on the blast | **The race ends**: `SPZone_UpdateRacing` (`0x00076548`), in sub-state 1, sees the player craft's `+0x6a64 & 0x1000` (Z1) and calls `0x00087d40(g_Hud)` (once, latched by `g_Hud+0x160`: `g_Hud+0x34 &= ~6`, frees `+0x98..+0xa8`, `+0x67c = k`), `RaceManager_SetPhase(rm, 3)` (zeroes `+0x1974`, `+0x196c`, `+0x1968`: `0x00056558`-`0x0005657c`, reached by every phase-3 path), `rm+0x2de0 = (short)(X+0x130) - rm+0x2de0`, `0x0005c630(rm, 0)`. `SPZone_UpdateFinish` (`0x000758a0`, phase 3 of `SPZone_Update`'s table): sub 0, once the phase time `+0x196c` (advanced by `dt` in `RaceManager_Update` at `0x0005ebc8`) reaches **1.0**: HUD node `*0x009879dc` `+0x34 &= ~6`, sub 1 (timer zeroed, `RaceManager_SetSubState` `0x00053fc0`); sub 1, at `rm+0x2dc4` = **4.5** (Z1): if `!rm+0x2d2c` `0x00055670(rm)`, then `0x00058ab0(rm, "EndRace Results")` and `SetPhase(4)`. So the results screen is pushed **5.5 s after the blast** (plus the tick or two between Z1 and the racing update). Live corroboration of the HUD half: [hud-start-gate.md](hud-start-gate.md)'s RPCS3 Zone run reads `g_Hud+0x34` `0x5e006` racing and `0x5e000` in phase 3, the two bits `0x00087d40` clears. | `0x00076620`-`0x00076700`, `0x000758a0`, `0x00053fc0`, `0x00053ff0` | 80 (HUD bits 85) | checked, applies, not wired: the same `4.5` write in case 4 (`0x131ac`); 2048's race-manager side not read | checked, applies, not wired: `+0x1aff0 = 4.5`; race-manager side not read |
| Z10 | 6, on entry | **The craft comes apart** (mode 6, roles 0 and 2; `0x000e1fec` -> `0x000e2cc8`): `+0x6a78 = 0`; visible bit `4` cleared on the live model, `+0x6af4` (engine flare) and `+0x6afc` (shell, if loaded); `Rumble_QueueOnPad(pad, 0x3c)` again; `DebrisManager_SpawnZoneDebris(+0x7e40, body matrix body+0x1d0, body velocity body+0x190, 0x70)`; `0x000f9610(*0x00992ce0 + 0x2d0)` (byte `+0x104 = 1`, `+0x108 = 0.0`, unread); `0x003f5878(body+0x200, tuning+0xbc, 0)` (a 24-slot positional pool, record `+0x48 = 0.0`, `+0x4c = 3.0` statically, unread: its readers through `*0x008b819c` decide whether it is a flash, a light or a force); then **shared** ship-destroy.md row 12: `*(body+0x4cc)+0x70 = 0`, `PhysicsWorld_RemoveBody`, stop `~SHIP_DESTROYED`, free the emitter | `0x000e1fd0`-`0x000e1ff0`, `0x000e2cc8`-`0x000e2da0`, `0x000e2714` | 85 | checked, differs: same hides (`+0x612c`, `+0x6148`, `+0x6150`), `FUN_811a468a(+0x73f0, body+8, body+0x90, 0x70)` **only if the shell loaded**, `DAT_8151fe58+0x3d0 = 1`, `+0x3d4 = 0`, `FUN_81031ae2(body+0x38, ..)`; no rumble; modes 6 and 14 take the same arm | checked, differs (as 2048, the debris is gated on the shell): same hides (`+0x7280`, `+0x72b8`, `+0x72c8`), rumble `0x3c`, `FUN_012cf870(+0x8650, body+0x10, 0x70)` **only if the shell loaded**, `DAT_01a11f28+0xc7c = 1`, `+0xc80 = 0`, `FUN_01606280(body+0x40, ..)` |
| Z11 | 6 | **The debris**: `DebrisManager_Construct` (`0x00105a48`, `DebrisManager.cpp`), built with the Zone flag, allocates **112** (`0x70`) `0x170`-byte debris objects and loads `Data/Ships/Zone/Zoneship_dead/zoneship_dead.vex` into `mgr+0x40` (hidden, `&= ~4`), then collects 112 child nodes of one class into `mgr+0x14c..+0x30c` (`0x00685f38`); `mgr+0x30c` records whether they were found. `DebrisManager_SpawnZoneDebris` (`0x00104a58`; prints `"Ship has no zone debris to spawn"` when not loaded) releases the previous set, makes `mgr+0x40` visible at the body matrix and, per piece, builds a launch record (node, position, velocity from the body plus a random term from `Libc_Rand`) and calls `Debris_Launch` (`0x0010c958`): body type `7`, `Body_SetBoxInertia`, added to the physics and collision worlds (`+0x70 = 1`, so the pieces collide), the node made visible. The record's `+0x58` byte is **0** for Zone pieces (`stb r21, 0x108(r29)`, `r21 = 0`, record base `0x00992d10 + 0xb0`, `0x00104d48`), and `Debris_Launch` attaches `WO_DEBRIS_FIRE` (fourcc `0x46424544`) only when it is set (`lbz r0,0x58(r30)` at `0x0010cba0`): **Zone debris carries no fire**. Ordinary ships build 19 `Ship_Damage%d` pieces (32 for `Ship_Damage_Nitro`) instead. The per-piece velocity formula is read from the disassembly only (Ghidra treats `0x00327500` as no-return and truncates `0x00104a58` and `0x0010c958`). | `0x00105a48`, `0x00104a58`, `0x0010c958` | 78 (velocity: 55) | checked, applies, not wired: `FUN_811a468a` with `0x70` | checked, applies, not wired: `FUN_012cf870` with `0x70` |
| Z12 | 6 onwards | A single-player wreck stays in state 6 (ship-destroy.md row 13): nothing of the craft is drawn after Z10 but the debris; the camera holds mode 2 | `0x000ed018` | 75 | not read | not read |

**Whether the craft still ticks in phase 3.** Z8 and Z10 need `Ship_UpdateEntity`
(`0x000eadb8`) to keep running after `SetPhase(3)`. It has no direct `bl` caller (raw
scan): it is reached through a virtual, not from `RaceManager_Update`, so nothing in the
race manager's phase switch gates it. Not proven live; **70** for "the debris flies at
2.5 s, 3 s before the results screen".

## Wired (2026-11-23, `zone-destroy-wire`)

`oag_title::destroyed::ZoneEnd` (HD, 2048 and Omega; Pulse and Pure `None`) carries the timers
and the model stems; `oag_livery::zone_end` loads the shell and the debris; the view clock and
the drawing are `oag_raceplay::zone_end` and `scene/zone_end.rs`. Test: `tests/zone_end.rs`
(timeline, cue, Pulse untouched), disc-backed `zone_craft_ground_truth` (all three titles' models).

| Row | State | Notes |
| --- | --- | --- |
| Z1 | wired | state 4 is one tick; the HUD hides at the blast (`Race::hud_shown`); the camera prologue is today's destroy camera, **inherited** (see Z2) |
| Z2 | inherited | camera mode 2's per-frame law is unread: the existing destroy camera runs, as for every title |
| Z3 | wired | no blast, plume, node sparks, wash or shake in a Zone end (`craft_flash`), no wreck model (the Zone `shipwreck.vex` is built and not drawn) |
| Z4 | not wired | the rumble is the ordinary destroyed rumble, unchanged |
| Z5 | wired on HD and 2048 | `shipdestroy` once on the blast (`Cue::ShipDestroy`, `ZoneEnd::cue`): HD from `env0_zone.bnk` (Ps3 tick), 2048 from its hashed `env0_zone.bnk` (Vita tick; resolves to one timeline of 6 voices, `zone_craft_ground_truth`). No other cue plays in a Zone end, `~BLOWUP` included. "After every other cue is stopped" is **not** wired: ours stops none. Omega: silent, its Wwise event is unread |
| Z6, Z7 | wired | state 4 one tick (`launch_hover::destroy_sequence`), state 5 runs the 5.5 s to the results, the shell's 2.5 s inside it |
| Z8 | wired, look unread | the shell plays over the hull for 2.5 s on `1 - max(0.4 t, 0)` (`zone_end::shell_time`). Our decoder returns HD's whiteshell `Anim Transform` as one key (translation and scale, no rotation channel) whose translation quantum and base come back denormal (`1.8e-43`, `5.9e-39`): so the clock moves no geometry, **but that payload may be misread** (conf 50); what the original varies with the clock is likely the material (`.rcsmaterial`, unread). 2048's shell carries a texture track, which the clock drives. Our blend is **chosen** (additive over a batch the build read as opaque or alpha-tested, the Detonator death shell's; a batch the build already blended keeps its factors) |
| Z9 | wired | the race ends 5.5 s after the blast (`eliminate_if_wrecked` waits for state 6, rules stop at the blast); the results are the existing flow |
| Z10 | wired in part | hull, flare and shell hide at 2.5 s, the debris is thrown. Not wired: the two side calls (`0x000f9610`, `0x003f5878`) and the second rumble |
| Z11 | wired, velocity chosen | 112 pieces split by their `ShipBit_N` node on all three titles (HD 3,838 triangles, 2048 and Omega the same count); the launch velocity is **chosen, not measured** (`zone_end::OUTWARD`, `LIFT`, `SPIN`, `GRAVITY`, the view's generator), the pieces rest on the craft's plane and collide with nothing; no fire |
| Z12 | wired | nothing of the craft is drawn after Z10 but the debris |

2048 and Omega: **ported** (2048's cue too, Omega's not). The models are the same 112 pieces and 5-node shell on both; their
`.rcsmodel` is read through `mesh::rcs::psp2::build_with_vex`, because the shell's motion is in
the `.vex` and neither title ships a skeleton for it. 2048's shell is 9 submeshes (60,294
triangles), Omega's one (28,229), HD's one (28,229): re-cooked, not identical.

## Mode 14 diverges (Detonator, not this lane's table)

For [detonator-hull.md](detonator-hull.md) and the Detonator pages, not edited here:

- Case 6, mode 14 (`0x000e1ff4` -> `0x000e1ffc`): the same three hides and rumble `0x3c`, then
  `0x001070a8(ship+0x7da4)` **instead of** the Zone debris (`0x000e2074`). So on HD a
  Detonator craft throws no `zoneship_dead.vex` pieces: this answers detonator-hull.md's
  "whether a Detonator wreck spawns it is unread" for HD (80). 2048's case 6 takes the
  debris arm for modes 6 and 14 alike (checked, differs).
- `Ship_UpdateWrecked`'s mode-14 arm (`0x000e3a7c`) writes `2.5 - timer` to `+0x7e50` and steps
  a keyframe table at `+0x7e84` (`+0x7ea0..`) instead of the shell's `0.4` law; Omega's
  `0x013080f0` case `0xe` has the same shape with a `-0.4 t + 1.0` write into a global
  record.

## Against Pulse

Pulse's Zone end ([zone-mode.md](../psp-pulse-usa/zone-mode.md), [zone-rest.md](../psp-pulse-usa/zone-rest.md)):
the craft becomes `zonewreck.vex`, `~BLOWUP` plays for the local player, the death camera is
mode 5 and the HUD hides on `DAT_08ab0838`. HD keeps the shape (a cue, a camera cut, a HUD
hide, an end screen) but none of the content: no wreck model, a different cue from a
different bank, camera mode 2, and a shell animation then 112 debris pieces in place of the
wreck.

## Functions

| Address | Name | Conf. | Evidence |
| --- | --- | ---: | --- |
| `0x00104a58` | `DebrisManager_SpawnZoneDebris` | 80 | names `"Ship has no zone debris to spawn"` (`attrib`); loops `param_5` pieces over `mgr+0x14c`, called with `0x70` from `Ship_SetState` case 6 mode 6 (`0x000e2d60`); the 2048 and Omega twins take the same `0x70` at the same point |
| `0x00105a48` | `DebrisManager_Construct` | 75 | the only constructor naming `DebrisManager.cpp` (`attrib` of `0x00782df0`) beside `0x00106578`; 112 Zone pieces and `zoneship_dead.vex`, or 19 `Ship_Damage` / 32 `Ship_Damage_Nitro` |
| `0x00104f18` | `DebrisManager_SpawnShipDebris` | 72 | names `"Ship has only zone debris to spawn"` (`attrib`), its refusal when the manager is a Zone one; otherwise launches 19 (`0x13`) pieces through `Debris_Launch` at the body matrix with `Libc_Rand` velocities, setting the record's fire byte `+0x58` for pieces past the count at `*0x008c17d0`; called on `Ship_UpdateExploding`'s 4 -> 5 edge outside the `0x206040` modes (ship-destroy.md open item 4) |
| `0x0010c958` | `Debris_Launch` | 72 | names `WO_DEBRIS_FIRE`; sets the piece body (type 7, inertia, velocity), adds it to the physics and collision worlds, shows the node; called once per piece by `DebrisManager_SpawnZoneDebris` |
| `0x000758a0` | `SPZone_UpdateFinish` | 78 | phase 3 of `SPZone_Update`'s table (`0x000768c0`); 1.0 s, then `rm+0x2dc4`, then `"EndRace Results"` and `RaceManager_SetPhase(4)` |
| `0x00053fc0` | `RaceManager_SetSubState` | 82 | `+0x1974 = arg; +0x196c = 0`, whole function |
| `0x00053ff0` | `RaceManager_IsPhaseTimeAtLeast` | 82 | `return k <= +0x196c`, whole function |
| `0x00683048` | `SceneNode_SetFlags` | 80 | `+0x34 \|= flags`, with the `0x20`/`0x40` side calls; called with `4` on the shell at load |
| `0x000a9b18` | `Camera_SetTarget_q` | 60 | `cam+0x1f0 = arg`, whole function; mode 2's enter reads `cam+0x1f0` as a craft (`+0x6944`) |
| `0x000aa090` | `Camera_EnterTrackView_q` | 60 | `Camera_SetMode`'s enter table entry 2 tail-calls it; `"AI track data"`, the target's body position |
| `0x002ff600` | `Sound_StopAllBankCues_q` | 60 | `0x00315c58`: every bank slot of `*0x00b6e308` through `0x00315598`, which stops every playing instance (`0x0031c7b0`) and its emitters |

Unnamed, below 50 or unread: `0x000884d8` and `0x00087d40` (HUD-side; what `g_Hud+0x98..+0xa8`
hold is unread), `0x000f9610`, `0x003f5878`, `0x001070a8`.

Twin named here (row in Omega's `names.tsv`):

| Binary | Address | Name | Conf. | Evidence |
| --- | --- | --- | ---: | --- |
| Omega | `0x013080f0` | `Ship_UpdateWrecked` | 78 | decrements the state timer `+0x7208` and the time in state; mode switch with case 6 animating `+0x72c8` by `1 - max(0.4 t, 0)` and case `0xe` counting `2.5 - t`: HD's `0x000e3768` shape |

## Measured on RPCS3: not run

Not attempted. ship-destroy.md's two failures (2026-11-22) stand: the packaged RPCS3
0.0.42 aborts HD save states in `ppu_load_prx` and a cold boot stalled at `boot`. The patched
build (`just build-rpcs3-watchpoints`, output `data/tools/rpcs3-watchpoints/build/bin/rpcs3`) does
not exist on the host and was not started in this lane.

**The read recipe**, for a build that boots a Racebox Zone race: break on
`Sound_PlayBankCue` (`0x002ffa58`) with `r5` = `0x00782538` (`shipdestroy`); from there poll
`ship+0x5fa4` (expect 4, then 5 one tick later, then 6 about 150 frames later at 60 Hz), the
shell's animation time (the `f1` of `0x002c1b30` calls with `r3 = ship+0x6afc`), the visible
bits of `+0x6adc`/`+0x6af4`/`+0x6afc` at state 6, the debris manager's `+0x40` node `+0x34`, and
the race manager `*0x0095ae78` `+0x1970`/`+0x1974`/`+0x196c` (expect phase 3 at the blast, sub 1
at 1.0 s, phase 4 at 5.5 s), photographing each change.

## Open

0. What the shell's animation time varies on HD (its material, unread: `.rcsmaterial`; and whether
   our `Anim Transform` decode of the whiteshell is right), `Sound_StopAllBankCues_q`'s counterpart
   in ours, Omega's `shipdestroy` Wwise event, and a live pair against RPCS3.
1. Camera mode 2's per-frame law (Z2): the per-frame twelve-way switch on `cam+0x40`, entry 2.
2. `0x003f5878`'s 24-slot pool (Z10): read its consumers through `*0x008b819c`.
3. The debris launch velocity (Z11) from the disassembly of `0x00104a58`
   (`0x00104be4`-`0x00104d50`), and the debris node class `0x00685f38` collects.
4. 2048's `Ship_UpdateWrecked` twin (Z8), by the `0.4` literal; 2048's and Omega's
   race-manager sides of Z9.
5. Omega's `shipdestroy` event container (Z5).
6. Case 4's `+0x5f9c == 0` branch (`0x000e28cc`): it stops the first node of class `0x00874018`
   in the live model (`0x0029f3e0`: state `2`, child `+0xe8` hidden and released).
   `Ship_ApplyDamage_q` writes `+0x5f9c = 0` for damage kind `2` (`r28`, `0x000e7c74`) and `1`
   otherwise; which damage a Zone death is, and what the node is, are unread.
