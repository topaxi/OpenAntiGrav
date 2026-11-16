---
status: ready-to-wire
titles: [all]
updated: 2026-11-06
summary: "Opening read is off the frame thread; Grid Selection is the worst menu tap; Deck rounds 2 and 3, Pure and HD walks not taken"
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

2026-11-06 (`menu-read`): a picker's opening read is off the frame thread
(`docs/rendering/frame-thread-cpu.md`, "A picker's opening read"). Left:

- **RACE CAMPAIGN's Grid Selection is now the worst menu tap on the Deck**: 11 ms at 800p, 12 at
  1080p, 17 to 20 at 4K, the first open and each Back from Ship Select, unchanged by the picker
  work. Not split into read and upload; none of it passes 50 ms.
- **Only one complete Deck round** was taken (A, B, A/A at three sizes): the maintainer's game was
  on the Deck for the other two.
- **A variant or skin the worker did not build.** It warms each team with no skin or variant; a
  hull whose variant needs another pipeline (HD per-circuit models, a Zone hull) compiles on the
  frame thread once. Not measured off Pulse.
- **The Pure chain's art player and HD's campaign screens** read the disc on the frame thread from
  their own archives. Only Pulse's campaign was walked. Pure's team gate (`pure_locked_teams`)
  also reads on Ship Select's opening frame when nothing is unlocked by `--unlock-all`.

## Next Steps

1. Finish the Deck measure when the Deck is free: the exact walks and command are at the end of
   `docs/rendering/frame-thread-cpu.md`, "A picker's opening read". Compare, within one session,
   before, after and a copy of before, rotated, at 800p, 1080p and 4K, worst frame `total` per tap.
   Baseline: 13 to 17 ms, 37 ms at 4K in the 2026-10-31 walk; one round on this tree read 12.7, 13.9
   and 19.9 ms before and 3.5, 4.2 and 5.2 after.
2. Walk Pure's SINGLE PLAYER chain and HD's campaign the same way and read the worst taps. Not
   taken: a Pure and an HD tap script were written but never run, so none is recorded. Move a read off the frame thread only if it passes 50 ms; otherwise write its number here.
3. Split RACE CAMPAIGN's Grid open (`Session::open_campaign`) and say whether it is a read at all.
