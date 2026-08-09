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
        // depth-range pair from mesh+0xcc/+0xd0 and a depth-bias byte at
        // batch+0x29, fed to FUN_0891e988
    } else {
        Gu_DepthMask(0); Gu_DepthFunc(6);
    }
    FUN_0891e988(g_display, depth_range_lo, depth_range_hi, depth_bias);

    if ((*batch & 0x20) == 0) Gu_Enable(5); else Gu_Disable(5);   // GU_CULL_FACE

    if ((batch[flags_byte] & 0x10) == 0)
        Gu_BlendFunc(GU_ADD, GU_FIX, GU_FIX, 0xffffff, 0xffffff); // dst + src
    else
        Gu_BlendFunc(GU_ADD, GU_FIX, GU_FIX, 0xffffff, 0);        // src, dst discarded

    if ((*batch & 0x40) == 0) { if (DAT_08b32c60 != 0) FUN_08907828(DAT_08b32c60, 0); }
    else                        FUN_08811898(0);
}
```

**`batch+0x29` is a depth bias, not an alpha reference** - corrected
2026-08-08, after an earlier version of this page named it `alpha_ref` from its
position in the call rather than from the callee. `FUN_0891e988`'s whole body
is `FUN_088111cc(ref * 6 + lo, ref * 6 + hi)`, i.e. it shifts both ends of the
`sceGuDepthRange` pair by the same `ref * 6`, which is a per-batch depth offset
and cannot be an alpha test of any kind. Nothing on this page depended on the
old reading, but a search for where the plume's alpha reaches the GE did, which
is how it was caught (see [exhaust.md](exhaust.md)).

(Renamed from the literal decompile for readability; `batch[flags_byte]` is
`*(byte *)((int)batch + 3)`, `GU_CULL_FACE` = state index `5` per the PSP SDK's
standard `sceGuEnable` constants, already load-bearing for
[`Batch::is_culled`](../../../../crates/formats/src/vex.rs) reading `*batch &
0x20` - though **not** matching it: the bit set takes the `Gu_Disable` branch,
so the predicate is its negation. See "The plume is two-sided" below.)

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

**The list replayed before the draw cannot override any of this, and that is
now read rather than assumed** (2026-08-08). In `FUN_089307b4`'s `& 2` group a
`+0xc0` display list is replayed when the material *changes*, and
`Mesh_SetBatchDrawState` is called **after** it, once per batch.

**That list belongs to the `Texture`, not to the material** - corrected
2026-08-08. `FUN_0890db54` reaches it as `texture_array[material.tex_index] +
0xc0` and `FUN_089307b4` hands the same pointer to `Gfx_BindTexture` in its
other branch; it is built by `Texture_BuildBindList` (`0x08928d4c`) and its
complete contents are texture state only - `TEXLEVEL`, `TEXTUREMAPENABLE`,
`TEXMODE`, `TEXFORMAT`, the per-mip `TEXADDR`/`TEXBUFWIDTH`/`TEXSIZE`, the CLUT
registers and `TEXFLUSH`. **No colour state at all, and no `0xc9` texture
function.** The conclusion the old wording supported still holds - nothing
replayed there can override the blend - but the attribution was wrong, and
anyone looking for a *material* list will not find one at this offset.

Either way the two fixed-factor equations above are the only ones a transparent
batch can be drawn with, which is what matters: a replayed list was the obvious
place to look for a missing alpha path, and it is ruled out - see
[exhaust.md](exhaust.md)'s section on the boost plume's vertex alpha.

**`mesh+0x70` is a pointer to the per-*model* object, not a per-mesh block** -
also corrected 2026-08-08. It is `Vex_LoadModel`'s own object (`model+0x1a4` is
the texture-pointer array, dereferenced as `*(int *)(mesh + 0x70) + 0x1a4` in
`FUN_0890cf84`), so sibling meshes in one file share it. Both of
`shipboost.vex`'s meshes therefore replay the *same* two lists.

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
is the fact the ground-truth test actually pins and the fact `race::Scene`'s
additive-blend choice for the boost plume actually needs.

**`race::Scene` draws the boost plume with `exhaust::BLEND`, not
`exhaust::TRAIL_BLEND`, and that is a deliberate departure from this page.**
Both are additive; they differ only in the source factor, `SrcAlpha` against
`One`. The plume is the one model whose authored vertex alpha is not constant -
bimodal, `0` on the orange rim and `255` in the white core - and a matched-pose
capture of a real pad crossing shows the original's fins feathering exactly
where that alpha falls, at a hue our `One`-factor build could not produce. The
recovered call here really is `GU_FIX` white on both sides, so **the GE path
that performs that per-fragment weight is not recovered**; the renderer
reproduces the measured observable and the mechanism stays open. The numbers,
and why the previous per-vertex premultiply was a bug rather than a
compensation, are in
[exhaust.md](exhaust.md#the-plumes-vertex-alpha-the-question-was-wrong-and-the-bug-was-ours).

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

**That verification was tried, once, informally, and the result is a
rejection - not committed, but worth recording so nobody repeats it
expecting a different answer.** `mesh_render::TRANSPARENT_BLEND` was swapped
locally from its `SrcAlpha`/`OneMinusSrcAlpha` lerp to the pure-additive
`One`/`One` this page recovers, and 01_Track's start straight (the same
`--race --track 'Data\Environments\01_Track\track.vex' --hold cross --ticks
205` capture `MESH_BLEND_ALPHA_PLAN.md` used) was re-rendered. The tilted
glass canopy panels either side of the track - a `transparent_draws` batch,
correctly classified - go from a readable tinted-glass look with visible
interior detail to a blown-out, nearly solid white/cyan glow. Reverted
immediately; never landed. This is exactly what the "is `GU_BLEND` actually
enabled" open question above predicts if that particular batch does *not*
have blending enabled in the original: forcing an always-summing blend onto
content that the real hardware may draw closer to a plain overwrite would
produce precisely this kind of blow-out. Not proof - a single content
example, at confidence 60 as a data point rather than a finding - but a real
regression on real content, and reason enough on its own not to flip the
constant without first reading the two `Gu_CallList` lists this page already
names as the cheapest way to settle blend-enable.

**One thing that was blamed on this page's open question and turned out not to
be about blending at all.** `16_Track`'s start-line gantry carried a blown-out
white board, and because a `col_banners2_ADD.tga` sign sits beside it the fault
was recorded as a candidate symptom of the blend question above. It was not.
The white board is `billboard8.tga`, an 8x8 4-bit texture, drawn by an
**opaque** batch - no blending involved on either side - and it was white
because `oag_formats::vex::textures` read its rows back to back where the
format pads each row to 16 bytes. Fixed there; see
[`vex.md`](../../../formats/vex.md#texture-rows-are-padded-to-16-bytes).
Recorded here so the blend question is neither credited with it nor reopened
for it: the two `Gu_CallList` lists below are still unread, and still the way
to settle blend-enable.

## The GE state a transparent batch is actually drawn under

2026-08-08, static read of `BOOT.BIN` throughout - **no live verification**, so
every figure here is a decompile and the confidences say so. This is what
finally answered [exhaust.md](exhaust.md)'s three-pass-old question about the
boost plume's vertex alpha, and the answer is that no alpha path exists.

### The two `Gu_CallList` lists are light directions, and that guess was wrong

Both are 9 words, capacity `0x28`, terminated `0x0b000000` (GE `RET`, which is
what `Gu_CallList`'s `CALL` needs). Written as raw GE words by `Vex_LoadModel`
(`0x08912b80`, at `0x08913258`-`0x089135b8`) and by `Vex_UpdateLightLists_q`
(`0x08912358`):

```text
list A (model+0x48)            list B (model+0x70)
0x63 LX0 = dir0.x              0x63 LX0 = dir0.x
0x64 LY0 = dir0.y              0x64 LY0 = dir0.y
0x65 LZ0 = dir0.z              0x65 LZ0 = dir0.z
0x5f LIGHTTYPE0 = 0            0x5f LIGHTTYPE0 = 1
0x66 LX1 = dir1.x              0x66 LX1 = dir1.x
0x67 LY1 = dir1.y              0x67 LY1 = dir1.y
0x68 LZ1 = dir1.z              0x68 LZ1 = dir1.z
0x60 LIGHTTYPE1 = 0            0x60 LIGHTTYPE1 = 1
0x0b RET                       0x0b RET
```

So **`*batch & 0x8000` selects only the light computation field** (`0` =
diffuse-only, `1` = diffuse+specular). No blend, no alpha test, no texture
function, no ambient, no material, no lighting-enable in either list.
Confidence **92**.

That closes this page's own first open question, and it is worth recording that
the *prediction* was wrong: these lists were named here as "the most likely
place blend-enable actually lives", and they are nothing of the kind.

### Where the state really is: `Mesh_BeginTransparentPass` (`0x0890d904`)

Called by `FUN_0890d508` and by `FUN_089307b4`'s `& 2` group immediately before
the batch loop:

```text
Gu_Disable(10)               GU_LIGHTING off
Gu_SpecularCoef(...)         0x5b MATERIALSPECULARCOEF = DAT_08abf490
Gu_TexMapMode(2, 0, 1)       0xc0 TEXMAPMODE uvgen = 2, 0xc1 TEXSHADELS LS0=0 LS1=1
Gu_Enable(4)                 GU_BLEND ON
Gu_CallList(model+0xb0+0xd0) material ambient / alpha / diffuse / specular
Gu_AlphaFunc(1, 0, 0xff)     0xdb  func ALWAYS, ref 0, mask 0xff
Gu_Disable(0)                GU_ALPHA_TEST OFF
Gu_Disable(0x11)             GU_COLOR_TEST off
```

- **`GU_BLEND` is enabled.** Confidence 90. The additive branch is a true
  `dst + src` glow.
- **`GU_ALPHA_TEST` is disabled**, and the func programmed is `ALWAYS` / ref `0`
  / mask `0xff`. Confidence 90. Any "the rim is alpha-clipped" reading is dead.
- **Lighting is disabled for transparent batches** - twice, since
  `FUN_089307b4`'s own first statement is also `Gu_Disable(10)`. So the `Trail`
  ribbon's lit-colour path (lighting on with all four lights off) does **not**
  apply to mesh batches, and the two lists above are inert as lighting.
  Confidence 85.

**This rests on the full `sceGuEnable` index table, read as a literal switch out
of `Gu_SetState` (`0x08811b58`) rather than recalled from the SDK** - confidence
95: `0`→ALPHATEST, `1`→ZTEST, `2`→scissor, `3`→STENCILTEST, `4`→ALPHABLEND,
`5`→CULLFACE, `6`→DITHER, `7`→FOG, `8`→DEPTHCLAMP, `9`→TEXTUREMAP,
`10`→LIGHTING, `11`-`14`→LIGHT0..3, `15`→AA, `16`→PATCHCULL, `17`→COLORTEST,
`18`→LOGICOP, `19`→REVERSENORMAL, `20`→PATCHFACING, `21`→TEXFUNC with the
colour-double bit. That retro-confirms this page's `GU_CULL_FACE = 5` and
`exhaust.md`'s `Trail` decode. **Note `GU_ALPHA_TEST` is index `0`, not `3`;
index `3` is `GU_STENCIL_TEST`.**

### The texture function is never set in this path

`Texture_BuildBindList` (`0x08928d4c`) emits no `0xc9`, and **the entire binary
contains only two `0xc9` emitters** - `Gu_TexFunc` (`0x0881141c`, the one
`Gfx_Init` calls once) and `Gu_SetState`'s case `0x15` (the `GU_FRAGMENT_2X`
path) - searched as `lui *, 0xc900`, two hits, both accounted for. So the mesh
path inherits `Gfx_Init`'s `GU_TFX_MODULATE` / `GU_TCC_RGBA` with colour-double
off. Confidence 90, and it closes `exhaust.md`'s ribbon texfunc question a
second, independent way (that one was closed by a live command-stream scan).

**The two arguments are now read rather than assumed** (2026-08-09). The single
call site is `0x0891f62c`, and the two instructions around it set `a0 = 0`
(`move a0, zero` at `0x0891f628`) and `a1 = 1` (the delay slot). `Gu_TexFunc`'s
body packs them as `(colour_double << 16) | (tcc << 8) | tfx | 0xc9000000`, so
that is `tfx = 0` = **`GU_TFX_MODULATE`** and `tcc = 1` = **`GU_TCC_RGBA`**,
exactly as the paragraph above states. Confidence **92**. This matters beyond
bookkeeping: `MODULATE` is what makes `fragment.rgb = vertex.rgb * texel.rgb`
an identity the rim argument above depends on, and had it been `GU_TFX_REPLACE`
the authored vertex colour would have been discarded entirely and the plume's
orange rim would never reach the framebuffer - which would have been the whole
answer to [exhaust.md](exhaust.md)'s missing-multiplier question. It is not.

### A trap: `model+0xb0` holds two lists and the obvious one is the wrong one

- `model+0xb0 + 0x00`, capacity `0x94`, built by
  `SceneLight_BuildLightingList_q` (`0x0887a4f0`): `0x17000001`
  **LIGHTINGENABLE = 1**, ambient colour/alpha, light enables, light positions,
  per-light colours.
- `model+0xb0 + 0xd0`, capacity `0x20`, built by `SceneLight_Rebuild`
  (`0x08879b14`): `MATERIALAMBIENT`, `MATERIALALPHA`, `MATERIALDIFFUSE`,
  `MATERIALSPECULAR`, all four from one colour.

Two sibling replay functions, and **they split exactly along
opaque/transparent**: `SceneLight_CallLightingList` (`0x0887a230`) replays
`+0x00` and its only caller is `FUN_0890d3ac`, the opaque/alpha-tested state
function; `SceneLight_CallMaterialList` (`0x0887a268`) replays `+0xd0` and its
only caller is `Mesh_BeginTransparentPass`. So opaque batches are lit,
transparent batches are not, and nothing in the transparent path ever emits
`0x17`. **Anyone who greps `0x0887a4f0`, finds `0x17000001` and concludes
transparent batches are lit has it backwards.**

### Nothing else scales fragment RGB

Full ordered state for a transparent batch, every item read from the binary:
`FUN_089307b4` entry disables lighting, enables `TEXTURE_2D` and sets all four
material registers white via `Gu_Color(0xffffffff)`; the `& 2` prologue above
runs; then per batch the texture bind list (texture state only), the mesh's
world matrix, `Mesh_SetBatchDrawState`, and the draw. The epilogue
`Mesh_EndTransparentPass` (`0x0890dc20`) restores uvgen `0`.

**The only multipliers are vertex colour x texture (`MODULATE`) x the blend
equation.** No fog, no colour test, no texenv colour, no colour-double, no
emissive (`MATERIALEMISSIVE` `0x54` still has no emitter anywhere), no logic op.
Confidence **85**. That is a hard negative for `exhaust.md`'s long-running
"something scales the plume's rim down without discarding its hue" - it is not
GE state.

### The transparent pass generates its texture coordinates

`Mesh_BeginTransparentPass` sets `TEXMAPMODE` uvgen **2 - environment (shade)
mapping** - from the vertex normal and lights `0`/`1`, and **nothing inside the
batch loop re-emits `0xc0` to undo it**: the whole binary has only two `0xc0`
emitters, `Gu_TexMapMode` (`0x08811508`) and `Gu_TexProjMapMode`
(`0x08811558`), and `Gu_TexMapMode`'s thirteen call sites are all pass
prologues and epilogues. So **authored UVs are not read for a transparent
batch.** Confidence **82** - the uvgen enum values come from pspgu/PPSSPP
rather than from `BOOT.BIN`, which is what holds it below 90.

What makes the choice deliberate rather than incidental: `FUN_0890dc64`, the
`& 8` pass prologue, sets uvgen **1** with `TEXSHADELS (0,0)`, while this one
sets uvgen **2** with `TEXSHADELS (0,1)`. `TEXSHADELS` only means anything under
shade mapping, and only the mode-2 pass sets it non-trivially. **And this is
what the two light-direction lists are for** - they program the exact registers
env-map UV generation reads, replayed per batch inside the pass that turned it
on.

The plume's degenerate authored UVs are explained by this rather than fixed by
it. See [exhaust.md](exhaust.md).

**Measured 2026-08-09 from the art, and it is what makes this pass matter
visually rather than academically:** `Data\Tex\pulse_boost2_ADD.tga` is 64x16
and is a *streak lookup*, not a surface texture - `u` is a brightness ramp from
a uniform white `(250, 248, 250)` at column 0 through violet `(143, 72, 218)`
to `(10, 4, 18)`, and `v` alternates hard row by row between a bright magenta
`(241, 110, 253)` and a dark navy `(20, 5, 122)`, which is the axis that breaks
a fin into discrete bands. `Data\Ships\Assegai\shipboost.vex`'s 121 authored
texcoords span `u` in `[0, 0.0078125]` - `1/128`, i.e. **column 0** - and that
is precisely the one column where the `v` alternation vanishes. So sampling the
authored UVs returns a constant white and the plume collapses to flat vertex
colour; every bit of the plume's structure comes from uvgen 2 replacing those
coordinates. Pinned across all eight PSP teams by
`crates/game/tests/boost_plume_ground_truth.rs`.

### The uvgen-2 equation, and which space each term is in

2026-08-09. The GE's own behaviour is not in `BOOT.BIN` - it is in the
hardware - so this half is emulator/SDK sourced and says so. **Two independent
PPSSPP backends were read and they agree**, which is what lifts it above a
single recollection:

- software rasteriser - `GPU/Software/Lighting.cpp`, `GenerateLightST` and its
  helper `GenerateLightCoord`:
  `s = (Dot(GetLightVec(gstate.lpos, getUVLS0()).NormalizedOr001(), worldnormal) + 1) / 2`,
  and the same with `getUVLS1()` for `t`.
- hardware transform - `GPU/Common/VertexShaderGenerator.cpp`, the
  `GE_TEXMAP_ENVIRONMENT_MAP` case:
  `v_texcoord = vec2(1.0 + dot(normalize(u_lightpos[ls0]), worldnormal), 1.0 + dot(normalize(u_lightpos[ls1]), worldnormal)) * 0.5`,
  with a `length(u_lightpos) == 0.0 ? worldnormal.z` fallback that is the same
  degenerate case `NormalizedOr001` encodes on the software side.

So the equation the task set out to confirm is **confirmed, with three
qualifications that the one-line form hides**:

1. **The light is used as a raw direction, normalised, and `LIGHTTYPE` is not
   consulted.** `GenerateLightCoord` reads `gstate.lpos` and normalises it; there
   is no `position - vertex` term and no directional/positional branch. So the
   `0x5f`/`0x60` `LIGHTTYPE` words the two lists differ in are **inert for uvgen
   2 in PPSSPP's model** - which is why `*batch & 0x8000` stays a gap here (see
   "What is still not recovered"), not a closed question.
2. **The normal is the *world* normal and it is normalised.**
   `TransformUnit::ModelToWorldNormal` is `Norm3ByMatrix43(coords,
   gstate.worldMatrix)` - the **world matrix only**, not the view matrix -
   followed by an unconditional `worldnormal.NormalizeOr001()`; the hardware
   path writes `normalizeOr001(mul(vec4(normal, 0.0), u_world).xyz)`. Both
   normalise, so a non-unit authored normal or a scaled world matrix does *not*
   move the UVs.
3. **The light vectors are in the same world space**, i.e. the space `u_world`
   maps into, with no view transform applied to either side.

Point 2 only means "world space" if this engine actually keeps a separate view
matrix, and it does: `FUN_08900884`/`FUN_08900a9c` write the camera node's 4x4
at `cam+0x50` into the display's view stack at `g_display + 0x1410 +
*(g_display + 0x1694) * 0x40` and then call `Gu_SetMatrix(1, ..)` - GE matrix
**1**, the view matrix (`0x3c`/`0x3d`, pinned in [exhaust.md](exhaust.md)). The
mesh path sets GE matrix **2** per batch. So GE world really is model-to-world
and no model-view folding happens for meshes. **This is the fork that decides
whether the effect is a matcap, and it lands on "not a matcap".** Confidence
**88** on the equation and on both spaces; the uvgen enum value `2` itself keeps
its **82** because it still comes from `pspgu.h`
(`GU_ENVIRONMENT_MAP = 2`, `sceGuTexMapMode(mode, lu, lv)`) rather than from
`BOOT.BIN`.

### The two light vectors are a fixed world-space pair, built once at load

Both writers of the `+0x48`/`+0x70` lists were read to the instruction. **They
do not read the same source, and which one runs is decided by `model+0x1a8`:**

| `model+0x1a8` | writer | `dir0`/`dir1` source |
| --- | --- | --- |
| `!= 0` | `Vex_LoadModel` (`0x08913258`-`0x089135b8`), once, at load | columns 0 and 1 of a fixed 4x4 global at `g_envmap_light_basis` (`0x08abf510`) |
| `== 0` | `Vex_UpdateLightLists_q` (`0x08912358`) | **rows** 0 and 1 of the live view matrix at `g_display + 0x1410 + *(g_display + 0x1694) * 0x40` |

**The two rows of that table say "columns" and "rows" because the two writers
use different address strides over matrices that are stored the same way.** Both
are PSP 4x4 column-major - four consecutive floats are one column, translation
lands at `+0x30`, which is what [exhaust.md](exhaust.md)'s live read of the view
stack saw as "translation in row 3". `Vex_LoadModel` reads `+0x00`/`+0x04`/`+0x08`
- **consecutive**, so one whole column. `Vex_UpdateLightLists_q` reads
`+0x1410`/`+0x1420`/`+0x1430` and `+0x1414`/`+0x1424`/`+0x1434` - **stride
`0x10`**, so one element out of each of the first three columns, which is a
matrix row. Do not read the two offset patterns as a transcription
inconsistency; the stride is the whole difference.

For a world-to-view rigid transform, rows 0 and 1 are the camera's right and up
axes in world space, and `dot(right, N_world)` is `N_view.x`. **So the runtime
branch is a matcap**: `uv = 0.5 * (normalize(N_view).xy + 1)`. Worth writing
down even though the plume does not take it, because it is what a reader
expecting PPSSPP's formula will assume the plume does.

**The boost plume does not take that branch.** `ExhaustFlare_Init`
(`0x08905444`) walks the ancestor chain for the `FUN_08a6bfc0` class token,
stores the hit at `flare+0xc0`, and then - for the model it builds from
`"%s\%sboost.vex"` with that ancestor's team name at `+0x370`→`+0x94` - writes
`*(int *)(model + 8) = flare->0xc0` **before** calling `Vex_LoadModel`.
`model+0x8` is the parent link (`node+0x4` is the type token, `node+0x8` the
next link up - the same three-field walk `SceneLight`'s `model+0xac` lookup
uses). So `Vex_LoadModel`'s own walk at `0x08913048` matches on its **first**
step and sets `model+0x1a8 = 1`. The plume therefore takes the **load-time,
fixed-basis** path, and `Vex_UpdateLightLists_q`'s block never runs for it.
Confidence **88**; the store and the walk are both instruction level, the
identification of the token class as the craft is inferred from the team-name
field it is immediately read for.

`g_envmap_light_basis` is built in `Vex_LoadModel` at
`0x08913078`-`0x08913248`, and its only two references in the whole binary are
that write and the read at `0x08913258` - it exists for nothing else. The build
is, at instruction level:

- `g_envmap_light_basis_angle_x` (`0x08abf500`) `= -1.0`, and
  `g_envmap_light_basis_angle_y` (`0x08abf504`) `= 0.3`, both read-only
  (`0x08abf500` has exactly one xref, the read at `0x0891307c`) - so they are
  constants, not tunables. `vcst.s S002, 2/PI` then `vmul.s` before `vcos.s` /
  `vsin.s` converts radians into the VFPU's half-cycle units, so **the units are
  radians**.
- the `vpfxs` shuffles build a plain `Rx(-1.0)` (columns `(1,0,0)`,
  `(0,cA,sA)`, `(0,-sA,cA)`) and a plain `Ry(0.3)` (columns `(cB,0,-sB)`,
  `(0,1,0)`, `(sB,0,cB)`), column-major.
- `vmmul.q E000,E100,E200` at `0x089131d0` combines them. **`vmmul`'s operand
  order is the trap here** - it transposes, and the two candidate readings give
  different vectors, so it was decoded rather than recalled. The printed
  register names resolve through the Allegrex module's attach tables
  (`data/tools/ghidra-allegrex/data/languages/allegrexVfpuBase.sinc`, where
  `vs` uses the inverted `vs_e` table and `vt`/`vd` the `vt_m`/`vd_m` one) to
  raw fields `vs = 4`, `vt = 40`, `vd = 32`. Those decode as matrix `1`, `2`, `0`
  respectively - which matches the printed `E100`/`E200`/`E000` and so confirms
  the table indexing - with transpose bits `0`, `1`, `1`. Pushing them through
  PPSSPP's `ReadMatrix`/`WriteMatrix` (`Core/MIPS/MIPSVFPUUtils.cpp`) and
  `Int_Vmmul` (`Core/MIPS/MIPSIntVFPU.cpp`, `d[a*4+b] = dot(&s[b*4], &t[a*4])`)
  gives `d[a*4+b] = (S2 · S1)(a, b)` and a transposed store that leaves the
  global equal to **`S2 · S1` = `Rx(-1.0) · Ry(0.3)`**. Matrix 2 was loaded from
  the `Rx` scratch (`sp+0x3030`) and matrix 1 from the `Ry` scratch
  (`sp+0x3070`).

`dir0` is that product's column 0 and `dir1` its column 1:

```text
dir0 = ( cos B, sin A * sin B, -cos A * sin B ) = ( 0.9553365, -0.2486722, -0.1596704 )
dir1 = ( 0,     cos A,          sin A         ) = ( 0.0,        0.5403023, -0.8414710 )
      A = -1.0 rad, B = 0.3 rad
```

They are orthonormal to within `f32` (`|dir0| = |dir1| = 1.000`,
`dir0 · dir1 = -2.5e-5`), which is the check that the composition order came out
right rather than transposed into something skewed. Confidence **95** on the two
angle constants and on the two `Rx`/`Ry` builds; **85** on the numeric vectors -
the `vmmul` operand order is derived from PPSSPP's matrix-register semantics
rather than measured, though the derivation was run twice by different routes
and reproduced. A transposed reading would instead give
`dir0 = (cos B, 0, -sin B)`, `dir1 = (sin B sin A, cos A, cos B sin A)`, so
**the two are distinguishable by eye at runtime: the recovered `dir1` has
`x == 0` exactly, the alternative has `dir0.y == 0` exactly.**

### Consequences for the plume: ship-locked, not camera-locked

- **Per model, never per batch.** The lists live on the model object
  (`mesh+0x70`), and both of `shipboost.vex`'s meshes share it - already
  established above. The only per-batch difference is `*batch & 0x8000`
  selecting list A or B, and the two differ **only** in `LIGHTTYPE`, which uvgen
  2 does not read.
- **Both vectors are constants for the whole run** on this path - written once
  in `Vex_LoadModel` from a global that is itself built from two literals.
- **They do not rotate with the ship.** The model matrix rotates the *normal*;
  `lpos` is world-space and untouched. So the UVs move when the **ship**
  rotates and stay put when the **camera** moves. A reimplementation that makes
  the plume shimmer as the camera orbits a stationary ship has it wrong.

### A shader-ready form

With `N` the model-space authored normal and `M` the ship's model-to-world
rotation:

```wgsl
let n = normalize((model_rot * vec4<f32>(normal, 0.0)).xyz);
let L0 = vec3<f32>( 0.9553365, -0.2486722, -0.1596704);
let L1 = vec3<f32>( 0.0,        0.5403023, -0.8414710);
uv = vec2<f32>(1.0 + dot(L0, n), 1.0 + dot(L1, n)) * 0.5;
```

`L0` drives `u` and `L1` drives `v` because `Mesh_BeginTransparentPass` emits
`TEXSHADELS` `LS0 = 0`, `LS1 = 1` and list A puts `dir0` in light 0 (`0x63`-`0x65`)
and `dir1` in light 1 (`0x66`-`0x68`). `L0`/`L1` are already unit, so the
`normalize` PPSSPP applies to them is a no-op here.

**Unverified, and cheap to get wrong: the `v` axis orientation.** Nothing here
pins whether the GE's `t` runs the same way as this project's texture upload,
and the equation is symmetric enough that a flipped `v` looks plausible rather
than broken. Treat `v = 1.0 - v` as the first thing to try if the plume reads
mirrored against a captured reference.

### The load-time list-B bug is real but cannot fire on the plume

This page already records, at confidence 95 on the words, that
`Vex_LoadModel`'s list B emits `0x63`/`0x64`/`0x65` + `0x5f000001` twice - light
**0** written with `dir0` and then overwritten with `dir1`, light 1 never
touched - where `Vex_UpdateLightLists_q`'s twin correctly emits
`0x66`/`0x67`/`0x68` + `0x60000001`. Under uvgen 2 that would make `u` and `v`
resolve toward the same vector and sample the texture along its diagonal.

**Measured 2026-08-09, and it does not happen here:** every batch of every one
of the eight PSP teams' `Data\Ships\<Team>\shipboost.vex` (2 meshes x 2 batches
x 8 teams = 32 batches) carries `pass_mask = 0x1232` - identical across all
eight teams - so `& 0x8000` is **clear** on all of them and every plume batch
replays **list A**, which is correct on both paths. `0x1232` is **not** a fully
decoded value: `& 0x0200` is what classifies these transparent, `& 0x0020` is
set - which by this page's own `if ((*batch & 0x20) == 0) Gu_Enable(5); else
Gu_Disable(5)` means **`GU_CULL_FACE` is disabled and the plume is drawn
two-sided** - and `& 0x1000` is undecoded here. Read off
`data/images/pulse-psp-usa.chd` through `oag_formats::vex::mesh_batches`. The
same dump shows the authored normals are already unit to within the 8-bit
quantisation (`|n|` in `[0.9923, 1.0000]` across all 32 batches), so the
`normalize` in the equation above is close to a no-op on real plume data - it
is kept because the world matrix, not the authored normal, is what would break
it.

## Every per-batch state setter, read to its command byte

2026-08-09, after `exhaust.md` re-opened "something scales the plume's rim down
and it is not the blend". `Mesh_SetBatchDrawState`'s five state calls were each
followed into the callee and identified by the GE command byte the callee
writes, rather than by name or by position. **This is a negative result and it
is the point of the section**: there is no source-alpha term anywhere in the
chain.

| Call, for the plume's batches | Address | Emits | Meaning |
| --- | --- | --- | --- |
| `Gu_CallList(model+0x70 + 0x48)` | - | replay | light list A (`& 0x8000` clear) |
| `Gu_DepthMask(1)` | `0x08811874` | `0xe7` | `ZWRITEDISABLE` - depth **writes off** |
| `Gu_DepthFunc(6)` | `0x08811850` | `0xde` | `ZTEST` function |
| `FUN_0891e988(..)` | - | `sceGuDepthRange` | per-batch depth bias, already read |
| `Gu_Disable(5)` | `0x08810db8` | enable table | `GU_CULL_FACE` **off** |
| `Gu_BlendFunc(0,10,10,0xffffff,0xffffff)` | `0x0881197c` | `0xdf`,`0xe0`,`0xe1` | `BLENDMODE` `(op<<8)|(dst<<4)|src`, `BLENDFIXEDA`, `BLENDFIXEDB` |
| `Gu_PixelMask(0xff000000)` | `0x08811898` | write mask | alpha channel not written, RGB written |

The `0xdd` anchor is what fixes the rest of that range: `Gu_StencilOp`
(`0x08811948`) writes `0xdd` with three packed fields, which pins `0xdd` =
`STENCILOP` and therefore `0xde` = `ZTEST` and `0xe7` = `ZWRITEDISABLE`.
`Gu_BlendFunc`'s body independently confirms it is `sceGuBlendFunc`: the
`(op << 8) | (dst << 4) | src` packing plus two 24-bit fix colours is that
function's exact encoding, which raises this page's `dst + src` reading from a
call-site argument match to a callee-body read. **Confidence 92.**

**Trap, and it is the reason this section exists.** In the live Ghidra database
`0x08811874` and `0x08811850` currently carry the stale names
**`Gu_BlendFunc_q`** and **`Gu_StencilOp_q`**, while
[`names.tsv`](names.tsv) and [exhaust.md](exhaust.md) correctly call them
`Gu_DepthMask` and `Gu_DepthFunc`. Anyone reading the decompiler rather than the
docs sees `Gu_BlendFunc_q(1)` sitting in `Mesh_SetBatchDrawState` ahead of the
real `Gu_BlendFunc` and concludes there is a second, earlier blend path to
investigate. **There is not** - `0xe7` is the depth-write mask. Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md) the docs are
authoritative; the reading here was arrived at from the command bytes and
happens to confirm the documented names, which is why their confidence goes
`85` -> `90`.

### The plume is two-sided, and `Batch::is_culled` reads backwards

`pass_mask & 0x20` **set** takes the `Gu_Disable(5)` branch, and state index `5`
is `GU_CULL_FACE` from `Gu_SetState`'s literal table (confidence 95, above). So
a batch with the bit set is drawn with **culling off - two-sided**. Every one of
the plume's 32 batches has it set.

`oag_formats::vex::Batch::is_culled` **was** `pass_mask & 0x0020 != 0`, with a
doc comment reading "whether back-face culling is enabled; clear means
two-sided". That is inverted against the GE call: the bit set means culling
*disabled*. Confidence **90**; the branch is read at instruction level and the
state-index table it depends on is the same one this page already scores 95.

**Corrected in the same change, 2026-08-09** - the predicate is now
`pass_mask & 0x0020 == 0` and its tests are flipped.

**It had no effect on any picture, and the reason is worth recording** so the
next reader does not go looking for one: **nothing in the workspace consumes
that predicate**, and every `mesh_render` pipeline sets `cull_mode: None`
deliberately (strip winding is reconstructed rather than read from the file, so
culling would turn a winding mistake into missing geometry). The
reimplementation therefore already draws the plume two-sided, matching the
original by accident rather than by reading this bit. The first consumer would
have culled exactly the surfaces the original draws two-sided - on the boost
plume, all 32 of its batches - which is why it is fixed now rather than left as
a comment.

### Environment-generated UVs do not suppress the orange rim

Measured 2026-08-09 over `Data\Ships\Assegai\shipboost.vex`'s 121 vertices,
using the light vectors recovered above:

| Authored colour | Alpha | Count | Generated `u` range | Mean `u` |
| --- | ---: | ---: | --- | ---: |
| `(255, 255, 255)` white | `255` | 63 | `[0.074, 0.750]` | 0.505 |
| `(255, 98, 5)` orange | `0` | 58 | `[0.074, 0.750]` | 0.437 |

Whole-model generated ranges are `u` in `[0.074, 0.750]`, `v` in
`[0.691, 0.985]`. **The two colour classes are not separated** - the orange
vertices span the same range as the white ones and sit slightly *brighter* on
the ramp, so switching from the authored UVs to generated ones cannot be what
darkens the rim. It moves the whole model off the texture's white column, which
is a large and correct change, but it is orthogonal to the rim.

This closes off one branch cleanly. `pulse_boost2_ADD` under `MODULATE` cannot
recolour an orange vertex either: with an authored blue of `5/255`, `blue_out =
0.02 * texel_blue` for any texel, so no sampled coordinate makes an
orange-vertex fragment blue-dominant. **The rim can only ever be darkened, never
turned violet** - which means whatever the missing term is, it is a multiplier
on the rim's contribution and not a coordinate change. Confidence **90** on the
measurement, which is arithmetic over decoded data.

The vertex-colour split itself is **not new here** - `exhaust.md` measured the
two-value rim/core split and the constant texture alpha of 238 on 2026-08-08.
What is new is that the recovered texgen does not act on it.

### The per-material texture transform: mechanism found, values not

[texture-animation.md](texture-animation.md) established the `& 0x10` gate on
2026-08-08 and left "where the block's values come from" at confidence **0**.
The writer chain is now read, and it does not close that gap:

`FUN_0890e160(mesh)` runs when `mesh+0x40` (a time) differs from `mesh+0x18c`,
walks every material, and for each one whose flags carry `& 0x10` calls
`FUN_08927204(time, material_block)` then `FUN_08927358(list_slot,
material_block)`. `FUN_08927358` is a five-word list builder - `0x48`
`TEXSCALEU`, `0x49` `TEXSCALEV`, `0x4a` `TEXOFFSETU`, `0x4b` `TEXOFFSETV`, `RET`
- reading floats at block `+0x18`/`+0x1c`/`+0x20`/`+0x24`, which is the same
block layout `FUN_089271cc` submits immediately. `FUN_08927204` is a curve
evaluator over a normalised time, and **its failure path is the important
part**: when the scale track evaluates to nothing it writes `1.0`/`1.0`, and
when the offset track does it writes `0`/`0`.

So the per-material texture transform **defaults to identity**. It is a live
mechanism for the plume - its material's `0x212` carries the bit, confirmed
again here on both meshes - whose actual values are still unknown, and absent
animation data it is a no-op. Treating it as the missing multiplier is not
supported by anything read so far. Confidence **85** on the chain and the
defaults, still **0** on the values.

**And PPSSPP cannot arbitrate whether it would even reach generated
coordinates.** The two backends disagree: `GPU/Software/TransformUnit.cpp`
applies neither scale nor offset in the `GE_TEXMAP_ENVIRONMENT_MAP` case, while
`GPU/Common/VertexShaderGenerator.cpp` multiplies the generated pair by
`u_uvscaleoffset.xy` - scale but not offset. The cross-backend agreement that
lifted the uvgen-2 equation's confidence above is **not available here**, so no
confidence is claimed either way. Deciding it needs hardware or a targeted
experiment, not more source reading.

## Applied names

Per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md): below 70
gains a `_q` suffix, below 50 is not renamed at all. Mirrored in
[`names.tsv`](names.tsv). All of these were read from the GE command byte in the
callee's own body, not inferred from call sites - the distinction that a
previous pass on this subsystem got wrong.

| Address | Kind | Name | Conf |
| --- | --- | --- | --- |
| `0x08811b58` | function | `Gu_SetState` | 92 |
| `0x08811814` | function | `Gu_AlphaFunc` | 92 |
| `0x088126ac` | function | `Gu_Material` | 90 |
| `0x088112c8` | function | `Gu_Color` | 88 |
| `0x08811508` | function | `Gu_TexMapMode` | 88 |
| `0x08811898` | function | `Gu_PixelMask` | 88 |
| `0x08811558` | function | `Gu_TexProjMapMode` | 85 |
| `0x08810a88` | function | `Gu_SpecularCoef` | 82 |
| `0x08928d4c` | function | `Texture_BuildBindList` | 85 |
| `0x0890d904` | function | `Mesh_BeginTransparentPass` | 85 |
| `0x0890dc20` | function | `Mesh_EndTransparentPass` | 82 |
| `0x0887a268` | function | `SceneLight_CallMaterialList` | 78 |
| `0x0887a230` | function | `SceneLight_CallLightingList` | 78 |
| `0x08879b14` | function | `SceneLight_Rebuild` | 75 |
| `0x0887a4f0` | function | `SceneLight_BuildLightingList_q` | 68 |
| `0x08912358` | function | `Vex_UpdateLightLists_q` | 65 |
| `0x0887a018` | function | `SceneLight_InitLists_q` | 65 |
| `0x08abf500` | data | `g_envmap_light_basis_angle_x` | 90 |
| `0x08abf504` | data | `g_envmap_light_basis_angle_y` | 90 |
| `0x08abf510` | data | `g_envmap_light_basis` | 88 |
| `0x08811874` | function | `Gu_DepthMask` | 90 |
| `0x08811850` | function | `Gu_DepthFunc` | 90 |

`Gu_SetState` carries 92 because its body *is* the `sceGuEnable` index table - a
literal switch from state index to GE enable command - which makes every other
`Gu_Enable`/`Gu_Disable` reading on this page and on
[exhaust.md](exhaust.md) a read rather than an SDK recollection.

The three `_q` names are the three whose *bodies* are decoded but whose role is
partly inferred: `SceneLight_BuildLightingList_q`'s list contents are fully read
while its return value's meaning (an accumulated light colour) is not,
`Vex_UpdateLightLists_q` writes both light lists correctly but its trigger was
never read, and `SceneLight_InitLists_q` only writes bare `RET`s and a flag.

**Deliberately not named:** `FUN_0881129c` (a two-argument colour setter used by
`FUN_0890d3ac`, body unread), `FUN_08927550`, `FUN_08928174`, `FUN_08944544`,
and `FUN_08a6bfc0`/`FUN_08a6bd6c` - the last two return their own address and are
RTTI type tokens rather than real functions.

## What is still not recovered

- ~~**Whether `GU_BLEND` is enabled when the additive branch runs.**~~ -
  **closed 2026-08-08 at confidence 90: it is enabled.** The guess about where
  to look was wrong in an instructive way, so both halves are recorded in the
  new section below. `Mesh_SetBatchDrawState`'s two `Gu_CallList` replays are
  **not** the home of blend state - they hold light directions - and the enable
  lives in the transparent pass's own prologue, `Mesh_BeginTransparentPass`
  (`0x0890d904`). The additive branch really composites `dst + src`.
- **Whether the loader rewrites batch-header `+3` between disc and the draw
  path.** Not searched for. The stride-arithmetic match earlier on this page
  supports the *offsets* being read as-authored; it does not by itself rule
  out an in-place fixup of this specific byte. This is the caveat behind the
  confidence-75 figure on "every sampled batch reads additive" above.
- **The PS2 equivalent draw path.** This page is PSP-only (`BOOT.BIN`);
  `Batch::header_flags` is populated identically for PS2 batches (the header
  layout is shared - see that field's own doc comment) but no PS2 function has
  been read to confirm the same bit means the same thing there.
- ~~**Whether `shipboost.vex`'s model has `model+0x1a8` set.**~~ - **closed
  2026-08-09 at confidence 88: it is set**, so the plume's light lists come from
  `Vex_LoadModel`'s fixed basis and not from the view matrix. `ExhaustFlare_Init`
  writes the boost model's `+0x8` parent link to the `FUN_08a6bfc0`-class
  ancestor it just found, immediately before calling `Vex_LoadModel`, so the
  ancestor walk matches on its first step. Evidence in the env-map section
  above. **The other half of this item stays open**: whether
  `SceneLight_Rebuild` writes an accumulated light colour or a hard
  `0xffffffff` into the `+0xd0` material list was not revisited, and
  `FUN_08a6bfc0`'s class is still identified only by the team-name field
  `ExhaustFlare_Init` reads off it, not by a `.vex` class name.
- **The `Vex_LoadModel` list-B divergence.** *Fact, instruction level* at
  `0x089134cc`-`0x08913538`: the second triple is emitted as `0x63`/`0x64`/`0x65`
  + `0x5f000001` - light **0** again - rather than `0x66`/`0x67`/`0x68` +
  `0x60000001`, with registers `a2`/`a3`/`t0` reused from the first triple.
  `Vex_UpdateLightLists_q`'s twin emits the correct light-1 commands.
  *Interpretation*: a copy-paste/register-reuse bug, so on that path light 1 is
  never set and light 0 ends up holding `dir1`. **Confidence 95 on the emitted
  words, 70 on "bug" as the reading** - do not collapse those two numbers.
  Still open **for other models**; ruled out for the plume, whose 32 batches all
  measure `pass_mask = 0x1232` and so all replay list A - see the env-map
  section above.
- **Whether `*batch & 0x8000` has any visible effect.** With lighting off the
  A/B difference is only the `LIGHTTYPE` computation field, which PPSSPP's model
  does not feed into env-map UV generation. Real-hardware behaviour unknown.
  **This is a gap, not a no-op** - do not write it up as one.
- **None of the 2026-08-08 GE read was verified live.** It is all static
  decompilation. The one half that *was* measured against real data is the
  vertex-type read, and that is a disc read rather than an emulator one.
- **Whether `mesh_render::TRANSPARENT_BLEND` should change project-wide.**
  Already self-documented as unconfirmed in
  `crates/render/src/mesh_render.rs`; this page's measurement is evidence
  against its `SrcAlpha`/`OneMinusSrcAlpha` lerp, in favour of the `GU_FIX`
  additive equation, but changing it touches every transparent batch on every
  track and ship and needs its own verification pass.
