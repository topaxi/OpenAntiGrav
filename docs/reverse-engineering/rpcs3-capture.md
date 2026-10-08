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

## Teleporting the craft in HD (2026-10-07, rpcs3-teleport)

`rpcs3-drive.py place --pose X,Y,Z[,YAW] [--pose ...]` puts the player's craft
at a world position in a running race, settles, and photographs it - the
RPCS3 counterpart of `psp-drive.py place`, with `oag-game --pose` as the
matching render. One boot serves any number of poses.

    uv run --with evdev python3 scripts/rpcs3-drive.py --image <abs>/hdfury-ps3-eu-dec.iso \
        --log-dir data/scratch/<lane>/logs place \
        --track 'Data\Environments\01_Vineta_K\track.vex' \
        --nav "Main Menu=right" --nav "Track Creation=right,right,right,right,right,right,right,right,right,right,right,right" \
        --oag-game <abs>/target/release/oag-game --out <dir> --pose=-839.8,-146.6,215.0

Each shot writes `NN.png` (trimmed, 1600x1058 at the default geometry) and
`NN.json` (asked pose, the pose the game kept, the pose 3 s later, drift).

**The chain (confidence 90, one boot to read it, three boots to reuse it).**
The craft array at `0x0098d7c0` holds eight ship pointers (stride 4); the
player's has `ship+0x7a60 == 0`, the others `0xffffffff`. `*(ship+0x6944)` is
the rigid body, and `*(*(ship+0x5fac)+0x270)` is the same pointer - the
`physics.md` statement that the craft entry and the world's body are one
object, confirmed live. The ships and bodies are on the heap and move between
boots (`0x33fb96e0`, `0x33fbe4e0`, `0x33fce8f0`...), so the chain is walked
every run.

**The body's fields (confidence 88)**, by reading them while driving, then
by writing them:

| Offset | Content | Evidence |
| --- | --- | --- |
| `+0x1d0 +0x1e0 +0x1f0` | basis rows, w 0: row0 = cross(row1, row2), row1 up, row2 forward | orthonormal; row2 equals the direction of travel; the same row order `psp-drive.py place` writes |
| `+0x200` | position, w 1 | slots 0-7 on the grid are `(6.1, -51.9, -195.9)` ... `(-132.3, -51.5, -175.2)` |
| `+0x110 +0x120 +0x130` | 3x3 transpose of the rows, 16-byte stride | equals the rows' columns |
| `+0x190` | linear velocity | 120 along row2 at speed |
| `+0x1a0 +0x1b0 +0x1c0` | three more xyz vectors (w 1.0), zeroed by `place` | changed with steering; role not isolated |

**World coordinates are ours, with no transform (confidence 85).** The eight
grid positions were read off the bodies; only slot 0 was compared with
`oag-game` (the trace gives the player slot alone):
(slot 0: `(6.10, -51.91, -195.92)` live against `(6.075, -50.08, -195.83)`
before our hover settles, forward `+x` in both) - both x and z are
discriminating, so an axis flip or a handedness change would show; none does.
The matched pictures below are the second check.

**The write is a plain paused write, and it is enough (confidence 85).**
The stub stops the whole emulator, so no breakpoint in the craft update is
needed (an async pause could in principle land
mid-step; none of the 11 placements showed it). A body-only
write of position and rows followed in the next tick: of the position triples
within 15 units of the body on the ship (14) and the entry (13), every one on
the entry and 11 of 14 on the ship followed; the three that did not
(`ship+0x160`, `+0x62e0`, `+0x6320`) did not follow, and what reads them is unknown. The hull points and probes on the entry re-derived from the body, and the first frame after a settle shows the camera at the new place. So `place` writes the body only, and the 11 placements below show nothing relying on those three.

**The game accepts it.** Placed with `--speed 0` (11 placements over 5 boots in total, five of them in the table) on Talon's Junction at three
points and Vineta K at the maintainer's, settled 8 s and re-read 3 s later:

| Pose asked | Kept after 8 s | Drift | Moved in next 3 s |
| --- | --- | --- | --- |
| Talon's `-297.96,-50.5,-172.83` | `-298.0,-49.5,-172.8` | 1.0 | 0.0 |
| Talon's `161.4,-37.9,194.4` | `159.8,-38.1,192.9` | 2.2 | 0.9 |
| Talon's `-200.1,-70.0,77.3` | `-203.0,-70.7,73.5` | 4.9 | 1.4 |
| Vineta K `-839.8,-146.6,215.0` (same on each of 3 boots) | `-840.6,-146.7,214.2` both | 1.1 | 0.3 |
| Vineta K, same with yaw 180 | `-841.0,-146.8,213.6` | 1.8 | 1.1 |

No respawn and no snap-back on any placement, including the 470-unit jump.
Both placements at the Vineta point settle to the same coordinates to 0.1,
so the settle is deterministic from rest. The drift is a slow creep along the
slope of the track at rest, not a correction.

**Two traps that cost a boot each.**

1. **A write during the fly-over does nothing.** The race opens on a `START
   RACE` prompt over a fly-over, and the craft is pinned to its grid slot
   until the countdown ends: the first run read back `-132.3,-51.4,-175.2`
   after all three writes. `place` taps cross and waits `--countdown` (25 s).
2. **A second member's RPCS3 shares everything by default.** Run an own
   instance with `XDG_CONFIG_HOME`/`XDG_CACHE_HOME`/`XDG_DATA_HOME`/
   `XDG_STATE_HOME` under the lane's scratch (copy `dev_hdd0`, `dev_flash` and
   `config.yml`; 3.4 GB, `cp --reflink=auto`), `OAG_RPCS3_DISPLAY`,
   `OAG_RPCS3_GDB=127.0.0.1:<port>`, and **`OAG_RPCS3_PAD_NAME`**: RPCS3's
   evdev profile binds a pad by name, so two instances both naming
   `OpenAntiGrav Virtual Pad` can bind each other's device. `rpcs3_pad.py`
   reads the variable for both the uinput name and the profile
   (`rpcs3_pad.install_input_config` into the lane's own config dir).
   `rpcs3-drive.py stop` is `pkill -x rpcs3` and reaches every instance: never
   use it beside another member.

**Frame size matters.** The 1600x1200 default display clips a 1920-wide frame: the first Talon's and Vineta pictures lost their right edge (the `POS` readout is cut). Run with `OAG_RPCS3_GEOMETRY=2000x1200x24` and `Resolution Scale: 100` and the trimmed shot is 1882x1058, 16:9 and whole; render ours with `--size 1882x1058`. Render ours at the **kept** pose (`render_with` in the JSON does), not the asked one: the settle moves the craft up to 5 units. `--team feisar_c1 --variant concept1` is recorded, the walks' default hull; the original's hull still reads darker than ours in the pair, which this lane did not chase.

**Camera.** `--camera-shots N` reads the pushbuffer N times per pose and runs the cross-frame pick; on a craft at rest it answered `camera null (no frame-unique value loaded ...)` for both poses on one boot, because nothing varies between frames. A placement with `--speed` above 0 might give it material; not tried.

**Boot repeatability.** The Vineta point kept `-840.6,-146.7,214.2` on each of three separate boots (with and without yaw 180: `-841.0,-146.8,213.6-213.7`).

**Matched pictures.** `data/reference/hd-capture/talons-teleport/` (three
poses, clipped frames) and `data/reference/hd-capture/vineta-teleport/` (the maintainer's
point and yaw 180, whole frames; `pair-00.png` is RPCS3 left, ours right). Talon's
matches in geometry, pad and lane at all three points. At Vineta K the
tunnel's curve and the wall are the same, and the **original shows a lit
floor and a teal hexagonal glass ceiling where ours shows a black water floor and a dark ceiling** - the maintainer's
report reproduces from the original side. The craft's own yaw in the RPCS3
frame (about 15 degrees) is the game's visual banking: the body's rows read
back identical to what was written.

**Attitude.** `place` takes the basis from `oag-game --pose ... --trace-out`
row 0 (the same nearest-spline-sample rule our side uses), so both sides
share one attitude rule. The camera is the game's own chase camera, which
snaps with the craft: the first frame after the settle already frames the new
place. The RPCS3 screenshot is 1600x1058 against our 1280x720, so compare
by landmark, not by pixel.

**Omega: not checkable.** No PS4 emulator is in the toolchain, so there is no
live body to read; its executable would need the same chain re-derived by
static reading. **2048:** the Vita3K path is a separate lane. **No `just`
recipe**: every other `rpcs3-drive.py` subcommand is called directly, and the
command needs a per-lane environment a recipe would hide.

## A per-frame craft trace (2026-10-08, `hd-handling`)

`scripts/rpcs3-trace.py` is the PS3 counterpart of `psp-trace.py`: one boot, any number of
`--run name=<script.inputs>[@x,y,z,yaw[,speed]]`, each teleported (the `place` chain above),
settled `--settle` frames with nothing held, then driven from an `.inputs` script while the
player's rigid body, craft object (`ship+0x5fac`, `0x600` bytes) and `PlayerInput` record
are read every frame through `/proc/<pid>/mem` (the no-pause read above). It writes
`<name>-<rep>.csv` (game time, step, throttle, steer, pitch, airbrakes, basis, position,
velocity) and the raw dumps beside it. Results: [hd-handling-ground-truth.md](../physics/hd-handling-ground-truth.md).

    uv run --with evdev python3 scripts/rpcs3-trace.py --image <abs>/hdfury-ps3-eu-dec.iso \
        --out data/scratch/<lane>/<boot> --settle 60 --repeat 2 \
        --nav "Main Menu=right" --nav "Single Player=wait,right" \
        --nav "Track Creation=wait,right,right,right,right,right,right,right,right" \
        --run thrust=verification/scenarios/hd-thrust.inputs@6.10,-51.91,-195.92,90

That walk is **Racebox Time Trial, Venom, weapons off, no AI** on Talon's Junction
(screenshotted per step under `<out>/screens/`; `wait` sleeps 3 s so a screen's entrance
animation does not eat the first tap). The campaign default walk races seven AI craft and
ends when they finish, which froze the player mid-boot on the first attempt.

What it had to learn, each of which produced a wrong-looking result first:

- **Pilot Assist is a save setting and was on.** `*(*0x008b098c)+0x473` (and `+0x474`
  beside it) is the flag `FUN_0022f8c8`, the FirstPlayDialog handler, sets with
  "change to pilotassist enabled". With it on, the throttle reads `92`
  (`<PilotAssistPenalty thrustPercentOnUse="92">` in `/data/xml/handlingstats.xml`) and the
  craft steers itself round a curve. The tool writes `00 00` there at the Main Menu
  (`--pilot-assist off`, the default) before the race is built. This is also why the
  2026-08-20 runs on [physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md) read 100 and
  this lane's first probe read 92.
- **Key the script on the game clock, not on frames.** `craft+0x308` is the game clock and
  the craft integrates each frame's own delta (0.6 to 1.5+ sixtieths; two at a time when
  RPCS3 drops to 30 fps, which happened on one boot of five). The script's state `k` is held
  from game time `k/60`; repeats then agree to a fraction of a degree. Compare in game time,
  never by row.
- **L2/R2 are analog axes.** RPCS3's stock evdev profile (which `oag.yml` leaves in place by
  naming only the device) reads the triggers from `ABS_Z`/`ABS_RZ`; `rpcs3_pad.Pad` now
  writes the axis with the button. HD's airbrakes are L2/R2, so the tool maps a script's
  `l`/`r` there; L1/R1 do nothing in a race.
- **A frame counter.** `find_world` locates an object holding the body in an inline array
  the way this page's physics section describes the world; its `+0x40` counts frames (it is
  not the physics world: vtable `0x00863780`). Each sample is kept only if that counter did
  not move under the reads and the body read back identical; no sample was ever rejected.

**Omega: not checkable**, as for the teleport.

## Capturing one frame's draws (2026-10-07, `vineta-k-fidelity`)

What the pushbuffer holds besides the camera: **every draw of the frame with the state it was issued under** - the fragment
program, the textures on every unit, blend, depth, cull, fog registers, the vertex-constant block, the index and vertex
ranges. `place --dump CHAIN:LEN` writes guest memory beside a shot; `place --hook PY` runs a module
(`scripts/rpcs3_draw_hook.py`) that decides further spans from what was read, while the target is still paused;
`--dump-attempts N` makes the hook ask for a retry when the paused frame is incomplete. `scripts/rsx_fifo.py` walks the stream,
`scripts/ps3-fp-live.py` disassembles a program read out of RAM.

    HOOK_LIGHT=1 uv run --with evdev python3 scripts/rpcs3-drive.py --image <abs>/hdfury-ps3-eu-dec.iso --log-dir <dir> place \
        --track 'Data\Environments\01_Vineta_K\track.vex' --nav "Main Menu=right" --nav "Track Creation=right,right,right,right,right,right,right,right,right,right,right,right" \
        --oag-game <abs>/target/release/oag-game --out <dir> --pose=-839.8,-146.6,215.0 --countdown 35 \
        --dump 0x40000000:0x20000 --dump-attempts 12 --hook scripts/rpcs3_draw_hook.py

**What measured here (confidence 90 unless stated):**

- **The guest address of IO offset `X` is `0x40000000 + X`** for the command stream, index arrays and main-memory vertex
  arrays; **VRAM is `0xC0000000 + offset`** and the stub reads it (fragment programs, textures, local vertex arrays). A texture
  format word's low two bits are the location (1 local, 2 main); `SET_SHADER_PROGRAM`'s too. An array offset's bit 31 is the
  location (1 main).
- **The ring is a chain of JUMPs through 4 KiB aux contexts, and the frame is written into whichever buffer the frame
  uses** (`0x74100..0x12d000` and `0x496100..` alternate between frames): follow the jumps and dump the targets you have not
  got. A zero word is a NOP, not the end. 273 to 287 draws and 12,000 packets for Vineta K.
- **A paused frame can be the wrong half of one**: 66 draws, or none, on a pause that caught the main thread mid-write. Resume
  for 0.4-0.7 s and pause again; the hook returns `None` under 200 draws. Two of ten boots were wasted before this.
- **A hook stage that returns nothing ends the loop** (`if not extra: break`): a stage with no spans must fall through to the
  next one in the same call. Two boots were wasted on this.
- **A render target reads as stale noise unless `Write Color Buffers` is on** (`config.yml`, `Video`). The sea's
  paraboloid probe at `0xc4065380` was a dither of DXT-looking blocks until it was turned on, then a smooth image
  (roughness 5.7 against 112). It costs nothing visible here.
- **Naming a bound texture:** DXT and linear `A8R8G8B8` textures sit in VRAM byte for byte as in the `.gtf` (little-endian
  texels), so five 64-byte samples (at 0, 0x400, 0x1000, 0x4000, 0x10000) plus width, height, format and mips identify one of
  the disc's 6,347 `.gtf` in this circuit's four archives. Use only the samples the `.gtf` has bytes for; a flat first block alone
  matched 50 files. Runtime targets (2048x2048, 1280x720 and 640x360 grabs, 32x32 glow cells) match nothing, as they should.
- **The fog pair is in the program.** The patched constant `{0, 0.031373, 0.031373, 0.0045}` is `Fog.Alternate Fog Color` and
  `Density`, `{0.039216, 0.086275, 0.070588, 0.00025}` the primary pair: the engine fills `fogColour` per draw group.
- **What did not work, so as not to repeat it:** projecting every draw into a screen id-buffer. Position is `s16 x scale + bias`
  (`c[466]`, `c[467]`) times the matrix in `c[256..259]` for the draws that use that vertex format and that matrix, and it
  reproduced the sea sheet's outline on the frame; most draws use another format, another projection layer (`c[256]` takes
  six distinct values) or a world matrix, so the buffer is dominated by two sky-layer draws. 32-bit indices
  (`0x1820` bit 4 clear) and triangle strips (`BEGIN` value 6, 64 draws) also need handling.
- **Booting: `rpcs3-drive.py stop` is `pkill -x rpcs3`**: never use it beside another member; end a run with the driver's own
  exit or kill the PID. A private instance needs its own `XDG_*` copy of `~/.config/rpcs3`, `OAG_RPCS3_DISPLAY`,
  `OAG_RPCS3_GDB`, `OAG_RPCS3_PAD_NAME`, `OAG_RPCS3_SCRATCH_CONFIG`, and `~/.cache/rpcs3` created (`TTY.log`'s directory).

## Giving the player a weapon and filming its effect (2026-10-07, `hd-weapon-ref`)

Nobody had a held weapon on RPCS3 before this. It needs no pad pickup and no
Ghidra call: **the held weapon is a state word in the craft's pickup slot.**

| Cell | Meaning | How read |
| --- | --- | --- |
| `*(ship + 0x5edc)` | the craft's pickup-slot object (`0x230` bytes apart in one pool, craft 0..7) | eight slots read on one boot, all `-1` at the grid |
| `slot + 0x204`, `+0x208` | the held weapon (`-1` empty); **write both** | a write of `+0x204` alone with `+0x208 = -1` never fired; writing the pair did |
| `slot + 0x210`, `+0x218` | an index that counts up per craft (`0..7`, `1..8`), not touched | AI slots, one boot |

Confidence 85: 12 states written on the player on four boots, each changing the HUD
icon, and the AI craft's own slots carried the same pairs when they held a weapon
(`ai*_state*.bin` in `data/scratch/hd-weapon-ref/e6/`).

**Buttons** (the virtual pad, measured on the player): **triangle fires**, **circle
absorbs** (state goes back to `-1` and the blue absorb glow plays), square, r1, l1, r2
and l2 do nothing with a weapon held. **States 11 and above crash the game** ("The PS3
application has likely crashed"): the fire and update jump tables have 0..10.

**Which state is which weapon** (the HUD's damage readout beside the pickup hex against
`WeaponStats_Race.xml`'s `damage`, plus what fired). Only the rows with evidence are
named, and a row below 80 says why:

| State | Reading | Evidence | Conf |
| --- | --- | --- | ---: |
| 0 | Rocket | readout 10 = Rocket's `damage`; two trails | 80 |
| 4 | Turbo | readout 0; the boost carried the craft away | 85 |
| 5 | Shield | readout 0; absorb-style blue glow, state spent | 70 |
| 7 | Plasma | readout 60 = Plasma's `damage`; fire handler sets bit `0x4`, the id `Weapon_FirePlasma` uses | 80 |
| 8 | Cannon | the HUD announces **"Machine Gun"**; readout 5 | 90 |
| **9** | **Bomb** | readout 15 = Bomb's `damage`; fired, a red-and-white bomb sits on the track, ringed, and detonates | **90** |
| 1, 2 | Missile, Quake | readouts 15 and 15 (both tables say 15), not told apart | 50 |
| 3, 6, 10 | unresolved | readouts 1, 0, 1; state 3 never fired | - |

The state is **not** Pulse's craft id: Pulse's Mine is 8 and Bomb 9 by id and 8 and 9 by
bit (`0x2`, `0x100`); here the same bits sit on the other numbers (state 9 sets `0x2`, state 8
`0x100`: `0x0012d970`'s sixteen handlers at `0x0012da48..`), and the readout and the
picture say state 9 is the Bomb.

### The recorder is the clock

`pause`/`resume` plus `screenshot` costs about a second a frame and has no clock.
RPCS3's own recorder (`Session.toggle_recording`, 30 fps, 1280x720, under
`$XDG_CONFIG_HOME/rpcs3/recordings/BCES00664/`) does, and the **HUD's race clock is the
game's own**: on this host the game ran at **0.59x** real time (HUD 20.2 s to 23.0 s over
144 video frames, three boots' worth of frames agreeing within a frame), so one video frame is
0.0197 game seconds. A mid-recording frame is only decodable from its keyframe: start
`ffmpeg -ss` a second before the event and drop the first frames.

`scripts/rpcs3-hd-weapon.py --state N [--teleport-back D]` is the recipe: boot, walk to
the race, tap START RACE and wait out the countdown, start the recorder, write the state,
press triangle. `--teleport-back` puts the craft D units behind the shot after it, so a
laid Bomb or Mine stays in front of the camera instead of under the craft.

### What a Bomb does in the original (Talon's Junction, the player standing, 0 km/h)

Measured by `data/scratch/hd-weapon-ref/e11` (bomb under the craft) and `e12` (craft put 30
units behind it):

- **The bomb tripped on its own owner**, 17 video frames (0.57 s of video time) after it was laid,
  with the player at rest on top of it: a pink dome with a white core, then the blast. (**Read in
  video time since 2026-10-07**: this page first read the film on the HUD clock, 0.35 s; see "Video
  time is the bomb's age" below.)
  `NormalBomb_Update` (`0x001443f8`) loops over every craft in `trigger_radius` (6 in
  `WeaponStats_Race.xml`) with no owner test in the decompile. This engine's trip excludes
  the owner (`force_bomb_trip`'s own comment); **not changed here, the simulation is
  not this lane's**: the difference is in the handover thread.
- **A laid Bomb is drawn as a small red-and-white bomb with a pink ring about three bomb
  widths across, with a second ring that grows outward into it** and a white flash at the
  centre, repeating about every 16 video frames (about 0.3 game seconds, read off 97 frames
  of one boot; `e12/ring.png`). This build draws none of it: **`HD_bomb_halo` is not drawn at all**.
- **The detonation, in the standing case**, camera 7 units behind the craft: the fireball
  fills the screen **pale yellow-white with a marbled texture** for the first 0.9 s (the HUD
  stays on top), recedes to the right of the frame by 1.2 s with the craft flung ahead at
  155 km/h, and leaves white haze by 1.8 s. No brown haze and no orange rim. Frames:
  `data/scratch/hd-weapon-ref/pair_bomb_stationary.png` (left original, right ours;
  rows: the armed bomb, then +0.2, +0.67, +1.17, +1.8 s).

### Video time is the bomb's age, not the HUD clock (`hd-bomb-match`, 2026-10-07)

The HUD race clock ran 0.59x of the recorder's time in these boots, and the entity ages follow
the recorder. Measured on `e11`/`e12` with `ffmpeg signalstats` per frame (`data/scratch/hd-bomb-match/yavg.clean`,
`e12c.clean`), in recorder seconds:

| Event | Film | Law (bomb age) |
| --- | --- | --- |
| lay to whiteout (trip) | 6.1 to 6.7 s = 0.6 s | owner exempt until `0.5` (Pulse's `Bomb_InArmingDelay`), plus a frame or two |
| fireball fills the frame | 6.7 to 8.33 s = 1.63 s | fireball phase `0 .. 1.5` |
| white haze ends | 9.7 s = 3.0 s after the trip | `NormalBombBlast` lifetime `3.0` |
| halo flash period (e12 centre crop) | 0.496 s over five periods | `3 + 13 frac(2 age)`: `0.5` |

Every one agrees to a frame in recorder time and none does on the HUD clock, so read a frame
of this capture as `age = (t - t_trip)` seconds and drive our side by `ticks = 60 age`. The cause
of the HUD's 0.59x (a clamped step, or the recorder's own rate) is not established; the
agreement is the finding. Confidence 80 that entity age is recorder time in these boots (four
independent timings), 50 for any reading of why.

### Polling guest memory live, without pausing (2026-10-07, `hd-weapon-fx`)

`scripts/rpcs3-mem-poll.py`. RPCS3 maps the PS3 address space at host `0x300000000`:
`/proc/<emulator pid>/mem` at `0x300000000 + A` is guest address `A`, readable by an
ancestor process under `ptrace_scope 1` (the driver is one). Checked by reading the
instruction word at `0x00155568` (`0x39400010`, `li r10,0x10`) and a slot word that the
GDB stub also returned. A sample costs microseconds and the game never stops, so one
second of effect is sampled a hundred times; the pause-per-read method of the sections
above manages one a second.

- **Find an object class by vtable**: scan `0x30000000-0x40000000` for the vtable word.
  `0x00864b38` (the Missile explosion's) finds exactly **16 objects, stride `0x2760`**
  (`0x34432920 ...`, differing per boot), the pool `MissileManager+0xcc` points at. The
  manager itself is `hit - 0xcc` of a scan for the first two pool pointers
  (`0x30966c80`, `0x3095f4f0`: the heap moves between boots, so rescan each boot).
- **Host time to video time**: a one-second `gdb.pause()` before the first shot freezes the
  recording. Found by mean frame difference (`< 0.02` for 5 frames at 30 fps), the freeze
  sat at video 5.2-6.2 s for host 0.0-1.0 s, so **video = host + about 5 s** in `m4`
  (confidence 60: one boot, the recorder's start latency varies).

Measured with it (`data/scratch/hd-weapon-fx/runs/m1..m4`, Talon's Junction, the player
standing on the grid):

| Fired state | What the memory and film show | Conf |
| --- | --- | ---: |
| 1 | a projectile flies for 5.5 s; a counter in the block after the manager (`manager + 0x198`, outside its `0x118` bytes) reads 1, 2, 3, 4 at the bounces and 5 as it ends, at host 2.1, 3.2, 4.2, 4.8, 5.5 s, and the projectile is gone at 5.5 s (the list count `manager + 0xc4` is 1 while it flies, 0 after). The Pulse Missile's bounce budget is 5 (`oag_weapons::projectile::missile::MAX_BOUNCES`). The end is a small orange-red burst in the film; **the explosion pool is untouched** (`+0x10c` stays 0, all 16 pool objects byte-identical over 40 s) | 70 that state 1 is the Missile |
| 2 | a projectile that ends 3.4 s later in a large yellow-white burst with orange streaks, filling the left half of the frame (`m1`, `m2`); **the explosion pool is untouched** | state 2 unresolved |
| 3, 6 | nothing drawn at the grid within 10 s (3); the craft is flung ahead (6) | - |
| 10 | on the grid alone (`m4`): no byte of the manager or the pool changes in 14 s | - |
| (m5: player 12 units behind the nearest rival, state 1 fired) | **a hit**: list count 1 to 0 and pool count 0 to 1 in the same 10 ms sample, 0.12 s after the press; the count back to 0 0.98 s later; the object's age field `+0x170` ran 0 to 0.967 linearly; film: full-frame white-out at video 9.67-10.0 s with the rival's hull in it, gone by 10.2 s | 80 |
| (m3, state 10 fired after 6) | `manager + 0x10c` goes 0 to 1 at host 43.44 s and back to 0 at 44.4 s: **one pool entry in use for 1.0 s**; the film's luma `> 225` for 13 frames at video 48.57-48.97 s, i.e. the white-out starts at the count's rise (offset +5.13 s), then yellow and orange ring discs | 70 attribution, 55 lifetime |

So `HD_missile_explosion`'s pool is not entered by the Missile ending on a wall (state 1,
5 bounces) and is entered for 1.0 s when a missile reaches a craft: once with a known firer
(`m5`, `--behind-rival 12`) and once with an unknown one (`m3`). `0x001423a8` reads the same way: it runs the pool's `Start` (`0x00155568`) only from
the missile list's *hit-test* branch (`0x00126b78`), a craft hit, and never from the wall
branch.

## Pausing on an effect and reading the frame behind it (2026-10-08, `hd-whiteout`)

`scripts/rpcs3-hd-whiteout.py` pauses the target a chosen delay after the Missile explosion pool's count
(`manager + 0x10c`, found as in "Polling guest memory live") rises, then screenshots and reads the queued
frame (`--no-draws` skips the pushbuffer and reads render targets only; `--bomb` fires state 9 instead and pauses
`--delay` seconds after the press). `scripts/rsx-draw-list.py <dir> <stem>` prints the dumped frame's draws with
blend, depth, colour mask, viewport, render target (`0x210`) and bound textures. Three boots (`data/scratch/hd-whiteout`)
and what they taught:

- **The pool's age field and the dumped frame differ by about two ticks**: the CPU's `+0x170` read at the pause was
  0.083 s when the frame's own `UV_offset` (`c[464].x`) was 0.05. Read a frame's age off its constants.
- **A retried dump is a later frame than the screenshot before it.** `rpcs3_draw_hook`'s incomplete-frame retry
  resumes for 0.5 s; the script now takes the screenshot after the dump succeeds. The Missile captures (attempt 0)
  were unaffected; bomb frames were retried one to nine times.
- **Render targets are readable only through the GDB stub, with `Write Color Buffers` on.** A read through
  `/proc/<pid>/mem` returned stale or zero data for every target (it bypasses the page protection the write-back hangs
  off). Read that way, the scene target `0x00f50000` and the screen `0x00010000` were all zero and `0x00cc0000` mostly black.
  `Resolution Scale` must be 100 for the pitches below.
- **The scene target `0x00f50000` is A8R8G8B8 with two samples across** (`SURFACE_FORMAT 0x3148`, pitch `0x2800` = 2560
  texels); decoded as fp16 it is `NaN` noise. Averaging pairs of texels gives the 1280x720 picture (before bloom and
  the composite). `0x02300000` (pitch `0x1400`, 320x180) held a bloom-chain image of an earlier frame.
- **Register numbers, read off the stream** (NV4097): alpha test enable `0x304`, alpha func `0x308` (`0x201` LESS,
  `0x204` GREATER), depth func `0xa6c`, depth mask `0xa70`, depth test `0xa74`, cull face `0x1830`, front face
  `0x1834`, **cull enable `0x183c`**, blend `0x310`/`0x314`/`0x318`. `rsx_fifo.py`'s `Walk` drops the subchannel
  bits, so blits on subchannels 3 to 7 land on the same method numbers (`0x300`...); a reader that needs to tell them apart keeps
  `(header >> 13) & 7`.
- **The partial-frame problem is worse for a bomb**: in the second bomb boot two of three dumps never completed in ten
  attempts (42 draws, and 152 seen on one retry), and no bomb tripped in any of its three captures. A bomb frame needs
  more attempts or a longer resume.

The result is in `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, "`hd-whiteout`".

## See also

- [rpcs3-debugger.md](rpcs3-debugger.md) - the stub, and the traps around it.
- [rcsmaterial.md](../formats/rcsmaterial.md) - what the shader tables hold.
- [methodology.md](methodology.md) - observe, hypothesise, verify, document.

### Trap: copying another member's RPCS3 config (2026-10-07)

A scratch `xdg/config/rpcs3` copied from another lane carries that lane's
`input_configs/global/oag.yml` `Device: OAG Pad <lane>` and its `GDB Server`
port. With a different `OAG_RPCS3_PAD_NAME` every press is silently dropped
(the walk stalls at Main Menu or wanders into "Manual Part 1"); fix the device
name and the port in your copy before the first boot.
