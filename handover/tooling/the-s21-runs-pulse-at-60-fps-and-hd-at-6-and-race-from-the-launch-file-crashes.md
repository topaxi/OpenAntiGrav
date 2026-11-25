---
status: measure
titles: [all]
updated: 2026-11-25
summary: "S21: HD at scale 25 races at 17 fps (CPU 23 ms, glass target 11 ms); --race stays in the flyby on the phone; mixer runs under --no-audio"
categories: [rendering]
size: L (50)
difficulty: medium (50)
---

# The S21 runs Pulse at about 60 fps and HD at about 6 (its launch-file `--race` crash is fixed)

The numbers, the method and the commands are in `docs/tools/android.md`, "Performance on the Galaxy S21" (Galaxy S21, Mali-G78 MP14, main `d5bfef6`,
release build, a 10 minute run per title and preset). Pulse defaults settle at 59.9 fps (median frame 14.7 ms, p99 24.4 ms over two laps);
HD defaults at 7.5 fps from a cold start and 5.9 from a warm one (median frame 118 and 176 ms), 11.1 at render scale 50. On a warm start (USB charging keeps the skin sensor at 36 C) the CPU and GPU caps step down within two minutes of racing (A78 2808 to 1352 MHz, GPU 858 to 403 MHz).

## Open

Ranked by the frame-time win expected on this phone. Each names the evidence and the command that would show the number move.

0. **The launch file's `--race` on the phone never leaves the pre-race flyby** (main `f928359`, HD Vineta K both directions): `simpleperf` has no
   `Race::tick`, `frame-phases` ticks 0.2 ms, the screen is the flyby camera with no HUD. Phone measurements must use the menu route until it is fixed.
   Measure: `--race --frame-log` on the phone, then `adb exec-out screencap -p` after 60 s and a `frame-phases` `ticks` above 1 ms.
0a. **HD racing at render scale 25 is CPU and GPU both** (`docs/tools/android.md`, "Where an HD frame goes on the S21"): frame thread about 23 ms
   (ticks 5.6, scene 8.3, submit 9.1), frame 47 to 62 ms. Per pass with `OAG_SKIP_PASSES` while racing: the glass target 10.6 ms; adverts and bloom inside the noise.
   A `simpleperf` of the frame thread: the audio mixer runs under `--no-audio` (6.8 %), `RaceView::capture_poses` 6 %, particles about 10 %; each a candidate for an exact CPU cut.
   Measure: the same 20 s `simpleperf --app` on a menu-route race and the `frame-phases` `ticks`.
   Picture-changing proposal for the maintainer: the glass target sized off the render scale (the original's is half its 1280x720 output per axis), up to 10.6 ms on Vineta K only.
1. **HD's phone default is a render scale the phone cannot draw.** Scene pass 114 ms at 1906x1080, 41 ms at render scale 50, 14 ms at scale 25 (fps 5.5, 10.6, 16.1, warm starts).
   A mobile default (or dynamic resolution on with a floor of 25 %, where the `minimum_resolution` default is 50) is a 2 to 3 times win for one setting.
   Measure: the same HD race at the new default, `--frame-log` `fps` and `gpu_ms` over race seconds 20 to 80 (docs table, "HD variant").
2. **Nothing reads the phone's thermal state.** The same HD defaults ran 8.4 fps from a cold start and 5.6 from a warm one (GPU 656 against 398 MHz), the Pulse race load 12.8 s cool against 24 to 31 s warm, and the status reached 3 (severe) after 4 minutes at AP 56 C. `AThermal_getThermalHeadroom` (API 31) or the status callback could pick the render scale or a frame cap before the system does it for the GPU clock; expected: steadier fps rather than a higher peak. Measure: the 10 minute runs in the docs section, fps per 30 s window against the thermal status.
3. **HD's frame thread is 28 ms by itself** (render scale 25: ticks 6.6, scene record 9.1, submit 11.5 ms), a cap near 35 fps whatever the GPU does, and the tick is 7 to 13 ms of a slow frame because the fixed step catches up.
   No CPU profile of HD exists; the Pulse one (item 5) is the best guess for `submit`. Measure: `simpleperf record --app org.openantigrav.game -g --duration 20` on HD at scale 25 and read the same inclusive shares.
4. **HD's textures decode to RGBA8 on Mali, which has no `TEXTURE_COMPRESSION_BC`: 2.96 GB of GPU memory against Pulse's 0.47 GB** on a 7 GiB phone, 71.6 s to load a race against 12.8 to 31 s.
   Transcoding to ASTC or ETC2 (or caching the transcode) would cut the memory about 3 to 8 times and the load; the frame-time effect is unknown (anisotropy `off` moved the scene pass 114 to 103 ms, inside the spread).
   Measure: Perfetto `android.gpu.memory` (`scripts/android-perf-trace.cfg`) and `--measure-race-load 1` before and after. Not isolated: that textures are the 2.96 GB is a reading of `oag_mesh::mesh_render::texture::upload`, not a count.
5. **Pulse's frame thread: wgpu encode and submit are 37 % of the app's samples** (`CommandEncoder::finish` to `encode_render_pass` 22 %, `Queue::submit` 15 %, `vkQueueSubmit` 8.5 %), `Scene::prepare_view` 36 % inclusive
   (`Queue::write_buffer` 9.9 %, node animation sampling 6.1 %, raycasts 4.0 %), Mali's backend thread 32 % of a core on the draws. Candidates: render bundles for the static track draws, fewer bind-group changes, one staging buffer for the per-view uploads,
   node animations sampled once a tick. Expected 2 to 4 ms of the frame thread's 9.7 ms. Measure: `frame-phases` `scene` and `submit` (4.4 and 3.9 ms) and the simpleperf shares.
6. **The frame thread sits on the A78 cluster** (46 to 54 % of the trace there, 4 to 22 % on the X1) while the thermal caps bring all three clusters to 1352 MHz; the X1 is the faster core at the same clock.
   `sched_setaffinity` to the X1 from inside the app, or an `APerformanceHint` session (Android 12 and later), would place it. Not measured: `adb shell taskset -p` is refused for a release app's thread.
   Measure: a build that pins, the Pulse defaults 10 minute run, settled fps against 59.9.
7. ~~**`--race` in the launch file overflows `android_main`'s 2 MiB stack.**~~ Fixed: the scene builds on a 16 MiB thread and the launch frames end before the loop (`docs/tools/android.md`, "The stack"); `scripts/android-race-phases.sh` can now run, and still passes no `--no-audio`, which a run on the phone should add.

## Measured and dropped

- HD, render scale 25 (`docs/tools/android.md`, "Where an HD frame goes on the S21"; on the flyby): drawing the resolve and HUD straight onto the surface (slower, 38.7 against 34.6 ms);
  skipping the glass target on frames with no glass draw (exact, but it fires on 2 of 26 frames); a prepass for the glass target (inside the noise, and not exact);
  a portrait surface to avoid SurfaceFlinger's GPU composition (no change); the main track's prepass off, `OAG_BINDLESS=0` (both slower). The catch-up cap is already 8 ticks (`oag_core::tick`).

- HD anisotropy `off` (6.0 against 5.5 fps) and shadows `off` (6.1): inside the run-to-run spread, nothing to take out.
- Pulse at render scale 50: the scene pass halves (4.6 to 2.1 ms) and the frame moves 1 to 2 ms; the three default runs read 68.0, 60.6 and 61.0 fps over their first minute, so 59.9 against 65.8 settled is not a result.
- Pinning with `taskset`: refused for a release app, see item 6.

## Next Steps

0. Fix item 0, then profile item 0a's frame thread for exact CPU cuts (the mixer under `--no-audio` first); the glass-target proposal is the maintainer's.
1. Item 1 first: a phone default for HD's render scale, then the same A/B table.
2. A simpleperf profile of HD at scale 25 (`docs/tools/android.md`, "Reproducing it") to size item 3.
3. Re-run the phase script on both phones (item 7 is fixed), with `--no-audio` added.
4. Compare presets only from one start state (skin 30 C or the same warm state), and run the Pulse defaults twice more to learn the spread (three runs so far).
