---
status: ready-to-wire
titles: [all]
updated: 2026-10-31
summary: "Picker, campaign open and cell launch reads are off the frame thread; the opening read, unbuilt variants and Pure/HD campaign art remain"
categories: [rendering]
difficulty: easy (35)
---

# Menu screens still read the disc on the frame thread

2026-10-31 (`deck-menu-stalls`). Track Select and Ship Select no longer stall: the previews keep
their pipelines and a worker opens the source, reads the unlock gate and builds the first
previews while the menus are up. Numbers and attribution: `docs/rendering/frame-thread-cpu.md`,
"The race box's selection screens on the Deck". What is left breaks the maintainer's rule that
disc reads and parses leave the render thread, though none of it passes 50 ms on the Deck today.

## Open

2026-10-31 (`menu-io-remaining`): each selection's stills and model, the campaign's disc open, a
cell launch's `stats.xml` open and the teams' pipelines moved to workers (`docs/rendering/frame-thread-cpu.md`,
"The remaining menu reads off the frame thread"). Left:

- **A picker's opening read is joined on the frame thread** (13 to 17 ms on the Deck, 37 ms at 4K).
  Removing it needs the worker to prefetch the opening entry's `Read`, whose key (the unlocked
  skins, from the records) the worker does not know today, or the previous screen kept up until
  it lands (every `stage.picker = None` before an open would have to keep the old one).
- **A variant or skin the worker did not build.** It warms each team with no skin or variant; a
  hull whose variant needs another pipeline (HD per-circuit models, a Zone hull) compiles on the
  frame thread once. Not measured off Pulse.
- **The Pure chain's art player and HD's campaign screens** read the disc on the frame thread from
  their own archives. Only Pulse's campaign was walked.

## Next Steps

1. Walk Pure's SINGLE PLAYER chain and HD's campaign on the Deck with the doc's `deckwalk.sh`
   method (`OAG_PHASE_SPIKE_MS=0`, worst frame total per tap) and read the worst taps.
2. Prefetch the opening entry's read (see above) so the opening frame joins nothing.
