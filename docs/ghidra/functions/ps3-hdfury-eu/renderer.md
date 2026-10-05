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
dead end below, recorded so it isn't retried the same way.

**2026-09-15: re-run on the post-`lvlx` image, scoped rather than blind, and
still negative - now with the scope wide enough to say why.** Three widths
this time, not just `stw`: `search_instructions(mnemonic="stw",
operand_pattern="0x11c(")` (202 hits), `"stfs"` (32), `"sth"` (3), 237 total
across the whole program, 84 of them on base register `r1`. Only `r1` is
dropped as a certain frame-local hit - it is the architectural stack pointer
on every PowerPC function, no exceptions. **`r31` was not dropped**, despite
this section's own text above reading as if it should be: a spot check of
five `r31`-based hits before trusting the filter (`Hud_LoadDefinition`,
`List_Construct`, `Block_DrawHorizontalEdge`, `SoundSystem_Init`,
`Shadow_ParseStencilVolumeGeometry`) found no `mr r31,r1` frame-chain
idiom anywhere in their prologues; two of the five instead show `or
r31,r3,r3` - `r31` holding the incoming `this` pointer, exactly the kind of
real object register this search is supposed to find, not discard. This
64-bit ABI keeps the frame in `r1` itself (`stdu r1,-N(r1)`) and has no
routine second frame register the way a 32-bit `mr r31,r1` convention would;
treating `r31` as frame-equivalent to `r1` would have silently dropped 53
genuine candidates.

Dropping only `r1` leaves **153 hits in 123 distinct functions**, base
registers `r31`(53) `r29`(25) `r30`(21) `r9`(21) `r28`(9) `r11`(8) `r27`(5)
`r3`(5) `r23`(2) `r22`(2) `r4`(2) - `RenderManager_Construct`/
`_ConstructComplete`'s own two known sentinel writes among them (reproducing
the already-known answer on `r30`, the self-check this method needs before
trusting it on the unknown part). The other 121 functions each write
`+0x11c` on some object register, a different one per function, consistent
with each being a different class's own field at a coincidentally-shared
offset rather than evidence of one shared type; classifying which if any is
`RenderManager`'s own type needs more than a base-register read.

That question is moot for *this* pass's actual purpose, though, which was
narrower: **is the writer among the code the `lvlx` reimport newly decoded?**
It is not. Cross-checking all 123 candidate functions against the set of
functions containing an `lvlx` instruction (293 functions program-wide -
matching the ~294 this binary's reimport is known to have unlocked) finds
**zero overlap** - none of the 123 candidates contain any `lvlx` instruction,
so none of them are code this session's earlier, pre-reimport sweep could
have missed for lack of instructions. The depth-override writer remains
unfound, and - unlike the two audio negatives on this same "stale" list -
this one is not explained by the reimport at all: whatever writes
`instance+0x11c` a real value, if it exists in this binary, was already
fully decoded before 2026-09-15 and simply wasn't found by a `stw`-only,
un-scoped sweep. The `0x00109028` dead end below is unaffected by the same
check for the same reason and is not re-run here.

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
  frame's 118 entries, so the `oag_mesh::mesh::rcs`/`LAYER_DEFAULT`
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
[`oag_fx::psys`](../../../../crates/fx/src/psys.wgsl) currently does one
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

**Closed 2026-09-14**: the route is `ShaderRegistry_RegisterPair` (`0x006774b8`,
`(slot, name, blob)`), called from each subsystem's static initialiser with both
arguments loaded from the TOC, which is why a `bl`-to-constructor scan never saw
it. Reading `r4`/`r5` at every `bl 0x006774b8` pairs the 62 - see
[menu-backdrop.md](menu-backdrop.md#the-shader-pairs-and-the-registration-route-the-census-could-not-see)
for the table covering the RadioHeads and the three `FEBackgroundAnimFury` passes.

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

**Its value, read 2026-09-25: `(0, 1)`, the identity** - `alpha * .y + .x` in
every fogged fragment block that declares it. `Scene_PrepareFrame` publishes
`block + 0x7c50` into slot 76 (`stw r4, 0x998(r11)` at `0x003ab590`), and
`Scene_InitRenderBlock` fills that vec4 from stack words it writes as
`(0.0, 1.0, 0.0, 0.0)` (`lvx` of `r1 - 0x10` at `0x003aa7c0`, `stvx` at
`0x003aa7e0`). `Ship_DrawModels` and `FUN_003eb890` point the entry elsewhere
for craft draws; nothing on the weapon path was found to. Confidence 80. See
[hd-unlit-programs.md](../../../rendering/hd-unlit-programs.md).

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
resort. **[stale 2026-09-15: computed before the lvlx reimport; re-run per toolchain.md#ps3]**

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
what places the knee there. `oag_post::hd_bloom` draws into a float
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

All of the above is implemented verbatim in `oag_post::hd_bloom`;
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

### The emissive glow belongs on the lit path, and the reference band was padding (2026-09-09)

The defect the section below filed and deliberately did not fix, taken to a
measurement. **Two findings, and the second is the bigger one**: the glow's
destination is settled by the disc and is unambiguous, and the reference band
this subsystem has been judged against since 2026-08-20 was **an artefact of
image padding** rather than a reading of the original's frame.

**The band first, because it reframes everything else.**
`data/reference/hd-capture/talons/00.png` is a **1600x1200 canvas holding a
1278x718 frame at +322+24**, and the remaining 52.2 % is black. Clipped-white
share is a *share*, so the padding drags it down by the padding fraction and
nothing else: measured whole the grab reads **3.850 %**, and measured over its
own picture it reads **8.056 %**. The 3.85 % is the number this project
carried as the *lower* bound of its reference band. It is the higher of the
two grabs that band was built from, not the lower, and the 1.8x disagreement
between the pair - the thing that made the band look wide and made our frame
look bright at either end of it - was measurable all along.

Trimmed and re-measured, all seventeen rpcs3 **race** grabs under
`data/reference/hd-capture/` (`scripts/hd-glow-sweep.py reference`):

| grab | canvas | clipped white | mean |
| --- | --- | --- | --- |
| `anulpha/00.png` | 1278x718 | 2.848 % | 0.4686 |
| `anulpha/01.png` | 1278x718 | 7.491 % | 0.6541 |
| `anulpha/02.png` | 1278x718 | 3.710 % | 0.5410 |
| `talons/00.png` | 1600x1200 **padded** | **8.056 %** (3.850 % whole) | 0.6390 |
| `talons/01.png` | 1600x1200 **padded** | **15.649 %** | 0.7215 |
| `talons-fifo/00..02` | 1278x718 | 6.994 / 7.030 / 7.021 % | 0.623 |
| `talons-fifo2/00..02` | 1278x718 | 6.966 / 15.796 / 15.472 % | 0.622-0.735 |
| `talons-fifo3/00..02` | 1278x718 | 6.923 / 15.980 / 14.414 % | 0.623-0.727 |
| `talons-heap/00`, `/01` | 1278x718 | 6.396 / **20.549 %** | 0.622 / 0.798 |
| `talons-rm/00.png` | 1278x718 | 7.895 % | 0.6343 |

**The band is 2.848 % to 20.549 %, median 7.491 %**, and it splits by what the
frame is doing rather than by which grab it is: a **grid** frame (0 km/h) reads
6.40-8.06 % on Talon's Junction, and an **in-motion** frame reads
14.41-20.55 %. `talons-heap/01.png` at 556 km/h is the 20.549 %, and it is the
original itself dissolving the track into white - so a frame of ours near 20 %
at speed is in character, not blown. `talons/00.png` and `/01.png` and the
four under `flare-size/` are the padded ones; every other grab in that tree is
a clean 1278x718.

**`scripts/clipped-white.py`'s own docstring still carries the 3.85/6.99 pair**
as its reference figures. It is not wrong about what those two files measure
whole - it is wrong that the whole file is the frame. Read the table above
instead, or `hd-glow-sweep.py reference`, which trims before it measures.

**The destination, read off the disc.** `uvanim_diffuse_emissive`'s fragment
blocks on Amphiseum, `scripts/ps3-microcode.py fp-file`, block #3 (lit, fogged):

```text
@0x14  TEX H0, f[TC4] unit0            ; albedo
@0x18  TEX H1.xyz, R0.zwzz unit1       ; the glow sample, at (u, (v+a)*b + time)
@0x19  MUL H1.xyz, H1, {const}         [<- 0xe8bcd7f5, the tint]
@0x1c  MAD H6.xyz, H1.wwww, {const}, H6   ; H6 = ambient + ndl * sun
@0x1e  ADD H4.xyz, R2, H6              ; + prelit -> the light sum
@0x22  MUL H4.xyz, H0, H4              ; albedo * light
@0x23  MAD H0.xyz, H0.wwww, H1, H4     ; + diffuse alpha * tinted glow
@0x28  MAD H0.xyz, R2.wwww, H0, R2     ; the fog lerp, after the glow
```

The accumulate at `0x23` reads `H4`, which the light has already multiplied.
Its unlit block #2 (`MUL` at `0x0b`, `MAD` at `0x0f`) and its second lit block
#4 (`MUL` at `0x23`, `MAD` at `0x25`) are the same pair in the same order.
**Three variants, no exceptions, confidence 95** - the ordering is read
straight off the microcode and needs no inference. `rcsmaterial.md`'s
line-1424 excerpt is block #2's and stops at the `MAD`, which is why the
ordering read as ambiguous from that page alone.

So `lit_linear` was the correct destination and `plain`-only was a straight
defect - `mesh.wgsl`'s own comment above the sum already described the
behaviour the code did not implement.

**The reach, and it was total.** The filed figure was "at most 3.60 % of the
frame, the far background and the hull". Re-measured by forcing the layer to
flat red and diffing against a build with it zeroed - three builds, so the
comparison is layer-against-nothing rather than red-against-real:

| circuit | pre-fix (`plain` only) | post-fix (both paths) |
| --- | --- | --- |
| Talon's Junction, grid | **0 px, byte-identical** | 122,038 px (10.386 %); 16,656 (1.417 %) over 8/255 |
| Amphiseum, grid | **1 px** | 118,675 px (10.100 %); 56,855 (4.839 %) over 8/255 |

**The pre-fix layer reached nothing at all**, which does not reproduce the
3.60 % but does reproduce the section below's own "zeroing HD's emissive
`glow` moves the frame 0.000" - two statements that were never consistent with
each other. Post-fix it reaches about a tenth of the frame on both.

**The domain is a choice, and the principle chose rather than the brightness.**
The RSX multiplies raw 8-bit throughout and sRGB-decodes no texture (see
"Per-texture sRGB/`GAMMA` decode: settled negative"), so *neither* domain
reproduces it; what a domain can preserve is the glow's magnitude relative to
the albedo it is added to. The authored path already decodes that albedo, so
the sample is decoded with it and the tint - a shader constant, an authored
magnitude - is not. The stand-in path keeps the undecoded form, its whole
point being that it is gamma. Both were measured, bloom on, `--ticks 0`:

| circuit | baseline | decoded (shipped) | undecoded |
| --- | --- | --- | --- |
| `amphiseum` | 6.515 % | 8.219 % | 8.490 % |
| `talons_junction` | 9.860 % | 9.889 % | 9.944 % |
| `15_anulpha_pass` | 6.776 % | 6.871 % | 6.919 % |
| `10_sebenco_climb` | 10.373 % | 10.438 % | 10.455 % |
| `zone_1` | 18.490 % | 18.047 % | 15.499 % |

The two are 0.02-0.27 points apart on four of the five, so the measurement
does not decide it and was never going to. **This is chosen, not measured**,
on the consistency argument above, and it carries no confidence score.

**The sweep, sixteen circuits, `--ticks 0` at 1440x816**
(`scripts/hd-glow-sweep.py sweep`). The baseline column reproduces the
2026-09-09 sweep exactly where that sweep was recorded - `04_chenghou_project`
1.319 %, `12_sol_2` 18.463 % - which is what says the harness is the same
instrument:

| circuit | baseline, bloom on | fixed, bloom on | baseline, off | fixed, off |
| --- | --- | --- | --- | --- |
| `amphiseum` | 6.515 % | **8.219 %** | 1.631 % | **3.737 %** |
| `modesto_heights` | 4.416 % | 4.416 % | 4.329 % | 4.329 % |
| `talons_junction` | 9.860 % | 9.889 % | 4.507 % | 4.509 % |
| `tech_de_ra` | 5.215 % | 5.215 % | 4.694 % | 4.694 % |
| `zone_1` | 18.490 % | **18.047 %** | 13.138 % | 13.146 % |
| `zone_2` / `zone_3` / `zone_4` | 0.488 / 0.486 / 0.492 % | unchanged | same | same |
| `01_vineta_k` | 1.410 % | 1.455 % | 1.410 % | 1.455 % |
| `02_track` | 2.657 % | 2.657 % | 2.657 % | 2.657 % |
| `03_track` | 10.640 % | 10.643 % | 10.640 % | 10.643 % |
| `04_chenghou_project` | 1.319 % | 1.331 % | 1.319 % | 1.331 % |
| `05_ubermall` | 12.815 % | 12.817 % | 12.815 % | 12.817 % |
| `10_sebenco_climb` | 10.373 % | 10.438 % | 10.373 % | 10.438 % |
| `12_sol_2` | 18.463 % | 18.463 % | 18.463 % | 18.463 % |
| `15_anulpha_pass` | 6.776 % | 6.871 % | 6.776 % | 6.871 % |
| **median** | 6.515 % | **6.871 %** | 4.507 % | 4.509 % |

The median moves 6.515 -> 6.871 % against a reference median of 7.491 %.
**That pair is the weakest comparison on this page and should not be leaned
on**: fourteen of the seventeen reference grabs are Talon's Junction and most
of them are in motion, against sixteen circuits of ours at the grid, so the
two medians are not drawn from the same population. **The matched comparison
is narrower and is the one to cite** - Talon's Junction at the grid, ours
**9.889 %** against that circuit's own grid range of **6.396-8.056 %**. We
were above it before the fix (9.860 %) and are still above it after, by 0.029
points, which is inside the noise of this measurement. So the honest statement
is that the fix **does not move the frame out of anything it was inside**, not
that it closes a gap.

Where the move is real is the circuit the layer is busiest on: Amphiseum's
arena floodlights, dull streaks across the roof before, now read as light
banks, and its `AG-SYS` hoarding lights up.

**Two things in that table are worth their own line.** Zone 1 gets *darker*,
and the table itself says why rather than leaving it asserted: with the bloom
**on** it goes 18.490 -> 18.047 % and with the bloom **off** it goes 13.138 ->
13.146 %, up. The darkening exists only where the chain runs, so it is the
blur ladder feeding the luminance adaptation - a brighter scene buys a lower
exposure and the resolve gives some of it back. **Adding light to this chain
is not monotonic in the output** and a term cannot be judged by its sign.

And **eleven of the sixteen circuits read identically with `[graphics] bloom`
on and off**, which is not this change's doing - it holds on the baseline
build too - and is unexplained. The five that do respond are the four named
environments plus Zone 1. **The obvious hypothesis is already dead**: six of
the eleven *did* move under this change (`01_vineta_k`, `03_track`,
`04_chenghou_project`, `05_ubermall`, `10_sebenco_climb`, `15_anulpha_pass`),
so they reach `lit_linear` and take the authored path and still do not respond
to the switch. Whatever it is, it is not "those circuits never light".

**On circuit, where the signs and tubes are**, `--autopilot --ticks 900`:

| circuit | baseline | fixed | the original, in motion |
| --- | --- | --- | --- |
| `talons_junction` | 15.904 % | **19.519 %** | 14.414-20.549 % |
| `amphiseum` | 4.798 % | 4.881 % | not grabbed |

Talon's Junction at 540 km/h is the largest move measured, +3.6 points, and it
lands inside the original's own in-motion range rather than past it. Read as a
player: the tunnel's ceiling strips and its right-hand wall panels light into
a continuous bar where they were broken and dull, and the yellow hoarding at
top left washes toward white - which is what `talons-heap/01.png` at 556 km/h
does to its own signs too.

**Verdict: shipped.** The disc settles the destination, the layer went from
zero pixels to a tenth of the frame, and the frame moved toward the reference
rather than past it once the reference was measured over its own picture.

**Not measured**: Fury's circuits, any mounted DLC pack, and Amphiseum in
motion against a grab there is none of.

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
`[graphics] bloom` reached only the PSP chain (`oag_post::bloom`) and
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
   with no measurement behind the new behaviour. **Fixed 2026-09-13** - see
   "The matched-camera comparison" below, which supplies that measurement:
   the tap offset now folds into the same `min` `drawn()` uses instead of
   being added after it. **"Inert at native" is not quite exact**: a
   before/after capture at `--render-scale 100` (no DRS, no FSR) differs by
   1,977 of 921,600 pixels, each by at most 4/255 - a hairline border-band
   effect from `uv_max` sitting half a texel inside the buffer's own edge
   even with nothing shrinking the drawn rectangle, not only under DRS/FSR as
   this entry originally read. Every aggregate this page or `hd-glow-sweep.py`
   reports is identical before and after to the printed precision, which is
   the sense in which "inert at native" still holds.
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

### The matched-camera comparison: the frame reads darker than the original, and the post chain is cleared of it (2026-09-13)

Every brightness figure above was **aggregates-at-a-distance**: two close but
not identical framings, compared as whole-image percentages because no exact
camera match existed yet. `docs/reverse-engineering/rpcs3-capture.md`'s
camera pick (same date) removed that constraint - `data/reference/hd-capture/
talons-matched/{00,01,03}.{png,json}` gives three Talon's Junction frames at
the original's own recovered `viewProj`, reproducible with `oag-game
--camera-pose`/`--camera-fov`. `scripts/hd-frame-compare.py` (committed this
session) renders each pose at the project's stated comparison setting
(native size, no MSAA/motion-blur/shadows/reconstruction) and reports
per-region luminance/clipped-white/halo statistics with the HUD and the
reference's own craft masked out of every region (our render draws no craft
at all at these poses - see the script's own doc comment for why: `main.rs`'s
"an RPCS3 capture gives the camera exactly and the craft only as whatever the
frame shows", and our craft never moved off the spawn point these captures
were never aimed at).

**The frame is darker than the original everywhere measured, not brighter -
the opposite of every aggregate-at-a-distance reading above.** Whole-frame
mean luminance (bloom on, HUD and craft excluded):

| pose | ours | reference | delta |
| --- | --- | --- | --- |
| `00` (grid, looking down the tunnel) | 0.470 | 0.599 | -0.129 |
| `01` (529 km/h banked corridor) | 0.346 | 0.496 | -0.150 |
| `03` (tight loop, weapon HUD up) | 0.384 | 0.627 | -0.243 |

Road-surface and distant-geometry regions show the same direction and a
similar magnitude at `00` and `01` (road: -0.06 to -0.15; distant: -0.10 to
-0.26); `03`'s much larger gap is not this finding - its `road surface`
region reads 0.250 against 0.588 because of the already-documented flat-black
`etched_glass_tech` floor panel (`rpcs3-capture.md`, "The pick is fixed",
and `rcsmaterial.md`), which this session's crop comparison corroborates
again rather than newly discovers. The `sky` region (visible only at `00`
and marginally at `03`; `01` is fully enclosed and reads as tunnel ceiling,
not sky, at 0.23-0.27 luma both sides - no evidence of a distinct sky there
either way) is close in mean luma (0.914 vs 0.910 at `00`) but under-clips
relative to the reference (35.4 % vs 55.9 % of pixels >= 250) - a smaller,
separate gap, addressed below.

**Two negative results narrow where the gap can be**, so the next session
does not re-run either. (1) **The exposure stage is already saturated at
`scale = 1.0`** for this scene: hard-coding `fs_encode`'s `scale` to `1.0`
(diagnostic only, not shipped) changes the pose-`00` whole-frame reading by
**0.000** in every region. Talon's Junction's own `.envsettings` (boost 20,
clamp 3, max 4) means `scale` only exceeds 1.0 when the adapted luminance
drops under 0.15, and this scene's own adapted luminance sits well above
that, so the exposure resolve this project implements cannot be the source
of a frame reading uniformly *darker*. (2) **`[graphics] bloom` on vs. off
moves the whole-frame reading by only 0.017-0.05 luma** at pose `00`
(0.461 -> 0.478 with the fix in place) - an order of magnitude short of the
0.13-0.24 gap above, so the additive bloom summand is not the dominant term
either, whichever side of the player switch a render is judged on.

**That points the remaining gap at the lit material path, not this chain.**
`mesh.wgsl`'s `lit_sum = ambient + prelit + vertex_light + sun_diffuse` (see
"The lit track material, read" and "The vertex-light constants are read"
above) already implements every term this page's own microcode reading
named as of 2026-09-06, so this is not a missing summand this page can point
at the way the sun-diffuse and vertex-light removals could in August - it
reads as a magnitude or data-plumbing gap in a term already present (a
per-material `.envsettings` value read short, a texture decoded a shade too
dark, or one of the still-unread terms this page's own "Left unread" line
already names: `shadowMapTex`'s `1 - shadow` factor, `prelitBias`, and the
RGBE `k`/`b` at `c464` for materials other than `track_surface`). Settling
which needs `mesh.wgsl`/`crates/mesh/src/mesh/` and
`crates/rcs/` - `lane-ship-hull`'s files this session, not this one's.
`sky_cube.rs`'s smaller, separate under-clip gap is the same territory
(`crates/mesh/src/mesh/sky_cube.rs`).

**Fixed here**: the `fs_blur` tap-offset defect two sections above, now that
a measurement exists to check it against (see that section's own updated
text for the exact change and its native-resolution footprint).

**On lead (i), whether Talon's Junction shows visible atmospheric fog at
all** (`hds-sky-fog-and-lighting-draw-from-the.md`'s own open question): a
side-by-side crop of pose `00`'s distant cityscape (the region a fog term
would show most clearly) shows the reference more blown toward white but
with the same building silhouettes crisp at the same apparent distance as
ours - no gradient of desaturation or softening with distance in either
image that reads as a distinct haze term, only an overall brightness
difference already accounted for above. **Confidence 55, and deliberately
not higher**: three frames of one circuit, judged by eye against a crop, not
a per-material microcode read. It weakly supports "fog is off, or
imperceptible, in this race type" over the vestigial-slot and
route-mismatch alternatives `hds-sky-fog-and-lighting-draw-from-the.md`
lists, but does not settle it, and says nothing about a circuit that
authors heavier fog than Talon's Junction does.

Reproduce with `scripts/hd-frame-compare.py [--pose 00|01|03] [--bloom
on|off] [--dump-regions]`; region boxes are chosen by eye and shared across
the three poses (documented in the script itself, no confidence score - a
comparison window, not an RE claim).

### The darkness gap does not reduce to an offset, a scale, or a single curve (2026-09-13, `lane-hd-dark-frame`)

Following the section above's own "not this lane's" pointer: the shape is
localised further, two comparison-harness defects are found and fixed, and
the lit-material path itself stays open. All three candidates the previous
session's own "Next Steps" ordered by cost are checked; none is a fix.

**The harness's own per-pixel regression cannot discriminate an offset from a
scale from a curve, and its R^2 says so** (0.01-0.26 across the three poses,
one of them negative-slope) - it needs pixel correspondence at exact
sub-pixel alignment, which a matched *pose* does not give a matched
*rasterisation*. `scripts/hd-frame-compare.py` gained a sixth output,
per-region quantile-quantile: `np.percentile` of each side's own luminance,
independently sorted, so no pixel correspondence is needed at all - a
near-constant `ref - ours` gap across ranks would say an offset, a gap
growing in proportion to the value a scale, a gap concentrated at one end a
curve. **None of those single shapes fits.** At pose `00`, `whole (excl HUD,
craft)`, `ref - ours` goes `+0.071` (q1) `-> +0.134` (q50) `-> +0.229` (q75)
`-> +0.009` (q95) `-> +0.000` (q99): the gap grows through the low-to-upper
range and collapses back to zero at both true black and true white. A single
power law would hold a constant exponent across that whole run; solving
`ours^g = ref` pointwise instead gives `g` from 0.15 to 0.78 depending which
two quantiles are used - not a constant, so **not a clean gamma/tonemap
curve either**, contrary to what the convergence at both ends might suggest
on its own. Pose `01` shows the same qualitative shape. **A single global transfer-
function mismatch does not fit the whole-frame aggregate, confidence 80**
(a direct, rank-matched, per-region measurement on two poses). That is
narrower than "candidate (a) is refuted": a per-material mix of a correct
and an incorrect encode path would produce exactly this kind of non-constant
pointwise exponent too, and 275 of Talon's Junction's 301 drawn materials
take a lit formula `track_surface`'s own microcode reading does not cover -
so this does not clear any individual material family, only the aggregate-
as-one-mechanism reading. Settling a per-material transfer-function defect
needs the per-material probe (`hd_light_probe.rs` run per material, or a
per-material `.envsettings` sweep), not this whole-region aggregate. The q1
row is the residual worth carrying forward too: at pose `00` post-aspect-fix
it is `ours 0.050, ref 0.078`, `+0.028` - a pure power law through the
origin gives zero gap at the floor, and this one does not quite. This does
not retire the ADR-0026 question itself - a scene this mixed-material need
not show one global
curve even if the disc's own encode is one - but it does mean the whole-frame
gap here is not explained by reaching for one.

**Candidate (b), `.envsettings` plumbing, is refuted for the core terms with
an exact match.** Talon's Junction's raw `track.envsettings` (dumped with
`scripts/psarc.py cat`) reads `"Lighting.Constant ambient color"=0.403922
0.392157 0.509804`, `"Lighting.Sun color"=2.000000 1.827451 0.886275`,
`"Lighting.Sun specular scale"=1.500000`, `"Lighting.Prelit ambient colour
scale"=4.000000...`, `"...power"=2.000000...` - and `crates/render/examples/
hd_light_probe.rs` (a pre-existing scratch probe, unmodified) echoes those
same values back verbatim after the full read/parse/upload path. Nothing is
read short, scaled, or byte/float-confused for this circuit's core light
terms.

**Two stale doc comments were found and fixed while checking (b), not the
cause of anything.** `76d0f58e` (2026-08-18) removed `Light::authored`'s
peak-divide and ambient clamp - the magnitude is uploaded as authored now,
same as the direction - but `mesh_render::Light::sun`'s own field doc still
said "with its magnitude divided out: the hue alone", and `mesh.wgsl`'s
header comment still said "The magnitude is not [the disc's]". Both
corrected to match what the code has done since that commit; no behaviour
changed, `cargo build -p oag-game` confirms `sun: colour` and no such divide
exists at either call site.

**A harness confound, found by inspection: poses `01` and `03`'s reference
frames are moving (529 km/h and 431 km/h, their own HUD says) and ours are
posed statically.** `COMPARISON_ARGS` turns `--motion-blur off` for a
like-for-like *setting*, but the reference is a real capture with the
original's own speed streak baked into its pixels by construction, which no
render setting can retroactively remove. At pose `01` this is broad white
streaks across roughly the lower half of frame - visible by eye in
`data/reference/hd-capture/talons-matched/01.png` - which no lighting fix at
any value reproduces. Pose `00` (grid, 74 km/h) is the only one of the three
that isolates the lit-material question from this; `01`/`03` corroborate the
gap's *direction* only. `scripts/hd-frame-compare.py`'s own docstring now
says so.

**A second harness defect, found and fixed: the comparison rendered at the
wrong aspect ratio, and a corner of the engine drops a viewport offset it
should carry.** `render()` wrote only `[graphics] bloom` into its scratch
`settings.toml`, leaving `display.aspect` at the project's own default,
`Aspect::Psp` (30:17) - not `Aspect::Wide` (16:9), which
`oag_display::display::aspect`'s own doc names as "Wipeout HD's own
1920x1080". At `--size 1280x720` (16:9) that mismatch is only 0.7% - "small
enough to look like a rounding error", the enum's own comment says of the
same number - but it is not zero: `oag_display::display::viewport` fits a
1270.6-wide rectangle centred in the 1280-wide canvas and hands the *fitted
size* to `oag_post::hd_bloom::Chain::run` without its *offset*
(`crates/raceplay/src/scene/frame.rs`'s `hd.run(..., (viewport.2 as u32,
viewport.3 as u32), ...)`, dropping `viewport.0`/`.1`). The chain's own final
encode pass then writes `set_viewport(0.0, 0.0, rect.0, rect.1, ...)` -
anchored at the canvas origin - while the scene was actually drawn centred,
so the rightmost ~10 columns of the canvas are never written by that pass at
all and read back as whatever `view` held before it (black). Measured: a
100%-black, full-height, 10-column-wide strip at `x=[1270,1280)` on every one
of the three poses, present with `--bloom` on and off alike, and **absent on
a Pulse race screenshot at the same size** (`pulse-psp-eu.chd`, 7 of 7,200
edge pixels zero - ordinary dark scene content, not a strip), which is what
places it on the `hd_bloom` path specifically rather than the shared capture
plumbing. Pinning `render()`'s settings.toml to `aspect = "wide"` (this
script's own fix, verified: `zero_pct` in `road surface` goes 2.44% -> 0.00%
at pose `00`, reproduced on all three) makes the letterbox offset zero and
the defect inert, matching what a corrected `frame.rs` caller would draw at
this exact canvas size. **Filed, not fixed, at the engine level**: the
general case needs `Chain::run`'s signature to carry an offset alongside
`(width, height)`, plus the same check against `oag_post::bloom`'s
matching call in the same function (same drop-offset shape, not observed to
manifest as a literal-black strip in the one Pulse capture checked, not
independently verified clean either) - `lane-ship-hull`'s files
(`crates/post/src/`, `crates/raceplay/src/scene/frame.rs`), not a
comparison script's.

**Net effect on the darkness reading: negligible, as the strip's size
predicts.** Re-running with the aspect fix moves pose `00`'s whole-frame
mean luma 0.470 -> 0.474, pose `01` 0.346 -> 0.351, pose `03` 0.395 -> 0.394
- each under 0.005 against gaps of 0.13-0.24, consistent with a ~0.8%-of-width
edge strip rather than a frame-wide term. The two fixes land because they are
correct and because the harness should not carry a confound irrespective of
size, not because either explains the gap.

**Still open, and this session's own candidate (c) and (d) are not newly
checked here**: the texture-decode brightness question (`oag-texture`'s
`.gtf` decoder itself carries no gamma/sRGB logic of its own -
`rg`-confirmed absence in `crates/texture/src`- so a decode-side darkening
would have to be a bit-level DXT/BC defect, and `gtf_ground_truth.rs` passing
in this session's gate run is the only evidence checked, not a pixel
comparison against a reference decoder); and any missing term beyond what
`renderer.md`'s own "Left unread" line already names
(`shadowMapTex`, `prelitBias`, the RGBE `k`/`b` for materials other than
`track_surface`). Reproduce the quantile table with
`scripts/hd-frame-compare.py --pose 00` (or `01`); the zero-pixel-share
column is `zero_pct` in the per-region table above it.

### The letterbox strip is fixed at the engine level, and a per-material probe of the darkness gap is inconclusive (2026-09-13, `lane-hd-material-curve`)

Picking up the previous section's two "filed, not fixed" items.

**The letterbox-offset drop is fixed, generally, at both call sites.**
`oag_post::hd_bloom::Chain::run` and `oag_post::bloom::Bloom::render`
now take the scene's own drawn `origin: (f32, f32)` alongside its size, and
each chain's own final write-back pass (`hd_bloom`'s "hd encode", `bloom`'s
"composite" - the only stage in either chain that writes into the caller's
shared canvas rather than a dedicated scratch texture) anchors its
`set_viewport` there instead of at `(0.0, 0.0)`. Verified directly rather
than only against `hd-frame-compare.py`'s own aspect-pinned surface (which
cannot see this by construction): rendering pose `00` at `--size 1280x720`
with the project's own default (non-wide) aspect, before this fix the
right-edge black column ran 10 px wide (`x=[1270,1280)`, matching the
previous section's measurement) against a ~4 px left margin; after, both
margins are a symmetric ~5 px, matching the expected
`(1280 - 1270.6) / 2` letterbox math. Pinned with a GPU unit test
(`post::hd_bloom::tests::the_encode_pass_honours_the_scenes_own_offset`)
that draws a uniform scene into an offset sub-rectangle and asserts the
resolved output lands there, confirmed to fail against the pre-fix code.
**Confirms the previous section's own "negligible" reading**: `hd-frame-compare.py`
pins `aspect = "wide"` specifically so this offset is already zero on its
own comparison surface, so no pose-00 tone number in this file moves.

**The per-material probe (`scripts/hd-material-probe.py`,
`crates/render/examples/hd_material_probe_dump.rs`) ran, and it does not
locate a single fixable term.** Method: `OAG_TINT_MATERIALS=1
OAG_OPAQUE_ONLY=1` (`crates/mesh/src/mesh/rcs/isolate.rs`) renders pose
`00` with every opaque material replaced by a flat, unlit colour keyed to
its slot ordinal, segmenting the frame by material without touching the lit
render at all; `hd_material_probe_dump` is the join key from slot to
material path and to the role bits `mesh::rcs::skin::roles` already resolves
(lightmap-lit, ambient-fed, sun-fed, second-texture glow).

**First finding, and the reason the straightforward version of this probe
does not work: fog reaches the tint diagnostic.** `mesh.wgsl`'s `fs_main`
calls `fogged()` unconditionally, on every path including the unlit
stand-in `lit == 0.0` takes under `OAG_TINT_MATERIALS` - confirmed directly:
a material's flat tint colour drifts smoothly toward a single frame-wide
colour as its on-screen distance from the camera grows, visible by eye in a
vertical scan of the tint render, and that limit colour matches the
circuit's own `Fog.Fog Color` (`0.263, 0.216, 0.380` linear, read off
`talons_junction/track.envsettings` the same way `envsettings_fog` does) to
within a handful of 8-bit units once gamma-encoded. A plain exact-match (or
small-tolerance-to-nearest-colour) classifier therefore only ever attributed
the closest ~30-44% of the frame to a known material - not a broken
instrument, a correct reading of an assumption (`isolate::tint`'s "matched
exactly" doc comment) that was never checked against a circuit that fogs.
**Not fixed at the shader level on purpose**: `fogged()` is called the same
way for every `lit` value already, so gating it off under
`OAG_TINT_MATERIALS` would need a signal `fs_main` does not otherwise carry,
and reaching for `in.lit == 0.0` instead would change production behaviour
for every genuinely prelit chunk - a debug-only need is not a reason to move
a bit real rendering reads. `classify_pixels` in the script instead matches a
pixel against the **segment** from its candidate slot's own tint colour to
the fog colour (both gamma-encoded, an exposure scalar `k = 1` assumed and
calibrated once by eye against an unclipped, non-zero, near-camera channel -
chosen, not measured, no confidence score), which recovers 86-89% coverage
at a 10-12 unit perpendicular-distance tolerance without growing the number
of distinct classified slots past what a 6-unit tolerance already found -
read as the ceiling being fog-dominated far geometry a straight-line
approximation of a curved true path cannot arbitrate, not classification
degrading.

**Second finding: every measured per-material delta is `ref - ours`
positive (+0.05 to +0.23 mean luma, 14 slots meeting a 500-pixel floor at
pose `00`), and it does not cluster tightly by the role bits available.**
Two materials sharing identical `lightmap|no_ambient` role bits span 0.05 to
0.23 - a 4x range within one bucket - which argues against "the whole
lightmap-lit family is off by one shared term" and toward something
per-material or per-texture. The per-slot affine/power-law fits mostly carry
weak R^2 (0.001-0.73), consistent with this file's own reading that
per-pixel correspondence at a matched *pose* is not a matched
*rasterisation* - the per-material **mean** is the trustworthy statistic
here, not the per-material curve.

**Third finding, and the one this session cannot resolve: the instrument
separates `track_surface` from the rest, but in the wrong direction.**
`track_surface`'s own microcode (block #8/#9, no `N.L` or sun term) is the
shader's documented exception to the blanket `sun_diffuse` this renderer
applies to every material alike (previous section and `mesh.wgsl`'s own
comment on `sun_diffuse`) - so `track_surface` should read *relatively*
brighter here than the general population, artificially boosted by sun it
should not have. It reads the other way: `track_surface`'s two classified
slots average `+0.21` against `+0.12` for the other twelve, at every
tolerance from 6 to 12 (stable once its own pixel count stops growing,
so not a coverage artifact). The probe is sensitive enough to separate the
two populations - which is what the check was for - but the separation
contradicts the single-bug reading rather than confirming it: whatever
makes `track_surface` measurably *darker* than the rest is not explained by
the extra sun it incorrectly receives, and this session does not have a
second, disc-sourced candidate for it. **No fix is applied**: per
`CLAUDE.md`, a fix has to be a number sourced from the disc or the
executable, and this probe's own result argues against the one candidate
term (the blanket sun addition) it set out to check, without producing
another one to check next. `docs/rendering/hd-ship-materials.md` carries the
per-material reading this project has of Wipeout HD so far; the open thread
itself is tracked separately, per this project's own work-in-flight
convention (see `CLAUDE.md`'s documentation tree section). The per-slot
table is `/tmp/oag-drive/material-probe-final.tsv` (not committed - a
scratch artifact of one run, reproducible with
`scripts/hd-material-probe.py`).

Confidence 70 on "fog reaches the tint diagnostic and the segment model
recovers most of the frame" - direct measurement of the render, the
envsettings value and the coverage sweep, all reproducible, but the `k = 1`
exposure assumption is calibrated by eye on one pixel rather than solved.
No confidence score on the darkness-gap findings themselves: a probe that
contradicts its own working hypothesis is evidence the hypothesis needs
revision, not a claim about what the revision is.

### The resolve's Fury circuits carry the front end's own Tone triple, and the plain variant is confirmed live (2026-09-13, `lane-hd-resolve-fill`)

The two open threads this section closes: **which resolve program runs in a
race** ("The correction variant's three extra colour parameters are photo
mode" above answered this statically at confidence 85; this is the live
corroboration), and **what feeds `scale` on a circuit whose own
`.envsettings` does not author the whole `Tone` family** - unread until now.

**Live, once: the correction switch reads 0 during an ordinary race.**
Racebox into Sol 2 (`--team feisar_c1 --hull-variant concept1`, the same route
`rpcs3-capture.md`'s matched-camera pairs use), paused mid-race,
`g_RenderGlobals+0x3e0` (`0x00c81e60` at this boot - `g_RenderGlobals` itself
sits at a fixed address, no pointer indirection, so the runtime address is
the static one) reads `0x00`. `003e37a0 beq cr7,r0==0 -> plain resolve` -
this is the exact byte the executable branches on, so a live zero here *is*
"the plain variant is bound", not evidence toward it. Confidence 90: one
live read, but it is a read of the discriminator itself, not a proxy for it,
and it agrees with the static chain (setter -> trampoline -> `PhotoMode_Update`,
no other caller) already at 85.

**Live, same boot: Sol 2's own `Tone` triple reads `(20.0, 3.0, 4.0)` -
identical to Talon's Junction's authored values - despite Sol 2's own
`.envsettings` never authoring two of the three keys.** The settings
singleton's storage pointer (`0x008b6fb4`) plus `+0x540` (12 bytes, the
`Tone adaption boost`/`Tone darkening clamp`/`Tone maximum brightness` triple
this page already located) reads `41a00000 40400000 40800000` big-endian -
`20.0, 3.0, 4.0` - read live during the race, not at menu. Cross-checked
against the disc: `data/environments/12_sol_2/track.envsettings` authors
`"HDR and Bloom.Tone adaption boost"=20.000000` and **nothing else in the
`Tone` family** - no `Tone darkening clamp`, no `Tone maximum brightness` -
confirmed by a full dump of the file, not a grep miss.

**A disc-wide survey explains where the other two numbers come from.**
Every `.envsettings` on the image, by `Tone darkening clamp`/`Tone maximum
brightness` presence (`scripts/psarc.py cat`, all thirteen circuits that
author a `track.envsettings` at all, plus the two front-end files):

| File | `Tone adaption boost` | `Tone darkening clamp` | `Tone maximum brightness` |
| --- | --- | --- | --- |
| `amphiseum`, `modesto_heights`, `talons_junction`, `tech_de_ra`, `zone_1` (DATA00, base HD) | 20 | 3 | 4 |
| `01_vineta_k`, `02_track`, `03_track`, `04_chenghou_project`, `05_ubermall`, `10_sebenco_climb`, `12_sol_2`, `15_anulpha_pass` (DATA02, Fury/DLC) | 20 | (absent) | (absent) |
| `zone_2`, `zone_3`, `zone_4` | no `track.envsettings` file at all | - | - |
| `fe.track.envsettings` (DATA00) | 20 | 3 | 4 |
| `fe.fury.track.envsettings` (DATA00) | 20 | 3 | 4 |
| `fe.track.envsettings` (DATA02) | 20 | (absent) | (absent) |

Every base-HD circuit and both front-end files agree on the same three
numbers; every Fury/DLC circuit's own file omits exactly the same two keys,
and DATA02's own front-end file omits them too. That is the shape of a
**persistent, cumulative settings store**, not sixteen independent
authorings: this page's own earlier reading of the registrar
("lazily-constructed... first built... [`FUN_003a9520`] returning the same
singleton", confidence 82) already says the storage is a single object built
once, not reconstructed per circuit. What was not chased until now is
*what a circuit's own file does to keys it does not mention* - and the live
read above answers that directly: **a key an `.envsettings` file does not
declare is left at whatever the store already held**, which for the whole
Fury/DLC set is the front end's own `20/3/4`, carried forward rather than
reset. Confidence 85 on the mechanism (persistence, confidence 82, plus one
live read that lands exactly on the front end's own published constant
rather than zero or noise); the write site itself - the text-line parser
that walks a `.envsettings` file and calls into the registrar per key found -
is not located this session, so "which function does the carrying" stays
open.

**Consequence for this project, found by reading `crates/raceplay/src/load/environment.rs`
rather than the disc (not a renderer change, so read-only this pass -
`crates/` is another lane's tonight):** `envsettings_bloom` requires all ten
`HDR and Bloom` keys present in the **circuit's own** file, and returns
`None` - "no complete HDR and Bloom block; the race draws without the read
bloom chain" - the moment one is missing. Confirmed by running the built
game: `RUST_LOG=info oag-game --race --track
'Data\Environments\12_Sol_2\track.vex' ...` logs exactly that line for Sol 2,
and the parallel run on Talon's Junction and Amphiseum logs the full
`exposure 4 - min(adapted x20, 3)` line instead. `scene.rs`'s own
`hd_bloom.map(...).transpose()` then never constructs
`oag_post::hd_bloom::Chain` at all for Sol 2 - not a bloom that runs
unbloomed, but **no gate, no blur, no exposure resolve and no HDR encode
pass of any kind**; the frame renders straight into the caller's own
(non-linear) target. **This is the whole `hds-frame-was-too-bright-and-too-bloomy.md`
thread's own "why do eleven of sixteen circuits read identically with
`[graphics] bloom` on and off" answered mechanically rather than by a
lit-material search**: the eleven are exactly `zone_2`/`zone_3`/`zone_4` (no
`.envsettings` at all) plus the eight Fury/DLC circuits in the table above
(missing two of the three `Tone` keys) - the five that *do* respond are
exactly the five whose own file is complete. The switch has nothing to
toggle on those eleven because the chain that would read it was never built.

**The offline check (goal 3, `scripts/hd-resolve-probe.py`, new this
session): applying the read exposure `scale` alone is a clean negative on
all three matched-camera circuits, not only Talon's Junction.** The script
takes an already-rendered `ours` frame from a `hd-frame-compare.py
--pair-dir` run, decodes an approximate linear value (gamma 2.2 - **chosen,
not measured, no confidence score**), multiplies by `scale = max - min(boost
* adapted, clamp)` using the read circuit triple, re-encodes, and reports the
region stats before/after against the reference. `adapted` is proxied as the
frame's own mean linear luma over the non-HUD/non-craft region - also chosen,
a single-frame stand-in for the real multi-frame adaptation lerp:

| Circuit, pose 00 | proxy `adapted` | `scale` | whole-frame mean luma, before -> after | reference |
| --- | --- | --- | --- | --- |
| Talon's Junction | 0.265 | 1.000 (floor) | 0.474 -> 0.473 | 0.599 |
| Amphiseum | 0.274 | 1.000 (floor) | 0.476 -> 0.475 | 0.229 |
| Sol 2 | 0.307 | 1.000 (floor) | 0.475 -> 0.474 | 0.627 |

`scale` saturates at exactly `max - clamp = 1.0` on all three - the proxy
`adapted` never drops under the `clamp / boost = 0.15` threshold that would
push it above 1 - so applying it moves every region by under 0.002 luma in
every case. **This is a clean extension of the existing Talon's-Junction-only
finding ("The exposure stage is already saturated at `scale = 1.0`") to all
three matched circuits, including Sol 2 where the whole chain is currently
skipped**: even if the missing chain were wired up exactly as read, the
multiplicative exposure term by itself would not explain Sol 2's own
darkness/under-clipping gap either, going by this proxy.

**What is not settled, and is squarely the next lane's**: whether the
missing *additive* bloom summand - never computed at all on Sol 2, since the
whole chain is absent, not merely unscaled - would close more of the gap.
Sol 2 authors `Bloom from frame contribution`=1.0 against Talon's Junction's
0.03 (33x), but also `Bloom adaption boost`=15 against Talon's 5; by this
page's own already-implemented gate formula (`contribution.y = (1 -
min(adapted * boost * 0.25, 1)) * authored`), at `adapted ~ 0.3` that gate is
`(1 - min(0.3*15*0.25, 1)) * 1.0 = (1 - 1) * 1.0 = 0` - the higher adaption
boost may gate the stronger authored contribution back toward zero on a
scene this bright. **This is a hypothesis read off the existing microcode
reading, not a measurement** - it needs the real chain constructed and run,
which is `crates/render`/`crates/game` work this lane does not touch.

**The exact wiring for the next lane**, so it is a mechanical change: seed
`environment::staging`'s call into `envsettings_bloom` (and, for consistency,
`envsettings_fog`/`envsettings_light`, which read the same file and may have
the same gap unread) from the title's own front-end `.envsettings`
(`fe.track.envsettings` for an HD circuit, `fe.fury.track.envsettings` or
DATA02's `fe.track.envsettings` for a Fury/DLC one - which of the two Fury
files the real engine actually loads first is not settled this session, and
does not matter for the Tone triple since both carry `20/3/4`) for every
`HDR and Bloom` key, then let the circuit's own file override whichever keys
it declares on top - mirroring the persistence measured live above, rather
than requiring the circuit file alone to be complete. That turns Sol 2 (and
the other seven Fury/DLC circuits) from "no chain at all" into "chain with
the carried Tone triple and the circuit's own bloom-gate/blur authoring",
which is what the disc itself runs.

Confidence summary for this section: 90 on the live correction-switch read,
85 on the live Tone-triple read and the persistence mechanism it implies, 82
(unchanged, cited) on the registrar being a single persistent object, and no
confidence score on the offline scale-check's chosen gamma/adapted proxy or
on the bloom-gate hypothesis above - both are experiments on read numbers,
not further reads themselves.

### The carry is wired: 16 of 16 environments build the bloom chain, and Sol 2's own gap narrows by 10-25% on the regions it reaches (2026-09-13, `lane-envsettings-carry`)

The "exact wiring" paragraph above, done.
`crates/raceplay/src/load/environment.rs::staged_envsettings` reads
`/data/fe/fe.track.envsettings` as a base layer and lays the circuit's own
file over it, keeping only the keys the circuit's own file declares as an
override - the registrar's own persistence, read live above, applied rather
than re-derived. `envsettings_bloom` reads through it;
`hd_envsettings_carry_ground_truth.rs` measures every environment on the
disc: **5 of 16 built the chain before this change, 16 of 16 after** (the
five were exactly the base-HD circuits whose own file was already complete -
`amphiseum`, `modesto_heights`, `talons_junction`, `tech_de_ra`, `zone_1`).

**One correction to this page's own prior note.** The "exact wiring"
paragraph left open which of `fe.fury.track.envsettings` or DATA02's own
`fe.track.envsettings` the real boot loads for a Fury/DLC race, saying "both
carry `20/3/4`" - that is only true of `fe.fury.track.envsettings`.
Re-checked directly against the disc this session: DATA02's own
`fe.track.envsettings` is **itself** missing `Tone darkening clamp`/`Tone
maximum brightness`, the identical two keys every Fury/DLC circuit's own file
omits. So a Fury/DLC race's own front-end file cannot be the sole seed and
still reach `(20, 3, 4)` - the value it does run at (per the live read above)
has to come from something loaded before it, which is consistent with the
mechanism (persistence across more than one load) but not with the "either
file, doesn't matter" reading. `fe.fury.track.envsettings` (DATA00) is
complete and its `HDR and Bloom` block is byte-identical to
`fe.track.envsettings` (DATA00, all ten keys checked) - so this project seeds
from the latter uniformly rather than resolving the real per-title boot
order, which stays open. Chosen, not measured; no confidence score.

**`envsettings_fog` and `envsettings_light` were checked for the same
partial-key shape and do not have it.** Every one of the 13 circuit files
that ships a `.envsettings` at all authors a complete `Fog`/`Lighting` block
on its own - the gap `envsettings_bloom` had does not exist for either
reader. The only environments where the carry would do anything for them are
`zone_2`/`zone_3`/`zone_4`, which ship no file at all (a different case:
nothing to override, not a partial override) - left as the pre-existing
silent stand-in light/unfogged draw rather than wired to the front end's,
because nothing this session read or rendered says what a Zone race's light
rig actually is; `fe.track.envsettings`'s own rig reads as a menu backdrop
(cyan `Sun color` of `0.09/0.84/0.97`, ambient reaching `3.0`), not
necessarily what a Zone circuit runs with.

**Measured against the matched-camera pairs (`scripts/hd-frame-compare.py
--pair-dir <pair> 00`), before/after this change, pose `00` only** (the pose
`hds-frame-was-too-bright-and-too-bloomy.md` already restricts analysis to,
for the motion-streak reasons that page gives):

| Circuit | region | before clip% | after clip% | reference clip% | before luma | after luma | reference luma |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Sol 2 (Fury/DLC, was inert) | whole (excl HUD, craft) | 15.08 | 16.51 | 27.30 | 0.475 | 0.494 | 0.627 |
| Sol 2 | sky | 0.00 | 1.48 | 15.89 | 0.471 | 0.505 | 0.861 |
| Sol 2 | road surface | 18.48 | 18.52 | 38.38 | 0.585 | 0.586 | 0.722 |
| Sol 2 | distant geometry | 3.28 | 5.94 | 14.43 | 0.367 | 0.411 | 0.539 |
| Amphiseum (base HD, unaffected) | whole (excl HUD, craft) | 9.19 | 9.19 | 0.26 | 0.414 | 0.414 | 0.229 |
| Talon's Junction (base HD, unaffected) | whole (excl HUD, craft) | 4.60 | 4.60 | 6.86 | 0.474 | 0.474 | 0.599 |

Sol 2's own regions **narrow toward the reference, on every region but one**:
roughly 13% of the whole-frame clip-share gap closes, 9% of the sky's, 24% of
distant geometry's, and road surface is flat (0.04 points) - a small,
one-directional move, not a fix. Amphiseum and Talon's Junction are **byte-
identical** before and after, to the fourth decimal place of every statistic
`hd-frame-compare.py` reports - confirming the carry does not touch a circuit
whose own file was already complete, and (for Amphiseum specifically)
correcting this page's earlier assumption that it was one of the eleven: its
own file is in the base-HD row of the survey table above and it already
built the chain before this session.

This corroborates, rather than merely repeats, the gate-cancellation
hypothesis the "exact wiring" paragraph raised without a built chain to test
it against: Sol 2 authors a 33x stronger `Bloom from frame contribution`
than Talon's Junction but also a 3x higher `Bloom adaption boost`, and the
already-implemented gate (`contribution.y = (1 - min(adapted * boost * 0.25,
1)) * authored`) folds most of that back toward zero at Sol 2's own
brightness - which is consistent with a chain that is now running and
producing a small, real, mostly-additive move rather than the large one its
raw `frame contribution` alone would predict. Not re-derived as a number this
session (would need the chain's own internal `adapted` state, not the
region-level proxy `hd-resolve-probe.py` used); the render itself is the
corroboration.

**The 55-79%/0-8% clipped-white figures named when this lane was scoped do
not appear in this measurement.** `scripts/hd-frame-compare.py`'s clip% is
masked (HUD, sky, road, distant-geometry regions separately, non-HUD/craft
for "whole") and taken over pose 00's matched-camera frame; the nearest value
in this table is Talon's Junction's own *sky* region at 55.90% (unaffected by
this change either way). `scripts/clipped-white.py` measures the same
`R,G,B >= 250` threshold but over an unmasked, undifferentiated canvas -
`hds-frame-was-too-bright-and-too-bloomy.md`'s own dated entry already
records that its docstring's published numbers include a padded-canvas
reading rather than a frame - so the two scripts are not comparable
one-for-one. Whichever figure motivated scoping this lane, it is not this
table's metric; reported as a discrepancy rather than silently reconciled.

**Player-eye read, both pose-00 renders against the original (`/tmp/oag-drive/images/`,
this session, not committed - `data/reference/` is not this lane's to
leave modified):** Sol 2's after-render shows a faint, real soft glow on the
brightest track-surface highlights and the distant white structure past the
starting gate that the before-render does not, visible on close inspection
but not at a glance - it reads as "the chain is now doing something" rather
than "the chain is doing what the original does": the original frame is
dramatically brighter and more blown-out overall (sky is a near-white glow
around visible cloud shapes, the whole track surface is a stop or more
brighter), which the numbers above already say this change does not close.
No overshoot in either direction - the after-render is not brighter than a
sane middle ground, just still short of the original's. Amphiseum's
after-render is pixel-identical to its before, and on its own account (not
this lane's business to fix) shows two pre-existing, separate gaps: the
grid-start trackside wall panels draw flat black where the original shows
them brightly lit cyan/white (already logged in
`hds-frame-was-too-bright-and-too-bloomy.md`'s "New 2026-09-13" bullet), and
the arena's own upper structure reads brighter/more blown in this project's
render than the original's more contained cyan glow - the opposite-sign gap
that page's quantile table already found.

Confidence summary: 90 (unchanged) on the disc-wide `HDR and Bloom` key
survey the carry is built on; the `fe.fury.track.envsettings` vs DATA02's own
`fe.track.envsettings` choice above is explicitly unscored (chosen, not
measured); the fog/light no-op finding is a direct disc read (confidence 90,
same basis as the original 39-key schema survey); the frame deltas in the
table are direct tool output, not an inference, so they carry no separate
confidence score of their own - what has one is the gate-cancellation
reading of *why* the move is small, which is unchanged at "hypothesis, not
measurement" from the "exact wiring" paragraph above.

### The Amphiseum sign flip is re-derived at a clean pose - the luma reading turns out to be inside the known bloom sensitivity, but a large hue gap survives with bloom on or off (2026-09-17, `lane-hd-track-lighting`)

**The measurement this thread's own "Open" list flagged as unusable is
redone from scratch, and its conclusion survives.** The prior sign-flip
finding (2026-09-13, `lane-hd-captures`) rested on `data/reference/hd-capture/
amphiseum-matched/`, which is gone from disk - confirmed directly, `ls -la`
shows no `amphiseum-matched`, `talons-matched` or `talons-ships` anywhere
under `data/reference/hd-capture/` - and the thread's own words say the
Amphiseum half of it was confounded anyway: captured already at 406-429
km/h with no isolated grid pose, through `hd-frame-compare.py`'s Talon's-
Junction-shaped region boxes, on a `sky` box that samples an indoor ceiling.
None of that evidence is trusted here; every number below is from a fresh
capture, this session, with the track field checked, not assumed.

**Bug found first, and it explains why no lane has produced a working
capture since 2026-09-15.** `scripts/rpcs3-drive.py`'s `cmd_capture` (and
every other subcommand) reads `with session(args) as session:` - binding
the `with` target to the same name as the function being called shadows it
inside that statement's own scope, so `session(args)` resolves to the
not-yet-assigned local and every subcommand raised `UnboundLocalError`
before RPCS3 ever launched. Introduced in `0a181a67` (2026-09-15, the muted-
config-copy commit), reproduced in two lines with no subprocess needed:

```python
def session(args):
    ...
def cmd_capture(args):
    with session(args) as session:   # UnboundLocalError: session
        ...
```

Fixed by renaming the module-level wrapper to `open_session`, leaving every
`as session:` binding alone - `scripts/rpcs3-drive.py`, this session.

**Reaching Amphiseum by `--nav` drops a `right` press about half the time,
not consistently.** `Track Creation=right,right,...` (9 presses, the count
`rpcs3-capture.md`'s own carousel table gives for The Amphiseum) landed on
Talon's Junction (8) once and, with a 10th press added to compensate, on
Modesto Heights (10) - i.e. the drop count is 0 or 1 per boot, not fixed,
consistent with `navigate()`'s own doc comment ("the d-pad produces no
`TTY.log` line ... this settles for a fixed pause per press rather than
pretending to observe one"). Verifying the landed circuit against the
pair's own `track` field, per `rpcs3-capture.md`'s existing warning, caught
both wrong landings before any number was computed from them; a third boot
at 9 presses landed correctly. Not fixed here - a real reliability gap in
`navigate()`, left as `capture`'s own honest refusal-by-verification rather
than patched on a two-sample guess.

**What makes Amphiseum's opening seconds already 400+ km/h, answered.**
`cmd_capture`'s shot loop holds `cross` for `--interval` seconds *before
every shot including the first*, following `--load` seconds of free run
after `InGame` - so shot 00 is never actually a t=0 pose, only however far
`--load` + one `--interval` happens to land past whatever gate is holding
the craft back. At `--load 20 --interval 2`, Amphiseum's craft sits
completely still (identical eye position, six decimal places, across shots
00-05) until `GO` appears on screen around shot 06 (t~32s post-`InGame`,
22 km/h) and then accelerates hard: 22 -> 162 -> 317 km/h in the next two
2-second intervals (shots 07, 08). The prior session's default `--load 50
--interval 6` capture landed shot 00 well past that liftoff point with a
full 6 seconds of held full thrust already spent, which is sufficient for
this game's grid-launch acceleration curve to reach 400+ km/h - not a
downhill start or a shorter countdown, both guessed and both wrong. Talon's
Junction's own countdown is *shorter* relative to the same `--load`: its
default-walk capture (this session, same `--load 20 --interval 2`) is
already fully still at shot 00 (0 km/h, timer `0.00.0`) and stays that way,
i.e. its countdown had not yet ended either, just further from `GO` than
Amphiseum's happened to be read at the old default's timing - the two
circuits were never on the same clock, the capture's own fixed `--load` just
made it look that way.

**The luma reading reproduces at a true 0 km/h grid pose on both circuits,
same boot protocol, same fixed capture script, `--dump-regions` checked
against each circuit's own composition - but the Amphiseum half of it
turns out not to be safe to call a sign flip.** `data/reference/hd-capture/
amphiseum-grid/00.{png,json}` and `talons-grid-recheck/00.{png,json}`
(gitignored, this session's own captures, `--load 20 --interval 2 --team
feisar_c1 --hull-variant concept1`), both HUD-confirmed `0 KM/H`, timer
`0.00.0`, camera picked cleanly (`unit_error` 5.2e-08 and lower, both
registers). `scripts/hd-frame-compare.py --pose 00`, whole-frame excluding
HUD and craft (the region that needs no per-circuit box shape - see below),
`--bloom on` (the script's own default, and the faithful setting since the
original's chain is unswitchable) against `--bloom off` (isolates the
chain's own contribution):

| Circuit | bloom | ours luma | ref luma | ref - ours | ours hue | ref hue | hue gap |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Talon's Junction | on | 0.476 | 0.597 | **+0.121 (ours darker)** | 193 deg | 184 deg | 9 deg |
| Talon's Junction | off | 0.453 | 0.597 | **+0.144 (ours darker)** | 192 deg | 179 deg | 13 deg |
| Amphiseum | on | 0.492 | 0.447 | **-0.045 (ours brighter)** | 256 deg | 102 deg | **154 deg** |
| Amphiseum | off | 0.439 | 0.447 | **+0.008 (essentially matched)** | 257 deg | 102 deg | **155 deg** |

**Talon's Junction's gap is robust to the bloom setting and stays clearly
signed either way** (0.121 on, 0.144 off - bloom's own contribution moves
it by 0.023, inside the 0.02-0.05 range this thread's table already
established for this circuit and Sol 2). Hue stays closely matched in both
settings (9-13 degrees, well inside compression noise) - consistent with
the existing "not a hue/encoding issue on this circuit" reading.
**Amphiseum's directional luma reading is not robust**: it is +0.045
(brighter) with bloom on and +0.008 (essentially zero, if anything very
slightly darker) with bloom off - a swing of 0.053 from bloom's own
contribution alone, comparable in size to the entire on-bloom reading.
Amphiseum is one of only five circuits (`amphiseum`, `modesto_heights`,
`talons_junction`, `tech_de_ra`, `zone_1`) whose `.envsettings` builds the
bloom chain at all (renderer.md's own "eleven of sixteen" finding above),
so this circuit is exactly where a bloom-chain overshoot would show up as
a brightness gap and nowhere else the established darker-frame gap is
measured. **The prior "Amphiseum measures the opposite sign" claim
(2026-09-13, `lane-hd-captures`) is not falsified by this - the bloom-on
number is what it measured too - but it is now known to rest on a
measurement inside the chain's own known sensitivity, not outside it the
way Talon's Junction's 0.121-0.144 sits.** Whether the bloom chain itself
is over-contributing on this circuit specifically, rather than a
per-material lighting gap, is now the more likely reading and is not
distinguished from the per-material hypothesis by anything measured this
session.

**The hue gap is a different story: it is essentially unchanged by the
bloom setting (154 degrees on, 155 degrees off), so it is not downstream
of bloom's own brightness contribution** - if it were, turning bloom off
(which visibly reduces clipping: ours 6.7% -> 2.9%, reference steady at
2.8-3.0%, nearly matched with bloom off) would be expected to move the
hue gap toward zero the way it moved the luma gap, and it does not, moving
1 degree. **`coloured_pct` (the share of each region's pixels above the
0.08 saturation floor the circular mean is weighted by) is high on both
sides, 75-92%** across every row measured - not a small, craft-livery-
dominated minority, which was the other confound worth checking before
trusting a whole-frame hue number (the reference frame shows opponent
craft this project's render does not, per `render()`'s own documented
"our own craft is wherever the race's normal spawn puts it" asymmetry,
and those liveries are among the most saturated pixels in frame - but
they are evidently not driving this number, since it survives a setting
change that halves the clipping share without moving). **154-155 degrees
is most of the way to the opposite side of the colour wheel** - the
reference's dome interior reads warm gold/tan, this project's render reads
cool blue-violet in the same region, confirmed by eye on both renders, not
only in the aggregate. A second Amphiseum pose (shot 06, 22 km/h, `GO`
banner on screen, camera barely moved from shot 00, bloom on) reproduces
both numbers closely (luma gap -0.053, hue gap 170 deg).

**So: the luma sign flip is not a safe claim on its own - it is a small
reading inside a known confound, on the one circuit where that confound
applies - but the hue gap survives the same confound check and is the
sturdier finding this session actually delivers.** Talon's Junction's
gap reads as brightness-only (hue intact, magnitude robust to bloom); a
real colour-temperature shift on Amphiseum, independent of bloom, is new
evidence the two circuits carry different defects rather than one gap
with a per-circuit sign - but that conclusion now rests on the hue
measurement, not the luma one. Whatever Amphiseum's own cause is, matching
Talon's Junction's fix would not be expected to touch either the hue gap
or (if it turns out to be a real, separate defect once bloom's own
possible overshoot is checked) whatever residual luma gap remains once
that is accounted for.

**Why hue, not only luma, this session.** The maintainer's own from-play
report named "lighting/illumination/**colors**", and this thread's own
"New 2026-09-13" entry already described the (now-fixed) grid-start wall
panel's residual as reading "flatter/greyer than the reference's cyan-
white" - a saturation/hue complaint. Mean luma also matched the reference
throughout the original 2026-08-20 brightness investigation and was exactly
why that defect survived review (clipped-white share discriminated there,
not luma) - so luma agreeing or disagreeing was never assumed to be the
whole story here either. `scripts/hd-frame-compare.py` gained mean HSV
saturation and a saturation-weighted circular mean hue per region this
session (unweighted circular mean of an angle is wrong at the 0/360 wrap;
weighting by saturation keeps near-grey pixels, whose hue is close to
meaningless, from outvoting strongly-coloured ones). No confidence score -
straight pixel arithmetic on committed frames, the same footing every other
number in this script's output already stands on.

**Two region caveats, both surfaced by `--dump-regions` on Amphiseum's own
frame rather than assumed from Talon's Junction's box shapes.** The `sky`
box lands on the dome's interior ceiling structure, not atmospheric sky (no
such thing exists in an indoor arena) - reported under that name for
continuity with the rest of the script's output, not because the label is
literally accurate on this circuit. The `road surface` boxes land on the
side floor panels beside the ship rather than open track, plausible but
unverified against a wider Amphiseum shot. **Neither caveat touches the
whole-frame number above**: `"whole (excl HUD, craft)"` only subtracts the
fixed HUD boxes and the centred craft box, both of which generalise across
circuits (the HUD is a fixed overlay; the craft is always centre-frame) -
it needed no Amphiseum-specific box work at all, which is why it is the
number this section leads with.

**A harness gap fixed along the way**: `hd-frame-compare.py` required an
exact pixel-size match between the reference PNG and this project's own
render, and raised on every pair this session captured - the reference
goes through `screenshot(trim=True)`'s crop-on-exact-`#000000`
(`rpcs3-capture.md`), which measured 1278x718 against this project's own
1280x720 canvas, a 1px margin per side rather than a real framing
difference. Center-crops both images to the smaller shared size when the
gap is 8px or less per axis; still raises loud past that, since a bigger
gap is a real mismatch this script should not silently paper over.

**Not run this session**: the `track_surface` per-material lead (this
thread's own sharpest one before today) and anything touching
`prelitScale`/`prelitPower`/`specular_exponent` - out of scope per this
lane's own brief until the sign question was settled, and the settlement
above argues for chasing Amphiseum's hue gap as a *separate* lead from
Talon's Junction's magnitude gap rather than assuming one fix reaches both.
Also not run: whether Amphiseum's own bloom chain is over-contributing
relative to the original's (a third hypothesis this session's bloom-on/off
check raised and did not distinguish from a per-material lighting gap -
the chain's exposure/scale constants are read off this circuit's own
`.envsettings` the same as Talon's Junction's, per the existing carry-
forward work, so "over-contributing" would mean the chain itself
misreads or misapplies something on this circuit rather than a missing
key). `mesh/`, `mesh.wgsl`, `emissive.rs` and `sky_cube.rs` were read this
session but not yet changed.

### Amphiseum's gap is vertical, not a single sign: the ceiling reads brighter, the floor reads darker, and the whole-frame number is an unstable average of the two (2026-09-17, `lane-hd-amphiseum-hue`)

**The maintainer's own answer, asked directly**: whether Amphiseum reads
brighter or darker than the original, and whether the complaint is
brightness or colour - **"Both, and it varies by area."** That single
sentence is the reason the whole-frame numbers in the section above were
never going to settle anything: a frame mean is an average, and an average
over a gap that changes sign by area is not a stable number at all - it
reads brighter or darker depending on how much of each area a given camera
framing happens to show, which is a fact about the camera, not about the
render.

**A box-free tile grid confirms this precisely and quantifies it.**
`scripts/hd-frame-compare.py` gained `--tiles ROWSxCOLS`: a plain `R x C`
grid over the frame (HUD and craft still excluded, same as every other
region), reporting luma and hue per cell with no claim about what a cell
contains - deliberately, because every hand-picked box on this circuit so
far has turned out to sample something other than its own name (`sky`
samples the dome ceiling, `road surface` samples a barrier wall). At
`--tiles 4x6` on the same 0 km/h grid pose, `--bloom on`:

| Row | What's there (by eye) | ref - ours (px-weighted) | typical hue gap |
| --- | --- | --- | --- |
| 0 | Dome ceiling, arch beams | **-0.121 (ours brighter)**, every one of 6 cells | 103-177 deg |
| 1 | Crowd stands, video screens | **-0.121 (ours brighter)** (folded into the row-0/1 weighted figure below), 5 of 6 cells brighter | 76-141 deg |
| 2 | Barrier chevrons, side floor | **+0.10 (ours darker)**, 3 of 4 valid cells darker | 50-86 deg |
| 3 | Floor closest to camera | **+0.10 (ours darker)** (folded into the row-2/3 figure), all 3 valid cells darker | 33-36 deg |

(Rows 0-1 and rows 2-3 are reported together above because that is the
natural split the per-cell table shows - top half uniformly one sign,
bottom half uniformly the other, with exactly two near-zero cells at the
seam.) **Pixel-weighted, the top half of the valid frame averages
ref-ours = -0.121 (ours brighter) and the bottom half averages +0.100
(ours darker)** - a 0.22-luma swing from vertical framing alone, arithmetic
directly off the tile table (top: 270,189 px, sum -32,697; bottom: 141,236
px, sum +14,102; the two together reproduce the whole-frame "-0.045"
figure above exactly, `(-32697+14102)/(270189+141236) = -0.0452`, which is
the whole-frame number's own derivation, not a separate measurement).
**This is larger than the 0.053 bloom-driven swing the section above
measured** - vertical framing moves the aggregate about four times as much
as the bloom setting does. Re-run at `--bloom off`: the same top-brighter/
bottom-darker split holds to within 1-2 hundredths of a luma per cell, so
this is not a bloom artefact either.

**The hue pattern is not simply "everything shifted the same way" - the
reference varies by row and ours does not.** Reference hue runs warm
(39-86 degrees, gold/yellow-orange) across the ceiling and stands, then
cool (155-198 degrees, cyan-blue) across the floor and barriers - two
genuinely different material colours, matching what the frame looks like
by eye (a warm-lit dome over a cool grey-blue track). **This project's own
render holds close to one hue family everywhere it draws colour at all**
(217-305 degrees, blue-violet, both top and bottom) - so the defect is not
only "our colours are shifted", it is closer to "our render is not
reproducing two materials' worth of colour *difference*, and collapses
toward a single one." That reads as a lit-material or lightmap-authoring
gap that is genuinely local to which material is being drawn, not a
uniform white-balance or transfer-curve error, which would be expected to
shift every row's hue by roughly the same amount rather than erase the
row-to-row difference the reference has.

**Talon's Junction's own tile grid, run for contrast, is close to
uniform**: 20 of 22 valid cells read "ours darker" (only two floor cells
at the very bottom-right read marginally brighter, -0.08/-0.10), and every
cell's hue gap is small (0-37 degrees, mostly under 20). This is the
concrete form of "Talon's Junction's gap is brightness-only and global,
Amphiseum's is not" the section above stated from the whole-frame numbers
alone - the tile grid is what actually shows it rather than inferring it
from one aggregate differing from another.

**Not yet done**: mapping which `.rcsmaterial`s draw into which rows
(`scripts/hd-material-probe.py`, noting its own open caveat that its
fog-segment classifier assumes the exposure scalar is `1.0` rather than
solving for it - worth checking whether that assumption holds worse in
the bright ceiling rows than it did on Talon's Junction, where it was
calibrated). The semantic labels in the table above ("dome ceiling",
"barrier chevrons") are read by eye against
`data/reference/hd-capture/amphiseum-grid/00.png`, not measured against
material names - a `--dump-regions`-style overlay for the tile grid was
not built this session, so treat the row/what's-there mapping as
orientation, not a claim with its own confidence score. `mesh/`,
`mesh.wgsl`, `emissive.rs`, `sky_cube.rs` still unchanged.

### The rows map to real materials, the albedo textures pixel-verify clean, and the ambient constant is itself the colour the ceiling reads (2026-09-17, later still, `lane-hd-amphiseum-hue`)

**Three things this session closes: which materials draw the ceiling rows,
whether their albedo `.gtf`s decode correctly, and where the ceiling's own
colour actually comes from.**

**1. The tile rows are real material boundaries, not an artefact of the
grid.** `scripts/hd-material-probe.py` gained `--pair-dir` (deriving
`--track` from the pair's own `00.json`, the same idiom
`hd-frame-compare.py`'s `track_arg` already uses, rather than the
hardcoded `talons_junction` the script shipped with - `talons-matched`,
its original pair, is confirmed gone from disk, so the default pair
changed to `talons-grid-recheck`) and the same center-crop tolerance
`hd-frame-compare.py` already carries for the reference's 1278x718 vs this
project's 1280x720 canvas. Run against `amphiseum-grid` pose 00
(`--bloom off`, coverage **79.1%**, below the script's own 90% floor - read
every number below as indicative, per the script's documented caveat that
its fog-segment classifier is calibrated by eye against Talon's Junction,
not solved for Amphiseum): the largest-magnitude "ours brighter" slots by
far are `animhexlights.rcsmaterial` (delta -0.526), `cf_diff_spec.rcsmaterial`
(-0.315, -0.151 across two slots), `base_diffusespecular.rcsmaterial`
(-0.277) and `lambert.rcsmaterial` (-0.023 to -0.146) - and reclassifying
`hd_material_probe_dump`'s own `slot_map` by screen position confirms they
draw **exactly the ceiling rows**: `animhexlights` at y 14-212 (centroid
y=158), `cf_diff_spec` at y 117-293, `lambert` at y 0-26, `base_diffusespecular`
(non-lightmap slot) at y 0-298 - against `track_wall` (y 296-523) and
`track_surface_no_emissive` (y 342-717) for the floor's own "ours darker"
population. No role bit cleanly separates the two groups (both sides carry
`no_ambient` on most slots - that bit does not gate the shader's ambient
term at all, see `mesh.wgsl`'s own "This ambient reaches materials the disc
never feeds it to" comment above) - the split is positional, matching the
tile grid exactly, not a role-bit distinction the material system already
expresses.

**2. The albedo textures pixel-verify clean - the channel-order/decode
hypothesis is refuted for this circuit's ceiling materials specifically.**
`crates/texture/examples/gtf_to_png.rs` (already existed, unused until now)
decodes a bare `.gtf` to PNG; extracting the three dome-family textures
straight off the disc (`scripts/psarc.py cat`) and decoding them -
`dc_hexgrid.gtf` (256x256, DXT1, `animhexlights`'s own texture),
`and_metaldark.gtf` (512x512, DXT1, `cf_diff_spec`'s), `dc_cement_base_edges.gtf`
(512x512, DXT1, `base_diffusespecular`'s) - shows **all three are neutral
grey-to-dark-grey by eye and by number**, no blue, violet or gold cast in
any of them. So this session's own version of the maintainer's brief lead
1 (channel order / `.gtf` decode) is a **clean negative** for the DXT1
albedo textures actually driving the ceiling's colour - whatever bends
these materials' output toward blue-violet is downstream of the texture
sample, in the lighting term, not in the decode.

**3. Near-grey albedo through a coloured light term reads as the light
term's own colour - and Amphiseum's own `Lighting.Constant ambient color`
is blue-violet on disc.** `mesh.wgsl`'s `lit_sum = scene.light.ambient +
prelit + vertex_light + sun_diffuse` multiplies the near-grey albedo
above, so a material dominated by one term reads close to that term's own
hue. Three of the four ceiling materials carry no `lightmap` role at all
(`animhexlights`, `cf_diff_spec`, and `lambert`'s non-lightmap slot), so
`prelit` is zero for them by construction (the no-lightmap placeholder);
their `vertex_light` is zero wherever the chunk declares no colour set; and
`sun_diffuse = scene.light.sun * clamp(dot(n, scene.light.direction), 0, 1)`
clamps to zero for a ceiling-facing normal (pointing down, toward the
camera) against Amphiseum's own authored sun direction
(`(0.20957, 0.57352, 0.79193)`, a strongly upward-pointing vector - the dot
product with a downward normal is negative, clamped away). That leaves
`scene.light.ambient` as **the only non-zero term** for most of the
ceiling's own drawn pixels. Reading Amphiseum's `track.envsettings`
directly (`scripts/psarc.py cat`): `"Lighting.Constant ambient
color"=0.745098 0.611765 0.925490` - RGB with B > R > G, hue **265.5
degrees**, a match in kind (not to the exact degree - other terms and
gamma still move it) to the **285 degrees** this session measured on the
rendered ceiling strip (`rgb=[0.64, 0.61, 0.65]`, sat 0.11, `coloured_pct`
66.5%, y 30-260 box). Talon's Junction's own `Constant ambient color` is
**also** blue-leaning on disc (`0.403922 0.392157 0.509804`, hue 246
degrees) - so the ambient constant being blue is not Amphiseum-specific -
but Talon's Junction's `Sun color` reaches HDR magnitude and is warm
(`2.0 1.827451 0.886275`, R and G » B) and its pose 00 geometry mostly
faces the sun, so `sun_diffuse` dominates there and the same blue ambient
never surfaces. **This is a plausible mechanism, not a closed one**: it
explains why Amphiseum shows the gap and Talon's Junction does not, using
only values already read off the disc and already wired into this
project's shader, with no new assumption beyond "the ceiling's own
`sun_diffuse` is near zero", which the authored sun direction and the
geometry's own facing both support but this session did not measure the
per-pixel `ndl` value to confirm.

**What is authored beside the wired ambient and is not consumed at all:
`Lighting.Sky colour`** - `140 140 140 0` on Amphiseum, `128 128 128 0` on
Talon's Junction, both **neutral grey**, unlike either circuit's tinted
`Constant ambient color`. `docs/formats/envsettings.md`'s own table already
recorded this key as "Read, unused" together with `Ambient false direction`
(`(0.1, 1.0, 0.1)` on Amphiseum, `(1.0, 1.0, 1.0)` on Talon's Junction - a
per-axis weight, plausible shape for a normal-driven blend, not decoded).
Neither is read by `crates/game` or `crates/render` today (confirmed this
session, `rg` over both crates) - `SKY_COLOUR` and the false-direction key
exist only as parsed `oag_tables::envsettings` constants with no consumer.
**A neutral `Sky colour` blended toward for up-facing-away-from-sun
geometry, in place of (or alongside) the tinted `Constant ambient color`,
is exactly the shape of mechanism that would turn Amphiseum's ceiling
neutral/warm without touching Talon's Junction's sun-dominated frame at
all** - but this is a hypothesis built from the *shape* of the authored
data, not a read of what the original engine's shader actually does with
either key, and per `CLAUDE.md`'s rule this is not something to wire on
that basis alone.

**New RE this session, in support of the above.** The function at
`0x003a83d8` (`EBOOT.elf`) is the environment-schema registrar: called from
`Environment_LoadRaceScene` (`0x003f44b4`), it walks the full `.envsettings`
key list this project's own `envsettings.rs` already names, writing each
into one struct at a persistent global base (`iRam008b6fb4`) - `Constant
ambient color` lands at struct offset `+0x430` (four floats, read through
`_opd_FUN_005d3ec0`, the same reader every other `vec3`/`vec4` colour key
in the file uses) and `Sky colour` at `+0x440` (read through a *different*
reader, `_opd_FUN_005d40b8`, consistent with this project's own reading
that it is four bytes rather than four floats - see `envsettings.rs`'s
"Two number encodings" section - and defaulting to `0xffffffff` rather
than a float default when the key is absent). **So `Sky colour` is
genuinely read into the same live per-environment struct `Constant ambient
color` lands in** - confirming and sharpening `envsettings.md`'s existing
"Read, unused" line with the actual function and offset, not just the
string's own address - **but whether anything downstream ever reads struct
offset `+0x440` back out is not traced this session.** That is the
concrete next step: xref the struct base past this registrar (it is
described elsewhere in this thread as "a persistent, cumulative object",
carried forward between the front end and a race) to find every reader of
`+0x440`, and check whether any of them gates on the surface normal the
way the `Ambient false direction` key's shape suggests. Named
`Environment_RegisterLightingSchema`, confidence 80 (the call pattern
matches the full documented key list exactly, one field write per key, and
it is reached from the already-named `Environment_LoadRaceScene`) - see
`names.tsv`. The rename linter's PascalCase/verb warnings on this name are
the tool's own convention mismatch, not a naming defect - see this
project's own recorded note on that.

**Not run this session**: tracing `+0x440`'s own consumer (if any) past
this registrar; measuring the per-pixel `ndl` term directly to confirm
`sun_diffuse` is actually zero on the ceiling's own drawn chunks rather
than assumed from the authored sun direction and normal facing; extending
the material cross-check past the 79.1%-coverage, `--bloom off` sample;
wiring anything - no shader or Rust code changed. `mesh/`, `mesh.wgsl`,
`emissive.rs`, `sky_cube.rs` read again this session, still unchanged.

### `Lighting.Sky colour`'s consumer is found and is a backdrop clear, not a material term; `Constant ambient color` is confirmed wired exactly as this project already assumes (2026-09-17, later still, `lane-hd-ambient-light`)

**A struct-offset correction to the section above, found while tracing its
own consumer.** `Environment_RegisterLightingSchema`'s registrar calls, read
again directly off the decompile: `_opd_FUN_005d3ec0(iVar16,iVar16 + 0x420,
puVar9,0)` where `puVar9 = PTR_s_Lighting_Constant_ambient_color_008b6fe8`
one line above - **`Constant ambient color` lands at `+0x420`, not `+0x430`**
as the previous session's entry states. `+0x430` is `Sun color`
(`_opd_FUN_005d3ec0(iVar16,iVar16 + 0x430,PTR_s_Lighting_Sun_color_008b6ff4,0)`,
the very next call). `Sky colour` at `+0x440` and `Sky rotation` at `+0x444`
are unaffected and were already right. Cross-checked against seven other
key/offset pairs this same decompile carries (`Fog Color +0x4e0`, `Fog
Density +0x500`, `Alternate Fog Color +0x4f0`, `Alternate Fog Density
+0x504`, `Bloom adaption rate +0x52c`, `Tone adaption boost +0x540`, `Tone
maximum brightness +0x548`) and all seven match this page's own
TOC-resolved table exactly - which is what says this decompile's TOC is
sound and the `+0x430` line was this thread's own error, not a second
disagreeing reading.

**`Lighting.Sky colour` (`+0x440`) does have a consumer, and it settles the
open RE question two sessions asked** - `Scene_PrepareFrame` (`0x003aa888`)
reads it at one call site, pinned by tracing the local it reads through
rather than trusting offset coincidence (the exact trap this page's own
"Read correctly" paragraph above warns about): `iVar44 = EnvSettings_GetOrCreate()`
is called immediately on entry to the block gated by `Lighting.Debug_Draw_sky`
(`+0x591`, defaults to `1` in the registrar - like every other `Debug_*` key
on this schema, a production render-enable flag despite the name, not a
debug-only toggle), `iVar44` is never reassigned before the read, and the
adjacent statement in the same block reads `*(float *)(iVar44 + 0x444)`
(`Sky rotation`) to scale a rotation angle - both offsets landing exactly
where the registrar wrote them is the corroboration, not just one address
matching by luck. **What it feeds is a screen backdrop/clear-fill, not a
material lighting term**: gated by `g_ZoneEffectsActive` (a global bool,
already named), the normal-race branch (`== 0`, which is Amphiseum's grid
pose - Zone mode is a separate game mode) packs the Sky colour byte into an
RSX clear-colour word and issues it through `Rsx_SetMethod`/a GCM fill call;
the Zone-mode branch calls `Sky_DrawGradientDome` instead, using entirely
different fields (a separate zone-effects colour table, not `+0x440`) -
Zone mode substitutes its own animated palette rather than reading the
circuit's authored sky colour at all. **This is a background pass that runs
before or independently of material draws, not a light term any
`.rcsmaterial` shader consumes** - it cannot explain the ceiling's colour,
because the ceiling is drawn opaque geometry (already established: real
`animhexlights`/`cf_diff_spec`/`lambert`/`base_diffusespecular` chunks, not a
gap in the geometry) that would occlude any backdrop fill underneath it
regardless of what colour that fill is. **This refutes the "does the
original consume `Lighting.Sky` where we only consume `Lighting.Constant`"
lead from the brief that opened this thread** - the original does consume
it, but for an unrelated subsystem, so wiring it into `mesh.wgsl`'s ambient
term would not be a recovered term, it would be inventing a use the disc's
own code never gives it.

**`Constant ambient color` (`+0x420`) is confirmed wired into the shader
exactly the way this project's own `scene.light.ambient` already assumes -
no missing operation here either.** Still inside `Scene_PrepareFrame`, under
the same `g_ZoneEffectsActive == 0` branch (so: the normal-race case, same
branch the Sky-colour finding above sits in), the function copies `+0x420`
(`Constant ambient color`), `+0x450` (`Sun direction`, normalised),
`+0x4a0` (`Ambient false direction`, normalised), `+0x4c0`/`+0x4d0`
(`Prelit ambient colour scale`/`power`) and `+0x4e0` (`Fog Color`) into a
scratch region, then binds each one into a per-draw shader-parameter table
through the same `(pointer, count)`-pair binding idiom this page's own
`fogColour` investigation already established (`Render_SetClipPlanes_q`'s
surrounding block) - traced pair by pair, not by position: `Constant
ambient color` to `iVar56+0x158`, `Sun direction` to `+0x1d8`, `Ambient
false direction` to `+0x178`, `Prelit ambient colour scale` to `+0x198`
(and copied a second time into a separate local slot, `+0x7c00`, that this
same function's own later vector arithmetic reads and writes back to - not
traced past that), `Prelit ambient colour power` to `+0x1b8`, `Fog Color` to
`+0xf8`. So the original's per-frame setup feeds the material pipeline a real
`Constant ambient color` term, unconditionally, every frame a race is not in
Zone mode - matching what `mesh.wgsl`'s `scene.light.ambient` already does
with this same disc value. Whatever produces Amphiseum's warm ceiling in the
original, it is not "the wired ambient term is fabricated" or "our ambient
source is a different key than the original's" - both engines read the same
`Constant ambient color`.

**`Lighting.Enable ambient false lighting` (`+0x5a0`) is confirmed inactive
on both circuits, checked through the persistent-store carry-forward this
project's own `staged_envsettings` already models, not just the circuit's
own file.** The registrar defaults this bit to `0`; `amphiseum/track.envsettings`
and `talons_junction/track.envsettings` (`scripts/psarc.py cat`) declare
neither `Enable ambient false lighting` nor `Enable Prelighting`, and
neither does either front-end file `staged_envsettings` carries values
forward from (`/data/fe/fe.track.envsettings`, checked in both its `DATA00`
and `DATA02` copies) - so the persistent-store mechanic that saved Sol 2's
own `Tone` triple does not save this bit here either; it is off by every
route. **This does not mean the false-direction machinery is dark**: the
normalised `Ambient false direction` vector and the `Prelit ambient colour
scale`/`power` values are still computed and bound to the shader parameter
table every frame regardless (the copy described above runs unconditionally
under `g_ZoneEffectsActive == 0`, with no check of `+0x5a0` anywhere in
`Scene_PrepareFrame`) - only whatever *downstream* code gates its own
behaviour on that bit is confirmed inactive on these two circuits; that
downstream consumer was not traced this session. The key names themselves
("Prelit ambient **false specular** power/intensity") read as a fake/fixed
specular-highlight system for lightmapped surfaces that have no dynamic
light to compute a real one from, not a diffuse hemisphere-ambient blend -
a shape of mechanism unlikely to explain a wholesale material hue shift even
if it were active, though this is a reading of the key names, not the
microcode, and carries no confidence score.

**A third candidate is checked and closed narrowly: neither circuit's own
`.vex` authors any `AmbientLight`/`DirectionalLight`/`PointLight` node.**
`Lighting.Enable dynamic lights=1` is authored on both circuits, and the
schema carries a real SPU-driven per-vertex dynamic-light subsystem this
project implements none of (`Debug_Draw_light_volume` at `+0x59c`,
`Debug_Stall_for_spu_light_volume` at `+0x5a7`, `Enable_spu_vertex_light` at
`+0x5a3`, all present as registered keys) - a plausible source for a
localised, per-area colour difference a flat ambient constant cannot
produce. A new one-off diagnostic, `crates/render/examples/light_census.rs`
(built on `oag_vex::vex::nodes`/`class_id`, the same API
`vex_class_ground_truth.rs` already validates against the whole HD disc),
swept every `.vex` file under both `data/environments/amphiseum/` and
`data/environments/talons_junction/` (9 files each, including the 814-node
`amphiseum/track.vex` and 826-node `talons_junction/track.vex`) and found
**zero** nodes of class `0x12c` (`AmbientLight`), `0x131`
(`DirectionalLight`) or `0x132` (`PointLight`) on either circuit. **State
this narrowly**: it rules out these three vex node classes as the dynamic
light's data source on these two circuits specifically - it does not rule
out the SPU light path being fed from a different, unenumerated source (a
class this sweep did not check, or data carried on HD's own `.rcsmodel`
geometry rather than the `.vex` scene tree), and it says nothing about any
other circuit.

**Net for this thread's own opening question**: three candidate mechanisms
for "does the original apply a light term to the ceiling that this project
does not" are now checked - `Lighting.Sky colour` (consumed, but by an
unrelated backdrop pass), `Enable ambient false lighting` (authored off,
every route checked), and vex-authored dynamic lights (none present) - and
all three are refuted. **The root cause of Amphiseum's ceiling colour gap
is not established by this session.** Per this project's own rule against
tuning to a reference, no shading code was changed - `mesh/`, `mesh.wgsl`,
`emissive.rs`, `sky_cube.rs` are unchanged. The sharpest remaining, unchecked
lead is the same one two sessions ago already named and did not run: the
per-pixel `ndl` on the ceiling's own drawn chunks, to confirm rather than
assume `sun_diffuse` is genuinely zero there, and a per-material microcode
sweep (`scripts/ps3-microcode.py`) of the four ceiling materials for which
named engine parameters (if any) their own fragment programs actually
declare - `fogColour`, `Ambient false direction` and `Constant ambient
color` are all now known-bound engine parameters a material's microcode
could reference by hash, but which of the four ceiling materials reference
which was not checked this session.

### The per-material microcode sweep: none of the four ceiling programs ever references the ambient constant, and a fifth material shares the same code-path defect (2026-09-17, `lane-hd-ceiling`)

**The sweep the previous session named and did not run.** For each of
Amphiseum's four ceiling materials (`animhexlights`, `cf_diff_spec`,
`lambert`, `base_diffusespecular` - the non-lightmap slot), this session
took the variant row this project's own `mesh::rcs::skin::variants` resolves
for the real ceiling chunks (a new example,
`crates/render/examples/hd_amphiseum_ceiling_variants.rs`, dumping
`Model::material_variants`) and dumped its fragment **and** vertex program
with `scripts/ps3-microcode.py`, resolving every patched constant slot's
name hash against this project's own preimage tables
(`oag_rcs::rcsmaterial::names`, the engine parameter table above, and
`docs/formats/rcsmaterial.md`'s preimage tables). The four resolved variants,
by material-probe slot (`scripts/hd-material-probe.py --pair-dir
data/reference/hd-capture/amphiseum-matched`, coverage 84.2% at this pose):

| Material | Slot | Feature hash | Fragment block (offset) | Declares `constantAmbientColour`? |
| --- | ---: | --- | --- | :---: |
| `base_diffusespecular.rcsmaterial` | 353 | `0x56c94426` | `0x2b60`, 1040 B | **No** |
| `cf_diff_spec.rcsmaterial` | 360 | `0x56c94426` | `0x84f0`, 1040 B | **No** |
| `animhexlights.rcsmaterial` | 325 | `0x56c94426` | `0x6f60`, 720 B | **No** |
| `lambert.rcsmaterial` | 207 (and 8 more) | `0x56c94426` | `0x3270`, 496 B | **No** |

**Zero of four.** None of the four programs this renderer actually draws for
Amphiseum's ceiling patches `~crc32("constantAmbientColour")` (`0x81db67ea`)
anywhere in their constant tables - confirmed by grep over the full patch
list `ps3-microcode.py` prints per block, not by absence of a plausible
name. `mesh.wgsl`'s `lit_sum = scene.light.ambient + prelit + vertex_light +
sun_diffuse` adds `scene.light.ambient` (`constantAmbientColour`)
unconditionally to all four regardless. **This is the wrong operation, not
a missing one**, confirmed directly rather than inferred from the `NO_AMBIENT`
role bit alone (which the code comment at `mesh.wgsl`'s "This ambient reaches
materials the disc never feeds it to" already flagged as unwired, for
different reasons - see below).

**The equation each program actually computes**, read instruction-by-instruction
(`lambert`'s block is the clearest - no specular, one texture):

```
lambert (feature 0x56c94426, fragment block @0x3270, vertex block @0x3020):
  ndl      = saturate(dot(normalize(N), directionalLight0DirectionWorldSpace))
  diffuse  = ndl * directionalLight0Colour
  ambientTerm = f[TC0].xyz     -- see vertex program below; NOT constantAmbientColour
  litColour = diffuse + ambientTerm      -- f[TC1], f[TC2] are also added but are
                                          -- literal (0,0,0) in this material's own
                                          -- vertex program (`MOV o[TC1].xyz, c[206]`
                                          -- where c[206] = (0,0,0,0)) - dead code,
                                          -- not a second real term
  fogFactor = exp2(-(view_depth * fogColour.w)^2)
  result   = lerp(fogColour.rgb, litColour * texture(unit0), fogFactor)

lambert's vertex program computes f[TC0].xyz:
  ambientTerm = pow(v[3], prelitBias) * prelitScaleSpecular
  -- v[3] is attribute 0x1aaf7631/colorSet1, decoded by this project's own
  -- `VertexDecl::light_colour_set()`/`Mesh::vertex_light` as HD's baked
  -- per-vertex light, and read RAW (no sRGB decode) - `LG2 v[3] -> MUL
  -- prelitBias -> EX2 -> MUL prelitScaleSpecular`, unlike the lightmap path's
  -- `pow(baked.rgb, 2.2)` predecode.

animhexlights, cf_diff_spec and base_diffusespecular's non-lightmap block
compute the same `litColour` (diffuse + f[TC0]-only ambientTerm, f[TC1]/f[TC2]
again literal zero in every one of the three vertex programs dumped), then
add a second, texture-driven term before the fog lerp:
  animhexlights: result += texture(unit0, diffuse-coord) * 0xef18f362
                         + texture(unit1, TextureGradient) * 0x7611a2d8 * texture(unit0)
  cf_diff_spec / base_diffusespecular: a Blinn-style specular,
    spec = (N.H)^SpecularPower * directionalLight0Colour * SpecularColour(or)
           SpecularColor * texture(unit1, specular map)
    added alongside the diffuse*albedo term, before the same fog lerp.
```

**`prelitBias`/`prelitScaleSpecular` are named from the executable's own
string table** (`Shader_InitEngineParams`, engine slots 13/12 above), not
matched to the `.envsettings` key names by string - "Specular" in
`prelitScaleSpecular` does not textually match `Lighting.Prelit ambient
colour scale`. The identification rests on the **values and the arithmetic
shape** instead: Amphiseum's `track.envsettings` authors exactly one
`LG2 -> MUL -> EX2 -> MUL` pair of constants system-wide for this role -
`"Lighting.Prelit ambient colour scale"=6.0 6.0 6.0` and `"Lighting.Prelit
ambient colour power"=3.5 3.5 3.5` - and the engine parameter table
(`Shader_InitEngineParams` above) declares exactly one `prelitScaleSpecular`/
`prelitBias` pair and no other prelit-named entry at all; `mesh.wgsl`
already reads these same two `.envsettings` keys into `scene.light.
prelit_scale`/`prelit_power` for the **lightmap** path's identical curve
shape (`prelit = prelit_scale * pow(baked_linear, prelit_power)`,
`crates/tables/src/envsettings.rs`). Confidence 80: the value/shape match is
exact and the two engine-table names are the only candidates, short of a
traced write into the constant-patch mechanism itself (not done this
session).

**A three-way permutation, not a two-way one - checked by dumping the third
variant.** `animhexlights` and `lambert` each ship a *third* resolved
feature hash on the ceiling's own material file, `0xfb61d927` (slots 329 and
280 respectively - other chunks of the same file, off the ceiling). Both
declare and patch `constantAmbientColour` outright:

```
animhexlights, feature 0xfb61d927, fragment @0x2030:
  parameter 0x81db67ea (constantAmbientColour) patch fslot 0x78, patch slot 0x12
  @0x11  MOV H4.xyz, {0x81db67ea}      ; ambientTerm = constantAmbientColour, flat
lambert, feature 0xfb61d927, fragment @0x2940:
  parameter 0x81db67ea (constantAmbientColour) patch fslot 0x56, patch slot 0x4
  @0x03  MOV H1.xyz, {0x81db67ea}      ; ambientTerm = constantAmbientColour, flat
```

So the original's own compiled shader table already carries **three**
ambient sources, selected by `Features::chunk_word`'s field bits (this
project's own name for the permutation, confirmed matching `rcsmaterial.rs`):
`IleLightmap` -> `pow(lightmap, prelit_power) * prelit_scale`, `IleVertex` ->
`pow(colour_set, prelitBias) * prelitScaleSpecular`, `Ambient` (neither) ->
flat `constantAmbientColour`. **`mesh.wgsl` always computes the third case**
(`scene.light.ambient`, i.e. `constantAmbientColour`) regardless of which of
the three the resolved chunk's own program actually is - correct only for
chunks that resolve to `Ambient`, wrong for every `IleLightmap`/`IleVertex`
chunk, which is every chunk this project's own role census already flags
`NO_AMBIENT` (`declared.takes_constant_ambient()` is false) but does not
gate on, per the standing `mesh.wgsl` comment above. Confidence 90: the
`Ambient`-variant microcode is dumped directly, twice, and both patch the
exact hash this project's own `scene.light.ambient` binds.

**Reconstructing the arithmetic for a neutral-grey texel** (all four
albedo textures pixel-verify neutral, per the previous session): the
`ambientTerm` is the only non-zero lighting input on a ceiling-facing chunk
(`sun_diffuse` clamps to zero against Amphiseum's own upward sun direction,
as established two sessions ago), so `ambientTerm`'s own hue is the surface's
hue. Read the four ceiling materials' actual baked colour-set data off the
disc (`crates/render/examples/hd_amphiseum_ceiling_vertex_light.rs`, new
this session) at the resolved `IleVertex` slots, and the curve is **not**
uniform across the population - it splits exactly along the same lines the
tile-grid and material-probe rows already drew:

| Material (slot) | Vertices | Raw mean hue | Top-decile-by-luma hue | Curved (`pow(x,3.5)*6`) top-decile hue |
| --- | ---: | ---: | ---: | ---: |
| `base_diffusespecular` (353) | 15,256 | 193.2° | 191.0° | 200.0° |
| `cf_diff_spec` (360) | 1,688 | 192.2° | 190.2° | 201.6° |
| `animhexlights` (325) | 1,712 | 70.1° | 56.5° | **45.5°** |
| `lambert` (207) | 72 | 2.9° | 78.4° | **42.8°** |

**Two of four land in the reference's own 39-86° warm-band hue, with two
caveats that keep this a hue result and not a closed one.** `animhexlights`
(45.5° curved, 1,712 vertices, stable across raw/top-decile/curved at
70/56/45°) is the load-bearing one of the two. `lambert`'s own number is not:
72 vertices total, a top decile of 7, and its own raw-mean (2.9°) and
top-decile (78.4°) hues disagree by 75° on the *same* population - that
instability is itself evidence the number is noise, not a second
confirmation, and it is reported as a hue only, not a finding. Second,
`animhexlights`' own top-decile colour is close to grey - `rgb=(0.6204,
0.6161, 0.5461)`, a channel spread of about 0.07 on a value of 0.62, roughly
11% saturation - and HSV hue is a much noisier statistic on a near-grey
colour than on a saturated one. The reference's own "39-86° warm" reading
(two sessions ago) used a saturation-weighted circular mean specifically
because an unsaturated pixel's hue is not to be trusted alone; this session's
`45.5°` was not checked against the reference's own saturation at the
matching region, for the reason in the next paragraph. Both materials' own
declared parameters are genuine colours, not modifiers of a texture that
could hide a different source: `animhexlights` multiplies its albedo by an
unnamed `0xef18f362` = `(0.060, 0.211, 0.424)` and its gradient sample by
`0x7611a2d8` = `(1,1,1)` (identity - `pads.md`'s "Speedup Pad" reading of
this hash does not generalise to this material, it is drawn straight
through here); neither is where the warm hue comes from - the colour-set
curve alone already lands there. The mean statistic is the wrong one to
read here and this session corrects it explicitly: `pow(x, 3.5)` amplifies
the top decile far more than the mean (a vertex at 1.0 curves to 6.0, one at
0.25 to 0.07 - an 85x spread), so the population that actually dominates a
curved sum is the brightest tenth, not the arithmetic mean, which is why the
table reports both.

**A magnitude check against the material-probe's own measured reference
luma does not corroborate the hue result, and is left as a discrepancy
rather than smoothed over.** Multiplying `animhexlights`'/`base_diffusespecular`'s
curved top-decile term by their own DXT1 albedo's mean RGB (`dc_hexgrid.gtf`
untried this pass; `dc_cement_base_edges.gtf` mean `(0.679, 0.655, 0.629)`,
`and_metaldark.gtf` mean `(0.173, 0.175, 0.172)`, both read with
`crates/texture/examples/gtf_to_png.rs`) overshoots
`hd-material-probe.py`'s own measured reference luma for these slots by
roughly 4-5x (e.g. `base_diffusespecular`'s curved term times its albedo
lands close to full white on two channels, against the probe's own
`ref_mean 0.1933`). The top decile is the population a curved *sum* is
dominated by in the vertex data, but a screen pixel is an interpolated blend
across a whole triangle, most of whose area is not its brightest vertex - so
this mismatch says the top-decile statistic is the wrong stand-in for "what
the surface looks like on screen," not that the curve or the ambient finding
is wrong. **The magnitude question is open; only the hue-family question was
checked this session**, and the doc says so rather than presenting one
positive check as if it were two.

**Two of four are a clean, disc-value negative, not an inconclusive one.**
`base_diffusespecular` and `cf_diff_spec` stay in a 190-202° cool-blue family
at every stage - raw mean, top decile, and after the curve, which can only
amplify the population's *existing* skew (an equal exponent on all three
channels is monotone and cannot change which channel is largest, so B>G>R
in stays B>G>R out; 193° cannot become 60°). Both materials' own
`SpecularColour`/`SpecularColor` parameters are achromatic (`(0.498, 0.498,
0.498)` and `(1, 1, 1)` respectively) so the specular term these two add
cannot be hiding a colour source either. **Applying the correct operation
to these two specific materials would not turn them warm** - whatever
supplies their share of the reference's warm reading, if any, is not in
this term. This is the fix's scope, stated precisely rather than
overclaimed.

**A self-caught data-handling error, recorded so the next session does not
repeat it**: a per-region follow-up initially read three tile-grid cells
(`r0c4`/`r0c5`/`r1c5`) as genuinely warm (52°/334°/4°) in *this* pose and
built a "fifth material corroborates the bug at a verified pixel" finding on
top of that. Those three warm readings are real, but they came from pose
`01`, not pose `00` - `scripts/hd-frame-compare.py --pair-dir
data/reference/hd-capture/amphiseum-matched --tiles 4x6` prints one grid per
pose (`00`, `01`, `03` by default) back to back, and grepping for
`r0c4`/`r0c5`/`r1c5` without separating them by pose merged the two.
Rerun with `--pose 00` alone: those same three cells read **200-205°**,
consistent with the rest of pose `00`'s top row and with `mesh.wgsl`'s own
render there (**226/259/243°**) - no warm signal in pose `00` at all, in
this region or apparently anywhere in its own top-row band. The
`cf_diff_spec`/`base_diffusespecular`-dominated per-region classification
this session ran was built from pose `00`'s own camera, so it cannot be
paired with pose `01`'s warm tile-grid reading regardless - and per this
same thread's own "Not chased" line from two sessions ago, poses `01`/`03`
are moving (431-529 km/h) with the original's own speed streak baked into
their reference frames by construction, which this project's static render
cannot reproduce and which this session has no tool to segment by material
at all. **So the "warm pixels dominated by the cool-negative pair, plus an
emissive fifth material" claim is retracted as stated** - it rested on
comparing a slot classification from one pose against a hue reading from
another. What survives, on its own footing: `uvanim_diffuse_emissive`'s own
resolved fragment program (`IleVertex`, feature `0x56c94426`, block
`@0x6d60`, dumped independently of any pixel pairing) shares the exact
`constantAmbientColour`-free, `f[TC0]`/`f[TC1]`/`f[TC2]`-additive shape the
four ceiling materials do, plus a genuine `EmissiveTexture` (`0xb1f2a176`)
glow layer - and its own role bits (`no_ambient|add_second`, not `no_sun`)
put it on the exact `mesh.wgsl` code path this section's fix targets
(`(in.slots & 192u) == 192u` is false for it, so it takes the full,
ambient-corrupted `lit_sum`, not the untouched `EMISSIVE` branch). That is a
fifth material sharing the code-path defect, established by microcode alone
- not a fifth material confirmed warm at a verified pixel, which is what
the draft this replaces claimed. A short RPCS3 recapture was separately
attempted, to get a static pose that shows more of the dome directly
(`scripts/rpcs3-drive.py capture --nav "Main Menu=right" --nav "Track
Creation=right,right,right,right,right,right,right,right" --team feisar_c1
--hull-variant concept1 --load 20 --interval 2`); it landed on Talon's
Junction instead - the menu route this session's build takes inserts a
"Single Player" screen the cited `rpcs3-capture.md` walk does not name, so
its eight-`right`s-at-`Track Creation` carousel count does not carry over
unchanged. Not chased further this session (menu-navigation exploration is
outside this lane's brief); RPCS3 and its Xvfb display were stopped
cleanly. **Still open, unchanged by this correction**: whether
`animhexlights`/`uvanim_diffuse_emissive`'s own colour is what the
reference's warm reading (wherever it genuinely occurs) actually traces to,
and whether `base_diffusespecular`/`cf_diff_spec` ever read warm at any
verified, correctly-paired pixel - not established either way this session.
RPCS3 was not otherwise needed for the core finding: the brief's gate for a
live read ("none of the four programs can produce warm gold from static
disc inputs") is false - `animhexlights` does, from disc values alone,
independent of any capture or pose.

**The fix implied, sequenced, and its exact scope, for the coordinator to
land in `mesh.wgsl` (not touched this session - out of this lane's files)**:

1. Gate `scene.light.ambient`'s addition on the chunk's own resolved
   permutation, not on a blanket bit. The existing `NO_AMBIENT` role bit
   (`declared.takes_constant_ambient()`) already answers "does this
   chunk's own program ever reference `constantAmbientColour`" correctly for
   every chunk measured this session and two sessions ago (Anulpha Pass,
   Talon's Junction) - gating `scene.light.ambient`'s addition on it directly
   (drop the term where `NO_AMBIENT` is set) is the first half.
2. **That drop alone regresses the frame to near-black** on a
   `IleVertex`/no-lightmap, sun-occluded chunk - `prelit` is already zero
   (no lightmap), `sun_diffuse` is already zero (facing away from the sun),
   and `vertex_light` (`in.colour.rgb`, already wired) is the *raw*
   colour-set byte with no curve, which for most of these vertices is far
   dimmer than the curved value. So the drop and the addition are one
   change, not two: add `scene.light.prelit_scale * pow(in.colour.rgb,
   scene.light.prelit_power)` in place of the flat `vertex_light` term on
   this same `NO_AMBIENT`-and-colour-set path, reusing the uniforms already
   bound for the lightmap curve rather than a second pair.
3. **State the domain explicitly, or the port inherits a decode step the
   microcode does not have.** The lightmap path decodes `baked.rgb` through
   `pow(_, 2.2)` before the prelit curve; the colour-set path's vertex
   program reads `v[3]` directly (`LG2 -> MUL -> EX2 -> MUL`, no sRGB
   step) - applying the lightmap's 2.2 predecode to the vertex-colour path
   would be inventing a step the disc's own vertex program does not run.
4. This is still a family of materials this project has already found is
   not one bit (`mesh.wgsl`'s own "Gating on that bit alone was tried on
   2026-08-24 and is a regression" comment, about the fully-`EMISSIVE`
   family) - the `IleVertex` case above is `NO_AMBIENT` but **not** `NO_SUN`
   (it does take `directionalLight0*`), so it is not `EMISSIVE`
   (`NO_AMBIENT | NO_SUN`) and the existing emissive branch does not touch
   it; the change above is additive to that branch, not a rewrite of it.

Confidence on the fix's *shape* (drop-and-replace, not drop-alone): 88 -
read directly off two materials' vertex and fragment microcode, cross-checked
against the already-wired lightmap curve's own uniforms. Confidence on it
*fully* explaining Amphiseum's hue gap: not claimed - two of the four
materials measured stay cool regardless, so this is a partial, precisely-
scoped fix, not a closing one.

### `uvanim_diffuse_emissive`'s own colour is read, and its largest populations do not confirm the warm reading (2026-09-18)

**Answers this thread's own open item**: does `uvanim_diffuse_emissive`,
already confirmed (two sessions above) to share the four ceiling materials'
exact `NO_AMBIENT`-and-not-`NO_SUN` code-path defect by microcode alone,
also share their *colour* - extending the fix's confirmed reach past four
materials? First, its own resolved slots needed locating:
`hd_amphiseum_ceiling_variants.rs`'s material-name filter now includes
`uvanim_diffuse_emissive.rcsmaterial`/`cf_uvanim_emssive.rcsmaterial`
(committed, reused rather than a one-off script). Amphiseum's model draws
`uvanim_diffuse_emissive` at nine slots; five resolve to the exact
`0x56c94426`/fragment-offset-`0x6d60` variant this thread already
identified by hash - `178, 354, 357, 382, 432` - and the rest resolve to
different, unrelated variants at other offsets, not checked here.
`hd_amphiseum_ceiling_vertex_light.rs` (unmodified - it already takes slot
numbers on the command line) read the same raw/top-decile/curved
reconstruction the four ceiling materials got, on all five:

| Slot | Vertices | Raw mean hue | Top-decile hue | Curved top-decile hue |
| ---: | ---: | ---: | ---: | ---: |
| 178 | 432 | 194.6° | 194.2° | 199.4° |
| 354 | **2,960** | 192.4° | 190.1° | 198.3° |
| 357 | 360 | 188.5° | 180.5° | 180.8° |
| 382 | **1,664** | 193.3° | 187.3° | 190.3° |
| 432 | 168 | 87.5° | 102.1° | 102.5° |

**Mixed, and the dominant instances do not confirm the warm reading.**
Slots 354 and 382 - 2,960 and 1,664 vertices, both larger than
`animhexlights`' own 1,712-vertex population this thread already treated as
load-bearing - sit at 190-198° curved, the same cool-blue family
`base_diffusespecular`/`cf_diff_spec` occupy, not the reference's 39-86°
warm band. Slot 178 agrees (199.4°, 432 vertices). Slot 432 does not: 87.5°
raw mean, 102.1° top-decile, 102.5° curved - stable across all three
stages, on 168 vertices, so it fails neither of the two reasons this thread
already discarded `lambert`'s reading for (too few vertices, and a 75°
raw-versus-top-decile disagreement on the same population). It is a real,
internally-consistent warm-ish outlier, not a discredited one - just outnumbered
30 to 1 by the 5,056 vertices across slots 178/354/382 that read solidly
cool. **Net: on the population that dominates, `uvanim_diffuse_emissive`
joins `base_diffusespecular`/`cf_diff_spec` as a third material whose own
vertex-colour-set data does not explain the reference's warm reading, with
one small instance (slot 432) going the other way** - it shares the code
defect (established) but not, from disc colour data alone, the hue the
defect was suspected of hiding, on the geometry that makes up most of what
this material draws. The "if its colour reads warm, the fix's reach is
broader" conditional in this thread's Next Steps resolves negative for the
dominant instances and unresolved for slot 432's own geometry (not
identified or checked against a capture); `EmissiveTexture`'s own sampled
colour (the glow layer this material also carries, separate from the
vertex-colour-set term measured here) is untouched by this session and is
the only remaining place a warm contribution from this specific material
could still come from at the slots that read cool.

`cf_uvanim_emssive.rcsmaterial` was swept incidentally by the same filter
change, but its slots at this feature hash resolve to a different fragment
block (offset `0x23f0`, not `0x6d60`) - a different variant than the one
this thread identified, so its own readings answer nothing here and are not
reported.

**2026-09-18, later still: `EmissiveTexture`'s own colour is read too, and
the glow term is real, wired, and reaches the screen everywhere checked -
but its estimated magnitude does not overturn the reading above.** New
tool, `crates/render/examples/hd_amphiseum_emissive_texture.rs`, reads
`rcsmodel::Material::samplers` directly for the `EmissiveTexture`
(`0xb1f2a176`) entry - `(name hash, path)` pairs, no unit lookup or shader
resolution needed - and `material.parameters` for `emissive.rs`'s own
`TINT` (`0xe8bcd7f5`, `quads` confirmed `1` on every read, matching a
float3). Three findings:

- **The sampled `EmissiveTexture` is achromatic on four of five slots**
  (`and_verticalemissive.gtf`, slots 354/357/382/432: mean `(0.397, 0.397,
  0.397)`, saturation `0.000` by this tool's own saturation-based guard -
  fixed this session after a `delta <= 1e-6` epsilon let a computed mean's
  floating-point noise through as a spurious `180°` on one of the five rows
  in the table above). Slot 178's own texture (`m_lightstripv01_e.gtf`) is
  genuinely saturated, `(0, 0.134, 0.134)` at saturation `1.000` - real
  cyan, not noise.
- **The glow's own colour is `TINT`, not the texture - and `TINT` is warm
  on exactly the two slots (354, 432) whose vertex-colour-set data read
  cool.** `TINT = (0.8257, 0.2331, 0.0231)` there (hue 15.7°, saturation
  0.972); `357`/`382` carry a cool-blue `TINT` instead (`(0, 0.505, 1.0)`/
  `(0, 0.575, 1.0)`, 206-210°) that agrees with their own already-cool
  vertex-colour reading; `178` is white (`(1,1,1)`, no colour
  contribution). This material's glow is wired and drawn: `Model::
  lightmaps[slot]` is `Some` on every one of the five (the decoded second
  texture reaches the renderer), and each slot's own `TINT` value is
  present in `Model::emissive`'s built, deduplicated table. **354 and 432
  are not two independent warm signals** - identical `TINT` and identical
  albedo (`and_stadiumglowstrip.gtf`) mean they are the same authored decal
  placed at two locations, not two different surfaces that happen to agree;
  432's own warm vertex-colour outlier and its warm `TINT` are one surface
  type, sampled twice, not a second confirmation.
- **Estimated magnitude, `tint * EmissiveTexture_mean * albedo_alpha`
  (`mesh.wgsl`'s actual blend, not reproduced - this is the same
  order-of-magnitude estimate the vertex-colour curve check used, not a
  pixel measurement): slots 354/432 add roughly `(0.026, 0.007, 0.001)`**
  - small, because their shared albedo's alpha mean is `0.078`, the lowest
  of the five - **against `357`/`382` adding roughly `(0, 0.05-0.09,
  0.09-0.16)`**, 3-6x larger, because their own albedo alpha (`0.23`/
  `0.40`) is proportionately higher. So accounting for the glow does not
  overturn the dominant-cool reading: the warm contribution on 354, the
  material's largest population by far (2,960 vertices), is a small nudge
  from its own weakest-alpha glow, while the cool contribution on the two
  vertex-colour-set-cool instances is the larger of the two effects
  measured, reinforcing rather than offsetting their own already-cool
  reading. **Not a pixel-level check** - no capture, no triangle-interpolated
  reconstruction, the same caveat the vertex-colour curve's own magnitude
  discrepancy already carries two sessions above - so this narrows rather
  than closes "does the glow layer explain any part of the reference's
  warm reading here," and does so in the negative for the instances that
  matter most by vertex count.

### `base_diffusespecular`/`cf_diff_spec` stay cool because of an unverified byte-order assumption in this project's own code, not because the disc's own data is cool (2026-09-18, confidence 70)

**The question this thread's own Next Steps names**: "Check
`base_diffusespecular`/`cf_diff_spec` specifically for a channel-order or
byte-order defect in their own baked colour-set decode." `oag_rcs::
rcsmodel::Mesh::vertex_light` reads the colour-set attribute's four bytes
as `[data[at], data[at+1], data[at+2], data[at+3]] = [r, g, b, mask]`, in
file order, no swap - and **that mapping is an assumption this project's
own code makes, not a read of anything the disc states.** `Attribute`
(`crates/rcs/src/rcsmodel/vertex_decl.rs`) carries a byte offset, a
component count and an RSX vertex type, and nothing else - no remap field
the way `.gtf`'s texture fetch has one (`Texture_BuildGcmRegisters`,
elsewhere on this page). Whether RSX vertex-fetch's `RSX_UBYTE_NORM` path
maps byte 0 to `.x` is exactly the fact in question, and nothing in this
codebase currently states it either way.

**Tested by swapping R and B and reconstructing the same way
`hd_amphiseum_ceiling_vertex_light.rs` already does** - new tool,
`crates/render/examples/hd_amphiseum_ceiling_channel_order_probe.rs`
(committed), top-decile-by-luma then Amphiseum's own authored curve,
before and after an R/B swap, on the same three materials this thread
already has readings for:

| Material (slot) | Vertices | As decoded, curved hue (sat) | R/B swapped, curved hue (sat) |
| --- | ---: | --- | --- |
| `base_diffusespecular` (353) | 15,256 | 200.0° (0.947) | **39.7°** (0.951) |
| `cf_diff_spec` (360) | 1,688 | 201.6° (0.949) | **38.4°** (0.951) |
| `animhexlights` (325, control) | 1,712 | **45.5°** (0.233) | 128.5° (0.145) |

**The swap lands both unexplained materials inside the reference's 39-86°
warm band, at saturation 0.95 - four times the saturation of
`animhexlights`' own accepted 45.5° reading - and breaks `animhexlights`'
own correct reading under the identical transformation.** That last part is
the control the hypothesis has to survive, not just the two materials it
would explain: a transformation that turns a known-good reading bad while
turning two known-bad readings good, on the same axis, at high confidence
on the population size that dominates the frame (15,256 and 1,688 vertices
against `animhexlights`' 1,712), is the shape of evidence for "the wrong
axis is flipped for two of three," not for "curve-fitting to the
reference." Per CLAUDE.md's rule against a corrective rotation: **this is
not tuning a parameter to match a target** - the swap is a fixed,
structural hypothesis (byte 0 is blue, not red) applied uniformly and
checked against a negative control, not a fitted rotation with no
falsifiable shape. **A caveat on the control's own weight**: an R/B swap
is an involution, so "it flips both directions" alone is automatic, not
evidence by itself - reversing any wrong-axis hypothesis breaks whatever
currently reads right on that axis. What actually carries weight is the
*asymmetry*: the swapped pair lands at saturation 0.951 on 15,256 and
1,688 vertices, while what it breaks is `animhexlights`' own 45.5°
reading at saturation 0.233 - a reading this thread already flagged as
near-grey and noisy two sessions above, not a confident one. Read this as
"the swap is confidently right on two large, saturated populations and
confidently wrong on one small, unsaturated one," not as two independent
confirmations of the same measurement.

**Two alternative mechanisms were checked and ruled out.** (1) *Shader
swizzle*: both `base_diffusespecular`'s and `animhexlights`' own vertex
microcode read the colour-set attribute identically - `LG2 R0.x, v[N].
xxxx` / `.yyyy` / `.zzzz`, straight order, no swizzle, on both (`ps3-
microcode.py vp-file`, blocks `0x2910` and `0x6cb0` respectively). If the
defect were in how a specific material's *shader* reads the attribute,
these two programs would differ; they do not. (2) *Wrong attribute
selected*: `VertexDecl::vertex_colour()` picks the first `components==4 &&
rsx_type==RSX_UBYTE_NORM` attribute that isn't `tangent` - a broad filter
that could in principle grab the wrong one on a chunk with two candidates.
Checked directly: `base_diffusespecular`'s and `cf_diff_spec`'s
declarations carry exactly **one** such attribute each (hash `0x1aaf7631`,
the same hash `animhexlights` uses), so there is no ambiguity to
mis-resolve. Both rule out a shader- or selection-level explanation,
leaving the byte order itself - authored, or read - as what is actually in
question.

**Only one circuit was sampled.** All three readings come from Amphiseum's
`track.vex` alone - the probe iterates every chunk of the named slot on
that one file, not a single representative chunk, but whether some chunks
elsewhere need one byte order and others the opposite is untested at the
per-chunk granularity a conditional fix would eventually need. Nothing here
speaks to any other circuit.

**Confidence 70, not higher, and not because the result is weak - because
the rubric's ceiling for this evidence type is 84 and this falls under it
without a runtime or register-level read.** What would move it: a decompile
of whatever this binary's vertex-array-format GCM setup looks like would
settle byte order directly, the way `Texture_BuildGcmRegisters` settled the
texture case - but the two cases are not symmetric. `Texture_
BuildGcmRegisters` is a real function with seven real callers, findable and
found by an earlier session precisely because it is callable. This page's
own "device layer" section already establishes that `cellGcmSetVertexDataArray`
is one of the draw-state entry points with **no import and no call site to
find at all** - "inline functions in the `cellGcmSys` headers that write RSX
methods straight into the command buffer" - so the vertex case may have no
equivalent named function to search for, only inlined method writes
scattered through the draw path. **Searched for a named function this
session and did not find one** (`EBOOT.elf` carries no `Vertex_*`-prefixed
function yet - `search_functions name_pattern=Vertex` returns only
`Image_SetVertexColours`, `Rsx_UploadVertexConstantBlock/Constants`, none a
per-chunk register builder), which is the useful negative: **the next
search should look for the RSX method-write pattern itself** (an immediate
load of the `NV4097_SET_VERTEX_DATA_ARRAY_FORMAT` method base, around
`0x1740`, in the draw-setup path) rather than for a named function, since
one may not exist to find. That read would tell whether byte order is a
hardware-fixed property (in which case exactly one of
`base_diffusespecular`/`cf_diff_spec`'s new reading or `animhexlights`' old
one is right, disc-wide, and the other's agreement
with its own reference band is coincidence) or something else entirely.

**Not changed: `Mesh::vertex_light`'s decode.** A global swap is not
justified by this evidence - it would fix two materials and break
`animhexlights`, and nothing found this session explains *why* some chunks
would need one order and others the opposite, which any code change would
need before it could be conditional rather than a coin flip. Left as a
disc-value finding for whoever reads the vertex-fetch registers next.

### `Enable_spu_vertex_light` is read at 14 sites, two named and reachable, and gates a double-buffered slot - the light computation itself is not traced (2026-09-18, confidence 75)

**Where this picks up.** Two sessions above, this page named a real, entirely
unread mechanism: `Lighting.Enable dynamic lights=1` is authored on both
checked circuits, and the schema carries `Enable_spu_vertex_light` (`+0x5a3`),
`Debug_Draw_light_volume` (`+0x59c`) and `Debug_Stall_for_spu_light_volume`
(`+0x5a7`) as registered keys - "a plausible source for a localised, per-area
colour difference a flat ambient constant cannot produce," never traced past
the registrar. This session traces it as far as static reading goes without
a live capture or an SPU disassembler, neither attempted here.

**The registrar confirms the offset and the default, and adds a fourth key
this page had not named.** `Environment_RegisterLightingSchema` (`0x003a83d8`,
decompiled directly this session) writes `*(undefined1 *)(iVar16 + 0x5a3) = 1`
before calling `_opd_FUN_005d46b8(iVar16, iVar16 + 0x5a3,
PTR_s_Lighting_Enable_spu_vertex_light_008b703c, 2)` - **`Enable_spu_vertex_light`
defaults to `1` (on)**, on both the fresh-init and the cached-reset path.
Immediately adjacent in the same registrar: `Debug_Draw_spu_light_volume` at
`+0x5a4` (default `0`), a distinct key from `Debug_Draw_light_volume` at
`+0x59c` not previously listed on this page - what, if anything,
distinguishes the two is not established here, only that both exist as
separate registered keys. Four keys now located in this cluster:
`Enable_spu_vertex_light` (`+0x5a3`, on by default), `Debug_Draw_light_volume`
(`+0x59c`, off), `Debug_Draw_spu_light_volume` (`+0x5a4`, off),
`Debug_Stall_for_spu_light_volume` (`+0x5a7`, off).

**14 read sites found disc-wide** (`search_instructions mnemonic=lbz
operand_pattern=0x5a3`, all real `lbz rN, 0x5a3(rM)` loads, not incidental
matches): `0x003ea4ec`/`0x003ea964` (one function, two sites), `0x003eb950`,
`0x003fa654`, `0x003fc32c`, `0x003ff9d4`, `0x00400b7c`, `0x00401f5c`,
`0x00402f60`, `0x00403c20`, `0x00405a1c`, `0x004076d4`, `0x004092c0`,
`0x0040d9cc`. **Two of fourteen sit in already-named functions**:
`Shadow_CompileAmbientShadowTrackRedraw` (`0x00402a88`, the read at
`0x00402f60`) and `Shadow_CompileShadowedTrackRedraw` (`0x004053e0`, the
read at `0x00405a1c`) - both cached shadow-redraw *command-list compilers*,
not the main per-frame material draw path. **The other twelve are
unexamined this session** - ten distinct `FUN_*` functions
(`0x003ea368`, `0x003eb890`, `0x003fa558`, `0x003fc140`, `0x003ff860`,
`0x00400a00`, `0x00401ba8`, `0x00403a30`, `0x004074e0`, `0x00408fa8`,
`0x0040d990`), named here so the next session does not re-derive the list.
**Do not infer the subsystem's role from the two that happen to be named
already** - two of fourteen is not a basis for characterizing what the
other twelve do, and nothing here rules out a material-lighting consumer
among them.

**In the one site decompiled, `Enable_spu_vertex_light` is read paired with
`Debug.Enable EdgeGeom` (`+0x5aa`, also on by default) in a single `&&`
condition** - `Shadow_CompileAmbientShadowTrackRedraw`'s decompile: `cVar34 =
*(char *)(iVar30 + 0x5a3); ... if ((cVar34 != '\0') && (*(char
*)(iVar30 + 0x5aa) != '\0')) { iVar30 = _opd_FUN_0040d390(); ... }`.
`Shadow_CompileShadowedTrackRedraw` was checked by instruction search, not
decompiled: it carries `lbz`s at `0x00405a1c`/`0x00405a44` reading the same
two offsets, 0x28 bytes apart - the same two keys read close together, not
a confirmed `&&` the way the first site's decompile shows. **"EDGE" here is
Sony's real middleware, confirmed by string, not a project-internal name**:
`"EdgeGeom"` (`0x007b1758`), `"Debug.Enable EdgeGeom"`/`"...stalling"`
(`0x007b09c8`/`0x007b09e0`), `"edgeDecompressorTaskset"` (`0x007a7d28`),
`"edgezlib_inflate_queue.cpp"` (`0x007bedc0`), and the literal embedded task
binary `"edgezlib_inflate_task.spu.elf"` (`0x007dd8c4`), alongside a full
`cellSpurs*`/SPURS-kernel string set. **These confirm EDGE's `zlib`
decompression module ships with its own named SPU task ELF - they do not
confirm an EDGE *geometry* job exists or that it computes vertex lighting.**
Only one bare `"EdgeGeom"` string and the two `Debug.Enable EdgeGeom*` key
names speak to EDGE geom specifically; nothing here traces what, if
anything, an EDGE geom job does, and the claim "EDGE geom computes the SPU
vertex light" is not established - only that the two keys are read together
at two call sites.

**What the gated call actually does, read directly - and it is buffer
selection, not a light computation.** `FUN_0040d390`:
`return *(undefined4 *)(*(int *)(iRam008b83b0 + 0x2080) * 4 + iRam008b83b0 +
0x2084);` - reads a 4-byte value at a per-slot offset. `FUN_0040d370`:
`return *(int *)(iRam008b83b0 + 0x2080) * 0x1000 + iRam008b83b0 + 0x80;` -
computes the address of a `0x1000`-byte (4 KiB) slot, indexed by the same
value both functions read from `+0x2080`. This is the shape of a
double-buffered (or multi-buffered) region with a running index and
fixed-size slots - consistent with, but not proven to be, an SPU job's
output buffer, the same shape this project's own `scripts/
rpcs3-trail-dump.py` and `hd-flare-sprite-dump.py` already read live for
other "double-buffered SPU-output vertex buffers." **What the buffer at
`iRam008b83b0 + 0x80 + index*0x1000` actually holds is not read this
session** - not vertex data, not a light value, not anything: only its
address and slot arithmetic are established.

**Confidence 75.** Decompilation-only evidence caps at 84 per the rubric,
and this session's central open question - what the selected buffer
contains - is inferred from stride and naming convention, not read. Two
independently-found call sites agreeing on the identical paired-flag shape,
and real middleware strings corroborating that "EDGE" names a genuine
Sony subsystem rather than a project-internal token, are what keep it above
the 50-69 "plausible inference" band.

**Next steps, in the order that costs least first.** (1) The twelve
unexamined call sites listed above - cheap, the addresses are in hand,
no new search needed. (2) A live RPCS3 read of `iRam008b83b0 + 0x80 +
index*0x1000` (translated to its runtime address) during a race with
`Enable_spu_vertex_light`/`Debug.Enable EdgeGeom` both at their default
`1` - this project already has the tooling shape for exactly this (the
trail/flare scripts read a live double-buffered SPU-output region the same
way), and it would settle what the buffer holds without needing SPU
disassembly at all. (3) Locating and decompiling `edgezlib_inflate_task.spu.elf`
(or any EDGE-geom-specific SPU task, if one exists and is separately
embedded) - **this needs an SPU-architecture Ghidra processor module this
project has never set up for any title**, PSP/PS2 tooling being MIPS/EE and
PS3's own PPU work being PowerPC; say this explicitly so nobody starts here
expecting the existing toolchain to reach it. Static PPU reading alone
cannot answer what runs on the SPU side.

**2026-09-18, later still: the first of the twelve unexamined call sites is
checked, and it retires the "both named sites are shadow-compile" framing
the confidence note above declined to generalise from.** `FUN_003fa558`
(`0x003fa558`) is called from `Scene_PrepareFrame` itself and is the
mainline per-frame material-draw dispatcher - the loop that iterates drawn
chunks and binds their parameters into the GCM draw-state struct, not a
cached shadow-redraw compiler. It carries the same gate, **with a confirmed
`&&`** (unlike the second shadow site, which was only instruction-searched):
`cVar2 = *(char *)(iVar22 + 0x5a3); ... if (((cVar2 != '\0') && (*(char
*)(iVar22 + 0x5aa) != '\0')) && (iVar26 = _opd_FUN_0040d390(), iVar26 != 0))
{ _opd_FUN_0040d370(); }` - run once per frame, before the per-chunk loop
starts, not per-material. **Three sites now read, one of them mainline: the
subsystem reaches the main draw path, not only shadow compilation** - the
"narrows what it could explain" inference this page already declined to
draw stays declined, but the premise that made it worth declining (shadow
sites only) no longer holds either.

**In this site, `FUN_0040d370`'s return value is also discarded.** Its own
disassembly is seven plain instructions (`lwz`/`add`/`blr`, no channel
operations) computing a slot address and returning it; here that address is
computed and thrown away, same as at the other checked site. Read this
narrowly: the pair reads as a guarded poll whose effect, if any, is not
visible in either decompile checked so far - not as evidence of a hidden
side effect, which would be a guess this project's own confidence rubric
has no room for below its floor.

**The buffer's base address chain is closed on the static side, and it
does not reach a usable address.** `iRam008b83b0`'s TOC-relative load
(`FUN_0040d390`: `lwz r11,-0x5014(r2)`) resolves cleanly against its own
function's TOC (`scripts/ps3-toc.py toc 0x0040d390` reports `exact`, no
defect this time) to address `0x008b83b0` - matching what the decompiler
already showed, not correcting it. `scripts/ps3-toc.py resolve 0x0040d390
-0x5014` reads the static word there as `0x00f4b300`, classified `None` -
not a string, not a named symbol, and **well past `OPD_HI`
(`0x008A54D8`)**, this binary's own code/data ceiling this page's earlier
sections already established. That places it outside any static segment
this project's tooling has reason to trust as meaningful: `0x008b83b0`
holds a pointer, and `0x00f4b300` is what the *unrelocated* ELF happens to
carry there, not the runtime-allocated buffer a job system would assign at
startup. Xrefs to both addresses (`get_xrefs_to`) return nothing, which is
consistent with a heap pointer assigned by an allocator this session did
not chase, not with a fixed global. **The static value is not the buffer
and chasing it further statically will not become one.**

**Live capture was attempted and is blocked by this machine, not by the
method.** `scripts/rpcs3-drive.py preflight`: no virtual input device
("the `evdev` module is not importable, so no virtual device can be
created at all") and no virtual display on `127.0.0.1:77` - both stated as
this machine's own limits, not as unconfigured-but-reachable. **The
recipe for whoever has a working RPCS3 setup**, so this does not need
re-deriving: read the pointer at `0x008b83b0` live (its real runtime
value, not the static `0x00f4b300`), follow it, read `+0x2080` for the
current slot index, then `+0x80 + index*0x1000` for that slot's own
4 KiB - the same double-buffer-reading shape `scripts/rpcs3-trail-dump.py`
already uses for other SPU output, with `Enable_spu_vertex_light`/
`Debug.Enable EdgeGeom` both left at their default `1` so the gate is open
when the read happens.

**Confidence 75 at the point this was written.** The third site strengthens
reach - the subsystem is mainline, not shadow-only - but the central
unknown, what the selected slot holds, was exactly as unread as before;
reach and content are different questions, and only content moves the
score. No code changed; none of the three lanes that session (this one, the
byte-order swap, the emissive-texture read) found evidence meeting the bar
this project sets for changing shipped behaviour, and each says so on its
own page rather than implying otherwise.

**2026-09-18, later still: the live capture happened, on the environment
this page said blocked it, and the content is read.** The blocker was this
machine's own state, not a hard limit - `uv run --with evdev` (rather than
plain `python3`) already satisfies the `evdev` dependency without a system
package, and `scripts/rpcs3-drive.py display` starts the missing Xvfb :77.
Both fixed live, in-session; `scripts/rpcs3-spu-light-dump.py` (committed)
is the reproducer, built on `rpcs3-trail-dump.py`'s own boot/drive/attach
shape.

**The pointer is never relocated.** `0x008b83b0`'s *live* value read
`0x00f4b300` on every boot - byte-identical to the static ELF value this
page's own TOC-resolved read already reported. Not a heap pointer the
engine fills in at startup; a fixed low address, assigned once. (The first
live attempt guarded on the `0x10000000-0x50000000` heap range
`rpcs3-trail-dump.py`'s own allocations use and rejected every read as a
result - corrected in the committed script, noted here so the next reader
does not repeat it.)

**What is there, measured twice, on two different circuits (an
unidentified Fury campaign default, then Amphiseum specifically): up to 8
records of 8 big-endian floats, `(x, y, z, w=1.0, A, A·0.25, A·0.1, D)`.**
The `A·0.25`/`A·0.1` ratios are exact - checked across every record read on
both runs, including scaled ones (`A` ranges roughly 40-440 across the
sample) - so each record carries one authored-or-computed scalar with two
fixed-ratio derived forms, not three independent values. `D` is a fourth,
independent per-record scalar that does **not** follow that ratio and
stays stable for a given position across multiple frames rather than
varying like noise (confirmed: a record at `(29.048, -49.855, 146.856)`
carried the identical `D = 0.9414657354354858` across three consecutive
Amphiseum snapshots). **Confidence 80 for this structural claim alone** - a
direct, repeated measurement on two circuits, the strongest evidence tier
below a corroborated runtime trace on a second binary.

**Some records move at racing speed; at least one stayed byte-identical
across every snapshot taken, on both circuits.** On the unidentified Fury
circuit, `(183.698395, -36.554867, 202.262344, ...)` was unchanged across
three consecutive 2-second-spaced reads. On Amphiseum, `(-15.340508,
-50.138016, -176.605759, ...)` was unchanged across all five. **This is a
property of the buffer's structure, reproduced on two circuits, not an
artefact of one track.** Whether the moving records specifically track the
player's own craft is **not confirmed** - inferred only from one record's
position lying close to where a *different, unrelated* capture placed the
player seconds earlier, which is not the same pause. The honest claim is
narrower: several records' positions change frame to frame by magnitudes
consistent with racing speed; which entity, if any, they track is open.

**Tested directly against the "these are authored `.vex` markers" reading,
and it fails.** New tool,
`crates/render/examples/hd_amphiseum_spu_light_marker_check.rs`, checked
Amphiseum's stationary position against **every node of every class**
(2,790 world-transform translations, all nine `amphiseum/*.vex` files, not
only `PointLight`) - the closest is `start_grid.vex`'s class `0x6e`
(`Transform`) at `(0, 0, -140.414)`, 63.71 units from the target. **63.71
units is a real miss at this circuit's own scale, not noise**: the buffer
records this session read span hundreds of units apart (positions from
roughly `-410` to `480` on one axis alone), so the 5-unit epsilon the tool
checked against is generous relative to node spacing, and the closest hit
is over twelve times that epsilon away. The closest node is also a weaker
candidate than its distance alone suggests - two of its three translation
components are exactly `0`, the shape of a root-relative or unparented
`Transform` rather than a placed marker, which argues further against it
being what the buffer's position traces to, not for it. **No authored
scene-tree node sits at the position the SPU buffer holds stationary.**
This is evidence against a `.vex`-authored-marker source, not proof of
one: the position could still derive from `.rcsmodel` geometry, a
racing-line/waypoint format this project has not read, or a genuinely
computed value with no authored anchor at all. It does mean the
"just track waypoints" reading, the obvious non-lighting alternative,
does not survive the one check available without more RE.

**What remains unread and unconfirmed, precisely.** Slot 1 (the buffer's
other half - the index alternates 0/1 across boots, and one early capture
read `index_raw: 1` but the script did not yet dump slot contents at that
point) has never actually been read; the `A·0.25`/`A·0.1` ratio is
confirmed for slot 0 only. No shader consumer for this buffer's contents
has been found - the standard 81-entry engine-parameter table this page
already documents has no slot shaped for an 8-record array, so if
anything reads this data, it is through a mechanism this project's
existing name-hash sweep (`hd-pointlight-sweep.py`) would not see: a
vertex texture fetch, most plausibly, which samples a buffer rather than
binding a named parameter, and which no tool in this project currently
checks for. **Confidence on anything naming what the records represent -
lights, markers, waypoints, something else - stays at 75 or below**; the
structural measurement (80) and the identity of what is measured are
different claims, and only the first is solid.

No code changed. This is a genuine, reproducible, live-verified structure
now - stronger evidence than any static reading in this section reached -
and still not evidence of what consumes it or what it represents, which is
what this project's own rule against inventing a term the disc does not
demonstrably compute continues to withhold from `mesh.wgsl` until one of
those two questions closes.

**2026-09-18, later still: vertex texture fetch, named above as the most
plausible mechanism the existing name-hash sweep cannot see, is checked
disc-wide and comes back clean.** NV40's vertex ISA has a real texture-fetch
opcode, `TXL` (`0x19` in `scripts/ps3-microcode.py`'s own `VEC_OPS` table) -
a vertex program that samples a texture would use it. Swept every vertex
block the same way `hd-pointlight-sweep.py` already sweeps fragment/vertex
parameter tables, but by opcode rather than by name hash: **zero `TXL`
instructions in any of the 60,324 vertex blocks across all 1,632
`.rcsmaterial` entries.** One nominal hit turned up in `EBOOT.elf`'s own
126 resident blocks (block 32, instruction 0) - not investigated further,
because its own decode reads as architecturally implausible on its face
(`TXLC o[POS]., v[0].xxxx, ?00000.xxxx` - a texture-fetch result written
straight to clip-space position, which no ordinary shader would do) and,
genuine or not, its shape (one sample, not a loop or array read) does not
match what a consumer of an 8-record buffer would need regardless. **Not
committed as a tool** - a three-line addition to an existing method, not a
reusable one, and the result is a clean negative rather than a lead to
re-run. Net: of the two consumer mechanisms this thread could think to
check (named engine parameters, vertex texture fetch), both are now
disc-wide negatives. What remains unchecked is a mechanism this session
did not think of, or a genuinely inert computation - and this project's
own repeated pattern elsewhere on this page (`AmbientLight` on Pulse,
`pointLight0*` disc-wide) is exactly "authored and computed, never
consumed," which is what this now most resembles without yet being it.

### The twelve unexamined call sites are read, and two of them are a real shader consumer this project's own two prior sweeps could not have found - plus a distinct producer (2026-09-18, later still, confidence 82)

**All twelve of the previously-unexamined `Enable_spu_vertex_light` call sites are now read** (Ghidra program state confusion cost real time this session - a concurrent session silently switched the MCP bridge's active program to the Vita `2048` binary partway through, so several early lookups against bare addresses returned "no function"/"unable to read bytes" against the wrong binary entirely; recovered by always passing `program="EBOOT.elf"` explicitly, per the tool's own warning). Two named shadow-redraw compilers were already known; `FUN_003fa558` (mainline draw dispatcher, discard pattern) was checked two sessions ago. Of the ten remaining:

**Six are more instances of already-seen shapes.** `FUN_003fc140`, `FUN_00403a30` (identical bodies, a "Zone Stage" command-stream compiler) and `FUN_003ff860`, `FUN_00400a00` (identical bodies, a second such compiler) all read `Enable_spu_vertex_light`/`Debug.Enable EdgeGeom`, call `FUN_0040d390`/`FUN_0040d370`, and **discard the address** exactly as the three sites this page already documented do - reset to `0`/`0` in fields `+0x150`/`+0x14c` immediately before and after, return value of `FUN_0040d370` never stored. `FUN_003ea368` (both its two call sites) and `FUN_003eb890` are a distinct object family - their own hashed-string constants (`RigidBody`, `AbsorbFader`, `LeachFader`, `AbsorbScroller`, `LeachScroller`) mark them as track-hazard/prop draw compilers, not shadow or mainline-material code - and here the pattern is not pure discard: `_opd_FUN_0040d390()`'s return (the buffer's read *value*) and `_opd_FUN_0040d370()`'s return (the slot *address*) are stored into context fields `+0x150`/`+0x14c` immediately before `Render_RunCompiledOps_q` runs, then reset to `0`/`0` immediately after. This project did not trace `Render_RunCompiledOps_q` to confirm those fields are read inside it, so this is circumstantial rather than a proven consumer, but it is a different shape from the six pure-discard sites and is flagged as such.

**Two are a real, direct, per-chunk shader consumer.** `FUN_004074e0` and `FUN_00408fa8` are two more instances of the same "Zone Stage" command-stream compiler family as the six discard sites above - same `EnvSettings_GetOrCreate`/`FUN_003aa2e8`/`2f8`/`308` preamble, same `local_190`/`local_1a0`-style opcode stream - but here `local_12c`/`local_144` (the buffer *value*, from `FUN_0040d390`) and `local_128`/`local_140` (the slot *address*, from `FUN_0040d370`) are cached **once per draw call in outer-scope locals, not reset**, and then read **per chunk**, gated by a bit test against a per-chunk-indexed byte table:

```c
if (((1 << (chunk_id & 7) & (uint)(byte)table[(chunk_id >> 3) + CONST]) == 0) || (address == 0)) {
    /* opcode 0x2d, operands (0, 0) - inert */
    variant_hash = Shader_GetVariantHash(flags);                 // no 0x800 bit
} else {
    /* opcode 0x2d, operands (address, value) */
    variant_hash = Shader_GetVariantHash(flags | 0x800);          // 0x800 bit set
}
```

(`FUN_004074e0`'s table is `PTR_DAT_008b8264[(chunk_id>>3) + 0x205400]`; `FUN_00408fa8`'s is `PTR_DAT_008b82b0[(chunk_id>>3) + 0x5400]` - two different base-pointer symbols, not confirmed to be the same underlying table this session, but an identical bit-test idiom on both.) This is a genuine per-chunk enable flag: when the chunk's own bit is set and the buffer's read value is nonzero, the compiled command stream carries `(slot_address, buffer_value)` as a project-internal opcode-`0x2d` operand pair, re-emitted fresh before every chunk (not once per draw call), and the compiler selects a **distinct shader variant** (`Shader_GetVariantHash(... | 0x800)`) for exactly those chunks - the same `0x800` bit convention already established at the two named shadow-redraw sites and the `RigidBody`/`Absorb`/`Leach` sites above. **This is why neither of this project's own two prior sweeps found a consumer**: opcode `0x2d` is this project's own render-command-stream vocabulary, not a shader-declared parameter name (`hd-pointlight-sweep.py`'s method) and not a `TXL` vertex-texture-fetch instruction (the disc-wide microcode sweep two sessions ago) - it is a project-internal command interpreted by whatever runs the `local_190`/`local_1a0` opcode stream (`Render_RunCompiledOps_q`, not traced this session), and nothing in this project's tooling watches that vocabulary. **What opcode `0x2d` actually does on the GPU side - which RSX register(s) `(address, value)` end up written to - is the single highest-value next step**, and it is the piece that would finally settle content: if `address` is bound as a `NV4097_SET_VERTEX_DATA_ARRAY_OFFSET`-style vertex-fetch source rather than an ordinary constant register, that would explain how a vertex program reads an 8-record array through a mechanism neither the name-hash sweep nor the `TXL` opcode sweep could see, and would also connect to the separate, still-open `base_diffusespecular`/`cf_diff_spec` byte-order lane's own stalled search for "the RSX vertex-array-format register builder."

**A distinct producer function is also found, at a different offset pair within the same `0x008b83b0` structure.** `FUN_0040d990` is not a reader - it *writes* an 8-float, 32-byte record to `iRam008b83b0 + 0x20a0 + count*0x20` and increments a running count at `iRam008b83b0 + 0x2098`, capped at `0x80` (128 slots) - a completely different offset pair from the `+0x2080`/`+0x80`-stride-`0x1000` double-buffered 8-record array this project has already live-captured, inside the same base structure. It is gated by the identical `Enable_spu_vertex_light` flag (`+0x5a3`) as every read site on this page. It is reached through a common one-instruction thunk, `FUN_006778c8` (`_opd_FUN_0040d990()`, tail call), which is itself called from **fifteen distinct sites**, all in the `0x000c0000-0x00155000` range - a completely different region of the binary from every function this section has examined so far, and far too many call sites to be one hardcoded light list. One examined caller, `FUN_000cfb80`, extracts a world-space position via a per-object transform lookup (`FUN_00323760`, keyed by a field at the caller's own `+0x6adc`) and a scalar that is floor-clamped to a minimum (`+0x6a7c`, clamped against `DAT_008a8b00`, then scaled by two further constants) before being passed to the producer - consistent with a per-object glow or intensity value contributing its own position and brightness as a dynamic-light candidate, though the calling object's own class is not identified this session (no name recovered, and the other fourteen call sites are unread). **Whether this 128-slot candidate list is what later gets selected down into the 8-slot buffer already live-captured is not established** - no selection/compaction function connecting the two offset pairs was located this session; the link is inferred only from the shared base pointer and matching 8-float record shape, not traced.

**Confidence 82** for the structural claims in this section (a real per-chunk consumer exists and embeds `(address, value)` via a project-internal opcode; a distinct, differently-gated producer exists, reached from many call sites) - two independently-found consumer sites agreeing on the identical bit-test-then-opcode-emit shape, and the producer's own gate and offset arithmetic read directly, is why this sits above the 70-79 "single-site inference" band the standing `75` above reflected. **Confidence stays at 75 or below for what the records represent** (a dynamic light, specifically) - this section adds a shader consumer and a producer, both consistent with "dynamic light," but does not itself trace what shader variant `0x800` does differently, nor confirm the 128-slot and 8-slot arrays are the same data at two lifecycle stages. No code changed.

### `Enable_spu_vertex_light`, slot 1: the "exact ratio on every record" claim from the previous entry does not hold generally - it is refuted on two records this session read (2026-09-18, later still)

Per this section's own "Next Steps," slot 1 (index alternates 0/1 across boots; only slot 0 had been read) is now captured - `scripts/rpcs3-spu-light-dump.py /tmp/hd-spu-light-dump-slot1`, five snapshots, landed on `index_raw: 1` on this boot. **The `A`/`A*0.25`/`A*0.1` ratio the previous entry called "exact on every record checked" holds on most of slot 1's records too** (e.g. snapshot `s0`, record 0: `A=40.0000`, fields read `10.0000`/`4.0000`, exact) **but is demonstrably not exact on at least two records, read directly off the slot's own raw bytes** (`s3_slot.bin`, not just the derived JSON):

- Record 0 at snapshot `s3`: `A=2.0`, the two derived fields read `2.0`/`0.4` - not `0.5`/`0.2` (`A*0.25`/`A*0.1`). `D=10.0`, far outside the `~0.7-2.1` range every other record in this and prior sessions' captures has shown.
- Records 2-7 at snapshot `s3` (six records, positions `-336.1`  through `-269.7` on the x axis): `A=80.0` on every one, but the derived fields read a **constant** `10.0`/`0.0` regardless - not `20.0`/`8.0`. The same six positions recur unchanged at snapshot `s4`, with the same constant `10.0`/`0.0` fields.
- By contrast, slot 1's own record 1 (position `~-395.9,-25.2,51.0`) reads `A=40.0` with an exact ratio (`10.0`/`4.0`) at `s3`, but at `s4` the same position's `A` has changed to `34.67` while the derived fields stay at `10.0`/`9.33` - neither matches `A*0.25`/`A*0.1` for the new `A`, though `10.0` matches the *previous* frame's `A*0.25` exactly.

**Read together, a plausible but unconfirmed hypothesis**: the three fields are not a pure per-frame multiply of one authored scalar, as the previous entry's "one authored-or-computed scalar with two fixed-ratio derived forms" reading assumed - they may be three independently-updating values (e.g. an SPU-side interpolation toward a target, at per-field rates that only coincide exactly at steady state), which would explain both the exact match on most records (settled state) and the mismatch specifically on records whose neighboring values look like they are mid-transition (the anomalously large `D=10.0` on the one very-low-`A` record most of all - consistent with an activation/spawn transient, not corruption: the raw bytes were re-read directly from `s3_slot.bin` and match the JSON exactly, so this is not a parsing error). **Not confirmed** - this session did not read enough consecutive frames at fine-enough spacing to test an interpolation hypothesis directly, and the records whose ratio breaks could equally be a different record *class* sharing the same 8-float layout coincidentally. **The prior "exact ratio, confidence 80" claim is narrowed**: exact-ratio is the common case, not a universal property of the structure - a session extending this needs to either capture consecutive frames close together (to watch a transition happen) or treat "A, A*0.25, A*0.1" as three separate fields to log independently rather than one value with two derived forms.

### Opcode `0x2d`'s PPU-side handler is read directly: it stores the buffer's `(address, value)` into context offsets `+0x14c`/`+0x150` - confirming the two consumer shapes are the same mechanism (2026-09-18, later still, confidence 85)

**Following this section's own "highest-value next step"**: `Render_RunCompiledOps_q` (`0x005d4a08`) is a generic bytecode interpreter over the same compiled-ops format `FUN_004074e0`/`FUN_00408fa8` build - `iVar1 = *param_1` reads an opcode int from the stream, then indirect-calls through a jump table (`PTR_PTR_008bf21c`, itself resolved via `scripts/ps3-toc.py toc 0x005d4a08` -> TOC `0x008bd3c4` + the `lwz r30,0x1e58(r2)` operand = `0x008bf21c`, matching the symbol name exactly) until the stream's terminator opcode (`1`).

**Opcode `0x2d`'s own handler is resolved by walking that table by hand** (`read_memory` at `0x008bf21c` -> array base `0x00927518`; `0x00927518 + 0x2d*4` = `0x009275cc` -> `.opd` address `0x008a0e88` -> code address `0x005d5f20`, the standard two-level PPC64 `.opd` indirection). `_opd_FUN_005d5f20`, decompiled directly:

```c
void _opd_FUN_005d5f20(int *param_1) {
    int *piVar1 = (int *)*param_1;
    int iVar2 = *piVar1;
    *param_1 = (int)(piVar1 + 2);
    int iVar3 = piVar1[1];
    param_1[0x53] = iVar2;   // context + 0x14c
    param_1[0x54] = iVar3;   // context + 0x150
}
```

Opcode `0x2d` reads its two operand words off the stream and stores them into context offsets `+0x14c` (`param_1[0x53]`) and `+0x150` (`param_1[0x54]`) - the **exact same two fields** the `RigidBody`/`Absorb`/`Leach` family (previous entry) writes directly, without going through an opcode stream at all. **This confirms, rather than merely suggests, that the two consumer shapes are one mechanism**: both ultimately place `(slot_address, buffer_value)` into context `+0x14c`/`+0x150`, one via a compiled bytecode op, the other by direct field assignment. The earlier entry's "circumstantial rather than a proven consumer" hedge for the `RigidBody`/`Absorb` sites is resolved - they and the opcode-`0x2d` sites write the identical fields, so whatever ultimately consumes `+0x14c`/`+0x150` (still unread) consumes both.

**What still isn't known: which later stage actually reads `+0x14c`/`+0x150` back out**, and whether it's the same for both consumer families. `FUN_005fc728`, the flush/submit function both `Shadow_CompileAmbientShadowTrackRedraw`/`Shadow_CompileShadowedTrackRedraw` **and** every "Zone Stage" compiler in this section (`FUN_003fc140`, `FUN_00403a30`, `FUN_003ff860`/`FUN_00400a00`'s family, `FUN_004074e0`, `FUN_00408fa8`, plus `FUN_00401ba8`) call at the end of building their own opcode stream, copies context fields `+0xd8`/`+0xdc` and `+0x154` (not `+0x14c`/`+0x150` directly) into a small, tightly-clustered API (`_opd_FUN_00465138` through `_opd_FUN_004688a0`, all in a ~0x600-byte span) and calls `_opd_FUN_005a20f8` to copy the opcode stream itself by reference. **Whether this is CPU-side GCM ring-buffer submission or an SPU job dispatch is not confirmed this session** - the shared caller set with the two `EdgeGeom`-gated shadow compilers is suggestive, not proof. If it is an SPU dispatch, opcode `0x2d`'s effect for the Zone-Stage family specifically is interpreted by an SPU-side mirror of this same bytecode format, not by `Render_RunCompiledOps_q` itself, and reading it further needs the SPU Ghidra module this project has never set up - the same barrier this section already named. If it is not an SPU dispatch, the consumer of `+0x14c`/`+0x150` is some other, unread PPU-side opcode handler in the same jump table (`0x00927518`), reachable the same way this session resolved `0x2d`.

**Confidence 85** for the opcode `0x2d` semantics themselves - read directly from a decompiled handler reached by an address chain each step of which is independently verifiable (TOC resolution, two pointer dereferences, `read_memory`), not inferred. Confidence stays lower for what `+0x14c`/`+0x150` feed into downstream, which this entry narrows but does not close. No code changed.

### Bit `0x800` gates a structurally distinct, much larger code path inside the actual RSX constant-upload function - scoped to the `RigidBody`/`Absorb` consumer family only (2026-09-18, later still, confidence 82)

**Following opcode `0x1c`'s own handler chain** (`_opd_FUN_005d5e18` -> `_opd_FUN_005d7320`, both read in the previous entry as "does not touch `+0x14c`/`+0x150` directly"): `FUN_005d7320` unconditionally calls `_opd_FUN_005d8700(context, shader_ptr, cached_shader_ptr, flags)` whenever context `+4`'s bits `0x1d0000` (shader-state-changed family) are set. `_opd_FUN_005d8700`, decompiled directly, is the real GCM/RSX constant- and vertex-data-upload function: its first block walks the resolved shader program's own parameter table and writes each declared constant's literal default into context slots `+0xf0`..`+0x12c` (16 four-byte slots), then calls `Rsx_UploadVertexConstants` - unrelated to `+0x14c`/`+0x150`, which are outside that slot range.

**Its second block is the connection.** Gated on the same `param_4 & 4` the caller passes and, critically, on **context `+4`'s bit `0x800`** - the identical bit `Shader_GetVariantHash(... | 0x800)` sets at every consumer site this section has found:

```c
if ((param_4 & 4) != 0) {
    if ((state_flags & 0x800) == 0) {
        /* short path: one call to FUN_00731108, one to FUN_005c5d60 */
    } else {
        /* long path: allocates a scratch buffer, unrolls a 4x4-float block-copy
           loop reading from the material's own vertex-format descriptor at
           negative offsets, writes the result to the scratch buffer, calls
           FUN_00730e08, then a second unrolled block-copy loop writes the
           scratch buffer's contents into a different destination (iVar9),
           before the same FUN_005c5d60 call as the short path */
    }
}
```

Bit `0x800` set is not a shader-variant label alone - it makes the **actual constant-upload function** take a structurally different path, allocating and populating a scratch buffer through an unrolled copy loop before the equivalent of the short path's final call runs. **What the copy loop restructures, and why, is not deciphered this session** - the loop's own stride and offset arithmetic (reading from a vertex-format descriptor, `piVar18`, at negative byte offsets) is read but not interpreted; a plausible but unconfirmed reading is that it builds a combined/interleaved vertex-array buffer to carry an extra attribute stream when the fixed GCM vertex-array slots are otherwise full, consistent with (but not proof of) the standing "vertex-fetch source" hypothesis for what the buffer feeds.

**Scope, not universality**: `_opd_FUN_005d8700` is reached only from `FUN_005d7320`, which is reached only from opcode `0x1c` inside `Render_RunCompiledOps_q`'s own interpreter (`get_xrefs_to` on `0x005d8700` returns exactly one call site plus its own jump-table `.opd` entry). This is the `RigidBody`/`Absorb`/`Leach` family's own consumption path - it says nothing about what the Zone-Stage family's SPU-submitted stream (if `FUN_005fc728` is indeed an SPU dispatch, still unconfirmed) does with the same bit.

**Confidence 82** - the branch condition and its consequences are read directly from a decompiled function reached by a confirmed call chain; the block-copy loop's own semantics are not interpreted, which is why this does not raise confidence on what the buffer *represents*, only on bit `0x800` having a real, distinct, GPU-upload-level effect rather than being inert past the shader-variant selection. No code changed.

### `FUN_005fc728` is confirmed as a genuine SPU job dispatch, not CPU-side GCM submission - the Zone-Stage family's own consumer path is real work handed to an SPU (2026-09-18, later still, confidence 90)

**The "unconfirmed" hedge in the previous two sections is settled.** `search_functions name_pattern=Spurs` against `EBOOT.elf` finds 34 real, named `cellSpurs*` imports (`cellSpursInitialize`, `cellSpursCreateTaskset`, `cellSpursAddWorkloadWithAttribute`, `cellSpursLFQueuePushBody`, `cellSpursReadyCountStore`, and thirty more) - the binary genuinely links and uses SPURS, the Cell SDK's own SPU work-dispatch runtime, not just the EDGE `zlib` task string set this page already knew about.

**`FUN_005fc728`'s own tail call chain reaches one of them directly**, traced one hop at a time: its last call, `_opd_FUN_004688a0(local_58, puVar4, local_60, 1)`, itself calls `_opd_FUN_00467c70` (an enqueue of some kind, not traced further) and then `_opd_FUN_004664e8(param_2, param_4)`; that function's entire body is:

```c
void _opd_FUN_004664e8(int param_1,undefined8 param_2) {
    cellSpursReadyCountStore(*(undefined4 *)(param_1 + 4),*(undefined4 *)(param_1 + 8),param_2);
}
```

`cellSpursReadyCountStore` is a real, named Cell SDK function - it increments an SPU workload's ready-work count, the standard SPURS mechanism that wakes an idle SPU to dequeue and execute newly-queued work. **This is not a CPU-side GCM ring-buffer submission function; it is a genuine SPU job dispatch**, three calls deep from every shadow-redraw and Zone-Stage compiler this section has read (`FUN_005fc728`'s own callers, listed in the previous section).

**What this settles and what it does not.** It settles that the Zone-Stage family's own compiled command stream (containing opcode `0x2d`'s `(address, value)` pair whenever the per-chunk bit is set) is handed whole to an SPU program for execution, not interpreted by `Render_RunCompiledOps_q` or any other PPU code this project can read. **It does not identify which SPU program** - `_opd_FUN_00465138`'s own builder API (called from at least eight other, unrelated call sites disc-wide per `get_xrefs_to`) is generic job-descriptor plumbing, not specific to this consumer; nothing traced this session names the actual `.spu.elf` payload or ties it specifically to `Enable_spu_vertex_light`/EDGE geometry versus some other SPU workload this engine runs. That identification, and everything past it, is squarely the SPU-disassembly barrier this section has named since the "14 read sites" entry: **static PPU reading has now been pushed as far as it goes** on this specific question - the remaining work is on the SPU side, needing tooling this project has never set up for any title.

**Confidence 90** - a real, named SDK function is the end of a directly-traced, three-hop call chain from the function in question; about as strong as static-only evidence gets. No code changed.

**Correction: this was not a new open question.** `docs/ghidra/functions/ps3-hdfury-eu/shadow-stencilvolume.md` and `docs/ghidra/captures/ps3-hdfury-eu/comments.tsv` already name `FUN_005fc728` as building "DMA-list-shaped descriptors matching this project's own already-documented SPU job pattern," citing `engine-trail.md`'s "the SPU job's setup" language, with the same seven callers this session independently re-derived. That entry left it as "generic infrastructure... through an SPU-offloaded path" without tracing a named SDK function; this entry's `cellSpursReadyCountStore` chain is what moves it from that inference to a confirmed one. Should have been cross-referenced from the first `FUN_005fc728` mention in this section rather than presented as newly found here - noted so the next reader does not read this as two independent discoveries.

### SPU disassembly is now unblocked, tooling-wise - `spu-objdump` validated against both of this project's own known SPU jobs, but this thread's own target job has no static file offset to feed it (2026-09-18, later still)

**The tooling half of the barrier is resolved.** `spu-elf` is a real, official upstream binutils target (the same cross-toolchain the Cell SDK itself ships), packaged for this machine as `ps3-spu-binutils` (installed by the maintainer this session, at `/opt/ps3dev/spu/bin/spu-objdump`) - trustworthy in a way a from-scratch or unfinished SLEIGH module is not. The one existing Ghidra SPU extension found, `aerosoul94/GhidraSPU`, was checked directly against its own source and models SPU's 128-bit registers as 64-bit with `shufb` (used in nearly every SPU scalar access) stubbed as an unmodeled `pcodeop` - the same "decompiler produces readable fiction" failure `docs/psp/allegrex-vfpu.md` already warns about, so it is not used here without first fixing that.

**Committed**: `scripts/ps3-spu-disasm.py` - maps a virtual address to `EBOOT.elf`'s own file offset (the part `objdump -b binary` cannot do, via the same `Image` class shape `scripts/ps3-toc.py` already uses) and runs `spu-objdump` on the extracted bytes. **Validated against both of this project's own known, named SPU jobs** (`docs/rendering/trail-ribbon.md`'s `Trails` at `0x00811680`/`0x4d40` bytes and `WakeTrail` at `0x00816400`/`0x1240` bytes): both decode as coherent SPU code, not garbage - a standard prologue (`stqd $80..$82,$126,$0,$1` register spills, `ai $1,$1,-96` stack allocation, `rdch $ch8` a DMA-completion channel read) starting at byte `0x30` of each blob, preceded by a `0x30`-byte header of four `ila`-shaped words then four size/offset-shaped words neither job's own code branches into. **The two jobs' instruction bytes are identical through at least their first `0x80` bytes**, differing only in embedded literal constants - independent, byte-level corroboration of `engine-trail.md`'s own "`WakeTrail` is structurally the engine trail's twin" finding, arrived at from the opposite direction (disassembly agreement, not decompiled-function-shape agreement).

**This thread's own target - whatever SPU program `FUN_005fc728` hands the compiled-ops stream to - is not at a static file offset the way `Trails`/`WakeTrail` are, and `PTR_DAT_008bf6cc` is not it.** A live read (`scripts/rpcs3-spu-job-binary-dump.py`, committed) confirms the pointer is never relocated (`0x00f73c00` live, byte-identical to the static value) and dumped 32 KiB from it - but the dump contains **zero** `stqd`/`rdch`/`brsl`/`hbrr` instruction-shaped bytes anywhere, nothing resembling the prologue both `Trails` and `WakeTrail` share. **`PTR_DAT_008bf6cc` is not the job's executable binary** - re-reading `FUN_005fc728`'s own call to `_opd_FUN_004652d0(puVar2,1,0,PTR_DAT_008bf6cc,0x40,2)` alongside the *other* setter calls in the same function (`_opd_FUN_004651c0(puVar2,0,1,0x10,iVar5)` etc., where `iVar5 = _opd_FUN_00469320(puVar4+0x48)` - decompiled directly, computing `(a + 0x3ff + b) >> 10 & 0x3fffff`, a size-in-KiB rounding formula, not an address) shows `PTR_DAT_008bf6cc` is one *field* among several the job descriptor carries, not its code. The 32 KiB dump instead looks like a live, sequentially-incrementing **queue of already-submitted job records** - repeating ~0x24-byte entries carrying a PPU-main-RAM-shaped address (`0x3180fXXX`) next to a small header and a counter that increments by exactly one per entry (`6, 7, 8, 9, 10` read consecutively) - consistent with `PTR_DAT_008bf6cc` sitting inside or adjacent to the shared job-descriptor ring, not naming the program.

**A more direct route than finding the binary: `FUN_005fc728` copies the compiled-ops stream itself into each job's own payload, confirmed by re-reading its own body.** When its `param_4` argument is nonzero (every draw-dispatcher call site passes `1`) and the calling context's `+0xd8`/`+0xdc` fields are populated (the compiled-ops buffer pointer and its entry count - the same fields `Render_RunCompiledOps_q`'s own family reads), it copies `count * 0x20` bytes from that buffer into a fresh slot in the shared ring (`_opd_FUN_005a20f8(puVar4 + iVar1 + 0x12100, iVar5, iVar3)`) and stores a pointer to the copy at the new job slot's own `+0xd8` (`puVar6[0x36] = ...`) - the same field offset as the context struct's own compiled-ops pointer. **This confirms, independent of finding the executable binary at all, that opcode `0x2d`'s `(address, value)` pair is physically present in the SPU job's own input data whenever the per-chunk gate is open** - not merely "handed to *a* job," but copied byte-for-byte into that job's own record. **Next step, and now the more promising one**: locate the *current* job slot at read time (`struct_ptr + *(struct_ptr+0x12080) - 0x160 + 0x12100`, where `struct_ptr` is `PTR_DAT_008bf6b8`'s own live value, `0x00f88d80` this session - both values already captured), read its own `+0xd8`/`+0xdc` fields, follow the copied-opcode-stream pointer, and read the actual bytes - this reads the *data* the SPU consumes without needing to disassemble the SPU *program* at all.

### The job's own executable binary is located, statically, and confirmed real code - a generic SPU kernel shared by nine subsystems, with a DMA-fetch dispatch matching the PPU builder's own field layout (2026-09-18, later still)

**Located, not just narrowed.** `FUN_005fc6c8` - decompiled directly, three lines - is the render-ops job queue's own one-time registration call: it writes to `PTR_DAT_008bf6b8 + 0x12084` (the same struct base `FUN_005fc728` derefences every call) and then calls `_opd_FUN_00466548(puVar1, param_1, param_2, param_3, 0x10, param_4)`. `FUN_00466548`, also decompiled directly, is a generic "register a SPURS workload" wrapper - it calls `cellSpursWorkloadAttributeInitialize` with the workload's program-module effective address and size supplied by two zero-argument accessors, `_opd_FUN_0046d880()` (`return PTR_DAT_008bb58c;`) and `_opd_FUN_0046d888()` (`return iRam008bb590 + 0x7f & 0xffffff80;` - a round-up-to-128 size), then `cellSpursAddWorkloadWithAttribute`. Read directly: `PTR_DAT_008bb58c = 0x00848780`, `iRam008bb590 = 0x2cf8` (rounds to `0x2d00`). `0x00848780` is inside `EBOOT.elf`'s own first program-header segment (`vaddr 0x10000`, `filesz 0x848e48`) - a real, static file offset, extracted and disassembled with `scripts/ps3-spu-disasm.py 0x00848780 0x2d00`.

**Confidence 88 that this is the correct binary** - the call chain from `FUN_005fc728`'s own struct base to this specific registration, through `FUN_005fc6c8`, is direct and unambiguous decompiled PPU evidence, capped just under the "decompilation only" ceiling because no live trace confirms the workload actually receives this project's render-ops queue's entries specifically (as opposed to registering correctly but never being fed).

**It is not exclusive to this thread's own job queue.** `get_xrefs_to 0x00466548` finds **nine** distinct callers across this codebase, all sharing the same `PTR_DAT_008bb58c`/`iRam008bb590` globals - meaning every one of those nine subsystems spins up its own `cellSpurs` workload instance running the identical SPU binary, differentiated only by name/priority/user-data, not by program. **One generic SPU kernel handles many logical job types**, which explains why `Trails`/`WakeTrail` (each its own separate, differently-sized blob, registered through the unrelated `FUN_002e2a48` path used for `cellSpursCreateTask`, not `cellSpursAddWorkloadWithAttribute`) look nothing like this one - they are SPURS *tasks* running inside a taskset, a different Cell SDK mechanism from the *workload* this section's binary is.

**The disassembly is real, structured code, hand-read (no decompiler - `spu-elf` gives instructions, not C) so scored in the "probable" band, not higher.** A stack-setup preamble (`lqa $1,0x1540` then arithmetic building a frame) leads into DMA/runtime-library calls (`brasl` to fixed addresses near the top of the 256 KiB local-store range, `0x3fec0`/`0x3fff0`-shaped targets - consistent with a linked-in newlib/DMA-helper region, the same shape a compiled C SPU program would have). **A job-type dispatch is visible**: three sequential `ceqi $2,$81,1` / `ceqi $2,$81,2` / `ceqi $81,$81,3` compares (`0x2af0`-`0x2b18`) branching to different `ila`-loaded targets, each followed by `brnz ...,0x1bd0` - a small-integer job-type switch, plausible given nine subsystems share one binary. **Immediately after it, a four-slot DMA-fetch loop** (`0x2b40`-onward: `wrch $ch16`/`$ch17`/`$ch18`/`$ch19`/`$ch20`/`$ch21` then `rdch $ch27` - the standard MFC channel sequence for `LSA`/`EAH`/`EAL`/`Size`/`TagID`/`Cmd`, i.e. a DMA get, followed by a tag-status wait) reads fields by index (`0`/`c0`/`80`/`100`-shaped local-store offsets), **the same field-index scheme (`0`-`3`) `FUN_005fc728`'s own descriptor builder writes via `_opd_FUN_004651c0`/`_opd_FUN_004652d0`** on the PPU side. This is a real structural correspondence - the SPU program DMA-fetches exactly the number of optional fields the PPU builder populates - not proof of what happens to any one field's *contents* once fetched, which is not traced past this point.

**Confidence 65 for the field-count correspondence specifically** ("plausible" band - a reasonable reading of raw disassembly, not runtime-verified, not decompiled); the binary-location claim above is scored separately and higher. **What remains, if anyone picks this up**: trace what the SPU program does with field `2`'s DMA'd-in data specifically (the offset that, on the PPU side, the compiled-ops stream's copied bytes are reachable from) - this needs sustained hand-disassembly of a genuinely general-purpose ~11.5 KiB SPU kernel, comparable in scope to `engine-trail.md`'s own `Trails`-job tracing effort, not attempted further this session. No code changed; the extracted binary itself is not committed (game content, per `docs/overview/legal.md`) - only the extraction recipe (`scripts/ps3-spu-disasm.py 0x00848780 0x2d00`) is.

### A disc-wide name-hash sweep for `pointLight0*` finds no consumer anywhere (2026-09-18)

**Confidence 85.** The per-material sweep two sessions above checked four
Amphiseum ceiling materials, at one resolved variant each, for
`constantAmbientColour`. This generalises the same question disc-wide and
to every compiled variant: does *any* `SHO` block anywhere on
`hdfury-ps3-eu-dec.iso` declare `pointLight0PositionWorldSpace`
(`0x69a6ba16`), `pointLight0Colour` (`0xead721a1`) or `pointLight0Falloff`
(`0x407290f8`) in its own parameter table - fragment-patched or
vertex-register-bound, whichever applies? Tool:
[`scripts/hd-pointlight-sweep.py`](../../../../scripts/hd-pointlight-sweep.py),
committed. Scanned: 1,632 `.rcsmaterial` entries (1,590 unique paths) across
the seven `DATA0N.PSARC` archives, plus every `SHO` block resident directly
in `EBOOT.elf` - 97,735 blocks from materials, 126 from the executable,
**97,861 total, all parsed, zero failures**. **Zero hits, either kind,
anywhere.**

**A first attempt at this got the wrong answer by reading the wrong thing,
and the failure is worth recording as its own trap.** The parameter
table's `vreg` field, when register-bound, gives a real hardware constant
register (`vreg`, not `vreg - 256` or any other transform) - but an
earlier version of this sweep instead tried to *infer* register use from
raw instruction bit patterns, comparing each vertex instruction's decoded
`CONST` field against `row + 256` for the three candidate table rows
(19/20/21), on the assumption that an engine-parameter-table row always
lands on the same numbered register in every material. **It does not.**
Each material's own parameter table assigns its own registers, and
nothing stops one colliding numerically with what a different material -
or the shared per-frame table - uses for something else. Caught two ways:
first, hand-disassembling one raw hit
(`amphiseum/fe/materials/cf_fetracks.rcsmaterial` block 121,
`scripts/ps3-microcode.py vp-file`) showed `c[19]` there is the fourth row
of that block's *own* locally bound `c272` 4x4 matrix (272-275), not
`pointLight0PositionWorldSpace` - a coincidence of allocation. Second, the
same register-guessing method was pointed at `positionScale`/
`positionBias` (rows 210/211), which this page's own earlier reading
already established real vertex code reads as `c[210]`/`c[211]` - and it
returned **zero** hits disc-wide, on a parameter proven positive. Two
independent negatives on a known-positive parameter is the method failing,
not the data. The fix was to stop inferring registers from code and read
the same parameter table `fp_patch_map` already trusts for the fragment
side - `vreg != 0xffff` is a real, named, register-bound vertex constant,
directly, no arithmetic. **The corrected method needs both its branches
proven live, since `pointLight0*`'s zero spans both.** `constantAmbientColour`
re-run on the fix exercises the patched branch: 19,464 hits across 1,515
materials, spanning ship, weapon, HUD and track materials, the shape
expected of a genuinely widely-used lighting constant. The register-bound
branch needed its own control, since a patched-only positive cannot prove
it - `positionScale`'s own hash (`0x9cc5ab3a`, read directly off
`cf_fetracks.rcsmaterial` block 121's parameter table, the same block the
false positive came from) finds 60,267 register-bound hits across 1,583
materials, including that same block, confirming the branch that would
have to fire for a real `pointLight0PositionWorldSpace` consumer to show
up is live and working. Both controls positive is what validates trusting
the zero on `pointLight0*`. **This is the shape this project already names
in `vex-classes.md`'s "one-field rotation" and `memory.md`'s TOC defect:
arithmetic that is internally consistent and still reads the wrong thing.**
Kept in the tool's own docstring so the next person sweeping a parameter by
register number reads this first.

**Scope note against the earlier hand check.** That check asked a
narrower question - does the *one specific compiled variant* a named
chunk's own resolved permutation selects reference the parameter - and
found the four ceiling materials' *selected* variants do not, while this
sweep (correctly) finds `constantAmbientColour` declared in *some* variant
of those same material files: `.rcsmaterial` ships many pre-compiled
permutations, and a material having an ambient-enabled variant somewhere
in its file does not mean the specific chunk drawn selects it. The two
findings do not conflict; they answer different-scoped questions, and
`pointLight0*`'s zero is the broader of the two - not one drawn variant,
every compiled permutation shipped.

**What this adds to the standing finding.** The per-circuit `.vex` node
census in the section above ("A third candidate is checked and closed
narrowly") found zero `PointLight` nodes on Talon's Junction and Amphiseum
specifically, and said so narrowly - it does not speak to any other
circuit. This sweep is disc-wide but on the *other* end of the pipe: not
whether a circuit authors a `PointLight` node, but whether any shipped
material's compiled shader program would do anything with one if it did.
Corroborating context already on this page and in `docs/formats/lighting.md`:

- HD's own `g_VexClassTable` (confidence 95, `vex-classes.md`) confirms
  class `0x132` genuinely is `PointLight` in HD's own executable, at the
  same id Pulse uses - so `oag_vex`'s disc-wide census of 1,160 `PointLight`
  nodes across HD's 742 `.vex` files (`crates/vex/src/lighting.rs`'s module
  docs) is real, correctly-classed data, not a renumbering artefact the way
  a borrowed Pulse id elsewhere on this page turned out to be.
- HD ships **no `PointLight_Importer.cpp`** - the by-name importer
  comparison above lists `PointLight` and `Dynamic Point Light` as
  **Pulse only**, alongside `Dynamic Shadow Occluder` and eleven others HD's
  41 importer translation units do not cover.
- Pulse's own `PointLight` was already a converged negative on the *other*
  side of this same pipe: never passed to `Vex_RegisterClass` on either PSP
  binary (`docs/formats/lighting.md`'s Open section).

So both engines now read the same way on this feature, checked by two
different methods on two different binaries: Pulse never registers the
authoring class; HD registers the class (nodes exist, correctly classed)
but no shipped shader - in any compiled variant, of any material, on this
disc - ever declares the engine parameters a point light would need to
affect a pixel. **HD's `pointLight0*` triple reads as a declared,
wired-into-the-parameter-table, never-consumed feature** - the same shape
this project's own `DirectionalLight`-consumer thread on Pulse converged
on, not a gap to fill on the render side. Per
["never invent what the assets already author"](../../../../CLAUDE.md),
this is a reason not to add a point-light term to `mesh.wgsl`, not a lead
to chase further, absent a runtime trace that overturns it.

**What this does not establish.** Only this one disc image was swept
(`hdfury-ps3-eu-dec.iso`); Fury/DLC `.rcsmaterial` files ship in the same
archives and were included, but no second HD SKU or binary corroborates
this, which is what a score above the mid-80s would need. Nothing here ran
live - a runtime watchpoint on the parameter-table slots themselves, the
way Pulse's `DirectionalLight` thread eventually did, was not attempted.
`g_VexClassTable`'s own registrar walk is still unwatched (`vex-classes.md`'s
Open section), so "the class is real" rests on the table read, not on a
traced `PointLight_RegisterClass`-equivalent - though HD has no such
function to trace, per the importer comparison above. And a declared,
register-bound or patched parameter is not thereby proven to be *read* by
live code past that point - `fp_patch_map`'s patched case is checked
against `fp_patch_slots` reaching a real code slot (the same rigor the
hand investigation applied), but a register-bound vertex entry is only
checked for existing in the table, not for the code actually consuming
that register on every path - the same caveat this project's `+0xcc`/
`+0xd0` per-model tint pair note elsewhere already carries for a "declared,
not traced" reading.

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
adaptation this page already documents, and which `oag_post::hd_bloom`
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
has to `f[TC1]`, and `crates/mesh/src/mesh/rcs.rs` writes `[1, 1, 1, 1]` for
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

**Re-measured 2026-09-10, the handover thread's three remaining Next Steps,
all closed in one disc walk**
(`crates/render/examples/hd_specular_orthogonality_sweep.rs` - 1,632
`.rcsmaterial` files, 76,358 variants parsed, 0 failed to parse).

1. **The two `NoSingleWriter` operands are not "no writer" - they are two
   writers to the same register, and the tool's single-instruction rule
   can't see past that.** Both belong to `nitro_perspex_new.rcsmaterial`
   (`260`, ship, non-Zone, two variants sharing identical code shape). Their
   winning `DP3` (`DP3_SAT R1.w, R2.xyzw, R3.xyzw`) reads `R2.xyz`; a
   per-lane writer search (new - walks every prior instruction touching any
   part of the register, rather than requiring one that covers all three
   lanes in a single write) finds `op3B R2.xyz, R2.xyzw, R0.zzzz` fully
   covering `x`/`y`/`z`, five instructions later followed by
   `MOV R2.z, R2.xyzw` - a write to `.z` alone that trips the clobber check.
   Read literally the source operand of that `MOV` is `R2` itself with an
   identity swizzle, so the value it writes to `.z` is whatever `.z` already
   held - **a no-op self-copy, not a real overwrite**, on the reading this
   page's own swizzle convention gives every other instruction. If that
   reading holds, `op3B` is still the value's true and only origin and the
   operand is mechanically indistinguishable from every other `Normalize`
   case (`op3B` reading the un-normalized vector against a scalar in a
   different register) - the same shape as the sibling `260` variants
   already classify. **Not applied as a fix**: recognising a self-copy as a
   no-op is a semantic judgement (does the compiler ever emit a genuinely
   redundant `MOV`, or does this hardware's `MOV` do something a bare
   identity read doesn't capture, e.g. a precision or format conversion
   between `R2` read and `R2` write) this pass didn't verify, and
   `classify()`/`last_writer` are deliberately structural, per their own doc
   comments. Reported as what was found, not silently patched into the
   tool. Rate: 2 of 344 operands (0.6%) in the `200`/`250`/`260`/`35`
   population - a small, now-explained gap, not a sign the method is
   unsound elsewhere.
2. **Orthogonality is settled, disc-wide, with a direct sweep rather than one
   file's ambiguous variants.** Plain reading first: `declares_zone=false`
   resolves specular exponents at `0` (1,965), `32` (1,444), `260` (84),
   `200` (52), `40` (32), `35` (24), `300` (14), `250` (8) - `200`/`260`/`35`
   are **entirely** non-Zone, confirming the 2026-09-05 entry's read of those
   three buckets. The sharper, file-scoped question
   `01_normal_diffuse_specularonalpha.rcsmaterial` left open - does *that
   file's own* non-Zone side ever resolve - is answered by restricting to
   files whose own variants include both a Zone-declaring and a non-Zone one
   (1,467 of 1,632 files, i.e. most files on the disc toggle): **287 of
   those 1,467 have a non-Zone variant that resolves**, spanning `0`, `32`,
   `40`, `300` and every circuit sampled by hand before (amphiseum,
   modesto_heights, talons_junction, tech_de_ra and more), including
   ordinary `track_surface`/`constantdiffuse_specular_normal`-family
   materials. `01_normal_diffuse_specularonalpha.rcsmaterial` itself is
   *not* one of the 287 - its own non-Zone variants still never resolve,
   consistent with the 2026-09-05 entry, but it is now confirmed to be the
   exception rather than the rule: the Zone-declaring flag and
   `specular_exponent()` resolving are orthogonal disc-wide, even though
   they happen to be coupled in that one file. Confidence on the
   orthogonality claim itself raised to disc-wide-measured (287 independent
   counter-examples); this does not touch the separate rim-control finding
   the 85 rests on, addressed next.
3. **The four-parameter cluster is the dominant declared set almost
   everywhere, and the by-hand read's 13 materials undersold one genuine
   exception.** A census of every Zone-declaring variant's exact declared
   subset of the 16 known `zone*` names, grouped by circuit, covers all 16
   circuits `hd_specular_patch_census.rs` already walks plus the four
   stand-alone Zone-mode arenas (`zone_1`-`zone_4`, absent from that list -
   these are not raced circuits) and the front-end/HUD/rank-medal
   `lambert*` family, 42,028 Zone-declaring variants total. In every
   ordinary circuit and in `fe`/`hud`, the top two sets are the
   already-published four-parameter cluster
   (`zoneColourTint`/`zoneEffectInner`/`zoneBaseInner`/`zoneBaseAltInner`)
   and its seven-parameter superset (adding the three `*Outer` siblings,
   `lambertshine`'s own shape from the 2026-09-05 sample) - together 34,712
   of 42,028 (82.6%), and a further 6,876 (16.4%) split between a
   `zoneBaseInner`/`zoneBaseAltInner`-only pair and its five-parameter
   `*Outer` superset (missing `zoneColourTint`/`zoneEffectInner`). **The four
   Zone-mode arenas break the pattern**: their top two sets are the
   `zoneBaseInner`/`zoneBaseAltInner` pair and its five-parameter superset,
   not the four-/seven-parameter cluster, plus a `zoneAnisoPower`-bearing
   family (also `zoneColourTint`/`zoneEffectInner`) present in no ordinary
   circuit's top sets at all - a real circuit-*type* difference, not a
   sampling artefact, and the "holds disc-wide" reading needs that
   qualifier: it holds for every raced circuit and the front end, not for
   the four dedicated Zone-mode arenas. `zoneAnisoPower` is not one of the
   85-confidence finding's four-parameter cluster and was not read by hand
   this pass - named here as what the arenas' distinct top set actually is,
   not interpreted further.

**What these three do and do not move.** All three are structural counts -
which instruction wrote a lane, which parameter set a block declares, which
variants resolve - exactly as the 2026-09-05 entry's own operand-shape work
was, and the same caution applies: none of this is evidence about whether
`200`/`250`/`260`/`35` are real shininess values, only about the two
questions the thread's Next Steps actually asked (the `NoSingleWriter` gap,
and orthogonality). **Confidence held at 85, not raised**: items 2 and 3
widen the sample the orthogonality and cluster-dominance claims rest on (13
materials to a full census) but do not touch the mechanism axis the 85
already priced in - the per-variant Zone toggle is still inferred from
`01_normal_diffuse_specularonalpha.rcsmaterial`'s own variant pattern, not
confirmed against the game's own render-path/variant-selection code, which
still needs Ghidra and is explicitly not attempted here.

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

**`op3B` resolves 2026-09-25, and `op3D` along with it, off RPCS3's own
opcode table rather than a stronger guess at the same confidence this page
already declined to cross.** `rpcs3/Emu/RSX/Program/Assembler/FPOpcodes.h`
(GPLv2, independently reverse-engineered against real hardware and shipping
PS3 games - not Mesa's NV30/NV40-era `nvfx_shader.h` this project's own
decoder is otherwise built from, which has no entry for either opcode) names
both: `RSX_FP_OPCODE_DIVSQ = 0x3B` ("Divide by Square Root", `a / sqrt(b)`)
and `RSX_FP_OPCODE_FENCT = 0x3D` ("Fence T?" - RPCS3's own hedge on the exact
meaning, not this project's).

**`DIVSQ` is what the "second usage shape" paragraph above was missing, and
it resolves the contradiction rather than adding a third guess.** A generic
two-operand `a / sqrt(b)` produces *both* recorded shapes from one formula:
`DP3(v, v) -> d; op3B(v, d)` is `v * rsqrt(d)` (normalize, this section's
original reading) and `op3B(x, x)` - both operands the same register, no
preceding self-`DP3`, exactly the ship's `pow(N.H, 40)` block's own shape
this page already flagged as not fitting a dedicated `NRM` - is `x / sqrt(x)
= sqrt(x)`, the standard single-instruction square-root idiom on hardware
with no native `SQRT`. A dedicated `NRM` opcode cannot produce the second
shape at all; `DIVSQ` produces both without special-casing either.

**Checked disc-wide before naming, not asserted from two hand-read
examples**, with a new census
(`crates/render/examples/hd_op3b_op3d_census.rs`) walking all seven archives'
`.rcsmaterial` files - 1,632 files, 76,358 fragment blocks, the corpus size
`rcsmaterial_ground_truth.rs` already establishes:

- **`op3B`: 183,623 uses** - 105,800 in the different-register (normalize)
  shape, 38,284 in the same-register (square-root) shape, and 39,539 taking
  an `Input` or `Constant` operand rather than two plain registers (expected
  argument variety for a generic arithmetic primitive, not a counter-example
  to either shape). **Confidence 84**: matches the primary source unhedged,
  structurally uniform disc-wide across both shapes that previously seemed
  irreconcilable. Capped below the 85+ band because nothing here traces an
  actual computed value against a known-correct oracle - no live GPU trace,
  the way "Established" would need.
- **`op3D`: 59,256 uses, every single one writing destination register 63** -
  the 6-bit destination field's all-ones value. Independently checked by hand
  on a sample of these: `nvfx_shader.h`'s own `NV40_FP_OP_OUT_NONE` bit (bit
  30 of the instruction's first dword) is set on every one, matching Mesa's
  documented meaning for that bit exactly rather than merely correlating with
  it. Zero counterexamples across the full disc-wide sweep - not a sampled
  rate, an exact invariant - corroborating this section's own earlier
  `etched_glass_tech`-only observation of the identical `R63, R0, R0` pattern
  at far larger scale. Whatever `0x3d`'s precise hardware semantics, it
  writes no real destination anywhere on this disc, consistent with "fence"
  and inconsistent with a real arithmetic contributor to any shading result.
  **Confidence 90** on "no data effect" specifically - the rubric's own
  top-structural-band ceiling for an exact arithmetic/structural invariant
  across many real files, held below "Established" because RPCS3's own name
  is hedged and no live hardware trace confirms an actual synchronisation
  effect.

**Now named in `fragment.rs` and `scripts/ps3-microcode.py`'s `FP_OPS`** -
`op3B` as `DIVSQ`, `op3D` as `FENCT`. `Program::dp3_feeding` is deliberately
**not** relaxed to accept `DIVSQ` in this pass: that was this section's own
prior caution for a different, unnamed reason (semantics not uniform), and
now that the semantics *are* understood and uniform, relaxing the gate to
recognise a `DP3`-then-`DIVSQ` normalize idiom as equivalent to a bare `DP3`
is a real, separate change to `specular_exponent()`'s own behaviour - left
for whoever picks it up next, not folded in here alongside a naming change.
`docs/rendering/pads.md`'s pad-glow specular scalar was blocked on exactly
this naming; traced by hand, both pad programs' chains use only `0x3b`,
never `0x3d` - see that page for what this clears and does not.

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
`oag_mesh::mesh::GpuVertex::sun_mask` field carries it into the shader
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

A consequence worth recording for the reimplementation: `oag_post::hd_bloom`'s
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

**2026-09-15: the three setters' "no caller anywhere" is re-run against the
`lvlx`-aware PS3 language (`toolchain.md#ps3`) and holds.** The static callers
search that this section's table rests on predates the 2026-09-15 reimport
that fixed `lvlx` decoding for 294 functions; re-run in full against the
current database (`PowerPC:BE:64:A2ALT-32addr-PS3`, 26,112 functions) by all
three routes for each of `Post_SetColourScaleRate` (`0x003def38`),
`Post_SetColourScaleTarget` (`0x003def50`) and `Post_SetColourScaleImmediate`
(`0x003def68`):

- `get_function_callers` on each address: `No callers found`, all three.
- `search_instructions(mnemonic="bl", operand_pattern=<address>)` and the same
  with `mnemonic="b"` (to catch a cross-TOC trampoline of the
  `Renderer_SetColourCorrection`/`FUN_00678ee8` shape), scanning all
  1,829,837 instructions in the program: zero matches for each address, both
  mnemonics, all three functions.
- Each function's own address searched as a big-endian 4-byte literal
  (`search_byte_patterns`, `00 3d ef 38`, `00 3d ef 50`, `00 3d ef 68`) -
  a positive control first, since the tool is documented to ignore `mask`
  and its behaviour on this program hadn't otherwise been exercised: each
  returns exactly one hit, its own `.opd` descriptor (`0x0088c430`,
  `0x0088c438`, `0x0088c440` respectively, matching `get_xrefs_to` on the
  function address), and no second pointer anywhere else in the image - the
  same "occurs exactly once" shape this page's `g_FullscreenTintColour`
  enumeration above already established for a different global.
- The descriptor address itself searched the same way (`00 88 c4 30`,
  `00 88 c4 38`, `00 88 c4 40`) - no matches, so no function-pointer table
  holds any of the three either. `get_xrefs_to` on the descriptor addresses
  themselves also returns none.

None of the three functions contains an `lvlx` instruction
(`search_instructions(mnemonic="lvlx", operand_pattern="v", function=...)`
returns zero for each), so they were not among the 294 functions the
reimport changed - the negative was never at risk of being an `lvlx` hole,
and this pass is a clean re-confirmation rather than a correction. The
"no caller anywhere" finding for all three setters stands.

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

### `Enable_spu_vertex_light`: the "four-slot DMA-fetch loop" is retracted - it does not exist - and the field-presence dispatch actually gates an atomic reference-count decrement against main memory (2026-09-18, later still, confidence 75)

**Picking up where the previous session's own "highest-value next step" left off**: trace what the SPU program (`EBOOT.elf` VA `0x00848780`, size `0x2d00`, re-extracted with `scripts/ps3-spu-disasm.py 0x00848780 0x2d00 --out /tmp/render-ops-job.bin`) does with the job-descriptor field data its own DMA-fetch dispatch reads. **The premise is wrong and is retracted here**: there is no four-slot DMA-fetch loop at SPU offset `0x2b40`. Re-reading the disassembly at that address (`0x2b40`-`0x2b6c`, file VA `0x84b2c0`-`0x84b2ec`) shows a small helper that stages three registers into local store, calls a subroutine at local offset `0xc00` (VA `0x849380`, a plain non-atomic MFC-channel helper reached from many places in this job, unrelated to the field dispatch), and returns - not a DMA-fetch loop, and not reached from the field-presence check at all. The actual mechanism, read this session, is different and better-supported than what it replaces.

**What `ceqi $2,$81,1`/`2`/`3` (SPU local offset `0x2af0`-`0x2b18`, VA `0x84b270`-`0x84b298`) actually gates, read in full.** This is not itself a DMA fetch; it is a **job-descriptor field-presence check** on a quadword register, `$80`, treated as four 4-byte sub-fields at byte offsets `0`, `4`, `8`, `12` (extracted via `rotqbyi $80,4`/`8`/`12`), tested for nonzero one at a time:

- Field 0 (`$80` itself, untouched): tested at `0x2aec` (VA `0x84b26c`).
- Field 1 (`rotqbyi $3,$80,4`): tested at `0x2b00` (VA `0x84b280`).
- Field 2 (`rotqbyi $3,$80,8`): tested at `0x2b14` (VA `0x84b294`).
- Field 3 (`rotqbyi $3,$80,12`): tested at `0x2b28` (VA `0x84b2a8`) - but only when job type `$81 != 3` (`ceqi $81,$81,3` then `andc $3,$3,$81` masks the field-3 test to zero when the job type is exactly `3`, a genuine per-job-type bypass, not a bug in this reading).

**Whichever field is nonzero first branches to `0x1bd0`** (VA `0x84a350`) **with that field's own raw value in `$3`.** All four fields land in the same shared subroutine; none is treated specially past this point except by its numeric value. If no field is set, execution falls through to `0x2b2c` (VA `0x84b2ac`) and returns immediately via a local-store-cached inert `(address, value)` pair - the same "opcode `0x2d`, operands `(0, 0)`" inert case the previous session already read on the PPU side.

**`0x1bd0` is a genuine, textbook SPU atomic decrement-and-test, not a data fetch.** Read directly, register by register:

```
0x1bd0  ila   $2, 0x80            ; LSA = local-store 0x80 (fixed staging buffer)
0x1bd8  andi  $5, $3, -128        ; EAL = field value, masked to its 128-byte reservation line
0x1be0  andi  $0, $3, 127         ; byte offset of the field's own address within that line
0x1be8  il    $3, 0xd0            ; MFC_GETLLAR  (Get Lock Line and Reserve)
0x1bec  wrch  $ch16, $2
0x1bf4  wrch  $ch18, $5
0x1bfc  wrch  $ch21, $3
0x1c00  rdch  $3, $ch27           ; wait for the reservation load
0x1c04  lqx   $7, $0, $2          ; $7 = staged quadword at LS 0x80 + byte offset
0x1c08  rotqby $8, $7, $4         ; $8 = the staged counter byte/halfword, extracted
0x1c10  ahi   $10, $8, -1         ; $10 = $8 - 1   (decrement)
0x1c1c  shufb $7, $10, $7, $9     ; insert the decremented value into the fetched line
0x1c20  ceqhi $8, $8, 1           ; mask = (original $8 == 1) ? all-ones : 0
0x1c24  wrch  $ch16, $2
0x1c2c  wrch  $ch18, $5
0x1c34  stqx  $7, $0, $2          ; write the modified line back to the staging buffer
0x1c38  il    $6, 0xb4            ; MFC_PUTLLC  (Put Lock Line Conditional)
0x1c38  wrch  $ch21, $6
0x1c3c  rdch  $6, $ch27           ; commit status
0x1c44  brnz  $6, 0x1be8          ; retry the whole GETLLAR/PUTLLC pair if the commit lost the race
0x1c48  brhz  $8, 0x1c54          ; if the original value was NOT 1, return now
0x1c4c  stqa  $6, 0x1530
0x1c50  brsl  $0, 0x2c70          ; else: call a second, distinct atomic-update helper
0x1c54  lqa   $0, 0x14e0          ; restore the caller's own return address
0x1c5c  bi    $0                  ; return
```

`0xd0`/`0xb4` are the real, fixed Cell MFC command encodings for `MFC_GETLLAR`/`MFC_PUTLLC` - the standard SPU lock-free compare-and-swap idiom (reserve a 128-byte line, modify a local copy, commit conditionally, retry on conflict), not something inferred from context. **Reading `$8`'s treatment together - decremented (`ahi -1`), written back, and separately tested for `== 1` before the decrement - is a reference-count decrement-and-test-for-zero**: whichever field is nonzero is treated as the effective address of a shared counter in main memory; this code atomically decrements it, and when the *pre-decrement* value was `1` (i.e., this call is the one that brings it to zero), it additionally calls a **second**, independently-implemented GETLLAR/PUTLLC helper at `0x2c70` (VA `0x84b3f0`) before returning - a "last one out" trigger, not merely a flag set. `$0` (the SPU link register) is saved to local store `0x14e0` before the retry loop starts and restored immediately before the final `bi $0`, which is how the field-presence dispatcher's own `ila $0, <continuation>` (see below) is preserved across the whole GETLLAR/PUTLLC sequence and any nested call to `0x2c70`.

**The second helper, `0x2c70`/its twin `0x2be0` (VA `0x84b3f0`/`0x84b360`), is the same GETLLAR/PUTLLC idiom again, against a *different* main-memory location** (`EAL` sourced from local store `0x1c0`, `LSA` fixed at `0x100` rather than `0x80`) - confirmed by the identical `0xd0` then `0xb4` command pair at its own two call sites (`0x2c08`/`0x2c48` VA `0x84b388`/`0x84b3c8` for one instance, `0x2c98`/`0x2cd0` VA `0x84b418`/`0x84b450` for the other). **This retires the prior session's "standard MFC channel sequence for LSA/EAH/EAL/Size/TagID/Cmd, i.e. a DMA get" reading of this exact code** (`Cmd` at these sites is never a plain `MFC_GET`/`MFC_PUT`; it is always `0xd0`/`0xb4`) - what looked like a generic six-channel DMA get is a second, textually similar atomic-RMW helper, reused from at least two other call sites in this job (`0x2664`/`0x2694`, VA `0x84aee4`/`0x84af14`) that this session did not trace further.

**Confidence 75.** The MFC command encodings (`0xd0`=`MFC_GETLLAR`, `0xb4`=`MFC_PUTLLC`) are fixed, documented Cell SDK constants, not inferred; the retry-on-conflict control flow and the decrement-and-test-for-`1` register dataflow are read directly off the disassembly, register by register, with no gaps. It does not reach 84 because this is still hand-disassembly with no decompiler and no live trace confirming the reading against a captured value - the same ceiling every entry in this section is held to. What is *not* established: what populates the local-store staging buffer at `0x80` before this code runs (the value being decremented, ultimately), and what the second helper's own target address represents. Both are open, not guessed at here.

### The four per-field continuation addresses point at a second, unrelated SPU program - not at this job's own code (2026-09-18, later still, confidence 80)

**Before the GETLLAR/PUTLLC finding above was trusted, the return-address setup around it needed checking, because it looked, at first read, like a second consumer worth naming.** Each of the four field tests above is preceded by `ila $0, <addr>` - `0x34f0` (field 0, set at `0x2ae0`/VA `0x84b260`), `0x3504` (field 1, `0x2afc`/VA `0x84b27c`), `0x3518` (field 2, `0x2b10`/VA `0x84b290`), `0x352c` (field 3, `0x2b24`/VA `0x84b2a4`) - and `$0` is the SPU link register, saved/restored around the GETLLAR/PUTLLC loop exactly as described above, then `bi $0`'d to on return. Read naively, this says "field 2 present → after the atomic decrement, continue execution at SPU local-store offset `0x3518`" - which would finally answer what the SPU does with field 2's own data.

**It is wrong, and checking it is what this entry records.** `0x3518` (and `0x34f0`/`0x3504`/`0x352c`) lie past SPU local-store offset `0x2d00` - and `0x2d00` is exactly what `iRam008bb590` (`0x2cf8`, confirmed again this session by direct `read_memory`) rounds up to via `_opd_FUN_0046d888`'s own `(x + 0x7f) & ~0x7f` formula, i.e. **the registered end of this job's own local-store image**, the same number the previous session used to justify extracting `0x2d00` bytes in the first place. Re-extracting a larger range (`scripts/ps3-spu-disasm.py 0x00848780 0x6000`) to see what is actually there: **SPU offset `0x2d00` (VA `0x84b480`) is the start of a second, self-contained SPU program**, with the *same* header shape as this job's own byte `0`: four opaque `ila`-loaded words, a `lqa` from a fixed local-store address, a three-quadword `stqd` stack/context setup, then an unconditional `br` to its own internal offset (`0x2d28` branches to `0x2e00`, i.e. `+0x2100` from its own base, mirroring this job's own byte-`0` header branching `+0x21f0` from its base). Its own internal branches, `hbrr` hints and `ila`-loaded jump-table base (`0x11580`) are all self-consistent *relative to its own base at `0x2d00`*, not this job's.

**Nothing in region A (`0x0`-`0x2cf7`, this job's own registered image) branches into region B (`0x2d00` onward), and nothing in region B branches back into region A**, once each region's addresses are read relative to its own base rather than the raw file offset (an initial mechanical scan flagged four apparent "region B → region A" branches at `0x2d00`/`0x2d10`/`0x2d40`/`0x2e30`, all landing at small numbers like `0x678`/`0xc00`/`0xc80` - these are region B's *own* header/local constants, coincidentally small enough to look like region-A addresses under naive absolute comparison; they are `0xc00`-relative-to-region-B's-own-base, not a cross-reference, the same way region A's own byte-`0` header constant `0x2dab0` is not a "real" address either). The two regions are independent SPU programs that happen to sit back-to-back in `EBOOT.elf`, consistent with this being one embedded blob among several the linker packed sequentially, not a multi-segment overlay of one job.

**This retracts the field-2-continuation reading, not just narrows it: `ila $0, 0x3518` cannot be a reachable branch target for this job**, because SPURS only DMAs the registered `0x2d00` bytes into local store at workload start (documented SPURS behaviour, and consistent with `iRam008bb590` being the literal size argument `_opd_FUN_00466548` passes to `cellSpursWorkloadAttributeInitialize`) - local store past that offset holds whatever a *different* workload left there, never this job's own code. **What the `ila $0, <addr>` values are actually for is now an open fork, not a traced answer**: either (a) these four branches are dead code for this specific `0x2d00`-byte-registered job - plausible given the PPU-side "six pure-discard sites" finding already on this page, where most real callers zero the descriptor's fields before *and* after use, so the nonzero-field path this dispatch guards may simply never fire for most or all of this job's nine PPU-side registrants; or (b) `$0` is not being used as a same-job branch target here at all, and this session's read of the save/restore pattern, while directly observed, is being over-interpreted as "return to per-field code" when it serves some other purpose this session did not consider. **Not chased further** - resolving between (a) and (b) needs either a live RPCS3 read of this job's own local store while `Enable_spu_vertex_light`'s gate is open and a caller with a genuinely nonzero descriptor field executes (the same live-capture shape this section already used for the `0x008b83b0` buffer), or tracing every one of `FUN_005fc728`'s own nine PPU-side callers for one that populates a nonzero field via `_opd_FUN_004651c0`/`_opd_FUN_004652d0`, neither attempted this session.

**Confidence 80** for the boundary-and-header-match finding itself - mechanically checked (`iRam008bb590` read directly, the two headers' shapes compared byte-for-byte, all of region B's own internal branch targets independently self-consistent), not inferred. This **retracts** the previous session's confidence-65 "four-slot DMA-fetch loop... the same field-index scheme `FUN_005fc728`'s own descriptor builder writes" claim - not because the field-index scheme is wrong (the PPU builder's 0-3 field indexing is untouched by this entry), but because there was no DMA-fetch loop at the address that claim named, and the actual per-field consumer (the GETLLAR/PUTLLC decrement, previous entry) does not read or fetch anything shaped like vertex data - it manipulates a single shared counter. No code changed; the extracted binaries (`/tmp/render-ops-job.bin`, `/tmp/render-ops-job-ext.bin`) are not committed, per this project's leakage policy - only the extraction commands above are.

### Opcode `0x2d`'s `(address, value)` operand pair is decompiled at its own PPU-side source: `address` is the already-live-captured SPU vertex-light buffer's current slot, `value` is a previously-unread companion array (2026-09-18, later still, confidence 85)

**Following this page's own next step** ("tracing `FUN_005fc728`'s nine PPU-side callers for one that populates a nonzero field") **landed on something better: the per-chunk compiler that emits opcode `0x2d` in the first place, decompiled in full.** `_opd_FUN_004074e0` (`0x004074e0`) - already named on this page as one of the two functions with a genuine per-chunk shader-variant gate on bit `0x800` - was decompiled directly. It resolves the operand question this page has carried as open since the "14 read sites" entry, from the emitting side rather than the consuming side:

**Once per draw call, before the per-chunk loop starts:**

```c
cVar20 = *(char *)((int)local_110 + 0x5a3);      // Enable_spu_vertex_light
*local_1a0[0] = 0x2d;
local_1a0[0][1] = 0;
local_1a0[0][2] = 0;
if ((cVar20 == '\0') || (*(char *)((int)local_110 + 0x5aa) == '\0')) {   // && Debug.Enable EdgeGeom
    local_12c = 0;
    local_128 = 0;
} else {
    local_12c = _opd_FUN_0040d390();   // buffer VALUE at the current slot
    if (local_12c != 0) {
        local_128 = _opd_FUN_0040d370();   // buffer slot ADDRESS
    }
}
```

`_opd_FUN_0040d390`/`_opd_FUN_0040d370` are **exactly the two functions this page already named and read** in the "14 read sites" entry: `_opd_FUN_0040d370` returns `*(int *)(iRam008b83b0 + 0x2080) * 0x1000 + iRam008b83b0 + 0x80` - the current slot's own address inside the double-buffered `0x1000`-byte-stride region this project has **already live-captured** (`scripts/rpcs3-spu-light-dump.py`, "up to 8 records of 8 big-endian floats"). `_opd_FUN_0040d390` returns `*(undefined4 *)(*(int *)(iRam008b83b0 + 0x2080) * 4 + iRam008b83b0 + 0x2084)` - a **previously-unread companion array at `iRam008b83b0 + 0x2084`**, indexed by the identical running slot index at `+0x2080` the address computation itself uses. Confirmed by direct `decompile_function` re-read this session, byte for byte against the earlier documented bodies - not a new read, but the first time this session connects them to opcode `0x2d`.

**Per chunk**, gated on the same per-chunk bit-test this page's "twelve unexamined call sites" entry already documented (`(1 << (chunk_id & 7) & table[...]) == 0) || (local_128 == 0)`):

```c
} else {
    *puVar33 = 0x2d;
    puVar33[1] = local_128;     // = _opd_FUN_0040d370()'s return: the buffer's current slot address
    puVar33[2] = local_12c;     // = _opd_FUN_0040d390()'s return: the +0x2084 companion value
    iVar21 = Shader_GetVariantHash(local_13c | uVar19 | 0x800);
```

**This is a direct, gapless decompile chain, not an inference**: the exact two return values this page already established read a live, double-buffered, main-memory light-candidate array are the exact two words opcode `0x2d` embeds in the compiled-ops stream whenever a chunk's own per-chunk bit is set. `address` is a genuine main-memory pointer into the buffer this project already has 8-record structural data for; `value` is a small 4-byte read from an array this page has never named before now (`+0x2084`), parallel to the `+0x80`-based 8-record array and sharing its index.

**What this does not establish.** Whether the compiled-ops stream (copied wholesale into the SPU job's ring-slot payload at `+0xd8`/`+0xdc` by `FUN_005fc728`, per the "job's own executable binary" entry) is literally what populates the `$80`/`$81` registers the SPU-side field-presence dispatch reads (two entries above) is **not confirmed this session**. Tracing `$80`/`$81`'s own SPU-side provenance backward from SPU offset `0x2ae0` lands, within a few hundred bytes, on a *third*, distinct `MFC_GETLLAR`(`0xd0`)/`MFC_PUTLLC`(`0xb4`) block (SPU offset roughly `0x2900`-`0x2ae0`) with the shape of a hash-table insert-or-lookup (`ceq`/`selb`/mask-heavy, operating on a different structure than either of the two atomic-decrement helpers already read) - not read this session, and the single most direct way to close the gap between "opcode `0x2d`'s PPU-emitted operands" and "what the SPU field-presence dispatch tests" that this page's evidence currently leaves open.

**Confidence 85.** Every step is a direct decompile with no gap: the per-chunk gate, the two-function call, and the operand assignment are read verbatim from `_opd_FUN_004074e0`'s own body, and the two called functions were already independently confirmed on this page. It does not reach higher because connecting this PPU-side finding to the SPU-side register dataflow (the remaining, genuinely open half of the question) is not yet made. No code changed.

### Correction: `$80`'s "job-descriptor field 0/1/2/3" and "field value is a main-memory effective address" readings are retracted - `$80`/`$81` are the subroutine's own callee-saved locals, reused as scratch, not caller-supplied data (2026-09-18, later still, confidence 75 for what survives)

**Checking the subroutine containing the field-presence dispatch (two sections above) for its true entry point, rather than continuing to trace forward from `0x2ae0`, finds a save/restore pair that invalidates part of that entry's own framing.** SPU offset `0x2920` - reached via `brsl $0, 0x2920` from `0x2904` - opens with:

```
0x2920  lqa  $2, 0x12f0
0x2924  stqa $0, 0x1580     ; spill the caller's own $0 (return address)
0x2928  shli $0, $2, 5
0x292c  stqa $80, 0x1590    ; spill the caller's own $80
0x2930  shli $80, $2, 6     ; $80 immediately reassigned - scratch from here on
0x2934  stqa $81, 0x15a0    ; spill the caller's own $81
0x2938  ila  $81, 0x1440    ; $81 immediately reassigned - scratch from here on
```

This is the **same** `0x1580`/`0x1590`/`0x15a0` triple the "no field set" fallthrough path at `0x2b30`-`0x2b3c` restores before its own `bi $0` - confirming `0x2920`-`0x2b3c` is one coherent region, and that region's *own* incoming `$0`/`$80`/`$81` are saved here, then **`$80` and `$81` are reassigned as scratch for the rest of the region** (confirmed further at `0x2a4c`, `lqx $80,$81,$80`, and `0x2a7c`, `rotqbyi $81,$7,12`, both inside the `MFC_GETLLAR`/`MFC_PUTLLC` hash-table-shaped block the previous entry flagged as unread). **The field-presence dispatch at `0x2ae0`-`0x2b28` runs before the restore, not after** - so the quadword it tests in `$80` is whatever that hash-table block computed internally, not the caller's own incoming value, and the `ceqi $2,$81,1/2/3` "job-type dispatch" two entries above is testing a scratch register on the same footing.

**This retracts two specific claims from the "four-slot DMA-fetch loop is retracted..." entry above, while leaving the rest of it standing:**

- **Retracted**: "a job-descriptor field-presence check on a quadword register, `$80`, treated as four 4-byte sub-fields" - there is no basis left for calling these four sub-words "descriptor fields." They are sub-words of a value this subroutine computed, and nothing traced this session ties them to `_opd_FUN_004651c0`'s own `0`-`3` field-index parameter (which, per the decompile in that same entry, packs its own "value" into only 8 bits of a 64-bit word passed to a completely different builder object - never address-shaped in the first place, a detail that should have been weighed harder before the field-index framing was written).
- **Retracted**: "the field's own value is treated as a main-memory effective address" - unsupported for the same reason; what `$80`'s sub-words actually represent is open again, not "a job-descriptor value."
- **Survives, unchanged**: the `MFC_GETLLAR`(`0xd0`)/`MFC_PUTLLC`(`0xb4`) identification at `0x1bd0`, the decrement-and-test-for-`1` register dataflow, and the retry-loop control flow - these are read directly off fixed MFC command encodings and unambiguous register moves, independent of what `$80`'s value *means*. What changes is only the label for *whose* value is being decremented: not necessarily a per-field job-descriptor counter, but a counter this subroutine's own internal hash/dedup logic (still unread) selects.
- **Survives, unchanged**: the region-boundary/second-SPU-program finding (mechanically checked against `iRam008bb590` and the header shapes, independent of this correction) and the opcode-`0x2d` PPU-side decompile in the entry immediately above (PPU-side evidence, untouched by an SPU-side register-provenance correction).

**Confidence for the surviving GETLLAR/PUTLLC mechanism moves to 75, matching where it was first placed** - a claim resting on a register whose provenance turned out to be internally-computed rather than caller-supplied belongs at the "single-site inference, not yet closed" tier, not higher, per the project's own confidence rubric.

**Named for the next session, so the pattern is not repeated a third time**: this is the second time in this thread that a plausible-looking structural reading of this SPU job (first "the four-slot DMA-fetch loop," now "`$80` is the job descriptor") dissolved on closer reading of adjacent code. The lesson both times was the same - a register or a code shape looked meaningful in isolation, and turned out to belong to a *different* unit of code (a different SPU program entirely, the first time; a callee's own spilled locals, this time) once its context was checked. Before naming what a register holds in this binary, check what saves and restores it, not just what reads it.

**Not chased further, per this session's own advisor-directed stop**: finding the *true* entry point of the `0x2890`-`0x2920` region (it is reached from inside a scan loop that itself does not set `$80`/`$81`, meaning their real origin is further back still) and checking whether its genuine SPU-ABI argument registers (`$3`/`$4`, not examined at all this session) carry the opcode-`0x2d` `(address, value)` pair directly. If they do, the PPU-emission finding two entries above and the SPU-side consumer close in one step. **This is exactly the shape of question a live capture answers directly and cheaply**: dump this job's own local store during a race with a real chunk taking the `0x800`-bit path (`scripts/rpcs3-spu-light-dump.py` already has the boot/attach/read shape this needs), rather than continuing to hand-trace register provenance through more unread code.

### Both named next steps are checked this session, and both come up short - the live-capture route is blocked by an already-documented project limitation, and the static trace runs out of writes to follow (2026-09-18, later still)

**The live-capture route is not available with this project's existing tooling, and this was already known before this session, just not connected to this thread.** `docs/reverse-engineering/rpcs3-debugger.md`'s own "What a stopped thread's PCs do and do not tell you" section states plainly: RPCS3's GDB stub (`scripts/rpcs3_debugger.py`, which `rpcs3-spu-light-dump.py` is built on) lists only PPU threads via `qfThreadInfo` - "the 200-330% CPU the process burns is RSX and SPU work the thread list does not cover." `rpcs3-spu-light-dump.py`'s own read of the `Enable_spu_vertex_light` double-buffer works only because that buffer sits in ordinary PPU-addressable main memory (the pointer chain from `0x008b83b0` resolves to a ordinary EA, which the earlier "480 MiB in six pieces" mapping already covers); it never touches an SPU core's own local store, and nothing in this project's GDB-stub client can. **Reading this job's local store live - its own registers, or its own `0x2d00`-byte working memory - needs a different RPCS3 interface than the one this project has scripted**, not just a new script against the existing one. Not investigated further this session; a candidate worth naming for whoever picks this up is RPCS3's own interactive debugger UI (SPU thread list, register and local-store views), which the existing headless/GDB-stub approach was chosen over specifically because it needs no such UI - trading that convenience away is a real cost, not a small one.

**The static route was pushed one step further and the trail runs cold, not just far.** Searching every instruction in the extracted `0x0`-`0x2920` range for a write to `$80` or `$81` as its destination operand finds **exactly one**, at SPU offset `0xd0c` (`stqd $80,32($1)`, a spill, followed by `ai $80,$3,0` - a genuine copy from a real SPU-ABI argument register, inside a small, self-contained function with its own stack frame) - and **nothing at all writes either register anywhere before that**, across roughly 3,400 bytes of code. This means either: (a) `$80`/`$81` hold whatever the SPU's registers contain at job start (unlikely to be meaningful on its own, though Cell SDK job systems sometimes do pass fixed argument values in specific registers by convention at workload entry - not checked), or (b) the disassembly's own linear byte order does not match the program's actual execution order closely enough for a straight top-to-bottom scan to find the real assignment - plausible, since `br`/`brsl` already jump around substantially in every function this thread has read on this page. **Distinguishing (a) from (b) needs finding this job's own true entry point** (the very first instruction SPURS transfers control to, not merely the first byte of the extracted range) and reading forward from there in *execution* order rather than *file* order - a different, more expensive kind of trace than anything done on this page so far, and not attempted this session.

**Net for this session's own advisor-directed close-out**: both routes named as "the next step" in the entry above were checked, and neither resolves the open question cheaply. The two prior entries' findings (the GETLLAR/PUTLLC mechanism at confidence 75, the opcode-`0x2d` PPU-side decompile at confidence 85, the region-boundary finding at confidence 80) all stand unchanged - this entry adds no new claim about the SPU job's behaviour, only a corrected map of which approaches are and are not available. **What this leaves for a future session, honestly ranked**: research whether RPCS3 exposes SPU thread state through any interface this project could script against (the real blocker, and the one worth solving once, since it would also serve every other open SPU question on this page - the `0x008b83b0` buffer's own consumer chief among them); failing that, a from-entry-point execution-order trace of this job's early code, accepting the cost. No code changed; no new binary extracted this session beyond what commits `14a9c156`/`f3ca940d`/`0959643c` already covered.

### The `+0x2084` companion array is read live: small integers in the buffer's own `0`-`8` range, not a float - "how many of the 8 slots are valid this frame" is a live-supported hypothesis, not a traced one (2026-09-18, later still, confidence 55)

**The one unblocked lead the previous entry's own close-out did not chase**: `iRam008b83b0 + 0x2084`, the companion array `_opd_FUN_0040d390` reads and opcode `0x2d` embeds as its `value` operand (two entries above), is ordinary PPU main memory - unlike the SPU job's own local store, it needs no new RPCS3 interface, only a one-line addition to `scripts/rpcs3-spu-light-dump.py`'s existing read (committed this session: reads `ptr + 0x2084 + index*4` alongside the slot it already dumps).

**Live, five snapshots two seconds apart, Amphiseum, default play** (`uv run --with evdev python3 scripts/rpcs3-spu-light-dump.py`): `companion_value_u32` reads `7, 7, 7, 2, 2` across the five snapshots. **As a float this is a subnormal near zero every time** (`9.8e-45`/`2.8e-45`) - the value is meant to be read as an integer, settling that half of the question outright. The value changes between snapshots (`7` -> `2`) while `index_raw` stays at `0` for four of the five (one snapshot reads `1`) - a real, live-varying quantity, not a fixed constant misread as a companion array.

**Both observed values sit inside the buffer's own already-established `0`-`8` record cap** (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s own "up to 8 records" finding), which is suggestive but not proof: a natural reading is that this is a per-frame **count of how many of the slot's 8 records are actually valid this frame**, needed precisely because the raw buffer bytes cannot distinguish a genuinely-updated record from a stale one left over from an earlier frame (`slot_nonzero_bytes` stays flat at `216`-`222` across all five snapshots regardless of the companion value, consistent with unused record slots holding old, still-nonzero data rather than being zeroed - which is exactly the situation an explicit count would exist to resolve). **This is a hypothesis the magnitude is consistent with, not a traced consumer** - no code anywhere, PPU or SPU, has been read this session or any prior one that treats this value as a loop bound or count. It could equally be a generation/version tag, a slot-selection index into some other table, or something this thread has not considered.

**Confidence 55.** This is a live, reproducible measurement (stronger evidence than any hand-disassembly reading on this page, and the first live SPU-adjacent evidence this specific sub-question has had), but the *interpretation* rests on magnitude-range correlation alone, with zero consumer traced - below the 70-84 "single-site inference" band this page uses for a read with at least one traced consumption site. Raising it needs either a consumer (PPU or, if ever reachable, SPU) that reads `+0x2084` and uses it as a bound, or a capture at a moment with a verifiably different number of nearby light-candidate objects to check the value tracks that count specifically rather than something else that happens to also stay small. Artefacts: `/tmp/hd-spu-light-companion/` (gitignored, not committed - `meta.json` and five `s*_slot.bin` raw dumps).

### The three surviving `Enable_spu_vertex_light` claims are re-derived independently, and all three stand - one is narrowed (2026-09-18, later still)

A fresh session re-checked the confidence-75/80/85 findings the previous close-out left standing, against the decompiler and the SPU disassembly rather than against the page, before extending the thread:

- **Opcode `0x2d`'s operand pair (confidence 85) stands.** `_opd_FUN_004074e0` re-decompiled in full: `local_12c = _opd_FUN_0040d390()`, `local_128 = _opd_FUN_0040d370()` only when `local_12c != 0`, and the per-chunk emit is `puVar33[1] = local_128; puVar33[2] = local_12c` under `Shader_GetVariantHash(... | 0x800)` - verbatim as documented. `_opd_FUN_0040d370`/`_opd_FUN_0040d390` re-decompiled byte for byte. One detail the earlier entry did not draw out: **`value` gates `address`** - when the companion word reads `0`, the slot address is never computed and every chunk gets the inert `(0, 0)` pair. That is the shape of a count, and it is the first hint the next section confirms.
- **The `MFC_GETLLAR`/`MFC_PUTLLC` decrement at SPU offset `0x1bd0` (confidence 75) stands, with one precision.** Re-disassembled (`scripts/ps3-spu-disasm.py 0x00848780 0x3000`): `il $3,0xd0` -> `wrch $ch21`, `il $6,0xb4` -> `wrch $ch21`, `rdch $6,$ch27`, `andi $6,$6,1`, `brnz $6,0x1be8` (retry), `ahi $10,$8,-1` (decrement), `ceqhi $8,$8,1` (pre-decrement test for `1`) - exactly as recorded. The counter is a **halfword**, not a byte: `chx $9,$0,$2` generates a halfword-insert mask and `ceqhi` is the halfword compare. Cosmetic against the mechanism, but the earlier "counter byte/halfword" hedge resolves to halfword.
- **The SPU offset `0x2d00` program boundary (confidence 80) stands, and is narrowed.** `iRam008bb590` re-read as `0x2cf8`; the bytes at offset `0x2d00` (VA `0x0084b480`) are the four-`ila`-word, `lqa $1,0xc00`, three-`stqd`, `br` header the earlier entry describes. What the earlier entry did not have: that second program is only **`0x280` bytes** long (`0x0084b480`-`0x0084b700`; its `br` at `0x2d28` lands at `0x2e00`, `+0x100` from its own base, not `+0x2100` as recorded), and at `0x0084b700` a **third** blob starts, with a different header shape entirely - the `0xc0dec0de`-magic job header the next-but-one section identifies. The boundary claim survives; the "region B branches `+0x2100`" arithmetic does not, and nothing rested on it.

Nothing here lowers any of the three confidences. What follows does change what one of them was *about*.

### Correction: `0x00848780` is the SPURS job-queue policy module, not the render-ops job - the job that actually receives opcode `0x2d`'s stream is `Live_RcsBlackbox` at `0x0084b700`, `0xd690` bytes (2026-09-18, later still, confidence 85)

**This is the fourth wrong turn in this sub-thread, and it is the one the other three were standing on.** The "job's own executable binary is located" entry named `0x00848780`/`0x2d00` as "the render-ops SPU job" because `FUN_005fc6c8` registers it through `cellSpursAddWorkloadWithAttribute`. That registration is real and re-confirmed. But `cellSpursWorkloadAttributeInitialize`'s program argument is, by the Cell SDK's own definition, a **policy module** - the SPURS-side scheduler that dequeues work descriptors and loads *job binaries* into local store - not a job. The distinction was visible on the PPU side all along and was not read:

**`FUN_005fc728` (the render-ops submit function, re-decompiled) names its job binary as descriptor field `0`:**

```c
_opd_FUN_00465588(puVar2, 0, 0, PTR_DAT_008bf6b8 + 0x48, 2);   // field 0: {ea, size} at struct+0x48
```

`_opd_FUN_00465588` (decompiled) reads `param_4[0]` as an effective address and `param_4[1]` as a byte size and packs them into one 64-bit descriptor word (`(size >> 4) << 0x32 | (ea >> 4) << 4 | 0x800000000000 | 0x40000000000`). **`FUN_005fc550`, the queue's own registration function (decompiled), fills that `+0x48` pair:**

```c
_opd_FUN_004699f8(PTR_DAT_008bf6b8 + 0x48, PTR_DAT_008bf6bc, uRam008bf6c0, PTR_s_Live_RcsBlackbox_008bf6c4);
```

`_opd_FUN_004699f8(slot, ea, size, name)` (decompiled) stores `slot[0] = ea; slot[1] = size` and enters `(ea, refcount, name)` into a 32-entry table at `PTR_DAT_008bb380` - a named, reference-counted SPU-binary registry. `read_memory` at `0x008bf6b8`: `PTR_DAT_008bf6bc = 0x0084b700`, `uRam008bf6c0 = 0xd690`, the name string at `0x007ce3f8` = `Live_RcsBlackbox`, and the queue's own name two words later = `LiveRcsBlackBox`. **`read_memory` at `0x0084b700`** shows the same header the `LightCulling` job (next-but-one section) and `docs/rendering/trail-ribbon.md`'s `Trails`/`WakeTrail` jobs share: four `ila`-shaped words, `0x30` (code offset), `0xd6b0` (local-store image size, `0x20` past the file size for bss), `0x1d50`, `0xf0`, the `0xc0dec0de` magic, `8`, `0x4000` (load address), `4`, then the `stqd $80,-16($1)` / `stqd $81,-32($1)` / `stqd $82,-48($1)` / `stqd $126,-64($1)` prologue at `+0x30`. `0x00848780` has none of this: no magic, a header that branches into itself, and `brasl $79,0x3fec0` calls to fixed top-of-local-store addresses - the SPURS kernel's own entry region, which only a policy module calls; a job calls the queue runtime through the function table its context hands it (`LightCulling` does exactly that, `lqx $28,$30,$83 ... bisl $0,$27` through `ctx+0x34`).

**What the render-ops job actually receives, also read this session** (`FUN_005fc728`'s disassembly, the 16-byte inline user data at `r1+0x70`): word `0` = `r25`, the ring-slot copy of the calling context (the `0x160`-byte struct `_opd_FUN_005a20f8` copies first - the one whose `+0xd8`/`+0xdc` are the compiled-ops stream pointer and count, re-pointed at the stream's own ring copy), words `1`/`3` = two values from `_opd_FUN_005cbeb8`/`_opd_FUN_005cbde0`, word `2` = `-1` or a shader-hash-derived word. So `Live_RcsBlackbox` DMAs the whole context, finds the ops stream through it, and interprets it there - the SPU mirror of `Render_RunCompiledOps_q` this page hypothesised, now with its binary located: **`0x0084b700`, `0xd690` bytes, `scripts/ps3-spu-disasm.py 0x0084b700 0xd690`** (13,692 disassembly lines; not read this session beyond confirming it is coherent code with 241 `bisl` calls and a `0x30`-offset prologue).

**Consequences for the entries above:**

- The `MFC_GETLLAR`/`MFC_PUTLLC` decrement, the `$80`/`$81` provenance hunt, the "job type `1`/`2`/`3`" dispatch and the hash-table block are all **job-queue runtime** behaviour, shared by the nine queues the earlier entry counted. "Whose counter" now has a natural PPU-side twin worth one sentence: `_opd_FUN_00469340` (decompiled this session) is a `lwarx`/`stwcx.` decrement-and-test-for-`1` on the same `PTR_DAT_008bb380` registry's refcount word, releasing the entry when it hits zero - the same idiom, on the same kind of object. Not confirmed to be the SPU decrement's target; a lead for whoever returns to the policy module, which this thread no longer needs to.
- The previous close-out's "genuinely blocked - needs an SPU-thread-capable RPCS3 interface or an execution-order trace from this job's true entry point" was **blocked on the wrong binary**. Neither route was wrong in itself; both were aimed at a program that does not consume opcode `0x2d`. The consumer that does is a static file range, extractable today.
- The confidence-88 "this is the correct binary" claim narrows to: the correct *workload* (policy module) for the render-ops queue. As a claim about which code interprets the ops stream, it is retracted.

**Confidence 85** for the identification - a gapless PPU decompile chain (`FUN_005fc728` field `0` -> `+0x48` -> `FUN_005fc550`'s registration -> `PTR_DAT_008bf6bc`/`uRam008bf6c0` read directly), corroborated at the byte level by the header shape three independently-located jobs share and the policy module lacks. Not higher because the `Live_RcsBlackbox` binary itself has not been read past its header.

### The `+0x2084` companion word is the frustum-survivor count: the compaction function this page said was never located is `FUN_0040d728`, called every frame from `Scene_PrepareFrame`, and three consumers read the word as a bound (2026-09-18, later still, confidence 88)

**Every function that touches `iRam008b83b0` is now read.** The pointer lives in a TOC slot (`-0x5014(r2)`, TOC `0x008bd3c4`), so `get_xrefs_to 0x008b83b0` returns nothing; `search_instructions lwz -0x5014(r2)` finds the eleven readers instead. Ten are the `0x0040d220`-`0x0040dae8` cluster; the eleventh, `FUN_000bf100`, uses a different TOC (`scripts/ps3-toc.py toc 0x000bf100` -> `0x008ad4d8`) and its `-0x5014(r2)` is an unrelated string pointer - excluded, not ignored. Nothing writes the slot (`stw -0x5014(r2)` matches zero instructions), consistent with the live capture's "never relocated" finding.

**`_opd_FUN_0040d728(frustum)` is the selection/compaction step**, decompiled and disassembled:

```c
uVar1 = *(uint *)(base + 0x2080);
*(uint *)(base + 0x2080) = uVar1 ^ 1;                 // flip the double buffer
for each candidate i in 0 .. *(base + 0x2098):        // the 128-slot list FUN_0040d990 fills
    v = (candidate.xyz, candidate[7]);                // vspltw v0,v1,3 - the record's LAST float as w
    if (Render_ClassifyAgainstPlanes_q(frustum, v) != 1)   // 1 = wholly outside
        copy 32 bytes to base + 0x80 + (uVar1 ^ 1) * 0x1000 + survivors * 0x20;  survivors++;
*(base + 0x2098) = 0;                                 // candidate list consumed
*(base + 0x2084 + (uVar1 ^ 1) * 4) = survivors;       // 0040d870: stw r24,0x2084(r9)
```

`0040d7cc lvx v1,r31,r23` (`r23 = 0x10`, the record's second quadword), `0040d7d4 vspltw v0,v1,0x3` (its fourth float), `0040d7d8`/`0040d7dc` two `vsel`s under the `vsldoi v31,v0,v1,4` mask (word 3 only) build `(x, y, z, record[7])` - a sphere for the plane test. Called once per frame, ungated, from `Scene_PrepareFrame` (`0x003abb60`, `_opd_FUN_0040d728(scene + 0x7cb0)` - the same `+0x7cb0` block `Render_SetClipPlanes_q` consumes elsewhere in the same function, i.e. the frame's clip planes). **This links the two offset pairs the "twelve unexamined call sites" entry could only juxtapose**: the 128-slot candidate list at `+0x2098`/`+0x20a0` is copied, frustum-filtered, into the double-buffered slot at `+0x80 + index*0x1000`, and the survivor count is the companion word.

**The producer's record layout, from `_opd_FUN_0040d990(float radius, float w, vector position, vector colour)` decompiled**: `record = (position.xyz, w, colour.xyz, radius)`. The "`A`, `A*0.25`, `A*0.1`" ratio the live captures kept finding is an **authored colour** `(1.0, 0.25, 0.1)` (orange) times an intensity - which is why slot 1's "anomalous" records (`80, 10, 0` - a different orange; a `D = 10.0` record) broke the ratio: they are lights of another colour and another radius, not an interpolation artefact. The "exact ratio" claim was never a structural property, only the dominant light colour on the circuits sampled. The fourth float, previously `D`, is the **radius**: used as the sphere radius by this frustum test, by the `LightCulling` job's chunk test (next section, `rotqbyi $52,$53,12`), and by the third consumer below (`0040db5c vspltw v13,v0,0x3`).

**Three consumers read the count as a bound, all decompiled:**

1. `_opd_FUN_0040d488` - `iVar2 = *(base + 0x2084 + index*4); if (iVar2 == 0) return inert; ... _opd_FUN_004655f0(job, 1, 0, index*0x1000 + base + 0x80, iVar2 << 5, 0)` - DMA field `1` of the `LightCulling` job is the current slot, **`count * 32` bytes**, exactly one 32-byte record per count.
2. `_opd_FUN_0040dae8(sphere)` - `mtspr CTR, count` then a loop over `count` records testing `(radius + sphere.w)^2 >= |position - sphere.xyz|^2` (`vcmpgefp.`), returning `1` on the first hit: "does any visible light touch this sphere." Called three times, from `FUN_003ea368` (twice) and `FUN_003eb890` - the `RigidBody`/`Absorb`/`Leach` family, which is how that family decides per object whether to stage `(slot address, count)` into context `+0x14c`/`+0x150` at all. The per-object PPU-side equivalent of what the SPU job does per track chunk.
3. `_opd_FUN_004074e0` itself (spot-check above): `value == 0` suppresses the address.

**The live capture already corroborated this, and nobody had diffed it.** `data/traces/hd-spu-light-companion/s*_slot.bin` (the previous entry's five Amphiseum snapshots, moved out of `/tmp` this session so they survive a reboot - `data/` is gitignored, so nothing changes about what is committed), diffed record by record: `s1 -> s2` (count `7`) changes records `0`-`6`; `s2 -> s3` changes `0`-`6`; **`s3 -> s4` (count `2`) changes exactly records `0`-`1`**, and record `7` is byte-identical across all four - the stale eighth record from some earlier frame with eight survivors, which is also what the "at least one record stayed byte-identical across every snapshot on both circuits" observation two entries back was seeing. `record_count: 8` in every snapshot was the script's own stop-at-first-zero scan reading a stale high-water mark, not a cap: **the slot holds up to 128 records** (`0x1000 / 0x20`, the same `0x80` the candidate list caps at), and only the first `count` are live. Any read of this buffer must bound itself by `+0x2084[index]`; `rpcs3-spu-light-dump.py`'s `record_count` should not be used as one.

**Confidence 88.** Producer, compaction and three consumers are all direct decompiles with the load-bearing vector lanes confirmed in disassembly (`vspltw ...,3`), the count is written by one function and read as a length or loop bound by three, and the existing live capture's record-by-record diff matches the count exactly on the one snapshot pair where it changed. This retires the previous entry's confidence-55 "magnitude-correlation only, no consumer traced" framing outright, and the "no selection/compaction function was located" and "whether the 128-slot list becomes the 8-slot buffer is not established" hedges in the "twelve unexamined call sites" entry with it. The other two `iRam008b83b0` functions, for completeness: `_opd_FUN_0040d890(param)` fills a `128 x 128` byte table at `base + 0x3100` with `(uint8)(powf(j * 0.00788, i * 0.0788) * 255.0)` (a falloff lookup, consumer unread; `param` stored at `+0x30a0`), and `_opd_FUN_0040d220` walks scene nodes of class `5` appending `(base pointer, 1, 4, 0x44)` entries to a per-node list - neither on this thread's path.

### The `LightCulling` SPU job is identified, its dispatch decompiled, and its `0x490` bytes read in full: it writes the per-chunk bit table that gates opcode `0x2d`, and `0x800` is the `SVC1`/`SpuVertexColours` permutation bit (2026-09-18, later still, confidence 85 PPU-side, 75 for the SPU computation)

**Identity.** `_opd_FUN_0040d3b0(1, 0xffff)` (decompiled) calls `_opd_FUN_004699f8(base + 0x2090, uRam008b83b4, uRam008b83b8, PTR_s_LightCulling_008b83bc)` - the same named-binary registration `Live_RcsBlackbox` uses (previous-but-one section) - and stores `~Crc32_HashString("SpuVertexColours")` at `base + 0`. `read_memory 0x008b83a0`: `uRam008b83b4 = 0x007f6880`, `uRam008b83b8 = 0x490`, the string at `0x007b4410` = `LightCulling`. `read_memory 0x007f6880`: the job header (`0x30` code offset, `0x4b0` image size, `0xc0dec0de`, `0x4000` load address) and the shared `stqd $80,-16($1)` prologue. `_opd_FUN_00469320(base + 0x2090)` (decompiled: `(*(ea+0x14) + 0x3ff + *(ea+0x18)) >> 10`) reads that header's `+0x14`/`+0x18` words for the job's local-store footprint in KiB - `2` here, `61` for `Live_RcsBlackbox` (`0xd6b0 + 0x1d50`).

**Dispatch: `_opd_FUN_0040d488(out, chunk_spheres_ea, chunk_count, bitmask_ea)`**, decompiled and disassembled (its fourth argument, `r6`, is missing from the decompiler's signature but stored at `r1+0x78`):

| descriptor field | call | contents |
| --- | --- | --- |
| `0` | `_opd_FUN_00465588(job, 0, 0, base+0x2090, 2)` | the job binary `{0x007f6880, 0x490}` |
| `1` | `_opd_FUN_004655f0(job, 1, 0, slot_address, count << 5, 0)` | the current light slot, `count` x 32 bytes |
| `2` | `_opd_FUN_004655f0(job, 2, 0, chunk_spheres_ea, (chunk_count & 0xfffffff) << 4, 0)` | `chunk_count` x 16 bytes |
| `3` | `_opd_FUN_00465330(job, 3, 0, 0, 0, 0)` | an output slot, no DMA |
| inline user data (`r1+0x70`, 16 bytes) | `_opd_FUN_004654e8(job, r1+0x70, 0x10)` | `{count, chunk_count, bitmask_ea, 0}` |

then `_opd_FUN_004688a0(...)` - the same `cellSpursReadyCountStore` submit chain `FUN_005fc728` uses - and the returned ticket is written to `out`. **Its one caller, `_opd_FUN_0040aba0`** (the track-visibility pass that owns the `Job_TrackVisibilityTest` / `Job_PushVisibleRenderables` / `Job_BuildVisibleArray` strings), decompiled: it zeroes the `0x1000` bytes at `PTR_DAT_008b8264 + 0x205400`, calls `_opd_FUN_0040d488(local_100, *(track + 0x24), *(track + 0x1c), PTR_DAT_008b8264 + 0x205400)`, stores the ticket at `+0x61208`, does its own PPU-side visibility work, and at its end spins on that ticket (`_opd_FUN_00465e10(queue, 3, ticket)` - decompiled, waits until the minimum of six per-SPU completion counters at `+0x42..+0x4e` reaches the ticket) before returning. `track + 0x1c` is the chunk count (the same field every loop in that function bounds on) and `track + 0x24` the per-chunk 16-byte array `_opd_FUN_004074e0` itself reads as `(centre.xyz, radius)` for its own sphere-versus-camera distance test (`uVar19 = *(iVar21 + 0x24) + chunk * 0x10`, then `vectorSubtract`/`vectorMultiplyAdd`) - the chunk bounding spheres.

**The two bit-table symbols are one table.** `PTR_DAT_008b8264 = 0x00d44e80`, `PTR_DAT_008b82b0 = 0x00f44e80`, exactly `0x200000` apart, so `FUN_004074e0`'s `PTR_DAT_008b8264 + 0x205400` and `FUN_00408fa8`'s `PTR_DAT_008b82b0 + 0x5400` are both `0x00f4a280` - closing the "not confirmed to be the same underlying table" hedge in the "twelve unexamined call sites" entry.

**What the SPU program does** (`scripts/ps3-spu-disasm.py 0x007f6880 0x490`, 299 lines, all read; `$126` = load base `- 0x4000`, so every `0x4xxx` literal is an image-relative address). After the shared prologue and a `0x1b0`-`0x2b0` block that fetches its three buffers through the queue runtime's function table (`bisl $0,$27` etc. with `$4 = 1, 2, 3`, results in `$82` lights, `$81` spheres, `$80` output) and reads the user data into `stack+32`:

```
2f8  lqd  $12,32($1)            ; user[0] = light count
304  brz  $12,0x450             ; no lights: clear the chunk's bit and continue
30c  fsmbi $13,15  / 310 ila $7,0x10203       ; lane-3 mask, splat-word-0 shuffle
31c  lqx  $11,$35,$81           ; sphere[chunk] = (cx, cy, cz, r)
320  rotqbyi $34,$11,12 / 324 shufb $10 = splat(r)
334  lqd  $53,16($6)  / 33c lqd $50,0($6)     ; light quad 2 (colour.xyz, radius), quad 1 (pos.xyz, w)
34c  rotqbyi $52,$53,12 / 350 shufb $51 = splat(light radius)
354  selb $49,$50,$51,$13       ; (px, py, pz, light radius)
358  fs   $47,$49,$11           ; (dx, dy, dz, .)
364  fm   $43,$47,$47  / 378 fa / 37c fa      ; dx^2 + dy^2 + dz^2 in lane 0
368  fa   $45,$46,$10           ; light radius + chunk radius
374  fm   $39,$45,$45           ; (r1 + r2)^2
384  fcgt $37,$38,$39 / 388 gb / 38c brnz $36,0x32c   ; dist^2 > (r1+r2)^2: next light
390  rotmi $58,$14,-3 / 398 andi $60,$14,7 / 3a8 shl $54,$59,$60
3b4  or   $4,$55,$54 / 3c0 cbd / 3c4 shufb / 3c8 stqd    ; out[chunk >> 3] |= 1 << (chunk & 7)
3d0  rotqbyi $4,$62,4 / 3d4 clgt $61,$4,$14 / 3d8 brnz  ; while chunk < user[1]
3e4  rotqbyi $73,$75,8          ; user[2] = bitmask EA
3f4-420  wrch $ch16 (LSA=out), $ch18 (EAL), $ch19 ((chunk_count + 7) / 8, rounded to 16), $ch20 (tag), $ch21 = 0x20 (MFC_PUT)
```

`0x450`-`0x478` is the no-light path: `rot $76,-2,chunk&7` / `and` clears the chunk's bit instead. So: for every chunk, set its bit if any visible light's sphere `(position, radius)` intersects the chunk's bounding sphere, then DMA the `(chunk_count + 7) / 8` bytes back to `bitmask_ea`. **That is the table `_opd_FUN_004074e0`/`FUN_00408fa8` test with `1 << (chunk & 7) & table[chunk >> 3]` before emitting opcode `0x2d` and selecting the `| 0x800` shader variant** - the same byte/bit split, the same base, zeroed by the caller immediately before the job and waited on before the draw compilers run.

**`0x800` is `SVC1`.** `Shader_GetVariantHash(word)` is `table[word]` over the 4096-entry permutation table (this page's "`Shader_GetVariantHash`" entry, confidence 80), and `docs/formats/rcsmaterial.md`'s independently-derived bit layout of that same word (confidence 95, naming `0x4074e0`/`0x408fa8` by address as its writers) has bit `11` = "set `SVC1`, clear `SVC0`"; `1 << 11 = 0x800`. `SVC1` is the permutation whose vertex program reads attribute `0x868f8229` = `~crc32("SpuVertexColours")` with the RGBE decode this page's "The vertex-light constants are read" section found unreachable from disc data - and `_opd_FUN_0040d3b0` stores exactly that hash at `base + 0` on registration. **The whole `Enable_spu_vertex_light` pipeline on the PPU side is therefore closed**: (1) fifteen producers push `(position, w, colour, radius)` candidates; (2) `Scene_PrepareFrame` frustum-culls them into the double buffer with a count; (3) the track pass hands the survivors and the chunk spheres to `LightCulling`, which marks every chunk any light touches; (4) the draw compilers emit `(slot address, count)` as opcode `0x2d` for exactly those chunks and select the `SVC1` variant; (5) `Live_RcsBlackbox` (previous-but-one section) receives that stream and, by the variant's own declared input, must produce the `SpuVertexColours` stream the `SVC1` vertex program reads. Step (5) is the one SPU-side computation still unread.

**Confidence 85** for steps (1)-(4) as a PPU-side chain (all decompiled, the SPU job's inputs and output address are the PPU's own arguments, and the bit-table protocol is read on both ends), **75** for the SPU job's arithmetic (hand disassembly, no decompiler, per this page's ceiling - though `0x490` bytes read end to end with the loop bounds matching the PPU-side `count * 32` and `chunk_count * 16` strides is as corroborated as hand disassembly gets), and **78** for `0x800 = SVC1 = SpuVertexColours` (two documented derivations of the permutation word agree; what would raise it is reading `Live_RcsBlackbox` writing that attribute stream). No code changed; the extracted binaries and their disassemblies live under `data/extracted/ps3/spu-jobs/` (`lightculling-0x007f6880.{bin,dis}`, `live-rcsblackbox-0x0084b700.{bin,dis}`, `spurs-jobqueue-pm-0x00848780*.{bin,dis}`) - gitignored game content, never committed, but no longer in `/tmp` where a reboot would take them.

**The names the three sections above land** (`names.tsv`, all in the 70-88 band so none carries `_q`):

| address | kind | name | confidence |
| --- | --- | --- | --- |
| `0x0040d990` | function | `SpuLight_AddCandidate` | 85 |
| `0x0040d728` | function | `SpuLight_CompactVisibleCandidates` | 88 |
| `0x0040d370` | function | `SpuLight_GetVisibleSlotAddress` | 88 |
| `0x0040d390` | function | `SpuLight_GetVisibleCount` | 88 |
| `0x0040dae8` | function | `SpuLight_AnyVisibleLightTouchesSphere` | 82 |
| `0x0040d488` | function | `SpuLight_DispatchLightCullingJob` | 85 |
| `0x0040d3b0` | function | `SpuLight_RegisterLightCullingJob` | 82 |
| `0x005fc550` | function | `RenderOps_RegisterLiveRcsBlackboxJob` | 82 |
| `0x004699f8` | function | `SpuJob_RegisterNamedBinary` | 80 |
| `0x00469340` | function | `SpuJob_ReleaseNamedBinary` | 80 |
| `0x00465e10` | function | `SpuJobQueue_WaitForTicket` | 78 |
| `0x007f6880` | data | `SpuJob_LightCulling_Binary` | 85 |
| `0x0084b700` | data | `SpuJob_LiveRcsBlackbox_Binary` | 85 |
| `0x00848780` | data | `SpursJobQueue_PolicyModule` | 80 |

`FUN_0040aba0` (the track-visibility pass) and `FUN_0040d890` (the falloff table) stay unnamed: the first is a `0x1300`-line function this session read only the head and tail of, the second has no consumer traced.

### The compiled-ops vocabulary is mapped, opcode `0x30` builds an `EdgeGeom` SPURS job per chunk with the light array at descriptor `+0xb8`/`+0xbc`, and `Debug.Enable EdgeGeom`'s half of the gate is read - the per-vertex light computation lives in the `EdgeGeom` job, `0x007f6d80`, `0x107e0` bytes (2026-09-18, later still, confidence 85 PPU-side)

**The jump table `Render_RunCompiledOps_q` dispatches through is walked end to end** (`PTR_PTR_008bf21c` -> `0x00927518`, `0x31` entries, each an `.opd` pointer resolved with one more `read_memory` - the same chain the "opcode `0x2d`'s PPU-side handler" entry used for one entry). Opcode `0x01` is the terminator (null entry); the rest, with what this session decompiled:

| opcode | handler | reads | effect |
| --- | --- | --- | --- |
| `0x00` | `0x005d5dd8` | - | no-op |
| `0x02` | `0x005d5e40` | 1 | `_opd_FUN_005d6e20(ctx, a)` (not read) |
| `0x03` | `0x005d5de0` | 1 | absolute jump: `stream = a` |
| `0x04` | `0x005d6a68` | 1 | relative skip: `stream += a + 4` |
| `0x05` / `0x06` | `0x005d69f0` / `0x005d6970` | 3 | conditional jump (absolute / relative) on a **bit table at `ctx+8`**: `(table[a >> 3] >> (a & 7)) & 1` compared against `sign(b)`, target `c` - the same byte/bit split the `LightCulling` table uses, from inside a stream |
| `0x07`-`0x10` | `0x005d6920` .. `0x005d6608` | 2-3 | state setters through `_opd_FUN_005d7410`/`74d0`/`7480`/`7430`/`6e68`/`6ff8`/`71d8`/`70c8`/`6ed8` (not read); `0x0c` copies 24 words and calls `Render_SetClipPlanes_q` |
| `0x11` | `0x005d65c0` | 1 | `ctx+0xd0` = shader variant hash, dirty `0x10000` on change |
| `0x12` | `0x005d6578` | 1 | `ctx+0xd4`, dirty `0x10000` on change |
| `0x13` | `0x005d6538` | 2 | `ctx+0xc4`/`+0xc8` = `(track+0x30, track+0x2c)`, `ctx+0xec = 0`, dirty `0x1f0000` |
| `0x1b` | `0x005d62d8` | 1 | `ctx+0xcc` = material index, dirty `0x1f0000` on change |
| `0x1c` / `0x1d` | `0x005d5e18` / `0x005d5df0` | 0 | `_opd_FUN_005d7320()` / `_opd_FUN_005d72b8()` - apply/flush (not read) |
| `0x24` | `0x005d61e0` | 2 | draw, RSX vertex buffers: `_opd_FUN_005d7590(ctx, model, chunk)` (not read) |
| `0x29` | `0x005d6020` | 5 | draw, model kind `1`: `_opd_FUN_005d78a8(ctx, a..e)` (not read) |
| `0x2b` / `0x2c` | `0x005d5f90` / `0x005d5f48` | 1 | set/clear `ctx+4` flag `0x2000` / `0x4000` |
| `0x2d` | `0x005d5f20` | 2 | `ctx+0x14c`/`+0x150` = light slot address, count (already documented) |
| `0x2e` | `0x005d5ee8` | 3 | `ctx+0x13c`/`+0x138`/`+0x134` - the render-ops queue triple `Scene_PrepareFrame` also writes from `_opd_FUN_005fca90`/`80`/`78` |
| `0x2f` | `0x005d5ec0` | 2 | `ctx+0x144`/`+0x148` = **`EdgeGeom` job binary EA, local-store size** |
| `0x30` | `0x005d5e80` | 1 | draw, model kind `5` (SPU geometry): `_opd_FUN_005d7b70(ctx, chunk_desc \| flags)` - **read in full below** |

`0x14`-`0x1a`, `0x1e`-`0x23`, `0x25`-`0x2a` were not decompiled this session. `_opd_FUN_004074e0` picks the draw opcode by the model's kind byte (`model+6`): `1` -> `0x29`, `5` -> `0x30`, otherwise `0x24` - so **only kind-`5` (SPU-processed, EDGE-compressed) geometry can ever carry a `SpuVertexColours` stream**, which is the same thing `rcsmaterial.md`'s `SVC0`/`SVC1` split says from the asset side.

**Where `ctx+0x144`/`+0x148` and the `Debug.Enable EdgeGeom` gate come from, decompiled in `Scene_PrepareFrame`** (`0x003ab1a8`-`0x003ab1d0`, after `_opd_FUN_005d4868(ctx, gcm)` resets the frame's ops context): `ctx+0x144 = *PTR_DAT_008b768c` (`_opd_FUN_003bdd10`), `ctx+0x148 = header[0x14] + header[0x18]` (`_opd_FUN_00468f80`), then **`ctx+4 |= 0x1000` if `Debug.Enable EdgeGeom` (`+0x5aa`) else `&= ~0x1000`**. `PTR_DAT_008b768c`'s slot is filled by `_opd_FUN_003bdd20(1, 0xffff)` -> `SpuJob_RegisterNamedBinary(slot, PTR_DAT_008b7690, uRam008b7694, PTR_s_EdgeGeom_008b7698)`; `read_memory 0x008b768c`: slot `0x00c514a0`, **binary EA `0x007f6d80`, size `0x107e0`** (`read_memory 0x007f6d80`: the same `0xc0dec0de` job header, image size `0x10800`, `+0x18 = 0x100`), the name string `EdgeGeom`. It sits immediately after `LightCulling` in the file (`0x007f6880 + 0x490`, rounded to `0x80`). `strings` on the extracted blob (`data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.bin`, gitignored) confirms it is Sony's `libedgegeom` job with game code linked in (`EDGE error: unknown output flavor %d for vertex stream`, `ERROR: attempt to use culling without allocating an extra uniform table!`, ...). **This is the `EdgeGeom` the `Debug.Enable EdgeGeom` key names**, and it is the third and last SPU program on this thread's path.

**Opcode `0x30`'s handler, `_opd_FUN_005d7b70(ctx, chunk_desc | flags)`, decompiled in full.** First it maps the material's vertex attributes: for each attribute the model declares (`*(mesh+0x38)`, count at byte `0`, 8-byte entries), it finds which of the sixteen slots `ctx+0xf0..+0x12c` holds that attribute's hash (the slots `_opd_FUN_005d8700` fills from the vertex program's own attribute table) and binds it (`_opd_FUN_005c3c2c(gcm, slot, fmt, size, stride, 0)`). Then, per chunk (`*(mesh+0x30)` chunks, `0x80`-byte descriptors at `mesh+0x34`):

- if `ctx+4 & 0x1000` is clear (**`Debug.Enable EdgeGeom` off**): `_opd_FUN_005c47cc(gcm, 5, 0, *(chunk+0xa), 0x10, *(chunk+0x74), 1)` - a plain RSX draw straight off the chunk descriptor, **no light data anywhere on this path**;
- if set: reserve `chunk[1] * 16 + 16` bytes in the RSX command buffer (`_opd_FUN_005c1440`/`GcmContext_Callback`, the hole the SPU fills with the processed draw), and build a `0x100`-byte job record `puVar13`:

```
+0x00..+0x3f  DMA list: (index bytes, index EA), (vertex bytes, vertex EA), (chunk desc | 0x10..), (material constants copy | 0x40..)
+0x40..+0x7f  job commands (this project's own encoding, cf. _opd_FUN_004651c0/_opd_FUN_004655f0):
              field 0 alloc ((ctx+0x148 + 0x3ff) >> 10) KiB; field 1/2 allocs; field 3 = DMA in ctx+0x144 / ctx+0x148 (the EdgeGeom binary);
              field 4 = DMA in this record; 0x60000000000; 7; 0xa0000000000
+0x84         RSX command-buffer offset of the reserved hole
+0x8c         index_count (three ushorts summed), +0x88 = vertex counts
+0x90/+0xa0   mesh+0x20 / mesh+0x10 (two quadwords: the chunk's transform/bounds)
+0xb0         flags: ctx+4 bits 13/14 (opcodes 0x2b/0x2c) and the mesh kind
+0xb4         mesh attribute-table byte 9
+0xb8         ctx+0x14c   = light slot address      <- opcode 0x2d's first operand
+0xbc         ctx+0x150   = light count             <- opcode 0x2d's second operand
+0xc0..+0xfc  ctx+0xf0..+0x12c (the sixteen attribute-slot bindings) and ctx+0x12c
```

then appends `record+0x40 | 0xc0000100000000` to the job list `local_b8` (allocated `chunks * 8` bytes on the stack, the per-chunk queue handed to the render-ops ring by the caller). **So the RigidBody/`Absorb` family's `+0x14c`/`+0x150` staging, and the Zone-Stage family's opcode `0x2d`, both end in an `EdgeGeom` job descriptor carrying `(light array EA, light count)` at `+0xb8`/`+0xbc`, next to the attribute bindings the job writes its output streams through.** The `SpuVertexColours` stream the `SVC1` vertex program declares can only be produced here. Whether `Live_RcsBlackbox` (the Zone-Stage family's SPU-side interpreter, previous sections) builds the identical record from the SPU is the natural assumption and is **not read** - but it does not matter for what the light math *is*: that math runs in `EdgeGeom` either way.

**`_opd_FUN_005d8700`'s `0x800` path is read too, and it is not the lighting - it is fragment-program constant patching.** Both branches under `param_4 & 4` patch the material's fragment program constants; without `0x800`, `_opd_FUN_00731108` patches each constant in place through GCM (`_opd_FUN_005c6024`/`_opd_FUN_005c60e8` per patch offset); with `0x800`, the microcode is copied to an aligned stack scratch (`_opd_FUN_005f9cd8`, the unrolled 16-byte copy loops), `_opd_FUN_00730e08` writes every constant's halfword-swapped float4 into the copy, the copy is written back to the material's own microcode buffer (`*(material+0x10)`) and bound with `_opd_FUN_005c5d60(gcm, offset | 1, ...)`. Neither branch reads `ctx+0x14c`/`+0x150`. Confidence 75 that this is only a "patch CPU-side and re-upload whole" versus "patch in place" distinction for the `SVC1` variant's fragment program; why the `SVC1` variant needs it is not read.

**What this closes and what it leaves.** Every PPU-side step of `Enable_spu_vertex_light` now has a decompiled consumer, and the SPU side has three named binaries with static file ranges: `LightCulling` (read), `Live_RcsBlackbox` (the Zone-Stage ops interpreter, unread), `EdgeGeom` (**the per-vertex light computation, unread** - `scripts/ps3-spu-disasm.py 0x007f6d80 0x107e0`, 16,862 disassembly lines, `data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.dis`). The next read is inside `EdgeGeom`: the code that DMAs `count * 32` bytes from the descriptor's `+0xb8` and the per-vertex loop that consumes `(position, colour, radius)` records. An anchor for it: the record offsets are what was read (`lwz r9,0x14c(r25); stw r9,0x38(r11)` with `r11 = record + 0x80`, then `0x3c(r11)` for the count - `0x005d8010`-`0x005d801c`, re-checked in disassembly), so the light fields are at record `+0xb8`/`+0xbc`; the job's command words occupy `+0x40`-`+0x7f` and the game payload starts at `+0x80`, but how the policy module presents that payload to the job (whether its user-data pointer is literally record `+0x80`, making the light fields user-data `+0x38`/`+0x3c`) is not confirmed - `LightCulling` reads its own user data through `ctx+0x20`, so check that path in `EdgeGeom` rather than assuming the offset. Confidence 85 for the PPU chain (all decompiled, addresses and names read directly); no code changed.

**Names this section lands**: `0x005d7b70` function `RenderOps_BuildEdgeGeomJob` 85, `0x003bdd20` function `EdgeGeom_RegisterJob` 82, `0x005d5ec0` function `RenderOps_OpSetEdgeGeomBinary` 82, `0x005d5e80` function `RenderOps_OpDrawEdgeGeomChunk` 85, `0x005d5f20` function `RenderOps_OpSetSpuLights` 85, `0x007f6d80` data `SpuJob_EdgeGeom_Binary` 85.

### `EdgeGeom`'s light path is read: per vertex, per light within range of the chunk, `max(0, 1 - |d|/D)^w * max(0, N.L) * colour`, summed and RGBE-packed into the `SpuVertexColours` stream (2026-09-18, later still, confidence 80)

**The user-data hedge in the previous entry closes first.** `EdgeGeom`'s job main (file offset `0x2600`, `data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.dis`) does `lqd $4,32($2)` on the job context (`$2 = *0x107e0`, the context pointer the runtime shim stores - `LightCulling` reads the same `ctx+0x20`), copies `0x80` bytes from there to `stack+0x180` and stores that pointer at LS `0x10800`. Every later `lqr $n,0x10800` / `lqx ..,0x30` / `rotqbyi 8` and `12` pair reads **payload `+0x38` and `+0x3c`** - i.e. record `+0xb8`/`+0xbc`, the light array EA and count. The user-data pointer is literally record `+0x80`, so the derived offsets stand.

**The light DMA** (`0x2e18`-`0x2ea0`, taken when payload `+0x38` is nonzero - `lqd $53,432($1); rotqbyi $52,$53,8; brnz $52,0x2e18` at `0x291c`): allocate `count << 5` bytes of local store (`0x3c80`/`0x3c90`), then `wrch $ch16` = the allocation, `$ch18` = payload `+0x38`, `$ch19` = `count << 5`, `$ch20` = the job's tag, `$ch21` = `0x40` (`MFC_GET`) - the same `count * 32` the PPU-side `LightCulling` dispatch used, from the same address opcode `0x2d` carried. The output stream is fetched as attribute id `9` (`0x4988(ctx, 9)` - whether `9` is EDGE's own colour attribute id is recall, not read here), and the job's entry hook (`0x178`-`0x2a4`, which reads LS `0x10800` and so runs after the main's store, not before) locates the vertex program's `SpuVertexColours` slot by searching the sixteen attribute bindings at payload `+0x40..+0x7c` for `0x868f8229` (`ilhu $4,0x868f; iohl $4,0x8229` at `0x1c8`-`0x1d0`) - the hash `SpuLight_RegisterLightCullingJob` stores at `base+0`, now seen consumed.

**Three subroutines do the work, all read in full:**

1. **`0x3510(lights, count, bmin, bmax)` - range cull against the chunk's bounds, compacting in place.** `centre = (bmin + bmax) / 2`, `half = (bmax - bmin) / 2` (the bounds come from `0x3650` over the decompressed positions, not read - confidence 70 for "min/max"). Per light: `delta = pos - centre`; the squared distance from the light to the box, `sum(max(delta - half, 0)^2) + sum(min(delta + half, 0)^2)`, is compared with `D^2` (`fcgt $3,$6,$4`; `$4` is the square of `delta`'s fourth lane, where the light's fourth lane was set to quad 2 word 3 = `D` and the bounds' fourth lanes are taken to be zero - not verified, `0x3650` unread); a light inside `D` of the box is written back compacted, **with quad 2's fourth word replaced by `1/D`** (`frest`+`fi` on quad 2, `shufb $52,$54,$9,$28`, `stqd $52,16($21)`) and quad 1 kept as `(x, y, z, w)`. Returns the surviving count.
2. **`0x33a8(positions, normals, count, lights, light_count, out)` - the light loop.** Per surviving light, `fceq $8,$9,1.0` on quad 1's `w`: `w == 1.0` takes the software-pipelined four-vertex SoA loop at `0x3880` (remainder at `0x36c0`); any other `w` takes the scalar loop at `0x3168`. Both compute, with `d = light.pos - P` (`fs $18,$28,$75` etc.):
   - `0x3880`: `inv = frsqest(|d|^2)` (estimate only, no Newton step), `ndl = dot(N, d * inv)`, `dist = frest(inv)`, **`att = 1 - (1/D) * dist`** (`fm $68,$25,$71; fs $57,1.0,$68`, `$25` = splat of quad 2 word 3 = the `1/D` step 1 wrote), `att = max(att, 0)`, `ndl = max(ndl, 0)` (`fcgt`/`selb` against zero), `k = att * ndl`, then `out[v] += k * quad2` (`fma $2,$48,$41,$47` per vertex - the fourth lane accumulates `k/D`, discarded below).
   - `0x3168`: the same `d`, a Newton-refined normalise (`frsqest`+`fi`, `frest`+`fi`), `ndl` as above, `att = max(0, 1 - (1/D) * dist)` (`fnms $53,$38,$57,1.0`), then **`att = att ^ w`** through a `log2`/`exp2` minimax polynomial pair (the `0x3d55..`/`0xbe0c..`/`0x3f31..` constants at `0x31b8`-`0x325c`, `$35` = splat of quad 1's `w` as the exponent), `k = max(0, ndl) * att`, `out[v] += k * quad2`.
   So the record is `(x, y, z, w = falloff exponent, r, g, b, D = range)`, and the fast path is the `w == 1` special case of the general `max(0, 1 - |d|/D)^w` falloff. Every live-captured record had `w = 1.0` (the `1.0` the previous entries recorded as "w") and `D = 0.6`-`2.0`.
3. **`0x3ad0(out, count)` - RGBE pack, in place.** Per vertex: `m = max(r, g, b)`; if `m > 1e-10`: shared exponent `e = (m's IEEE exponent field) + 2`, scale `= mantissa(m in [0.5,1)) * 256 / m`, `rgb *= scale` (lands the largest channel in `[128, 256)`), then `(r, g, b, e) * (1/255)` stored as a float4 the EDGE output compressor quantises to bytes. The vertex program's own `x = 255.0` / `y = 128.0` decode constants (this page's "The vertex-light constants are read") are the right shape for this encoding, but the exact round-trip is unverified: this packer writes `biased_exponent + 2` (`k + 129`), where textbook RGBE stores `k + 128`, so a decode that assumes the standard bias would read every value at twice this packer's intent. **Check that first when the `SVC1` combine is read** - on a thread about a frame that was too bright, a factor of two is not a rounding detail.

**The two PPU culls and the SPU agree on what `D` is: a range.** `SpuLight_CompactVisibleCandidates` and `LightCulling` test spheres of radius `D`; `0x3510` tests a box against `D`; the per-vertex loop divides by it. The earlier "radius" label stands, and the live `0.6`-`2.0` values mean these lights reach one to two world units - a glow under an object, not a track floodlight - which is consistent with the producer's callers being per-object (fifteen sites, `FUN_000cfb80`'s clamped intensity).

**Not read:** `0x3650` (bounds), the `0x3030` branch (payload flags bit `1` set - `RenderOps_BuildEdgeGeomJob` sets it from the mesh kind, so a second vertex layout), the eight per-format decompressor variants the `0x455c` jump table selects (`payload+0x34 - 10`, step `4`; the variants differ from their `0x44e4` no-light twins by registering the extra output attribute - `il $46,9` - not by any light math), and how the `SVC1` vertex program combines the decoded RGBE with the rest of its lighting (a microcode read, not an SPU one).

**Confidence 80 for the record layout and the `w == 1` formula, 70 for "`w` is an authored falloff exponent"** - the latter rests on the `0x3168` path alone, which no live-captured record (all `w = 1.0`) has been seen to exercise. Hand disassembly, so below the decompile band - but three independently-read stages (cull, loop, pack) hand each other values in the shapes the PPU side predicts (`count * 32` DMA, `0x868f8229` lookup, `(x, y, z, w | r, g, b, D)` consumed field by field, RGBE with a `128` bias meeting the shader's own `y = 128` decode), and the `1/D` rewrite in stage 1 is what makes stage 2's `fm`/`fs` read as a range rather than a slope. What would raise it: a live capture of one chunk's `SpuVertexColours` output against this formula, or reading `FUN_000cfb80`'s `D` argument as an authored range. No code changed - `mesh.wgsl` still adds nothing for this; wiring it needs the `SVC1` program's own combine read first, and a decision on whether a one-to-two-unit glow is visible enough to be worth the per-chunk plumbing.

### The `SVC1` combine is read: the decoded `SpuVertexColours` term is added to the diffuse irradiance before the albedo multiply, and the RGBE round trip is exact to `256/255` - the "factor of two" flag is withdrawn (2026-09-18, later still, confidence 88)

**The exponent bias first, since the previous entry asked for it.** The packer at `0x3ad0` and the vertex program's decode are inverses, and multiplying them out is all it takes. Every constant is in the packer's own prologue: `$21 = 0x7f800000` (the IEEE exponent mask; `rotmi $32,$39,-23` extracts the field `E = k + 127` for `m = 1.f * 2^k`), `$18 = 2` (`a $36,$18,$32`, so `e = k + 129`), `$20 = 0x3f000000` (`selb $35` swaps the exponent for `0.5`'s, so `$35 = 1.f / 2 = m / 2^(k+1)`, the frexp mantissa in `[0.5, 1)`), `$19 = 256.0` (`fm $29,$37,$19` with `$37 = frest(m)`, so `256 / m`), `$24 = 0x2edbe6ff ~ 1e-10` (the guard) and `$22 = 0x3b808081 = 1/255` (`fm $10,$28,$22`, the final scale). So `scale = (m / 2^(k+1)) * 256 / m = 2^(7-k)`, the largest channel lands at `1.f * 2^7` in `[128, 256)`, and the stored float4 is `(rgb * 2^(7-k), k + 129) / 255` - which EDGE's output compressor quantises to bytes `(rgb * 2^(7-k), k + 129)`. The decode in every `SVC1` vertex block (`MAD R.x, v.wwww, 255, -128; EX2; MUL o.xyz, v.xyzx, R.wwww` on the hardware-normalised bytes) is then `(rgb * 2^(7-k) / 255) * 2^((k + 129) - 128) = rgb * 2^8 / 255 = rgb * 256/255`. **The round trip is exact to +0.4 %**, the `1/255` in the packer being the inverse of the `255` in the shader. `k + 129` is not off by one from textbook Radiance RGBE - it *is* textbook Radiance, whose exponent is frexp's (`k + 1`, mantissa in `[0.5, 1)`) plus 128; the previous entry's "`k + 128`" compared an IEEE exponent to a frexp one. The 2026-08-20 entry above already recorded "Radiance RGBE with the mantissa left as a `[0, 1]` fraction"; the two halves had simply never been multiplied together. Nothing here touches this thread's brightness gap.

**The combine, on `talons_junction/materials/track_surface.rcsmaterial`** (`scripts/ps3-sho.py variants` for the table, `scripts/ps3-microcode.py vp-file`/`fp-file` for the code). The `SVC1` and `SVC0` variants of every permutation **share the fragment program**: #5/#10 `HalfBright Ambient Sun Spot0` both select `fp = 0x4ab0`, #6/#11 `0x51d0`, #7/#12 `IleLightmap` `0x5930`, #8/#13 `ShadowToAlpha Ambient` `0x6100`, #9/#14 `0x6450`, and likewise across the `ZoneMode`/`ZoneTrans` rows and the `StaticQuake` class - 30 of 30 pairs in the file. The whole variant is a vertex-program substitution. The `SVC0` block #5 writes `o[TC0].xyz = 0` and `o[TC1].xyz = 0` (both from its `c[208] = (0, 2, 1, 0)` literal); its `SVC1` twin #10 declares `0x868f8229` at slot 4, carries `c[208] = (255, 128, 0, 2)`, and writes

```text
 4  MAD R2.x, v[4].wwww, c[208].xxxx, -c[208].yyyy   <- a * 255 - 128
 9  EX2 R3.w, R2.xxxx
14  MUL o[TC0].xyz, v[4].xyzx, R3.wwww               <- rgb * 2^(a*255-128) into TC0
 0  MOV o[TC1].xyz, c[208].zzzz                       <- TC1 stays 0
```

and the shared fragment program `0x4ab0` consumes it at the one place the two interpolants meet:

```text
@0x0c  MOV R4.xyz, f[TC0]                               <- the SPU light (0 under SVC0)
@0x20  DP3 R0.x, R0, {sunDir 0x02df31e5}                <- N.L
@0x26  MAD H3.xyz, R0.xxxx, {sunColour 0x2dba643d}, {ambient 0x81db67ea}
@0x28  ADD H3.xyz, R4, H3                               <- + SPU light
@0x30  TEX H2, R1 unit0                                 <- albedo
@0x3a  MAD H0.xyz, H3, H2, H0                           <- (ambient + sun*N.L + spu) * albedo + specular
@0x3d  MAD H0.xyz, R0.zzzz, H0, R1                      <- fog lerp
@0x3e  ADD H0.xyz, R2, H0                               <- + f[TC1] (0 in both variants here)
```

The `IleLightmap` family (`0x5930`, `@0x38`-`@0x39` `MOV R2.xyz, f[TC0]; ADD H1.xyz, R2, H1` ahead of `@0x4b MAD H0.xyz, H1, H0(albedo), H4`) and the `ShadowToAlpha` family (`0x6100`, `SVC1` vp #13 puts the term in `TC1` and `@0x03 ADD H2.xyz, f[TC1], {ambient}` precedes `@0x08 MAD R2.xyz, H2, H1(albedo), -bias`) do the same thing with the interpolant index moved. **So `SpuVertexColours` is a diffuse-irradiance addend: it joins the ambient, the sun's `N.L` term and the lightmap in the sum the albedo multiplies, and it never reaches the specular, the fog or the post-fog `f[TC1]` emissive add.** The 2026-08-20 entry's "`f[TC1]` is the vertex light, and it is RGBE" was reading the `ShadowToAlpha` family's own interpolant assignment, and stands.

**Across the disc** (`scripts/ps3-sho.py svc-twins`): 1,598 materials carry `SVC1`, and every one of their 27,312 distinct `SVC1` vertex blocks carries the `(255, 128)` literal and reads `0x868f8229` - the decode is uniform. Fragment sharing is a property of the compile, not of the design: `DATA00` shares the block on 12,282 of 13,026 twin pairs, the later archives' compiles mostly do not (13,038 of 33,981 disc-wide), and on the three differing pairs read by hand the twin was compiled without the term rather than with a different combine - `detonator_shield.rcsmaterial` (`RigidBody`, `DATA00`: `SVC1` fp `0xb1b0` does `@0x21 MAD H2.xyz, ndl, sun, ambient; @0x33 ADD H2.xyz, f[TC0], H2; @0x34 MUL H2.xyz, H6(albedo*tint), H2`, the `SVC0` twin `0x3b20` has no such add and uses `TC0` for its tangent instead), `materials/lambertemissive.rcsmaterial` (`DATA02`: `ADD H5.xyz, f[TC0], H2` after the same `MAD` and before `MUL H4.xyz, H5, H4(albedo * (1 - emissive))`; its `SVC0` twin omits the add and re-packs the uv into `TC0.w`), and `pvsblocker/*/emissivealpha.rcsmaterial` (`DATA00`: the `SVC1` vertex program decodes into `TC0.xyz` and the fragment program `0x25e0` never reads it - an emissive material has no diffuse sum to add to, so `SVC1` is a no-op there and the twin only re-packs `TC0`-`TC2` into `TC3`).

**Confidence 88.** The vertex side is a direct microcode read on 27,312 blocks, the packer side is read off its own constants, and the two are exact inverses - that agreement is independent corroboration of both, not one reading. The combine is read on five fragment programs across the three static lighting families, the `RigidBody` class and one later-archive compile, all placing the term in the same slot; the `SVC1`-on-emissive no-op is a sixth, on one material. What it does not cover: the 20,943 differing pairs not read by hand (the claim there is "same slot on every one read", not "on every one"), and the `Spot1`-`Spot3`/`ShadowMap`/`IBL` rows, which were not opened. **For `mesh.wgsl` this is one additive term into the pre-albedo light sum** - the slot the baked colour set and the lightmap already feed - with no new interpolant, pass or fragment restructure; what remains for wiring is the per-chunk light list itself (the `EdgeGeom` formula above, `D` of `0.6`-`2.0` units) and whether a glow that size is visible enough to carry it. No code changed.

### `EngineLightData.xml` loads to `ship+0xfc`/`+0x100`, and one of `SpuLight_AddCandidate`'s fifteen producers is read in full - the two are not yet shown to be the same mechanism (2026-09-20, confidence split: 88 for the load site and the producer's own arithmetic, unconfirmed for the link between them)

**The load site.** `Ship_LoadEngineLightData` (`0x000d25e0`, confidence 72 - the read is unambiguous, the class attribution is not) opens `Data\Ships\Zone\EngineLightData.xml` (or `%s\EngineLightData.xml` against the ship's own path, `*(ship+0x6298)+0x1b4`), finds the `EngineLightData` root, and reads two child elements by name (`scripts/ps3-toc.py resolve` on the two-TOC binary, not Ghidra's own single-TOC reading - `0x000d25e0` uses TOC `0x008ad4d8`, not the `0x008bd3c4` an earlier entry on this page resolved a different function's loads against):

```text
"Distance" -> stfs f1,0xfc(r31)     ship+0xfc  = <Distance>
"Radius"   -> stfs f1,0x100(r31)    ship+0x100 = <Radius>
```

confirmed by disassembly (`lwz r4,-0x4800(r2)` through `-0x47f8(r2)` resolve to `'EngineLightData'`/`'Distance'`/`'Radius'` at `0x00781f40`/`0x50`/`0x60`) and called from deep inside the ship's own construction routine (`FUN_000ddd58`, `0x000ddd58`-`0x000dfd8f`, unnamed - `_opd_FUN_000d25e0(param_1);` near its end, on the same `param_1` that routine spends its whole body building: `Collision_AddObject`, `RaceManager_GetInstance`, and physics-body allocation all appear in it).

**The data, across every ship.** All 36 `enginelightdata.xml` (13 base ships + `zone`, 22 `_c1`/`_n1` colour-scheme variants; `scripts/psarc.py cat` per file) carry only `Distance` and `Radius`, no colour: `Distance` ranges `-0.4` to `1.5`, `Radius` `0.7` to `2.0` - the same span the live-captured `EdgeGeom` light `D` occupies (`0.6`-`2.0`, this page's "`EdgeGeom`'s light path is read"), which is what makes this worth tracing at all.

**One producer, read in full.** `FUN_000cfb80` (one of `SpuLight_AddCandidate`'s fifteen callers via the `FUN_006778c8` thunk, this page's earlier "`Enable_spu_vertex_light`, slot 1" section) is unconditional every call:

```text
dVar9 = clamp(*(obj+0x6a7c), floor DAT_008a8b00=0.0)     // written back to obj+0x6a7c
iVar7 = FUN_00323760(*(obj+0x6adc))                       // = *(obj+0x6adc) + 0x80, trivial
position = (iVar7+0xb0).xyz                                // a transform's translation row
colour   = one-shot global literal (100.0, 50.0, 50.0)     // built once, shared by every caller
D = dVar9 * (DAT_008a8be8=3.33333 * DAT_008a8bec=40.0)     // = dVar9 * 133.332
w = DAT_008a8b14 = 1.0                                       // exact
SpuLight_AddCandidate(D, w)   // position/colour go via vs34/vs35, not the two double params
```

(constants read directly off `0x008a8b00`-`0x008a8bef`, byte-exact). **`w`'s match is the strong part**: `DAT_008a8b14 = 1.0` exactly, and every live-captured record on this page has `w = 1.0` - independent corroboration that this producer (or one sharing its constant pool) is a real contributor to the captured buffer. The colour literal is a 2:1:1 orange/amber tint, consistent with an engine glow. `FUN_00323760`'s triviality (`+0x80`, nothing else) means `*(obj+0x6adc)` is a pointer to *some* transform-bearing object - which one is not established.

**What is not established, stated plainly so it isn't assumed by the next reader:** `obj+0x6a7c`/`obj+0x6adc` (`FUN_000cfb80`'s fields) and `ship+0xfc`/`ship+0x100` (`EngineLightData.xml`'s fields) are different offsets on what may or may not even be the same object - no copy, no aliasing, and no shared caller were found connecting them this session. The `D` arithmetic (`clamp(x, 0) * 133.332`) does not support `x` being `Radius` directly: `Radius`'s own range (`0.7`-`2.0`) times `133.332` would put `D` in the hundreds, far outside the `0.6`-`2.0` this page has actually captured, so `obj+0x6a7c` is some other per-frame quantity (a decaying/animated value, given both `FUN_000cfb80` and the sibling `FUN_000e41b0` floor-clamp it to zero on every call rather than reading it once) - not `Radius` read straight through. The correlation between `EngineLightData.xml`'s `Radius` span and the live `D` span may still be real, but if so the connection runs through a scale or an intermediate write this session did not find, not through this producer's own arithmetic. Two live threads for whoever picks this up: trace what else writes `obj+0x6a7c` (the decaying value `FUN_000cfb80` and `FUN_000e41b0` both clamp), and what `*(obj+0x6adc)` actually points at (a specific node, or the object itself at some fixed sub-offset).

Confidence 88 for the load site and for `FUN_000cfb80`'s own arithmetic, each independently (direct decompile, TOC-resolved string literals, byte-exact constant reads); no confidence assigned to a causal link between them, because none was found. `Ship_LoadEngineLightData` and its evidence landed in `names.tsv`. No code changed.

### `obj+0x6a7c` and `obj+0x6adc` are traced to their setters: a depletable gameplay resource, and the ship's own currently-loaded model - neither is `EngineLightData.xml` (2026-09-20, later still, confidence 80-85)

Following on directly from the section above, both open threads it named are closed.

**`obj+0x6adc` is a synced alias of `obj+0x6ae0`, the ship's own main model.** `Ship_SyncActiveModelPointer` (`0x000db090`, confidence 80) is short and unambiguous:

```c
void Ship_SyncActiveModelPointer(int ship) {
    int model = *(int *)(ship + 0x6ae0);
    if (*(int *)(ship + 0x6adc) == model || model == 0) return;   // idempotent, skips a null model
    // ... clears a "referenced" bit (0x4) on the *old* obj+0x6ae4/+0x6ae8/+0x6aec if it was set ...
    *(int *)(ship + 0x6adc) = model;                               // the actual sync
    // ... Ship_LoadModel-adjacent housekeeping (0x000d2e30, 0x000d2d20) ...
}
```

and `obj+0x6ae0` is confirmed the ship's primary mesh by the TOC-resolved format strings its loader builds paths from: `Ship_ReloadModelForSkin` (`0x000dbdc8`, `0x000dbdc8`-`0x000ddd37`, confidence 78 - the read is clear, "ForSkin" rests on the one inference below) is a no-op if `ship+0x6950` (a model/skin identity field) already equals its new value, otherwise releases the four existing models and reloads `'%s\%s.vex'` with `'ship'` into `obj+0x6ae0`, `'%s\shipwreck.vex'` into `+0x6ae4`, `'%s\ship_lod.vex'` into `+0x6ae8`, `'%s\ship_lod1.vex'` into `+0x6aec` (all four strings and the path root `*(ship+0x6298)+0x1b4` - the same root field `Ship_LoadEngineLightData` reads - resolved via `scripts/ps3-toc.py resolve` against this function's own TOC), then calls `Ship_SyncActiveModelPointer` at its very end, having zeroed `obj+0x6adc` first. So the chain is: skin/model change -> reload `ship.vex`/wreck/two LODs -> `obj+0x6adc` synced to whichever of those is the primary model. **"ForSkin" is the one unconfirmed part**: `ship+0x6950`'s identity as specifically a *skin* selector (rather than, say, a damage-state or LOD-tier index) is inferred from the reload triggering on it changing and reloading a livery-adjacent resource set (`'livery4'`/`'Colour_Ramp'`/`leacheffect.vex` sit in the same TOC constant run), not read directly.

`FUN_00323760`'s own triviality (`return param+0x80;`, this page's previous entry) means the position `FUN_000cfb80` submits is `(active_model+0x80)+0x30` - **a fixed offset into the ship's currently-loaded mesh object**, not a distinct "engine flare" attach node. The `WARNING: SHIP MISSING ENGINE FLARE IMPORT NODE` string (`0x007824e0`) is real but its own reference resolves inside `Environment_RegisterStageSchema`, a large schema-registration function unrelated to this path - it gates a *different* system (most likely the particle-based engine flare, `%s\engineflare.vex`, this page's earlier reads), not this SPU light candidate. **The "engine flare node" hypothesis in the previous entry is retracted**: the light sits at the ship's own model transform.

**`obj+0x6a7c` is a real depletable resource, decremented externally - not `Radius`.** A byte-pattern search for `stfs fN,0x6a7c(r3)` (the `this`-in-`r3` calling shape, distinct from the `r31`-based shape `FUN_000cfb80`/`Ship_LoadEngineLightData` use) finds exactly one writer, `FUN_000e3768` (`0x000e3768`, unnamed - see below for why):

```c
void FUN_000e3768(Ship *self, float amount) {
    self->f_0x6a78 -= amount;
    self->f_0x6a7c -= amount;   // the same field FUN_000cfb80 clamps and scales into D
    self->f_0x6a88 += amount;   // a running total
    // ... a RaceManager-gated block further down, guarded on self+0x628c (a control-mode
    //     enum, 0/1/2) and calling three unnamed functions (0xa9f20/0xabc20/0xa9b18) whose
    //     shape (RaceManager lookup, a match against another object's own +0x1ec against
    //     self) reads as a pickup/trigger response, not read further this session ...
}
```

confirmed via `get_xrefs_to`/vtable read: `FUN_000e3768` is slot 6 (of at least twelve) in a per-frame subsystem-dispatch vtable at `0x00874b00`, sitting between `FUN_000e3728` and `FUN_000e3d18`, three slots before `FUN_000e41b0` - the same big per-tick function this page's earlier entry read as also clamping `obj+0x6a7c`, confirming both are methods of the one class dispatching through this table. **What this rules out plainly**: the `133.332` scale factor already ruled out `obj+0x6a7c == Radius` directly (previous entry); this section adds that it is not a static per-ship value *at all* - it is spent by an external caller through a vtable slot, at a rate this session did not trace, and the two functions that read it (`FUN_000cfb80`, `FUN_000e41b0`) only ever floor-clamp what's left, never assign a fresh value. **What it might be, stated as a hypothesis and not a finding**: the twin-pool-decrement-plus-accumulator shape and the pickup-shaped block right after are consistent with a shield/absorb/boost energy spend - this project's own prior sessions already named an "`Absorb`" family in this same address neighborhood (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "confirming the `RigidBody`/`Absorb` family's direct field-store" entry) - but no direct evidence ties `FUN_000e3768` to that name specifically, so it is left unnamed (project convention: a name needs a meaningful label, and this session cannot confidently supply one).

**Net effect on the open question**: `EngineLightData.xml`'s `Distance`/`Radius` (`ship+0xfc`/`+0x100`) and `FUN_000cfb80`'s own inputs (`obj+0x6a7c`, a depleting resource; `obj+0x6adc`, the active model pointer) are now both fully traced to their own setters, and **neither setter reads the other's field** - the correlation the previous two entries flagged (`Radius`'s `0.7`-`2.0` span matching live `D`'s `0.6`-`2.0`) has no code path connecting it found across three sessions' worth of tracing, and given how far the static reading has now gone, that is closer to a coincidence of scale than an undiscovered link. Whoever wires this next should treat `FUN_000cfb80` as an independent, order-of-magnitude-verified glow producer (position = ship model, colour = fixed literal, `D` = a spent-resource-scaled range) rather than an `EngineLightData.xml` consumer.

Confidence 80 for `Ship_SyncActiveModelPointer`'s behaviour and the position conclusion (direct decompile, corroborated by the vtable read); 78 for `Ship_ReloadModelForSkin`'s behaviour (direct decompile, TOC-resolved strings), lower on the "ForSkin" label; 80 for `FUN_000e3768`'s own arithmetic (direct decompile, one confirmed writer via exhaustive byte-pattern search), no confidence on what the resource represents. `Ship_SyncActiveModelPointer` and `Ship_ReloadModelForSkin` landed in `names.tsv`. No code changed.

### All thirteen reachable `SpuLight_AddCandidate` call sites are read at least for their light-submission shape - real, diverse, and worth wiring (2026-09-20, later still, confidence 70-85 per site, see table)

`get_function_callers(FUN_006778c8)` (the one-instruction tail-call thunk to `SpuLight_AddCandidate`) returns **thirteen** sites, not the "fifteen" the 2026-09-18 entry's producer discovery named - a discrepancy left open below. Every one is read far enough to know what shape of light it submits: whether position/colour are per-object or fixed, and whether `D`/`w` are computed, read from a table, or literal. None of the thirteen was wired into `mesh.wgsl` this session - this entry is the survey the previous entry's "name the other thirteen" step asked for, not the implementation.

| Site | Position | Colour | `D` | `w` | Read as |
| --- | --- | --- | --- | --- | --- |
| `FUN_000cfb80` | ship's own model transform (`Ship_SyncActiveModelPointer`) | shared literal `(100, 50, 50)` | `clamp(obj+0x6a7c, 0) * 133.332` - a depleting resource, one writer `FUN_000e3768` | fixed `1.0` | ship glow, full chain traced (previous two entries) |
| `FUN_000e41b0` | same fields as above | same | same | same | a second, RaceManager-gated call site for the *same* ship light - not a distinct producer |
| `FUN_00122098` | four wheel/contact transforms (`param+0x140/0x150/0x160/0x170`) | not read (truncated past the light call) | a per-object struct `*(puVar9+0x1c)` | `*(puVar9+0x20)`, same struct | speed-gated (only above a threshold read from `param+0x1a8`); shares a decal-ring-buffer write (`+0x630`/`+0x634`, a skid-mark/tyre-mark system) with the site below - **collision/skid spark**, read as a hypothesis |
| `FUN_001310e8` | four transforms, same shape as `FUN_00122098` (`param+0x230` family) | not read | fixed global `DAT_008aa6c0` | fixed global `_DAT_008aa69c` | sibling of `FUN_00122098`, same decal-ring-buffer and speed-gate pattern, fixed rather than per-object `D`/`w` - likely a second wheel/axle of the same mechanic |
| `FUN_00123de8` | a transform at `param+0x90` | one-shot literal `(7, 5, 1)` | fixed global `DAT_008aa124` | fixed global `DAT_008aa128` | simplest read: two constants and one transform, class of `param_1` not identified |
| `FUN_0013b300` | **two** independent lights: `param+0xa0` (gated on `param+0x5c != 0`) and `param+0xe0` (gated on `param+0x60 != 0`) | not read | each from its own small two-float struct (`*PTR_DAT_008aaa18`, and `pfVar3 = puVar7[-0xab0]`) | paired with its own `D` in the same struct | two independent emitter slots on one object - a twin-mount shape (twin engines, twin weapon hardpoints) |
| `FUN_0014a7d8` | **caller-supplied** - `param_2`'s own transform, not read from `param_1` | one-shot literal `(7, 5, 1)`, same as `FUN_00123de8` | TOC-relative constant floats (a settings-table entry, not chased to the table itself) | ditto | a generic "submit at this transform" utility - involves `RaceManager_GetInstance` and a debug/telemetry string, so likely dispatched by more than one caller (not enumerated) |
| `FUN_0014f928` | **not set here** - inherits whatever the caller left in `vs34`/`vs35` | ditto | two variants of one authored curve over `param+0x2dc`: `-(field * k - k) + b` | same curve, second scale | two closely related lights off one normalized field (health/charge/heat) - a "the fuller/hotter this is, the bigger the glow" shape |
| `FUN_001547c8` | not set here (caller-supplied, like `FUN_0014f928`) | one-shot literal `(500, 200, 50)` - an order of magnitude brighter than the ship's `(100, 50, 50)` | `param+0x17c * DAT_008ab258` | `param+0x17c * DAT_008ab25c + DAT_008ab260` | linear in one field, very bright - a flash/pickup/impact shape rather than an ambient glow |
| `FUN_00155568` | `param_2+0x30` transform, explicit | same `(500, 200, 50)` literal as `FUN_001547c8` | same formula shape, different field | same | embedded in a much larger routine building a 16-slot per-racer table with a name-hash/leaderboard shape (`RaceManager_GetInstance`, a 16-iteration loop, string-length hashing) - the light call itself reads as `FUN_001547c8`'s sibling, the surrounding table logic not chased |
| `FUN_00127468` | `WeaponExplosions_Draw`-adjacent, an explosion-slot transform indexed by parity | not read | fixed globals (`DAT_008aa28c`/`DAT_008aa290`) | ditto | explosion-size-gated (only when `param+0x50 <= DAT_008aa25c`); read in the 2026-09-18 session, listed here for completeness |
| `FUN_00108c48` | `FUN_00297790()`'s return `+0x30` - an accessor called with *no* argument first (likely "the current/active X") | a **fixed global**, dereferenced (`*(global ptr)`), not per-object | TOC constant | TOC constant | double-gated (`*(param+0x134)+0xd0 != 0` and a global registry count `!= 0`); the decompile truncates just after the light call into more vector math this session didn't finish reading - a second light or extra state may follow |
| `FUN_00115028` | **caller-supplied via a raw pointer argument** (`*(param)`, not `param+k`) | fixed global, dereferenced | fixed global | fixed global | its one caller, `FUN_00114c78`, is a **track-level** producer: a loop that spawns markers at regular intervals along a measured distance (`RaceManager_GetInstance`, a `0x4541424c`-tagged debug message, per-interval object construction via `FUN_0013c8e8`) and submits one light per interval - the first non-ship-attached, non-transform-object producer found this session, consistent with sector/gate/pad markers spaced along the track rather than carried by an object |

**What this settles.** The mechanism is not a single ship-glow special case: it is used by at least a collision/impact system (two sites sharing a decal-ring-buffer), a weapon-explosion system, a track-interval marker system, a twin-emitter object, two curve-driven "fuller/hotter is bigger" shapes, and a double-gated highlight of some kind - real diversity, all landing in the same well-understood pipeline (`SpuLight_AddCandidate` -> `SpuLight_CompactVisibleCandidates` -> `LightCulling` -> `EdgeGeom`'s per-vertex loop -> the `SVC1` combine, all previously closed on this page). That is the basis for treating the mechanism as worth wiring on its own terms, independent of any single producer's exact identity.

**What is not settled, and blocks a faithful per-producer port today:** the object class behind most `param_1`s (only the ship's is identified), the exact resource behind `FUN_000e3768`'s decrement, the settings-table entries several sites read via TOC-relative constants (never chased to their source table), and the "fifteen vs thirteen" count - either two producers reach the thunk through a path `get_function_callers` does not see (an indirect/virtual call, or a second thunk), or the original count from the 2026-09-18 session was itself approximate. None of this blocks implementing the **mechanism** (candidate list, culling, per-vertex accumulate, RGBE pack/decode, the `mesh.wgsl` additive term - all independently confirmed on this page already); it blocks porting any *specific* producer faithfully until that producer's own inputs are identified as concretely as the ship's were.

Confidence 82 for the table's structural claims (thirteen call sites, each site's own read of what feeds `SpuLight_AddCandidate`'s scalar/vector arguments - direct decompile and disassembly throughout); lower, noted per-row, for what each producer represents. No renames this entry - none of the thirteen reaches this session's naming bar (a meaningful subsystem label backed by identified class), so all stay as raw `FUN_` addresses rather than guessed names. No code changed.

### Thirteen is exhaustive, and the numbers rule out every one of them as the source of the captured buffer - the 128-slot candidate list and the live-captured 8-slot buffer are very likely different data (2026-09-20, later still, confidence 82)

**Thirteen is complete, not partial.** `get_xrefs_to(SpuLight_AddCandidate)` (`0x0040d990`) shows exactly two references: its own `.opd` entry and the single `UNCONDITIONAL_CALL` inside `FUN_006778c8`, the thunk. There is no second path - every caller reaches `SpuLight_AddCandidate` through that one thunk, and `get_function_callers` on the thunk already enumerated all of them. **"Fifteen" was approximate; thirteen is the real, exhaustive count.**

**Every producer's own `D` is now read**, following up on the previous entry's open "colour: not read" cells with a direct disassembly pass (`FUN_00122098`, `FUN_001310e8`, `FUN_0013b300`) and TOC resolution of the constant-table producers:

| Site | `D` | vs. captured `0.6`-`2.0` |
| --- | --- | --- |
| `FUN_00122098` | `70.0`, fixed (a per-caller struct, but the struct's own value is a constant) | 35-116x too large |
| `FUN_001310e8` | `10.0`, fixed | 5-16x too large |
| `FUN_0013b300` (both of its two lights) | `50.0`, fixed | 25-83x too large |
| `FUN_00123de8` | `50.0`, fixed | 25-83x too large |
| `FUN_0014a7d8` | `100.0`, fixed | 50-166x too large |
| `FUN_00108c48` | `20.0`, fixed | 10-33x too large |
| `FUN_000cfb80` | `clamp(field, 0) * 133.332` - the one producer whose `D` *could* land in range, if the field is `~0.0045`-`0.015` | unverified - the field's own range is unread |

`FUN_0014f928` and `FUN_001547c8`/`FUN_00155568` compute `D` from a per-object field too (not reduced to a number here), so they are not ruled out the same direct way, but their **colour** is: every literal catalogued in the previous entry - `(100, 50, 50)`, `(7, 5, 1)`, `(500, 200, 50)` - and every colour source checked this entry (`FUN_00122098`, `FUN_001310e8`, `FUN_0013b300`: all a fixed TOC-relative read, not computed) is a **fixed ratio**. The five captured records this page has read (`data/traces/hd-spu-light-companion/`) show colour at a **constant `1 : 0.25 : 0.1` ratio across varying magnitudes (`40` to `440`)** - a computed, per-instance value, not a literal. No producer's colour matches that ratio: `(100,50,50)` is `1:0.5:0.5`, `(7,5,1)` is `1:0.71:0.14`, `(500,200,50)` is `1:0.4:0.1`. None is close, and a literal cannot vary in magnitude at all, which the captured records do.

**Read together: no producer catalogued this session matches the captured buffer on both `D` scale and colour ratio, and the ones that could not be ruled out on `D` (curve-computed) already fail on colour, or vice versa.** This is the direct test of the open question the 2026-09-18 session named and left unresolved ("whether this 128-slot candidate list is what later gets selected down into the 8-slot buffer already live-captured is not established") - **the evidence now leans the other way.** Not certain: `FUN_000cfb80`'s field is still unread, so it is not formally excluded, only unverified: even if it separately matches on `D`, its colour (`(100, 50, 50)`, ratio `1:0.5:0.5`) does not match the captured `1:0.25:0.1` ratio, which *does* exclude it on the colour axis - the cleanest exclusion of any single site, because colour there is a checked literal, not an open field.

**What this means for wiring**: none of the thirteen can be ported today without either inventing an unread value (the fields behind `FUN_000cfb80`'s and the two curve-driven sites' own `D`) or wiring a value this page's own cross-check just showed does not match the only ground truth available. That is not a stall on effort - the producer search is now exhaustive and every number in it is read - it is a finding: **the captured 8-slot buffer most likely comes from somewhere other than these thirteen gameplay call sites**, and locating that source (or a fourteenth path this session's static reading cannot see, e.g. an SPU-side write bypassing the PPU candidate list entirely) is what the next session should chase before wiring anything into `mesh.wgsl`. The mechanism itself (culling, `EdgeGeom`'s per-vertex loop, the RGBE pack and the `SVC1` combine) stays fully confirmed and unaffected by this - what is now in question is only which runtime data feeds it.

Confidence 82: the exhaustiveness claim is a direct, two-hit xref query (as strong as this kind of check gets); the `D` values are direct TOC-resolved constant reads, byte-exact; the colour-ratio mismatch is arithmetic on already-captured, already-committed data. Confidence stays lower on the conclusion itself ("different data") since it is an inference from ten data points, not a trace of the 8-slot buffer's own write site. No code changed.

### The captured buffer's producer is found: `EngineFlare_SubmitSpuLight` (`0x0029ff28`), one of eighteen call sites Ghidra could not see - the "thirteen is exhaustive" negative result is retracted, and the pipeline is verbatim end to end (2026-09-20, static lane, confidence 85)

This section resolves the contradiction the previous entry left standing: a confidence-88 traced write (`SpuLight_CompactVisibleCandidates` copies the 128-slot list into the buffer `scripts/rpcs3-spu-light-dump.py` captured) against a confidence-82 numeric exclusion of every producer. **The exclusion was built on an incomplete producer list, not on a transform.** Every question the lane was asked is answered below, in order, and the answers to the first three are all "no transform anywhere".

**0. `w` per call site.** All the previously catalogued sites that pass a constant pass `w = 1.0` byte-exact (`0x3f800000` at `0x008a8b14`, `0x008a9cac`, `0x008a9890`, `0x008aa128`, `0x008aa290`, `0x008aa69c`, `0x008aaf68`, `0x008c1890`, `0x008c1a54` - each resolved with `scripts/ps3-toc.py resolve` against the site's own TOC, `0x008ad4d8` for every one). Three sites compute `w` and can never produce `1.0` with a positive `D`: `FUN_0014f928` (both lights: `w = 7.0 * (1 - field) + 1.5`, constants `0x008ab0e0 = 0x40e00000` and `0x008ab0ec = 0x3fc00000`), `FUN_001547c8` and `FUN_00155568` (`w = 7.0 * field + 1.5`, `0x008ab25c`/`0x008ab260`, `D = field * 150.0`, `0x008ab258 = 0x43160000`). Since every captured record has `w = 1.0`, those three are excluded on `w` alone - the buffer is a subset view of the list in exactly the sense the brief anticipated, but the subset argument turned out not to be what matters.

**1. `SpuLight_AddCandidate` stores its arguments verbatim.** Disassembly `0x0040da38`-`0x0040dab0`: `stfs f31,0xc(r10)` puts `f2` at `record[3]`; the six `vspltw`s (`0x0040d9dc`-`0x0040d9f8`) splat `v2` (`v31`, saved at `0x0040d9b0`) words 0/1/2 into `record[0..2]` and `v3` (`v30`, saved at `0x0040d9a0`) words 0/1/2 into stack `0xd0`/`0xd4`/`0xd8`, `stfs f30,0xdc(r1)` (`f30 = f1`, `0x0040d9b8`) completes the second quadword, `stvx v0,r10,r11` (`r11 = 0x10`) lands it at `record+0x10`. So `record = (v2.x, v2.y, v2.z, f2, v3.x, v3.y, v3.z, f1)`: **position in `v2`, `w` in `f2`, colour in `v3`, `D` in `f1`**, no scale, no inversion, no intensity fold, no global multiply. The decompiler's `in_vs34._0_4_ | in_vs34._8_4_` is its rendering of the splat-through-stack idiom, not an `or`. Confidence 90 (mechanical read of a 60-instruction function).

**2. `SpuLight_CompactVisibleCandidates` copies verbatim.** `0x0040d7f8`-`0x0040d81c` is four `ld`/`std` pairs - 32 bytes, unmodified. The only arithmetic is the cull sphere `(x, y, z, record[7])` built by the two `vsel`s for `0x005bd0d8` and discarded. Nothing is written into a record field. Confidence 90. Together with (1): **any magnitude variation in a captured record is the producer's own** - the previous entry's "authored colour times a per-instance scalar" reading at this page's `+0x2084` section can only be true if a producer computes it, which is exactly what the producer below does.

**3. `FUN_000cfb80`'s literal is `(100, 50, 50, 50)`, byte-exact.** `0x000cfb90 lis r9,0x4248` (`50.0`), `0x000cfbc4 lis r0,0x42c8` (`100.0`), stores to stack `0x70`/`0x74`/`0x78`/`0x7c` as `100, 50, 50, 50`, `lvx` and cached at `*(-0x49fc(r2)) + 0x30` = `0x0098d7c0 + 0x30 = 0x0098d7f0` under a once-flag at `0x0098d7e0`; `FUN_000e41b0` (`0x000e4790`-`0x000e47b8`) initialises the same slot with the same four words. `get_xrefs_to 0x0098d7f0` lists no other real writer (the `FUN_0040e318` hit is a mis-resolved `stvx` against `PTR_DAT_008b83e0+0x240`, a different base). `D = clamp(obj+0x6a7c, 0) * (0x40555555 = 3.3333 * 0x42200000 = 40.0)`, `w = 0x3f800000` - all three confirmed. So the previous entry's reading of this producer stands, and it is *not* the buffer's source: `1:0.5:0.5` is not `1:0.25:0.1`, and per (1)/(2) nothing downstream could change that ratio. **The `+0x2084` section's "`(1.0, 0.25, 0.1)` times an intensity" reading was right about the shape and wrong about which producer** - see (6).

**4. The three unread colours, now read.** `FUN_0014f928`: light 1 colour `(500.0, 100.0 * (field + 1), 50.0)` (`0x0014f970 lis 0x43fa`, `fmadds` with `0x008ab0e4 = 0x42c80000`), `D = *(0x008c1aa8) * 40.0 = 40.0`; light 2 colour `(20.0, 5.0, 0.5)` (`0x0014f9b0`/`0x0014f9c4`/`0x0014f9b4`), `D = 100.0`; both `w = 7 (1 - field) + 1.5`. The previous table's "not set here - inherits `vs35`" was wrong: only the *position* (`v2`, saved in `v31`) is inherited, the colour is built in-function. `FUN_001547c8` and `FUN_00155568`: colour `(500, 200, 50)` (`0x001547f4`/`0x001547f8`, `0x0015598c`/`0x00155990`), `D = field * 150`, `w = 7 field + 1.5`. None is `1:0.25:0.1`, and all three are already excluded on `w`.

Every "fixed global, dereferenced" colour is also now read, and each has exactly one writer, a `(1, 0xffff)` static initialiser (GCC's `__static_initialization_and_destruction_0(1, 65535)` shape, reached from the `.ctors` table at `0x008600a0`): `FUN_00115028` reads `0x00993df0 = (0.21, 0.30, 1.0, 1.0)` written by `FUN_00114fc0` (`0x008a9c9c = 0x3e570a3d`, `0x008a9ca0 = 0x3e99999a`), `D = *(0x008c1848) = 15.0`; `FUN_00108c48` reads `*(0x009208c8) -> 0x00993020 = (1.0, 0, 0, 0)` written by `FUN_00108a70`, `D = 20.0`; `FUN_00122098`'s first site reads `0x00993fa0 = (0.0, 0.003, 0.03, 0.03)` written by `FUN_00121a38`, `D = *(0x008c188c) = 70.0`; `FUN_001310e8`'s first site reads `0x009943e0 = (0.3, 0.3, 0.2, 0.2)` written by `FUN_00130990`, `D = 10.0`; `FUN_0013b300` reads `0x00994490 = (0.0, 5.0, 10.0, 10.0)` written by `FUN_0013adf8`, `D = 50.0`; `FUN_00127468` builds `(14.0, 10.0, 14.0)` inline (`0x001274ac`/`0x00127560`), `D = 100.0`. The "runtime-written global" hole in the previous entry is closed: none of them is.

**5. `FUN_000cfb80`'s field range is moot.** Its colour excludes it regardless of `D` (see 3), so `obj+0x6a7c`'s range was not traced further. `w` is `f2 = lfs -0x49c4(r2)` = `0x008a8b14 = 0x3f800000`.

**6. The list of callers was wrong, and the real producer was in the gap.** A byte scan of the ELF's executable segments for every `bl` whose target is `0x006778c8` (the thunk) or `0x0040d990` finds **33 call sites in 24 functions**, against the 15 sites in 13 functions `get_function_callers`/`get_xrefs_to` return. The 18 Ghidra cannot see - `0x0011ea34`, `0x0011ea8c`, `0x0011fae8`, `0x001216cc`, `0x001225b4`, `0x001225e8`, `0x001246cc`, `0x00126078`, `0x00131454`, `0x0013636c`, `0x001363f0`, `0x001401f8`, `0x001514bc`, `0x001514f8`, `0x00155540`, `0x002a0198`, `0x002d139c`, `0x002d146c` - all sit past an `lvlx` in their function, the Cell instruction stock Ghidra's sleigh lacks, so the disassembler halts before reaching them (`docs/reverse-engineering/toolchain.md`, "Some Cell vector instructions are missing"; the open instance is the stock-language import, 26,100 functions). No `b` (tail call) reaches either address except the thunk's own, no data word anywhere in the file holds the `.opd` entry `0x0088cb28` or the thunk's address, so **33 direct `bl` sites is the exhaustive count for this ELF** (scope: every word of every `PT_LOAD` `PF_X` segment decoded as a PPC `b`/`bl`, and every word of every `PT_LOAD` segment compared against the two addresses - a fourteenth path would have to be a `bctrl` through a computed register or code outside the ELF, neither seen anywhere in this cluster); the previous entry's "thirteen is exhaustive (two-hit xref query)" is retracted - the query was exhaustive over what Ghidra had disassembled, which was not the binary. Two of the eighteen are inside functions already in the table (`FUN_00122098` has three sites, `FUN_001310e8` two), so even the catalogued producers were undercounted. Reproduce with the scan in `data/scratch/spu-light-static-report.md` or `scripts/scan-ps3-cell-vector-ops.py`'s method; the one-liner is: decode every 4-byte word in the `PF_X` segments as PPC, keep primary opcode 18 with `LK = 1`, compare the sign-extended `LI` target.

**The producer.** `0x002a0198` is inside `0x0029ff28`, which Ghidra decompiles as two lines and `halt_baddata()` because of `0x0029ffc0 lvlx v0,0,r29` (`0x7c00ec0e`, XO 519). Called once per enqueue, unconditionally, from `EngineFlare_Enqueue` (`0x002a077c`), so once per ship per frame. Read in full off `disassemble_bytes` (which does decode past the hole once pointed at it):

```text
EngineFlare_SubmitSpuLight(this)                               0x0029ff28
  ship   = *(this+0x134)                                       0029ff38
  radius = *(ship+0x100)                                       0029ff64  lfs f31,0x4(r29), r29 = ship+0xfc
  if (*(this+0x34) & 0x1000) FUN_00323aa0(this)                 refresh transform, re-read ship
  node   = *(this+0x38)          the flare node's 4x4: row2 +0x20 = its Z axis, row3 +0x30 = position
  dist   = *(ship+0xfc)                                        0029ffc0  lvlx v0,0,r29 -> v0.x, vspltw
  blend  = *(this+0x144) + *(this+0x150)                       0029ffd0-d8   the boost snap + decay
  anchor = node.row3 - node.row2 * dist                        0029ffdc-e0   behind the nozzle, along Z
  base   = *(ship+0x7d2c) ? (40, 10, 4) : (4, 10, 40)          0029fff4-2a0010 / 2a00a4-2a00c0
  t      = *(this+0x240)                                       0029ffec
  if (t <= 0.0 [0x008b2ef0] && *(this+0x2b4) == 0)             2a0018 / 2a01e0
      colour = base * (blend * 10.0 + 1.0)                     2a01ec-2a0218  (r10 = 0x4120, 0x008b2ef4)
  else, t = clamp(t, 0, 1) [0x008b2ef0, 0x008b2ef4]:
      t >= 1: colour = (base.x + 40.0, 10, 0)                  2a00e8-2a010c  (0x0079bd10 = 40.0)
      else:   colour = (4 + 40 t, 10, 40 (1 - t))              2a0038-2a0090 / 2a0230-2a0260
  off    = *(this+0x250)            vec4, rewritten every EngineFlare_Update tick:
                                    (U(-0.1,0.1), U(-0.1,0.1), 0, U(-0.1,0.1))   002a33a0-002a340c,
                                    ranges 0x008c23cc/0x008c23d0 = 0x3dcccccd, FUN_0028c660 = uniform RNG
  position = anchor + off.xyz                                  2a0110-2a0194
  D      = radius + off.w                                      2a0180  fadds f1,f31,f1
  w      = 1.0                                                 2a0114  lfs f2,0x5a1c(r2) = 0x008b2ef4
  SpuLight_AddCandidate(D, w, position, colour)                2a0198
```

`ship+0x7d2c` is the Fury-skin byte `Ship_SetFuryTrailFlag` writes (engine-trail.md, "The red trail is the Fury skin"); `+0x144`/`+0x150` are exactly the sum `EngineFlare_PlaceShapes` scales `EF_Main`/`EF_Boost` by - the boost blend (snapped to `1.0` above the gate, decayed `* 0.8` per substep) plus the slower afterburner blend (engine-flare.md, engine-trail.md "`EngineFlare_PlaceShapes` then scales") - so whatever drives the plume's size drives the light's intensity with it, and `(40, 10, 4)` exactly is both terms at rest; `*(this+0x38)+0x20` is the flare node's Z axis `EngineFlare_RenderTick` already uses. `+0x240` is a countdown (`EngineFlare_Update` decrements it at `0x002a33e4`-`0x002a3410`); what arms it, and what `+0x2b4` is, are not read - they select a blue-to-orange transition tint whose numbers are above, and the capture never showed it firing on the normal path.

**Every number in the live capture is this function's.** `data/traces/hd-spu-light-companion/s*_slot.bin`, re-read this session from the raw bytes: all 40 records have `w = 1.0`; 21 of them are exactly `(40, 10, 4)` and 7 more within `0.02` of it (Fury skin, `blend` at or near `0`; Amphiseum is a Fury event); the rest are `(40, 10, 4) * k` with `k = 1 + 10 blend` - `44.61 = 40 * 1.115`, `82.95 = 40 * 2.07`, `171.07 = 40 * 4.28`, `244.8 = 40 * 6.12`, `360.0 = 40 * 9.0`, `440.0 = 40 * 11.0` (the snap, `blend = 1.0`), and the near-exact ones (`40.003`, `40.006`, `40.017`, `40.020`, `40.254`) are the `* 0.8`-per-substep decay's tail, not noise; the ratio is `1 : 0.25 : 0.1` on every one because it is the literal's own ratio. `D` spans `0.622`-`2.053`, which is `EngineLightData.xml`'s `Radius` (`0.7`-`2.0`, this page's load-site entry) `± 0.1` of jitter, and each ship's `D` wanders around its own centre frame to frame - the "stays stable for a given position" observation. Seven to eight live records per frame is the grid. The `(80, 10, 0)` records the slot-1 capture called anomalous are the `t >= 1` branch (`40 + 40, 10, 0`), so the transition tint *was* observed once, on a different boot. The one record that is still not this producer is slot 1's `D = 10.0, (2.0, 2.0, 0.4)`: `D = 10.0`, `w = 1.0` matches `FUN_001310e8`'s first site exactly, whose colour static-initialises to `(0.3, 0.3, 0.2)` - same shape (r = g > b), different magnitude, so either that `0x2d90`-tagged block is reloaded from data at runtime or a second `D = 10` producer sits among the sixteen unread gap sites. Open, and minor.

**The correlation three sessions could not find a code path for was real**: `Ship_LoadEngineLightData`'s `Distance` places the light behind the nozzle along the flare node's Z, and its `Radius` is the light's range. The path was never going to be found from the ship side because the consumer is the flare object holding a back-pointer to the ship, and never from the callers side because Ghidra had lost it behind an `lvlx`.

**The other seventeen gap sites, scanned for their `bl`-adjacent constants only** (`lfs f1/f2,d(r2)` and `lis` float immediates in the sixty instructions before each call; not full reads): `0x0011e640` (two sites, `D = 30.0` at `0x008a9f14`, `w = f31` computed); `0x0011f810` (`D = 24.0`, `w = 1.0`); `0x00121418` (`D = 15.0`, `w = 1.0`); `FUN_00122098` sites 2/3 (`w = 1.0` at `0x008aa080`, `D` from a struct); `0x00123fb0` (`D = 100.0`, `w = 1.0`, colour with `14`/`10` literals); `0x00125d98` (`D = 5.0`, `w = 1.0`); `FUN_001310e8` site 2 (`D = 5.0`, `w = 1.0`); `0x00136160` (two sites, no constants in range - struct-fed); `0x00140078` (struct-fed); `0x001512f8` (two sites, an inline twin of `FUN_0014f928` - same `0x008ab0e0`/`0x008ab0ec` curve, `D = 40`/`100`, so excluded on `w`); `0x00155420` (twin of `FUN_001547c8`, `w = 7 field + 1.5`); `0x002d1208` (two sites, `D = 100.0` with `30`/`6` literals, `D = 23.0` with `700`/`50` literals, `w = 1.0` at `0x008b3c8c`). None is `D` in `0.6`-`2.0` where `D` was readable; none needs reading to close this thread.

**Retractions, stated as such**: (a) the previous entry's "thirteen is exhaustive" and everything inferred from it - the buffer *is* the compacted candidate list, as the confidence-88 trace said; (b) the same entry's "no producer's colour matches ... a literal cannot vary in magnitude" - the literal is scaled by the boost blend in the producer, before the verbatim store; (c) this page's earlier "EngineLightData.xml's Radius correlation has no code path" - it has one, `EngineFlare_SubmitSpuLight`; (d) the "fifteen vs thirteen" discrepancy was 15 call sites in 13 functions, both counts correct and both wrong.

**Confidence 85** for `EngineFlare_SubmitSpuLight`: hand disassembly of the whole function with every constant TOC-resolved and byte-exact, its caller read, and the live capture matching it on colour literal, ratio, scaling law, `w`, `D` range and record count; short of 90 only because the transition-tint arming (`+0x240`/`+0x2b4`) is unread and no breakpoint confirmed the call in the emulator. `EngineFlare_SubmitSpuLight` lands in `names.tsv`. No code changed - what `mesh.wgsl` would add is now fully specified from this page: per ship, one light at `flare_node.pos - flare_node.z * Distance + jitter`, range `Radius + jitter`, exponent `1.0`, colour `(40,10,4)` or `(4,10,40)` by skin, times `1 + 10 * boost_blend`.

### 2026-09-20, live lane: the "very likely different data" conclusion above is overturned - a live, same-instant capture of both structures finds byte-for-byte verbatim copies from the candidate list in the compacted visible buffer (confidence 88)

**Method.** `scripts/rpcs3-spu-light-candidates.py` (new, a copy of `rpcs3-spu-light-dump.py` extended rather than edited) reads, in one paused GDB-stub stop, all four structures the two prior sections above only ever read separately: `+0x2098` (candidate count) and up to 128 candidate records at `+0x20a0`, `+0x2080` (current index), `+0x2084[0]`/`+0x2084[1]` (**both** slots' own frustum-survivor counts, not just the current index's), and both compacted double-buffer slots' own records at `+0x80`/`+0x1080`. Five snapshots, Amphiseum, default play, ~2s apart - the same shape `rpcs3-spu-light-dump.py` and the earlier `data/traces/hd-spu-light-companion/` capture already used. Raw bytes and decoded JSON under `data/traces/hd-spu-light-candidates/` (gitignored `data/`, not committed).

**Finding 1: exact, byte-for-byte matches exist between the candidate list and the compacted visible buffer, read at the same paused instant.** Snapshot `s1` has one match (candidate list vs. both slots); `s4` has seven (three in slot 0, four in slot 1). Verified at the **raw `.bin` byte level**, not the decoded JSON: candidate record `1` in `s4_candidates.bin` and slot-1 record `1` in `s4_slot1.bin` are the identical 32-byte string `434d2f8fc20a8c62434b8f8f3f80000040000000400000003ecccccd41200000` - `(x=205.186, y=-34.637, z=203.561, w=1.0, r=2.0, g=2.0, b=0.4, D=10.0)`. This is the direct test the previous section's own close asked for ("locating that source... is what the next session should chase"), and the answer for at least this producer is **the candidate list is the buffer's source, unmodified** - the previous section's "very likely different data" does not hold as a general claim.

**Finding 2: the matching family is a second, distinct light type the earlier single capture never happened to catch, and it resolves a previously-unread fixed colour global.** All nineteen `D=10.0` candidate records across the five snapshots share `w=1.0`, colour `(2.0, 2.0, 0.4)` exactly (ratio `1:1:0.2`) and `D=10.0` exactly - no variance at all, consistent with fixed-global fields rather than a computed quantity. `10.0` is the exact value the previous section's own table already assigned to `FUN_001310e8` ("`10.0`, fixed... 5-16x too large" against the *old* capture's `0.6`-`2.0` window) - its colour was listed there as "fixed global `DAT_008aa6c0`", unresolved. This live read gives that global's actual value for the first time: `(2.0, 2.0, 0.4)`. The family is **absent from `s0`'s candidate list and present in `s1`-`s4`**, and its position moves steadily across those four snapshots (`(162.6,-37.9,187.9)` -> `(175.4,-37.0,192.6)` -> `(190.4,-35.8,198.1)` -> `(205.2,-34.6,203.6)`, roughly 15-17 units every ~2s) - consistent with the previous section's own "speed-gated... wheel/axle" read of `FUN_00122098`/`FUN_001310e8`'s shared shape (a light that only exists once some object crosses a speed threshold, and tracks a fast-moving object's position) rather than a static per-track fixture. Attribution to `FUN_001310e8` specifically rests on the exact `D` match plus this behavioural fit, not a live breakpoint read - confidence 75, not 88, on the identity.

**Finding 3: the compacted visible buffer's own `D` is not structurally bounded to `0.6`-`2.0`.** That range, from the single earlier capture (`data/traces/hd-spu-light-companion/`), was this session's own working assumption for "does it match the buffer" comparisons across this page's last three sections. This capture's compacted buffer (both slots, multiple snapshots) carries `D=10.0` records that passed frustum culling exactly like the `D~1` records do - `0.6`-`2.0` was the earlier five-snapshot sample's own range, not a property of the buffer itself.

**Finding 4: the dominant "orange" family (`1:0.25:0.1` ratio) is present in every snapshot's candidate list, closely resembles the documented visible-buffer range, and still did not byte-match this session** - 26 candidate-list records across all five snapshots, magnitude `40.00`-`440.00` (matching the earlier capture's own `40`-`440` almost exactly) and `D` `0.901`-`2.003` (matching its `0.6`-`2.0`), but no exact match against either slot in any snapshot. Read together with Finding 1, this is not a second negative result - it is the expected shape of "same pipeline, different tick": this family's own position and `D` are recomputed fresh on every call (this page's own `FUN_000cfb80`/`EngineLightData.xml` sections already establish `D` as a continuously-updated, spent-resource-scaled quantity for at least one producer of this shape), so catching an exact match needs the pause to land inside the single tick where a fresh submission and the next compaction have not yet diverged - true by construction for the fixed-field `D=10.0` family, generally false for a family whose fields change every call. `s0`'s own near-misses show this directly: candidate and slot positions/`D`s differ by amounts consistent with one tick's worth of drift (e.g. `D=0.996` in the candidate list vs. `D=1.062` in the current slot for what reads as the same light), not a different value entirely.

| Family | Colour | Ratio | `D` | Behaviour | Verbatim match this session |
| --- | --- | --- | --- | --- | --- |
| "orange" | `40.00`-`440.00` magnitude | `1 : 0.25 : 0.1` | `0.901`-`2.003` | present every snapshot, values drift tick to tick | no (expected - see Finding 4) |
| `D=10` ("wheel/skid"?) | fixed `(2.0, 2.0, 0.4)` | `1 : 1 : 0.2` | fixed `10.0` | absent `s0`, present `s1`-`s4`, position tracks a moving object | **yes**, `s1` and `s4`, both slots |

**Confidence**: 88 for the verbatim-copy finding itself (direct byte-for-byte read of a known, static, non-relocated pointer, reproduced across two independent snapshots and both buffer slots - the strongest evidence tier this page's own rubric has for a live capture); 75 for the `D=10` family's identity as `FUN_001310e8` (exact `D` match plus behavioural fit, not a live PPU-side attribution read - see below); 80 for Finding 4's "same pipeline, different tick" account of the orange family's non-match (consistent with everything already established about that producer's `D` being continuously recomputed, not itself directly observed mid-recomputation this session).

**Net effect on the previous two sections**: "the 128-slot candidate list is probably not the 8-slot buffer's source" is withdrawn as a general claim - it is now confirmed, live, to be the literal and unmodified source for at least one producer. What is still open: byte-for-byte confirmation for the orange family specifically (its ranges match closely but a tick-boundary artefact prevented an exact catch this session), and full attribution of *which* of the thirteen call sites produces the orange family - unresolved by this capture, since none of the orange-family candidate records could be matched to a specific caller without a live breakpoint read (below). No code changed; `scripts/rpcs3-spu-light-candidates.py` is new, `scripts/rpcs3-spu-light-dump.py` is untouched.

### 2026-09-20, live lane, continued: the orange family's true call site is found, live, and it is not one of the thirteen (confidence 90)

**Method.** `scripts/rpcs3-spu-light-candidates.py attribute` re-boots with `PPU Decoder: Interpreter (static)` (required for `Z0` breakpoints to fire at all - `docs/reverse-engineering/rpcs3-debugger.md`) and arms a breakpoint directly at `SpuLight_AddCandidate`'s own entry (`0x0040d990`), not the thunk - the thunk tail-calls without pushing a frame, so `LR` read at the callee's entry is already the real caller's own return address. Per hit: `LR`, the two float args `f1`/`f2` (`SpuLight_AddCandidate(D, w)`, per this page's own read of the call), then the entry breakpoint is swapped for a one-shot breakpoint at `LR` itself so that, by the time *that* fires, the call has returned and `+0x2098`'s new count together with the record at `+0x20a0 + (count-1)*0x20` is unambiguously this call's own output. **First attempt failed and was diagnosed, not retried blind**: using `Debugger.wait_at()` directly against an active breakpoint hung `qfThreadInfo` on a genuine socket timeout after the very first hit - `wait_at()` never calls `drain()`, and `SpuLight_AddCandidate` is called many times a frame, so the breakpoint queued its own unsolicited stop-reply during `wait_at`'s own resume/sleep window exactly as `rpcs3-debugger.md`'s "call `drain()` after every breakpoint stop" warns. Fixed by reimplementing the catch as the `resume -> wait_for_stop -> drain -> check every thread's PC` shape `hd-flare-owner-break.py`'s `Breaker.stop_at` already uses (not imported - reimplemented inline, four lines). Second attempt: 40/40 hits, 0/40 misses (every `LR` breakpoint confirmed by a follow-up register read, not assumed).

**Finding: all 40 hits share the exact same `LR`, `0x2a019c` (call instruction at `0x2a0198`), and every single one is the orange family.** `f1` (the `D` argument) matches the freshly-written record's own `D` field exactly on all 40 hits, confirming the ABI mapping directly rather than by inference; `f2` (`w`) is `1.0` exact on all 40, also matching the record; colour ratio is `(0.25, 0.1)` - i.e. `1:0.25:0.1` - on all 40; `D` ranges `0.626`-`2.065`, squarely inside the buffer's own established `0.6`-`2.0` range. This is the orange family, caught at its own emission point, end to end, with no gap.

**`0x2a019c` is not within any of the thirteen catalogued call sites.** The nearest is `0x00155568` (`FUN_00155568`), `1,354,804` bytes below - far larger than any named function on this page (`Live_RcsBlackbox`, the largest, is `0xd690` = `55,952` bytes), so this is unambiguously a *different* function, not an offset inside one of the thirteen. It sits `0x70c` (`1,804`) bytes **before** the already-named `EngineFlare_RenderTick` (`0x002a08a8`, `hd-flare-owner-break.py`) - the same code neighbourhood, not the same function (`EngineFlare_RenderTick`'s own gate gates a draw call, not a light submission, and this page's own reading of it makes no mention of `SpuLight_AddCandidate`).

**This contradicts the previous section's "thirteen is exhaustive" claim, which was a static two-hit `get_xrefs_to(SpuLight_AddCandidate)`/`get_function_callers(FUN_006778c8)` result, not a live one.** Either a fourteenth call to the same thunk exists that Ghidra's static xref analysis did not attribute (a function boundary Ghidra has not recognised as a function at `0x2a0198`, so no xref analysis runs over it at all, is the most common cause of exactly this kind of miss), or the thunk itself is duplicated at the byte level somewhere Ghidra treated as a different, unnamed instruction rather than a call into the known thunk function. This live lane cannot resolve which, deliberately - it does not use `ghidra-mcp` (that is `lane-spu-light-static`'s tool, not this one's). **This is the single highest-value next step for whoever reads this next**: decompile whatever function contains `0x2a0198` (`0x2a0198 - 4` is the `bl` itself) and read its own callers - that answers both "which of the thirteen (if any) actually calls through here" and, far more importantly, resolves the orange family's full identity the way `FUN_000cfb80` and `FUN_001310e8` are already resolved.

**A behavioural note, not measured**: the 40 hits' own positions cluster into what looks like several repeating groups (e.g. hits 1, 8, 15, 22, 29, 36 sit within a few units of each other; likewise 2/9/16/23/30/37, 3/10/17/24/31/38, and so on - roughly seven recurring clusters), consistent with one shared call site inside a per-object loop submitting one orange light per object per frame (matching this session's own `candidate_count` of 6-13 per snapshot) rather than a single object sampled at different times. Recorded as a hypothesis only - the actual loop was not decompiled this session.

Raw hits: `data/traces/hd-spu-light-candidates/attribution.json` (gitignored, not committed).

**Confidence 90** for the call-site identification itself (40/40 exact, register-confirmed `LR` matches, `f1`/`f2` matching the record's own fields exactly - as direct as a live capture gets on this transport); 82 for "not one of the thirteen" (address-range exclusion, unambiguous given the size gap, but resting on this page's own function-size assumptions rather than a decompiled boundary); 40 for the per-object-loop hypothesis (position clustering only, not traced).

### The static and live lanes converged: `0x2a0198` is `EngineFlare_SubmitSpuLight`'s own call (2026-09-20, merge note, confidence 90)

The two sections above were written in parallel by two members who could not see each other's result. The live lane's 40/40 breakpoint hits with `LR = 0x2a019c` (the `bl` at `0x2a0198`) land inside `EngineFlare_SubmitSpuLight` (`0x0029ff28`), the function the static lane recovered from a byte scan and read in full the same afternoon - so the "unnamed fourteenth caller" the live section leaves open is the named producer the static section describes, found two independent ways (hand disassembly plus capture cross-check on one side, a register-confirmed live attribution on the other). Confidence 90 for the identification: the live `LR` is an exact address, and the static read's constants match every one of the 40 live records. Still open after both: the `D = 10.0, (2, 2, 0.4)` family (`FUN_001310e8` site 1 on `D`/`w`/shape, 75), and what arms the `+0x240` transition tint.

### Implementation note: the engine light is wired, the capture is Talon's Junction, and the count is 37 (2026-09-20, implementation lane)

Three corrections from wiring `EngineFlare_SubmitSpuLight` into `mesh.wgsl` (`oag_raceplay::engine_light`, `oag_mesh::mesh_render::spu_light`), none of which changes the reading above:

1. **`EngineLightData.xml` ships 37 times, not 36.** `scripts/psarc.py list` over all seven archives: 9 directories in `DATA02` (eight teams and `zone`), 4 in `DATA03`, and all **24** `_c1`/`_n1` variants in `DATA06` - the load-site entry's "22 variants" undercounted by two. `detonator`, `zone battle` and `test` ship none. `crates/tables/tests/enginelight_ground_truth.rs` asserts the count and the spans (`Distance` `-0.4`..`1.5`, `Radius` `0.7`..`2.0`, both as read above). Four float spellings occur (`0.4f`, `1f`, `0.3`, `-0.4f`); the reader strips one trailing `f`.
2. **The companion capture (`data/traces/hd-spu-light-companion/`) is Talon's Junction, not Amphiseum.** Every one of its 40 records was scored against the collision soup of each of the sixteen circuits (`crates/game/examples/hd_engine_light_which_circuit.rs`): Talon's Junction places all 40 between 2.98 and 4.41 units (mean 4.03) from the nearest triangle - a ride height, `5.5 * 0.75 = 4.1` - while Amphiseum places 3 of 40 within 5 units and the rest 10-147 away. The `+0x2084` and `EdgeGeom` entries above that say "Amphiseum" describe the same capture; nothing in them depends on which circuit it was. `rpcs3-capture.md`'s own note that the Racebox nav plan "landed on Talon's Junction this session, not Amphiseum" is the likely cause.
3. **The light never reaches the floor at ride height, in the original or here.** With `D` at `0.62`-`2.05` and every record a ride height off the surface, the term is zero on the track under a craft in level flight - and bound to the track chunks alone it changed zero pixels of a 1280x720 Amphiseum frame. This project's records on Talon's Junction sit 2.85-4.53 units (mean 4.09) off the surface over a 1,200-tick race, so the placement matches the original's to within the ride-height spread. The surfaces within `D` are the craft's own engine housing (`Distance` runs to `-0.4`, inside the nozzle) and whatever the craft is within a unit or two of - walls on a scrape, the floor on a landing. 74 of the ship materials compile `SVC1` twins (`scripts/ps3-sho.py svc-twins <image> materials/ships`: 1,776 pairs, 888 vertex blocks all carrying the `(255, 128)` decode and `0x868f8229`), so the list is bound to the hulls as well as the track. **Chosen, not measured: the hull binding.** The supporting evidence is static only - the material census and the geometry - with no live read of a hull chunk's `SVC1` bit, so it carries no score; the track binding and the record's arithmetic carry the sections above's own 80-88. Boosted, the `1 + 10 * blend` gain puts a 440-unit light a hand's breadth from the housing and the whole rear of the hull washes warm (`data/scratch/hd-engine-light/talons-t487.png` against `talons-t470.png` at rest); whether the original's hull does the same is the live check that would settle the binding either way.

Not wired: the `t`/`+0x2b4` blue-to-orange transition branch (arming unread) and the other 23 producers. Code: `crates/tables/src/enginelight.rs`, `crates/livery/src/engine_light.rs`, `crates/raceplay/src/engine_light.rs`, `crates/raceplay/src/load/engine_light.rs`, `crates/mesh/src/mesh_render/spu_light.rs`, `crates/mesh/src/mesh.wgsl` (`spu_light_sum`).

### `Ship_DrawModels` carries the same per-object `0x800`/`SVC1` gate the track's Zone-Stage compilers do - narrows, but does not close, whether the original's hull is ever a real receiver (2026-09-25, confidence 80 for the gate's existence, unscored for whether it ever fires)

**The question this section narrows**: whether the original ever draws the hull with the `SVC1` twin at all - the top open candidate for why this project's engine light washes a boosted hull white where the original's stays copper, tracked in this project's own handover backlog under rendering. The discriminating read named there: break in the ship draw compiler, or check whether any caller outside the track pass hands the `SpuLight` slot to a ship draw. Done statically, off the twelve-call-site sweep this page already ran (the "twelve unexamined call sites are read" section, 2026-09-18) - two of the ten `FUN_*` addresses named there without being decompiled, `0x003ea368` and `0x003eb890`, are `Ship_DrawModels` and its "one-entity twin" (both named independently by [ship-sun-occlusion.md](ship-sun-occlusion.md) and [absorb-feedback.md](absorb-feedback.md), which read them for the ship-sun-occlusion pass and the absorb/leach shell parameters respectively, neither for this).

**Decompiling both directly** (`decompile_function` on `0x003ea368` and `0x003eb890`, `program=EBOOT.elf`): each iterates the ship render record table `ship-sun-occlusion.md` already identified (`PTR_DAT_008b7da0`, count `+0x933c4`, index list `+0x933c8`, `0x1b0`-byte records - literally "the ship table", not a hazard/prop list), and per active ship (`record+0x140 != 0`) runs, before the ship's own `Render_RunCompiledOps_q` draw call:

```c
*(ctx+0x150) = 0; *(ctx+0x14c) = 0;
count = SpuLight_GetVisibleCount();                     // 0x0040d390
gate  = Enable_spu_vertex_light && Debug.Enable_EdgeGeom; // ctx+0x5a3 / +0x5aa, EnvSettings_GetOrCreate()
if (!gate || (record+0xe4 & 0x800) == 0 || count == 0
    || !SpuLight_AnyVisibleLightTouchesSphere())          // 0x0040dae8
{
    variant_bits = 0;
} else {
    slot = SpuLight_GetVisibleSlotAddress();               // 0x0040d370
    variant_bits = 0x800;
    *(ctx+0x150) = count; *(ctx+0x14c) = slot;
}
shader = Shader_GetVariantHash(variant_bits | ...);         // 0x003f0ff8
```

This is the exact shape this page already decompiled for the two Zone-Stage track compilers (`FUN_004074e0`/`FUN_00408fa8`, "the twelve unexamined call sites are read" section) - same flag reads, same three `SpuLight_*` calls, same `| 0x800` on `Shader_GetVariantHash`, same `ctx+0x14c`/`+0x150` staging this page's opcode-`0x2d` handler entry already proved is read by the identical downstream consumer (`_opd_FUN_005d5f20` / `_opd_FUN_005d8700`) regardless of which family wrote it. The only difference is the *object* being gated: a track chunk's bounding sphere there, a ship's own bounding sphere here.

**What `record+0xe4`'s `0x800` bit is - read as far as this session goes, and no further.** It is a per-ship flag word, not the material-variant word itself: other bits already read on other pages are `0x1` (activity), `0x2` (sun-occlusion eligibility, `ship-sun-occlusion.md`), `0x8`, `0x80`, `0x100` (a render-list dedup guard, read directly this session off `Ship_AddToRenderList` at `0x003e4cc8` - it returns early when this bit is already set, and does not touch `0x800`), `0x2000` and `0x4000` (activity, `zone-effectsettings-loader.md`). **No writer of bit `0x800` specifically was found this session.** A brute-force `search_instructions mnemonic=stw operand_pattern="0xe4(r"` returns hundreds of stack-frame-save false positives (any `stw rN, 0xe4(rM)` in the whole 21 MiB image, not filtered to this record's own base pointer) and does not converge in reasonable time; `Ship_AddToRenderList`, the one function already named as touching this exact field, tests a different bit and writes nothing. **So this reads only that the *gate* exists and has the right shape - not that the bit it tests is ever actually set on a real ship during a race.** That is exactly the same evidentiary position the 74 `SVC1`-twin-compiled ship materials were already in before this session (renderer.md's earlier "Not wired" entry above: "available, not proven picked") - this section adds a second gate downstream of that one, with the identical unread-bit shape, not a settled answer past it.

**What would move this past a narrowing.** Two paths, neither attempted this session: (1) find `record+0xe4`'s own writer statically - the search above was not exhaustive, only the one obvious candidate function was checked, and a scoped search (e.g. functions that also write nearby fields this page already reads, `+0xe0`/+0xe8`/`+0xec`/`+0x100`/`+0x130`, since those are the fields `Ship_DrawModels` itself reads alongside `+0xe4`) is the next cheap step; (2) a live RPCS3 read, pausing via the GDB stub with no breakpoints (the shape `absorb-feedback.md`'s "Live on RPCS3" section already used successfully on this same record table), reading the count/index at `PTR_DAT_008b7da0+0x933c4`/`+0x933c8` and then the u32 at the player's own record `+0xe4` mid-race, testing bit `0x800` directly - this would settle it at confidence ~90 either way without needing the writer at all.

**What this does *not* do: falsify the top candidate.** It narrows it from "no selection mechanism was ever found reaching a ship" to "a selection mechanism reaching ships exists and has the right shape, but whether it ever actually selects `SVC1` for a hull in play is still unread." `SpuLights::none()` for the craft (the thread's original fallback) remains a live option, not excluded by this section - it would be premature to relabel the binding "measured" or to treat it as wrong to keep, either way, until one of the two paths above closes the bit.

**Confidence 80** for the gate's existence and shape (two independent, previously-decompiled sibling functions - the Zone-Stage pair - already established every other piece of this shape at 80-85; this section adds that the identical shape recurs verbatim in `Ship_DrawModels`, read directly off its own decompile, with the object identity independently corroborated as ships rather than assumed from the `RigidBody`/`AbsorbFader`/`LeachFader` hashed strings, which `absorb-feedback.md` already showed drive nothing on this disc). **No score** for "the hull is ever actually `SVC1` in a race" - that claim is not read at all this session, only gated on an unread bit.

The crop comparison this session pulled (`data/scratch/hd-engine-light/crops/{orig,ours}-crop.png`, off `data/traces/hd-spu-light-companion/race.png` and `data/scratch/hd-engine-light/talons-t487.png`) shows the original's copper tint confined to the housing/fin geometry against this project's wider wash across flat underside panels - but the two frames are different teams/cameras (unverified against the capture's own team) at different angles, so this is not the "same ship and track, player size, more than one frame" comparison the task asked for, and is read here only as a qualitative prompt for the open bit above, not as evidence on its own.

No renames needed: `Ship_DrawModels` (`0x003ea368`, confidence 78) and `Ship_AddToRenderList` (`0x003e4cc8`, confidence 74) are already named on [ship-sun-occlusion.md](ship-sun-occlusion.md); `SpuLight_AnyVisibleLightTouchesSphere` (`0x0040dae8`), `SpuLight_GetVisibleCount`/`GetVisibleSlotAddress` (`0x0040d390`/`0x0040d370`) and `Shader_GetVariantHash` (`0x003f0ff8`) are already named on this page. No code changed.

### The hull's `record+0xe4 & 0x800` bit is set at load and the gate selects `SVC1` on the player's hull in a live race - the craft binding is measured (2026-09-25, confidence 90)

**This closes the open question the section above left.** Both routes it named were run: the writer was found statically, and the bit and the taken branch were read live.

**The writer, statically.** `record+0xe4` has one writer in the renderer module: `ModelRecord_Create` (`0x003f0348`), which scans the `0x1b0`-byte table off `PTR_DAT_008b7da0` for a free slot (`+0xe8 == 0`) and stores its third argument straight into the flag word, `0x003f0404: stw r5,0xe4(r29)`. A byte scan of `0x3e4000`-`0x3f2000` finds no other `stw rS,0xe4(rA)` outside the stack; the `stwx` loops at `0x3e7fec`/`0x3e8024`/`0x3e806c` and `0x3e9be0`-`0x3e9cf8` write `+0xe0`, not `+0xe4`. A whole-image scan of the 177 non-stack `stw rS,0xe4(rA)` finds none preceded by an `ori` that sets `0x800`. `ModelRecord_Create` is reached only through the TOC trampoline `0x006794d8`, and that only from `SceneModel_RegisterRenderRecord` (`0x002c0890`), which passes its own `r5` through untouched (`or r25,r5,r5` at `0x002c08c4`, `rldicl r5,r25,...` at `0x002c0948`/`0x002c0964`) and stores the returned index at `model+0x204c`. `Ship_ReloadModelForSkin` (`0x000dbdc8`) registers the hull `ship.vex` it stores at `ship+0x6ae0` (`0x000dc054`) with `li r5,0x1ccb` (`0x000dc044`, call `0x000dc058`); the wreck and both LODs (`+0x6ae4`/`+0x6ae8`/`+0x6aec`) get `0x1c8b`, and `+0x6af4` gets `0x480`. `0x1ccb` and `0x1c8b` both carry `0x800`; `0x480` does not.

**What the bit does at load.** `ModelRecord_Load` (`0x003eeb08`, called from `ModelRecord_Create` on the synchronous path) tests `record+0xe4 & 0x800` right after the model loads and calls `SpuLight_AttachVertexStream` (`0x0040d220`) on it. That walks every segment of type `5` (EdgeGeom) and appends one input-stream descriptor per geometry block, pointing at the SPU-light buffer base `*0x008b83b0` with format bytes `(1, 4, 0x44)`, skipping a block that already has one. So the hull's EdgeGeom blocks carry the `SpuVertexColours` stream from the moment the hull loads.

**Which pass draws it.** `Ship_DrawModels`' pass loop runs twice: pass 0 draws records with `0x80` set, pass 1 the rest. The hull's `0x1ccb` has `0x80`, so it goes through the gate at `LAB_003ea904` in pass 0.

**Live, RPCS3.** `0.0.42-19980`, `PPU Decoder: Interpreter (static)`, private config and cache, Campaign walk into a Talon's Junction race, thrust held. The player's craft is the entry of the eight at `0x0098d7c0` with `craft+0x7a60 == 0`; on both boots it was entry 7 (`0x33fbe380`, then `0x33fc2250` on the boot the breakpoints below ran on). One paused read on that boot, before any breakpoint:

| Field | Model | `+0x204c` (index) | `record+0xe4` |
| --- | --- | --- | --- |
| `+0x6adc` / `+0x6ae0` (hull) | `0x34028140` | `0x199` | `0x1ccb` |
| `+0x6ae4` | `0x3402af80` | `0x19a` | `0x1c8b` |
| `+0x6ae8` | `0x3402d010` | `0x19b` | `0x1c8b` |
| `+0x6aec` | `0x3402fd60` | `0x19c` | `0x1c8b` |

The table base read `*0x008b7da0 = 0x00c86880`, so the player's hull record is `0x00c86880 + 0x199 * 0x1b0 = 0x00cb1ab0`, and the u32 at `0x00cb1b94` is `0x1ccb`. The earlier boot read the same indices and flags for its player. Every one of the other seven crafts has a hull record at `+0x6ae0` reading `0x1ccb`. Their LOD records read `0x1c8b`.

Then four `Z0` breakpoints for 240 s of race, each stop read and stepped off:

- `0x003ea988`, just after `lwz r0,0xe4(r19)` in the pass loop: `r19` is the record, `r0` its flags, `r9` the `Enable_spu_vertex_light && Debug.Enable_EdgeGeom` gate, `r31` the visible-light count. 65 stops. `r9 = 1` on every one, and the count was 3 or 4.
- `0x003eb368`, the instruction after `SpuLight_GetVisibleSlotAddress` on the taken branch (the next instruction is `li r5,0x800`): 26 stops, **10 of them with `r19 = 0x00cb1ab0`, the player's hull**. The other 16 were other crafts' hull and LOD records. `r3` held the visible slot (`0x00f4b380`/`0x00f4c380`) and `r31` the count.
- `0x003eb858` (the first, `0x4000`-gated loop) and `0x003ec244` (the twin `0x003eb890`): no stops. Neither draws a hull on this path.

Taken-branch stops on the player's hull came both at cruise and while the visible list held a boosted light. The list read at the stop had one record at `(24.5, 61.2, 244.8)`, then `(20.4, 51.0, 203.8)`: that is `(4, 10, 40) * (1 + 10 * blend)` decaying. The HUD in the screenshot taken at the same stop shows the player on a speed-pad boost at 449 km/h: `data/scratch/hd-svc1-bit/bp-081.png`. That the boosted record is the player's own light is consistent with the frame, not read. The player's own position was not read.

**What the picture shows.** At that boost, on the original, the player's hull is drawn with `SVC1`. The blue light shows as a glow on the central nozzle and the housings around it. The flat panels and wings keep their livery, with no wash. The player's livery on this boot is dark with yellow stripes, on the Piranha planform. The light is classic `(4, 10, 40)`, not the Fury `(40, 10, 4)` of the earlier `race.png`.

**Confidence 90** that the hull record's `0x800` bit is set in a race and that the gate selects `SVC1` for the player's hull. Evidence: the static writer chain, one live read of the bit, and 10 live stops on the taken branch with the record pointer matching.

**What this falsifies, and what it leaves.** The top candidate on this page, that the original never selects `SVC1` for a hull, is falsified. The craft binding in `crates/raceplay/src/scene/frame.rs` is now measured.

The binding is still what produces this project's rear wash. The A/B was run at player size on Talon's Junction, boosted by a fired Turbo, which arms the same `exhaust.boost` the flame's boost blend reads. It took the craft's `SpuLights` out and left everything else. On `Assegai_n1` (Fury light `(40, 10, 4)`) at tick 295, the inner rear panels either side of the nozzle go flat white with the binding and show their grey, yellow and purple structure without it. That is about 600 pixels of a 1440x816 frame, all at the rear. The comparison is `rear-ab.png`, with the two frames beside it, under `data/scratch/hd-svc1-bit/`. Two same-command runs differ by about 2 % of pixels at this tick, from the frame's own motion blur. So the comparison was made against the run whose blur state matched and read by eye, not by pixel count. The white wing rims and the pink on the `_n1` wings are there with the binding removed as well. They are a separate term.

So the gap is in the term's inputs or its evaluation, not in whether it applies. At a boost, the original's hull shows the glow on the central nozzle and the housings (`bp-081.png`, classic light at about `x6`), not a flat white panel. The next candidates, unmeasured:

1. **The space the `EdgeGeom` job evaluates the falloff in.** If `RenderOps_BuildEdgeGeomJob` hands the SPU object-space light positions and the hull's world matrix carries a scale, then `|d|/D` is in object units there and in world units here. The reach of `D` differs by that scale. Read how opcode `0x30` builds the light array, and the hull's matrix scale.
2. **The anchor-to-panel distance.** The anchor's world position matches the original's records. The distance from it to the inner rear panels has not been measured on either side.
3. **The boost blend at the frame compared.** Ours at tick 295 is near the snap (`x11`). The original frame's visible boosted light was about `x6`.

Names: `ModelRecord_Create` (`0x003f0348`, 80), `ModelRecord_Load` (`0x003eeb08`, 65), `SpuLight_AttachVertexStream` (`0x0040d220`, 72) and `SceneModel_RegisterRenderRecord` (`0x002c0890`, 65) are in `names.tsv` against this page.

### Candidate 1 (boost blend at the compared frames) is closed, candidate 3's vertex-output register is noted but not measured, and candidate 2 (anchor-to-panel distance) gets a disc-verified, per-ship mechanism that predicts which hulls wash (2026-09-25, later still, confidence 85 for the per-ship correlation, unscored for whether it is the original's own behaviour too)

**Candidate 1 does not survive: the snap-and-decay law is already measured identical on both sides.** `EngineFlare_Update`'s own decompile (this file, "The boost gate is a timer, a snap and an exponential decay" via `engine-flare.md`) is `*(this+0x144) = 1.0f` under the boost-timer gate and `*= (1 - Thrust Chase Rate)` per 120 Hz substep otherwise - a direct read, not an inference - and `crates/raceplay/src/engine_light.rs`'s `Flame::advance` is the same two branches. There is no ramp-vs-snap discrepancy to find. What still varies is *which tick* a screenshot lands on relative to the snap, and the two reference frames this thread has been comparing against turn out to differ in more than that (next paragraph).

**The `x11`-vs-`x6` framing in the "What this falsifies" section above compares two different ships**, not two blend states of the same ship. `bp-081.png`'s player is on the **Piranha** planform (this file's own "Live, RPCS3" entry above: "The player's livery on this boot is dark with yellow stripes, on the Piranha planform"). The `rear-ab.png`/`tick 295` wash this thread has been chasing is on `Assegai_n1`. `Piranha` and `Assegai` are not the same hull, and `EngineLightData.xml` (below) says they are not even close.

**A same-command, same-tick, same-circuit A/B across three ships, this session, settles which hulls wash in this engine and which do not:**

```sh
oag-game data/images/hdfury-ps3-eu-dec.iso --no-audio --race \
  --track 'Data\Environments\talons_junction\track.vex' --team <TEAM> \
  --mode single_race --hold cross --give Turbo --press square \
  --ticks 220 --screenshot <out>.png
```

`--press square` toggles every tick, so `--give Turbo` keeps re-arming the pickup and the boost blend oscillates between the snap (`x11`) and one tick of decay (`x7.4`) for the whole run - the frame at tick 220 is a valid "near-snap" sample, the same blend regime the `rear-ab.png` comparison used.

- `--team piranha`: `data/scratch/hd-light-boost/piranha-snap.png`. No wash - livery intact, glow at the nozzle only. Matches `bp-081.png`'s picture qualitatively (different skin, same planform and same `EngineLightData.xml`).
- `--team assegai_n1`: `data/scratch/hd-light-boost/assegai_n1-snap.png`. Washes, same shape `rear-ab.png` already showed.
- `--team harimau`: `data/scratch/hd-light-boost/harimau-snap.png`. Also washes, at the belly/nozzle - a third ship, confirming this is not `Assegai_n1`-specific.

**`EngineLightData.xml` (`scripts/psarc.py cat`, all twelve teams' base directories) explains the split.** `Distance` sets how far *back along the flare's own Z axis* the anchor sits from the locator - `0.0` puts the anchor exactly at the node, a positive value pulls it back into open air behind the nozzle:

| Team | Distance | Radius | Washes here |
| --- | ---: | ---: | --- |
| `ag_systems` | 0.0 | 1.0 | not captured this session |
| `assegai` | 0.0 | 1.0 | yes (`assegai_n1`) |
| `harimau` | 0.0 | 1.0 | yes |
| `triakis` | 0.0 | 1.0 | not captured this session |
| `qirex` | 0.1 | 0.7 | not captured this session |
| `goteki` | 0.3 | 1.0 | not captured this session |
| `auricom` | 1.0 | 1.0 | not captured this session |
| `feisar` | 0.8 | 1.0 | not captured this session |
| `icaras` | 0.8 | 1.0 | not captured this session |
| `mirage` | 0.8 | 1.0 | not captured this session |
| `egx` | 1.2 | 2.0 | not captured this session |
| `piranha` | 0.4 | 2.0 | no |

A `_c1`/`_n1` variant carries the same `Distance`/`Radius` as its base team (`piranha`/`piranha_c1`/`piranha_n1` all `0.4`/`2.0`; `assegai`/`assegai_c1` both `0.0`/`1.0`) - the split is per team, not per skin. Every team this session captured with `Distance = 0.0` washes; the one team with `Distance = 0.4` does not. That is two data points on one side and one on the other, not a proof, but it is the disc's own authored data drawing the same line the screenshots do: a `Distance = 0.0` anchor sits exactly at the flare locator, which on these hulls is close enough to the surrounding panels that `Radius = 1.0` already reaches them at rest, and every panel already inside `D` gets the full `1 + 10 * blend` multiply at boost. A `Distance` of several tenths, on a hull whose nozzle recesses similarly, is enough to put the same panels outside `D` even at `Radius = 2.0`.

**What this does and does not settle.** It gives candidate 2 a concrete, disc-verified mechanism, and it explains why this thread's two reference frames (`bp-081.png` Piranha, `rear-ab.png`/`talons-t487.png` Assegai_n1) disagree without needing a code bug: they are different ships with different authored `Distance`. It does **not** show the original's own `Distance = 0.0` ships behave the same way at boost - no live capture of `assegai`, `harimau`, `ag_systems` or `triakis` boosted exists yet, on either side (`bp-081.png` is Piranha). Until one lands, "our `Distance = 0.0` ships wash and it is correct" and "our `Distance = 0.0` ships wash and the original's do not" are both still open - what is closed is that the wash is not explained by a mismatched boost blend or a wrong snap/decay law, and it correlates with authored per-ship data rather than one craft's own bug.

**Candidate 3's other reading, not measured.** The `SVC1` vertex program's decode writes `o[TC0]` (this page, "The `SVC1` combine is read": `14  MUL o[TC0].xyz, v[4].xyzx, R3.wwww`), a texture-coordinate output slot, not `o[COL0]`/`o[COL1]`. RSX-class hardware is documented to clamp colour-interpolator outputs to `[0, 1]` and not texcoord-shaped ones, which would rule out a per-vertex hardware clamp as a cause of the wash (an unclamped `TC0` interpolant would already match `mesh.wgsl`'s own unclamped `spu_light: vec3<f32>` varying) - but that hardware behaviour was not tested here, only read off which output slot the microcode targets, so it is a plausible reading and not a measurement.

**Next step, if this is picked back up:** a live RPCS3 capture of a `Distance = 0.0` team (`assegai`, `harimau`, `ag_systems` or `triakis`) boosted at player size, the same way `bp-081.png` caught Piranha - `data/scratch/hd-svc1-bit/drive.py` already reads the player's hull record and the visible SPU light list at the same breakpoint a screenshot is taken on (`hits.jsonl`, `"lights"` field on `BP_TAKEN_PASS` hits), so the same script run gets both halves of the comparison at once. If that hull washes too, the "gap" reported from play is this engine correctly reproducing a boost effect the original also has on those ships, and the thread closes with no code change. If it does not, candidates 1 and 3 are closed, which leaves the vertex-side geometry itself (the hull's own panel-to-anchor distances against `D`, `hd_engine_light_reach_probe.rs`'s approach applied to the hull mesh instead of the track) as what is left to measure.

### The original does not wash a `Distance = 0.0` hull at the boost snap: Assegai and Harimau, live on RPCS3, player's own light read boosted (2026-10-02, confidence 75, "seen" on one verified boot per ship plus one unverified Harimau boot)

**Question** (the thread's open one, `Distance = 0.0` teams): does the *original* wash the player's hull at boost on a team whose `EngineLightData.xml` puts the light anchor exactly at the flare locator? **Falsifier written before the capture:** a clean, non-washed Assegai or Harimau hull in the original at a frame where the player's own light reads boosted. **It came out clean, so "our wash is correct per ship" is falsified.**

**Method.** Private RPCS3 (interpreter, own config, own `dev_hdd0` copy so a team pick cannot reach the maintainer's saves, own display and GDB port), Fury campaign walk into Talon's Junction, `Team Selection` stepped by hand to the team and read off the screen (carousel order from `Feisar`: 1 Qirex, 2 Piranha, 3 AG Systems, 4 Triakis, 5 Goteki, 6 EG-X, 7 Assegai, 9 Harimau). The team the race actually flies is then **read from memory, not trusted from the screen**: the player is the entry of the eight at `0x0098d7c0` with `craft+0x7a60 == 0`; its team is the string at `*(*(craft+0x6298)+0x1b4)` (`Data\Ships\Assegai`, `Data\Ships\Harimau`), and `craft+0xfc/+0x100` read `Distance 0.0`, `Radius 1.0`. Breakpoint `0x003eb368` (the `SVC1` taken branch) reads the visible SPU light list (8 floats per record: position, `w`, colour, `D`) and a screenshot is taken at the stopped frame.

**Finding the player's own light in the list.** The list holds every craft's engine light, so "a light reads 360" does not mean the player is boosted: the first Harimau boot's flagged frames were a Piranha AI (`D` about 2.0). Dumping all eight crafts at two stops, the node triple at `craft+0x6980` tracks the player's light exactly (the offset `(+3.64, +0.25, +1.29)` held to 0.07 across two stops, deltas `(-1.96, -0.15, -0.68)` vs `(-1.98, -0.21, -0.67)`). Every frame below is taken only where the light nearest `craft+0x6980 + offset` (within 2.0) has colour `(36, 90, 360)`, i.e. `(4, 10, 40) * 9` - a `Distance 0.0`, non-Fury-skin player at `boost_blend` 0.8 - and the colour is the classic `(4,10,40)` base, not the Fury `(40,10,4)`.

| Boot | Team (memory) | Player light at the stop | Frames | Hull |
| --- | --- | --- | --- | --- |
| `assg1` | `Assegai`, 0.0 / 1.0 | `(-237..-270, -74, 55..44)`, `(36,90,360)`, `D` 1.0-1.1, 15 stops | `bp-assg1-BOOST-013..083.png` | dark blue livery, panel lines and shading intact on the spine and wings, small bright nozzle glow only |
| `hari5` | `Harimau`, 0.0 / 1.0 | same track points, `(36,90,360)`, 12 stops | `bp-hari5-BOOST-053..103.png` | white and orange livery, panel lines intact, wing trim colour intact, nozzle glow only |
| `hari3` | `Harimau` | boosted light at `D` 1.0 on the same trajectory, **owner not verified** (no node read that boot) | `bp-hari3-BOOST-*.png` | same as `hari5` |
| `hari2` | `Mirage` (0.8, a control, **owner not verified**) | boosted light in the list | `bp-hari2-BOOST-*.png` | clean |

Two more Assegai boots (`assg2`, `assg3`) never reached a pad with the player's light boosted; they are not counted. All files are under `data/scratch/hd-boost-wash/` (`pairs.png` has two original frames beside two of ours).

**What this does and does not settle.**

- **Settled (75):** in the original, a `Distance 0.0` hull whose own light is at `360` shows no flat-white or bloomed panels around the nozzle. A `360` light is a 9x gain within one unit of the panels beside the nozzle, and a pre-albedo add of that size saturates any albedo it reaches whatever the ambient, so the clean frame is not a dark-tunnel artefact: **the original's per-vertex term is not reaching those panels with that weight.** Ours, **same place, same tick, classic skin, light term on versus off** (an uncommitted env toggle that hands the craft `SpuLights::none()`, reverted; frames under `data/scratch/hd-boost-wash/ab/`): `--team assegai` at tick 295 goes flat white on the yellow spine and the left wing with the term on and keeps its yellow and blue structure with it off (`crop295-assegai.png`); `--team assegai_n1` at tick 295 is the `rear-ab.png` result again (`crop295-assegai_n1.png`). At tick 220 the same A/B is mild on `assegai` and `harimau` (the inner panels beside the nozzle a little whiter; about 1,500-2,000 of 1.17 million pixels differ, `crop-assegai.png`, `crop-harimau.png`), so the wash is tick- and view-dependent in ours, not constant. Read by eye at player size, two ticks per skin.
- **Not a like-for-like frame:** `oag-game` cannot be placed at the pad. Our boosted frames are at a different, sunlit stretch (`ours-assegai-noturbo-330.png` already shows a pale spine at 172 km/h with no boost), so the paired screenshots do **not** show our wash on their own; the on/off A/B above does. A same-place original/ours pair needs our craft driven to the pad.
- **Livery differs and is a separate gap:** the original's in-race `Data\Ships\Harimau` is the white and orange scheme the carousel shows; our `--team harimau` flies an orange and blue one. Not chased here.
- **The earlier "correlates with `Distance`" table is not the original's behaviour:** the original's `Distance 0.0` hulls do not wash, so `Distance` does not by itself explain the gap. What is still ours and not the original's is open, and is **not** the boost blend (closed), the bloom (measured), the colour (same classic `(4,10,40)` base), or the light's numbers (read off the original's own list above).

**What is left (not measured):** the per-vertex weight inside `EdgeGeom` (`0x007f6d80`) on a hull. Candidates, each a read of that SPU job's per-vertex loop rather than a guess: the `1 - |d|/D` term evaluated in a space or scale the hull's vertices are not in (our `mesh.wgsl::spu_light_sum` computes it in world units from `uniforms.model * placed`), the `N.L` term (the original may take the normal from the compressed `EdgeGeom` stream, not the file's), or whether the hull's `SVC1` stream is built from a different light subset than the track's (the `BP_TAKEN_PASS` stops all carried one record, `0xca6e90` on `assg1`, whose identity as the player's hull was not read this lane).

## HD's lightmap is read raw: the extra `pow(_, 2.2)` was the frame-wide darkness (2026-10-05, `hd-darkness`)

**Cause, confidence 85.** `mesh.wgsl`'s authored branch ran the lightmap through
`pow(texel, 2.2)` before `prelitScale * pow(_, prelitPower)`. Nothing on the disc
asks for it: block #9 of `track_surface.rcsmaterial` (see "The lit track material"
above) is `TEX`, then `LG2`/`MUL`/`EX2` by the authored power, then `MAD` by the
scale - no step in front of the curve - and "Per-texture sRGB/`GAMMA` decode:
settled negative" (confidence 90) finds no texture on the disc carrying a decode.
Omega's path already reads its atlas raw on the same grounds. The decode came in
with ADR-0026's blanket "the samples are sRGB-decoded" (`c31a8fe95`), never from a
microcode read of the lightmap, and it compounds with the authored power: on
Amphiseum (`Prelit ambient colour power` 3.5, scale 6) the curve ran at 3.5 * 2.2 =
7.7, which leaves a mid-grey lightmap at 0.5^7.7 = 0.005 of its authored value.
Talon's Junction authors power 2 / scale 4, Sol 2 power 1.3 / scale 0.87.

**Where it showed (before the change, `hd-material-probe.py` on Amphiseum pose 00,
delta = reference - ours).** Only the lightmapped group was dark: `track_surface`
(slot 547, 58,037 px) +0.087, `track_wall` +0.272, lightmapped `base_diffusespecular`
+0.201; the non-lightmapped groups sat at or above the reference. That ruled out a
missing light source: Sol 2 authors `Enable spu vertex lights = 0` and is the darkest
circuit, and the engine-light thread measured track chunks out of an engine light's
reach at ride height. After: `track_surface` +0.007, `track_wall` +0.111, `cf_diff_spec`
lightmapped +0.002.

**Whole-frame luma excluding the HUD, ours / reference**, `scripts/hd-frame-compare.py`
on copies under scratch, before then after:

| set | pose | before | after | reference |
| --- | --- | --- | --- | --- |
| Talon's Junction | 00 | 0.404 | 0.533 | 0.618 |
| | 01 | 0.376 | 0.497 | 0.601 |
| | 03 | 0.437 | 0.479 | 0.618 |
| Amphiseum | 00 | 0.164 | 0.237 | 0.245 |
| | 01 | 0.469 | 0.553 | 0.457 |
| | 02 | 0.297 | 0.479 | 0.427 |
| Sol 2 | 00 | 0.493 | 0.580 | 0.684 |
| | 01 | 0.479 | 0.566 | 0.698 |
| | 02 | 0.611 | 0.653 | 0.911 |

Amphiseum 00 closes (-0.081 to -0.008) and the floor reads cyan-lit where it was near
black. **Amphiseum 01 and 02 now overshoot (+0.096, +0.052)**: 01's excess is the
billboards, which were already white where the reference is saturated red-orange
before this change (an emissive/glow combine, not the lightmap), and 02 was
-0.130 before. Omega and 2048 frames are unmoved (Omega `data/extracted/ps4` default
race, 2 pixels by one level, which is shader-recompile noise; 2048 byte-identical):
Omega takes the nova branch and 2048's frame draws no HD-lightmapped surface.

**Still open, not changed.** (1) The remaining Talon's/Sol 2 gap (-0.09 to -0.14, Sol 2 02
-0.26) is not this term. The authored branch multiplies a decoded albedo by the light
and encodes `1/2.2` afterwards, so for a light sum `L` the frame carries `L^0.4545`
where the original's raw 8-bit multiply carries `L`; that is ADR-0026's domain choice.
Applying the light sum as `L^2.2` (equivalent to the original's raw-domain product) was
rendered once as a diagnostic and was worse on Amphiseum 00 (0.140) and mixed
elsewhere, so it is not made; the exposure/encode end of the chain (question 1 in
"Per-texture sRGB/`GAMMA` decode") is the next place to look. (2) Sky luma changes sign
between circuits (Sol 2 sky ours 0.51 / 0.55 / 0.50 against 0.86 / 0.94 / 0.94,
Amphiseum 00 ours 0.63 against 0.49) and does not touch the lightmap; the Sol 2 and
Talon's reference poses are 429-448 km/h frames with glare and adaptation state ours
cannot reproduce at rest, so it was not separated here. (3) The matched poses are
moving frames with a craft; only Talon's 00 was said to be stationary.
