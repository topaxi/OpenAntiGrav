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
| How the internal rig applies `pitch` and `headtilt` | answered 2026-08-08: `pitch` is a rise over a run of 10; `headtilt` rolls the up vector by `craft+0x844`, **identified 2026-08-09 as a smoothed steering-driven lean**; ported and measured 2026-10-02 (lean RMS 0.0003) - see below |
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
observed behaviour (below). Confidence **82** for `craft+0x6d` meaning "hide own
ship" - raised from the 70 this page originally scored it at, once
[`shield-pickup.md`](shield-pickup.md)'s `ShipShield_Update` turned up as a
second, independent consumer reading the same byte the same way (this page's
own correction, moved here 2026-09-16: that page's own note said the raise
landed "on that page" and never actually edited this line).

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
`4` close, `7` track. `Camera_CycleModeForward` (`0x088807c8`) cycles it `1 -> 4 -> 3 -> 2 -> 7 -> 1` and
`Camera_CycleModeBack` (`0x08880838`) is its inverse; both are reached from `RaceManager_Update`
(`0x08829778`) while the front end is on `Race End Photo` (or under `g_game_mode 0xc`), bound to up/down,
not to SELECT; left/right on the same screen switch the watched craft. **Correction 2026-10-04**: by
geometry `1` is a nose view (5 ahead, looking forward) and `4` a far chase view (12 behind, 3 above),
measured; see [race-finish.md](race-finish.md#race-end-photos-d-pad-modes-1-and-4-and-the-unreachable-cases-2026-10-04).

**Do not confuse the two.** Read live off the running game, the global camera
object at `0x08b32c64` held mode `7` and did not change across ten SELECT
presses, while the picture changed on every one. The player's in-race view is
the string setting above; `+0x1dc` is the spectator/photo camera.

Confidence **90**: the mode was polled across the same ten presses that visibly
changed the view and never moved.

**Mode `5` exists and is the death camera, found 2026-08-24 from the other
side.** The list above is what the *photo-mode* code names, and its cycle
(`1 -> 4 -> 3 -> 2 -> 7 -> 1`) skips `5`; `Ship_SetState`'s case 4 - the
craft's own explosion - calls `Camera_SetMode(camera, 5)` at `0x08844524`
after handing the camera the wrecked craft as its subject. So the enum has at
least one value the photo path never reaches, and this is it. What mode 5
looks like is read below ("The destroy camera"). See
[zone-mode.md](zone-mode.md), which carries case 4 at instruction level.

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
disc value, and `crates/raceplay/src/camera.rs::chase_pos_length` supplies exactly
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

## The destroy camera (mode 5), read and measured 2026-10-01

Read statically (headless Ghidra on a scratch copy of the project) and measured on a
running PPSSPP: Talon's Junction (`16_Track`), a Venom player craft on the start line
put into `Ship_SetState(.., 4)` by `scripts/psp-wreck-capture.py --camera`, which logs
the camera controller's fields every frame; a second run placed the craft elsewhere
(`--place`) to tell two candidate rules apart. Mode 5 shares one arm of
`Camera_UpdateSpectatorView` with modes 6 and 7, and the three differ only in
`camera+0x268` (`Camera_SetMode`: **35.0**, 17.0, 50.0; the constructor leaves 60.0).

**The camera is not on the craft, and it is not the chase rig.** It stands at one of
the circuit's own authored `Camera` nodes (`track.vex`, class `0xf7`, **ten** on
Talon's Junction) and looks at the wreck from there, zooming until the craft fills
`+0x268` units of the picture. On the start line that is a camera **488.6 units** away
at a **4.10 degree** vertical field. (An earlier note read "168.7 units from the
wreck"; that was a different circuit and a different craft position.)

| Address | Name | What it does | Conf. |
| --- | --- | --- | ---: |
| `0x0888058c` | `Camera_SetSubject` | `camera+0x1e0 = craft`; picks the station by `Camera_PickStation` unless the craft carries its own at `+0xc3c` (zero on every craft read), forces mode 7, takes the starting field `fov(60) + rand(-10, 20)` (at least 3) and the starting focus | 85 |
| `0x0887fedc` | `Camera_PickStation` | the station whose **aim point** is nearest the subject (`dot(d, d)`, strictly less so the first of equals wins; a y-plane filter that is off with `DAT_08abff10 <= 0`, read here as `0`) | 88 |
| `0x08880984` | `Camera_FramingFov` | `2 * atan((camera+0x268 * 0.5) / distance)` in degrees, `distance` from the station's eye to the subject; `65.0` with no station | 88 |
| `0x08880c04` | `Camera_UpdateSpectatorView` | the per-frame look-at, one `switch` on `camera+0x1dc` (cases 0 to 8) | 90 |
| `0x0887fd3c` | `Camera_UpdateSpectator` | the controller's per-frame update: a ten second timer, the hand-over to another craft, then `Camera_UpdateSpectatorView` | 80 |
| `0x08880168` | `Camera_RepickNearSubject_q` | picks a random station among those whose aim is within 60 units of the subject, but only when the current one's aim is farther than 60 | 62 |

**A station is an authored `Camera` node, and its aim is in its payload.** The runtime
object holds the eye at `+0x90` and an aim point at `+0xa0`. Both are in
`track.vex`: the eye is the node's world translation and **the aim point is the
payload's three floats at `+0x10`** (`Camera` is 48 bytes; `oag_vex::camera::Camera::aim`).
They matched the live object to the digit on all ten nodes (eye `468.3436, -22.8052,
-39.8696`, aim `344.1187, -43.1941, -137.0839` for the one picked). The page that
documents the payload for Wipeout HD flyers (zero there) is not wrong; Pulse's circuits
use the field.

**The rule, per frame** (after the craft has moved), as `oag_render::camera::destroy`:

```text
pick:    argmin |aim - craft|
fov0:    2*atan(30 / |eye - craft|) degrees + rand(-10, 20), at least 3
fov_target = 2*atan(17.5 / |eye - craft|) degrees
focus_target = craft position            (frozen once the craft is in state 6)
focus += (focus_target - focus) * 0.4    (camera+0x250, from +0x264: Venom 0.4)
fov   += (fov_target - fov) * 0.06       (camera+0x228, from +0x260)
z = normalize(eye - focus) with z.y *= max(1 - fov * 0.008, 0.4)
x = normalize(up x z), y = z x x, eye = the station's eye
```

**Measured, per frame.** The field started at 19.914203, read 18.965461 the next frame
(`19.914203 + (4.101826 - 19.914203) * 0.06 = 18.965461`) and 6.730304 at the
twenty-ninth, converging on `fov_target` 4.101826 (`framing(35, 488.6)`). At the
179th frame the view node's columns were right `(0.320020, 0, -0.947411)`, up
`(-0.051120, 0.998543, -0.017267)` and back `(0.946031, 0.053957, 0.319554)` with
translation `-eye`, which the formula reproduces to `2e-4`
(`oag_render::camera::destroy::tests`). The focus rate `0.4` was read only on Venom;
Flash `0.5`, Rapier and Phantom `0.6` are read in the constructor (`FUN_0887f9bc`).

**The pick is by aim, not by eye.** The craft placed at `(-450, -40, -100)` was given
station 6 (aim 115.5 away) over station 2 (eye 186 away, aim 303 away). On the start
line the nearest two aims are 343.2 (station 7) and 347.2 (station 0): a near tie, so a
craft a few units off the line is given the other one, which one capture showed.
Confidence **88**.

**What the picture is.** Through 4 degrees the wreck is a dark shape in the middle of
the frame at about 35 units across, the walls and the track around it as from a
telephoto lens on the grandstand; the HUD is gone by 30 frames into state 5 (not
ported). At state 6 the focus stops following the craft, the player's blast washes
the screen (kind 7) and `Camera_ArmShake(0.8, 0.6, camera, 1)` throws the view about
by more than the field, so the frames that follow are a different part of the circuit
for a quarter of a second. `FUN_0883e064` arms `(0.3, 0.4, camera, 3)` at state 5.

**Ported for a PSP disc only**: the aim point was read off the PSP's `track.vex` and the PS2's
was not looked at, so `race::destroy_camera` keys on the loader's `pulse_psp` flag. The cut
counts as a camera cut for the temporal upscaler's history (`Race::camera_cuts`); the motion
blur is stateless and smears the cut frame and the largest shake frames as it does a respawn.

**Not read or not ported.** The camera hands over to another craft after ten seconds
(`+0x3c`), and when the craft's state-6 timer runs out; what ends the destroy camera in
a mode that respawns the player is read only as "the craft is racing again"
(**chosen, not measured**). The re-pick when the craft is more than 60 units from the
current aim (`Camera_RepickNearSubject_q`) cannot fire on a wreck that lies still and is
not ported. The starting field's `rand(-10, 20)` is unseeded in the original and drawn
here from a stream of its own. Whether a player's craft ever carries its own station at
`+0xc3c` is not known. Case 4 also hides the HUD by degrees, which this port does
not.

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
| `0x0888058c` | `Camera_SetSubject` | 85 |
| `0x0887fedc` | `Camera_PickStation` | 88 |
| `0x08880984` | `Camera_FramingFov` | 88 |
| `0x08880c04` | `Camera_UpdateSpectatorView` | 90 |
| `0x0887fd3c` | `Camera_UpdateSpectator` | 80 |
| `0x08880168` | `Camera_RepickNearSubject_q` | 62 |
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
confidence-82 reading of `craft+0x6d` above.

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

**Corroborated from the audio side, 2026-08-24, and the score moves 80 -> 88.**
`SoundManager_Update` (`0x0893a2b0`) copies this same matrix off the camera
object every frame to place the sound listener, and it assumes *both* halves of
the reading above and nothing else: it takes `(m[0][0], m[1][0], m[2][0])` - the
first **column** - as the world right axis to pan against, and it writes
`-(camera+0x70)` as the listener's world position. A second subsystem, written
against the same struct by different code, agreeing on the transposition *and*
on the negated eye, is the independent check the 80 was short of. See
[positional-audio.md](positional-audio.md).

## The fov unit is degrees, measured against the original's own frame

The unrecovered fov unit (`crates/raceplay/src/camera.rs` reads it as degrees and
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
[`crates/display/src/display.rs`](../../../../crates/display/src/display.rs)'s
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
`oag_fx::exhaust::CRAFT_ROW_SCALE` - the first found empirically (it was the
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

**Applied 2026-08-08.** `crates/raceplay/src/camera.rs::chase_params` multiplies all four
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
  to the attribute times the smoothed raw-stick lean (`craft+0x844`, identified
  2026-08-09 and measured 2026-10-02 - see below). Ported 2026-10-02.
- **No `g_craft_scale`** anywhere in this rig, per the section above.
- Two further dynamic terms exist and are named here so their absence from a
  reimplementation is not read as an oversight: `craft+0x810`, a per-frame smoothed
  look-around offset added to the aim point, and `craft+0x880`, a roll about the
  view axis. Neither moves where the cockpit is.

  **`craft+0x880` is identified, 2026-09-06: it is the barrel roll.** It is the
  eased form of the roll phase `craft+0x87c`, written at `0x08841f88` in
  `FUN_088418e0`, and the same `* 6.28` also rotates the *ship's* display matrix
  about its nose in that function. So this line is the cockpit view rolling with
  the manoeuvre, not a standalone camera effect - see
  [input-bindings.md](input-bindings.md#the-roll-is-drawn-0x87c-eases-into-0x880-which-rolls-the-ship-about-its-nose).

**Applied 2026-08-08** as `oag_render::camera::internal`, with `pitch` implemented
as the rise it is. `headtilt` was carried and unapplied until 2026-10-02, when the
steering lean it multiplies was ported and measured - see "`craft+0x844` is a
smoothed steering lean" below. The original discards `<BackwardCamera headtilt>`
outright, so only the internal block's value ever reaches a view.

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

**Read at instruction level and measured 2026-10-02; ported as
`oag_physics::controls::update_camera_lean`.** Corrections to the prose above,
from the instructions at `0x0883fab4` and a PPSSPP capture of `entity+0x844` and
`entity+0x848` (Talons Junction, Assegai/Venom, `verification/scenarios/steer-lean.inputs`,
`data/traces/talons-junction-steer-lean.csv`):

- **The base is the entity**, not the craft: `FUN_0883fab4`'s `a0` is the same
  `entity` the `0x854` filter lives on (`entity+0x94` is the craft). The page's
  "craft+0x844" is entity+0x844 throughout.
- **`0.6` and `0.3` clamp the delta, not the lean.** `d = target - follower`; `d`
  is clamped to `+/-0.6` when its sign matches the raw stick's and to `+/-0.3`
  otherwise (stick centred, or opposing), then `follower += d * 5.4 * dt`. The
  lean itself therefore settles at the target, `+/-1.0` at most, not at `0.6`.
- **The input is closed**: `*(*(craft+0x94)+0x78)` +0x00 is the raw `+/-100` stick
  record (`cannon-quake-leachbeam.md`, item 5), so `target = stick * 0.01`.
  The capture agrees: a full right settles at `0.982`, a record of 98.2.
- **A partial stick is not linear.** Scripted `stick_x=-0.5` settles at `-0.109`
  (a record of about `-11`), and the ramped `steer` at about `-11`..`-18`; our
  `ShipControls::steer_x * 100` gives `-50` for the same input and `-37` on the
  ramped `steer`. The original shapes the analog axis before the record; ours does
  not. The lean inherits whatever the stick path does, and this is a gap in the
  stick path rather than in the lean.
- **Omitted, chosen, not measured**: the `+/-10.0` terms from `entity+0x8a4` and
  `+0x8a8` and the negate on `entity+0x860 & 2` are treated as clear. None of the
  three is identified, and all three were clear throughout the capture (the lean
  never left `+/-1`).

**Measured**: replaying the lean filter over the capture reproduces `+0x844` and
`+0x848` to **RMS 0.0003** over 480 ticks (a full right, a release, a half left, a
release), at one tick of capture latency; `crates/trace/tests/camera_lean_ground_truth.rs`
pins it, and a mutated delta limit fails it. Confidence **90** for the filter.

**The roll.** `up' = up - side * (lean * headtilt)`, taken perpendicular to the view
direction, then rolled by the barrel roll. The `side` row (`entity+0x37c`) is the
ship's **left** (the recorded `right_*` columns are the left, `oag-trace run
--basis left-up-forward`), so in this engine a positive lean adds the ship's **right**
to the up vector: the view leans into the turn. Over the same capture, re-taken with a local, uncommitted extension of
`psp-trace.py` that also read the tripod at `entity+0xa0`, the rows behind
`entity+0x374`/`+0x37c` and `entity+0x810`, and with the body's own rows, the angle between the predicted and
the recorded tripod up is smallest at `k = +0.3 * lean` (RMS 0.261 rad) and rises
on both sides (`k = 0`: 0.312, `+0.6`: 0.284, `-0.3`: 0.416, `-0.6`: 0.526), where
`0.3` is this ship's authored `headtilt`. **That fit is consistent with the sign and
scale and does not prove them**: the margin (0.261 against 0.312) is smaller than
the 0.26 rad the model leaves unexplained, because `tgt += craft[0x810]` and the rows
`craft+0x374`/`+0x37c` point at nodes whose full behaviour was not ported or fitted
here. The sign rests on two firmer things: the instruction-level minus in
`FUN_088455ec`, and the measured `left-up-forward` reading of the body's row 0.
Confidence **70** for the sign and scale
of the roll, **90** for the lean it multiplies. Ours is
`oag_render::camera::internal::view`'s `tilted_up`, with `HEADTILT_SIDE_SIGN = -1`.

The tripod rows at `entity+0xa0` are stored mirrored in `z` against the body's rows
(the tripod's yaw is minus the body's, to 0.1 degree at rest), which is why a naive
dot product against the body looks like a camera lagging by 180 degrees.

The same function computes a **second** pair the same way - `craft+0x858` as the
follower at rate `5.0` and `craft+0x854` smoothed at `3.0` - from a richer input
(`steer * 0.01` minus and plus two `0.005` terms, gated off by
`craft[0x860] & 0x1000`). **`craft+0x854` is not a camera value**: its only
consumer outside this function is `FUN_088418e0`, the contact-reaction function.
That is why nothing here is renamed - see below.

**Qualified 2026-09-06.** "Not a camera value" holds, but `craft+0x854` is not
off the display path either: `FUN_088418e0` reads it at `0x08842148` as
`craft[0x854] * 0.5`, one of the two terms of the roll angle it applies to the
*ship's own* transform (the other being the barrel roll's eased phase). So it is
a **lean the model is drawn with**, on a matrix the camera then follows. That
does not raise its confidence 0 - what the quantity means physically is still
unidentified, and it must not be transcribed - but a reimplementation that treats
it as inert is dropping a visible term. See
[input-bindings.md](input-bindings.md#stage-2a-the-ships-own-display-matrix-0x08842140---0x08842264).

**Confidence 80** for the arithmetic, read end to end at instruction level, with
the reader and writer sets established by an operand scan over all 635,908
instructions rather than by an xref search; raised to **90** for the `0x844` filter
on 2026-10-02 by the capture above. What `+0x844` *means* is no longer open: it is
the raw stick, smoothed. The **second** pair (`0x854`, the ship's display lean) is
still not ported and its input still carries the two `0.005` terms and the
`0x1000` gate described above; the camera does not read it.

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

### `craft+0x7c` stays quiescent through a real wall contact too, 2026-08-27

Checked on Talon's Junction with a standing-start, unsteered `--hold cross` run
(`scripts/psp-trace.py`, `entity+0x7c`/`entity+0x790` added to `ENTITY_FIELDS`
as `fov_intercept`/`fov_additive` in `scripts/psp_trace_fields.py`) - the same
scenario `data/traces/talons-junction-standing-start.csv` and this page's own
"nothing arms the collision stun" section already use. Read at the
`Ship_UpdateCraft` breakpoint, so a tick stale relative to `FUN_088455ec`'s own
update - the same publish-order lag `g_camera_fov_degrees` carries above - which
is immaterial for a field that never leaves zero. Two independent restarts both
produced a real wall contact, not a clean run: `shield` dropped
(`135 -> 134.7959` on one, a comparable drop on the other) and one contact
killed `10.76` units/s of `vel_z` in a single tick - about `646` units/s²,
the same order as the `-790` units/s² hit `ppsspp-debugger.md` records for a
standing-start crash. `fov_intercept` read exactly `0` on every tick of both
captures, through and after the contact tick.

`fov_additive` is a **third independent confirmation of the speed law**, not
just a sanity check: fit against `0.075 * dot(fwd, vel)` computed from each
row's own `fwd_*`/`vel_*` columns, the residual is on the order of `1e-7` at
every sampled tick (`float32` precision), with no discontinuity at the contact
tick. It does **not** match `0.075 * speed_cached` this closely - a ~3 % gap at
matching ticks - and that is not a discrepancy: `speed_cached` is the
*previous* tick's `dot(vel, fwd)` (`engine.md`, confidence 95), while this
store reads the current tick's, so an accelerating craft separates the two by
design. Comparing against the wrong column here would misread an artefact of
the phase offset as an error in the law.

This is a *scrape*, not the AI/rival hit `Ship_ApplyCollisionImpulse`'s stun
gate is shown above to require - `stun_timer` read `0.0` throughout, matching
that section's own captures. So this narrows rather than closes the question:
`craft+0x7c` is now measured quiescent through an actual wall contact, but a
craft-on-craft or weapon impulse (the only inputs that arm the stun) has not
been captured. Confidence **80** for quiescent-through-a-wall-contact, on two
restarts of one circuit; no writer for the field has been found on any path.

### `craft+0x7c` stays quiescent through an armed collision stun too, 2026-08-27

The remaining case - a hit that actually arms `stun_timer` - closed this
thread. Two things had to be sorted out first, both live-verified rather than
assumed:

- **AI craft never run `FUN_088455ec` at all.** A live watch on all eight
  craft's `Ship_ApplyCollisionImpulse` entries found `fov_additive` pinned at
  exactly `0` on every AI craft, every tick, regardless of speed - only the
  player's entity ever carries a nonzero value. So a capture of an AI-on-AI
  hit (five genuine ones were captured this session, impulses of realistic
  magnitude - e.g. `(-24.6, -1.5, -9.2)`, `(22.5, -1.4, 38.1)` units) answers
  nothing about this field: `fov_intercept` reads `0` on an AI craft whether
  or not the field has a writer, because nothing ever runs the code that
  would write it. Only the player's own entity is an informative subject.
- **Getting the player specifically hit through gameplay proved fragile.**
  Random driving with weaving found zero player hits in ~1,600 ticks (the
  five AI-AI hits above all landed on other craft); teleporting the player
  close behind a specific AI craft, and firing seven simultaneous rockets
  from every AI at once, each destabilised the emulator (one froze the race,
  one hung the debugger) without landing a player hit either. Neither
  scenario is required by the thread's own wording ("craft-on-craft **or**
  weapon-impulse"), and `Weapon_PostBlastImpulse_q` accumulates into the
  pending-impulse slot with a plain `vadd.q` - nothing about the slot is a
  handshake - so the consumer path was tested directly instead: a write of
  `(-20.0, -3.0, -10.0)` (matching the two genuine AI-AI captures' order of
  magnitude) into `*(entity+0x4c)+0x110` at a `Ship_ApplyCollisionImpulse`
  breakpoint on the player's own entity, filtered live off `craft ==
  *(entity+0x94)` rather than guessed.

The injection was consumed exactly as a real hit is: `stun_timer` read `0.0`
before the write and `0.4828` one tick after (`0.5` minus one tick's `dt`,
matching `Ship_ApplyCollisionImpulse`'s own `+= 0.5` and the per-tick
countdown this page's contact-response citations already establish), then
decayed steadily (`0.4661`, `0.4494`, ... `0.0991` over the next 23 ticks) -
the positive control this measurement needs, since a write that merely lands
in memory without being consumed would prove nothing. `fov_intercept` read
exactly `0.000000` on **every one of those 24 ticks**, through the stun's
full arm-and-decay. `fov_additive` kept tracking the speed law throughout
(confirming the entity was live and correctly resolved the whole time, per
the offset-chain fix below) - its own value moved only because the craft's
`dot(fwd, vel)` was near zero and slightly negative at the time, unrelated to
the stun.

**Caveat stated plainly**: this tests the *consumer* side of the pending-
impulse mechanism, not a naturally-occurring writer - it proves
`Ship_ApplyCollisionImpulse` taking the hit branch does not touch
`craft+0x7c`, not that a real rival or weapon hit could never route through
some other path this session didn't exercise. The two genuine AI-AI captures
(realistic impulse magnitudes, independently confirmed nonzero) are cited as
corroboration that the injected magnitude is representative, not as a
substitute for a natural player hit. Confidence **75** - live-injected on the
verified consumer path with a real positive control, one tick short of a
fully natural capture.

**A real bug surfaced getting here, now fixed**: `scripts/psp-watch-
pending-impulse.py`'s `ENTITY_TO_CRAFT = 0xFC0` (`craft = entity + 0xFC0`)
does not hold - checked against a known player craft address, it resolved 7
of 8 live `Ship_ApplyCollisionImpulse` a0 values to out-of-RAM-range
addresses, and the one slot that "worked" in the original 2026-08-19 capture
was not the player. `Ship_ApplyCollisionImpulse`'s a0 **is** the entity
directly (the same object `craft+0x1c4` reaches elsewhere); `craft` is
`*(entity + 0x94)`, the reciprocal `ENTITY_OWNER` pointer shield.md already
documents. See `contact-response.md`'s correction for the live evidence
(the reciprocal check, and the fov law matching to float precision only
under this reading).

This closes the open question of whether the original's FOV-widens-with-speed
law is driven off `craft+0x7c`: the field is now measured quiescent through a
clean run, a real wall contact, and an armed-and-decaying collision stun. No
writer for the field has been found on any path in this codebase.

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

**2026-08-25 update**: one of those arguments is now partly read. The second
argument is stored verbatim at `entity + 0x368`, drives the constructor's own
craft-naming branches and an "is_ai"-shaped flag at `entity + 0x48`, and
`Ship_Damage` treats it as a controller/ownership-kind selector rather than a
count - see
[shield.md](shield.md#entity--0x368-is-craft_construct_qs-own-second-argument)
for the read and what is still unconfirmed (no static caller of
`Craft_Construct_q` exists to check the actual values against).

Deliberately **not** renamed: `FUN_088455ec` (the internal rig's update - it also
writes the fov shake term, a smoothed look-around offset and a view roll, so
"update the internal camera rig" describes less than half of it, and the rest was
not read), and `FUN_08885e84` for the reason its own section gives.

## The default view is `OPT_CLOSE`, measured 2026-10-01

**The question** (the 2026-10-01 weapons lane's open item): at the same state the
original's craft is about 1.4 times larger on screen than ours. Which term of the
camera differs - distance, height, look-at, fov, aspect, craft scale, or which block?

**What would have falsified "it is the view block".** If the original's eye
offset in the craft's own frame were `(-14.25, +3.00)` (the far block at 0.75) and
the craft still looked larger, the cause would be a projection or scale term. It
was `(-11.25, +3.00)`.

**Method.** `scripts/psp-camera-pair.py` (new; the camera half of
`psp-weapon-pair.py`): restart the Time Trial to its countdown, hold `cross`, take
the first frame the throttle word is non-zero as GO, and at every stop in
`Weapons_DispatchFire` read the camera node (`*(s7 + 0x3c)` learned from one hit of
`0x0883c13c`), the body's four rows, `g_camera_fov_degrees`, and photograph the
window at native 480x272. Talon's Junction White, Time Trial, Venom/Assegai,
PPSSPP v1.20.4, a **fresh `HOME`** each boot (no save of any title), walked there
by `psp-drive.py menu` so no profile choice was ever made.

**Measured, two cold boots, GO+120 (speed 106.2):**

| Quantity | Boot 1 | Boot 2 | Reads as |
| --- | ---: | ---: | --- |
| eye in the craft's frame (right, up, forward) | `(-0.008, +3.002, -11.250)` | `(-0.008, +3.001, -11.250)` | `<ExternalCameraClose>` `(-15, +4)` x 0.75 |
| `\|eye - craft\|` | 11.644 | 11.644 | close 11.643, far would be 14.562 |
| `g_camera_fov_degrees` | 67.936 | 67.936 | `60 + 0.075 * dot(fwd, vel)`, the recovered law |
| the setting string `Camera_UpdatePlayerView` returns (`v0` at `0x0883c13c`) | `OPT_CLOSE` | not read | |

Held across the whole GO..GO+130 window on boot 1: the offset stays
`(~0, +3.00, -11.25)` and the distance 11.643-11.647 from a standing start to
110 units/s, so it is the settled close rig and not a spring transient.

**Conclusion.** The original's fresh profile flies **`OPT_CLOSE`**. Our default was
`far` - this project's own choice, documented as "not recovered" because nothing had
read an empty profile - so ours drew the craft with the eye 14.56 instead of 11.64
units back. Every other term agrees: the eye scale (0.75 on both sides: the test
`the_external_blocks_carry_the_crafts_global_scale` pins 11.643), the fov (authored
60 plus the speed term, the same projection), the aspect and the display scale of the
craft. Confidence **88**: two cold boots agree to three decimals and the picture
agrees; the *writer* of the default into a fresh profile (a code literal or a
default parameter of the settings lookup) was not located, which is why it is not
higher. It is corroborated independently by Wipeout 2048's `Options_Definition.xml`
(`CameraP1 default="OPT_CLOSE"`, a different title on the same engine vocabulary) and
by `data/traces/pad0-boost.csv`, whose eye sits 11.64 units from the craft - this page's
own capture section above already said so while the project's default stayed `far`.

**Side by side, native 480x272, original (left) against ours (right), three ticks**
(`side-by-side.png`; GO+60/90/120 against our ticks
343/373/403, which are matched by the standing-start offset and not by tick count -
our standing start is about ten ticks slower): with the default changed the craft's
span on screen agrees at all three, where before it was about 1.4 times narrower
(far 180 px against close 250 px across the craft, enlarged 3x). Left over and not
this lane's: the HUD's green chevron and the song ticker, the exhaust plume's
length and the speed read-out, which differ in the same frames.

**`--camera-view close|far` is not a no-op.** At our tick 403 the two renders differ
(`ours/close.png` against `ours/far.png`, the eye 11.64 against 14.56 units back) and
`each_view_frames_the_craft_from_its_own_block` pins that the three views are three
cameras. The weapons lane's "rendered alike" is not reproduced; the flag is accepted
only under `--screenshot` (`cli.rs` says why), so a windowed run reads the setting
instead. What was real is that the *race start* held the far block whatever
`CameraView::default()` said; both are now the one choice (`chase_block_for`).

**Consequences carried.** `CameraView::default()` is `Close`; a new `settings.toml`
gets `camera_view = "close"`. **A settings file written by an earlier build already
says `far`** (the first run wrote the default out) and stays on it: that cannot be
told from a choice, so it is not migrated. The Wipeout 2048 options picker starts on
`OPT_CLOSE`, as its own definition declares. A comparison against a capture taken on
`OPT_FAR` (the `hull-sparks.md` ones) now needs `--camera-view far`.

## The spectator view's craft-relative modes, and the director's hand-off rules (2026-10-02)

Lane `pulse-spectator-cam`. Read from the decompile and the raw disassembly of
`Camera_UpdateSpectatorView` (`0x08880c04`), and **measured on PPSSPP** (software renderer,
Pulse PSP USA, a Single Race, `scripts/psp-spectator-capture.py`: a breakpoint on the function's
single `jr ra` at `0x08882ae8`, once a frame, logging the camera object, the matrix the view
wrote and the matrix of the craft it read, from the same instant). 407 frames in modes 2 and 3
(the mode word written to 2 and 3 on stretches of the run, and the director's own rolls), 700
frames in all, plus two wreck runs with the camera object logged every frame
(`scripts/psp-wreck-capture.py --camera`).

### The switch, and what the decompile hides

The decompile's `case` labels are right, but the function is long enough that they are easy to
misread, so the jump table is recorded here: `lui at, 0x8a8; lw at, -0x30e8(at)` is
`0x08a7cf18`, nine words:

| Mode `cam+0x1dc` | Target | What it is |
| ---: | --- | --- |
| 0 | `0x08880f68` | a look-at from the station's eye at the craft (not read further) |
| 1 | `0x088810d8` | a craft view of the same family as 2 and 3, with a field of 65 (not read further) |
| **2** | `0x0888132c` | **the rear view, below** |
| **3** | `0x088816e8` | **the front view, below** |
| 4 | `0x08881ba8` | a craft view that uses the same three globals as mode 3 (not read) |
| 5, 6, 7 | `0x08880cb4` | the node cameras ([above](#the-destroy-camera-mode-5-read-and-measured-2026-10-01)) |
| 8 | `0x0888219c` | not read |

**The craft's matrix is `*(entity + 0x794)`**: four rows of four floats, unit length, the rows
**left, up, forward** and then the position. It equals the rigid body's own matrix (the one at
`*(craft + 0x1cc)`) to the bit in every frame captured; no `0.75` scale is on it (that factor
is applied at draw time, see [the 3/4 section](#the-34-factor-is-g_craft_scale-a-code-literal-and-it-is-the-scale-this-project-already-knew)).
The first row is the craft's **left**: `row0 x row1` is `row2`, the nose, so the rows are not a
right-handed `X, Y, Z = right, up, back`. Whether a barrel roll is inside this matrix was not seen
(no roll ran).

The view matrix the function writes (at `[*0x08ab10b0] + 0x40`) stores the camera's three axes as
**columns** and, in the fourth row, the **negated eye** (not the rotated translation), the same
layout [the pose section](#the-live-pose-is-capturable-per-tick-and-the-node-stores-it-transposed)
recorded.

### Case 2: the rear view (mode 2), confidence 93

```text
camera axes = the craft's own                      (right, up, back)
n   = normalize(up + (0, 0.5, 0))                  up = row 1
u   = n / |n . up|                                 (FUN_0897e140 is fabsf)
eye = position + 2.5 * u + 6.0 * back              back = the camera's own third axis
field of view = 65.0 degrees, written every frame (g_camera_fov_degrees)
```

In the original's own rows this is `rows' = (-row0, row1, -row2)`: the code builds a yaw of
`2/PI * PI = 2` quarter turns with `vcos.s` and `vsin.s` (the VFPU's angle unit is a quarter
turn, so `2.0` is 180 degrees), multiplies it onto the craft matrix **in the craft's own frame**
and moves the translation row by `2.5 * u + 6 * row2'`. The result is a camera 6 units **behind**
the craft and 2.5 above it, looking where it flies. `u` is the craft's up tipped half a unit
toward the world's up and rescaled to stay 1 unit along the craft's own up, so a rolled craft's
camera is lifted toward the sky rather than into the track.

### Case 3: the front view (mode 3), confidence 93

```text
camera axes = the craft's, turned 180 degrees about its own up
eye = position + 12.0 * forward + 3.0 * up
field of view = 65.0 degrees
```

The same function turns the matrix by a yaw (`cam+0x344`) and a pitch (`cam+0x340`) first, then adds
`row2 * g(0x08ab10d0) + row1 * g(0x08ab10cc) + row0 * g(0x08ab10c8)`: **12.0, 3.0 and 0.0**, read from
the file and from the running emulator (`log["globals"]`). The camera sits **12 units ahead** of
the craft and 3 above it, looking back at it. Both globals are read only by this function (cases 3
and 4). **`cam+0x340`/`+0x344` are `0.0` in the director**: the constructor zeroes both
(`0x0887fca8`), and the only writers are `FUN_08814014` (called from `InGame_UpdatePauseInput`
`0x08813244`) through `FUN_088808a8` (pitch) and `FUN_08880918` (`yaw += dt * 0.05`, wrapped at
`+-2 pi`): the pause menu's look-around, which a spectating player does not touch. Neither
function is a callee of the view and neither is renamed here.

**The previous lane's names were swapped by guesswork** (`above` for 2, `front` for 3): by geometry
2 is the close rear view and 3 is the front view. Neither was ever above the craft.

**Measured**: for every frame in mode 2 and 3 the rotation of the written matrix equalled the
prediction exactly and the eye to `5.7e-5` units (a throwaway script, not kept,
407 frames, 199 in mode 2 and 208 in mode 3), with the craft matrices read live, including banked and
pitched ones (`up.y` down to 0.86). `oag_render::camera::craft_view` carries the two most banked
frames as tests. Frames: `m.png` (mode 2 on top, mode 3
below).

### What `cam+0x1e8` is, and what the director cuts by

Cases 0, 2 and 3 end with `cam+0x1e8 = FUN_0883e434(cam+0x1e4)`, which is
`*(char *)(*(entity + 0xae4) + 0x60)`: a **signed byte that counted up 12, 13, 14, ... 19 as the
craft advanced** along the circuit (about one step per 40 to 60 frames, 700 frames), and equals
the value the function wrote back each frame (`node_byte` in the log). It looks like the craft's current
track section; the structure `entity+0xae4` points to was not read, so this is a **hypothesis** and
`FUN_0883e434` is not renamed. **It is not a cut input**: `Camera_UpdateSpectator` cuts by `cam+0x1d4`, the
current station, whatever the mode is, and clears `+0x1e8` to `-1` with the ten second timer (the same
frame it is written again).

### The director's own rules, read in full

`Camera_UpdateSpectator` (`0x0887fd3c`):

1. `cam+0x3c += dt`, and bit 4 of the player's scene node flags (`*(player + 0x8b0) + 0x2c`) is set.
2. Past `10.0` s: the timer is zeroed, `cam+0x1e0` (subject) is cleared unless `cam+0x274` is set, and
   `+0x1e8 = -1`.
3. **If the subject is in state 6 and its timer `entity+0x874` is `<= 0`, the subject is cleared.**
4. A cleared subject is re-picked by `Camera_PickSubject` (below).
5. With no station (`cam+0x1d4 == 0`) it takes the nearest (`Camera_PickStation`) and sets the
   drawn craft (`+0x1e4`) to the subject, **without rolling a mode**. With a station it runs the 60-unit test on
   the **subject's** position: if a new station was taken, **`Camera_PickRandomMode`, and
   `+0x1e4 = +0x1e0`**; otherwise it tries again from the *drawn* craft's position and ignores the result (the
   station may change, the mode and the drawn craft do not).
6. `Camera_UpdateSpectatorView`.

**The craft drawn is `+0x1e4`, not the subject `+0x1e0`**: every case reads `*(+0x1e4) + 0x794`, and the
subject only becomes the drawn craft at a cut. Seen: at view frame 599 of the capture the ten second timer
re-picked the subject and a cut took it at the same frame (`subject` and `previous` both change in the log); in
the wreck run the subject changed at `k+240` and the drawn craft at `k+279`.

`Camera_PickSubject` (`0x08880a58`), read in full: with `cam+0x274 == 0` and a race manager, the subject is
the **player** (`manager+0x2c0`); if the drawn craft carries `flags(+0x860) & 0x1000` the drawn craft is
reset to the player; then, while the subject equals the drawn craft and more than one craft is live, a random
live craft is taken (`rand() % count`). With `cam+0x274 != 0` the subject is the global `DAT_08ab0df8` (a photo
mode or a replay, not seen). **`Camera_PickRandomMode` (`0x08880b38`) is gated on `cam+0x26c`** (a byte, `1`
from the constructor): when it is zero the function changes nothing.

### After a player wreck the camera stays in mode 5 for good, confidence 92

`Ship_SetState` case 4 (`0x08844508` to `0x08844524`): `Camera_SetSubject(cam, entity, 1)`,
`cam+0x2c |= 6`, **`cam+0x26c = 0`** (`sb a2, 0x26c(a0)` with `a2 = 0`, `0x08844520`), then
`Camera_SetMode(cam, 5)`. With the flag clear `Camera_PickRandomMode` does nothing, so the cuts that follow
still move the camera between nodes but never out of mode 5 (view width 35). That is why the
original "stays in mode 5" where the port started its director in mode 7. The other writers of
`+0x26c` are `FUN_088dfb90` (the griefing report, not a race ending) and two callers in `0x08966xxx` (not read).

**What triggers the hand-off is the wreck's own timer, not a constant** (`wreckA`, 2026-10-02: wreck injected
at race time 1.02 s, ten camera fields logged per frame; the earlier `wreck2` run agrees to the frame):
state 4 for 30 frames, state 5 for 90, state 6 for 120: `entity+0x874` crosses zero at **`k+240`**, the same frame
`cam+0x1e0` changes to another live craft. The drawn craft stays the wreck until the next cut: `k+279` in this
run (the new subject first had to be more than 60 units from the node's aim point and a node within 60 units of
it had to exist), `k+290` in the earlier one. **The "259 ticks" the end-photo lane quoted is a single sample of
the cut delay**, not a rule; the fixed rule is `k+240` for the subject and the first qualifying cut for the picture.
The ten second timer was zeroed at `k` (`timer3c 0.000`) and reads `4.0` at `k+240`, so the next
re-pick is at `k+600`.

### Names

No function is renamed here. `Camera_UpdateSpectatorView` goes `85 -> 90` (cases 2, 3 and the exit
measured; cases 0, 1, 4 and 8 still unread), `Camera_UpdateSpectator` `70 -> 80` (read in full, its timer
and cut rules seen live) and `Camera_PickSubject`, `Camera_PickRandomMode` `80 -> 85` and `75 -> 85` in
[`race-finish.md`](race-finish.md). Three data names, the row scales case 3 and case 4 read (static bytes in
the executable, and live):

| Address | Name | Value | Confidence |
| --- | --- | ---: | ---: |
| `0x08ab10c8` | `g_spectator_row_scale_left` | `0.0` | 85 |
| `0x08ab10cc` | `g_spectator_row_scale_up` | `3.0` | 85 |
| `0x08ab10d0` | `g_spectator_row_scale_forward` | `12.0` | 85 |

Not read: cases 0, 1, 4, 8; `cam+0x274`; the structure at `entity+0xae4`; whether a barrel roll is in
`+0x794`; what `FUN_088dfb90` does to the camera; the PS2's version of any of this.
