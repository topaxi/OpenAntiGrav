---
status: ready-to-wire
titles: [pure, 2048, omega]
updated: 2026-11-02
summary: "Hit sparks, absorb and wreck nodes now inherited per title; open: Pure big wreck blast, 2048/Omega other hit triggers; Pure VECTOR laps"
difficulty: medium (30)
---

# Where a title still runs a chosen rule while Pulse has a recovered one

2026-10-03, from the `fire-law-inherit` lane. The maintainer's rule: a title with no measured
law of its own runs Pulse's, never this project's chosen fallback, labelled **inherited from
Pulse, unmeasured on <title>**. The opponent fire law is done (all five titles ship a
`WeaponAIstats.xml`, `docs/gameplay/race-modes.md`, "Firing on the original's law"). The rest of
the survey is below; none was trivial enough to fix in that lane.

## Open

- ~~**Wreck effects, hit sparks, absorb burst**~~ wired 2026-11-02 (`inherit-hit-effects`), each
  title on its own disc data and locators, `Title` data with `InheritedFrom` provenance
  (`docs/rendering/hull-sparks.md`, "Other titles"): Pure takes Pulse's hit sparks and wreck node
  blast and death sparks (its absorb was already its own, measured); 2048 and Omega take HD's
  absorb burst and Cannon craft-hit spark. **Still open from it:** (a) Pure's big wreck blast: its
  executable names `WO_SHIP_FXNODE_BIGEXPLO`, `WO_SHIP_EXPLO_SMOKE` and `WO_SHIP_EXPLOSION_DEBRIS`
  (not `WO_SHIP_EXPLOSION`) and `Ship_SetState` is unread, so the big blast and the ship shockwave
  are left out there; recipe: xref those three strings in `/pure/BOOT-psp-pure-usa.BIN`. (b) 2048
  and Omega draw no wreck effect (HD names none; their `WO_DEBRIS_SPARKS`/`WO_DEBRIS_FIRE` preload
  is unread), and the non-Cannon weapon hit trigger and HD's `WO_DAMAGE_*` smoke are unwired there
  (checked, applies, not wired). (c) Pure's weapon-hit `.COLLISIONS` cue now plays with the spark
  (Pulse's gate, inherited).
- **Pure's `VECTOR` lap count** falls back to Venom's `3` with a load warning (`race-modes.md`,
  campaign rungs): chosen, not measured. Pulse's rung table is the known rule.
- **Zone ship handling on Pure and HD** is unread; Pulse's rule (the player's own team numbers)
  is the known one (`race-modes.md`, Zone ship).
- **The absorb half and Mine/Bomb/Turbo/Cannon fire policy** of the opponent weapon AI are this
  project's own on every title, Pulse included: a Pulse gap, not a title one.
- **HD's `EliminatorAIStats` row** is parsed since 2026-10-03 (`weapons::ai`, every copy
  ground-truthed) and **not consumed**: no mapping to Pulse's mode-8 terms reaches 70 (use
  scales 10.5/12.5/15.5 against a flat x5; absorb scale falling with difficulty against one
  `0.001`), so HD stays on Pulse's terms. HD's loader is pinned (`weapon-ai-stats.md`,
  RaceManager `+6240`, fields `+176..+228`); its **reader is not found** - next is a Ghidra
  xref on that field or an RPCS3 watchpoint. Omega's named file has no row, its
  `WeaponAIStats2048.xml` does: pick the file by evidence before wiring.

- ~~**HD's gantry `GO` pulse phase**~~ closed 2026-10-04 (`hd-go-pulse`): HD's race manager
  holds the gantry's time in `[3.83, 5.25)` s from the release, so `GO` is lit on the step and
  loops 86 ticks, matching the capture's dark centres to two ticks
  (`docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`). **Still open from that read:**
  - ~~**HD's lap windows are not played**~~ closed 2026-10-04 (`hd-gantry-laps`): played by
    `BoardWindow::of` (`race/gantry/board.rs`), lap 0 until the first crossing (read live on
    RPCS3), board states measured on RPCS3 (FX-350 art, `FINAL LAP` strobe, chequered flag).
    **Left from it:** (a) the `ship+0x7810` increment site is not found (no direct `stw`; the
    value is rewritten every frame, so a GDB write does not stick; a `Z2` watch needs the patched
    RPCS3, `just build-rpcs3-watchpoints`); the `0 -> 1` at the first crossing is inferred
    (constructor `0x000ddd58` zeroes it, endrace reads `-1`), about 70. (b) Our chequered flag
    reads grey where the original's is black and white: the board-shading gap in
    `start-gantry.md` ("board's background is black"), not the clock. (c) A lap crossing was not
    captured live; the windows were forced through the TOC floats instead
    (`scripts/rpcs3-drive.py lapboard --window`).
  - **The start tick in code**: what runs `RaceManager_ResetGantryTime` (`0x00055488`) during
    a countdown. The capture bounds tick 70 from below only, and two cues point later: our
    backdrop is part way to green at tick 271 where the original is red, and the capture's
    digits come ~13 ticks later than ours.
  - **Plasma's `UV_offset`** (`plasma.md`, 2026-09-25): `AnimNode_UpdateTransformTree`'s slot
    `+0x40` on that node class is `MeshImporter_SetTime`, so the binding is the node's animation
    time, not a keyframe evaluator's output.

## Next Steps

1. Pure's big wreck blast (the three `WO_SHIP_*` names above) and 2048/Omega's non-Cannon hit trigger and `WO_DAMAGE_*` smoke. (The gantry clock landed 2026-10-03: HD runs Pulse's rule on its own `GO` edge, frame 203 / tick 70; 2048 and Omega place no gantry - `docs/rendering/start-gantry.md`.)
