---
status: re-first
titles: [pulse, pure]
updated: 2026-11-27
summary: "Open: PS2 heading outline, title legal line pitch and backdrop, Pure HUD digit edges, PSP pause focus phase"
categories: [rendering]
size: S (50)
difficulty: medium (50)
---

# What is left of the four worst menu pairs

Landed (`docs/tools/compare-page.md`, "The four worst front-end pairs, split by region" and "Pulse's heading scales and fractional pens"): Pulse's pause rows
bloom, the title screen's two texts draw in their authored faces, the PS2 team pair fits the craft's sway phase, PS2 headings draw at their authored scale and
Pulse's pens step to whole units. Pause 4.01 to 2.70, title 4.54 to 2.64, PS2 team 4.60 to 2.21 mean grey.

## Open

- **Stroke weight**: the original's PS2 heading glyphs carry a dark outline about 1 px wide at 2x that ours lacks. Scales, faces and every glyph's
  left edge match (`docs/ui/menus-original.md`, "Pulse's text pens step up to whole units"), so what is left of the PS2 header boxes (6.6 to 8.1 mean
  grey) is the ink itself.
- **Title legal line's second row** (`pulse-psp/menu-title`, legal box 9.8): the original's lines are 11 px apart, ours 11.9, so our second line is 1 px
  low. Whether the line step or the glyph quad's `y` is floored is not read (`docs/architecture/frontend-boot.md`, "Show Logo's faces and its legal
  line's wrap").
- **Where a line with a fractional `x` steps** (centred, right-aligned, scrolling text): ours counts the whole-unit step from the text's own `x`,
  chosen, not measured.
- **Title backdrop**: ours draws light rays across the top left that the original's frame lacks (`pulse-psp/menu-title`, top boxes 2.2 and 2.6); the backdrop movie's frame was never fitted for this pair
  (`compare-menu-movie-phase.py`).
- **Pure's race HUD** (`pure-psp/menu-pause`, `pure-psp/vineta`): the clock and bronze-time digits differ at their edges (11.4 and 12.5), and the pennants behind them move. A HUD
  and race lane, not the pause. The lap counter is closed (`docs/tools/compare-page.md`, "Pure's lap-one close-up").
- **The focused row's phase (PSP)**: the focused row's stamp rides the colour pulse and the pause drive route does not pin it (`--menu-focus-level`).
  The PS2 pause is measured and closed: its rows write no alpha, so it blooms nothing (`docs/ui/campaign-screens.md`, "The Selector's sprite is the only alpha writer
  of Pulse PS2's front end"); no PS2 pause pair exists to score.

## Next Steps

1. The legal line's line step: read `Text_Draw`'s wrap loop (an hour), then wire.
2. The title backdrop's movie frame: fit with `compare-menu-movie-phase.py` (an hour).
