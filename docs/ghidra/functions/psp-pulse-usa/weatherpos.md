# `weatherPos`'s registration is found, and its runtime constructor fires on a real race

> **2026-10-02:** what `weatherPos` is for is settled - the weather's anchor in a covered section. See [weather.md](weather.md).

Found while working the open question of unrecovered particle triggers:
the four environmental effects (`WO_RAIN`, `WO_SNOW`, `WO_MODESTO_STEAM_A`,
`WO_BLUE_WELDER`) need to know which track places them and where, and
`weatherPos` `0x3da` ([`vex.md`](../../../formats/vex.md#node-types)) is the
scene-node class shaped to carry that - `skycube.md` already flagged it as
"1 to 21 per track, zero-length payload, undecoded". This page is the
registration-site half of closing that, plus a class-identity mechanism, a
runtime constructor, and now **the constructor's own caller, confirmed live**
- the placement-and-selection half (which of four effects, and where exactly)
is still open below.

**Revision notes (same session, in order):**

1. An earlier version of this page called the value at the registered
   descriptor's `+4` "a trivial self-address-returning thunk" and treated it
   as a dead end, the same framing `pob.md` uses for `ParticleSystem`'s slot.
   That was wrong in kind, not just in detail - see
   ["The `+4` field is a class-identity tag, not a dead handler"](#the-4-field-is-a-class-identity-tag-not-a-dead-handler)
   below. `pob.md`'s equivalent claim for `ParticleSystem` was not
   re-verified this session and may carry the same error; noted in the
   parent handover thread rather than changed here.
2. A later version of this page then exhausted every *static* lead on the
   runtime constructor's caller and reported it as genuinely unreached in
   this binary. **That conclusion is superseded - a live PPSSPP capture found
   the caller in minutes.** The static search wasn't wrong about what it
   found (nothing, by every method tried); it was wrong to treat "no static
   caller" as evidence of "never called" strongly enough to stop there. See
   ["Confirmed live: the constructor is called, repeatedly, loading a real race"](#confirmed-live-the-constructor-is-called-repeatedly-loading-a-real-race)
   below.

## `WeatherPos_RegisterClass` (`0x0892c684`)

| | |
| --- | --- |
| **Address** | `0x0892c684` (USA) |
| **Confidence** | **90** |

Found the same way `lighting.md` and `fog.md` found their classes' sites: among
the 46 `Vex_RegisterClass` (`0x08908eb8`) call sites, by decompiling each until
its second argument (the class id) matched. Both binaries' imports carry the
"unrelocated address constant" wart documented in
[`positional-audio.md`](positional-audio.md#read-this-before-trusting-an-address-on-this-page)
in this exact region - `Vex_RegisterClass` itself decompiles as
`func_0x00104eb8`, not by name - so the call sites were found by
`search_instructions` for a `jal` to that truncated target rather than by
`get_xrefs_to`/`get_function_callers`, which both return nothing here. The
class id 0x3da (994) was then found among the delay-slot `li a1, ...`
immediates that follow each of the 50 matches; exactly one, at `0x0892c6a4`,
carries `0x3da`, one instruction after the `jal` at `0x0892c6a0` inside this
function.

```c
void WeatherPos_RegisterClass(void)
{
    Vex_RegisterClass(0x8c298, 0x3da);
    *(void**)(0x8c2d0) = &SOME_BLOCK_A;   // +0x38, not a string - see below
    *(void**)(0x8c2c0) = &STR_file;       // +0x28, the literal 4 bytes "file"
    *(void**)(0x8c29c) = WeatherposTag(); // +0x4,  0x08a6ba64
    *(void**)(0x8c2d0) = &SOME_BLOCK_B;   // +0x38, overwrites the first
    *(void**)(0x8c2c0) = &STR_file;       // +0x28, same value written again
    *(void**)(0x8c29c) = WeatherposTag(); // +0x4,  0x08a7255c, this one survives
    FinalizeClass(0x2bc120);
}
```

Two things worth recording exactly as read, not smoothed over:

- **The descriptor at `0x8c298` is built twice and only the second write
  survives.** The raw disassembly (not just the decompiler's rendering)
  confirms it: `sw a0, 0x38(s0)` / `sw s1, 0x28(s0)` / `sw v0, 0x4(s0)` each
  fire twice against the *same* `s0`, which is loaded once at function entry
  and never changed. `+0x28` gets the identical value both times - the address
  of the four bytes `"file\0"` (`0x08a88a38`), repeated several times in
  memory around there, which reads like a generic allocator category tag
  rather than anything `weatherPos`-specific. `+0x38` gets two *different*
  pointers, neither a string: each points at a small block that itself holds
  more truncated pointers (the same "unrelocated address constant" shape as
  everything else in this region), not decoded further. This does not match
  the `update`/`submit`/`draw`/`init` slot layout `vex.md` and `exhaust.md`
  document at `+0x24`/`+0x34`/`+0x44`/`+0x7c` - those are filled generically at
  instantiation time by the base-node constructor (`LodGroup`'s finding in
  `vex.md`), not by `Vex_RegisterClass`'s own initialiser, so there is no
  conflict, just a different, narrower field set being written here. Why the
  write happens twice rather than once is not explained by anything in this
  function; it costs nothing at runtime (dead store) so it reads like two
  near-identical source statements the compiler did not fold.

### The `+4` field is a class-identity tag, not a dead handler

**The value written to `+4` is not a "handler" at all - it is `weatherPos`'s
identity tag, and the mechanism that reads it is live.** `+0x08a7255c` -
`lui v0, 0x27; jr ra; addiu v0, v0, -0x1aa4` - returns exactly its own
address and touches nothing else, which is what the previous version of this
page called a dead-end thunk. Three independent call sites say otherwise:

- `Vex_CollectNodesByClass`, the generic tree-walker `vex.md` already documents gathering
  every `Mesh` for `Vex_LoadModel`, does not compare a class id at all - it
  compares `*(node + 4) == *(filter + 4)`. A node's own `+4` is stamped from
  its class's registered descriptor at construction time, so this is a
  pointer-identity check against exactly the value `Vex_RegisterClass`
  installs.
- `Vex_LoadModel` builds that `filter` locally, once per node type it wants,
  by *calling* the corresponding tag-getter and storing the result at the
  filter struct's `+4` - `local_28c = func_0x00267d48(); ...
  func_0x0026d364(param_1, local_33dc, 2000, &local_254, auStack_290);` is the
  `Mesh` gather (cap 2000, matching `vex.md`'s reading of `Vex_CollectNodesByClass`), and
  two further gathers in the same function use the same shape with different
  tag-getters and caps (1000, 0x40). `func_0x00267d48` is a thunk of the exact
  same self-address-returning form as `weatherPos`'s.
- `FUN_0892c404` (below), a runtime object constructor, stamps a freshly
  allocated instance's `+4` with the *same* thunk `weatherPos`'s registration
  ends on (`0x0026e55c`, i.e. `0x08a7255c`) - tagging the new object as a
  `weatherPos` instance for exactly the comparison above to recognise later.

So the six-thunk cluster at `0x08a7255c`..`0x08a725a0` (each 12 bytes -
`lui`/`jr`/`addiu`, one per class, not 8) is not "a small table of
interchangeable placeholder returns" - it is six classes' identity tags,
sitting where their translation units happened to link. `weatherPos`'s is the
one at `0x08a7255c`.

**The discarded first-write tag, `0x08a6ba64`, is not another class's tag at
all - it is a generic, widely shared one.** It sits in a second, larger
cluster of the same shape (at least ten consecutive 12-byte thunks read at
`0x08a6ba34`..`0x08a6baa4`, run not confirmed to end there, each still
returning exactly its own address), but unlike `weatherPos`'s real tag
(exactly 3 callers, all identified above), `search_instructions` finds
**25 separate functions** calling `0x08a6ba64` specifically, spanning
addresses from `0x0890xxxx` to `0x08a7xxxx` - clearly unrelated classes'
registration or construction code, not a coincidence at that count. That
confirms the hypothesis: `0x08a6ba64` is a shared "generic base node" tag
that many classes' registration/construction code stamps as a first pass,
`weatherPos`'s own registration among them, before some (not all -
`weatherPos` is one of the ones that do) overwrite it with a real per-class
tag. (One of the 25, `FUN_0890bd80`, was checked against `vex.md`'s cited
address for `LodGroup`'s own `init` slot and did not resolve to it cleanly
through this session's arithmetic - not asserted as a match, just noted as
tried and inconclusive.)

**Settled 2026-10-02 in [placed-particle-systems.md](placed-particle-systems.md):
`0x08a6bd18` is `ParticleSystem`'s live identity tag, and `ParticleSystem`
nodes - not `weatherPos` - are what place `WO_BLUE_WELDER` and
`WO_MODESTO_STEAM_A`, which leaves `WO_RAIN`/`WO_RAIN_LENS`/`WO_SNOW` as
`weatherPos`'s candidates.** The paragraph below is kept as written.
**This means `pob.md`'s identical framing for `ParticleSystem`'s slot
(`FUN_08a6bd18`, called there "a trivial self-address-returning thunk", "the
same dead end") is now suspect too** - it was not re-checked this session
against `Vex_CollectNodesByClass`'s callers, and the mechanism above says a thunk of this
exact shape is very unlikely to be inert. Flagged in the parent handover
thread; `pob.md` itself is left as it stood pending that check.

Class dispatch being by descriptor lookup rather than an immediate compare
([`vex.md`](../../../formats/vex.md#node-types)) still means there is no
`li 0x3da` to search for.

## The runtime constructor, statically

`FUN_0892c404(param_1, param_2, param_3)`, a few hundred bytes past
`WeatherPos_RegisterClass` in the same region, allocates a fresh `0xa0`-byte
object (`func_0x00142e40(0xa0, &STR_file, 0x21a)`, the same generic `"file"`
tag and a `0x21a` category constant), links it (`func_0x00140bd4`, the same
sibling-linkage primitive `vex.md` records for `LodGroup`'s constructor) when
`param_2` is non-null, then calls `FUN_0892c340(new_object)` - confirmed from
the raw disassembly, not just the decompiler's rendering, since `a0` is
`move`d from the new object's own register right before the `jal` - which in
turn stamps `+0x38`/`+0x28` generically and `+4` with `weatherPos`'s own tag
(`0x0026e55c`, `0x08a7255c`). `FUN_0892c404` then stamps `+4` with the same
tag again itself, redundant with what `FUN_0892c340` already wrote - the same
harmless double-write pattern as `WeatherPos_RegisterClass`'s own descriptor
above. `FUN_0892c340` has exactly one caller in this binary: `FUN_0892c404`.

**Every static search for `FUN_0892c404`'s own caller came back empty, by
four independent methods**: `get_xrefs_to` and `get_function_callers` (as
expected in this wart-affected region); `search_instructions` for its address
in *both* valid `lui`/`addiu` encodings; and `search_byte_patterns` for its
raw little-endian address anywhere in the program, code or data. That result
stood in an earlier version of this page as "genuinely unreached in this
binary" - **wrong**, corrected below. The static search wasn't lying; the
caller is reached through an indirect call this project's addressing
workaround for the unrelocated-address wart cannot make visible (below), so a
search for the callee's address as an immediate was always going to come back
empty regardless of whether it is called.

## Confirmed live: the constructor is called, repeatedly, loading a real race

**Confidence 95** for "`FUN_0892c404` executes during a real race's load,
called from `FUN_08908f98`" - a live breakpoint hit is the strongest evidence
class this project uses, on par with the shield and camera pages' own live
captures. Not full data-agreement territory (100) because only Talon's
Junction and only two hits were observed, per the caveats in Open below.

2026-08-25, PPSSPP v1.20.4 (SDL build, Xvfb, no window visible to anyone),
websocket debugger on port 47810, `pulse-psp-usa.chd`. Method, in full, per
[`ppsspp-debugger.md`](../../../reverse-engineering/ppsspp-debugger.md):
connected while the CPU was stepping, armed an execution breakpoint at
`0x0892c404` (`cpu.breakpoint.list` echoed it back disassembled as
`addiu sp,sp,-0x20` - `FUN_0892c404`'s own first instruction, confirming the
address before a single instruction had run), then drove `scripts/psp-drive.py
menu` into a Time Trial on Talon's Junction (`16_Track`, the same file the
"no in-file signal" section below reads statically).

**It hit.** The walk's own log stops at `"loading the race"` - the moment
`settle_into_race`'s next command (`input.buttons.press`) times out, because
the CPU is no longer running to answer it. A fresh connection confirms why:

```
cpu.status: stepping=True, pc=0x0892c404
```

`cpu.getAllRegs` at that exact stop:

| Register | Value | Role |
| --- | --- | --- |
| `a0` (`param_1`) | `0x08b65a30` | **`weatherPos`'s own live descriptor** - see below, not a pass-through |
| `a1` (`param_2`) | `0x08fddba0` | the node `func_0x00140bd4` links the new instance under |
| `a2` (`param_3`) | `0x45000000` → as `f32`: `2048.0` | stored into the new instance's `+0x4c` - likely a generic default, see below |
| `ra` | `0x0890901c` | **the call site** |

**`a0` is not a pass-through, and it is the decisive confirmation.** Reading
`0x08b65a30` directly finds `weatherPos`'s own descriptor fields, live and in
memory, at exactly the offsets `WeatherPos_RegisterClass` writes: `+0x44 =
0x000003da` (994, `weatherPos`'s own class id, confirming the static reading
of `vex.md`'s table), `+0x4 = 0x08a7255c` (the surviving identity tag from
above, exact match), `+0x38 = 0x08ad2474` (the surviving block pointer, exact
match), `+0x28 = 0x08a88a38` (the `"file"` tag, exact match). So this is not
merely "execution happens to be at `FUN_0892c404`'s address" - the live
memory it is running against *is* `weatherPos`'s own descriptor, corroborated
on four independent fields. (Mechanically: `a0` at the `+0x7c` call site is
`inst + offset`, the *result* of `FUN_08908f98`'s first indirect call
through `+0x74`, not its own `param_1` re-passed - a correction to how that
call reads, worth recording since it is easy to misattribute from the
decompiler's argument order alone.)

**`a1` is confirmed live to be a `Transform` node** (`+0x44 = 0x0000006e`,
`Transform`'s own class id in `vex.md`'s table - the same offset weatherPos's
own class id sits at, read directly, not inferred), tying this straight back
to the static reading below: Talon's Junction's `weatherPos` instances each
sit under an unnamed `Transform` in the `.vex` tree, and this is that
`Transform`, live. Its own class-identity tag at `+4` is the shared "generic
base" one (`0x08a6ba64`, the 25-caller tag from above) rather than a
`Transform`-specific tag - consistent with `Transform` being common enough
that nothing bothered giving it its own.

**A second live read reproduced the exact same call** (identical `a0`/`a1`/
`a2`/`ra` to the first session's capture, confirming this call site is
deterministic across runs) and dumped `a1`'s fuller structure. Two things in
it: `+0x4c = 0x45000000` (`2048.0`) **on the parent `Transform` too** - the
same value `a2`/`param_3` carries and the new `weatherPos` instance's own
`+0x4c` gets stamped with. A value shared between parent and child rather
than authored per-instance reads like a generic default (a range or LOD
cutoff inherited from the base-node constructor) more than a per-instance
weather selector - **softening the "strongest concrete lead" claim from the
first pass**, not confirming it. Bytes past `+0x60` look like unrelated
neighbouring heap data (`0xfeadfead`, a classic freed-memory poison pattern,
among them) rather than more of this struct - not read further.

**A second attempt at `FUN_08908f98`'s per-record data, this time breaking
at the resolve call itself (`0x08908ff4`) rather than deep inside
`FUN_0892c404`, found what the record actually is - a raw numeric class id**,
not a tag pointer: observed values `0x125` (`Mesh`), `0x3c1` (`Texture`),
`0x3c0` (`Anim Transform`) and `0x3c5` (`Airbrake`), all exact matches to
`vex.md`'s own class table, while driving through ship/track loading. That
settles what "the record" is - `func_0x00104b68(class_id)` genuinely resolves
a class by its numeric id, the reading the code always suggested and the
first pass's `param_2 + 4` read had obscured (that address holds `a1`'s own
*tag*, not an array cursor - a different field entirely, now that `a1` is
understood to be `param_2` itself, a real node, not an iteration-state
struct). **This resolve call fires constantly** - once per `Mesh`/`Texture`/
etc. spawned anywhere, including during ordinary front-end navigation - so
isolating specifically `weatherPos`'s own hit (`0x3da`) among the noise was
not achieved this session: interleaving discrete button presses with a
breakpoint that fires many times per screen transition proved too fragile to
land reliably in the time available. Removing this breakpoint and keeping
only `weatherPos`'s own (`0x0892c404`) is what let the walk complete.

`get_function_by_address(0x0890901c)` resolves to `FUN_08908f98`
(`08908f98`-`0890906b`), decompiled:

```c
void FUN_08908f98(int param_1, int *param_2)
{
    uint index = 0;
    if (*(short *)(*(int *)(param_1 + 0x48) + 0xc) != 0) {
        do {
            /* advance param_2's own array cursor to the next 16-byte-aligned record */
            ...
            int handle = func_0x00104b68(*record);            // resolve a class by id
            int inst = (**(code **)(*(int *)(handle + 0x38) + 0x74))
                           (..., param_1, *(undefined4 *)(param_1 + 0x4c));
            (**(code **)(*(int *)(inst + 0x38) + 0x7c))(..., param_2);  // the +0x7c init call
            index += 1;
        } while (index < *(ushort *)(*(int *)(param_1 + 0x48) + 0xc));
    }
}
```

This is a **generic per-class-group spawner**: read a count, and for that
many records, resolve a class descriptor and call its `+0x7c` slot - the
exact `init` layout `vex.md` and `exhaust.md` already document, and the same
slot [the earlier static section above](#the-4-field-is-a-class-identity-tag-not-a-dead-handler)
found reads zero in `weatherPos`'s *shipped, static* image. Both readings
were right: the slot is empty in `.data` and filled at runtime, exactly as
`vex.md`'s account of `LodGroup`'s own `init` predicted a static read could
not tell apart. **`FUN_0892c404` is that filled-in `+0x7c` method, and this
call resolved to `weatherPos`** - not inferred from the landing address
alone, but read directly out of `a0`'s own live descriptor content above.

**Resumed and hit again immediately**, same `a0`/`a1`/`a2`/`ra`, differing
only in caller-saved scratch registers (`v1`, `t0`, `t1`) and the emulated
tick count - consistent with `FUN_08908f98`'s loop calling through the same
site once per record in its group, `weatherPos`'s among them. Not counted
further this session (see Open).

**Why the static searches were always going to fail**: the call is
`(**(code **)(...))(...)`, an indirect call through a value loaded from
memory at runtime - there is no `jal 0x0892c404` anywhere to find, in either
`lui`/`addiu` encoding or as a raw address in `.data`, because the CPU never
loads that literal address as an immediate at all. It loads `weatherPos`'s
class-table pointer (itself reached only after `func_0x00104b68` resolves a
class id at runtime) and calls through `+0x7c` of *that*. Every search this
session ran was sound; none of them could see through a jump that only
exists as a computed value in a register. The lesson for future work in this
region: a class's real behaviour is only found by driving the class handler,
not by searching for its address.

## Placement: no in-file signal, confirmed on one track

`oag-view --nodes 'Data\Environments\16_Track\track.vex' --class 0x3da` (Talon's
Junction, `pulse-psp-usa.chd`) shows all 14 `weatherPos` instances with `0
byte(s)` payload and **no Maya name** - the `header_size >= 0x20` name field
`vex.md` documents is absent on every one. Walking each instance's parent chain
up to `World` finds only unnamed `Transform` nodes, so - unlike a `Texture`
node, which carries its own runtime path in the payload when nothing else names
it - a `weatherPos` instance has no string anywhere in the tree that could say
which of `WO_RAIN`/`WO_SNOW`/`WO_MODESTO_STEAM_A`/`WO_BLUE_WELDER` it is. The
node's own transform gives a position (the established "placement is the
node's own transform chain, not the payload" pattern
[`README.md`](../../../formats/README.md) already uses for `PointLight` and
the pads), but nothing in the `.vex` file says which effect plays there, or
whether all 14 on one track share one effect or several do.

`Data\Plugins\PI001\Definition.xml` (the front-end `PI_Track` table
[`track.md`](../../../formats/track.md#what-a-real-track-contains)
already reads) was checked too - its full attribute set (`type`,
`soundregister`, `location`, `availableInZone`, `collisionCageEnabled`,
`Reversed`) carries nothing weather-related on any of the 24 entries.

## Open

- **The trigger's *selection* is still not recovered - which of the four
  effects, and where exactly - even though the constructor's call chain, and
  that it really is `weatherPos`, are now both confirmed live.** `weatherPos`
  instances are real, constructed objects at race-load time, corroborated on
  four fields of their own live descriptor, not dead code, spliced under a
  live-confirmed `Transform` parent. Nothing here says which of `WO_RAIN`/
  `WO_SNOW`/`WO_MODESTO_STEAM_A`/`WO_BLUE_WELDER` a given one carries, or
  whether Talon's Junction's 14 all carry the same one. No authored data
  (`.vex` names, `Definition.xml`) has a signal. `param_3` (`2048.0`) is
  **downgraded** from "strongest concrete lead" - it turns out to be shared
  with the parent `Transform`'s own `+0x4c`, which reads like a generic
  default (a range or LOD cutoff) rather than a per-instance selector.
- **`FUN_08908f98`'s per-record data format is now known - a raw numeric
  class id, not a tag pointer** - confirmed by breaking at the resolve call
  itself (`0x08908ff4`) rather than reading `param_2 + 4` deep inside
  `FUN_0892c404` (which was reading `a1`'s own identity tag, a different
  field, now that `a1` is confirmed to be a real node rather than a bare
  iteration struct). What is *not* recovered: `weatherPos`'s own resolve
  call, `0x3da`, specifically - the breakpoint fires on every `Mesh`/
  `Texture`/`Anim Transform`/`Airbrake` spawn program-wide (all four observed
  this session, matching `vex.md`'s table), so it is too noisy to isolate by
  interleaving discrete input with it. The likely path in: filter on the
  value at the delay-slot's dereferenced address equalling `0x3da` specifically
  while free-running (no input needed once past the menu, since `weatherPos`
  loads automatically with the track) rather than trying to drive the front
  end with this breakpoint armed at all.
- **`FUN_08908f98` (`0x08908f98`-`0890906b`) is a generic per-class-group
  spawner, not `weatherPos`-specific**, so it is not renamed this session -
  its exact intended name (what `param_1`'s `+0x48` structure and `+0x4c`
  field are) is not pinned enough for `Subsystem_VerbNoun`. Worth a proper
  read and a name once someone is in this area for another class.
- **How many `weatherPos` instances actually construct, and whether every
  one reaches `FUN_0892c404` or only some, is not counted.** Two hits were
  observed with identical `a0`/`a1`/`a2` before this pass stopped
  (deliberately, to keep the session bounded) - consistent with, but short
  of confirming, all 14 of Talon's Junction's instances going through the
  same call site.
- **`WO_MODESTO_STEAM_A` may not even be a Pulse-authored trigger.** `modesto_heights`
  matches **Pure**'s `03_Modesto_Heights`
  ([`hd-status.md`](../../../formats/hd-status.md#the-circuits-are-the-psps)),
  not any Pulse circuit - so the effect asset shipping on the Pulse disc's
  `Data\Psys\` (per the parent thread's census) may be dead weight from a
  shared pipeline rather than something any Pulse track actually fires. Not
  checked against Pure's own disc.
