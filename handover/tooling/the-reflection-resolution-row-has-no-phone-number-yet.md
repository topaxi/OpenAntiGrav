---
status: measure
titles: [hd, 2048, omega]
updated: 2026-11-25
summary: "graphics.reflection_resolution is wired and measured on desktop; S21 numbers for five HD circuits and the Android default are open"
categories: [rendering]
size: S (30)
difficulty: easy (30)
---

# The reflection resolution row has no phone number yet

`graphics.reflection_resolution` (`original` | `scaled`, per title, `--reflection-resolution` on the command line) sizes the screen-space glass and sea target
from the render scale. The design, the sizes and the desktop GPU numbers are in `docs/rendering/water.md`, "The reflection target's resolution".
The default is `original` everywhere, byte-identical on desktop. The only phone figure is the earlier per-pass one: Vineta K's glass costs 10.6 ms at render scale 25
(`docs/tools/android.md`, "Where an HD frame goes on the S21").

## Open

1. **S21 frame time per circuit, `original` against `scaled` at render scale 25.** Vineta K (640x360 glass, 160x90 scaled), Moa Therma and Ubermall (960x540 sea
   reflection, 240x135 scaled; reflected chunks drawn), Chenghou and Modesto (the same target holding the sky alone). The sea circuits' cost on the phone is unmeasured.
2. **Whether Android defaults a phone to `scaled`.** The maintainer decides after item 1. Visual cost: a coarser view through the glass and in the sea.
3. **A 2048 or Omega circuit under `scaled`, disc-backed.** The same call site and law, no disc test loads one.

## Next Steps

On the S21 (serial `R5CR7015FJZ`, belongs to whichever lane holds it; one at a time), a release build with the new row
(`OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk`, then `adb install -r`), media volume 0, `--no-audio` on every run, start each pair at the same AP sensor temperature.
The launch file takes both arms as arguments, so no settings file changes between them (`docs/tools/android.md`, "Reaching a race, and what does not work", for the menu
route; the launch file's `--race` stays in the pre-race flyby on the phone, so measure on the menu route):

```sh
F=/sdcard/Android/data/org.openantigrav.game/files
for ARM in original scaled; do
  printf '%s\n' $F/data/oag/images/hdfury-ps3-eu-dec.iso --no-audio --no-intro --frame-log \
      --autopilot --seed 1 --render-scale 25 --reflection-resolution $ARM > /tmp/args
  adb push /tmp/args $F/oag-args.txt
  adb shell am start -n org.openantigrav.game/android.app.NativeActivity
  # menu route: taps in "Reaching a race"; pick the circuit below; let it race 3 minutes
  adb logcat -v threadtime -s oag RustStdoutStderr > data/scratch/<lane>/$CIRCUIT-$ARM.log
  adb shell am force-stop org.openantigrav.game
done
# then the median frame_ms and gpu_ms of the racing seconds of each log's --frame-log lines (the first 15 s are the flyby and the countdown)
```

Per circuit, on the HD track select (a tap on a hex moves the cursor and does not confirm; read the name off a `screencap`, then `input keyevent 96` twice):

| Circuit | Track select name | Target at scale 25 under `scaled` | What to read |
| --- | --- | --- | --- |
| Vineta K | VINETA K | 160x90 | frame ms and `gpu_ms` in the `--frame-log`, both arms; look at the tunnel glass in a `screencap` |
| Moa Therma | MOA THERMA | 240x135 | the sea ahead of the tower, a `screencap` per arm |
| Ubermall | UBERMALL | 240x135 | the pool and any water |
| Chenghou | CHENGHOU PROJECT | 240x135 (sky alone) | frame ms only; the sea shows the sky |
| Modesto | MODESTO HEIGHTS | 240x135 (sky alone) | frame ms only |

The loader report line (`the glass and sea target is WxH at render scale 25`) confirms the arm took effect; it is `debug` level, so run with `RUST_LOG=oag=debug` as an
`OAG_NAME=value` line if it is needed. Add the per-circuit milliseconds to the water page's table, then ask the maintainer about the Android default. Delete this thread when item 2 is answered.
