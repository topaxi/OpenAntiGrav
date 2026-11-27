---
status: ready-to-wire
titles: [all]
updated: 2026-11-27
summary: "Screen filter look choices: psp-lcd stripe strength, psp-3000 line ripple; decided: barrel filters run at fixed 100 strength"
size: XS (60)
difficulty: easy (60)
---

# Screen filter look choices

The colour defects are fixed and measured:
[screen-filters.md, "Colour audit"](../../docs/rendering/screen-filters.md#colour-audit).

## Open

- `psp-lcd` `subpixel`: 0.25 (landed, white shows as 228, race frame luminance
  x0.82) or 0.5 (plainer stripe, white 206, x0.65).
- `psp-3000`: `static_lines` and `motion_lines` draw nothing at the default
  `cell = 1` (the cosine is zero at every pixel centre). Keep the look as it is
  and drop the two tunables' claim, or fix the phase so rows alternate by
  0.035 (static), which changes a look judged good.
- Maintainer to look at `psp-lcd`, `crt-aperture` and the other CRT presets in
  motion; the audit judged stills.

## Next Steps

- Apply the chosen `subpixel` default in `assets/shaders/screen/psp-lcd.wgsl`.
- If the ripple is wanted: in `psp-3000.wgsl` take the phase from
  `floor(row)` and re-run `screen::colour_tests`.

## Decided: no strength row for filters that transform the view

Maintainer, 2026-11-27: `crt-interlaced` and `crt-shadow-mask` move pixels (barrel), so a strength
mix below 100 shows the curved and the flat frame on top of each other. Those two run at a fixed 100
and SCREEN FILTER STRENGTH is not offered (hidden or greyed) while one is selected; every other filter
keeps the row. Prefer a property the filter file declares (ADR-0053: filters are loadable, so a mod's
barrel filter opts out the same way) over a list of built-in names. Check `crt-aperture` for a barrel
too and say which way it falls. Tests: the row disappears for the two, the pass ignores a stored
strength of 50 for them, and a stored 50 still applies to `psp-lcd`.
