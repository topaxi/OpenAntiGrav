# Pads

How a `Speedup Pad` is bound, how a craft is tested against one, and what the
test hands back. The force the hit then applies is
[`Ship_ApplySpeedupPad`](engine.md#the-track-section-force-is-the-speed-pad-boost-and-it-is-the-wrong-sign)
in `engine.md`; this page is the trigger side.

The data side - the payload layout and what the shipped tracks author - is
[`docs/formats/pads.md`](../../../formats/pads.md). The reimplementation is
`crates/formats/src/pads.rs`.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089264f4` | `Pad_Bind` | 85 |
| `0x088866bc` | `Pad_ContainsPoint` | 85 |
| `0x0888686c` | `Pad_SweptTest_q` | 65 |
| `0x08887144` | `Pads_TestCraft_q` | 65 |

## `Pad_Bind` (`0x089264f4`)

The class `0x3bd` bind handler, from the class table in
[`vex.md`](../../../formats/vex.md).

```c
uVar1 = Mesh_Bind(param_1, param_2);        // 0x0890e998
iVar2 = *(int *)(param_1 + 0x58);           // the payload the Mesh bind parked
*(param_1 + 0x1b0) = *(iVar2 + 0x10);       // four words: box minimum
*(param_1 + 0x1b4) = *(iVar2 + 0x14);
*(param_1 + 0x1b8) = *(iVar2 + 0x18);
*(param_1 + 0x1bc) = *(iVar2 + 0x1c);
*(param_1 + 0x1c0) = *(iVar2 + 0x20);       // four words: box maximum
*(param_1 + 0x1c4) = *(iVar2 + 0x24);
*(param_1 + 0x1c8) = *(iVar2 + 0x28);
*(param_1 + 0x1cc) = *(iVar2 + 0x2c);
*(float *)(param_1 + 0x1b4) -= 2.0;         // min.y
*(float *)(param_1 + 0x1c4) += 8.0;         // max.y
for (i = 0; i < 8; i++) *(param_1 + 0x1d0 + i * 4) = 0;
```

**The first call is what settles the class.** `0x0890e998` is the `Mesh` bind:
it fixes up vertex and index pointers, walks the material array at `+0x30`,
resolves textures, and builds the GE colour commands at `+0xdc`. So a
`Speedup Pad` **is a `Mesh`**, its payload is a mesh payload, and the pair of
`vec4`s it then reads at `+0x10` and `+0x20` is the mesh's own bounding box -
already documented at that offset in [`vex.md`](../../../formats/vex.md).

That is the finding that makes the pads drawable at all: they carry geometry, in
the track file, and nothing but the class id distinguishes them from any other
mesh. Confidence **85** - unambiguous decompilation, and the payload agrees
across all 762 pad nodes on the disc.

The vertical expansion is asymmetric and in `+y`, which is world up (see
[`track.md`](../../../formats/track.md)), so a pad is entered from above. It
turns a plate about 0.15 units thick into a volume about 10 units tall.

The eight words zeroed at `+0x1d0` are one per racer. What they hold is not a
latch - see `Pad_SweptTest_q` below.

### The literals are only meaningful because the scale is 1

The `Mesh` bind divides the same `+0x10`/`+0x20` floats by a quantisation scale
before storing its own bounds at `+0x80`. `Pad_Bind` does **not**: it copies them
raw and adds `-2.0` and `+8.0` in world units. That is only coherent if a pad
mesh's scale is `1.0`.

Checked rather than assumed: all 544 `Speedup Pad` boxes on the 40 PSP track
files come out between **9.55 and 9.71 units wide**, which is a craft's width and
one authored size. A quantisation scale would vary per mesh and would put these
orders of magnitude out. Asserted in
`crates/formats/tests/pads_ground_truth.rs`.

## `Pad_ContainsPoint` (`0x088866bc`)

```c
m = *(*(pad + 8) + 0x30);                    // the node's 4x3 matrix
d = vsub_t(point, *(m + 0x30));              // minus the translation
local = vtfm3_t(m, d);                       // into pad-local space
for each axis:
    if (local[a] < min[a])      out[a] = min[a] - local[a];
    else if (local[a] > max[a]) out[a] = max[a] - local[a];
    else                        out[a] = 0.0;
*param_4 = sqrt(dot(out, out));              // the caller's per-racer word
return (that == 0.0 && *(pad + 0x1a0) == 0.0);
```

Three things fall out, and all three matter to a reimplementation:

1. **The test runs in pad-local space.** The box at `+0x1b0`/`+0x1c0` is never
   compared against world coordinates; the point is inverse-transformed first.
   Rows 0 to 2 of a `.vex` matrix are orthonormal, so the inverse rotation is a
   dot against each row. Pads are rotated to follow the track, so an
   axis-aligned world-space test is not an approximation of this - it is a
   different answer.
2. **What it returns through `param_3` is the matrix, not the pad.** This is what
   `engine.md` previously read as a "section" with a "direction at `+0x20`". The
   matrix is at node `+0x30` in the row-major, translation-in-row-3 layout every
   `.vex` uses, so `+0x20` is floats 8 to 10: **row 2, the pad's local `+Z`**.
   That is the direction the boost pushes along.
3. **`pad+0x1a0` gates every hit.** A non-zero value suppresses a hit that is
   geometrically inside. **The writer was not found.** A `Weapon Pad`'s
   `<WeaponPad refresh_time>` is the obvious candidate, which would make this a
   pickup-respawn timer that a speedup pad never sets. `crates/formats/src/pads.rs`
   carries the term inert rather than dropping it.

Confidence **85**: unambiguous decompilation, and the direction reading is
corroborated by the pads' own geometry being flat plates whose local `+Z` runs
along the track.

## `Pad_SweptTest_q` (`0x0888686c`)

Named with `_q`: the function is read confidently, but the reading of its `25.0`
branch below is an inference.

```c
w = pad + slot * 4;
*(float *)(w + 0x1d0) -= moved;              // `moved` = |position - previous|
if (*(float *)(w + 0x1d0) > 0.0) return 0;   // cannot have reached it yet
if (moved < 25.0 && dot(previous, previous) > 0.0) {
    for (t = 0.25; t <= 1.0; t += 0.25)
        if (Pad_ContainsPoint(pad, lerp(previous, position, t), out, w + 0x1d0))
            return 1;
    return 0;
}
return Pad_ContainsPoint(pad, position, out, w + 0x1d0);
```

**The eight words at `+0x1d0` are a distance cache, not a trigger latch.** Each
holds the distance from that racer to this pad, as
`Pad_ContainsPoint` last measured it, and is decremented by how far the racer
moved since. The full test only runs once the racer could plausibly have reached
the pad. `docs/formats/track.md` previously recorded these as "per-ship trigger
latches"; they are the opposite - an optimisation, with no bearing on whether a
pad fires twice.

The four-point interpolation is a swept test against tunnelling: at 120 units a
second and a 60 Hz step a craft moves about 2 units a tick, but a pad is only
about 9 units long and a dropped frame is enough to step over one.

**The `moved < 25.0` guard is read as a teleport reject** - a respawn or a
warp moves further in one step than any craft drives, and sweeping across it
would sample a line through arbitrary geometry. That reading is an inference from
the magnitude, not from anything the code says, which is what holds this at
**65**. What would retire it: a breakpoint under PPSSPP showing the branch taken
on a respawn and not in ordinary driving.

## `Pads_TestCraft_q` (`0x08887144`)

```c
moved = |position - *(world + slot * 0x10 + 0xb50)|;   // since last tick
*out = 0;
hit = false;
for (i = 0; i < *(int *)(world + 0x108); i++)
    hit |= Pad_SweptTest_q(moved, *(world + i * 4 + 0x40), position,
                           world + slot * 0x10 + 0xb50, out, slot);
*(world + slot * 0x10 + 0xb50) = position;             // for next tick
return hit;
```

Walks a list of pads and remembers each racer's previous position at
`world + slot * 0x10 + 0xb50` - which is where `Pad_SweptTest_q`'s `previous`
comes from, and why the first tick of a race (`previous == 0`) skips the sweep.

**65 rather than 85.** That `DAT_08b32c88` - the object `Ship_ApplySpeedupPad`
passes as `world` - is specifically the *speedup*-pad list is inferred from the
call site. The registrar that fills `+0x40` was not read, so nothing here rules
out one list holding both pad classes and the caller filtering. What would retire
it: reading the writer of `+0x108`.

## What is not implemented

- **`pad+0x1a0`'s writer.** Carried inert. See `Pad_ContainsPoint` above.
- **Weapon pads.** The payload decodes identically and is asserted against the
  disc, but nothing consumes one: there is no pickup system.

## History

- **2026-08-03** - page created. `Pad_Bind` and `Pad_ContainsPoint` at 85,
  `Pad_SweptTest_q` and `Pads_TestCraft_q` at 65. Supersedes two readings in
  `engine.md`: the record `Ship_ApplySpeedupPad` locates is a pad's **matrix**
  rather than a track "section", and the counters it bumps on entering a new pad
  are pad-visit statistics rather than lap and sector counting.
