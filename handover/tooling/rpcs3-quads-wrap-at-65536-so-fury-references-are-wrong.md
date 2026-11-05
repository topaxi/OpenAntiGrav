---
status: measure
titles: [hd]
updated: 2026-11-05
summary: "RPCS3 indexes emulated QUADS with u16: patch it to 32-bit, re-capture the Fury references, census other HD draws past 65,535 vertices"
categories: [tooling, rendering]
---

# RPCS3 wraps emulated primitives at 65,536 vertices, so the Fury references are wrong

Finding and evidence: `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "The Fury references carry an RPCS3 defect",
and `docs/reverse-engineering/rpcs3-capture.md`.

## Open

1. **No reference without the wrap exists.** Every Fury backdrop pair on the compare page (`menu-*`, Main Menu, Ship Select,
   Campaign Select) holds the wrapped picture. Size M (50): the patch is two sites, the build is not incremental here
   (`data/tools/rpcs3-watchpoints/build` holds 282 objects and no binary).
2. **Census.** Any other HD draw of `QUADS`, `POLYGON`, `TRIANGLE_FAN` or `LINE_LOOP` past `0xffff` vertices carries the same
   defect in race references. `scripts/rsx-pass-events.py` prints `cnt`; none other is known. Size S (50).
3. **What was measured through the defect** and needs re-reading: the tint ratios and the footprint on `menu-backdrop.md`.

## Next Steps

1. In a private copy of the tree: in `rpcs3/Emu/RSX/Common/BufferUtils.cpp`
   `write_index_array_for_non_indexed_non_native_primitive_to_buffer`, `iota16` and the callers in `RSXOffload.cpp`, write `u32`;
   in `VK/VKVertexBuffers.cpp` `generate_emulating_index_buffer` use `sizeof(u32)` and `VK_INDEX_TYPE_UINT32`. Add it to
   `scripts/patches/` beside the watchpoint patches, and build with `scripts/build-rpcs3-watchpoints.sh`'s recipe.
2. Check it: on Main Menu with the patched build, the near hull appears and `cnt` past `0xffff` no longer shows the cut at
   record 16,384. Re-capture frame 484's state on path 6 (`mirage_c1`, paused, bands at zero).
3. Score ours against that, then re-take the pairs. Upstreaming the fix is the maintainer's call.
