# Magstrip leftovers, none load-bearing

The overlay question is closed ([track.md](../docs/formats/track.md)) and these three are what was left unexamined. (1) The magstrip's own animation, if any - `_magsurface3_1verb`'s `verb` suffix is unexplained, and material `+0x10`'s flag drives `FUN_0892733c` texture-matrix uploads in the batch walker `FUN_0890d0cc`. (2) Batch `+0x14` - a 4-byte file field between the batch's position scale (`+0x10`) and its s16 bounding box (`+0x18`), still unread. (3) Reproducing the corner tick-frames, only if anyone still cares - the magnification-artefact explanation stands.

**Pass bit `0x1000` is closed, 2026-08-25.** It is not a magstrip-specific
field: it is the general mesh-layer discriminator between draw keys
`0x45000000` and `0x4a000000`, decoded 2026-08-18 on
[draw-order.md](../docs/rendering/draw-order.md) and confirmed live here by
decompiling `Mesh_CompileBatchSet` (`0x0892f35c`) directly - see
[mesh-draw.md](../docs/ghidra/functions/psp-pulse-usa/mesh-draw.md#the-layer-derivation-and-what-it-is-worth).
The base strip (`pass_mask` `0x1021`/`0x1001`, bit set) draws on layer
`0x45000000`; the far-LOD overlay copy (`OVERLAY_PASS = 0x0021`, bit clear)
draws on the default `0x4a000000` - a real layer split, just not the one that
matters here: the original never has both visible at once (the overlay's PVS
section keeps it out of the racing view), so the fix that already landed -
authored-section placement, not depth or draw order - is what actually
resolves the z-fight. `mesh-draw.md`'s own "`& 0x1000` is undecoded here" note
(2026-08-09) was stale against its later self and is corrected in the same
change.

## Open

- Magstrip's own animation, if any - `_magsurface3_1verb`'s `verb` suffix is unexplained
- Batch `+0x14`'s meaning - a 4-byte gap in the file layout, unrelated to `pass_mask`
- Corner tick-frames have not been reproduced

## Next Steps

- Reproduce the corner tick-frames, if it still matters - the magnification-artefact explanation already stands as an explanation
- If anyone picks up batch `+0x14`: read it across a spread of batches (magstrip and otherwise) from `data/images/pulse-psp-usa.chd` via `oag_formats::vex::mesh_batches` first, to see whether it is ever non-zero before reaching for Ghidra
