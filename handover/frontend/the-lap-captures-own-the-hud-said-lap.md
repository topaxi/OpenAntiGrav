# The lap capture's own "the HUD said `Lap 2 of 3`" claim is now in tension with the finding that closed this row, and nobody has reconciled them

2026-08-19. The sweep [oag-trace.md](../../docs/tools/oag-trace.md#a-whole-lap-and-where-it-came-from) and `wall.rs`/`contact-response.md` owed since `lap_capture_ground_truth.rs` measured `talons-junction-time-trial-lap.csv` as 8.5% forward progress then a reversal, zero completed laps, is done - all three now say so, sourced to that test's own numbers rather than restated loosely. **What surfaced doing that sweep is a real, unresolved contradiction the sweep itself cannot settle.** The same doc section carries a second, independent claim: at tick 3,087 the game's own HUD read `Lap 2 of 3`, taken as a live observation during the capture session - direct evidence the original completed a lap on this run. A file that never gets past 8.5% of the circuit before reversing away from the line cannot be the same run whose HUD read a completed second lap. One of three things is true and nothing here says which: the committed CSV is not the run the HUD reading came from (a different, shorter save under the same name); the tick-3,087 alignment between "when the HUD was read" and "this file's own row 3,087" is wrong; or `lap_capture_ground_truth.rs`'s measurement has a bug of the same shape its own doc comment says an earlier draft did (the wrap-around progress trap). Needs whoever re-opens this to either find the original capture session's raw log (if it still exists) or re-run the autopilot capture fresh and compare. Until then: neither the "8.5%, reversal, no lap" reading nor the "Lap 2 of 3" reading should be treated as the settled account of what this file contains - only the two things not in dispute survive, the start-line ring-point match and the wall-contact percentage (which does not depend on direction of travel).

## Open

- The committed CSV may not be the run the "Lap 2 of 3" HUD reading came from - it could be a different, shorter save under the same name
- The tick-3,087 alignment between when the HUD was read and this file's own row 3,087 may be wrong
- `lap_capture_ground_truth.rs`'s measurement may have the same wrap-around progress bug an earlier draft of it had

## Next Steps

- Find the original capture session's raw log, if it still exists
- Otherwise, re-run the autopilot capture fresh and compare against the committed CSV
