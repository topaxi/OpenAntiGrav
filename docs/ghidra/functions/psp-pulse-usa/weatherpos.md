# `weatherPos`'s registration is found, and so is a runtime constructor nothing statically calls

Found while working [the open thread on unrecovered particle triggers](../../../../handover/the-particle-effects-are-played-from-the-disc.md):
the four environmental effects (`WO_RAIN`, `WO_SNOW`, `WO_MODESTO_STEAM_A`,
`WO_BLUE_WELDER`) need to know which track places them and where, and
`weatherPos` `0x3da` ([`vex.md`](../../../formats/vex.md#node-types)) is the
scene-node class shaped to carry that - `skycube.md` already flagged it as
"1 to 21 per track, zero-length payload, undecoded". This page is the
registration-site half of closing that, plus a class-identity mechanism and a
runtime constructor found while reading it; the placement-and-selection half
is still open below.

**Revision note (same session):** an earlier version of this page called the
value at the registered descriptor's `+4` "a trivial self-address-returning
thunk" and treated it as a dead end, the same framing `pob.md` uses for
`ParticleSystem`'s slot. That was wrong in kind, not just in detail - see
["The `+4` field is a class-identity tag, not a dead handler"](#the-4-field-is-a-class-identity-tag-not-a-dead-handler)
below. `pob.md`'s equivalent claim for `ParticleSystem` was not re-verified
this session and may carry the same error; noted in the parent handover
thread rather than changed here.

## `WeatherPos_RegisterClass` (`0x0892c684`)

| | |
| --- | --- |
| **Address** | `0x0892c684` (USA) |
| **Confidence** | **90** |

Found the same way `lighting.md` and `fog.md` found their classes' sites: among
the 46 `Vex_RegisterClass` (`0x08908eb8`) call sites, by decompiling each until
its second argument (the class id) matched. Both binaries' imports carry the
"unrelocated address constant" wart documented in
[`psp-pulse-usas-import-carries-unrelocated-address-constants.md`](../../../../handover/psp-pulse-usas-import-carries-unrelocated-address-constants.md)
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

- `FUN_08a71364`, the generic tree-walker `vex.md` already documents gathering
  every `Mesh` for `Vex_LoadModel`, does not compare a class id at all - it
  compares `*(node + 4) == *(filter + 4)`. A node's own `+4` is stamped from
  its class's registered descriptor at construction time, so this is a
  pointer-identity check against exactly the value `Vex_RegisterClass`
  installs.
- `Vex_LoadModel` builds that `filter` locally, once per node type it wants,
  by *calling* the corresponding tag-getter and storing the result at the
  filter struct's `+4` - `local_28c = func_0x00267d48(); ...
  func_0x0026d364(param_1, local_33dc, 2000, &local_254, auStack_290);` is the
  `Mesh` gather (cap 2000, matching `vex.md`'s reading of `FUN_08a71364`), and
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

**This means `pob.md`'s identical framing for `ParticleSystem`'s slot
(`FUN_08a6bd18`, called there "a trivial self-address-returning thunk", "the
same dead end") is now suspect too** - it was not re-checked this session
against `FUN_08a71364`'s callers, and the mechanism above says a thunk of this
exact shape is very unlikely to be inert. Flagged in the parent handover
thread; `pob.md` itself is left as it stood pending that check.

Class dispatch being by descriptor lookup rather than an immediate compare
([`vex.md`](../../../formats/vex.md#node-types)) still means there is no
`li 0x3da` to search for.

## A runtime constructor exists, and nothing in this binary statically reaches it

`FUN_0892c404(param_1, param_2, param_3)`, a few hundred bytes past
`WeatherPos_RegisterClass` in the same region, allocates a fresh `0xa0`-byte
object (`func_0x00142e40(0xa0, &STR_file, 0x21a)`, the same generic `"file"`
tag and a `0x21a` category constant), splices it into `param_2`'s child list
when `param_2` is non-null (`func_0x00140bd4`, the same sibling-linkage
primitive `vex.md` records for `LodGroup`'s constructor), then calls
`FUN_0892c340(new_object)` - confirmed from the raw disassembly, not just the
decompiler's rendering, since `a0` is `move`d from the new object's own
register right before the `jal` - which in turn stamps `+0x38`/`+0x28`
generically and `+4` with `weatherPos`'s own tag (`0x0026e55c`, `0x08a7255c`).
`FUN_0892c404` then stamps `+4` with the same tag again itself, redundant with
what `FUN_0892c340` already wrote - the same harmless double-write pattern as
`WeatherPos_RegisterClass`'s own descriptor above.

**`FUN_0892c340` has exactly one caller in this binary: `FUN_0892c404`.** Not
proven shared boilerplate, despite writing a generic-looking `"file"` tag -
just not ruled out either.

**`FUN_0892c404` itself has no caller found by any of four independent
searches**: `get_xrefs_to` and `get_function_callers` return nothing (as
expected in this wart-affected region); `search_instructions` for its address
in *both* valid `lui`/`addiu` encodings (`0x00128404` as `lui 0x13` +
`addiu -0x7bfc`, and as `lui 0x12` + `addiu 0x8404`) also returns nothing -
unlike `Vex_RegisterClass` and `FUN_08a71364`, which this same technique found
dozens of call sites for; and `search_byte_patterns` for its raw little-endian
address (`04 C4 92 08`) anywhere in the program - code or data - also finds
nothing, which rules out it being stashed in a function-pointer *table*
(a per-class jump table, say) rather than reached by a fixed instruction. So
this is not the earlier session's search-encoding mistake repeating; it is a
consistent negative result across every static search this session tried.

`weatherPos`'s own class table's `+0x7c` (`0x08890314`, from `WeatherPos_RegisterClass`'s
own `0x8c298` argument + image base) reads as four zero bytes in the shipped
image. `vex.md` reports `LodGroup`'s equivalent slot as a real,
`LodGroup`-specific pointer (confidence 88) - this session tried to
re-confirm that by the same method and could not get a coherent result from
`LodGroup`'s own registration site, which resolves through the same
unrelocated-address wart and did not reproduce `vex.md`'s cited table address;
left as `vex.md`'s existing finding rather than re-derived here. Taken at
face value it says `weatherPos`'s `+0x7c` being zero is meaningful, not
generic filler - but it is not proof either way: `vex.md`'s own reading of
`LodGroup` shows this slot can also be filled generically at *instantiation*
time rather than at registration time, which a static read of an
unconstructed class's table cannot distinguish from "never gets one". Either
something calls
`FUN_0892c404` through a function-pointer slot this session did not locate,
or - matching the pattern `vex.md` already documents for `LodGroup`'s tier
switch and for `engine_fire`/`exitglow`/`gate`'s missing registrations - this
retail binary authors a `weatherPos` constructor it never statically
exercises.

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

- **The trigger is still not recovered**, and no authored data (`.vex` names,
  `Definition.xml`) says which effect a `weatherPos` instance plays or picks
  between the four candidates. What is now ruled out: a data-authored name or
  flag anywhere this project reads. What is **not** ruled out any more: a
  per-class runtime path exists (`FUN_0892c404`) - it just is not statically
  reached, so either its caller has not been found yet or the retail binary
  authors it without exercising it.
- **`FUN_0892c404`'s caller is the concrete next probe.** Ruled out this
  session: `Vex_LoadModel`'s other two per-node-type gathers. The cap-1000
  gather's tag-getter (`0x08a6b9bc`) has 35 callers including
  `Texture_LoadEngineFlare`, `Texture_LoadEffectSurfaces` and
  `Texture_LoadEngineNoise` by name - it gathers `Texture` nodes, not
  `weatherPos`. The cap-`0x40` gather's tag-getter (`0x08a6ba4c`) has 7
  callers, none `weatherPos`-related. So neither gather is the path in.
  `weatherPos`'s own class table's `+0x7c` reads zero statically (above),
  consistent with no wiring but not proof of it. Untried: a live emulator with
  a breakpoint on `0x0892c404` during a race on a track with `weatherPos`
  instances (Talon's Junction has 14) - the only way left to tell "never
  wired" from "wired at instantiation time, which a static read of an
  unconstructed table can't see" apart.
- **`WO_MODESTO_STEAM_A` may not even be a Pulse-authored trigger.** `modesto_heights`
  matches **Pure**'s `03_Modesto_Heights`
  ([`hd-status.md`](../../../formats/hd-status.md#the-circuits-are-the-psps)),
  not any Pulse circuit - so the effect asset shipping on the Pulse disc's
  `Data\Psys\` (per the parent thread's census) may be dead weight from a
  shared pipeline rather than something any Pulse track actually fires. Not
  checked against Pure's own disc.
