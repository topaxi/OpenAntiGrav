---
status: blocked-maintainer
titles: [hd]
updated: 2026-11-05
summary: "A wide-index RPCS3 build and re-taken Fury menu pairs exist: adopt them into data/reference, then re-read the tint ratios and footprint"
categories: [tooling, rendering]
---

# Fury references without RPCS3's index wrap exist and wait to be adopted

Census, patch, method and numbers: `docs/reverse-engineering/rpcs3-index-wrap.md`. Only Fury menu pairs carry the wrap; race pairs do not.
Binary: `data/tools/rpcs3-wide-indices/src/build/bin/rpcs3`, rebuilt by `scripts/build-rpcs3-wide-indices.sh`.

## Open

1. **The compare page still holds the wrapped pairs.** `menu-campaign-select`, `menu-cell-select`, `menu-grid-select`, `menu-ship-select`, `menu-track-select`,
   `gap-main`, `gap-options`, `gap-racebox`. Agents do not write `data/reference`. Size XS (80).
2. **What was measured through the defect** and needs re-reading: the tint ratios and the footprint on `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`. Size S (50).
3. Why two stock boots read R mean `19` and `44` at one pinned state. Size S (40).
4. Upstreaming the fix is the maintainer's call.

## Next Steps

1. Re-take the pairs with `OAG_RPCS3_BIN` set and `scripts/rpcs3-hd-menu-pair.py`, then copy them over the eight directories.
2. Repeat the tint reads on the wide build, with the rules in the doc's "Taking a reference with it".
