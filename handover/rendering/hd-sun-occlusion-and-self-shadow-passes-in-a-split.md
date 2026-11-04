---
status: measure
titles: [hd]
updated: 2026-11-04
summary: "HD opens 16 small passes (8 sun-occlusion, 8 self-shadow) a frame; culled-hull skipping is single-view only; ceiling and a split-aware cull first"
categories: [rendering]
size: M
difficulty: medium
---

# HD's per-craft sun-occlusion and self-shadow passes

2026-11-04, from the per-frame allocation scout and wgpu's guidance (few render passes). A frame
on HD opens about 54 render passes on Anulpha (`docs/rendering/frame-thread-cpu.md`, "Where
`submit` goes"), and per-pass setup and teardown was about 8% of the Deck's frame thread there:
wgpu-core sizes each pass's usage scope to the device's whole buffer and texture count, then
clears it. Eight of the passes are sun occlusion, one per craft, and eight are self-shadow, one
per craft (`crates/render/src/shadow/occlusion.rs`, `self_shadow.rs`).

## Open

- **A craft whose hull is culled gets no sun-occlusion or self-shadow pass, in single-view frames
  only** (`Scene::hull_seen`, `craft_hull::hull_in_view`; the 2026-10-27 change in the same doc
  section). A split frame draws every craft's two passes whatever either view sees. The test is the
  hull draw's own, so a skipped layer is one no pixel reads; a split needs "seen by any view".
- **Folding the passes** (one pass over an array of layers, or instancing the per-craft draw) is the
  larger change: days, a design question about how the layers are bound and read, and a risk of a
  negative result on a desktop part that is not tiled.
- The scout counted 344 allocations a frame in these passes on HD, about 120 removable by fewer
  writes per craft (alloc scout item 3): separate from the passes themselves.

## Next Steps

1. The ceiling: skip every sun-occlusion and self-shadow pass behind an env switch in a throwaway
   build (pictures wrong) and read the Deck's `frame-phases` total, HD Anulpha, one view and
   two-player split, 800p, 1080p, 4K. If it is under about 0.3 ms, close this with the number.
2. If the ceiling is real: extend `hull_seen` to a split (any view sees the hull), byte-identical
   pictures per view, then decide on folding.
