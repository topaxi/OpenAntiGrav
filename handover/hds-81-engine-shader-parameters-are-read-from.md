# HD's 81 engine shader parameters are read from their own initialiser, and `time` is entry 0

2026-08-24. Table, slot order, cross-checks and the two recovered preimages (`engineTrail` = `0xbb48e390`, `globalAlphaScaler` = `0x4c13d3af`) are on [renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md). **Not on the page**: Ghidra's decompile of `0x003f1300` names sound banks and `.vex` paths - every string in it is wrong, because it sits above the `0x32d5e0` TOC break, and `ps3-toc.py` walking the function's own `lwz rX,disp(r2)` is the only way to read it. That is worth reaching for before any other high-address function is believed. The flame's `Speed * time` surface scroll is now unblocked; what it needs is the two-sample lookup, not more reverse engineering.

## Open

- Ghidra's decompile of `0x003f1300` names sound banks and `.vex` paths that are all wrong, because it sits above the `0x32d5e0` TOC break

## Next Steps

- Implement the two-sample lookup for the flame's `Speed * time` surface scroll (now unblocked, no more RE needed)
