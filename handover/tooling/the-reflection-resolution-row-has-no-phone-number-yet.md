---
status: measure
titles: [hd, 2048, omega]
updated: 2026-11-25
summary: "reflection_resolution is wired; S21 Vineta K quarter measured (41.9 to 24.9 ms); half and the four other circuits are open; default stays original"
categories: [rendering]
size: S (30)
difficulty: easy (30)
---

# The reflection resolution row has no phone number yet

`graphics.reflection_resolution` (`original` | `half` | `quarter`, per title, `--reflection-resolution` on the command line) sizes the screen-space glass and sea target
as a fixed fraction of the original's, whatever the render scale. A photo always draws the original's size. The design, the sizes and the desktop GPU numbers are in `docs/rendering/water.md`, "The reflection target's resolution".
The default is `original` everywhere, byte-identical on desktop. The only phone figure is the earlier per-pass one: Vineta K's glass costs 10.6 ms at render scale 25
(`docs/tools/android.md`, "Where an HD frame goes on the S21").

## Open

1. **S21 frame time per circuit, `half` on Vineta K and `original` against `half` and `quarter` on the others, at render scale 25.** Vineta K `original` against `quarter` is done: median
   41.9 to 24.9 ms, 23.5 to 37.9 fps, ABBA at AP 32 C (`docs/tools/android.md`, "What `quarter` buys on Vineta K"). Open: Vineta K `half` (320x180); Moa Therma and Ubermall (960x540 sea
   reflection, 480x270 and 240x135; reflected chunks drawn); Chenghou and Modesto (the same target holding the sky alone). The sea circuits' cost on the phone is unmeasured.
   Also open: why `quarter` buys 17 ms when leaving the glass pass out read 10.6 ms (the scene `gpu_ms` barely moved, the present wait did).
2. **Decided, not open:** the default is `original` on every platform (hardware, not platform, sets what a player picks); no Android-only default. The phone numbers only
   tell a player what `half` and `quarter` buy on a phone. Visual cost: a coarser view through the glass and in the sea.

## Next Steps

On the S21 (serial `R5CR7015FJZ`, belongs to whichever lane holds it; one at a time), a release build with the new row
(`OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk`, then `adb install -r`), media volume 0, `--no-audio` on every run, start each pair at the same AP sensor temperature.
The launch file takes both arms as arguments, so no settings file changes between them. `--race` reaches racing on the phone now, but takes its circuit from `[race] track`
(Talon's Junction on a Pulse-written profile), so name the circuit with `--track /data/environments/<folder>/track.vex` (`01_vineta_k` is measured; read the other folders off the
loader report's `racing on` line); the race is 3 laps, about 100 s, and the figures come from racing seconds 10 to 100. Cool to the same AP and skin temperature before each arm, order ABBA:

```sh
F=/sdcard/Android/data/org.openantigrav.game/files
OUT=/tmp/reflection-resolution          # per-arm logcat files; anything outside the checkout
mkdir -p $OUT
CIRCUIT=vineta-k                        # label for the file names; set the --track below to match
for ARM in original half quarter; do   # the pair that is measured is original and quarter
  printf '%s\n' $F/data/oag/images/hdfury-ps3-eu-dec.iso --no-audio --no-intro --frame-log \
      --autopilot --seed 1 --render-scale 25 --reflection-resolution $ARM --race \
      --track /data/environments/01_vineta_k/track.vex OAG_LOG_FILE_FILTER=warn,oag=debug > /tmp/args
  adb push /tmp/args $F/oag-args.txt
  adb logcat -c
  adb shell am start -n org.openantigrav.game/android.app.NativeActivity
  sleep 150
  adb exec-out screencap -p > $OUT/$CIRCUIT-$ARM.png
  adb logcat -d -v threadtime -s oag RustStdoutStderr > $OUT/$CIRCUIT-$ARM.log
  adb shell am force-stop org.openantigrav.game
done
```

Per circuit (`--track /data/environments/<folder>/track.vex`; the folders are the `row(...)` names in `crates/hd/src/frontend.rs`, lower-cased: `04_chenghou_project`,
`05_ubermall`, `modesto_heights`; Moa Therma's is one of `02_track` and `03_track`, read it off the loader report's `racing on` line):

| Circuit | Target under `half` / `quarter` | What to read |
| --- | --- | --- |
| Vineta K | 320x180 / 160x90 | `half` only (`original` and `quarter` are done) |
| Moa Therma | 480x270 / 240x135 | frame ms per arm and the sea ahead of the tower, a `screencap` mid-race per arm |
| Ubermall | 480x270 / 240x135 | frame ms, the pool and any water |
| Chenghou | 480x270 / 240x135 (sky alone) | frame ms only; the sea shows the sky |
| Modesto | 480x270 / 240x135 (sky alone) | frame ms only |

`OAG_LOG_FILE_FILTER=warn,oag=debug` as a launch-file line writes `files/state/oag/logs/oag-game.log`, whose `the glass and sea target is WxH at reflection resolution <arm>` line confirms the arm.
Read the median and p95 of the per-second `frame_ms` of racing seconds 10 to 100 (a 3-lap race is about 100 s, so a race is one window; the screen after it is the results). Add the per-circuit
milliseconds to `docs/tools/android.md`, "What `quarter` buys on Vineta K", and delete this thread when item 1 is done.
