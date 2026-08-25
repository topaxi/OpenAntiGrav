# The ghost-ship renderer is read and written down nowhere else

Ghidra by-catch from the magstrip detour, kept here because it exists in no docs page and nothing is renamed (confidence ~55). Class `0x3d4` `MeshNode_Ghost` registers a vtable at `0x08ad171c` from `FUN_08911480` (the generic `Mesh` class `0x125` registers `0x08ad1694` in `FUN_089100a0`; vtable entries at base `+0xc`, 8-byte stride). It overrides submit - `FUN_08910320`, which enqueues the same mesh up to three times with sort keys `0x4d000000 \| 0..2` - and draw, `FUN_08910fe0`, a three-phase state machine: phase 0 untextured depth-lay, phase 1 per-material texture with `Gu_DepthFunc(EQUAL)` and a fixed-colour blend whose brightness ramps with distance to the player ship over 5..25 units, phase 2 `Data\Tex\staticglow.mip` projected through a **texture matrix** (`Gu_SetMatrix(3, world x view x scale/offset)`, `TexMapMode(matrix)`, scale globals `4.0`/`1.8` at `0x08abf4c4`/`c8`, offsets re-randomised per frame at `0x08abf4e4`/`e8` by `FUN_0891055c`), with stencil and alpha-test `GEQUAL 0x80`. Reads as the **time-trial ghost renderer** - proximity fade plus sparkle. [roadmap M6](../docs/overview/roadmap.md) has the checkbox; it needs replay data, so it lands late.

## Open

- Nothing about the ghost-ship renderer is written up in any docs page, and nothing is renamed (confidence ~55)
- It needs replay data before it can land (blocked on M6)

## Next Steps

- Get replay data in place, then land the ghost-ship renderer under M6
