---
status: ready-to-wire
titles: [all]
updated: 2026-10-31
summary: "Picker stalls fixed; each selection's stills and model read and the campaign screens' disc open still run on the frame thread"
categories: [rendering]
difficulty: medium (40)
---

# Menu screens still read the disc on the frame thread

2026-10-31 (`deck-menu-stalls`). Track Select and Ship Select no longer stall: the previews keep
their pipelines and a worker opens the source, reads the unlock gate and builds the first
previews while the menus are up. Numbers and attribution: `docs/rendering/frame-thread-cpu.md`,
"The race box's selection screens on the Deck". What is left breaks the maintainer's rule that
disc reads and parses leave the render thread, though none of it passes 50 ms on the Deck today.

## Open

- **Each selection reads on the frame thread.** `PickerStage::refresh_preview` reads the stills
  (`oag_game::preview::slideshow`, 8 to 10 ms on the Deck) and the model (1 to 7 ms) and extends
  the sprite sheet inline. Moving them to a worker needs the old preview and stills kept on screen
  until the new ones land, so the window never shows a hole where the original shows content
  (check a frame against the original if anything would show).
- **The campaign screens open the disc on the frame that opens them.**
  `Session::open_title_archives` (RACE CAMPAIGN, Pure's team gate in `pure_locked_teams`, a 2048
  DLC campaign cell) still calls `open_source` inline, about 90 ms on the Deck for a Pulse disc.
  `Session::menu_source` already holds the opened archives for the current source; those callers
  could take a clone of its `SharedArchives`.
- **The worker builds only the current circuit's and team's preview.** On another title (HD's
  per-circuit models, a Zone hull) a later entry may need a pipeline neither built; it then
  compiles on the frame thread once. Not measured off Pulse.

## Next Steps

1. Measure: `deck-walk` recipe in the doc section above, `OAG_PHASE_SPIKE_MS=0`, worst
   frame-phase total per transition; for the campaign, walk START, RACE CAMPAIGN instead.
2. Make `open_title_archives` use `Session::menu_source()` first, with the inline open as the
   fallback it is in `open_picker`.
3. Load a selection's stills and model on an `oag_thread::Task`, keep drawing the previous
   preview until it lands, then build the `Preview` on the frame thread with the kept pipelines.
