# `oag-game`

The composition root, and the only binary that is the game rather than a tool for
looking at it. It has two halves, and one leads into the other.

- **The front end**, which is where it boots: `LogoFMV` playing the disc's own
  40-second intro, then the Language
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

WASD steers (arrow keys also work), X or Return thrusts, Q and E are the left
and right airbrakes, C and V are square and triangle, Escape quits. The keys
are the same
[abstract button layer](../ghidra/functions/psp-pulse/input.md) the front end uses;
there is one mapping from a keyboard to a snapshot and both halves go through it,
one set of devices for the whole session, so a key held across the handoff stays
held.

**A gamepad works too**, and needs no configuration: left stick or d-pad steers,
R2 or A thrusts, the shoulder buttons are the airbrakes and L2 is both of them.
The whole table, and the two bindings that are interpretations rather than
one-to-one, are in [packaging](packaging.md#gamepad). Keyboard and pad merge into
one button state before any edge is computed, so a press on either is one press.

## Finding the disc image

The positional argument is optional. Left out, the image is searched for:
`data/images/` under the current directory first, which is what a checkout has,
then beside the executable if it is an AppImage, then
`~/.local/share/oag/images/`; `$OAG_IMAGE` short-circuits the lot. The full order
is in [packaging](packaging.md#where-the-disc-image-comes-from), and
`crates/game/src/source.rs` is the code. **No image ships with the engine** and
none ever will - see [legal](../overview/legal.md).

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
| `--movie <entry>` | Which movie to play: a `Data.wad` entry name, or `hash:XXXXXXXX` for one of the reels whose name is not recovered. Defaults to `Data\Movies\Intro.PMF`, which is what the `LogoFMV` screen plays and the only movie the disc's own boot ever opens - see [frontend boot](../architecture/frontend-boot.md#what-the-disc-actually-does-at-boot). |
| `--reel` | Boot `Intro Screen->IntroMovie1` instead of `LogoFMV`: the code-side state with the frame-counted holds at 144, 231 and 260, playing the 260-frame dev/pub reel those counters describe. **Not the boot order** - the disc never enters that state at boot, and the reels carry no Pulse branding. See [the dev/pub reel](../architecture/frontend-boot.md#the-devpub-reel). Implies `--movie hash:b1ba72c3` unless `--movie` is given. |
| `--movie-frames <n>` | Convert only the first `n` frames. All of them by default, which for the 1200-frame intro is 33 MiB of cache and about 80 seconds of `ffmpeg`, once. |
| `--screen <name>` | With `--screenshot`, draw one named screen straight out of the front-end XML and stop, instead of running the sequence. A debugging view of the screens the boot order does not reach - `--screen "Show Logo"` is the Pulse logo and "Press START button". Names are the XML's own; a `Parent->Child` path works too. |
| `--race` | Go straight to the race. Checked before the front end loads anything, so it never parses the menus or transcodes the intro. |
| `--track <name>` | The track's `.vex` entry in `Data.wad`. |
| `--team <name>` | Which team's `handlingstats.xml` and model to fly. |
| `--class <name>` | `venom`, `flash`, `rapier` or `phantom`. |
| `--art` | Draw the track's art meshes instead of its driveable ribbon. |
| `--collision` | Overlay the collision soup - the geometry the physics world is actually made of, the same view [`oag-view --collision`](oag-view.md#collision) draws - on top of whichever track model was chosen above. |
| `--log-every <n>` | Print a telemetry line every `n` ticks. `0` for none. |
| `--size <WxH>` | With `--screenshot`, the image's size. Default `1440x816`. |
| `--dry-run` | Report what was loaded and exit. |

All of them apply to a race started from the front end as well, since it is the same
race. `--class` in particular is parsed before anything is loaded, so a misspelling
is not discovered forty seconds of intro later.

## Either disc

The first positional argument is any Pulse source, and that now includes the PS2
release:

```sh
just play data/images/pulse-ps2-eu.chd --race
just play data/images/pulse-ps2-eu.chd --race --screenshot /tmp/ps2.png --ticks 60 --hold cross
```

Nothing else about the command changes, and that is the finding rather than a
convenience: **the archive layout was the whole of the difference.** The entry
names are not per-platform - the PS2 disc carries
`Data\Ships\<Team>\handlingstats.xml` and `Data\Environments\<n>_Track\track.vex`
under the names the PSP uses - the [handling schema is the same one](../formats/handling-stats.md#the-ps2-release-ships-the-same-schema),
and the mesh and collision decoders already read both. So `--track` and `--team`
mean the same thing on either source, and no flag selects a platform.

What the game used to do instead was spell the PSP's layout out:
`PSP_GAME/USRDIR/Data.wad`, hardcoded, so the PS2 disc failed at the first read.
`oag_assets::pulse::Layout` replaced that. It looks at the source's own file list
and takes the first archive it recognises, `Data.wad` or `WADS2.WAD`, plus the
companion beside it, `FE.wad` or `WADSP.WAD`. **Found by name, not derived from
the platform**, which matters because the PS2 archives sit in a directory named
after the disc's serial - `54748/` on SCES-54748 - so a path constant would be
right for one pressing and wrong for the next. The reported platform comes from
[`oag-disc`'s identification](../ps2/pulse-disc-layout.md) and is used for the
report line and the error text, never to choose a decoder.

Two things are genuinely worse on the PS2 source, and both say so rather than
being papered over.

**Models draw untextured.** Measured on both discs: `Assegai\Ship.vex` fills
**8 of 8** texture slots on the PSP and **0 of 5** on the PS2, and
`16_Track\track.vex` 135 of 135 against 0 of 140. The slots exist on both - a PS2
`.vex` still declares one `Texture` node per texture - but the pixels are not in
the file. They are separate archive entries gathered into a nested WAD of Graphics
Synthesizer upload packets, and which entry belongs to which model is not
recovered. The decoder exists - `oag-view --mesh ... --textures <entry>` skins one
by hand, see [ps2-texture](../formats/ps2-texture.md) - so what is missing is the
lookup. The load report names each model whose slots did not all fill, giving the
two counts, and nothing guesses at a set to avoid printing that line.

The report is keyed on **unfilled slots, not on an absent texture list**, and the
distinction matters: the driveable ribbon is generated geometry that declares no
slots at all on either disc, so reporting it would blame a PS2 texture gap for a
mesh that was never textured and never came off a `.vex`.

**The boot movie has no picture.** The PS2 ships no `.PMF` in any archive at all;
its intro is `DATA/MOVIES/INTRO512.PSS`, an MPEG-2 program stream loose in the ISO
filesystem, with `.IPF` beside it for the Image Processing Unit. Neither is
decoded and neither is addressable as an archive entry, so there is nothing for
the movie leg to play and the front end runs without a picture, the same path
`--no-video` already took. A movie **named** with `--movie` and not found is still
an error: the defaults are defaults, and a source that lacks one is answering a
default, where `--movie` is a request.

The rest of the PS2 front end gets **further than expected**, and this was measured
rather than assumed: `WADS2.WAD` carries `Data\Plugins\PI001\GUI\Skin.xml` under
the PSP's own name, and it boots to 11 screens, 56 globals, five language plugins
and a 1,724-string English table. Two things there do not resolve, both
degrading to a report line: the `.fnt` font entry exists but is zero bytes, so the
menu draws in the built-in 5x7, and the front-end images are under neither
`Data\FE\Images\pulse_logo.mip` nor `gameshare_backdrop.mip`, so the sprite sheet
is empty.

One finding worth recording separately, because it is about the *original* rather
than about this build: the PS2 screens name their movies `Intro.pss` and
`Backdrop.ipf`, so the `Movie` widget's PSP rule of `src` **plus `.PMF`** produces
`Data\Movies\Intro.pss.PMF`. The `src` already carries its extension on this
release, so that name-building rule is PSP-specific. Nothing acts on it yet -
neither container is decoded - but it is the kind of assumption that would
otherwise be discovered much later.

Expect a race to load more slowly off the PS2 disc. 5,861 of `WADS2.WAD`'s 7,200
entries are [LZSS](../formats/lzss.md) where no PSP archive compresses anything,
and the track blob is still decompressed twice (see [below](#two-things-oag-render-could-grow)).

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

Everything below was measured on the USA PSP disc with **Assegai in the Venom class
on `16_Track`** - the reference scenario every PPSSPP capture uses, which
`race::DEFAULT_TRACK`/`DEFAULT_TEAM` now default to exactly - with thrust held from
the first tick. It is reproducible with the command under [Running it](#running-it)
and asserted by
[`race_ground_truth.rs`](../../crates/game/tests/race_ground_truth.rs).

**What works.** A ship spawns on the racing line the right way up, its hover probes
find the track's collision geometry, and it accelerates forward - along its own
forward axis, not merely somewhere - down the spline. Over two seconds it is in
contact on **120 of 120 ticks with both probes on every one of them**, holding a hover
height between 3.8 and 4.4 against a 4.125 target, and never more than 4.3 units from
the spline against a 114-unit envelope. Every downward probe from a spread of two
hundred samples along the whole spline finds collision geometry within one probe
reach, so the spline decode and the collision decode agree about where the track is,
and `--art` shows the art meshes agreeing with both.

### Walls now push back

A ship no longer passes through track walls. `oag_physics::wall` runs at the end of
every `step`, after the integrator, and pushes the hull back out of anything it
ended up inside, removing the inward velocity with restitution.

Three parts of it are read off the original, and the rest is not:

- **The hull is a box**, and its extents are the ship's own `<Misc width height
  length/>`. No dimension here is invented.
- **Restitution is `0.05` for `Wall`.** The file's `-1.0` for floors is a
  **sentinel meaning "never bounce"**, not a coefficient - implementing the
  literal would add energy on every contact - so it is carried as `Option<f32>`
  and combined by a rule that forces zero when either side is a sentinel. See
  [collision.md](../formats/collision.md#surface-types-and-the-restitution-sentinel).
- **Contact generation is box-against-triangle**, as
  [the RE page](../ghidra/functions/psp-pulse/collision.md#raycasts) records.

**The response law itself is an implementation choice, not a reading.** The
original's contact *generation* is documented - a per-triangle separating-axis
reject then ten box sample points, into 0x40-byte slots - but what consumes those
contacts is not, so this is an ordinary projection: push out by the penetration
depth, then remove the inward velocity and give back `restitution` of it. M3's
trace comparison is what will replace it. It runs as a projection after
integration rather than as a force term deliberately: a contact spring stiff
enough to stop a ship inside one frame is exactly the stiffness an integrator that
evaluates force once and holds it across three sub-steps cannot carry.

Four consequences worth knowing before reading the picture:

- **Only walls respond.** `Floor` and `MagFloor` are skipped because the hover
  spring owns them, and a lateral probe firing on a floor would shove a
  hard-banked ship sideways off a surface it is meant to be resting on. `Reset` is
  skipped one layer down by the raycaster. So in a real race the contact
  restitution is always `0.05`.
- **A swept query catches tunnelling.** Walls are zero-thickness single-sided
  shells, so a ship that crosses one within a frame is past every hull probe. One
  ray along the frame's displacement covers that case.
- **No angular response.** Contacts apply at the centre of mass, so a glancing hit
  slows a ship without yawing it. The original's contacts carry a point and would
  produce torque.
- **One contact per frame**, the deepest. The original keeps 128 slots, so a ship
  wedged into a corner resolves one wall per frame rather than both.

Pinned by unit tests in `oag_physics::wall`, by two end-to-end tests in
[`ship_dynamics.rs`](../../crates/physics/tests/ship_dynamics.rs) - one that a ship
flown at a wall stays on the near side and turns round, one that a flight over open
floor is bit-identical with and without a distant wall - and, against real track
geometry, by
[`wall_collision_ground_truth.rs`](../../crates/game/tests/wall_collision_ground_truth.rs).
That last one is the only test that sees a shipped wall's own winding, scale and
triangle density; the synthetic ones author all three. **It passes against the
real disc**: the hull measures 5.5 x 3.5 x 13.0, read from the **ship's** own
`<Misc>` (Assegai in the Venom class - nothing about the hull comes from the
track), and a ship fired at a real wall triangle on `16_Track` stops 6.5 units
past the plane. That is exactly half the hull length, which is where a box that
size comes to rest, and it turns round.

Collision *response* beyond this - damage and the shield pool - is not
implemented.

### Reset zones respawn the ship

`Reset Collision` geometry is a trigger, not a surface: touching it puts the ship
back on the track.
[collision.md](../ghidra/functions/psp-pulse/collision.md#surface-types-in-practice)
records that at confidence **86**, and that is the whole of what is recovered -
**where** the ship respawns to is not recorded anywhere.

So the two halves are split by how well evidenced they are:

- **Detection** (`oag_physics::reset`) follows the evidence. It reuses the hull
  probes and the swept ray from the wall constraint, and requires no penetration
  depth, because a trigger volume is touched rather than pushed against.
- **Recovery** (`race.rs`) is a **guess, scored 40**. The ship goes back to the
  racing line at spawn height on the spline sample nearest where it was *before*
  the tick that triggered - the same code path the race start uses. The likelier
  real mechanism is a last-passed checkpoint or track section; `Ship::segment` is
  the field that would hold one and nothing populates it meaningfully yet.
  Velocity, orientation and every control state go to zero, because
  `Ship::place_at` resets the whole physics state. Whether the original preserves
  any speed through a respawn is also unrecorded.

**Reset detection deliberately does not go through the `Raycaster` trait.** That
returns the nearest hit across every collider, and `include_reset` only makes
`Reset` a *candidate* - so a reset volume below the track is always masked by the
floor in front of it and the trigger would essentially never fire. `reset::contact`
walks the collider list itself and queries only the `Reset` ones.

Two guards, because a respawn loop is indistinguishable from a hang: a 30-tick
cooldown, and a give-up after five consecutive respawns that prints a complaint
instead of freezing the race.

**The default track has no reset zones.** Measured on the USA PSP disc,
`16_Track` carries 66 wall, 123 floor, 7 mag-floor and **zero** reset colliders.
Of the tracks checked, `01`, `03`, `04`, `05`, `07`, `09`, `10` and `13` have
them and `02`, `06`, `14` and `16` do not, which matches
[collision.md](../formats/collision.md)'s census of 26 reset nodes across 40
tracks. To see one in `just play`, pass `--track` for one of the tracks that has
them; `oag-view --collision` prints the per-class census for any track.

**Confirmed against real disc data**, not only synthetically: flown into a real
reset triangle on `03_Track`, the ship respawns 4.0 units from the nearest spline
sample. See
[`reset_zone_ground_truth.rs`](../../crates/game/tests/reset_zone_ground_truth.rs).

### The suspension was never the problem: the default track was

This section previously recorded a growing oscillation about the hover target that
cost the ship its probe contact at about 100 ticks and threw it clear by 166, with a
missing inertia tensor as the amplifier and single-probe contact as the trigger - the
ship was grounded on 83 of 120 ticks and on *both* probes on only 6. **That was a ship
being flown over the wrong track's collision geometry.**

The scenario is Talon's Junction White, and the directory holding it was assumed to be
`01_Track`. It is `16_Track`, which is the first `<PI_Track>` in
`Data\Plugins\PI001\Definition.xml` at `soundregister="1"`, where `01_Track` is
`soundregister="18"`. The decisive evidence is geometric rather than nominal: the 200
recorded positions of a real capture of this scenario were cast against every track on
the disc, and `16_Track` is the only one that finds geometry under **200 of 200** of
them - at a mean height of **4.002**, against the `4.125 - 0.147 = 3.978` this
project's own hover spring independently predicts for a ship at rest. Every other
track and every sign convention misses outright or lands tens of units away.

Three things follow, and the third matters most:

- The suspension numbers above are what a working air cushion looks like, and both
  `TARGET_GLOBAL_SCALE = 0.75` and the `<Misc>` box inertia are corroborated rather
  than merely plausible - the measured 4.002 is the height they predict.
- The trace comparison's headline finding, that `grounded` read `0.0` from the very
  first tick while the recording read `1.0` on all 200, was **track selection, not the
  force law**. With `16_Track` the two agree exactly for the first 29 ticks.
- A wrong *name* produced a symptom that read as a wrong *force law*, and three
  separate physics explanations were built on top of it. The check that broke it -
  cast the recording's own positions at every candidate and see which one the ship is
  actually standing on - needs no disassembly and takes a minute.

### The ship stays on the track now

**It used to be thrown off, and that limitation is retired.** A ship at full throttle
left the 114-unit envelope at about tick **257** - not by falling off the surface but
by carrying far more speed into a corner than the corner would take. What fixed it is
the wall contact response in `oag_physics::wall`, matched to the `3.67 %`-per-frame
speed loss measured off a real capture.

Measured over ten seconds of the reference scenario, and
[asserted](../../crates/game/tests/race_ground_truth.rs) rather than remembered:

| | |
| --- | --- |
| Worst distance from the spline | **27.2** of a 114.0 envelope, at tick 520 |
| Grounded | **600 of 600** ticks |
| Respawns | **0** - it stays on by driving, not by being put back |
| Non-finite | never |

The test that recorded the old failure said to delete itself once the physics improved.
It has been replaced rather than deleted, by the same measurement asserted the other
way round, so a regression that threw the ship off again would be caught.

### What still does not match: the speed, and the trace comparison

**The speed is not the original's.** This run peaks at about **122 units/s**; captures
of the same scenario hold **23.6 to 25.1**. What that difference means took two wrong
diagnoses to establish and is worth not re-deriving:

> **Superseded (2026-07-28).** This section used to read "there is no speed
> equilibrium", with a factor of ~17 attributed to the thrust side. The
> standing-start capture inverted it: the force law is correct end to end (rms
> `0.127` over a 47-unit launch), and the "equilibrium" the old captures held was
> the **wall** setting their speed - both were recorded in sustained wall contact
> for their entire length, losing `3.67 %` of their velocity per frame in a
> post-integrate contact pass no force accumulator can see. There is no speed
> equilibrium on a clean straight, and there should not be one. See
> [force-balance-ground-truth.md](../physics/force-balance-ground-truth.md).

So a peak of 122 on a mostly wall-free run is not by itself a defect, and **no
constant is tuned to bring it down**: every constant on the thrust side is confirmed at
instruction level and the drag coefficients are confirmed in both binaries. What is
genuinely open is on the contact side, and it is where a trace comparison still
diverges:

- **Contact generation.** No capture in `data/traces/` has more than 60 consecutive
  wall-free ticks, so there is no clean reference straight to compare a wall-free run
  against yet.
- **The angular response.** `crates/physics` resolves contacts at the centre of mass,
  so the original's `cross(r, impulse)` torque and its contribution to the resolver's
  denominator are both absent. That is the largest remaining gap between this crate's
  contact response and the original's.

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
