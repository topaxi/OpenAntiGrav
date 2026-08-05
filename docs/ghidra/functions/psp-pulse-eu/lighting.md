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
`DirectionalLight`'s fields; recorded here so a future pass doesn't
re-open them expecting a lighting connection.

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
  consumer is still unfound after three independent passes.** `World_CollectMarkerLists`
  (confidence 85) builds a 4-entry `DirectionalLight` list at track load from
  genuine authored data - live-verified against `oag-view --nodes` ground
  truth on three independent class counts at once. This is a materially
  different status than `AmbientLight`/`PointLight`, both confirmed inert:
  `DirectionalLight`'s *data pipeline*, to the point of sitting in memory
  correctly populated, is real. What's still missing is anything reading
  `world+0x40`/`+0x7c` afterward. One live capture and two Ghidra passes
  (see "The object itself" above) have now exhausted every lead that
  doesn't require tracing `World_LoadTrack`'s own 1,228 instructions
  directly - the class-based lookup path is exhaustively closed (exactly 5
  references to the class stub, all read), the per-frame draw dispatcher's
  one ambiguous touch turned out to be an out-of-bounds artifact, and every
  sibling display-list function was checked. **Converged, not abandoned**:
  either the reader is inside `World_LoadTrack` itself (the trace this
  thread has twice judged not worth the known tooling blind spots), or it
  doesn't render-consume the list at all and the collection exists for some
  other purpose this project hasn't identified.
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
