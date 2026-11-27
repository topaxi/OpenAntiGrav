---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-27
summary: "Open after the pause bloom and title faces: PS2 Ship Select heading face, title legal glyph pitch, Pure HUD digit edges, PSP pause focus phase"
categories: [rendering]
size: M (55)
difficulty: medium (50)
---

# What is left of the four worst menu pairs

Landed (`docs/tools/compare-page.md`, "The four worst front-end pairs, split by region"): Pulse's pause rows bloom, the title screen's two texts draw in their authored faces
and the legal line wraps at 360, the PS2 team pair fits the craft's sway phase. Pause 4.01 to 2.70, title 4.54 to 3.37, PS2 team 4.60 to 2.35 mean grey.

## Open

- **PS2 Ship Select heading** (`pulse-ps2/menu-team`, header box `(40,0,480,60)` 10.18 mean grey, 0.3 of the frame): the original's `SHIP SELECT` is smaller and heavier than
  ours (width 195 against 231 px of 1280), the `Title` role's `Pulse_14` face as on the PSP, where ours draws the `menu` face (`oag_pulse::frontend::PS2_MENU_SKIN` keeps `title_font: None`,
  a 2026-09-27 PCSX2 capture of `MAIN MENU` having read as the rows' face). Evidence is one frame; the flip moves every PS2 menu page's heading and the slots
  (`face_atlas_slot`), so it wants a before and after on all PS2 `menu-*` pairs and a PS2 `MAIN MENU` frame beside it. Inherited from PSP unless that frame says otherwise.
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

1. A PCSX2 frame of the PS2 `MAIN MENU` and of Ship Select's heading at 1:1, then flip `title_font` for the PS2 skin and re-render every PS2 menu pair (an afternoon, wire).
2. Fit the legal line's pitch against the frame (an hour to measure, then wire).
