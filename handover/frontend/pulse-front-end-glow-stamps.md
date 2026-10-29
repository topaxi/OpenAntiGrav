---
status: ready-to-wire
titles: [pulse]
updated: 2026-10-29
summary: "Front-end bloom runs for the Selector; open: card title, footer tag, RealGlow from XML, other screens, Pure"
---

# Pulse front-end glow stamps

The mechanism landed with `pulse-frontend-bloom` (`docs/ui/campaign-screens.md`, "The front end's bloom, wired for the cursor"):
`Draw::GlowStamp { rect, value }`, `Renderer::render_with`'s scratch pass, `oag_title::Campaign::frontend_glow`. Only the
`Selector` stamps today (`80 + 150 t`, measured), so the cursor box matches the original to 0.3 grey on 11 of 12 phase-matched
frames. Everything else the original stamps is read and not wired.

## Open

- The card title: a pulsing `70..220` in 15 steps (a `Text` with the pulse flag, `(220 - s) * PulseMultiplier`, `s` stepping 15
  per 1/20 s in 0..150), 1120 pixels on Grid Selection. Needs `Draw::Text` to carry its stamp or the screen to emit a stamp over the
  text's glyph quads (the union of the per-glyph quads, not the line box: 1120 px, not 63x20).
- The footer tag `NewsItemTagText`, `RealGlow="64"`, 392 px (28x14) on every Pulse screen.
- `RealGlow` is authored (`128` x16, `1` x6, `100` x4, `64` x1 in `Data.wad`'s front-end `.cod`) and our loader does not read it:
  read it onto `Text`/`Image` widgets and emit the stamps from the data, not from per-screen code.
- Main Menu (`14526` px at 110 and two more regions), Track Select (`3958` px cycling, `1628` at 255), Ship Select (`1680` cycling,
  `20` at 247): regions measured, widgets and laws not identified.
- Where the code issues the `Selector`'s ref: `FUN_0889812c` stamps `(1 - f) * 150 + 70` (70..220) for an `Image` with flag
  `0x200`, which is not the 80..230 read.
- Pure's front end: the constants are the same (`BOOT.BIN` byte search); whether and where it stamps is not measured.
- Pulse PS2 shares the title record and now draws the cursor bloom; its original shows a halo too, but the PS2 chain's strength is not compared.
- Not seen in a window: the live `menu_stage` path was read (one `render_with`, Clear) but a lavapipe Xvfb grab came back black.

## Picker clock (2026-10-29, pulse-picker-clock)

Ship Select (authored `Mode3D` pose plus the hull's own sway) and Track Select (0.70 s step cross-fade) are wired and clocked off the
game's animation clock; PS2's craft placement is unmeasured (keeps the old orbit, `craft_mode3d_applies`). Open: the footer ticker
(scrolls in the original, empty in ours at tick 0) on the same clock; the Assegai `Classic` arrows and the footer tag differ in the pair.

## Next Steps

1. Read `RealGlow`/`PulseMultiplier`/`pulse` onto `oag_ui::screen`'s `Text` and `Image` (15 minutes to find the parser, an afternoon
   to thread the glyph quads through `Draw::GlowStamp`).
2. Emit the footer tag and card title stamps and re-score Grid/Cell pairs over the whole frame, not the cursor box.
3. Take alpha histograms of Pure's front end (`scripts/psp-glow-mask-dump.py` style, `memory.read` at `0x04000000`) before wiring it.
