---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-22
summary: "Detonator is a Mode (stage = lap, grade, camera, board); mine placement read, not wired; EMP, cannon, score, auto-speed, Zone Battle unread"
categories: [rendering]
---

# Detonator races its stages without mines, EMP or auto-speed

2026-11-13, lane `modes-zone-battle-detonator`. `oag_race::Mode::Detonator` races fourteen
stages, one per lap, on HD, 2048 and Omega (`docs/gameplay/race-modes.md`, "Detonator";
evidence `docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`). A player sees the stage
grade escalate lap by lap and the race end after lap 14, and nothing to shoot.

## Open

- **End screen headline is wired** (`Headline::Detonator`, `ER_ZDETONATOR_COM`): it draws on
  2048 and Omega, nothing on HD (no HD archive authors the text). Score and stage rows open.

- **Weapon table decoded, not spent.** `WeaponStats::detonator()` reads the Detonator-only
  attributes (Mine scoring, Cannon reload/rebounds, Bomb shots and points, the `EMP` block,
  `energy_recharge_per_stage`) on HD, Omega and 2048 (`docs/formats/weapon-stats.md`,
  "Detonator's attributes"). Nothing in a race reads it; each field's meaning waits for the
  law that spends it.

- **Mines and bombs: the placement law is read, nothing is wired.**
  `docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md` (HD, confidence 70-88 per piece;
  Omega read line by line, 2048 structurally: both "checked, applies, not wired"). The data is
  two global files, not per-track: `/data/xml/DetonatorLayouts/PatternFile.xml` (18 `Pattern`
  groups of `ClusterN` rows) and `GamePlayValues.xml` (`MineCountStage1..14` = 80..200, lane
  width 3.5, the 20-section jump blackout); same content on all three titles. The per-track
  `%s.xml` path is formatted and never read. Phase 4 needs a reader for both (`oag-tables`,
  generic XML) and `oag_title::Detonator` entry names; Phase 5 the `DetonatorField` (200-mine
  pool) and the placer on the seeded `Rng`. Still unread: the collision return codes the lap
  survey tests (`0/2/4/5/127`; a stand-in "lane free when the downward probe hits track" is
  needed and must say so), the route-fork walk, what the mine/bomb inits do with the matrix and
  `sec+0x40`/`+0x60`, how a mine or bomb leaves its pool (the bomb pool is 2 and the spawn does
  not check it), `0x0012ae38`/`0x0012ae10`/mine vslot `+0x3c`. Models:
  `data/Weapons/detonator_bomb_final_equalisers.rcsmodel` (2048), `DetonatorMine` (368 bytes,
  `0x0013a7d8`) and `DetonatorBomb` objects; effects `WO_BOMB_EXPLO_DETONATOR`,
  `WO_MINE_EXPLO_DETONATOR[_DEAD]`, `WO_CANNON_SPARKS_DETONATOR` (2048 `data2.psarc`), whose
  triggers need the hit read first.
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

**The medal side is read, the wiring waits for the race.** A `NitroBattle` cell's medal is its
race position against the rung's `1`/`2`/`3` (`<=`, unflipped; confidence 70,
`docs/ghidra/functions/ps3-hdfury-eu/cell-medal-award.md`), and `Cell::evaluate_medal_for_difficulty`
already reads it that way for a title whose `counting_modes` omits the mode. Not wired:
`oag_game::campaign::race_mode_for_cell` refuses the mode, and `RaceStage::campaign_tier` has no
value arm for `Other("NitroBattle")`, so the day it launches that arm needs the race-position
value (the original passes `param_1[0xb22]` there, a `u16` off the race record, not read).

## 2048 Combat is unread

`GameMode_GetKindLabel` (`0x812b021a`) shows `FE_GAMEMODE_ELIM` as `COMBAT`, so 2048's Combat
label is its Elimination; but `TrackStartup_Read`'s Combat board test `FUN_812b0880(mode, 1)`
(`track-startup-sound-bank.md`) is unread, so which mode id picks `321Go_2048_Combat.vex` is
not known. Read `FUN_812b0880` before mapping the board to `Mode::Eliminator` on 2048.

## Next Steps

1. Phase 4: read `PatternFile.xml` and `GamePlayValues.xml` (census on HD `DATA00`, 2048
   `data.psarc`, Omega `data00.psarc`), name them on `oag_title::Detonator`.
2. Phase 5: the lap survey, `DetonatorField` and the placer from `detonator-placement.md`, drawn
   inert. Before the hit: read the mine/bomb update, how either leaves its pool, and the cannon.
3. Read Detonator's engine branch for the auto-speed law.
4. Zone Battle: the read recipe above.
