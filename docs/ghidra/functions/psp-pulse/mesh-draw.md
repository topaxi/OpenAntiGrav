# Mesh batch draw state: the blend the ordinary draw path actually uses

What `Mesh`-class batches (ship hull, track, and the boost plume alike) set
for blend, depth and cull state before drawing, and which of two
`Gu_BlendFunc` branches a given batch takes - not a flare- or ribbon-specific
function, both of which have their own pages ([`exhaust.md`](exhaust.md)).

| | |
| --- | --- |
| **Address** | `Mesh_SetBatchDrawState` `0x0890d994` |
| **Confidence** | 85 |
| **Callers** | `FUN_0890db54` (`0x0890db90`), `FUN_089307b4` (`0x08930ddc`) |

The `85` above covers the decompile itself - the function's behaviour and the
blend-equation encoding, both direct Ghidra reads. Claims about which
*real batches* take which branch (the boost plume, `TRANSPARENT_BLEND`'s
project-wide applicability) carry their own, lower figures inline below;
where this page and other pages give a number next to a specific claim, that
number governs, not this header.

## What it does

```c
void Mesh_SetBatchDrawState(int mesh, ushort *batch)
{
    if ((*batch & 0x8000) == 0)
        Gu_CallList(*(int *)(mesh + 0x70) + 0x48);
    else
        Gu_CallList(*(int *)(mesh + 0x70) + 0x70);

    if ((batch[flags_byte] & 0x10) == 0) {
        // depth state branches on *batch & 2, both sides read the same
        // depth-range pair from mesh+0xcc/+0xd0 and an alpha-ref byte at
        // batch+0x29, fed to FUN_0891e988
    } else {
        Gu_DepthMask(0); Gu_DepthFunc(6);
    }
    FUN_0891e988(g_display, depth_range_lo, depth_range_hi, alpha_ref);

    if ((*batch & 0x20) == 0) Gu_Enable(5); else Gu_Disable(5);   // GU_CULL_FACE

    if ((batch[flags_byte] & 0x10) == 0)
        Gu_BlendFunc(GU_ADD, GU_FIX, GU_FIX, 0xffffff, 0xffffff); // dst + src
    else
        Gu_BlendFunc(GU_ADD, GU_FIX, GU_FIX, 0xffffff, 0);        // src, dst discarded

    if ((*batch & 0x40) == 0) { if (DAT_08b32c60 != 0) FUN_08907828(DAT_08b32c60, 0); }
    else                        FUN_08811898(0);
}
```

(Renamed from the literal decompile for readability; `batch[flags_byte]` is
`*(byte *)((int)batch + 3)`, `GU_CULL_FACE` = state index `5` per the PSP SDK's
standard `sceGuEnable` constants, already load-bearing for
[`Batch::is_culled`](../../../../crates/formats/src/vex.rs) matching `*batch &
0x20` exactly.)

Two `Gu_BlendFunc` calls happen on *every* invocation, not a blend-enable
toggle - the only variable is which of two fixed-factor equations it
programs. Decoded against the already-confirmed encoding
(`ExhaustFlare_BuildDisplayList`'s own `Gu_BlendFunc(0,2,10,0,0xffffff)` decodes
as `GU_ADD, GU_SRC_ALPHA, GU_FIX` per [`exhaust.md`](exhaust.md), pinning
`0`=`GU_ADD`, `2`=`GU_SRC_ALPHA`, `10`=`GU_FIX`):

- **`header byte+3 & 0x10` clear**: `Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX
  0xffffff)` = `dst + src`, pure additive. **Bit-for-bit identical** to the
  `Trail` ribbon's own independently-confirmed, live-verified-enabled blend
  (`exhaust.md`: `sceGuBlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)`).
- **`header byte+3 & 0x10` set**: `Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX
  0)` = `src` alone, `dst` factor zero - visually a replace, and the visual
  result is the same whether or not `GU_BLEND` happens to be enabled for this
  branch (a zero destination factor and a disabled blend stage both just leave
  `src`).

`oag_formats::vex::Batch::is_additive_blend()` decodes this bit; see that
function's own doc comment for the exact bit and confidence.

## `param_2` is the on-disk batch header, not a `Material`

Both callers pass a pointer that is walked exactly like
`oag_formats::vex::mesh_batches`' own loop over the on-disk batch list:

- `FUN_0890db54` iterates `param_3`, advancing it by `param_3[6] + 0x40` or
  `+0x80` (`+0x40` byte offset in the alternate-header case) depending on
  `*(byte *)(param_3+3) & 0x40`. That is `u16_at(header, +0x0c) + header_size`
  - **byte-identical stride arithmetic** to `oag_formats::vex::mesh_batches`'
  own `step = header_size + payload_size` (`payload_size` is the same
  `u16_at(payload, at + 0x0c)`). The runtime walks the on-disk batch array *in
  place*, with the same stride formula reading the same offsets - the
  strongest evidence here that `param_3` is the header as authored, not a
  repacked runtime copy. `*param_3 & param_4` as the pass-mask/terminator test
  (matching `mesh_batches`' `pass_mask & terminator == 0` break condition) is
  a second, independent match on the same reading.
- `FUN_0890d994(param_1, param_3)` (now `Mesh_SetBatchDrawState`) is called
  with that same pointer.
- `FUN_089307b4`'s call site (`Mesh_SetBatchDrawState(iVar17, iVar16)`) passes
  `iVar16 = piVar12[2]`, a per-draw-item field walked with the identical
  `*(byte *)((int)puVar13 + 3) & 0x40` test a few lines above and below in the
  same function - the same header, reached through a different iteration.

The `& 0x40` bit-test match (used for `mesh.wgsl`'s alternate-header
selection, not the blend branch) is corroborating but weaker on its own -
`0x40` never actually appears set in any batch this page measured, so by
itself it is code-shape agreement rather than data-confirmed agreement. The
stride-arithmetic match above is what actually pins byte identity, since it
holds regardless of which bits any particular batch sets.

**Not established: whether the loader touches this byte after load.**
Materials *are* repacked at load time (runtime stride `0x14` per material,
versus the packed on-disk layout `oag_formats::vex` parses), so the loader
demonstrably restructures some fields between disc and runtime. Nothing here
rules out the loader poking batch-header `+3` in place; no write to that
offset has been searched for.

`FUN_089307b4` is the mesh draw path's pass-group dispatcher: it walks up to
four separately-gated batch groups (`param_3 & 1/2/4/8`), and
`Mesh_SetBatchDrawState` is called only inside *that function's* `& 2`
group's loop - the transparent pass. The `& 1` (opaque) group uses
`FUN_0890d3ac` instead and never touches blend state at all; `& 4`
(alpha-tested) sets depth directly off `*batch & 2` without a blend call; `& 8`
likewise. `FUN_0890db54` (the other caller) gates its own call on a different
bit, `*param_3 & 0x2000` - outside both `is_transparent()`'s `0x0700` and
`is_alpha_tested()`'s `0x0800` - which is also the bit `Mesh_CompileDisplayLists`
uses to pick between its two compile-side helpers (`FUN_0890cf84`/
`FUN_0890d508`), not a pass-mask category this project has named. So
`is_additive_blend()` is confirmed operationally meaningful for a batch
`FUN_089307b4` classifies transparent; whether/how it matters for whatever
`FUN_0890db54`'s `0x2000`-gated path draws is not established.

## Measured: every real transparent batch sampled reads as additive

Read directly off `pulse-psp-usa.chd` through `oag_formats::vex::mesh_batches`
(no live capture needed, given the byte-identity evidence above): every
team's `shipboost.vex` (3 teams sampled, 4 transparent batches each), a
`Ship.vex` per team (1 transparent batch each), and both `01_Track` (142
transparent batches) and `16_Track` (254 transparent batches) - **zero**
batches read the `0x10`-set "replace" branch. `header_flags` itself is not
stuck at zero (`0x00`, `0x01`, `0x02` and `0x20` all appear across the
sample, so the byte is genuinely varying and being read correctly), the
`0x10` bit specifically just never comes up set in any batch checked. Pinned
as a permanent regression check:
`crates/game/tests/boost_plume_ground_truth.rs`'s
`every_psp_teams_boost_plume_batches_are_additive_blend` (all 8 PSP teams).

**A caveat this measurement cannot resolve on its own**: a bit that is
constant across roughly 4,100 sampled batches is consistent with two readings
that this pass does not distinguish between - "no authored PSP content uses
the replace branch" and "the `0x10` bit does not mean what this page claims,
or gets rewritten before the draw path reads it." The stride-arithmetic
match above (not this histogram) is what supports byte identity; this
histogram only supports "every sampled batch's `0x10` bit reads clear," which
is the fact the ground-truth test actually pins and the fact
`race::Scene`'s `exhaust::TRAIL_BLEND` choice for the boost plume actually
needs.

Taken together, this resolves the open question for the boost plume
specifically at **confidence 75** (not this page's headline 85: the byte
identity is solid, but "every sampled batch reads additive" leans on a
same-valued sample not distinguishing the two readings above) - every
sampled batch reads additive, not a guess from the `_ADD` texture name - and
is suggestive, not yet confirmed project-wide, that
`mesh_render::TRANSPARENT_BLEND`'s current `SrcAlpha`/`OneMinusSrcAlpha` lerp
may be the wrong equation for every transparent batch on every track and
ship, not only the boost plume. That is a much bigger blast radius than this
page's scope (see "What is still not recovered" below) and needs its own
screenshot-driven verification before any project-wide change.

## What is still not recovered

- **Whether `GU_BLEND` is enabled when the additive branch runs.**
  `Mesh_SetBatchDrawState` never calls `Gu_Enable`/`Gu_Disable(4)`
  (`GU_BLEND`'s state index). The two `Gu_CallList` replays it does make,
  selected by `*batch & 0x8000` against `mesh+0x70`'s `+0x48`/`+0x70` prebuilt
  lists, are the most likely place blend-enable actually lives - not read this
  pass. For the additive branch this matters (enabled means true `dst + src`
  glow, disabled means the same overwrite as the replace branch); for the
  replace branch it does not, since a zero destination factor and a disabled
  stage look identical either way. The convergent match with `Trail`'s own
  already-live-confirmed-enabled blend state is treated here as strong
  circumstantial support (confidence 75, not higher) rather than a
  substitute for reading the two lists.
- **Whether the loader rewrites batch-header `+3` between disc and the draw
  path.** Not searched for. The stride-arithmetic match earlier on this page
  supports the *offsets* being read as-authored; it does not by itself rule
  out an in-place fixup of this specific byte. This is the caveat behind the
  confidence-75 figure on "every sampled batch reads additive" above.
- **The PS2 equivalent draw path.** This page is PSP-only (`BOOT.BIN`);
  `Batch::header_flags` is populated identically for PS2 batches (the header
  layout is shared - see that field's own doc comment) but no PS2 function has
  been read to confirm the same bit means the same thing there.
- **Whether `mesh_render::TRANSPARENT_BLEND` should change project-wide.**
  Already self-documented as unconfirmed in
  `crates/render/src/mesh_render.rs`; this page's measurement is evidence
  against its `SrcAlpha`/`OneMinusSrcAlpha` lerp, in favour of the `GU_FIX`
  additive equation, but changing it touches every transparent batch on every
  track and ship and needs its own verification pass.
