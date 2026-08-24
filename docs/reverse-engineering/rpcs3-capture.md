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

## See also

- [rpcs3-debugger.md](rpcs3-debugger.md) - the stub, and the traps around it.
- [rcsmaterial.md](../formats/rcsmaterial.md) - what the shader tables hold.
- [methodology.md](methodology.md) - observe, hypothesise, verify, document.
