# Photo mode

What each title's photo mode is, read off its executable and its data (2026-11-06, lane
`photo-read`). Scores follow the [rubric](../reverse-engineering/confidence-rubric.md). The
functions and their evidence are on each binary's page:
[Pulse EU](../ghidra/functions/psp-pulse-eu/photo-mode.md),
[HD](../ghidra/functions/ps3-hdfury-eu/photo-mode.md),
[2048](../ghidra/functions/vita-2048-eu-v104/photo-mode.md),
[Omega](../ghidra/functions/ps4-omega-eu/photo-mode.md).

## By title

| Title | Has one | Entry | Shot | Conf. |
| --- | --- | --- | --- | ---: |
| Pulse PSP | yes | Pause (`EXT_PHOTO`, single-player screens only); SELECT on `Race End Photo` | 480x272 JPEG 4:4:4, quality argument 90, `/PICTURE/WIPEOUT_PULSE/` | 90 |
| Pulse PS2 | no save path | See [below](#pulse-ps2) | none | 70 |
| Pure | **no** | none | none | 90 |
| HD/Fury | yes | Pause, including split screen; SQUARE on `Race End Photo` | JPEG at display resolution (1920x1080 measured), quality argument 90, exported to the XMB | 90 |
| 2048 | yes, its own | Pause and `Race End Photo`: `PhotoModeTouchButton` to `InGame VitaPhoto` | JPEG at the display surface, `ScePhotoExport` | 84 |
| Omega | yes, cut down | Pause; SQUARE at the finish per the XML, the string says OPTIONS | The console's Share function, no file of its own | 80 |

## Pure has none

Confidence 90. No string containing `photo` (any case) in `BOOT.BIN`, EU or USA, nor `.jpg`. All
1,241 entries of `Data.wad`, `FE.wad` and `FEData.wad` of the EU disc, extracted and searched: no
`photo` in any, including the language files (nine files with `Entry ID=`, up to 909 rows each).
`InGame_Definition.xml` has `InGame Pause` and no photo screen. Short of 94: no Pure pause menu was
walked for this, and the USA disc's archives were not searched, only its binary.

## Pulse

Screens, all in `Data\Plugins\PI001\GUI\InGame_Definition.xml` (`31b50f2e`), `Photo Mode Top Level`:

| Screen | Task | Purpose |
| --- | --- | --- |
| `InGame Photo` | `Pause` | The two rows and the legends |
| `InGame Taking Photo` | none | Empty, so no menu is in the shot; 30 frames, then the save |
| `InGame Taking Photo Saving` | `Pause` | `TRC_SAVING`, waits on the writer thread |
| `InGame Photo Message` | `Pause` | Result dialog, `FE_CONTINUE` back to `InGame Photo` |
| `InGame Race End Photo` | `Pause` | A prompt, `ER_PHOTO_TAKE2`, then `InGame Taking Photo`. No `goto` to it in this file, and only the camera gate names it in code |

**Entry from pause.** `InGame Pause SP`, `SP Time Trial`, `SP Speed Lap`, `SP Tournament` carry
`EXT_PHOTO` as row 3 and `goto="InGame Photo"`. `InGame Pause MP` and `MP Tournament Host` do not:
no photo mode in multiplayer. `GSDisableEntriesBitField="0x04"` strikes the row under Game Share.

**Entry from the finish.** `Race End Photo`'s `Race End Photo Redirect`, `forward="select"`, default
`InGame Photo`, `GSDisable="1"`. The redirect's task flips from `run` to `Pause`.

Rows:

| Row | Values | Default |
| --- | --- | --- |
| `Photo Camera Mode` (`ER_PHOTO_CAM`) | `OPT_INT` internal, `OPT_CLOSE` close, `ER_PHOTO_CAM_FRONT` front, `ER_PHOTO_CAM_ABOVE` above, `ER_PHOTO_CAM_TRACK` track | internal |
| `Photo Hud Display` (`ER_PHOTO_HUD`) | `FE_OFF`, `FE_ON` | `FE_OFF` |

Controls: CROSS takes (the redirect's forward), CIRCLE leaves. The camera:

| Row | Camera | Stick, no digital button held | Digital |
| --- | --- | --- | --- |
| internal | Mode 1: 5 ahead of the nose, looking forward | none | |
| close | Mode 4: 12 behind, 3 above | pitch and yaw | Triangle resets both |
| front | Mode 3: 12 ahead, 3 above, facing back | pitch and yaw | Triangle resets both |
| above | Mode 2: 6 behind, 2.5 above (`camera.md`: "above" is not above) | none | |
| track | Mode 7: the authored `Camera` node nearest the craft | X zooms `+0x270`, `[3, 40]` | L `-5`, R `+5` |

Pitch `+= stick * 0.05` per frame, clamped to **`[-1.0, 0.6]`** radians; stick up is negative, lifting
the eye. Yaw `+= stick * 0.05`, free, wrapped at `+-2 pi`. Confidence 90, live. Reset on a mode change
into close or front, and on leaving.

**There is no free camera on Pulse, and the track does not limit it.** The eye is a rigid offset from
the craft turned by two angles, or an authored node. `|eye - craft|` read 12.3693 at every pitch and
yaw, and at pitch `0.6` the eye sits under the road (the frame shows the hull from below). Confidence 90.
Track "zoom" is the field of view: `+0x270 = z` draws `z + 0.06 * (25.1 - z)` degrees at 112 units
(six values matched; the target is the node framing).

## HD/Fury

Rows, ranges and laws: [the HD page](../ghidra/functions/ps3-hdfury-eu/photo-mode.md#photomode_update).
Screen tree as Pulse's, plus `InGame Taking Photo Transition Screen`, `Pause`, `delay="1"`, so the
menu is gone a second before the shot. Rows, top to bottom: camera, HUD, speed blur, exposure,
saturation, effects, then aperture, focus or shutter for the chosen effect.

| Row | Range, step | Opens on |
| --- | --- | --- |
| Camera | internal, close, track | internal |
| HUD | off, on | off |
| Speed blur | 0 to 100, 5 | 0 |
| Exposure | 0 to 100, 1 | `(int)(33.3 - 33.3 g)`, `g` the race's grade: 21 on Vineta K, live |
| Saturation | 0 to 100, 1 | 20, which is neutral; live |
| Photo effects | off, depth of field; motion blur track and ship in race phases 2 and 3 | off |
| Aperture, focus | 0 to 100, 1 | 40, 20 |
| Shutter speed | 0 to 100, 1 | 20 |

A depth of field shot accumulates 7 more frames, a motion blur shot 120. `close` orbits freely on two
axes, tilts, zooms `[10, 100]` from 65, pans `+-5` (pan static only); `track` zooms `[3, 40]` from 20
and tilts; L1, R1 change the craft. Confidence 90 for the rows that were read live, 84 for the rest.

## 2048

A touch screen, `InGame VitaPhoto`, with three effect toggles (motion blur track, motion blur ship, depth
of field) and a preview before saving. HD's `InGame Photo` code remains in the binary, unreachable.
`MSC_PHOTO_CANCEL` is the Glory Moment, a front-camera picture sent to other racers, not photo mode.
`MSC_PHOTO_CHANGE_CAMERA` and `MSC_PHOTO_CYCLE_*` are text on the help image: the camera toggle is
button 5 between close and internal, the cycle is L and R. Its depth of field law differs from HD's.
Evidence: [the 2048 page](../ghidra/functions/vita-2048-eu-v104/photo-mode.md).

## Omega

**Checked, differs** against 2048 and HD
([page](../ghidra/functions/ps4-omega-eu/photo-mode.md)). HD's `InGame Photo`, not 2048's touch screen,
losing speed blur and saturation and gaining a separate motion blur list, and the shot is the console's
Share. Not run:
shadPS4 has no memory readout, so nothing here has a live number.

## Pulse PS2

`SCES_547.48` holds `InGame Photo`, `InGame Taking Photo`, `InGame Taking Photo saving` and `Photo Mode`
in the camera gate shape, and no `ER_PHOTO_*` state strings and no `.jpg`. PS2 has no save path in its
binary; its XML was not read. Confidence 70. Inherited from PSP if a PS2 port wants it.

## Split screen and multiplayer

Pulse: none, the row is absent from `InGame Pause MP` and `MP Tournament Host`. HD: **yes**, the `Photo`
block is on `InGame Pause Split Player 1` and `2`, the tournament pair and the Zone pair, from either
pad. Read, not run. Omega: same blocks, same screens. 2048: not checked in a split.

## What entering stops

**Pulse, measured (PPSSPP, Pulse USA, 90).** In `InGame Photo`, `Ship_UpdateCraft` and
`RaceManager_Update` take no hits, the craft stands to the bit, and only `Camera_UpdateSpectatorView`
runs. The same holds for `InGame Pause`, so photo mode is the pause plus a camera. The race resumes
where it stopped. Numbers: [Pulse EU, Live](../ghidra/functions/psp-pulse-eu/photo-mode.md#live-ppsspp-pulse-usa).
**Audio stops with it.** The engine note, the cue sequencer and the music callback take no hits in pause or
photo mode and the SAS voices stand still, while the mixer keeps ticking (80).

**HD, measured (RPCS3, 90).** Same: the craft stands to the bit, Continue resumes. Audio not measured.
[HD, Live](../ghidra/functions/ps3-hdfury-eu/photo-mode.md#live-rpcs3).

**Neither title's camera stops at the track.** Pulse: 12.3693 from the craft at every angle, under the
road at pitch `0.6`. HD: at orbit 1.34 the frame looks up through the road. A port needs no collision
to be faithful.

**Omega: not checkable** live, shadPS4 has no memory readout.

## What ours does

One photo mode for every title, on by default (Pure included, which has none in the original).
Code: `oag_raceplay::photo` (the camera), `oag_game::photo` (the screen and the files),
`oag-game`'s `session::photo` (the session half). Everything here that is not a number measured
off an original is **chosen, not measured**; where Pulse's and HD's laws differ the wider one is
taken, and a value one title measured is used on the rest unchanged.

**Entry.** The pause menu's PHOTO MODE row, which is the front end's main page over the race
parked behind it (Escape, or Back on a phone): the row is parked off the page unless a race with
one view is held, as a choice with nothing to offer is (`Menu::offer`). The built-in mod
`oag-extras` attaches it, so it is on every title's main page (after OPTIONS' own rows; on HD and
Omega's strip it is the tab right of OPTIONS). On Pulse's `Race End Photo`, SELECT opens it once
the screen takes input (61 ticks after the finish), and leaving returns to the screen. Left out:
split screen (no single picture, no single pad: a follow-up), a LAN or online race (the server's),
and a build that cannot write a file (a page, a phone).

**While it is open** the race does not tick (`oag_game::photo::ticks`), whatever state it is in,
so its state hash does not move and the race resumes where it stood; the audio is stepped as under
the pause. The camera is a render-side override (`CameraOverride`), put back on leaving. Leaving spends
Circle, Cross, Start, Select and the shoulders, so none reaches the race. Escape (and Back, and the
window losing focus, which does nothing then) leaves the mode and goes no further; the next Escape
opens the menus.

**Lenses** (the camera row): `internal` (5 ahead of the nose, Pulse's measured), `close` (12 behind
and 3 above, free to orbit; Pulse's measured offset, **chosen** on the others), `track` (the
circuit's authored `Camera` node nearest the craft, aimed at it; only where a circuit authors
one, left out of the cycle otherwise). The mode opens on `internal`, as both originals do.

**Controls**, all on one stick and the buttons a keyboard has (the abstract input has one stick):

| Input | Does |
| --- | --- |
| stick, arrow keys | orbit `close` (yaw and pitch, each free, wrapped at +-2 pi, 0.05 rad a tick: HD's law); zoom and tilt `track` |
| SELECT held + stick | `close`: zoom (up and down, field 10 to 100 from 65) and tilt (left and right) |
| SQUARE held + stick | `close`: pan, +-5 |
| SELECT tap | next lens |
| SQUARE tap | HUD on or off (off on opening) |
| L, R | previous, next craft, wrapping, skipping craft out of the race |
| TRIANGLE | reset the orbit, tilt, pan and fields |
| START | hide or show the help |
| CROSS | take the shot (refused for 0.15 s after opening: HD's guard) |
| CIRCLE, right click | leave |

A click on the four chips along the top (camera, HUD, TAKE PHOTO, BACK) does the same, and a click
with the help hidden shows it. Not ported: HD's separate tilt triggers (L2, R2 are the airbrakes'
axes here), the right-stick zoom, the touch gestures of 2048 (a follow-up).

**The shot.** The frame the player sees, at the size they see it (`Framebuffer::output`, the
presentation target: the picture at window size, **before** the display grade and anything drawn
over the surface), written as a PNG to `photos/` under the user's data directory
(`<data dir>/oag/photos/oag-<title>-<unix seconds>-<n>.png`, `n` making the name unique). The
screen's text is drawn over the surface, so it is not in the file; the race's HUD is, only when
its row is on, and the end screens and track panel never are. The result message ("PHOTO SAVED")
shows and the mode stays open on the same camera. `crates/game/tests/photo_mode_ground_truth.rs`
checks the file is pixel-identical to a `--drive` screenshot of the same tick with the help
hidden, and of the same size.

**Words.** One id per row, `OAG_PHOTO_*` in `assets/ui/strings/english.toml`; where a title's disc
words a row the disc's own text is played instead (`oag_title::photo::Strings`: Pulse's
`ER_PHOTO_CAM` and `ER_PHOTO_HUD`; HD's and Omega's `MSC_PHOTO_HUD`, `_HELP`, `_ROT`, `_PAN`,
`_RESET` and `_SHIP`), checked to be in each disc's string table by the ground-truth test. The
rest is ours, with French named under `[untranslated]`.

**Checked against the other lineage.** 2048 and Omega both run this mode (driven to a race, the
mode opened from the pause menu, orbited, the HUD row on, a shot taken and read): ported, with
`photo_mode_ground_truth` on Pulse and a `--drive` walk on each of the five titles. `Race End Photo`
exists on Pulse only in this build (HD's end-race flow does not open on it), so SELECT at the finish
is Pulse's alone. 2048's own touch screen,
its preview step and its Glory Moment are not used.

**Left open:** exposure, saturation, speed blur,
depth of field and motion blur (`oag-post`, `photo-effects`), the photo counter stat, split screen,
touch gestures, saving on a page or a phone, HD's real `close` rest offset, and a shot at the
scene's own pixels (the file is the presentation target, upscaled from the scene drawn at its
configured render size: the dynamic resolution is held there while the mode is open).
