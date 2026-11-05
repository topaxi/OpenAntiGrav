---
status: measure
titles: [pure]
updated: 2026-11-05
summary: "Pure's HUD outline and BorderColor reading are fixed (pure-status.md); open: the original's time digits run about 3 px wider over six glyphs"
---

# Pure's HUD digits run about 3 px wider in the original

2026-11-05, `pure-hud-glow`. The outline law landed (`docs/formats/pure-status.md`, "Pure's HUD text outline"). What
is left in the top-left digit box (Vineta K 18.1 mean grey, Chenghou 20.0) is placement: the original's `0.16.6`
ends at x 93, ours at 90.

## Open

- The recorded GE list gives each glyph quad's x and u; compare the six `CurrentTime` quads' advances with
  `oag_ui::font`'s advance for `HUDFont.fnt` at scale 0.9 (`ink_aligned_text` and `raster` are off on Pure, unmeasured).
- Pure's `Hud` layout writes `scale= "0.8"` on one front-end text widget (`Data.wad` entries 426 and 443); it reads
  now, so check that screen once.
- `data/reference/compare-page/pure-psp/*/pair.json` still says "ours prints TOTAL and has no speed gauge": stale, the
  side shot shows BRONZE TIME and a gauge.

## Next Steps

1. `scripts/psp-ge-dump.py dump` on the Vineta K state, read the `0x19f` prims at y 1.5 and 35.0 (x of each glyph).
2. Set `HudArt::raster`/`ink_aligned_text` or the advance rule from what the quads show, per-title data with a source.
