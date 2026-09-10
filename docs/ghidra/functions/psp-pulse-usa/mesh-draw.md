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
[`Batch::is_culled`](../../../../crates/vex/src/vex.rs) reading `*batch &
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

`oag_vex::vex::Batch::is_additive_blend()` decodes this bit; see that
function's own doc comment for the exact bit and confidence.

## `param_2` is the on-disk batch header, not a `Material`

Both callers pass a pointer that is walked exactly like
`oag_vex::vex::mesh_batches`' own loop over the on-disk batch list:

- `FUN_0890db54` iterates `param_3`, advancing it by `param_3[6] + 0x40` or
  `+0x80` (`+0x40` byte offset in the alternate-header case) depending on
  `*(byte *)(param_3+3) & 0x40`. That is `u16_at(header, +0x0c) + header_size`
  - **byte-identical stride arithmetic** to `oag_vex::vex::mesh_batches`'
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
versus the packed on-disk layout `oag_vex::vex` parses), so the loader
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

## Live: uvgen 2 is real, and it is bracketed around the transparent pass

2026-08-10, measured on PPSSPP under Xvfb (`docs/reverse-engineering/ppsspp-debugger.md`'s
preferred setup) with a breakpoint armed **before the race loaded**, so
display-list compilation was in scope.

**`Mesh_BeginTransparentPass` (`0x0890d904`) does run** - one hit, during the
race load, immediately after the front end committed to TIME TRIAL. So the only
site that programs `Gu_TexMapMode(2, 0, 1)` is live code, not a path the game
never takes. That much is settled, and it is what `oag_render::texgen`
reproduces.

**It is scoped, not global.** Its caller `FUN_0890d508` is a three-line
bracket:

```c
Mesh_BeginTransparentPass();   // Gu_TexMapMode(2, 0, 1) - environment map on
FUN_0890db54(mesh, ...);       // the transparent draws
FUN_0890dc20(...);             // Gu_TexMapMode(0, 0, 1) - back to authored coords
```

which identifies `FUN_0890dc20` - previously known only as "a tiny function
that sets mode 0" - as the pass's **restore**.

**Why a race-time breakpoint sees none of this.** Sampling `Gu_TexMapMode`
during a moving race catches only mode `0` (from `0x08910e2c`) and mode `1`
(`GU_TEXTURE_MATRIX`, from `0x08910dc0`), roughly 3:1, and **never mode 2** -
because the mode-2 call happens once, at compilation, and is recorded into a
display list rather than executed per frame. A stationary craft at the
countdown produces no `Gu_TexMapMode` calls at all, which is a false negative
worth naming: an earlier pass concluded from exactly that sample that the
engine never programs the mode during a race.

**What is still not settled is which list the plume's batches land in.**
`Mesh_CompileDisplayLists` calls the bracket **twice** (`0x0890fcd4`,
`0x0890fd54`) *and* `Mesh_CompileGeometryPass` twice (`0x0890feb8`,
`0x0890ff38`), each preceded by its own `Gu_Start(GU_CALL, ...)`. So one model
yields several independently-recorded lists, only some of them inside the
uvgen-2 bracket. The plume's batches are on the geometry-pass side per this
page's own earlier correction, and a freshly started list does not inherit the
bracket's state - so the mode live when the plume's list is *replayed* is the
open question, unchanged. **Since settled: mode 0 - see "The plume is replayed
under `TEXMAPMODE` 0" below.**

**Net effect on the implementation:** `oag_render::texgen` moves from
"empirically better, with its mechanism attributed through a pass the plume
does not take" to "the mechanism exists, is live, and is bracketed" - which is
a real upgrade in status and still not a proof that the plume is inside it.

### The live shadow reads mode 0 - and why that is weaker evidence than it looks

`Gu_TexMapMode` keeps a shadow: it stores `mode & 3` into **`ctx+0xf4`**, where
`ctx` is the sceGu context at **`0x08adc3ac`**. That address is the struct's
**base, not a pointer to it** - a first attempt dereferenced it and got zero.
**Corrected 2026-08-10:** `+0x08` is the master list's *start* (read live:
`0x48bacc40` at two independent freezes - a "current write pointer" would not
sit exactly at the start twice), and **`+0x0c` is the current write pointer**
(read live: `0x48bad3a8`, mid-frame). Both are uncached mirrors of main RAM
(`0x48bacc40 & 0x1fffffff = 0x08bacc40`), not VRAM as an earlier version of
this paragraph said.

The shadow is readable while the game free-runs, so it costs nothing and does
not slow the emulator - the failure mode that defeated every breakpoint-based
attempt. Sampled 160 times across a race with the boost armed by hand:

| | samples |
| --- | ---: |
| mode 0, `GU_TEXTURE_COORDS` | **160** |
| mode 1 / mode 2 | 0 |

including all 160 taken while `boost_timer > 0.2`, i.e. with the plume
revealed.

**This does not show that mode 2 is unused, and it must not be cited that
way.** Mode 2 is set inside `FUN_0890d508`'s bracket and restored a few draws
later, so its window is microseconds per frame while the sampler polls over
seconds; catching it would be luck. What the sample does establish is the
**steady state** - what is live outside that bracket - and that is
unambiguously authored coordinates.

So the two live results are consistent and neither is decisive for the plume:
`Mesh_BeginTransparentPass` genuinely runs (breakpoint, one hit, at load), and
the steady-state mode is 0 (polling, 160 of 160). Which of the two the plume's
own compiled list is replayed under still needs the list's bytes read
directly - scan it for a `0xc0`-prefixed word - rather than any form of
sampling. That is the one method left that has no timing exposure, and it is
where the next pass should start.

### A harness note worth keeping

Crossing a speed pad from a teleported craft is unreliable - breakpoint
stepping starves the approach, an unsteered run walks into a wall, and even a
placement `0.2` units off pad 0's own axis at 120 units/s never reached it.
**It is also unnecessary.** `ExhaustFlare_OnSpeedupPad` does exactly one thing
to the visual, so the boost can be armed by hand:

```text
craft -> +0x1c4 -> +0x78 = the Engine Flare        (psp-trace's --flare chain)
write 0.8f at flare+0xb8                            = the boost timer
```

Verified live: the value reads back, and `Exhaust_UpdateEngineSound` decays it
exactly as the recovered code says. That reveals `<Team>boost.vex` with no
driving at all.

## Which `TEXMAPMODE` the plume gets was **not** settled, and this page overstated it

**Since settled: mode 0, `GU_TEXTURE_COORDS` - see the next section.** This
section stands as the record of why every earlier method fell short.

2026-08-10. `Gu_TexMapMode` (`0x08811508`) has **13** callers, and reading the
arguments at four of them shows the engine uses at least three different modes:

| Site | In | Call | Meaning |
| --- | --- | --- | --- |
| `0x0890d948` | `Mesh_BeginTransparentPass` (`0x0890d904`) | `(2, 0, 1)` | environment map, LS0 = light 0, LS1 = light 1 |
| `0x0890e584` | `FUN_0890e304` | `(1, 0, 0)` | `GU_TEXTURE_MATRIX` |
| `0x0890e76c` | `FUN_0890e304` | `(0, 0, 0)` | `GU_TEXTURE_COORDS` - the authored UVs |
| `0x0890dc30` | `FUN_0890dc20` | `(0, 0, 1)` | authored UVs |

**The uvgen-2 finding this page carries was attributed through
`Mesh_BeginTransparentPass`, and the plume does not take that path** - this
page already corrected that scope once, for the alpha test, and the same
correction applies here and was not made. Nor does the plume's own state
builder set the mode: neither `Gfx_BuildBatchStateList` nor
`Mesh_CompileGeometryPass` emits `0xc0`.

And the plume's draws are **recorded into a display list at load** by
`Mesh_CompileDisplayLists` (`0x0890fe...`, two call sites, each preceded by
`Gu_Start(GU_CALL, ...)`), so the mode that applies is whatever is live when
that list is *replayed* - ambient state, not something the list carries.

**So "the transparent pass generates its texture coordinates" is true of the
pass it was read from and unproven for the plume.** What keeps the
implementation defensible meanwhile is empirical rather than structural:
`oag_render::texgen` measurably improved the plume when it landed (extent
+12 %, orange -9 %, `b - r` closest to the original of three builds), which is
evidence that *something* generates them, not proof of which mode.

Settling it needs the replay site traced - which of the 13 setters last ran
before the plume's list is called - or a live `0xc0` read at a breakpoint on
the plume's draw. **Until then the plume's two short batches remain the open
question they were**: 9 and 10 vertices whose authored UVs decode to a single
point, harmless under generated coordinates and a flat one-texel sample under
authored ones.

## The plume is replayed under `TEXMAPMODE` 0 - settled, live

**2026-08-10, confidence 92.** Runtime read on PPSSPP (remote debugger,
`scripts/ppsspp_debugger.py` harness), Talon's Junction TIME TRIAL / VENOM,
boost held up by rewriting `0.8f` at `flare+0xb8`. Single binary, so the
rubric's runtime-trace ceiling of 94 applies; the two points under it are for
one title, one track, one race mode.

**The boost plume's batches draw with `TEXMAPMODE` mode 0,
`GU_TEXTURE_COORDS` - the authored UVs.** Not mode 2. The environment-map
bracket is real and runs every frame (73 `PRIM`s per frame execute under mode
2, measured below), but the plume is not inside it, exactly as the
geometry-pass structural reading predicted.

### The chain, walked live

```text
craft 0x09a04170 -> +0x1c4 -> 0x09a031b0 (entity)
                 -> +0x78  -> 0x09a2af50 (Engine Flare; +0xc0 owner check OK)
                 -> +0x160 -> 0x09a2dbd0 (the <Team>boost.vex model)
model+0x20 -> 0x09a2f230   mesh 1 (header 0x09a2e150, u16 flags 0x1232,
                                   mesh+0x68 = 0 list-A, +0x6a = 2 list-B)
sibling (signature scan) -> 0x09a2f460   mesh 2 (same shape)
```

Both meshes have `mesh+0x158 = 0` and their compiled geometry list at
**`mesh+0x15c`** - the `list = 1` slot of the `mesh + list*4 + 0x158` table
earlier on this page, consistent with the plume being list-B-only geometry.

### Every byte of the replayed chain, read: no `0xc0` anywhere

Mesh 1's `+0x15c` list is 21 words, and its entire recursive CALL closure was
decoded word by word (mesh 2's is the same shape):

```text
BASE/CALL 0x0416b900    interned per-batch state list (EDRAM, 28 words) - no 0xc0
BASE/CALL 0x09a2df80    per-material texture transform (5 words)        - no 0xc0
BASE/CALL 0x09a2f6d0    texture bind list (16 words: TBP/TBW/TSIZE/
                        CLUT/TFLUSH)                                    - no 0xc0
VTYPE 0x1200013d, VADDR, PRIM (strip)      x2, one per batch
RET
```

So the list carries no mode of its own, and the mode that applies is ambient
state at replay - confirming the structural reading, and moving the question
to what that ambient state is.

Two side findings from the same read:

- **The per-material texture transform list is compiled lazily, on first
  draw.** Before the boost was armed, `0x09a2df80` was unwritten heap
  (`0xfeadfead` canaries); after one armed frame it reads
  `TexScale(1.0, 1.0)`, `TexOffset(u, 0)`, `RET` - with `u` **animating**
  (read `0.956` and `0.504` at two freezes, per mesh). This is the
  per-material `Gu_TexScale`/`Gu_TexOffset` transform HANDOVER.md carries as
  real-and-unported, and it means the plume's authored UVs are sampled
  *through a moving u-offset*, not statically.
- **Nothing CPU-calls the compiled geometry lists per frame.** A breakpoint
  on `Gu_CallList` (`0x08810598`) conditioned on `a0 ==` either list never
  fires, and a full-RAM scan finds no `CALL` word targeting them anywhere.
  Instead the deferred draw path re-records the same six-command sequence
  into per-item EDRAM blocks each frame (found at `0x041b88xx`, `0x041c3dxx`
  and mirrors - byte-identical to the compiled list, ending in `RET`), and
  the master list at the sceGu context's list start (`0x08bacc40`) is a flat
  run of `BASE`/`CALL` pairs into those blocks in sorted order, holding no
  state words itself.

### The decisive read: the recorded frame, walked in GE order

With the CPU frozen mid-frame, a script walked the recorded stream exactly as
the GE would - from the master list's start to its current write pointer,
following `BASE`/`CALL`/`JUMP`/`RET`, tracking every `0xc0` word - and stopped
at each plume `VADDR` (`0x01a2e240`, `0x01a2e680`, `0x01a2ed60`; the fourth
batch sits between the same words in mesh 2's block). The mode live at the
plume's own draw commands, in the order the GE consumes them:

| Sample | plume batches reached | mode at each | PRIMs under mode 0 | under mode 2 |
| ---: | ---: | --- | ---: | ---: |
| 1-6 | 3 of 3 observed, every sample | **0, all** | 596-660 | 73 |

The 73 mode-2 `PRIM`s per frame are the positive control: the walker's mode
tracking does see the `Mesh_BeginTransparentPass` brackets (`0xc0000002` ...
draws ... `0xc0000000`, visible as paired words in the trail), and the plume
is never among them - it draws downstream of a bracket's *restore*, under
mode 0, in six of six independently frozen frames.

**What this kills:** `oag_render::texgen`'s environment-map coordinates are
not what the original applies to the plume. The original samples the plume's
authored UVs - two of its four batches decode to a single UV point - through
the animated per-material `TexOffset` u-scroll above. The empirical
improvement texgen bought (recorded in HANDOVER.md) was real but for the
wrong reason; the recovered mechanism to port is authored UVs + the animated
transform.

The scroll itself was recovered in the same session, down to its authored
keyframes in the `.vex` file: `u` ramps `2/256 -> 253/256` over key times
1..90 in 60 Hz frames, `v = 0`, scale constant `(1.0, 1.0)`, evaluated at
the flare's own life timer (`flare+0x88`, reset at each reveal) - so it
plays **once per boost**, bright to dark, ending exactly as the plume's
1.5 s life expires. Mechanism, clock, evaluator/updater names and the
on-disc block layout: [texture-animation.md](texture-animation.md), "The
values gap is closed"; file layout: `docs/formats/vex.md`, "The
texture-transform keyframe block".

Scripts for the whole read are session scratch
(`plume-lists*.py`, `ge-walk*.py`); the reusable pieces are the harness
recipe already on this page and `scripts/ppsspp_debugger.py`.

## `Gfx_BuildBatchStateList` decoded, and the boost plume's blend with it

`0x0891f890`, confidence **90**. This is the state builder on the path the
boost plume actually takes (`Mesh_CompileGeometryPass`, `0x0890d0cc`), as
opposed to `Mesh_SetBatchDrawState` on `FUN_089307b4`'s. Read 2026-08-10 while
chasing the plume's fidelity; it settles a question `exhaust.md` had carried
open for four passes.

`param_2` is the batch's `pass_mask`, `param_3` a material flags word. Traced
against the plume's own measured `pass_mask = 0x1232`:

```text
Gu_PixelMask(0)                                  every channel writable - the glow mask
Gu_TexWrap(REPEAT, REPEAT)                       (& 4) and (& 8) both clear
Gu_Disable(CULL_FACE)                            (& 0x20) set
                                                 (& 0x700) = 0x200, so the blended group:
Gu_DepthFunc(6);  Gu_DepthMask(1)
                                                 (& 0x100) clear, (& 0x200) set:
Gu_Enable(BLEND)
Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX 0xffffff)
Gu_ColorFunc(GU_NOTEQUAL, 0, 0xffffff);  Gu_Enable(COLOR_TEST)
Gu_Enable(ALPHA_TEST);  Gu_AlphaFunc(GU_GREATER, 0, 0xff)
                                                 (& 0xc0) clear, (& 0x700) set:
Gu_StencilOp(KEEP, KEEP, KEEP);  Gu_Disable(STENCIL_TEST)
```

**The blend is `src.rgb * src.a + dst`** - the `SrcAlpha`/`One` that
`oag_render::exhaust::BLEND` already programs and that `exhaust.md` recorded as
an unexplained empirical fit. It is not a fit; `pass_mask & 0x200` selects it,
and the format layer's `BlendClass` has described that bit as "additive and
source-alpha weighted" all along. The competing `GU_FIX`/`GU_FIX` reading is
`Mesh_SetBatchDrawState`'s, and **the plume never reaches that function**.

Three smaller consequences:

- **The plume disables the stencil test and stamps no reference**, so what it
  contributes to the glow mask is its own fragment alpha through the blend,
  not a stamped constant. That is what `mesh_render::GlowMask::Written` gives
  it.
- `Gu_ColorFunc(GU_NOTEQUAL, 0, 0xffffff)` discards fragments whose RGB is
  exactly black. Under an additive blend that is **algebraically inert** for
  any content - a fill-rate optimisation on a part where fill is the scarce
  resource. Recovered-but-inert state, of the kind
  [`methodology.md`](../../../reverse-engineering/methodology.md) warns against
  porting as a "fix".
- `FUN_088117cc` is `sceGuColorFunc`, read from its body: GE commands `0xd8`
  `COLORTEST`, `0xd9` `COLORREF`, `0xda` `COLORTESTMASK`.

## The texture filter is one global setting, and the mip chain is bounded by the asset

Read 2026-08-10, chasing a reported difference in the exhaust's texture detail
([exhaust.md](exhaust.md), "the ribbon measured against a gameplay capture").
Both halves came back as **negatives against the reimplementation**, which is
why they are recorded: each one is a search another pass would otherwise run
again.

### `Gu_TexFilter` (`0x088113f0`)

| | |
| --- | --- |
| **Confidence** | 88 |

```c
void Gu_TexFilter(u32 min, u32 mag);   // emits mag << 8 | min | 0xc6000000
```

The body writes the GE command word directly, the same evidence class as the
other `Gu_*` helpers this directory documents at 85-90, and `0xc6` is
`TEXFILTER`. **It is the only `0xc6` emitter in the binary** - a `lui`
scan over all 636,020 instructions returns exactly one match, at `0x08811404`.

**All four call sites pass the same pair, `(min = 7, mag = 1)`:**

| Call site | In |
| --- | --- |
| `0x0891e4d8` | `Gfx_FlushRenderManager` |
| `0x0891f618` | `Gfx_Init` (`FUN_0891f320`) |
| `0x0888b63c` | unnamed |
| `0x088ef068` | unnamed |

`min = 7` is `GU_LINEAR_MIPMAP_LINEAR` and `mag = 1` is `GU_LINEAR`, so the
whole program draws **trilinear-minified, bilinear-magnified** and nothing ever
selects a nearest filter. There is no per-material, per-pass or per-texture
filter override anywhere, because there is nowhere for one to be emitted.

**The consequence is a negative.** "The original keeps hard texel edges the
reimplementation blurs away" is not available as an explanation for any
difference in any textured surface - `oag_render`'s samplers already use
`Linear`/`Linear`, which is the same state.

### How many mip levels the GE actually gets

`Texture_BuildBindList` (`0x08928d4c`, above) takes the level count from the
**texture object's own `+0x05` byte** and uses it twice:

```c
u32 levels = texture[0x05];
*out++ = 0xc2000000 | (levels - 1) << 16 | swizzle;   // TEXMODE, maxmips
for (u32 i = 0; i < levels; i++)                      // TEXADDR/TEXBUFWIDTH/TEXSIZE
    emit_level(i, width >> i, height >> i, ...);      // per level
```

That `+0x05` is the same field [`psp-texture.md`](../../../formats/psp-texture.md)
records at `.mip` header `+0x06` and `oag_texture::texture` exposes as
`Texture::mip_count`. So **the asset decides the chain depth, and the GE never
samples below the level the artists shipped.** Measured on the two textures
this mattered for: `pulse_boost2_ADD` (embedded in `shipboost.vex`) declares
**2** levels, `Engine_noise.mip` declares **4**.

`Gu_TexLevelMode` (`0x088114b0`, confidence 80) is the `0xc8` `TEXLEVEL`
emitter - `(bias & 0xff) << 16 | mode | 0xc8000000`, the bias scaled by
`DAT_08ab0524` and clamped to `[-0x80, 0x7f]`. `Gfx_FlushRenderManager` calls
it `(mode = 2, bias = 1.0)`; `Texture_BuildBindList` emits mode `2` with a
computed bias, or mode `0` with none, on the texture's `+0x06 & 0x18` bits.
Either way level selection is automatic, as it is under wgpu.

**This one is a real divergence and it is measured inert.** `oag_render`
synthesises a **full** box-filtered chain down to 1x1 (`mesh_render::mip_chain`,
7 levels for a 64x16 texture) and the exhaust path uploads **level 0 only** with
no `mipmap_filter` at all - neither matches the asset's own count. Both were
tested at the reference pose on 2026-08-10:

| Experiment | Result |
| --- | --- |
| cap `mip_chain` at 2 levels, matching `pulse_boost2_ADD` | **3 pixels** differ by more than 4/255 across the whole frame |
| give the exhaust textures a full chain and `MipmapFilterMode::Linear` | mean absolute difference **0.02/255**, max 20 |

So neither surface is minified enough at race distance for the chain depth to
reach the picture, and **neither is a candidate for any visible difference**.
Worth fixing on its own account - a synthesised level is not the authored one,
and `Texture::mip_count` is already parsed and currently ignored by every
consumer - but not worth attributing a symptom to.

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

Read directly off `pulse-psp-usa.chd` through `oag_vex::vex::mesh_batches`
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
because `oag_vex::vex::textures` read its rows back to back where the
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
two-sided**. `& 0x1000` was undecoded when this paragraph was written
(2026-08-09); it no longer is - see "The layer derivation, and what it is
worth" further down this page, decoded 2026-08-18. It is the plume's draw-layer
bit, set here as on every batch whose mesh-level key already resolves to
`0x45000000`. Read off
`data/images/pulse-psp-usa.chd` through `oag_vex::vex::mesh_batches`. The
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

`oag_vex::vex::Batch::is_culled` **was** `pass_mask & 0x0020 != 0`, with a
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
| `0x088113f0` | function | `Gu_TexFilter` | 88 |
| `0x088117cc` | function | `Gu_ColorFunc` | 90 |
| `0x0891f890` | function | `Gfx_BuildBatchStateList` | 90 |
| `0x088114b0` | function | `Gu_TexLevelMode` | 80 |
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

## The plume is not drawn by `FUN_089307b4` at all, and the batch-set path is list-A-only

**2026-08-09, confidence 90 on the mechanism, 92 on the two measurements.**
Opened to settle a narrow question - *which group mask (`param_3 & 1/2/4/8`)
does the boost plume's draw item carry, and does every plume batch reach the
GE?* - and it turned out the question was aimed at the wrong function. The
answer to the question as asked is that **the plume has no draw item in that
structure at all**, and the answer to what the question was really after is
that **all four plume batches are drawn and none is dropped**.

Everything below is static decompilation of `psp-pulse-usa/BOOT.BIN` plus one
disc measurement; nothing was verified live.

### The two independent draw paths

`Vex_LoadModel` (`0x08912b80`) ends by calling `Mesh_BuildModelDrawData`
(`0x08912018`) when its flag argument has neither `& 1` nor `& 4`. That
function starts **both** paths for every model:

1. `Mesh_BuildLayerBatchSets` (`0x0892eda4`), given `model+0x98` - the layer key
   `Vex_LoadModel` was called with. This is the **batch-set** path, whose
   dispatcher is `FUN_089307b4`.
2. `Mesh_CompileDisplayLists` (`0x0890fad8`) **twice per mesh**, once with
   `list = 0` and once with `list = 1`. This is the **per-mesh compiled-list**
   path.

The two are not alternatives and not a fallback pair; both run.

### The batch-set path counts only list A, so it cannot see the plume

`Mesh_CountBatchesPerList` (`0x0890e7a8`), called from `Mesh_InitFromPayload`
(`0x0890e998`) at load, writes two separate counts onto the mesh object:

- `mesh+0x68` - batches reachable from the **list-A** head (`mesh_header+0x04`,
  relocated), counted while `pass_mask & 1`.
- `mesh+0x6a` - batches reachable from the **list-B** head (`mesh_header+0x08`),
  counted while `pass_mask & 2`.

`Mesh_CompileBatchSet` (`0x0892f35c`), which fills the four arrays
`FUN_089307b4` walks, reads **`mesh+0x68` only** - both in its sizing pre-pass
and as the bound of its classification loop, which starts at the list-A head
and strides by `payload_size + header_size` with no terminator test. `mesh+0x6a`
is referenced once, and only to decide whether to call `FUN_08944e50`.

**A mesh with zero list-A batches therefore contributes zero draw items to
every batch set, whatever its layer key.** Measured off
`data/images/pulse-psp-usa.chd` (`oag-wad cat` on
`Data\Ships\Assegai\shipboost.vex`, parsed against the header layout in
[`../../../formats/vex.md`](../../../formats/vex.md)): both plume meshes,
`bflare1Shape` and `bflare2Shape`, have `listA_off = 0x0` and
`listB_off = 0xb0`. Relocation makes the list-A head point at the **mesh header
itself**, whose first `u16` is `mesh_flags = 0x1232` - `& 1` clear - so the
list-A walk terminates before its first iteration and `mesh+0x68 = 0`. Both
meshes carry two list-B batches each, all four `pass_mask = 0x1232`.

So the boost plume never enters `FUN_089307b4`. Neither does any other
list-B-only geometry, which is a large share of a track (see
[`../../../formats/vex.md`](../../../formats/vex.md)'s "List B is real, distinct
geometry" section).

### What actually draws the plume

`Mesh_CompileDisplayLists` (`0x0890fad8`) selects the list from its second
argument - `list = 1` takes `mesh_header+0x08` with terminator `2`, `list = 0`
takes `mesh_header+0x04` with terminator `1` - and then compiles up to five
display lists, each behind its own gate on the **mesh's** flag word:

| Cached at | Built by | Gate |
| --- | --- | --- |
| `mesh + list*4 + 0x144` | `FUN_0890cf84` | `mesh_flags & 0x800` |
| `mesh + list*4 + 0x14c` | `FUN_0890d508` (which calls `FUN_0890db54`) | `mesh_flags & 0x2000` |
| `mesh + 0x154` | `Mesh_SetBatchLighting` (`0x0890d3ac`), with `-1` | always |
| `mesh + list*4 + 0x158` | `Mesh_CompileGeometryPass` (`0x0890d0cc`) | `mesh+0x7a == 0` or `list == 1` |
| `mesh + list*4 + 0x160` | `FUN_0890d678` | `mesh_header+0x0c & 4`, or the reflection global |

The plume's `mesh_flags = 0x1232` has `& 0x800` and `& 0x2000` both **clear**,
so only the `+0x154` and `+0x158` lists are ever built for it.

`Mesh_CompileGeometryPass` (`0x0890d0cc`) is the one that emits its geometry:

```c
while (*batch & terminator) {
    if ((*batch & 0x800) == 0) {
        Gu_CallList(*(int *)(batch + 0x2c) + 0x10);   // per-batch GE state
        // stencil when *batch & 0xc0; FUN_0892733c when the material has 0x10
        Gu_CallList(texture + 0xc0);                  // or a reflection target
        Mesh_EmitDrawArray(batch);
    }
    batch += ((batch[3] & 0x40) ? 0x80 : 0x40) + payload_size;
}
```

It walks the **whole** list and skips only batches with `pass_mask & 0x800`
set. All four plume batches have it clear. **Every plume batch is submitted;
nothing is dropped.** That closes the last of the six candidate multipliers
listed on [`exhaust.md`](exhaust.md) as negative.

### The correction this forces: `Mesh_SetBatchDrawState` is not on the plume's path

`Mesh_SetBatchDrawState` (`0x0890d994`) has exactly two callers, and **both are
gated on the `0x2000` bit** - `FUN_0890db54` is reached only through
`FUN_0890d508`, which `Mesh_CompileDisplayLists` gates on `mesh_flags & 0x2000`,
and `FUN_089307b4`'s `& 2` group is the array `Mesh_CompileBatchSet` fills from
`pass_mask & 0x2000`. The plume has `0x2000` clear on both the mesh word and
all four batch words.

**So the "Every per-batch state setter, read to its command byte" table earlier
on this page is describing a function the plume never reaches.** The blend,
depth-mask, cull and pixel-mask values in it are correctly decoded and remain
correct *for `0x2000` batches*; they are not evidence about the boost plume.
The header note that "where this page and other pages give a number next to a
specific claim, that number governs" now has to be read the other way round for
that table: its claim scope is narrower than it says.

Where the plume's per-batch GE state really comes from is
`Gfx_AcquireBatchStateList` (`0x0891df48`), called once per batch at load from
`Mesh_InitBatch` (`0x0890e8b4`) as

```c
batch->0x2c = Gfx_AcquireBatchStateList(g_display, pass_mask, header_byte3,
                                        mesh->0xcc, mesh->0xd0, batch->0x29);
```

which is an **interned, refcounted cache** of at most `0x96` entries at stride
`0x1c`, keyed on exactly that tuple, with the compiled list at `entry+0x10` and
a dirty flag at `entry+0x18`. `Mesh_CompileGeometryPass` replays `entry+0x10`.
The function that fills `entry+0x10` was not identified in this pass; the
candidates are `FUN_0891ee98`, `FUN_0891f320` and `FUN_0891f890`, all in the
same module, and **`FUN_0891f320` is almost certainly it** - the texfunc read
already recorded on [`exhaust.md`](exhaust.md) at confidence 92 comes from
`0x0891f62c`, which lies inside it. That reading is therefore on the plume's
real path and stands. **This is the single highest-value unread function left
in the mesh path**: it is where a per-fragment weight on a `0x1232` batch would
have to live, and it has never been decompiled.

### `Mesh_SetBatchLighting` (`0x0890d3ac`) is a lighting setter, not a second blend path

It carried a duplicate `Mesh_SetBatchDrawState` name in the live Ghidra
database, which is what made `FUN_089307b4`'s four groups look like they shared
one state setter. Its body sets no blend state at all:

```c
if ((batch->vertex_type & 0x60) == 0 || (batch->vertex_type & 0x1c) != 0) {
    if (ambient == -1) Gu_Disable(10);              // GU_LIGHTING off
    else { Gu_Enable(10); Gu_Disable(0xb..0xe);     // all four light slots off
           Gu_Ambient(ambient);
           FUN_0881129c(1, 0xffffffff); FUN_0881129c(2, 0xffffffff); }
} else {
    SceneLight_CallLightingList(mesh->0x70 + 0xb0);
    FUN_0881129c(1, mesh->0x6c | 0xff000000);
    FUN_0881129c(2, mesh->0x6c | 0xff000000);
    FUN_0881129c(4, 0);
}
Gu_Color(0xffffffff);
```

`batch+0x0a` is the GU vertex type, so `& 0x60` is its normal field and `& 0x1c`
its colour field: **a batch with no normals, or with vertex colours, takes the
first branch.** The plume has both normals and `GU_COLOR_8888` (`0x1c` set), so
it takes the first branch, and `Mesh_CompileDisplayLists` compiles the
`mesh+0x154` list with `ambient = -1` - **`Gu_Disable(10)`, lighting off,
`Gu_Color(0xffffffff)`**. That is a *sixth* negative: no ambient term weights
the plume's fragments either. Confidence 85; the `& 0x60`/`& 0x1c` reading as
`GU_NORMAL`/`GU_COLOR` is from the standard PSP `sceGuDrawArray` vertex-type
packing, not from a decompiled decoder.

`mesh+0x6c` reaching `Gu_Ambient` on the *other* branch is a real per-mesh
colour and is not read here; it cannot touch the plume, which never takes that
branch.

### `Mesh_RegisterBatches` (`0x0890c84c`) is the only walker that visits both lists

Worth recording because it is the clearest statement in the binary of what list
A and list B mean, and because it is where `mesh+0x4c` picks up its low bits:

```c
void Mesh_RegisterBatches(int mesh) {
    header = mesh->0x58;
    for (b = *(header + 4); b && (*b & 1); b = next(b)) Mesh_InitBatch(mesh, b);
    for (b = *(header + 8); b && (*b & 2); b = next(b)) Mesh_InitBatch(mesh, b);
    mesh->0x4c |= *(u16 *)(texture_of(last_material) + 0xb4);
}
```

The two terminators are `pass_mask & 1` for the head at `mesh_header+0x04` and
`pass_mask & 2` for the head at `mesh_header+0x08`, which is exactly the rule
[`../../../formats/vex.md`](../../../formats/vex.md) states and
`oag_vex::vex::mesh_batches` implements. The `|=` only touches the low 16
bits, so it cannot disturb the layer byte the classification tests read out of
the top 12. Confidence 80 - the body is a direct read, the texture field at
`+0xb4` is not.

### `Mesh_BuildBatchDrawCommands` (`0x0892e8f0`) confirms three parser rules

The 0x28-byte draw items `Mesh_CompileBatchSet` builds are turned into GE
commands by `Mesh_BuildBatchDrawCommands`, which writes a five-word list into
`item+0x14`:

| Word | Value | GE command |
| --- | --- | --- |
| `item+0x14` | `batch->0x0a \| 0x12000000` | `0x12` `VTYPE` |
| `item+0x18` | `(vaddr & 0xf000000) >> 8 \| 0x10000000` | `0x10` `BASE` |
| `item+0x1c` | `(vaddr & 0xffffff) \| 0x01000000` | `0x01` `VADDR` |
| `item+0x20` | `prim << 16 \| count \| 0x04000000` | `0x04` `PRIM` |
| `item+0x24` | `0x0b000000` | `0x0b` `RET` |

and selects between the batch's two vertex blocks as

```c
batch->0x28 = (batch->0x06 != 0);
count = batch->0x28 ? batch->0x06 : batch->0x04;
prim  = batch->0x28 ? batch->0x09 : batch->0x08;
vaddr = batch + header_size + (batch->0x28 ? batch->0x0e : 0);
```

Three things this pins, at confidence 88, that
[`../../../formats/vex.md`](../../../formats/vex.md) and
`crates/vex/src/vex.rs` had inferred rather than read:

- **`use_alternate = u16_at(payload, at + 6) != 0`** in `vex.rs` is bit-for-bit
  the original's rule. It was a hypothesis; it is now a read.
- **`+0x0e` is the alternate block's byte offset from the primary**, ending the
  `unk_0x0e` doc comment's "meaning is not established".
- **`+0x28` is that selector cached as a `bool`, rewritten on every draw**, not
  "written at load" as `vex.md` currently says. Worth correcting there.

It also writes `item+0x10` = `FLT_MAX` when there is no alternate block and
`-1.0` when there is - a sort key consumed by `Mesh_CompileBatchSet`'s four
comparator sorts.

### The layer key at `mesh+0x4c`, and how a mesh can leave its model's layer

`Mesh_InitFromPayload` (`0x0890e998`) settles it, at confidence 85:

1. It starts as the model's own key - `Vex_LoadModel`'s third argument, which is
   `0x4d000000` for `<Team>boost.vex` ([`exhaust.md`](exhaust.md)).
2. If it is `0x45000000` (the ordinary scene layer) it is re-derived from the
   first batch's `pass_mask` into `0x45000000`, `0x4a000000` or `0x31000000`.
   `Mesh_BuildLayerBatchSets` builds exactly those three extra sets, plus
   `0x40000000`, only when its own argument is `0x45000000`.
3. **If any material in either list has `(material+3) & 0xfc` non-zero**, the key
   is overwritten with `0x01500000 + (idx-1) * 0x1000000`, `idx` being that
   field `>> 2`, and cached at `mesh+0x7a`. The same index selects an entry in
   the `DAT_08b323c0` table that `Mesh_CompileGeometryPass` and `FUN_089307b4`
   both use to bind a render-target texture with `Gu_TexScale(1.0, -1.0)` - a
   flipped `v`, i.e. **planar reflection surfaces**. Not pursued further here.

Measured: both plume meshes have a single material whose `+0x03` is `0x00`, so
neither is diverted, and `mesh+0x4c` stays `0x4d000000`.

`mesh+0xc8` gets a second, derived key through a small fixed mapping
(`< 0x30000000` adds `0x100000`; `0x66`->`0x68`, `0x53`->`0x55`, `0x61`->`0x63`,
`0x58`->`0x5a`, `0x6b`->`0x6c`, `0x4b`->`0x4d`, otherwise `0x4c000000`). Its
consumer was not traced.

### Live-database name drift found on the way, beyond the three already known

The docs win per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).
`names.tsv` was checked and is clean of all of these - the drift is in the
Ghidra database only, and a fresh import replayed from `names.tsv` will not
reproduce it.

| Address | Name in the live DB | What it is |
| --- | --- | --- |
| `0x0890d3ac` | `Mesh_SetBatchDrawState` (duplicate of `0x0890d994`) | `Mesh_SetBatchLighting`, above |
| `0x0892e8f0` | `Ship_UpdateEngine_q` | `Mesh_BuildBatchDrawCommands`, above |
| `0x0892eda4` | `Ship_UpdateEngine_q` (same name, second function) | `Mesh_BuildLayerBatchSets`, above |
| `0x08811630` | `Gu_Fog_q` | `Gu_TexOffset` - emits `0x4a`/`0x4b`, `TEXOFFSETU`/`TEXOFFSETV`. Already correct in `names.tsv`; **DB-only drift, and it caught this page out once** |

`Ship_UpdateEngine` proper is `0x0884c5c8` (`engine.md`), and is unrelated to
both. The duplicate names are why `get_function_callers` by name returns
contradictory answers in this region; query by address here.

### Correction to this page's own earlier description of `FUN_089307b4`

The paragraph above beginning "`FUN_089307b4` is the mesh draw path's pass-group
dispatcher" is right that `Mesh_SetBatchDrawState` (`0x0890d994`) appears only
in the `& 2` group, and right that `& 1` uses `FUN_0890d3ac`. It is wrong that
`& 4` "sets depth directly off `*batch & 2` without a blend call": the `& 4`
group calls `FUN_0890d3ac` too, with the same four arguments the `& 1` group
uses. The four groups map to `Mesh_CompileBatchSet`'s four arrays as

| `param_3` bit | array / count | membership test |
| --- | --- | --- |
| `1` | `+0x68` / `+0x64` | `pass_mask & 0x800` set |
| `2` | `+0x70` / `+0x6c` | `pass_mask & 0x2000` set |
| `4` | `+0x78` / `+0x74` | `pass_mask & 0x800` clear |
| `8` | `+0x80` / `+0x7c` | `(batch+3) & 4` set - the caustic pass |

`Mesh_CompileBatchSet` bakes groups `1|2|4` into `set+0xac` and group `8` into
`set+0xb0`; `Mesh_DrawBatchSet` (`0x0893021c`) replays `+0xac` unconditionally
and `+0xb0` after `Texture_BindCausticFrame`, falling back to a live
`FUN_089307b4(set, 0, 0xf)` when nothing was baked.

### Open, and named so it is not re-derived

- ~~**`FUN_0891f320`** - the per-batch state-list compiler.~~ - **resolved in
  the next section, and the guess was wrong.** `0x0891f320` is `Gfx_Init`, the
  display-subsystem constructor; the texfunc read at `0x0891f62c` cited on
  [exhaust.md](exhaust.md) therefore comes from the **global GE default**, not
  from a per-batch list. That default is still what a `0x1232` batch inherits,
  since `Gfx_BuildBatchStateList` never touches texfunc - but it is weaker
  evidence than "read at the plume's own call site", and the confidence-92
  figure on that line should be read as covering the global default only. The
  real per-batch compiler is `Gfx_BuildBatchStateList` (`0x0891f890`).
- **Who replays the `mesh+0x158` list per frame.** `Mesh_CompileGeometryPass`
  builds it; the deferred draw callback that issues it, downstream of
  `Gfx_FlushRenderManager`'s sort, was not followed.
- **`FUN_0890d678`** and **`FUN_0890cf84`**'s and **`FUN_0890d508`**'s full
  bodies, and the `DAT_08b323c0` reflection-target table.
- **`mesh+0x6c`**, the per-mesh ambient colour on `Mesh_SetBatchLighting`'s
  second branch. Never written to by anything read here.
- **The planar-reflection subsystem - an unmapped subsystem, not just an unread
  function.** Recorded here so it is findable rather than re-derived. What was
  seen, all of it in this pass and none of it followed up:
  - `Mesh_InitFromPayload` (`0x0890e998`) reads `(material + 3) & 0xfc` across
    both batch lists. When any material has it non-zero, the value `>> 2` is
    cached at `mesh+0x7a` (range 1..8) and the mesh's layer key at `mesh+0x4c`
    is **overwritten** with `0x01500000 + (idx - 1) * 0x1000000`, moving the
    mesh out of its model's own layer.
  - `Mesh_BuildLayerBatchSets` (`0x0892eda4`) builds one batch set per such
    index, `1..8`, but **only when its own layer argument is `0x45000000`** -
    i.e. only for ordinary scene geometry.
  - Both `Mesh_CompileGeometryPass` (`0x0890d0cc`) and `FUN_089307b4`'s `& 4`
    group use the same `>> 2` index into a table at **`DAT_08b323c0`**, whose
    entries carry a mode at `+0xa8`, a matrix or plane at `+0x98`, a stencil
    reference at `+0x94` and a texture at `+0xac`. Mode `>= 1` binds
    `entry+0xac` as a texture with **`Gu_TexScale(1.0, -1.0)`** - a flipped `v`,
    which is what reading back a render target rendered from a mirrored camera
    needs. Mode `0` instead calls `FUN_0891fd98(entry+0x98)`, enables
    `GU_STENCIL_TEST` with `entry+0x94` as the reference, and disables fog.
  - Neither `FUN_0891fd98`/`FUN_0891fdf0` (paired, push/pop shaped) nor the
    `DAT_08b323c0` table's producer was read.
  - **The feature is used by shipped content - measured, so this is not a dead
    branch.** Every `Mesh` node of two circuits scanned off
    `pulse-psp-usa.chd`, `(material+3) & 0xfc` tabulated `>> 2`:

    | Circuit | meshes | materials | non-zero | indices seen |
    | --- | ---: | ---: | ---: | --- |
    | `01_Track` | 594 | 1,364 | **15** | 1, 5, 6, 8 |
    | `16_Track` | 602 | 1,717 | **8** | 1, 2, 6, 7, 8 |

    Rare but real, and spread over six of the eight available slots between the
    two tracks - consistent with a handful of authored reflective or
    render-to-texture surfaces per circuit rather than with stray bits. The
    flagged materials also all carry `(material+3) & 0x03 == 1`, a separate flag
    in the same byte that was not decoded.
  - **What to do next**, in order: locate one flagged mesh's node name and
    texture to see what kind of surface it is; then read `FUN_0891fd98` and the
    `DAT_08b323c0` producer. This project draws none of it today, and a track
    surface that should mirror is currently drawn as an ordinary textured one.

## The plume's real per-batch GE state, and why five multiplier candidates all read negative

**2026-08-09, confidence 90.** Found by following the previous section's open
item. The chain from a `.vex` batch to the GE words it replays is now closed at
instruction level with no inference left in it:

```
Mesh_InitBatch (0x0890e8b4)                 at load, once per batch
  batch->0x2c = Gfx_AcquireBatchStateList(g_display, pass_mask, header_byte3,
                                          mesh->0xcc, mesh->0xd0, batch->0x29)

Gfx_CompileDirtyBatchStateLists (0x0891e054) per frame, over all 0x96 entries
  if (entry->refcount && entry->dirty) {
      Gu_Start(1, entry->0x10, 0xc0);       the 192-byte EDRAM slot Gfx_Init gave it
      Gfx_BuildBatchStateList(g_display, entry->pass_mask, entry->header_byte3,
                              entry->depth_lo, entry->depth_hi, entry->bias);
      ...
      entry->dirty = 0;
  }

Mesh_CompileGeometryPass (0x0890d0cc)        per batch, per draw
  Gu_CallList(*(int *)(batch + 0x2c) + 0x10)
```

`Gfx_BuildBatchStateList` (`0x0891f890`) takes **exactly** the six-tuple
`Gfx_AcquireBatchStateList` interns on, and `Gfx_CompileDirtyBatchStateLists`
records it straight into the slot the draw path replays. It is *the* per-batch
state setter for every batch the engine draws through
`Mesh_CompileGeometryPass` - which, per the previous section, is every list-B
batch including all four of the boost plume's.

### Four `_q` names in this path are wrong, and three of them mattered

Resolved by reading each callee to its GE command byte, the way the
`Mesh_SetBatchDrawState` table earlier on this page was. The `0xdd` =
`STENCILOP` anchor that table established is what fixes the neighbours.

| Address | Live-DB name | Emits | Actually |
| --- | --- | --- | --- |
| `0x08811814` | `Gu_StencilFunc_q` | `0xdb`, `(mask<<16)\|(ref<<8)\|func` | `ATST` - **`Gu_AlphaFunc`** |
| `0x08811850` | `Gu_StencilOp_q` | `0xde` | `ZTEST` - `Gu_DepthFunc` (already known) |
| `0x08811874` | `Gu_BlendFunc_q` | `0xe7` | `ZWRITEDISABLE` - `Gu_DepthMask` (already known) |
| `0x088117cc` | *unnamed* | `0xd8`, `0xd9`, `0xda` | `CTEST`/`CREF`/`CMSK` - **`Gu_ColorFunc`** |

`0x08811814` is the new one and it is the load-bearing one. The GE test-function
enum (`0` never, `1` always, `2` equal, `3` notequal, `4` less, `5` lequal,
`6` greater, `7` gequal) and the `sceGuEnable` state enum (`0` `GU_ALPHA_TEST`,
`3` `GU_STENCIL_TEST`, `4` `GU_BLEND`, `5` `GU_CULL_FACE`, `10` `GU_LIGHTING`,
`11`-`14` the four lights, `0x11` `GU_COLOR_TEST`) are the standard PSP SDK
ones; they are corroborated four times over inside these functions by
co-occurrence - `Gu_Enable(3)` next to `Gu_StencilFunc`/`Gu_StencilOp`,
`Gu_Enable(4)` next to `Gu_BlendFunc`, `Gu_Enable(10)` next to `Gu_Ambient` and
`Gu_Disable(0xb..0xe)`, `Gu_Enable(0x11)` next to `Gu_ColorFunc`.

### What the plume's batches actually program

Evaluated for the measured `pass_mask = 0x1232`, `header_byte3 = 0x01` (both
read off `pulse-psp-usa.chd`, all four batches, previous section):

```c
Gu_PixelMask(0);                              // every channel written
Gu_TexWrap(0, 0);                             // pass_mask & 4, & 8 both clear: repeat/repeat
Gu_Disable(GU_CULL_FACE);                     // pass_mask & 0x20 set: two-sided
                                              // header_byte3 & 0x10 clear -> not the additive branch
                                              // pass_mask & 0x700 == 0x200 -> the transparent branch
Gu_DepthFunc(6);
Gu_DepthMask(1);                              // depth writes off
Gu_Enable(GU_BLEND);
Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0x000000, 0xffffff);
Gu_ColorFunc(GU_NOTEQUAL, 0x000000, 0xffffff);
Gu_Enable(GU_COLOR_TEST);                     // discard fragments whose RGB is pure black
                                              // -- the two below are OUTSIDE the 0x100/0x200/0x400
                                              //    nest: every transparent batch gets them --
Gu_Enable(GU_ALPHA_TEST);
Gu_AlphaFunc(GU_GREATER, 0, 0xff);            // discard fragments whose alpha is 0
sceGuDepthRange(mesh->0xcc + bias*6, mesh->0xd0 + bias*6);
Fog_Disable();                                // header_byte3 & 1 set, & 8 clear
Gu_StencilOp(0, 0, 0); Gu_Disable(GU_STENCIL_TEST);   // pass_mask & 0xc0 clear, & 0x700 set
```

### The alpha and colour tests are fill-rate optimisations, not the orange-rim mechanism

**This section replaces an over-claim made earlier the same day and retracted
within the hour.** The first version of it read the alpha test as *the* answer
to why the plume's authored orange rim does not appear. That was wrong, and the
measurement that kills it is below. Recorded rather than deleted because the
wrong reading is an easy one to make again.

**Scope first, because the first version got this wrong too.** `Gu_Enable(
GU_ALPHA_TEST)` and `Gu_AlphaFunc(GU_GREATER, 0, 0xff)` sit at the end of
`Gfx_BuildBatchStateList`'s `pass_mask & 0x700` branch, **outside** the
`0x100`/`0x200`/`0x400` nest. They therefore apply to **every transparent batch
in the game** - every track, every ship - not to the plume or to `0x200`
specifically.

**Why `0xdb` is `ATST`, from structure rather than from enum recall.** This is
better evidence than the command-byte ordering and it is inside the same
function. The opaque branch (`pass_mask & 0x700 == 0`) calls the same setter two
ways: with `(1, 0, 0xff)` - function `1`, *always*, i.e. no test - on batches
with `pass_mask & 0x800` **clear**, and with `(6, 0x7f, 0xff)`, `(6, 0, 0xff)`
or `(6, 0x10, 0xff)` - function `6`, *greater*, with a per-batch reference of
half, zero and `0x10` - on batches with `0x800` **set**. `0x800` is
`is_alpha_tested()`'s own bit. A setter that programs an alpha *reference* on
exactly the alpha-tested batches and *always* everywhere else is `ATST` and
nothing else. **That also independently confirms `is_alpha_tested()`'s bit**,
which had no operational evidence before.

**And now the measurement that makes both tests visually inert here.** Dumped
from `/tmp/shipboost.vex` (`oag-wad cat`), all four batches, vertex type
`0x013d` = `GU_TEXTURE_8BIT | GU_COLOR_8888 | GU_NORMAL_8BIT | GU_VERTEX_16BIT`,
stride 20, colour at `+4`:

| RGBA | vertices |
| --- | ---: |
| `(255, 255, 255, 255)` | 63 |
| `(255, 98, 5, 0)` | 58 |

121 vertices, **exactly two colours, alpha strictly `{0, 255}`**. Under the
recovered `GU_SRC_ALPHA` / `GU_FIX 0xffffff` blend a fragment contributes
`src * srcAlpha`, so an alpha-zero fragment contributes **exactly zero** with or
without the alpha test. `GU_GREATER, 0` discards precisely the fragments that
were already contributing nothing. The colour test is inert for the same
reason: a black fragment adds zero under an additive blend whatever its alpha.

**So both tests are fill-rate optimisations - they stop the GE writing pixels
that would not have changed the framebuffer - and neither changes the
picture.** Implementing them would be a no-op on this content.

**The colour test is redundant for a stronger reason than the alpha test, and
the argument does not depend on the authored content at all.** The blend is
`out = src*srcA + dst*1`. A fragment whose RGB is exactly zero contributes
`0*srcA + dst = dst` - the destination, unchanged - which is bit-identical to
discarding it, **for any alpha and any authored colour**. The alpha test's
redundancy is content-dependent (it needs `srcA == 0` on the killed fragments,
which the strictly bimodal authored alpha supplies); the colour test's is
algebraic. Both are inert here.

**Naming the pattern, because it recurs and it is a trap.** A fixed-function
console renderer programs per-fragment *tests* to avoid paying for writes that
an additive blend would have made invisible anyway. On the PSP that is a real
saving - it is fill rate, the scarcest resource on the part. On a reimplementation
with a wholly different fill budget those same tests are recovered-but-inert
state: correct to *document*, wrong to *port*, and actively harmful to port as a
"fix" because they cost a fragment branch and change nothing. **Recovering a GE
call is not by itself an argument for implementing it.** The question to ask of
each recovered state is whether it can change a pixel given the blend that batch
also programs.

**What the `0x200` block reduces to for a reimplementation**, once both tests
drop out: an additive `SrcAlpha`/`One` blend, depth-tested with depth writes
off, two-sided, fog off, repeat wrap on both axes. That is all of it. Everything
else `Gfx_BuildBatchStateList` emits for a `0x1232` batch is either inert or
already implied by those five.

**One caveat, honestly unresolved.** The two arguments above are about the
**colour** channels. A discarded fragment writes nothing at all, whereas a
blended one still writes the *alpha* channel (PSP blending applies to RGB;
`Gu_PixelMask(0)` leaves every channel writable). In a `GU_PSM_5551` target the
single alpha bit **is** the stencil buffer, so "writes alpha" and "writes
stencil" are the same statement, and these batches do run with
`Gu_StencilOp(0, 0, 0)` and `Gu_Disable(GU_STENCIL_TEST)`. Whether the tests
therefore differ in the framebuffer's alpha/stencil bit was **not established**
- it needs the framebuffer pixel format and the GE's alpha-write rule under a
disabled stencil test, neither of which was read. It cannot matter to this
project, whose render target is a separate RGBA8 colour buffer with its own
depth/stencil attachment, but it is the reason the redundancy claim is scoped to
"the picture" rather than to "the framebuffer".

**What actually suppresses the orange rim, then**, is the thing the project
already does: the orange vertices are authored at **alpha 0**, and the blend is
weighted by source alpha. The rim contributes nothing at its vertices and only
its interpolated interior contributes at all. That mechanism was already
correct in `oag_render::exhaust::BLEND` before this pass started. The five
candidate multipliers on [`exhaust.md`](exhaust.md) read negative because
**there is no multiplier and there never was** - the suppression is the authored
alpha meeting a source-alpha blend, and it is already implemented.

Any residual orange difference against a capture therefore has a **different**
cause, and the most likely candidate is that the comparison predates the uvgen-2
fix, which changed the plume's sampled texel from a constant white column to the
real streak lookup. Re-measure before opening a new thread on it.

### `oag_render::exhaust::BLEND` is exactly right, and is no longer a departure

The recovered call is `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0x000000,
0xffffff)` - source weighted by source alpha, destination weighted by fixed
white, i.e. **`SrcAlpha` / `One`**. That is bit-for-bit what
`oag_render::exhaust::BLEND` already uses.

This page currently describes that choice as "a deliberate departure from this
page ... the GE path that performs that per-fragment weight is not recovered".
**Both halves of that sentence are now wrong in the project's favour**: the
per-fragment weight is recovered, it is `GU_SRC_ALPHA`, and the renderer's
existing constant matches the original rather than departing from it. The
`GU_FIX`/`GU_FIX` reading it was compared against belongs to the `0x2000`
batches, not to the plume.

It is also identical to `ExhaustFlare_BuildDisplayList`'s own
`Gu_BlendFunc(0,2,10,0,0xffffff)`, already decoded on
[`exhaust.md`](exhaust.md) - so the flare and the plume share one blend
equation, which they were never known to.

### Three `pass_mask` bits decoded, inside the `0x0700` transparent class

`is_transparent()`'s `0x0700` mask is confirmed operationally - it is the exact
test `Gfx_BuildBatchStateList` branches on - and the three bits inside it are
now separated:

| Bit | Programs |
| --- | --- |
| `0x100` | `Gu_Enable(GU_BLEND)`, `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_ONE_MINUS_SRC_ALPHA, 0, 0)`, `Gu_Disable(GU_COLOR_TEST)` - ordinary alpha blend |
| `0x200` | `Gu_Enable(GU_BLEND)`, `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX 0xffffff)`, `Gu_ColorFunc(GU_NOTEQUAL, 0, 0xffffff)`, `Gu_Enable(GU_COLOR_TEST)` - **additive, source-alpha weighted**; the plume's |
| `0x400` | `Gu_Disable(GU_BLEND)` - in the transparent class but not blended |

Only the blend equation and the colour test differ between the three. The
depth-write disable, the alpha test and the stencil setup are **common to all
of them**, emitted outside the nest.

Checked in that order: `0x100` wins over `0x200`, which wins over `0x400`.
`mesh_render::TRANSPARENT_BLEND`'s `SrcAlpha`/`OneMinusSrcAlpha` is therefore
**correct for `0x100` batches and wrong for `0x200` ones**, which is a sharper
statement than this page's earlier "evidence against it, in favour of the
`GU_FIX` additive equation" - both equations are real, and the batch's own bit
picks between them. That resolves the "whether `TRANSPARENT_BLEND` should
change project-wide" open item into a concrete rule rather than a project-wide
flip.

`header_byte3 & 0x10` selects a separate branch entirely -
`Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)` when
`header_byte3 & 2` is clear and `(GU_ADD, GU_FIX, GU_SRC_ALPHA)` when set, with
`Gu_Disable(GU_ALPHA_TEST)`. That is the pure-additive pair the
`Mesh_SetBatchDrawState` table describes, reached here through a different
gate; no plume batch takes it (`header_byte3 = 0x01`).

### Recommended code changes, for the agent that owns `crates/render`

Described rather than made, per this pass's scope. None of this is verified
against a live capture.

1. **Make `mesh_render::TRANSPARENT_BLEND` per-batch**, keyed on
   `pass_mask & 0x0700`: `0x100` -> `SrcAlpha`/`OneMinusSrcAlpha`, `0x200` ->
   `SrcAlpha`/`One`, `0x400` -> no blend. This is the one with real visual
   consequences, and it affects every track and ship, not the plume.
2. **Keep `exhaust::BLEND` exactly as it is.** Remove the comment calling it a
   departure; it matches the original exactly.
3. **Disable fog on batches with `header_byte3 & 1`.**
4. **Do not implement the alpha or colour tests** for their visual effect - the
   section above measures them inert on this content. They are worth adding only
   if the renderer ever wants the original's fill-rate behaviour, which it does
   not.
5. ~~`is_alpha_tested()`'s `0x800` bit now has operational evidence and a
   per-batch alpha reference behind it (`0x7f`, `0`, `0x10`); if the renderer
   ever implements alpha-tested batches, that reference is where it comes
   from.~~ - **done 2026-09-10.** The selector is read, the reference is
   plumbed per draw call, and the pixel cost is measured. See
   ["The alpha-test reference's selector, and the pad that
   discriminates"](#the-alpha-test-references-selector-and-the-pad-that-discriminates)
   below.

### Still open here

- ~~**`FUN_0891e8c8`**, the last unread step in this chain.~~ - **closed
  2026-08-09 at confidence 88, and it changes nothing.** `Gfx_ResetTexTransform`
  (`0x0891e8c8`) is two calls in full:

  ```c
  Gu_TexScale(1.0f, 1.0f);
  Gu_TexOffset(0.0f, 0.0f);
  ```

  A **texture-transform reset**, and the two calls are a coherent pair. No blend
  equation, no enable/disable, no test, nothing that touches fragment colour or
  alpha. **The alpha-test result above is not overridden by it.**

  **Corrected the same day.** The first version of this entry read the second
  call as `Gu_Fog_q` - the live Ghidra database's name for `0x08811630` - and
  spent a paragraph speculating about whether it neutralised
  `Gfx_BuildBatchStateList`'s `Fog_Apply` branch. It does not, because it is not
  a fog call: `0x08811630` emits GE commands **`0x4a`/`0x4b`**, `TEXOFFSETU` and
  `TEXOFFSETV`, i.e. `sceGuTexOffset`. `names.tsv` has carried the correct name
  at confidence 88, cited to [texture-animation.md](texture-animation.md), the
  whole time; **this page trusted the database over the docs, which is exactly
  what ADR-0005 forbids and exactly what the stale-name table below warns
  about.** The speculation it produced was wholly spurious.
- **`FUN_0891e9d0`**, the third fog branch (`header_byte3 & 8` clear and `& 1`
  clear).
- Whether `entry->0x14`, set to `Gu_Finish() + 4`, is a length used anywhere
  other than bookkeeping.
- None of this was verified live; it is all static decompilation, though the
  `pass_mask` and `header_byte3` values it is evaluated at are disc
  measurements.

## The render queue, its key, and its sort (2026-08-18)

Recovered while answering a question this page had already half-answered: it
records that `Mesh_CompileBatchSet` runs "four comparator sorts" and that
`Gfx_FlushRenderManager` is "sort plus deferred draw callbacks", and neither
said what is sorted or by what. The whole ordering model is on
[`docs/rendering/draw-order.md`](../../../rendering/draw-order.md); what follows
is the four functions and their evidence.

**A note on addresses.** The decompiler renders call targets in this region
unrelocated - `func_0x0011a35c` and the like. The image base is `0x08804000`, so
`0x0011a35c` is `0x0891e35c`; every address below is the relocated one, and the
mapping is confirmed by `Gfx_FlushRenderManager` itself decompiling at
`0x0891e3c0 = 0x08804000 + 0x11a3c0`.

### `Gfx_Enqueue`

| | |
| --- | --- |
| **Address** | `0x0891e35c` |
| **Confidence** | 92 |

Appends one 8-byte entry - item pointer at `+0x00`, sort key at `+0x04` - to the
array at `display+0x16a0`, bumping the count at `display+0x5520`. The array is
bounded at 2,000 entries by where the count sits. When `display+0x1180` is not
`0xffffffff` it replaces the key's low twenty bits and keeps the top twelve:

```c
key = (d->0x1180 & 0xfffff) | (key & 0xfff00000);
```

92 rather than higher because what writes `+0x1180` was not traced. 63 call
sites, of which the ones this page cares about are `0x0892ece0` (a batch set)
and `ExhaustFlare_Submit`.

### `Gfx_CompareQueueKeys`

| | |
| --- | --- |
| **Address** | `0x0891ddec` |
| **Confidence** | 95 |

The `qsort` comparator `Gfx_FlushRenderManager` passes. Four instructions, and
there is nothing in it to interpret:

```asm
0891ddec  lw    v0, 0x4(a0)
0891ddf0  lw    a0, 0x4(a1)
0891ddf4  jr    ra
0891ddf8  _subu v0, v0, a0
```

`a->key - b->key`, so **ascending on the whole signed 32-bit word**. It reads
`+0x04` of each element, which is what fixes the queue entry's layout as
`{ item, key }` rather than the other way round.

Not 100 only because the rubric caps a static reading; there is no reading of
these four instructions that differs.

### `Gfx_ViewDepth`

| | |
| --- | --- |
| **Address** | `0x0890486c` |
| **Confidence** | 88 |

Transforms a point by the view matrix at `_DAT_00059568` with `vtfm4_q` and
returns the **z** component (`auVar1._8_4`). Returns `10000.0` unchanged when any
of the point's three components is the `0x7f800001` sentinel. Named by
`exhaust.md` since before this pass; the row is new because the address had never
been written down beside it.

### `0x0892ece0`, a batch set's submit method

**Deliberately unnamed.** Its shape is unambiguous - it enqueues `self` with
`*(self+0xbc)`, which `Mesh_CompileBatchSet` stores its layer-key argument into
at `0x0892f40c` (`sw s0, 0xbc(s1)`) - but it is reached through a vtable and no
call site names it, so a `Subsystem_VerbNoun` here would be a guess about which
subsystem owns it. Below 70; the hypothesis is written down instead.

```c
if (*(int *)(set + 0xbc) != 0x40000000 ||
    _DAT_0005ab8c <= -*(float *)(_DAT_002ad0b0 + 0x74))
    Gfx_Enqueue(g_display, set, *(int *)(set + 0xbc));
```

**Two things this settles.** A mesh batch set is queued with its **bare layer
key** and no depth term, so mesh geometry is not depth-sorted by the original at
all. And the `0x40000000` set is *skipped* unless a distance test passes - a
cull living inside the submit, not an ordering rule.

### The sort, `0x08972860`

**Left unnamed on purpose**: it is a textbook `qsort` - Bentley-McIlroy
three-way partition, median-of-three under 0x28 elements and a ninther above,
insertion sort under 7, tail recursion on the larger half - and forcing this
project's `Subsystem_VerbNoun` convention onto a libc routine would say something
false about where it came from. **It is not stable**, which is the property a
reimplementation has to know it is free to choose within.

### The layer derivation, and what it is worth

`Mesh_InitFromPayload`'s re-derivation is written up above under "The layer key
at `mesh+0x4c`". Restated as a rule, and it fires only when the model's own key
is `0x45000000`:

```text
payload[+0x0c] & 0x8                    -> 0x31000000
else flags & 0x0080                     -> 0x4a000000
else flags & 0x2000 && flags & 0x0040   -> 0x4a000000
else flags & 0x1000                     -> 0x45000000
else                                    -> 0x4a000000
```

`oag_vex::vex::mesh_layer` is this, and
`crates/vex/tests/vex_layer_ground_truth.rs` censuses it over `Data.wad`:
**21,055 mesh nodes, 66.4 % on `0x45` and 33.6 % on `0x4a`**, with the `0x31`
branch never firing on this disc. Of 5,610 transparent batches, 63.7 % / 36.3 %.
So the rule separates a real two-thirds/one-third split rather than naming a
field nothing distinguishes by - which is the check that was run before any of
it reached the renderer.

**Independently confirmed 2026-08-25, and pinned to the batch rather than the
mesh.** `Mesh_CompileBatchSet` (`0x0892f35c`) carries the identical four-test
rule a second time, decompiled directly rather than inferred from
`Mesh_InitFromPayload`'s side:

```c
if ((mesh+0x4c & 0xfff00000) == 0x45000000) {   // only when the mesh's own key already says so
    if ((batch_flags_byte & 8) == 0) {
        if ((batch->pass_mask & 0x80) == 0) {
            if ((batch->pass_mask & 0x2000) == 0 || (batch->pass_mask & 0x40) == 0) {
                layer = (batch->pass_mask & 0x1000) ? 0x45000000 : 0x4a000000;
            } else { layer = 0x4a000000; }
        } else { layer = 0x4a000000; }
    } else { layer = 0x31000000; }
}
```

Confirms the tests read `batch->pass_mask`, the same `+0x00` field
[`vex.md`](../../../formats/vex.md) documents, not a mesh-level flags word -
this is where a mesh whose own key is the ambiguous `0x45000000` gets split
batch by batch between the two real layers. This retires this page's own
"`& 0x1000` is undecoded here" note in the plume section above, and answers
what the bit means on Moa Therma's magstrip:
`crates/render/tests/magstrip_ground_truth.rs`'s `OVERLAY_PASS = 0x0021` (no
`0x1000`, the far-LOD copy) versus the base strip's `0x1021`/`0x1001` (`0x1000`
set) is exactly this bit, and it means the two copies are enqueued on
*different* layers - base strip on `0x45000000`, overlay on the default
`0x4a000000` - not just drawn in file order. It changes nothing about the fixed
artifact: the original never has both visible at once (the overlay's own PVS
section keeps it out of the racing view), so the layer split was never what
resolved the z-fight - authored-section placement was, already landed. The
bit's role elsewhere (splitting a real 66.4/33.6 mesh population) stands
independently of what it happens to do for this one asset.

## The alpha-test reference's selector, and the pad that discriminates

**2026-09-10, confidence 86.** `Gfx_BuildBatchStateList` programs one of three
alpha references and this page recorded all three without saying what chooses
between them. It is two bits of the batch's own header, and the chain that
proves they *are* the batch's own header is three functions long with no
transform in it.

### The branch

Inside the opaque class (`param_2 & 0x700 == 0`) and outside the
`header_flags & 0x10` branch that disables the test:

```c
if ((param_2 & 0x800) == 0) {                 // not alpha-tested
    Gu_Enable(0); Gu_AlphaFunc(1, 0, 0xff);   // GU_ALWAYS - no test
} else {
    Gu_Enable(0);
    if ((param_3 & 0x20) == 0) {
        if ((param_2 & 0x80) == 0) { Gu_DepthFunc(6); Gu_AlphaFunc(6, 0x7f, 0xff); }
        else                       { Gu_DepthFunc(6); Gu_AlphaFunc(6, 0,    0xff); }
    } else {                         Gu_DepthFunc(7); Gu_AlphaFunc(6, 0x10, 0xff); }
}
```

Two selector bits, then: **`header_flags & 0x20`** picks `0x10`, and within its
clear side **`pass_mask & 0x80`** picks `0` over `0x7f`. `header_flags & 0x20`
moves `Gu_DepthFunc` as well (`7` rather than `6`), so the bit marks a material
class and not only a stricter cutout - independent corroboration that it is a
real distinction and not a decompiler artefact.

### Why `param_2` and `param_3` are the on-disk `pass_mask` and byte 3

`Mesh_InitBatch` (`0x0890e8b4`), at load, once per batch:

```c
Gfx_AcquireBatchStateList(g_display, *batch, *(u8 *)(batch + 3),
                          mesh->0xcc, mesh->0xd0, *(u8 *)(batch + 0x29));
```

`Gfx_AcquireBatchStateList` (`0x0891df48`) interns that tuple into a 150-entry,
stride-`0x1c` table, and `Gfx_CompileDirtyBatchStateLists` (`0x0891e054`)
replays the entry's `+0x08` (u16), `+0x0a` (u8), `+0x00`, `+0x04` and `+0x0b`
into `Gfx_BuildBatchStateList` in that order. **Nothing between the file and
the builder changes either word**, which is what makes the reference a pure
function of two fields `oag_vex::vex::Batch` already parses.

A side answer to this page's own open question, and only a partial one:
`Mesh_InitBatch` writes `batch+0x28` and `batch+0x29` and **not** `+0x00` or
`+0x03`. So the batch's own init path does not touch the two selector words;
that is not proof nothing else does.

### The disc says three of the four combinations occur, and the third one settles it

Censused over every `.vex` on three discs - mesh nodes and `Speedup
Pad`/`Weapon Pad` nodes alike, since a pad's payload is a mesh payload
(`crates/vex/tests/alpha_test_reference_ground_truth.rs`):

| `pass_mask & 0x880` | `header_flags & 0x30` | reference | PSP Pulse | PSP Pure | PS2 Pulse |
| --- | --- | --- | ---: | ---: | ---: |
| `0x0800` | `0x00` | `0x7f` | 1,500 | 956 | 1,313 |
| `0x0880` | `0x00` | `0` | - | 349 | 20 |
| `0x0880` | `0x20` | `0x10` | 7,823 | 1,749 | 14,046 |

The middle row is the one worth the space. It is the only combination where the
recovered branch and the obvious rival - "`header_flags & 0x20` alone selects,
`pass_mask & 0x80` is unrelated" - give different answers: `0` here, `0x7f`
there. Those 349 Pure batches are its **`Speedup Pad`s**, whose glow texture
tops out at alpha `58/255`; under the rival reading every one of them discards
whole and the pads render nothing, which is the exact failure
`crates/render/tests/pad_alpha_test_ground_truth.rs` was written for in
2026-08-17. It renders them and counts lit pixels, and they draw.

**86 rather than 84** for that reason: the decompile alone is the rubric's
"decompilation only, consistent call sites" ceiling, and what lifts it is that
the shipped data separates this reading from its rival and the separation is
rendered rather than merely counted. **Not 90**, because none of it is a
runtime trace: no breakpoint has seen the GE programmed this way.

### What it costs on screen

`crates/render/examples/threshold_probe.rs` renders a circuit offscreen at
1024x1024 at four yaws, once with each batch's own reference and once with the
old flat `1/255`:

| Circuit | lit before -> after, 4 frames | pixels differing |
| --- | --- | ---: |
| `01_Track` | 85,249 -> 85,102 | 1,298 |
| `16_Track` | 522,427 -> 522,342 | 6,170 |

**Nothing structural leaves the picture** - the two frames read as the same
circuit at every angle - and the lit count moves *up* at two of the eight
framings, which is the signature the change predicts rather than a
contradiction of it: the cutout pipeline returns alpha `1.0` and writes depth,
so a texel at alpha 3 that cleared `1/255` was painted **solid** and occluded
what was behind it. Discarding it reveals brighter geometry.

Pure's `Z3_whiteblue_cloud_GLOW.tga` is the clean example: a uniform alpha of 3
across 4,096 texels on 13 batches, drawn as solid cloud until now, discarded
outright by the reference the file itself asks for.

### Still open here

- The branch **order** between the two bits is untestable on Pulse's PSP disc,
  where they are always set together; it is Pure's and PS2's pads that separate
  them, and only in the one direction the discs happen to author. Nothing on
  any disc sets `header_flags & 0x20` with `pass_mask & 0x80` clear.
- The selector was read in the PSP Pulse executable only. Pure's and PS2's
  executables author the same patterns in the same fields and neither has been
  read.
- `header_flags & 0x10`, which disables the test outright, is authored by **no
  batch on any of the three discs**. The branch is real in the decompile and
  dead in the data.
