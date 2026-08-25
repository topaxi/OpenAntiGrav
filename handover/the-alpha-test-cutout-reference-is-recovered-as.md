# The alpha-test cutout reference is recovered as three values, not one, and the per-batch selector is not

`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md` reads `is_alpha_tested()`'s branch of `Gfx_BuildBatchStateList` as `Gu_AlphaFunc(GU_GREATER, ref, 0xff)` with `ref` one of `0x7f`, `0` or `0x10` depending on the batch - three real call sites already in that decompiled function, not a guess. `mesh.wgsl`'s `ALPHA_TEST_THRESHOLD` (2026-08-17) uses `0`, the most permissive, chosen because it is provably safe (a batch authored for a stricter reference only shows a few extra near-zero-alpha texels, never hides real geometry) rather than because it is *right*. **Next step**: read what selects between the three in `Gfx_BuildBatchStateList` itself - the function is already open from this pass, and `mesh-draw.md`'s own recommendation 5 names this as the place a correct per-batch value comes from.

## Open

- The per-batch selector between the three alpha-test reference values (`0x7f`, `0`, `0x10`) is not recovered
- `mesh.wgsl`'s `ALPHA_TEST_THRESHOLD` uses `0` because it is provably safe, not because it is confirmed correct

## Next Steps

- Read what selects between the three reference values in `Gfx_BuildBatchStateList` (already open from this pass; `mesh-draw.md` recommendation 5 names this as the place it comes from)
