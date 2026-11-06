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
| Pulse PSP | yes | Pause (`EXT_PHOTO`, single-player screens only); SELECT on `Race End Photo` | 480x272 JPEG, quality 90, `/PICTURE/WIPEOUT_PULSE/` | 84 |
| Pulse PS2 | no save path | See [below](#pulse-ps2) | none | 70 |
| Pure | **no** | none | none | 90 |
| HD/Fury | yes | Pause (`MSC_PHOTO_PHOTOMODE`); SELECT at the finish | JPEG at display resolution, quality 90, exported to the XMB | 84 |
| 2048 | two | Pause: `InGame VitaPhoto` (touch); legacy blocks `InGame Photo` (pad) | JPEG, `ScePhotoExport` | 84 |
| Omega | yes, cut down | Pause; OPTIONS at the finish | The console's Share function, no file of its own | 80 |

## Pure has none

Confidence 90. No string containing `photo` (any case) in `BOOT.BIN`, EU or USA, nor `.jpg`. All
1,241 entries of `Data.wad`, `FE.wad`, `FEData.wad` of the EU disc, decompressed and searched: no
`photo` in any, including the nine language files (`Entry ID=` count 845 to 909 each), and
`InGame_Definition.xml` has `InGame Pause`, no photo screen. Pure's PSP save icon strings (`ICON0.PNG`,
`PIC1.PNG`) are save data. Short of 94: Pure was not played for it. Pulse's `EXT_PHOTO` is in the
full-version advert in the Pulse language file ("The full version has loads of new features
including ... photo mode"), which is Pulse's own demo text and says nothing of Pure.

## Pulse

Screens, all in `Data\Plugins\PI001\GUI\InGame_Definition.xml` (`31b50f2e`), `Photo Mode Top Level`:

| Screen | Task | Purpose |
| --- | --- | --- |
| `InGame Photo` | `Pause` | The two rows and the legends |
| `InGame Taking Photo` | none | Empty, so no menu is in the shot; 30 frames, then the save |
| `InGame Taking Photo Saving` | `Pause` | `TRC_SAVING`, waits on the writer thread |
| `InGame Photo Message` | `Pause` | Result dialog, `FE_CONTINUE` back to `InGame Photo` |
| `InGame Race End Photo` | `Pause` | An unused prompt: `ER_PHOTO_TAKE2`, redirect to `InGame Taking Photo` |

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

Pitch `+= stick * 0.05` per frame, clamped to **`[-1.0, 0.6]`** radians. Yaw `+= stick * 0.05`, free,
wrapped at `+-2 pi`. Confidence 84, one binary. Reset on a mode change into close or front, and on
leaving.

**There is no free camera on Pulse.** The eye is a rigid offset from the craft turned by two angles,
or an authored node, so nothing in `PhotoMode_Update` asks the track where the eye may go, and the
stick cannot leave the orbit radius. `+0x270` feeds `Camera_FramingFov`'s view width, so "zoom" is
field, not position.

## HD/Fury

Rows, ranges and laws: [the HD page](../ghidra/functions/ps3-hdfury-eu/photo-mode.md#photomode_update).
Screen tree as Pulse's, plus `InGame Taking Photo Transition Screen`, `Pause`, `delay="1"`, so the
menu is gone a second before the shot. Rows, top to bottom: camera, HUD, speed blur, exposure,
saturation, effects, then aperture, focus or shutter for the chosen effect.

`Exposure` and `Saturation` open on the race's own grade whatever the XML says; `Saturation`'s neutral
is 20%. Motion blur effects appear only in race phases 2 and 3. A depth of field shot accumulates 7
more frames, a motion blur shot 120. The camera can pan `+-5` and rotate freely, with no track test
in the code that was read.

## 2048

Two modes, HD's pad mode and a touch mode with three effect toggles. `MSC_PHOTO_CANCEL` is the Glory
Moment (a front-camera picture sent to racers), not photo mode; `MSC_PHOTO_CHANGE_CAMERA` and
`MSC_PHOTO_CYCLE_*` are text on the help image. Evidence:
[the 2048 page](../ghidra/functions/vita-2048-eu-v104/photo-mode.md).

## Omega

**Checked, differs** against 2048 and HD
([page](../ghidra/functions/ps4-omega-eu/photo-mode.md)). HD's screen, losing speed blur and
saturation and gaining a separate motion blur list, and the shot is the console's Share. Not run:
shadPS4 has no memory readout, so nothing here has a live number.

## Pulse PS2

`SCES_547.48` holds `InGame Photo`, `InGame Taking Photo`, `InGame Taking Photo saving` and `Photo Mode`
in the camera gate shape, and no `ER_PHOTO_*` state strings and no `.jpg`. PS2 has no save path in its
binary; its XML was not read. Confidence 70. Inherited from PSP if a PS2 port wants it.

## Split screen and multiplayer

Pulse offers none (the row is absent from `InGame Pause MP`). HD's `Block name="Photo"` occurs in pause
blocks marked `InputPad="1"`, and `PhotoMode_Update` reads `g_GameState+0xe4`/`+0xe5` for the paused
pad, so HD allows it in a local split. See the live section.

## Live

Pending.
