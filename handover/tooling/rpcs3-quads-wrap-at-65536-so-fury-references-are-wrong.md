---
status: blocked-maintainer
titles: [hd]
updated: 2026-11-05
summary: "Wide-index RPCS3 build works, one boot: the re-taken Fury menu pairs are held unadopted; a second boot, tint ratios and footprint are unread"
categories: [tooling, rendering]
---

# Fury references without RPCS3's index wrap exist, held and seen on one boot

Census, patch description, method and numbers: `docs/reverse-engineering/rpcs3-index-wrap.md`. Only Fury menu pairs carry the wrap; race pairs do not. On Linux Vulkan
only `QUADS`, `POLYGON` and `LINE_LOOP` are emulated; `TRIANGLE_FAN` and `QUAD_STRIP` are native. The patch is local and not for upstream:
`data/tools/rpcs3-wide-indices/` holds the diffs, `build.sh` and the binary.

## Open

1. **Held by the maintainer:** adopting re-taken pairs for `menu-campaign-select`, `menu-cell-select`, `menu-grid-select`, `menu-ship-select`, `menu-track-select`, `gap-main`,
   `gap-options`, `gap-racebox`. Whole-frame pair RMSE barely moves, so adoption buys little alone. Size XS (80).
2. **A second patched boot of the pinned state.** Four cycles missed `mirage_c1`; the level-gap claim stays at 78 until one lands. Any cloud repeated on both builds would do. Size S (50).
3. **What was measured through the defect** and needs re-reading: the tint ratios and the footprint on `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`. Size S (50).

## Next Steps

1. Wait for the maintainer's word on Open 1. To re-take: `OAG_RPCS3_BIN` set, `scripts/rpcs3-hd-menu-pair.py`, into scratch.
2. Take Open 2 and Open 3 in one session on the wide build, with the rules in the doc's "Taking a reference with it".
