# Magstrip leftovers, none load-bearing

The overlay question is closed ([track.md](../docs/formats/track.md)) and these four are what was left unexamined. (1) The magstrip's own animation, if any - `_magsurface3_1verb`'s `verb` suffix is unexplained, and material `+0x10`'s flag drives `FUN_0892733c` texture-matrix uploads in the batch walker `FUN_0890d0cc`. (2) Batch `+0x14`, and pass bit `0x1000` - present on every magstrip batch except the far-LOD copy's, meaning still open ([mesh-draw.md](../docs/ghidra/functions/psp-pulse-usa/mesh-draw.md) also has `& 0x1000` undecoded). (3) Reproducing the corner tick-frames, only if anyone still cares - the magnification-artefact explanation stands.

## Open

- Magstrip's own animation, if any - `_magsurface3_1verb`'s `verb` suffix is unexplained
- Batch `+0x14` and pass bit `0x1000` meaning is undecoded (also undecoded in `mesh-draw.md`)
- Corner tick-frames have not been reproduced

## Next Steps

- Reproduce the corner tick-frames, if it still matters - the magnification-artefact explanation already stands as an explanation
