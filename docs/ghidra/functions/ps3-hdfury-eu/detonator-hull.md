# Detonator: its own hull, its own handling file, its own mine, bomb and pad models

Functions in `EBOOT.elf` (WipEout HD Fury, PS3, EU, `BCES00664`), Ghidra program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`. Static reading only, 2026-11-22: nothing here ran. The bodies
were read off `llvm-objdump -d --mattr=+altivec,+vsx` over their real extent, and every TOC slot
was resolved through the function's own OPD TOC (`scripts/ps3-toc.py resolve`; Ghidra's TOC is
wrong for this binary, [memory.md](memory.md)). The data checks are against `DATA00.PSARC` and
`DATA02.PSARC` of `hdfury-ps3-eu-dec.iso` (`scripts/psarc.py list|cat|extract`).

This page answers the maintainer's report that Detonator flies a custom hull, as Zone does, and that
our mines and bombs wear the wrong models. Both reports hold. The Zone half of the hull loader is
[zone-craft.md](zone-craft.md). The mode's rules are [detonator-mode.md](detonator-mode.md), and
the objects' laws are [detonator-mine.md](detonator-mine.md), [detonator-bomb.md](detonator-bomb.md)
and [detonator-cannon.md](detonator-cannon.md). The 2048 and Omega halves are
[`vita-2048-eu-v104/detonator-hull.md`](../vita-2048-eu-v104/detonator-hull.md) and
[`ps4-omega-eu/detonator-hull.md`](../ps4-omega-eu/detonator-hull.md).

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x00107b58` | `Ship_BindDetonatorHullParams` | 72 |
| `0x0013a7d8` | `DetonatorMine_Construct` | 80 |
| `0x002524e8` | `TeamSelection_CollectModelsForMode` | 62 |
| `0x000b6d68` | `WorldManager_CountPads` | 60 |

`0x0013a248` is `DetonatorMine_Construct`'s twin: the same body, the same model loads, and no
caller found by a `bl` scan. It stays unnamed, as the bomb blast's twin `0x00137e88` does.

## Summary: what a Detonator race loads

| What | Entry | Who loads it | Confidence |
| --- | --- | --- | ---: |
| Hull | `Data\Ships\Detonator\Ship.vex` (`.rcsmodel`) | `Ship_ReloadModelForSkin`, mode `0xe` | 82 |
| Livery | key `zoneship_zone` -> the craft entry's `texturelocation`, `zoneship_<team>` from the team's `PI_TeamModel name="detonator"`; with no entry, `Zoneship_Faisar` | same | 78 (mechanism), 65 (which entry) |
| Wreck | `Data\Ships\Detonator\shipwreck.vex` | same | 80 |
| Death shell | `Data\Ships\Detonator\ship_deathshell.vex` | same | 80 |
| Locators | `Data\Ships\Detonator\locators.vex` | same | 80 |
| Engine flare | `Data\Ships\Detonator\engineflare.vex` | same | 80 |
| LODs, leach effect, shield | not loaded in a race | same | 78 |
| Handling | `Data\Ships\Detonator\handlingstats.xml` | `Team_LoadHandlingStats` | 82 |
| Engine lights | `Data\Ships\Zone\EngineLightData.xml` (Zone's) | `Ship_LoadEngineLightData` | 80 |
| Mine | `Data\Weapons\mine_rita_football`, `mine_rita_spikes`, `Electric_Visualiser` | `DetonatorMine_Construct` | 82 |
| Bomb | `Data\Weapons\detonator_bomb_final`, `_final_equalisers`, `_glowedges` (+ four blast models) | `DetonatorBomb_Construct`, `DetonatorBombBlast_Construct` | 82 |
| Cannon round, muzzle flash | `Data\Weapons\detonator_cannonbolt`, `hd_muzzleflash_detonator` | `PulseBullet_Construct` | 85 |
| EMP pad | `Data\New_Mode_Pads\Detonator_Pad\Detonator_Pad.vex` at every speed pad and every weapon pad | `WorldManager_Construct` | 70 |
| `detonator_pickup`, `detonator_mine_explosion_kaleidoscopic` | no loader found | - | 70 (negative) |

## The hull: `Ship_ReloadModelForSkin` (`0x000dbdc8`), mode `0xe`

Function name and its generic half: [renderer.md](renderer.md) (78). Livery-name source and Zone
branch: [zone-craft.md](zone-craft.md) (80). New TOC slots read here:

| TOC disp | slot | target |
| --- | --- | --- |
| `-0x45dc` | `0x008a8efc` | `"Data\Ships\Detonator\"` (`0x007821f8`) |
| `-0x45d8` | `0x008a8f00` | `"Data\Ships\Detonator\Ship.vex"` (`0x00782210`) |
| `-0x45e0` | `0x008a8ef8` | `"zoneship_zone"` (`0x007821e8`) |
| `-0x45c4` | `0x008a8f14` | `"%s\shipwreck.vex"` |
| `-0x45ac` | `0x008a8f2c` | `"%sship_deathshell.vex"` |
| `-0x45b0` | `0x008a8f28` | `"%s\Zoneship_dead/zoneship_dead_whiteshell.vex"` (Zone's death shell) |
| `-0x4584`, `-0x4580` | `0x008a8f54`, `0x008a8f58` | `"%s\locators.vex"`, `"%s\engineflare.vex"` |
| `-0x45c0`, `-0x45bc`, `-0x45b8` | `0x008a8f18`..`0x008a8f20` | `"%s\ship_lod.vex"`, `"%s\ship_lod1.vex"`, `"%s\leacheffect.vex"` |
| `-0x48d0` | `0x008a8c08` | the byte at `0x009384e1` |

The flow, with the byte at `0x009384e1` clear (its meaning is unread; it is `0` in every ordinary
race, [endrace-voices.md](endrace-voices.md)):

1. **The mode is read once.** With the byte clear, `r23 = g_GameState+0xe0` (`0x000dc4b0`). With
   the byte set, `r23` stays `0` and neither the Zone nor the Detonator branch runs.
2. **The livery name** is copied into the stack buffer `r1+0x120`, exactly as for Zone: from the
   craft entry `ship+0x6298`, field `+0x1b0` (`0x000dbf3c`). With no entry and the mode in the
   mask `0x206040`, which holds `0xe`, it is the literal `Zoneship_Faisar` (`0x000dc500`-`0x000dc51c`).
3. **Mode `0xe` branches** at `0x000dbf50` (`cmpwi cr7,r23,14`, to `0x000dcbb0`). The branch
   copies `Data\Ships\Detonator\` into the base buffer `r1+0x2b0` (`r18`) and
   `Data\Ships\Detonator\Ship.vex` into the path buffer `r1+0x230` (`r19`). It does not read the
   entry's model location `+0x1b4`. It then pushes one pair onto the global substitution table
   `0x00f868e0`: it increments the count at `+0x70` (`0x000dcd70`-`0x000dcd9c`), builds `std::string`s
   from `zoneship_zone` (`0x000dcda0`, `bl 0x00685a88`) and from the livery buffer `r17`
   (`0x000dcdbc`), and stores them at record `count * 0x38`, `+0` and `+0x1c`
   (`0x000dce24`-`0x000dce60`). So the key is the same as Zone's, `zoneship_zone`. Zone's branch
   truncates the key to 13 bytes; Detonator's uses a whole-string assign. Both then join the common
   path at `0x000dbfa0`.
4. **No shield.** The common path skips `ShipShield_Construct` (`0x000dbfec`) when the mode is 6
   or `0xe` (`0x000dbfb8`-`0x000dbfc0`).
5. **The hull** loads from `r19` (`0x000dc034`, `bl 0x00685540`) into `ship+0x6ae0`.
6. **The wreck** is formatted from `r18` with `%s\shipwreck.vex`. It is loaded if `FwFile_Exists`
   finds it (`0x000dcb40`) and stored at `ship+0x6ae4`.
7. **In a race, modes 6 and `0xe` load no LOD and no leach effect** (`0x000dc0c4`-`0x000dc0e4`,
   then `0x000dc9e0`): `ship+0x6ae8`, `+0x6aec` and `+0x6af8` are zeroed. **The death shell** is
   formatted next (`0x000dcec0`-`0x000dcee0`): `%sship_deathshell.vex` over `r18`, so
   `Data\Ships\Detonator\ship_deathshell.vex`. Zone instead gets
   `Data\Ships\Zone\\Zoneship_dead/zoneship_dead_whiteshell.vex` (`0x000ddc10`). It is loaded if it
   exists (`0x000dd4e0`) into `ship+0x6afc`.
8. **Locators and engine flare** come from `r18`: `Data\Ships\Detonator\\locators.vex` and
   `\\engineflare.vex` (`0x000dca1c`, `0x000dca84`).
9. **For mode `0xe` only**, after the Zone-mask parameter bind on the hull and the wreck
   (`0x000dc338`-`0x000dc404`, the same bind Zone gets), the code calls
   `Ship_BindDetonatorHullParams(ship+0xc0, model, ship+0x6284)` for the hull (`0x000dc3c0`) and for
   the wreck (`0x000dc438`).

**Confidence 82** on the hull, wreck, death shell, locators, flare and the skipped LOD and leach
loads: one static read, every string resolved, the same structure in 2048's and Omega's loaders, and
a disc folder that ships exactly these files. The rubric ceiling for a decompile-only read is 84.
**78** on the substitution: the push is read, but the table's reader is still not found
([zone-craft.md](zone-craft.md)).

### `Ship_BindDetonatorHullParams` - `0x00107b58`

The function binds three material parameters of `model+0x204c`'s material set, by name, to a
global block at `0x00992f40` (TOC `-0x3cb8`):

- `ShipZone_Colour` -> `+0`, after `bl 0x00677be8(ship+0x6284, block)`, which writes the craft's
  colour into it;
- `Charge1` -> `+0x10`;
- `Charge2` -> `+0x14`.

Its only callers are the two mode-`0xe` sites above (`bl` scan). **72**: the names come from the
binary, and the call shape is read. The block's writer is `Ship_UpdateDetonatorHullParams`
(`0x00107c28`, 62, [detonator-ammo-gauge.md](detonator-ammo-gauge.md)): `Charge1` goes to `1` while
its input `v` is over `0`, `Charge2` while `v >= 1`, each smoothed; `v` is the EMP charge mapped
between its ready and full thresholds (70), so Charge1 lights once the EMP is ready and Charge2 once
it is full. The
cannon clip is **not** it: the clip drives the separate `detonator_bulletgaugesegments` block. The hull's materials carry
these names hashed (no plain string in any `detonator_*.rcsmaterial`).

### Which craft entry, and so which livery

`ship+0x6298` holds a `PI_TeamModel`. Every team in `DATA00`'s
`Data\Plugins\frontend\definition.xml` authors one `PI_TeamModel name="detonator"` with
`modellocation="Data\Ships\Detonator"` and `texturelocation="zoneship_<team>"`: twelve rows, for
example Qirex -> `zoneship_quirex`, Feisar -> `zoneship_faisar`. The front end's team-select model
list, `TeamSelection_CollectModelsForMode` (`0x002524e8`, called from `0x00252c10`), filters each
team's models by the screen's mode at `+0xf0`:

- mode `0xe`: the model named `detonator` (`0x00252750`, then `0x00252bec`);
- mode 6: `zone`;
- modes 13 and 21: `nitro`;
- otherwise: every model except `zone` and `detonator` (`0x00252760`-`0x0025278c`).

The AI's variant draw in `Race_CreateAiRacer` (`0x00058e38`) skips `detonator` and `zone` too. So
the player's Detonator craft is the team's `detonator` model, and its livery name is
`zoneship_<team>`. **65**: the race-side write of `ship+0x6298` is unread. This is the same open
question as Zone's.

The absorb shell is the one per-craft file that does read the entry's directory:
`ShipAbsorbShell_Load` (`0x000dba30`) formats `%s\AbsorbEffect.vex` from `entry+0x1b4`. With the
`detonator` entry that is `Data\Ships\Detonator\AbsorbEffect.vex`, which ships.

## The disc's Detonator folder (`DATA00.PSARC`, `/data/ships/detonator/`)

- **Shipped:** `ship`, `ship_lod`, `ship_lod1`, `ship_ghost`, `shipwreck`, `ship_deathshell`,
  `shipshield`, `absorbeffect`, `leacheffect` and `engineflare` (`.vex` + `.rcsmodel` each), plus
  `locators.vex`, `shadow.stencilvolume`, `handlingstats.xml`, `fe/thumb0.gtf` and
  `textures/detonator_deathshell_compare.gtf`. There is no `EngineLightData.xml`.
- **Six of the `.rcsmodel` files are one placeholder.** `shipwreck`, `ship_lod`, `ship_lod1`,
  `ship_ghost`, `shipshield` and `absorbeffect` are byte-identical (17,476 bytes, MD5
  `208c2efc...`). Each is a small outline model (`OutlineShape`,
  `nitro_emissive_outlines.rcsmaterial`, Maya source `.../DLC3/Ships/Detonator/shipwreck.ma`). So a
  Detonator wreck draws that outline, not a broken hull.
- **The hull's texture requests:** `ship.rcsmodel` names eight textures, all under
  `data/ships/detonator/zoneship_zone/`: `detonator_ao`, `detonator_bullet_segments`,
  `detonator_colours_flashing_glow`, `detonator_glass`, `detonator_hd_absorbinternal`,
  `detonator_n`, `detonator_stripe` and `iridescence`. Its materials are `detonator_*.rcsmaterial`
  under `data/materials/ships/`.
- **The liveries:** thirteen folders, `zoneship_zone` and twelve teams (`ag_sys`, `assegai`,
  `auricom`, `egx`, `faisar`, `goteki`, `harimau`, `icaras`, `mirage`, `piranha`, `quirex`,
  `triakis`). Each holds the same eleven files. **Only `detonator_ao.gtf` differs per team**
  (13 distinct of 13); `detonator_stripe.gtf` differs only in `zoneship_zone`, and the other nine are
  identical everywhere. So the team's identity is in its ambient-occlusion map. Unlike Zone's
  default folder, `zoneship_zone` here is complete, so an unswapped hull still draws with the
  default AO.
- **Locators:** `locators.vex` authors `cannon_flash_left`, `cannon_flash_right`, `engine_flare`,
  `boost_flare`, `ship_muzzle`, `wingtip_left`/`right`, `arc_anchor_point` and
  `ship_collision_fx`..`fx9`. Zone's `locators.vex` (`DATA02`) has the same node names at other
  positions (the files differ).

## The handling file: `Team_LoadHandlingStats` (`0x001e6ff8`)

The function is named on [opponent-handling.md](opponent-handling.md) (80), whose prose already
reads the Detonator branch. Re-read here for mode 14 specifically:

```
if byte(0x009384e1) == 0:                                   // 0x001e7038
    mode = g_GameState+0xe0
    if mode == 0xe:  parse "Data\Ships\Detonator\handlingstats.xml"   // 0x001e7138, TOC +0x1690
    elif mode == 0xd: parse "Data\Ships\Zone Battle\handlingstats.xml" // 0x001e716c
    elif mode <= 21 and (1 << mode) & 0x206040:
                     parse "Data\Ships\Zone\handlingstats.xml"         // 0x001e7124
    else: "%s\handlingstats.xml" over the variant's or the team's directory
```

**Mode `0xe` is tested before the mask, so a Detonator craft never reads Zone's file.**
**Confidence 82.** This is a static read, and 2048 (`0x810bfd54`) and Omega (v1.07 `0x014af2a0`)
carry the same three-way test.

The earlier line on [detonator-mode.md](detonator-mode.md) ("flies `/data/ships/zone/handlingstats.xml`")
was wrong for the handling file only. The other two `0x206040` readers, `Craft_Update` (no brakes)
and `Craft_UpdateHover` (the trailing ray), still cover mode 14.

A side observation: the team's `+0xb8` byte gates only the `"%s HANDLING STATS"` registration
(`0x001e7028`-`0x001e7034`). The parse runs on every call (both paths reach `0x001e7038`), so the
"does nothing twice" line on [opponent-handling.md](opponent-handling.md) describes the registration
only. Confidence 75; it changes nothing for a single-craft Detonator race.

### The two files, numerically

`DATA00` `/data/ships/detonator/handlingstats.xml` (1,398 bytes) against `DATA02`
`/data/ships/zone/handlingstats.xml` (1,392 bytes). Both are `<Stats team="ZoneMode">`, one
classless block. Every attribute not listed is equal, including all of `Physical`, `Antigrav`,
`Engine` (`accelcap 20`, `amount 300`, `falloff 500`, `gain 900`, `turbo 150`), `Brakes`,
`AirbrakeGraphics`, `Pitch` and `BonnetCamera`.

| Element.attribute | Zone | Detonator |
| --- | ---: | ---: |
| `Turning.amount` | 1.8 | 2.2 |
| `Turning.falloff` | 1000 | 1200 |
| `Airbrake.amount` | 9 | 6.0 |
| `Airbrake.falloff` | 500 | 1100 |
| `Airbrake.sideshift` | 550 | 650 |
| `Tilt.internalAmount` | 1.4 | 0.3 |
| `Tilt.shipAmount` | 1.4 | 0.3 |
| `Misc.width` | 5.0 | 4.0 |
| `InternalCamera.height` | 0 | 1.4 |
| `ExternalCameraFar.lookat_height` | 0 | -1 |
| `ExternalCameraFar.pos_height` | 4.0 | 5.5 |
| `ExternalCameraFar.pos_length` | -13.5 | -12.5 |
| `ExternalCameraClose.lookat_height` | -4 | -1 |
| `ExternalCameraClose.pos_height` | 3.6 | 3.9 |
| `ExternalCameraClose.spring_horiz` / `spring_vert` | 12 / 12 | 10 / 10 |

So the Detonator craft turns harder, with less airbrake and less tilt, and is narrower, on the same
engine. `Data\Ships\Zone Battle\handlingstats.xml` (`DATA00`, 1,394 bytes) differs from Zone's only
in `Turning` (1.65/925), `Airbrake.amount` 3.3, `falloff` 800 and `turn` 12.

### Engine lights: Zone's

`Ship_LoadEngineLightData` (`0x000d25e0`, [renderer.md](renderer.md)) reads
`Data\Ships\Zone\EngineLightData.xml` when the byte at `0x009384e1` is clear and the mode is in
`0x206040` (`0x000d2838`-`0x000d2854`). Otherwise it reads `%s\EngineLightData.xml` over the
entry's location. **80.** Detonator ships no `EngineLightData.xml`, which agrees.

No other `Data\Ships\Zone\` string in the binary is reached by mode 14. Zone's hull and death
shell are mode-6 literals. `Data/Ships/Zone/Zoneship_dead/zoneship_dead.vex` belongs to the debris
manager (`0x00105a48`/`0x00106578`), on a path its caller selects; whether a Detonator wreck spawns
it is unread.

## The Detonator objects' models

### Mine: `DetonatorMine_Construct` (`0x0013a7d8`)

The constructor is called by both `MineManager` constructors (`0x0011b86c`, `0x0011bc1c`). In mode
`0xe`, with the flag byte set, they build `DetonatorMine`s, not `Mine`s
([detonator-placement.md](detonator-placement.md)). It stores `DetonatorMine.cpp` at `+0x30` and the
vtable `0x008642f0` (TOC `-0x2bc8`) at `+0`. Then it loads three models through the shared model
loader (`bl 0x002c1ec8`), each registered with `SceneModel_RegisterRenderRecord`:

| Offset | Model | TOC |
| --- | --- | --- |
| `+0xcc` | `Data\Weapons\mine_rita_football.vex` / `.rcsmodel` | `-0x2b48`, `-0x2b44` |
| `+0xd0` | `Data\Weapons\mine_rita_spikes.vex` / `.rcsmodel` | `-0x2b38`, `-0x2b34` |
| `+0xd4` | `Data\Weapons\Electric_Visualiser.vex` / `.rcsmodel` | `-0x2b30`, `-0x2b2c` |

It binds `MineBrightness` and `MineColour` on the football, and on the visualiser `MasterVolume`,
`PitchOffset`, `PitchVolume`, `BassHit`, `TrebleHit`, `Tempo` and `ElectricityColour` (music-driven
electricity). **`detonator_mine_normal_alpha.rcsmaterial` is the material of both
`mine_rita_football.rcsmodel`** (with `mine_rita_football_n_1024x1024.gtf`) **and
`mine_rita_spikes.rcsmodel`** (with `mine_rita_spikes_n.gtf`). `electric_visualiser.rcsmodel`
uses `electricity.rcsmaterial` and three `electricity_*` textures. All of them ship in `DATA00`.
The colour sources are the circuit's effect-settings keys `%s.Detonator Mine Colour` and
`%s.Detonator Mine Electricity Colour`, registered by `Environment_RegisterStageSchema`
([zone-effectsettings-loader.md](zone-effectsettings-loader.md)). **82** on the models. The binds'
values, their writer and the per-stage table are read on
[detonator-mine-material.md](detonator-mine-material.md) (66-78; the shader's combine is unread).

### Bomb: `DetonatorBomb_Construct` (`0x00135328`)

[detonator-bomb.md](detonator-bomb.md) names the constructor. It loads three models:

| Offset | Model | Material(s) |
| --- | --- | --- |
| `+0xb4` | `Data\Weapons\detonator_bomb_final` (TOC `-0x2d20`, `-0x2d14`) | `detonator_bomb_normal`, with `bomb_n_32x32_plain.gtf` and `spectrum.gtf` |
| `+0xb8` | `Data\Weapons\detonator_bomb_final_equalisers` (`-0x2d1c`, `-0x2d10`) | `detonator_bomb_normal_equalisers`, `detonator_bomb_emissive`, `detonator_bomb_rays_plain` |
| `+0xbc` | `Data\Weapons\detonator_bomb_glowedges` (`-0x2d18`, `-0x2d0c`) | `detonator_bomb_explosion_shockwave` |

It binds `BombBrightness`, `BombColour`, `Envmap`, `BombEQBrightness`, `BombEQColour`, `Colour`,
`RainbowColour`, `Brightness`, `V_Offset`, `Distortion`, `Speed` and `U_Offset_Speed`, then builds
the blast (`DetonatorBombBlast_Construct`: `detonator_bomb_sphere`, `_glowedges`, `_shockwave` and
`_lightrays`, on [detonator-bomb.md](detonator-bomb.md)). The colours are the effect-settings keys
`%s.Detonator Bomb Inner Colour` / `Outer Colour`. **82.**

### Cannon round and muzzle flash

`PulseBullet_Construct` (`0x001235a8`, [detonator-cannon.md](detonator-cannon.md), 85) loads
`Data\Weapons\detonator_cannonbolt` (`hd_detonator_cannonbolt_halo.rcsmaterial`,
`hd_detonator_cannonbolt.gtf`, `_glow.gtf`) and `Data\Weapons\hd_muzzleflash_detonator`
(`hd_muzzleflash.rcsmaterial`, `pulse_muzzleflash_blue_glow.gtf`).

### EMP pad: `WorldManager_Construct` (`0x000b8590`)

After the track's `%s\padReplacement%s.vex` loads, `WorldManager_CountPads` (`0x000b6d68`) counts
the track's pad importers by class identifier. Objects whose `+8` is `0x00873ca0`
(`SpeedupPad_Importer`, stored by its constructors `0x002dd2a0`/`0x002dd340`) go into `+0x114`.
Those with `0x00873ca8` (`WeaponPad_Importer`, `0x002e0168`/`0x002e0210`, [pads.md](pads.md)) go
into `+0x1e0`. In mode `0xe` (`0x000b8d08`, to `0x000b9010`) the constructor then runs two loops,
one over each count. Each iteration loads
`Data\New_Mode_Pads\Detonator_Pad\Detonator_Pad.vex` / `.rcsmodel` (TOC `-0x5250`, `-0x524c`),
registers it, and binds it to the pad's transform list. Modes 13 and 21 do the same with
`Zone_Battle_Pad`. So **every speed pad and every weapon pad carries a `Detonator_Pad` model in
Detonator**. The pad uses `diffuse_normal_specular_emmissive_alpha` and `emissivecolourchange`
materials, with `ds_weaponup_cs.gtf` and `ds_weaponup_ne.gtf`. **70**: the loop bodies' transform
binding was read only far enough to see the per-pad list. Before these loops the constructor
deletes the circuit's own pad objects (`detonator-emp.md`, "Do the circuit's own pads still
draw?", 80): the pad underneath is not drawn. Still unread: what a craft crossing the pad does (the EMP charge
flag `craft+0x6a64 & 0x8000000`, [detonator-mode.md](detonator-mode.md)).
`WorldManager_CountPads` is **60**: the two type-identifier compares and the two stores are read,
the walk around them is not.

### Shipped, but no loader found

`Data\Weapons\detonator_pickup` (`hd_leachbeam_ball_glow`/`bloomring` materials) and
`Data\Weapons\detonator_mine_explosion_kaleidoscopic` (`explosion_kaleidoscopic.rcsmaterial`) ship
in `DATA00`, but no string naming either exists in `EBOOT.elf` (`strings` over the image: no
`pickup.vex`, no `kaleido`). No `WO_*DETONATOR*.POB` or `/data/xml/` entry names them either. **70**,
negative: a model could still reach them through another model's node or a table not scanned.
The mine's death is the `.pob` pair `WO_MINE_EXPLO_DETONATOR[_DEAD]`
([detonator-mine.md](detonator-mine.md)), not this model, as far as read.

## Wired

All six rows below are wired (`oag_title::Detonator::{craft, handling,
objects}`, `oag_hd::race::DETONATOR_CRAFT`, `DETONATOR_HANDLING`, `DETONATOR_OBJECTS`;
`oag_livery::entry::own_craft`; `oag_raceplay::detonator_models`; tests
`crates/raceplay/tests/detonator_hull_ground_truth.rs`). What the wiring does not do, and why:

- **No wreck and no death shell.** HD's PS3 hull build (`oag_livery::one`) loads no wreck in any
  mode and no loader for `ship_deathshell` exists, so Detonator's `shipwreck.vex` (the outline
  placeholder) and `ship_deathshell.vex` are not read. The loader report names the hull's folder.
- **No shield.** A Detonator craft skips the shield shell, as `ShipShield_Construct` is skipped
  in mode `0xe` (the folder's `shipshield` is the placeholder). Zone, which the original also
  skips for mode 6, is untouched.
- **The engine light is Zone's** (`oag_livery::entry::light_craft`: every mode on the mask).
- **Livery fallback `zoneship_faisar`** is data on HD (`DetonatorCraft::livery_fallback`); 2048
  and Omega name none.
- **Handling: HD, 2048 and Omega** read their own file before the Zone mask
  (`oag_raceplay::load::sim::mode_handling`); the two unauthored values (`landing_rebound`,
  `rebound_jump_time`) are Zone's live read on HD and inherited from HD elsewhere.
- **`electric_visualiser` is drawn on HD** as a camera-facing ring through its material's own program
  ([detonator-mine-draw.md](detonator-mine-draw.md), "The visualiser, wired"); 2048 and Omega do not draw it. The mine's and bomb's colour binds (`MineColour`,
  `MineBrightness`, `BombColour` ...) are unwired; their law is read
  ([detonator-mine-material.md](detonator-mine-material.md)), so the models draw unbound and the
  football reads bright white until a per-instance constant path exists.
- **Pads**: the Detonator pad draws at every speed pad's and weapon pad's own `to_world`
  (`PadVolume::to_world`) and the circuit's own pad geometry is **not** drawn under it on HD, Omega and 2048
  (`DetonatorPads::replaces_circuit_pads`, `detonator-emp.md`). Whether the model's origin is the pad's
  transform origin is **chosen**, not measured.
- **The bomb's HD halo** (`bomb_blast_hd.halo`) still draws on a laid Detonator bomb, as before:
  unread.
- **The cannon** (2026-11-22): `detonator_cannonbolt` on every round and
  `hd_muzzleflash_detonator` at the hull's `cannon_flash_left`/`right` locators (`DetonatorObjects::cannon`,
  `oag_raceplay::detonator_fx`); the simulation reads the same locators (`load::muzzle::cannon_flash`), and
  `Loaded::cannon_muzzles(slot)` is the livery's reading of them, which a test holds equal. Details
  and what is chosen: [detonator-cannon.md](detonator-cannon.md), "Drawn and heard".
- **2048 and Omega**: all wired with HD's names. 2048's archives ship the objects under both
  `Data\Weapons` and `Data\Weapons2048`; the directory its loaders join is unread, so HD's is
  chosen. Omega's pad path is inherited from HD (it reads it from a data field). A 2048 native
  team authors no `PI_TeamModel name="detonator"`, so its hull keeps the default `zoneship_zone` skin.

## What the port must change

1. **Hull**: in Detonator every craft flies `/data/ships/detonator/ship.vex`, not the team's race
   hull. The livery swap uses key `zoneship_zone`, with the value taken from the team's
   `PI_TeamModel name="detonator"` `texturelocation` (`zoneship_<team>`; fallback
   `zoneship_faisar`), resolved as `data/ships/detonator/<value>/<file>`. Load the wreck
   (`shipwreck.vex`, the outline placeholder), the death shell (`ship_deathshell.vex`),
   `locators.vex` and `engineflare.vex` from the same folder. Load no LODs, no leach and no shield.
   Use `absorbeffect.vex` from the same folder through the entry's directory. Today `ZoneCraft`
   applies only in `Mode::Zone`, so Detonator draws the team's own hull.
2. **Handling**: `/data/ships/detonator/handlingstats.xml`, not `/data/ships/zone/handlingstats.xml`
   (`oag_hd::race::ZONE_HANDLING` is applied to every `on_zone_mask` mode). The hover rig, no brakes
   and Zone's `EngineLightData.xml` stay as they are.
3. **Mine model**: `/data/weapons/mine_rita_football` + `/data/weapons/mine_rita_spikes` (both
   `detonator_mine_normal_alpha`) + `/data/weapons/electric_visualiser`, in place of `hd_mine`.
4. **Bomb model**: `/data/weapons/detonator_bomb_final` + `detonator_bomb_final_equalisers` +
   `detonator_bomb_glowedges`, in place of the race Bomb's.
5. **Pads**: a `/data/new_mode_pads/detonator_pad/detonator_pad` model on every speed pad and every
   weapon pad.
6. **Cannon** (wired): `/data/weapons/detonator_cannonbolt` and
   `/data/weapons/hd_muzzleflash_detonator`, from the hull's `cannon_flash_left`/`right` locators.

## Lineage (2048 and Omega)

- **2048 v1.04: checked, applies, not wired.**
  [`vita-2048-eu-v104/detonator-hull.md`](../vita-2048-eu-v104/detonator-hull.md). The hull is
  `Data\art\published\hdships\Detonator\Ship.vex` (`Ship_LoadModelSet`, mode `0xe`) with the key
  `zoneship_zone`, and the same eleven per-team textures. The handling loader `0x810bfd54` tests
  `0xe` first, and its `handlingstats.xml` is byte-identical to HD's. The mine, bomb, bolt, flash and
  pad entry names are the same strings in its executable. The cannon's two models resolve in its
  archives under HD's names (`patch_ground_truth.rs`).
- **Omega: checked, applies, not wired; the livery key differs.**
  [`ps4-omega-eu/detonator-hull.md`](../ps4-omega-eu/detonator-hull.md). The hull is
  `hdships\Detonator\Ship.vex` with the key `zoneship_team` (its Zone key too), and per-team folders
  carry only `Team.gnf`. The handling test is the same, and the file is byte-identical. The mine,
  bomb, bolt and flash names are the same. Omega's executable has no `Detonator_Pad` string: its
  mode-`0xe` pad path comes from a data field. The cannon's two models resolve in its archives
  under HD's names (`omega_title_ground_truth.rs`).

## Open

- The substitution table's reader ([zone-craft.md](zone-craft.md)). It would raise the livery reading
  above 78.
- The race-side write of `ship+0x6298` (which `PI_TeamModel` a Detonator craft carries).
- `Charge1`/`Charge2`: who writes the block at `0x00992f40`.
- The `Detonator_Pad` loop's transform binding, and whether the original pad still draws.
- The mine's and bomb's colour and brightness binds: read, see
  [detonator-mine-material.md](detonator-mine-material.md); what stays open there is the shader's
  combine and the bomb blast's own binds.
