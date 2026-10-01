# The default chase view is close on every title, and only Pulse PSP's default is measured

2026-10-01. `CameraView::default()` and the settings default are `Close` for every
title, a maintainer decision taken the same day: close is assumed to be the default
in every title until a title's own original says otherwise. The measurement behind it
is Pulse PSP's alone: two cold boots of the original with an empty profile fly
`OPT_CLOSE`, eye `(-11.25, +3.0)` in the craft's frame, 11.643 units back (far would
be 14.562). See `camera.md`, "The default view is `OPT_CLOSE`, measured 2026-10-01"
([camera.md](../../docs/ghidra/functions/psp-pulse-usa/camera.md)).

## Per title

| Title | Default view | Close vs far distance |
| --- | --- | --- |
| Pulse PSP | **measured** `OPT_CLOSE` (two cold boots) | **measured**: close 11.643, far 14.562 (the `<ExternalCameraClose>`/`<ExternalCameraFar>` blocks at the 0.75 craft scale) |
| Pulse PS2 | assumed close, unmeasured | unmeasured; only `<ExternalCameraFar fov>` is confirmed equal to the PSP's (three ships), the offsets are not compared |
| Pure | assumed close, unmeasured | unmeasured |
| HD / Fury | assumed close, unmeasured | unmeasured; HD's `handlingstats` authors both blocks with the same seven attributes ([hd-status.md](../../docs/formats/hd-status.md)), values not compared |
| 2048 | close **declared** by its own options definition (the picker starts on `OPT_CLOSE`), not measured on a running original | unmeasured |
| Omega | out of scope (no racing) | - |

## Open

- Each unmeasured title's default view, read from the running original on an empty
  profile (PCSX2 for Pulse PS2, PPSSPP for Pure, RPCS3 for HD/Fury and 2048).
- Whether each title's close and far blocks differ from Pulse PSP's: the offsets can
  be read straight off each disc's `handlingstats` (no emulator needed), the runtime
  scale off a running original.
- Two ground-truth fixtures calibrated to the far eye now pin `Far` explicitly
  (`hd_engine_flare_ground_truth`, `sfx_ground_truth`); revisit them if HD's default
  turns out to be measured differently.

## Next Steps

1. Read every title's `<ExternalCameraClose>`/`<ExternalCameraFar>` (or its own
   equivalent) off the discs and fill in the right-hand column.
2. Measure the default view per title on its emulator with an empty profile, the
   way `scripts/psp-camera-pair.py` did for Pulse PSP.
