# Ship parts: tagged nodes, and why one of them has no handler to find

Opened while implementing `Airbrake` (`0x3c5`), the roadmap's "the airbrakes
visibly deploy" item. It ends in a **negative result about the binary** and a
positive one about the files, and the negative is the part worth writing down:
there is no per-class update function for a tagged ship part, so looking for one
is a search that cannot terminate.

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

## The negative: there is no handler to find

The rotation axis is not in the file - the `Airbrake` node's payload is **zero
bytes**. The obvious next step is to read what the engine does with the class,
and that step does not exist. Five searches, five dead ends:

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
indirect dispatch, and there is no per-class function to decompile. **Anyone
who tries this again will spend the same hour.** The way in, if it is ever
worth it, is a live read: break in the ship update under PPSSPP and watch which
node matrix changes when the airbrake goes down. That is a measurement, not a
code read.

## What was implemented, and what is flagged

`oag_render::mesh::Flap` carries the vertex span and the hinge; `Flap::deflect`
returns `hinge * R * hinge^-1`, because `build_with_textures` has already baked
every ancestor transform into the vertices and rotating them directly would
swing the flap about the model's origin six units away.

**The axis is chosen, not recovered.** Local X, because it swings the flap the
way an airbrake looks like it should, and because the mirrored hinges then make
the two sides open apart with one angle for both and no sign flipped by hand.
Under the [confidence rubric](../../../reverse-engineering/confidence-rubric.md)
that is below 50, so it is not named as a finding anywhere and the doc comment
on `deflect` says so.

Everything around it is recovered: the hinge pose (90), the deflection in
radians (92), and the two rates (70 on their units).

## Cross-platform

Not checked against `SCES_547.48` (PS2). The `.vex` class ids are shared across
the two builds, so the tree shape is expected to hold; the descriptor table's
address is not.
