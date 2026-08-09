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
| Where the 3/4 factor on the external offsets comes from | answered 2026-08-08: `g_craft_scale`, a code literal |
| How the internal rig applies `pitch` and `headtilt` | answered 2026-08-08: `pitch` is a rise over a run of 10; `headtilt` rolls the up vector by `craft+0x844`, **identified 2026-08-09 as a smoothed steering-driven lean** - see below |
| How many camera rigs a craft has | answered 2026-08-08: **four**, internal / backward / external close / external far |
| Whether the external rig is reproduced | answered 2026-08-08, third pass: yes, to `0.008` RMS over a 150-tick capture of the original |

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
[engine.md](engine.md) found, and it means `amount` is an **angle**, which
[engine.md](engine.md)'s `AirbrakeGraphics` note (confidence 80, offsets only)
did not say. Confidence **92**.

**Applied 2026-08-04.** `oag_gameplay::handling::SCALED_FIELDS` now has five
entries and `airbrake_graphics_for` does the conversion. It is a separate
function from `handling_for` deliberately: the block is per team rather than per
speed class, and the flaps are a model animation that no force term reads, so it
stays out of `oag_physics::params::Handling` where it would move the determinism
hashes for a graphics change. The `25 -> 0.4363323` pair above is what that
crate's test asserts against.

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

**Superseded 2026-08-08.** It was worth someone's time, and it was found: see
[the 3/4 factor is `g_craft_scale`](#the-34-factor-is-g_craft_scale-a-code-literal-and-it-is-the-scale-this-project-already-knew)
below. It is a code literal in the craft constructor, it is the same global scale
this project had already recovered twice from the physics and the exhaust, and it
is now applied. The confidence **0** on its origin above no longer stands.

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
| `0x08878874` | `Camera_SubmitScene` | 80 |
| `0x08885d78` | `Camera_PublishTripod` | 88 |
| `0x08845ed0` | `Ship_UpdateCameraRigs` | 82 |
| `0x0881bf50` | `Hud_Update` | 80 |
| `0x08901dc4` | `VexCamera_BuildProjection` | 85 |
| `0x089021c8` | `VexCamera_EvaluateFovCurve` | 85 |
| `0x08b34310` | `g_camera_fov_degrees` | 90 |
| `0x08b34314` | `g_camera_tan_half_fov_horizontal` | 85 |
| `0x08b34318` | `g_camera_tan_half_fov` | 85 |

The six new names are from the 2026-08-08 pass above. `Camera_SubmitScene` is
the race camera node's submit: it installs the view matrix into the display's
view stack, builds and installs the projection, fills a frustum-plane set into
the camera object and ends at `Gfx_Enqueue` - the projection half is read at
instruction level and confirmed by a write breakpoint on the stack slot, the
plane block is read as shape, hence 80 rather than 90. `Hud_Update` is named
for its body (it reads the `Dynamic_HUD` and `Multiplayer_Tags` settings by
name and dispatches about fifteen HUD element updates off a bit mask), and the
fov shake is one block inside it.

`VexCamera_BuildProjection` and `VexCamera_EvaluateFovCurve` are the *other*
projection path, the one authored `.vex` camera nodes use: the fov lives on a
u16 key/value curve at `cam+0x54` (`+0x00` key count, `+0x02` a flag byte whose
bit 0 selects the orthographic branch, `+0x04` key times, `+0x08` values,
`+0x1c` the frustum aspect), evaluated piecewise-linearly and scaled
`* 180.0 / 65535.0` for a perspective camera or `* 500.0 / 65535.0` for an
orthographic one, with **65.0 degrees** as the value when the curve is empty.
Two such cameras are live during a race on Talon's Junction, at aspects `2.0`
and `4.0` - neither is the player's, which is why an earlier attempt to read
the player fov by breakpointing `VexCamera_BuildProjection` found nothing that
matched the picture.

Deliberately **not** renamed: `FUN_0883198c` (the trace the pull-in hypothesis
rests on, confidence 40), `FUN_08885e84` (selects a rig by name; what it does
with it is not read), `FUN_088807c8` and `FUN_08880838` (the spectator mode
cycle - the *direction* each one moves is a guess, and a name that got it
backwards would be worse than no name), and `FUN_08814014`, whose photo-mode
body is only skimmed here.

## Cross-platform

The PS2 build has been read against this page; see
[ps2-pulse-eu/camera.md](../ps2-pulse-eu/camera.md) and, for the parameter blocks,
[ps2-pulse-eu/handling-xml.md](../ps2-pulse-eu/handling-xml.md).

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

**Correction, 2026-08-08:** the paragraph above presents that inline chase
distance as the PS2's distinguishing algorithm, and it is not distinguishing.
The **PSP** `Camera_UpdatePlayerView` contains the same three pieces - the
`min(|delta| - 1, 3)` clamp, the `FUN_0883198c` geometry probe, the
`d = prev + (d - prev) * 0.5` low-pass and the `y += (7.5 - d) * 0.5` lift
below 7.5 - read straight out of its own decompilation. What is genuinely
absent from the PS2 side is only the 0.75, so the sentence that stands is
"no multiply by three quarters appears in the PS2 function"; the rest was a
false contrast. Noticed in passing while chasing the fov chain and recorded
rather than left, because the next reader would otherwise use the shared code
as evidence of a difference.

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
  of their authored value at runtime, ~~from a source that was not found. Do not
  hardcode that; do not ignore it either.~~ from `g_craft_scale`
  (`0x08ab0e1c`), a code literal that is the **same** global scale
  `oag_physics::hover::TARGET_GLOBAL_SCALE` already is. Apply it to all four
  external offsets and to none of the internal ones. See the section below.
- `<BonnetCamera>`, `<BackwardCamera>` and `<InternalCamera headtilt>` exist and
  are parsed; the SELECT cycle never reaches the first two, and
  `<BackwardCamera headtilt>` is not even stored.

## The live pose is capturable per tick, and the node stores it transposed

`scripts/psp-trace.py --camera` records the player camera's pose into every row
of a capture, as twelve `cam_*` columns beside the ship's. Method, refined from
the one-shot probe above (2026-08-04, PPSSPP v1.20.4, `UCUS98712`):

- The camera node's **address** is learned once per run: one hit of a breakpoint
  at `0x0883c13c`, read `s7`, node = `*(s7 + 0x3c)`. The breakpoint is then
  removed and the node's 64 bytes are simply read at the ship breakpoint each
  tick - the node is a stable heap object for the life of the race, and a
  second live breakpoint is impossible anyway (see the debugger page: **only
  the most recently added execution breakpoint fires**).
- `+0x30` is negated on the way into the file, per the finding above, so the
  CSV's `cam_pos_*` is a world-space eye like the ship's `pos_*`.

**The node's rotation is stored transposed relative to the ship node.** The
ship node's rows at `+0x00/+0x10/+0x20` are the ship's world axes; the camera
node's rows are not the camera's. Measured on a live capture, external view,
craft placed 50 units before a speedup pad:

- the eye sits **11.64** units from the ship - the `OPT_CLOSE` distance the
  table above measured, to three digits, which corroborates both captures;
- the recorded `fwd` **row** points `dot = -0.333` off the ship, while the
  negated third **column** `-(right_z, up_z, fwd_z)` points at it with
  `dot = +0.987` - the residual matching the raised look-at point;
- rendering our engine from the column reading reproduces the emulator's
  framing; the row reading frames a different view of the same corridor.

So the camera's world axes are the stored matrix's *columns*, look along the
negated third column - i.e. the node holds a world-to-camera rotation, which
with the separately-stored negated eye is the shape of a view matrix kept in
two halves. Confidence **80**: one session, one view setting, but the
competing reading is ruled out by sign, and the eye distance cross-checks
against an independent capture. `oag_trace::replay::camera_orientation_of` is
the one place the transposition is undone.

## The fov unit is degrees, measured against the original's own frame

The unrecovered fov unit (`crates/game/src/race.rs` reads it as degrees and
says so) is now measured. Method: craft placed at rest, one tick captured with
`--camera` plus a window screenshot, our engine rendered from the captured
row's exact ship and camera pose at 480x272, and the fov swept while comparing
frames by RMSE (`just frame-compare`). Sweeping 45-75 in fives and 56-64 in
twos:

| fov passed | RMSE against the original's frame |
| ---: | ---: |
| 56 | 8552 |
| 58 | 8398 |
| **60 (= the authored value, and the minimum)** | **8299** |
| 62 | 8312 |
| 64 | 8421 |

**The authored `<ExternalCameraFar fov>` (60 for this ship), applied as the
vertical fov in degrees at the authored 480x272 aspect, matches the original's
projection at the RMSE minimum, bounded by the sweep to about two degrees.**
Confidence **85**: one ship, one view, one track section - but the method is a
direct pixel comparison against the original's own output, and a wrong unit
(radians, or a half-angle) would sit nowhere near the minimum. The runtime
**3/4 factor above does not enter the projection**: it scales the eye offset,
which a recorded eye already contains, and this measurement says it does not
also scale the fov. See `docs/tools/frame-compare.md` for the workflow.

**Confirmed at instruction level 2026-08-08, and the unit is no longer
inferred**: see the next section. `Camera_SubmitScene` builds the projection as
`m11 = 1/tan(g_camera_fov_degrees * pi/180 * 0.5)` and `m00 = m11 / 1.7647059`,
and `Camera_PublishTripod` computes `tan(fov/2)` from the same value and
multiplies it by a literal `480.0 / 272.0`. So the field really is a vertical
fov in degrees at a hardcoded 480x272 aspect, and the projection read live off
the running game reads `m11 / m00 = 1.764706` exactly with a vertical fov of
**60.04 degrees** at rest, against the authored 60. Confidence for the unit
rises to **94**.

*Corrected from 95 on 2026-08-09.* The evidence here is a runtime trace of one
binary, statically read and live-confirmed, and the
[rubric](../../../reverse-engineering/confidence-rubric.md)'s evidence table
caps that at **94 until a second binary agrees** - the 95-100 band requires PSP
*and* PS2. Nothing about the reading changed; it was scored one point above what
its own evidence class allows.
[projection-vs-the-original.md](../../../rendering/projection-vs-the-original.md)
already used 94 for the same finding, so the two now agree.

## The field of view is dynamic, and this is the whole chain

Recovered 2026-08-08, prompted by [exhaust.md](exhaust.md)'s open item on what
widens the fov during a speed-pad boost. The store-offset sweep that earlier
passes used bottomed out (87 `swc1 ..., 0x50(reg)` sites, none of them the
camera), so each link below was pinned instead with a **PPSSPP memory write
breakpoint**: arm a write watch on the address, resume, and read back the pc
that trips it. Every one of them reported a single pc on every sample, which
is what makes "written only by" a measurement rather than an absence of
evidence from a search.

| Link | Address | What it does |
| --- | --- | --- |
| the projection | `Camera_SubmitScene` `0x08878874` | `m11 = 1/tan(fov * pi/180 * 0.5)`, `m00 = m11 / 1.7647059`, near `= cam+0x80 + 0.5 * (65.0/fov - 1.0)`, far from `g_display+0x1698`; installs it at `display+0x1190` and calls `Gu_SetMatrix(0, ..)` |
| the live fov | `g_camera_fov_degrees` `0x08b34310` | written **only** at `0x08885df8`, inside `Camera_PublishTripod` `0x08885d78`, as `*(tripod + 0x50)`; the same function publishes `g_camera_tan_half_fov` `0x08b34318` = `tan(fov/2)` and `g_camera_tan_half_fov_horizontal` `0x08b34314` = that times a literal `480.0 / 272.0` |
| the tripod's fov | `tripod + 0x50` | written **only** at `0x0884642c`, inside `Ship_UpdateCameraRigs` `0x08845ed0`, as `ship->0x850`; the far tripod's `+0x50` (at `ship+0x300`) takes `ship->0x84c` the same way |
| where those come from | `Ship_UpdateCameraRigs`' tail | with `ship->0x860 & 4` set, `lerp(authored, ship->0x868, ship->0x86c)` saturating at `1`; otherwise the authored `<ExternalCameraClose fov>` (`params+0x60`) and `<ExternalCameraFar fov>` (`params+0x44`) unchanged - **and then both get `+ ship->0x790` added, every frame** |
| the additive term | `ship + 0x790` | written **only** at `0x0881c2b4`, inside `Hud_Update` `0x0881bf50` |

`Hud_Update`'s term is a **shake in the field of view**, not a zoom:

```c
if (amp != 0 || phase > 0)  phase += dt * 2*pi * 4.0;
if (amp != 0)               clamped = clamp(amp, -30.0, +30.0);   /* degrees */
if (phase > 2*pi)           phase = 0;
a      = clamped * (1.0 + 2.0 * (rand01 - 0.5));   /* re-jittered every frame */
offset = cosf(phase) * a - a;                      /* 0 at both ends of a cycle */
```

with `amp` read from `ship+0x80` and a sibling term (`sinf(phase) * -|a|`)
stored beside it at `+0x40`. `ship` here is the **scene-side** craft - the same
pointer `flare+0xc0` holds - and not `Ship_UpdateCraft`'s `a0`; the two are
different objects (`0x09a029c0` and `0x09a03980` in one live session) and
reading the wrong one gives zeros for every field in this section.

Measured live (Talon's Junction, Assegai, external view, craft placed at
130 units/s and free-running): the two tripod fovs read `60.00 + offset` on
**every** tick, with `offset` a decaying oscillation - about `+9.4` degrees at
130 units/s, `+6` at 120, `+3` at 100 and `+0.5` at 75. `ship+0x80` reads `0`
on most ticks with transient spikes (`32.9`, `121.3`, `-30.7` seen, the last
two outside the clamp, which is what the clamp is for), so it is a request
that is consumed rather than a level.

Confidence **85** for the chain: five links, each pinned by a write breakpoint
that reported one pc on every hit, with the arithmetic read at instruction
level and the result cross-checked against the live projection matrix. The
`ship+0x860 & 4` override branch is read from the decompiler only - it never
armed in any capture here - so it carries **70** on its own.

**What this does not settle is whether a speed pad uses either path.** Every
`ship+0x790` burst captured coincided with a *speed drop*, i.e. an impact, and
the one clean run through pad 0's line never armed the flare's boost timer.
The override branch - a target fov with a blend factor - is the shape a boost
kick would take and is the first place to look; the shake is the shape an
impact kick would take. Both are now named, which is the useful half of the
answer: **nothing else in the binary can move the projection's fov.**

**None of this is owed a port, and the reason is worth stating so the next
pass does not treat it as one.**
[`crates/game/src/display.rs`](../../../../crates/game/src/display.rs)'s
`BoostFovKick` is a **deliberate invention of this project** - a boost reads
better with a fov kick, and that is the whole justification. It is not a
reimplementation of anything above, it is not being fitted to the original,
and its `DEFAULT` is a taste decision rather than a measurement. So the two
shapes differing (`BoostFovKick` widens a *tangent* by a multiplier; the
original's `ship+0x790` is additive degrees on a `+/-30` clamp) is not a
defect, and the "90-95 degrees" figure
[exhaust.md](exhaust.md) recorded from a 2026-08-07 tower-width measurement
must **not** be used to retune it.

What this section is for instead is the two facts underneath it that the
projection genuinely does depend on, and that were inferred before and are
measured now: the fov field is **vertical degrees**, and the aspect is a
hardcoded **480.0/272.0**. Both are literals in `Camera_PublishTripod`, and
both are already what `Race::projection` assumes.

## The 3/4 factor is `g_craft_scale`, a code literal, and it is the scale this project already knew

Recovered 2026-08-08, and it **closes the confidence-0 open question above**: the
factor's origin was "not found" and the page told the next reader not to hardcode
it until it was. It is found.

`g_craft_scale` (`0x08ab0e1c`) is written **once**, by the craft constructor
`Craft_Construct_q` (`0x08840c74`), as a literal `0.75` at `0x08841000`. The
reciprocal `1.3333334` is written into `0x08ab0e20` on the very next instruction.
Neither comes from the XML: the store is an immediate.

It is **not a camera factor.** The same global is read by

| Consumer | What it scales |
| --- | --- |
| `Ship_UpdateCameraRigs` (4 reads) | the eye *and* the look-at point of both external rigs |
| `Ship_HoverFourCorner` (4 reads) | the four hover-probe corner offsets, `(+/-2.5, ?, +/-5.0)` |
| `Craft_Construct_q` itself | the three `<Misc>` hull dimensions on their way into the collider, and the initial `Body_SetPosition` height (`0.75 * 150.0`) |
| `Ship_InitCraft`, `Ship_UpdateCraft`, `FUN_088418e0`, `FUN_08837e64` | not read here |

so it is a **global scale on craft-space geometry**. That is the same `0.75` this
project had already recovered twice by other routes and named
`oag_physics::hover::TARGET_GLOBAL_SCALE` and
`oag_render::exhaust::CRAFT_ROW_SCALE` - the first found empirically (it was the
missing suspension headroom), the second read off the craft's world matrix row.
This pass gives all three an address, a single write site and a value that is an
immediate rather than a measurement.

The exact form in the rigs is, for each external view,

```c
eye  = shipPos + (eye  - shipPos) * g_craft_scale;
look = shipPos + (look - shipPos) * g_craft_scale;
```

which is *nearly* the same thing as scaling all **four** offsets - `pos_height`,
`pos_length`, `lookat_height`, `lookat_length`. `fov` and the two spring constants
are not touched, being not lengths.

> **Correction, 2026-08-08, third pass: "the same thing as scaling all four
> offsets" holds for the look-at point and fails for the eye.** The scale runs
> *after* `prevEye = eye`, so the state the spring carries between frames is
> **unscaled**. A rig that pre-multiplies its offsets instead carries a *scaled*
> state - and the craft those two are measured from has moved in between, so they
> are different states rather than the same one in different units. The
> difference only exists while the spring is working, which is exactly what the
> static probes in this page cannot see. Measured over the 150 ticks of
> `data/traces/pad0-boost.csv`: pre-multiplied offsets miss the original's own
> recorded eye by `0.121` RMS, applying the scale after the spring by `0.008`.
> `oag_render::camera::chase::ChaseParams::craft_scale` is the field that exists
> because of this; `crates/game/tests/chase_camera_ground_truth.rs` is the
> measurement.

Confidence **85**: the value and the write site are read directly and the store is
a single immediate; the reading of it as a general craft-space scale rather than as
five coincidences rests on the five consumers above all scaling craft-space
lengths.

**Applied 2026-08-08.** `crates/game/src/race.rs::chase_params` multiplies all four
offsets by `oag_physics::hover::TARGET_GLOBAL_SCALE`, for both external blocks.
Three independent numbers agree that this is right:

| Source | Close view | Far view |
| --- | ---: | ---: |
| authored block x 0.75 | 11.643 | 14.562 |
| this page's static probe (above) | 11.643 | 14.562 |
| a 150-tick capture of the original, 94-153 units/s | ~11.6 | not captured |

The per-tick capture is the new one and it is the strongest: the eye-to-craft
distance is flat at 11.6 across the whole speed range, which also retires
"speed-dependent chase distance" for a second time.

**`<InternalCamera>` is not scaled, and that is deliberate rather than an
oversight.** The internal rig reads `g_craft_scale` nowhere, and this page's own
probe measured `OPT_INT` at `+3.000` against an authored `length` of 3. Two
independent confirmations of an asymmetry that looks like a bug, so it stays.

**Consequence worth stating for whoever reads a screenshot next.** Applying this
moved the *default* far view's eye from 19.4 units out to 14.56, which is a visible
change to what this project renders. It lands at the same time as
`Race::ship_model_matrix` starting to apply the same `0.75` to the drawn model, and
the two **partially cancel in apparent craft size**: model 1.0 with the eye at 19.4
and model 0.75 with the eye at 14.56 look similar, while either change alone does
not match the original. Neither should be reverted on the strength of a screenshot
of the other.

## There are four tripods per craft, not two

`Craft_Construct_q` (`0x08840c74`) builds and registers all four by name, from
`"%s "`-prefixed templates, where `%s` is `player`, `player <n>` or an AI name:

| Rig | Craft offset | Name template | XML block |
| --- | --- | --- | --- |
| internal | `craft+0x0a0` | `"%s internal tripod"` (`0x08a7b9b4`) | `<InternalCamera>` |
| backward | `craft+0x150` | `"%s backward tripod"` (`0x08a7b9c8`) | `<BackwardCamera>` |
| external close | `craft+0x200` | `"%s external close tripod"` (`0x08a7b9dc`) | `<ExternalCameraClose>` |
| external far | `craft+0x2b0` | `"%s external far tripod"` (`0x08a7b9f8`) | `<ExternalCameraFar>` |

Each is `0xb0` bytes: `+0x00..0x3f` a 4x4 pose, `+0x50` the fov
`Camera_PublishTripod` reads, `+0x94` whatever it hands to `FUN_08878644`. The
literals `Camera_UpdatePlayerView` looks up by name (`player internal tripod` and
friends, `0x08a7b5dc` onward) are these four with `player` substituted, which is
what joins the SELECT cycle to the rigs.

**So `<BackwardCamera>` does get a rig**, which the earlier finding that the SELECT
cycle "never reaches" it left open. What binds it is still not determined; that it
exists as a live tripod rather than as parsed-and-discarded data is new. No fifth
rig is built, so `<BonnetCamera>` genuinely has none.

Also read at construction, and useful: `craft+0x6d` (the hide-own-ship flag) is
initialised to `0` and mirrored to `craft+0x6f` immediately, which is weak
corroboration that the un-set state is an external view; and `craft+0x868`, the fov
override's target, is initialised to `60.0`.

Confidence **88**: four registrations read directly, each with its own literal and
its own craft offset, and the offsets match the two `Ship_UpdateCameraRigs` writes
this page already recorded (`craft+0x200` and `craft+0x2b0`).

## `FUN_08885e84` selects a rig and does nothing geometric

Left unread above ("selects a rig by name; what it does with it is not read"), and
now read. It is three lines: hash the name, look the tripod up in the registry at
`0x08b317b8`, increment its refcount at `+0xa0`, store the pointer into
`controller+0x3c`. That is the whole function, and `Camera_PublishTripod` then
reads `controller+0x3c` on the next frame. **Still not renamed**: what
`controller+0x3c` belongs to is not established, so a `Camera_`-prefixed name would
be a claim about the wrong subsystem. Confidence 55 on the reading, which is above
the do-not-rename floor but not above the bar for a name that would be misleading
if the object turns out not to be the camera controller.

## The two external rigs, at instruction level

`Ship_UpdateCameraRigs` (`0x08845ed0`) runs the identical algorithm twice, once per
external block. With `params = *(*(craft+0x94)+0x6c)`, `fwd = *(craft+0x374)`,
`up = *(craft+0x378)`, `side = *(craft+0x37c)` and `dt = param_1`:

```c
anchorLook = shipPos + fwd * lookat_length + up * lookat_height;
anchorPos  = shipPos + fwd * pos_length;              /* no up term here */
d          = anchorPos - prevEye;
eye        = prevEye + up   * dot(d, up)   * spring_vert  * dt
                     + side * dot(d, side) * spring_horiz * dt
                     + fwd  * dot(d, fwd)  * spring_horiz * dt;
eye        = anchorLook + normalize(eye - anchorLook) * length(anchorPos - anchorLook);
prevEye    = eye;                       /* close: craft+0x7e0, far: craft+0x7f0 */
eye       += up * pos_height;
eye        = shipPos + (eye  - shipPos) * g_craft_scale;
look       = shipPos + (anchorLook - shipPos) * g_craft_scale;
Tripod_LookAt_q(tripod, &eye, &look, up);
tripod[0x50] = craft[0x850] /* close */ or craft[0x84c] /* far */;
```

Four things fall out, and they score `oag_render::camera::chase`'s four undeclared
choices:

1. **The two springs split along and across the craft's own up axis** - `up` at
   `spring_vert`, both of `side` and `fwd` at `spring_horiz`, in the craft frame
   and not the world's. That is exactly what `chase.rs` chose without evidence.
2. **The integrator is `error * rate * dt`**, a plain first-order lag with no
   transcendental. Also exactly what `chase.rs` chose. The original does **not**
   clamp the factor; `chase.rs` does, as its own guard.
3. **Only the eye is sprung** and the look-at point is rigid. Confirmed.
4. **The eye's distance from the look-at point is rigid.** The spring result is
   re-projected onto the sphere of radius `|anchorPos - anchorLook|` centred on the
   look-at point, so the spring rotates the eye *about* the aim point and never
   moves it nearer or further.
5. **`pos_height` is applied after the spring**, so the vertical offset is rigid
   and the spring acts on a forward-only offset point.

Confidence **85** for the algorithm: read as instructions at one site that runs
twice.

**Items 4 and 5 were the outstanding work on this camera and are now applied**,
together with the ordering correction to `g_craft_scale` above - see the section
below, which raises all three from a reading to a measurement.

### Applied, and the whole rig measured against the original's own camera

`data/traces/pad0-boost.csv` is the only capture carrying the original's craft
pose **and** its camera pose on the same tick, 150 consecutive ticks of a
speed-pad crossing at 40-154 units/s. Driving `oag_render::camera::chase` with
that capture's pose and its own `dt`, seeded from its tick 0, and comparing
against the recorded `cam_pos_*`:

| Model | RMS, world units | Max |
| --- | ---: | ---: |
| free spring, `pos_height` before the spring | 4.938 | 6.824 |
| `pos_height` after the spring only | 4.954 | 6.831 |
| rigid radius only | 0.415 | 0.556 |
| both, `g_craft_scale` pre-multiplied into the offsets | 0.121 | 0.332 |
| **both, `g_craft_scale` applied about the craft after the spring** | **0.008** | **0.060** |

Three things follow that the instruction reading alone did not give:

- **Items 4 and 5 are only worth having together.** Either alone is worse than
  useless: `pos_height` after the spring without the rigid radius is *slightly
  worse* than the old model.
- **The scale's placement matters and the earlier reading of it was wrong** - the
  correction above.
- **The 11.6-unit constant camera-to-craft distance is confirmed and its range
  widened**: `11.571`-`11.674` across 40-154 units/s, against an authored
  `0.75 * hypot(15, 4) = 11.643`. A `0.10`-unit spread over a 3.9x speed range
  retires "speed-dependent chase distance" for a third time.

Recorded because it is easy to get wrong on re-reading: the along-forward
component of that offset is **not** the `10.9 -> 9.5` an earlier note quoted; it
is flat at `-11.25` across the whole capture, and the sideways component stays
under `0.4`. The rig is close to rigid in the *craft's* frame too, which the
rigid-radius model reproduces without being told to - the constraint it actually
enforces is about the look-at point 25 units ahead.

Confidence **90** for the rig as a whole, up from 85: an instruction-level read
plus a 150-tick numerical agreement to three decimal places, on a capture that
was not used to fit anything. `crates/game/tests/chase_camera_ground_truth.rs` is
the comparison, `#[ignore]`d because it needs both a disc image and the capture.

**One thing was not confirmed and should not be transcribed.** The far block's
`pos_height` term decompiles as `up * up.y * (pos_height + 2 * stats[0x74])`, with
an extra `up.y` factor and a doubled `stats+0x74` that the close block does not
have. It was not checked at assembly level. The measured vertical offset sat at
2.91-3.01 - `0.75 * 4.0` - through a run in which the craft pitched and rolled, so
any extra `up.y` factor is either absent or stayed at about 1 throughout, and a
plain scale matches every measurement. Treated as a decompiler artefact.
`stats+0x74` is unidentified and reads `0` on the evidence available, so nothing
here depends on it.

## The internal rig, at instruction level: `pitch` is a rise over a run of 10

`FUN_088455ec` is the internal rig's update - the only caller of
`Tripod_LookAt_q` that passes `craft+0xa0`. With `row1 = *(shipNode+0x10)`, the
ship node's up row:

```c
eye  = shipPos + fwd  * params[0x08 /* length */];
eye += row1 * (params[0x04 /* height */] + craft[0x870]);   /* -> craft+0x7b0 */
tgt  = eye + fwd * 10.0;                                    /* literal 0x41200000 */
tgt += row1 * params[0x0c /* pitch */];
tgt += craft[0x810];                    /* a look-around offset this fn integrates */
u    = row1 - side * (craft[0x844] * params[0x10 /* headtilt */]);
up   = normalize(cross(eye - tgt, cross(u, eye - tgt)));
up   = Rot(craft[0x880] * 6.28) * up;                       /* a roll about the view axis */
Tripod_LookAt_q(craft + 0xa0, &eye, &tgt, &up);
craft[0xf0] = craft[0x840];             /* = tripod+0x50, the fov */
```

So:

- **`<InternalCamera pitch>` is a vertical offset of the aim point over a fixed
  forward run of `10.0` world units, not an angle.** A `pitch` of 1 tilts the view
  up by `atan(1/10)`, about 5.7 degrees; reading the attribute as degrees or
  radians would be wrong by whatever the ship happens to author. The `10.0` is a
  code literal at the site. Confidence **85**: one site, read as instructions, with
  the literal visible.
- **`<InternalCamera headtilt>` rolls the view's up vector**, by
  `side * craft[0x844] * headtilt`, so the horizon leans by an amount proportional
  to the attribute times an unidentified per-craft quantity. The *shape* is
  confidence 80; `craft+0x844` was **not** identified, so the magnitude and the
  sign of the roll are unknown.
- **No `g_craft_scale`** anywhere in this rig, per the section above.
- Two further dynamic terms exist and are named here so their absence from a
  reimplementation is not read as an oversight: `craft+0x810`, a per-frame smoothed
  look-around offset added to the aim point, and `craft+0x880`, a roll about the
  view axis. Neither moves where the cockpit is.

**Applied 2026-08-08** as `oag_render::camera::internal`, with `pitch` implemented
as the rise it is and `headtilt` **parsed, carried and deliberately not applied** -
a lean applied with the wrong sign leans the horizon the wrong way through every
corner, which is worse than a horizon that does not lean. The original already
discards `<BackwardCamera headtilt>` outright, so an unapplied headtilt is at least
a thing this format does elsewhere.

## Correction: `craft+0x790` has a second write site, and it is speed-proportional

The fov-chain section above records `ship+0x790` as "written **only** at
`0x0881c2b4`, inside `Hud_Update`", on the strength of a write breakpoint that
reported one pc on every hit. **There is a second store**, at the top of
`FUN_088455ec`:

```c
craft[0x790] = dot(fwd, *(shipNode + 0x140)) * 0.075 + craft[0x7c];
```

with `0.075` a constant at `0x08a7b6a0` and `shipNode+0x140` a velocity-shaped
vector. So the additive fov term has a **speed-proportional** component, not only
the HUD's oscillating shake - and `0.075 * 130 = 9.75` degrees against the `+9.4`
that section measured at 130 units/s is close enough to say this is most of what
that measurement was seeing.

**Why the write breakpoint missed it**: that measurement was taken in an *external*
view, and this store lives in the internal rig's update. Whether that update runs
while an external view is selected was not established, and it is the obvious next
check - if it does not, the two write sites are simply never live at the same time,
which is also why one pc was reported on every hit.

### Settled on the running game, 2026-08-09: the law is exact, confidence 94

`g_camera_fov_degrees` (`0x08b34310`) read live at a `Ship_UpdateCraft`
breakpoint, alongside the body's own `fwd` (`+0x20`) and `vel` (`+0x140`):

```
fov_degrees(frame N) = 60 + 0.075 * dot(fwd, vel) at frame N-1
worst residual over 19 consecutive samples: 0.0007 degrees
```

Float32 precision, over a sweep from 122 to 146 units/s. **`craft+0x7c` is
exactly `0`** and the intercept is the authored `60` at every sample. The
one-frame offset is publish order - `Camera_PublishTripod` runs later in the
frame than `Ship_UpdateCraft` - and not a lag in the law; anything sampling the
fov and the body together must account for it.

**The driver is settled by a case no pixel method could reach.** One sample
during a placement transient had forward velocity `-76.5` while `|vel|` was
`69.4`, and the fov read **54.26** - *below* the authored 60. A speed-magnitude
driver cannot go below the authored value at any speed; the dot product predicts
exactly that. So `+0x140` is the body velocity, the projection is onto forward,
and the earlier off-axis objection is closed by construction.

Also settled: **this store runs while an external view is selected** (the
capture is the external chase view), which this page previously left open.

Confidence **94** - a runtime trace of the exact quantity, the rubric's ceiling
until a second binary agrees. Everything below this heading is the reasoning
that got here and is superseded by it wherever they differ.

### Confirmed against the original's own pixels, 2026-08-09, and the decomposition with it

**Both constants of this store are now independently recovered from 16 frames of
the original**, and the puzzle the paragraph below this used to pose dissolves.

Registering our render against the emulator's frames over `pad0-boost.csv`
measures the fov the original must have been using at each tick. Fitting exactly
the form above - `dot(fwd, vel)` from the capture's own basis and velocity
columns - gives `fov = 60.181 + 0.07685 * dot(fwd, vel)`, rms 0.407 degrees, with
95 % intervals of `[0.0710, 0.0827]` on the coefficient and `[59.54, 60.82]` on
the intercept. The recovered `0.075` and the authored `<ExternalCameraFar fov>`
of `60` both sit inside. See
[`projection-vs-the-original.md`](../../../rendering/projection-vs-the-original.md)
for the instrument, its three validations and the out-of-sample check.

Three consequences:

- **`craft+0x7c` is 0 to within +/-0.64 degrees** across 148 ticks and a
  41-151 units/s range. The additive term is the speed-proportional store and
  nothing else.
- **The `+9.4 / +6 / +3 / +0.5` series above is not this term.** It is the
  `Hud_Update` shake decaying after the craft was placed - which is exactly what
  "a decaying oscillation" described, and it falls off faster than speed because
  it is not a function of speed at all. The two measurements were never of the
  same quantity, which is why one could not be fitted to the other.
- **This store does run while an external view is selected**, since the capture
  it was measured in is the external chase view. That is inference from effect
  rather than from execution; an exec breakpoint on `0x088455ec` under an
  external view would settle it directly and costs one emulator session.
- **`shipNode+0x140` is identified: it is the rigid body's own velocity.**
  `iVar12 = *(craft + 0x794)` is the body, and `scripts/psp_trace_fields.py`'s
  runtime-measured field map puts `right`/`up`/`fwd`/`pos` at
  `+0x00`/`+0x10`/`+0x20`/`+0x30` and **`vel` at `+0x140`** - the same pointer's
  `+0x10` is read in this very function as the up axis for the camera's height
  offset, which corroborates the base. So the store is exactly
  `dot(fwd, vel) * 0.075 + craft[0x7c]`, with **no smoothing anywhere in it**.
  A smoothed-velocity reading was proposed as the reconciliation for a pixel
  objection and is **refuted**; the objection turned out to be a confound in the
  measurement, living entirely in the three hardest-yawing ticks of the capture
  (`|residual|` correlates with yaw rate at +0.78, and that metric fits no
  rotation term). Driver confidence **85**. See
  [projection-vs-the-original.md](../../../rendering/projection-vs-the-original.md).

### Three more offsets fall out of the same read

Recorded because they were free and two of them close questions this page asks
elsewhere. All from `FUN_088455ec`, and all consistent with the camera-parameter
block at `*(*(craft + 0x94) + 0x6c)` being `<InternalCamera>`'s
`fov`/`height`/`length`/`pitch`/`headtilt` at `+0x00`/`+0x04`/`+0x08`/`+0x0c`/`+0x10`
exactly as the parser table above has it - which is itself corroboration that
this function is the internal rig.

| Offset | What it is | How it is known |
| --- | --- | --- |
| `craft+0x794` | pointer to the **rigid body** | its `+0x140` is velocity and its `+0x10` is the up row, against the runtime field map |
| `craft+0x374` | pointer to the body's **forward** row | its first row is scaled by the camera block's `length` and pushed along the eye offset |
| `craft+0x37c` | pointer to the body's **side** row | its first row is scaled by `craft[0x844]` and then by the block's `headtilt`, and **subtracted from the up axis** |

That last row is the `headtilt` application this page records as parsed but
unapplied: `up' = up - side * craft[0x844] * headtilt`, now read at instruction
level rather than inferred.

### `craft+0x844` is a smoothed steering lean, and the headtilt item is closed

**Identified 2026-08-09**, which was the whole of the remaining work on
`headtilt`: the sign and scale of the lean are now knowable.

`craft+0x844` has exactly **two readers in the whole image** - the `headtilt`
expression above, and its own producer. That producer is a two-stage filter,
evaluated per frame at `0x0883fab4`:

```c
target  = steer * 0.01;                       /* *(*(craft+0x94) + 0x78), first float */
if (craft[0x8a4] > 0) target -= 10.0;         /* two flags, symmetric */
if (craft[0x8a8] > 0) target += 10.0;
if (craft[0x860] & 2) target = -target;       /* a mirror flag */

rate = clamp(target - craft[0x848], +/-0.6 or +/-0.3) * 5.4;  /* limit depends on
                                                 the sign of the raw input */
craft[0x848] += rate * dt;                    /* rate-limited follower */
craft[0x844] += (craft[0x848] - craft[0x844]) * dt * 4.0;     /* first-order smooth */
```

So the lean **leans into the turn**, is driven by steering, is rate-limited and
then smoothed with a `4/s` first-order filter, and saturates at `0.6` (or `0.3`
on the other sign).

**This is not yet portable, and an earlier revision of this section said it was.**
That claim - "a port needs no new capture: everything above is arithmetic on
values this project already has" - contradicts the confidence note three
paragraphs below, which puts **0** on what the input actually is. Both were
written in the same pass and only one can be acted on. The *shape* of the filter
is portable; its **input is not**, because `*(craft+0x94) + 0x78`'s first float
is unidentified and the `+/-10.0` terms sitting beside a `steer * 0.01` say its
units are not what a first reading assumes. Porting on the arithmetic alone
would ship a lean of unknown magnitude and call it recovered.

**What is settled is the sign**: the craft leans *into* the turn. What is not is
the scale. Identifying that one field - the first float of
`*(craft+0x94) + 0x78` - is what makes this portable, and it is a smaller job
than the filter was.

The same function computes a **second** pair the same way - `craft+0x858` as the
follower at rate `5.0` and `craft+0x854` smoothed at `3.0` - from a richer input
(`steer * 0.01` minus and plus two `0.005` terms, gated off by
`craft[0x860] & 0x1000`). **`craft+0x854` is not a camera value**: its only
consumer outside this function is `FUN_088418e0`, the contact-reaction function.
That is why nothing here is renamed - see below.

**Confidence 80** for the arithmetic, read end to end at instruction level, with
the reader and writer sets established by an operand scan over all 635,908
instructions rather than by an xref search. **Confidence 0** for what either
quantity *means* physically: `*(craft+0x94) + 0x78`'s fields are not identified,
and the `+/-10.0` contributions from `craft+0x8a4`/`0x8a8` are large against a
`steer * 0.01` term, which says the input's units are not what a first reading
assumes.

### A wrong name was live in the Ghidra database, on this very function

`0x0883fab4` was named **`Ship_UpdateStartBoost`** in the database. It is not:
it has no boost window, no `craft+0x294`, and it computes the two lean filters
above. The real `Ship_UpdateStartBoost` is
[`0x0883fdec`](engine.md), which is what
`names.tsv` records and what `engine.md` describes - so **two different
functions carried the same name**, and only one of them legitimately.

This is [the workflow page's](../../workflow.md) "the database can disagree with
`names.tsv` and the docs win" trap, live and costing a reading: the decompiler
answers to the name, so a search for the start boost lands on a steering filter.
`names.tsv` is unchanged and was never wrong; the database had acquired an extra
copy of the name, most likely from one of the fuzzy cross-application sweeps.

**Reverted to `FUN_0883fab4` rather than renamed to something better**, which the
[rubric](../../../reverse-engineering/confidence-rubric.md) requires: what the
function computes is read, what its two outputs *are* is not, and its consumers
span two subsystems, so any name would be a guess dressed up. A wrong name stops
people looking, and this one already had.

Confidence **80** for the store and its constant read directly, **88** with the
pixel confirmation; **75** for the decomposition, up from 0, on one capture of
one ship on one circuit.

`craft+0x7c` is still unidentified as a *field* - what is established is only
that it is quiescent on a clean run, so a capture with an impact in it could
still show it carrying something.

## Applied renames, 2026-08-08 (second pass)

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08840c74` | `Craft_Construct_q` | 65 |
| `0x08885a1c` | `Tripod_LookAt_q` | 65 |
| `0x08ab0e1c` | `g_craft_scale` | 85 |

`Craft_Construct_q` and `Tripod_LookAt_q` both take the `_q` suffix the rubric
requires below 70. `Craft_Construct_q` is unmistakably *a* constructor - it zeroes
several hundred bytes of one object, builds four tripods, loads the handling stats
and the model, and creates the rigid body - but it also takes four arguments whose
meaning was not read, and whether "craft" or "racer" is the right noun for the
object is not established. `Tripod_LookAt_q` is named from its three call sites all
passing `(tripod, &eye, &look, up)` and never from its body, which was not opened;
that it *is* a look-at matrix build is inference from the arguments.

Deliberately **not** renamed: `FUN_088455ec` (the internal rig's update - it also
writes the fov shake term, a smoothed look-around offset and a view roll, so
"update the internal camera rig" describes less than half of it, and the rest was
not read), and `FUN_08885e84` for the reason its own section gives.
