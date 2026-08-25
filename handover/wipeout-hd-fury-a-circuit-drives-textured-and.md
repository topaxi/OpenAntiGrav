# Wipeout HD / Fury: a circuit drives, textured and blended, off the PS3 disc

2026-08-18. `cargo run -p oag-game --bin oag-game -- --race --hold cross --ticks 600 data/images/hdfury-ps3-eu-dec.iso` drives Talon's Junction on HD's own collision, PSARC archives, `.rcsmodel` geometry and `.gtf` textures - all four landed with disc-backed ground-truth tests covering all 643 `.rcsmodel` files and 15,762 materials. Full history, the chunk-layout discriminators and every trap (byte `+0x06` selects the layout, `+6` packs a unit normal, PVS is one 64-bit mask rather than a lo/hi pair, `+0x05` submesh descriptors versus `0x01`'s header-only layout) are on [rcsmodel.md](../docs/formats/rcsmodel.md), [hd-status.md](../docs/formats/hd-status.md) and [psarc.md](../docs/formats/psarc.md) - **do not requote from this file**. Still open: the second texture slot at a material's `+0x78` plays six roles - normal map / emissive map / coverage mask unwired still, plus lightmap and two more now read off the compiled shader itself ([rcsmaterial.md](../docs/formats/rcsmaterial.md)). 53 of 7,333 `.gtf` (Morton-swizzled layouts, cubemaps) refuse to decode and draw white; the `.pvs` per-section mapping that would let a renderer draw one segment instead of all 904 chunks is unread; and of 56 cross-circuit shared-geometry nodes, 38 have no donor model identified. **Reframed 2026-08-20**: the 18 that do are now confirmed a second way (9 of 13 index-byte-identical, the other 4 in two vertex-count tiers of the same asset) and the phenomenon is disc-wide, not Talon's-Junction-specific - two circuits resolve every node they carry. That settles *identity* as a shared per-title asset key, independently cooked per circuit, not a runtime cross-file lookup; whether the shipped executable ever holds a second circuit's `.rcsmodel` open while a level is resident is the one falsifiable question left, unasked, and the RPCS3 GDB harness below is the route that would answer it. See [rcsmodel.md](../docs/formats/rcsmodel.md) open item 6 - **do not requote from this file**.

## Open

- The second texture slot at a material's `+0x78` still has three unwired roles: normal map, emissive map, coverage mask.
- 53 of 7,333 `.gtf` files (Morton-swizzled layouts, cubemaps) refuse to decode and draw white.
- The `.pvs` per-section mapping, which would let a renderer draw one segment instead of all 904 chunks, is unread.
- 38 of 56 cross-circuit shared-geometry nodes have no donor model identified.

## Next Steps

- Use the RPCS3 GDB harness to check whether the shipped executable ever holds a second circuit's `.rcsmodel` open while a level is resident - see [rcsmodel.md](../docs/formats/rcsmodel.md) open item 6.
