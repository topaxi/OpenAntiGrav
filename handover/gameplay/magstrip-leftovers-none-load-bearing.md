# Magstrip leftovers, none load-bearing

The overlay question is closed ([track.md](../../docs/formats/track.md)) and one item remains: reproducing the corner tick-frames, only if anyone still cares - the magnification-artefact explanation stands. Pass bit `0x1000`, the magstrip's own animation, its `_magsurface3_1verb` name, and batch `+0x14` are all closed or narrowed below.

**Pass bit `0x1000` is closed, 2026-08-25.** It is not a magstrip-specific
field: it is the general mesh-layer discriminator between draw keys
`0x45000000` and `0x4a000000`, decoded 2026-08-18 on
[draw-order.md](../../docs/rendering/draw-order.md) and confirmed live here by
decompiling `Mesh_CompileBatchSet` (`0x0892f35c`) directly - see
[mesh-draw.md](../../docs/ghidra/functions/psp-pulse-usa/mesh-draw.md#the-layer-derivation-and-what-it-is-worth).
The base strip (`pass_mask` `0x1021`/`0x1001`, bit set) draws on layer
`0x45000000`; the far-LOD overlay copy (`OVERLAY_PASS = 0x0021`, bit clear)
draws on the default `0x4a000000` - a real layer split, just not the one that
matters here: the original never has both visible at once (the overlay's PVS
section keeps it out of the racing view), so the fix that already landed -
authored-section placement, not depth or draw order - is what actually
resolves the z-fight. `mesh-draw.md`'s own "`& 0x1000` is undecoded here" note
(2026-08-09) was stale against its later self and is corrected in the same
change.

**The magstrip's own animation is closed, 2026-08-25: there is none.**
Neither magstrip material carries the texture-transform gate
(`flags & 0x10`), and both come back with no keyframe block at all from
`oag_formats::vex::mesh_tex_transforms` -
`crates/render/tests/magstrip_ground_truth.rs`'s
`the_magstrip_material_carries_no_texture_transform`. The mechanism this was
asking about (material `+0x10`'s flag driving `FUN_0892733c`'s texture-matrix
uploads) is real and already implemented generically
(`oag_render::mesh_render::TexAnims`); the magstrip's two materials just do
not use it. Separately, `_magsurface3_1verb.tga`'s `"verb"` suffix is the
**only** occurrence of that string across all 5,017 texture names on the
disc's 307 version-6 `.vex` files (a `Dmagsurface3_1verb.tga` "Dark" variant
is the other) - not a naming convention, so there is nothing further to
decode from it. Left as an unexplained authoring idiosyncrasy, not a lead.

**Batch `+0x14` is narrowed but still open, 2026-08-25.** It is not padding:
censused across all 65,279 non-VIF PSP batches on `Data.wad`
(`crates/formats/tests/batch_position_gap_ground_truth.rs`), read as an
`f32` it is nonzero, positive and finite on every one. It does not track the
batch's own `scale` at a fixed ratio, but sits in a loose band (median 1.46,
10th-90th percentile 1.10-2.65) against half the batch's own bounding-box
diagonal - consistent with, not proof of, some kind of per-batch bounding
radius. No consuming instruction has been found for it in
`psp-pulse-usa/BOOT.BIN`, and whether it is even one `f32` rather than two
packed sub-fields was not tested. See `docs/formats/vex.md`'s "Not
determined" list for the same writeup.

## Open

- Batch `+0x14`'s meaning - narrowed (see above), not decoded; no consuming instruction found yet
- Corner tick-frames have not been reproduced

## Next Steps

- Reproduce the corner tick-frames, if it still matters - the magnification-artefact explanation already stands as an explanation
- If anyone picks up batch `+0x14` from Ghidra: the position-scale field it sits next to (batch `+0x10`) has no known consuming instruction either, so finding what reads *that* first may be the faster route in - whatever reads `+0x10` is likely to touch `+0x14` in the same few instructions
