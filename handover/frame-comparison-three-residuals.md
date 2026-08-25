# Frame comparison: three residuals

The pipeline landed and settled the fov unit. Left: (1) the fov reading rests on one ship and one view - a second team's authored value plus the internal view closes it fully, about an hour of emulator time; (2) the shot-versus-row phase is bounded at two ticks but not pinned, which only a fast-moving per-tick capture would do; (3) `place` always writes the basis rows and the `+0xc0` transpose together, so whether the integrator's post-loop rebuild covers the transpose alone was never isolated - academic while the pair works.

**2026-08-25: the fast-moving per-tick capture happened, and it narrowed (2) rather than closing it.** Full account and the trap notes it produced:
[frame-compare.md's Open ends](../docs/tools/frame-compare.md#open-ends). Numbers, for the record:

Setup: `just pads --before 50 > /tmp/pads.csv`, then
`psp-drive.py place --pads-csv /tmp/pads.csv --pad 0 --before 50 --speed 150 --settle 30`
chained in the same shell call as `psp-trace.py --camera --shot-every 1
--ticks 20`, under Xvfb (`OAG_SHOT_DISPLAY=:98`, no compositor in this
session). Settled speed ~100-120 units/s, well above the ~25-30 units/s a
standing-start `--hold cross` capture tops out at - which itself was the
first finding: a cruise-speed capture is *not* fast enough to resolve this,
its NCC scores across neighbouring rows sat within 0.01 of each other,
indistinguishable from noise.

Control (render vs render, both from our own engine, row 10 as reference,
same box excluding the ship and HUD): rows 8-12 scored 0.529, 0.583, **1.000**,
0.710, 0.630. Clean single peak at the self-match, smooth (if asymmetric -
expected, forward motion in a tunnel foreshortens differently than backward)
falloff either side. The instrument resolves one tick of motion.

Shot vs render (the actual question), same box, three ticks sampled from the
same 20-tick capture:

| shot tick | row scores (NCC) | peak | margin |
| --- | --- | --- | --- |
| 10 | r08 0.419, r09 **0.562**, r10 0.460, r11 0.444, r12 0.454 | row 9 (phase -1) | 0.10, clean |
| 15 | r12 0.444, r13 **0.464**, r14 0.450, r15 0.442, r16 0.410, r17 0.392 | row 13 (phase -2) | 0.014, weak, non-monotonic |
| 18 | r16 **0.417**, r17 0.386, r18 0.382, r19 0.406 | row 16 (phase -2) | 0.011, weak, r18 (the "correct" row) scores worst of the four |

Only tick 10 clears the noise floor the render-vs-render control implies. The
other two have registration shifts that move when the correlation search
window is tightened (see the linked doc) - a periodic-scene artifact, not
signal. Working hypothesis, not yet tested: the Xvfb `import -window root`
screenshot path (used because this session had no compositor) has a lag that
is not the same constant the niri clipboard path was calibrated against, and
possibly not even constant itself.

## Open

- The fov reading rests on only one ship and one view
- The shot-versus-row phase's **sign and rough magnitude are now evidenced**
  (negative, i.e. the shot lags the row; 1-2 ticks), but not pinned to one
  number - two of three samples disagree with the third, and the disagreement
  tracks the screenshot mechanism rather than the game.
- Whether the Xvfb `import -window root` screenshot path has the same lag as
  the niri clipboard path the original ~2-tick bound came from is untested.
- Whether the post-loop rebuild covers the `+0xc0` transpose alone (versus paired with the basis rows) was never isolated

## Next Steps

- Get a second team's authored fov value plus the internal view to close the fov reading fully (about an hour of emulator time)
- Re-run the phase capture on a yawing track section (a pad approach into a
  corner, not a straight corridor) so the phase reads off `dx` as a linear
  ramp across candidate rows instead of off a peak height - this sidesteps
  the periodic-scene ambiguity a straight tunnel produces entirely.
- If a real or niri-emulated compositor becomes available, repeat the same
  capture through the clipboard screenshot path and compare against the Xvfb
  numbers above - if the noisy ticks clean up, the lag is Xvfb-specific and
  worth its own line in ppsspp-debugger.md; if not, the phase genuinely
  varies and the tool needs a per-shot correction rather than one constant.
