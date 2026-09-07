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
radius. See `docs/formats/vex.md`'s "Not determined" list for the same
writeup.

**The `+0x10`-first route in was taken, 2026-09-07, and it worked - for
`+0x10`.** `Mesh_InitFromPayload` (`0x0890e998`) walks both batch lists only
to reach the **last** batch, reads that one's `+0x10` (`lwc1 f12, 0x10(s2)`,
`0x0890ef5c`) into the runtime mesh at `+0x54`, and divides the mesh header's
own `f32` bounding box through it; `Vex_UpdateNodeWorldMatrix`
(`0x08944544`, named this pass at 85) applies it with `vmscl.q` at
`0x0894464c`. Full chain on
[vex.md](../../docs/formats/vex.md#the-chain-that-consumes-it-walked-instruction-by-instruction),
and the file-side invariant it depends on is now a test: 21,055 meshes, zero
whose batches disagree about `+0x10`.

**The hoped-for spillover did not happen: `+0x14` is read by nothing.**
Confidence 80. Every batch-list walker was enumerated by the `payload_size`
load each one needs (`lhu ..., 0xc(...)`, 55 sites program-wide) and checked
for any `+0x14` access - all stack slots. Independently every float load at
`+0x14` in the whole executable was enumerated across `s`/`a`/`v`/`t` bases
plus `lv.s`, and the only one in mesh code is the *mesh header's* bounding
box. An integer read, or a read through a base already offset by `0x10`,
would evade both sweeps - that is the gap in the negative. Two dead
hypotheses recorded so nobody re-runs them: `+0x14` is **not** the alternate
vertex block's own scale (no batch on `Data.wad` has an alternate block at
all - 0 of 65,279), and it is **not** two packed `u16`/`f16` sub-fields
(20,979 distinct low halves, 4 zeros, under a genuinely spread exponent -
mantissa noise). It is also not mesh-wide: uniform within 11,167 of 21,055
meshes, where `+0x10` is uniform within all of them.

## Open

- Batch `+0x14`'s meaning - narrowed hard (see above), not decoded. It is a per-batch quantity `psp-pulse-usa` authors and never reads, so the answer is not in that executable
- Corner tick-frames have not been reproduced

## Next Steps

- Look for a `+0x14` consumer in a **different** executable - `ps2-pulse-eu/SCES_547.48` first, then `psp-pure-usa/BOOT.BIN`. Both are already imported. Note PS2 batches are shaped differently (`vex.md`: `f32` bounding box at `+0x20`/`+0x30`, `scale` always exactly 1.0), so a PS2 hit would be a different field at the same offset, not the same one - which is itself worth knowing
- Do **not** fit more geometry against the `s16` box. Six candidate ratios have been run against it and every band is looser than a factor of two; the remaining information is in the vertices or in another executable
- Reproduce the corner tick-frames, if it still matters - the magnification-artefact explanation already stands as an explanation
