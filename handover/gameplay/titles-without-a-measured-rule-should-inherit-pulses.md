# Where a title still runs a chosen rule while Pulse has a recovered one

2026-10-03, from the `fire-law-inherit` lane. The maintainer's rule: a title with no measured
law of its own runs Pulse's, never this project's chosen fallback, labelled **inherited from
Pulse, unmeasured on <title>**. The opponent fire law is done (all five titles ship a
`WeaponAIstats.xml`, `docs/gameplay/race-modes.md`, "Firing on the original's law"). The rest of
the survey is below; none was trivial enough to fix in that lane.

## Open

- **Wreck effects, hit sparks, absorb burst** (`wreck_fx.rs`, `hit_sparks.rs`, `absorb.rs`):
  Pulse-only. Pure and 2048/Omega draw nothing; HD has its own mechanisms. An honest absence,
  not a chosen rule, but under the maintainer's rule an unmeasured title would play Pulse's
  emitters on its own locators. Needs each title's locator names first.
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
  - **HD's lap windows are not played**: by `ship+0x7810` (1 on the first lap), `[6.017, 9.3)`
    otherwise, `[9.5, 9.9)` on `lap == total - 1`, `[12.35, 13.3)` on `lap == total` (the
    branch that cues `FINAL_LAP`). Ours keeps looping `GO` after the line crossing. Needs HD's
    asset states at those times read, `ship+0x7810` reconciled with `standing.lap`, and
    `clip_to_panel`'s later-state draws put back. RE first, not ready to wire.
  - **The start tick in code**: what runs `RaceManager_ResetGantryTime` (`0x00055488`) during
    a countdown. The capture bounds tick 70 from below only, and two cues point later: our
    backdrop is part way to green at tick 271 where the original is red, and the capture's
    digits come ~13 ticks later than ours.
  - **Plasma's `UV_offset`** (`plasma.md`, 2026-09-25): `AnimNode_UpdateTransformTree`'s slot
    `+0x40` on that node class is `MeshImporter_SetTime`, so the binding is the node's animation
    time, not a keyframe evaluator's output.

## Next Steps

1. Wreck effects, hit sparks and the absorb burst: each title's locator names first. (The gantry clock landed 2026-10-03: HD runs Pulse's rule on its own `GO` edge, frame 203 / tick 70; 2048 and Omega place no gantry - `docs/rendering/start-gantry.md`.)
