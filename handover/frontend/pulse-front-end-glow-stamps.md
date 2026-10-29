---
status: measure
titles: [pulse]
updated: 2026-10-29
summary: "Text glow, tag and picker stamps wired; open: EDRAM alpha for Cell/Track/Ship, Main Menu regions, tag text, title, Pure"
---

# Pulse front-end glow stamps

Wired by `pulse-glow-stamps` (`docs/ui/campaign-screens.md`, "The front end's bloom, wired from the widgets' own attributes"):
`Text` reads `RealGlow`/`PulseMultiplier`/`pulse`, `Draw::GlowText` stamps a text's glyph quads, the footer tag stamps its authored
`RealGlow`, Grid/Cell/Track/Ship Select texts emit from the data.

## Open

- The footer tag draws no text: the original fills `NewsItemTagText` at run time (`AAA` in every capture) and stamps those glyph
  quads; ours stamps the measured 28x14 box over an empty tab, so the tag box still scores 32.5 against 47.3 before.
- Glyph quads: the original's are ours grown one pixel left and up, but still 5 columns wider across `GRID 1` (56 vs 51); the cell
  advance or the `1`'s quad is the lead. One string measured.
- Main Menu's regions (`14526` px at 110, `3652` at 128, `2674` at 115), Track Select's `1628` at 255 and Ship Select's 20 at 247: no widget located; no EDRAM alpha for Cell, Track or Ship Select was read this lane (phase fitted, not matched).
- The title screen's `BOOT_PRESS_START` and `BOOT_PRESS_ANY2` are `pulse="true"`, so the original stamps them (not measured on the title); `frontend::draw` draws them through its own path.
- The text colour pulse (`FUN_088cd2b8`, toward white by `s / 150`) is not drawn.
- `FUN_0889812c` still stamps `(1 - f) * 150 + 70` for an `Image` with `+0x2c & 0x200`; the `Selector`'s issuer is not located. `Image` `RealGlow` is authored (scrollbars, table highlights, full-screen fills) and not read.
- Pure's front end: no tag glow authored; whether it stamps anything is not measured. PS2 draws these stamps through the shared title record, uncompared.

## Next Steps

1. EDRAM alpha dumps (`scripts/psp-glow-mask-dump.py` style) of Cell, Track and Ship Select at known pulse levels, then phase-match their scores (an afternoon).
2. Find the glyph-quad width rule from the `1` and `R I` runs of the `GRID 1` mask (an hour).
3. Draw the tag's text when its source is found (not this thread).
