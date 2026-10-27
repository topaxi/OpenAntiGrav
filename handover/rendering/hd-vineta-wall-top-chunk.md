---
status: measure
titles: [hd]
updated: 2026-10-27
summary: "Vineta K chunk 1556 (slot 633) covers the wall-top pool; PVS, frustum, flags, material all refuted; read bucket B1/B4 consumers"
difficulty: medium (40)
---

# Vineta K's wall-top warm pool: slot 633 covers it

Evidence and numbers: `docs/rendering/hd-track-tone.md`, "Vineta K's warm light pool on the wall top".

## Open

- Why the original draws neither surface of chunk 1556 (slot 633) at the start grid. Done 2026-10-27 (`hd-unreferenced-chunks`,
  `docs/rendering/hd-track-tone.md`): the per-surface `c[467]` key, the original's one PVS cell (bit set), the frustum (centred),
  flags, material family and bounds entry all fail to separate it from the drawn chunks 1554, 1555, 1557 beside it; about 95 % of
  PVS+box-eligible orphans are drawn across seven lists, leaving ~30 unexplained exceptions with no common field.
- Chunks 1554 to 1557 are the same cluster; 1554, 1555 and 1557 are drawn, 1556 is not.
- A second boot and a mid-lap pose for the draw-list absence (one boot so far; not needed by the census).

## Next Steps

1. Read the **consumer of bucket B1** (`Scene_SortAndLightVisibleChunks` `0x003fdae8` routes flag bits 4 and 5 clear to B1; its
   consumer is unresolved in `zone-effectsettings-loader.md`) and the B4 consumer `FUN_00400a00`; look for a per-record test
   the submit loop (`0x003fab50`) does not make. Name only at 70 or more.
2. If a per-record test is found, express it over data (material record field, surface field) and census the ~30 exceptions
   against it; wire in the scene's unreferenced pass (`crates/mesh/src/mesh/rcs/pads.rs` area), not a slot skip.
3. Otherwise capture a second Vineta K frame at the pair's exact camera (`c[465]` holds the eye in the dump) and list again.
