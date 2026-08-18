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
| `SortRoot.cpp` | `0x002dbe08` | draw-order root |
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
| `0x002bebb0` | `MeshImporter_Construct` | 78 |
| `0x00650330` | `Gcm_Init` | 88 |
| `0x005c9194` | `Gcm_InitDevice` | 82 |
| `0x005bd2f8` | `GcmContext_Callback` | 76 |
| `0x008c0854` | `g_GcmContext` (data) | 85 |
| `0x005cd728` | `ShaderRegistry_Find` | 86 |
| `0x005cd6d8` | `ShaderRegistry_Register` | 84 |
| `0x005cd700` | `ShaderRegistry_RegisterComplete` | 78 |
| `0x008bf1ac` | `g_ShaderRegistryHead` (data) | 84 |

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
`bl ShaderRegistry_Register` across its 14 call sites recovers **62 registration
sites, 62 distinct names, and 62 payload pointers - and all 62 payloads begin
`53 48 4f 08`, `"SHO\x08"`.** That is the same `SHO` block
[rcsmodel.md](../../../formats/rcsmodel.md#the-rcsmaterial-was-the-leading-hypothesis-and-it-is-not-the-answer)
found inside a `.rcsmaterial`. So HD has **one shader container, used in two
places**: per-surface material shaders ship in `.rcsmaterial` files, and
engine-owned effect shaders are the same records linked into the executable.

The image holds **126 `SHO` blocks**, 124 of them in one contiguous run
`0x00927800`-`0x00936d80` at `0x80`-granular spacing, against 121 `_vp`/`_fp`
names plus `accuview`, `quincunx` and `quincunxalt` - the anti-aliasing modes,
which register the same way and are the only registered names not carrying a
stage suffix. The 62 unpaired names (all of `RadioHead*`, `psys_*`,
`FEBackgroundAnim*`, `HUD*`, plus `LensFlare`, `MagStripArc`, `engineflare`,
`cloudshader`, `lineshader`, `FatLine`) are **not evidence of a second
mechanism**: their registrars call the other constructor of the pair,
`0x005cd700`, for which Ghidra lists no callers at all, and the blob count
already accounts for them.

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

**A warning for whoever disassembles one.** The binder functions are at `0x003af980` and
`0x003b31c8`, both **above `0x0032d5e0`**, and the decompiler output there is
actively misleading rather than merely wrong: floats loaded from the TOC render
as string-pointer variables, so arithmetic reads as `(float)PTR_s_Emp__d_008a754c`.
Use `scripts/ps3-toc.py resolve` on every constant in that range.

## What was deliberately not read

- **The draw path.** No mesh submission, no state setting, no shader binding.
  It is inline command-buffer writing and finding it needs a search for RSX
  method constants, not a call graph.
- **`SortRoot.cpp`'s four functions** (`0x002dbe08`, `0x002dbe50`,
  `0x002dbe98`, `0x002dbf40`). All four are constructors writing the same two
  vtables and the `__FILE__` at `+0x30`; two of them additionally tail-call
  `0x00327500`. **None has a caller**, so base cannot be separated from
  complete and the pairs cannot be named - below 50, so per
  [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md) the
  hypothesis is written here and nothing is renamed.
- **Post-processing has no `.cpp` of its own**, and that turned out to be the
  wrong place to look: it is a set of named shader programs, not a class. See
  [the shader section](#shaders-are-in-exactly-two-places-and-neither-is-a-file-type-on-the-disc).
- **`RenderManager`'s methods.** This page names its constructors and its
  singleton, and reads no method, no vtable slot and no frame entry point. The
  512-byte table, the two matrix arrays and the 768 KiB buffer are all
  unexplained.
- **Everything about `Model.cpp`, `Billboard.cpp`, `Font.cpp`, `Pen.cpp` and
  the `Fx*` family** beyond where they start.
