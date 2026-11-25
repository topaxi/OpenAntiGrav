---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-25
summary: "AUTO reflection resolution: step the glass and sea target down by its own timed cost before the render scale moves; the S21 gains 17 ms at quarter"
categories: [rendering]
size: M (50)
difficulty: medium (50)
---

# Reflection resolution AUTO follows the timed reflection pass

## Open

REFLECTION RESOLUTION (`original`, `half`, `quarter`) is a fixed choice, sized at load. On the S21 (Mali-G78), HD Vineta K at render scale 25 runs 41.9 ms median under `original` and 24.9 ms under `quarter`, back to back from the same temperature (`docs/tools/android.md`, "Performance on the Galaxy S21"). On a desktop RDNA GPU the same pass costs about a millisecond. The maintainer wants it scaled automatically on hardware where it costs a lot, and left at full size where it does not: decided by measured time, never by platform.

## Next Steps

Start after per-pass timing has merged, so the reflection target's pass has its own GPU time (`GpuCost`, the overlay's row for the glass and sea target).

1. **Resizable without a reallocation.** Allocate the target at `original` and draw `half`/`quarter` into a sub-viewport, the way dynamic resolution varies a viewport and not an allocation (ADR-0037). Sampling scales the UVs to the used rect.
   - Clamp at the rect's edge, so no bleed from stale texels outside it.
   - The fixed `half`/`quarter` choices go through the same path.
2. **An `auto` value, the new default for REFLECTION RESOLUTION**, with `string_id` and English text; `original` stays selectable. Photo shots always draw the full-size target (maintainer).
3. **The policy, in `oag-present`'s controller, generic:**
   - **Input:** the timed reflection pass in ms and its share of the GPU frame.
   - **Stepping down:** when the frame is over budget and the reflection share is above a threshold, step reflections down (original, then half, then quarter) **before** the render scale moves.
   - **Stepping up:** when there is headroom, with hysteresis and a hold time like the render scale's.
   - **Thresholds:** write them down with how they were chosen. Starting point: step down while its share exceeds 15 % and the frame misses budget.
   - **With no timestamps** (`Timing::can_time_a_pass` false): `auto` stays `original`, and the loader report says so.
4. **Report it:** the current step on the perf overlay and in `--frame-log`; the benchmark result records it.
5. **Tests:**
   - the policy as a pure function over a sequence of readings: a desktop-like cost never leaves `original`; a Mali-like cost reaches `quarter` before the render scale moves; hysteresis prevents oscillation;
   - the sub-viewport sampling: a quarter rect reads the same texels as a quarter-size allocation;
   - photo shots use the full target.
   - Break each test.
6. **Phone check:** S21, HD Vineta K, render scale 25 under `auto`. It should settle at quarter and match the fixed quarter's numbers within noise. Desktop: stays original.

The lineage check is HD, 2048 and Omega: each ships the glass/sea target per `docs/rendering/water.md`; say which ones the step applies to.

**7. An `off` value, only if it is worth it (maintainer, 2026-11-25: "add only if significant fps gain (~4-5 fps on S21)").**
- **Measure first.** On the S21 (HD Vineta K, render scale 25, same protocol as the quarter run in `docs/tools/android.md`), compare `quarter` against the pass skipped (`behind_glass.rs` already has an early exit, `if !draw`).
- **Add it only if** `off` beats `quarter` by about 4 fps or more on average. Otherwise record the measured numbers in `docs/rendering/water.md` and do not add the value.
- **If added:**
  - It is a manual choice only; `auto` never goes below `quarter` (losing the sea mirror changes the look, not just the sharpness).
  - The glass and sea sample a cheap stand-in (the sky, or the material's own tint), not an empty target.
  - `string_id` and English text, and a test that the pass is not encoded under `off`.
