# Lighting: `AmbientLight`/`DirectionalLight` registration, `PointLight`'s absence, and where ambient actually goes

First pass under the reversed target-of-record policy (see `corroboration.md`'s
dated notice) - investigated in `/psp-pulse-eu/BOOT.BIN` first, cross-verified
against `/psp-pulse-usa/BOOT.BIN`. Opened to settle the M6 roadmap item on
`AmbientLight` `0x12c`, `DirectionalLight` `0x131` and `PointLight` `0x132`,
whose payloads [`docs/formats/lighting.md`](../../../formats/lighting.md)
decodes from shipped data alone. This page is the handler-recovery half.

## Registration: two of three classes have a `Vex_RegisterClass` site, one does not

All 46 `Vex_RegisterClass` call sites were enumerated on both binaries (via
xrefs to `Vex_RegisterClass` itself - EU `0x08908838`, USA `0x08908eb8` -
following the established rule that its **second** argument is the class id)
and every one individually decompiled to read that argument. `Vex_RegisterClass`
takes `(method_table_address, class_id)`, confirmed directly from
`FogCube_RegisterClass`'s body (`Vex_RegisterClass(0x874f8, 0x3d3)`, matching
the already-documented `fogCube` class id).

### `AmbientLight_RegisterClass` (`0x0892d960`)

| | |
| --- | --- |
| **Address** | `0x0892d960` (EU) |
| **Confidence** | **90** |

```c
void AmbientLight_RegisterClass(void)
{
    Vex_RegisterClass(0x8a840, 300);  // 300 = 0x12c
    ...
}
```

Cross-verified: USA's counterpart is `0x0892de84`
(`Vex_RegisterClass(&DAT_08b65c90, 300)`), found the same way among USA's own
46 call sites. Both binaries agree on the class id as a decimal literal `300`
rather than the hex form other sites use - not itself informative, just how
this particular compilation emitted it. Decompilation is unambiguous and the
finding is corroborated on a second binary (per the confidence rubric, that
combination sits in the 85-94 band; not runtime-verified, so not higher).

### `DirectionalLight_RegisterClass` (`0x08934fc8`)

| | |
| --- | --- |
| **Address** | `0x08934fc8` (EU) |
| **Confidence** | **90** |

```c
void DirectionalLight_RegisterClass(void)
{
    Vex_RegisterClass(0x8ad50, 0x131);
    ...
}
```

Cross-verified: USA's counterpart is `0x089354ec`
(`Vex_RegisterClass(&DAT_08b661a0, 0x131)`). Same evidence standard as above.

### `PointLight` (`0x132`): no registration site on either binary

**Not renamed - there is no function to name.** Every one of the 46 call sites
on both EU and USA was read (including `Collision_RegisterNodeClasses`'s five
separate calls, checked individually: `0x3b9`, `0x3e6`, `999`/`0x3e7`, `0x3ba`,
`0x3cd` - none is `0x132`), and `0x132` does not appear as the second argument
anywhere. This is a counting argument over an exhaustive enumeration, not an
inference from absence of evidence in a sample - the same standard
[`vex.md`](../../../formats/vex.md) already applies to `engine_fire`/
`exitglow`/`gate`, which this finding now joins. `PointLight` is authored (10
instances, all on `13_Track`/`_reversed` - see `lighting.md` under `formats/`)
but the original never calls `Vex_RegisterClass` for it, meaning **the class
has no per-node bind/init handler at all**; whatever placement or colour data
its payload carries is inert on the code side. This rules out (not just
leaves open) any hardware-light-slot consumer keyed off `PointLight` nodes
specifically.

One dead end worth recording so it isn't retried: a 47th apparent call site,
`FUN_0891b75c` (EU), decompiles as `Vex_RegisterClass()` with **no visible
arguments** - not a hidden `PointLight` registration, but a shared two-method
constructor helper whose caller (`FUN_0891d898`, EU) passes it
`(0x88190, 0x3c7)` directly; `FUN_0891b75c` forwards those same registers into
`Vex_RegisterClass` without touching them itself. Confirmed by reading its own
disassembly and its one caller. Not a lighting class.

## The self-address stub: a project-wide pattern, not a lighting one

Every `ClassX_RegisterClass` wrapper populates its class descriptor's two
method-table slots by *calling* a tiny 3-instruction function
(`lui v0,HI / addiu v0,v0,LO / jr ra`) that does nothing but return its own
address - confirmed identical in shape on four independent samples spanning
two classes and two binaries (`AmbientLight`'s and `DirectionalLight`'s shared
first-slot helper `FUN_08a6b344`, `AmbientLight`'s own second-slot helper
`FUN_08a6b350`, and `FogCube_RegisterClass`'s single-slot helper
`FUN_08a6b3bc`). In EU the returned value is the function's **un-rebased**
offset (`0x267350` for the function at `0x08a6b350` - exactly
`0x08a6b350 - 0x08804000`, the PSP image base), matching `corroboration.md`'s
existing finding that this EU rebuild's non-code segments aren't statically
rebased; in USA the same idiom returns the plain absolute address
(`0x08a6bb00` for the function at that exact address, no offset). This is a
**compiler-emitted, project-wide function-pointer materialization idiom**, not
anything specific to lighting - `FogCube_RegisterClass` uses the identical
pattern for its own (already-working, already-implemented) init function.

**This closes off tracing the per-class payload-reading logic through the
stored method pointer directly**: calling through to the address these stubs
return does not reach a function that touches its arguments or reads a
payload - it just re-returns its own address. Stage 1's payload layout
(16-byte `{r,g,b,intensity}` for `AmbientLight`/`DirectionalLight`) is
therefore confirmed **only from shipped data**, not from reading an init
function's body in Ghidra - stated plainly here because the original plan for
this stage expected the opposite. The `DirectionalLight` direction-source
question (parent rotation basis vs. translation) is correspondingly **not
answered by this page**; see `docs/formats/lighting.md`'s own direction-basis
survey for what data alone can say about it.

## Where ambient actually goes: no hardware light slot is ever enabled in the mesh draw path

This is the real find of this pass, reached by following `Gu_Ambient`'s
callers (EU `0x0881116c`) rather than the registration path above, and it
changes what M6's rendering stage should build.

### `Mesh_ApplyMaterialLighting` (`0x0890cea8`)

| | |
| --- | --- |
| **Address** | `0x0890cea8` (EU) |
| **Confidence** | **78** |

```c
void Mesh_ApplyMaterialLighting(int material, undefined4 ctx, int batch, int ambient)
{
    if (/* material flag test, +0xa bits 0x60/0x1c */) {
        if (ambient == -1) {
            Gu_Disable(GU_LIGHTING);           // state 10
        } else {
            Gu_Enable(GU_LIGHTING);
            Gu_Disable(GU_LIGHT0);             // state 0xb
            Gu_Disable(GU_LIGHT1);             // state 0xc
            Gu_Disable(GU_LIGHT2);             // state 0xd
            Gu_Disable(GU_LIGHT3);             // state 0xe
            Gu_Ambient(ambient);
            Gu_Disable(GU_LIGHT0);             // disabled again, unconditionally
            Gu_Disable(GU_LIGHT1);
            Gu_Disable(GU_LIGHT2);
            Gu_Disable(GU_LIGHT3);
            ...
        }
    } else {
        /* prelit / material-colour path - no GU_LIGHTING at all */
        ...
    }
    ...
}
```

Two call sites, both read in full:

- `Mesh_CompileDisplayLists` (`0x0890f5d4`, EU, already documented) calls it
  **twice, both times with the literal `0xffffffff`** (`-1`) - i.e. every
  display list this function ever bakes takes the `Gu_Disable(GU_LIGHTING)`
  branch. Lighting is never baked on at display-list-compile time.
- `FUN_0890ed84` (EU, the real per-frame draw dispatcher, not yet named) calls
  it with `*(int *)(material + 0x6c)` - a genuine per-material runtime value,
  gated on `!= -1` for a display-list caching decision. This is the one call
  site where the `Gu_Enable(GU_LIGHTING)` branch can actually run.

**Even in the one branch where lighting turns on, all four hardware light
slots (`GU_LIGHT0..3`, GE states `0xb..0xe`, already pinned via
`Trail_BuildStateList`) are explicitly disabled - twice, once before the
ambient write and once after.** Only the ambient term ever reaches the GE in
this path. The seven sibling display-list builder functions
`Mesh_CompileDisplayLists` and `FUN_0890ed84` both call for the other cached
list slots (`FUN_0890ca80`, `FUN_0890d004`, `FUN_0890cbc8`, `FUN_0890d174`,
`FUN_0890d240`, `FUN_0890d324`) were each individually decompiled and read in
full: **none of them calls `Gu_Enable` with `0xb`, `0xc`, `0xd` or `0xe`
either.** Not sampled - all seven read.

### `Mesh_ApplyShinemapReflection_q` (`0x0890d8d4`)

| | |
| --- | --- |
| **Address** | `0x0890d8d4` (EU) |
| **Confidence** | **55** (`_q`) |

`Gu_Ambient`'s second caller in the mesh block, found the same xref sweep.
Builds a `Gu_SetMatrix(3, ...)` texture matrix from a VFPU sine/cosine
computation over a per-node reflectivity byte
(`*(byte *)(*(int *)(material+0x58)+0xe)`) - reads as the reflective/chrome
material pass the roadmap's M6 list already flags as open (`"the 0x2000 extra
pass and its second texture index at material +0x08 - it lands on exactly the
*_shinemap textures"`), hence the name and the `_q`: the shinemap
identification is inferential (matches the roadmap's own description) rather
than confirmed against a real `*_shinemap` texture load. Its own ambient
colour comes from **the same kind of single global RGB triple** as below, not
from a per-material field - three consecutive floats at EU `_DAT_002b90c4`
(R) / `_DAT_002b90c8` (G) / `_DAT_002b90cc` (B), USA `DAT_08abf494` (R) /
`DAT_08abf498` (G) / `DAT_08abf49c` (B). Confirms `Mesh_ApplyMaterialLighting`
is not a one-off: **two independent draw-time consumers both read ambient as
one flat global colour**, neither ever touches `GU_LIGHT0..3`.

### The finding, stated plainly

**Across every function in the mesh draw path that this pass could reach from
`Gu_Ambient`'s own callers, the PSP GE's four hardware light slots are never
enabled.** `AmbientLight` feeds a single active ambient term (a global RGB
triple, not a per-instance value threaded through per light node - consistent
with ~1.85 `AmbientLight` nodes authored per track file on average, per
`docs/formats/lighting.md`'s density histogram: only one can be "active" at
once through this mechanism). `DirectionalLight`, despite being genuinely
registered (unlike `PointLight`), has **no consumer found in this pass** -
open, not ruled out, since this pass followed `Gu_Ambient`'s xrefs rather than
`DirectionalLight`'s own (unreachable, per the stub-pattern section above)
init function.

**This reframes the M6 rendering question this pass was opened to answer.**
The premise going in was "13_Track authors 8 lights against 4 hardware slots,
so something selects 4 of them" - that premise assumed hardware per-light
`GU_LIGHT0..3` usage exists somewhere in the mesh path, and every consumer
this pass could reach says it does not. The evidence does not yet rule out a
consumer entirely outside the mesh-material draw path (a different subsystem
reading `DirectionalLight`/`PointLight` for something other than GE hardware
lighting), but no such consumer was found, and the two real ambient consumers
found instead are flat, single-value, and never touch the light slots.

## Open

- **The global ambient RGB triple's writer is unfound.** `get_xrefs_to` on
  both the EU (`_DAT_002b90c4`) and USA (`DAT_08abf494`) addresses returns
  only the one read each already cited above - no writer xref, on either
  binary. Likely written through a computed base+offset Ghidra's static
  analysis doesn't trace (the same class of miss `Xml_ReadGlobalSettings`'s
  table writes already produce elsewhere in this project), not evidence the
  value is never written. Whether this global is populated from an
  `AmbientLight` node's decoded colour, and by what selection rule if more
  than one is authored per file, is the natural next step and the one that
  would actually confirm (or refute) that the format-decoded `AmbientLight`
  payload reaches this exact value.
- **`DirectionalLight`'s consumer, if any, is unfound.** Registered but no
  reader found by this pass's method (following `Gu_Ambient`'s xrefs, which
  only surfaces ambient consumers by construction). A future pass following
  `DirectionalLight_RegisterClass`'s method-table entries a different way -
  or a live capture toggling a track's authored directional lights and
  diffing the rendered frame - would settle whether it's consumed at all.
- **`material+0x6c`'s own writer is unfound**, same class of gap as the global
  triple above - `Mesh_ApplyMaterialLighting`'s real per-frame call site reads
  it, but nothing in this pass traced where it's set.
- **`PointLight`'s payload is inert on the code side**, confirmed rather than
  assumed - see the registration section above. Its trailer field (`{1,0,0,0}`
  on every sample, per `docs/formats/lighting.md`) has no consumer to explain
  it, because there is no consumer.
