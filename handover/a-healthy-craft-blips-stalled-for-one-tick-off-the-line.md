# A healthy AI craft now reads as stalled for exactly one tick, likely at the countdown release

`crates/game/tests/stall_rescue_ground_truth.rs::a_healthy_craft_never_looks_stalled_for_a_single_tick`
is deterministically red as of 2026-09-05, reproduced identically across two full
`just test-data` runs (one contended at load 22-29, one on a quiet machine)
taken minutes apart - not a flake, not an environment gap:

```
ace 16_Track: longest stall 1 ticks, laps 3, respawns 0
assertion `left == right` failed: a healthy craft on 16_Track looked stalled for 1 consecutive ticks, which is the margin this threshold rests on
  left: 1
 right: 0
```

Note this is a **different** test from `no_circuit_sustains_a_stall_past_the_rescue_threshold`
in the same file, which still passes (asserts `longest_stall < STALL_TICKS`,
i.e. `< 120`; this one asserts `longest_stall == 0` exactly, "the margin this
threshold rests on" per its own doc comment). Do not confuse the two when
searching for prior context - HANDOVER.md's extensive stall-rescue history
(the "bounce in place" gap on `05_Track`/`07_Track` novice, closed
2026-09-02) is about the sustained-stall test, not this one, and does not
cover this failure.

## Leading hypothesis

`cc395862` ("fix(game): hold AI opponents at the line through the start
countdown", 2026-09-02) made `Race::step_opponents` gate every opponent's
throttle to zero for `COUNTDOWN_TICKS` (272 ticks) at race start, matching the
newly-measured original. This test's own stall predicate, restated in
`solo()` rather than calling the private one it mirrors:

```rust
let stalled = ship.physics.thrust > 0.0
    && !ship.standing.finished()
    && ship.physics.body.linear_velocity.length() < race::STALL_SPEED;
```

During the 272-tick hold, `thrust == 0.0`, so `stalled` is correctly `false`
throughout - the hold itself cannot be what's counted, since `thrust > 0.0` is
required. But **the instant thrust releases**, there is necessarily one tick
where `thrust` has just gone positive again while `linear_velocity` hasn't
risen off (near) zero yet - which is exactly a one-tick "stalled" reading by
this predicate's own definition. That fits the observed symptom exactly:
precisely 1 tick, not a sustained run, on a specific circuit/difficulty
combination (only `16_Track` at `ace`, not `03_Track` or `06_Track` in the same
sweep) that plausibly has the slowest post-release acceleration of the three
tested.

**This is a hypothesis from reading the two code paths together, not a
confirmed root cause.** I did not instrument the actual tick-by-tick data to
confirm the transition happens exactly at `COUNTDOWN_TICKS` on this circuit.

## Why this matters for the fix, if the hypothesis holds

If confirmed, this is **not a physics bug** - it's the test's own premise
("a healthy craft never even touches the stall predicate for a single tick")
running into a countdown-hold mechanic that didn't exist when the test's
`== 0` assertion was written. That's the exact shape of failure the task
brief for this baseline named as a known pattern this week: a countdown
change silently invalidating a warm-up-shaped test premise. The fix, if this
is confirmed, is very likely to the **test's** margin (e.g. start counting
from `COUNTDOWN_TICKS + 1` the same way `shuriken_ground_truth`'s
`WARM_UP_TICKS` already does, or exempt the single release-tick explicitly),
not to `oag_race`'s stall predicate or `cc395862`'s countdown gate - both of
which are independently correct in isolation.

**Do not lower `STALL_TICKS` or relax the stall predicate to make this pass.**
If the hypothesis is right, the predicate is behaving exactly as designed; only
the test's zero-tolerance margin is stale.

## Open

- Confirmed only by reading, not by tracing actual tick data. Need `thrust` and
  `linear_velocity.length()` logged for ticks `COUNTDOWN_TICKS - 2` through
  `COUNTDOWN_TICKS + 5` on `16_Track` at `ace` to confirm the stall reading
  lands exactly at the release tick.
- **`16_Track` is not evidence of anything circuit-specific - re-check this
  before reading anything into it.** The `assert_eq!` sits *inside* the
  `for track in ["16_Track", "03_Track", "06_Track"]` loop, so the panic on
  `16_Track` (first in the array) stops the test before `03_Track` or
  `06_Track` ever run - the log shows exactly one `println!` line before the
  failure. There is no evidence yet that the other two tracks don't have the
  same one-tick blip, or a different one. If the transition-tick hypothesis is
  right, it should reproduce on all three, since nothing about it is
  circuit-specific; reorder the array or (better) run each track as its own
  assertion so one failure doesn't hide the other two's results.
- Whether the *player's* thrust gate (`Race::tick`, gated identically since
  before `cc395862`) has the same one-tick edge and simply isn't exercised by
  any test that would notice it.

## Next Steps

1. Add a print of `(tick, thrust, speed)` inside `solo()`'s loop for the ticks
   around `COUNTDOWN_TICKS` on `16_Track` ace, confirm the transition-tick
   theory.
2. If confirmed: adjust this test's own margin to account for the one
   unavoidable release-tick, the same way `shuriken_ground_truth::WARM_UP_TICKS`
   already adds `+ 120` past `COUNTDOWN_TICKS` rather than starting exactly at
   it. Consider whether `STALL_TICKS`'s own definition should explicitly note
   this as a known, harmless one-tick artifact of the countdown gate.
3. If **not** confirmed (the transition-tick theory doesn't hold): re-open this
   as a genuine physics or AI regression and look at what else changed in
   `crates/ai` or `crates/physics` since 2026-09-02's fix landed.
