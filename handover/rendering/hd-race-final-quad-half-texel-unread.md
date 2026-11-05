---
status: ready-to-wire
titles: [hd]
updated: 2026-11-05
summary: "A race's final composite reads half a source texel off, measured: oag_post::hd_bloom's fs_encode reads with no offset, wire it as a title law"
---

# A race's final composite reads half a source texel off, and ours does not

Measured: `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "A race's final composite is the same quad". The offset is `0.5 / width`, `0.5 / height` of the
scaled source, so it follows the dynamic width on a race.

`oag_post::hd_bloom`'s `fs_encode` reads the scene with no offset, so each race pixel here is one scene texel where the original averages four in linear light: half a
texel of softness on every race pair.

## Open

1. The tap in `fs_encode`, with ours' own source width. Size S (55).

## Next Steps

1. Add the tap as a title law (`Title` data, ADR-0058), with the backdrop's own present pass moved
   onto it (`handover/frontend/fury-backdrop-pass-graph-is-measured-and-ours-differs.md`, Open 4). Re-render the HD pairs and quote
   the sharpness change.
