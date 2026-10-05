# Ship parts: tagged nodes, and the handler that only a live read found

Opened while implementing `Airbrake` (`0x3c5`), the roadmap's "the airbrakes
visibly deploy" item. It found a negative result about the binary (the class
system has no per-class dispatch for a tagged node) and, on a second pass with
a live PPSSPP session, the per-instance handler that dispatch had been hiding -
see [the live read](#the-rotation-axis-recovered-from-a-live-read) below.

## What the file says, which is more than the class census did

`Data\Ships\<Team>\Ship.vex`, read with `oag-view --nodes`:

```
0x06e Transform  locator1              64 byte(s), 1 child
  0x3c5 Airbrake   Airbrake_Left        0 byte(s), 1 child
    0x125 Mesh       Airbrake_LeftShape   704 byte(s)
```

Three facts, all checked on **all eight** playable teams by
`crates/render/tests/airbrake_flaps_ground_truth.rs`:

- **`Airbrake` parents real geometry.** It has exactly one `Mesh` child. An
  earlier note in `HANDOVER.md` said the opposite - that `Airbrake` nodes are
  *siblings* of the meshes and parent nothing - and that was read off
  **`shipboost.vex`**, the boost plume, where two teams carry stray `Airbrake`
  nodes with no children. Right about that file, wrong about this one, and it
  nearly cost the whole feature: the conclusion drawn from it was "this needs a
  renderer rewrite before it can start".
- **The hinge is the grandparent `Transform`**, whose 64-byte payload is a 4x4.
- **The two locators are mirror images in X**: `+1.5246` and `-1.5246` on
  Assegai, and straddling on every other team (`+0.8287`/`-0.8287` on Feisar up
  to `+2.2922`/`-2.2922` on Goteki). The sides are named by the artists,
  `Airbrake_Left` on the `+x` locator, so nothing has to infer a side from a
  sign.

Confidence **90** for the layout: read from the decoded tree of eight files, not
from one.

## The `<AirbrakeGraphics>` rates are not the force law's

`HandlingXml_ParseAirbrakeGraphics` (`0x08839c68`) stores three floats at
`+0x6c`, `+0x70` and `+0x74` of the block the camera parameters end at - see
[camera.md](camera.md), which found them and established that `amount` is
scaled by `pi/180` at load and is therefore an **angle**. Confidence 92.

The authored values, every playable team:

| | `amount` | `up_speed` | `down_speed` |
| --- | ---: | ---: | ---: |
| every team | 20 to 35 degrees | **500** | 80 to 120 |

**`up_speed` is 500 on all eight**, against a `down_speed` between 80 and 120.
That asymmetry is what identifies the units. `<Airbrake gain/falloff>`, one
element away, ramps the *physics* airbrake on the `0..=100` scale, and on that
same reading a flap deploys in `0.2 s` and folds back over about a second -
which is what an airbrake does. Read instead as fractions of full deflection
per second, `500` would be full travel in two milliseconds and the parameter
would not be worth authoring. Confidence **70**: an argument from the authored
values and from the neighbouring element's convention, with no consumer located.

So the flap and the airbrake it depicts ramp at **different rates**, which is
the game saying outright that this is graphics. It is why the deflection lives
on `oag_game::race::Race` beside the boost's FOV kick rather than in
`ShipState`, where it would move the determinism hashes every time somebody
adjusted an animation.

## The negative: class-id dispatch has no handler

The rotation axis is not in the file - the `Airbrake` node's payload is **zero
bytes**. The obvious next step is to read what the engine does with the class,
and *that* dispatch does not exist. Five searches, five dead ends:

- No `0x3c5` immediate anywhere in `BOOT.BIN` (`li`, `ori` and `addiu` all
  return nothing), so nothing looks the class up by number.
- The class name string `Airbrake` (`0x08a84e68`) has **no code reference**. It
  is reached from a static descriptor table at `0x08ab2468`, whose records are
  `{base descriptor, class id, name}` - twelve bytes, and **no handler
  pointer**.
- That table entry has no references either.
- `Vex_FindClassDescriptor` (`0x08908b68`) has exactly two callers,
  `FUN_08908f98` and `Vex_LoadModel` (`0x08912b80`). Both are **load time**. The
  class system is not consulted per frame.
- `Exhaust_Update` (`0x089058b0`) - the worked example of the same shape, a
  tagged node the ship code drives every frame - has **no direct callers at
  all**.

Taken together: a tagged part is found by walking the ship's tree through an
indirect dispatch, and there was no per-class function to decompile *from the
class table*. That conclusion still stands. What it does not rule out, and what
cost an hour to learn the hard way, is a per-instance handler reached some other
way - which is exactly what the live read below found.

## The rotation axis, recovered from a live read

The open question was exactly this: break under PPSSPP and watch what moves.
The static search above had already shown class-id dispatch does not exist,
so the productive move was a **read watchpoint**, not another execution
breakpoint.

1. **A player craft's `craft+0x2d8`/`+0x2dc`** (`HandlingXml_ParseAirbrakeGraphics`'s
   two graphics-only deflection states, `engine.md`) ramp cleanly under a live
   session: held, `craft+0x2d8` climbs to 100 within the game's `up_speed`
   window; released, it decays 100 -> 76.6 -> 53.3 -> 29.9 -> 0 over four
   samples 0.3 s apart - **83 units/s**, inside the authored `down_speed` range
   (80-120) on every team. That alone raises `craft+0x2d8`/`+0x2dc` and the
   "these are graphics-only deflection states" reading from 80 to **92**: a
   live decay rate matching the authored rate is a stronger form of the same
   argument the file-only reading made.
2. **A `memory.breakpoint.add` read watchpoint on `craft+0x2d8`**, `log: true`,
   armed for one second while the deflection was nonzero, caught 3,210 hits at
   three program counters: `0892da28`/`0892da2c` (**1,929 hits**, inside
   `FUN_0892d9fc`) and two addresses inside `Ship_UpdateAirbrakes`
   (`0x0884c9a4`, already named, 88 confidence, `engine.md`) at 643 each - the
   physics-side consumer. `FUN_0892d9fc` is the graphics-side one, and by far
   the dominant reader.
3. **Decompiling and disassembling `FUN_0892d9fc`** (now named `Airbrake_Update`
   at `0x0892d9fc`, confidence 90) shows exactly the shape `Exhaust_Update` predicted: reached
   with no direct callers (indirect dispatch, same as the class-table search
   above found for every tagged node), taking the node itself as `param_1`.
   It reads a left/right flag off `param_1+0x64`, walks `param_1+0x68` ->
   entity -> `entity+0x94` -> craft (the reciprocal `ENTITY_OWNER` edge
   `engine.md` already established) to reach `craft+0x2d8`/`+0x2dc`, divides
   by 100, and - if nonzero - multiplies by the handling stats block's
   `+0x6c` (`amount`, camera.md/this page, confidence 92) to get an angle in
   radians. Zero deflection branches to a separate function (stow / clear
   override, `FUN_00141588`); nonzero builds a matrix and installs it via
   `FUN_00141284(node, &matrix, 0)`. Both callees are unrelocated import-stub
   constants in this project's Ghidra import and did not resolve to real
   addresses here.
4. **The matrix itself, read off the raw VFPU disassembly rather than the
   decompiler's translation** (Ghidra resolves the `vpfxs` prefixes to
   swizzles directly):
   ```
   vcos.s S010,S003          ; cos(angle)
   vsin.s S012,S003          ; sin(angle)
   vpfxs [1,0,0,0] ; vmov.q C100,C010   row0 = (1, 0, 0, 0)
   vpfxs [0,X,Z,0] ; vmov.q C110,C010   row1 = (0, cos, sin, 0)
   vpfxs [0,-Z,X,0]; vmov.q C120,C010   row2 = (0, -sin, cos, 0)
   vpfxs [0,0,0,1] ; vmov.q C130,C010   row3 = (0, 0, 0, 1)
   ```
   Row-major, four rows `(1,0,0,0)`/`(0,cos,sin,0)`/`(0,-sin,cos,0)`/`(0,0,0,1)`
   - a textbook rotation about **local X**, the row that never varies. That is
   the same axis `oag_mesh::mesh::Flap::deflect` already used, chosen rather
   than recovered at the time.
5. **The unit conversion is its own corroboration.** The angle is scaled by
   the VFPU constant `2/PI` (Ghidra names it `vcst.s S002,2/PI` directly)
   before `vcos.s`/`vsin.s`, which is exactly the radians -> PSP quarter-turn
   scale that `amount`-is-radians (confidence 92, argued from authored values
   alone) predicts mechanically. A wrong reading of `amount`'s units would not
   produce the one scale factor that makes this trig call self-consistent.

**Confidence 90** for the axis: recovered from live disassembly of the actual
per-instance handler, not inferred from file structure, corroborated by an
independent watchpoint count and by the unit-conversion constant. Raised from
below 50 (chosen, not recovered) at the point this section was written.

## What was implemented

`oag_mesh::mesh::Flap` carries the vertex span and the hinge; `Flap::deflect`
returns `hinge * R * hinge^-1`, because `build_with_textures` has already baked
every ancestor transform into the vertices and rotating them directly would
swing the flap about the model's origin six units away. `R` is local X, which
is now a recovered finding (confidence 90, above) rather than a chosen one -
`Mat4::from_rotation_x` composes the same rotation `Airbrake_Update`'s matrix
does; the sign is column-major-vs-row-major transpose of the same rotation, and
the existing mirrored-hinge tests already pin the resulting direction against
what an airbrake should do.

Everything else was already recovered: the hinge pose (90), the deflection in
radians (92, now 92 from a live rate match too), and the two rates (raised to
92 in the section above).

## Cross-platform

Not checked against `SCES_547.48` (PS2). The `.vex` class ids are shared across
the two builds, so the tree shape is expected to hold; the descriptor table's
address is not.
