---
status: measure
titles: [all]
updated: 2026-11-22
summary: "Touch photo mode and replay viewer are built and unit-tested; the gesture constants, layout and the phone's cutout insets need a phone"
---

# Touch photo mode and the replay viewer need a phone pass

Landed: the multi-touch camera for photo mode and the replay viewer
([photo mode](../../docs/ui/photo-mode.md), "Touch"; [replays](../../docs/ui/replays.md), "Touch"),
and photo mode, the rewind keeper and auto-recording on for phones. Every gesture is a unit test on
synthetic touches and `photo_mode_ground_truth::fingers_move_the_view_and_never_the_race` drives it
through `Session::touch` on desktop. None of it has been held in a hand.

## Open

- **The constants are guesses.** `oag_input::gesture` (`TAP_SLOP` 0.02 h, `TAP_TICKS` 18,
  `DOUBLE_TAP_*`, `PINCH_SLOP`, `TWIST_SLOP`, `FRICTION` 0.90, `MIN_FLING`) and
  `oag_raceplay::photo::touch` (`TOUCH_ORBIT`, `TOUCH_PAN`, `TOUCH_SLIDE`, `TOUCH_DOLLY`,
  `TOUCH_MOVE`) are fractions of the window's height with no DPI: tune them on a phone.
- **The sign of every gesture** is pinned by a test but whether it *feels* like a viewer
  (orbit direction on `close`, the drag-to-move direction, roll) is not known.
- **A drag on `track` or `internal` spends its first tick seeding the free lens**, so the opening
  movement is lost; whether that is visible is a phone question.
- **The layouts keep to `oag_input::touch::SAFE_X`/`SAFE_Y`**, a chosen inset: the real cutout and
  gesture-bar insets are not queried (winit gives none), and the SCREEN SIZE row's inset is not applied
  to photo mode's or the viewer's chips. The effects panel's rows are 25 grid units (0.092 h), under the
  0.11 h of a secondary target, because ten rows must fit.
- **2048 has no MENU chip.** Its pause screen is `None` (`Title::pause_menu`), so on a phone photo mode
  is reached by the Back key only. A `Control::Menu` in `oag_input::touch`, drawn while paused where the
  title has no pause screen, would put it on the glass.
- **The viewer's seek cost on a phone.** A held finger on the bar seeks every four ticks; each seek
  restores a keyframe and plays 60 ticks. The rewind keeper (up to 16 MiB) and the auto-replay ring (up to
  32 MiB on disk) now run on a phone. Measure both.
- **The new strings are English-only** (`OAG_PHOTO_TOUCH_HINT`, `OAG_PHOTO_TOUCH_HINT_FREE`,
  `OAG_REPLAY_CAMERA`, named in the other languages' `[untranslated]`).
- **iOS** compiles the same paths and was never built or run (checked, not wired).
- **Gallery and share export** is the other thread.

## Next Steps

Hold a phone, play a race, open photo mode from the pause menu and the viewer from REPLAYS, and
change the constants above until the camera does what a thumb expects. Then delete this thread and
move what is still true into the two doc pages.
