# The renderer: where it lives, what it is made of, and how much of it is Pulse

A sweep of `EBOOT.elf`'s rendering layer, 2026-08-18, done to answer the
lineage question this directory exists for: HD is the first title after Pulse
and runs on completely different hardware, so is its renderer recognisably the
same shape?

**The short answer is yes for the scene layer and no for the device layer**, and
the seam between them is sharp enough to point at. The asset half of the same
question is [hd-status](../../../formats/hd-status.md); this is the executable
half.

Nothing here has been run under an emulator. Every score rests on static
reading of one binary, which [caps it at 84](../../../reverse-engineering/confidence-rubric.md).

## The renderer is in this binary, and `DFEngine.sprx` is not where to look

`EBOOT.elf` imports libgcm directly: `_cellGcmInitBody`,
`cellGcmSetDisplayBuffer`, `cellGcmSetFlipMode`, `cellGcmSetFlipImmediate`,
`_cellGcmSetFlipCommand`, `cellGcmAddressToOffset`, `cellGcmMapMainMemory`,
`cellGcmBindTile`, `cellGcmUnbindTile`, `cellGcmBindZcull`,
`cellGcmUnbindZcull`, `cellGcmSetTileInfo`, `cellGcmGetControlRegister`,
`cellGcmGetOffsetTable`, `cellGcmGetLabelAddress`, `cellGcmGetConfiguration`,
and the four handler setters. **26 of the 332 imports are `cellGcm*`.**

From `DFEngine.sprx` it imports exactly **one** symbol, `DFEngine_0x17D490A3`,
whose NID the database does not resolve. So the earlier note that gameplay
lives in `EBOOT.elf` rather than the closed module extends to the renderer:
the device layer, the command buffers and the scene layer are all here.

**What is *not* here, and this is the important negative:** none of the
draw-state entry points appear as imports, because on PS3 they are not calls.
`cellGcmSetBlendFunc`, `cellGcmSetVertexDataArray`, `cellGcmSetDrawIndexArray`
and their neighbours are inline functions in the `cellGcmSys` headers that
write RSX methods straight into the command buffer. **Their absence from the
import list is not evidence they are absent from the binary**, and an import
census cannot find the draw path. This is the single biggest difference from
[Pulse's import stubs](../psp-pulse-usa/imports.md), where `sceGu*` really is a
stub table and 306 of 335 names were recovered by hashing.

## How this sweep was possible without re-importing the database

Every claim below depends on reading a TOC-relative load correctly, and this
database resolves 59% of them against the wrong base - the defect written up in
[memory.md](memory.md#every-function-has-its-own-toc-and-ghidra-uses-one-for-all-of-them).
`AssignPs3R2FromOpd.java` is the in-Ghidra fix and this database predates it;
re-importing needs Ghidra closed and was not done here.

Instead the resolution is done **against the file, outside Ghidra**, by
[`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py): it walks the OPD, gives
each function the TOC its own descriptor declares, and resolves a
`lwz rX,disp(r2)` from there. Its self-check is `memory.md`'s worked example,
which it must reproduce exactly:

```console
$ scripts/ps3-toc.py resolve 0x003914b0 -0x6634
0x008b6d90 -> 0x007afe68 'Small is  %3.2f MB (%d bytes)\n'
```

`"forward"` there is the wrong answer and the whole reason the script exists.

It also confirms the arithmetic that makes Ghidra usable as-is where it is
wrong: **a module-B function's Ghidra data reference sits exactly `0xfeec`
below the true slot**, `0x008bd3c4 - 0x008ad4d8`. Seen twice in this sweep -
Ghidra's `PTR_s_FEMain_Screen_cpp_008b0968` in the GCM init is really
`0x008c0854`, and its `PTR_s__s_screen_m_xml_008aec28` in the device init is
really `0x008beb14`.

**The whole render layer sits below `0x0032d5e0`**, so it is module A and
Ghidra's default TOC is correct there. That is why the scene-layer readings
below could be taken from the decompiler directly, and only the two device-layer
functions (both above `0x0032d5e0`) needed the script.

### What the `.cpp` attribution does and does not prove

`scripts/ps3-toc.py map` reproduces the `__FILE__` trick across the whole
binary: the C++ base class stores its own `__FILE__` at object offset `0x30`,
and the allocation macro passes it at every call site. It is **validated
against work already committed** - `Collision.cpp` attributes to `0x00034350`
and `0x00034aa0`, which are `Collision_AllocateBuffers` and
`Collision_Construct` in [names.tsv](names.tsv), recovered independently.

What it maps is **constructors and allocation sites, not classes**. A class's
update, submit and draw methods do not name their own file and do not appear.
So the tables below are an inventory of what exists and where it starts, and
they say nothing about behaviour.

**The scan decodes the single `lwz rD,disp(r2)` form only**, not the
`addis`/`lwz` pair a displacement past +-32 KiB would need. That limit is
checked rather than assumed: of the 443 distinct `.cpp` names in the image it
attributes 434, and **every one of the 9 it misses is an SPU-side or library
translation unit** - `jobmain.cpp`, the `edgezlib_*` and `edgeanim_*` asserts,
`mp3dec/*`, `LinkObj.cpp`. No importer and no renderer-family file is among
them, so the censuses below are complete for the layer they describe.

## The render layer's address range

436 of the 462 `.cpp` names attribute to at least one function. The rendering
and scene-graph translation units occupy one contiguous run,
**`0x00279xxx`-`0x002ecxxx`**, with a second copy of each importer's registration
around `0x006bxxxx`:

| `.cpp` | First function | What it is |
| --- | --- | --- |
| `FxHudDamage.cpp` | `0x002795f0` | full-screen damage effect |
| `FxScreen.cpp` | `0x00279860` | full-screen effect base |
| `FxWeather.cpp` | `0x0027b790` | weather effect |
| `MaintainParticlesSpu.cpp` | `0x0027cb78` | the SPU particle job's PPU side |
| `ParticleManager.cpp` | `0x0027f840` | particle system owner |
| `FxScreenMist.cpp` | `0x0028fed8` | screen mist |
| `Billboard.cpp` | `0x00299f58` | billboard primitive |
| `Font.cpp` | `0x002accd8` | font |
| `LensFlare.cpp` | `0x002aee58` | lens flare |
| `Model.cpp` | `0x002c04b0` | model |
| `Mesh_Importer.ps3.cpp` | `0x002bebb0` | **the one platform-suffixed importer** |
| `Pen.cpp` | `0x002cdbe8` | 2D drawing primitive |
| `RenderManager.cpp` | `0x002d5710` | the renderer |
| `SortRoot.cpp` | `0x002dbe08` | sorted-list base, guessed as draw-order root - [not render-confirmed](#what-was-deliberately-not-read) |
| `EngineTrail/TrailEffectManager.cpp` | `0x002e2da8` | engine trails |
| `FxLensflare.cpp` | `0x00637f70` | lens flare, module B |

`Pen.cpp` and `Font.cpp` next to each other is worth noting: that is the same
2D pairing Pulse has, and `Pen` is not a name a fresh codebase invents.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x002d6190` | `RenderManager_CreateInstance` | 84 |
| `0x002d5ed0` | `RenderManager_ConstructComplete` | 80 |
| `0x002d5710` | `RenderManager_Construct` | 76 |
| `0x002d6300` | `RenderManager_FlushDrawQueue` | 90 |
| `0x002d6a78` | `RenderManager_PrepareEye_q` | 55 |
| `0x002d4b58` | `RenderManager_CompareQueueKeys` | 82 |
| `0x00864cb8` | `g_FrontendRootVtable` (data) | 84 |
| `0x001635a0` | `FrontendRoot_ApplyLayerMatrix_q` | 65 |
| `0x002bebb0` | `MeshImporter_Construct` | 78 |
| `0x00650330` | `Gcm_Init` | 88 |
| `0x005c9194` | `Gcm_InitDevice` | 82 |
| `0x005bd2f8` | `GcmContext_Callback` | 76 |
| `0x008c0854` | `g_GcmContext` (data) | 85 |
| `0x005c18d8` | `Rsx_UploadVertexConstantBlock` | 88 |
| `0x005cd728` | `ShaderRegistry_Find` | 86 |
| `0x005cd6d8` | `ShaderRegistry_Register` | 84 |
| `0x005cd990` | `ShaderRegistry_LoadAll` | 86 |
| `0x005cd8c0` | `ShaderRegistry_UnloadAll` | 84 |
| `0x008bf1ac` | `g_ShaderRegistryHead` (data) | 84 |
| `0x005a9998` | `Texture_BuildGcmRegisters` | 75 |

### `RenderManager_CreateInstance`, and the size of a `RenderManager`

`0x002d6190` is three statements:

```c
void RenderManager_CreateInstance(void)
{
  instance = FwMemAllocator_AllocateDefault(0x4d90);   // via the stub at 0x00676748
  RenderManager_ConstructComplete(instance);
  render_globals[0x14 / 4] = instance;
}
```

`0x00676748` is a TOC stub for `FwMemAllocator_AllocateDefault`, already named
in [memory.md](memory.md), so the allocation is not a guess. **84** because
allocate-then-construct-then-store is unambiguous and the size corroborates
itself: `0x4d90` bytes, and the constructor's highest field write is `+0x4d84`.

What separates the two constructors is that call site, not their code. Under
the C++ ABI GCC emits a complete-object and a base-object constructor
separately, which is why this binary keeps producing near-identical pairs - the
same shape [mode-manager.md](mode-manager.md) works through for `ModeManager`,
and the same rule applies:

- `0x002d5ed0` has exactly one caller and that call site is `new`, so it is the
  **complete-object** constructor. 80.
- `0x002d5710` has no callers at all and is reached only through its OPD entry,
  so the base-object reading is by elimination rather than by evidence. **76**,
  and it is the weakest name on this page.

They are **not** byte-identical - they agree for `0x52` bytes and then diverge -
even though Ghidra decompiles them to the same text. Anyone re-checking this
should diff the bytes, not the decompiler output.

### What the constructor sets up

Read at `0x002d5ed0`, all direct decompiler reads:

- **A `0xc0000`-byte (768 KiB) buffer**, aligned `0x40`, tagged
  `RenderManager.cpp` line `0xa2`, memset to zero, and stored into the render
  globals block at `+0x04`.
- **`0x280` and `0x1c0` written to that same globals block** at `+0x0c` and
  `+0x10` - 640 and 448. What that pair is for is **not established**; it is
  not the PS3 output resolution, which the device init negotiates separately,
  and it does not match any figure in [hd-hud](../../../formats/hd-hud.md).
  Recorded because it is measured, not because it is understood.
- **A resolution-dependent constant pair.** If a queried height is below `0x2d0`
  (720) the fields at `+0x104` and `+0x108` get `84.0f`, otherwise `96.0f`. So
  something in the renderer is scaled between the sub-720 and 720-and-above
  cases, which is exactly the axis a PS3 title has and a PSP title does not.
- **A 512-byte table** at `+0x4b48` cleared as 64 eight-byte entries.
- Four calls to one framework function with the selectors `2, 1, 0, 3`, around
  two `0x40`-byte block copies into arrays at `+0x120` and `+0x3a0` indexed by
  counters at `+0x620` and `+0x624`. `0x40` bytes is a 4x4 float matrix and the
  shape is a matrix-stack push, but **nothing here proves that** and no name is
  recorded for it.

### `Gcm_Init` and `Gcm_InitDevice`

`0x00650330` is the `cellGcmInit` inline wrapper: it loads the context pointer
from its own TOC and tail-calls `_cellGcmInitBody`. **88** - one call to a
named import, and the only recovered fact is which global is the context.
That global is `0x008c0854`, resolved through the function's real TOC; Ghidra
labels it `0x008b0968`, `0xfeec` low, which is what made it look like a string
pointer.

`0x005c9194` is its only caller and is the device bring-up. In order:

1. `cellGcmInit(ctx, 0x10000, ...)` - a **64 KiB** command buffer.
2. `cellGcmGetConfiguration`, `cellGcmGetLabelAddress(0)` and `(1)` (the
   difference is kept as the label stride), `cellGcmGetOffsetTable`,
   `cellGcmGetControlRegister`.
3. `cellVideoOutGetState` / `cellVideoOutGetResolution`, then a walk over a
   table of `0xc`-byte candidate entries calling
   `cellVideoOutGetResolutionAvailability` until one is available - the
   resolution negotiation.
4. `cellVideoOutConfigure`, then aspect and scan-mode fields stored from the
   returned state.
5. `cellGcmSetFlipMode(2)`.
6. **Four `CellGcmContextData` records** - `{begin, end, current, callback}` -
   over four 4 KiB segments at `+0x10000`, `+0x11000`, `+0x12000` and
   `+0x13000` of the IO base, every `callback` null.
7. Six words pushed into the command buffer: `0x41fcc`, `0x6f`, `0x401a8`,
   `0xfeed0001`, `0x41450`, `0x80004`.

**82.** The named imports carry it, and the context layout is corroborated
independently by `GcmContext_Callback` below. The step-7 words are recorded
raw and **deliberately not decoded here**: read as RSX method headers,
`0x401a8` with `0xfeed0001` is a one-word write to method `0x1a8` with the
main-memory DMA handle, which would be `NV4097_SET_CONTEXT_DMA_REPORT` - but
that reading is **60 at best**, no method table was consulted against this
firmware revision, and the other two pairs do not decode as cleanly. A wrong
method name here would be worse than none.

`0x005bd2f8` is `if (context->callback) return context->callback(...)` and
returns 1 otherwise. It is called from the device init when `current + 6`
would pass `end`, which is what fixes the field at `+0x0c` as the callback and
therefore the whole `{begin, end, current, callback}` layout. **76** - the
behaviour is certain and only the name is a convention.

### Every word of that init was confirmed live, and the command stream is a chain (2026-09-05)

Read out of a running race through the RPCS3 stub - see
[rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md), which uses
this for the camera. **Confidence 90.**

`g_GcmContext` is **two** dereferences from the `CellGcmContextData`, not one:
`[0x008c0854]` is `0x013be314`, libgcm's own `CellGcmContextData *`, and
`[0x013be314]` is `0x016a0ccc`, the struct. It reads

    0x016a0ccc:  40001000 40007ffc 400013a4 016a0504
                 begin     end      current  callback

confirming the `{begin, end, current, callback}` order the callback function
had already fixed behaviourally.

**The ring this context owns is the device bring-up buffer and nothing else.**
Its `current` is `0x400013a4` in two separate boots and never advances, and its
32 KB do not change by one byte across three shots six seconds apart in a live
race. The last word before `current` is

    0x400013a0:  20010000

an RSX **JUMP** (`0x20000000 | io_offset`) to IO offset `0x10000` - which is
`0x40010000`, the first of the four 4 KiB auxiliary contexts step 6 above
recovered statically. That is independent runtime corroboration of step 6, and
of step 7: each of those four segments holds the same ~0xb8 bytes of commands,
`0x41fcc/0x6f`, `0x401a8/0xfeed0001` and `0x41450` among them.

Each auxiliary segment ends in a JUMP of its own:

    0x400100b4:  20074100    ->  0x40074100
    0x400110b4:  20075100    ->  0x40075100
    0x400120b4:  20074100
    0x400130b4:  20075100

Two targets, alternating - a double-buffered pair. **The frame's actual draw
commands are there**, at IO `0x77000` and up, and nowhere below it.

### `Rsx_UploadVertexConstantBlock` (`0x005c18d8`), the bulk constant uploader

`Rsx_UploadVertexConstants` (`0x005c176c`) is not the only emitter of
`NV4097_SET_TRANSFORM_CONSTANT_LOAD`. `0x005c18d8` is the bulk form, and it is
the one a camera matrix arrives through. **88.**

Its arguments are `(context, start_index, vec4_count, values)`. It writes the
count as `vec4_count >> 3` blocks of eight `vec4` with the literal header
`0x00841efc` - `(33 << 18) | 0x1efc`, an index plus 32 floats - and then a
tail of `vec4_count & 7` more, whose header it *computes*:

```c
*puVar21 = (int)(uVar25 << 0x14) + 0x40000U | 0x1efc;
```

`(n << 20) + 0x40000 | 0x1efc` is `((4n + 1) << 18) | 0x1efc`, so `n` `vec4`s
cost `4n + 1` words. **`n = 4` gives `0x00441efc`**, one index and sixteen
floats: a whole `float4x4` in a single packet. That header is what appears
hundreds of times per frame in the live pushbuffer, and `0x00441efc` never
appears as an immediate anywhere in the image precisely because it is computed
here rather than written down.

The context handling is identical to `Rsx_UploadVertexConstants`' - `+0x8` as
`current`, `+0x4` as `end`, `GcmContext_Callback` when the write would pass it
- which is the second reason to read the two as a pair.

**What that gets, measured**: the constant registers actually loaded during a
Talon's Junction race are `c[462..467]` through the count-5 form (the
material's own block, matching the `track_surface.rcsmaterial` values read
below) and **`c[256]` and `c[260]`** through the count-17 form. Under this
page's `N + 256` rule those are the shader's `c[0..3]` and `c[4..7]`, and
`c[0..3]` is where `viewProj` lands.

### The per-eye draw dispatch, and what it says about sort order

Found 2026-08-26 chasing HD's open question of whether it depth-sorts
transparent draws anywhere. `SortRoot`
was already spent as a lead (above); this is the render layer's own call site,
reached from `Game_PresentLoop_q` rather than from `SortRoot`.

`RenderManager_PrepareEye_q` (`0x002d6a78`) and
`RenderManager_FlushDrawQueue` (`0x002d6300`) are always called as a pair,
`Prepare` immediately before `Flush`, at all seven call sites: six inside
`Game_PresentLoop_q`'s mono/stereo branches (matching the "mirrored two-pass
draw sequence for the stereo path" [engine-trail.md](engine-trail.md) already
described from the caller's side) and once more from the boot function before
the loop starts, priming the first frame. Both take the `RenderManager`
instance itself as `param_1` - confirmed by field agreement, not guessed:
`Flush` reads and writes `+0x620`, `+0x624`, `+0x44b0` and `+0x630`, all fields
`RenderManager_Construct`/`_ConstructComplete` initialise on the same struct.

`RenderManager_FlushDrawQueue`, in order:

1. Zeroes the two matrix-stack depth counters at `+0x620`/`+0x624` - the same
   counters the constructor sets to zero at startup, so this is a per-frame
   stack reset, not a one-time initialisation.
2. **Sorts** the array at `+0x630` - see the correction below; this step was
   missed on first read.
3. Walks the (now sorted) array at `+0x630` of `{object, extra}` 8-byte pairs,
   index `0` to the count held at `+0x44b0`, calling `(*object->vtable[0x1c])(object,
   render_manager, extra)` on each - a virtual dispatch through the queued
   object's own vtable, not RenderManager's.
4. Zeroes `+0x44b0` at the end, so the same array at `+0x630` is empty again
   for the next frame's `Prepare`/`Flush` pair. Its capacity is not
   established - nothing read here bounds the allocation, only the live
   count.

**2026-09-04 correction: the walk is strict index order with no
comparison, but the walk is not the whole function.** The paragraph this
replaces read the dispatch *loop* correctly - no distance term, no
material/layer read, no branch keyed on the entry's contents inside the
loop itself - and wrongly generalised that to the function as a whole.
`RenderManager_FlushDrawQueue` opens by loading a function-pointer OPD
(`PTR_PTR_008b3d94`, resolved by direct memory read as `{0x002d4b58,
0x008ad4d8}` - a real OPD pair, not a decompiler artefact) and calling
`FUN_00677088(param_1 + 0x630, count@+0x44b0, 8, comparator)`, where
`FUN_00677088` is a one-line trampoline whose body the decompiler already
resolves to a literal `qsort(...)` call. **`0x002d4b58` is
`RenderManager_CompareQueueKeys`**: `return *(int*)(a+4) - *(int*)(b+4);` -
an ascending sort on the entry's `+0x04` word, four instructions, the exact
comparator shape as Pulse's `Gfx_CompareQueueKeys`
([`mesh-draw.md`](../psp-pulse-usa/mesh-draw.md)) on the identical `{item,
key}` 8-byte layout. **This happens before the dispatch loop runs**, so
"whatever populates `+0x630` decides the draw order" is wrong: the sort
does, same as Pulse. What still isn't established is what values the `extra`
word holds going into the sort (layer+depth, like Pulse, or something
else) - that question moves to the enqueue site, unchanged from before.
The comparator's own OPD carries TOC `0x008ad4d8`, the module-A TOC Ghidra
already assumes correctly binary-wide - `0x002d4b58` sits below this page's
`0x0032d5e0` module-A/B boundary, so its decompile is trustworthy at face
value with no [`ps3-toc.py`](memory.md#every-function-has-its-own-toc-and-ghidra-uses-one-for-all-of-them)
correction needed.

**90**, runtime-verified 2026-08-26 (below): a live breakpoint confirmed `r3`
at entry is the `RenderManager` instance and that `+0x44b0`/`+0x630` hold a
real, populated draw queue during an actual race, not just field-offset
agreement with the constructor. Capped short of the rubric's 94 ceiling for a
single-binary trace because the breakpoint confirms the *dispatch* loop's
shape and data, not every claim on this page - the enqueue site itself was
still unread at the time of this trace (it has since been read separately,
below). `PrepareEye` is unaffected by that trace and stays where it
was (**55**, `_q`): it does per-eye resolution/aspect bookkeeping (writes
`+0x104`/`+0x108`, the same pair `RenderManager_Construct` seeds from a
sub-720/720-and-above split) and conditionally calls `0x0027cd60`, but nothing
pins its exact purpose beyond "runs once before each eye's draws".

**What this did not establish, and does now (below): where `+0x630` gets
populated.** No `stw` with a literal `0x630` offset appears anywhere in the
*render layer's own address range* - correctly so, per the enqueue idiom
below, since every confirmed call site lives outside that range. The open
question this left - "no sort in the dispatch/flush step, on whatever list
ends up in this array" not yet showing whether the array holds every
transparent draw, or whether a sort happens on the enqueue side before an
entry lands in it - is answered by the idiom's own key computation: yes, a
sort-relevant term (the depth bits) is computed at enqueue time, before the
entry ever reaches the array this section describes.

### The enqueue idiom, and what the `+0x04` key encodes

**2026-09-04.** Found by getting xrefs to `RenderManager_CreateInstance`'s own
singleton slot (`render_globals + 0x14`, itself reached through
`PTR_DAT_008b3d00`) and, separately, by searching the whole binary (not just
the render layer's address range) for the literal immediates `0x630` and
`0x44b0` together - the enqueue call is not a single shared function the way
Pulse's `Gfx_Enqueue` is. It is an **inlined idiom repeated at every producer
site**, one instantiation per caller, which is exactly why no literal `0x630`
store was ever found inside the render layer's own `0x00279xxx`-`0x002ecxxx`
range: none of these sites live there.

Confirmed on the same object `RenderManager_Construct` builds, not a
coincidentally-shaped struct: two of the five sites below
(`0x000ba268`, `0x000a3c38`) read `instance+0x624` and index into
`instance + count*0x40 + 0x3a0` - exactly the constructor's documented
"`0x40`-byte block copies into arrays at `+0x120` and `+0x3a0` indexed by
counters at `+0x620` and `+0x624`" (above).

The idiom, read identically at five independent call sites, takes one of two
observed forms. Four sites carry only a per-caller layer constant, defaulting
to zero depth unless a shared per-instance field overrides it:

```c
uint key = LAYER << 20;                   // a per-caller 12-bit constant: 0x300, 0x4d0, 0x570 or 0x5b0
if (instance->depth_override != 0xffffffff)  // instance+0x11c; 0xffffffff is "no override"
    key = (instance->depth_override & 0xfffff) | (LAYER << 20);
instance->queue[instance->count].key  = key;   // entry+0x04, at instance+0x630+count*8+4
instance->queue[instance->count].item = self;  // entry+0x00
instance->count += 1;                          // instance+0x44b0
```

(`0x00084e08` - twice, back to back, under layers `0x570` and `0x5b0` -
`0x000a3c38`, `0x000ba268`, all with layer `0x300`.)

The fifth site, `0x0012fba8`, computes its **own** depth term instead of
defaulting to zero, and only lets `instance+0x11c` override that when
present - direct evidence for "back-to-front", not just a name inherited
from Pulse's wording:

```c
float distance = /* a vector transform against a per-object matrix, then */ ...;
uint key = 0x4d300000;                                  // layer 0x4d3, zero depth by default
if (distance < DAT_008aa60c) {
    distance *= DAT_008aa600;
    if (distance <= DAT_008aa604)
        key = (~(uint)(long long)distance & 0xfffff) | 0x4d300000;  // complemented: farther -> smaller key
}
if (instance->depth_override != 0xffffffff)              // same override field as the other four sites
    key = (instance->depth_override & 0xfffff) | (key & 0xfff00000);
```

The bitwise complement (`~distance & 0xfffff`) is the mechanism: a larger
raw distance produces a *smaller* depth field, so the ascending `qsort`
(above) draws farther objects first - back-to-front, read directly rather
than assumed from Pulse's naming.

**This is Pulse's exact key layout - twelve bits of layer over twenty bits of
back-to-front depth** ([`mesh-draw.md`](../psp-pulse-usa/mesh-draw.md),
`Gfx_CompareQueueKeys`/`Gfx_Enqueue`), not just a similarly-shaped mechanism:
the bit widths match (`& 0xfffff` is 20 bits; the layer constants observed -
`0x300`, `0x4d0`, `0x4d3`, `0x570`, `0x5b0` - all sit in bits 20-31, a 12-bit
field) and the sentinel-for-no-override idiom (`0xffffffff`) matches Pulse's
own "a mesh batch set enqueues with its bare layer and no depth term at all"
default for the four sites that don't compute their own depth. **One layer
value overlaps Pulse's own**: `0x4d0` (`0x00109028`) shares its top byte
with `ExhaustFlare_Submit`'s key, `0x4d000000`. Pulse's own census
([above](#the-per-eye-draw-dispatch-and-what-it-says-about-sort-order))
shows a layer is a coarse family bucket, not a per-effect identity - 21,055
mesh nodes split across only two values - so this is read as a bucket
match, not a claim the two titles enqueue the identical effect: `0x00109028`
attributes to `MagstripWake.cpp` (below), and a magstrip's glowing wake
trail sharing an additive-glow bucket with an engine's exhaust flare is an
expected pairing, not a coincidence needing a stronger explanation.

`instance+0x11c` plays the same role in this key layout that
`display+0x1180` does in Pulse's `Gfx_Enqueue`
([`mesh-draw.md`](../psp-pulse-usa/mesh-draw.md#gfx_enqueue) - same
"replaces the key's low twenty bits, keeps the top twelve" shape, same
`0xffffffff` sentinel) - a per-frame depth override neither codebase has
identified the writer of. Same open question, one title each, not two
unrelated ones; both are tracked as still-open in the handover thread this
finding belongs to, not repeated here.

**82** - five independent sites decompiling to the same key-construction
shape (four identical, one a computed-depth variant that still shares the
override field and the bit layout), matched bit-for-bit against Pulse's
already-confirmed scheme. Capped by this page's static-reading ceiling (84);
would be higher with a runtime trace confirming a queued entry's key against
its visible draw order, which hasn't been attempted.

None of the five call sites is named - each is a large, otherwise-unread
function - but `scripts/ps3-toc.py map` (2026-09-04) narrows which
translation unit three of the five belong to. Byte-distance-to-the-nearest-
attributed-function is a weak signal on its own (a translation unit's *code*
and an unrelated one's can interleave); what's read here instead is each
site's own TOC-relative global-data slots against the confirmed
constructor's - the linker groups a translation unit's slots contiguously,
so a shared or immediately-adjacent slot is structural evidence, not
proximity:

- `0x00109028` reads `PTR_DAT_008a98c0`, which falls **inside**
  `MagstripWake.cpp`'s own slot range (`008a98b0`-`008a98dc`, confirmed
  directly via `PTR_s_MagstripWake_cpp_008a98d8`) - between its own slots,
  not merely adjacent, the strongest of the three. **`MagstripWake.cpp`**,
  not the more obvious first-guess `DebrisManager.cpp` its code address
  sits closer to; code address and TOC-slot address don't have to agree;
  here they didn't.
- `0x0012fba8` (the site that computes its own depth) reads
  `008aa5fc`-`008aa60c`, immediately after `BombManager_Construct`'s own
  slots (`008aa570`-`008aa5e4`, confirmed directly via
  `PTR_s_BombManager_cpp_008aa5dc`) - one contiguous run, no gap.
  **`BombManager.cpp`** - which itself constructs `DetonatorBomb`
  (`BombManager_Construct` calls `DetonatorBomb_Construct` directly), tying
  this to the same `DetonatorBomb.cpp` the `SortRoot` investigation named,
  though `DetonatorBomb.cpp`'s own attributed range (`0x00134b48`) is a
  separate file, well past this site - a bomb's blast computing its own
  back-to-front depth fits `BombManager.cpp` owning the effect, not
  necessarily `DetonatorBomb` itself.
- `0x000a3c38` (the matrix-stack-push site) reads `008a7cc4`-`008a7d04`,
  immediately after `Camera.cpp`'s own confirmed range
  (`008a7c78`-`008a7cc0`, via `PTR_s_Camera_cpp_008a7cb4`) - one contiguous
  run. **`Camera.cpp`.** (It also reads `PTR_g_PhysicsHalfStep_008a7cb8`,
  the same slot `Camera.cpp`'s constructor reads for that name - consistent
  with the same TU, but `g_PhysicsHalfStep` is a game-wide global reached
  through a per-TU slot, so this corroborates rather than proves anything
  past the contiguous-range evidence on its own.)
- The other two do not resolve as cleanly. `0x000ba268`'s slots
  (`008a82a4`-`008a82b4`) fall in the gap *between* `WorldManager.cpp`'s own
  range (ending `008a8274`) and `AIManager.cpp`'s (starting `008a82c0`) -
  neither attributed file covers it, which is itself informative about
  `map`'s own coverage: a third translation unit with no constructor call
  of its own sits silently in that gap. `0x00084e08`'s slots
  (`008a75b8`-`008a76bc`) overlap the same broad neighbourhood as both
  `Demo_RaceManager.cpp`'s and `HUD.cpp`'s without landing inside or
  adjacent to either specifically, and its own content (a zone-list lookup
  against `RaceManager_GetInstance()` and a ratio between two fields of a
  zone record) doesn't disambiguate which file it belongs to - left
  unattributed.

Below 50 confidence for what *class* any of the five draws, file-level
attribution or not - the enqueue tail itself is read directly and is not in
question. **Still open**: where `instance+0x11c` gets an actual value - no
call site among the five computes one, it is only ever read. One real
constraint, found and worth recording so the next pass doesn't repeat the
search: `RenderManager_Construct` and `_ConstructComplete` both initialise
it to the `0xffffffff` sentinel itself (`li r0,-1` immediately precedes
`stw r0, 0x11c(r30)` in both, checked by disassembly rather than assumed
from the store alone), so at construction the override starts *disabled*,
the expected direction - something still has to write an actual depth
value to *enable* it, and nothing found so far does. A blind binary-wide
search for `stw ...,0x11c(...)` is not the way to find that writer -
`0x11c` is a near-universal stack-frame local-variable offset, and the
search returns hundreds of unrelated hits on `r1` (the stack pointer) for
one relevant hit on an object register; the same shape as the `0x00109028`
dead end
below, recorded so it isn't retried the same way.

### Runtime-verified: 118 real draws, two object families, no watchpoint support

**2026-08-26**, against a live race on Talon's Junction (RPCS3
`v0.0.42-19777-3be5aa99`, `PPU Decoder: Interpreter (static)` - the only mode
`Z0` breakpoints fire under, per
[rpcs3-debugger.md](../../../reverse-engineering/rpcs3-debugger.md)). A
breakpoint on `RenderManager_FlushDrawQueue` (`0x002d6300`) hit mid-race and
`r3` read back `0x321894d0`. From there, direct memory reads:

- `+0x44b0` (the draw count) was **118** - a real per-frame scene, not a
  handful of UI elements, which is evidence the array is the general draw
  queue and not something narrower.
- The first ten `+0x630` entries share **one** object pointer
  (`0x328d5170`) and **one** vtable (`0x00864cb8`), each with a different
  `extra` value - `82, 86, 87, 91, 96, 100, 101, 105, 106, 109`,
  monotonically increasing. One object submitting many draws with a rising
  per-entry parameter reads as a batch emitter (particles, glyphs, or
  similar), not a single mesh.
- The next six share **one** vtable (`0x00867e58`) but each has its own
  object pointer, evenly spaced **624 bytes** apart
  (`0x303cef60, 0x303cf1d0, 0x303cf440, ...`) - a fixed-stride array of
  distinct instances, consistent with a simple `for` loop over a typed
  array. Their `extra` fields also rise monotonically, in steps of exactly
  **2** (`0x58002a90, 92, 94, 96, 98, 9a`).
- **Both families' `extra` sequences are sorted ascending with no
  exceptions.** **2026-09-04: reread after finding
  `RenderManager_CompareQueueKeys` (above) - this reads as evidence *for* a
  sort, not against one.** Family 1's keys (`82`-`109`) and family 2's
  (`0x58002a90`-`0x58002a9a`) sit three orders of magnitude apart, and the
  small-key family occupies the low indices while the large-key family
  follows - global ascending order across two unrelated object families is
  exactly what a completed ascending sort on this key produces, not what
  independent per-family enqueue order would be expected to coincide into.
  Whether the breakpoint (at `RenderManager_FlushDrawQueue`'s entry,
  `0x002d6300`) caught the array before or after its own internal `qsort`
  call is not established either way from this trace alone, and does not
  need to be: the sort's existence is read straight off the decompile
  (above), not inferred from this data. The claim these two bullets
  supported - "no sort exists anywhere upstream of the dispatch loop" - is
  retracted regardless of which side of the `qsort` call this trace caught.
- Neither vtable's constructors sit in the render layer's own address range
  (`0x00864cb8`'s at `0x0016xxxx`/`0x0046xxxx`, `0x00867e58`'s at
  `0x0020xxxx`/`0x005ebxxx`) - both outside `0x00279xxx`-`0x002ecxxx`. Below
  50 confidence for what either class actually is, so neither is named; the
  addresses are recorded as a lead for whoever picks this up next, not a
  conclusion. It does corroborate the established pattern
  ([`SortRoot`](#what-was-deliberately-not-read),
  [`DetonatorBomb`](detonator-bomb.md)) that objects queue themselves with
  the render layer from gameplay-side code rather than the render layer
  owning them. **2026-09-04, confirmed as identity, not merely related**:
  a direct memory read of `0x008ab774` (the slot `FrontendRoot_Construct`,
  `0x00164270`, writes into its own object with `*param_1 =
  PTR_PTR_008ab774`) shows its own first word **is** `0x00864cb8` - the
  earlier "related through at least one indirection, not established as
  identity" hedge is resolved: this queued vtable *is* `FrontendRoot`'s.
  **84** - a raw read of static data, no TOC or decompiler trust involved.

  Its own vtable slot `0x1c` (the callback `RenderManager_FlushDrawQueue`
  dispatches, resolved through a further OPD indirection to `0x001635a0`)
  is not HUD drawing at all: it switches on the entry's own key
  (`param_3`, the same "extra" word the enqueue idiom writes) across five
  layer constants - `0x52`, `0x57`, `0x60`, `0x65`, `0x6a` (twelve-bit
  values, `>> 20`) - collapsing to **three code paths**: `0x52`/`0x65`
  share one (`0x13` apart), `0x57`/`0x6a` share another (also `0x13`
  apart), and `0x60` has its own. **All three write into both of
  `RenderManager`'s matrix-stack arrays**, `param_2 + *(param_2+0x624)*0x40
  + 0x3a0` and `param_2 + *(param_2+0x620)*0x40 + 0x120` - the exact fields
  and exact `0x40`-byte stride `RenderManager_Construct` already
  established, but **only at the current index**: no store to `+0x620` or
  `+0x624` appears anywhere in this function, both are read-only here, so
  this replaces the top-of-stack matrix rather than pushing a new one.

  **An unexplained numeric correspondence, recorded rather than leaned
  on**: the 2026-08-26 trace's ten observed `extra` values for the *other*
  queued family (`82, 86, 87, 91, 96, 100, 101, 105, 106, 109` - decimal)
  are `0x52, 0x56, 0x57, 0x5b, 0x60, 0x64, 0x65, 0x69, 0x6a, 0x6d` in hex,
  and this switch's five constants all appear in that set (alternating
  positions, not "the low five" - a slip an earlier pass here made).
  **This is not read as corroboration**: `FrontendRoot`'s own enqueue site
  was never found among the five in the idiom above, so there is no
  evidence it writes its key the same way (`layer << 20 | depth`) rather
  than some other encoding, and the trace's *other* family in the same
  capture was recorded as full 32-bit words (`0x58002a90`, not a
  right-shifted byte) - one capture recording two families two different
  ways is exactly the thing that would need explaining first. Whether
  `FrontendRoot`'s keys are these five values directly, or `layer << 20`
  values that happen to share these bytes, is the open question a located
  enqueue site would settle. The unhandled values aren't random either:
  four of the five handled constants have a companion exactly `+4` that
  falls through this switch as a no-op (`0x56`, `0x5b`, `0x64`, `0x69`;
  `0x6a`'s is `0x6d`, `+3`) - a second structured pattern alongside the
  `0x13`-apart pairing above, recorded as a lead rather than explained.

  Read as a **layer-boundary matrix swap, not a push**: `FrontendRoot`
  enqueues itself once per layer transition so that, when the sorted queue
  reaches that point, its own "draw" callback fires and replaces whatever
  matrix is currently at the top of the stack with the one the *next*
  layer's real draws need (e.g. an orthographic pass after a perspective
  one) - a transform-stack marker riding the same queue, not a HUD element
  drawing itself. **65**, `_q`: the mechanism (write into the confirmed
  matrix-stack arrays, keyed by layer) is read directly; the purpose (why a
  swap specifically, and which layer needs which matrix) is inference, not
  read. Named `FrontendRoot_ApplyLayerMatrix_q` (`0x001635a0`) accordingly.
  **What this does not establish**: whether anything else in the queue draws a
  HUD element - the second live-observed object family (`0x00867e58`) is
  still unidentified, and the 2026-08-26 trace inspected 16 of that
  frame's 118 entries, so the `oag_render::mesh::rcs`/`LAYER_DEFAULT`
  question this lead was chasing narrows, it doesn't close.
- A `Z2` (write watchpoint) armed on `instance+0x630` to try to catch the
  enqueue site's own PC got back an **empty reply**, not `OK` - RPCS3's GDB
  stub does not implement write watchpoints on this build. Recorded because
  the toolchain page only says `Z0` was measured; `Z2` now is too, and it is
  a dead end, not an unknown.

The enqueue site itself is still unread - this corroborates "no sort" with
real per-frame data rather than closing the open see-through-surfaces
depth-sort question outright.

## The lineage result: HD keeps Pulse's importer-per-class layout

This is the finding worth the sweep.

Pulse registers **46 `.vex` classes**, and
[vex.md](../../../formats/vex.md#node-types) establishes how: *"one translation
unit per class, in static-initialiser link order"*, alphabetical by class name
with monotonically increasing method-table addresses.

HD ships **41 importer translation units** - 40 `<Class>_Importer.cpp` plus
`Mesh_Importer.ps3.cpp` - and their first functions run in **two alphabetical
runs of increasing address**: `AnimTransform` `0x002961d8` through `WOSpot`
`0x002e12e8`, then `Airbrake` `0x002e51c8` through `EngineFire` `0x002eb6f8`.
Same organising principle, same link-order signature, one console generation
across. HD's scene layer is not a rewrite.

**The one importer with a platform suffix is `Mesh`**, and that is exactly the
node whose payload [left the `.vex` file](../../../formats/hd-status.md) for a
PS3 container of its own. The executable's own file naming agrees with what the
asset probe found from the other side.

Class-by-class against Pulse's table, by name:

| | Classes |
| --- | --- |
| **Both** (38 HD importers, 42 Pulse table rows) | `World` `Transform` `AnimTransform` `Lod` `Camera` `GridCamera` `Mesh` `Texture` `AmbientLight` `DirectionalLight` `Lensflare` `Shadow`¹ `TrackData`² `Section` `StartPosition` `SpeedupPad` `WeaponPad` `Collision`³ `ShipCollisionFx` `Airbrake` `EngineFlare` `ShipMuzzle` `EngineFire` `ShipCannonFlash` `ParticleSystem` `Trail` `Quake` `Blob` `Skycube` `CloudCube` `CloudGroup` `WeatherPos` `Sound` `SoundCone` `Speaker` `WOSpot` `WOPoint` `AnimationTrigger` |
| **Pulse only** (13) | `MeshNode_Ghost` `NurbsSurface` `PointLight` `Dynamic Point Light` `Dynamic Shadow Occluder`¹ `gate` `textureBlob` `fogCube` `sea` `seareflect` `seaweed` `exitglow`⁴ `Unused 1` |
| **HD only** (3) | `ShipWingtipVortices` `ShipAbsorbNode`⁴ `Default`⁵ |

Pulse's table has **55 rows**, 46 of which have a registration site; the counts
above are against all 55.

¹ HD's one `Shadow_Importer.cpp` is mapped to Pulse's `shadow` `0x3cb`. It could
instead be `Dynamic Shadow Occluder` `0x3c3`, or cover both. Not established.
² Pulse calls the class `WO Track`; the payload is the same per hd-status.
³ Pulse has five separate collision classes (`Floor` `Wall` `Mag Floor` `Cage`
`Reset`); HD has one `Collision_Importer.cpp`. Whether that is a merge or just
a shared translation unit is **not** established here.
⁴ `ShipAbsorbNode` has no Pulse counterpart by name, and Pulse's `exitglow`
`0x3e4` has no HD counterpart by name. They are the obvious pair and are
**deliberately not** paired here - nothing was read that connects them, and
`exitglow` is authored 13 times on `16_Track` with no registration site, so
guessing at it would close a question that is still open.
⁵ A fallback importer, matching Pulse's `Default` class-table behaviour.

Two consequences worth carrying forward:

- **`engine_fire` and `cannon_flash` get real importers in HD.** Both are
  authored in Pulse's `.vex` files and **neither has a registration site in
  Pulse** - the open question
  [exhaust.md](../psp-pulse-usa/exhaust.md) records. HD registering them is not
  proof of what Pulse does with them, but it does say the classes were real to
  the studio rather than dead authoring, and HD is where their handler can be
  read.
- **The sea and weather-geometry classes are gone**, along with `fogCube` and
  the second point-light pair. `FxWeather.cpp` and `FxScreenMist.cpp` exist
  instead, so weather moved from scene nodes into full-screen effects.

The mapping is **by name**, so it is an inference, not a measurement: no HD
class ID has been read out of this binary, and Pure already showed that
[the class-ID space gets renumbered](../../../formats/pure-status.md#the-class-id-space-is-renumbered)
between titles. **75** for the table as a whole; the two consequences above
inherit that and no more.

## Shaders are in exactly two places, and neither is a file type on the disc

The disc has **no shader file type at all** - the
[extension census](../../../formats/hd-status.md#the-headline) runs `.gtf`,
`.rcsmaterial`, `.xml`, `.vex`, `.rcsmodel`, `.pob`, `.bnk`, `.bik`, `.mp3`,
`.fnt`, `.probes`, `.pvs`, `.nnt` and stops. There is no `.vpo`, `.fpo`, `.cgb`
or shader archive, and `EBOOT.elf` names none. So every piece of compiled RSX
microcode HD runs is in one of two containers, and this section is what the
executable says about each.

### 1. Per-surface: the `.rcsmaterial`, which the executable barely names

[rcsmodel.md](../../../formats/rcsmodel.md#the-rcsmaterial-was-the-leading-hypothesis-and-it-is-not-the-answer)
established from the file side that a `.rcsmaterial` is a compiled RSX shader
container - `SHO` blocks, a hashed parameter table, microcode - and that a
`.rcsmodel`'s string pool names one per material. The executable side agrees and
adds one thing.

**`EBOOT.elf` contains no `.rcsmaterial` extension string**, unlike `.rcsmodel`
which it has twice and appends by hand. Every material the executable knows
about it names in full, and there are **exactly 11**, all the same file per
circuit:

```text
data/environments/01_vineta_k/fe/materials/cf_fetracks.rcsmaterial
data/environments/15_anulpha_pass/fe/materials/cf_fetracks.rcsmaterial
...  11 in total, one per front-end circuit
```

They are not referenced from code. They are **field 6 of a 7-pointer record**
in `.data`, stride `0x1c`, the array starting at `0x00924314`:

| Field | Example (`01_Vineta_K`) |
| --- | --- |
| `+0x00` | `Data/Environments/01_Vineta_K/fe/TrackSelectEmblem.gtf` |
| `+0x04` | `Data/Environments/01_Vineta_K/fe/TrackSelectEmblem_fury.gtf` |
| `+0x08` | `Data/Environments/01_Vineta_K/fe/Preview.bik` |
| `+0x0c` | `Data/Environments/01_Vineta_K/fe/track01.vex` |
| `+0x10` | `Data/Environments/01_Vineta_K/fe/track01.rcsmodel` |
| `+0x14` | `data/environments/01_vineta_k/fe/materials/cf_fetracks.rcsmaterial` |
| `+0x18` | `data/environments/01_vineta_k/fe/fe_grad.gtf` |

That is the **track-select record**: an emblem, its Fury variant, a preview
video, and the scene/geometry/material/gradient set for the little rotating
circuit model. **88** - the stride is confirmed by eleven consecutive records
and the field roles are the filenames' own.

Its consequence for the format work is the pairing at `+0x10` and `+0x14`: a
`.rcsmodel` and a `.rcsmaterial` **named side by side, as separate assets**.
Everywhere else the material comes out of the model's own string pool, so this
is the one place the binding is visible from outside the file - and it is a
path pair, not an index. The lowercase/mixed-case split (`Data/...` for the
model, `data/...` for the material) says the two were added by different hands,
which fits a material system bolted to an existing model pipeline.

### 2. Engine-owned: 121 shader programs named by the executable

`EBOOT.elf` carries **121 distinct `_vp`/`_fp` program names** in TOC slots -
`FunkLayerBloomGate_fp` at `0x007b1318` is reached from the pointer array at
`0x008b73c4`, which runs the whole `FunkLayerBloom` set back to back. So each
name is a TOC-addressed key that binder code loads with `lwz rX,disp(r2)`.

| Group | Count | Examples |
| ---: | ---: | --- |
| `FunkLayer*` | 32 | the post chain - see below |
| `RadioHead*` | 31 | `RadioHead0`..`RadioHead12`, plus `RadioHead5_point/line/quad/texture` and `SPU_RadioHead` |
| `downsample*` | 11 | `downsamplescaleaddfeedbackcorrection_fp`, `downsamplescaleanaglyph_fp`, `downsamplescalefiltercolourscale_fp` |
| `FEBackgroundAnim*` | 9 | including `FEBackgroundAnimFuryWave` and `FEBackgroundAnimFuryBlend` |
| `psys_*` | 6 | `psys_lit`, `psys_normal`, `psys_simplegeom` |
| `Live*` | 5 | `LiveStencilShadow`, `LiveGeometryPrimitives{,Constant,Specular}` |
| `HUD*`, `PrimList*` | 8 | `HUDBoostTextured`, `HUDShockTextured`, `PrimListUTU/UUU` |
| singles | 19 | `LensFlare`, `engineflare`, `trail`, `Shockwave`, `MagStripArc`, `FatLine`, `cloudshader`, `lineshader`, `liveRsxPanXForm_CubemapConvolve_fp`, `liveRsxPanXForm_CubemapToDualParaboloide_fp` |

**The `FunkLayer*` set is the post-processing chain**, and it is the answer to a
question the rest of this page had to leave open - there is no `Bloom.cpp`
because bloom is not a class:

- **Bloom, 8 programs**: `FunkLayerBloom_vp`, `FunkLayerBloomGate_fp`,
  `FunkLayerBloomDownsample_fp`, `FunkLayerBloomBlurVertical_fp`,
  `FunkLayerBloomBlurHorizontal_fp`, `FunkLayerBloomRadial_vp/_fp`,
  `FunkLayerBloomRadialGate_fp`. A gate, a downsample, a separable two-pass
  blur, and a radial variant with its own gate.
- **Depth of field, 7 programs**: `FunkLayerDof_fp`, `FunkLayerDofKernel_fp`,
  `FunkLayerDofCopy_fp`, and `FunkLayerDofBlurAvg/Avg2/Min/Min2_fp`.
- **The rest**: `FunkLayerZoom`, `FunkLayerCorruption`, `FunkLayerColour2d`,
  `FunkLayerReplayBar`, `FunkLayerCopy/CopyAlpha/CopyBlend`,
  `FunkLayerBlendBuffer`, `FunkLayerFx_vp`/`FunkLayerFxUv_vp`,
  `FunkLayerDebugLightRender`.

Three of these land directly on this project's open work.
[Bloom](../../../rendering/README.md) is an unchecked M6 roadmap item and this
names its exact pass structure. `psys_lit`/`psys_normal`/`psys_simplegeom` are a
three-way split of the particle shading that
[`oag_render::psys`](../../../../crates/render/src/psys.wgsl) currently does one
way. And `liveRsxPanXForm_CubemapConvolve_fp` plus
`liveRsxPanXForm_CubemapToDualParaboloide_fp` are almost certainly what the
disc's 28 `.probes` files are produced or consumed by, which is the first thread
anyone has on that format.

### Where the microcode is: measured, and it is the `.rcsmaterial`'s own container

This started as an inference at 75 - no shader file type on the disc, so the
bytes must be in `EBOOT.elf` - and was then **measured**, by following the
registry rather than by looking for blobs.

`ShaderRegistry_Find` (`0x005cd728`) is a case-insensitive linked-list walk:

```c
undefined4 *ShaderRegistry_Find(char *name)
{
  for (node = *g_ShaderRegistryHead; node; node = node[3])
    if (strcasecmp(name, (char *)node[0]) == 0)
      return node + 2;
  ...                                   // fall back to a shared empty slot
}
```

and `ShaderRegistry_Register` (`0x005cd6d8`) is the node constructor that fills
it in, one static initialiser per program:

```c
node[0] = name;      // the "FunkLayerBloom_vp" string
node[1] = payload;   // <-- the second datum
node[2] = 0;         // resolved lazily; Find returns &node[2]
node[3] = head; head = node;
```

**`node[1]` is the microcode.** Reading `r4` and `r5` at every
`bl ShaderRegistry_Register` in the whole image - not just in the call sites
Ghidra lists - recovers **62 registration sites, 62 distinct names, 62 distinct
payload pointers, and all 62 payloads begin `53 48 4f 08`, `"SHO\x08"`.** That is
the same `SHO` block
[rcsmodel.md](../../../formats/rcsmodel.md#the-rcsmaterial-was-the-leading-hypothesis-and-it-is-not-the-answer)
found inside a `.rcsmaterial`. So HD has **one shader container, used in two
places**: per-surface material shaders ship in `.rcsmaterial` files, and
engine-owned effect shaders are the same records linked into the executable.

**`ShaderRegistry_LoadAll` (`0x005cd990`) confirms the pairing a second way**,
independently of any call site. It walks the list and, for each node, builds an
object *from* `node[1]` and stores it in `node[2]`:

```c
for (node = *g_ShaderRegistryHead; node; node = node[3]) {
    build(&tmp, node[1], 0,0,0,0);      // from the SHO block
    ... refcount swap ...
    node[2] = tmp;                       // what Find hands out
}
```

`ShaderRegistry_UnloadAll` (`0x005cd8c0`) is its inverse: same walk, release
`node[2]`, set it to zero. So the blob is the *source* and `node[2]` the built
program, which is why `Register` leaves `node[2]` null - the registry is
declared at static-init time and compiled in one pass later.

### The accounting closes exactly, and one part of it does not

The image holds **126 `SHO` blocks**. Every one has `0x08` as its fourth byte;
there is no `SHO\x0?` variant anywhere at four-byte alignment. **124 of them lie
in one contiguous run**, `0x00927800`-`0x00936d80`, spaced on `0x80` boundaries.

That 124 is exact:

| | Count |
| --- | ---: |
| `_vp`/`_fp` program names | 121 |
| `accuview`, `quincunx`, `quincunxalt` - the anti-aliasing modes, the only registered names with no stage suffix | 3 |
| **Names total** | **124** |
| **`SHO` blocks in the shader run** | **124** |

The two blocks outside that run, at `0x00765f40` and `0x00766090`, are in
`.rodata` among the strings, are outside the cluster and outside the count, and
are **not** shader blobs.

**What does not close is which name goes with which blob.** 62 pairings are
measured; the other 62 names and the other 62 blobs are matched only by that
count. The obvious explanation is wrong and was checked: `0x005cd700` is the
second constructor of the pair and **nothing in the image branches to it** - a
whole-image `bl` scan finds zero sites, agreeing with Ghidra. So the remaining
62 register by some route a direct-branch scan does not see, and **what that
route is, is open**. The unpaired set is coherent rather than random - all of
`RadioHead*` and `SPU_RadioHead*`, all `psys_*`, all `FEBackgroundAnim*`, both
`HUD*`, plus `LensFlare`, `MagStripArc`, `engineflare`, `cloudshader`,
`lineshader` and `FatLine` - which is a hint that they share a mechanism, not
evidence of one.

`FunkLayerBloom_vp`'s block at `0x0092ec80`, and why the reading is not just
magic-matching:

```text
+0x00  53484f08                    'SHO', 8
+0x0c  u16 0x0002                  parameter count
+0x12  u16 0x0028                  parameter table offset
+0x28  083fdb95 0204 0001 01d3 ffff
+0x34  15f68309 0204 0001 01d2 ffff
```

Two entries at a 12-byte stride, `(name hash, u16 type, u16 count, u16 register,
0xffff)` - **exactly the binding record `rcsmodel.md` describes for a
`.rcsmaterial`**, and exactly what the binder at `0x003af980` walks: it reads the
count from `+0x0c`, the table offset from `+0x12`, steps 12 bytes, and compares
the first word against the **complement** of a hash computed from the parameter
name. Header fields, record layout and consuming code all agree, and the
declared count of 2 matches the two hash-shaped words present. **90.**

The residual doubt is not about the container. It is that no microcode has been
disassembled, so nothing here says what any program *does* - only where it is
and how it is addressed.

**`0x003b31c8` is not a binder** (corrected 2026-08-18; see
[the name hash](#the-name-hash-is-crc-32) below). It is the Detonator scoring
subsystem's static init/teardown pair - `param_1 == 1` constructs and
`param_1 == 0` tears down - and it registers `bomb_score`, `multiplier`,
`bomb_hit_score`, `bomb_increment`, `Data/XML/DetonatorScoring.xml`,
`DetonatorScoring` and `Bullet`. It reached this list because it sits in the same
address range and calls the same registry; nothing was read from it before.

**A warning for whoever disassembles one.** The binder function is at `0x003af980`,
**above `0x0032d5e0`**, and the decompiler output there is
actively misleading rather than merely wrong: floats loaded from the TOC render
as string-pointer variables, so arithmetic reads as `(float)PTR_s_Emp__d_008a754c`.
Use `scripts/ps3-toc.py resolve` on every constant in that range.

### The block, framed whole

**Read 2026-08-18 with [`scripts/ps3-sho.py`](../../../../scripts/ps3-sho.py),
which is where every number below comes from.** The block is a descriptor with
three tables and the program's own data behind them:

```text
+0x00  'SHO', 8
+0x04  u32   1 for a fragment program, 0 for a vertex one
+0x08  u16   2 on all 124
+0x0a  u16   attribute count
+0x0c  u16   parameter count
+0x0e  u16   sampler count
+0x10  u16   attribute table offset, 0x18 on all 124
+0x12  u16   parameter table offset
+0x14  u16   sampler table offset
+0x16  u16   where the program data begins
```

| Table | Stride | Record |
| --- | ---: | --- |
| Attributes | 8 | `(name hash, attribute slot)` |
| Parameters | 12 | `(name hash, u16 type, u16 count, u16 vertex register, u16 fragment slot)` |
| Samplers | 8 | `(name hash, texture unit)` |

**Confidence 90, and the framing is checked rather than asserted.** Each table
starts exactly where the previous one ends - `params == attributes + 8*count`
and `samplers == params + 12*count` - and that holds on **all 124 blocks, with
zero exceptions**. [`vex-classes.md`](vex-classes.md) is why the check is there:
a one-field rotation of a record can be self-consistent and wrong, so a layout
that only *reads plausibly* is not read at all.

`+0x04` is the program kind and not something correlated with it: on all 124,
the word is 1 exactly when the block declares no attributes. **75 fragment
programs, 49 vertex.** The parameter record's last two fields are the same
either/or - one is always `0xffff` - which is RSX's own split: a vertex
program's constants live in the constant file and a fragment program's are
patched into its own microcode. Type is `0x0200 | components` and `count` is
rows, so a 4x4 matrix declares `(0x0204, 4)`.

**The microcode is inline, at `+0x16`**, preceded by a block of embedded float
constants. `FunkLayerBloom_vp`'s begins at `0x0092ffe0` with `00031c6c
005c6055 0186c083 60407ffc` - four-word NV40 vertex instructions. This page
previously said only that no microcode had been disassembled. **It now is**:
[`scripts/ps3-microcode.py`](../../../../scripts/ps3-microcode.py) decodes both
program kinds, in the executable and in a `.rcsmaterial` alike, against the
NV40 instruction encodings in Mesa's nouveau headers. Three container facts
were established empirically on the way and are recorded in the script's own
header with the checks that pinned them: vertex instructions are stored as
four big-endian dwords in the spec's own order (all 24 permutations scored,
identity wins at 1.00 with one END bit, on the last instruction); fragment
instructions store each dword's 16-bit halves swapped; and a vertex program's
`c[N]` is the parameter table's register `N + 256` (the table binds `viewProj`
to c256 and the code multiplies the position by `c[0]..c[3]`).

### 29 parameter, attribute and sampler names, by preimage

The hash is `~crc32` ([below](#the-name-hash-is-crc-32)), so a candidate that
lands is a **preimage** rather than a resemblance. A previous sweep of ~2,000
names over thirteen `.rcsmaterial` hashes matched none; sweeping the executable's
own 124 blocks instead - 34 attributes, 111 parameters, 20 samplers - matched
29 across five sweeps of about 780,000 candidates in total. At that scale the
expected number of false hits is under 0.01.

| Kind | Names |
| --- | --- |
| Parameter | `viewProj` `worldView` `proj` `worldViewProj` `kWorldViewProj` `fogFactors` `time` `scale` `size` `offsets` `colourRamp` |
| Attribute | `position` `normal` `inPos` `inCol` `inUV` `uv` `colour` `color` `offset` `fAlpha` `fLength` |
| Sampler | `texture` `texture0` `texture1` `texture2` `diffuseSampler` `blurSampler` `depthSampler` `srcTexture` `dstTexture` `sourceImage` `texSampler` |

**Every one also agrees in shape with what the table declares for it**, which is
the second and independent check. The four matrices are the four names ending
`Proj`/`View` and each declares `count 4`; `time`, `scale` and `size` are
scalars; the eleven sampler names are the ones that carry a texture unit and no
register at all. The sharpest of the three is the attribute slots: `inPos` sits
at slot 0, `inCol` and `colour` at slot 3, and `inUV` at slot 8 - **position,
COLOR0 and TEXCOORD0 in NV40's own conventional attribute numbering**, which
nothing in the wordlist knew about.

**The attribute names are the same namespace `.rcsmodel` uses.** `position`
(`0xb9d31b0a`) and `normal` (`0xde7a971b`) are the hashes a chunk's vertex
declaration writes, byte for byte -
[`rcsmodel.md`](../../../formats/rcsmodel.md). So a mesh's declared attributes
and a program's declared inputs are matched by name hash, which is how a stride
authored per chunk binds to a program compiled once. Five hashes remain unnamed
above ten uses and are listed by `ps3-sho.py census`; `0x1aaf7631`, the
13,485-use attribute `rcsmodel.md` cannot name, appears in **no** block's
attribute table.

### The engine's own parameter table, read from its initialiser (2026-08-24)

The preimage sweep above recovered 29 names from 780,000 candidates. **The
executable hands over 81 of them for nothing**, in slot order, with their
shapes - and it settles what a wordlist never could: *which* parameters the
engine supplies and where their values come from.

`Shader_InitEngineParams` (`0x003f1300`) runs once and does two things.
First it hashes eight **technique** names - `ZAlphaOnly`, `Ambient`,
`AmbientShadow`, `Static`, `StaticQuake`, `RigidBody`, `SunOcclusionLightmap`,
`SunOcclusionVertex` - into eight bare `u32`s at `base + 0x4a20 .. 0x4a3c`.
Then it initialises **81 parameter entries** at `base + 0x4000 + i * 0x20`,
each through one call:

```c
Shader_InitParamEntry(base + i * 0x20, ~Crc32_HashString(name), 0xc000,
                      name, 0, vec4_count);
```

and `Shader_InitParamEntry` (`0x005d4cf8`) is nine assignments:

```text
+0x00  the ~crc32 hash        +0x10  0
+0x04  0xc000, the kind       +0x14  0
+0x08  the name string        +0x18  the value pointer, 0 until a frame fills it
+0x0c  0                      +0x1c  how many vec4s
```

**This is the `.rcsmodel` material record's own 0x20-byte parameter entry**
([engine-flare.md](engine-flare.md)), built in memory with the name attached.
The framing closes arithmetically: the entries run `0x4000` to `0x4a20`, which
is `0xa20 / 0x20` = **exactly 81**, and `0x4a20` is where the eight technique
hashes begin - no gap and no overlap.

The kind `0xc000` has bit 15 set, which is the bit
`Material_SetInstanceParamPointer` tests to skip samplers - so the per-instance
binder never writes an engine entry, and the two sources cannot collide.

| | | | |
| --- | --- | --- | --- |
| 0 `time` | 21 `pointLight0Falloff` | 42 `shadowMapTexSize` | 63 `zoneTexOuterNearest` |
| 1 `viewProj` | 22 `textureSpot0PositionWorldSpace` | 43 `quakePointA` | 64 `zoneTexVis` |
| 2 `view` | 23 `textureSpot0Proj` | 44 `quakePointB` | 65 `zoneAnisoPalette` |
| 3 `world` | 24 `textureSpot0Tex` | 45 `quakeOffset` | 66 `zoneAnisoPaletteOuter` |
| 4 `eyePositionWorldSpace` | 25 `textureSpot0ShadowTex` | 46 `quakeTrackUpNormal` | 67 `zoneAnisoPower` |
| 5 `positionBias` | 26 `textureSpot0Colour` | 47 `distortion` | 68 `GradientColour` |
| 6 `positionScale` | 27 `textureSpot0Falloff` | 48 `refractProject` | 69 `GradientColour1` |
| 7 `fogColour` | 28 `textureSpot1PositionWorldSpace` | 49 `reflectProject` | 70 `GradientColour2` |
| 8 `paraboloidReflectionTex` | 29 `textureSpot1Proj` | 50 `screenSpaceRefractionTex` | 71 `GradientColour3` |
| 9 `paraboloidIblTex` | 30 `textureSpot1Tex` | 51 `screenSpaceReflectionTex` | 72 `iblScalePower` |
| 10 `constantAmbientColour` | 31 `textureSpot1ShadowTex` | 52 `zoneColourTint` | 73 `ambientShadowMatrix` |
| 11 `falseLightDirectionPower` | 32 `textureSpot1Colour` | 53 `zoneEffectInner` | 74 `ambientShadowBlendFactor` |
| 12 `prelitScaleSpecular` | 33 `textureSpot1Falloff` | 54 `zoneEffectOuter` | 75 `ambientShadowTex` |
| 13 `prelitBias` | 34 `textureSpot2PositionWorldSpace` | 55 `zoneBaseInner` | 76 `globalAlphaScaler` |
| 14 `directionalLight0DirectionWorldSpace` | 35 `textureSpot2Proj` | 56 `zoneBaseOuter` | 77 `auroraBrightness` |
| 15 `directionalLight0Colour` | 36 `textureSpot2Tex` | 57 `zoneBaseAltInner` | 78 `auroraOffset` |
| 16 `directionalLight0Proj` | 37 `textureSpot2ShadowTex` | 58 `zoneBaseAltOuter` | 79 `auroraColour` |
| 17 `directionalLight0ShadowTex` | 38 `textureSpot2Colour` | 59 `zoneOrigin` | 80 `engineTrail` |
| 18 `directionalLight0LightmapTex` | 39 `textureSpot2Falloff` | 60 `zoneTexInner` | |
| 19 `pointLight0PositionWorldSpace` | 40 `shadowMatrix` | 61 `zoneTexOuter` | |
| 20 `pointLight0Colour` | 41 `shadowMapTex` | 62 `zoneTexInnerNearest` | |

**Ghidra shows the wrong strings for every one of them.** `0x003f1300` is above
the `0x32d5e0` TOC break [memory.md](memory.md) documents, so its decompilation
names sound banks and `.vex` paths. The list above is
[`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py) resolving the function's
own `lwz rX,disp(r2)` against its own OPD TOC, and the order is instruction
order.

#### Nine cross-checks, from two other subsystems

The slot order is not taken on trust. Two draw-state builders read on
[engine-trail.md](engine-trail.md) - `Trail_BuildDrawState` (`0x002e30d8`) and
its sibling at `0x002a51e8` - each write live values into this table by fixed
offset, and both the offset *and* the vec4 count have to agree with the name:

| Offset written | Slot | Name | vec4s written | vec4s the initialiser declares |
| --- | ---: | --- | ---: | ---: |
| `+0x18` | 0 | `time` | 1 | 1 |
| `+0x38` | 1 | `viewProj` | 4 | 4 |
| `+0x58` | 2 | `view` | 4 | 4 |
| `+0x78` | 3 | `world` | 4 | 4 |
| `+0x98` | 4 | `eyePositionWorldSpace` | 1 | 1 |
| `+0xf8` | 7 | `fogColour` | 1 | 1 |
| `+0x158` | 10 | `constantAmbientColour` | 1 | 1 |
| `+0xa18` | 80 | `engineTrail` | 1 | 1 |

Three matrices at four vec4s each and five scalars/colours at one, landing on
the three matrix-shaped names and five scalar-shaped names, in order. The
eighth row is the sharpest: **`~crc32("engineTrail")` is `0xbb48e390`**, which
is the trail material's per-craft colour-mix parameter, recovered independently
on [engine-trail.md](engine-trail.md) as the Fury-skin flag and previously
carrying no preimage at all. A ninth check comes from the flame:
`~crc32("globalAlphaScaler")` is `0x4c13d3af`, the `float2` engine-flare.md
listed as unresolved, and it sits at slot 76.

**Confidence 92.** The table is read out of its own initialiser rather than
guessed; the framing closes on `0xa20 / 0x20`; and the slot order is confirmed
from a different subsystem by eight offset-and-shape agreements plus two
recovered preimages that were open questions on another page.

#### What supplies a material's declared parameter

A `.rcsmaterial` block declares parameters by hash and says nothing about where
their values come from. Two sources answer, both bound into the same draw
state - the model's own instance array at `+0xc4` and this table at `+0xd8` -
and both are arrays of the identical 0x20-byte entry. Checking the partition
against the two materials that are fully read:

- `flame_test` block #1 declares seven parameters. Six (`power1`, `scale1`,
  `min1`, `0x92fc84bf`, `0x17d9b3d3`, `Speed`) are in every craft's
  `engineflare.rcsmodel`. The seventh is `time`, which **no** craft authors and
  which is engine slot 0. No gap, no overlap.
- `hd_enginetrail_bluered` declares three. `0xe296b1ed` and `TrailSpeed` are
  model-authored; `engineTrail` is authored `0.0` by the model **and** is engine
  slot 80, which `Trail_BuildDrawState` overwrites per craft. So the model
  authors a default and the engine overrides by name.

That a parameter resolves against this table **by hash** is the one link read
from the shape of the data rather than from the resolving code: the entries
carry the hash at `+0x00` in the same format the instance entries use, and both
arrays are bound into the same draw state. Everything downstream of it is read.
Confidence 85 on that link alone.

### What this says about fog, and what it does not

`fogFactors` is `~crc32` `0xd04a156c`, **one `float4`, in 9 blocks, and all 9
are vertex programs** - at registers c455 to c463, one per program. So
**HD's fog is computed per vertex and interpolated**, and its coefficients are a
single four-component constant.

**The curve is not read, and three things that look like it are not it.**

- **`%s.sections[%d].fogStart`, `fogLength` and `fogExponent`
  (`0x00788cc8`-`0x00788d08`) are not race fog.** Their neighbours in the same
  key block are `effectMode`, `fovy`, `duration`, `dofStart`, `dofStrength`,
  `dofFactor`, `focusStart` and `focusEnd`, and the `.cpp` immediately after
  them at `0x00788e58` is `BackgroundController_Item.cpp`. They are the front
  end's background camera flythrough - a per-section rig with depth of field.
  `0x00187010` names the indexed form and `0x00187400`/`0x001876f8` the flat
  one.
- **`%s.Lighting.Fog colour`, `Fog density`, `Alt Fog colour`, `Alt Fog
  density`, `Track Fog colour` and `Track Fog density`
  (`0x007b24d8`-`0x007b2568`), all named by `0x003d0b98`**, are a second,
  prefixed namespace over the same values the 33 `.envsettings` files write
  flat - the namespace [`envsettings.md`](../../../formats/envsettings.md) had
  open. A `Track Fog` pair exists that no file on the disc writes.
- **`Fog.Fog Density` itself** is named twice, at `0x00786260` (by `0x006909a8`)
  and `0x007b0630` (by `0x003a83d8` and `0x003a9520`, the same two functions
  that name `Lighting.Sky colour`).

None of that said whether the term is `exp`, `exp2`, linear or squared, and the
authored densities span `0.0003` to `0.03`, over which those candidates diverge
by more than the picture.

### The race fog curve is read, out of the circuit materials' own microcode

Read 2026-08-18 with [`scripts/ps3-microcode.py`](../../../../scripts/ps3-microcode.py).
The circuit's fog is **not** the `fogFactors` vertex path above - that constant
appears only in the executable's own 124 blocks (`FunkLayerBloom_vp` among
them, whose chain is a *power* curve for the front end). A circuit draws
through the `SHO` blocks in its own `.rcsmaterial` files, and those do it
**per fragment**:

```text
clouds.rcsmaterial, fragment block #3 (talons_junction):
@0x13  MUL R3.x, f[TC3].wwww, {..}.wwww      <- view depth * fogColour.w
@0x23  MUL R3.w, -R3.xxxx, R3.xxxx           <- negate and square
@0x2c  MUL R1.w, R3.wwww, {1.44269,..}.xxxx  <- * log2(e)
@0x2f  EX2_SAT R1.w, R1.wwww                 <- f = e^-(k*d)^2, saturated
@0x31  MAD R2.xyz, -{fog rgb}, R1.wwww, {fog rgb}
@0x33  MAD H2.xyz, R1.wwww, H2, R2           <- lerp(fog colour, lit, f)
```

So the curve is `f = exp(-(coefficient * view_depth)^2)` - squared
exponential - and the distance is **clip-space `w`**: the paired vertex block
writes `o[TC3]` as world position with `.w` copied from the position it just
projected, so the interpolant is view depth, not radial distance.

**The coefficient and the colour arrive together, and the parameter is named
by preimage.** Both inline constants are patched by the *same* material
parameter - the patch chain is `fslot -> u16 index at block+fslot -> offset
list -> 16-byte code slots`, and following it lands both on one `float4` whose
name hash `0x3dc31258` is `~crc32("fogColour")`. Two more preimages fell out
of the same tables: `positionScale`/`positionBias` (`0x9cc5ab3a`/`0xa4972b78`),
the dequantisation pair every circuit vertex program applies as `v * scale +
bias` - the shader interface confirming
[`rcsmodel.md`](../../../formats/rcsmodel.md)'s bias-and-scale reading from
the disc's other side - and `diffuse` (`0x515e298e`), sampler unit 0.

**The census, over every material of Talon's Junction**: 54 `.rcsmaterial`,
857 fragment programs, **657 declare a patched `fogColour` and 657 contain the
`log2(e)`-into-`EX2` chain** - and on the 20 blocks of
`diffuse_specular_v01.rcsmaterial` the two properties hold block-for-block,
an exact iff. The remainder are the depth/shadow-shaped variants that write
no colour at all.

**What is still not read**: how the engine fills `fogColour.w` from
`Fog.Fog Density`, and what selects the `Alternate` pair. `oag-render` now
draws an HD race on this curve with the authored density passed through
unscaled - `mesh_render::Fog::authored_exp2` says which halves are the disc's
and which are that reading - judged against an rpcs3 reference frame of the
same grid. Confidence 84 on the curve (the static-reading cap; the formula is
the microcode's own arithmetic), 60 on density-unscaled.

### `Fog.Fog Density`'s field offset is read; the PPU fill from there is not (2026-08-26)

`FUN_003a83d8`/`FUN_003a9520` are named above as the functions that register
`Fog.Fog Density` and `Lighting.Sky colour` by string. **Both carry their own
TOC, `0x008bd3c4`, not the global `0x008ad4d8` `scripts/ps3-toc.py toc`
resolves for the rest of the image** - so `decompile_function` on either
renders a completely different, entirely coherent, entirely wrong function:
every `PTR_DAT_008a70xx`/`PTR_s_..._008a70xx` symbol it prints is Ghidra
resolving the same `lwz rX,d(r2)` displacements against the wrong table, and
what comes out reads as a **Race End / split-screen HUD field
initialiser** - `Race End Save`, `Race End Photo`, `EndRace Results`, `Zone
Gold/Silver/Bronze Medal`, HUD XML paths - not fog or lighting at all. It is
the same failure mode as the `CCookie.cpp`/`SrcRel` trap already on this page
(HANDOVER.md, "the same defect has a second failure mode"), on the very
functions this page already cites by address, so the trap is worth naming
here rather than only in HANDOVER: **do not `decompile_function` these two
without resolving through the real TOC first**, or the picture that comes
back is a different, real, coherent struct - not garbage, which is what
makes it costly to notice.

Read correctly - `disassemble_function` plus `scripts/ps3-toc.py resolve
0x003a83d8 <displacement>` on each `lwz r5,d(r2)` that feeds a registration
call - the function is one long flat list of
`register(struct, struct+offset, name, type)` calls into a lazily-constructed
settings singleton (storage pointer at `0x008b6fb4`, first built by
`FUN_003a83d8` on `param_2 == 0`, `FUN_003a9520` returning the same
singleton). All 76 resolve cleanly, self-consistent with what this page
already knew about the `HDR and Bloom` keys (`Bloom adaption rate` lands on
`+0x52c`, `Tone adaption boost` on `+0x540`, `Tone maximum brightness` on
`+0x548` - exactly the offsets "The bloom chain, read pass by pass" above
already cites). The fog and lighting neighbourhood:

| Offset | Key | Registrar (type) |
| --- | --- | --- |
| `+0x4e0` | `Fog.Fog Color` | `FUN_005d3ec0` (0) |
| `+0x500` | `Fog.Fog Density` | `FUN_005d4418` (0) |
| `+0x4f0` | `Fog.Alternate Fog Color` | `FUN_005d3ec0` (0) |
| `+0x504` | `Fog.Alternate Fog Density` | `FUN_005d4418` (0) |

`+0x500` is a scalar (`FUN_005d4418`'s type-0 slot, the same registrar every
plain float on this table uses - `Tone adaption boost` among them), which at
least rules out `Fog.Fog Density` being anything but one number. Confidence
82 - the offsets are read off the real TOC and cross-checked against this
page's own prior, independently-sourced HDR/Bloom offsets, but nothing here
yet confirms the *type* tag's meaning from first principles.

**Still not read**: what copies `+0x500` from this singleton into the
`fogColour` shader constant's `w` component, and whether that copy scales it.
`get_xrefs_to` on the singleton's storage address (`0x008b6fb4`) finds
nothing - consistent with the consumer being itself a TOC-mismatched
function Ghidra cannot resolve the load for, the same defect one level
removed. Confidence on "unscaled" stays at 60, unmoved.

**Three negative results from chasing it (2026-08-26), worth recording so the
next pass does not repeat them.**

- **The hash never appears as a literal.** `search_byte_patterns` for
  `3D C3 12 58` (`fogColour`'s crc32, big-endian) finds nothing anywhere in
  the image. This is now *checked*, not merely the "no immediate hashes"
  claim carried forward - the binder genuinely resolves by string at runtime.
- **`FUN_005a2090` is slot resolution, not value fetch.** Traced from
  `downsamplescaleaddfeedback_fp`'s own parameter binder
  (`FUN_005e29c0`, disassembled end to end): passed a TOC-relative name
  string, it hashes and linear-scans a *program instance's own declaration
  table* (16-byte stride entries, count at `+0xc`, offset at `+0x12` of the
  table header) to find which slot that program declares the name at. It
  answers "where", never "what" - a dead end for finding the fog value
  itself, useful only for finding *other* programs' own parameter tables.
- **The value itself arrives as a float argument, not a global read.**
  `FUN_005e29c0`'s four resolved parameters (`scale`, `scaleFeedback`,
  `scaleAdd`, `fullscreenTintColour`) are bound from `f1`/`f2`/`f3` -
  register arguments the function receives, not a load from any singleton.
  So the pattern for a per-pass parameter wrapper is: the *caller* holds the
  current value and passes it in; the wrapper only resolves where to put it.
  If `fogColour` is patched the same way, the lead is **`fogColour`'s own
  wrapper's caller**, not a load of `0x008b6fb4` - the caller is where
  `+0x500` (or whatever reads it) would surface, if this pattern holds for
  materials the way it does for post passes. Not yet confirmed that
  materials use the same argument-passing wrapper shape rather than the
  fslot-offset direct patch `scripts/ps3-microcode.py`'s docstring describes
  - the two may be the same mechanism seen from different sides, or two
  different ones; that ambiguity is itself unresolved.

**An rpcs3 live read was considered and deliberately not started.** Reading
the patched constant in a loaded circuit's compiled microcode against
`+0x500`'s live value would raise confidence on "unscaled" for *that one
circuit at that moment*, but cannot by itself rule out a per-circuit
multiplier or the `Alternate` selector - it is evidence toward the existing
60, not a resolution of it. It also needs the runtime-to-static address
mapping ("The address space during a race is 480 MiB in six pieces",
[rpcs3-debugger.md](../../../reverse-engineering/rpcs3-debugger.md)) worked
out for `0x008b6fb4` specifically, a screen-keyed menu walk into a race, and
a GDB-stub session with its own trap list - a fresh session's worth of setup,
not a continuation of this one.

### `fogColour` is engine parameter slot 7; its value-pointer setter is not found (2026-08-26, continued)

The ambiguity the section above left open - whether materials patch by the
argument-passing wrapper shape or the fslot-offset direct patch - resolves in
favour of neither read standing alone: `fogColour` is **engine parameter slot
7** in the 81-entry table "The engine's own parameter table, read from its
initialiser" above already recovered (`Shader_InitEngineParams`,
`Shader_InitParamEntry`), which is a *third* mechanism from the same section -
material-declared parameters resolve against this shared table by hash rather
than always carrying their own value. Its entry sits at a concrete,
now-located address: `Shader_InitEngineParams`'s own `lwz r29,-0x54c0(r2)`
(resolved against its real TOC, `0x008bd3c4`, the same TOC as the two
functions above) loads a runtime-allocated struct pointer from static storage
`0x008b7f04` - four bytes short of the `0x008b7f40` name table the first pass
of this thread walked, the same shader-engine data region. `fogColour` is
entry 7 of that struct's `+0x4000` table (`+0x4000 + 7*0x20 = +0x40e0`), so
its **value pointer** - `0` until "a frame fills it", per `Shader_InitParamEntry`'s
own layout - lives at `+0x40f8` of whatever `0x008b7f04` holds at runtime.

**What does *not* fill it, checked and ruled out.** `Material_SetInstanceParamPointer`
(`0x005d4bb0`, already named on
[engine-trail.md](engine-trail.md)) looked like the candidate - it is exactly
"find a `0x20`-byte entry by hash, write its `+0x18`" - but its own first
comparison, `!(entry[+0x04] & 0x8000)`, **skips every entry whose kind has bit
15 set**, and `0xc000` (every engine parameter's kind, `fogColour` included)
has that bit set by construction. The function that binds instance overrides
is provably the wrong one for an engine-global slot. Its real callers
(reached through its cross-TOC OPD trampoline, `0x00677018`, since a direct
`get_function_callers` on `0x005d4bb0` finds only the trampoline itself) are
29 addresses spanning the image, `Trail_InitManagerBuffers` (`0x006b6870`,
the known `TrailSpeed` binding) among them and nothing yet tying the rest to
fog.

**The one lead this session's search actually narrows to two addresses, and
both go dark.** Loading `0x008b7f04` needs the literal instruction
`lwz rX,-0x54c0(r2)` against this exact TOC, and a masked
`search_byte_patterns` for that instruction (any destination register) finds
it in exactly **two** places in the whole 21 MB image: `0x0067f044` and
`0x0067f1c8`, inside two near-identical five/six-line functions
(`0x0067f030`, `0x0067f1b8`) that each do nothing but stash the pointer into
an object field and call `0x00326328` - a constructor-shaped pair, not the
setter itself. **Neither has a single statically-visible caller.**
`get_function_callers` finds none for either; `get_xrefs_to` on both finds
only their own `.opd` table entries referencing themselves, which is Ghidra
seeing the OPD's own `{address, TOC}` pair, not a call. Whatever constructs
these objects does it through a computed/indirect call - the same class of
gap `renderer.md`'s draw-path note already names ("no import census or call
graph finds it") - so this is where static analysis runs out, not merely
where the session did.

Confidence 88 on the offset chain (`base` at `0x008b7f04`, `fogColour` at
`+0x40e0`, its value pointer at `+0x40f8`) - read off the real TOC, and the
slot number cross-checked eight ways already on this page. Confidence on
"unscaled" itself is unmoved at 60; this narrows the address to look at, not
the answer.

### The "two addresses" lead is refuted - they read a different global, not `0x008b7f04` (2026-09-01)

**`0x0067f030` and `0x0067f1b8` do not touch `0x008b7f04`.** The paragraph
above assumed their `lwz rX,-0x54c0(r2)` resolves "against this exact TOC" -
i.e. the same one as `Shader_InitEngineParams`'s own `-0x54c0(r2)`,
`0x008bd3c4` - without reading either function's actual TOC off its `.opd`
entry. Doing that now: each function has exactly one `.opd` entry
(`get_xrefs_to` on both still finds nothing but that single self-reference,
reconfirming the "no static caller" finding stands), and the entry itself -
read as raw bytes, `{code address, TOC}` - is `0x008AD4D8` for both
(`0x00873b70` for `0x0067f030`, `0x00873b98` for `0x0067f1b8`; both sit in a
contiguous run of ten near-identical `.opd` entries, `0x0067AED0` through
`0x000B1878`, that all share this same `0x008AD4D8` TOC - a normal
one-module-shares-one-TOC pattern). `Shader_InitEngineParams`'s own `.opd`
(`0x0088c770`, read the same way) carries TOC `0x008BD3C4`, confirmed to
match the doc's citation. **The two TOCs differ** (`0x008AD4D8` vs
`0x008BD3C4`), and PPC ELF's `-0x54c0(r2)` is TOC-relative: under
`0x008AD4D8` it resolves to static storage `0x008A8018`, not `0x008B7F04` -
arithmetic anyone can redo (`toc - 0x54c0`), not a judgement call, so this
correction is confidence 95, not a competing guess. The original
`search_byte_patterns` hit matched the instruction's *bytes* (opcode plus the
`-0x54c0` immediate), which are TOC-independent by construction; nothing in
that search actually checked which TOC each hit resolves against, so "against
this exact TOC" in the paragraph above was asserted, not verified.

**What `0x0067f030`/`0x0067f1b8` actually construct is a different,
unidentified object.** `0x008A8018` currently holds `0x008630F0`, a pointer
into a large repeating data block (`0x00863080`-ish onward) inside the same
`0x00862000`-`0x00869000` address range this project already uses for C++
vtables (`collision.md`, `race-manager.md`, `physics.md`). Most words in that
block repeat a shared default OPD (`0x00885A80`, itself referenced from ~90
other tables in the same range - a common "default slot" stub), with
occasional rows overriding one or two slots with a class-specific OPD; the
two functions' own OPDs (`0x00873B98`/`0x00873B70`) appear together at one
such row (`0x00863120`/`0x00863124`). Whether that block is a genuine vtable,
a class-registration list, or something else, and what reads it, is
**unconfirmed** - no `lis`/`addis` load of `0x008630f0`, `0x00863120` or
`0x008A8018` exists anywhere in the image (checked via `search_instructions`
on the resolved literals), and the one function found reading the
neighbouring TOC slot on the same TOC (`0x0067f028`, a one-line getter for
`-0x54c8(r2)`; its caller `FUN_003f0950`, found via `get_xrefs_to` on
`0x008a8010`) is unrelated - it reads a sun-direction normalisation constant
pair (`PTR_DAT_008b7efc`/`PTR_DAT_008b7f00` under *its own*, different TOC)
and contains no `mtctr`/`bctrl` at all. **This is a second, independent point
where static analysis runs out**, on ground that turns out not to be the
fogColour chain.

**Consequence for the Next Steps below:** finding what calls `0x0067f030`/
`0x0067f1b8` would not answer whether `fogColour` is unscaled, because they
never read `0x008b7f04`. The right search is TOC-scoped, not byte-scoped: a
`-0x54c0(r2)` `lwz` inside a function whose *own* `.opd` TOC is `0x008bd3c4`
- not a `search_byte_patterns` hit on the instruction bytes alone. That
search is run below.

### The TOC-scoped search: all 81 engine parameters named, the real getter found, still no writer (2026-09-01)

**The corrected search.** `search_instructions` for the literal `lwz
rX,-0x54c0(r2)` finds it in **eight** places program-wide (not the two the
byte-pattern search found before - that one was scoped to *an* instance, not
*this* one), and reading each hit's own `.opd` TOC word narrows them to the
ones that actually run under `0x008bd3c4`: `Shader_InitEngineParams` itself,
plus three others - `0x003f0ff8`, `0x003f1010`, `0x003f1028` - all three
`.opd`-adjacent to `Shader_InitEngineParams`'s own (`0x0088c750`-`0x0088c770`,
one module). The other two hits (`0x000b2af0`, `0x000b43b0`) carry TOC
`0x008AD4D8` - the same wrong TOC as the refuted pair above, unrelated by the
same reasoning.

**`Shader_InitEngineParams` decompiled in full names every one of the 81
entries**, confirming the existing cross-checked reading at first-party
strength: entry 7 (`+0x40e0`) is built from `Crc32_HashString(PTR_s_fogColour_008b7f8c)`
- the literal string `"fogColour"` on the disc, not an inference. The
sequence, `+0x4000` to `+0x4a00` in `0x20` steps: `(unnamed, PTR_DAT_008b7f70)`,
`viewProj`, `(unnamed, PTR_DAT_008b7f78)`, `world`, `eyePositionWorldSpace`,
`positionBias`, `positionScale`, **`fogColour`**, `paraboloidReflectionTex`,
`paraboloidIblTex`, `constantAmbientColour`, `falseLightDirectionPower`,
`prelitScaleSpecular`, `prelitBias`, `directionalLight0DirectionWorldS[pace]`,
`directionalLight0Colour`, `directionalLight0Proj`, `directionalLight0ShadowTex`,
`directionalLight0LightmapTex`, `pointLight0PositionWorldSpace`,
`pointLight0Colour`, `pointLight0Falloff`, `textureSpot0PositionWorldSpace`,
`textureSpot0Proj`, `textureSpot0Tex`, `textureSpot0ShadowTex`,
`textureSpot0Colour`, `textureSpot0Falloff`, `textureSpot1*` (same six
fields), `textureSpot2*` (same six), `shadowMatrix`, `shadowMapTex`,
`shadowMapTexSize`, `quakePointA`, `quakePointB`, `quakeOffset`,
`quakeTrackUpNormal`, `distortion`, `refractProject`, `reflectProject`,
`screenSpaceRefractionTex`, `screenSpaceReflectionTex`, `zoneColourTint`,
`zoneEffectInner`, `zoneEffectOuter`, `zoneBaseInner`, `zoneBaseOuter`,
`zoneBaseAltInner`, `zoneBaseAltOuter`, `zoneOrigin`, `zoneTexInner`,
`zoneTexOuter`, `zoneTexInnerNearest`, `zoneTexOuterNearest`, `zoneTexVis`,
`zoneAnisoPalette`, `zoneAnisoPaletteOuter`, `zoneAnisoPower`,
`GradientColour`, `GradientColour1`, `GradientColour2`, `GradientColour3`,
`iblScalePower`, `ambientShadowMatrix`, `ambientShadowBlendFactor`,
`ambientShadowTex`, `globalAlphaScaler`, `auroraBrightness`, `auroraOffset`,
`auroraColour`, `engineTrail`, then the eight technique hashes
(`+0x4a20`-`0x4a3c`) already read. This is a byproduct worth having on its
own - the first two entries are unnamed strings this session did not chase
(`PTR_DAT_008b7f70`/`PTR_DAT_008b7f78`; likely `technique` and one more,
unread).

**`0x003f1010`, renamed `Shader_GetEngineParamTable`** (confidence 85):
`return g_ShaderEngineParamBase + 0x4000;` - a one-line accessor for exactly
the 81-entry table's base, nothing else. **`0x003f0ff8`, renamed
`Shader_GetVariantHash`** (confidence 80): `return *(g_ShaderEngineParamBase
+ index*4);`, a flat word-indexed read over the *first* `0x4000` bytes of the
same allocation - i.e. a different table sharing the base pointer, not the
81-entry one. Its known callers (`FUN_003ea368`, `FUN_003eb890`, seven more,
all `Render_RunCompiledOps`-adjacent material-effect code) pass a
runtime-computed OR'd flags value, never a fixed index, so this reads a
shader-**variant** selector, not an engine parameter - a real function, just
not on the fogColour path. **`0x003f1028`, renamed
`Shader_BuildVariantHashTable`** (confidence 78): fills `0x1000` (4096)
consecutive words at `g_ShaderEngineParamBase` with `~Crc32_HashString` of
permuted technique/feature-name fragments (`HalfBright`, `ShadowMap`,
`ZoneMode`, `FalseLight`, `ZoneTrans`, `NoAlbedo`, `IleLightmap`/`IleVertex`/
`Ambient`, `Spot0`-`Spot3`) - the table `Shader_GetVariantHash` reads. So the
single allocation at `g_ShaderEngineParamBase` (`0x008b7f04`'s pointee) is
two tables back to back: `0x0000`-`0x3fff` the 4096-entry variant-hash cache,
`0x4000`-`0x4a3c` the 81-entry (+8 technique-hash) engine parameter table -
which is why `Shader_GetVariantHash`'s callers looked like a lead and
weren't; they are reading the neighbouring table through the same base
pointer.

**`Shader_GetEngineParamTable` has no static caller either** - `get_xrefs_to`
and `get_function_callers` both find nothing for it, not even a trampoline
(checked directly against its `.opd` slot, `0x0088c758`, the way
`Material_SetInstanceParamPointer`'s cross-TOC trampoline was found before -
here there isn't one). This is the same "reached through a computed/indirect
call" wall as the refuted pair, except this time on the function that
actually is on the fogColour path, confirmed by name and by TOC. Two
independent sessions have now exhausted the static leads this thread named
(the byte-scoped search, then the TOC-scoped one); **the rpcs3 live-read
alternative in Next Steps is the remaining path**, not a fallback of last
resort.

### A controlled live read: the engine-param table's fogColour slot is dead in gameplay, but that doesn't answer "unscaled" (2026-09-01)

**What was actually read, stated up front so the boundary is clear**: this is
a live read of `Shader_GetEngineParamTable`'s `fogColour` slot
(`0x008b7f04`'s pointee `+0x40f8`, the value pointer) and of the settings
singleton's `Fog.Fog Density` (`0x008b6fb4`'s pointee `+0x500`). **It is not**
a read of the compiled/patched microcode constant slot the Next Steps
alternative named (the fslot-offset direct patch `scripts/ps3-microcode.py`'s
docstring describes) - that target is still unread.

**Method: `scripts/rpcs3-drive.py capture --region`**, walking `<addr
chain>:<len>` expressions (`@` dereferences) against a live, paused GDB
session - see its own docstring for the chain grammar. Three boots on
Talon's Junction (its own `track.envsettings` authors `"Fog.Fog
Density"=0.001000`, confirmed by re-reading the file, not assumed): 11
screenshot-confirmed in-race reads across the first two (3 interior in run 1;
4 more - one confirmed exterior, per `04.png` - in run 2, whose remaining 3
landed after a wall collision put the run on the Results screen, screenshot
`07.png` confirmed, so not counted as in-race), plus a third, **controlled**
run of 4.

**The measurement (confidence ~90 on the read itself)**: `fogColour`'s value
pointer resolves to a stable, non-null address (`0x00aed480`, identical
across all three boots) whose 16 bytes read `(0, 0, 0, 0)` on **every one**
of the 15 reads, in-race and out. The struct base itself cross-checks clean
- `0x00d3e220` this run, whose first word reads a `~crc32`-shaped value,
exactly what `Shader_BuildVariantHashTable` fills at offset 0 of the same
allocation, confirming the chain arithmetic independently of the fogColour
question.

**Why this isn't just an absent reading: the third run controls for it.**
A bare zero from one slot proves nothing on its own - the mechanism could be
unbuilt this frame, the pause could have landed outside any render pass, or
the chain could be silently wrong. So the same pause instants also read
`viewProj` (slot 1, `+0x4038`, a `mat4`) and `eyePositionWorldSpace` (slot 4,
`+0x4098`), two slots that **must** hold a real, per-frame-changing camera
if the mechanism is live at all. Both did, every time - real eye coordinates
that move race to race (`[1.5, -46.9, -176.9]`, `[563.6, -17.0, -5.5]`,
`[442.5, -3.9, 269.9]`, `[28.0, -46.8, 149.6]`) and a real, non-degenerate
view-projection row alongside them. **The engine-parameter mechanism is
therefore confirmed live and correctly wired at the exact instants
`fogColour` reads zero.** That is what makes the null worth publishing
rather than a boot that happened not to catch anything.

**What this does and does not settle - three possibilities survive, not
one.** It retires *this table* as fogColour's live source in normal race
gameplay on this circuit, but does not distinguish:

1. fog reaches the shader via the fslot-offset direct patch instead, bypassing this table entirely;
2. fog is not enabled at all in this race type/configuration, and both routes would read empty here regardless;
3. the slot is vestigial - declared by `Shader_InitEngineParams`, never filled by anything at runtime.

A grep of this page and the handover thread for what the 2026-08-18 reference
frame showed about visible fog came up silent - it records the render being
*judged* against that frame, not whether haze was visible in it - so
possibility 2 is not ruled out either. **The "unscaled" confidence stays at
60**, unmoved: the question "is `fogColour.w` `Fog.Fog Density` unscaled"
cannot be answered by reading a slot that does not carry the value in the
first place. What this retires is the *approach* the last two sessions took
(find `+0x40f8`'s writer), not the question - the next lead is the
fslot-offset direct patch mechanism, unread, or a live read of the compiled
microcode constant itself.

### The registry is resolved: every post program's block is addressable

Read 2026-08-18 with [`scripts/ps3-registry.py`](../../../../scripts/ps3-registry.py).
`ShaderRegistry_Register`'s signature is `Register(slot, name, block)` - the
**third** argument points straight at the program's `SHO` block, established
by reading the call site at `0x003b3938` rather than assumed. Resolving the
`lwz r4/r5, d(r2)` operands of every `bl` to it, each against its own
function's OPD TOC, ties 62 of the 121 named programs to their blocks, the
whole `FunkLayer` post chain among them; every resolved pointer lands on a
`SHO\x08` magic, which is the self-check.

### The bloom chain, read pass by pass

With names on blocks, `scripts/ps3-microcode.py fp <block>` reads the M6
bloom exactly. All constants below are the microcode's own; the patched ones
line up one-for-one with the `.envsettings` `HDR and Bloom` keys:

- **`FunkLayerBloomGate_fp` (`0x92d580`), the bright pass**:
  `gate = frame.rgb * frame.a * <Bloom from alpha contribution>`
  ` + frame.rgb * pow(dot(frame.rgb, (0.9, 1.77, 0.33)), <Bloom from frame exponent>) * <Bloom from frame contribution>`
  plus a patched bias and a subtractive clamp term. The luminance weights are
  inline - `(0.3, 0.59, 0.11) * 3` - and the `pow` is the usual `LG2`/`EX2`
  pair. So HD blooms from the **glow-mask alpha and from HDR luminance at
  once**, where Pulse's recovered gate is alpha-only.
- **`FunkLayerBloomDownsample_fp` (`0x92d780`)**: a single-tap scaled copy
  with an alpha scale-and-bias - no filtering in the shader; the sampler does
  it.
- **`FunkLayerBloomBlurVertical_fp` (`0x92d880`) /
  `Horizontal` (`0x92dc80`)**: a nine-tap separable kernel, weights
  `1, 0.8, 0.5, 0.2, 0.1` mirrored and divided by exactly `4.2` (the final
  `MUL` by `0.238095` = `1/4.2`), tap spacing a patched parameter -
  `Bloom horizontal size`/`vertical size` are the authored values that reach
  it.
- **`FunkLayerCopy_fp` (`0x92cd80`)** is a plain textured copy and
  **`FunkLayerColour2d_fp` (`0x92ce00`)** a flat-colour quad: **there is no
  tone-curve program anywhere in the chain.** The `Tone adaption boost` /
  `Tone maximum brightness` family therefore feeds a PPU-computed **exposure
  scale**, most plausibly folded into the per-draw patched lighting
  constants; the settings block that receives those keys is registered by
  `0x003a83d8`/`0x003a9520`/`0x006909a8` and the frame-time consumer is not
  yet read. Nothing implements a guessed curve on the strength of this - the
  absence of one is the finding.

**What implementing the read bloom needs**, and why it is not in this change:
the gate's luminance term is `pow(lum, 4) * 0.03` on Talon's Junction, which
only produces the reference frame's glow when `lum` runs over 1.0 - the gate
reads the **pre-exposure linear scene**. This renderer's race target is
`Rgba8Unorm` and saturates, so a faithful bloom needs the HD path rendered to
a float target first (scene in linear, gate, blur, composite, then the
exposure stand-in and encode). That is the M6-for-HD work item, now with
every constant read. *(Done the next day - the two sections below are what
finished it.)* **The "pre-exposure linear scene" half of that paragraph is
withdrawn** - see "The bloom chain runs on 8-bit surfaces" below, which reads
the surface format itself and settles that no value above 1.0 ever reaches
the gate.

### The bloom chain runs on 8-bit surfaces (2026-08-20)

Read off `EBOOT.elf` with `llvm-objdump` and an instruction sweep; **Ghidra's
decompiler refuses this binary** (PPC64/TOC), so this section is disassembly
only. Per-function TOC for the whole render cluster is `0x8bd3c4`.

`FUN_005c9a50` builds the `NV4097_SET_SURFACE_FORMAT` word - `colour | depth |
aa<<12 | type<<8 | log2w<<16 | log2h<<24` - emitted under header `0x00200200`
by `FUN_005bd6dc`/`FUN_005c1b80`. Its colour and depth inputs are both field
`+0x24` of a render-target object, and `FUN_005a6ce0` is that object's factory:
`create(out, width, height, format, antialias, ...)`, storing
`+0x1c`/`+0x20`/`+0x24`/`+0x28`/`+0x2c` at `0x5a6d94..0x5a6da0`. **That `+0x24`
is the raw libgcm enum is a self-check rather than an assumption**: `0x5a6dd0`
and `0x5a6dd4` branch on it being `32` or `64`, which are `Z16 << 5` and
`Z24S8 << 5` - depth targets carry the pre-shifted bits 5-7, colour targets
bits 0-4.

Sweeping the literal `li r6, N` at all 39 call sites of `FUN_005a6ce0`:

| Format | Sites |
| --- | --- |
| `A8R8G8B8` (8) | 27 |
| `Z16 << 5` (32) / `Z24S8 << 5` (64) | 3 / 3 |
| `R5G6B5` (3) | 2 |
| `B8` (9) | 1 |
| `F_W16Z16Y16X16` (11) | **1** |
| computed | 2 |

- **The scene colour surface is `A8R8G8B8`**, created at `0x003e17a0` inside
  `FUN_003e12e0`, the frame's render-target set allocator - the same function
  that calls the ladder allocator `FUN_003af980` at `0x003e23d8`. Its depth
  companion at `0x003e1818` is `Z24S8` and shares its antialias register.
  Confidence 85: `FUN_003e12e0` creates two colour+depth pairs plus a
  full-res `aa = 0` target at `0x003e19d4` that looks like the MSAA resolve
  destination, and which of them the chain's first downsample reads is unread.
  All three are format 8, so the conclusion does not depend on it.
- **All seven ladder buffers are `A8R8G8B8`**, `li r6, 8` at seven separate
  sites - `0x3b09d8` (half), `0x3b0a54`/`0x3b0ac4`/`0x3b0b34`/`0x3b0ba4`
  (four quarter), `0x3b0c1c`/`0x3b0c8c` (two eighth). Confidence 90, and this
  is the part that carries the finding: even were the scene FP16, the
  half-res downsample it is copied into is 8-bit and the ROP clamps there.
- **The one `F_W16Z16Y16X16` surface in the executable** is `0x003c919c` in
  the vtable-dispatched `FUN_003c9080`, half-width and full height, outside
  both `FUN_003e12e0` and the ladder. Unidentified, and not the scene.

**So the gate is an LDR bright pass.** Its `dot` weights are
`(0.3, 0.59, 0.11) * 3`, which puts the knee at luma ~1/3 of the surface's own
range; a white texel gives `3^4 * 0.03 = 2.43`. The `* 3` on the weights is
what places the knee there. `oag_render::post::hd_bloom` draws into a float
target for precision and applies the hardware's clamp where the chain samples
the scene; before that clamp its gate was being handed arguments around
`1.6e5`, which is the whole of why an HD race rendered as a wall of glow.

**Two open questions this leaves**, both stated rather than assumed away:

1. **Whether those 8 bits hold linear or gamma-encoded light.** The knee sits
   at 1/3 *of whatever the buffer holds*, and the two readings are different
   pictures. Nothing in the executable gamma-encodes: no `1/2.2`, `1/2.4` or
   `1/2.233` float exists anywhere in it (checked big-endian and in the
   fragment-microcode half-word-swapped ordering), and
   `NV4097_SET_SHADER_PACKER` - the sRGB write enable - is never emitted with
   a constant header. Taken at face value that says gamma, which conflicts
   with [ADR-0026](../../../architecture/adr/0026-hd-authored-lighting-is-linear.md).
   Confidence 70, and held rather than acted on: the scan can only find
   *literal* exponents while the gate's own `pow` is an `LG2`/`EX2` pair with
   a patched one, and ADR-0026 rests on a measurement against a real
   reference frame. The obvious tiebreaker - that a gamma buffer would pin
   `scale` at 1.0 forever and make the whole `Tone` family inert - **was run
   and does not discriminate**: the reference frame's mean luma is 0.585
   gamma / 0.42 linearised, and both are far above the `0.15` where
   `4 - min(20 * adapted, 3)` stops varying.
2. **Per-texture sRGB/`GAMMA` decode: settled negative, 2026-09-06, confidence
   90.** The six addresses named in the previous version of this entry
   (`0x5bf288`, `0x5bf328`, `0x5c653c`, `0x5c6644`, `0x640d20`, `0x64ad9c`) are
   not where the format/`CONTROL1`/`FILTER` words are computed - each is a
   dumb word-by-word copy from a precomputed 14-word struct into the push
   buffer at `0x1a00 + unit * 0x20`; nothing there reads a texture's own
   fields. The struct is built once, by
   [`Texture_BuildGcmRegisters`](../../../formats/gtf.md#remap-is-a-per-channel-source-and-force-table-and-it-is-read-now)
   (`0x005a9998`), which [`gtf.md`](../../../formats/gtf.md) already reads for
   `FORMAT` and `CONTROL1`. The one field that page does not cover is
   `FILTER`'s top nibble: `rlwimi r12, r24, 0x8, 0x0, 0x3` (identical at
   `0x5a9ad8`, `0x5a9b4c` and `0x5a9cfc`, all three of the function's format
   branches), where `r24` is the packed `(format << 16) | remap` argument.
   Rotate-left-8 then keep IBM bits 0-3 selects IBM bits 8-11 of the
   unrotated word, which are bits `0x80`/`0x40`/`0x20`/`0x10` of the *format
   byte itself* - exactly the bits [gtf.md's layout table](../../../formats/gtf.md#layout)
   already names (the format enum's own top bit, `UN` at `0x40`, `LN` at
   `0x20`) plus the always-set `0x80` high bit every `CELL_GCM_TEXTURE_*`
   constant carries. Checked against all ten format bytes on the disc
   (`0x88, 0x86, 0x87, 0xa5, 0x85, 0x81, 0x9e, 0xa8, 0xa6, 0xa7`): the nibble
   is `0x8` for every one with neither flag, `0xa` for the four with `LN`
   set, and `0x9` for `0x9e` (`A8B8G8R8`, whose own enum value already has
   bit `0x10` set) - fully explained by fields already documented, no residual
   bit. **No texture on this disc carries a distinct gamma/sRGB-decode
   control anywhere in its per-unit register set**, so if HD ever gamma-
   decodes a sampled texture it is not a per-texture choice the engine
   makes at load. **This does not settle question 1**, contrary to what the
   previous version of this entry claimed: a texture's *input* decode and
   the scene surface's *output* encoding are different ends of the pipe, and
   a negative result on one says nothing about the other. Question 1 stays
   open at confidence 70.

### The chain runner, and every engine-fed parameter (2026-08-19)

`FUN_003b4690` is the PPU function that runs the whole `FunkLayerBloom` set
and patches its parameters - found by walking who loads the descriptor table
at `0x8b73c4` (the ten program-name pointers, followed by the parameter
names `size, uvSize, contribution, additiveColour, offsetScale, offset,
screenOrigin, screenSize, distanceScalar, uvScale, uvScaleOrigin, texture,
sourceImage`). Its setup twin `FUN_003af980` allocates the target ladder:
one half-res, four quarter-res and two eighth-res buffers (`size >> 1/2/3`).
Confidence 90 throughout this section - static reading, two independent
routes (microcode operand order and PPU fill) agreeing on every value.

What it settles, each previously an unknown of the gate:

- **`contribution.w` is hard-coded zero** - the microcode's
  `(1 - contribution.w * frame.a)` damping factor never engages.
- **`additiveColour` is an event flash**: `{v,v,v,0}` with
  `v = flash * k + 0.1` while a flash field is live, `{0,0,0,0}` otherwise.
- **`contribution.y` is the authored `Bloom from frame contribution` faded
  by luminance adaptation**: `y = (1 - min(adapted * <Bloom adaption boost>
  * 0.25, 1)) * authored`, `0.25` an inline constant (TOC value `0x8b743c`).
- **The adaptation is a CPU readback loop**: the quarter-res scene is halved
  iteratively to a handful of pixels, read back, its mean colour stored at
  FunkLayer+0x240 and its luminance - weights `(0.3, 0.59, 0.11)`, TOC
  values `0x8b74fc..0x8b7504` - lerped into the persistent state at +0x254:
  `adapted += <Bloom adaption rate> * (avgLum - adapted)`.
- **The blur runs at quarter res with tap step `authored size / buffer
  size`**: the multipliers are the TOC constants `1/480` and `1/270`
  (`0x8b7508`/`0x8b750c`) - the quarter buffers of a 1080p frame.
- **The gate reads the quarter-res scene**, not the full frame: downsample
  to half, to quarter, gate, blur-vertical, blur-horizontal, ping-ponging
  the quarter pair.

The settings block the parameters come from is pinned by its registrar
(`FUN_003a83d8`): each `HDR and Bloom.*` key string maps to a field of the
singleton `FUN_003a9520` returns - `+0x52c` adaption rate, `+0x530/0x534/
0x538` frame contribution/exponent/alpha contribution, `+0x53c` adaption
boost, `+0x540/0x544/0x548` the Tone family, `+0x54c` `Bloom feedback` (a
key no circuit file authors), `+0x550/0x554` the blur sizes, `+0x558..0x570`
the radial set. That mapping is what turns every "plausibly this key" in
the sections above into a read.

### The exposure is read: `scale` on the resolve, not a tone curve (2026-08-19)

The `Tone` family's consumer is `FUN_003e3268` (the every-frame present
path; `FUN_003df7c0` is a standalone re-run of its tail). It is the sole
caller of the chain runner, stores the returned adapted luminance at
`0x008c3520+0x28`, and computes

```text
scale = <Tone maximum brightness> - min(<Tone adaption boost> * adapted,
                                        <Tone darkening clamp>)
```

(`fsel`-min, quoted in full in the analysis notes). On Talon's Junction
(boost 20, clamp 3, max 4) that is `4 - min(20 * adapted, 3)`: a black frame
is pushed 4x, anything with adapted luminance over 0.15 rides at exactly
1.0, and the image is never darkened below 1x - **the "tonemap" is a plain
scene multiplier with a floor**, which is why no tone-curve program exists.

`scale` is bound to the resolve program `downsamplescaleaddfeedback_fp`
(block `0x92a500`), whose four parameter names all fall to crc32 preimage:
`scale` (`0x13b9da7b`), `scaleFeedback` (`0xaa718745`), `scaleAdd`
(`0xabd84d0a`), `fullscreenTintColour` (`0xde0aade6`). Its microcode reads:

```text
feedback = lerp(scene, 2 * feedbackBuffer, feedbackBuffer.a * scaleFeedback)
out      = saturate(feedback * scale + bloom * scaleAdd + tint)
```

with `scaleAdd` fed the constant 1.0 - **the bloom is added after the
exposure scale, unscaled**; that fill was re-read end to end on 2026-08-20 and
holds at confidence 92 (below, "`scaleAdd` re-read") - `scaleFeedback` fed `Bloom feedback + a runtime
float` (inert at the authored default 0), and the resolve ending on its
`ADD_SAT` with no gamma arithmetic. The correction variant
(`downsamplescaleaddfeedbackcorrection_fp`) adds `saturation`/`finalScale`/
`finalBias` parameters whose fill is a render-context field this reading
did not chase.

All of the above is implemented verbatim in `oag_render::post::hd_bloom`;
its module header lists the four things that are deliberately *not* modelled
(GPU-side adaptation in place of the readback, the event flash, the feedback
mix and tint, and the final display encode).

### `scaleAdd` re-read, and the parameter names are literals (2026-08-20)

The resolve's three floats were traced from the PPU side rather than from the
program's parameter table, and the two orders **disagree** - a trap worth
carrying. `FUN_005e29c0`/`FUN_005e3f68` stash the arguments (`fmr 31,1 /
fmr 30,2 / fmr 29,3`) and bind each into a lazily-resolved slot on the render
globals (`0x5e2bac`/`0x5e2c28`/`0x5e2ca4` into `+0x218`/`+0x228`/`+0x238`).
Each slot's resolver loads a TOC pointer, and against TOC `0x8bd3c4` those
pointers are **plain ASCII strings**, not hashes:

| Slot | TOC+ | Pointer | String |
| --- | --- | --- | --- |
| `+0x218` = f1 | 8084 | `0x007cdd38` | `scale` |
| `+0x228` = f2 | 8100 | `0x007cdd58` | `scaleAdd` |
| `+0x238` = f3 | 8128 | `0x007cdda8` | `scaleFeedback` |
| `+0x248` | 8132 | `0x007cddb8` | `fullscreenTintColour` |

So the crc32-preimage argument above is no longer what the four names rest on.
**The program's own parameter table orders them `scale, scaleFeedback,
scaleAdd` while the PPU binds `scale, scaleAdd, scaleFeedback`** - inferring
the mapping from table order gives the wrong answer.

What reaches `scaleAdd` at all four call sites (`0x3dfa18`, `0x3dfbbc`,
`0x3e3800`, `0x3e3cb4`) is `lfs 2, -22232(2)` - `0x8b7cec`, which holds `1.0`
- with no `fmuls` or `fsel` between the load and the `bl`. Confidence 92, two
routes. Note `0x8b7cec` is that compilation unit's **shared** `1.0`, loaded
from the same displacement at 31 sites, so "the constant at `0x8b7cec`" means
only "the literal 1.0". Two corrections to the section above: `+0x28` holds
the returned adapted luminance only transiently - `0x3e36d8` overwrites it
with the clamped product before any reader - and `FUN_003e3268` is not the
chain runner's caller; the two `bl 0x3b4690` sites are `0x3e2e58` and
`0x3e3080`, both below it.

**The chain issues seven draws, where this project issues five.** Tallying
`bl` targets across `0x3b4690..0x3b8317`: the surface-bind wrapper
`FUN_005a40f8` 14 times, and `FUN_005a4668`, `FUN_005cb6e0`, `FUN_005c1754`
and `FUN_005c176c` 7 times each (parameter bind `FUN_005a3ef0`, 29). Only the
`FUN_005a4668` count is a pass count: **14 surface binds is not 14 passes**,
because several sit on mutually exclusive arms of the same branch - `0x3b50c0`
and `0x3b7334` are the two sides of the `bne` at `0x3b5090` - and one,
`0x3b5350`, is a loop body. The bind census below is what settles the two
extra, and it is *not* a coarser blur level.

### The chain was unswitchable, and how bright it actually is (2026-09-09)

A regression hunt, opened on a from-play report that HD/Fury's bloom "seemed
to have gone much stronger recently". **It found no regression and one real
defect**, and both halves are worth carrying.

The metric throughout is **clipped-white share** - the fraction of pixels with
R, G and B all at least 250 - and it is now `scripts/clipped-white.py` rather
than a scratch file. Mean luminance does *not* discriminate here and never
did; that is why the 2026-08-20 defect survived review. Captures are
`oag-game --race <iso> --ticks 0 --size 1440x816 --screenshot`, native, no
dynamic resolution and no upscaler, against the rpcs3 grabs under
`data/reference/hd-capture/`. **Close framings, not identical, so aggregates
only - never a pixel diff.**

**No regression, measured four ways.**

| what | clipped white | mean |
| --- | --- | --- |
| recorded state after the 2026-08-20 fixes | 10.20 % | 0.497 |
| the same framing on 2026-09-09 | 9.860 % | 0.513 |

Six matched framings, rendered by a build at `5189f942^` (before HD's glow
layer landed) and by `2871f895`, using `--pose` so both drew the same view:

| pose | pre-window | today |
| --- | --- | --- |
| `500.0,-28.2,-48.0` | 4.355 % | 4.281 % |
| `184.0,-35.9,186.7` | 15.081 % | 13.560 % |
| `-410.8,2.6,62.2` | 7.479 % | 7.352 % |
| `-630.4,-5.1,-200.3` | 13.975 % | 13.663 % |
| `-156.0,-51.6,-627.0` | 3.780 % | 3.271 % |
| `255.8,-45.3,-178.3` | 6.628 % | 6.352 % |

Every one is *lower* today. The frame did not get brighter in that window.
Nor does it climb with time: stationary at ticks 0/60/120/300/600/1200 it
reads 9.86/10.12/10.17/10.15/10.34/10.58 %, and out on the circuit under
autopilot it sits at 5.8-6.0 %, inside the reference band. There is no
accumulating term.

**The gate's two suspects are both inert.** Forcing `frame.a` to zero in
`fs_gate` still gives a **byte-identical PNG** at tick 0, exactly as it did on
2026-08-20: `GlowMask::Protected` masks alpha off everywhere that matters, so
`alpha_contribution` contributes nothing to a race frame. And zeroing HD's own
emissive `glow` summand in `mesh.wgsl` moves the frame by 0.000 points at
every tick measured - see the defect below for why.

**The defect: the player's bloom switch never reached this chain.**
`race::Scene` gated the bloom on `bloom_enabled && hd.is_none()`, so
`[graphics] bloom` reached only the PSP chain (`oag_render::post::bloom`) and
Wipeout HD - the only title *this* chain draws for - bloomed regardless.

| title | `bloom = true` -> `false` |
| --- | --- |
| Wipeout Pulse, autopilot tick 900 | 2.069 % -> 0.408 % |
| Wipeout HD, grid | **byte-identical** |

`settings::default_bloom` returns `false` and `Graphics::bloom`'s doc comment
says why, so **the switch is off unless a player turned it on** - which makes
HD/Fury the only title that bloomed out of the box, not a quirk of one
configuration.
`hd_bloom::Glow::Suppressed` now skips the gate and both blurs and clears the
resolve's bloom input. It does **not** skip the chain: the ladder feeds the
luminance adaptation and the exposure resolve is what encodes the linear scene
target at all. No measured constant moves, and a `bloom = true` capture is
byte-identical to the same capture before the change.

**What that says about our magnitude.** With the switch honoured the grid
frame reads 9.860 % with the bloom and 4.507 % without, against **3.85 %**
(`data/reference/hd-capture/talons/00.png`) and **6.99 %**
(`talons-fifo/00.png`) on the two rpcs3 grabs. So our bloomed frame is
brighter than either reference and our unbloomed one is between them. That is
consistent with what the 2026-08-20 work already concluded - the remaining
error is a missing *material*, not a bloom constant - and it is **not** a
licence to scale a read constant down. The switch is a player control, not a
calibration.

**Two things this pass deliberately did not do**, both filed rather than
fixed:

1. In `hd_bloom.wgsl`'s `fs_blur` the tap offset is added *after* `drawn()`,
   so `min(uv * uv_scale, uv_max)` does not constrain the taps - contrary to
   the comment above `drawn()`. Inert at native, where `uv_scale` is 1.0;
   wrong under dynamic resolution or FSR. Fixing it changes DRS-path behaviour
   with no measurement behind the new behaviour.
2. `mesh.wgsl` sums HD's emissive `glow` into `plain` and not into
   `lit_linear`, and the fragment resolves
   `mix(plain_rgb, authored_rgb, scene.light.enabled * in.lit)` - so on HD,
   whose rig is always enabled, every chunk with `in.lit == 1` drops the glow
   entirely. Forcing `glow` to a flat red changes **3.60 %** of the frame at a
   delta above 8/255 (42,305 of 1,175,040 pixels), and only the far background
   and the hull: the track, the tubes and everything near the camera are on
   the authored path and never see it. That makes the layer `5189f942` landed
   very nearly invisible. Fixing it makes the frame *brighter*, which is the
   opposite of what the report asked for, so it belongs to its own pass with
   its own reference comparison.

### The 14 surface binds, read (2026-08-20)

`FUN_005a40f8(ctx, depth, colour0, colour1, colour2, colour3)` takes **pointers
to slots**, not targets: it dereferences each, counts the non-null colour ones,
reads the first one's `+0x1c/+0x20/+0x24/+0x28` as width/height/**format**/pitch
and the depth slot's `+0x24` as the depth format, and feeds them to the
surface-format word builder `FUN_005c9a50`. All 14 call sites pass a **zeroed
stack slot for depth and for colour1..3**, and a field of the chain object
(`r25`) for colour0 - so every bloom pass is single-target, depthless
(`FUN_005a40f8` substitutes `0x40`, `Z24S8 << 5`, at `0x005a45fc` when the depth
slot is null). A third independent route to the 8-bit finding falls out of the
same read: the one path where no colour target is bound at all, `0x005a4558`,
hard-codes `li r6, 8`.

Cross-referencing the `r25` offset at each bind against the allocation order in
`FUN_003af980` - seven `FUN_005a6ce0` calls, each `li r6, 8`, with the
dimension arguments shifted right by 1, 2, 2, 2, 2, 3, 3 (`rldicl` at
`0x3b09b8`, `0x3b0a2c`, `0x3b0bf4`) - gives the ladder:

| Slot | Res | Bound as target at |
| --- | --- | --- |
| `+0xd0` | half | `0x3b49c4` |
| `+0xd4` | quarter | `0x3b6298`, `0x3b65c8`, `0x3b7ccc` |
| `+0xd8` | quarter | `0x3b4d38`, `0x3b6460` |
| `+0x84` | quarter | `0x3b7334`, `0x3b7510` (indexed `+0x84 + 4 * [+0x80]`) |
| `+0x88` | quarter | (same indexed pair) |
| `+0xdc` | eighth | `0x3b50c0`, `0x3b5350`, `0x3b6f74` |
| `+0xe0` | eighth | `0x3b5a1c` |
| caller's | full | `0x3b6a14`, `0x3b79f0` (`[sp+0x87c]`) |

**The two eighth-res buffers are the luminance reduction, not a second blur.**
Three of the seven draws target them, `0x3b5350` is inside the loop that begins
at `0x3b5318`, and the pair ping-pongs (`+0xdc` and `+0xe0` swap roles between
`0x3b5318` and `0x3b58a4`, alongside a matching swap of `+0xec`/`+0xf0`). That
is exactly the "halved iteratively to a handful of pixels, then read back"
adaptation this page already documents, and which `oag_render::post::hd_bloom`
deliberately replaces with a GPU 1x1 ping-pong. **So the pass gap is a
substitution this project already declares, not missing work** - and a coarser
blur level would have had the wrong sign for the defect that prompted the read:
the resolve adds bloom *after* the exposure scale and then saturates, so an
extra additive level raises clipped white, while our frame measures brighter
than the reference, not dimmer.

Confidence 85. Static reading of one function; the slot-to-resolution mapping
is two routes (allocation order and shift amount) but the reduction-loop
reading rests on control flow alone, and **what the 29 parameter binds put in
each pass's `sourceImage` is still unread** - that is what would raise it to a
full producer/consumer graph. The `+0x84`/`+0x88` indexed pair and the radial
keys at `+0x558..+0x570` are likewise untouched.

### The lit track material, read: there is no sun in the diffuse path (2026-08-20)

`scripts/ps3-microcode.py fp-file` on `talons_junction`'s
`track_surface.rcsmaterial` (18 fragment blocks), with the sampler and
parameter name hashes taken by the same `~crc32` preimage the `fogColour`
section above establishes. New preimages: **`lightmap` (`0x37b5db58`)**,
`DiffuseTexture` (`0x11cb4f74`), `NormalTexture` (`0x739a786e`),
`shadowMapTex` (`0x730df9ee`), `prelitBias` (`0x002c73e8`),
`SpecularColour` (`0x370a63cb`), `SpecularPower` (`0x81e0e773`); and on the
vertex side `position` (`0xb9d31b0a`), `normal` (`0xde7a971b`),
`tangent` (`0xdbe5f417`), `viewProj` (`0x2e7d5f33`).

**Block #9 (`0x6450`) is the small lightmapped variant** and reads end to end:

```text
@0x00  TEX H0.xyz, f[TC4].zwzz unit1     <- lightmap, on TC4's *second* uv pair
@0x01..0x11  LG2 / MUL / EX2 x3          <- pow(lightmap.rgb, k) per channel
@0x12  MAD H2.xyz, H0, {scale}, f[TC1]   <- + the interpolated TC1 term
@0x15  TEX H1.xyz, f[TC4] unit0          <- albedo, on TC4.xy
@0x16  MAD R2.xyz, H2, H1, -{bias}       <- (light) * albedo - bias
@0x18  TXP R1.x, f[TC0] unit2            <- shadowMapTex, projected
@0x19  ADD H0.w, -R1.xxxx, {1}           <- alpha = 1 - shadow
@0x1b  MAD H0.xyz, {fog}, R2, {fogColour}
```

**Block #8 (`0x6100`) is the same variant with no lightmap** and is three
instructions of lighting:

```text
@0x03  ADD H2.xyz, f[TC1], {const}
@0x07  TEX H1.xyz, f[TC4] unit0
@0x08  MAD R2.xyz, H2, H1, -{bias}
```

So **the diffuse term carries no `N.L` and no sun colour at all**: it is
`(pow(lightmap, power) * scale + f[TC1]) * albedo - bias`, and the whole
difference between a lightmapped and a non-lightmapped surface is whether the
first summand exists. Directional light *does* reach this material, but only
through the **specular** path - block #7 (`0x5930`) normalises a light
direction, dots it against the normal-mapped normal (`DP3_SAT R2.z, R3, R2`)
and raises it to a power (`LG2`/`EX2`), and declares `SpecularColour` and
`SpecularPower` to match. **"HD has no sun" would be wrong; "HD's diffuse has
no sun" is what the microcode says.**

**`f[TC1]` is the vertex light, and it is RGBE.** Most vertex blocks write
`MOV o[TC1].xyz, c[208].xxxx` - a broadcast engine constant - but the two that
source it from an input register (`v[4]` in one, `v[3]` in the other; the
`attribute` lines name only the hashed slots, so which declared attribute that
register is fed from is **not** established here) write

```text
 4  MAD R0.x, v[4].wwww, c[208].xxxx, -c[208].yyyy
 9  EX2 R4.w, R0.xxxx
15  MUL o[TC1].xyz, v[4].xyzx, R4.wwww
```

which is `rgb * exp2(a * k - b)` - a shared-exponent HDR decode, not a tint.
That is what the vertex-colour census in [`HANDOVER.md`](../../../../HANDOVER.md)
found without being able to name: of Talon's Junction's 983 chunks, 327 declare
`lightmapUV` and no colour set and 351 the reverse, **0 both**. A chunk carries
its baked light in the atlas or in its vertices, the shader adds whichever it
has to `f[TC1]`, and `crates/render/src/mesh/rcs.rs` writes `[1, 1, 1, 1]` for
every HD vertex - which is why wiring that attribute in as a *multiplied* tint
blacked out the banner quads and the ship hulls when it was tried.

Confidence 86 on the shape (static reading of the microcode's own arithmetic,
two variants agreeing, and the vertex side corroborating the fragment side);
**0 on the coefficients**, and that is the blocker. Every `{0, 0, 0, 0}` above
is a real zero *in the file* - the tool resolves payloads, and does print
`{2, -1, 0, 0}`, `{1.44269, 0, 0, 0}` and `{32, 0, 0, 0}` elsewhere in the same
dump - because those slots are **patched at draw time**. Two of them are
already known from the circuit's own `.envsettings`: `Lighting.Prelit ambient
colour scale` is 4 and `... colour power` is 2, which `oag-render` already
applies. `prelitBias` and the RGBE `k`/`b` at `c464` (declared `0x3466fc0e`,
preimage not found; note the disassembly's `c[N]` is the declared `c[N+256]`)
are not. **One shader change was made on this reading**: `mesh.wgsl`'s `authored`
term dropped its `sun * (ndl * baked.a)` summand, which nothing in either
variant computes. On the Talon's Junction grid that moves the clipped-white
share from **14.25 % to 10.22 %** against the reference frame's 5.8 %, and
mean luma from 0.583 to 0.487. The residual darkness is the missing `f[TC1]`,
and it is deliberate: an absence this page can point at beats an invention
that happened to fill the gap. Removing the *specular* as well - the
`baked.a` gate on the lightmap-less placeholder - was measured too (9.08 %
clipped, mean 0.467) and **not** taken, because the specular block that was
read (#7) does compute a directional term and only its diffuse sibling does
not.

**The removal was first recorded as wider than its evidence. Reading a ship
material narrowed the gap the other way** - see "Ships have no Lambert
diffuse either" below. What stays true is that `mesh.wgsl` has one lit path
for all HD geometry while the reads cover two materials, and that whatever
replaces it will have to be per-material.

**Left unread**: whether `shadowMapTex`'s `1 - shadow` in `H0.w` reaches colour
in a later pass - `oag-render` has no shadow map at all - and what fills
`c464`. The registrar route that pinned the `HDR and Bloom.*` keys
(`FUN_003a83d8`/`FUN_003a9520`) is the way to chase the latter.

### The vertex-light constants are read, and where they live (2026-08-20)

The question left by the section above - what fills the `k`/`b` of
`o[TC1].xyz = v.xyz * exp2(v.w * k - b)` - is **answered**, and the route is
not the one this page proposed. The registrar (`FUN_003a83d8`/`FUN_003a9520`)
was a dead end: sweeping all 693 `.rcsmaterial` on the disc, the decode
appears in **6,946 vertex blocks across 222 materials**, and in every one the
register is *undeclared*, at an index sliding from `c[198]` to `c[208]`. A
register no shader names cannot be reached by a route that maps
`.envsettings` key strings onto settings fields.

**The values are in the file, in a table this page had not framed.**
`Rsx_UploadVertexConstants` (`0x005c176c`) is what pointed at it. It emits
`NV4097_SET_TRANSFORM_PROGRAM_START` (`0x1ea0`, count 1),
`NV4097_SET_VERTEX_ATTRIB_OUTPUT_MASK` (`0x1ff0`, count 2) and `0x1ef8`, then
loops on `0x00141efc` - `NV4097_SET_TRANSFORM_CONSTANT_LOAD`, **count 5** -
writing one word from a stream advancing 4 bytes and four from a stream
advancing 16. One index plus one `vec4`, per iteration. Reading its own
arithmetic backwards gives the layout, at the sub-object the caller reaches as
`block + u16@+0x16`:

```text
+0x16  u16   constant count
+0x18  u32[] the constant indices
       vec4[] the values, at align16(0x18 + count * 4)
```

On `track_surface.rcsmaterial`'s block at `0x66c0` that is
`c[464] = (255, 128, 0, 2)` and `c[463] = (1, 0, 0, 0)` - and the decode in
that same block reads `c[208]`, which is `c464` under this page's `N + 256`
rule. **The table names the register the disassembly reads**, which is the
second route: the `+256` mapping and the constant values confirm each other.
`c464.w = 2` and `c463.x = 1` are the `v * 2 - 1` the same program applies to
its normal, and `c464.z = 0` is the zero it broadcasts.

Across the whole disc the decode's constants are **`x = 255.0` and
`y = 128.0` in all 6,946 blocks** - Radiance RGBE with the mantissa left as a
`[0, 1]` fraction. Confidence 92.

**And that decode is unreachable on this disc.** It reads attribute
`0x868f8229`, and **no `.rcsmodel` declares it** - 0 of 123 files contain the
hash. **The reason is now read**: `0x868f8229` is `~crc32("SpuVertexColours")`,
a stream the SPU writes rather than one a model file carries, and the variant
key's `SVC0` token - what all the disc's static geometry selects - means "no
such stream". See [rcsmaterial.md](../../../formats/rcsmaterial.md), "What
selects a variant". What the models carry is a plain colour set (`0x1aaf7631` on 297 of
Talon's Junction's chunks, `colorSet1` on 54), and its fourth byte is **41 %
zero and 52 % full** over 240,400 lit vertices: a mask, not an exponent.
Applying the RGBE decode to it produces values around `1e-39` and blacks the
frame out - measured, before the mistake was caught.

The applicable form is the other one the same vertex programs carry:
`MOV o[TC1].xyz, v[N].xyzx`, the colour set moved across unchanged. That is
what `oag_rcs::rcsmodel::Mesh::vertex_light` reads and `mesh.wgsl` now
adds into its authored sum, and it restores the ship livery, the grandstands
and the track's contrast that the sun removal had flattened: clipped white
**10.20 %** against the reference's 5.8 %, mean **0.497** against 0.585.

**Zero for a chunk with no colour set is read too**: the vertex blocks with no
such attribute write `MOV o[TC1].xyz, c[K].xxxx`, and resolving `c[K]` through
each program's own constant table gives **0.0 in 11,180 of the 11,184 blocks**
that do it (4 carry no table).

The selector `oag_formats` uses for it - four normalised bytes, not
`tangent` - was checked against **all 123 `.rcsmodel` on the disc**, where the
only four-byte-normalised hashes are `0x1aaf7631` (3,672 chunks),
`VertexColour1` (496), `colorSet1` (484) and `tangent` (1,208).

**`f[TC1]` is this material's interpolator, not a convention.** A taxonomy
sweep of all 693 materials found the same `ADD interpolator, {const}` into
`MAD light, albedo, -bias` closer arriving on **`f[TC0]`** in
`talons_junction/bluemetal` block #2, so which varying carries the vertex
light is per-program and has to be read per material rather than assumed.
`oag-render` binds one interpolator and is right only for the family read
here. Confidence 82 on the `bluemetal` reading, which is a second-hand
measurement recorded rather than re-derived.

One of these is now settled, one is not. **`0x1aaf7631` and `colorSet1` are
not one attribute - refuted, 2026-09-06, confidence 78.** The 297 + 54 = 351
count only ever showed they are *complementary*: a chunk carries one or the
other, never both, and neither sits beside a `lightmapUV`. That is exactly
what "a chunk authors one colour set or the other" predicts and says nothing
about whether the two hash to the same underlying attribute. [The four-byte
census](../../../formats/rcsmodel.md#the-four-byte-attributes-are-three-different-things-and-one-is-vertex-colour)
already on this disc gives them different fourth-byte distributions over a
combined 4,939,968 vertices: `colorSet1`'s fourth byte is **255 on all
682,796** of them, and `0x1aaf7631`'s is **23 % at 255, 66 % at 0** over its
4,257,172. One hash is a constant alpha of one; the other is a two-valued
mask on two-thirds of its vertices. Two attributes with the same identity
would carry the same fourth byte on every vertex that has one, and these do
not, so the two are distinct attributes that happen to occupy the same
structural role (a per-chunk choice of colour set) rather than one attribute
under two names. `0x1aaf7631` itself stays unnamed - this rules out one
candidate identity, it does not supply another, and [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)'s
confidence-50 floor for a rename applies to a data attribute the same as a
function.

**What the fourth byte gates is still unread**, so nothing consumes it here.
`0x1aaf7631` is declared by no `SHO` shader block (the taxonomy sweep already
recorded above), so the only place left to look is per-material microcode via
`scripts/ps3-microcode.py` - a fresh sweep, not a re-read of what is already
here.

`0x005c6c5c` writes `0x00041ea4` with the payload `(arg3 << 4) | arg2`, and
three inlined copies of the same emission sit at `0x005bf578`, `0x00642b50`
and `0x0064bdd4`. None has a code cross-reference - only its OPD descriptor -
and the method is not identified here, so it is **left unnamed** per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

### Ships have no Lambert diffuse either (2026-08-20)

The section above first recorded ships as a counter-example - that
`detonator_ship_rich_iridescent.rcsmaterial` carried a directional diffuse
the sun removal had taken away. **That was wrong, and reading the material
properly reverses it.**

Its **vertex** side settles what `f[TC1]` is. Block #1 (`0x3370`) builds

```text
10  ADD R0.xyz, -R4.xyzx, c[209].xyzx   <- camera - world position
14  DP3 o[TC1].z, R0.xyzx, v[1].xyzx    <- . normal
15  DP3 o[TC1].x, R0.xyzx, R2.xyzx      <- . tangent
18  DP3 o[TC1].y, R0.xyzx, R2.xyzx      <- . bitangent
```

with `c[209]` the parameter `0x3466fc0e` - which this pins as the **camera
position**. So `f[TC1]` on a ship is the **view vector in tangent space**,
and the `DP3_SAT` against the normal map that the earlier note read as `N.L`
is `N.V`. **No ship material declares a light direction on the vertex side
at all**: all four checked declare the same set - `viewProj`, camera,
`positionScale`, `positionBias`, `world`, and two further `float4 x4`
matrices (`0x9f21b213`, `0xa2419ba3`).

The **fragment** side is where a direction does arrive, and it arrives the
same way track surfaces get theirs. Block #2 (`0x3a00`) reconstructs the
world normal from the tangent basis and the normal map, then

```text
@0x13  DP3_SAT R0.y, normalize(f[TC5]), {const}   <- N.L
@0x17  ADD R2.xzw, normalize(f[TC1]), {const}     <- V + L, the half vector
@0x24  DP3 R2.z, normalize(half), normal
@0x40  LG2 / @0x43 MUL by 40 / @0x46 EX2          <- pow(N.H, 40)
@0x4c  MUL R0.x, spec, R0.yyyy                    <- times N.L
```

That `{const}` is the light direction, patched by the same `0x02df31e5` that
`track_surface` block #7 patches into its own half-vector - the specular
this project already implements. **What ships have and track surfaces do not
is a view-driven colour dodge and burn** (`1/(1 - N.V)` and `1/N.V`
reciprocals against a per-channel map) **and an environment lookup on a third
sampler**, neither of which is a light term and neither of which
`oag-render` implements.

> **This generalisation was made on two materials and is false.** Five more
> were read on 2026-08-20 and every one of them carries a Lambert sun diffuse:
> `diffuse_with_specular_from_alpha` (86 chunks of Talon's Junction),
> `..._scalar` (80), `diffusewithalphachannel` (43), `track_wall` (33) and
> `glasstest` (33) - **275 chunks against `track_surface`'s 15**. Each
> normalises its interpolated world normal, `DP3`s it against a patched
> direction, multiplies by a patched colour, **gates it with a sun-occlusion
> scalar**, adds the prelit term and multiplies the sum into the albedo, with
> the `pow(N.H, e)` specular added afterwards. Confidence 88 on the structure;
> 75 on the two constants being `directionalLight0DirectionWorldSpace` and
> `directionalLight0Colour`, which rests on the declaration tables alone. See
> "The sun is real and it is masked" below. **`track_surface` is the exception
> on this circuit, not the rule**, and the shader change justified on it is
> wrong for the majority.
The sun removal is better founded than the first note claimed, and **the
reason a hull renders dark is a missing material, not a missing light**:
`ship.rcsmodel` is 57 meshes of which **4 declare a colour set and none
declares a `lightmapUV`**, so 53 of them fall to `albedo * ambient` in a
renderer that has no iridescence and no environment map. That is a
per-material shader path, which is a feature rather than a patch, and
nothing here fakes it.

**One thing this does falsify.** `mesh.wgsl` hard-codes the specular
exponent at 32 and calls it "an inline constant of the microcode, the same in
every lit variant". It is not: sweeping every material for the literal a
saturated dot is multiplied by between its `LG2` and its `EX2` gives **5**
(759 blocks), **10** (704), **32** (295), **26.156** (10), **40** (10, the
ship) and **300** (6). 32 is the commonest of the three round values and
stays as the stand-in, now labelled as one. Confidence 80 - the sweep keys on
an instruction pattern rather than on each block's meaning, so some of the 5s
and 10s may be another `pow`.

**Reimplemented and disc-verified (2026-09-01), and the ambiguity above is
worse than "some of the 5s and 10s".** `fragment::Program::specular_exponent`
ports the sweep: an `LG2`/`MUL`/`EX2` chain whose `LG2` reads a register a
saturated `DP3` most recently wrote (the `N.H`/`N.L` idiom every specular and
sun-diffuse read on this page shares), returned only when the chain's result
reaches the program's output colour (the same "last colour write" rule
[`output_lit_by`'s `taint`](../../../../crates/rcs/src/rcsmaterial/fragment.rs)
already uses). Verified against the ship's own worked example
(`pow(N.H, 40)`, `@0x40`/`@0x43`/`@0x46` above) and against a synthetic case
built from `track_surface`'s own lightmap curve, which the bare instruction
shape alone does not exclude (see below) - both pass as unit tests.

**A disc-wide sweep with both gates does not reproduce the six counts
above, and the gap is now understood well enough to say why, not just that
it fails.** Population, excluding Zone-declaring blocks:

```text
0          573 blocks   35     4
1.4426949   19          200    8
250         4           260    24
300         10          32     409
40          2
```

Three findings, not one:

1. **The `0` bucket - the largest - is very likely the "patched at draw
   time" case this page's own RGBE and sun-direction readings already
   established** (`c[464]`/`c[463]` above): a material that *declares*
   `SpecularPower` (`0x81e0e773`, first named on this page) but leaves its
   file-static literal at `0.0`, patched to a real value at export or draw
   time. `pow(x, 0) = 1` is not a plausible authored shininess, so these 573
   blocks are the strongest candidates for "the exponent is not in the file
   at all" rather than for a fifth real value. Unverified beyond the
   plausibility of the theory - closing it needs the `patch fslot` to
   `const@slot` decoder `rcsmaterial.md` already flags as owed, not a guess
   dressed as a finding.
2. **`5` and `10` do not survive excluding Zone at all** - every occurrence
   of either literal in a saturated-dot-fed, output-reaching chain is on a
   Zone-declaring block, i.e. the rim exponents. That means the published
   `5`/`10` counts either did not exclude Zone, or were reading a different,
   narrower population than "every material" states - either way, "5 and 10
   are specular values" does not hold up against this reimplementation.
3. **`1.4426949`, `200`, `250`, `260` and `35` are new, and none of them is a
   plausible shininess.** `1.4426949` is `log2(e)`, the idiom for computing
   `exp(x) = exp2(x * log2(e))` rather than `pow(x, e)` - a real, different
   use of the identical three instructions that the saturated-dot gate does
   not exclude, because an `exp()` curve can just as well be fed by a
   saturated dot product (a falloff, not a power). `200`/`250`/`260`/`35`
   read as further instances of the same problem: a Fresnel-style or
   falloff term sharing the specular idiom's shape and its `N`-something
   operand, not a shininess value in that range.

**Net at that point: `32`, `40` and `300` were the only values this
reimplementation still found plausible as real specular exponents**
(`26.156` did not recur in this sweep at all - a scope difference from the
earlier one, not investigated further).

**Wired 2026-09-01.** `1.4426949` (`log2(e)`) turned out not to need a
policy decision at all: it is a *misclassification*, not an ambiguous real
value - the identical instruction shape computing `exp(x)` rather than
`pow(x, e)` - and `Program::specular_exponent` now excludes it structurally
(a third gate, matching the constant exactly rather than filtering by
magnitude; see its own doc comment). That leaves `200`/`250`/`260`/`35`
genuinely unresolved as "confirmed real" versus "another pow sharing the
idiom's shape", exactly as `32`/`40`/`300` were - and per this project's own
rule, a confidence-80 reading that has survived every discriminator found so
far is wired as read, not held back a second time for an uncertainty already
priced into that number. `mesh::rcs::skin::roles` now calls
`specular_exponent()` per material slot, trusting every answer **except a
literal `0.0`**: `pow(x, 0) = 1` is not a plausible authored shininess, and
that specific value keeps reading as `SpecularPower` patched at draw time
(finding 1 above) rather than as a sixth real constant.

**`200`/`250`/`260`/`35` traced 2026-09-03, over four drafts before the
method was trustworthy - all four kept in this history rather than silently
corrected away, per this project's own evidence rule.**
`Program::specular_exponent_dp3()` now names the winning chain's own `DP3`
(`crates/rcs/src/rcsmaterial/fragment.rs`), so
`crates/render/examples/hd_specular_unresolved_trace.rs` can print its two
operands rather than only the chain's value - the piece the paragraph above
left unread. Two things had to be fixed to look at the right population
first: `hd_specular_patch_census.rs`'s own 16-circuit sweep resolves fragment
blocks through a circuit's *model-variant* table, and that population
contains **none** of `5`/`10`/`40`/`200`/`250`/`260`/`35`/`26.156` at all,
only `0`/`32`/`300` - so a caller has to sweep every `.rcsmaterial` file
directly, the way the original count did, or these four values are
invisible. Doing that (`oag_assets::psarc::Archive` over all seven `PSARC`s,
`1,632` materials - agreeing with `rcsmaterial.md`'s own disc-wide count)
reproduces the population exactly: `260` 48, `200` 16, `250` 12, `35` 8,
alongside `32` 2,764, `0` 2,173, `5` 841, `10` 253, `300` 18, `40` 8.

All 84 occurrences of the four trace to **eight distinct files**: the ship's
own `nitro_body_new.rcsmaterial`/`nitro_perspex_new.rcsmaterial` (`260`),
`zone_death_panel.rcsmaterial`/`zone_death_electricity.rcsmaterial` (`200`,
also ship materials - both are Zone-mode hazard props mounted *on* a craft,
not track geometry), the two weapon materials `hd_bomb.rcsmaterial`/
`hd_mine.rcsmaterial` (`35`), and `05_ubermall`'s inflatable prop
`materials/martin_inflatable2.rcsmaterial` (`250`, present byte-identically
in both `DATA02`'s base copy and `DATA03`'s DLC copy).

**Checked against the Zone-rim hypothesis directly, since `5` and `10` turned
out to be entirely that - and this holds up.** Declaring one of the sixteen
`zone*` parameters `renderer.md`'s own name listing carries (`zoneTexInner`,
`zoneEffectOuter`, etc., hashed with `rcsmaterial::name_hash` rather than
trusted by name) is true for only 4 of the 84 - both DLC and base copies of
one of `martin_inflatable2`'s six variants - and `false` for the rest,
including both `zone_death_*` ship materials despite the name. So unlike
`5`/`10`, these four are **not** primarily a Zone-rim artifact; the name
coincidence on `zone_death_*` is exactly that, a coincidence - the material
is used during Zone mode, not gated by a Zone shader parameter.

**The operand-shape trace took four passes to get right, and the first
three are worth reading because each looked plausible before the next one
broke it.** Draft 1: a `last_writer` search that ignored write masks
entirely reported "every operand carries its own normalize tail", including
annotating an unnamed opcode (`0x3b`, not in nouveau's table) as `<-
normalize` from its position alone - a guess dressed as a name, exactly what
the confidence rubric's sub-50 rule exists to prevent. Draft 2, made
lane-aware (requiring the writer to cover every lane the read swizzle
names) but still treating a `DP3` as a four-lane read when it only ever
consumes three: the tally collapsed to `Normalize` 4 of 168, `Sum` (the
half-vector shape) 0, and flagged one confirmed `32`-resolving block as
anomalous (its operand traced to `EX2`). Draft 3 added a clobber check (a
lane-aware writer can still be stale if a later write touches an
overlapping lane) and, on the strength of that, concluded the method could
not discriminate at all: `NoSingleWriter` 164 of 168, and the same ratio -
`5,354` of `5,528` - on the `32` bucket used as a control.

**Draft 3's own conclusion doesn't survive either, for two reasons found
together.** First, `32` was never a valid control: it carries the same
confidence-80 label and the same lightmap-pow-curve false-positive risk
this page's own sweep already named, so "indistinguishable from `32`" shows
two uncertain populations resembling each other, not that either is real.
The only member of this whole population whose semantics were established
by *reading*, not by instruction shape, is the ship's own `pow(N.H, 40)` -
the worked transcript above naming `ADD R2.xzw, normalize(f[TC1]), {const}`
as `V + L`. Second, correcting `DP3`'s three-lane read (still outstanding
after draft 2's partial fix) and narrowing every other opcode's read width
to the lanes its own destination mask actually writes - a property of a
masked SIMD ALU generally, not a claim about what any specific unnamed
opcode computes - moves the tally again, sharply: `NoSingleWriter` drops to
**0**.

**A fifth check reverses the previous paragraph's own "the addresses don't
match" claim - `0x3a00` is confirmed as exactly the documented block, and
finding that also names the real reason `specular_exponent()` misses it.**
Decoding `0x3a00` and walking every instruction with its byte position
(`crates/render/examples/hd_specular_calibration_check.rs`) shows the
doc's own `@0xNN` addresses are paragraph counts (`byte_position / 16`, not
raw byte offsets - the earlier paragraph's check used byte offsets and so
never found the match) - and under that convention, six independent details
land exactly where `renderer.md`'s worked example says: `@0x13` is a `DP3`
writing `R0.y`, saturated, against `{const}` (`N.L`); `@0x17` is an `ADD`
writing `R2.xzw` against `{const}` (the unusual three-lane mask is a
fingerprint, and it matches); `@0x40`/`@0x43`/`@0x46` are an `LG2`, a `MUL`
whose constant is literally `[40.0, 0, 0, 0]`, and an `EX2`. Address, lane,
saturation and the literal `40` all agree - this is the block.

**So why does `specular_exponent()` return `None` on it?** The `LG2` at
`@0x40` reads register `R0`, and its most recent full writer is `@0x3f`
(paragraph), an **unnamed `0x3b` instruction**, saturated, self-referential
(`R0.w = f(R0, R0)`) - the same "normalize tail" shape this page's operand
work has read throughout. `Program::specular_exponent()`'s own gate
(`dp3_feeding`) requires that writer to be a literally-named `DP3`
(`insn.name() == Some("DP3")`), and `0x3b` is not in the opcode table this
project has named. **That gate is a false negative on at least this one
confirmed instance**: the saturate condition holds, the shape holds, only
the mnemonic check fails, because the hardware idiom this material actually
uses feeds its `LG2` from the fused normalize instruction rather than from a
bare `DP3` directly. Not a claim about a measured rate across the disc -
one instance, confirmed by decode, is what this pass establishes.

**One thing this does *not* call into question: the ship's `pow(N.H, 40)`
reading itself stands.** `@0x24`'s raw operands are `(R2, R2)` - a self-dot,
computing `|half|²` as part of normalizing the accumulator `@0x17` built,
not literally "`normalize(half)` dotted against `normal`" as the transcript's
own prose reads. That is not a new error: the same transcript already names
`@0x17`'s raw register operand `R2` as `normalize(f[TC1])`, which is the
*value* `R2` holds by that point in the block, not its literal operand
either. **The transcript names semantic values, not raw operands** - read
that way, `@0x24` self-dotting the half-vector accumulator to normalize it
is exactly what the surrounding prose describes, and re-deriving the N.H
cross-dot itself is a separate piece of work, not something this check
unsettles.

**Consequence for the operand-shape work: the eight wired `40.0`
occurrences and this one hand-confirmed case are disjoint populations.**
`hd_specular_unresolved_trace.rs`'s "calibration" ran against whichever
block in this file `specular_exponent()` *does* accept (`0x57e0`/`0x7340`) -
neither of which is `0x3a00`, the one actually read by hand - so it was
never calibrated against a known-real case at all, for exactly this reason
rather than the vague one the previous paragraph gave. The
`Sum`/`Normalize`/`Neither` tally (`82`/`58`/`28` of 168) stands as raw,
lane-correct structural data with no calibration - unchanged in number, now
correctly explained. One transcript, `zone_death_panel.rcsmaterial`, kept as
an example of the mechanical shape rather than as evidence of meaning:

```text
[13] op3B R2.xyz, R2.xyzw, R0.wwww
[14] DP3_SAT R2.x, R0.xyzw, R2.xyzw
[15] DP3 R1.w, R0.xyzw, R3.xyzw
[17] DP3 R0.w, R3.xyzw, R3.xyzw
[18] ADD R3.xyz, R1.xyzw, R3.xyzw        <- R1 + R3, two distinct sources
[19] DP3 R2.y, R3.xyzw, R3.xyzw          <- dot(sum, sum)
[20] op3B R3.xyz, R3.xyzw, R2.yyyy       <- scaled by the dot above
[21] DP3_SAT R2.w, R0.xyzw, R3.xyzw      <- the winning DP3
    operand 0: R0.xyzw -> Normalize
    operand 1: R3.xyzw -> Sum
```

**Left wired, not retracted - on the population and Zone-exclusion findings
alone.** Confidence stays at **80**, unmoved: those two hold up and neither
argues these four values are wrong. The operand-shape work across five
drafts settles nothing about meaning for `200`/`250`/`260`/`35` either way -
it corrected real bugs in its own method three times over, confirmed its
attempted calibration block was the wrong one, and then, checking *why*,
found a genuine, confirmed false negative in `specular_exponent()` itself
(the `0x3b`-fed chain above), which the `op3B` section below settles: **not
fixed**, on two independent reasons neither this thread nor its own new
evidence moves - `op3B`/`NRM` sits at confidence ~70 by a considered prior
decision, and the newly-found second usage shape shows `op3B`'s own
semantics are not uniform even under that hypothesis. What is left open:
what feeds a `Sum` chain's own two `ADD` operands, and re-deriving the
ship's `N.H` cross-dot itself now that `@0x24` is read correctly as the
accumulator's own normalize step rather than the cross-dot.

**The `0.0` bucket is closed, 2026-09-02, with the `patch fslot` to
`const@slot` decoder this paragraph used to leave as a next step.**
`fragment::Program::patches` ports it into Rust from
`scripts/ps3-microcode.py`'s `fp_patch_slots`/`fp_patch_map` - the same chain
`docs/formats/rcsmaterial.md`'s "The glass family's second slot" resolved by
hand - and `specular_exponent_slot()` names the code slot a resolved chain's
constant occupies, patched or not. `mesh::rcs::skin::roles` now checks that
slot against `Program::patches(SPECULAR_POWER)` before discarding a `0.0`:
where it is patched, the material's own `.rcsmodel` parameter table (the same
table `Flame::from_material` reads) supplies the real value; only a `0.0`
that is *not* patched, or a material the decoder cannot resolve at all, still
falls back to the shared `32` stand-in
(`mesh::vertex::GpuVertex::specular_exponent`,
`mesh::DEFAULT_SPECULAR_EXPONENT`). Every other resolved value is unchanged,
carried straight into `mesh.wgsl`'s `pow(ndh, in.specular_exponent)`.

**Verified disc-wide before being trusted, not assumed from the mechanism
alone.** `crates/render/examples/hd_specular_patch_census.rs` swept 16
circuits and found the two questions this rests on both close together:
every one of the 62 materials whose `0.0` chain is patched from
`SpecularPower` also authors a non-zero value for it - at 30 to 100,
non-round, a different population from the six shared-literal values above -
and none of the other 233 materials in the `0.0` bucket does. The same sweep
also checked the assumption `patches` rests on: no two distinct declared
parameters of a resolved block ever patch the same code slot, over 4,586
resolved blocks, so asking "does `SpecularPower` patch this slot" has no
ambiguous second answer. Pinned as a disc invariant by
`crates/rcs/tests/specular_power_ground_truth.rs`, over the three models
`rcsmodel_common::PAIRS` already shares with the other `.rcsmodel`
ground-truth binaries.

**This closes a fifth of the `0.0` bucket, not the whole of it.** The
paragraph above originally called the entire bucket "the strongest candidate
for `SpecularPower` patched at draw time"; the census shows that is true for
62 of 295 sampled zeros and leaves the other 233 - four fifths - genuinely
unexplained. Their `0.0` is faithful to the file (nothing here invents a
value for them), and they still fall back to the shared `32`. What the
remaining four fifths' `0.0` actually means is open.

**Superseded 2026-09-04: `Program::dp3_feeding`'s writer search was found
lane-unsound both ways and fixed - every population number in this whole
section, above this paragraph, was measured under the buggy gate and no
longer describes the current disc.** The naive "any nonzero mask" writer
search both credited a `DP3` that wrote the wrong lane (1,679 of 6,141
then-resolved blocks) and missed a real one behind an unrelated write to a
different lane (5,556 blocks, the larger error). Fixed lane-aware and
clobber-checked; independently cross-verified at 10,087 of 10,087
currently-resolved blocks lane-sound (`crates/render/examples/
hd_dp3_feeding_lane_check.rs`), confirming the prediction two paragraphs
above the fix ("`8.3 %`... larger than the naive gate's `1.1 %`... a
lane-correct `dp3_feeding` therefore has two independent effects... it can
subtract false positives and add newly-qualifying blocks") - the resolved
count did rise, not just correct itself: 6,141 to 10,087. The `0`/`32`/
`260`/`200`/`40`/`35`/`300`/`250` population table, the `56.0 %`/`27.6 %`/
`8.3 %`/`8.0 %` fallback breakdown, the `5`/`10` Zone-rim exclusion (still
holds, re-checked) and the `SpecularPower` patch census two paragraphs above
this one are all being re-derived against the fixed gate, tracked outside
this page (see `crates/render/examples/hd_dp3_feeding_lane_check.rs` and
`hd_specular_population_recheck.rs` for the re-measurement tooling and its
first-pass numbers) rather than edited in place here - this page's own rule
for a superseded finding is a dated paragraph noting what changed, not a
silent rewrite of the numbers above it.

**Re-measured 2026-09-05, three of the open handover thread's next steps
taken in order (this page is cited as evidence there, not the other way
round); a fourth was found premature and deliberately not taken.**

1. **`200`/`250`/`260`/`35` operand-shape trace re-run against the doubled
   population** (`hd_specular_unresolved_trace.rs`, unchanged - see its own
   note that no code change was needed). **Read carefully, since a first
   pass at this comparison mixed up filtered and unfiltered counts and
   wrongly called the thread's own number stale.** This tool never applies
   the Zone exclusion - it prints `declares_zone` per block but tallies
   every one - so its total is the *unfiltered* count: **172**, not 168.
   Filtering by `declares_zone=false` in its own output reproduces the
   thread's published Zone-excluded **168** exactly (`200`: 52, `250`: 8,
   `260`: 84, `35`: 24, unchanged from the thread's first pass) - the four
   extra unfiltered occurrences are exactly the Zone-declaring `250`s this
   page's own 2026-09-03 draft already named (`martin_inflatable2`'s base
   and DLC copies, "4 of the 84"). The thread's number was correct; the
   discrepancy was in comparing it against the wrong column. The
   operand-evidence tally over the unfiltered 344 operands: `Sum` 142,
   `Normalize` 138, `Neither` 62, and - new - `NoSingleWriter` **2** (was 0
   at 168 operands). Traced to source: both belong to the ship's own
   `nitro_perspex_new.rcsmaterial` (`260`, `declares_zone=false`, operand 0
   of its winning `DP3`, two separate variants) - **not** the Zone-declaring
   `250` occurrences, so this is a real, small gap in the operand-shape
   method within the already-"confirmed real" `260` bucket, unrelated to the
   Zone reattribution below. Not investigated further - flagged as this
   measurement's own open item.
2. **A handful of newly Zone-declaring blocks read by hand, with a positive
   control and a per-variant check this pass added after the first draft
   lacked both** (`hd_specular_zone_sample.rs`, new).
   `hd_specular_population_recheck.rs`'s own filtered/unfiltered split
   localises where the 64 % Zone-declaring growth actually is: `200`/`260`/
   `35` are identical filtered and unfiltered (zero Zone-declaring blocks in
   any of the three), `5`/`10` are the already-confirmed rim exponents
   (134/76, 3.2 % of the 6,464 Zone-declaring total), and the other
   **96.8 %** sits in the `0` and `32` buckets (2,538 and 3,690
   Zone-declaring occurrences respectively). Sampling those two buckets, one
   to two materials per archive, over 13 distinct `.rcsmaterial` files
   across four archives, three circuits, a DLC copy, and `DATA06`'s
   front-end `/data/fe/rank/materials/medal.rcsmaterial` (a UI rank-medal
   icon with no relationship to Zone-mode gameplay): **every one declares
   the same four-parameter cluster** (`zoneColourTint`, `zoneEffectInner`,
   `zoneBaseInner`, `zoneBaseAltInner`). **This page's own history already
   warns that an uncontrolled operand-shape read is not evidence** ("`32`
   was never a valid control") - so this pass also sampled `5`/`10` directly
   as a positive control, and the result rules out using
   `classify()`'s `Sum`/`Normalize`/`Neither` categories as a discriminator
   here at all: the confirmed rim materials (`glasstest`,
   `temp_testing_mat_diffuse`, `lambertshine`, `jd_simplespecular`,
   `and_tunnelmat`) show the identical mix of shapes the `0`/`32` sample
   does, so "the DP3 chain looks like ordinary specular, not Fresnel" is not
   a claim this method can support either way - dropped, not carried
   forward. **What the control sample does show, and what actually answers
   the question, is the declared-parameter overlap itself**: every `5`/`10`
   material samples the identical `zoneColourTint`/`zoneEffectInner`/
   `zoneBaseInner`/`zoneBaseAltInner` cluster the `0`/`32` majority does
   (`lambertshine` a superset, adding the three `*Outer` siblings) - so the
   cluster's presence cannot be distinguishing "a rim chain" from "an
   ordinary one", because confirmed rim materials and confirmed non-rim
   majority materials both declare it identically. A second check this pass
   added - dumping `declares_zone` across *every* variant of the first
   sampled material rather than only the one variant the main loop's own
   filter selects - shows the cluster is **not** file-level: it toggles
   across contiguous variant ranges of the same file
   (`01_normal_diffuse_specularonalpha.rcsmaterial`'s 70 variants: 0-14
   false, 15-34 true, 35-49 false, 50-69 true). **This one file does not by
   itself settle orthogonality, and is not overstated as doing so**: within
   it, only four variants resolve an exponent at all (16, 21, 51, 56, every
   one `32.0`), and all four sit on the Zone-declaring side of the split -
   so this file alone is equally consistent with "the flag is unrelated to
   the specular chain" and with "only Zone-declaring variants happen to
   carry a chain this method can resolve". **The orthogonality claim rests
   on the rim control above, not on this file**: a confirmed rim material
   declaring the identical cluster is what rules out the cluster as a rim
   detector, independent of whether any given file's Zone and non-Zone
   variants both resolve. Net: the four-parameter cluster measures "this
   variant supports the Zone colour-tint overlay" - which confirmed rim
   materials also carry, for an unrelated reason - not "this chain computes
   a rim exponent". **Only `5`/`10` remain confirmed as Zone-rim exponents;
   the 64 % figure does not mean what "consistent with rim^N chains"
   suggested.** Confidence 85 - the
   declared-parameter overlap against the rim control is direct, controlled
   structural evidence; the per-variant split independently establishes the
   cluster is not blanket file-level boilerplate but does not by itself
   establish orthogonality (see above); 13 hand-read materials against
   several thousand is not an exhaustive or statistical sweep, and the
   mechanism
   (a per-variant Zone render-path flag) is inferred from one file's variant
   pattern, not confirmed against the game's own render-path selection code.
3. **`hd_specular_patch_census.rs` re-run in full, including the subset
   question the thread's raw counts left open.** The aggregate numbers
   reproduce the thread's first-pass figures exactly: 469 resolved-`0.0`
   blocks, 86 patched by `SpecularPower`, all 86 authoring a non-zero value
   (30 to 100), 383 (82 %) unexplained; 4,586 blocks checked for a shared
   patch slot, 0 collisions - the same 4,586 as the pre-fix run, because this
   census's own reachable-variant walk is independent of `dp3_feeding`'s
   correctness. **The subset question was answered by rerunning the same
   harness at the commit before the fix** (`ec762612^`, in a disposable
   worktree, removed after) rather than left as "not checked": of the old
   run's 13 distinct `(material, authored value)` pairs (62 slot
   occurrences over 11 files), **12 survive unchanged into the new 86**: the
   fix is a near-strict superset, not a replacement population. The one
   exception - `05_ubermall/materials/reflectplane_dc_seawater.rcsmaterial`
   slot 310, authored `52` - drops out of the patched-`0.0` bucket in the new
   run; its chain most likely now resolves to a real non-zero value directly
   under the lane-sound gate, landing in `other_nonzero` instead, consistent
   with the fix's known effect rather than a data loss. The 24 new
   `(material, value)` pairs the fix adds span three more files
   (`amphiseum/materials/biodome_reflect`, `dc_windowstest`, `metallic`;
   `modesto_heights/materials/reflectplane_dc_seawater`,
   `windowsdiffusespecular_nonemissive_customr`,
   `windowsnormaldiffusespecular`; `tech_de_ra/materials/biodome_reflect`,
   `temp_testing_mat_colour_spec_alpha`), taking the patched-material count
   from 11 to 20 distinct files. Confidence 95 - exact reproduction of the
   published aggregate, and the subset comparison is a direct historical
   rerun, not an inference.
4. **The fourth next step - folding these numbers into `fragment.rs`'s
   `specular_exponent`/`dp3_feeding` doc comments - was deliberately not
   taken.** That step is conditioned on the numbers having "settled", and
   finding #1's own `NoSingleWriter` anomaly (0 at 168 operands, 2 at 344) is
   a new, unexplained discrepancy this pass raised rather than closed;
   finding #2 also reframes what the 64 % figure means, which needs to
   survive review before it is quoted from `fragment.rs` as settled fact.
   The doc comments' current "not pinned to a disc-wide population count"
   wording stays accurate and is left as-is rather than replaced with
   numbers that might move again.

### The sun is real and it is masked (2026-08-20)

Seven of Talon's Junction's materials were read variant by variant, naming what
each interpolator carries - the per-material work the two refuted general rules
left behind. Three things came out, and the first two change pages above.

**1. Five of the seven carry a Lambert sun diffuse.** The idiom, from
`diffuse_with_specular_from_alpha`'s vertex-coloured variant (block `0x7410`,
86 chunks - the commonest material on the circuit):

```text
@0x0a  DP3 R0.w, f[TC3], f[TC3]          \  normalize the interpolated
@0x0c  op3B R3.xyz, f[TC3], R0.wwww      /  world normal
@0x0f  DP3 R0.w, R3, {..}                <- N . L, against a patched direction
@0x12  MUL H6.xyz, R0.wwww, {..}         <- * a patched colour
@0x14  MOV R3.zw, f[TC5].xxxy            <- the occlusion scalar
@0x15  MAD H6.xyz, R3.zzzz, H6, R2       <- occl * that + the prelit term (TC0)
@0x23  TEX H2, f[TC5].zwzz unit0         <- albedo
@0x25  MUL H3.xyz, H2, H3                <- the whole sum multiplies the albedo
```

with `pow(N.H, e)` added *after* that multiply, structurally separate. The two
patched slots are the only `float3`/`float4` the block declares -
`0x02df31e5` `directionalLight0DirectionWorldSpace` and `0x2dba643d`
`directionalLight0Colour`. Confidence 88 on the diffuse, 75 on the names, which
rest on the declaration table because the `patch fslot` to `const@slot` mapping
is unresolved.

**The `patch fslot` to `const@slot` mapping is resolved (2026-08-20), reading
the glass family's second slot** (`docs/formats/rcsmaterial.md`). Within the
vertex/fragment sub-header, `+0x14` is a `u32` entry count, `+0x18` an array
of `u32` entry-offsets relative to the sub-header, and each entry is
`u16 count` followed by that many `u16` code-slot indices the parameter
patches. Validated against this block before trusting it elsewhere: parsed
this way, `0x02df31e5` (`directionalLight0DirectionWorldSpace`) patches
const-slot `0x9` - the `N.L` `DP3` at `@0x0f`, exactly as read above - and
`0x2dba643d` (`directionalLight0Colour`) patches `0x13`, the diffuse `MUL` at
`@0x12`. Both agree with the instruction-level read this section already had,
which is what makes the chain trustworthy rather than merely self-consistent.
Confidence 85 on the mapping.

**`op3B` is very likely `NRM`** (normalize), confidence ~70: RSX is a G70-class
chip, one generation past the NV30/NV40 Mesa headers `scripts/ps3-microcode.py`
is built from, and G70 fragment shaders are documented to add a dedicated
normalize instruction beyond that instruction set. `op3B`'s usage shape here -
always a self-`DP3` immediately followed by `op3B` combining the original
vector with that dot's result - is `v * rsqrt(dot(v, v))`, and this page's own
comment above already glosses it that way ("normalize the interpolated world
normal") without the opcode table agreeing yet. Not applied to
`scripts/ps3-microcode.py`'s `FP_OPS` table - confidence 70 is the line this
project's own naming rule draws for a rename, and an opcode table entry is the
same kind of claim. `op3D` has no hypothesis at all; seen only in
`etched_glass_tech`, always as `op3D R63, R0, R0` - self-referencing, into the
same "special" destination `SGT` also targets, which is what a predicate or
flag-setting op would look like and is exactly that far from being a reading.

**A second `op3B` usage shape, found 2026-09-03 tracing `200`/`250`/`260`/
`35`'s calibration attempt (above), does not fit the pattern this section
already names - kept here rather than folded into it as if it agreed.** The
ship's own confirmed `pow(N.H, 40)` block feeds its `LG2` from
`op3B R0.w, R0, R0` (paragraph `0x3f`): **both operands are the same
register, with no preceding `DP3` self-dot in the local window**, unlike
every occurrence this section's own transcript shows (`DP3 X, V, V` then
`op3B V, V, X` - two instructions, `op3B`'s second operand the dot's result,
not `V` again). If `op3B` truly is `NRM`, one instruction taking `(V, V)`
directly and writing only `.w` reads as computing the reciprocal-length
scalar in a single step rather than "combine a vector with a precomputed
dot" - a real fused instruction can plausibly do both, gated by which lanes
the caller's own mask asks for, so this is not necessarily a contradiction.
It is, however, a second calling shape this page had not recorded, and it is
exactly the kind of variance that keeps the specular-gate question below.

**Consequently: `Program::dp3_feeding` should not be relaxed to accept
`op3B` in place of a literal `DP3`, on the evidence gathered so far.** Two
independent reasons, not one: this project's own naming rule already holds
`op3B`/`NRM` at confidence ~70 without crossing into a rename (the paragraph
above, deliberately, not an oversight this pass should second-guess without
new evidence strong enough to move it); and the newly-found second usage
shape means "does `op3B` compute the dot this gate wants" is not
uniformly true even under the `NRM` hypothesis - relaxing the gate would
risk trading one confirmed false negative for an unknown number of false
positives, accepting a chain as a real `pow(N.H, e)` read whenever `op3B`'s
own semantics in that instance are not actually a cross-vector dot. Leaving
`specular_exponent()` conservative - reporting nothing rather than a
possibly-wrong reading - is the correct choice until `op3B` itself resolves,
not a gap to route around.

**The shipped `Program::dp3_feeding` gate is itself lane-unsound in a
measured fraction of what it currently resolves - found while trying to
explain the 96 % fallback rate below, and more consequential than that
explanation.** `dp3_feeding`'s writer search only checks `insn.mask != 0` -
"wrote *some* lane of the register" - not whether the writer covered the
specific lane(s) the paired `LG2` actually reads. `LG2` has arity 1 and reads
one lane (its own swizzle, broadcast); `DP3` almost always writes a
*different* subset of lanes (commonly `.xyz`, sharing the register with an
unrelated `.w` write from something else entirely), so "most recent writer of
any lane" routinely credits a `DP3` for a lane it never wrote.
`crates/render/examples/hd_dp3_feeding_lane_check.rs` measured this directly:
of the 6,141 blocks `specular_exponent()` currently resolves, **1,679
(27.3 %) are lane-unsound** - the credited `DP3` did not write the lane the
`LG2` reads, so the value `specular_exponent()` reports for those blocks may
not be the exponent the `LG2`/`MUL`/`EX2` chain actually consumes. This is a
correctness gap in shipped detection code, not a documentation-only finding,
and is filed as its own `## Open` item in the handover thread rather than
fixed in the same pass that found it - both because scope discipline says a
gate change needs its own verification pass, and because fixing it will move
every number measured against the current `6,141`, including the ones below.

**The 96 % `specular_exponent_unresolved` fallback rate `Report` counts
(2026-09-04) has a mostly mundane explanation, re-measured disc-wide with a
lane-correct gate rather than the naive one above.**
`crates/render/examples/hd_specular_unresolved_reasons.rs` categorises every
one of 76,358 fragment blocks, using the lane-correct check (not
`dp3_feeding`'s own naive one - see above): `56.0 %` have an `LG2` whose
feeding register is not lane-soundly written by a saturated, literally-named
`DP3` (`op3B`-fed among them, but not the majority of it - see below);
`27.6 %` have no `LG2` at all, genuinely no `pow`/`exp` curve; `8.3 %` pass
the lane-correct `DP3` gate but `specular_exponent()` still fails (`MUL`/
`EX2` absent, `log2(e)`, or dead code - larger than the naive gate's `1.1 %`
**not because the lane-correct gate is stricter, but the opposite: it is
more permissive, and the arithmetic proves which direction moved.**
`Lg2NotDp3Fed` drops by exactly `5,556` blocks (`48,329` naive to `42,773`
lane-correct) and `Lg2Dp3FedButNoChain` grows by the same `5,556` (`802` to
`6,358`); `NoLg2` and `Resolved` are unchanged in both runs, so those `5,556`
moved *into* the DP3-fed population, not out of it. The mechanism: a `DP3`
writes the lane an `LG2` reads (say `.x`), then something unrelated writes a
*different* lane (`.w`) of the same register before the `LG2` runs. The naive
"most recent writer of any lane" search finds that `.w` writer, sees it is
not a `DP3`, and reports not-fed - a false negative. The lane-correct search
never considers `.w` a candidate or a clobber, since `.w` is outside what the
`LG2` reads, and finds the real `DP3` underneath it. This false-negative
class, at `5,556` blocks, is more than three times the size of the `1,679`
false positives `dp3_feeding` itself carries); `8.0 %` resolve via
`specular_exponent()` itself (unaffected by this file's own gate, still
`6,141` blocks exactly, though `27.3 %` of those are the lane-unsound
population above). **A lane-correct `dp3_feeding` therefore has two
independent effects on the `6,141`, not one: it can subtract false positives
and add newly-qualifying blocks whose `MUL`/`EX2`/`reaches_output` tail
completes now that the DP3 gate itself passes lane-correctly** - the fix, if
made, is not simply "shrink the resolved count to remove the unsound 27.3 %."
**Within the
56.0 %, what actually feeds the `LG2` is still mostly ordinary arithmetic,
not `op3B`, but `op3B` is a larger share of it than the naive gate showed**:
`ADD`/`ADD_SAT` together `53.4 %`, `TEX` `11.4 %`, `MOV` `5.9 %`, `op3B`/
`op3B_SAT` combined `23.1 %` (`19.3 %` + `3.8 %`, up from the naive gate's
diluted `~13.6 %` once the lane-mismatched entries are removed from the
denominator), and a plain **unsaturated** `DP3` - a real `DP3` by name, only
missing the saturate bit this gate also requires - `1.2 %` (down from the
naive gate's `~2.8 %`, most of that difference having been lane-mismatched
entries miscounted as unsaturated `DP3` hits). `LG2` is a general-purpose
primitive shared by fog curves, rim falloffs and other combines that share
the identical `LG2`/`MUL`/`EX2` shape over an unrelated saturated dot or none
at all (this page's own Zone-rim and `log2(e)` exclusions are evidence the
bare shape already over-matches), so most of the 56.0 % still reads as "this
`LG2` was never a specular term" rather than as further confirmed misses -
`op3B` at `23.1 %` of it is a real, non-trivial share though, larger than
first measured.

**2. The colour set's fourth byte is that occlusion scalar**, which closes an
open question above. The vertex program routes `v[colourSet].w` to a spare
channel - `o[TC5].x`, or `o[TC6].z` on `track_wall` - and the fragment program
uses it in **two** places: the `MAD` that gates the sun, and the multiply that
gates the specular. The *lightmapped* variants of the same materials use
`lightmap.a` in exactly those two places. Two independently decoded carriers
filling one role is the check, and it predicts the census this page already
records - 41 % zero, 52 % full, a mask. Confidence 86.

**3. So the sun removal shipped in `mesh.wgsl` was a crude fix for a missing
mask.** `oag-render` applied `sun * (ndl * baked.a)` with `baked.a` at **1.0**
from the no-lightmap placeholder - full sun, unoccluded, on every surface
without a lightmap. The disc applies the same sun gated by a mask that is zero
on 41 % of vertices. Removing the term measured better (16.5 % -> 10.2 %
clipped) because unmasked sun was worse than none; the faithful fix was to
decode the colour set's alpha as the mask and restore the sun behind it.

**Wired 2026-08-20, same day.** `oag_rcs::rcsmodel::Mesh::vertex_light`
now reads all four bytes (`[r, g, b, mask]`), a new
`oag_render::mesh::GpuVertex::sun_mask` field carries it into the shader
(not `colour.a`, which is already the PSP/PS2 boost plume's baked falloff and
this project's own bloom glow mask - a third meaning would have collided with
both), and `mesh.wgsl`'s `lit_texel` restores `sun * ndl * mask` in the
diffuse sum and gates the specular by the same `mask = baked.a * sun_mask`
rather than by `baked.a` alone. Applied to every lit chunk alike, not only the
five material families measured above, for want of the per-material branch
that would tell `track_surface`'s 15-of-301 minority apart - the same kind of
stand-in the shared specular exponent already is, and wrong in the same
direction (a small minority reads lit where the disc's own formula would
leave it ambient-only).

Verified two ways, neither of them "the frame looks better": the decoded
mask's zero/full split off the real disc data reads 41.4 % / 51.7 %,
matching this page's independently-measured census on the same 240,400
vertices; and re-rendering Talon's Junction at a fixed race tick against
itself before the change moves 88.6 % of the frame's pixels and its mean
brightness from 128.7 to 137.3 (both 0-255), which is the direction restoring
a removed light source should move it. Not yet checked against an RPCS3
reference frame pixel-for-pixel - see `HANDOVER.md`.

**What each variant carries, and why a table has to be per variant.** The seven
vertex programs take two shapes and the shape follows the chunk key - the same
quantities on different channels:

| Carried | Lightmapped | Vertex-coloured |
| --- | --- | --- |
| prelit light | `o[TC0].xyz` is a zero constant | `pow(colourSet.rgb, prelitBias) * prelitScaleSpecular` |
| sun-occlusion | `lightmap.a`, sampled | `o[TC5].x` / `o[TC6].z` = `colourSet.w` |
| world normal | `o[TC0..2].w` | `o[TC3].xyz` or `o[TC4].xyz` |
| view vector | `o[TC2].xyz` | `o[TC0..2].w` |
| albedo uv | `o[TC4].xy` | `o[TC5].zw` / `o[TC6].xy` |
| lightmap uv | `o[TC4].zw`, `o[TC6].xy` on `track_wall` | - |
| view depth | `o[TC3].w` | `o[TC5].y` / `o[TC6].w` |

Read by a subagent; the `diffuse_with_specular_from_alpha` block above was
re-read here instruction by instruction before any of it was written down.


## The shader parameter names are a table in the binary, not a wordlist problem

**Recovered 2026-08-24, confidence 95.** `EBOOT.elf` carries the engine's whole
shader vocabulary as an array of `const char *` at **`0x008b7f08`** -
`g_ShaderParameterNames`, 107 entries with one empty. Every hash the `SHO`
tables carry for a parameter, a sampler or a feature token is a name in it, so
the `~crc32` preimages this page and
[rcsmaterial.md](../../../formats/rcsmaterial.md) had been recovering "a few at
a time" against a candidate wordlist are all available at once.

**Why it is 95 and not higher.** Three names that were already known
independently - `lightmap`, `constantAmbientColour` and the vertex attribute
`Uv1` - hash to their own entries here, and every other entry hashes to a word
some shipped shader table actually uses. What keeps it off 100 is that the
table's *purpose* is inferred from its content: nothing in the image references
it. No `lis`/`addi` pair builds its address and the word `0x008b7f08` never
appears as a pointer, which is also why the runtime location of the *values*
is not reachable by a cross-reference - see
[rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md), which hunts
`viewProj` in a memory dump for exactly that reason.

The two that matter most for a matched-pose comparison:

| Name | Hash | Type | What |
| --- | --- | --- | --- |
| `viewProj` | `0x2e7d5f33` | `float4 x4` | the world -> clip matrix |
| `eyePositionWorldSpace` | `0x3466fc0e` | `float3` | the camera position |

Three more settle open readings elsewhere in this tree at a stroke:
`directionalLight0DirectionWorldSpace` (`0x02df31e5`) and
`directionalLight0Colour` (`0x2dba643d`) were this page's own 75-confidence
guess for the sun's two constants and are now read; `positionScale`
(`0x9cc5ab3a`) and `positionBias` (`0xa4972b78`) are the chunk dequantisation
`oag_rcs::rcsmodel` reads at a chunk header's `+0x40` and `+0x30`, named by
the engine in the same terms.

The whole table, in file order:

| `ShadowToAlpha` `0xffb75262` | `HalfBright` `0x855aa07f` | `Sun` `0x960677e8` |
| `ShadowMap` `0x56a86a01` | `FalseLight` `0xaacd10cc` | `ZoneMode` `0x5655520a` |
| `ZoneTrans` `0x0333fd5b` | `NoAlbedo` `0x405d6be9` | `SVC1` `0xe5b8ce51` |
| `SVC0` `0x92bffec7` | `IBL` `0xc2d0b09e` | `Ambient` `0x735324ab` |
| `IleVertex` `0x6a92551b` | `IleLightmap` `0xb04cd0f4` | `Spot0` `0x03501d26` |
| `Spot1` `0x74572db0` | `Spot2` `0xed5e7c0a` | `Spot3` `0x9a594c9c` |
| `ZAlphaOnly` `0x31dc632b` | `AmbientShadow` `0x0b8d3814` | `Static` `0x7893d2ec` |
| `StaticQuake` `0xd29c9ee2` | `RigidBody` `0xdd70bfd5` | `SunOcclusionLightmap` `0x73fc2269` |
| `SunOcclusionVertex` `0x702db2aa` | `time` `0x906b67ba` | `viewProj` `0x2e7d5f33` |
| `view` `0x01025471` | `world` `0xc588eebc` | `eyePositionWorldSpace` `0x3466fc0e` |
| `positionBias` `0xa4972b78` | `positionScale` `0x9cc5ab3a` | `fogColour` `0x3dc31258` |
| `paraboloidReflectionTex` `0x9edd3243` | `paraboloidIblTex` `0x872ed3bd` | `constantAmbientColour` `0x81db67ea` |
| `falseLightDirectionPower` `0xfce47a44` | `prelitScaleSpecular` `0x8670f0be` | `prelitBias` `0x002c73e8` |
| `directionalLight0DirectionWorldSpace` `0x02df31e5` | `directionalLight0Colour` `0x2dba643d` | `directionalLight0Proj` `0x3b00ed5c` |
| `directionalLight0ShadowTex` `0x9becc725` | `directionalLight0LightmapTex` `0xa51b8b14` | `pointLight0PositionWorldSpace` `0x69a6ba16` |
| `pointLight0Colour` `0xead721a1` | `pointLight0Falloff` `0x407290f8` | `textureSpot0PositionWorldSpace` `0xa434de79` |
| `textureSpot0Proj` `0x9f21b213` | `textureSpot0Tex` `0x1f0f1fa1` | `textureSpot0ShadowTex` `0x21c54273` |
| `textureSpot0Colour` `0xa18fdfb3` | `textureSpot0Falloff` `0xb380b94e` | `textureSpot1PositionWorldSpace` `0x73d65e21` |
| `textureSpot1Proj` `0xa2419ba3` | `textureSpot1Tex` `0xa7b378c4` | `textureSpot1ShadowTex` `0xce07294d` |
| `textureSpot1Colour` `0x07f8d407` | `textureSpot1Falloff` `0x7f2ab9d0` | `textureSpot2PositionWorldSpace` `0xd080d888` |
| `textureSpot2Proj` `0xe5e1e173` | `textureSpot2Tex` `0xb506d72a` | `textureSpot2ShadowTex` `0x2530924e` |
| `textureSpot2Colour` `0x3610ce9a` | `textureSpot2Falloff` `0xf1a5be33` | `shadowMatrix` `0x6c1a1be7` |
| `shadowMapTex` `0x730df9ee` | `shadowMapTexSize` `0x86a174d7` | `quakePointA` `0xfda0bc88` |
| `quakePointB` `0x64a9ed32` | `quakeOffset` `0x4bc7a9f1` | `quakeTrackUpNormal` `0x3fe2642d` |
| `distortion` `0x9fc59444` | `refractProject` `0x590bc10e` | `reflectProject` `0xac608eb9` |
| `screenSpaceRefractionTex` `0x88a0df95` | `screenSpaceReflectionTex` `0xec2b3fc2` | `zoneColourTint` `0xa410aa44` |
| `zoneEffectInner` `0x5b79f09f` | `zoneEffectOuter` `0x4290f307` | `zoneBaseInner` `0x1cc42e87` |
| `zoneBaseOuter` `0x052d2d1f` | `zoneBaseAltInner` `0x6f756a07` | `zoneBaseAltOuter` `0x769c699f` |
| `zoneOrigin` `0x0ab4efed` | `zoneTexInner` `0xd5e000d1` | `zoneTexOuter` `0xcc090349` |
| `zoneTexInnerNearest` `0x00e5b679` | `zoneTexOuterNearest` `0x52834137` | `zoneTexVis` `0x1f6f85a3` |
| `zoneAnisoPalette` `0xabfaed85` | `zoneAnisoPaletteOuter` `0xdb88e56b` | `zoneAnisoPower` `0x7d494659` |
| `GradientColour` `0x29b4bba0` | `GradientColour1` `0x87211769` | `GradientColour2` `0x1e2846d3` |
| `GradientColour3` `0x692f7645` | `iblScalePower` `0x7480de6d` | `ambientShadowMatrix` `0x72a9a183` |
| `ambientShadowBlendFactor` `0xe3686260` | `ambientShadowTex` `0xa567d33c` | `globalAlphaScaler` `0x4c13d3af` |
| `auroraBrightness` `0xbfbe5fce` | `auroraOffset` `0xe51e436f` | `auroraColour` `0x46ecec71` |
| `engineTrail` `0xbb48e390` |  |  |

## What was deliberately not read

- **The draw path itself.** `RenderManager_FlushDrawQueue`
  (`0x002d6300`, [above](#the-per-eye-draw-dispatch-and-what-it-says-about-sort-order))
  is the dispatch loop, but no mesh submission, no state setting and no
  shader binding is read inside the per-object vtable call it makes - that is
  inline command-buffer writing and finding it needs a search for RSX method
  constants inside whatever implements vtable slot `0x1c` for each queued
  class, not a call graph.
- **Where `RenderManager+0x630` gets populated.** **2026-09-04: answered -
  see [the enqueue idiom](#the-enqueue-idiom-and-what-the-0x04-key-encodes)
  above.** The "plain sequential submission, not a hidden sort" reading two
  bullets up is retracted for the same reason: `RenderManager_FlushDrawQueue`
  calls `qsort` on the array via `RenderManager_CompareQueueKeys` before the
  dispatch loop runs, and the entry's `+0x04` key is twelve bits of layer over
  twenty of back-to-front depth, Pulse's exact scheme, written by an idiom
  inlined at every producer rather than a single shared function - which is
  also why no literal `0x630` store was ever found *inside the render
  layer's own address range*: none of the five confirmed call sites sit
  there. What's still open is which class each producer call site belongs
  to (below 50, unnamed) and where the depth input (`instance+0x11c`) itself
  gets computed.
- **What the two live-observed queued object classes are.** One vtable at
  `0x00864cb8`, one at `0x00867e58` - see the runtime section above for the
  constructor addresses. Neither sits in the render layer's own address
  range and neither is named; below-50 confidence for any specific class
  guess.
- **`0x005cd700`**, the registry's second constructor. It is byte-similar to
  `ShaderRegistry_Register` and has **no branch to it anywhere in the image**,
  so the base-versus-complete split that
  [mode-manager.md](mode-manager.md) resolves by call site cannot be resolved
  here - these are self-registering nodes built by static initialisers, not a
  class hierarchy, so that rule says nothing about which is C1 and which is C2.
  Below 50; the hypothesis is recorded and the function is left unnamed.
- **`SortRoot.cpp`'s four functions** (`0x002dbe08`, `0x002dbe50`,
  `0x002dbe98`, `0x002dbf40`). All four are constructors writing the same two
  vtables and the `__FILE__` at `+0x30`. They decompile as two identical
  pairs: `0x002dbe08`/`0x002dbe50` are byte-for-byte identical to each other,
  as are `0x002dbe98`/`0x002dbf40`, which additionally tail-call
  `0x00327500`. **"None has a caller" (recorded 2026-08-18) is wrong for one
  of the four**, corrected 2026-08-25: `0x002dbe50` has six real
  `UNCONDITIONAL_CALL` xrefs (`0x00137690`, `0x00151ad8`, `0x00154cf0`,
  `0x00137e88`, `0x001525f8`, `0x00155088`), each calling it as the first
  operation of its own constructor, before setting its own vtable - the
  textbook base-subobject-constructor call shape
  [mode-manager.md](mode-manager.md) resolves by call site elsewhere on this
  page. That separates the `0x002dbe08`/`0x002dbe50` pair: `0x002dbe50` is
  the base (C2) constructor, `0x002dbe08` the complete (C1) - still below 50
  for an actual class *name*, so nothing is renamed, but the base-versus-
  complete split itself is now evidenced rather than blocked. The
  `0x002dbe98`/`0x002dbf40` pair remains genuinely callerless on both halves
  (`get_xrefs_to` on each returns only its own `.opd` self-entry at
  `0x00883050`/`0x00883058`) - re-verified 2026-08-25, unchanged.

  **The six callers argue against "draw-order root" being SortRoot's
  confirmed use, not for it.** All six sit at `0x00130000`-`0x00156000`,
  outside this page's own render-layer span (`0x00279xxx`-`0x002ecxxx`); one
  (`0x00137690`) cites `../../../Code/System/System\LinkObj.h` in a pooled-
  allocator call, consistent with `SortRoot` being a generic sorted
  intrusive-list container descended from a near-universal object base
  (`0x00328d48`, itself called from 60+ sites spanning both this address
  range and the render layer proper, including `MeshImporter_Construct`) -
  not evidence tying it to render draw order specifically. Neither of
  `0x002dbe08`'s or `0x002dbe98`'s/`0x002dbf40`'s complete-object forms has
  any caller either, direct or indirect, so no render call site instantiates
  a bare `SortRoot` that this scan can find. Whether the renderer's own
  transparent-draw ordering uses this class at all is still open - below 50,
  so per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)
  the hypothesis is written here and nothing is renamed.

  **2026-08-25, continued: the six callers are three classes, not six.**
  They pair up by identical decompiled bodies exactly like `SortRoot.cpp`
  itself does: `0x00137690`/`0x00137e88` (class "X"), `0x00151ad8`/`0x001525f8`
  ("Y"), `0x00154cf0`/`0x00155088` ("Z") - each pair sharing one vtable
  address (`0x008aa838`, `0x008ab114`, `0x008ab27c`) and tail-calling one
  shared per-class finisher (`0x00135b90`, `0x0014fa08`, `0x00154858`).
  All three share one shape: each allocates N identical 0x2080-byte
  sub-objects through a debug-tracked pooled allocator that cites
  `LinkObj.h:542` (N = 4 for X, up to 11 for Y, 1 for Z), configures floats
  on them by hash-keyed property lookup (`FUN_00677008`/`FUN_00677018`,
  the same "look up by name, apply if found" shape the `SHO` parameter
  binder uses, unconnected to it otherwise), and builds reciprocal-of-delta
  interpolation tables over some of those fields - the arithmetic shape of a
  piecewise curve's per-segment slope, not anything render-specific.
  Followed one dead end here: each class also carries a second vtable
  (`0x00864230`/`0x00864ab8`/`0x00864b38`) sharing all but three entries
  across the three classes - a second interface with three type-specific
  overrides, not the class-name string a first read of the layout suggested;
  no string was found there. **None of the three is named** - the pooled
  N-instance-plus-curves shape reads as an emitter or spawn-point framework
  of some kind, but that is a shape, not a name, and below 50 per ADR-0005.
  The productive next move is identifying *that* framework (the pooled
  allocator and the two helper functions it shares across all three classes
  are likely reused well beyond these three), not chasing `SortRoot` further.
- **Post-processing has no `.cpp` of its own**, and that turned out to be the
  wrong place to look: it is a set of named shader programs, not a class. See
  [the shader section](#shaders-are-in-exactly-two-places-and-neither-is-a-file-type-on-the-disc).
- **Most of `RenderManager`'s methods.** This page now names its constructors,
  its singleton and its per-eye `Prepare`/`Flush` pair
  ([above](#the-per-eye-draw-dispatch-and-what-it-says-about-sort-order)), but
  no other method and no vtable slot. The 512-byte table, the two matrix
  arrays and the 768 KiB buffer are all still unexplained.
- **Everything about `Model.cpp`, `Billboard.cpp`, `Font.cpp`, `Pen.cpp` and
  the `Fx*` family** beyond where they start.

## The name hash is CRC-32

**Confidence 92**, recovered 2026-08-18 while checking whether `0x003b31c8` was
the second binder this page named. It is not - it is Detonator scoring's static
initialiser - but it is where the hash is visible, because it hashes seven names
in the clear and stores each result.

`Crc32_HashString` (`0x005a2090`) is a textbook table-driven CRC-32:

```c
u32 Crc32_HashString(const u8 *s) {
    if (!s || !*s) return 0;
    u32 crc = 0xffffffff;
    do { crc = (crc >> 8) ^ table[(crc ^ *s++) & 0xff]; } while (*s);
    return ~crc;
}
```

with the 256-entry table at `0x008aea18 + 0x200` (the index is masked `0x3fc`,
i.e. `(crc ^ byte) & 0xff` scaled by 4). **Callers store the complement of what
it returns** - `*(u32 *)(obj + 0xf4) = ~Crc32_HashString(name)` - which is the
raw CRC register before the final xor, and is exactly the "complement of a hash
computed from the parameter name" the binder at `0x003af980` compares against.

**Confirmed against real data rather than by shape alone.** The parameter hash
`0x2e7d5f33` appears in the `SHO` block of every `.rcsmaterial` examined, at
register `0x0100` with count 4 - a four-row matrix. `~crc32("viewProj")` is
`0x2e7d5f33`. A guessed name landing on a 32-bit hash is not a coincidence, and
the declared shape agrees with the name.

**What this unlocks, and what it does not.** Naming any parameter is now a
wordlist problem rather than a cryptographic one. It is not solved by it: a
sweep of ~2,000 plausible Cg and engine parameter names over the other thirteen
shared hashes matched **none**, so the remaining names are not the obvious ones
and the productive source is likely the microcode's own symbol usage or the
`_vp`/`_fp` program names.

### It also over-names `ShaderRegistry_Register`

The same call site registers `bomb_score` and `Data/XML/DetonatorScoring.xml`
through `ShaderRegistry_Register` (`0x005cd6d8`). So the function is a **generic
named-node registry constructor** and the `ShaderRegistry_` prefix claims a
subsystem it does not own; the 62 registrations whose payloads begin `SHO\x08`
are one use of it among others. Left renamed for now rather than churned, and
recorded here because [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md)
makes this page authoritative over the database.

## The resolve's full-screen colour inputs, enumerated (2026-08-30)

The section ["`scaleAdd` re-read"](#scaleadd-re-read-and-the-parameter-names-are-literals-2026-08-20)
above left one thread hanging: `fullscreenTintColour` arrives at the binder as a
*pointer argument*, so "the caller holds the current value and passes it in" -
and the caller was never chased. This section chases it, and then chases the
other three colour-shaped parameters beside it, because the useful result turned
out to be the **complete enumeration** rather than the one parameter.

**The headline.** The post chain's resolve pass has exactly four full-screen
colour inputs, and **none of them is fed from the per-stage `.effectSettings`
table** (`g_effect_settings_stages`, `0x00c7efb0`, see
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)):

| Parameter | Program | Source | Verdict |
| --- | --- | --- | --- |
| `fullscreenTintColour` | both resolve variants | `g_FullscreenTintColour` (`0x00c50f00`) | **provably always `(0,0,0,0)`** |
| `saturation` | correction variant only | photo mode, via `Renderer_SetColourCorrection` | photo mode |
| `finalScale` | correction variant only | photo mode, greyscale by construction | photo mode |
| `finalBias` | correction variant only | photo mode, greyscale by construction | photo mode |

A fifth full-screen colour exists one pass away - `colourScale` on
`downsamplescalefiltercolourscale_fp` - and is **not** resolved here; see
"Still open" at the end.

This is a stronger negative for "HD has a 2048-style whole-frame composite
grade fed by the stage table" than the per-key getter searches in
[zone-effectsettings-loader.md](zone-effectsettings-loader.md), because it
enumerates the *inputs* exhaustively instead of searching for a consumer.

### Program state and TOC, verified first

Per [memory.md](memory.md)'s two-TOC defect, every function read below had its
OPD entry read before its TOC-relative loads were trusted:

| Function | OPD entry | TOC | Ghidra's default correct? |
| --- | --- | --- | --- |
| `FUN_003e3268` (present path) | `0x0088c4c8` | `0x008bd3c4` | no |
| `FUN_003df7c0` (present tail) | - | `0x008bd3c4` | no |
| `0x005e29c0` (resolve binder) | `0x008a1470` | `0x008bd3c4` | no |
| `0x005e3f68` (correction binder) | `0x008a1480` | `0x008bd3c4` | no |
| `0x003df0d8` (colour-correction setter) | `0x0088c458` | `0x008bd3c4` | no |
| `0x0023f568` (photo mode) | `0x0087feb0` | `0x008ad4d8` | **yes** - below `0x32d5e0` |
| `0x003e3e50` (screen fade) | `0x0088c4d0` | `0x008bd3c4` | no |

Two independent self-checks that the `0x008bd3c4` reading is right, not merely
self-consistent: `lfs f2,-0x56d8(r2)` resolves to `0x008b7cec` = `0x3f800000` =
the `1.0` this page already documents reaching `scaleAdd`; and the eight
parameter-name slots at `+0x1f94..+0x2014` resolve to eight consecutive
readable ASCII strings (`scale`, `scaleAdd`, `scaleFeedback`,
`fullscreenTintColour`, `saturation`, `finalScale`, `finalBias`, plus the two
program names). A wrong TOC produces a plausible wrong string, never eight
coherent ones in a row.

`Environment_UpdateStageBlend` (`0x003da540`) and `Scene_PrepareFrame`
(`0x003aa888`) were checked present under their recovered names before any of
this, so the database is the one the other pages describe.

### `fullscreenTintColour` is a one-shot global that nothing ever sets

The value reaches the binder in **`r10`**, the eighth GPR argument. At both call
sites in each present function the instruction immediately before is
`lwz r10,-0x56dc(r2)`, and against TOC `0x008bd3c4` that slot is `0x008b7ce8`,
holding **`0x00c50f00`** - a 16-byte `.bss` object, hereafter
`g_FullscreenTintColour`.

The binder itself (`0x005e29c0`, and identically `0x005e3f68`) does two things
with it and nothing else:

```text
005e2cb8  if (!inited[+0x250]) { chain[+0x260] = (0,0,0,0); inited = 1; }
005e2cf8  if (r10 == 0) r10 = &chain[+0x260];      // 005e34cc, the zero default
005e2d00  slot = chain[+0x248];                     // "fullscreenTintColour"
005e2d5c  lvx v0, 0, r10                            // read the float4
005e2d74  bind(slot, v0)
```

`lvx` only - the callee never writes through the pointer. `0x005e30bc` is the
same read on the alternate bind path, and `0x005e4300`/`0x005e487c` are the
correction binder's pair.

**The caller then zeroes it.** Immediately after the resolve returns,
`FUN_003e3268` runs

```text
003e3838  li   r0, 0
003e383c  lwz  r9, -0x56dc(r2)          ; r9 = 0x00c50f00
003e3844  stw  r0, 0xe0..0xec(r1)       ; a zeroed float4 on the stack
003e3858  lvx  v0, r1, r0
003e385c  stvx v0, 0, r9                ; *g_FullscreenTintColour = (0,0,0,0)
```

and `FUN_003df7c0` does the same at `0x003dfa50`-`0x003dfa74`. That is the
shape of a **one-shot per-frame request**: something sets the tint, the present
path applies it and clears it.

**Nothing sets it.** Four checks, the last two independent of Ghidra:

1. The literal `0x00c50f00` occurs **exactly once** in the whole 21 MB image, at
   the TOC slot `0x008b7ce8`. There is no second pointer to it and no pointer
   into its page.
2. Module 1 cannot reach that slot at all: `0x008b7ce8 - 0x008ad4d8 = 0xa810`,
   past a signed 16-bit displacement, so none of that module's 11,037 functions
   can name it with a `d(r2)` load.
3. A raw scan of the executable's text segment for **any** instruction with
   `rA == r2` and displacement `0xa924` (`-0x56dc`) finds **nine**, matching
   Ghidra's own list exactly. Three are in module 1 (`0x000aab68`,
   `0x000ab3f0`, `0x0067e7f0`) and resolve against `0x008ad4d8` to an unrelated
   slot. The remaining six are the four load-and-pass sites (`0x003dfa0c`,
   `0x003dfbb0`, `0x003e37f4`, `0x003e3ca8`) and the two clears (`0x003dfa54`,
   `0x003e383c`).
4. Both binders only ever `lvx` from the pointer, so no write escapes through
   the callee either.

So `fullscreenTintColour` is **bound every frame with the value `(0,0,0,0)`** -
which is not the same as "not bound", and is why a microcode read alone would
never have caught it. In the resolve's
`out = saturate(feedback * scale + bloom * scaleAdd + tint)` the term is inert.

**Confidence 85.** The enumeration is airtight for PPU code in the ELF's own
text segment. It cannot speak for the four overlay spaces Ghidra reports on this
program, nor for an SPU module DMA-ing into PPU memory - neither was checked,
and both are implausible for a tint but neither is excluded.

A consequence worth recording for the reimplementation: `oag_render::post::hd_bloom`'s
module header lists the feedback mix and tint among the things it deliberately
does not model. For the tint half that is now **provably correct** rather than a
deferral.

**A closed lead, so nobody reopens it.** `FUN_0067e7f0` is a two-instruction
`lwz r3,-0x56dc(r2); blr` and looks exactly like `float4 *GetFullscreenTint()`.
It is not. Its OPD entry (`0x00873940`) gives it TOC `0x008ad4d8`, so the slot
it reads is `0x008a7dfc`, which holds a function descriptor. It never touches
`0x00c50f00`.

**A second closed lead: the `0x00c81a80` / `0x00c81a5c` adjacency is
coincidence.** They are `0x24` apart and it is tempting.
`Scene_PrepareFrame`'s `Scene.Texture Colour` output at `0x00c81a5c` is written
as `0x3aac(r30)` off `iVar8 = 0x00c7dfb0`; the object at `0x00c81a80` is a
separate global reached through its own TOC slot `0x008b7c38`. Different slots,
different objects.

### The correction variant's three extra colour parameters are photo mode

`FUN_003e3268` calls **two** binders on mutually exclusive arms, and the switch
is a single byte:

```text
003e3798  lbz  r0, 0x3e0(r31)      ; r31 = g_RenderGlobals = 0x00c81a80
003e379c  cmpwi cr7, r0, 0
003e37a0  beq  cr7, 0x003e3c60     ; -> plain downsamplescaleaddfeedback_fp
          (fall through)           ; -> downsamplescaleaddfeedbackcorrection_fp
```

The correction binder `0x005e3f68` resolves **seven** slots where the plain one
resolves four, the three extra being `saturation` (`+0x328`, from `f4`),
`finalScale` (`+0x338`, a float4 read through stack parameter 10) and
`finalBias` (`+0x348`, a float4 read through stack parameter 11). At the call
site (`0x003e3830`) those two pointers are `r31 + 0x3f0` and `r31 + 0x400`, and
`saturation` is `lfs f4, 0x24(r23)` off the FunkLayer chain object
(`0x008c3520`).

**The setter is `Renderer_SetColourCorrection` (`0x003df0d8`)**, five
instructions of it:

```c
void Renderer_SetColourCorrection(bool on, ..., const float4 *scale,
                                  const float4 *bias, float a, float b) {
    g_RenderGlobals[0x3e0] = on;
    if (on) {
        g_RenderGlobals[0x3f0] = *scale;      // finalScale
        g_RenderGlobals[0x400] = *bias;       // finalBias
        g_RenderGlobals[0x3e4] = a;
        funkLayer[0x24]        = b;           // saturation
        *(float *)0x00c51118   = a;
    } else {
        *(float *)0x00c51118   = *(float *)(0x008c3640 + 0x28);
    }
}
```

**It has no direct caller anywhere.** Its only reference is the cross-TOC
trampoline `FUN_00678ee8`, whose body is `std r2,0x28(r1); addis r2,r2,1;
subi r2,r2,0x114; b 0x003df0d8` - and `0x10000 - 0x114 = 0xfeec`, which is
exactly `0x008bd3c4 - 0x008ad4d8`. That is an independent corroboration of
[memory.md](memory.md)'s two-TOC finding *and* of the trampoline reading, from a
function neither was derived from.

The trampoline has exactly two callers, both in **`FUN_0023f568`**: `0x0023fa80`
with `r3 = 0` (disable) and `0x0023ff64` with `r3 = 1` (enable).

**`FUN_0023f568` is photo mode.** It sits at `0x0023f568`, below `0x32d5e0`,
so it is in the module Ghidra's default TOC is *correct* for - which is why its
string references can be read directly here when they cannot be elsewhere in
this binary. Resolving its `d(r2)` loads against `0x008ad4d8` gives, in order:

```text
Photo Mode Controls Bar, InGame Taking Photo Redirect, Race End Photo,
Photo Camera Mode, Photo Hud Display, Photo Blur, Photo Use DOF,
DOF Focus Distance, DOF Aperture, Exposure, Saturation,
Photo Mode Controls, Photo Toggle Help Display, Shutter Speed,
MSC_PHOTO_DEPTH, InGame Taking Photo Transition Screen, MSC_PHOTO_MBT,
MSC_PHOTO_MBS, FE_MOTION_TRACK, FE_MOTION_SHIP, Fov
```

Renamed `PhotoMode_Update` (confidence 78 - the strings are decisive about
*what* the function is, less so about it being the update rather than the
apply half of it; its own sole caller `FUN_00241940` is unread).

At the enable site both float4s are built **grey**: `finalScale` is one GPR
word splatted into all four lanes (`0x0023ff24`-`0x0023ff30`), `finalBias` is
`f30` splatted into all four (`0x0023ff34`-`0x0023ff44`). A per-channel colour
grade is therefore *structurally impossible* through this path - it can only
scale and bias luminance uniformly, which with `saturation` is exactly a
photographic exposure/saturation control and not a hue shift.

The three values all derive from **one float at `[sp+0x7c]`** scaled by three
different TOC constants (`f1 = (x - k1) * k2`, `f2 = x * k3`, and the splatted
scale). **Which photo-mode slider that float is has not been established** -
both `Exposure` and `Saturation` are strings in the same function and neither
was tied to that stack slot.

Confidence 85 on the chain (setter -> trampoline -> photo mode); the `0x3e0`
flag, the two pointer arguments and the greyscale construction are each read
directly off instructions.

### The names this section lands

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x005e29c0` | function | `Post_BindResolveParams` | 85 |
| `0x005e3f68` | function | `Post_BindResolveCorrectionParams` | 85 |
| `0x003df0d8` | function | `Renderer_SetColourCorrection` | 85 |
| `0x0023f568` | function | `PhotoMode_Update` | 78 |
| `0x00c50f00` | data | `g_FullscreenTintColour` | 88 |
| `0x00c81a80` | data | `g_RenderGlobals` | 72 |
| `0x005e1140` | function | `Post_BindFilterColourScaleParams` | 85 |
| `0x003def38` | function | `Post_SetColourScaleRate` | 80 |
| `0x003def50` | function | `Post_SetColourScaleTarget` | 80 |
| `0x003def68` | function | `Post_SetColourScaleImmediate` | 80 |

`Post_BindResolveParams` and `Post_BindResolveCorrectionParams` are named from
the program-name string each one loads from its own TOC -
`downsamplescaleaddfeedback_fp` (`+0x1fb8`) and
`downsamplescaleaddfeedbackcorrection_fp` (`+0x2004`) - and from the four
respectively seven parameter names each resolves. `g_FullscreenTintColour` is
named by the one parameter it feeds. `g_RenderGlobals` stays at 72 and is
deliberately vague: it is provably the render TU's own global state object
(the colour-correction flag, the fade triple, the render-target ladder index),
but nothing here establishes what the class is called or what most of its 20 KiB
holds.

`Post_BindFilterColourScaleParams` is named the same way, from
`downsamplescalefiltercolourscale_fp` (`+0x1f78`) and `colourScale` (`+0x1f84`).
The three `Post_SetColourScale*` setters are named from the single field each
writes and from the parameter that field ends up in; 80 rather than 85 because
"colour scale" is the shader's word and "rate/target/immediate" is a reading of
three one-line functions, well supported by the consumer but not named by
anything.

`FUN_003e0668` and `FUN_003e3e50` are **not** named: they are the same routine
twice and nothing here establishes which is which, or what the second one's two
extra arguments select.

### `colourScale` is the fifth input, and it is inert too

`downsamplescalefiltercolourscale_fp` declares a `colourScale` parameter (name
string `0x007cdd10`, TOC slot `0x008bf348`) which the plain and correction
resolves do not have. Its binder is `Post_BindFilterColourScaleParams`
(`0x005e1140`, OPD `0x008a1450`, TOC `0x008bd3c4`), which binds it into slot
`+0x168` at `0x005e16bc` from a vector argument.

**Its two callers are the same routine twice.** `FUN_003e0668` and
`FUN_003e3e50` are instruction-for-instruction the same screen-fade pass with
different argument counts (three GPRs versus one) - `lvx` the same three fields
of `g_RenderGlobals`, the same five `vspltw`/`fsel` lanes, the same `> 0.0`
gate, the same tail. So there is no second mechanism hiding behind the second
caller, which was the obvious way this could have been mis-read.

The pass keeps a **rate-limited fade triple** in `g_RenderGlobals`:

| Field | Role |
| --- | --- |
| `+0x3b0` | rate, per lane |
| `+0x3c0` | target colour |
| `+0x3d0` | current colour - stepped toward the target, then passed as `colourScale` |

Three two-instruction setters exist for it, all of them writing a caller-supplied
`float4` straight into one field:

| Address | Writes | Name |
| --- | --- | --- |
| `0x003def38` | `+0x3b0` | `Post_SetColourScaleRate` |
| `0x003def50` | `+0x3c0` | `Post_SetColourScaleTarget` |
| `0x003def68` | `+0x3c0` **and** `+0x3d0` | `Post_SetColourScaleImmediate` |

**None of the three is called from anywhere in the executable.** Each address
occurs exactly once in the whole image - in its own OPD entry (`0x0088c430`,
`0x0088c438`, `0x0088c440`) - and *those* entries are referenced by nothing, so
there is no function-pointer route either. No `b` or `bl` targets any of them,
which is what rules out the cross-TOC trampoline route that
`Renderer_SetColourCorrection` does use: that one has a trampoline
(`FUN_00678ee8`, a plain `b`), and these have none.

Nor is the triple written inline anywhere else. Every function that can name
`g_RenderGlobals` at all uses `lwz d,-0x578c(r2)`, and all 54 such instructions
in module 2 fall between `0x003de200` and `0x003e3e60`; a scan of that band for
any store at displacement `0x3b0`-`0x3df`, and for any `li rX,0x3b0/0x3c0/0x3d0`
feeding an indexed `stvx`, finds only the setters above, the two fade readers,
and one initialiser.

**That initialiser fixes the value at identity.** At `0x003dfe40`, inside
`FUN_003dfc28`, the triple is seeded from two TOC-held float4s: rate from
`0x007b2ee0` = `(1, 0, 0, 0)` and colour from `0x007b2ef0` = `(0.1, 0, 0, 0)`,
each **splatted from lane 0** - so `target = current = (1,1,1,1)` and
`rate = (0.1, 0.1, 0.1, 0.1)`. The fade's own gate is `current.x > 0.0`
(`lfs f0,-0x56fc(r2)` = `0.0`), so the pass *does* run - and multiplies the
frame by `(1,1,1,1)`.

So `colourScale` is bound every frame with the identity, for the same structural
reason `fullscreenTintColour` is bound every frame with zero: the API that would
move it exists and is unreachable. Confidence 82 - the enumeration is the same
kind as the tint's and equally exhaustive over PPU text, but it rests on the
initialiser being the only seeding and on the two fade routines being the only
readers, neither of which has a second independent route.

### What this closes, for the Zone question

All five of the post chain's full-screen colour inputs are now accounted for,
and **none is reached from `g_effect_settings_stages` or from anything Zone
touches**:

| Input | Shipped value | Driven by |
| --- | --- | --- |
| `fullscreenTintColour` | `(0,0,0,0)` every frame | nothing |
| `saturation` | photo-mode only | `PhotoMode_Update` |
| `finalScale` | photo-mode only, grey | `PhotoMode_Update` |
| `finalBias` | photo-mode only, grey | `PhotoMode_Update` |
| `colourScale` | `(1,1,1,1)` every frame | nothing |

**HD has no live whole-frame colour grade.** That is the opposite of 2048,
where the equivalent table *is* applied as a composite pass
([zone-effectsettings-loader.md](zone-effectsettings-loader.md)), and it means
whatever recolours track and scenery in HD's Zone mode has to act **before** the
resolve - per-material or per-light - not on the composited frame. The composite
route is now closed by enumeration rather than by failing to find a consumer.

**A play observation corroborates the enumeration, and nearly on its own.** A
driven Zone race screenshotted twice ~45 s apart from the same camera angle
shows stage 0 ("Sub-Venom") with track, sky and buildings all tinted cyan, and
stage 21 ("Flash") with a **purple** track and walls beside **yellow**
buildings. A resolve-stage grade is
`saturate(scene * scale + bloom * scaleAdd + tint)`: the multiply preserves the
ratio between any two pixels' channels, so two surfaces that read as the same
hue before it read as the same hue after it. Turning one purple and the other
yellow needs their pre-grade hues to have differed substantially already, which
is the opposite of both looking cyan one stage earlier.

Suggestive rather than proof, and worth saying why: the `saturate` is a
nonlinearity, and a strong additive `tint` followed by clipping *can* separate
hues that were close. So this is a second, independent argument for the same
conclusion at confidence 75, not a replacement for the enumeration - which is
what actually closes the route, and does so without needing any of this.

What this does **not** settle: what does drive the visible Zone recolour. The
`FunkLayerColour2d_fp` flat-colour quad is untouched by this pass and is the one
remaining post-chain program that could paint a full-screen colour by a
different route; and the per-material side - `Environment_UpdateStageBlend`'s
own outputs at `0x00c81a5c` and the `g_effect_settings_stages` rows - is where
[zone-effectsettings-loader.md](zone-effectsettings-loader.md) already says the
search belongs.

## `FunkLayerColour2d_fp` is a solid-colour quad, and its colours are constants (2026-08-30)

The enumeration above left `FunkLayerColour2d_fp` as the one post-chain program
that could paint a full screen by a route the resolve does not cover. It is now
read end to end, and it is a clean negative for the Zone question.

**The programs.** `FunkLayerColour2d_vp` (block `0x0092ea80`) declares exactly
one parameter - `colour`, hash `0x05079a31`, `float4 x1`, register c467 - one
attribute (`position`), and three instructions:

```text
MOV o[POS].xy, v[0].xyxx
MOV o[POS].zw, c[210].xxxy
MOV o[TC0],    c[211] | END        ; c[211] == register 467 == colour
```

`FunkLayerColour2d_fp` (block `0x0092ce00`) declares **zero** parameters and
**zero** samplers, and is one instruction:

```text
MOV H0, f[TC0] END
```

So every pixel it writes is the same constant, and **it never samples the
framebuffer**. Read with `scripts/ps3-microcode.py` and `scripts/ps3-sho.py`,
whose block framing validates itself; confidence 92.

**Its one draw wrapper**, `FunkLayer_DrawColourQuad` (`0x003cbc58`, OPD
`0x0088c108`, TOC `0x008bd3c4`), takes the colour as a `float4 *` in `r4`,
`lvx`es it at `0x003cbce8`, and binds it into the `colour` slot. The programs
and the slot index come from two globals a resolver fills once:
`FunkLayer_ResolvePrograms` (`0x003cbdb0`, called from `FUN_003e12e0`, the
frame's render-target set allocator) writes `g_FunkLayerPrograms`
(`0x00c7df20`) `+0x0`/`+0x4` with the vp/fp and `g_FunkLayerParamSlots`
(`0x008c2c98`) `+0x0` with `colour`'s slot index.

**Only one function in the module reads those two entries for a draw.** Walking
all 13 functions between `0x003cb868` and `0x003cd850` with real boundaries and
tracking both global pointers through register copies, the offsets each reads
are:

| Function | slot offsets | program offsets | binds |
| --- | --- | --- | --- |
| `0x003cbc58` | **`0x0`, `0x4`** | **`0x0`** | 2 |
| `0x003cbdb0` (resolver) | - | `0x0`-`0x18` | 0 |
| `0x003cc490` | `0x8`,`0xc`,`0x10` | `0xc` | 0 |
| `0x003cc750` / `0x003cce10` / `0x003cd038` | `0x4`,`0x10` | `0x8` | 0 |
| `0x003cca80` | `0x8`,`0xc`,`0x18`,`0x1c` | `0xc`,`0x18` | 2 |
| `0x003cd2e0` (register/release) | - | all | 0 |

`0x0`/`0x4` is Colour2d; everything else is the Fx, FxUv, Copy and CopyBlend
family.

**Every call site passes a constant.** `FunkLayer_DrawColourQuad` has eleven
callers. One is in `Scene_PrepareFrame` (`0x003ac928`), and the colour it
passes is built inline three instructions earlier:

```text
003ac8f8  lis  r0, 0x3f80          ; 1.0
003ac8fc  stw  r0, 0x25c(r1)       ; w = 1.0
003ac900  stw  r0, 0x250(r1)       ; x = 1.0
003ac910  stw  r9, 0x258(r1)       ; z = 0.0   (r9 = 0)
003ac914  stw  r9, 0x254(r1)       ; y = 0.0
```

- an opaque **red** `(1, 0, 0, 1)`, hard-coded, with nothing from the stage
table anywhere near it. The other ten are all in `FUN_003b8550`, which resolves
`FunkLayerCorruption_vp`/`_fp` and their `powerFactors`/`color`/`texture`
parameters against its own TOC - the glitch overlay - and builds each quad's
colour on its own stack too.

**So Colour2d cannot be the Zone recolour**, and the reason is not only that it
is a flat fill: a flat quad under a multiply blend *would* be a legitimate
full-screen tint, so the shader's shape alone would not have settled it. What
settles it is that no call site is fed anything but a constant.

Confidence 85. The eleven call sites are an exhaustive `bl` enumeration, and
the two globals' offsets were walked with real function boundaries; what is
*not* established is the blend state at each site, which is why the argument
above is made on the arguments rather than on the fragment program.

### `Texture_BuildGcmRegisters`, and what it settles about a `.gtf`'s `remap`

`0x005a9998`, **confidence 75**. Found 2026-09-02 while corroborating
[`gtf.md`](../../../formats/gtf.md)'s reading of the `+0x10` `remap` field,
which until then rested on the published `CELL_GCM_REMAP_MODE` packing plus
what it predicted about the disc's own files - nothing from the executable.

Seven texture-creation wrappers call it (`0x005a9d28`, `0x005aa1d8`,
`0x005aa3a0`, `0x005aa598`, `0x005aa998`, `0x005aab88`, `0x005aad30`), and it
writes a fixed block of register-shaped words into the texture object at
`+0x20` through `+0x4c`. Two of those writes are the interesting ones:

```c
uVar4 = (ushort)(param_7 >> 0x10);      // the packed word's high half
uVar3 = (uint)param_7;
...
*(uint *)(param_1 + 0x24) = ... | uVar4 & 0x1f00 | ...;   // format's low 5 bits
*(uint *)(param_1 + 0x30) = uVar3 & 0x1ffff;              // remap, 16 bits + order
```

**`param_7` is one packed `(format << 16) | remap` word**, and its low
seventeen bits - the sixteen-bit remap plus the `order` bit above it - go to
one slot of their own. `& 0x1f00` on the high half is the same low-five-bits
format extraction `oag_texture::gtf::Format::from_byte` does.

**The engine constructs these words in code, it does not only copy them out of
files.** Searching the whole binary for the three values a `.gtf` carries:

| Word | `ori` sites | Where |
| --- | ---: | --- |
| `0xaae4` | 40+ (truncated) | `Hud_LoadDefinition`, `Environment_LoadStageTextures`, and much else |
| `0xa9ff` | 6 | `FUN_00174a30`, `FUN_00175798` (x4), `FUN_005a8df8` |
| `0xa9e4` | 1 | `FUN_0040dc08` |

`FUN_00175798` builds four **single-channel** runtime surfaces per iteration,
each with the packed word `0x0100a9ff` - low half `0xa9ff`, high half `0x0100`,
whose low five bits are `0x01`, `B8`. That is the same pairing the disc's own
9 `B8` files carry, arrived at independently: the engine's own one-channel
surfaces and the disc's one-channel textures use the same remap.

**A fourth word settles the pair order, which the disc's three could not.**
The function compares its argument against two literals, `0x1b00aae4` and
`0x1c0009e4`. Decomposing `0x09e4` under `gtf::Remap`'s reading - controls in
the high byte, `A`, `R`, `G`, `B` from the least significant pair up - gives
alpha forced to one, red read, **green and blue forced to zero**: a
single-channel-in-red texture, which is a thing. Under the opposite pair order
the same word reads as green alone with blue forced on and alpha off, which is
not. The same test applied to `0xa9ff` on a `B8` texture is starker: the
opposite order forces *blue* to one, and blue is the only channel a `B8`
stores, so the reading would discard the texture's entire content.

75 rather than higher because the register *slots* are identified by their
shape and their contents rather than against a decompiled `cellGcmSetTexture` -
`+0x30` taking exactly the remap word is why it reads as
`NV4097_SET_TEXTURE_CONTROL1`, and nothing here proves the command-buffer write
that follows. The remap *packing* itself is the better-supported half; see
[`gtf.md`](../../../formats/gtf.md).
