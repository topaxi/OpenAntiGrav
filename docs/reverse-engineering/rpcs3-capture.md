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

### The `Track Creation` carousel's order, read off the disc (2026-09-13)

`scripts/rpcs3-drive.py browse --nav "Main Menu=right" --nav "Track
Creation=right" --screen "Track Creation" --button right --steps 26`
photographs every step; reading the screenshots names each `right` count.
The first eleven base-game entries, `N` presses from the default highlight:

| Presses | Circuit |
| --- | --- |
| 1 | Anulpha Pass |
| 2 | Moa Therma |
| 3 | Chenghou Project |
| 4 | Metropia |
| 5 | Sebenco Climb |
| 6 | Ubermall |
| 7 | Sol 2 |
| 8 | Talon's Junction |
| 9 | The Amphiseum |
| 10 | Modesto Heights |
| 11 | Tech De Ra |

Not walked further this session. **Qualified 2026-09-29**: presses 12-24
repeat the same twelve names with the cursor still on the top hex row and no
reverse glyph, so `right` most likely wraps at twelve and the second pass is
not a reverse-direction half (see `docs/ui/campaign-screens.md`, "Wipeout
HD/Fury: `Track Creation`"; confidence 80).
**Moa Therma appears in this list** despite no `Data/Environments/*` folder
on this disc naming it and no `.envsettings` file for it anywhere across all
seven `DATAxx.PSARC` archives (checked: the disc's `.envsettings` count is
exactly 33, matching `envsettings_ground_truth.rs`'s own count with no
Moa Therma-named file among them) - the carousel entry, its preview video and
its wireframe circuit model all render, so *something* on this disc backs it;
what environment folder it actually loads is not read here (a capture with
`--nav "Track Creation=right"` once would answer it for free via `TTY.log`'s
`Loading track model` line, cheap for the next session that needs it).

A plan reaches a chosen circuit by embedding the count directly, e.g. Sol 2:

    --nav "Main Menu=right" \
    --nav "Track Creation=right,right,right,right,right,right,right"

`capture`'s own `walk_to_race` fires a screen's whole button list in one
`navigate()` call the one time it sees that screen name as current, then the
outer loop's own `cross` confirms - so this is the 7 `right`s followed by
one confirm, not 7 separate confirms. Verify the result against the
written pair's own `track` field (`TTY.log`'s line, free) rather than
trusting the press count: a confirm dropped mid-transition would leave
`Track Creation` current for a second outer-loop pass and double the
plan's presses, landing on the wrong circuit silently.

### Racebox races the same hull the Team Selection screen does not show

**Qualified 2026-09-29**: the "standard livery highlighted by default on a
fresh save" below was read off a profile that already held a save. On a
genuinely empty `savedata` both Racebox and the Fury campaign open
`Team Selection` on the `concept1` row (`080`/`085` stat digits), and a
profile with a save opened on `normal` - so the screen and the race do not
disagree about the *default*, and what they do when a model was picked
earlier is unread. See `docs/ui/campaign-screens.md`, "Walked on RPCS3
with the input pressed, 2026-09-29".

**Confirmed, 2026-09-13.** Racebox's `Team Selection`/`Ship Select` screen
shows Feisar's standard blue/white/yellow livery highlighted by default, on
a fresh save, with every other slot in the "NAVIGATE TEAM" hull-variant
column padlocked - reading that screen alone says the race will be the
standard hull. It is not: the actual in-race craft in every `sol2-matched`
and `amphiseum-matched` shot is the matte grey/orange `concept1` livery, the
same one `talons-matched`'s own corroboration already established for the
Fury-campaign default walk. So the Ship Select screen's own preview does not
reflect the hull a Racebox race actually spawns with, on either route - a
second, independent instance of the same "screen and race disagree" pattern
`talons-matched`'s section above found once. `--team feisar_c1
--hull-variant concept1` is therefore the right pairing to record for a
Racebox capture too, without needing to select anything in that hex grid.

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
dumps** - an eye on the X axis at 11.0 and 34.4 units, `fov` 39 - which reads
like a packet-format problem and is not one: measured below, `ps3_pose`
already finds the real camera in this data without any packet awareness at
all. Read the dumps rather than the JSON until the fix below lands.

## A packet-aware finder exists; the wrong pick was never the packet format (2026-09-05)

**Confidence 95**, three shots of `talons-fifo3` measured directly.
`ps3_pose.packet_candidates` scans for `TRANSFORM_CONSTANT_LOAD_HEADER`
(`0x00441efc`) and scores only the register index and sixteen floats that
follow each one, rather than sliding a `score` call over every four-byte
offset in the dump. On these three shots that is 66-68 candidates instead of
218-241, in 6 ms instead of 180-220 ms - and it recovers the same real camera
(`eye` within 0.01 of the byte-slider's own hit, in all three shots) that
`ps3_pose.candidates` already found. The cost win is real; a packet-format
gap was never why `capture` reported the wrong camera.

**The actual bug is in `describe`'s pick, and it is independent of which
finder runs.** `describe` keeps whichever hit has the lowest `unit_error`
across every blob, and several other packets in the very same dump decompose
to an *exactly* zero error - `eye (-0.0, -0.0, 12.0)`, `(-0.0, -0.0, 30.0)`
and similar, recurring dozens of times a frame. `0.0 < 7.128e-08`, so that
tie always wins over the real camera. Reproduced directly:

    >>> min(ps3_pose.candidates(blob, base=...) for each region, by error)
    (0x40077...c, 'col', 0.0, (11.002091407775879, -0.0, -0.0))

which is bit-for-bit the wrong pose already recorded in
`talons-fifo3/00.json`. Feeding `packet_candidates`' output through the same
`min(unit_error)` rule reproduces the identical class of wrong pick - a
different exact-zero tie, not the camera - so wiring the packet-aware finder
into `describe` as a drop-in replacement would still report a wrong camera,
just with a `"finder": "packet"` label next to it that reads as fixed when it
is not. It is *not* wired in for exactly this reason: an absent camera is an
honest gap, a plausible-looking wrong one is not (the same rule this
project applies to a stand-in asset applies to a stand-in camera pose).

`ps3_pose.packet_candidates` also settles - by absence - the earlier worry
that `talons/00.json`'s recorded pose (`eye` near the origin at `z=-3.3069`,
`fov_y_deg` 92.9, `aspect` 1.9) needed a stricter `score` to reject. It does
not: `score` is unchanged and still accepts that matrix when handed it
directly (`unit_error` exactly `0.0`), because it is one of the seven
authored cube-face/shadow matrices this page already described as
algebraically valid and simply not the camera - not a broken matrix `score`
should catch. What actually keeps it out of a packet-aware capture is that it
lives in the EBOOT's static data segment, where `packet_candidates` finds
**zero** `0x00441efc` headers at all; a region with no packet is never
offered as a candidate in the first place, which is a stronger exclusion than
any algebraic test could give it.

The register index is one concrete gain for the `viewProj`-vs-`worldViewProj`
question this page's previous section left open: every packet hit now
carries which RSX register it loaded (`256` or `260`), which the byte-slider
structurally cannot report. That makes the two hypotheses cheaper to compare
- whether the single recurring value at `c[256]` and the one at `c[260]`
agree frame over frame - but it does not settle which name belongs to which
register; only the overlay render this page already named as the deciding
test does that.

Fixing the pick needs the multiplicity this page's "Picking the camera out"
section already named as the real discriminator - a value single-valued
within a frame and different between frames - which needs at least two
frames' worth of packets in hand at once, not a per-blob `min`. That is
tracked as an open item, not solved here.

## The pick is fixed, and a rendered overlay confirms it (2026-09-13)

**Confidence 96 for a pick once made** - verified against `talons-fifo3`'s
own dumps and independently corroborated by three rendered overlays; coverage
is separately limited, not covered by this number - see the two refusal
modes a few paragraphs down. `ps3_pose.pick_camera`
implements the cross-frame multiplicity discriminator the sections above
name and never solve: build the set of frames each exact matrix value
appears in, keep only the values unique to one frame (throws out every
degenerate constant, which recurs bit-for-bit across every frame it appears
in at all), then require the survivor to be loaded into **both** RSX
registers `256` and `260` in that frame. That second filter is not
redundant - `talons-fifo3`'s frame `01` has a second frame-unique value that
is itself a degenerate (`unit_error` exactly `0.0`), and it differs from the
real camera in exactly this: it is only ever loaded into `260`, never `256`.
Where the two filters do not land on exactly one candidate, `cmd_capture`
writes `"camera": null` plus `"camera_reason"` and `"camera_candidates"`
(the survivor count) - never a plausible-looking wrong pose. Re-run offline
against `talons-fifo3`'s three existing dumps, the pick reproduces this
page's own table exactly (eye, `unit_error`, `fov_y` and `aspect` all match
to the digit), now labelled with the register it came from: `256` **and**
`260` both, in all three frames - so a live camera write updates the shader's
`c[0..3]` and `c[4..7]` identically rather than only one of them, which is
new evidence for "a matrix landing in both is evidence it is the camera"
(the "Picking the camera out" table's last row) without yet settling which
name (`viewProj` vs `worldViewProj`) belongs to which register - that still
needs the overlay to fix a sign/handedness convention, not a register.

`cmd_capture`'s default `--region` is now the pushbuffer set three shots of
`talons-fifo3` used (`0x40010000:0x4000`, `0x40060000:0x20000`,
`0x40080000:0x20000`), not the EBOOT data segment this page's own
measurement showed holds nothing but the seven static matrices.

**The overlay is taken, and it lines up.** `talons-matched` (four shots,
default Fury-campaign walk into Talon's Junction, `--team feisar_c1`
matching the walk's own default - see below) picked a camera cleanly on
three of its four shots and refused honestly on the fourth. Checked directly
rather than left unexplained: shot `02`'s dump does hold a frame-unique,
algebraically clean candidate (`unit_error` 9.5e-08, a plausible eye) - it
just lands in register `256` alone that frame, not both `256` and `260`, so
`CAMERA_REGISTERS`' dual-register requirement refuses it alongside a
different frame-unique but exact-`0.0`-error degenerate that hit `260`
alone. A real, documented limitation of that filter (see its own doc
comment in `ps3_pose.py`), not a wrong pick and not a mystery either -
refusing here is `pick_camera` costing recall to keep its no-wrong-pose
guarantee, on a frame this session happened to catch it doing so. Rendering
`oag-game --race --track
/data/environments/talons_junction/track.vex --team feisar_c1 --size
1280x720 --camera-pose <9 numbers> --camera-fov <fov> --screenshot` from
each of the three picks and blending 50/50 against the matching `NN.png`
(pairs and blends alike under `data/reference/hd-capture/talons-matched/`
and `data/reference/hd-capture/talons-matched-overlay/`, both gitignored)
shows the tunnel's green pipes, the guard rails, the road's lane markings,
the white support pylons and the ship's own silhouette landing on top of
each other in all three, including a fast banked-corner frame (`01`, RPCS3
side motion-blurred at 529 km/h) and a tight loop (`03`). **No mirroring, no
inversion**: the transpose/handedness convention this project already uses
is the right one, so nothing needed bisecting.

`03`'s pair also surfaced a rendering difference, unrelated to the camera: a
road-surface panel a few dozen units ahead renders as flat black in
`oag-game` where RPCS3 shows shaded geometry. Not chased - it reads as the
same gap [rcsmaterial.md](../formats/rcsmaterial.md) and
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md) already
document for `etched_glass_tech` (Talon's Junction's glass floor): a traced
shading path with "no route to draw it" at all, pending a sampler-routing
decision. This is new corroborating evidence for that existing gap, not a
new one, and is recorded as such rather than reopened here.

**Team and hull variant, verified rather than assumed.** No `TTY.log` line
names either on the walk into a race - checked directly against the
Campaign default walk (Team Selection, Team Launch Transition, Launch Game,
InGame all print nothing about a ship path) and against Racebox's own walk
to a Tech De Ra race, same result. `cmd_capture --nav-shots` photographs
`Team Selection` on the way through regardless, and on two independent cold
boots of the Fury-campaign default walk it shows
**Feisar** highlighted (confidence 92: a direct screenshot read, agreeing
across two boots, but only one measurement *kind*). The hull variant is not
readable off that same screenshot - the ship-model honeycomb's selection
cursor does not show clearly in a still frame - so it leans on
[engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md)'s
already-measured runtime read, "all eight craft of a Fury-campaign event
carry `deref` = `concept1`" (confidence 82 there), and this session adds an
independent visual corroboration: the raced craft's livery in every
`talons-matched` frame is the matte grey/orange "concept" paint, not the
blue/white/yellow standard Feisar scheme `screen-Team-Selection.png` itself
shows. Combined, `--team feisar_c1` **confirms** prior work's own match
rather than assuming it (confidence 88 for the pairing overall, capped by
the single-boot-kind evidence behind the team half).

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

## Cell Selection: `DifficultyButton` toggle, per-rung icon shape (2026-09-28)

**Reads a widget's own state across a screen's default `DifficultyButton`
cycle, with no race entered** - `scripts/rpcs3-drive.py browse`, not
`capture`: `capture`'s own `walk_to_race` only stops at `RACE_ARRIVED`, and
`browse` is the one that screenshots a screen's own carousel/toggle at every
step instead. This is what settled `docs/ui/campaign-screens.md`'s "Which
block is which difficulty" section from a reasoned-to mapping to a measured
one.

```sh
# scripts/rpcs3-drive.py's own DISPLAY_NUMBER is hardcoded to :77; this
# session used OAG_RPCS3_DISPLAY (added this pass, defaults to 77 unchanged)
# because the brief assigned this lane :96 - not because :77 was ever in use
# (a concurrent lane's own PPSSPP work runs on :93/:94, a different emulator,
# no actual collision either way).
OAG_RPCS3_DISPLAY=96 uv run --with evdev python3 scripts/rpcs3-drive.py \
  --image data/images/hdfury-ps3-eu-dec.iso \
  browse --screen "Cell Selection" --button triangle --steps 6 \
  --settle-step 3 --max-presses 12 --timeout 300 \
  --out data/scratch/<your-lane>/rpcs3-cellsel-difficulty
```

**No `--nav` plan is needed to reach `Cell Selection` this way**: the
`browse` loop presses `cross` at every screen's own default-highlighted row
until it reaches `--screen`, the identical path `RACE_WALK` already names -
`Main Menu` -> `Campaign Selection` -> `Grid Selection Fury` -> `Cell
Selection`, landing on `grid8_3_1` (`Fury`, `Race`, Talon's Junction, Venom,
weapons on, 3 laps - the grid's own first cell with an authored
`Locked="false"`, `oag_hd::campaign`'s own precedence-resolved order,
`data/scratch/.../grid_08.xml` read directly off `DATA00.PSARC` confirms the
cell name). **A `--nav "Cell Selection=..."` entry is dead here**: `browse`'s
walk loop breaks out the moment `current_screen() == args.screen`, before
`session.navigate(plan)` ever runs - so reaching a *different* cell (an
`Elimination`/`NitroBattle` one, to check whether its own
`Novice`/`Skilled`/`Elite`-named target triple reads the same icon-shape
mapping) needs a different driver, not an extra `--nav` flag on this one.

Once at `Cell Selection`, `--button triangle` cycles the footer's own rung
one step per press, wrapping `NOVICE -> SKILLED -> ELITE -> NOVICE`;
`square`/`l1`/`r1` were no-ops on this rig on an earlier pass (2026-09-21)
despite the footer's own icon reading `Square` - not re-tested this pass
since `triangle` worked cleanly. `--steps 6` gives two full cycles, two
frames per rung, past the atlas's own "many-frame rotation strip" shape (a
single frame can catch an odd phase).

**Pair each frame to the rung by its own footer text, never by press
count.** Face-button presses drop at HD's own ~9 fps here and produce no
`TTY.log` line to re-synchronise against, unlike a screen transition.
`00.png` - the unpressed frame, taken immediately on reaching the screen
with no settle at all (`browse`'s own `--settle-step` only applies between
*subsequent* presses, unlike `capture --nav-shots`'s `photograph()`, which
does sleep `SCREEN_SETTLE` before its shot - see "A screen must settle
before it is photographed" above) - is a comb-artifact mid-transition
capture on this boot and unreadable. **Its pre-press rung is unknown; do not
infer it from the triangle-cycle's own periodicity against `01.png`** -
that assumes the first press did not drop, which is exactly the reasoning
this paragraph's own first sentence rules out.

**This boot did not run on a fresh profile.** An existing save
(`BCES00664-AUTO-` under `~/.config/rpcs3/dev_hdd0/home/00000001/savedata/`,
left by an earlier HD lane) was on disk, and moving it aside - the
`justfile:700`-documented recipe - was refused by this session's own
permission classifier as a write outside the repository. This turned out not
to matter for the icon-shape measurement above: `Target0/1/2 Medal`'s row is
a live target-threshold indicator keyed on the currently-browsed rung
(`model.difficulty()`), not on any earned/saved state, so that reading is
save-independent - but it does mean `00.png` (unreadable regardless, see
above) says nothing about the fresh-profile default rung either way.

**A separate, pre-existing capture does carry that arrival rung, cleanly.**
`data/reference/hd-capture/talons-matched/screen-Cell-Selection.png`
(2026-09-13, `cmd_capture`'s own `--nav-shots`, which *does* sleep
`SCREEN_SETTLE` before its shot and involves no `DifficultyButton` press at
all on the way in) is the settled arrival state of this exact cell
(`grid8_3_1`) on whatever profile that boot ran on, and it reads `AI
DIFFICULTY (NOVICE)` - see `docs/ui/campaign-screens.md`'s "Which block is
which difficulty" section for the full comparison against this project's
own `Difficulty::Medium` (`SKILLED`) default. Whether *that* profile was
itself fresh is not established either, so this is corroborating rather
than dispositive, but it is a real settled reading, unlike `00.png` above.

**The fresh-profile default is now settled directly, `hd-difficulty` lane,
2026-09-28.** `~/.config/rpcs3/dev_hdd0/home/00000001/savedata/` was
verified empty (`ls -la`, zero entries) immediately before this boot - the
lead moved the existing `BCES00664-AUTO-` save aside for this pass
specifically, so unlike every capture above this one is not merely
"unread", it ran on a save file that did not exist yet:

```sh
OAG_RPCS3_DISPLAY=96 uv run --with evdev python3 scripts/rpcs3-drive.py \
  --image data/images/hdfury-ps3-eu-dec.iso \
  capture --nav-shots --shots 1 --timeout 300 \
  --out data/scratch/hd-difficulty/rpcs3-fresh-default
```

`--nav-shots` walks the identical default path (`Main Menu` -> `Campaign
Selection` -> `Grid Selection Fury` -> `Cell Selection`, landing on
`grid8_3_1`) and photographs each screen's own *settled* arrival state -
`SCREEN_SETTLE` sleep included, unlike `browse`'s unpressed `00.png` above -
before pressing on toward the race the rest of `--nav-shots`' own walk
enters. `screen-Cell-Selection.png` reads `AI DIFFICULTY (NOVICE)` and
`TARGET (NOVICE)`, both, with no `DifficultyButton` press anywhere on the
way in - confidence 90 for the default rung (`Difficulty::Easy`), and
independent corroboration of the `Race`-mode `RB_AI_DIF` reading this
section's own icon-shape measurement above already had, from a genuinely
different boot. This project's own `CellSelection::difficulty` now defaults
to `Difficulty::Easy` on HD/Fury to match (`with_default_difficulty`,
Pulse's `Medium` untouched) - see
`docs/ui/campaign-screens.md`'s "Which block is which difficulty" section
for the implementation.

## Reading a live global without a camera hunt (2026-10-05, magstrip-hd-measure)

`capture --region ADDR:LEN --keep-dumps --shots 1` is the whole recipe for reading a known static
address out of a running race: the dump lands as `<shot>-<addr>.bin` (big-endian) beside the
screenshot, the "camera null" lines can be ignored. Two fixes the lane made to `rpcs3-drive.py`:
`capture` ignored `OAG_RPCS3_GDB`'s port (it always dialled 2345, so a member on its own port got
`ConnectionRefusedError`), and `record` crashed on a leftover `args.nav_shots` block from `capture`.
`record --drive 60` writes a 30 fps MP4 under `~/.config/rpcs3/recordings/BCES00664/` (the stock
config's directory, not the lane's); frame it with `ffmpeg -ss S -t 4 -i x.mp4 -vf fps=30`. Used for the
HD magstrip jitter scales and tuning block: `docs/ghidra/functions/ps3-hdfury-eu/magstrip-wake.md`.

## See also

- [rpcs3-debugger.md](rpcs3-debugger.md) - the stub, and the traps around it.
- [rcsmaterial.md](../formats/rcsmaterial.md) - what the shader tables hold.
- [methodology.md](methodology.md) - observe, hypothesise, verify, document.
