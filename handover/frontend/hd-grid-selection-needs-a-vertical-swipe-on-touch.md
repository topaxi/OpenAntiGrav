---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-05
summary: "HD and Fury Grid Selection cannot switch grids on a touch device; a vertical swipe should step them, Omega to be checked"
size: S (60)
difficulty: easy (60)
---

# HD's Grid Selection needs a vertical swipe on touch

Reported from a touch device: on the HD and Fury campaign's `Grid Selection` a player cannot switch
to another grid. Omega probably shares it, unchecked. The wanted control is a swipe up or down to
step between grids.

## Open

- `GridSelection::hd_pointer` (`crates/ui-screens/src/campaign/hd.rs`) reads `scroll`, `clicked` and
  `back`, and ignores `Pointer::drag`, so a finger's drag does nothing here. `step` is already what
  the scroll wheel and the arrows call.
- The arrow targets come from the images `Flyer Left Arrow` and `Flyer Right Arrow`
  (`hd_grid_targets`). Why they do not serve a touch player is unread: a missing image, a rect too
  small for a finger, or a card rect or arrow overlap. Find out, since it may be a second bug.
- Which way is next, and how far a swipe must travel, are chosen, not measured: the original is
  played on a pad. A swipe must step once per gesture, and must not also confirm the card.

## Next Steps

1. Reproduce with a headless pointer test on HD's layout, and confirm whether Omega takes the same path.
2. Accumulate the gesture's vertical `drag`, step once past a threshold, rearm on lift.
3. Test: a swipe each way steps one grid, a long swipe steps once, a tap still confirms, a horizontal
   drag steps nothing. Add the gesture to `docs/ui/campaign-screens.md`, then delete this thread.
