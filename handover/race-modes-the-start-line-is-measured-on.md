# Race modes: the start line is measured on one circuit only

Time trial, speed lap and Zone run. `Course::START_LINE_OFFSET` is fitted on `16_Track`; the authored `Start Position` frame would replace it at confidence 88 across all 40. Two guards exist because both failed in practice: `RaceState::lap_gate` requires the near half *then* the far half of a lap (the ship spawns behind the line, so its first crossing is a wrap, and rocking over the line would otherwise record an unbeatable best lap), and lap 1's clock starts at the line rather than at the standing start.

**2026-08-30: one of the three ways to retire the constant is dead.** `Course::path_boundaries()` landing on the visible start line across circuits was checked directly - a sweep of all 40 Pulse circuit files' load reports, comparing the nearest path boundary to the true start line. Result: 11.6 to 1878.4 units apart, median 648, only 4 of 40 under 100 units. Not "the boundary is the line" - a path split is authored for its own reasons, unrelated to the grid. See `docs/gameplay/lap-counting.md` ("The lead that was checked and killed") and `crates/race/src/course.rs`'s updated doc comments on `START_LINE_OFFSET` and `path_boundaries`. No code changed - `START_LINE_OFFSET` still stands, at the same confidence 65 it had before, because this only closed off one retirement path, not the open question itself.

## Open

- `Course::START_LINE_OFFSET` is fitted on `16_Track` only, still not verified across the other 40 circuits. Two retirement paths remain, both needing something this session's data can't supply: the unread code that lays out the grid (`docs/formats/track.md`, "How a ship gets its grid slot" - needs a Ghidra project open on `pulse-psp-usa`), or a live capture of a standing start on a second circuit (needs PPSSPP capture tooling, `scripts/psp-drive.py`).
- Lap 1's clock starts at the line rather than at the standing start

## Next Steps

- Find `FUN_0894d440`'s caller chain or whatever computes the grid layout in `pulse-psp-usa` `BOOT.BIN`, per `docs/ghidra/functions/psp-pulse-usa/grid.md`'s "What is still open" - if it also computes the start line, that retires `START_LINE_OFFSET` at higher confidence than a fit. Needs a Ghidra project open; not done this session.
- Alternatively, capture a standing-start time trial on a second circuit (Moa Therma was only driven and eyeballed, not captured) to get a second measured distance and see whether it agrees with 137.9.
