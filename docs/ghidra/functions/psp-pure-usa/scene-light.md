# Pure's hull light list: the circuit's own lights, under Pure's own class ids

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Read 2026-10-26 (lane `pure-render-fidelity`), static
only: decompiles and the instruction search, no live read. Companion to Pulse's
[scene-light.md](../psp-pulse-usa/scene-light.md), which read the same machine live and
reproduced its list from the disc to the unit.

## What was wrong

Pure's hull drew 0.39 to 0.78 of the original's brightness and did not move with the
circuit (the pairs in [pure-status.md](../../../formats/pure-status.md#pures-hull-takes-the-circuits-own-lights-2026-10-26)).
The loader said `hull lights: the circuit authors none`. That was a reader fault, not the
data: `oag_vex::lighting` matched `AmbientLight` and `DirectionalLight` against Pulse's
class ids `0x12c` and `0x131`, and `oag_vex::world_lights` matched `wopoint`/`wospot`
against `0x3cf`/`0x3ce`. Pure renumbers every class ([pure-status.md](../../../formats/pure-status.md#the-class-id-space-is-renumbered)),
so none of the four matched anything on a Pure file.

## The light nodes are there

Every one of Pure's 16 circuits authors its lights, under these ids, each node keeping
Maya's own shape name (`crates/pure/tests/class_table_ground_truth.rs`,
`the_light_classes_are_on_every_pure_circuit`):

| Class | Pulse | Pure | Payload | On Pure's 16 circuits |
| --- | ---: | ---: | --- | --- |
| `AmbientLight` | `0x12c` | `0x125` | 16 bytes, `ambientLightShape1` and the like | 21 nodes, one or two per circuit |
| `DirectionalLight` | `0x131` | `0x12a` | 16 bytes, `directionalLightShapeN` | 30 nodes, none on the four classics, up to five on Sol 2 |
| `wospot` | `0x3ce` | `0x380` | 32 bytes, `wospotShapeN` | 8, all on Chenghou Project |
| `wopoint` | `0x3cf` | `0x381` | 32 bytes, `wopointShapeN` | 654 over 7 circuits (130 on Modesto Heights) |

The two generic Maya classes sit seven below Pulse's, as `Mesh` does (`0x125` to
`0x11e`). The two track-light classes sit between `Reset Collision` (`0x37f`) and
`Ship Collision Fx` (`0x382`), the same slots they take between `0x3cd` and `0x3d0` on
Pulse. Confidence **90** for all four: exact names, sizes and counts on every circuit,
and the consumer below read; not watched live.

## The consumer is Pulse's builder

`FUN_08841648` (named `SceneLight_BuildLightingList` here, confidence **85**) was found by
the one `lui 0x5d00` pair outside the GE wrapper: the `0x5d0000ff` ambient-alpha word Pulse's
`SceneLight_BuildLightingList` (`0x0887a4f0`) writes when its context has no light list.
The decompile matches Pulse's statement for statement:

- the same signature (`dt, context, model, two countdown pointers, four scale bytes,
  display list`), the same `LIGHTINGENABLE` (`0x17000001`) first;
- with no light list (`context + 0x6d0`, Pulse's `+0x6cc`) the same grey
  `0x30 * scale >> 8` ambient and one default light at full white (`0x18000001`,
  `0x63`..`0x65`, `0x5f`, `0x8f`, `0x90`, `0x91`);
- otherwise the summed ambient (`+0x40` count, `+0x58` list; Pulse `+0x3c`, `+0x54`),
  one GE light per directional, and the point-class slots after them, each colour byte
  `min(255, trunc(c * 255)) * scale >> 8`.

Every context field is four bytes later than Pulse's, which is the whole difference read.
So the nodes above reach the hull the way Pulse's do, and
`oag_pure::effects::LOOKS.hull_lights` turns the same rig on for Pure's PSP.

**The scale bytes the builder takes are the spline's, as on Pulse.** `FUN_08843d1c` is
Pulse's `Spline_SampleSegment` (`0x0887c7e8`): the same `vfim 1/6` B-spline basis over
four points, the same three calls to an accumulator after the first point is stored, and
the same tail, `slti a0, a0, 0x103` at `0x08844164` and four `sb 0xff` to
`+0x62..+0x65` below that version. Every Pure circuit's `WO Track` is version `0x103`, so
its authored bytes are read (Vineta K's first point authors `[89, 89, 255, 255]`), and
ours already applies them per craft through `oag_race::Course::light_scale_at`, which
is title-agnostic. Confidence **80** for the sampler (disassembly read against Pulse's).
**Not read:** Pure's twin of `Craft_ApplyHullLightScale` (`0x0883e444`), the copy of the
blended bytes onto the hull model; ours assumes Pulse's.

## The mesh draw takes the node's world matrix, rotation included

`FUN_0886cf04`, the `Mesh` node constructor (the twin of Pulse's `0x0890c2bc`,
[mesh-draw.md](../psp-pulse-usa/mesh-draw.md)), builds the display-list fragment at
`mesh + 0x184`: `0x3a000000` (`WORLDMATRIXNUMBER` 0), then twelve
`0x3b000000 | (word >> 8)` from words 0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14 of
`*(mesh + 0x30)`, then `0x0b000000` (`RET`). That is the full 4x3 matrix, so the GE
takes a vertex normal through the node's rotation on Pure as on Pulse, and
`oag_mesh::mesh` carrying a normal through its node (the 2026-10-26 normals change,
[vex.md](../../../formats/vex.md#a-vertex-normal-is-in-its-nodes-space-and-the-nodes-rotation-carries-it))
is right for Pure. Confidence **85**: the constructor read, its callers not.

## Result

Four compare pairs, mean RGB of a 10x14 patch of the teal body at `(200, 222)`,
original / before / after, and grey MAD over the hull region `(190..290, 175..272)`:

| Circuit | original | before | after | hull MAD before -> after |
| --- | --- | --- | --- | --- |
| Vineta K | 110 128 113 | 60 75 68 | 88 110 98 | 50.28 -> 33.79 |
| Chenghou Project | 108 121 100 | 62 71 58 | 87 107 88 | 39.65 -> 27.17 |
| Blue Ridge | 108 128 110 | 59 75 71 | 99 121 109 | 36.75 -> 20.49 |
| Modesto Heights | 80 94 83 | 46 52 41 | 77 91 80 | 16.32 -> 15.73 |

Modesto's yellow wing panels turn the original's orange under its warm ambient and its
130 `wopoint`s. Pulse's output is unchanged: its tables keep the version-6 ids, and three
Pulse compare pairs (Arc Prime White, Fort Gale Black, Amphiseum White) render
pixel-identical before and after.

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x08841648` | `SceneLight_BuildLightingList` | 85 | Pulse's `0x0887a4f0`, context fields four bytes later. |
| `0x08843d1c` | `Spline_SampleSegment` | 80 | Pulse's `0x0887c7e8`, the same `< 0x103` scale-byte gate. |
| `0x0886cf04` | not renamed | 85 | The `Mesh` node constructor; writes the world-matrix fragment. |
