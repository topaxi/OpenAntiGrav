# HD's see-through surfaces draw with the file's own blend equation; the texture-coordinate residue is diagnosed but not fixed

2026-08-18. **(a) Landed**: `Material::blend` now carries both blend factors instead of keying on destination alone - 367 of 2,574 see-through materials were drawn with the wrong equation (144 unweighted additive, 142 straight-alpha instead of premultiplied, 81 previously drawn opaque and unaffected). Pulse authors no such pair and is untouched by construction. [rcsmodel.md](../docs/formats/rcsmodel.md#the-factor-values-and-which-are-mapped). **(b) Not fixed**: 290 of 1,112 draw calls on a circuit span over 100 texture tiles. Diagnosed as two coordinate *types* sharing the last four bytes of a vertex - big-endian halves on most stride-18 submeshes, big-endian `Unorm16` on others (RSX's `CELL_GCM_VERTEX_U16N`) - distinguished by texel-density, since no field of the file (stride, `83 XX`, material, any header byte) separates the groups; end to end this took implausible coordinates from 10.37% to 2.38%. `Mesh::texcoord_format` reads the type from content and nothing is implemented off it. **(c) Still open**: stride-22 carries a *second* texture coordinate set at `+0x0e`, unused because which of the second texture slot's four uses (normal/emissive/mask/lightmap) applies per material is in the `.rcsmaterial`'s compiled shader (Cg microcode) - a 2026-08-18 sweep of the `SHO` parameter table ruled out a discriminator there and found instead that parameters key on `Crc32_HashString`'s complement (confirmed: `~crc32("viewProj") == 0x2e7d5f33`), so naming one is now a wordlist problem, not a cryptographic one. Also open, deliberately: transparent draws are not depth-sorted anywhere (list order only, affects Pulse too, no Pulse capture re-measured); `Transparency::Mode2` is not routed to the cutout pass (`material.rs` already refutes alpha-test for it); and whether ADR-0020's `Rgba8Unorm` decision holds for RSX rather than the PSP's GE is unanswered.

## Open

- Which of the second texture slot's four uses (normal/emissive/mask/lightmap) applies per material is unresolved
- `Mesh::texcoord_format` detects the coordinate type but nothing is implemented off it (290 of 1,112 draw calls still affected)
- Transparent draws are not depth-sorted anywhere (also affects Pulse, unmeasured there)
- Whether ADR-0020's `Rgba8Unorm` decision holds for RSX rather than the PSP's GE is unanswered

## Next Steps

- Wordlist-search the `Crc32_HashString`-complement hashes against the `.rcsmaterial` Cg parameter names to identify which second-texture use applies per material
- Wire `Mesh::texcoord_format`'s detected type into the draw path (currently detected but unused)
