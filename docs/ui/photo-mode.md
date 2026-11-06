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
