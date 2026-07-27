# Camera views

How the player-selectable in-race camera works in Wipeout Pulse (PSP,
UCUS-98712), image base `0x08804000`.

This page is written **while** the investigation runs rather than after it, on
purpose: a previous pass established the first finding below and then lost
everything else it had, because nothing was on disk. Each finding is appended as
it is made.

## Known before this pass

**SELECT cycles the in-race camera view.** Established by an earlier session
(credited here; that session wrote nothing to disk and its evidence beyond the
bare observation is lost). SELECT is abstract button index **15** in the layer
[input.md](input.md) documents, so a handler looking for the view toggle is
looking for a test of bit 15 on the input object's pressed mask at `+0x48`.

Confidence **60** as inherited: reported as observed on the running game, but
without a recorded screenshot, address or reproduction. **It is right**, and
everything below is the corroboration - the handler, the button index, the three
values it cycles and the observed period all agree with it.

## Status of this investigation

| Question | State |
| --- | --- |
| The views and their order | answered, below |
| Where the selected index lives in memory | answered, below |
| Sign of `ChaseParams::pos_length` | answered, below: the disc value is a signed offset and is used as-is |
| A third parameter block for an internal view | answered: there are **five** blocks |
| Whether the view is persisted to the profile save | answered: yes, as a string setting |

## There are three player-selectable views, and SELECT cycles them

`Camera_UpdatePlayerView` (`0x0883c0cc`) is the player camera update and the
function that owns the view cycle. It opens with the SELECT test and the cycle,
and the whole thing is a **string** rotation, not an integer one:

```c
setting = Settings_GetString(g_settings, "Camera", 0, 1);   /* FUN_08808664 */
if (Input_IsPressed(g_input, 0xf, 0)) {                     /* 0xf == SELECT */
    Input_ConsumePress(g_input, 0xf);
    *(u8 *)(g_settings + 0x45b) = 1;                        /* settings dirty */
    if      (streq(setting, "OPT_INT"))   next = "OPT_CLOSE";
    else if (streq(setting, "OPT_CLOSE")) next = "OPT_FAR";
    else if (streq(setting, "OPT_FAR"))   next = "OPT_INT";
    else                                  next = setting;   /* unrecognised: no change */
    Settings_SetString(g_settings, hash("Camera"), next, strlen(next) + 1);
}
```

So the cycle is exactly

**`OPT_INT` -> `OPT_CLOSE` -> `OPT_FAR` -> `OPT_INT`**, three views, and it
**wraps**.

Index 15 (`0xf`) is SELECT in the abstract button layer, matching
[input.md](input.md); the press is consumed, so one press cannot advance the
view twice.

The three views are then applied by name, each selecting a differently named
camera rig ("tripod") and setting one flag on the player craft:

| Setting value | Rig passed to `FUN_08885e84` | Cached index at `controller+0x3c` | `craft+0x6d` |
| --- | --- | ---: | ---: |
| `OPT_INT` | `player_internal_tripod` | 0 | 1 |
| `OPT_CLOSE` | `player_external_close_tripod` | 1 | 0 |
| `OPT_FAR` | `player_external_far_tripod` | 2 | 0 |

`craft+0x6d` is copied to `craft+0x6f` immediately afterwards and is `1` only
for the internal view, so it reads as **"hide the player's own ship"** - the one
thing that distinguishes a cockpit view from an external one. That reading is
inference from the correlation, not from the consumer, so it is scored lower
than the cycle itself.

The rig names line up with the on-disc parameter blocks: `player_internal_tripod`
with `<InternalCamera>`, and the two external tripods with
`<ExternalCameraClose>` and `<ExternalCameraFar>`.

Confidence **88** for the cycle and its order: read directly out of the
decompilation, the three string literals are unambiguous, and it agrees with the
observed behaviour (below). Confidence **70** for `craft+0x6d` meaning "hide own
ship".

**This is not the only SELECT test in the binary.** `FUN_08879960` also calls
`Input_IsPressed(g_input, 0xf, 0)` and toggles a `bool` at `param_1+0x3c`, next
to code writing a 65-degree field of view and a 480/272 aspect into globals. It
was **not** identified, and whether it runs during a race at all is unknown, so
nothing here rules out a second SELECT consumer. The search that found it was
`li a1, 0xf` across the program, which is a lower bound - an index that is not
materialised by that exact instruction, or a test through `FUN_0894f258` (the
release-edge sibling `InGame_UpdatePauseInput` uses), would not appear in it.

### Observed, on the running game

Driven through PPSSPP's debugger in a Time Trial on Talon's Junction White
(see [ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md)),
pressing SELECT ten times and screenshotting after each press gives a cycle of
**period three**: two views in which the player's ship is drawn behind the
camera at two different distances, and one in which no ship is drawn at all.
The period is exact - the "no ship" frames fall on presses 2, 5 and 8 of the
run, with nothing in between differing in kind.

The `<ExternalCameraFar>` view sits further back and higher than
`<ExternalCameraClose>`, which is what the two blocks' `pos_length` magnitudes
predict (the far block's is the larger of the two on every ship on the disc).

Confidence **85**: reproduced over ten presses, and the period matches the code
exactly; the mapping of a particular screenshot to a particular *name* is by
eye, which is why it is not higher.

## Where the selected view lives in memory

It lives in the **settings/profile object**, `g_settings` = the pointer at
`0x08b31774`, as a string keyed by the hashed name `"Camera"`, with values
`"OPT_INT"`, `"OPT_CLOSE"` and `"OPT_FAR"` (string literals at `0x08a7b5c0`,
`0x08a7b5c8` and `0x08a7b5d4`). A reimplementation wanting one small enum should
note that the original does **not** keep one: the setting is the string, and the
integer at `controller+0x3c` is only a cache used to notice a change and swap
the rig.

`*(u8 *)(g_settings + 0x45b) = 1` on every change is a dirty flag, and the same
byte is set by the photo-mode path in `FUN_08814014` on transitioning to the
photo state. So **the selected view is persisted with the rest of the profile
settings**, not held only for the duration of a race - which answers the last
open question on this page, at confidence **75** (the write into the settings
blob and the dirty flag are both read directly; that the blob is what reaches
the memory stick is inferred from `0x08b31774` being the profile object the
save path uses).

Confidence **85** for the storage location and the three values.

## There are five camera parameter blocks on the disc, not two

`Handling_ParseStats` (`0x0883a2f0`) dispatches five camera elements, each to its
own attribute parser, into one per-team structure:

| XML element | Parser | Offsets, in struct order |
| --- | --- | --- |
| `<InternalCamera>` | `HandlingXml_ParseInternalCamera` (`0x08838820`) | `+0x00` fov, `+0x04` height, `+0x08` length, `+0x0c` pitch, `+0x10` headtilt |
| `<BonnetCamera>` | `HandlingXml_ParseBonnetCamera` (`0x08838978`) | `+0x14` fov, `+0x18` height, `+0x1c` length, `+0x20` pitch |
| `<BackwardCamera>` | `HandlingXml_ParseBackwardCamera` (`0x08838aa0`) | `+0x24` fov, `+0x28` height, `+0x2c` length, `+0x30` pitch |
| `<ExternalCameraFar>` | `HandlingXml_ParseExternalCameraFar` (`0x08838bc8`) | `+0x34` pos_height, `+0x38` **pos_length**, `+0x3c` lookat_height, `+0x40` lookat_length, `+0x44` fov, `+0x48` spring_horiz, `+0x4c` spring_vert |
| `<ExternalCameraClose>` | `HandlingXml_ParseExternalCameraClose` (`0x08838d8c`) | `+0x50` .. `+0x68`, same seven in the same order |

All five parsers read their values with `Xml_AttributeAsFloat` (`0x0895379c`),
which **cannot read exponent notation** - see [xml-reader.md](xml-reader.md).

**The block closes exactly.** `5 + 4 + 4 + 7 + 7 = 27` floats = `0x6c` bytes,
`+0x00` through `+0x68` with no gap and no overlap, and `+0x6c` is already the
next subsystem: `+0x6c`, `+0x70` and `+0x74` hold `<AirbrakeGraphics>`'s
`amount`, `up_speed` and `down_speed`.

**`<AirbrakeGraphics amount>` is converted from degrees to radians at load.**
`HandlingXml_ParseAirbrakeGraphics` (`0x08839c68`) stores
`amount * 0.017453292` - `pi/180` to seven digits - into `+0x6c`, while
`up_speed` and `down_speed` beside it are stored raw. Read out of the loader,
and corroborated by the live block holding one team's authored 25 as
`0.4363323`. That is a **fifth** pre-scaled parameter to add to the four
[engine.md](engine.md) found, it belongs in
[handling-stats.md](../../../formats/handling-stats.md)'s Units note and in
`oag_gameplay::handling::SCALED_FIELDS`, and it means `amount` is an **angle**,
which [engine.md](engine.md)'s `AirbrakeGraphics` note (confidence 80, offsets
only) did not say. Confidence **92**.

The camera block itself was confirmed by reading the live structure out of a running
race: every camera field in memory equals the document's value **unscaled**, so
unlike the four handling parameters [engine.md](engine.md) found, none of the
camera parameters is pre-scaled by the loader.

So the answer to "is there a third parameter block for the cockpit view" is that
there are three more, and one of them - `<InternalCamera>` - is what `OPT_INT`
uses. `<BonnetCamera>` and `<BackwardCamera>` are **not** reachable from the
SELECT cycle, which has only three entries; what does reach them is not
determined.

**`<BackwardCamera headtilt>` is a dead attribute.** Its parser tests `fov`,
`height`, `length` and `pitch` and nothing else, so the authored `headtilt` on
that element is read from the file and discarded. `<InternalCamera>` does store
its `headtilt`, at `+0x10`. Worth knowing before a reimplementation treats the
two elements as the same type.

Confidence **92** for the whole layout: read out of the five attribute
dispatches, one store per attribute, and then confirmed field by field against
the live structure, whose 27 floats reproduce one team's document in order.

## A second, separate camera enum exists, and it is not this one

`camera+0x1dc` is an integer camera mode set by `Camera_SetMode` (`0x08880724`),
with values named
by the photo-mode code in `FUN_08814014`: `1` internal, `2` above, `3` front,
`4` close, `7` track. `FUN_088807c8` cycles it `1 -> 4 -> 3 -> 2 -> 7 -> 1` and
`FUN_08880838` is its inverse; both are reached from the **race-end and photo**
paths (`FUN_08829778`), bound to up/down, not to SELECT.

**Do not confuse the two.** Read live off the running game, the global camera
object at `0x08b32c64` held mode `7` and did not change across ten SELECT
presses, while the picture changed on every one. The player's in-race view is
the string setting above; `+0x1dc` is the spectator/photo camera.

Confidence **90**: the mode was polled across the same ten presses that visibly
changed the view and never moved.

## `pos_length` is a signed offset along forward, and the disc value is used as-is

This is the open item [HANDOVER.md](../../../../HANDOVER.md) records, and it is
now settled against the original running.

**Method.** Break at `0x0883c13c`, inside `Camera_UpdatePlayerView` and just
after the `"Camera"` setting lookup returns, so `s4` is the camera controller and
`s7` the render node owner. Then

- ship node = `*(craft + 0x794)`, a 4x4 at `+0x00`, rows at `+0x00`/`+0x10`/`+0x20`
  and its **world position** at `+0x30`;
- camera node = `*(s7 + 0x3c)`, same layout, except that its `+0x30` holds the
  **negated** eye position - the function ends by storing `vneg.q` of the eye it
  computed into exactly that field, which is what identifies the sign;
- `delta = eye - shipPosition`, projected onto the ship's own three rows.

All three rows measured unit length, so the projections are distances in world
units.

**Measured, on a stationary ship, one sample per view:**

| Setting | `delta . row0` | `delta . row1` (up) | `delta . row2` (forward) | `\|delta\|` |
| --- | ---: | ---: | ---: | ---: |
| `OPT_INT` | 0.000 | -0.000 | **+3.000** | 3.000 |
| `OPT_CLOSE` | 0.000 | +3.000 | **-11.250** | 11.643 |
| `OPT_FAR` | 0.000 | +2.999 | **-14.250** | 14.562 |

**The measured direction is the authored direction, exactly.**
`(-14.25, +3.00)` is parallel to the authored `(pos_length, pos_height) =
(-19, +4)`, and `(-11.25, +3.00)` is parallel to the authored `(-15, +4)`: both
views, both components, the same **positive** scalar `0.75` (constant, and
measured again at speed - see below). That is the whole argument, and it survives
whatever produces the scalar, because `Camera_UpdatePlayerView` rewrites only the
*magnitude* along the previous ship-to-eye ray (`0x0883c5b4`-`0x0883c684`:
renormalise, scale by a scalar distance, negate, store) and so cannot flip a
sign. A positive multiple of the authored vector means the offset is used with
the file's own signs.

Two things follow:

1. **`row2` is the ship's forward axis, positive toward the nose.** The internal
   camera sits at `+3.000` along it with zero vertical offset, and
   `<InternalCamera>` on this ship reads `length` 3 and `height` 0 - an exact
   match at scale 1, and the only reading under which a cockpit view sits at the
   nose of a 13-unit hull. The two external cameras are on the **opposite** side
   of the same axis, which is what "behind" means and what the screenshots show.
2. Therefore the disc stores **`eye = position + forward * pos_length + up *
   pos_height`**, with `pos_length` negative meaning behind, on the same signed
   axis as `lookat_length` (stored positive, i.e. ahead). One axis, one sign
   convention, three fields.

**Consequence for the Rust side.** `oag_render::camera::chase::anchor` computes
`position + up * pos_height - forward * pos_length`, so it wants the *negated*
disc value, and `crates/game/src/race.rs::chase_pos_length` supplies exactly
that. **The behaviour is correct today and the negation must not be removed from
both places.** What is wrong is only the documented convention: `chase.rs`
describes `pos_length` as "a positive distance behind", which is not what the
format holds. No Rust was changed here; if this is tidied later, the minimal
change is to make `ChaseParams::pos_length` a signed along-forward offset,
compute `anchor` as `position + up * pos_height + forward * pos_length`, and
delete `race::chase_pos_length` - one sign flip in each of the two files, no
behavioural difference. `race.rs` currently scores that adapter at 80; on this
evidence the geometry itself is confidence **92** (measured on the original,
three views, exact agreement with the authored numbers on the one view that is
not rescaled, and the measured direction exactly parallel to the authored one on
the two that are), so the score on the adapter can rise with it. Note that the
3/4 factor below is a separate open question and does **not** affect the sign.

### The external offsets are used at exactly three quarters of their authored value

Both external samples come out at exactly **0.75** of the authored offset -
`-14.25` against `-19.0`, `-11.25` against `-15.0`, `+3.0` against `+4.0` - while
the internal view is not scaled at all. It **does not touch the answer above**,
because the scalar is positive and applies to the whole vector; but nobody should
transcribe it, and its cause is not established.

**It is not applied at load** - the live parameter block holds `-19.0` and `4.0`
exactly - so it happens somewhere between the tripod and the eye.

Three explanations were on the table and two are now dead. The first stationary
run could not tell a **collision pull-in** (`Camera_UpdatePlayerView` renormalises
the ship-to-eye vector and rewrites the eye at a recomputed distance along the
same ray, shortening it when the trace `FUN_0883198c` finds geometry between the
two - and the ship was parked against the start gantry) from a **speed-dependent
chase distance**, which is what the series' camera is generally believed to do.
A first attempt to separate them was invalid and is worth recording as a trap:
it held thrust for 8 s, moved the ship 6.6 world units and reported `0 kmh` -
the **false start**
[ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md) describes.
The projections came back bit for bit identical, which was evidence that
*nothing had changed*, not evidence of invariance.

Redone after a clean restart - `start`, `down` x4, `cross`, ~30 s with nothing
held, then thrust - the ship covered **541 world units over seven samples**,
climbing and turning through open track, and:

| Sample | Ship position | `delta . forward` | `delta . up` | ratio to authored |
| --- | --- | ---: | ---: | ---: |
| at rest | `(6, -50, -196)` | -14.250 | 2.999 | 0.7500 |
| moving | `(174, -46, -198)` | -14.250 | 2.986 | 0.7500 |
| moving | `(340, -36, -172)` | -14.250 | 2.912 | 0.7500 |
| moving | `(391, -31, -143)` | -14.250 | 2.910 | 0.7500 |
| moving | `(440, -28, -108)` | -14.250 | 2.911 | 0.7500 |
| moving | `(490, -28, -72)` | -14.250 | 3.014 | 0.7500 |
| moving | `(535, -24, -34)` | -14.250 | 2.999 | 0.7500 |

So **neither** hypothesis survives. A collision pull-in would relax the moment
the ship left the gantry; a speed-dependent distance would grow toward the
authored value as the ship accelerated. The forward projection is instead
`-14.250` on every sample, at rest and at speed, in the open and under
structures - a **constant factor of exactly 3/4** on the authored `-19.0`, and
the same 3/4 on `<ExternalCameraClose>`'s `-15.0`. The vertical projection sits
at 3/4 of the authored `4.0` and wobbles by about 0.1 as the ship pitches, which
is the vertical spring (`spring_vert`) doing its job; the forward projection does
not wobble at all.

Where the 3/4 is applied is **not** determined. It is not an immediate constant
anywhere in the camera code (`0x3f40` appears as the upper half of a `lui` in
eight places, none of them in the camera or tripod range), so it is either loaded
from data or is the product of something else. Confidence **90** that the factor
is a constant 0.75 independent of speed and position, from seven samples spanning
541 units; confidence **0** on its origin, which was not found.

**Do not hardcode 0.75 in a reimplementation** - find where it comes from first.
It is worth someone's time: a chase camera 25 % closer than its data says is a
visible difference, and the tidy 3/4 suggests a knob rather than an accident.

## Applied renames

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0883c0cc` | `Camera_UpdatePlayerView` | 88 |
| `0x08838820` | `HandlingXml_ParseInternalCamera` | 92 |
| `0x08838978` | `HandlingXml_ParseBonnetCamera` | 92 |
| `0x08838aa0` | `HandlingXml_ParseBackwardCamera` | 92 |
| `0x08838bc8` | `HandlingXml_ParseExternalCameraFar` | 92 |
| `0x08838d8c` | `HandlingXml_ParseExternalCameraClose` | 92 |
| `0x08880724` | `Camera_SetMode` | 82 |

Deliberately **not** renamed: `FUN_0883198c` (the trace the pull-in hypothesis
rests on, confidence 40), `FUN_08885e84` (selects a rig by name; what it does
with it is not read), `FUN_088807c8` and `FUN_08880838` (the spectator mode
cycle - the *direction* each one moves is a guess, and a name that got it
backwards would be worse than no name), and `FUN_08814014`, whose photo-mode
body is only skimmed here.

## Cross-platform

The PS2 build has been read against this page; see
[ps2-pulse/camera.md](../ps2-pulse/camera.md) and, for the parameter blocks,
[ps2-pulse/handling-xml.md](../ps2-pulse/handling-xml.md).

| Function | PSP | PS2 (`SCES_547.48`) |
| --- | --- | --- |
| `Camera_UpdatePlayerView` | `0x0883c0cc` | `0x0014f208` |
| `HandlingXml_ParseInternalCamera` | `0x08838820` | `0x0014cc98` |
| `HandlingXml_ParseBonnetCamera` | `0x08838978` | `0x0014ce90` |
| `HandlingXml_ParseBackwardCamera` | `0x08838aa0` | `0x0014cda0` |
| `HandlingXml_ParseExternalCameraFar` | `0x08838bc8` | `0x0014cf80` |
| `HandlingXml_ParseExternalCameraClose` | `0x08838d8c` | `0x0014d0d8` |
| `HandlingXml_ParseAirbrakeGraphics` | `0x08839c68` | `0x0014de78` |
| `Camera_SetMode` | `0x08880724` | not located |

**Confirmed by the second binary:** all 27 camera floats at the same offsets in
the same order; `<BackwardCamera headtilt>` parsed and discarded there too;
`<AirbrakeGraphics amount>`'s degrees-to-radians `0.017453` scale at `+0x6c`
with `up_speed`/`down_speed` raw beside it; the three-view SELECT cycle in the
same wrapping order with the same three string literals; and the hide-own-ship
flag written then mirrored to a neighbouring byte, which strengthens the
confidence-70 reading of `craft+0x6d` above.

**The 3/4 factor is not shared.** The PS2 `Camera_UpdatePlayerView` computes its
chase distance inline - `min(length - 1, 3)`, a geometry probe, a per-frame
half-step low-pass, and a vertical lift below 7.5 - and no multiply by three
quarters appears anywhere in it. So whatever produces the PSP's constant 0.75 is
not a constant both builds share, and the PS2 path should be treated as a
different algorithm rather than as a second copy to read the answer off.

## For reimplementation

- Three views, one setting, wrapping in the order `OPT_INT`, `OPT_CLOSE`,
  `OPT_FAR`. Keeping the original's three-value enum is right; keeping it as a
  *string* is not necessary, but the persisted form is a string and a profile
  reader has to know that.
- The internal view is the only one that hides the player's own ship.
- `pos_length` and `lookat_length` are **signed offsets along the ship's forward
  axis**, `pos_height` and `lookat_height` signed along its up axis. Use the
  file's values unchanged: `eye = position + forward * pos_length + up *
  pos_height`. See the note above on which of the two negations in the Rust tree
  should go.
- Nothing in the camera block is pre-scaled at load, unlike four of the handling
  parameters - but the two external offsets do reach the eye at exactly **3/4**
  of their authored value at runtime, from a source that was not found. Do not
  hardcode that; do not ignore it either.
- `<BonnetCamera>`, `<BackwardCamera>` and `<InternalCamera headtilt>` exist and
  are parsed; the SELECT cycle never reaches the first two, and
  `<BackwardCamera headtilt>` is not even stored.
