# Draw order: the original has one queue, one key, and one sort

**Status: recovered, confidence 85.** Wipeout Pulse does not draw in file order,
and - for scene geometry - it does not depth-sort either. Every drawable submits
itself to a single per-frame queue with a 32-bit key; the queue is sorted
ascending once and then dispatched. This page is what that key is, where each
part of it comes from, and how much of it `oag-render` implements.

Recovered 2026-08-18 by static decompilation of `psp-pulse-usa/BOOT.BIN`.
Nothing here has been verified against a running original, which
[caps it at 94](../reverse-engineering/confidence-rubric.md); the pieces below
carry their own scores. The function pages are
[mesh-draw.md](../ghidra/functions/psp-pulse-usa/mesh-draw.md) and
[exhaust.md](../ghidra/functions/psp-pulse-usa/exhaust.md).

## The queue

`Gfx_Enqueue` (`0x0891e35c`, confidence 92) appends one 8-byte entry to an
array in the display object - the item pointer at `+0x00`, the key at `+0x04` -
and bumps a count. The array is at `display+0x16a0` and the count at
`display+0x5520`, which bounds it at exactly **2,000 entries**.

```c
void Gfx_Enqueue(Display *d, void *item, u32 key) {
    if (d->depth_override != 0xffffffff)                    // d+0x1180
        key = (d->depth_override & 0xfffff) | (key & 0xfff00000);
    d->queue[d->count].item = item;
    d->queue[d->count].key  = key;
    d->count++;
}
```

The override at `+0x1180` replaces the **depth** half of every key while leaving
the layer alone. Nothing here reads what sets it.

`Gfx_FlushRenderManager` (`0x0891e3c0`, confidence 85) sorts and dispatches:

```c
qsort(d->queue, d->count, 8, Gfx_CompareQueueKeys);
for (i = 0; i < d->count; i++)
    d->queue[i].item->vtable->draw(..., d, d->queue[i].key);
d->count = 0;
```

The key is handed back to each draw callback as its third argument.

## The comparator, which is four instructions

`Gfx_CompareQueueKeys` (`0x0891ddec`, confidence 95 - the highest score
anything on this page carries, because there is nothing in it to interpret):

```asm
lw   v0, 0x4(a0)      ; a->key
lw   a0, 0x4(a1)      ; b->key
jr   ra
subu v0, v0, a0       ; return a->key - b->key
```

**Ascending, on the whole 32-bit word, signed.** So a lower key draws first.

The sort itself (`0x08972860`) is a textbook `qsort` - Bentley-McIlroy three-way
partition, median-of-three below 0x28 elements and a ninther above it, insertion
sort under 7. **It is not stable**, which matters below.

## The key: twelve bits of layer, twenty of depth

```text
bits 31..20   layer
bits 19..0    back-to-front depth, where the submitter computes one
```

Ascending order, so **lower layer draws first**:

| Key | What | Where it comes from |
| --- | --- | --- |
| `0x01500000` + n*`0x1000000`, n=0..7 | planar reflection targets | a material's `(+0x03) & 0xfc`, `>> 2` |
| `0x30000000` | the camera | `Camera_SubmitScene` |
| `0x31000000` | mesh payload `+0x0c & 0x8` | [`vex::mesh_layer`] |
| `0x40000000` | an extra batch set, **distance-gated** | `Mesh_BuildLayerBatchSets` |
| `0x45000000` | batch `pass_mask & 0x1000`, narrowly | [`vex::mesh_layer`] |
| `0x4a000000` | every other mesh - the default | [`vex::mesh_layer`] |
| `0x4d000000` \| depth | the exhaust flare, **and every list-B mesh** (2026-10-19) | `ExhaustFlare_Submit`, `Mesh_SubmitNode` |
| `0x4d000000` | **every `Trail` (the engine ribbon)**, depth zero (2026-10-21) | `Trail_Submit` (`0x0892a408`) |

### List-A mesh geometry carries no depth at all, list B does (corrected 2026-10-19)

**List B is the exception, found 2026-10-19 (`moa-strip-band`).** The
paragraph below is true of the batch-set path, which is list A only. A mesh's
**list-B** batches - every blended batch of a PSP circuit, trees and arrow strips
among them - are queued by `Mesh_SubmitNode` (`0x0890cc80`) at `mesh+0xc8` OR-ed
with `0xfffff - trunc(-z * 349.525)`, `z` being the view depth of the centre of the
mesh's own box (`ViewCull_ViewDepth`, `0x08902b24`; the `0x4d` flare formula
below, taken per mesh). `mesh+0xc8` is derived at the end of `Mesh_InitFromPayload`
from the model's key *before* the per-mesh re-derivation: `0x4d000000` for the
ordinary scene key `0x45000000` (a branch-likely delay slot at `0x0890ed28`; the
page read the last two rows of the mapping swapped until now), so **list B is one
layer, far to near**, and a `0x45` tree nearer than a `0x4a` strip draws after it.
Measured: the order this key predicts for nodes 261, 267, 878, 258, 875, 293 and
294 of `03_Track` is the order of GE prims 413-425 of a recorded Moa frame, and
that frame is not in layer order (`0x45` and `0x4a` meshes alternate).
Implemented as `Model::blended_order`, per view, from the unjittered
view-projection's `w`; the units are the model's own (**chosen, not measured**:
the original multiplies `z` by `display+0x4310 / 65`, unread, which moves only the
3,000-unit clamp). Of `03_Track`'s 131 transparent draws all 131 carry the key and
5 sit under an `Anim Transform`: those are keyed from the **time-zero** pose of their
anchor (**chosen, not measured**: whether the original refreshes `mesh+0xb0` as the
node moves turns on the `0x40000` flag, whose setter was not read). The order is held
by `glow_mask_ground_truth.rs`. Confidence 85.

This is the part that overturns the obvious guess for list A. A mesh does not enqueue
itself; `Mesh_BuildLayerBatchSets` (`0x0892eda4`) groups a model's meshes into
**batch sets**, one per layer, each built by `Mesh_CompileBatchSet`
(`0x0892f35c`) with its layer key stored at `set+0xbc`. The set's own submit
method (`0x0892ece0`) then does:

```c
if (set->layer != 0x40000000 || <distance test>)
    Gfx_Enqueue(g_display, set, set->layer);
```

**The bare layer, with the low twenty bits zero.** So between two meshes the
layer is the entire order, and within a layer it is submission order - modulo
the `qsort` being unstable. A renderer that added a back-to-front depth sort
over a circuit's batches would be *less* faithful than one that did not.

The `0x40000000` set is skipped entirely unless a distance test passes, which is
a cull rather than an ordering rule and is recorded here so nobody inherits it by
accident.

### The ribbon is keyed at depth zero, so every list-B mesh draws after it (2026-10-21)

`Trail_Submit` (`0x0892a408`) enqueues the engine ribbon at the constant `0x4d000000`. Read off the live queue on the
Talon's race pair (166 entries, `scripts/psp-queue-read.py`): all ten `Trail` items carry exactly that key, ahead of the 40-odd list-B
meshes (`0x4d0d00d5` and up) and of the player's flare (`0x4d0ff75e`). The ribbon writes no depth, so a translucent
circuit mesh near the craft draws over it: the floor patch under the player removes about six sevenths of the ribbon's
colour at the tail box (replay bisection, [exhaust.md](../ghidra/functions/psp-pulse-usa/exhaust.md), "The ribbon is
queued first"). Implemented as `Recorder::draw_ribbons` and `Draw::TrackBlended`: the hulls, then the ribbons, then the
circuit's blended list, then the flares. HD's tube keeps the old order, its key unread. The flare, which carries its own
depth, is still drawn after the whole blended list; interleaving it is open.

Re-verified independently on 2026-10-21 (`trail-order-option`) on three states, two circuits and one fresh GE-replay
removal; the key's 20-bit depth can be overridden by `display+0x1180` (inactive in every read), and six non-`Trail`
items share the depth-zero key. The order is now the TRAIL ORDER setting on the PSP ribbon (`trail_order`, `original`
by default, `over-floors` chosen, not measured). Details in [exhaust.md](../ghidra/functions/psp-pulse-usa/exhaust.md),
"Re-verified independently, and the order made a setting".

### Hull, pads and trees on Platinum Rush White: hull 0x45, pads 0x4a, trees 0x4d plus depth (2026-11-07)

Measured on PPSSPP 1.20.4, Pulse USA, a Single Race on `13_Track` (Platinum Rush White, dev-unlock byte, VENOM,
craft at `(-542.8, 2.9, -292.9)`, race clock 4.02), two separate boots of the same state. Two instruments:
(1) `Gfx_Enqueue` (`0x0891e35c`) stopped on every hit over one frame, logging the item (`a1`), the key (`a2`) and the
caller (`ra`) - 147 entries per frame, the same keys on both boots; (2) the GE list of the frame
(`scripts/psp-ge-dump.py dump` and `census`, 638 prims) with a **key bump**: the same state loaded with
a breakpoint on `Gfx_FlushRenderManager` (`0x0891e3c0`) that adds `0x30000000` to the key of a chosen set of queue
entries before the sort, a second connection recording the next frame, and the prims' new positions read off the
census. A prim is named by its texture address and world box, not its index. The class of an entry is the
`ra` of its `Gfx_Enqueue` call plus its layer.

| Class | Key | Caller of `Gfx_Enqueue` | Where its prims drew (638-prim frame) | State (`ztest`, `zmask`, blend, alpha test) |
| --- | --- | --- | --- | --- |
| The four craft's hull sets | `0x45000000` | `0x0892ed4c` (the batch-set submit) | 299-316, 369-371 | opaque: 6, writes, no blend, alpha test on; the shield and glass passes: 2 (EQUAL), no write, blend |
| Speed pads (all 11 of the circuit, drawn whether or not in view) | `0x4a000000` | `0x0892ed4c` | 312-350, three passes of 11 | pass 1: **7, writes**; pass 2: 6, writes; pass 3 (a 10-vertex cutout): 6, writes; none blended, alpha test on |
| Weapon pads | `0x4a000000` | `0x0890cdc8` (not the batch-set path) | 535-543, pairs | 7 then 6, writes, no blend, alpha test on |
| Trees and every other list-B mesh | `0x4d000000` + depth | `0x0890cee0` (`Mesh_SubmitNode`) | 578-599, the blended block at the end | **blend on, alpha test on (func 6, reference 0), `ztest` 6, `zmask` 1: no depth write** |
| `Trail` ribbons, flares | `0x4d000000`, `0x4d0xxxxx` | `0x0892a440`, `0x08904a14` | as above | no depth write |
| The ship object (4 in view) | `0x46000000` | `0x08902f18` | not read | not read |

**The bump moved what its key says.** Bumping the `0x4a` batch sets sent the 22 speed-pad prims from 312-333 to 675-696 of
730 and left the hull at 301; bumping the `0x45` sets sent the hull from 299 to 496 and left the pads ahead of it;
bumping the three `0x4a` entries called from `0x0890cdc8` sent the weapon pads from 535 to 721 and left the speed
pads. Repeated on a second boot with identical positions. So **hull `0x45000000` < speed and weapon pad `0x4a000000`
< list B `0x4d000000` + depth**, and in the original frame a tree draws after the craft and after every pad: 581
against 299, 312 and 535.

The depth state is what makes that the right picture. A list-B tree writes **no depth** (`zmask` 1) and tests
"nearer wins" (`ztest` 6): it is painted over what is already in the buffer where it is nearer, and a craft or pad
drawn after it would overwrite it whatever the distance. The original's hull and pads come first, so a nearer tree
lies over both. The tree prims of the same frame that are opaque (`blend` 0, alpha test on, `zmask` 0, list A) write
depth.

**Ours draws the PSP in the same order and holds the same depth state by default.** `Scene::prepare_view`
(`crates/raceplay/src/scene/frame.rs`) pushes the circuit (solid, sky, shine), the pads, the gantry, the shadows, the
hulls, `Draw::Ribbon`, `Draw::TrackBlended`, then wrecks, weapons, flares, plumes and `Draw::Effects`, whenever
`Race::ribbons_first()` holds: a PSP-preset ribbon (not HD's tube) and `trail_order = original`, the default and the
maintainer's own setting for every title. The blended pipeline tests `Less` and writes no depth
(`crates/mesh/src/mesh_render.rs`), the original's nearer-wins and `zmask` 1. Where a tree overlaps a hull in a frame of
ours the tree draws over it: a synthetic pose on this circuit with the camera beside a tree (the craft 11.6 units ahead
of the eye, the camera put by hand) leaves 98 of 130,560 pixels, all on the craft, that differ between
`--trail-order original` and `over-floors`, and with `original` the foliage lies over the craft's wing.

**The symptom is the other order.** With `ribbons_first()` false the circuit draws whole inside `Draw::Track`, so the
blended list (no depth write) goes down before the pads and the hulls, and a hull or pad drawn later overwrites a nearer
tree: the tree "draws behind" it. That is exactly what `trail_order = over-floors` does, and what the HD tube
(`hd_trail_active`) does on HD, Omega and 2048. On Pulse and Pure by default it does not happen, and **the
maintainer's report was not reproduced on White**: three matched poses of ours against the original (the start grid,
the approach to speed pad 2, spline row 2430 beside trees 755 and 756) show trees where the original shows them, and no
pose reachable by a hovering craft put a tree between the chase camera and the craft or a pad. What remains open is
which build or setting the report came from; it needs the maintainer's spot on the circuit.

Confidence: the layers and the order, **85** (a runtime trace of one frame of one circuit, two boots, the bump
reproducing on each; it stops at 85 because it is one state); the depth states, **85** (read off the GE command words
of the same frame); the claim that a nearer tree lies over the hull and pads in the original, **80** (inferred from the
order and the depth state, not seen as a pair frame: no pose put a tree over a craft in the original). Platinum Rush
Black was not reached (below). **Not measured:** the weapon pads' exact submit (`0x0890cdc8`, unread), the ship object's
prims, PS2.

**Cross-title check (CLAUDE.md).** Pure: **checked, applies, not wired** - the same `trail_order` path, its queue keys
unread. HD: **checked, differs** - `hd_trail_active` sends the circuit whole through `Draw::Track`, so its blended
list precedes the hulls and pads (the HD queue is a different pass, `docs/rendering/trail-ribbon.md`; its layers are
unread). 2048 and Omega: **not checkable here** - no queue read; they share HD's `ribbons_first() == false` path, so
any translucent scenery with no depth write is drawn before the craft there too. A reorder for HD/2048/Omega needs
that title's own queue keys first.

### The exhaust flare is the one thing that computes a depth

`ExhaustFlare_Submit` (`0x0890490c`, confidence 85):

```c
float z = Gfx_ViewDepth(node->world + 0x30);      // 0x0890486c
u32 depth = 0;
if (z < 0.0f) {
    float t = -(z * 349.525f);
    depth = t > 1048575.0f ? 0 : 0xfffff - ((u32)t & 0xfffff);
}
Gfx_Enqueue(g_display, self, depth | 0x4d000000);
```

`Gfx_ViewDepth` transforms the point by the view matrix with `vtfm4_q` and
returns the **z** component, so negative is in front of the camera. `0xfffff /
349.525 = 3000.0`, so the field spans **3,000 world units** and one step is
**0.00286** of one - fine enough that two flares essentially never collide. A
point carrying the `0x7f800001` sentinel gets `10000.0` instead, i.e. key 0.

Larger distance gives a smaller key, so ascending order draws the furthest
first: **this is a back-to-front sort, and it is why the transparent pipeline
can leave depth write off** rather than that being a stylistic choice.

### The shine pass is in the solid half of the queue

The `0x2000` extra pass (a circuit's `*_shinemap` second texture) is drawn from the
mesh's own list-A compiled lists, which are queued at the mesh's list-A key
(`0x45`/`0x4a`), so it draws **before** every list-B mesh (`0x4d`, depth-keyed).
**Measured on one recorded Moa frame (`psp-ge-dump.py`, 469 prims): all 59
environment-mapped circuit prims (`TEXMAPMODE` 2) are at indices 1 to 288, ahead of
the list-B block that starts at 411**; the two environment-mapped prims after it
are a blended batch's own second pass (node `530`, which ours does not draw, see
`mesh-draw.md`) and a hull's. Confidence 85 (one frame, one circuit).
Ours drew it after the blended list, which put an additive veil over every tree
that stands in front of a shine-mapped wall: the pale band over Moa Therma's trees
(tick 160, a crown over the grandstand: `122,141,134` against the original's
`49,50,38`; without the pass `46,48,37`, with the pass moved `52,52,40`). It now
draws between the sky and the blended list (`Scene::draw_track`, `TRACK_STAGES`).

## What `oag-render` implements, and what it does not

`DrawCall::layer` carries the recovered layer and `Model::sort_by_layer` puts all
three draw lists in ascending order by it, stably, **once at build time** - the
key mesh geometry is queued with does not depend on the camera, so neither does
the order. `mesh::build_with_textures` sorts before returning, so
`pvs::DrawSections` (built from those lists and index-parallel to them) needs to
know nothing about it; `mesh::merge` sorts again after merging, which is the
case where sorting *across* sources is right rather than an over-reach, because
the original's queue is global.

Measured over `Data.wad`, by
[`vex_layer_ground_truth.rs`](../../crates/vex/tests/vex_layer_ground_truth.rs):

| | `0x45000000` | `0x4a000000` | `0x31000000` |
| --- | ---: | ---: | ---: |
| mesh nodes (21,055 over 307 files) | 13,983 (66.4 %) | 7,072 (33.6 %) | 0 |
| transparent batches (5,610) | 3,573 (63.7 %) | 2,037 (36.3 %) | 0 |

Both are gates rather than trivia. A derivation that put every mesh on one layer
would be a correct reading of a field that distinguishes nothing, and a
transparent set that all landed on one layer would reorder nothing that shows -
only the transparent list is order-dependent, since opaque and cutout draws are
depth-tested with depth write on. **The `0x31` branch never fires on this disc**,
so it is implemented and unexercised.

Four ways this is not the original, all of them ours:

1. **A stable sort where the original's `qsort` is not.** Equal keys are
   freedom the original leaves; taking it deterministically is a choice, not a
   claim about which order it picks.
2. **Three lists rather than one queue.** `oag-render` draws opaque, then
   cutout, then transparent, each sorted; the original interleaves them within a
   batch set. The layer ordering is right *within* each list.
3. **A depth term for list B only, and the exhaust's own key is not
   implemented** - `oag_fx::exhaust` draws in its own pass. The batch-set path
   (list A) has no depth term, which is right.
4. **No `0x40000000` set and no reflection layers.** Neither is built, so
   neither is ordered.

## PS2: list B takes the PSP's depth key (inherited, 2026-10-31)

The PS2 executable's mesh submit is not read, and until this change PS2 VIF
batches carried no `DrawCall::depth_point`, so a PS2 circuit's blended meshes
drew in file order. In Talon's Junction (`16_Track\zone_track.vex`, Zone) that
put a far row of dark window panels in front of the nearer translucent grid
(maintainer report, tick 2100 near `[-198.8, -50.5, -769.8]`; our frame at that
pose shows a dark panel band where PSP shows white glow). PS2 list-B batches now
get the PSP's key (`Model::blended_order`), **inherited from PSP, no PS2
confidence**: the PS2 law itself stays open (read the submit in `SCES_547.48`
beside `Gfx_BuildBatchStateList` at `0x001e9088`). Pinned by
`crates/render/tests/ps2_blended_depth_ground_truth.rs`.
