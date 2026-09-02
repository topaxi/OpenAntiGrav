# `Anim Transform` `0x3c0` is read and played, and closing it fixed a placement defect

2026-08-18. Measured over all twelve Pulse circuits, then decoded in Ghidra and ported (`crates/render/tests/scenery_animation_ground_truth.rs`, [scenery-animation.md](../docs/rendering/scenery-animation.md), [anim-transform.md](../docs/ghidra/functions/psp-pulse-usa/anim-transform.md)). **The texture-transform path was already complete** - 922 of 922 gated materials parse, reach a drawn batch and index the shader's table, worst circuit 18 distinct tracks against a ceiling of 63 - so the arrows, banners and holograms were never the gap. **`Anim Transform` was**: 393 nodes, 474 meshes below them, and `vex::world_transforms` gave the class the identity, which dropped its *placement* along with its animation and left **245 of those meshes at the world origin** (38 of them still scrolling their texture correctly - the clearest symptom, 34 on `07_Track`). Now 8, each put there by its own keys. **Three things a later reader should not re-derive.** (1) **Every `jal` target and `lui`/`addiu` immediate in this program is link-time, not runtime** - add `0x08804000`, the `.text` base; the decompiler's `func_0x...` addresses do not exist. (2) The payload field map is pinned by a **tiling** check, not by plausibility: the six key arrays fill `[0x50, len)` exactly on all 393 nodes, so one wrong offset breaks the first file. (3) `texture-animation.md` named `0x08a6bb84` as Mesh's class tag; it is **`Anim Transform`'s**, and Mesh's is `0x08a6bd48` - both pinned by their own registration functions, and corrected on that page. Twelve names in `names.tsv`. **The clock question is closed too**: both mechanisms read `+0x40` off one object, `g_ingame` (`0x08ab0818`), whose only writers are `InGame_Construct` and `InGame_Destruct` - so there is no per-model clock rule to find, and the race clock here is a reproduction. **Residual**: what advances `g_ingame->0x40`; the **second relocation base** (data references do *not* all use `.text`'s `+0x08804000` - the fallback clock global and the class descriptors compute into `.text` under it, so they need a base nobody has established); `AnimEnd` is read by the binder and by nothing else; the rate multiplier `AnimTransform_Update` can apply is inert because nothing writes its `+0x5c` numerator. `mesh.wgsl` skips the inverse transpose for normals under a non-uniform node scale, which is safe **because measured**: all 37 meshes under one are prelit, asserted so a title that lights one fails first.

**2026-09-02: the second relocation base is found.** It is per-segment, not a
single second base: this ELF's two `PT_LOAD` program headers each get their
own load-time base, `+0x08804000` for segment 0 (`.text` through `.data`) and
`+0x08ad9798` for segment 1 (`.cplinit`/`.linkonce.d`/`.ctors`/`.bss`, where
the fallback clock global lives). Confirmed by exact arithmetic reproducing
`texture-animation.md`'s independently-recorded `DAT_08b317b0`, and by the
`.rel.text` relocation record itself (bits 16-23 of `r_info` are the segment
index). HANDOVER.md's relocation trap section is corrected to match. Full
account: [anim-transform.md](../docs/ghidra/functions/psp-pulse-usa/anim-transform.md#the-second-relocation-base-is-found-it-is-per-segment-not-per-image).
**The class descriptor is checked too, same session**: `AnimTransform_Register`'s
own `desc` local resolves to `0x08b620f8` under the segment-1 base (not
`0x0888c960`, which lands inside an unrelated function under the single-base
reading) - named `g_anim_transform_desc`, confidence 85, in `names.tsv`.

## Open

- What advances `g_ingame->0x40` is not established.
- `AnimEnd` is read by the binder and by nothing else.
- The rate multiplier `AnimTransform_Update` can apply is inert because nothing writes its `+0x5c` numerator.

## Next Steps

- `g_ingame->0x40`'s advancer and the `+0x5c` numerator are each a fresh
  Ghidra trace, not blocked on anything above - `g_ingame`'s address is
  established, so a search for what writes its `+0x40` (or a live watchpoint)
  is the direct route; `+0x5c` needs finding what, if anything, ever writes a
  node's own `+0x5c` rather than the shared clock.
