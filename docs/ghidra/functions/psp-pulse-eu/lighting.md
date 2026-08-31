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

**Three call sites, not two** - corrected 2026-08-31 (sixth pass, below) via
`search_instructions` on the un-rebased `jal` target (`0x00108ea8`), the same
enumeration standard this page uses elsewhere rather than trusting a count
arrived at by inspection alone:

- `Mesh_CompileDisplayLists` (`0x0890f5d4`, EU, already documented) calls it
  **twice, both times with the literal `0xffffffff`** (`-1`) - i.e. every
  display list *this function* bakes takes the `Gu_Disable(GU_LIGHTING)`
  branch. **Scoped 2026-08-31 (seventh pass, below): this is true of
  `Mesh_CompileDisplayLists` specifically, not of display-list compilation in
  general** - `FUN_08930290` is a second, previously unknown baking function
  that passes the genuine per-material value instead, so a compiled list
  *can* carry `Gu_Enable(GU_LIGHTING)`. The M6 conclusion is unaffected
  either way: `Mesh_ApplyMaterialLighting` disables all four hardware light
  slots itself, regardless of which caller reaches that branch - see below.
- `FUN_0890ed84` (EU, the real per-frame draw dispatcher, not yet named) calls
  it with `*(int *)(material + 0x6c)` - a genuine per-material runtime value,
  gated on `!= -1` for a display-list caching decision. This is a call site
  where the `Gu_Enable(GU_LIGHTING)` branch can actually run.
- `FUN_08930290` - found 2026-08-31, scanned but not fully read (it is large,
  roughly 200 decompiled lines, and not otherwise characterized on this
  page). Calls it twice, both times with `*(undefined4 *)(iVar + 0x6c)` - the
  identical genuine-runtime-value pattern `FUN_0890ed84` uses, not the `-1`
  literal `Mesh_CompileDisplayLists` uses. **This means the branch that turns
  lighting on is reachable from at least two functions, not the one this page
  previously named** - correcting the "one call site" phrasing below.

**Even in the branches where lighting turns on, all four hardware light
slots (`GU_LIGHT0..3`, GE states `0xb..0xe`, already pinned via
`Trail_BuildStateList`) are explicitly disabled - twice, once before the
ambient write and once after.** Only the ambient term ever reaches the GE in
this path, in both `FUN_0890ed84` and `FUN_08930290`. The seven sibling
display-list builder functions `Mesh_CompileDisplayLists` and `FUN_0890ed84`
both call for the other cached list slots (`FUN_0890ca80`, `FUN_0890d004`,
`FUN_0890cbc8`, `FUN_0890d174`, `FUN_0890d240`, `FUN_0890d324`) were each
individually decompiled and read in full: **none of them calls `Gu_Enable`
with `0xb`, `0xc`, `0xd` or `0xe` either.** Not sampled - all seven read.
`FUN_08930290`'s own sibling list slots, if it has any, were not checked -
see "A sixth pass" below.

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
enabled.** The two consumers found - `Mesh_ApplyMaterialLighting` and
`Mesh_ApplyShinemapReflection_q` - both read ambient from **the same flat
global RGB triple**, not a per-instance value threaded through any light
node. `DirectionalLight`, unlike `PointLight`, is not just registered but
**live-confirmed to be collected into a real, correctly-populated per-track
list** (`World_CollectMarkerLists`, below) - its final render-side consumer
is still **not found in this pass**, open rather than ruled out, since this
pass followed `Gu_Ambient`'s xrefs rather than `DirectionalLight`'s own
(unreachable, per the stub-pattern section above) init function.

**Live-verified (2026-08-05): `AmbientLight` does not feed that global at
all.** A PPSSPP capture (headless/Xvfb, no window shown - see
`docs/reverse-engineering/ppsspp-debugger.md`) read the global's three floats
at five points across two structurally different tracks: fresh boot, three
checkpoints racing Talon's Junction (`16_Track`), two checkpoints racing Moa
Therma (`03_Track`). **Every single read came back `(0.2, 0.8, 1.0)` - the
compiled-in static default, unchanged.** Talon's Junction authors `AmbientLight`
`(0.169, 0.193, 0.207)`; Moa Therma authors `(0.297, 0.258, 0.217)` and
`(0.440, 0.375, 0.348)` on its two instances - neither remotely close to what
was actually read, on two tracks chosen precisely because their authored
values differ sharply from both the default and each other. This retracts the
weaker "AmbientLight feeds a single active ambient term" reading the static
analysis alone supported: **the global is a fixed engine constant, not a sink
for any authored `AmbientLight` data**, at the confidence rubric's runtime-trace
tier rather than the decompilation-only tier the rest of this page sits at.

**This reframes the M6 rendering question this pass was opened to answer.**
The premise going in was "13_Track authors 8 lights against 4 hardware slots,
so something selects 4 of them" - that premise assumed hardware per-light
`GU_LIGHT0..3` usage exists somewhere in the mesh path, and every consumer
this pass could reach says it does not. The evidence does not yet rule out a
consumer entirely outside the mesh-material draw path (a different subsystem
reading `DirectionalLight`/`PointLight` for something other than GE hardware
lighting), but no such consumer was found, and the two real ambient consumers
found instead are flat, single-value, and never touch the light slots.

## `DirectionalLight` is actively collected at track load, capped at exactly 4

Found by following a different thread than `Gu_Ambient`'s callers: the
self-address-stub value each class's method table stores (see above) is not
just inert bookkeeping - `World_CollectNodeLists` (documented separately)
uses the exact same stub-return values as **class filter keys**, walking the
node tree and comparing each node's stored method pointer against a known
class's stub value to build a per-class instance list. The same pattern
exists for lighting.

### `World_CollectMarkerLists` (`0x0887a1c4`)

| | |
| --- | --- |
| **Address** | `0x0887a1c4` (EU) |
| **Confidence** | **85** |

Called once from `World_LoadTrack` (`0x088835f0`, track-load time - the same
timing `World_CollectNodeLists` uses for pads/fog). Builds **four** typed
lists this way, each with its own fixed capacity:

| Class (via its stub) | World field (count / list) | Capacity |
| --- | --- | --- |
| `AmbientLight` (`FUN_08a6b350`) | `+0x3c` / `+0x54` | 10 |
| `DirectionalLight` (`FUN_08a6b35c`) | `+0x40` / `+0x7c` | **4** |
| `0x3cf` "wopoint" (`FUN_08a6b368`) | `+0x4c` / `+0x8c` | 200 |
| `0x3ce`, unidentified (`FUN_08a6b374`) | `+0x50` / `+0x3ac` | 200 |

**`DirectionalLight`'s capacity is exactly 4 - the PSP GE's own hardware
light-slot count.** Not a coincidence this page treats lightly: `AmbientLight`
collected alongside it in the same function caps at 10, and the two unrelated
marker classes cap at 200, so 4 is not a generic small-list default this
function uses everywhere - it is specific to the one class whose count
happens to match the hardware constraint this whole investigation opened
around. This is real, positive evidence the original **does** track a
bounded set of directional lights per track, in a shape a hardware-light
consumer could use directly.

**Live-verified 2026-08-05: the collection genuinely runs, on real per-track
data.** A PPSSPP breakpoint at this function's entry (`0x0887a1c4`), reading
`$a0` (the world pointer) and then the four count fields after the call
returns, on Talon's Junction (`16_Track` - not `13_Track`; corrected below)
produced:

| Offset | Field | Live value | `oag-view --nodes` on `16_Track/track.vex` |
| --- | --- | --- | --- |
| `+0x3c` | `AmbientLight` count | **1** | **1** |
| `+0x40` | `DirectionalLight` count | **3** | **3** |
| `+0x44` | unaccounted (see Open) | 0 | - |
| `+0x48` | unaccounted (see Open) | 0 | - |
| `+0x4c` | `wopoint` count | **21** | **21** |
| `+0x6cc` | sum of the above | 25 | 1+3+0+0+21+0 = 25 ✓ |

Three independent per-class counts match the disc's own authored data
exactly, not just the one this pass cares about - this is the strongest
evidence tier this project's rubric has for "this function does what its
decompile says," short of a second binary. Confidence raised from 65 (`_q`)
to **85** and the name dropped its `_q`: what remains open is not whether
this function collects real data (settled), but whether anything downstream
*reads* the list it built. The breakpoint fired 9 times total during one
track load, cycling through this function's two other call sites
(`0x08900bb8`, `0x08900e1c`) as well as the primary `World_LoadTrack` one
(`0x088848e4`) - the other two returned all-zero counts against different
root-node pointers (front-end/menu scene graphs, not the raceable track),
consistent with the same function being reused generically rather than
being lighting-specific, which is also why the name stays
`World_CollectMarkerLists` rather than something lighting-specific.

**A capture trap worth recording**: breaking directly at the `jal`
instruction (`0x088848e4`) and reading `$a0` immediately reads the stale
pre-delay-slot value - MIPS o32 loads the argument in the delay slot
(`move a0,a1`), which has not executed yet at that `pc`. Breaking at the
*callee's* entry instead (`0x0887a1c4`) reads the correct, already-loaded
`$a0`.

`World_LoadTrack` itself is 1,228 instructions with 169 calls - tracing
which of them reads world `+0x7c`/`+0x40` afterward, or whether a per-frame
function reads it later (the collection happens once at load, not
necessarily applied then), is real, bounded follow-up work this pass did
not have room for. This is the strongest lead toward `DirectionalLight`
having a real consumer that this project has found - stronger than "no
consumer found" alone, and now stronger still given the collection itself
is live-confirmed correct - but it is not yet the consumer itself.

### The object itself, and three passes that ruled out every lead but one

**A live capture dumped one of the three live `DirectionalLight` node
pointers in full** (`world+0x7c`'s three non-null entries were
`0x097a5c10`, `0x097a5d10`, `0x097a5e10`, spaced exactly `0x100` apart - a
uniform pool, not inline structs). The third, dumped to 256 bytes,
confirmed:

- `+0x04` = `0x08a6b35c` - the `DirectionalLight` class stub, matching the
  filter key `World_CollectMarkerLists` uses.
- `+0x44` = `0x00000131` - the `DirectionalLight` class id, `0x131`.
- `+0x60..0x6c` = a unit-length float vector, `(-0.967, -0.259, -0.024, 0)`.
- `+0x70..0x7c` = `(0.193, 0.238, 0.290, 1.0)` - matches the documented
  `{r, g, b, intensity}` payload shape exactly.

**The object's real allocated size is `0x80` (128) bytes, not 256** -
found via its constructor pair, `DirectionalLight_Construct` (`0x08934d94`,
confidence 88) and `DirectionalLight_Init` (`0x08934cb0`, 85):
`FUN_0894691c(0x80, ...)` allocates exactly 128 bytes, zeroes them, calls
the init function, then tags `+4` with the class stub - the standard
node-construction shape this project already knows from elsewhere (`Vex_LoadModel`'s own
callers use the identical pattern). `AmbientLight` has the same pair,
`AmbientLight_Construct`/`AmbientLight_Init` (`0x0892d7ec`/`0x0892d708`,
88/85), allocating `0x70` (112) bytes - exactly 16 less, exactly the size
of the direction vector `DirectionalLight` carries and `AmbientLight`
doesn't. Both classes' payloads end precisely at their own allocation
boundary; not a coincidence in one sample, the same pattern twice.

**This retires the live capture's one ambiguous lead.** A breakpoint on
`FUN_0890ed84` (the per-frame draw dispatcher) caught one hit whose `$a0`
landed at `node_base + 0x90` on a live `DirectionalLight` node - past the
documented payload, in what looked like unmapped territory. It is 16 bytes
past the object's real 128-byte end: the 256-byte dump over-read into
whatever the pool allocator placed next in that slot. The touch was real,
the object it looked like it belonged to wasn't.

**The direction vector at `+0x60..0x6c` is itself worth flagging to
whoever owns [`docs/formats/lighting.md`](../../../formats/lighting.md)'s
open "rotation row vs. translation row" question**: the on-disk payload
`AmbientLight`/`DirectionalLight` share is 16 bytes,
`{r, g, b, intensity}` - no direction field. A direction vector existing in
the *runtime* object at all means something computes it once, at
construction or track-load time, from the node's world transform - which
is direct evidence a direction is read from the matrix, just not yet which
row. `DirectionalLight_Init` is the obvious next place to look for that
specific read, if this thread continues.

**A second, unrelated structural finding surfaced along the way**:
`FUN_08a6b5bc`/`FUN_08a6b5c8` (EU) looked like they might be more
lighting-adjacent class stubs, since they sit in the same self-address-stub
block. They are not - `FUN_08a6b5bc` keys the **World/marker-container
object itself** (`FUN_0887a0fc` constructs one, zeroing exactly the fields
`World_CollectMarkerLists` later fills: `+0x3c/0x40/0x44/0x48/0x4c/0x50/0x6cc`),
and `FUN_08a6b5c8` keys a second, larger (`0xc80`-byte) child scene object
`Race_CreateModeObject` builds right after, via `FUN_08886950` - which is
itself the function that calls `World_LoadTrack`. Neither reads
`DirectionalLight`'s fields, so neither is a lighting class - but
**correction, "A fifth pass" below**: `FUN_08a6b5bc` specifically turns out
to be load-bearing for this thread anyway, as the class key that identifies
the World/marker-container object itself. A future pass shouldn't skip it
for the reason this paragraph originally gave.

**Exhaustively checked: `DirectionalLight`'s class stub (`0x08a6b35c`) has
exactly 5 references in the whole binary**, all now read - the collector
(`World_CollectMarkerLists`), the constructor pair above,
`DirectionalLight_RegisterClass`, and the stub's own self-reference. No
sixth reference exists anywhere that identifies a `DirectionalLight` node
*by its class*. Combined with a one-hop check of `FUN_0890ed84` (no
`+0x40`/`+0x7c` read, any depth) and every sibling display-list-builder
function (`FUN_0890ca80`, `FUN_0890d004` and its own three previously
unexpanded callees, `FUN_0890cbc8`, `FUN_0890d174`, `FUN_0890d240`,
`FUN_0890d324` - none reads `+0xc4` or does an independent class-tagged
tree walk either), three independent passes (one live capture, two Ghidra)
have now ruled out every lead that doesn't require a direct, non-class-keyed
read of `world+0x7c` - which puts the only remaining path back inside
`World_LoadTrack`'s own 1,228 instructions, the trace this thread has
twice judged not worth it given known tooling blind spots on that specific
function. Recorded as a genuine, converged negative result, not an
abandoned search.

### A fourth pass: `World_LoadTrack`'s own body is now disassembly-verified to hold no consumer, and its name is doubtful

2026-08-31. Took the "trace `World_LoadTrack`'s own 1,228 instructions" lead
the two prior passes judged not worth it, and closed it - but at the same
time found the function's own name is likely wrong, so the result is a
converged negative on a differently-shaped question than the one asked.

**`World_LoadTrack_q` (`0x088835f0`, confidence 55 per its `_q` suffix) is
called from `FUN_08886950` as a generic `0x3c`-byte (60-byte) object
constructor, not as a function handed "the world."** Disassembling
`FUN_08886950` in full shows it building a run of sibling scene objects, each
via the identical alloc/zero/tag/construct idiom already documented above for
`DirectionalLight_Construct`/`AmbientLight_Construct`
(`func_0x0014291c(size, ...)` to allocate, zero it, tag `+8` with the parent,
then call the class's own construct function on the fresh object). The very
last one in that run allocates exactly `0x3c` bytes and calls
`func_0x0007f5f0` on it (`0x08886d0c`: `jal 0x0007f5f0`, `a0` set to the fresh
allocation two instructions earlier at `0x08886d00`) - and `0x0007f5f0 +
0x08804000 = 0x088835f0`, `World_LoadTrack_q`'s own address. `World_LoadTrack_q`'s
own body never writes past `param_1+0x38`, which fits inside 60 bytes -
consistent with it being this constructor's target, not a function operating
on the world object (`DirectionalLight`'s own collected list capacity lives
at `world+0x40`/`+0x7c`, both past a 60-byte object's end - the same
over-read shape as the `node_base+0x90` false lead retired above, this time
caught before it produced one). Recorded as a hypothesis worth a future
pass's attention, not a rename: no replacement name is anywhere near the
50-confidence floor this project requires before touching a name at all.

**With that reframing, the call this page has tracked throughout
(`0x088848e4`, into `World_CollectMarkerLists`) is `World_LoadTrack_q` calling
out to go find the world object, not being handed it** - its own decompiled
body resolves the callee's `$a0` by walking a sibling list rooted at its own
`param_1+8` for a node matching a tag (`func_0x002675bc()`), the same
tag-lookup idiom used elsewhere in this codebase for resource lookup by type,
matching the live capture's confirmation that `$a0` at
`World_CollectMarkerLists`'s entry is genuinely the world pointer at
runtime even though `World_LoadTrack_q` itself is never handed it as an
argument.

**The disassembly-verified part, independent of decompiler trust**: the
instructions immediately after the `jal` at `0x088848e4` (`0x088848e8` through
the function's end at `0x0888491f`) are register restores (`lw s0..s7,fp,ra`)
and `jr ra` - pure epilogue, no memory read of any kind. This was checked in
raw disassembly, not the decompiler's text, specifically because the
decompiler is demonstrably unreliable on this exact call (see below) and its
prose summary should not be trusted for a tail this load-bearing. **This
closes the "trace `World_LoadTrack`'s own instructions" lead with a genuine
negative**: whatever function's name turns out to be right for `0x088835f0`,
nothing in its body reads `world+0x40`/`+0x7c` after the collection call - the
consumer, if one exists, is not here and cannot be, structurally.

**Why the decompiler couldn't be trusted for this call, and why `get_xrefs_to`
returned nothing for it**: `World_CollectMarkerLists`'s own address
(`0887a1c4`) has zero recorded references in the database
(`get_xrefs_to`/`get_function_callers` both return empty), even though the
call at `0x088848e4` demonstrably targets it - the decompiler prints the call
as `func_0x000761c4(...)`, an unresolved low pseudo-address, and
`0x000761c4 + 0x08804000 = 0x0887a1c4` exactly. This is not a new defect:
it is the same `jal` relocation wart `docs/ghidra/workflow.md`'s "Known-imperfect,
and why it does not block" section already documents from
`docs/ghidra/functions/psp-pulse-usa/billboards.md`'s
`Billboard_ConstructResource` finding (`real = pseudo + 0x08804000`, `R_MIPS_26`
not applied even where the `lui`/`addiu` pair is fixed) - this page's own
find is a second binary (`psp-pulse-eu`, not `psp-pulse-usa`) and a second,
unrelated function showing the identical shape, which corroborates that
finding rather than adding a new one. Worth recording here specifically
because it explains why `get_xrefs_to`/`get_function_callers` came back empty
for `World_CollectMarkerLists` despite the decompiled body and the live
capture agreeing it is genuinely called: **the reference database itself is
silent on this class of call, not just the printed constant** - a future pass
chasing "no callers found" on an EU function should check for this pattern
(a `func_0x0000XXXX`/`func_0x000XXXXX` call target under `0x00800000`) before
concluding the function is actually uncalled.

### A fifth pass: `Race_CreateModeObject` confirms the exact object chain, and `FUN_08886950`'s own `param_1` is one hop short of the world object

2026-08-31, same session. The fourth pass's Open update above asked whether
`FUN_08886950`'s own `param_1` is the object `World_CollectMarkerLists`
receives. Decompiling `Race_CreateModeObject` (`0x08821038`, already named,
confidence not separately recorded on this page before now) answers it
directly - it is the function that builds both objects "A second, unrelated
structural finding" above only inferred the class keys for:

```c
void Race_CreateModeObject(int param_1)
{
    ...
    iVar2 = func_0x0014291c(0x6d0, ...);      // alloc 0x6d0 bytes
    *(int *)(iVar2 + 8) = param_1;             // tag: parent = Race_CreateModeObject's own param_1
    func_0x000760fc();                         // ctor: 0x000760fc + 0x08804000 = 0x0887a0fc = FUN_0887a0fc
    *(undefined4 *)(iVar4 + 4) = func_0x002675bc();  // class tag = FUN_08a6b5bc (World/marker-container)
    ...
    iVar2 = func_0x0014291c(0xc80, ...);      // alloc 0xc80 bytes
    *(int *)(iVar2 + 8) = iVar4;               // tag: parent = the 0x6d0-byte object just built
    func_0x00082950();                         // ctor: 0x00082950 + 0x08804000 = 0x08886950 = FUN_08886950
    *(undefined4 *)(iVar4 + 4) = func_0x002675c8();  // class tag = FUN_08a6b5c8 (the 0xc80-byte scene object)
    ...
}
```

This nails down, with a real allocation size for the first time
(`0x6d0`/1,744 bytes for the World/marker-container object, alongside the
already-documented `0xc80` for its child), exactly the chain "A second,
unrelated structural finding" above only inferred from the two class stubs:
**the World/marker-container object is built first, and the `0xc80`-byte
scene object `FUN_08886950` constructs (its own `param_1`) is tagged as that
World object's own *child*, with its own `+8` field set to point back at the
World object.**

**This answers the fourth pass's question: no, `FUN_08886950`'s own `param_1`
is not the world object - it is one hop short of it.** The world object is
`*(FUN_08886950_param_1 + 8)`. Since the fourth pass already read
`FUN_08886950`'s full body and found it never reads that `+8` field for its
own purposes (it only ever writes to its own object and tags/registers its
own children), `FUN_08886950` is ruled out as the consumer for this reason
specifically, not merely "checked and found nothing" - it never resolves the
world pointer at all, so it structurally cannot read fields on it.

**A object-scoped global exists for the `0xc80` object, and three of its
readers were sampled and ruled out - but this global is not the route to the
world object either, so this does not narrow the search.** `FUN_08886950`
stashes its own `param_1` (the `0xc80` object, not the world object) into
`_DAT_00059838` (`*(_DAT_00059838 + 8)` would reach the world object, one
hop further). `get_xrefs_to` on `0x00059838` cleanly returns 15 references
(14 real reads plus `FUN_08886950`'s own write - this works because it is a
data address, not a `jal` target, so it doesn't hit the wart above). Three
were decompiled to see the shape of what this global's readers do:
`FUN_08887ad4` reads `_DAT_00059838 + 0xb40` (a view-matrix upload,
`Gu_SetMatrix`-shaped); `FUN_0891e4ac` and one branch inside the fog-setup
function at `0x08905b08` both call `Fog_FindVolume` (`0x088875c0`, already
named) directly on `_DAT_00059838`, which reads *that* object's own
`+0xb3c`/`+0x81c`/`+0xc50` fields (a per-object fog-volume list) - toggling
`GU_FOG` (state 7) and computing fog GE constants respectively. **All three
read fields of the `0xc80` object itself, none dereferences its `+8`** - so
this samples as evidence the global isn't how a lighting consumer would
reach the world object, not as a cleared search. The other 11 xrefs were not
decompiled.

**The bounded search this converges on**: every function that resolves the
world object *by its class* must call `FUN_08a6b5bc` (`0002675bc`
un-rebased) the same way `Race_CreateModeObject` and `World_LoadTrack_q` do -
this is the same standard the doc already applied to `DirectionalLight`'s
own class stub ("exactly 5 references... all now read").

### A sixth pass: the class-key enumeration, and a new lead in an already-documented function

2026-08-31, same session, immediately following the fifth pass above.
`get_xrefs_to` on `0x08a6b5bc` returns empty, as expected - it is another
`jal` target and hits the same wart. `search_instructions` on the un-rebased
operand (`jal 0x002675bc`) is not subject to that wart (it matches on the
raw operand text, not the reference database) and finds **12 call sites
across 10 unique functions**, program-wide - the union of the two methods is
just this list, since `get_xrefs_to` contributed nothing. `World_LoadTrack_q`'s
own class-key call sits at `0x088848b0`, only `0x34` bytes before its `jal`
to `World_CollectMarkerLists` at `0x088848e4` - independent, address-level
corroboration of the tag-walk-then-forward reading the fourth pass drew from
its decompile alone:

| Function | Status |
| --- | --- |
| `Race_CreateModeObject` (`0x08821038`) | Already read (fifth pass) - this is a tag *write* (constructing the world object), not a read of its class |
| `FUN_0887a0fc` | The world object's own constructor (already named in "A second, unrelated structural finding" above) - self-tagging at construction (a write, per the page's already-established zeroing of `World_CollectMarkerLists`'s own target fields) is the expected, already-documented shape; not separately decompiled this pass |
| `World_LoadTrack_q` (`0x088835f0`) | Already read (fourth pass) - resolves the world object only to forward it to `World_CollectMarkerLists`; ruled out as a consumer itself |
| `FUN_08900984` (`0x08900984`-`0x08900c53`) | **Decompiled and ruled out this pass** - constructs a *fresh, disposable* `0x6d0`-byte world object of its own (not a lookup of an existing one) and calls `World_CollectMarkerLists` on it as its last real act before an unrelated list-removal bookkeeping step; no read of `+0x40`/`+0x7c` follows. Contains `0x08900bb8`, one of the "two other call sites" this page's live capture already covered (front-end/menu scene graph, all-zero counts) - the decompile now explains *why* those counts were zero (a freshly built, empty scene graph, not the raceable track) rather than merely matching the address |
| `FUN_08900c54` (`0x08900c54`-`0x08900e53`) | **Decompiled and ruled out this pass**, identical shape to `FUN_08900984` - builds its own fresh world object, calls `World_CollectMarkerLists` on it, returns immediately after with no field read. Contains `0x08900e1c`, the other already-covered call site |
| `Mesh_ApplyMaterialLighting` (`0x0890cea8`) | **Already fully quoted on this page (confidence 78) - and its own quoted body contains this exact tag-walk**, missed until this pass connected the two: it caches the resolved world pointer at `material+0xc4`, but the instructions already quoted above never read a field back off that cached pointer - the material-ambient reads that follow it (`param_1+0x70`, `param_1+0x6c`) are the *material's own* fields, not the world object's. **Resolved this pass**: `material+0xc4` is write-only across every known caller. `search_instructions` on the un-rebased `jal` target found this function has **three** call sites, not the two this page previously named (see the correction above `Mesh_ApplyMaterialLighting`'s own quoted body) - `Mesh_CompileDisplayLists` and `FUN_0890ed84` were already read in full and confirmed clean for `+0xc4`, and the newly found third caller `FUN_08930290` - now fully characterized below as a display-list-baking function, not a draw dispatcher - shows no `+0xc4` read either. A blind `lw ...,0xc4(...)` sweep across the whole binary was tried first and abandoned as not viable (`+0xc4` is a common offset - 50+ unrelated matches before the scan even finished); checking the three actual callers by name is what converged |
| `FUN_088b7504` | **Decompiled and ruled out (seventh pass)** - same shape as `Race_CreateModeObject`/`FUN_08900984`/`FUN_08900c54`: constructs its own fresh `0x6d0`-byte world object and tags it with the class key. A tag *write*, not a read. Also writes several unrelated float constants (`0.7`, `0.7`, `20.0`, `100.0`) to its own object - looks like a distance/LOD parameter block for a specific game mode, not examined further |
| `FUN_088b8d00` | **Decompiled and ruled out (seventh pass)**. Calls the class key **three times** (`0x088b8e4c`, `0x088b8fa0`, `0x088b90e8`) but not as a tree-walk of its own - it passes the resolved class key **as a query parameter** into a different helper, `func_0x000b43fc` (un-rebased), a pattern not seen elsewhere on this page. The rest of the function is text/resource lookups gated on a difficulty-like 3-way branch (`param+0x94`/`+0x98`) - reads as a UI/results-text function, not a render one. No `+0x40`/`+0x7c` reference anywhere in the body |
| `FUN_0890e494` | **Decompiled (seventh pass) - and turns out to be the actual writer of `Mesh_ApplyMaterialLighting`'s `material+0xc4` cache**, not a reader of `DirectionalLight`. It runs the identical tag-walk and writes the result straight to `param_1+0xc4` at what reads as material/mesh-node construction time; `Mesh_ApplyMaterialLighting`'s own tag-walk (already documented above) is a lazy-init fallback for the case this hasn't run yet, not the primary writer. Confirms rather than reopens the "write-only" finding above - this is simply *which* function does most of the writing. The rest of the function resolves two more classes into `param_1+0x70`/`+0x74`, does texture-UV scaling math, and ends in the same `+0xdc..+0x108` colour-packing block seen in `FUN_0890ed84`/`Mesh_CompileDisplayLists`/`FUN_08930290` - a generic, reused block, not lighting-specific |
| `Vex_LoadModel` | **Decompiled and ruled out (seventh pass)** - already a named, high-confidence function for a completely different purpose (model loading). Runs the same tag-walk, caching the result at its *own* `+0xac` (a different offset than the material's `+0xc4` - different struct, same idiom) with no field read following it. Corroborates that resolving the world object by class is a **generic, reused idiom** across this codebase, not something lighting-specific |

**Net effect: the class-key enumeration is now fully closed, not partially.**
All 12 call sites across all 10 unique functions have been decompiled this
session or previously - `FUN_0887a0fc` alone was not separately decompiled,
its self-tagging inferred from the page's own established zeroing evidence
for the world object's constructor, above - and **none reads `world+0x40` or
`world+0x7c`**. Six are tag *writes* (constructing a fresh world object, or a
material/model caching the resolved pointer for later, unconsumed use):
`Race_CreateModeObject`, `FUN_0887a0fc`, `FUN_08900984`, `FUN_08900c54`,
`FUN_088b7504`, `Vex_LoadModel`. Two forward the resolved pointer elsewhere
without reading it themselves: `World_LoadTrack_q` (to
`World_CollectMarkerLists`) and `FUN_0890e494` (into `material+0xc4`, for
`Mesh_ApplyMaterialLighting`'s own later, equally fruitless check). One
queries a different helper with the class key as a parameter rather than
walking the tree: `FUN_088b8d00`. This is a genuinely exhaustive negative
result across every function that resolves the world object *by its class*
- the same standard this page already applied to `DirectionalLight`'s own
class stub, now applied to the mechanism a consumer would most plausibly use
to reach it.

**What this enumeration cannot see, precisely stated**: a consumer that
holds the world pointer some other way - passed down as an argument, read
back from a cached field like `material+0xc4` or `Vex_LoadModel`'s own
`model+0xac`, or reached through a global - never calls `FUN_08a6b5bc` itself
and so cannot appear in a search keyed on that call. `FUN_08930290` (below)
is exactly this shape, which is why it survived an otherwise exhaustive
sweep and remains the one open lead.

### An eighth pass: `FUN_08930290` characterized - it is a display-list baker, not a second draw dispatcher, and it is ruled out too

2026-08-31, same session. **Retracting this page's own "second per-frame
draw dispatcher" label from the seventh pass above**: `FUN_08930290` is not
that. Its real callers, found the same way as everywhere else on this page
(`search_instructions` on its un-rebased `jal` target, `0x0012c290`), are
`FUN_0892ee38` (4 sites) and `FUN_0892fcf8` (1 site) - both decompiled in
full this pass.

`FUN_0892ee38` builds a model wrapper from a raw node array: it sorts nodes
into four category lists by material flags (matching
`Mesh_CompileDisplayLists`'s own flag categories, `0x800`/`0x2000`/a
per-node byte's `0x4` bit), then bakes display lists for them via the exact
allocation sequence `Mesh_CompileDisplayLists` itself uses
(`func_0x0000cb28`/`func_0x0000c09c`/`func_0x0000ca9c`/`func_0x0000ca80`/`func_0x00142b20`),
calling `func_0x0012c290(param_1, 1, 7)` for the main categories and,
conditionally, `func_0x0012c290(param_1, 1, 8)` for the fourth. `FUN_0892fcf8`
does the lazy-first-use equivalent: `if (*(param_1+0xac) == 0) { ... call
func_0x0012c290(param_1, 0, 0xf) ... }` - `param_1+0xac` being exactly
`Vex_LoadModel`'s own world-pointer cache field, so this triggers the bake
once, the first time a model's world pointer hasn't been resolved yet, and
does lighter per-frame-shaped maintenance work on later calls instead.

**`FUN_08930290` is therefore a display-list-baking function analogous to
`Mesh_CompileDisplayLists`, for a second, previously undocumented
model-construction pathway - invoked at model build time or on first use,
not every frame.** It is not an alternate to `FUN_0890ed84`: that function's
own "real per-frame draw dispatcher" label stands, corroborated this pass -
`FUN_08910adc` (one of its two callers) forwards to it directly for the
`0x45000000`-tagged mainline case (`func_0x0010ad84(param_1,param_2,param_3)`,
`0x0010ad84 + 0x08804000 = FUN_0890ed84`) and does ghost/overlay rendering
otherwise. The two functions have unrelated roles that happen to both reach
`Mesh_ApplyMaterialLighting`.

**This forces a real scoping correction, not just a relabelling**: this
page's own claim that "lighting is never baked on at display-list-compile
time" was true only of `Mesh_CompileDisplayLists`, corrected above.
`FUN_08930290`'s `mask & 1` branch bakes `Mesh_ApplyMaterialLighting` with
the genuine `*(material + 0x6c)` value, exactly like `FUN_0890ed84` does -
so a *compiled* display list can carry `Gu_Enable(GU_LIGHTING)` after all.
The M6 conclusion is unaffected: `Mesh_ApplyMaterialLighting` disables all
four hardware light slots itself regardless of which of its three now-known
callers reaches that branch, per its own quoted body above.

**`+0xcc`/`+0xd0` are resolved: caller-supplied per-model values, not
lighting.** `FUN_0892ee38` writes them directly from its own arguments
(`*(iVar18+0xcc) = param_4; *(iVar18+0xd0) = param_5;`), and both
`FUN_08930290`'s `mask & 8` branch and `FUN_08910adc`'s ghost/overlay path
read them back and feed them to the same function, `func_0x0011a464`
(un-rebased), alongside a per-node byte and (in the `mask & 8` case)
`Mesh_ApplyShinemapReflection_q`. Two model-level values threaded through
from construction, consumed by a shared drawing helper - the shape reads as
a **per-model tint pair**, which points at (does not confirm) this
project's existing per-team-hull-colour work rather than anything
`DirectionalLight`-related - worth flagging to whoever owns that thread, not
chased further here.

**`FUN_08930290` is ruled out as `DirectionalLight`'s consumer, fully
characterized rather than merely scanned.** Its four mask branches touch:
material ambient (`mask & 1`, via `Mesh_ApplyMaterialLighting`), an
unexamined transform pass (`mask & 2`, `func_0x00109490`), vertex/UV
matrix math (`mask & 4`), and the shinemap-plus-tint pass (`mask & 8`,
above). None reads `world+0x40` or `world+0x7c`. This was the one lead the
class-key enumeration (sixth/seventh passes) could not see by construction
- and it comes up empty too.

### A ninth pass: a live read watchpoint on the runtime addresses, not another static trace

2026-08-31, same session, prompted directly by a question about whether the
eight static passes above could have missed a conditionally-triggered
consumer (a weapon effect, specifically). They could have - none of them
proves a negative for code paths never reached. `docs/reverse-engineering/ppsspp-debugger.md`'s
own memory watchpoint section says plainly: "it is the tool for 'does
anything read this?', which no amount of static sweeping answers as well."
This pass used it.

**Method** (`scripts/psp-watch-directionallight-consumer.py`, new this pass):
broke at `World_CollectMarkerLists`'s USA counterpart
(`0x0887a368` - confirmed by `diff_functions` as a 100% structural match,
98/98 instructions equal, to EU's `0x0887a1c4`; the USA binary was used
because `psp-drive.py`'s menu/restart automation is calibrated for it, not
because EU stops being the target of record) on a fresh `psp-drive.py
restart` of Talon's Junction, read `$a0` for the live world pointer
(`0x0980a1d0`), and armed two non-halting read watchpoints
(`memory.breakpoint.add`, `enabled: false, log: false` - counting only, no
log-flood risk per the doc's own warning) on `world+0x40` (the
`DirectionalLight` count, 4 bytes) and `world+0x7c` (the list, 16 bytes). A
third watchpoint armed a live craft's rigid-body position as the positive
control the doc's own methodology requires - "a watchpoint that never fires
looks exactly like a watchpoint that does not work."

**Result: the control fired 489,461 times; both `DirectionalLight` addresses
fired zero, across driving and three weapons.** Coverage: ~2 minutes of
active driving at speed (throttle 100%, real track progress, hundreds of
units covered on Talon's Junction) plus three of the four weapons
`scripts/psp-fire-weapon.py` wired at the time - `rocket` (three
projectiles, fanned), `burst` (rapid cannon fire) and `backward`
(bomb/mine).

**One transient hit is worth stating precisely rather than glossing over,
because it is exactly the kind of thing that would look alarming out of
context.** A separate run of this same rig (arming the watchpoint
immediately after catching `World_CollectMarkerLists` at its own entry,
before its body had run) caught **one** read of `world+0x40` and **zero** of
`world+0x7c`. This is not an external consumer - it is `World_CollectMarkerLists`
finishing the very call this pass broke into: this page's own "The object
itself" section above already documents that this function "sums `+0x3c`
(`AmbientLight`) + `+0x40` (`DirectionalLight`) + `+0x44` + `+0x48` + `+0x4c`
(`wopoint`) + its own local into `+0x6cc`" - a count-only checksum, never
touching the list at `+0x7c`. One hit on the count and zero on the list is
exactly that signature, not a new consumer; recorded here because a watchpoint
that never explains its own hits is not trustworthy, and this one does.

### Missile and Energy Drain remain untested, and why - a real, reproducible PPSSPP crash, not a shrug

Both weapons are actually already identified with real confidence -
`docs/ghidra/functions/psp-pulse-usa/missile.md`'s id-to-bit table gives
Missile bit `0x40` (confidence 90, `Weapon_FireMissile` named and read in
full) and "10 LeachBeam" bit `0x8000` (settled from a second, independent
direction) - `scripts/psp-fire-weapon.py` simply didn't have them in its
`WEAPONS` dict yet, so this pass called its `fire()` function directly with
the raw bits instead of extending the script.

**Neither bit was ever observed consumed, and both attempts (on two
separate, freshly booted, confirmed-live emulators - the second explicitly
verified racing and gaining control-watch hits seconds before) were
followed by PPSSPP crashing**: `E[MEMMAP] Bad memory access detected!
00000030 ... Stopping emulation`, at the identical JIT block
(`08872f98_z_un_08872f54`, `movaps xmm6,[rbx+rbp]`) both times, roughly four
minutes into each session. `Weapon_FireMissile`'s own decompiled body
(`0x088685cc`) was read to check the obvious hypothesis - that
`psp-fire-weapon.py`'s own `craft+0x1bc = 0xffffffff` write (borrowed from
the rocket path, which has no target) corrupts something a homing weapon
needs - and it doesn't hold up: `Weapon_FireMissile` writes that exact same
value to that exact same field itself, unconditionally, as its second
instruction. Since the fire bit was never observed cleared either (which
`Weapon_FireMissile` also does unconditionally, near the top, before the
crash-implicated code), the simplest reading is that **the handler never
ran at all** for a raw-bit-set craft - `Weapons_DispatchFire` dispatches
something else, or gates on state a direct bit write doesn't satisfy - and
the crash is a separate event, not this function executing and failing.

**Whether the crash is caused by the bit-write attempt at all, or is a
coincidence with something else happening deterministically a few minutes
into any race on this seed (an AI opponent naturally picking up and firing
one of these two weapons, for instance, since seven AI craft are also
calling the same dispatcher every tick with real state), is not
established.** The timing repeated closely across two independent boots,
which argues weakly for "something deterministic about this race," not
specifically for "my write caused it." Untangling that needs its own pass -
watching `Weapons_DispatchFire` itself (all craft, not just index 0) for
which craft and which bit is live in the tick immediately before the crash
would settle it directly. **This is where the coverage gap actually
stands**: not "these two weapons were skipped," but "these two weapons hit
a real, reproducible, undiagnosed emulator crash on both attempts," which is
a stronger reason to flag than a silent gap would be.

**This is still the strongest evidence tier this project's rubric has for a
negative claim, on everything it could test** - live, positive-control-verified,
across load and multiple untested-before gameplay conditions, and it agrees
with all eight static passes. Not "still hasn't been found": genuinely never
read, under every condition this pass could get the emulator to sustain.
The honest remaining uncertainty is now narrow and named: Missile, Energy
Drain, and whatever this pass's crash prevented from being reached, not the
open-ended "maybe some function somewhere" this thread carried into this
pass.

## Open

- **The global ambient RGB triple's writer is unfound, and the live capture
  above says there's nothing to find.** `get_xrefs_to` on both the EU
  (`_DAT_002b90c4`) and USA (`DAT_08abf494`) addresses returns only the one
  read each - no writer xref, on either binary. A live capture settled why:
  it's genuinely never written past its compiled default, on two tracks with
  sharply different authored `AmbientLight` colours, across a fresh boot and
  five in-race checkpoints. Not a static-analysis miss (the class of thing
  `Xml_ReadGlobalSettings`'s computed-offset table writes produce elsewhere
  in this project) - closed, by the strongest evidence tier this project's
  rubric has. `material+0x6c` (`Mesh_ApplyMaterialLighting`'s other, per-material
  input) was not separately live-tested and stays open on its own, since it
  gates *whether* the ambient branch runs at all, not what colour it uses -
  but the colour question, the one this pass opened to answer, is closed.
- **`DirectionalLight`'s collection is live-confirmed real; its final
  consumer is still unfound after six independent passes, but the search
  space is now a short, enumerated list rather than an unbounded trace.**
  `World_CollectMarkerLists` (confidence 85) builds a 4-entry
  `DirectionalLight` list at track load from genuine authored data -
  live-verified against `oag-view --nodes` ground truth on three independent
  class counts at once. This is a materially different status than
  `AmbientLight`/`PointLight`, both confirmed inert: `DirectionalLight`'s
  *data pipeline*, to the point of sitting in memory correctly populated, is
  real. What's still missing is anything reading `world+0x40`/`+0x7c`
  afterward. `World_LoadTrack_q` is ruled out at the disassembly level (pure
  epilogue right after the collection call), and `FUN_08886950` is ruled out
  structurally (`Race_CreateModeObject`, fifth pass, confirms its `param_1`
  is one hop short of the world object, and it never reads that hop). The
  class-based lookup path - every function that resolves the world object by
  its class stub `FUN_08a6b5bc` - is now **exhaustively closed** (sixth and
  seventh passes): all 12 call sites across all 10 unique functions read or
  decompiled, and **none reads `world+0x40` or `world+0x7c`**. Six are tag
  writes constructing a fresh world object or caching the resolved pointer
  for later, unconsumed use; two forward it elsewhere without reading it
  themselves; one queries a different helper with the class key as a
  parameter instead of walking the tree. `Mesh_ApplyMaterialLighting`'s own
  `material+0xc4` cache, a separate question this pass also closed, is
  write-only across all three of its callers (one more caller than this page
  previously named - see below). See "A sixth pass" above for the full table
  and reasoning per function. The per-frame draw dispatcher's one ambiguous
  touch already turned out to be an out-of-bounds artifact, and every
  sibling display-list function was already checked for `+0x40`/`+0x7c` too
  - every class-based path to the world object, not just the ones this page
  happened to read first, comes up empty. The only lead this couldn't cover
  by construction, `FUN_08930290`, is also now fully characterized and
  closed (eighth pass) - no consumer of `world+0x40`/`+0x7c` was found in
  any static mechanism this project could identify. **A ninth pass then
  settled the question live**: a read watchpoint on the actual runtime
  addresses, positive-control-verified, across ~2 minutes of driving plus
  the rocket/burst/backward weapons - zero hits on both fields throughout,
  the strongest evidence tier this project's rubric has, agreeing with every
  static pass. **Missile and Energy Drain remain untested** - both attempts
  to fire them hit a real, reproducible PPSSPP crash (`Bad memory access...
  Stopping emulation`, identical JIT block both times) whose cause (the
  test's own bit-write, or an unrelated deterministic race event) is not
  established; see "Missile and Energy Drain remain untested" below. Either
  the collection exists for a purpose this project hasn't identified, or it
  has none past track load - not a claim this page can rule between, but the
  search itself has converged as far as it currently can.
- **`Mesh_ApplyMaterialLighting` has three call sites, not the two this page
  previously named - and the third, `FUN_08930290`, turned out to be a
  display-list-baking function analogous to `Mesh_CompileDisplayLists`, not
  a second draw dispatcher (this page's own seventh-pass label for it was
  wrong, retracted in the eighth pass below).** Found while enumerating
  `Mesh_ApplyMaterialLighting`'s real callers via `search_instructions` on
  the un-rebased `jal` target - the same wart-driven undercount as
  everywhere else on this page. `FUN_08930290` bakes `Mesh_ApplyMaterialLighting`
  with a genuine runtime ambient value at model-construction or
  first-use time (not `Mesh_CompileDisplayLists`'s `-1` literal), which
  scopes this page's "lighting is never baked at compile time" claim to
  `Mesh_CompileDisplayLists` specifically - the M6 conclusion is unaffected,
  since `Mesh_ApplyMaterialLighting` disables all four hardware light slots
  regardless of caller. Its `+0xcc`/`+0xd0` read is resolved as a
  caller-supplied per-model tint pair, unrelated to lighting.
- **`world+0x44`/`world+0x48` read as `0` on the one track checked live**,
  but that doesn't settle what they're *for* - a zero on `16_Track` is
  consistent with either "these two classes are never authored on this
  track" or "nothing ever writes them regardless of track." `World_CollectMarkerLists`
  sums `+0x3c` (`AmbientLight`) + `+0x40` (`DirectionalLight`) + `+0x44` +
  `+0x48` + `+0x4c` (wopoint) + its own local into `+0x6cc`, but never writes
  `+0x44`/`+0x48` itself - two more per-class counts, populated (if at all)
  by some other collector. A quick follow (self-address-stub search at
  `0x08a6b380`, `0xc` bytes past the last used stub) led into
  `FUN_08886950`, a much larger scene-setup function unrelated to this list
  on first read; not chased further.
- **`material+0x6c`'s own writer is unfound**, same class of gap as the global
  triple above - `Mesh_ApplyMaterialLighting`'s real per-frame call site reads
  it, but nothing in this pass traced where it's set.
- **`PointLight`'s payload is inert on the code side**, confirmed rather than
  assumed - see the registration section above. Its trailer field (`{1,0,0,0}`
  on every sample, per `docs/formats/lighting.md`) has no consumer to explain
  it, because there is no consumer.
