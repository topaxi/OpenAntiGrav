# HD's original opponent flies its campaign: what it does not do yet

2026-10-12, lane `orig-ai-hd`. HD's opponent is Pulse's driver function for
function with five gated branches; it is read, its thrust law reproduced on
83 RPCS3 samples at two levels, and every HD campaign race flies it.
Evidence: [ps3-hdfury-eu/ai-controls.md](../../docs/ghidra/functions/ps3-hdfury-eu/ai-controls.md)
(map, measurements, Omega) and [gameplay/ai.md](../../docs/gameplay/ai.md)
("The original driver, recovered", HD rows). The `.nnt` networks stay dormant
and untouched ([ai-net.md](../../docs/ghidra/functions/ps3-hdfury-eu/ai-net.md)).

Settled: `oag_ai::original::Laws` carries the branches (`Laws::PULSE` is Pulse),
`oag_hd::race::ORIGINAL_DRIVER` holds HD's with provenance, raceplay resolves the
halved-lookahead row off the disc's plugin definition, and HD's campaign skill
comes off the circuit's own `stats.xml` (the Fury cell `grid8_3_1` resolves the
`-0.07`/`3.48` HD logged). `crates/raceplay/tests/hd_original_ai_ground_truth.rs`
holds the figures.

## Open

- **Not exercised live**: the finished branches (spread `75`, wander to
  `8 - place`), Eliminator's dropped `WhenBehind`, the halved lookahead on
  Ubermall (Phantom) and Amphiseum reversed. Read at instruction level only.
- **Not ported** (named on the evidence page): the Eliminator spin block
  (`AI_UpdateCraft` step 11, a timer at `driver+0x28`), the Duel override and
  Duel rail gap, the split-screen multiplier, several humans' reference craft.
- **Frame order on HD**: where `AIManager_Update` falls against the world step
  is not measured (Pulse's is); HD's world-step function is not identified.
- **The steering, planner and pitch** have no live leg on either title.
- **Same formation gap as Pulse?** Not compared: the HD ground truth asserts
  the law's stops and the reference racer, not how fast the field closes.
- **Omega**: its executable inlines HD's driver (static census, the same
  windows); `RaceDefaults::original_opponent` stays `false` until its race is
  complete enough to fly it.
- Proposals for this project's own driver that the HD reading suggested
  (2026-10-08, none queued): a low-passed speed for the lookahead, a
  speed-scheduled lookahead and steer gain (DuelStats' `Input` rows),
  per-class gains, a clamp on cross-track correction, a three-point preview.

## Next Steps

1. A Fury Eliminator cell on RPCS3 with `scripts/rpcs3-hd-ai-thrust.py`: the
   `--predict` model already drops `WhenBehind` in modes `8`/`0x14`; a run
   there raises change 3 to measured.
2. A Phantom race on Ubermall forward: log `driver+0x8b4` and the lookahead
   time near the line (a stop at `0x00100954` reading `f1`).
3. Let a capture run past an opponent's finish to exercise changes 1 and 2.
