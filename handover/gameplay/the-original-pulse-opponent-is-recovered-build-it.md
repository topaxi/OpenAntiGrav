# The original Pulse opponent flies the campaign: close what it does not match yet

2026-10-11, lane `orig-ai-pulse` (RE); built 2026-10-12, lane
`orig-driver-pulse`. The maintainer's decision of 2026-10-11: the campaign gets
a third AI mode, the original's own opponent with its rubber banding; race
box, remix and multiplayer keep ours.

The specification and the build are [ai.md, "The original driver,
recovered"](../../docs/gameplay/ai.md#the-original-driver-recovered-2026-10-11)
and its [built](../../docs/gameplay/ai.md#built-2026-10-12-oag_aioriginal)
table; the instruction-level evidence is
[ai-controls.md](../../docs/ghidra/functions/psp-pulse-usa/ai-controls.md).
Do not requote either here.

What is settled, in one breath: `oag_ai::original` is the whole driver (thrust
law to the bit against the capture script, wander, shuffle, lane planner and
ring, steering, pitch, rail), `oag_physics::rail::apply` is its one body
write, applied between the controls and the step because one
`AiManager_Update` runs per `Body_StepWorld` (measured), and every Pulse
campaign race flies it (`Race::use_original_ai`, keyed on the campaign cell,
`RaceDefaults::original_opponent` true on Pulse only). The authored lane
band turned out to be an exclusion zone, not a hold. On the disc
(`crates/game/tests/original_ai_ground_truth.rs`) the law reads the capture's
`75.0`, `101.7` and `34.6`, and the field laps Talon's Junction with no
respawn; our AI is untouched (lone-craft regression identical, twelve clean).

## Open

- **Our field closes into formation slower than the original's.** Same law,
  same tables: in `hard-park` the original's medians fall to the `48` lower
  stop by 10 s (median gap `-25` to `-48`); ours hold `75` to `102` on or
  behind their slots. At skill `0.9` the median is `50.2` live against our
  `60.5`. Airbrakes are not it (0.0-0.1 % of samples), and the throttle
  reaches the engine unclamped (`101.7`, asserted). Either the craft's
  speed at a throttle or the line it holds differs; numbers in ai.md.
- **No campaign cell was captured.** The comparison is the Single Race
  capture; `AI_ResolveSkillScale`'s campaign branch has no live leg.
- **The steering and the lane planner have no live leg**, and how much of
  the line the rail holds is not separated (NOP `0x088548dc` and log).
- **Mapped, not measured** (each named in `oag_raceplay::original_ai`'s
  header): own sample, lap progress, lookahead walk, contacts from the wall
  response only (no craft-to-craft), race time from the green light, grid
  order. A cell with no rung shuffles at level 1, chosen.
- **The session hook has no automated test** (it needs a GPU-built stage);
  a windowed front-end walk is its only evidence.
- **Replays** do not record the mode; none records an opponent race yet.
- **`StartBoost`** at the launch (`ai-stats.md`) is still unwired.
- **Other titles.** HD flies it since 2026-10-12 (lane `orig-ai-hd`, its own
  thread [hds-original-opponent-flies-its-campaign.md](hds-original-opponent-flies-its-campaign.md)).
  2048 and Omega ship the same tables; Omega's executable inlines HD's driver
  (static); neither is wired. Pure is another schema.
- **The campaign skill was never resolved before 2026-10-12**: the session read
  `self.race_options` after taking it, so every Pulse campaign launch flew at the
  ambient scale. Fixed in `orig-ai-hd`; a Pulse cell now gets its own
  `stats.xml` value, which no Pulse windowed walk has re-checked since.

## Next Steps

1. Capture a Pulse campaign cell on PPSSPP (`scripts/psp-ai-thrust.py`, after
   `CellSelection_CommitSelection`) and run the same comparison as
   `original_ai_ground_truth.rs` against it: the campaign branch's first live
   leg.
2. Chase the formation gap: log one opponent's speed, throttle and lateral
   error over 10 s on both sides (PPSSPP: `racer+0x50`, `driver+0x0c`, the
   lane point) and compare with `Race`'s own.
3. Give the steering a live leg: log the lookahead sample and the lane ring at
   `Ai_ComputeControls`' entry and recompute the steer.
