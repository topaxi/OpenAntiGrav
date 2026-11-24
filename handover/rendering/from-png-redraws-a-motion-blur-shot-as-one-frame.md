---
status: ready-to-wire
titles: [hd, 2048]
updated: 2026-11-24
summary: "`--from-png` draws a motion blur shot as one frame; the blur needs a frame per plan tick and a camera kept relative to the craft."
categories: [tooling]
size: M (55)
difficulty: medium (50)
---

# `--from-png` redraws a motion blur shot as one frame

2026-11-24. A shot's text records its picture controls, including `motion_blur` (`track` or
`ship`), `shutter` and `frames` (`docs/formats/shot-png.md`, `[picture]`). `--from-png` draws the
other controls (exposure, saturation, speed blur, depth of field) and says in its output that a
blur shot is redrawn as the one frame, because the capture route (`race_capture.rs`) draws once.

## Open

- **Draw the plan's frames.** `Plan::new` (`oag_raceplay::photo::shot`) from the recorded kind,
  shutter and frame count gives each frame's `stand` tick, `alpha` and lens shift. The capture
  already drives the race forward to the shot's tick; the frames that stand before it are the
  last `shutter` ticks of that drive, so draw a frame as each `stand` passes (set
  `Race::set_draw_alpha`, `photo_set_shift`), average them as the session does
  (16 bits a channel, `session::photo_rewind`), then draw the HUD once over the average. No
  rewind or keyframe walk is needed, only the drive's own ticks, but the frames must be drawn in
  order and the capture reads back after each.
- **The camera must ride the craft for a track blur.** `from_png::prepare` pins the absolute eye
  of the shot's tick (`--camera-pose`), so a track blur would hold the camera and smear the
  craft: a ship blur's picture, wrong. Record the camera relative to the subject craft
  (offset and orientation in the craft's frame) in `[camera]` or `[picture]`, and have the
  capture pose the camera from the craft at each frame for a track blur, from the shot tick's
  pose for a ship blur (`Shot::start`: `photo_hold`).
- **File size.** `race_capture.rs` is at 1,487 of 1,500 lines. Put the frame loop in a new
  `race_capture/` submodule (as `race_capture/picture.rs` does for `--picture`).
- **Risk.** The track blur camera is the unproven part: how `Photo::pose` follows the craft per
  tick has to be reproduced outside the mode. A test that a `--from-png` track blur keeps the craft
  sharp and smears the scenery (the detail ratios of
  `a_motion_blur_shot_smears_what_it_should_and_any_input_abandons_it`) settles it.
- **Check against Omega and 2048**: HD and 2048 accumulate (`oag_title::photo::Accumulation`);
  Pulse, Pure and Omega do not, so the section never carries a blur there.

## Next Steps

1. Record the camera relative to the craft in the text (`Meta`), written by photo mode.
2. Add the frame loop submodule in `race_capture/`, gated on the recorded blur.
3. Extend `shot_fidelity.rs` with a `--from-png` of the existing track and ship blur shots, comparing
   detail kept in the hull and scenery regions against the saved shot's.
4. Drop the "one frame" line from `from_png::picture_lines` and `docs/formats/shot-png.md`.
