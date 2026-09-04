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

The resulting path is passed to `0x003ee398` (not renamed - one known direct
call site). It is a `Crc32`-hashed find-or-load cache: a red-black-tree
lookup by the path's hash, returning a cached handle on hit; on miss it opens
and reads a stream (`FUN_00679ec8`/`FUN_00679ed8`) and parses the result with
`0x005ee7d0` (also not renamed) into a newly allocated 64-byte block. The
returned handle is stored at **model-instance offset `+0x128`**.

*A dead end from the first pass, caught before it was written down as
evidence*: both functions have a data cross-reference (`0x0088c728` for
`0x003ee398`, `0x008a1808` for `0x005ee7d0`) that first read as a shared
vtable/dispatch slot, implying possible reuse for other resource kinds.
Checked with `scripts/ps3-toc.py u32`: each resolves to exactly `{function
address, 0x008bd3c4}` - an ordinary PPC64 **OPD descriptor**, the
`{code, TOC}` pair every function in this ELF has (see this directory's own
`README.md`). It is not evidence of a shared dispatch table at all, and
"maybe reused elsewhere" below is a live open question, not a lead.

## The parsed shape rhymes with Pulse's occluder payload; the bytes don't match it

`0x005ee7d0`'s parse loop, read directly off its decompile rather than
inferred from variable names: an outer repeat-count taken from the resource
buffer's own first `u32`. Per iteration, a header of **two `u32` fields**
(not `u16` - both are read through a `uint *`), followed by a third leading
`u32` word whose role is unread, then (if the first header field, `n`, is
positive) a loop of `n` 24-byte (6-`f32`) records: three of the six floats
per record (at byte offsets `0xc`/`0x10`/`0x14` within the record) feed a
running component-wise min/max - a bounding box - while the whole record is
also copied/transformed into a second buffer whose own size and pointer come
from a sub-call, `0x005ee2c0(scratch, n, m)`, not itself decompiled this
pass. After the `n` records, a further block sized by the header's **second**
field, `m` - as a **word count** (`m << 2` bytes), not as a record count of
`n`'s kind - is `memcpy`'d out separately. The outer loop then advances past
both to the next iteration's header.

That is the same coarse shape as the PSP `DynamicShadowOccluder` `.vex`
payload documented in [shadow-occluder.md](../psp-pulse-usa/shadow-occluder.md)
- small leading counts, a geometry array, a derived bounding box - **but
neither the same byte layout nor, on this reading, the same relationship
between the two counts**: Pulse's `n` (faces) and `m` (vertex slots) each
index their own same-shaped record array; HD's `n` indexes 24-byte geometry
records while `m` sizes an unrelated following word-blob. Confidence 50 on
this whole paragraph (hypothesis, not layout) - `0x005ee2c0` is unread and is
exactly what would resolve what the second buffer and the `m`-sized blob
actually are.

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
- **`0x005ee2c0`, the sub-call `0x005ee7d0` makes with `(n, m)`.** Not
  decompiled this pass; it is what actually sizes and locates the second
  buffer and the `m`-word blob, and is the most direct way to firm up the
  record-format paragraph above past confidence 50.

## Open

- Where in the PS3 PSARC archives (if anywhere) an actual
  `shadow.stencilvolume` entry lives - unsearched
- What `0x005ee2c0` does, and therefore what `m` actually sizes and what the
  second (transformed-copy) buffer holds - confidence 50 without it
- Who calls the registered `LiveStencilShadow` technique per object, and
  whether `lightDirection`/`extrusionDistance` are ever set to anything other
  than a default - the draw path is unread
- Whether the path-join truly resolves to a fixed, non-per-model directory
  entry - confidence 65, resting on six unverified string-utility readings
