---
status: ready-to-wire
titles: [2048]
updated: 2026-11-05
summary: "2048 pad focus landed; left: controls ours does not draw, a sequence form of --press, a pad-aware compare route"
size: S (15)
difficulty: easy (20)
---

# 2048's front end: what is left of controller focus

Landed (`docs/formats/2048-frontend.md`, "Controller focus"): the original answers no pad on its touch
screens (executable and Vita3K both read), so the pad focus is the tile's own pressed colour,
`Orange2048`, on every grid, with Up/Down, Cross and Circle chosen. Per-screen tests are in
`crates/game/tests/vita_2048_focus_ground_truth.rs` and `crates/ui/src/frontend/tests/focus.rs`.

## Open

- Controls ours does not draw, so the pad cannot reach them: `OptionsAudio`'s `Music Sequential` and
  `Music Random` toggles, `OptionsControls`' `FE_CTRL_VIEW_CONFIG` tile, `2048extras`' AR Museum
  (`<aTouchButton>` typo), `team`'s `ar_view`.
- `--press` is one mask, so `options`, `profile` and `2048extras` have no headless still with a focus;
  the walk is only in the ground-truth tests. A sequence form of `--press` would give the compare
  page those pairs.
- `--original-menu` does not hide the pad focus, so a pair taken with `--press` differs from the
  original by the focused tile (the pair records say so).
- `TouchButton`'s `+0xfc` flag (also draws `Orange2048`) is unread.

## Next Steps

1. Read `FUN_810edc10`'s `+0xfc` writer and the `Touchlist` selected look on Vita3K.
2. Draw the toggles above from their `TouchButton` widgets, then they join the pad's tile list.
