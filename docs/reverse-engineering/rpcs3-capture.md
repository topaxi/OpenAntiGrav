# Capturing a reference frame and its camera from RPCS3

[rpcs3-debugger.md](rpcs3-debugger.md) establishes that RPCS3 can be driven
into a race with no window and read at the Ghidra corpus's own addresses. This
page is what that is *for*: pairing a frame the original renders with the
camera it rendered it from, so this project's own frame can be laid over it
instead of eyeballed at approximately the same place.

    python3 scripts/rpcs3-drive.py display
    uv run --with evdev python3 scripts/rpcs3-drive.py capture \
        --shots 4 --interval 6 --out data/reference/hd-capture/anulpha

Each shot writes `NN.png` and `NN.json`. The JSON carries the circuit, the
camera, and the `oag-game` command line that renders this project from it.
**Nothing else**: the raw memory is game data, so it stays under `data/` behind
`--keep-dumps` and out of everything tracked - see
[legal.md](../overview/legal.md).

## Reaching a circuit other than the one every default row leads to

The walk into a race presses cross and takes the highlighted row every time,
which lands on **Campaign**, event 01/08, Talon's Junction - and on a fresh
save every other cell in that grid is padlocked, so Campaign cannot reach a
chosen circuit at all.

**Racebox can.** It is the Main Menu's second tab, so one `right`, and its
`Track Creation` screen is a plain `TRACK SELECT` with a circuit carousel:

    --nav "Main Menu=right" --nav "Track Creation=right"

`--nav SCREEN=BUTTONS` presses those buttons on arriving at that screen and
before the next cross. It is repeatable, and `--nav-shots` photographs every
distinct screen on the way in so the next plan can be written from what is
actually on them - nothing in this tree describes HD's menus, and `TTY.log`
names a screen without saying what is on it.

**A screen must settle before it is photographed.** `TTY.log` names the new
screen at the *start* of its transition, so a shot taken on the name change
catches the animation: the first pass of this photographed two menus mid-flight
and both came back as unreadable red smear. `Session.SCREEN_SETTLE` is the
three seconds that fixed it.

## What a frame's camera actually is

Wipeout HD's shaders take the camera as two named engine parameters:

| Name | `~crc32` | Type | What |
| --- | --- | --- | --- |
| `viewProj` | `0x2e7d5f33` | `float4 x4` | the world -> clip matrix |
| `eyePositionWorldSpace` | `0x3466fc0e` | `float3` | the camera's position |

`viewProj` is the one worth capturing, because it is the one that decides the
frame: position, orientation, field of view, aspect and the near and far planes
are all inside it. Capturing position and angles separately invites a
disagreement about handedness or about how a field of view is derived to be
mistaken for a rendering difference.

### The names are recovered, not guessed

`EBOOT.elf` carries the engine's whole shader vocabulary as a `const char *`
table at **`0x008b7f08`**, 107 entries, one of them empty. Every parameter and
feature-token hash this project had been brute-forcing a wordlist for is in it:
`viewProj`, `view`, `world`, `eyePositionWorldSpace`, `positionBias`,
`positionScale`, `fogColour`, `prelitBias`, `prelitScaleSpecular`,
`directionalLight0DirectionWorldSpace`, `directionalLight0Colour`,
`shadowMatrix`, `paraboloidReflectionTex`, the `zone*` family, and so on.
Confidence 98: each name hashes to a word the shader tables already carry, and
the three that were already known independently - `lightmap`,
`constantAmbientColour`, `Uv1` - land on their own hashes.

**Nothing in the executable points at that table.** No `lis`/`addi` pair builds
its address and the word `0x008b7f08` never appears as a pointer anywhere in the
image, so the runtime location of the *values* is not reachable by a static
cross-reference. That is why the capture hunts for the matrix rather than
reading it from a known global.

## The hunt, and the test that makes a candidate trustworthy

`scripts/ps3_pose.py` scans a dump for sixteen consecutive big-endian floats
that read as a perspective world -> clip matrix. Sixteen arbitrary floats are
not a projection: in the row-vector convention (`clip = v * M`) the clip `w` of
a world point is `x*m03 + y*m13 + z*m23 + m33`, which for a perspective is the
signed distance from the camera plane - so **`(m03, m13, m23)` is a unit
vector**. That is six digits of agreement random data does not produce.

The camera position follows from the same matrix: the frustum's side planes are
the Gribb-Hartmann combinations of its columns, and three of them meet at the
eye. `--self-test` plants a known camera in 256 KB of random bytes and insists
the finder recovers its eye to within 0.05 units; it does, with one false
positive in that much noise, and the false positive has no plausible eye.

Both the matrix and its transpose are tried, and which one matched is recorded
rather than assumed.

## Closing the loop

`ps3_pose.decompose` turns the matrix into the numbers a renderer is set up
from - eye, forward, up, right, vertical field of view and aspect - by
inspection: `c0.xyz` is `right * f/aspect`, `c1.xyz` is `up * f` and `c3.xyz`
is the unit forward. `oag-game --camera-pose` takes the first nine of those and
`--camera-fov` the tenth, so a captured frame and ours can be rendered from one
camera.

## `viewProj` is not in the executable's data, and that is measured

**The first live capture answered this, and it is the thing to know before
spending a boot.** A dump of the whole `0x00860000`+`0xe0000` span - the
EBOOT's initialised data and the BSS behind it - holds **seven** matrices that
pass the perspective test, and every one of them is a static constant: unit
error exactly `0.0`, an axis-aligned basis, an eye within 3.3 units of the
origin. Those are authored cube-face and shadow matrices. The live camera is
in a heap allocation.

So a capture reaches it by pointer chain, and `--region` takes one, with `@` as
a suffix meaning "read the pointer here and carry on from there":

| Chain | What |
| --- | --- |
| `0x860000` | a literal address |
| `r2` | the PPU's TOC pointer, out of the register dump |
| `0x936fd4@` | the pointer stored at `Render_FrameContextPtr` |
| `r2+0x6828@+0x14@` | the `RenderManager` instance |

The last one is free from Ghidra: `RenderManager_CreateInstance`
(`0x002d6190`) is three statements, `instance = alloc(0x4d90)` followed by
`*(*(r2 + 0x6828) + 0x14) = instance`. Reading its 19,856 bytes is **34
packets** against the data segment's 1,556, which is the difference between a
second and a minute.

**A degenerate matrix passes every algebraic test**, which the same first run
also proved by reporting a region of zeroed memory as the camera - eye at the
origin, field of view of nothing. `score` now also requires the basis the
matrix decomposes to be orthonormal (six constraints), the field of view to be
between 10 and 170 degrees, the aspect to be between 0.5 and 4, and the eye not
to be the origin.

### Where the live camera is not

Four places have been dumped from a running race and searched, and the point of
recording it is that each is a boot someone else does not have to spend:

| Where | Chain | What is there |
| --- | --- | --- |
| The EBOOT's data and BSS | `0x860000:0xe0000` | seven matrices, every one a static constant |
| The `RenderManager` | `0x008b3d00@+0x14@:0x4d90` | one, the same 90-degree 1:1 cube face |
| `Render_FrameContextPtr`'s target | `0x936fd4@:0x8000` | eight, all the same cube face |
| 1 MB of heap at `0x30000000` | `0x30000000:0x100000` | **0.3 %** of it changes between two frames five seconds apart, and exactly one changed window parses as a camera - another 90-degree 1:1 |
| libgcm's own data area | `0x008c0854@@:0x20000` | 128 KB with **0.00 %** churn across three shots twelve seconds apart, no matrix and no constant packet |
| The main GCM ring | `0x008c0854@@@:0x8000` | device bring-up commands, static; no `0x1efc` method anywhere in it |

Relaxing the unit-`w` test - in case the projection scales that row - adds
nothing convincing to any of them. The last two rows are the boots this page's
next section spent before it found the right pointer level.

**So the working hypothesis was that no CPU-side copy of `viewProj` is kept**:
the vertex program is fed `c[0..3]` and the engine computes the matrix and
pushes it straight into the command buffer, in which case the only place it
exists is the RSX pushbuffer - where it is *exactly* findable rather than
heuristically, as the operand of a `NV4097_SET_TRANSFORM_CONSTANT_LOAD`.

## `viewProj` is in the pushbuffer, and here is where (2026-09-05)

**Confirmed, confidence 93.** Three boots. What follows is the whole route,
because two of the three were spent on a wrong pointer level and a wrong idea
of what a matrix looks like in a FIFO, and neither is worth repeating.

### The context is two dereferences deep, not one

`g_GcmContext` (`0x008c0854`) is not the address of the `CellGcmContextData`.
It holds `0x013be314`, which is libgcm's own `CellGcmContextData *`, which
holds `0x016a0ccc`, which is the struct:

    0x013be314:  016a0ccc 00000000 00000000 00000000    <- one pointer, then nothing
    0x016a0ccc:  40001000 40007ffc 400013a4 016a0504
                 begin     end      current  callback

Read one level short - which is what `--region "0x008c0854@:0x20"` does - the
context looks like a struct whose `end`, `current` and `callback` are all
**zero**, so every chain built on `+0x4` or `+0x8` resolves to null and is
skipped, and the boot produces nothing at all. At the second level
`begin < current < end` holds and `callback` is a plausible descriptor.

    --region "0x008c0854@@:0x20"        # the context struct
    --region "0x008c0854@@@:0x8000"     # the ring, from `begin`
    --region "0x008c0854@@+0x8@-N:N"    # behind the write head

### That ring is the bring-up buffer; the frame is three JUMPs away

`0x40001000..0x40007ffc`, and it does not change by one byte across three
shots six seconds apart in a live race - `current` is `0x400013a4` in two
separate boots and never advances. It ends in `20010000`, an RSX JUMP
(`0x20000000 | io_offset`) to `0x40010000`; the four 4 KiB auxiliary contexts
there each end in a JUMP of their own, to IO `0x74100` and `0x75100`
alternating. The draw commands are downstream of *those*: **220-394 constant
loads a frame over IO `0x77000..0x9f000`.** Below that, what has actually been
read is the four auxiliary segments (`0x10000..0x14000`) and `0x60000..0x77000`,
and there are none in either; `0x14000..0x60000` has never been dumped and is
not claimed either way. See
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md).

### A matrix in a FIFO is not sixteen consecutive floats - except when it is

This is the trap that makes a null result meaningless.
`Rsx_UploadVertexConstants` (`0x005c176c`) emits **one packet per `vec4`**:
header `0x00141efc`, an index, four floats. Under that emitter a `float4x4` is
four rows **24 bytes apart** with two non-float words before each, and
`ps3_pose.candidates`, which slides over raw bytes looking for sixteen
consecutive floats, cannot see it at all.

`Rsx_UploadVertexConstantBlock` (`0x005c18d8`) is the other emitter, and it
*does* write the sixteen floats consecutively - header `0x00441efc`, an index,
then the matrix. So during a race both forms are in the stream:

| Header | count | Payload | Registers loaded |
| --- | --- | --- | --- |
| `0x00141efc` | 5 | index + one `vec4` | `c[462..467]`, the material's own block |
| `0x00441efc` | **17** | index + **a whole `float4x4`** | `c[256]` and `c[260]` |

Under `renderer.md`'s `N + 256` rule those two are the shader's `c[0..3]` and
`c[4..7]` - and `c[0..3]` is exactly where the hypothesis said the vertex
program is fed `viewProj`.

### Picking the camera out: multiplicity, not a score

`c[256]` takes **89, 104 and 106 distinct values** in the three frames, so it
is generally a per-object `worldViewProj`. That is the discriminator, and it
is a stronger one than any algebraic test:

| distinct values in one frame | changes between frames | what it is |
| --- | --- | --- |
| many | - | a per-object world matrix |
| one | no | a static constant (a cube face, a bias) |
| one | **yes** | **the camera** |

Exactly one value per frame passes `ps3_pose.score`, and the same matrix is
also written to `c[260]`, eight or nine times a frame:

| Shot | Address | eye | `fov_y` | aspect | unit error |
| --- | --- | --- | --- | --- | --- |
| 00 | `0x40077308` | -143.58, -48.44, -175.13 | 60.0001 | 1.777778 | 7.1e-08 |
| 01 | `0x40077308` | 88.88, -46.39, -178.36 | 68.9974 | 1.777778 | 1.9e-08 |
| 02 | `0x400779fc` | 502.72, -24.36, -61.46 | 64.8137 | 1.777790 | 3.1e-05 |

**Which tests it passed, said out loud**, because this page's own warning is
that a degenerate matrix passes every algebraic one:

1. **No denormal components at all** - the false positive in
   `data/reference/hd-capture/talons/00.json` is ~1e-38 noise with a lone
   `1.0` in the `w` slot, and this has none.
2. **`aspect` is 1.777778**, 16:9 to six digits, which is HD's own 1280x720.
3. **Unit error 7e-08**, against a tolerance of 3e-2 - seven digits, not one.
4. **An orthonormal basis**, with `up` within six degrees of world up: a race
   camera is nearly level and this one banks slightly.
5. **`fov_y` is 60.0001 and then moves** - an authored base value, widening
   and narrowing frame to frame, which is the speed-dependent field of view
   the series has always had.
6. **The eye moves coherently**: 232 units over the first six-second interval
   and 430 over the second, accelerating, with `cross` held down throughout,
   and the height staying between -24 and -48. By shot 02 `forward` has swung
   from `+X` to `(0.80, -0.06, 0.59)`: the ship is in a bend.
7. **It is a command operand**, not an offset that happens to parse, and it is
   re-uploaded eight or nine times a frame.
8. **It goes to both `c[256]` and `c[260]`** - `c[0..3]` and `c[4..7]` - with
   the same sixteen floats. If the engine's convention is `viewProj` in the
   first and `worldViewProj` in the second, those coincide *exactly* on a draw
   whose world matrix is the identity, and a matrix landing in both is evidence
   that it is the camera rather than some object's transform.

**What this does not settle** is which of those two it is. A *translated*
object's `worldViewProj` decomposes to an identical basis, field of view and
aspect, and an eye offset by a constant - the eye is the camera in that draw's
own space and equals the world-space eye only where the world matrix is the
identity. The two are indistinguishable in these numbers, and the way they
differ is the one that misreads as a translation bug rather than a wrong
matrix. Rendering from the pose and overlaying the captured frame settles it.

The four earlier eliminations are explained rather than contradicted: there
need be no CPU-side copy, because the matrix reaches the RSX as a FIFO
operand and nothing else has to hold it.

### Reproducing it

    uv run --with evdev python3 scripts/rpcs3-drive.py capture \
        --shots 3 --interval 6 --keep-dumps \
        --out data/reference/hd-capture/talons-fifo3 \
        --region "0x40010000:0x4000" \
        --region "0x40060000:0x20000" \
        --region "0x40080000:0x20000"

**`capture`'s own finder reports a different and wrong camera for these
dumps** - an eye on the X axis at 11.0 and 34.4 units, `fov` 39 - because
`ps3_pose.candidates` knows nothing about packets. Wiring the packet-aware
finder into the harness is what closes the loop; until it is, read the dumps
rather than the JSON.

## What is free, and what costs packets

The circuit costs nothing: HD prints `Loading track model Data\Environments\...`
to `TTY.log` on every load, and the capture reads it there. Everything else is
`m` packets at about 41 ms each, so the defaults read one span - the EBOOT's
initialised data and the BSS behind it, `0x00860000` for `0xe0000` bytes, about
a minute. `--region @addr:len` dereferences a pointer first, which is how a
render context reached through a global costs kilobytes instead of a megabyte.

The frame is trimmed to the emulator's own rectangle before it is written:
the root window is the *display's* size and RPCS3 presents into a rectangle
somewhere inside it, so an untrimmed capture compares a desktop against a
render. The crop is on exactly `#000000` and nothing near it, so a dark scene
keeps its own black.

**The stop comes before the screenshot, deliberately.** `screenshot()` grabs the
virtual root window rather than asking RPCS3 for a frame, so it is unaffected by
the target being stopped - which means the memory read describes the frame that
is on screen rather than one several frames later. Getting that order wrong is a
silent skew in every comparison made afterwards.

**One debugger session per emulator launch**, so the capture loops many poses
inside a single connection rather than booting per pose; the reasons are in
[rpcs3-debugger.md](rpcs3-debugger.md)'s trap list and they are all
unrecoverable.

## A root-window grab is not the framebuffer, and only one of them can measure geometry

**Confidence 92**, measured 2026-09-02 while settling the main menu's tab
corner.

`scripts/rpcs3-drive.py`'s `screenshot()` grabs the X root window, which is what
every capture in `data/reference/` up to now used. What that returns is
whatever the emulator *presented into its own window*, and on a bare Xvfb with
no window manager that window is **1280x720 and stays there** - `Start games in
fullscreen mode` has nothing to fullscreen against, so neither the display's own
geometry nor `Resolution Scale` changes the picture. A 1600x1200 display and a
2560x1440 one both give the same 1278x718 after the black trim. That is enough
to read a colour and not enough to measure a widget corner, and it is why the
strip-tab chamfer went two rounds of eyeballing before it was measured.

**RPCS3's own frame grab is the one to use for geometry.** It goes through the
home menu (`Take Screenshot`, row four) and writes the *framebuffer* to
`~/.config/rpcs3/screenshots/<TITLE_ID>/`, and unlike the root-window grab it
**does honour `Resolution Scale`**: at `200` against a configured `1920x1080` it
writes **3840x2160**, three times the linear resolution of what the window
shows. `scripts/rpcs3-drive.py shot --screen "Main Menu"` drives it end to end.

```sh
# in ~/.config/rpcs3/config.yml, under Video:
#   Resolution Scale: 200
OAG_RPCS3_GEOMETRY=2560x1440x24 python3 scripts/rpcs3-drive.py display
uv run --with evdev python3 scripts/rpcs3-drive.py shot --screen "Main Menu"
```

**Put `Resolution Scale` back to `100` afterwards.** It is a global setting, and
leaving it up silently changes every later capture's resolution and costs frame
rate on a driven run.

Two things this does not buy. The scaled framebuffer renders the *same geometry*
at more samples, so it resolves a shape the game rasterises and says nothing new
about one the game samples out of a texture - which is itself the useful
distinction, since a magnified texture mask shows uniform runs where rasterised
geometry does not. And HD's own output is 1280x720 whatever the scale, so a
scale-100 capture already *is* native; the scaling buys resolution above native,
not a fix to a downscale that was never happening.

## See also

- [rpcs3-debugger.md](rpcs3-debugger.md) - the stub, and the traps around it.
- [rcsmaterial.md](../formats/rcsmaterial.md) - what the shader tables hold.
- [methodology.md](methodology.md) - observe, hypothesise, verify, document.
