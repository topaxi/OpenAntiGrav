---
status: measure
titles: [all]
updated: 2026-11-04
summary: "wgpu says ending a pass is costly on tiled GPUs; we open 20 to 54 passes a frame and have never counted or timed them on a tiled phone"
categories: [rendering, tooling]
size: M
difficulty: medium
---

# Pass count on a tiled-GPU phone

2026-11-04. wgpu's guidance says a render pass is costly to end on tiled-rendering GPUs, so the
fewest passes and submissions are best. Our Deck work found per-pass setup at about 8% of the frame
thread on a GPU that is not tiled. The phone that ran so far was a Galaxy S24 with an Xclipse 940
(`docs/tools/android.md`), an AMD-derived part; a Snapdragon (Adreno) or Mali phone is the classic
tiled case, and the maintainer has such a phone. Nothing has been counted or timed there.

## Open

- **Which GPU the phone has** decides the question: `adb shell dumpsys SurfaceFlinger | grep -i
  gles`. Adreno and Mali are tile-based; Xclipse is not classically so.
- **Passes a frame per title**: the perf probe counts `pipeline_sets` and marks `pass-open`; a count
  of `begin_render_pass` per frame per title (Pulse, Pure, HD; one view and a split) is not on record
  except HD Anulpha's about 54.
- **Whether the passes show up in the phone's frame**: `--frame-log` `scene` and `submit` on the
  phone with the pass-count levers of the sibling threads (HD sun-occlusion and self-shadow passes,
  bloom and post-chain passes: 10 bloom downsamples and 7 motion-blur passes a frame) switched off
  one at a time.
- The split finish is off on big.LITTLE parts by default (`oag_gpu::split_finish::wanted`, the S24
  measurement), so a phone run is also a data point for the split policy.

## Next Steps

1. One-time setup on the Arch desktop: `docs/tools/android.md` ("Build" and "Install on a phone"),
   `adb` from the `android-tools` package and the `android-udev` rules, USB debugging on.
2. `just apk` (or a perf-probe build), install, `just launch-android`, and read the GPU name from the
   log or `dumpsys`; count passes a frame per title from the probe's `OAG_RENDER_PERF=1` lines.
3. Ablate one pass family at a time on the phone and read `scene` and `submit`; if one family is more
   than about 5% of the frame thread there, file it as its own thread.
