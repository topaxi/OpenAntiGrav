---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-13
summary: "Detonator is a Mode (stage = lap, grade, 0.7 camera, board); mines, EMP, cannon, score, auto-speed unread; Zone Battle and 2048 Combat rules unread"
categories: [rendering]
---

# Detonator races its stages without mines, EMP or auto-speed

2026-11-13, lane `modes-zone-battle-detonator`. `oag_race::Mode::Detonator` races fourteen
stages, one per lap, on HD, 2048 and Omega (`docs/gameplay/race-modes.md`, "Detonator";
evidence `docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`). A player sees the stage
grade escalate lap by lap and the race end after lap 14, and nothing to shoot.

## Open

- **Mines and bombs.** `FUN_000654e0(rm, oldStage)` (HD) places a stage's objects; Ghidra
  truncates it on bad instruction data. Read from the disassembly (`scripts/ps3-dform-scan.py`
  helps where the body is cut short): the pattern records at `rm+0x2e30` (stride `0xd60`,
  minimum stage at `+0x3b80`), where they are loaded from (a per-track file? `detonator.points2`
  is a front-end file), the per-stage count table (`+0x14 + stage*4` off a global), and
  `FUN_00064f70` (spawn). 2048: `FUN_8117cf22`; Omega: `FUN_012a2300`, which Omega also calls
  once at race start. Models: `data/Weapons/detonator_bomb_final_equalisers.rcsmodel` (2048),
  `DetonatorBomb.cpp` / `NormalBomb.cpp` items under `BombManager`; effects
  `WO_BOMB_EXPLO_DETONATOR`, `WO_MINE_EXPLO_DETONATOR[_DEAD]`, `WO_CANNON_SPARKS_DETONATOR`
  (2048 `data2.psarc`). Their triggers need the placement and the hit read first.
- **Pulse cannon and EMP.** The EMP gauge law is read (page, "What `Detonator_UpdateRace`
  does that is not wired"); the EMP pads (`data/New_Mode_Pads/Detonator_Pad`, 2048), the
  charge flag's writer (`craft+0x6a64 & 0x8000000`), the cannon, its reload (circle) and
  `weaponstats_Detonator.xml` (2048 `data1.psarc`) are not. `docs/gameplay/weapon-lineage.md`
  already lists EMP as unwired.
- **Auto-speed.** The manual: "Like Zone mode, acceleration is automatic and ever-increasing".
  No law read; the player throttles. Read what Detonator's engine branch does (HD's hover
  code gates modes 6, `0xd`, `0xe` together, `hover-target.md`), and whether `class="Zone"`
  cells fly `/data/ships/zone` or `/data/ships/detonator`.
- **Score, end screen, HUD.** `rm+0x6de3c` (objects destroyed) and the score add in the top
  block (`FUN_000642a0`); `ER_ZDETONATOR_COM` / `ER_DETONATOR_STAGE` / `PERCENT_MINES_CLEARED`;
  `detonator_hud.xml` (the race draws the time trial's HUD). Campaign cells (`mode="Detonator"`,
  14 on `DATA00`'s Fury grids) are kept raw and do not launch.
- **The out-of-race ending** (`+0x6a64 & 0x1000`) is not modelled: nothing in a lone Detonator
  race can destroy the craft until the mines exist.
- **Omega's grade** is not drawn (Omega draws no Zone grade either); Omega's camera scale is
  inherited from HD, its craft constructor unread. **2048's per-stage `.gxt` textures** are
  unwired, as Zone's are.
- **2048 `zone_1`..`zone_4` draw a black floor** in every mode (Time Trial too): not
  Detonator's, seen while capturing.

## Zone Battle (HD mode `0xd`, Omega also `0x15`; 2048) is unread

The brief's second priority. Nothing in `docs/` recovers its rules: the stage source
`*(rm + 0x2dfc + n*4)` (per viewport) has no recorded writer
(`zone-effectsettings-loader.md`, tenth pass); `race-hud.md` names MPNitro as the Zone Battle
manager and the campaign authors it as `mode="NitroBattle"` (`AICount="7"`, `laps="0"`, a real
`NitroElim*` target triple: `crates/hd/tests/campaign_grids_ground_truth.rs`). Read recipe:
`SPNitro`'s race-manager vtable (`race-manager.md` table) for its `Update`, then
`search_instructions` for `stw` at `0x2dfc(` (folded-bias trap: also scan with
`scripts/ps3-dform-scan.py` over the update's real extent) to find what steps a viewport's
stage and what the elimination targets count. 2048 and Omega checked against it per CLAUDE.md.

## 2048 Combat is unread

`GameMode_GetKindLabel` (`0x812b021a`) shows `FE_GAMEMODE_ELIM` as `COMBAT`, so 2048's Combat
label is its Elimination; but `TrackStartup_Read`'s Combat board test `FUN_812b0880(mode, 1)`
(`track-startup-sound-bank.md`) is unread, so which mode id picks `321Go_2048_Combat.vex` is
not known. Read `FUN_812b0880` before mapping the board to `Mode::Eliminator` on 2048.

## Next Steps

1. Read `FUN_000654e0` and `FUN_00064f70` from the disassembly; write the placement law on
   `detonator-mode.md`.
2. Read the DetonatorBomb update and the cannon hit, then wire mines as world objects
   (seeded `Rng` for the pattern draw) with their disc models and effects.
3. Read Detonator's engine branch for the auto-speed law.
4. Zone Battle: the read recipe above.
