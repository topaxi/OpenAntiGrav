---
status: blocked-maintainer
titles: [all]
updated: 2026-11-27
summary: "Screen filters keep colour now; psp-lcd stripe strength and psp-3000's inert ripple await the maintainer's eye."
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
