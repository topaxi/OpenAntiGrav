---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-27
summary: "Open: PS2 per-heading scale and 0.8 pen law, title legal glyph pitch, Pure HUD digit edges, PSP pause focus phase"
categories: [rendering]
size: M (55)
difficulty: medium (50)
---

# What is left of the four worst menu pairs

Landed (`docs/tools/compare-page.md`, "The four worst front-end pairs, split by region"): Pulse's pause rows bloom, the title screen's two texts draw in their authored faces
and the legal line wraps at 360, the PS2 team pair fits the craft's sway phase. Pause 4.01 to 2.70, title 4.54 to 3.37, PS2 team 4.60 to 2.35 mean grey.

## Open

- **PS2 per-heading authored scale**: `Additional`'s `FE_OPT_PLUS`/`FE_EXTRAS` author no `scale` (1.0), ours draws every heading at `title_scale` 0.8. `menu-options` header 8.11 to
  9.50 since the face flip (7.57 at 1.0). Needed on the live pages (`menu.toml`) as much as on the compare-only ones, so a compare-only fix is not enough
  (`docs/ui/menus-original.md`, "Pulse PS2's headings draw in the Title face").
- **Fractional-scale pen law**: PS2 `Pulse_14` headings at 0.8 run 3.4 % short of the original (mean advance 15.73 against 16.27 px), exact at 1.0. The same shape as the legal line below
  (0.7, 1.07 to 1.08 wider). The existing mechanism to try is `Renderer::set_whole_pixel_advances` (`round(advance * scale)`, Pulse's HUD).
- **Stroke weight**: the original's PS2 heading glyphs carry a dark outline about 1 px wide at 2x that ours lacks.
- **Title legal line's glyph pitch** (`pulse-psp/menu-title`, legal box `(0,244,420,28)` 17.9): wrapped right now, but the original's `Pulse_14` glyphs at `scale 0.7` are about 1.07 to 1.08
  times wider and its lines 11 px apart against ours 12.3 (a hand 1.08 x scale takes the box to 14.7, not to the press-start box's 7.8). The pen law (`bearing_pens`) or a whole-pixel quad
  rounding as Pure's `snapped_quad` is the lead (`docs/architecture/frontend-boot.md`, "Show Logo's faces and its legal line's wrap").
- **Title backdrop**: ours draws light rays across the top left that the original's frame lacks (`pulse-psp/menu-title`, top boxes 2.2 and 2.6); the backdrop movie's frame was never fitted for this pair
  (`compare-menu-movie-phase.py`).
- **Pure's race HUD** (`pure-psp/menu-pause`, `pure-psp/vineta`): the clock and bronze-time digits differ at their edges (11.4 and 12.5), and the pennants behind them move. A HUD
  and race lane, not the pause. The lap counter is closed (`docs/tools/compare-page.md`, "Pure's lap-one close-up").
- **The focused row's phase (PSP)**: the focused row's stamp rides the colour pulse and the pause drive route does not pin it (`--menu-focus-level`).
  The PS2 pause is measured and closed: its rows write no alpha, so it blooms nothing (`docs/ui/campaign-screens.md`, "The Selector's sprite is the only alpha writer
  of Pulse PS2's front end"); no PS2 pause pair exists to score.

## Next Steps

1. Per-heading authored scale, and the fractional pen law on the same lane (about 2 h, wire).
2. Fit the legal line's pitch against the frame (an hour to measure, then wire).
