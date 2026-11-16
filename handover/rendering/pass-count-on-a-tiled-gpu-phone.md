---
status: blocked-hardware
titles: [all]
updated: 2026-11-04
summary: "Ending a pass costs on tiled GPUs; the S24 (Xclipse 940) is not tiled, so an Adreno or Mali phone is needed; passes a frame uncounted there"
categories: [rendering, tooling]
size: M
difficulty: medium
---

# Pass count on a tiled-GPU phone

2026-11-04. wgpu's guidance says a render pass is costly to end on tiled-rendering GPUs, so the
fewest passes and submissions are best. Our Deck work found per-pass setup at about 8% of the frame
thread on a GPU that is not tiled. **The phone on hand is the tiled-GPU test's wrong device**: an
SM-S921B, the Galaxy S24 (EU) with the Exynos 2400 and an Xclipse 940 (`adb shell dumpsys
SurfaceFlinger`: "ANGLE (Samsung Xclipse 940) on Vulkan 1.3.304"), the same phone
`docs/tools/android.md` already ran on. Xclipse is AMD RDNA3-derived, not classically tile-based. A
Snapdragon (Adreno; the US S24 is one), Mali or PowerVR phone is the tiled case. Nothing has been
counted or timed on a tiled device.

## Open

- **A tiled device**: an Adreno, Mali or PowerVR phone. `adb shell dumpsys SurfaceFlinger | grep -i
  gles` names the GPU. Without one this thread stays blocked on hardware.
- **Passes a frame per title**: the perf probe counts `pipeline_sets` and marks `pass-open`; a count
  of `begin_render_pass` per frame per title (Pulse, Pure, HD; one view and a split) is not on record
  except HD Anulpha's about 54.
- **Whether the passes show up in the phone's frame**: `--frame-log` `scene` and `submit` on the
  phone with the pass-count levers of the sibling threads (HD sun-occlusion and self-shadow passes,
  bloom and post-chain passes: 10 bloom downsamples and 7 motion-blur passes a frame) switched off
  one at a time.
- **What the S24 can still give now**: passes a frame per title and the cost of the pass families on a
  big.LITTLE phone with an immediate-mode GPU (cpu_capacity 253 to 1024, so the split finish is off
  there by default, `oag_gpu::split_finish::wanted`). That is a data point for the split policy and
  for the CPU side of pass count, not for the tiled question.

## Next Steps

1. One-time setup on the Arch desktop: `docs/tools/android.md` ("Build" and "Install on a phone"),
   `adb` from the `android-tools` package and the `android-udev` rules, USB debugging on.
2. `just apk` (or a perf-probe build), install, `just launch-android`; count passes a frame per title
   from the probe's `OAG_RENDER_PERF=1` lines. The S24 serial on the maintainer's desktop is
   `R3CX707JA9T`.
3. Ablate one pass family at a time on the phone and read `scene` and `submit`; if one family is more
   than about 5% of the frame thread there, file it as its own thread.
