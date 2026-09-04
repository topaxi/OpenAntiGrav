# HD's stencil-volume shadow path: a real shader, a real companion file, an unread record format

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
caps at 84 per the confidence rubric.

## The shader technique is registered as a static object

`ShaderRegistry_Find`/`ShaderRegistry_Register` (`renderer.md`) are the
low-level primitives; three functions around them are specific to this
technique and are now named:

- **`Shader_ResolveLiveStencilShadowConstants`** (`0x005edb48`, confidence
  82) finds both `LiveStencilShadow_vp` and `LiveStencilShadow_fp` by name,
  refcounts the previous technique's shaders out, then resolves three named
  constant handles by `Crc32_HashString`: **`worldViewProj`**,
  **`lightDirection`**, **`extrusionDistance`**.
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

So the file the engine looks for is **a fixed name in the model's own
directory**, not a per-model-named companion the way `BEData.wad`'s
`shadowShape`/`shadow_mineShape` nodes are named after their weapon on the
Pulse/2048 side. No PSARC archive has been searched for an actual
`shadow.stencilvolume` entry yet - see Open.

The resulting path is passed to `0x003ee398` (not renamed - single known
direct call site, but also reached through a data/vtable slot at `0x0088c728`,
so it may be shared with other resource kinds; confidence for "this is
exclusively the shadow-volume loader" is correspondingly lower than for the
mechanism itself). It is a `Crc32`-hashed find-or-load cache: a red-black-tree
lookup by the path's hash, returning a cached handle on hit; on miss it opens
and reads a stream (`FUN_00679ec8`/`FUN_00679ed8`) and parses the result with
`0x005ee7d0` (also not renamed, same reasoning) into a newly allocated
64-byte block. The returned handle is stored at **model-instance offset
`+0x128`**.

## The parsed shape rhymes with Pulse's occluder payload; the bytes don't match it

`0x005ee7d0`'s parse loop: an outer repeat-count, then per iteration a pair of
16-bit counts `(n, m)` followed by `n` 24-byte (6-`f32`) records, each record
reduced component-wise into a running min/max - a bounding box - while the
raw record data is retained via a `memcpy`'d buffer. That is the same coarse
shape as the PSP `DynamicShadowOccluder` `.vex` payload documented in
[shadow-occluder.md](../psp-pulse-usa/shadow-occluder.md) - small leading
counts, a geometry array, a derived bounding box - **but not the same byte
layout**: Pulse's is a 32-byte face record (unit normal + vertex-count +
indices) plus a 16-byte `(w,x,y,z)` vertex record, no bounding box carried in
the payload itself (bbox is authored elsewhere in the `.vex` node). HD's is a
flat array of 24-byte, 6-float records with no visible face/vertex split, and
the bounding box is *computed* here rather than read. Whether HD's 6 floats
are two packed `vec3`s (e.g. a pair of extruded silhouette-edge endpoints) or
something else is unread - this is a hypothesis about shape, not layout,
confidence 60, no name for the record format yet.

## What was deliberately not chased this pass

- **The draw call.** Nothing here traces who calls
  `Shader_ResolveLiveStencilShadowConstants`'s registered technique per
  object, or reads `lightDirection`/`extrusionDistance` at draw time. The
  renderer's inline-command-buffer problem
  (`renderer.md#what-was-deliberately-not-read`) applies here too - RSX
  method writes aren't calls, so a call-graph search won't find it.
- **The actual PSARC entry.** No archive has been searched for a
  `shadow.stencilvolume` file; this pass is entirely `EBOOT.elf` static
  reading. Finding and decoding the real file is what would turn the record
  shape above from a hypothesis into a closure test, the way
  `shadow_occluder_ground_truth.rs` closes Pulse's.
- **Whether `0x003ee398`/`0x005ee7d0` are shadow-exclusive.** The vtable/data
  xref on each (`0x0088c728`, `0x008a1808`) was not followed to its owner, so
  "this loader is generic, shadow.stencilvolume is one caller among others"
  is not ruled out.

## Open

- Where in the PS3 PSARC archives (if anywhere) an actual
  `shadow.stencilvolume` entry lives - unsearched
- The exact meaning of the two `u16` counts and the 24-byte record layout
  `0x005ee7d0` parses - hypothesis only, confidence 60
- Who calls the registered `LiveStencilShadow` technique per object, and
  whether `lightDirection`/`extrusionDistance` are ever set to anything other
  than a default - the draw path is unread
- Whether `0x003ee398`/`0x005ee7d0` are reused for resource kinds other than
  this one, via the vtable slots at `0x0088c728`/`0x008a1808`
