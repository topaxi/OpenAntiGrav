# `weatherPos`'s registration is found, and its surviving handler is a dead end

Found while working [the open thread on unrecovered particle triggers](../../../../handover/the-particle-effects-are-played-from-the-disc.md):
the four environmental effects (`WO_RAIN`, `WO_SNOW`, `WO_MODESTO_STEAM_A`,
`WO_BLUE_WELDER`) need to know which track places them and where, and
`weatherPos` `0x3da` ([`vex.md`](../../../formats/vex.md#node-types)) is the
scene-node class shaped to carry that - `skycube.md` already flagged it as
"1 to 21 per track, zero-length payload, undecoded". This page is the
registration-site half of closing that; the placement-and-selection half is
still open below.

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
    *(void**)(0x8c2d0) = "..."; // string A
    *(void**)(0x8c2c0) = "..."; // string B, shared
    *(void**)(0x8c29c) = SomeThunk_A();   // 0x08a6ba64
    *(void**)(0x8c2d0) = "..."; // string A', overwrites the first
    *(void**)(0x8c2c0) = "..."; // string B, same value written again
    *(void**)(0x8c29c) = SomeThunk_B();   // 0x08a7255c
    FinalizeClass(0x2bc120);
}
```

Two things worth recording exactly as read, not smoothed over:

- **The descriptor at `0x8c298` is built twice and only the second write
  survives.** The raw disassembly (not just the decompiler's rendering)
  confirms it: `sw a0, 0x38(s0)` / `sw s1, 0x28(s0)` / `sw v0, 0x4(s0)` each
  fire twice against the *same* `s0`, which is loaded once at function entry
  and never changed. The field at `+0x28` gets the identical value both times;
  `+0x38` and `+0x4` each get overwritten with a second, different value. This
  does not match the `update`/`submit`/`draw`/`init` slot layout `vex.md` and
  `exhaust.md` document at `+0x24`/`+0x34`/`+0x44`/`+0x7c` - those are filled
  generically at instantiation time by the base-node constructor
  (`LodGroup`'s finding in `vex.md`), not by `Vex_RegisterClass`'s own
  initialiser, so there is no conflict, just a different, narrower field set
  being written here. Why the write happens twice rather than once is not
  explained by anything in this function; it costs nothing at runtime (dead
  store) so it reads like two near-identical source statements the compiler
  did not fold, not a bug worth chasing further.
- **The surviving handler, at `0x08a7255c`, is a trivial self-address-returning
  thunk** - `lui v0, 0x27; jr ra; addiu v0, v0, -0x1aa4` returns exactly its
  own address and touches nothing else. This is the same dead end
  [`pob.md`](../../../formats/pob.md) already found for `ParticleSystem`'s
  registered handler (`0x08a6bd18`, "a trivial self-address-returning
  thunk"). The discarded first-write handler at `0x08a6ba64` is one of *six*
  consecutive, identical eight-byte thunks starting there
  (`0x08a7255c`..`0x08a725a0`), each just returning a different constant a few
  bytes apart - a small table of interchangeable placeholder returns, not
  per-class logic.

**So the registration call site answers "is `weatherPos` handled at all" (yes,
alphabetically among the 46, same as every other environment class) but not
"how" - the field that would carry real behaviour holds a thunk, exactly the
pattern that already closed off `ParticleSystem` as a lead.** Class dispatch
being by descriptor lookup rather than an immediate compare
([`vex.md`](../../../formats/vex.md#node-types)) means there is no `li 0x3da`
to search for either.

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

- **The trigger is still not recovered.** The registered handler is a dead end
  and no authored data (`.vex` names, `Definition.xml`) says which effect a
  `weatherPos` instance plays or picks between the four candidates. What is
  now ruled out: a per-class handler function, and a data-authored name or
  flag anywhere this project reads.
- **What has not been tried**: the generic tree-walker that collects nodes by
  class for runtime use (the same shape as `FUN_08a71364`, which
  `vex.md`/`Vex_LoadModel` uses to gather every `Mesh`) has not been located
  for class `0x3da`. If one exists, its caller is the next lead - it is the
  only place left that could hold a per-track or per-name dispatch (a string
  compare against the track's own name, for instance) deciding which `.pob`
  plays. Searching from `Vex_LoadModel`'s own callers outward, rather than
  from the class table inward, is untried.
- **`WO_MODESTO_STEAM_A` may not even be a Pulse-authored trigger.** `modesto_heights`
  matches **Pure**'s `03_Modesto_Heights`
  ([`hd-status.md`](../../../formats/hd-status.md#the-circuits-are-the-psps)),
  not any Pulse circuit - so the effect asset shipping on the Pulse disc's
  `Data\Psys\` (per the parent thread's census) may be dead weight from a
  shared pipeline rather than something any Pulse track actually fires. Not
  checked against Pure's own disc.
