---
status: measure
titles: [pure]
updated: 2026-10-12
summary: "Pure sibling driver: LeadZone, pitch fade and steering output unmeasured live; what sets the x0.7 flag unread"
---

# Pure's original opponent: a sibling driver, built and flown in Pure's campaign

2026-10-12, lanes `orig-ai-pure` (the read, the throttle) and `pure-orig-driver` (the
rest, the flip). Pure's opponent is the earlier design Pulse grew out of. Evidence:
[psp-pure-usa/ai-controls.md](../../docs/ghidra/functions/psp-pure-usa/ai-controls.md)
and [gameplay/ai.md](../../docs/gameplay/ai.md) ("Pure: a sibling driver").

Settled and built: the legacy block of `AIRaceStats.xml` (throttle reproduced on 1,688
of 1,689 PPSSPP samples), the ringless planner (209 of 209 live calls), the position-only
rail, the pitch fade, every opponent every frame. `Ai_ScoreOtherCraft` and the
track-object bumps (`FUN_08918d30`) are dead on the shipped game and not ported.
`oag_pure::race::ORIGINAL_DRIVER` selects it; `original_opponent` is `true`, so a Pure
campaign event flies it. `crates/game/tests/pure_original_ai_ground_truth.rs` finishes
the field on Vineta K and Chenghou Project.

## Open

- **Not measured live**: the steering's output, the pitch fade, the `LeadZone` branch
  (the parked player never led), the speed floor and the `35` floor.
- **The `driver+0x80` flag** (`x 0.7` thrust and a lane wander) is set by a craft state
  at `craft+0xd0 -> +0x134` that is unread; one live planner sample had it set. Never
  set here: chosen, not measured.
- **The 2 s fork hold** is not built; the original field on every title takes this
  project's own branch choice.
- **One sample** of 1,689 kept the controller's default `100` (Vector, 45.1 s, slot 5):
  read as the lookahead-failure exit, not chased.
- **The two `rand()` draws** (tweak, eight cruise speeds) use the seeded `Rng`:
  chosen, not measured.
- **Cross-title**: HD, 2048 and Omega ship `_kart` files whose text equals Pure's
  pair; no executable was found reading them and the reader was not run on them.
- **Pure's timestep is variable** (`dt * 60` read `1.0112` live); the planner runs at
  this engine's fixed 60 Hz.

## Next Steps

1. A PPSSPP run with the player teleported ahead (as `scripts/psp-ai-thrust.py
   --teleport` does on Pulse) to exercise `LeadZone` with
   `scripts/psp-pure-ai-thrust.py`.
2. Read `craft+0xd0 -> +0x134` (what sets the `x 0.7` flag) from `Ai_Update`'s step 1.
3. A windowed walk of a Pure campaign event to see the field from the player's seat.
