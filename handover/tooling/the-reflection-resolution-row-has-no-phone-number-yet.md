---
status: measure
titles: [hd, 2048, omega]
updated: 2026-11-25
summary: "graphics.reflection_resolution is wired and measured on desktop; S21 numbers for five HD circuits are open; the default stays original everywhere"
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
2. **Decided, not open:** the default is `original` on every platform (hardware, not platform, sets what a player picks); no Android-only default. The phone numbers only
   tell a player what `scaled` buys on a phone. Visual cost: a coarser view through the glass and in the sea.

## Next Steps

On the S21 (serial `R5CR7015FJZ`, belongs to whichever lane holds it; one at a time), a release build with the new row
(`OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk`, then `adb install -r`), media volume 0, `--no-audio` on every run, start each pair at the same AP sensor temperature.
The launch file takes both arms as arguments, so no settings file changes between them (`docs/tools/android.md`, "Reaching a race, and what does not work", for the menu
route; the launch file's `--race` stays in the pre-race flyby on the phone, so measure on the menu route):

```sh
F=/sdcard/Android/data/org.openantigrav.game/files
OUT=/tmp/reflection-resolution          # per-arm logcat files; anything outside the checkout
mkdir -p $OUT
CIRCUIT=vineta-k                        # one of: vineta-k moa-therma ubermall chenghou modesto
for ARM in original scaled; do
  printf '%s\n' $F/data/oag/images/hdfury-ps3-eu-dec.iso --no-audio --no-intro --frame-log \
      --autopilot --seed 1 --render-scale 25 --reflection-resolution $ARM > /tmp/args
  adb push /tmp/args $F/oag-args.txt
  adb logcat -c
  adb shell am start -n org.openantigrav.game/android.app.NativeActivity
  # menu route: taps in "Reaching a race", then pick $CIRCUIT on the track select; race 3 minutes
  sleep 240
  adb exec-out screencap -p > $OUT/$CIRCUIT-$ARM.png
  adb logcat -d -v threadtime -s oag RustStdoutStderr > $OUT/$CIRCUIT-$ARM.log
  adb shell am force-stop org.openantigrav.game
done
```

`--reflection-resolution` reaches the menu route's profile (`oag_game::app` applies every render override to every title's
profile before the menu opens), so the two arms differ in the load's target size and nothing else.

Per circuit, on the HD track select (a tap on a hex moves the cursor and does not confirm; read the name off a `screencap`, then `input keyevent 96` twice):

| Circuit | Track select name | Target at scale 25 under `scaled` | What to read |
| --- | --- | --- | --- |
| Vineta K | VINETA K | 160x90 | frame ms and `gpu_ms` in the `--frame-log`, both arms; look at the tunnel glass in a `screencap` |
| Moa Therma | MOA THERMA | 240x135 | the sea ahead of the tower, a `screencap` per arm |
| Ubermall | UBERMALL | 240x135 | the pool and any water |
| Chenghou | CHENGHOU PROJECT | 240x135 (sky alone) | frame ms only; the sea shows the sky |
| Modesto | MODESTO HEIGHTS | 240x135 (sky alone) | frame ms only |

The launch file only sets `OAG_*` variables, so the loader report line (`the glass and sea target is WxH at render scale 25`, `debug` level) is out of reach
there; the arm shows in the `screencap` (the glass and the sea are visibly coarser under `scaled`) and in the frame time. Read the median `frame_ms` and `gpu_ms` of the racing
seconds of each log's `--frame-log` lines (the first 15 s are the flyby and the countdown). Add the per-circuit milliseconds to the water page's table and delete this thread when item 1 is done.
