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
