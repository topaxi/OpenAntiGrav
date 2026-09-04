# HD's stencil-volume shadow path: a real shader, a real companion file, and a two-pass stencil test over a shifted box proxy

2026-09-04. Traced from `docs/rendering/shadows.md`'s open item: HD carries a
`LiveStencilShadow_vp`/`_fp` shader pair and a `/shadow.stencilvolume` string
beside its shadow-map pipeline, and nobody had looked at the code behind
either yet. Same kind of pass as
[shadow-occluder.md](../psp-pulse-usa/shadow-occluder.md) gave Pulse's
`0x3c3`, done here on `ps3-hdfury-eu` specifically. Evidence throughout is
`scripts/ps3-toc.py resolve` against each load's own instruction address (not
the global TOC - see this directory's own `memory.md`), cross-checked by
disassembling the exact block rather than trusting decompiled local-variable
names, which is what caught a wrong first reading (below).

Nothing here has been run under an emulator; every score is static-only and
caps at 84 per the confidence rubric - except the specific RSX register
identities in the draw-call section below, cross-checked against a local
`rpcs3` checkout's own `Emu/RSX/gcm_enums.h` (the source `just
build-rpcs3-watchpoints` clones) rather than general OpenGL knowledge, the
same "read an open-source emulator's own source" move `material-state.md`
already used.

## The shader technique is registered as a static object

`ShaderRegistry_Find`/`ShaderRegistry_Register` (`renderer.md`) are the
low-level primitives; three functions around them are specific to this
technique and are now named:

- **`Shader_ResolveLiveStencilShadowConstants`** (`0x005edb48`, confidence
  82) finds both `LiveStencilShadow_vp` and `LiveStencilShadow_fp` by name,
  refcounts the previous technique's shaders out, then resolves three named
  constant handles by `Crc32_HashString`: **`worldViewProj`**,
  **`lightDirection`**, **`extrusionDistance`**. All five string loads sit in
  one contiguous table off this function's own TOC (`0x008bd3c4` displacements
  `0x2108`/`0x210c`/`0x2110`/`0x2114`/`0x2118`) and were each resolved
  individually with `scripts/ps3-toc.py resolve` against the exact
  instruction issuing the load - not read off Ghidra's own symbol names,
  which this directory's `memory.md` already documents as untrustworthy on
  this binary's module-B functions.
- **`Shader_SetLiveStencilShadowTechniqueActive`** (`0x005ee580`, confidence
  80) is the register/unregister dispatcher: `(mode=1, sub=0xffff)` registers
  both shaders and two vertex attributes (`IN_position` at slot 0,
  `IN_normal` at slot 2); `(mode=0, sub=0xffff)` releases the previous
  technique's refcounted resources.
- **`Shader_ConstructLiveStencilShadowTechnique`** (`0x005ee7c0`) and
  **`Shader_DestructLiveStencilShadowTechnique`** (`0x005ee7b0`), confidence
  80 each, are one-line trampolines calling the above with `mode=1`/`mode=0`
  - a static object's constructor/destructor pair, not a per-model or
    per-frame call. `Shader_ResolveLiveStencilShadowConstants` is called once
    from the renderer's own boot-time init routine, alongside every other
    technique's registration.

**`lightDirection` is a named constant here, and Pulse's equivalent has no
such thing.** `Shadow_RenderOccluderVolume` (`psp-pulse-usa`,
[shadow-occluder.md](../psp-pulse-usa/shadow-occluder.md)) derives its
extrusion direction purely from the occluder's own local axis transformed by
its own world matrix and never reads a light. HD's technique binds a
`lightDirection` constant, which is consistent with the sun being the light
source here (`docs/rendering/shadows.md`'s cost table already assumes this)
but is not itself proof of it - the constant's *value* at draw time is
unread; this only shows the shader accepts one.

## The draw call is found: a two-sided depth-fail stencil test over a rigidly-shifted box proxy, colour-mask bracketed

**`Shadow_DrawOccluderStencilVolumes`** (`0x003e6918`, confidence 80) is the
per-frame draw call. It walks the model-instance array (`PTR_DAT_008b7da0`,
with a count/index list at `+0x933c4`/`+0x933c8` - the same global region
`0x003eeb08` uses for its own material-cache pointer) and, for each instance
with a byte flag at `self+0x140` set (role unread - not "visible", that was
an unverified guess in an earlier draft of this page) and the shadow-volume
flag (`self+0xe4 & 2`) set, runs:

1. `SET_COLOR_MASK(0,0,0,0)` - colour writes off.
2. **`Shadow_AccumulateStencilVolume`** (`0x005ed658`, confidence 84) - the
   depth-fail ("Carmack's Reverse") stencil-write pass.
3. `SET_COLOR_MASK(1,1,1,0)` - RGB on, alpha off.
4. `FUN_004053e0` (not renamed - PVS culling via
   `Pvs_IsUsable`/`Pvs_NearestCellCached`, zone-texture binding,
   `Shader_GetVariantHash`-based technique selection, its own alpha blending
   enabled with `SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA` - see below for what it
   submits and how).
5. `SET_COLOR_MASK(0,0,0,0)` - colour writes off again.
6. **`Shadow_ClearStencilVolume`** (`0x005ede20`, confidence 84) - stencil
   test and reset, no colour output.

**What FUN_004053e0 draws is not confirmed.** An earlier draft of this page
guessed "the shadow-casting instance's own regular visible mesh" from
parameter shapes alone, without decompiling far enough to check - that guess
is retracted. What *is* confirmed: it is the only one of the three calls
with colour writes on, and the only one with blending enabled, which makes
it the stronger candidate for whatever actually puts a visible darkening on
screen - the two `Shadow_*StencilVolume` passes write no colour at all.

**Checked further, 2026-09-04: `FUN_004053e0` doesn't draw immediately at
all.** It never calls `FUN_005a3e58` (the low-level primitive draw the two
stencil passes use directly - confirmed by listing every caller of
`FUN_005a3e58` and `0x004053e0` is not among them). Instead it compiles a
small bytecode-shaped command stream (shader-parameter-binding opcodes,
terminated) and hands it to `FUN_005fc728`, which builds DMA-list-shaped
descriptors matching this project's own already-documented SPU job pattern
(`engine-trail.md`'s "the SPU job's setup"). `FUN_005fc728` itself has
**seven** callers in this same rendering module - a shared, generic
job-submission primitive, making `FUN_004053e0` one of several sibling
"build a chunk-render job, then submit it" functions in this file, not a
bespoke one-off written just for this shadow path. That is evidence for
"generic infrastructure" again, same as the first correction, but through an
SPU-offloaded path rather than the immediate draw the stencil passes use -
it narrows the *mechanism* without settling *what geometry* gets submitted.
Fully resolving that would need SPU-job-level tracing at the scale
`engine-trail.md`'s `Trails` job investigation used, and this directory's
own `README.md` scopes HD as "opportunistic," not a milestone target - not
attempted here.

Every RSX register identity here was checked call-by-call against a local
`rpcs3` checkout's own `Emu/RSX/gcm_enums.h`, not assumed from general OpenGL
familiarity - including the colour-mask function itself, `0x005c2380`, whose
header word (`0x40324`) decodes to `NV4097_SET_COLOR_MASK` (`0x324`) the same
way `material-state.md` already decoded `Rsx_SetMethod`'s header math:

| Call | Register (raw byte offset) | Value |
| --- | --- | --- |
| `Shadow_AccumulateStencilVolume` | `0x183c` `NV4097_SET_CULL_FACE_ENABLE` | off - draw both faces |
| | `0x328` `NV4097_SET_STENCIL_TEST_ENABLE` | on |
| | `0x348` `NV4097_SET_TWO_SIDED_STENCIL_TEST_ENABLE` | on |
| | stencil func, both faces | `ALWAYS` (`0x207`), ref 1, mask `0xff` |
| | stencil op, front / back | `(KEEP, DECR_WRAP, KEEP)` / `(KEEP, INCR_WRAP, KEEP)` |
| `FUN_004053e0` | `0x310` `NV4097_SET_BLEND_ENABLE` | on |
| | blend equation | `FUNC_ADD` (`0x8006`) |
| | blend func | `SRC_ALPHA` (`0x302`) / `ONE_MINUS_SRC_ALPHA` (`0x303`) |
| | `0x304` `NV4097_SET_ALPHA_TEST_ENABLE` | on |
| `Shadow_ClearStencilVolume` | `0x183c` | back on |
| | `0x328` | on (single-sided) |
| | stencil func | `NOTEQUAL` (`0x205`), ref 0, mask `0xff` |
| | stencil op | `(KEEP, KEEP, ZERO)` - resets the stencil on pass |

That is the standard two-sided depth-fail stencil register/op setup, except
that its clear step both draws nothing (colour off) and self-cleans (zeroes
the stencil where accumulated, so the next caster in the same frame needs no
separate clear) - whatever visual effect the shadow actually has must be
`FUN_004053e0`'s job, not the two named stencil functions'. Both
`Shadow_*StencilVolume` passes upload the `LiveStencilShadow` technique's
vertex constants (`Rsx_UploadVertexConstants`, via the technique object
cached at `PTR_DAT_008bf4c4` - see `Shader_ResolveLiveStencilShadowConstants`
above) and draw the **same** `self+0x128` handle via `FUN_005a3e58` with a
literal `5` as an argument that, if it is a GCM primitive type, `gcm_enums.h`
makes `CELL_GCM_PRIMITIVE_TRIANGLES` - **not** `TRIANGLE_STRIP`, which is `6`
(GCM's primitive list is 1-based, unlike OpenGL's 0-based one; an earlier
draft of this page cited `5` as `TRIANGLE_STRIP` without checking the actual
enum and that was wrong). The parsed shadow-volume geometry is drawn twice,
once per stencil pass, not once for the volume and once as a full-screen
quad the way a textbook write-up often shows it.

**Checked directly, 2026-09-04, once the vertex shader below raised the
question: neither pass re-uploads a different `extrusionDistance`.** Both
`Shadow_AccumulateStencilVolume` and `Shadow_ClearStencilVolume` compute
their constant-block offset (`iVar7`) from the exact same cached technique
object (`PTR_DAT_008bf4c4`) the exact same way and call
`Rsx_UploadVertexConstants` once each, with nothing in either function's
decompile that varies `extrusionDistance` between them. So this is **not** a
classic near-cap/far-cap shadow volume built from two differently-extruded
copies of the same geometry - both draws submit the *same*, identically-
shifted closed box. That is still a legitimate use of two-sided depth-fail
stencil (any already-closed, watertight solid can be tested this way to
determine whether a given pixel's depth sample lies inside it, without
needing a separate near/far pair - that construction is only one way to
build the closed volume the technique requires, and this shader uses the
other). What the two passes actually compute, put together with the vertex
shader finding below, is closer to **"does this pixel's depth sample fall
inside a fixed box template, shifted rigidly toward `lightDirection` by
`extrusionDistance` and positioned at the caster"** than to a true
silhouette-derived shadow volume the way Pulse's `Shadow_RenderOccluderVolume`
builds one.

**This also revises the record-format reading below, now closed rather than
hypothesized.** `self+0x128` is not walked as a raw `n`-record array by the
draw call - it is walked as a `std::vector`-shaped object (`param_1[2]`/
`[3]`/`[4]` as begin/end/capacity, `FUN_00734bd0` as the grow-on-full
helper), one 16-byte `{index-buffer ptr, vertex-buffer ptr, index count,
vertex count}` entry pushed per **outer** loop iteration inside
`Shadow_ParseStencilVolumeGeometry`, not per `n`-record. The draw reads each
entry's index-count field (offset `+8`) as the primitive count - see the
next section for how `Shadow_AllocateStencilVolumeBuffers` settles what `n`
and `m` actually are.

## The trigger is a per-model flag, and the sibling file is fixed-named

`0x003eeb08` is not renamed - it clones materials, builds per-submesh render
entries, hashes material parameter names, and compiles a per-instance
render-command stream, well past what one pass can name confidently as a
whole (see its own plate comment). One narrow slice of it is closed:

**When the model instance's own flag word at `self+0xe4` has bit `0x2` set**,
the code at `0x003efba4`-`0x003efcb0` (placed out-of-line from the hot path,
which is why it appears early in the decompile but late in the address
range) does the following, traced instruction-by-instruction because the
decompiler's local-variable names for this block turned out to not line up
with which literal string went where on a first read:

1. Copies the model's own base path into a scratch string.
2. Normalizes it: replaces `\` with `/` (two single-character string
   constants at `-0x54fc(r2)` and `-0x54f8(r2)` off this call site's own
   TOC, resolved and read directly as `'\'` and `'/'`).
3. Finds the last `/` in the normalized path.
4. Appends the literal string **`shadow.stencilvolume`** (`-0x54f4(r2)`,
   resolved to `0x007b3398` = `/shadow.stencilvolume`) after it.

**Confidence for this join being "a fixed name, not per-model-scoped": 65.**
The leaf itself is solid (`shadow.stencilvolume` is a literal, confirmed
above); "therefore not per-model" rests on six small string-utility calls
(`0x00399430`, `0x003951d8`, `0x0039be08`, `0x00397588`, `0x0039a568`,
`0x003987c8`) whose exact semantics (wrap/replace/find-last/concat/append/
destroy, read off argument shape and call order, not decompiled individually)
are inferred rather than confirmed. The directory the last-`/` search resolves
into could itself still be per-model. No PSARC archive has been searched for
an actual `shadow.stencilvolume` entry yet - see Open.

The resulting path is passed to **`Resource_FindOrLoadByHashedPath`**
(`0x003ee398`, confidence 75 - named 2026-09-04, deliberately **not**
`Shadow_`-prefixed: nothing in the function is shadow-specific, the path
string is entirely the caller's, and it is only reached from this one call
site here). It is a `Crc32`-hashed find-or-load cache: a red-black-tree
lookup by the path's hash, returning a cached handle on hit; on miss it opens
and reads a stream (`FUN_00679ec8`/`FUN_00679ed8`, not decompiled) and parses
the result with **`Shadow_ParseStencilVolumeGeometry`** (`0x005ee7d0`,
confidence 75) into a newly allocated 64-byte block. The returned handle is
stored at **model-instance offset `+0x128`**.

*A dead end from the first pass, caught before it was written down as
evidence*: both functions have a data cross-reference (`0x0088c728` for
`Resource_FindOrLoadByHashedPath`, `0x008a1808` for
`Shadow_ParseStencilVolumeGeometry`) that first read as a shared
vtable/dispatch slot, implying possible reuse for other resource kinds.
Checked with `scripts/ps3-toc.py u32`: each resolves to exactly `{function
address, 0x008bd3c4}` - an ordinary PPC64 **OPD descriptor**, the
`{code, TOC}` pair every function in this ELF has (see this directory's own
`README.md`). It is not evidence of a shared dispatch table at all, and
"maybe reused elsewhere" below is a live open question, not a lead.

## The record format is closed: an explicit vertex buffer and a u32 index buffer

`Shadow_ParseStencilVolumeGeometry`'s parse loop, read directly off its
decompile rather than inferred from variable names: an outer repeat-count
taken from the resource buffer's own first `u32`, stored back onto the
destination object's offset `0x0`; offsets `0x8`/`0xc`/`0x10` are a
`std::vector`'s begin/end/capacity pointers. Offsets `0x20`/`0x30` hold a
running min/max reduction over the same three per-record floats (byte
offsets `0xc`/`0x10`/`0x14`) - **this page previously called that pair a
"bounding box"; it is not**. The byte-verified record layout below (position
at offsets `0`/`4`/`8`, unit normal at `0xc`/`0x10`/`0x14`) means this
reduction runs over *normals*, not positions, and a min/max over the set of
face normals in a fixed six-face box is a constant (`[-1,-1,-1]..[1,1,1]`)
regardless of the model - not a meaningful spatial bound. What `0x20`/`0x30`
are actually for is unresolved; the "bbox" framing is retracted everywhere
below and in `HANDOVER.md`/the thread file. Per outer iteration: a header of **two `u32` fields** (not
`u16` - both are read through a `uint *`), followed by a third leading `u32`
word whose role is unread, then **`Shadow_AllocateStencilVolumeBuffers`**
(`0x005ee2c0`, confidence 82) allocates two buffers sized by the header:

- an **`n`-element, 24-byte-stride vertex buffer** (`FUN_005a88c8(dest, n,
  0x18, 2, ...)` - the `0x18` stride matches the 24-byte/6-`f32` input
  record loop exactly)
- an **`m`-element index buffer**, format encoded rather than an explicit
  stride (`FUN_005a8750(dest, 0, m, ...)`)

**Closed by an exact arithmetic invariant, not just a plausible reading**:
`FUN_005a8750`'s own branchless index-width computation
(`((param_2^0x10)-((param_2^0x10)>>31)-1)>>31 & 0xfffffffe) + 4`), evaluated
for the literal `param_2 = 0` this call site passes, comes out to **4** - a
4-byte, `u32` index - and separately, `Shadow_ParseStencilVolumeGeometry`'s
own `memcpy` of the index blob copies `m << 2` bytes, i.e. exactly `m` `u32`s.
Two independently-read functions agreeing on the same byte size without
either citing the other is the strongest evidence on this page short of a
runtime trace or a real captured file - the confidence rubric's own 85-94
band ("an exact arithmetic invariant... is worth more than any amount of
re-reading"). `FUN_005a88c8`/`FUN_005a8750` are both confirmed generic buffer
constructors (read in full, neither is shadow-specific - not renamed).

So, closed: (if `n` is positive) a loop of `n` 24-byte input records is
copied/transformed into the new vertex buffer, and separately feeds a
running min/max reduction over three of its six floats (see the retraction
above - this is not a bounding box). The `m`-sized, `m*4`-byte index blob
that follows the `n` records is `memcpy`'d into the new index buffer. One
`{index-buf ptr, vertex-buf ptr, m, n}` entry is then `push_back`'d per outer
iteration - **`n` is the vertex count and `m` is the drawn index count**,
read by `Shadow_AccumulateStencilVolume`/`Shadow_ClearStencilVolume` as
exactly that.

That is the same coarse shape as the PSP `DynamicShadowOccluder` `.vex`
payload documented in [shadow-occluder.md](../psp-pulse-usa/shadow-occluder.md)
- small leading counts and a geometry array - but a **different topology
encoding, not merely different bytes**: HD pairs an explicit vertex buffer
with a separate index buffer, where Pulse's `n` (faces) and `m` (vertex
slots) each index their own same-shaped record array with no separate index
list at all. Bridging the two into one implementation-facing representation,
if that is ever wanted, means resolving an indexed-triangle-list encoding
against a face-list-with-inline-indices one, not just a byte-for-byte remap.

Confidence 82 on the `n`=vertex/`m`=index-`u32` split as read from
`EBOOT.elf` alone (the arithmetic invariant plus three independent call
sites - the allocator, the vector push, the draw - agreeing); see the next
section for why the on-disc **record layout** itself now sits higher than
that, at 92, once real files closed it byte-for-byte. 75 on
`Shadow_ParseStencilVolumeGeometry`'s and `Resource_FindOrLoadByHashedPath`'s
own overall roles as functions (single call site each, not runtime-traced);
the copy loop's own internal byte-shuffling (which destination offset gets
which source float, across the vectorized/decompiler-mangled code above) is
lower confidence still and is flagged unresolved rather than guessed at.

## Byte-verified: 39/39 real files close the record format and reveal a fixed box topology

2026-09-04. `scripts/psarc.py list|extract` against all seven PSARC archives
on `data/images/hdfury-ps3-eu-dec.iso` (`PS3_GAME/USRDIR/DATA0{0,2,3,6}.PSARC`)
finds **39** `data/ships/<name>/shadow.stencilvolume` entries disc-wide, one
per ship/skin variant (`feisar`, `feisar_c1`, `feisar_n1`, `detonator`,
`qirex`, ... - every entry under `data/ships/`, none anywhere else on the
disc). Every one of the 39 decodes with the exact same big-endian header,
`struct.unpack_from('>IIII', data, 0)` = `(outer=1, n=24, m=108, pad=0)`, and
every one's file size matches `16 + n*24 + m*4 = 1024` bytes exactly - **39
for 39**, not a sample.

Decoding `n=24` vertex records as `(x, y, z, nx, ny, nz)` `f32`s (the layout
`Shadow_ParseStencilVolumeGeometry`'s copy loop implied, now checked against
real bytes rather than only the decompile) gives, per file, exactly **6
distinct unit-length axis-aligned normals** - `(±1,0,0)`, `(0,±1,0)`,
`(0,0,±1)` (each recovered as `±1` to float rounding noise, e.g.
`-3.28e-08` in place of an exact `0`) - with 4 vertices sharing each normal:
**a 6-face box, unwelded so each face gets its own flat normal**, exactly
matching the "24 vertices, 4 per face" split. The `m=108` index buffer
(36 triangles) splits into two groups by *position*, not just by index
range: indices `0..35` (12 triangles, 2 per face x 6 faces) are the box's
only real geometry, and each face's 4 corners already coincide in 3D space
with the matching corners of its neighbouring faces (a duplicate-vertex-
per-face-normal box is watertight by *position* even though every corner
has 3 separate index-carrying copies, one per adjacent face). Indices
`36..107` (24 triangles) are **degenerate on all 39 of 39 files, not a
sample**: every one of these 24 has two of its three vertices at the exact
same position (two of a corner's three per-face copies), so it rasterizes
zero area and contributes nothing to the stencil count - and the raw
108-`u32` index array is **byte-identical across all 39 files**, not merely
same-shaped, confirming the index list really is one fixed disc-wide
constant rather than 39 coincidentally-matching ones.

**An earlier version of this page called these 24 "edge-bridging
triangles... sealing the box into one closed manifold" - that overstated
their role.** The box was already watertight from the 12 real triangles
alone; the other 24 are inert padding, not sealing geometry. Why the
exporter emits them at all - a fixed per-vertex triangle-fan template that
always walks all three of a corner's copies regardless of whether they
coincide, most likely - is unconfirmed and left as an open question rather
than guessed at. Vertex *positions* vary per file - `feisar`'s box is a
different size than `detonator`'s or `qirex`'s bounding footprint - but the
**topology (vertex count, normal set, index list, and which 24 of 36
triangles are degenerate) is byte-identical across every one of the 39
files**. This is the exact arithmetic-invariant-across-many-real-files
case the confidence rubric scores 85-94; record format confidence raised
from 82 to **92**.

Cross-title lineage worth stating plainly: HD's `n`/`m` are invariant across
the *entire disc*, where Pulse's `DynamicShadowOccluder` varies per weapon
(`pulse_mine` at 4/4, `pulse_bomb` at 11/12, per
[shadow-occluder.md](../psp-pulse-usa/shadow-occluder.md)) - Pulse authors a
hull per object; HD authors one topology once and refits its vertices per
caster.

Reproduce: `scripts/psarc.py list data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA0{0,2,3,6}.PSARC | grep stencilvolume`,
then `extract` any entry and `struct.unpack_from('>IIII', data, 0)` /
`'>6f'` per 24-byte record / `'>{m}I'` for the index blob.

## The vertex shader is disassembled: a uniform rigid shift, not a per-vertex silhouette extrusion

2026-09-04, continuing the same pass. `Shader_SetLiveStencilShadowTechniqueActive`
registers the technique's two shaders with `ShaderRegistry_Register(slot,
name, block)`; the third argument resolves (`scripts/ps3-toc.py u32`, then
confirmed by reading the `"SHO\x08"` magic directly out of memory) to
**`0x00929600`** for `LiveStencilShadow_vp` and **`0x00929580`** for
`LiveStencilShadow_fp` - both built into `EBOOT.elf` itself, not an external
`.rcsmaterial`, the same "shaders live in the executable's own shader run"
pattern `renderer.md` already established. `scripts/ps3-microcode.py vp
0x00929600` disassembles it in full - 8 instructions:

```
attribute 0xd0333cc7  slot 0   (IN_position)
attribute 0x58150554  slot 2   (IN_normal)
parameter 0x4c06f24f  float4 x4  c256   (worldViewProj)
parameter 0x12c7d82c  float3 x1  c467   (lightDirection)
parameter 0x711247ea  float1 x1  c466   (extrusionDistance)

0  DP3 R63.x, v[2].xyzx, c[211].xyzx     ; dot(normal, lightDirection) -> R63.x
1  MOV R1.xyz, c[211].xyzx               ; R1 = lightDirection
2  MOV R0.xyz, v[0].xyzx                 ; R0 = position
3  MAD R0.xyz, R1.xyzx, c[210].xxxx, v[0].xyzx  ; R0 = position + lightDirection * extrusionDistance
4  MUL R1, R0.yyyy, c[1]
5  MAD R1, R0.xxxx, c[0], R1
6  MAD R0, R0.zzzz, c[2], R1
7  ADD o[POS], R0, c[3] | END            ; standard worldViewProj * R0
```

The three parameter-name hashes are exact preimage matches - `0x4c06f24f` =
`~crc32("worldViewProj")`, `0x12c7d82c` = `~crc32("lightDirection")`,
`0x711247ea` = `~crc32("extrusionDistance")`, computed directly and checked
against the values `ps3-microcode.py` printed, not assumed from the names
`Shader_ResolveLiveStencilShadowConstants` already resolved. Declared
constant `c467` reads as code register `c[211]` and `c466` as `c[210]`, per
this same directory's already-established "`c[N]` is the parameter table's
register `N + 256`" rule (`renderer.md`).

**This settles the mechanism the box topology only suggested, and it is not
what the "silhouette extrusion" framing assumed.** Every vertex - not a
subset selected by facing - gets the *same* displacement,
`lightDirection * extrusionDistance`, added to its position before the
`worldViewProj` transform (instruction 3). There is no per-vertex branch or
blend keyed on the normal anywhere in the program: instruction 0 computes
`dot(normal, lightDirection)` into `R63.x`, and no later instruction reads
`R63` - the value is computed and never used again in this 8-instruction,
one-`END` program. So this is a **rigid-body shift of the whole sealed box
along a uniform direction**, not a per-vertex silhouette extrusion the way
Pulse's `Shadow_RenderOccluderVolume` computes one (which explicitly varies
which vertices move based on facing). A fixed, disc-wide, per-caster-fitted
box template is exactly what this mechanism wants: there is nothing
model-specific for the vertex program to key off of, so the topology and
the per-vertex position are all it needs. Checked against the two draw
functions directly (see the draw-call section above): neither
`Shadow_AccumulateStencilVolume` nor `Shadow_ClearStencilVolume` re-uploads
a different `extrusionDistance` between passes, so there is no separate
unshifted/shifted pair anywhere in this path either - both draws submit the
same, once-shifted box. `LiveStencilShadow_fp` is trivial
and confirms this is a stencil-only pass: `MOV H0, {1,0,0,0} | END`, one
instruction, no texture reads, no lighting - it exists only so the
fixed-function pipeline has *something* to shade, since colour output is
masked off for both stencil passes anyway (see the draw-call section
above).

One loose end, flagged rather than asserted past: `ps3-microcode.py`'s own
docstring documents that it originally dropped per-instruction predication
on **fragment** programs (fixed since) and says nothing about vertex-program
predication; whether NV40 vertex instructions can carry a similar predicate
this tool doesn't yet decode is unconfirmed, so "R63 is dead code" rests on
what this disassembler prints, not a guarantee no hidden condition exists.
Nothing in this specific 8-instruction program's printed operands
references a predicate, though, and the reading above needs no such
mechanism to explain what actually reaches `o[POS]`.

Confidence 84 on the mechanism (a uniform per-vertex shift, not a facing-
dependent one) - a direct disassembly of the actual executed instructions,
unambiguous and with every parameter name confirmed by exact hash preimage,
but this is one shader block, not an arithmetic invariant checked across
many real files the way the record format above is, and nothing here is
runtime-traced - the same 84 ceiling this page's intro already states for
everything except a cross-checked constant identity. This replaces the
confidence-65 "shader probably extrudes along the normal" framing an
earlier pass in this same file guessed at - that guess is now known to be
wrong in the specific way described (no per-vertex, normal-dependent
extrusion at all), not merely unconfirmed.

## What was deliberately not chased this pass

- **`FUN_004053e0`'s SPU job in full.** Confirmed it hands off to
  `FUN_005fc728` (a shared job-submission primitive, seven callers in this
  module) rather than drawing immediately - the strongest candidate for the
  shadow's actual visible effect, colour-on and blend-enabled, but what
  geometry the job actually contains is unconfirmed and would need
  SPU-job-level tracing to settle, the scale `engine-trail.md`'s `Trails`
  job investigation used. Not attempted here.
- **Whether `lightDirection`/`extrusionDistance` are ever set to anything
  other than a default.** The upload call (`Rsx_UploadVertexConstants`) is
  now found, but what values it uploads at runtime is unread.
- **Why `Shadow_ParseStencilVolumeGeometry`'s copy loop reads the way it
  does.** The loop's per-vertex source/destination offsets are heavily
  vectorized and decompiler-mangled (see the retracted "bbox" note above);
  the on-disc format is now closed independently, against real files, so
  this doesn't block anything further, but the loop's own internals are
  still not confidently mapped byte-for-byte.

## Open

- ~~Whether `LiveStencilShadow_vp` actually extrudes per-vertex along
  `lightDirection`/`extrusionDistance`.~~ **Disassembled 2026-09-04**: no -
  it applies a *uniform* `lightDirection * extrusionDistance` shift to
  every vertex regardless of facing, not a per-vertex silhouette extrusion;
  the normal is read into a dot product that is never used again. Also
  checked directly against both draw functions: neither varies
  `extrusionDistance` between them, so both stencil passes test the *same*
  once-shifted box, not a near-cap/far-cap pair (confidence 84) - see above
- Why the exporter emits 24 degenerate (zero-area) triangles per box when
  the 12 real face-quad triangles alone are already watertight - a fixed
  per-vertex triangle-fan template that always walks all three of a
  corner's per-face copies regardless of whether they coincide is the
  leading guess, unconfirmed
- Why `data/ships/detonator/`'s box is ship-scale (~6 x 3 x 14 units,
  same order of magnitude as `qirex`'s ~5.6 x 3 x 15), when
  [detonator-bomb.md](detonator-bomb.md) names `DetonatorBomb` as a weapon
  class - either this path is a distinct ship-sized entity that happens to
  share the name, or a weapon's own shadow proxy is authored ship-sized and
  the box is not simply "fitted to the caster's own bounds"; not resolved
  here
- Whether every other model class on the disc (props, track pieces, other
  weapons) that casts a `LiveStencilShadow` shares this same fixed 24-vertex
  box template, or whether some use a different fixed topology - only the
  `data/ships/*` entries were checked
- Whether anything besides `Shadow_DrawOccluderStencilVolumes` reads
  `self+0x128`, or feeds the `+0x933c4`/`+0x933c8` instance list - single
  known caller each, not traced further
- Whether `Resource_FindOrLoadByHashedPath` has other callers elsewhere in
  the binary for unrelated hashed-path resources - not searched for
- Whether the path-join truly resolves to a fixed, non-per-model directory
  entry - confidence 65, resting on six unverified string-utility readings
- What geometry `FUN_004053e0`'s SPU job actually contains, and thus what
  the shadow's visible effect actually is - the strongest lead is that it is
  the only colour-on, blend-enabled call in the whole sequence, and that its
  job-submission mechanism (`FUN_005fc728`) is shared generic infrastructure
  rather than a one-off, but its geometry source is unconfirmed either way
- What the byte flag at `self+0x140` gates - not established as "visible" or
  anything else, just observed as a nonzero check
- What offsets `param_1+0x20`/`+0x30` (the retracted "bbox") are actually
  for, now that they're known to reduce over normals rather than positions
