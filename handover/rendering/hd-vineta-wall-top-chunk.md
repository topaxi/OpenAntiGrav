---
status: measure
titles: [hd]
updated: 2026-10-27
summary: "Vineta K wall-top pool: unreferenced chunk 1556 (slot 633) covers slot 392; the original draws no such chunk, rule unknown"
difficulty: medium (40)
---

# Vineta K's wall-top warm pool: slot 633 covers it

Evidence and numbers: `docs/rendering/hd-track-tone.md`, "Vineta K's warm light pool on the wall top".

## Open

- Why the original does not draw `.rcsmodel` chunk 1556 (slot 633, `diffuse_normal_specular_emmissive`, `lit_default`, flags 0,
  hash matching no `.vex` node). Lead: ours draws every unreferenced chunk, the original drew 30 of 2,081 in one frame (glow,
  billboard, glass, sea). Candidates: PVS cell, mode-gated bit (`+0x06` bits 9 and 10), engine draw filter on unreferenced
  chunks, depth order. Read `Pvs_*` and the per-chunk record consumers in Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`).
- Chunks 1554 to 1557 (slots 779, 633, 634, 481) are the same cluster; check them in the same pass.
- Second boot, and a mid-lap pose, for the draw-list absence (one boot so far).

## Next Steps

1. Census every circuit: unreferenced, flags-0 chunks on non-pad, non-glow materials, and whether the original's draw list
   (`hd-draw-dump.py`, bias key on `c[467]`) ever contains one.
2. Once the rule is named, wire it in the scene's unreferenced pass (`crates/mesh/src/mesh/rcs/pads.rs` area), not a slot skip.
   Size: about 0.23 M area x error on this pair; wiring is small once the rule is known (confidence 60).
