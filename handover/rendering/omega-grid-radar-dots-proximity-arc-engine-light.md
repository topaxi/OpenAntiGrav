---
status: re-first
titles: [omega, 2048]
updated: 2026-11-16
summary: "Grid craft box leftovers: radar dots (law read, unwired), Omega HD-mode proximity arc (gate undecoded)"
difficulty: medium (50)
categories: [frontend]
---

# Omega grid craft box: radar dots, the HD-mode proximity arc, the engine light

2026-11-16, lane `grid-ring-glow`. The white ring on the 2048-mode grid is the HUD radar and is drawn
(`docs/ghidra/functions/ps4-omega-eu/hud-radar.md`). Two things in the same box stay open; the engine light moved to its own thread.

## Open

- **Radar dots (`RadarDot0`-`7`).** Law read on Omega's 1.07 `FUN_012d3c50` (confidence 75), written on
  the evidence page, section "The dots: read, not wired". Needs a readout carrying each other craft's
  position relative to the player (right, forward, distance) and a draw that moves and fades the eight
  dot sprites. Not checked on 2048's own update (the range `hud+0x6dc` has no float-load hit by
  `search_instructions`; the update reaches it through another base register). No capture with an
  opponent behind the player exists to check against.
- **Omega HD-mode proximity arc.** Omega's HD-mode grids (`omega/the-amphiseum`, `omega/modesto-heights`)
  draw `ProximityBar` (`Data\XML\hud_proximity.xml`); HD's own grids do not. Bound at `hud+0xc30` by 1.07
  `FUN_0127c860`, hidden there, shown in a widget group by `FUN_0126b030` (`0x01278fb6`, `0x012792cb`)
  under a state test on `DAT_01a78e98[0x3364]` and `+0x17861` that is not decoded. The `Prox0`-`4`
  segments' law (red lights) is also unread (`hud+0x968..0x990`, read near `FUN_0126b030` lines on
  `+0x970`/`+0x990`). Do not wire on HD.
- ~~Engine light consumer.~~ Read 2026-11-16 (`engine-glow-re`): a vertex light per craft, in
  [its own thread](omega-and-2048-engine-light-is-a-vertex-light.md).

## Next Steps

1. Radar dots: add the relative-position readout, wire the dots, and capture a 2048 frame with an
   opponent behind (Vita3K) to check the placement constant `DAT_0186e838`.
2. HD-mode arc: decode the group's state test in `FUN_0126b030` before drawing anything.
