# Pulse's draw order is recovered, and it is a layer sort rather than a depth sort

2026-08-18. **The finding overturns the obvious fix.** This crate drew in file order and the plan was "add a back-to-front depth sort"; reading what the original does shows that would have been *less* faithful for track geometry, not more. Every drawable calls `Gfx_Enqueue` (`0x0891e35c`) with a 32-bit key into **one** queue, `Gfx_FlushRenderManager` (`0x0891e3c0`) `qsort`s it ascending and dispatches, and the comparator (`0x0891ddec`, confidence 95, four instructions) is `a->key - b->key` on the entry's `+0x04` - which is also what fixes the entry layout as `{ item, key }`. The key is **twelve bits of layer over twenty of back-to-front depth**, and the thing to carry: **a mesh batch set enqueues with its bare layer and no depth term at all** (`0x0892ece0` submits `*(set+0xbc)`; `Mesh_CompileBatchSet` stores its layer argument there at `0x0892f40c`). Only `ExhaustFlare_Submit` computes a depth, over 3,000 world units at `0.00286` a step. So between two meshes the layer is the whole order and within a layer it is submission order. `vex::mesh_layer` is `Mesh_InitFromPayload`'s derivation, `DrawCall::layer` carries it, `Model::sort_by_layer` applies it stably **once at build** (the key does not depend on the camera), and `pvs::DrawSections` stays index-parallel because it is built from the lists afterwards. **The census was the gate, not the write-up**: a derivation that put every mesh on one layer would reorder nothing, so `vex_layer_ground_truth.rs` measured it first - 21,055 mesh nodes over `Data.wad` split 66.4 %/33.6 % between `0x45` and `0x4a`, and **5,610 transparent batches split 63.7 %/36.3 %**, which is the figure that matters because only the transparent list is order-dependent. `01_Track` moves 1,841 pixels. **The `0x31` branch never fires on this disc** - implemented and unexercised. **Four divergences are named on [draw-order.md](../docs/rendering/draw-order.md) rather than hidden**: the original's `qsort` (`0x08972860`) is **unstable** and ours is stable, so equal keys are freedom we take deterministically; the original has one queue where we have three lists, so layer order is right *within* each; no depth term is implemented anywhere, including the flare's; and neither the `0x40000000` set (which `0x0892ece0` distance-gates, a cull inside the submit) nor the eight reflection layers are built. **A naming correction landed with it**: `Gfx_Enqueue` and `Gfx_ViewDepth` sat at 65 and 60, below the `_q` floor; reading their bodies raised them to 92 and 82, so the suffix is gone from both. **The address trap in this region, and it cost a wrong decompile**: the decompiler renders call targets unrelocated (`func_0x0011a35c`), the image base is `0x08804000`, and taking `0x119dec` for `0x08919dec` lands in the middle of a font routine. `0x0011a35c` is `0x0891e35c`. **Still open, and named**: what writes the depth override at `display+0x1180`; what the `0x40000000` set holds and what its distance test is; and whether HD orders the same way - `oag_render::mesh::rcs` leaves every PS3 chunk on `LAYER_DEFAULT` and the stable sort keeps chunk order. **That last one has a lead now**: the PS3 renderer sweep's `.cpp` attribution puts **`RenderManager.cpp` at `0x002d5710` and `SortRoot.cpp` at `0x002dbe08`** ("draw-order root"), which is the same two-part shape Pulse has - a render manager that flushes and a sort beneath it. It is a *name-level* parallel and nothing more: `SortRoot.cpp`'s four functions are all constructors with no callers, left unnamed below 50, so what it sorts and by what key is unread. See [renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md).

**2026-09-04: the `SortRoot.cpp` lead above is spent, not merely unconfirmed.**
`HANDOVER.md` records it was chased on 2026-08-25: `0x002dbe50` (one of the
four constructors) turned out to have six real callers, and following them
landed in `DetonatorBomb.cpp` - a bomb weapon's constructor, tied to
`RaceManager_GetInstance()`. `SortRoot`'s only located use is gameplay code
two hops from the render layer, not a draw-order root. Treat the name-level
`RenderManager.cpp`/`SortRoot.cpp` parallel as refuted. See
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md#what-was-deliberately-not-read),
which names the fresh lead this file's next step now points at instead: RSX
method constants inside the render layer's own address range
(`0x00279xxx`-`0x002ecxxx`), since PS3 draw-state calls are inlined
command-buffer writes and neither an import census nor a call graph finds
them.

**2026-09-04: whether HD orders draws the same way is answered - yes, same
mechanism as Pulse.** Chasing the fresh lead above (RSX method constants)
wasn't needed: rereading `RenderManager_FlushDrawQueue` (`0x002d6300`)
directly turned up a `qsort` call the earlier renderer.md pass had missed,
right before its dispatch loop, on the same `+0x630` array the loop walks.
The comparator - `RenderManager_CompareQueueKeys` (`0x002d4b58`, confidence
82, now named) - is `return *(int*)(a+4) - *(int*)(b+4);`, four instructions,
the identical shape and identical `{item, key}` 8-byte entry layout as
Pulse's `Gfx_CompareQueueKeys`. This retracts renderer.md's own prior
conclusion from its 2026-08-26 runtime trace ("the strongest evidence...
against a sort existing anywhere upstream of the dispatch loop") - that
trace never advanced past the function's own `qsort` call, so it inferred
"no sort" from a loop the sort had already run before reaching. (The
trace's own key values, reread, actually favour a completed sort: two
object families three orders of magnitude apart in key size sit in
ascending-family order across all sixteen entries - not proof by
themselves, since the decompile settles it regardless, but worth knowing
the data doesn't cut the other way either.) Full evidence and the
correction trail:
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md#the-per-eye-draw-dispatch-and-what-it-says-about-sort-order).

**2026-09-04: what the `+0x04` key encodes is answered too - it's Pulse's exact
scheme, twelve bits of layer over twenty of depth, not just a similar
shape.** Found by tracing `RenderManager_CreateInstance`'s own singleton slot
and, separately, searching the whole binary for the enqueue idiom's literal
immediates (`0x630`/`0x44b0` together) - the enqueue call isn't a single
shared function the way Pulse's `Gfx_Enqueue` is, it's the same few lines
inlined at every producer, which is exactly why no literal `0x630` store
ever turned up inside the render layer's own address range: none of the
five confirmed call sites (`0x00084e08`, `0x000a3c38`, `0x000ba268`,
`0x00109028`, `0x0012fba8`) live there. Four default to a bare 12-bit layer
constant (`0x300`, `0x570`, `0x5b0`) with zero depth, overridden when a
per-instance field (`instance+0x11c`, sentinel `0xffffffff` for "no
override") is set - masked to 20 bits, the same layer-over-depth split as
Pulse. The fifth (`0x0012fba8`) instead **computes its own depth term**
(a vector distance, bitwise-complemented so farther is a smaller key -
direct evidence for "back-to-front", not just inherited wording) and only
lets `instance+0x11c` override *that*. **One layer value overlaps Pulse's
own, read as a bucket match rather than an identical-effect claim**: `0x4d0`
shares its top byte with `ExhaustFlare_Submit`'s key, `0x4d000000`, and
(below) attributes to `MagstripWake.cpp` - a glowing wake trail sharing an
additive-glow layer bucket with an engine's exhaust flare is the expected
pairing Pulse's own census already implies (a layer is a coarse family
bucket - 21,055 meshes split across just two values - not a per-effect
identity). See
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md#the-enqueue-idiom-and-what-the-0x04-key-encodes)
for the full evidence and all five decompiles.

None of the five sites is named - what effect each one draws is still open,
below 50 confidence - and where `instance+0x11c` itself gets written is
still unread; nothing found so far writes it, only reads it. It plays the
same role in this key layout that `display+0x1180` (this page's own first
`## Open` bullet, above) does in Pulse's - one open "who writes the depth
override" question in two titles, not two unrelated ones. One more lead surfaced along
the way, below 50 and not acted on: one of the two live-observed queued
vtables (`0x00864cb8`) is related, through at least one indirection, to
`FrontendRoot_Construct`'s own vtable slot - not established as the same
vtable - which would suggest HUD/front-end elements are drawn through this
same queue mid-race if it holds up. See renderer.md's "What was
deliberately not read" for the field-agreement evidence and its limits.

**2026-09-04: three of the five enqueue call sites now attribute to a file,
via shared TOC-slot ranges rather than code-address proximity** (code
address and TOC-slot address don't have to agree, and one of the three
corrects a weaker first guess that used address gaps alone - see
renderer.md). `0x00109028` (the `0x4d0`-layer site) is the strongest: it
sits *inside* `MagstripWake.cpp`'s own slot range, not the more obvious
`DebrisManager.cpp` its code address sits closer to. `0x0012fba8` (the site
that computes its own depth) sits in the slot range immediately following
`BombManager.cpp`'s - `BombManager_Construct` itself calls
`DetonatorBomb_Construct`, tying back to the `DetonatorBomb.cpp` the
`SortRoot` investigation already named, but `DetonatorBomb.cpp`'s own
attributed range sits well past this site, so a bomb's blast effect fits
`BombManager.cpp` owning it, not necessarily `DetonatorBomb` itself.
`0x000a3c38` sits in the slot range immediately following `Camera.cpp`'s -
the weakest of the three, since one corroborating slot it shares with
`Camera.cpp`'s constructor is a game-wide global reached through a
per-file slot, not exclusive to this file. The other two (`0x00084e08`,
`0x000ba268`) don't resolve cleanly by either method; `0x000ba268` in
particular falls in a gap between two confirmed files that neither covers -
a third, silent translation unit `map`'s constructor-only attribution
misses, not just an attribution failure. File attribution is not class
attribution - `map` only sees constructors and allocators, so this narrows
where to look next, it doesn't name what draws. See
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md#the-enqueue-idiom-and-what-the-0x04-key-encodes)
for the slot evidence on all five.

## Open

- Who writes the per-frame depth override is unread in **both** titles, and it's one question now, not two: Pulse's `display+0x1180` and HD's `instance+0x11c` play the identical role (the sentinel-checked override the enqueue idiom reads before falling back to a computed or zero depth) in the identical key layout
- What the `0x40000000` set holds and what its distance test does is unread
- What class each of HD's five confirmed enqueue call sites belongs to - what effect it draws - is unread (below 50 confidence); three now have a plausible owning *file* (`0x000a3c38` in `Camera.cpp`, `0x0012fba8` in `BombManager.cpp`, `0x00109028` in `MagstripWake.cpp`), not yet a class or a confirmed purpose
- Whether the `0x00864cb8` queued vtable relates to `FrontendRoot`'s own vtable by identity or only by indirection (a base subobject or secondary vtable) is unread past one xref-agreement check (below 50 confidence)

## Next Steps

- Find where `instance+0x11c` gets written (bracketing around one of the five known enqueue sites, or a write-watchpoint attempt despite the earlier `+0x630` one coming back unsupported) to close out the key format's last unknown
- Confirm `0x00109028`'s class within `MagstripWake.cpp` and read enough of it to say what it actually draws - a magstrip's own wake trail is the most concrete guess so far, given the `0x4d0` glow-bucket layer and the direct slot match
- If `0x00864cb8` turns out to be `FrontendRoot`'s own vtable rather than merely related to it, that would mean HD draws its HUD/front-end through the same sorted queue as world geometry - worth confirming before assuming `oag_render::mesh::rcs`'s `LAYER_DEFAULT`-for-everything is the only gap versus Pulse's per-mesh layer
