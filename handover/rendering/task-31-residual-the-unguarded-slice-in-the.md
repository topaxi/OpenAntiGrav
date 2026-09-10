# Task #31 residual: the unguarded `slice(..)` in the race's draw path

`oag-view --collision`'s panic on an empty vertex buffer is fixed. **Corrected 2026-09-10 (thread audit note): the premise below was already stale the day this thread was split out.** `Drawable::draw` was never left deliberately unguarded - commit `cec399e7` (2026-08-16, splitting the 11,294-line race file into its own seams) added an empty-model early return immediately before the `slice(..)` calls, and that guard was already present when this thread was created (2026-08-25). It is confirmed still present today at `crates/game/src/race/drawable.rs:552-554` (the function moved from line 275 to 535 in the same split; the old line number was never updated). The `orbit` guard is the one still genuinely without coverage - it needs a window.

## Open

- The `orbit` guard (`crates/view/src/orbit.rs`) has no test coverage - `crates/view/src/orbit.rs` has zero `#[test]` in the file

## Next Steps

- Add coverage for the `orbit` guard - it needs a window
