---
status: re-first
titles: [omega, 2048]
updated: 2026-11-16
summary: "Grid craft box leftovers: radar dots (law read, unwired), Omega HD-mode proximity arc (gate undecoded), engine light consumer (not found)"
difficulty: medium (50)
categories: [frontend]
---

# Omega grid craft box: radar dots, the HD-mode proximity arc, the engine light

2026-11-16, lane `grid-ring-glow`. The white ring on the 2048-mode grid is the HUD radar and is drawn
(`docs/ghidra/functions/ps4-omega-eu/hud-radar.md`). Three things in the same box stay open.

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
- **Engine light consumer (brief item 2: the warm cast at the nozzle).** Loader on 1.07 is
  `FUN_01335030` (`EngineLightData.xml`: `Distance` to `ship+0xbc`, `Radius` to `ship+0xc0`; the base
  doc's `FUN_0130e090`). Float loads of `[reg + 0xbc]` (`search_instructions`, `VMOVSS`) are in
  `FUN_0103c000`, `FUN_01083a10`, `FUN_01083d80`, `FUN_012cb090`, `FUN_012d9190`, `FUN_01731ec0`,
  `FUN_0177e140`. `FUN_01731ec0` is ruled out (the cloud-sprite builder, `Wipeout_Clouds_C.gnf`).
  The rest are unread; a consumer may also read through `VMULSS`/`VADDSS` operands, which this
  search did not cover. The `engineflare2048` program pair is unread too.

## Next Steps

1. Engine light: decompile the six remaining readers and keep the one whose `+0xbc` base is a ship
   (it also reads `+0xc0`, and the ship's matrix); then a light position `ship + forward * Distance`
   with `Radius` falloff is the expected shape. Then HD's same law
   (`docs/ghidra/functions/ps3-hdfury-eu/`), then wire through `Title` data.
2. Radar dots: add the relative-position readout, wire the dots, and capture a 2048 frame with an
   opponent behind (Vita3K) to check the placement constant `DAT_0186e838`.
3. HD-mode arc: decode the group's state test in `FUN_0126b030` before drawing anything.
