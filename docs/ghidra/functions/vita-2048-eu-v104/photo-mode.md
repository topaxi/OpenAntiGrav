# 2048: photo mode

Read 2026-11-06, lane `photo-read`, `/2048/eboot-vita-2048-eu-v104.elf`. Player-facing law:
[docs/ui/photo-mode.md](../../../ui/photo-mode.md). 2048 carries **two** modes and an unrelated
camera feature; `InGame_Definition.xml` is patch `data1.psarc`'s copy (43,418 bytes).

## Functions

| Address | Name | Conf. | What it does |
| --- | --- | ---: | --- |
| `0x8112e6bc` | `PhotoMode_ScreenEnter` | 84 | HD's `0x0023e570`: same string order, same effect-list rule |
| `0x8112ea02` | `PhotoMode_Update` | 84 | HD's `0x0023f568` with touch gestures added |
| `0x813d6048` | `VitaPhoto_Construct` | 80 | `Frontend/Screens/WV_PhotoMode_Screen.cpp`, screen type `Vita Photo Mode` |
| `0x81146898` | `VitaPhoto_Update` | 84 | Eight-state capture machine on `this+0x160` |
| `0x812ad888` | `RaceManager2048_Construct` | 78 | `GameMode_RaceManager.cpp`; binds `gloryImage`, `CancelPicture`, `TakePicture` |

`0x81145efa` and `0x81146898` had no function in the database; both were created (a vtable at
`0x81510b48` points at them) and `0x81145efa` is unread. The two strings at `0x81145f9c` and
`0x81146a78` sit in those.

## `InGame Photo`, the pad mode

Reachable from the legacy pause blocks (`Block name="Photo"`, `goto="InGame Photo"`); the same rows
and laws as HD: same `0.3, 0.04` stick response, `0.15` s guard, same depth of field, same
`[3, 40]` track field starting 20, `FE_MOTION_TRACK`/`FE_MOTION_SHIP` compares, rebuilt list adding
`MSC_PHOTO_MBT`/`MSC_PHOTO_MBS` on phase 2 or 3 (`RaceManager +0x11d50`). What 2048 adds in `close`:

- Touch drag, `+0x376`/`+0x378` times `0.1`, clamped to `+-1`, drives orbit.
- Two fingers, `+0x38a` to `+0x398`: their angle times `15` rolls, the change in distance times
  `0.035` zooms.

## `InGame VitaPhoto`, the touch mode

Pause menus carry `PhotoModeTouchButton` with `redirect="InGame VitaPhoto"`. Screen `Pause`.

Effects are three toggles, one at a time. Sliders are `TouchSlider`s, min 1, max 100:

| Button | Slider, default | Law at the shot |
| --- | --- | --- |
| `Motion Blur Track Button` | `Shutter Speed`, 20 | `DAT_81898344 = (v * 0.01)^2 * 0.25`, `0x82` frames, camera delta |
| `Motion Blur Ship Button` | `Shutter Speed`, 20 | same value `* 0.05`, `0x82` frames |
| `Depth of Field Button` | `DOF Aperture` 30, `DOF Focus Distance` 19 | `7` frames, `DAT_81898334 = (a * 0.01)^2 * 900 + 100`; live focus `((f - min)/(max - min))^3 * 994.5 + 5.5` |

No effect: live `18.0` for the blurred states. **Depth of field has a different law from the pad
mode's**, on purpose or not.

State `this+0x160`:

| State | Meaning |
| ---: | --- |
| 0 | Idle. `Take Photo Button`: `FUN_81145e04(0)` hides the UI, then 2 with an effect or 4 without. With an effect on and no input for `2.5` s (`0.2` stick threshold) the capture starts by itself |
| 1 | Fills the job at `DAT_81898330`, `FUN_81021992(job, 0)`, then 3 |
| 2 | Three frames of settling, then 1 |
| 3 | Accumulating; any input or a tap abandons to 0; done goes to 4 |
| 4 | Preview, UI shown. Tap or input with `this+0x110 < 0` returns to 0; `Take Photo Button` to 5 |
| 5 | Frame 3 with no effect: `FUN_81022da4(1)`. Frame 6: writes, sound `NGP_Camerashot`, to 6 |
| 6 | Waits on the writer `DAT_8151f0c4+0xf01`; result 0 is `TRC_SAVE_OK` and stat `+0x3239c` `+= 1`; `MSC_NO_MEDIA`, `MSC_PHOTO_NO_SPACE`, `MSC_PHOTO_ERROR` |
| 7 | Waits on `Message Confirm Button`, back to 4 or 0 |

`VitaPhoto_Construct` builds a JPEG writer on GXM's colour surface width and height
(`FUN_8129fb3c(.., w, h, 0x50)`), so the **pixels are the display surface's, quality 80**, with a
`0x64000`-byte buffer. It calls `ScePhotoExport` and `SceShutterSound` (both imported). Confidence
80: `0x50` is that function's last argument, matched against Pulse's and HD's `0x5a` by position.

## What the brief's strings do

- `MSC_PHOTO_CHANGE_CAMERA`, `MSC_PHOTO_CYCLE_LEFT`, `MSC_PHOTO_CYCLE_RIGHT`, and the rest of
  `PhotoControlsTxt`: **labels on one help image**, `Data\FE\NewImages\Photomode_controls.gtf`, shown by
  `Photo Controls Button`. No code reads them. The actions are `InGame VitaPhoto`'s pad handling,
  shared with the mode above (`FUN_811466a6`, called last in `VitaPhoto_Update`, unread: recipe,
  decompile it; expect L, R for the cycle and a button for the camera row).
- `MSC_PHOTO_CANCEL` ("CANCEL PHOTO") and `MSC_PHOTO_CANCEL_PHOTO` ("CANCELLING PHOTO IN...") are
  **not photo mode.** They sit on `InGame2048`'s `gloryImageBG`: the Glory Moment, a front-camera
  picture sent to other racers (`kill_joy`, `Options_Glory_Moment_On`). `RaceManager2048_Construct`
  stores a 5.0 s countdown (`param_1[0xac8d]`) beside `CancelPicture`/`TakePicture`. Leave it out
  of any port; it needs a camera and a session.
