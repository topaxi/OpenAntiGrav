# `oag-game`

The composition root, and the only binary that is the game rather than a tool for
looking at it. It has two halves, and one leads into the other.

- **The front end**, which is where it boots: the intro reel and the Language
  Selection screen, both driven by the disc's own data. Everything about it is on
  [front-end boot](../architecture/frontend-boot.md).
- **A race**, which is what picking a language starts: a track and a ship loaded off
  the same disc, the simulation stepped at a fixed 60 Hz from the keyboard, drawn
  from behind the ship by the chase camera the ship's own data describes.

`--race` skips the front end and goes straight to the second. It is the same race by
the same route through the same code; what it saves is parsing the menus and
transcoding the intro.

This page is about the race. Nothing is written anywhere and the disc image is opened
read-only.

## Running it

```sh
just play           # intro, menu, then a race
just play --race    # the race on its own
```

Arrow keys steer, X or Return thrusts, Q and E are the left and right airbrakes,
Escape quits. The keys are the same
[abstract button layer](../ghidra/functions/psp-pulse/input.md) the front end uses;
there is one mapping from a keyboard to a snapshot and both halves go through it,
one `Keyboard` for the whole session, so a key held across the handoff stays held.

```sh
# No display needed: run the simulation for 45 ticks and write one frame.
just play --race --screenshot /tmp/race.png --ticks 45 --hold cross

# Or through the menus, which is the same picture reached the way a player does.
just play --screenshot /tmp/launch.png --until "Launch Game" --press start,cross --ticks 60
```

`--screenshot` renders through the same scene the window draws, so what it captures
is what a player would have seen. `--ticks` advances the simulation first and
`--hold` holds abstract buttons - `cross` for thrust, `left`, `right`, `l`, `r` -
on every one of those ticks, the same spelling both flags have in the front end.

A capture that reaches `Launch Game` follows the handoff: the front end's leg ends
there whatever `--until` says, and what is left of `--ticks` is spent on the race.
Note that `--press` **pulses** its buttons on alternating ticks, because two rising
edges are what it takes to skip the intro and then pick a language - so the same
`--press start,cross` that gets there leaves the throttle on only half of the race's
ticks, and the ship is correspondingly slower than under `--hold cross`.

| Flag | What it does |
| --- | --- |
| `--race` | Go straight to the race. Checked before the front end loads anything, so it never parses the menus or transcodes the intro. |
| `--track <name>` | The track's `.vex` entry in `Data.wad`. |
| `--team <name>` | Which team's `handlingstats.xml` and model to fly. |
| `--class <name>` | `venom`, `flash`, `rapier` or `phantom`. |
| `--art` | Draw the track's art meshes instead of its driveable ribbon. |
| `--log-every <n>` | Print a telemetry line every `n` ticks. `0` for none. |
| `--size <WxH>` | With `--screenshot`, the image's size. Default `1440x816`. |
| `--dry-run` | Report what was loaded and exit. |

All of them apply to a race started from the front end as well, since it is the same
race. `--class` in particular is parsed before anything is loaded, so a misspelling
is not discovered eight seconds of intro later.

`--dry-run` is worth knowing about on its own: it prints what came off the disc,
including the values this mode has to make an assumption about, so the assumptions
can be checked against the numbers rather than taken on trust.

## How the front end hands over

The window, the GPU device and the surface are created once and both halves draw
through them; what changes at `Launch Game` is only which stage owns the frame. The
front end reaching that state is the whole of the trigger - `Frontend::is_finished`,
which the state machine sets on entering it - and the load happens on the frame
*after* the one that drew it, so the last front-end frame is on screen while the
track is read.

There is no menu between the picker and the grid, because there is no menu: the
original's `MainMenu_Definition.xml` is in the root XML's `LoadXML` list and none of
it is built. So `Launch Game` starts one race, on the track and in the ship the
command line names, which is [documented as a
divergence](../architecture/frontend-boot.md#where-we-knowingly-differ) rather than
presented as the original's flow. Nothing about a race is chosen from a menu yet.

A load that fails - a missing track, a model that will not decode - is reported and
the front end stays on screen, rather than the window vanishing.

## The window, and the shape it is given

The window asks for 1440x816, three times the PSP's own 480x272, and it asks as a
**fixed** size: that sets its minimum and maximum to the same value, which is the
signal a tiling compositor floats a window on instead of squeezing it into whatever
column its layout has. Measured under niri, which tiles the window to a 948x1152
portrait slot without it. It is an early-stages default, not a claim that a game
window should never resize.

The renderer does not rely on it. A field of view in a data file is only defined at
the shape it was authored for, and the original renders into 480x272 and nothing
else, so any other window shape needs a choice. The choice, in
`oag_render::camera::fit_vertical_fov`: at the PSP's aspect and anything wider the
authored value is used unchanged, and below it the vertical field opens up by
exactly enough to hold the horizontal field the authored shape would have had.
Without it a portrait window crops the sides away - the track edges leave the frame
and the ship fills a slot - which reads as a broken camera when the camera and its
data are both fine. It is a presentation choice and not a reading of the game, so it
carries no confidence score; what it is *not* allowed to do is change the picture at
the authored aspect, and a capture at 1440x816 is byte-identical across the change.

`--size` renders a capture at any shape, which is how that is checked without a
compositor:

```sh
just play --race --art --size 948x1152 --screenshot /tmp/tall.png
```

## What is drawn

The **driveable ribbon** by default, exactly as [`oag-view --track`](oag-view.md)
draws it: the track surface from the spline's own half-widths, the authored racing
line in yellow, the AI corridor edges in green. That is the geometry the simulation
spawns on, so a picture of ship-plus-ribbon shows directly whether the ship is where
the physics thinks it is. `--art` draws the track's art meshes instead, which looks
more like a game and proves less.

Both go through `oag-render`'s one mesh pipeline, the same one the asset viewer uses.
Drawing the art once is worth doing as a check in its own right: the ship sits on the
art surface in the same place it sits on the ribbon, so the three decodes that had
never been seen together - spline, collision and render geometry - are in one world
space.

## Where the numbers come from

Everything is read from the player's own disc; nothing in this mode is tuned, and no
physics constant is adjusted from here.

| Thing | Source |
| --- | --- |
| The spline, and the pose a ship starts in | the track's `WO Track` node, via [`track`](../formats/track.md) |
| What the hover probes hit | the track's [collision nodes](../formats/collision.md) |
| Every force-law parameter | the team's [`handlingstats.xml`](../formats/handling-stats.md) |
| The chase camera's seven values | that file's `<ExternalCameraFar>` |

The four pre-scaled handling fields are scaled exactly once, inside
`oag_gameplay::handling_for`; see the
[handling-stats page](../formats/handling-stats.md).

### Four readings this mode had to make

Each is a decision the data forced and none of them was recoverable from the
executable, so each is recorded here with a
[confidence score](../reverse-engineering/confidence-rubric.md).

**`fov` is degrees** (confidence 85). The unit is not recovered, and
`oag_render::camera::chase::ChaseParams::fov` deliberately refuses to guess, so the
conversion is written at the call site in `race.rs`. Every camera block on the disc
carries a value in the sixties or seventies, which is a normal field of view in
degrees and an impossible one in radians.

**`pos_length` is negated on the way in** (confidence 80). `oag-render` documents
`ChaseParams::pos_length` as a positive distance *behind* the ship and computes the
eye as `position - forward * pos_length`. The disc stores a signed offset *along*
forward instead: every external camera block observed has it negative, and the
`Close` camera - the one that sits nearer the ship - has the smaller magnitude, which
fits that reading and no other. Passing the file's value through unchanged puts the
eye in front of the ship looking further ahead, so the ship is behind the camera and
nothing of it is drawn. `chase.rs` calls that mistake "obvious on screen, so this one
is cheap to check once there is a ship to look at"; this mode is that check, and the
negation is its answer. It arguably belongs in `oag-render` rather than here.

**A ship starts at `ride_height` above the surface line** (confidence 70). Three
candidate heights exist and they are three different quantities:

- `track::HOVER_LIFT`, the three units the load pass lifts each *AI-line* control
  point by, which says nothing about ships. Starting there leaves the suspension
  compressed by the difference, and one frame of the spring at that compression
  throws the ship clear of the track. That was measured, not predicted.
- `hover::target_height`, the height the spring holds, which on the observed data
  comes out **greater than the probe raycast length** - so a ship starting at its own
  target height cannot see the ground and begins in free fall.
- `ride_height`, which is both the raycast length and the primary term of the target.
  A ship starting there is in contact on its first frame.

The third is what is used, and the second is a finding in its own right; see below.
Note that "in contact" means one of the two probes: they sit fore and aft along the
hull, so at exactly the cast length they straddle the limit and one drops out.

**A `.vex` ship is authored nose along `+Z`** (confidence 85), while
`oag_physics::Body::forward` is `-Z`. Drawn straight from the body's orientation the
ship therefore faces *backwards*, and since the chase camera sits behind the body -
which is the model's nose side - what you get is a head-on view of a ship that is
driving away from you. `race.rs` composes a half turn about the model's up axis,
`MODEL_YAW`, at the boundary between the body and the drawn model; `oag-view --mesh`
is untouched, because a viewer shows a model in its own space.

That is measured, not assumed. Sliced along `z`, the default team's hull is 5.3 units
across at the `-Z` end, carries the model's full height there and 1,175 of its 1,334
vertices, and tapers to 0.9 across at the `+Z` end. The wide, detailed,
full-height end of a racing ship is its engine block; the narrow tapering end is its
nose. It is 85 rather than higher because it is an inference from the shape of one
team's hull - nothing in the executable has been read that states the convention.

## What it actually does, and what it does not

Everything below was measured on the USA PSP disc with Feisar in the Venom class, on
`01_Track`, with thrust held from the first tick. It is reproducible with the command
under [Running it](#running-it) and asserted by
[`race_ground_truth.rs`](../../crates/game/tests/race_ground_truth.rs).

**What works.** A ship spawns on the racing line the right way up, its hover probes
find the track's collision geometry, and it accelerates forward - along its own
forward axis, not merely somewhere - down the spline. At two seconds it has travelled
tens of units, is doing tens of units per second, and has never been more than about
a third of the "off the track" envelope away from the spline; it was in contact with
the surface on roughly seven ticks in ten, though on **one** hover probe rather than
two for almost all of them. Every downward probe from a spread of two
hundred samples along the whole spline finds collision geometry within one probe
reach, so the spline decode and the collision decode agree about where the track is,
and `--art` shows the art meshes agreeing with both.

**What does not work: it cannot fly a lap.** At around 100 ticks a growing
oscillation about the hover target height costs the ship the probe contact it had
left; it pitches, is thrown clear of the surface, and by around tick 166 it is
outside the envelope. From there it leaves the track sideways and falls indefinitely
at a terminal 234 units per second, still finite after 3600 ticks. From other
starting heights the same instability instead produces a non-finite state within a
few hundred ticks, which is why the test accepts either.

Three measurements say this is the recovered force law and not the composition above
it, and none of the three is in the shipped code:

1. It reproduces with the same parameters on a **flat infinite floor** with no track,
   no spline, no collision decode and no camera, driven exactly the way
   `crates/physics/tests/ship_dynamics.rs` drives it.
2. At **a quarter of the timestep** the same parameters are stable.
3. At the full timestep with a **textbook box inertia tensor** built from `<Misc>`,
   the same parameters are stable.

A single probe in contact is an off-centre force, so it is a pitch torque applied
every tick, and over the measured two seconds that is the state the ship is in for
almost all of the ticks it is grounded at all. That looks like the trigger. The
amplifier is the one thing the ship data does not carry: an **inertia tensor**.
`Body::inertia` is `(1, 1, 1)` because nothing observed says how the original builds
one, while the hover probes apply their force about six units from the centre of
mass, so the torque they generate is divided by 1 rather than by a real moment.
Deriving a box tensor from the hull dimensions would make the demo look better and
would be an invented constant, which
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md)'s reasoning says is worse
than a visible gap.

The other suspect is that `hover::target_height` exceeds `<Antigrav ride_height>` on
this data, because of the additive `antigrav_height_adjust` term that
`oag_physics::hover` already records as a guess at confidence 50 with "nothing was
found that writes it". `ride_height` is *also* the probe's raycast length, so a target
above it means there is no height at which the spring is at rest and the probes are in
contact: the oscillation is structural under that reading rather than being a stiff
spring settling. Zeroing the offset makes target and reach equal. That is one line in
`oag_physics::hover` and is not this mode's to change.

Neither is a tuning problem and neither is fixed here. Both are for M3's
[trace comparison](../reverse-engineering/verification-protocol.md), which is the
first point at which anything about the force law can be checked against the original
rather than against itself.

Two other known gaps, both expected: an **inverted ship falls off**, because the
magstrip magnetic hold is not decoded and not implemented (see
[physics](../physics/README.md)), and there is **no sideshift**, because its button
binding was never recovered.

## Two things `oag-render` could grow

Neither blocks anything; both are places this mode reaches around a seam rather than
through it.

- **A view-projection writer.** `mesh_render::write_uniforms` computes an *orbit*
  camera from the model's bounding sphere, which is what an asset viewer wants and is
  not something a chase camera can use, so `race.rs` declares the shader's uniform
  block itself and writes the two matrices directly. A unit test asserts its size
  against `mesh_render::UNIFORMS_SIZE` so the duplicate cannot drift silently.
- **A from-bytes entry point for `track`.** `track::load` takes an archive
  specifier, so a caller that also wants the file's collision nodes - which every
  caller that wants to drive on it does - decompresses the same blob twice.

## Requirements

A GPU with a Vulkan, Metal or DX12 driver, as for [`oag-view`](oag-view.md). The
screenshot path needs a device but not a display. The simulation itself needs
neither, which is what lets the ground-truth test fly a ship in CI-shaped
conditions - though it is `#[ignore]`d there, because it needs a disc image.
