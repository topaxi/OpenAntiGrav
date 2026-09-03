# The `Dynamic Shadow Occluder` runtime reader

Step 4 of [`docs/rendering/shadows.md`](../../../rendering/shadows.md)'s plan
decoded the `0x3c3` payload's two record arrays
(`crates/formats/tests/shadow_occluder_ground_truth.rs`) but left one thing
the data alone cannot answer: how the hull is *projected* into a shadow. This
page reads the function that does it.

**Static reading of `psp-pulse-usa`'s `BOOT.BIN`. Not runtime-verified.**

## The cast

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089038c8` | `Shadow_RenderOccluderVolume` | 80 |

Confidence is capped at the rubric's **70-84 Probable** band: strong,
convergent structural evidence, no runtime trace.

## Read this before trusting an address

Same trap as [`autopilot.md`](autopilot.md) and
[`positional-audio.md`](positional-audio.md): this import carries
**unrelocated address constants**. A pointer stored in `.data` prints
`base_address - 0x08804000` short, so `get_xrefs_to` and
`get_function_callers` are silent for this function - confirmed directly,
both return nothing for `0x089038c8`. The one static reference that exists
was found by computing the unrelocated form (`0x089038c8 - 0x08804000 =
0x000ff8c8`) and searching for its little-endian bytes as data, not as a
`jal` operand - see below.

## What it operates on

`Shadow_RenderOccluderVolume(void *self)` reads a structure pointer at
`self+0x50`. Three of the fields it reads from what that pointer points to
match [`shadow_occluder_ground_truth.rs`](../../../../crates/formats/tests/shadow_occluder_ground_truth.rs)'s
already-pinned payload layout exactly, at the same byte offsets, independent
of this reading:

- **`+0x00`**: a `u16`, sanity-checked `< 0x100` before anything else runs.
  This is `n`, the face-record count - the ground-truth test's own sanity
  bound on the same field (`PSP_OCCLUDERS` tops out at 129 nodes, `n` values
  measured over lengths 272..4432, always small).
- **`+0x02`**: a `u16` read as the vertex-record count `m` (`*(short
  *)(*(int *)(self + 0x50) + 2)`) - the second header field the ground-truth
  test pins.
- **`+0x30`/`+0x40`**: read as a padded `vec4` AABB min and max
  (`iVar30+0x30/0x34/0x38` and `+0x40/0x44/0x48`), used to pick whichever box
  corner is farthest along the projection direction. This is the *same* field
  the design doc calls "the padded `vec4` bbox copy at `+0x30`/`+0x40`",
  found independently by the payload-closure sweep and confirmed to match the
  packed box at `+0x0c`/`+0x18` on 97 of 129 nodes.

Three independently-pinned struct offsets landing exactly where this function
reads them is the strongest evidence available short of the class-ID
dispatch table actually naming the handler (below).

## What it does

1. **Builds the node's own world matrix** from `self+0x30` (a source 4x3/4x4
   transform) into scratch storage, the ordinary "compose this node's
   placement" step every scene-graph node does.
2. **Derives the shadow direction from the node's own orientation, not any
   light.** A fixed local axis constant (one of two candidates, selected by a
   global flag) is transformed by the node's own rotation and normalized.
   Nothing in this function reads a light direction, a `DirectionalLight`
   register, or any other external source. **This answers
   [`shadows.md`](../../../rendering/shadows.md)'s open question "where the
   light direction for a Pulse shadow comes from"**: it does not come from a
   light at all. Each occluder (and the craft's own drop shadow, below)
   projects along its own authored local axis, transformed into world space -
   consistent with the rest of the disc's evidence that Pulse has no light
   rig to take a direction from.
3. **Finds how far to project**, branching on whether the node is
   free-standing (`self+0x84 == 0`, the track-side occluder case) or attached
   to a parent (the craft-shadow case, below):
   - Free-standing: tests up to five nearby plane-shaped records for the
     first one the projected ray crosses, capped at a maximum distance of
     `1000.0`.
   - Attached: reads a height field off the parent object directly
     (`parent+0x94` then `+0x2f8`, offset by `10.0`) and divides by the
     projection direction's vertical component - a cheap "drop straight to
     the known ground height" shortcut rather than a ray cast.
4. **Extrudes**: writes a small `0.2`-scaled bias offset and the full
   projection-distance offset into two scratch vectors, then walks the `m`
   vertex records building near-cap and far-cap points for each - textbook
   shadow-volume extrusion (a near bias to avoid self-shadowing, a far cap at
   the projected distance).
5. **Silhouette test**: walks the `n` face records, dot-producting each
   face's stored normal against the projection direction from up to four
   reference points and recording a front/back-facing byte per face, then
   builds an edge list from the boundary between them - the standard
   "silhouette edges separate front-facing from back-facing polygons" step of
   stencil shadow-volume construction.
6. **Draws**, through a short run of calls whose shape matches a stencil
   pass: a state/matrix select, a draw call using the edge-list vertex/index
   buffers just built, and calls bracketing it that read as enabling and
   restoring stencil test state.

## The craft's own drop shadow is the same function

[`exhaust.md`](exhaust.md#the-crafts-075-render-scale-confirmed-three-ways---and-a-residual-that-is-not-it)
already named `FUN_089038c8` from an unrelated angle - tracing
`g_craft_scale` (`_DAT_002ace1c`) - and described it as "a stencil
shadow-volume pass over the craft; it computes its ground projection, scales
it by `1.0 / g_craft_scale`". That is the `self+0x84 != 0` branch above: this
single function renders **both** the craft's own drop shadow (attached mode)
and every free-standing `Dynamic Shadow Occluder` hull on a circuit
(unattached mode, reading the `0x3c3` payload). Two independent readings -
one from a scale-residual investigation, one from the occluder payload's own
field layout - landed on the same function and agree on what it does where
they overlap (the `1.0 / g_craft_scale` division matches `_DAT_002ace1c`
exactly). That agreement is most of why this clears 80 rather than sitting at
70.

## Why it has no callers `get_xrefs_to` can find

Vex node classes dispatch through a per-class method table, not an inline
compare - `exhaust.md`'s reading of `Vex_RegisterClass` already established
this ("Class dispatch is by descriptor lookup, never by immediate compare...
the registration call site is the only route"). Consistent with that: the
only static reference to this function in the whole binary is as **data**,
not as a `jal` target. Searching for the unrelocated pointer's raw
little-endian bytes (`c8 f8 0f 00`) finds exactly one hit, at `0x08ad1258` -
slot 15 (0-based) of an 8-byte-stride table, the same shape `exhaust.md`
describes for a class's method table. `0x08ad1258` sits below
`Vex_RegisterClass`'s documented base table for `Transform`
(`0x08ad22f4`), consistent with - but not proof of - an alphabetically
earlier class name such as `Dynamic Shadow Occluder`.

**Not traced further**: which of `Vex_RegisterClass`'s 46 callers assigns
this particular table, which would confirm the `0x3c3` link statically rather
than by structural inference alone. That is real, additional work (walking
call sites whose class-id argument is itself an unrelocated constant), not
attempted this pass.

## Open

- **The class-registration call site.** Finding the `Vex_RegisterClass`
  caller that registers `0x3c3` and assigns the method table containing
  `0x08ad1258` would move this past the rubric's "Probable" band without
  needing a runtime trace - the missing static link, not the missing
  behaviour.
- **The fixed local axis constant's value.** Read at one of two addresses
  selected by a global flag; both are unbacked by concrete bytes in this
  Ghidra import (no `.data`/`.rodata` content at either), so the axis itself
  (down? forward? something authored per-title?) is unread.
- **The four-reference-point silhouette test.** Faces are tested against the
  projection direction from up to four slightly different points rather than
  one; which four and why is not worked out.
- **The near-cap bias constant (`0.2`) and far-cap distance cap (`1000.0`)**
  are read directly off the decompilation, not cross-checked against any
  disc-authored value.
- **No runtime trace exists for any of this.** A PPSSPP watchpoint on
  `self+0x50` during a lap that passes a known occluder-bearing circuit
  section would be the fastest way past 84.
