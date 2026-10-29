---
status: measure
titles: [pulse]
updated: 2026-10-29
summary: "Text, tag, picker, Main Menu, title, end-of-race stamps wired; open: Track/Ship Select, tag text, title face, colour pulse, quad width, Pure"
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
- Track Select's `1628` at 255 and Ship Select's 20 at 247: no widget located; no EDRAM alpha for Cell, Track or Ship Select was read
  (phase fitted, not matched). Main Menu and the title are done (`docs/ui/campaign-screens.md`, "Main Menu, the title and the fills").
- The title's `BOOT_PRESS_START` draws in our `Default` face; the original uses `font="Menu"` (uppercase, wider: its stamp box is
  `(242,219,461,239)`, ours spans the right 124 px), so the press-start box does not score better yet. Routing the boot frame's text
  through a face role is the lead; the stamp follows the glyphs once it is.
- The text colour pulse (`FUN_088cd2b8`, toward white by `s / 150`) is not drawn as the original's stepped ladder: ours is a raised
  cosine. The main-menu frames carry the bloom in the ink, so they cannot give the law; a frame read before the bloom (the scene
  buffer, not the displayed one) can. `Skin::selected_stamp` follows that cosine meanwhile (chosen, not measured).
- The selected Main Menu row's stamp phase against the screen's opening, and whether the other `<Menu>` pages (Racebox, options) stamp
  the same constants, are unmeasured. Scrollbars (`RealGlow="128"`) and the full-screen `RealGlow="1"` fills sit on screens ours draws
  from `menu.toml`, so `Fill::glow_stamp` is wired only for the end-of-race `tablehighlight` (no EDRAM frame of it was taken).
- `FUN_0889812c` still stamps `(1 - f) * 150 + 70` for an `Image` with `+0x2c & 0x200`; the `Selector`'s issuer is not located.
- Pure's front end: no tag glow is authored; whether it stamps anything is not measured (two EDRAM attempts landed on PPSSPP's own
  memory-stick `Load` dialog). PS2 draws these stamps through the shared title record, uncompared; the Main Menu constants are
  inherited from PSP there.

- One `s` runs on two periods now: `Text::glow_stamp` steps its ladder at 20 per second (1.0 s) while the title's text throb is the
  measured 1.1 s `PULSE_PERIOD` and the Main Menu's selected stamp rides the 1.1 s colour pulse; the title halo drifts against its text.
- The title and Main Menu gate on `MenuSkin.glow` (Pulse only), while the picker/footer stamps gate on `Campaign::frontend_glow`: two
  per-title sources for one fact, a size-ratchet choice (`boot.rs`), to rule on at merge.
- The end-of-race `tablehighlight` stamp is emitted but not drawn: the results screen overlays with `LoadOp::Load`.

## Next Steps

1. EDRAM alpha dumps (`scripts/psp-glow-mask-dump.py` style) of Cell, Track and Ship Select at known pulse levels, then phase-match their scores (an afternoon).
2. Route `BOOT_PRESS_START`'s `font="Menu"` through the boot frame (an hour), then re-score the title box.
3. Find the glyph-quad width rule from the `1` and `R I` runs of the `GRID 1` mask (an hour).
4. Draw the tag's text when its source is found (not this thread).
