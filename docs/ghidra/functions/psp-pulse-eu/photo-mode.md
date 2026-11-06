# Pulse PSP EU: photo mode

Read 2026-11-06, lane `photo-read`, `/pulse/BOOT-psp-pulse-eu.BIN`. The player-facing law, with
the other titles beside it, is [docs/ui/photo-mode.md](../../../ui/photo-mode.md). This page is
the evidence.

The USA build carries the same code at fixed offsets: `FUN_08814014` is `0x08813f24 + 0xf0`,
`FUN_088808a8` and `FUN_08880918` are `0x08880704`/`0x08880774 + 0x1a4` (`FUN_088808a8` decompiled
in the USA program and matched, constants included), and `FUN_08880788`
(`race-finish.md`, "Not read") is `0x088805e4 + 0x1a4`.

## Functions

| Address | Name | Conf. | What it does |
| --- | --- | ---: | --- |
| `0x08813f24` | `PhotoMode_Update` | 84 | The whole mode, once per frame, dispatched on the front end's state name. Sole caller `InGame_UpdatePauseInput` (`0x08813154`), first line |
| `0x08880704` | `Camera_AddPhotoPitch` | 84 | `cam+0x340 = clamp(cam+0x340 + stick * 0.05, -1.0, 0.6)` |
| `0x08880774` | `Camera_AddPhotoYaw` | 84 | `cam+0x344 += stick * 0.05`, wrapped by `2 pi` at `+-2 pi` |
| `0x088805e4` | `Camera_FollowPlayer` | 78 | `Camera_SetSubject` on `*(DAT_08b2f734 + 0x2c0)`, then `cam+0x1e4 = cam+0x1e0` |
| `0x0883b0d8` | `Camera_UpdatePlayerViewUnlessPhoto` | 78 | Skips `Camera_UpdatePlayerView` while the state is any of the five photo states |
| `0x0891e788` | `PhotoMode_BeginJpegSave` | 80 | Encodes the display buffer, checks free space, opens the file, starts the writer thread |
| `0x08936838` | `JpegWriter_Construct` | 84 | Thread `JpegWriterThread`; JFIF header, SOF0, three components sampled `0x11`; width, height, quality |
| `0x08935850` | `JpegWriter_Encode` | 84 | 8x8 blocks over `+0xeae`/`+0xeb0`, ends `ff d9` |

`Input_GetAxis` is `0x0894ec44` here (`0x0894f198` on USA, `input.md`). `Camera_SetMode` is
`0x08880580`.

## `PhotoMode_Update`, by state

`cam` is `DAT_08b30be4`, the spectator camera. `DAT_08ab0868` is the race's own camera,
`DAT_08ab0858` a third object tested at `+0x2c & 2`. `this+0x44` is a tick counter, `this+0x48`
the last camera row. The whole function is gated on `DAT_08aafda8 != 0`.

**`InGame Photo`, circle (`Input_IsPressed(g_input, 4)`)**: consume, find redirect
`InGame Taking Photo Redirect`, read the screen to return to at `redirect+0x223`. Back to
`Race End Photo` or under `g_game_mode 0xc`: the spectator camera stays (`cam+0x2c |= 2`, the race
camera's bit cleared) and is updated once. Otherwise the race camera is switched back on. Either
way `cam+0x340 = cam+0x344 = 0`, `g_settings+0x45b = 1`, `StateMachine_TransitionTo`.

**`InGame Photo`, no circle:**

1. `*(DAT_08aaffb8 + 0x121) = (Photo Hud Display == FE_ON)`. Default `FE_OFF`.
2. Unless `DAT_08ab0858+0x2c & 2`: clear the race camera's bit 2, set `cam`'s, then re-subject:
   `Camera_FollowPlayer` when `cam+0x1e4 == 0` or (`DAT_08aaff63` or mode not `0xc`, and
   `DAT_08b2f734+0x7c8 != 3`), else `FUN_088803e8(cam, cam+0x1e4, 0)`. So from a race the subject
   is the player, every frame; from a finish the craft the d-pad chose stays.
3. `Photo Camera Mode` picks the `Camera_SetMode` value:

   | Row `value` | String | Mode | Pitch and yaw | Zoom |
   | --- | --- | ---: | --- | --- |
   | `internal` | `OPT_INT` | 1 | no | no |
   | `close` | `OPT_CLOSE` | 4 | yes | no |
   | `front` | `ER_PHOTO_CAM_FRONT` | 3 | yes | no |
   | `above` | `ER_PHOTO_CAM_ABOVE` | 2 | no | no |
   | `track` | `ER_PHOTO_CAM_TRACK` | 7 | no | yes |

   Changing to `close` or `front` from another row zeroes pitch and yaw; `above`, `internal` and
   `track` only store `this+0x48`.
4. `track` shows `ER_PHOTO_ZOOMCAM`. With no digital button held this frame or last
   (`input+0x3c`, `+0x40` low 16 bits zero): `cam+0x270 = clamp(cam+0x270 + Input_GetAxis(0), 3, 40)`.
   Otherwise L (8) is `-5` and R (9) `+5`, same clamp, on the press edge.
5. `close` and `front` show `ER_PHOTO_ROTATECAM` and `ER_PHOTO_RESETCAM`. With no digital button:
   `Camera_AddPhotoPitch(axis 1)` then `Camera_AddPhotoYaw(axis 0)`. Otherwise triangle (6), on
   the press edge, zeroes both.
6. The camera's own update (`vtable+0x24`) runs, so the view is live while the race is paused.

**`InGame Taking Photo`**: `this+0x44` counts up and at `> 0x1d`, so on the 30th frame:

1. `sceIoDevctl`-shaped call `0x2425824` at `DAT_08a78e20`: result 1 is `TRC_LOCKED`.
2. No stick (`DAT_08abfa8c == 0`): `TRC_NO_MS`.
3. Creates `msroot:/PICTURE/` and `msroot:/PICTURE/WIPEOUT_PULSE/` as needed.
4. Sleep flag `DAT_08abfef2`: `ER_PHOTO_SLEEP`, to `InGame Photo Message`.
5. File name `%swo_%d%02d%02d_%02d%02d%02d.jpg`: the directory, then year, month, day, hour,
   minute, second off the clock (`0x08a76624`, an import stub).
6. `PhotoMode_BeginJpegSave`. `0`: arm the save icon (`FUN_0891eb9c(224, 120, 32, 32)`, a 32x32
   rectangle on screen centre) and go to `InGame Taking Photo Saving`. `1`: `ER_PHOTO_SPACE`, needed
   KB is `bytes / 1024 + 1`. `2`: `ER_PHOTO_RESOURCE`. `3`: `TRC_NO_MS`. Those go to
   `InGame Photo Message`.

**`InGame Taking Photo Saving`**: waits for `writer+0xed2 == 0`. `writer+0xed3` is success:
`TRC_SAVE_OK`, and `*(g_settings + 0x224) += 1`, the count behind `PRO_STATS_PHOTO`, `MSC_DATA_PHO`
and `TKR_PHOTOS`. Failure is `TRC_NO_MS` or `TRC_MS_INSUFKB2`.

## The saved file

`JpegWriter_Construct` is called once, from `FUN_0891edfc`, the renderer's constructor, at
`0x0891f32c`: `a1 = 0x1e0`, `a2 = 0x110`, `t0 = 0x5a`. **480 x 272, baseline JPEG, quality 90, 4:4:4**
(all three components `0x11`, marker `ff c0`). `JpegWriter_Encode` reads the buffer `0x88000` into
the display pair (which of the two by `+0x5df0`) with stride 512, 4 bytes a pixel: `0x88000` is
`512 * 272 * 4`, so it encodes what was displayed, with whatever the HUD flag let through.
Confidence 84.

`PhotoMode_BeginJpegSave` compares the encoded size against free space
(`FUN_0893f1d8`, `sceIoDevctl 0x2425818`), opens with mode `0x2202`, copies the bitstream to a heap
block and hands it to the thread. It then **copies `0x88000` bytes back over the working buffer**,
because the encoder used the second half of the frame pair as scratch.

## History

- 2026-11-06: all rows new, static.
